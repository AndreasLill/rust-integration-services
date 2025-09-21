use std::path::PathBuf;

use russh::keys::HashAlg;

pub struct SftpAuthKey {
    pub user: String,
    pub key_path: PathBuf,
    pub hash_alg: Option<HashAlg>,
    pub passphrase: Option<String>,
}