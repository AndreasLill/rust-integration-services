use crate::sftp::{sftp_auth_basic::SftpAuthBasic, sftp_auth_key::SftpAuthKey};

pub struct SftpAuthentication {
    pub basic: Option<SftpAuthBasic>,
    pub key: Option<SftpAuthKey>,
}

impl SftpAuthentication {
    pub fn new() -> Self {
        SftpAuthentication {
            basic: None,
            key: None,
        }
    }
}