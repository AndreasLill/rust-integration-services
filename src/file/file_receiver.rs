use std::{collections::HashMap, path::{Path, PathBuf}, pin::Pin, sync::Arc, time::Duration};

use regex::Regex;
use tokio::{signal::unix::{signal, SignalKind}, sync::Mutex, task::JoinSet};
use uuid::Uuid;

use crate::common::tracking_info::TrackingInfo;

type FileCallback = Arc<dyn Fn(TrackingInfo, PathBuf) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
type OnFileReceivedCallback = Arc<dyn Fn(TrackingInfo, PathBuf) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[derive(Clone)]
pub enum FileReceiverEventSignal {
    OnFileReceived(TrackingInfo, PathBuf),
}

pub struct FileReceiver {
    path: String,
    filter: HashMap<String, FileCallback>,
    poll_interval: u64,
    on_file_received: OnFileReceivedCallback,
}

impl FileReceiver {
    pub fn new(path: &str) -> Self {
        FileReceiver {
            path: path.to_string(),
            filter: HashMap::new(),
            poll_interval: 500,
            on_file_received: Arc::new(|_,_| Box::pin(async {})),
        }
    }

    pub fn poll_interval(mut self, interval: u64) -> Self {
        self.poll_interval = interval;
        self
    }

    pub fn filter<T, Fut>(mut self, filter: &str, callback: T) -> Self 
    where
        T: Fn(TrackingInfo, PathBuf) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Regex::new(filter).expect("Invalid Regex!");
        self.filter.insert(filter.to_string(), Arc::new(move |tracking, path| Box::pin(callback(tracking, path))));
        self
    }

    pub async fn start(self) {
        let path = Path::new(&self.path);
        match &path.try_exists() {
            Ok(true) => {},
            Ok(false) => panic!("The path '{}' does not exist!", &self.path),
            Err(err) => panic!("{}", err.to_string()),
        }

        let mut main_join_set = JoinSet::new();
        let mut callback_join_set = JoinSet::new();

        let mut interval = tokio::time::interval(Duration::from_millis(self.poll_interval));
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
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
                                let tracking = TrackingInfo::new().uuid(Uuid::new_v4().to_string());
                            
                                let file_path_clone = file_path.clone();
                                let tracking_clone = tracking.clone();
                                let on_file_received = Arc::clone(&self.on_file_received);
                                callback_join_set.spawn(async move {
                                    on_file_received(tracking_clone, file_path_clone.to_path_buf()).await;
                                });
                                
                                main_join_set.spawn(async move {
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

        while let Some(_) = main_join_set.join_next().await {}
        while let Some(_) = callback_join_set.join_next().await {}
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

    pub fn on_file_received<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn(TrackingInfo, PathBuf) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_file_received = Arc::new(move |tracking, path| Box::pin(callback(tracking, path)));
        self
    }
}