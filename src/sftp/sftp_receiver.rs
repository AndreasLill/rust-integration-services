use std::{collections::HashMap, path::{Path, PathBuf}, sync::Arc};

use bytes::Bytes;
use regex::Regex;
use russh_sftp::client::SftpSession;
use tokio::{fs::File, io::{AsyncReadExt, AsyncWriteExt}};

use crate::sftp::{sftp_auth_basic::SftpAuthBasic, ssh_client::SshClient};

pub struct SftpReceiver {
    host: String,
    remote_dir: PathBuf,
    delete_after_download: bool,
    regex: Regex,
    auth_basic: Option<SftpAuthBasic>,
}

impl SftpReceiver {
    pub fn new<T: AsRef<str>>(host: T) -> Self {
        SftpReceiver { 
            host: host.as_ref().to_string(),
            remote_dir: PathBuf::from("/"),
            delete_after_download: false,
            regex: Regex::new(r"^.+\.[^./\\]+$").unwrap(),
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

    pub async fn receive_to_path<T: AsRef<Path>>(self, target_path: T) -> anyhow::Result<()> {
        let sftp = self.connect_and_authenticate().await?;
        tracing::debug!("connected to {}", &self.host);

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

        tracing::debug!("complete");
        Ok(())
    }

    pub async fn receive_to_bytes(self) -> anyhow::Result<HashMap<String, Bytes>> {
        let sftp = self.connect_and_authenticate().await?;
        tracing::debug!("connected to {}", &self.host);

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

        tracing::debug!("complete");
        Ok(files)
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