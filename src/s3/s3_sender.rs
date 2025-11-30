use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_sdk_s3::{Client, config::Credentials};
use bytes::Bytes;

use crate::s3::{s3_auth::S3Auth, s3_sender_config::S3SenderConfig};

pub struct S3Sender {
    config: S3SenderConfig,
    sdk_config: Option<SdkConfig>
}

impl S3Sender {
    pub fn new(config: S3SenderConfig) -> Self {
        S3Sender {
            config,
            sdk_config: None
        }
    }

    async fn load_sdk_config(&mut self) -> &SdkConfig {
        if self.sdk_config.is_some() {
            return self.sdk_config.as_ref().unwrap();
        }

        let mut sdk_config = aws_config::defaults(BehaviorVersion::latest());

        if let Some(endpoint) = self.config.endpoint() {
            sdk_config = sdk_config.endpoint_url(endpoint.as_str());
        }

        if let S3Auth::Basic { user, password } = self.config.auth() {
            let creds = Credentials::new(user.to_owned(), password.to_owned(), None, None, "static");
            sdk_config = sdk_config.credentials_provider(creds);
        }

        let sdk_config = sdk_config.region(Region::new(self.config.region().to_owned()))
            .load()
            .await;

        self.sdk_config = Some(sdk_config);
        self.sdk_config.as_ref().unwrap()
    }

    pub async fn put_object<S: AsRef<str>>(&mut self, key: S, body: Bytes) -> anyhow::Result<()> {
        let config = self.load_sdk_config().await;

        Client::new(config)
            .put_object()
            .bucket(self.config.bucket())
            .key(key.as_ref())
            .body(body.into())
            .send()
            .await?;

        Ok(())
    }
}