use std::path::PathBuf;

pub struct SftpAuth {
    pub user: String,
    pub password: Option<String>,
    pub private_key: Option<PathBuf>,
    pub private_key_passphrase: Option<String>,
}