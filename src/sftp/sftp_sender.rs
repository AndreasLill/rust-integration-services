use std::path::{Path, PathBuf};

use russh::keys::HashAlg;
use tokio::{fs::File, io::{AsyncReadExt, AsyncWriteExt}};

use crate::sftp::{sftp, sftp_authentication::SftpAuthentication, sftp_auth_basic::SftpAuthBasic, sftp_auth_key::SftpAuthKey};

pub struct SftpSender {
    host: String,
    remote_dir: PathBuf,
    authentication: SftpAuthentication,
}

impl SftpSender {
    pub fn new<T: AsRef<str>>(host: T) -> Self {
        SftpSender { 
            host: host.as_ref().to_string(),
            remote_dir: PathBuf::from("/"),
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

    /// Send a file from the local file system to the sftp server.
    pub async fn send_file<T: AsRef<Path>>(self, file_path: T, file_name: Option<String>) -> anyhow::Result<()> {
        let sftp = sftp::connect_and_authenticate(&self.host, &self.authentication).await?;
        let file_name = match file_name {
            Some(name) => name,
            None => file_path.as_ref().file_name().unwrap().to_string_lossy().to_string(),
        };
        let remote_file_path = self.remote_dir.join(file_name);
        let mut remote_file = sftp.create(remote_file_path.to_str().unwrap()).await?;
        let mut source_file = File::open(file_path.as_ref()).await?;

        tracing::debug!("uploading {:?} to {:?}", file_path.as_ref(), &remote_file_path);
        let mut buffer = vec![0u8; 1024];
        loop {
            let bytes = source_file.read(&mut buffer).await?;
            if bytes == 0 {
                break;
            }
            remote_file.write_all(&buffer[..bytes]).await?;
        }

        tracing::debug!("upload complete");
        Ok(())
    }

    // Send bytes as a file to the sftp server.
    pub async fn send_bytes<S: AsRef<str>>(self, bytes: &[u8], file_name: S) -> anyhow::Result<()> {
        let sftp = sftp::connect_and_authenticate(&self.host, &self.authentication).await?;
        let remote_file_path = self.remote_dir.join(file_name.as_ref());
        let mut remote_file = sftp.create(remote_file_path.to_str().unwrap()).await?;
        tracing::debug!("uploading bytes to {:?}", &remote_file_path);
        remote_file.write_all(&bytes).await?;

        tracing::debug!("upload complete");
        Ok(())
    }
}