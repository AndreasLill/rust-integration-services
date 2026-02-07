use std::{path::{Path, PathBuf}, sync::Arc};

use aws_sdk_s3::{Client, primitives::ByteStream};
use bytes::Bytes;
use regex::Regex;
use tokio::{fs::File, io::AsyncWriteExt};

pub struct S3ClientBucket {
    client: Arc<Client>,
    bucket: String,
}

impl S3ClientBucket {
    pub fn new(client: Arc<Client>, bucket: String) -> Self {
        S3ClientBucket {
            client,
            bucket
        }
    }

    pub async fn put_object_file(&mut self, key: impl AsRef<str>, file_path: impl AsRef<Path>) -> anyhow::Result<()> {
        let stream = ByteStream::from_path(file_path).await?;
        let _result = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key.as_ref())
            .body(stream)
            .send()
            .await?;

        Ok(())
    }

    pub async fn put_object_bytes(&mut self, key: impl AsRef<str>, bytes: impl Into<Bytes>) -> anyhow::Result<()> {
        let bytes = bytes.into();
        let _result = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key.as_ref())
            .body(bytes.into())
            .send()
            .await?;

        Ok(())
    }

    pub async fn get_object(&mut self, key: impl AsRef<str>, target_path: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
        let result = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key.as_ref())
            .send()
            .await?;

        let file_path = target_path.as_ref().join(PathBuf::from(key.as_ref()));
        let mut file = File::create(&file_path).await?;
        let mut stream = result.body.into_async_read();

        tokio::io::copy(&mut stream, &mut file).await?;
        file.flush().await?;

        Ok(file_path)
    }

    pub async fn get_object_as_bytes(&mut self, key: impl AsRef<str>) -> anyhow::Result<Bytes> {
        let result = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key.as_ref())
            .send()
            .await?;

        let body = result.body.collect().await?;
        let bytes = body.into_bytes();

        Ok(bytes)
    }

    pub async fn get_objects(&mut self, target_path: impl AsRef<Path>) -> anyhow::Result<Vec<PathBuf>> {
        let mut objects = Vec::new();

        let mut paginator = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(contents) = page.contents {
                for obj in contents {
                    if let Some(key) = obj.key {
                        let result = self.client.get_object()
                            .bucket(&self.bucket)
                            .key(&key)
                            .send()
                            .await?;

                        let file_path = target_path.as_ref().join(PathBuf::from(&key));
                        let mut file = File::create(&file_path).await?;
                        let mut stream = result.body.into_async_read();

                        tokio::io::copy(&mut stream, &mut file).await?;
                        file.flush().await?;

                        objects.push(file_path);
                    }
                }
            }
        }

        Ok(objects)
    }

    pub async fn get_objects_with_regex(&mut self, target_path: impl AsRef<Path>, regex: impl AsRef<str>) -> anyhow::Result<Vec<PathBuf>> {
        let mut objects = Vec::new();
        let regex = Regex::new(regex.as_ref())?;

        let mut paginator = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(contents) = page.contents {
                for obj in contents {
                    if let Some(key) = obj.key {
                        if !regex.is_match(&key) {
                            continue;
                        }

                        let result = self.client.get_object()
                            .bucket(&self.bucket)
                            .key(&key)
                            .send()
                            .await?;

                        let file_path = target_path.as_ref().join(PathBuf::from(&key));
                        let mut file = File::create(&file_path).await?;
                        let mut stream = result.body.into_async_read();

                        tokio::io::copy(&mut stream, &mut file).await?;
                        file.flush().await?;

                        objects.push(file_path);
                    }
                }
            }
        }

        Ok(objects)
    }

    pub async fn delete_object(&mut self, key: impl AsRef<str>) -> anyhow::Result<()> {
        let _result = self.client
        .delete_object()
        .bucket(&self.bucket)
        .key(key.as_ref())
        .send()
        .await?;

        Ok(())
    }
}