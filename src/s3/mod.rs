#[cfg(feature = "s3")]
mod s3_client;
#[cfg(feature = "s3")]
mod s3_client_config;
#[cfg(feature = "s3")]
mod s3_client_get_object;
#[cfg(feature = "s3")]
mod s3_client_put_object;
#[cfg(feature = "s3")]
mod s3_client_delete_object;

#[cfg(feature = "s3")]
#[cfg(test)]
mod test {

    use crate::s3::{s3_client::S3Client, s3_client_config::S3ClientConfig};

    #[tokio::test]
    async fn put_object() {
        tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
        let config = S3ClientConfig::builder()
        .endpoint("http://127.0.0.1:9000")
        .region("auto")
        .access_key("minioadmin")
        .secret_key("minioadmin")
        .build();

        tracing::info!("{:?}", config);
        assert!(config.is_ok());

        let result = S3Client::new(config.unwrap())
        .put_object("test", "file_test.txt")
        .body("test")
        .send()
        .await;

        tracing::info!("{:?}", result);
        assert!(result.is_ok())
    }

    #[tokio::test]
    async fn get_object() {
        tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
        let config = S3ClientConfig::builder()
        .endpoint("http://127.0.0.1:9000")
        .region("auto")
        .access_key("minioadmin")
        .secret_key("minioadmin")
        .build();

        tracing::info!("{:?}", config);
        assert!(config.is_ok());

        let result = S3Client::new(config.unwrap())
        .get_object("test", "file_test.txt")
        .send()
        .await;

        tracing::info!("{:?}", result);
        assert!(result.is_ok())
    }

    #[tokio::test]
    async fn delete_object() {
        tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
        let config = S3ClientConfig::builder()
        .endpoint("http://127.0.0.1:9000")
        .region("auto")
        .access_key("minioadmin")
        .secret_key("minioadmin")
        .build();

        tracing::info!("{:?}", config);
        assert!(config.is_ok());

        let result = S3Client::new(config.unwrap())
        .delete_object("test", "file_test.txt")
        .send()
        .await;

        tracing::info!("{:?}", result);
        assert!(result.is_ok())
    }
}