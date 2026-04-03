use std::{marker::PhantomData, sync::Arc};

use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_sdk_s3::{Client, config::{Credentials, SharedCredentialsProvider}, types::{CompletedMultipartUpload, CompletedPart}};
use bytes::{Bytes, BytesMut};
use futures::StreamExt;
use tokio_util::io::ReaderStream;

use crate::{common::stream::ByteStream, s3::s3_client_config::S3ClientConfig};

pub struct NoBucket;
pub struct HasBucket;

pub struct GetObject;
pub struct PutObject;

pub struct S3Client<State> {
    client: Arc<Client>,
    bucket: Option<String>,
    key: Option<String>,
    _state: PhantomData<State>,
}

impl S3Client<NoBucket> {
    pub fn new(config: S3ClientConfig) -> Self {
        Self {
            client: Arc::new(Self::build_client(config)),
            bucket: None,
            key: None,
            _state: PhantomData
        }
    }

    fn build_client(config: S3ClientConfig) -> Client {
        let creds = Credentials::new(config.access_key, config.secret_key, None, None, "static");
        let provider = SharedCredentialsProvider::new(creds);
        let region = Region::new(config.region);
        
        let sdk_config = SdkConfig::builder()
        .region(region)
        .credentials_provider(provider)
        .behavior_version(BehaviorVersion::latest())
        .endpoint_url(config.endpoint.as_str());

        Client::new(&sdk_config.build())
    }

    pub fn bucket(&self, bucket: impl Into<String>) -> S3Client<HasBucket> {
        S3Client {
            client: self.client.clone(),
            bucket: Some(bucket.into()),
            key: None,
            _state: PhantomData
        }
    }
}

impl S3Client<HasBucket> {
    pub fn get_object(&self, key: impl Into<String>) -> S3Client<GetObject> {
        S3Client {
            client: self.client.clone(),
            bucket: self.bucket.clone(),
            key: Some(key.into()),
            _state: PhantomData
        }
    }

    pub fn put_object(&self, key: impl Into<String>) -> S3Client<PutObject> {
        S3Client {
            client: self.client.clone(),
            bucket: self.bucket.clone(),
            key: Some(key.into()),
            _state: PhantomData
        }
    }

    pub async fn delete_object(&self, key: impl AsRef<str>) -> anyhow::Result<()> {
        let _result = self.client
        .delete_object()
        .bucket(self.bucket.as_ref().unwrap())
        .key(key.as_ref())
        .send()
        .await?;

        Ok(())
    }
}

impl S3Client<GetObject> {
    pub async fn as_bytes(&self) -> anyhow::Result<Bytes> {
        let result = self.client
            .get_object()
            .bucket(self.bucket.as_ref().unwrap())
            .key(self.key.as_ref().unwrap())
            .send()
            .await?;

        Ok(result.body.collect().await?.into_bytes())
    }

    pub async fn as_stream(&self) -> anyhow::Result<ByteStream> {
        let result = self.client
            .get_object()
            .bucket(self.bucket.as_ref().unwrap())
            .key(self.key.as_ref().unwrap())
            .send()
            .await?;

        let stream = ReaderStream::new(result.body.into_async_read());
        Ok(ByteStream::new(stream))
    }
}

impl S3Client<PutObject> {
    pub async fn from_bytes(&self, bytes: impl Into<Bytes>) -> anyhow::Result<()> {
        let bytes = bytes.into();
        let _result = self.client
            .put_object()
            .bucket(self.bucket.as_ref().unwrap())
            .key(self.key.as_ref().unwrap())
            .body(bytes.into())
            .send()
            .await?;

        Ok(())
    }

    pub async fn from_stream(&self, stream: ByteStream) -> anyhow::Result<()> {
        let bucket = self.bucket.as_ref().unwrap();
        let key = self.key.as_ref().unwrap();

        let create_res = self.client
            .create_multipart_upload()
            .bucket(bucket)
            .key(key)
            .send()
            .await?;
        
        let upload_id = create_res.upload_id().ok_or_else(|| anyhow::anyhow!("No upload ID"))?;
        let upload_result = self.multipart_upload(upload_id, stream).await;

        if let Err(err) = upload_result {
            let _result = self.client
            .abort_multipart_upload()
            .bucket(bucket)
            .key(key)
            .upload_id(upload_id)
            .send()
            .await;

            return Err(err);
        }

        Ok(())
    }

    async fn multipart_upload(&self, upload_id: &str, stream: ByteStream) -> anyhow::Result<()> {
        let bucket = self.bucket.as_ref().unwrap();
        let key = self.key.as_ref().unwrap();
        let min_part_size: usize = 5 * 1024 * 1024;
        let mut completed_parts = Vec::new();
        let mut part_number = 1;
        let mut buffer = BytesMut::with_capacity(min_part_size);
        let mut raw_stream = stream.as_stream();

        while let Some(chunk) = raw_stream.next().await {
            let chunk = chunk?;
            buffer.extend_from_slice(&chunk);

            if buffer.len() >= min_part_size {
                let part = self.upload_part(upload_id, part_number, buffer.split_off(0).into()).await?;
                completed_parts.push(part);
                part_number += 1;
            }
        }

        if !buffer.is_empty() {
            let part = self.upload_part(upload_id, part_number, buffer.into()).await?;
            completed_parts.push(part);
        }

        let completed_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        self.client
            .complete_multipart_upload()
            .bucket(bucket)
            .key(key)
            .upload_id(upload_id)
            .multipart_upload(completed_upload)
            .send()
            .await?;

        Ok(())
    }

    async fn upload_part(&self, upload_id: &str, part_number: i32, bytes: bytes::Bytes) -> anyhow::Result<CompletedPart> {
        let upload_part_res = self.client
            .upload_part()
            .bucket(self.bucket.as_ref().unwrap())
            .key(self.key.as_ref().unwrap())
            .upload_id(upload_id)
            .part_number(part_number)
            .body(bytes.into())
            .send()
            .await?;

        Ok(CompletedPart::builder().e_tag(upload_part_res.e_tag().unwrap_or_default()).part_number(part_number).build())
    }
}