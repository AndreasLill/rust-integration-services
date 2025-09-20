use std::{collections::{HashMap, HashSet}, panic::AssertUnwindSafe, path::{Path, PathBuf}, pin::Pin, sync::Arc, time::Duration};
use futures::FutureExt;
use regex::Regex;
use tokio::{signal::unix::{signal, SignalKind}, sync::Mutex, task::JoinSet};
use uuid::Uuid;

type FileCallback = Arc<dyn Fn(String, PathBuf) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub struct FileReceiver {
    source_path: PathBuf,
    routes: Vec<(Regex, FileCallback)>,
}

impl FileReceiver {
    pub fn new<T: AsRef<Path>>(source_path: T) -> Self {
        FileReceiver {
            source_path: source_path.as_ref().to_path_buf(),
            routes: Vec::new(),
        }
    }

    /// Callback that returns all file paths from the route using regular expression.
    pub fn route<T, Fut, S>(mut self, filter: S, callback: T) -> Self 
    where
        T: Fn(String, PathBuf) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        S: AsRef<str>,
    {
        if self.routes.iter().any(|(r, _)| r.as_str() == filter.as_ref()) {
            panic!("Route already exists with filter {}", filter.as_ref())
        }
        let regex = Regex::new(filter.as_ref()).expect("Invalid Regex");
        self.routes.push((regex, Arc::new(move |uuid, path| Box::pin(callback(uuid, path)))));
        self
    }

    pub async fn receive(self) {
        let mut receiver_join_set = JoinSet::new();
        let mut interval = tokio::time::interval(Duration::from_millis(1500));
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver");
        let lock_list = Arc::new(Mutex::new(HashSet::<String>::new()));
        let route_map = Arc::new(self.routes.clone());
        let mut size_map = HashMap::<String, u64>::new();
        
        log::trace!("started on {:?}", self.source_path);
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
                                if !route.is_match(&file_name) {
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
                            
                                log::trace!("[{}] file received at {:?}", uuid, file_path);
                                receiver_join_set.spawn(async move {
                                    let callback_fut = callback(uuid.to_string(), file_path.to_path_buf());
                                    let result = AssertUnwindSafe(callback_fut).catch_unwind().await;
                                    match result {
                                        Ok(_) => log::trace!("[{}] file processed", uuid),
                                        Err(err) => log::error!("[{}] {:?}", uuid, err),
                                    };

                                    let mut unlocked_list = lock_list.lock().await;
                                    unlocked_list.remove(&file_name);
                                });
                            }
                        }
                    }

                    interval.tick().await;
                }
            }
        }

        log::trace!("shut down pending...");
        while let Some(_) = receiver_join_set.join_next().await {}
        log::trace!("shut down complete");
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