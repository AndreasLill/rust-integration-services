use crate::s3::{s3_client::S3Client, s3_client_config::S3ClientConfig};

#[tokio::test]
async fn client_test() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
    let config = S3ClientConfig::builder()
    .endpoint("http://127.0.0.1:9000")
    .access_key("minioadmin")
    .secret_key("minioadmin")
    .build();

    tracing::info!("{:?}", config);
    assert!(config.is_ok());

    let mut client = S3Client::new(config.unwrap());

    let result = client.put_object("test", "file_test.txt").body("test").send().await;
    tracing::info!("{:?}", result);
    assert!(result.is_ok());

    let result = client.get_object("test", "file_test.txt").send().await;
    tracing::info!("{:?}", result);
    assert!(result.is_ok());

    let result = client.delete_object("test", "file_test.txt").send().await;
    tracing::info!("{:?}", result);
    assert!(result.is_ok());
}