use std::{collections::HashMap, path::{Path, PathBuf}};

use bytes::Bytes;
use regex::Regex;
use russh::keys::HashAlg;
use tokio::{fs::File, io::{AsyncReadExt, AsyncWriteExt}};

use crate::sftp::{sftp, sftp_auth_basic::SftpAuthBasic, sftp_auth_key::SftpAuthKey, sftp_authentication::SftpAuthentication};

pub struct SftpReceiver {
    host: String,
    remote_dir: PathBuf,
    delete_after_download: bool,
    regex: Regex,
    authentication: SftpAuthentication,
}

impl SftpReceiver {
    pub fn new<T: AsRef<str>>(host: T) -> Self {
        SftpReceiver { 
            host: host.as_ref().to_string(),
            remote_dir: PathBuf::from("/"),
            delete_after_download: false,
            regex: Regex::new(r"^.+\.[^./\\]+$").unwrap(),
            authentication: SftpAuthentication::new(),
        }
    }

    /// Basic authentication using password.
    pub fn auth_basic<T: AsRef<str>>(mut self, user: T, password: T) -> Self {
        self.authentication.basic = Some(SftpAuthBasic {
            user: user.as_ref().to_string(),
            password: password.as_ref().to_string()
        });
        self
    }

    /// Authentication using a private key.
    /// What hash algorithm to choose?
    /// Ed25519 = None
    /// ECDSA = None
    /// RSA = Some(HashAlg::Sha256) or Some(HashAlg::Sha512)
    pub fn auth_key<T: AsRef<str>>(mut self, user: T, key_path: T, hash_alg: Option<HashAlg>, passphrase: Option<String>) -> Self {
        self.authentication.key = Some(SftpAuthKey {
            user: user.as_ref().to_string(),
            key_path: PathBuf::from(key_path.as_ref()),
            hash_alg,
            passphrase
        });
        self
    }

    /// Sets the remote directory for the user on the sftp server.
    pub fn remote_dir<T: AsRef<Path>>(mut self, remote_dir: T) -> Self {
        self.remote_dir = remote_dir.as_ref().to_path_buf();
        self
    }

    /// Delete the remote file in remote after successfully downloading it.
    pub fn delete_after_download(mut self, delete: bool) -> Self {
        self.delete_after_download = delete;
        self
    }

    /// Sets the regex filter for what files will be downloaded from the sftp server.
    /// 
    /// The default regex is: ^.+\.[^./\\]+$
    pub fn regex<T: AsRef<str>>(mut self, regex: T) -> Self {
        self.regex = Regex::new(regex.as_ref()).expect("Invalid Regex");
        self
    }

    /// Download all files from the sftp server to the local file system.
    pub async fn receive_to_path<T: AsRef<Path>>(self, target_path: T) -> anyhow::Result<()> {
        let sftp = sftp::connect_and_authenticate(&self.host, &self.authentication).await?;
        let entries = sftp.read_dir(self.remote_dir.to_str().unwrap()).await?;

        for entry in entries {
            if !entry.file_type().is_file() {
                continue;
            }
            if !self.regex.is_match(&entry.file_name()) {
                continue;
            }

            let remote_file_path = self.remote_dir.join(entry.file_name());
            tracing::debug!("matched remote file at {:?}", remote_file_path);
            let mut remote_file = sftp.open(remote_file_path.to_str().unwrap()).await?;

            let local_file_path = target_path.as_ref().join(entry.file_name());
            let mut local_file = File::create(&local_file_path).await?;

            let mut buffer = vec![0u8; 1024];
            loop {
                let bytes = remote_file.read(&mut buffer).await?;
                if bytes == 0 {
                    break;
                }
                local_file.write_all(&buffer[..bytes]).await?;
            }
            tracing::debug!("remote file {:?} downloaded to {:?}", &remote_file_path, &local_file_path);

            if self.delete_after_download {
                sftp.remove_file(remote_file_path.to_str().unwrap()).await?;
                tracing::debug!("remote file deleted {:?}", &remote_file_path);
            }
        }

        Ok(())
    }

    /// Download all files from the sftp server.
    /// Returns a HashMap with file name as key and bytes as value.
    pub async fn receive_to_bytes(self) -> anyhow::Result<HashMap<String, Bytes>> {
        let sftp = sftp::connect_and_authenticate(&self.host, &self.authentication).await?;
        let mut files = HashMap::<String, Bytes>::new();
        let entries = sftp.read_dir(self.remote_dir.to_str().unwrap()).await?;

        for entry in entries {
            if !entry.file_type().is_file() {
                continue;
            }
            if !self.regex.is_match(&entry.file_name()) {
                continue;
            }

            let remote_file_path = self.remote_dir.join(entry.file_name());
            tracing::debug!("matched remote file at {:?}", remote_file_path);
            let mut remote_file = sftp.open(remote_file_path.to_str().unwrap()).await?;

            let mut buffer = Vec::new();
            remote_file.read_to_end(&mut buffer).await?;
            files.insert(entry.file_name(), Bytes::from(buffer));
            tracing::debug!("remote file {:?} downloaded", &remote_file_path);

            if self.delete_after_download {
                sftp.remove_file(remote_file_path.to_str().unwrap()).await?;
                tracing::debug!("remote file deleted {:?}", &remote_file_path);
            }
        }

        Ok(files)
    }
}