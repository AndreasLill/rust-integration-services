use std::path::PathBuf;

pub struct SftpAuthPrivateKey {
    pub user: String,
    pub path: PathBuf,
    pub passphrase: Option<String>,
}