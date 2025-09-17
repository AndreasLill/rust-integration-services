use std::path::{Path, PathBuf};

use async_ssh2_lite::{AsyncFile, AsyncSession, SessionConfiguration, TokioTcpStream};
use futures_util::{AsyncReadExt};
use regex::Regex;
use tokio::{fs::{File, OpenOptions}, io::AsyncWriteExt, task::JoinSet};
use tokio::net::TcpStream;
use uuid::Uuid;

use super::sftp_auth::SftpAuth;
use crate::{common::event_handler::EventHandler, sftp::sftp_receiver_event::SftpReceiverEvent, utils::error::Error};

pub struct SftpReceiver {
    host: String,
    remote_path: PathBuf,
    delete_after: bool,
    regex: String,
    auth: SftpAuth,
    event_handler: EventHandler<SftpReceiverEvent>,
    event_join_set: JoinSet<()>,
}

impl SftpReceiver {
    pub fn new<T: AsRef<str>>(host: T, user: T) -> Self {
        SftpReceiver { 
            host: host.as_ref().to_string(),
            remote_path: PathBuf::new(),
            delete_after: false,
            regex: String::from(r"^.+\.[^./\\]+$"),
            auth: SftpAuth { user: user.as_ref().to_string(), password: None, private_key: None, private_key_passphrase: None },
            event_handler: EventHandler::new(),
            event_join_set: JoinSet::new(),
        }
    }

    pub fn on_event<T, Fut>(mut self, handler: T) -> Self
    where
        T: Fn(SftpReceiverEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.event_join_set = self.event_handler.init(handler);
        self
    }

    /// Sets the password for authentication.
    pub fn password<T: AsRef<str>>(mut self, password: T) -> Self {
        self.auth.password = Some(password.as_ref().to_string());
        self
    }

    /// Sets the private key path and passphrase for authentication.
    pub fn private_key<T: AsRef<Path>, S: AsRef<str>>(mut self, key_path: T, passphrase: Option<S>) -> Self {
        self.auth.private_key = Some(key_path.as_ref().to_path_buf());
        self.auth.private_key_passphrase = match passphrase {
            Some(passphrase) => Some(passphrase.as_ref().to_string()),
            None => None,
        };
        self
    }

    /// Sets the remote directory for the user on the sftp server.
    pub fn remote_path<T: AsRef<Path>>(mut self, remote_path: T) -> Self {
        self.remote_path = remote_path.as_ref().to_path_buf();
        self
    }

    /// Delete the remote file in sftp after successfully downloading it.
    pub fn delete_after(mut self, delete_after: bool) -> Self {
        self.delete_after = delete_after;
        self
    }

    /// Sets the regex filter for what files will be downloaded from the sftp server.
    /// 
    /// The default regex is: ^.+\.[^./\\]+$
    pub fn regex<T: AsRef<str>>(mut self, regex: T) -> Self {
        self.regex = regex.as_ref().to_string();
        self
    }

    /// Download files from the sftp server to the target local path.
    /// 
    /// Filters for files can be set with regex(), the default regex is: ^.+\.[^./\\]+$
    pub async fn receive_once<T: AsRef<Path>>(mut self, target_local_path: T) -> tokio::io::Result<()> {
        let local_path = target_local_path.as_ref();
        if !local_path.try_exists()? {
            return Err(Error::tokio_io(format!("The path '{:?}' does not exist!", &local_path)));
        }

        let tcp = TokioTcpStream::connect(&self.host).await?;
        let mut session = AsyncSession::new(tcp, SessionConfiguration::default())?;
        session.handshake().await?;

        if let Some(password) = self.auth.password {
            session.userauth_password(&self.auth.user, &password).await?;
        }
        if let Some(private_key) = self.auth.private_key {
            session.userauth_pubkey_file(&self.auth.user, None, &private_key, self.auth.private_key_passphrase.as_deref()).await?;
        }

        let remote_path = Path::new(&self.remote_path);
        let sftp = session.sftp().await?;
        let entries = sftp.readdir(remote_path).await?;
        let regex = Regex::new(&self.regex).unwrap();
        let event_broadcast = self.event_handler.broadcast();

        for (entry, metadata) in entries {
            if metadata.is_dir() {
                continue;
            }

            let file_name = entry.file_name().unwrap().to_str().unwrap();
            if regex.is_match(file_name) {

                let remote_file_path = Path::new(&self.remote_path).join(file_name);
                let remote_file = sftp.open(&remote_file_path).await?;
                let local_file_path = local_path.join(file_name);
                let local_file = OpenOptions::new().create(true).write(true).open(&local_file_path).await?;

                let uuid = Uuid::new_v4().to_string();
                event_broadcast.send(SftpReceiverEvent::OnDownloadStart(uuid.clone(), local_file_path.clone())).ok();

                match Self::download_file(remote_file, local_file).await {
                    Ok(_) => {
                        event_broadcast.send(SftpReceiverEvent::OnDownloadSuccess(uuid.clone(), local_file_path.clone())).ok();
                        if self.delete_after {
                            sftp.unlink(&remote_file_path).await?;
                        }
                    },
                    Err(err) => {
                        event_broadcast.send(SftpReceiverEvent::OnError(uuid.clone(), err.to_string())).ok();
                    },
                }
            }
        }

        drop(event_broadcast);
        while let Some(_) = self.event_join_set.join_next().await {}

        Ok(())
    }

    async fn download_file(mut remote_file: AsyncFile<TcpStream>, mut local_file: File) -> tokio::io::Result<()> {
        let mut buffer = vec![0u8; 1024 * 1024];
        loop {
            let bytes = remote_file.read(&mut buffer).await?;
            if bytes == 0 {
                break;
            }
            local_file.write_all(&buffer[..bytes]).await?;
        }

        local_file.flush().await?;
        remote_file.close().await?;

        Ok(())
    }
}