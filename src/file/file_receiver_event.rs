use std::path::PathBuf;

#[derive(Clone)]
pub enum FileReceiverEvent {
    OnFileReceived(String, PathBuf),
    OnFileProcessed(String, PathBuf),
    OnError(String, String),
}