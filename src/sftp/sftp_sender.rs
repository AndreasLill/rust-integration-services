use std::path::{Path, PathBuf};

use async_ssh2_lite::{AsyncSession, SessionConfiguration, TokioTcpStream};
use futures_util::AsyncWriteExt;
use tokio::{fs::OpenOptions, io::AsyncReadExt};

use crate::sftp::sftp_auth::SftpAuth;

pub struct SftpSender {
    host: String,
    remote_path: PathBuf,
    file_name: String,
    user: String,
    auth: SftpAuth,
}

impl SftpSender {
    pub fn new<T: AsRef<str>>(host: T, user: T) -> Self {
        SftpSender { 
            host: host.as_ref().to_string(),
            remote_path: PathBuf::new(),
            file_name: String::new(),
            user: user.as_ref().to_string(),
            auth: SftpAuth { password: None, private_key: None, private_key_passphrase: None },
        }
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

    /// Sets a new file name for the sent file.
    pub fn file_name<T: AsRef<str>>(mut self, file_name: T) -> Self {
        self.file_name = file_name.as_ref().to_string();
        self
    }

    /// Send a file using streaming with a buffer to support large file sizes.
    /// The original file name will be used unless a new file name is specified.
    pub async fn send_file<T: AsRef<Path>>(self, source_path: T) -> tokio::io::Result<()> {
        let source_path = source_path.as_ref();
        if !source_path.try_exists()? {
            return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("The path '{:?}' does not exist!", source_path)));
        }

        let tcp = TokioTcpStream::connect(&self.host).await?;
        let mut session = AsyncSession::new(tcp, SessionConfiguration::default())?;
        session.handshake().await?;
        
        if let Some(password) = self.auth.password {
            session.userauth_password(&self.user, &password).await?;
        }
        if let Some(private_key) = self.auth.private_key {
            session.userauth_pubkey_file(&self.user, None, &private_key, self.auth.private_key_passphrase.as_deref()).await?;
        }
        
        let mut target_name = source_path.file_name().unwrap().to_str().unwrap();
        if !self.file_name.is_empty() {
            target_name = &self.file_name;
        }
        let remote_path = Path::new(&self.remote_path).join(target_name);
        
        let sftp = session.sftp().await?;
        let mut remote_file = sftp.create(&remote_path).await?;
        let mut source_file = OpenOptions::new().read(true).open(source_path).await?;
        let mut buffer = vec![0u8; 4 * 1024 * 1024];

        loop {
            let bytes = source_file.read(&mut buffer).await?;
            if bytes == 0 {
                break;
            }
            remote_file.write_all(&buffer[..bytes]).await?;
        }

        remote_file.flush().await?;
        remote_file.close().await?;

        Ok(())
    }

    /// Send bytes as a new file on the sftp server. A new file name is required.
    pub async fn send_bytes(self, bytes: &[u8]) -> tokio::io::Result<()> {
        if self.file_name.is_empty() {
            return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("A file name is required!")));
        }

        let tcp = TokioTcpStream::connect(&self.host).await?;
        let mut session = AsyncSession::new(tcp, SessionConfiguration::default())?;
        session.handshake().await?;
        
        if let Some(password) = self.auth.password {
            session.userauth_password(&self.user, &password).await?;
        }
        if let Some(private_key) = self.auth.private_key {
            session.userauth_pubkey_file(&self.user, None, &private_key, self.auth.private_key_passphrase.as_deref()).await?;
        }

        let remote_path = Path::new(&self.remote_path).join(self.file_name);
        let sftp = session.sftp().await?;
        let mut remote_file = sftp.create(&remote_path).await?;
        remote_file.write_all(bytes).await?;
        remote_file.close().await?;

        Ok(())
    }

    /// Send a string as a new file on the sftp server. A new file name is required.
    pub async fn send_string<T: AsRef<str>>(self, string: T) -> tokio::io::Result<()> {
        self.send_bytes(string.as_ref().as_bytes()).await
    }
}