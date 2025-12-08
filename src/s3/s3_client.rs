use std::sync::Arc;

use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_sdk_s3::{Client, config::{Credentials, SharedCredentialsProvider}};

use crate::s3::{s3_client_config::S3ClientConfig, s3_client_delete_object::S3ClientDeleteObject, s3_client_get_object::S3ClientGetObject, s3_client_put_object::S3ClientPutObject};

pub struct S3Client {
    client: Arc<Client>
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

    pub fn put_object(&mut self, bucket: impl AsRef<str>, key: impl AsRef<str>) -> S3ClientPutObject {
        S3ClientPutObject::new(self.client.clone(), bucket.as_ref(), key.as_ref())
    }

    pub fn get_object(&mut self, bucket: impl AsRef<str>, key: impl AsRef<str>) -> S3ClientGetObject {
        S3ClientGetObject::new(self.client.clone(), bucket.as_ref(), key.as_ref())
    }

    pub fn delete_object(&mut self, bucket: impl AsRef<str>, key: impl AsRef<str>) -> S3ClientDeleteObject {
        S3ClientDeleteObject::new(self.client.clone(), bucket.as_ref(), key.as_ref())
    }
}