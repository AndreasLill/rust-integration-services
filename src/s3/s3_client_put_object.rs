use std::{path::{Path, PathBuf}, sync::Arc};

use aws_sdk_s3::{Client, primitives::ByteStream};
use bytes::Bytes;

pub struct S3ClientPutObject {
    client: Arc<Client>,
    bucket: String,
    key: String,
    body: Option<Bytes>,
    file: Option<PathBuf>,
}

impl S3ClientPutObject {
    pub fn new(client: Arc<Client>, bucket: impl AsRef<str>, key: impl AsRef<str>) -> Self {
        Self {
            client,
            bucket: bucket.as_ref().to_owned(),
            key: key.as_ref().to_owned(),
            body: None,
            file: None,
        }
    }

    pub async fn send(self) -> anyhow::Result<()> {
        if self.body.is_some() && self.file.is_some() {
            return Err(anyhow::anyhow!("Cannot use both .body and .file on the same object."));
        }

        if let Some(bytes) = &self.body {
            return self.send_bytes(bytes.clone()).await;
        }

        if let Some(path) = &self.file {
            return self.send_file(path).await;
        }

        self.send_bytes(Bytes::new()).await
    }

    async fn send_bytes(&self, bytes: Bytes) -> anyhow::Result<()> {
        let _result = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&self.key)
            .body(bytes.into())
            .send()
            .await?;

        Ok(())
    }

    async fn send_file(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let stream = ByteStream::from_path(path).await?;
        let _result = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&self.key)
            .body(stream)
            .send()
            .await?;

        Ok(())
    }

    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn file(mut self, path: impl AsRef<Path>) -> Self {
        self.file = Some(path.as_ref().to_owned());
        self
    }
}