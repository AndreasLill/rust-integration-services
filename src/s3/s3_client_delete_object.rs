use std::sync::Arc;

use aws_sdk_s3::Client;

pub struct S3ClientDeleteObject {
    client: Arc<Client>,
    bucket: String,
    key: String,
}

impl S3ClientDeleteObject {
    pub fn new(client: Arc<Client>, bucket: impl AsRef<str>, key: impl AsRef<str>) -> Self {
        Self {
            client,
            bucket: bucket.as_ref().to_owned(),
            key: key.as_ref().to_owned(),
        }
    }

    pub async fn send(self) -> anyhow::Result<()> {
        let _result = self.client
        .delete_object()
        .bucket(&self.bucket)
        .key(&self.key)
        .send()
        .await?;

        Ok(())
    }
}