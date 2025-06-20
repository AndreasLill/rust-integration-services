use std::path::{Path, PathBuf};

use async_ssh2_lite::{AsyncSession, SessionConfiguration, TokioTcpStream};
use futures_util::{AsyncReadExt};
use regex::Regex;
use tokio::{fs::OpenOptions, io::AsyncWriteExt, signal::unix::{signal, SignalKind}, sync::mpsc, task::JoinSet};
use uuid::Uuid;


#[derive(Clone)]
pub enum SftpReceiverEventSignal {
    OnDownloadStart(String, PathBuf),
    OnDownloadSuccess(String, PathBuf),
}

pub struct SftpReceiver {
    host: String,
    remote_dir: String,
    delete_after: bool,
    regex: String,
    auth_username: String,
    auth_password: String,
    auth_private_key: String,
    auth_private_key_passphrase: String,
    event_broadcast: mpsc::Sender<SftpReceiverEventSignal>,
    event_receiver: Option<mpsc::Receiver<SftpReceiverEventSignal>>,
    event_join_set: JoinSet<()>,
}

impl SftpReceiver {
    pub fn new(host: &str, user: &str) -> Self {
        let (event_broadcast, event_receiver) = mpsc::channel(128);
        SftpReceiver { 
            host: host.to_string(),
            remote_dir: String::new(),
            delete_after: false,
            regex: String::from(r"^.+\.[^./\\]+$"),
            auth_username: user.to_string(),
            auth_password: String::new(),
            auth_private_key: String::new(),
            auth_private_key_passphrase: String::new(),
            event_broadcast,
            event_receiver: Some(event_receiver),
            event_join_set: JoinSet::new(),
        }
    }

    pub fn on_event<T, Fut>(mut self, handler: T) -> Self
    where
        T: Fn(SftpReceiverEventSignal) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut receiver = self.event_receiver.unwrap();
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver.");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver.");
        
        self.event_join_set.spawn(async move {
            loop {
                tokio::select! {
                    _ = sigterm.recv() => break,
                    _ = sigint.recv() => break,
                    event = receiver.recv() => {
                        match event {
                            Some(event) => handler(event).await,
                            None => break,
                        }
                    }
                }
            }
        });

        self.event_receiver = None;
        self
    }

    /// Sets the password for authentication.
    pub fn auth_password(mut self, password: &str) -> Self {
        self.auth_password = password.to_string();
        self
    }

    /// Sets the private key path and passphrase (empty string if passphrase is unused).
    pub fn auth_private_key(mut self, key_path: &str, passphrase: &str) -> Self {
        self.auth_private_key = key_path.to_string();
        self.auth_private_key_passphrase = passphrase.to_string();
        self
    }

    /// Sets the remote directory for the user on the sftp server.
    pub fn remote_dir(mut self, remote_dir: &str) -> Self {
        self.remote_dir = remote_dir.to_string();
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
    pub fn regex(mut self, regex: &str) -> Self {
        self.regex = regex.to_string();
        self
    }

    /// Download files from the sftp server to the target local path.
    /// 
    /// Filters for files can be set with regex(), the default regex is: ^.+\.[^./\\]+$
    pub async fn receive_files(mut self, target_local_path: &str) -> tokio::io::Result<()> {
        let local_path = Path::new(target_local_path);
        if !local_path.try_exists()? {
            return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("The path '{:?}' does not exist!", local_path)));
        }

        let tcp = TokioTcpStream::connect(&self.host).await?;
        let mut session = AsyncSession::new(tcp, SessionConfiguration::default())?;
        session.handshake().await?;
        if !self.auth_password.is_empty() {
            session.userauth_password(&self.auth_username, &self.auth_password).await?;
        }
        if !self.auth_private_key.is_empty() {
            let private_key_path = Path::new(&self.auth_private_key);
            session.userauth_pubkey_file(&self.auth_username, None, private_key_path, Some(&self.auth_private_key_passphrase)).await?;
        }

        let remote_path = Path::new(&self.remote_dir);
        let sftp = session.sftp().await?;
        let entries = sftp.readdir(remote_path).await?;
        let regex = Regex::new(&self.regex).unwrap();

        for (entry, metadata) in entries {
            if metadata.is_dir() {
                continue;
            }

            let file_name = entry.file_name().unwrap().to_str().unwrap();
            if regex.is_match(file_name) {

                let remote_file_path = Path::new(&self.remote_dir).join(file_name);
                let mut remote_file = sftp.open(&remote_file_path).await?;
                let local_file_path = local_path.join(file_name);
                let mut local_file = OpenOptions::new().create(true).write(true).open(&local_file_path).await?;

                let uuid = Uuid::new_v4().to_string();
                self.event_broadcast.send(SftpReceiverEventSignal::OnDownloadStart(uuid.clone(), local_file_path.clone())).await.unwrap();

                let mut buffer = vec![0u8; 1024 * 1024];
                loop {
                    let bytes = remote_file.read(&mut buffer).await?;
                    if bytes == 0 {
                        break;
                    }
                    local_file.write_all(&buffer[..bytes]).await?;
                }

                self.event_broadcast.send(SftpReceiverEventSignal::OnDownloadSuccess(uuid.clone(), local_file_path.clone())).await.unwrap();
                local_file.flush().await?;
                remote_file.close().await?;

                if self.delete_after {
                    sftp.unlink(&remote_file_path).await?;
                }
            }
        }

        while let Some(_) = self.event_join_set.join_next().await {}
        
        Ok(())
    }
}