use std::{collections::HashMap, path::{Path, PathBuf}, pin::Pin, sync::Arc, time::Duration};
use regex::Regex;
use tokio::{signal::unix::{signal, SignalKind}, sync::{broadcast, Mutex}, task::JoinSet};
use uuid::Uuid;

use crate::file::file_tracking::FileTracking;

type FileCallback = Arc<dyn Fn(FileTracking, PathBuf) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[derive(Clone)]
pub enum FileReceiverEventSignal {
    OnFileReceived(FileTracking, PathBuf),
}

pub struct FileReceiver {
    path: String,
    filter: HashMap<String, FileCallback>,
    event_broadcast: broadcast::Sender<FileReceiverEventSignal>,
    event_join_set: JoinSet<()>,
    poll_interval: u64,
}

impl FileReceiver {
    pub fn new(path: &str) -> Self {
        let (event_broadcast, _) = broadcast::channel(100);
        FileReceiver {
            path: path.to_string(),
            filter: HashMap::new(),
            event_broadcast,
            event_join_set: JoinSet::new(),
            poll_interval: 500,
        }
    }

    pub fn poll_interval(mut self, interval: u64) -> Self {
        self.poll_interval = interval;
        self
    }

    pub fn filter<T, Fut>(mut self, filter: &str, callback: T) -> Self 
    where
        T: Fn(FileTracking, PathBuf) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Regex::new(filter).expect("Invalid Regex!");
        self.filter.insert(filter.to_string(), Arc::new(move |tracking, path| Box::pin(callback(tracking, path))));
        self
    }

    pub fn on_event<T, Fut>(mut self, handler: T) -> Self
    where
        T: Fn(FileReceiverEventSignal) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut rx = self.event_broadcast.subscribe();
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver.");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver.");
        self.event_join_set.spawn(async move {
            loop {
                tokio::select! {
                    _ = sigterm.recv() => break,
                    _ = sigint.recv() => break,
                    event = rx.recv() => {
                        match event {
                            Ok(event) => handler(event).await,
                            Err(broadcast::error::RecvError::Closed) => break,
                            Err(broadcast::error::RecvError::Lagged(_)) => {}
                        }
                    }
                }
            }
        });
        self
    }

    pub async fn start(self) {
        let path = Path::new(&self.path);
        match &path.try_exists() {
            Ok(true) => {},
            Ok(false) => panic!("The path '{}' does not exist!", &self.path),
            Err(err) => panic!("{}", err.to_string()),
        }

        let mut join_set = JoinSet::new();
        let mut interval = tokio::time::interval(Duration::from_millis(self.poll_interval));
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver.");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver.");
        let ignore_list = Arc::new(Mutex::new(Vec::<String>::new()));
        let filter_map = Arc::new(self.filter.clone());
        
        loop {
            tokio::select! {
                _ = sigterm.recv() => break,
                _ = sigint.recv() => break,
                _ = async {
                    let files: Vec<PathBuf> = match Self::get_files_in_directory(&path).await {
                        Ok(files) => files,
                        Err(err) => panic!("{}", err.to_string()),
                    };
        
                    for (filter, callback) in filter_map.iter() {
                        let regex = Regex::new(filter).unwrap();
        
                        for file_path in &files {
                            let file_name = file_path.file_name().unwrap().to_str().unwrap().to_string();
                            
                            if regex.is_match(&file_name) {
                                let mut unlocked_list = ignore_list.lock().await;
                                if unlocked_list.contains(&file_name) {
                                    println!("Ignored: {}", &file_name);
                                    continue;
                                }
                                unlocked_list.push(file_name.to_string());
                                drop(unlocked_list);
                                
                                let callback = Arc::clone(&callback);
                                let ignore_list = Arc::clone(&ignore_list);
                                let file_path = Arc::new(file_path.to_path_buf());
                                let tracking = FileTracking {
                                    uuid: Uuid::new_v4().to_string(),
                                };
                                let event_broadcast = Arc::new(self.event_broadcast.clone());
                            
                                event_broadcast.send(FileReceiverEventSignal::OnFileReceived(tracking.clone(), file_path.to_path_buf())).ok();
                                join_set.spawn(async move {
                                    callback(tracking, file_path.to_path_buf()).await;
                                    let mut unlocked_list = ignore_list.lock().await;
                                    if let Some(pos) = unlocked_list.iter().position(|item| item == &file_name) {
                                        unlocked_list.remove(pos);
                                    }
                                });
                            }
                        }
                    }

                    interval.tick().await;
                } => {}
            }
        }

        while let Some(_) = join_set.join_next().await {}
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