use std::path::Path;

use async_ssh2_lite::{AsyncSession, SessionConfiguration, TokioTcpStream};
use futures_util::AsyncWriteExt;
use tokio::{fs::OpenOptions, io::AsyncReadExt};

pub struct SftpSender {
    host: String,
    remote_dir: String,
    target_name: String,
    auth_username: String,
    auth_password: String,
    auth_private_key: String,
    auth_private_key_passphrase: String,
}

impl SftpSender {
    pub fn new(host: &str, user: &str) -> Self {
        SftpSender { 
            host: host.to_string(),
            remote_dir: String::new(),
            target_name: String::new(),
            auth_username: user.to_string(),
            auth_password: String::new(),
            auth_private_key: String::new(),
            auth_private_key_passphrase: String::new(),
        }
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

    /// Sets a new file name for the sent file.
    pub fn target_name(mut self, target_name: &str) -> Self {
        self.target_name = target_name.to_string();
        self
    }

    /// Send a file to the SFTP server.
    pub async fn send_file(self, source_path: &str) -> tokio::io::Result<()> {
        let source_path = Path::new(source_path);
        match &source_path.try_exists() {
            Ok(true) => {},
            Ok(false) => return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("The file '{}' does not exist!", &source_path.to_str().unwrap()))),
            Err(err) => return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, format!("{}", err.to_string()))),
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

        let mut file = OpenOptions::new().read(true).open(source_path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;
        
        let mut target_name = source_path.file_name().unwrap().to_str().unwrap();
        if !self.target_name.is_empty() {
            target_name = &self.target_name;
        }
        let remote_path = Path::new(&self.remote_dir).join(target_name);
        
        let sftp = session.sftp().await?;
        let mut remote = sftp.create(&remote_path).await?;
        remote.write_all(&buffer).await?;
        remote.close().await?;

        Ok(())
    }
}