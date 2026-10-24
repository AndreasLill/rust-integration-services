use crate::s3::{s3_client::S3Client, s3_client_config::S3ClientConfig};

#[tokio::test]
#[ignore="integration test with local s3 server"]
async fn test_s3_client() {
    tracing_subscriber::fmt().init();
    let config = S3ClientConfig::builder()
    .host("http://127.0.0.1")
    .port(9000)
    .access_key("rustfsadmin")
    .secret_key("rustfsadmin")
    .build()
    .unwrap();

    let client = S3Client::new(config);

    let result = client.bucket("test").put_object("test.txt").body("hello_s3").await;
    assert!(result.is_ok());

    let result = client.bucket("test").get_object("test.txt").await;
    assert!(result.is_ok());
    let body = result.unwrap().to_string().await.unwrap();
    assert_eq!(body, "hello_s3");

    let result = client.bucket("test").delete_object("test.txt").await;
    assert!(result.is_ok());
}

#[test]
fn test_s3_config() {
    let config = S3ClientConfig::builder()
    .host("http://127.0.0.1")
    .port(9000)
    .build()
    .unwrap();

    assert_eq!(config.host, "http://127.0.0.1");
    assert_eq!(config.port, 9000);
    assert_eq!(config.region, None);
    assert_eq!(config.access_key, None);
    assert_eq!(config.secret_key, None);
}

#[test]
fn test_s3_config_host() {
    let result = S3ClientConfig::builder()
    .port(9000)
    .build();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "host is required");
}

#[test]
fn test_s3_config_port() {
    let result = S3ClientConfig::builder()
    .host("http://127.0.0.1")
    .build();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "port is required");
}