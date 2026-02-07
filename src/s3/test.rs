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
    let client = S3Client::new(config.unwrap());

    let result = client.bucket("test").put_object_bytes("file_test.txt", "text").await;
    tracing::info!("{:?}", result);
    assert!(result.is_ok());

    let result = client.bucket("test").get_object("file_test.txt", "/tmp").await;
    tracing::info!("{:?}", result);
    assert!(result.is_ok());

    let result = client.bucket("test").get_object_as_bytes("file_test.txt").await;
    tracing::info!("{:?}", result);
    assert!(result.is_ok());

    let result = client.bucket("test").get_objects("/tmp").await;
    tracing::info!("{:?}", result);
    assert!(result.is_ok());

    let result = client.bucket("test").get_objects_with_regex("/tmp", "^file_test.*$").await;
    tracing::info!("{:?}", result);
    assert!(result.is_ok());

    let result = client.bucket("test").delete_object("file_test.txt").await;
    tracing::info!("{:?}", result);
    assert!(result.is_ok());
}