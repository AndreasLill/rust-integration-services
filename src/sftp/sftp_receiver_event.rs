use std::path::PathBuf;

#[derive(Clone)]
pub enum SftpReceiverEvent {
    DownloadStarted {
        uuid: String,
        path: PathBuf
    },
    DownloadSuccess {
        uuid: String,
        path: PathBuf
    },
    Error {
        uuid: String,
        error: String
    },
}