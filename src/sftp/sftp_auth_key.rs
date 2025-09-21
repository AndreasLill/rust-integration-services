use std::path::PathBuf;

pub struct SftpAuthKey {
    pub user: String,
    pub key_path: PathBuf,
    pub passphrase: Option<String>,
}