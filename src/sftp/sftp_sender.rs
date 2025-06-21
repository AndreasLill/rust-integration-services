use std::path::{Path, PathBuf};

use async_ssh2_lite::{AsyncSession, SessionConfiguration, TokioTcpStream};
use futures_util::AsyncWriteExt;
use tokio::{fs::OpenOptions, io::AsyncReadExt};

pub struct SftpSender {
    host: String,
    remote_path: PathBuf,
    file_name: String,
    auth_username: String,
    auth_password: String,
    auth_private_key: String,
    auth_private_key_passphrase: String,
}

impl SftpSender {
    pub fn new<T: AsRef<str>>(host: T, user: T) -> Self {
        SftpSender { 
            host: host.as_ref().to_string(),
            remote_path: PathBuf::new(),
            file_name: String::new(),
            auth_username: user.as_ref().to_string(),
            auth_password: String::new(),
            auth_private_key: String::new(),
            auth_private_key_passphrase: String::new(),
        }
    }

    /// Sets the password for authentication.
    pub fn auth_password<T: AsRef<str>>(mut self, password: T) -> Self {
        self.auth_password = password.as_ref().to_string();
        self
    }

    /// Sets the private key path and passphrase (empty string if passphrase is unused).
    pub fn auth_private_key<T: AsRef<str>>(mut self, key_path: T, passphrase: T) -> Self {
        self.auth_private_key = key_path.as_ref().to_string();
        self.auth_private_key_passphrase = passphrase.as_ref().to_string();
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
        if !self.auth_password.is_empty() {
            session.userauth_password(&self.auth_username, &self.auth_password).await?;
        }
        if !self.auth_private_key.is_empty() {
            let private_key_path = Path::new(&self.auth_private_key);
            session.userauth_pubkey_file(&self.auth_username, None, private_key_path, Some(&self.auth_private_key_passphrase)).await?;
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
        if !self.auth_password.is_empty() {
            session.userauth_password(&self.auth_username, &self.auth_password).await?;
        }
        if !self.auth_private_key.is_empty() {
            let private_key_path = Path::new(&self.auth_private_key);
            session.userauth_pubkey_file(&self.auth_username, None, private_key_path, Some(&self.auth_private_key_passphrase)).await?;
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