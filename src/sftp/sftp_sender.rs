use std::{path::{Path, PathBuf}, sync::Arc};

use russh_sftp::client::SftpSession;
use tokio::{fs::File, io::{AsyncReadExt, AsyncWriteExt}};

use crate::sftp::{sftp_auth_basic::SftpAuthBasic, ssh_client::SshClient};

pub struct SftpSender {
    host: String,
    remote_dir: PathBuf,
    auth_basic: Option<SftpAuthBasic>,
}

impl SftpSender {
    pub fn new<T: AsRef<str>>(host: T) -> Self {
        SftpSender { 
            host: host.as_ref().to_string(),
            remote_dir: PathBuf::from("/"),
            auth_basic: None,
        }
    }

    /// Basic authentication using password.
    pub fn auth_basic<T: AsRef<str>>(mut self, user: T, password: T) -> Self {
        self.auth_basic = Some(SftpAuthBasic {
            user: user.as_ref().to_string(),
            password: password.as_ref().to_string()
        });
        self
    }

    /// Sets the remote directory for the user on the sftp server.
    pub fn remote_dir<T: AsRef<Path>>(mut self, remote_dir: T) -> Self {
        self.remote_dir = remote_dir.as_ref().to_path_buf();
        self
    }

    pub async fn send_file<T: AsRef<Path>>(self, file_path: T, file_name: Option<String>) -> anyhow::Result<()> {
        let sftp = self.connect_and_authenticate().await?;
        tracing::debug!("connected to {}", &self.host);

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

    pub async fn send_bytes<S: AsRef<str>>(self, bytes: &[u8], file_name: S) -> anyhow::Result<()> {
        let sftp = self.connect_and_authenticate().await?;
        tracing::debug!("connected to {}", &self.host);

        let remote_file_path = self.remote_dir.join(file_name.as_ref());
        let mut remote_file = sftp.create(remote_file_path.to_str().unwrap()).await?;
        tracing::debug!("uploading bytes to {:?}", &remote_file_path);
        remote_file.write_all(&bytes).await?;

        tracing::debug!("upload complete");
        Ok(())
    }

    async fn connect_and_authenticate(&self) -> anyhow::Result<SftpSession> {
        let config = russh::client::Config::default();
        let ssh = SshClient {};

        tracing::debug!("connecting to {}", &self.host);
        let mut session = russh::client::connect(Arc::new(config), &self.host, ssh).await?;
        
        if let Some(auth) = &self.auth_basic {
            session.authenticate_password(&auth.user, &auth.password).await?;
        }

        let channel = session.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp = SftpSession::new(channel.into_stream()).await?;

        Ok(sftp)
    }
}