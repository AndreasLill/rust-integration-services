use std::path::PathBuf;

#[derive(Clone)]
pub enum SftpReceiverEvent {
    OnDownloadStart(String, PathBuf),
    OnDownloadSuccess(String, PathBuf),
    OnError(String, String),
}