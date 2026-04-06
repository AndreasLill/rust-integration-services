use std::{marker::PhantomData, path::{Path, PathBuf}, sync::Arc};

use bytes::Bytes;
use russh::keys::{HashAlg, PrivateKeyWithHashAlg};
use russh_sftp::client::SftpSession;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::io::ReaderStream;

use crate::{common::stream::ByteStream, sftp::{sftp_client_config::SftpClientConfig, ssh_client::SshClient}};

pub struct Empty;
pub struct GetFile;
pub struct PutFile;

pub struct SftpClient<State> {
    config: Arc<SftpClientConfig>,
    path: Option<PathBuf>,
    _state: PhantomData<State>,
}

impl SftpClient<Empty> {
    pub fn new(config: SftpClientConfig) -> Self {
        SftpClient {
            config: Arc::new(config),
            path: None,
            _state: PhantomData
        }
    }

    pub fn get_file(&self, path: impl Into<PathBuf>) -> SftpClient<GetFile> {
        SftpClient {
            config: self.config.clone(),
            path: Some(path.into()),
            _state: PhantomData
        }
    }

    pub fn put_file(&self, path: impl Into<PathBuf>) -> SftpClient<PutFile> {
        SftpClient {
            config: self.config.clone(),
            path: Some(path.into()),
            _state: PhantomData
        }
    }

    pub async fn delete_file(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let session = self.connect().await?;
        session.remove_file(path.as_ref().to_string_lossy()).await?;

        Ok(())
    }
}

impl SftpClient<GetFile> {
    pub async fn as_bytes(&mut self) -> anyhow::Result<Bytes> {
        let session = self.connect().await?;
        let path = self.path.as_ref().unwrap().to_string_lossy();

        let mut remote_file = session.open(path).await?;
        let mut buffer = Vec::new();
        remote_file.read_to_end(&mut buffer).await?;
        remote_file.shutdown().await?;

        Ok(Bytes::from(buffer))
    }

    pub async fn as_stream(&mut self) -> anyhow::Result<ByteStream> {
        let session = self.connect().await?;
        let path = self.path.as_ref().unwrap().to_string_lossy();

        let remote_file = session.open(path).await?;
        let reader = ReaderStream::new(remote_file);

        Ok(ByteStream::new(reader))
    }
}

impl SftpClient<PutFile> {
    pub async fn from_bytes(&mut self, bytes: impl Into<Bytes>) -> anyhow::Result<()> {
        let session = self.connect().await?;
        let path = self.path.as_ref().unwrap().to_string_lossy();
        tracing::trace!("uploading bytes to {:?}", path);

        let mut remote_file = session.create(path).await?;
        remote_file.write_all(&bytes.into()).await?;
        remote_file.shutdown().await?;

        tracing::trace!("upload complete");
        Ok(())
    }

    pub async fn from_stream(&mut self, mut stream: ByteStream) -> anyhow::Result<()> {
        let session = self.connect().await?;
        let path = self.path.as_ref().unwrap().to_string_lossy();
        tracing::trace!("uploading bytes to {:?}", path);

        let mut remote_file = session.create(path).await?;
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?; 
            remote_file.write_all(&chunk).await?;
        }
        remote_file.shutdown().await?;

        tracing::trace!("upload complete");
        Ok(())
    }
}

impl<State> SftpClient<State> {
    async fn connect(&self) -> anyhow::Result<SftpSession> {
        let config = self.config.clone();
        tracing::trace!("connecting to {}", config.endpoint);
        let mut session = russh::client::connect(Arc::new(russh::client::Config::default()), &config.endpoint, SshClient {}).await?;
        
        if let Some(auth) = &config.auth_basic {
            session.authenticate_password(&auth.user, &auth.password).await?;
            tracing::trace!("authenticated using basic");
        }

        if let Some(auth) = &config.auth_private_key {
            let key = russh::keys::load_secret_key(&auth.path, auth.passphrase.as_deref())?;
            let hash_alg = match &key.algorithm() {
                russh::keys::Algorithm::Rsa { .. } => Some(HashAlg::Sha256),
                _ => None,
            };

            let key_with_alg = PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg);
            session.authenticate_publickey(&auth.user, key_with_alg).await?;
            tracing::trace!("authenticated using key");
        }

        let channel = session.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp = SftpSession::new(channel.into_stream()).await?;

        tracing::trace!("connected to {}", config.endpoint);
        Ok(sftp)
    }
}
