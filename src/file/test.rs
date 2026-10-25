use crate::file::file_client::FileClient;


#[tokio::test]
async fn client_test() {
    tracing_subscriber::fmt().init();
    let client = FileClient::new();

    let result = client.write("/tmp/test.txt").body("hello_file").await;
    assert!(result.is_ok());

    let result = client.read("/tmp/test.txt").await;
    assert!(result.is_ok());
    let body = result.unwrap().to_string().await.unwrap();
    assert_eq!(body, "hello_file");

    let result = client.copy("/tmp/test.txt").to("/tmp/test_copy.txt").await;
    assert!(result.is_ok());

    let result = client.delete("/tmp/test_copy.txt").await;
    assert!(result.is_ok());

    let result = client.delete("/tmp/test.txt").await;
    assert!(result.is_ok());
}