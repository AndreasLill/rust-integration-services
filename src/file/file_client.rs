use std::{marker::PhantomData, path::{Path, PathBuf}};

use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::io::ReaderStream;

use crate::common::stream::ByteStream;

pub struct Empty;
pub struct Write;
pub struct Read;
pub struct Copy;
pub struct Move;

pub struct FileClient<State> {
    overwrite: bool,
    path: Option<PathBuf>,
    _state: PhantomData<State>,
}

impl FileClient<Empty> {
    pub fn new() -> Self {
        FileClient  {
            overwrite: false,
            path: None,
            _state: PhantomData
        }
    }

    pub fn write_to(&self, path: impl Into<PathBuf>) -> FileClient<Write> {
        FileClient {
            overwrite: self.overwrite,
            path: Some(path.into()),
            _state: PhantomData
        }
    }

    pub fn read_from(&self, path: impl Into<PathBuf>) -> FileClient<Read> {
        FileClient {
            overwrite: self.overwrite,
            path: Some(path.into()),
            _state: PhantomData
        }
    }

    pub fn copy_from(&self, path: impl Into<PathBuf>) -> FileClient<Copy> {
        FileClient {
            overwrite: self.overwrite,
            path: Some(path.into()),
            _state: PhantomData
        }
    }

    pub fn move_from(&self, path: impl Into<PathBuf>) -> FileClient<Move> {
        FileClient {
            overwrite: self.overwrite,
            path: Some(path.into()),
            _state: PhantomData
        }
    }

    pub async fn delete(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        tokio::fs::remove_file(path.as_ref()).await?;

        Ok(())
    }
}

impl FileClient<Write> {
    pub async fn from_bytes(&self, bytes: impl Into<Bytes>) -> anyhow::Result<()> {
        let mut file = tokio::fs::File::create(&self.path.as_ref().unwrap()).await?;
        file.write_all(&bytes.into()).await?;
        file.flush().await?;

        Ok(())
    }

    pub async fn from_stream(&self, mut stream: ByteStream) -> anyhow::Result<()> {
        let mut file = tokio::fs::File::create(&self.path.as_ref().unwrap()).await?;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }

        file.flush().await?;
        Ok(())
    }
}

impl FileClient<Read> {
    pub async fn as_bytes(&self) -> anyhow::Result<Bytes> {
        let mut file = tokio::fs::File::open(&self.path.as_ref().unwrap()).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        Ok(Bytes::from(buffer))
    }

    pub async fn as_stream(&self) -> anyhow::Result<ByteStream> {
        let file = tokio::fs::File::open(&self.path.as_ref().unwrap()).await?;
        let reader = ReaderStream::new(file);

        Ok(ByteStream::new(reader))
    }
}

impl FileClient<Copy> {
    pub async fn copy_to(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        tokio::fs::copy(&self.path.as_ref().unwrap(), path.as_ref()).await?;

        Ok(())
    }
}

impl FileClient<Move> {
    pub async fn move_to(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        tokio::fs::copy(&self.path.as_ref().unwrap(), path.as_ref()).await?;
        tokio::fs::remove_file(&self.path.as_ref().unwrap()).await?;

        Ok(())
    }
}