use std::{collections::HashMap, path::{Path, PathBuf}, pin::Pin, sync::Arc, time::Duration};

use regex::Regex;
use tokio::{signal::unix::{signal, SignalKind}, sync::{broadcast, Mutex}, task::JoinSet};
use uuid::Uuid;

use crate::common::tracking_info::TrackingInfo;

type FileCallback = Arc<dyn Fn(TrackingInfo, PathBuf) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[derive(Clone)]
pub enum FileReceiverEventSignal {
    OnFileReceived(TrackingInfo, PathBuf),
}

pub struct FileReceiver {
    path: String,
    filter: HashMap<String, FileCallback>,
    event_broadcast: broadcast::Sender<FileReceiverEventSignal>,
    poll_interval: u64,
}

impl FileReceiver {
    pub fn new(path: &str) -> Self {
        let (event_broadcast, _) = broadcast::channel(100);
        FileReceiver {
            path: path.to_string(),
            filter: HashMap::new(),
            event_broadcast,
            poll_interval: 500,
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

    pub fn get_event_broadcast(&self) -> broadcast::Receiver<FileReceiverEventSignal> {
        self.event_broadcast.subscribe()
    }

    pub async fn start(self) {
        let path = Path::new(&self.path);
        match &path.try_exists() {
            Ok(true) => {},
            Ok(false) => panic!("The path '{}' does not exist!", &self.path),
            Err(err) => panic!("{}", err.to_string()),
        }

        let ignore_list = Arc::new(Mutex::new(Vec::<String>::new()));
        let mut join_set = JoinSet::new();
        let mut interval = tokio::time::interval(Duration::from_millis(self.poll_interval));
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        loop {
            tokio::select! {
                _ = sigterm.recv() => break,
                _ = sigint.recv() => break,
                _ = async {
                    let files: Vec<PathBuf> = match Self::get_files_in_directory(&path).await {
                        Ok(files) => files,
                        Err(err) => panic!("{}", err.to_string()),
                    };
        
                    for (filter, callback) in self.filter.iter() {
                        let regex = Regex::new(filter).unwrap();
        
                        for file in &files {
                            let file_name = file.file_name().unwrap().to_str().unwrap().to_string();
                            
                            if regex.is_match(&file_name) {
                                let mut unlocked_list = ignore_list.lock().await;
                                if unlocked_list.contains(&file_name) {
                                    println!("Ignored: {}", &file_name);
                                    continue;
                                }
                                unlocked_list.push(file_name.to_string());
                                drop(unlocked_list);
                                
                                let callback = Arc::clone(&callback);
                                let file = Arc::new(file.clone());
                                let ignore_list = Arc::clone(&ignore_list);
                                let tracking = TrackingInfo::new()
                                    .uuid(Uuid::new_v4().to_string());
                                
                                self.event_broadcast.send(FileReceiverEventSignal::OnFileReceived(tracking.clone(), file.to_path_buf())).ok();
                                join_set.spawn(async move {
                                    callback(tracking.clone(), file.to_path_buf()).await;
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