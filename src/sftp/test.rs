use crate::sftp::{sftp_client::SftpClient, sftp_client_config::SftpClientConfig};

#[tokio::test]
#[ignore="integration test with local sftp server"]
async fn test_sftp_client() {
    let config = SftpClientConfig::builder()
    .host("127.0.0.1")
    .port(2222)
    .auth_basic("user", "password")
    .build()
    .unwrap();

    let mut client = SftpClient::new(config);

    let result = client.put_file("upload/file.txt").body("hello_sftp").await;
    assert!(result.is_ok());

    let result = client.get_file("upload/file.txt").await;
    assert!(result.is_ok());
    let body = result.unwrap().to_string().await.unwrap();
    assert_eq!(body, "hello_sftp");

    let result = client.delete_file("upload/file.txt").await;
    assert!(result.is_ok());
}

#[test]
fn test_sftp_config() {
    let config = SftpClientConfig::builder()
    .host("127.0.0.1")
    .port(2222)
    .auth_basic("user", "password")
    .build();
    assert!(config.is_ok());

    let config = config.unwrap();
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 2222);

    let auth_basic = config.auth_basic.unwrap();
    assert_eq!(auth_basic.user, "user");
    assert_eq!(auth_basic.password, "password");
}