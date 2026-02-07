use std::sync::Arc;

use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_sdk_s3::{Client, config::{Credentials, SharedCredentialsProvider}};

use crate::s3::{s3_client_bucket::S3ClientBucket, s3_client_config::S3ClientConfig};

pub struct S3Client {
    client: Arc<Client>,
}

impl S3Client {
    pub fn new(config: S3ClientConfig) -> Self {
        Self {
            client: Arc::new(Self::build_client(config)),
        }
    }

    fn build_client(config: S3ClientConfig) -> Client {
        let creds = Credentials::new(config.access_key().to_owned(), config.secret_key().to_owned(), None, None, "static");
        let provider = SharedCredentialsProvider::new(creds);
        let region = Region::new(config.region().to_owned());
        
        let mut sdk_config = SdkConfig::builder()
        .region(region)
        .credentials_provider(provider)
        .behavior_version(BehaviorVersion::latest());

        if let Some(endpoint) = config.endpoint() {
            sdk_config = sdk_config.endpoint_url(endpoint.as_str());
        }

        Client::new(&sdk_config.build())
    }

    pub fn bucket(&self, bucket: impl AsRef<str>) -> S3ClientBucket {
        S3ClientBucket::new(self.client.clone(), bucket.as_ref().to_string())
    }
}