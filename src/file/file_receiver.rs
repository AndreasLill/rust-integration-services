use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}, pin::Pin, sync::Arc, time::Duration};
use regex::Regex;
use tokio::{signal::unix::{signal, SignalKind}, sync::Mutex, task::JoinSet};
use uuid::Uuid;

use crate::common::event_handler::EventHandler;

type FileCallback = Arc<dyn Fn(String, PathBuf) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[derive(Clone)]
pub enum FileReceiverEventSignal {
    OnFileReceived(String, PathBuf),
    OnFileProcessed(String, PathBuf),
}

pub struct FileReceiver {
    source_path: PathBuf,
    routes: HashMap<String, FileCallback>,
    event_handler: EventHandler<FileReceiverEventSignal>,
    event_join_set: JoinSet<()>,
}

impl FileReceiver {
    pub fn new<T: AsRef<Path>>(source_path: T) -> Self {
        FileReceiver {
            source_path: source_path.as_ref().to_path_buf(),
            routes: HashMap::new(),
            event_handler: EventHandler::new(),
            event_join_set: JoinSet::new(),
        }
    }

    /// Callback that returns all file paths from the route using regular expression.
    pub fn route<T, Fut, S>(mut self, filter: S, callback: T) -> Self 
    where
        T: Fn(String, PathBuf) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        S: AsRef<str>,
    {
        Regex::new(filter.as_ref()).expect("Invalid Regex");
        self.routes.insert(filter.as_ref().to_string(), Arc::new(move |uuid, path| Box::pin(callback(uuid, path))));
        self
    }

    pub fn on_event<T, Fut>(mut self, handler: T) -> Self
    where
        T: Fn(FileReceiverEventSignal) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.event_join_set = self.event_handler.init(handler);
        self
    }

    pub async fn receive(mut self) {
        let mut receiver_join_set = JoinSet::new();
        let mut interval = tokio::time::interval(Duration::from_millis(1500));
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver");
        let lock_list = Arc::new(Mutex::new(HashSet::<String>::new()));
        let route_map = Arc::new(self.routes.clone());
        let mut size_map = HashMap::<String, u64>::new();
        
        loop {
            tokio::select! {
                _ = sigterm.recv() => break,
                _ = sigint.recv() => break,
                _ = async {} => {
                    if let Ok(files) = Self::get_files_in_directory(&self.source_path).await {
                        for file_path in &files {
                            for (route, callback) in route_map.iter() {
                                let file_name = file_path.file_name().unwrap().to_str().unwrap().to_string();
                                
                                // Check if the filter regex matches the file.
                                let regex = Regex::new(route).unwrap();
                                if !regex.is_match(&file_name) {
                                    continue;
                                }
                                
                                // Check if the file is being copied by comparing the current size with the previous polling size.
                                let size = tokio::fs::metadata(file_path).await.unwrap().len();
                                match size_map.get_mut(&file_name) {
                                    Some(old_size) => {
                                        if size > *old_size {
                                            *old_size = size;
                                            continue;
                                        }
                                        size_map.remove(&file_name);
                                    }
                                    None => {
                                        size_map.insert(file_name, size);
                                        continue;
                                    }
                                }
    
                                // Add file to a locked list to avoid multiple tasks processing the same file.
                                let mut unlocked_list = lock_list.lock().await;
                                if unlocked_list.contains(&file_name) {
                                    continue;
                                }
                                unlocked_list.insert(file_name.clone());
                                drop(unlocked_list);
                                
                                let callback = Arc::clone(&callback);
                                let lock_list = Arc::clone(&lock_list);
                                let file_path = Arc::new(file_path.to_path_buf());
                                let uuid = Uuid::new_v4().to_string();
                                let event_broadcast = Arc::new(self.event_handler.broadcast());
                            
                                event_broadcast.send(FileReceiverEventSignal::OnFileReceived(uuid.clone(), file_path.to_path_buf())).await.unwrap();
                                receiver_join_set.spawn(async move {
                                    callback(uuid.clone(), file_path.to_path_buf()).await;
                                    let mut unlocked_list = lock_list.lock().await;
                                    unlocked_list.remove(&file_name);
                                    event_broadcast.send(FileReceiverEventSignal::OnFileProcessed(uuid.clone(), file_path.to_path_buf())).await.unwrap();
                                });
                            }
                        }
                    }

                    interval.tick().await;
                }
            }
        }

        while let Some(_) = receiver_join_set.join_next().await {}
        while let Some(_) = self.event_join_set.join_next().await {}
    }

    async fn get_files_in_directory(path: &Path) -> tokio::io::Result<Vec<PathBuf>> {
        let mut files: Vec<PathBuf> = Vec::new();
        let mut entries = tokio::fs::read_dir(&path).await?;

        while let Some(file) = entries.next_entry().await? {
            let file = file.path();
            if file.is_file() {
                files.push(file);
            }
        }

        Ok(files)
    }
}