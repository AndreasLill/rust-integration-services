use crate::{common::stream::ByteStream, s3::{s3_client::S3Client, s3_client_config::S3ClientConfig}};

#[tokio::test]
async fn client_test() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
    let config = S3ClientConfig::builder().endpoint("http://127.0.0.1:9000").access_key("minioadmin").secret_key("minioadmin").build().unwrap();
    let client = S3Client::new(config);

    let result = client.bucket("test").put_object("test.txt").from_bytes("bytes").await;
    assert!(result.is_ok());

    let result = client.bucket("test").get_object("test.txt").as_bytes().await;
    assert!(result.is_ok());
    tracing::info!("{:?}", result);

    let result = client.bucket("test").delete_object("test.txt").await;
    assert!(result.is_ok());

    let result = client.bucket("test").put_object("test.txt").from_stream(ByteStream::from("bytestream")).await;
    assert!(result.is_ok());

    let result = client.bucket("test").get_object("test.txt").as_stream().await;
    assert!(result.is_ok());
    tracing::info!("{:?}", result.unwrap().as_bytes().await);

    let result = client.bucket("test").delete_object("test.txt").await;
    assert!(result.is_ok());
}