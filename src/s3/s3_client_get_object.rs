use std::{path::{Path, PathBuf}, sync::Arc};

use aws_sdk_s3::Client;

use tokio::{fs::File, io::AsyncWriteExt};

pub struct S3ClientGetObject {
    client: Arc<Client>,
    bucket: String,
    key: String,
    download_dir: PathBuf,
    file_name: Option<String>
}

impl S3ClientGetObject {
    pub fn new(client: Arc<Client>, bucket: impl AsRef<str>, key: impl AsRef<str>) -> Self {
        Self {
            client,
            bucket: bucket.as_ref().to_owned(),
            key: key.as_ref().to_owned(),
            download_dir: PathBuf::from("/tmp"),
            file_name: None,
        }
    }

    pub async fn send(self) -> anyhow::Result<PathBuf> {
        let result = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(&self.key)
            .send()
            .await?;

        let file_name = match &self.file_name {
            Some(name) => name,
            None => &self.key,
        };
        let file_path = self.download_dir.join(PathBuf::from(&file_name));
        let mut file = File::create(&file_path).await?;
        let mut stream = result.body.into_async_read();

        tokio::io::copy(&mut stream, &mut file).await?;
        file.flush().await?;

        Ok(file_path)
    }

    /// Set a local download directory for the downloaded object.
    /// 
    /// 
    /// Default: `/tmp`
    pub fn download_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.download_dir = dir.as_ref().to_owned();
        self
    }

    /// Override the file name for downloaded object.
    pub fn file_name(mut self, file_name: impl AsRef<str>) -> Self {
        self.file_name = Some(file_name.as_ref().to_owned());
        self
    }
}