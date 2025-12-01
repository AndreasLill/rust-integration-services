#[cfg(feature = "s3")]
mod s3_sender;
#[cfg(feature = "s3")]
mod s3_sender_config;

#[cfg(feature = "s3")]
#[cfg(test)]
mod test {
    use crate::s3::{s3_sender::S3Sender, s3_sender_config::S3SenderConfig};

    #[tokio::test]
    async fn put_object() {
        let config = S3SenderConfig::builder()
        .endpoint("http://127.0.0.1:9000")
        .bucket("test")
        .region("auto")
        .access_key("minioadmin")
        .secret_key("minioadmin")
        .build();

        assert!(config.is_ok());

        let result = S3Sender::new(config.unwrap())
        .put_object("file_test.txt", "Text".into())
        .await;

        assert!(result.is_ok())
    }
}