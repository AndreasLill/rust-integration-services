use std::{marker::PhantomData, path::{Path, PathBuf}};

use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

use crate::common::stream::{ByteStream, IntoByteStream};

pub struct Empty;
pub struct Write;
pub struct Copy;

pub struct FileClient<State> {
    path: Option<PathBuf>,
    _state: PhantomData<State>,
}

impl FileClient<Empty> {
    pub fn new() -> Self {
        FileClient  {
            path: None,
            _state: PhantomData
        }
    }

    pub fn write(&self, path: impl Into<PathBuf>) -> FileClient<Write> {
        FileClient {
            path: Some(path.into()),
            _state: PhantomData
        }
    }

    pub async fn read(&self, path: impl AsRef<Path>) -> anyhow::Result<ByteStream> {
        let file = tokio::fs::File::open(path.as_ref()).await?;
        let reader = ReaderStream::new(file);

        Ok(ByteStream::new(reader))
    }

    pub fn copy(&self, path: impl Into<PathBuf>) -> FileClient<Copy> {
        FileClient {
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
    pub async fn body(&self, stream: impl IntoByteStream) -> anyhow::Result<()> {
        let mut file = tokio::fs::File::create(&self.path.as_ref().unwrap()).await?;
        let mut stream = stream.into_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }

        file.flush().await?;
        Ok(())
    }
}

impl FileClient<Copy> {
    pub async fn to(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        tokio::fs::copy(&self.path.as_ref().unwrap(), path.as_ref()).await?;

        Ok(())
    }
}