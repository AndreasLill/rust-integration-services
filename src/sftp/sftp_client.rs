use std::{marker::PhantomData, path::{Path, PathBuf}, sync::Arc};

use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::sftp::{sftp, sftp_client_config::SftpClientConfig};

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
        let session = sftp::connect(self.config.clone()).await?;
        session.remove_file(path.as_ref().to_string_lossy()).await?;

        Ok(())
    }
}

impl SftpClient<GetFile> {
    pub async fn as_bytes(&self) -> anyhow::Result<Bytes> {
        let session = sftp::connect(self.config.clone()).await?;
        let path = self.path.as_ref().unwrap().to_string_lossy();

        let mut remote_file = session.open(path).await?;
        let mut buffer = Vec::new();
        remote_file.read_to_end(&mut buffer).await?;

        Ok(Bytes::from(buffer))
    }
}

impl SftpClient<PutFile> {
    pub async fn from_bytes(&self, bytes: impl Into<Bytes>) -> anyhow::Result<()> {
        let session = sftp::connect(self.config.clone()).await?;
        let path = self.path.as_ref().unwrap().to_string_lossy();
        tracing::trace!("uploading bytes to {:?}", path);

        let mut remote_file = session.create(path).await?;
        remote_file.write_all(&bytes.into()).await?;

        tracing::trace!("upload complete");
        Ok(())
    }
}