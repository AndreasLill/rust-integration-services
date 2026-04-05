use crate::sftp::{sftp_client::SftpClient, sftp_client_config::SftpClientConfig};

#[tokio::test]
async fn client_test() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
    let config = SftpClientConfig::builder().endpoint("127.0.0.1:2222").auth_basic("user", "password").build().unwrap();
    let client = SftpClient::new(config);

    let result = client.put_file("upload/file.txt").from_bytes("hello world").await;
    assert!(result.is_ok());

    let result = client.get_file("upload/file.txt").as_bytes().await;
    assert!(result.is_ok());
    tracing::info!(?result);

    let result = client.delete_file("upload/file.txt").await;
    assert!(result.is_ok());
}