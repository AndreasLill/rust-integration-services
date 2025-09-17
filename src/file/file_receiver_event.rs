use std::path::PathBuf;

#[derive(Clone)]
pub enum FileReceiverEvent {
    FileReceived {
        uuid: String,
        path: PathBuf
    },
    FileProcessed {
        uuid: String,
        path: PathBuf
    },
    Error {
        uuid: String,
        error: String
    },
}