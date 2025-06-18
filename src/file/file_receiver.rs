use std::{collections::HashMap, path::{Path, PathBuf}, pin::Pin, sync::Arc, time::Duration};
use regex::Regex;
use tokio::{signal::unix::{signal, SignalKind}, sync::{mpsc, Mutex}, task::JoinSet};
use uuid::Uuid;

type FileCallback = Arc<dyn Fn(String, PathBuf) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[derive(Clone)]
pub enum FileReceiverEventSignal {
    OnFileReceived(String, PathBuf),
}

pub struct FileReceiver {
    path: String,
    filter: HashMap<String, FileCallback>,
    event_broadcast: mpsc::Sender<FileReceiverEventSignal>,
    event_receiver: Option<mpsc::Receiver<FileReceiverEventSignal>>,
    event_join_set: JoinSet<()>,
}

impl FileReceiver {
    pub fn new(path: &str) -> Self {
        let (event_broadcast, event_receiver) = mpsc::channel(128);
        FileReceiver {
            path: path.to_string(),
            filter: HashMap::new(),
            event_broadcast,
            event_receiver: Some(event_receiver),
            event_join_set: JoinSet::new(),
        }
    }

    pub fn filter<T, Fut>(mut self, filter: &str, callback: T) -> Self 
    where
        T: Fn(String, PathBuf) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Regex::new(filter).expect("Invalid Regex!");
        self.filter.insert(filter.to_string(), Arc::new(move |uuid, path| Box::pin(callback(uuid, path))));
        self
    }

    pub fn on_event<T, Fut>(mut self, handler: T) -> Self
    where
        T: Fn(FileReceiverEventSignal) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut receiver = self.event_receiver.unwrap();
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver.");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver.");
        
        self.event_join_set.spawn(async move {
            loop {
                tokio::select! {
                    _ = sigterm.recv() => break,
                    _ = sigint.recv() => break,
                    event = receiver.recv() => {
                        match event {
                            Some(event) => handler(event).await,
                            None => break,
                        }
                    }
                }
            }
        });

        self.event_receiver = None;
        self
    }

    pub async fn read_directory(mut self) -> tokio::io::Result<()> {
        let path = Path::new(&self.path);
        match &path.try_exists() {
            Ok(true) => {},
            Ok(false) => panic!("The path '{}' does not exist!", &self.path),
            Err(err) => panic!("{}", err.to_string()),
        }

        let mut join_set = JoinSet::new();
        let filter_map = Arc::new(self.filter.clone());

        let files = Self::get_files_in_directory(&path).await?;
        for (filter, callback) in filter_map.iter() {
            let regex = Regex::new(filter).unwrap();

            for file_path in &files {
                let file_name = file_path.file_name().unwrap().to_str().unwrap().to_string();
                
                if regex.is_match(&file_name) {
                    let callback = Arc::clone(&callback);
                    let file_path = Arc::new(file_path.to_path_buf());
                    let uuid = Uuid::new_v4().to_string();
                
                    self.event_broadcast.send(FileReceiverEventSignal::OnFileReceived(uuid.clone(), file_path.to_path_buf())).await.unwrap();
                    join_set.spawn(async move {
                        callback(uuid, file_path.to_path_buf()).await;
                    });
                }
            }
        }

        while let Some(_) = join_set.join_next().await {}
        while let Some(_) = self.event_join_set.join_next().await {}

        Ok(())
    }

    pub async fn poll_directory(mut self, interval: u64) {
        let path = Path::new(&self.path);
        match &path.try_exists() {
            Ok(true) => {},
            Ok(false) => panic!("The path '{}' does not exist!", &self.path),
            Err(err) => panic!("{}", err.to_string()),
        }

        let mut join_set = JoinSet::new();
        let mut interval = tokio::time::interval(Duration::from_millis(interval));
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver.");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver.");
        let ignore_list = Arc::new(Mutex::new(Vec::<String>::new()));
        let filter_map = Arc::new(self.filter.clone());
        
        loop {
            tokio::select! {
                _ = sigterm.recv() => break,
                _ = sigint.recv() => break,
                _ = async {
                    match Self::get_files_in_directory(&path).await {
                        Ok(files) => {
                            for (filter, callback) in filter_map.iter() {
                                let regex = Regex::new(filter).unwrap();
                
                                for file_path in &files {
                                    let file_name = file_path.file_name().unwrap().to_str().unwrap().to_string();
                                    
                                    if regex.is_match(&file_name) {
                                        let mut unlocked_list = ignore_list.lock().await;
                                        if unlocked_list.contains(&file_name) {
                                            continue;
                                        }
                                        unlocked_list.push(file_name.to_string());
                                        drop(unlocked_list);
                                        
                                        let callback = Arc::clone(&callback);
                                        let ignore_list = Arc::clone(&ignore_list);
                                        let file_path = Arc::new(file_path.to_path_buf());
                                        let uuid = Uuid::new_v4().to_string();
                                    
                                        self.event_broadcast.send(FileReceiverEventSignal::OnFileReceived(uuid.clone(), file_path.to_path_buf())).await.unwrap();
                                        join_set.spawn(async move {
                                            callback(uuid, file_path.to_path_buf()).await;
                                            let mut unlocked_list = ignore_list.lock().await;
                                            if let Some(pos) = unlocked_list.iter().position(|item| item == &file_name) {
                                                unlocked_list.remove(pos);
                                            }
                                        });
                                    }
                                }
                            }
                        },
                        Err(_) => {},
                    }

                    interval.tick().await;
                } => {}
            }
        }

        while let Some(_) = join_set.join_next().await {}
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