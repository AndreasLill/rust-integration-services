use crate::{common::stream::ByteStream, file::file_client::FileClient};


#[tokio::test(start_paused = true)]
async fn client_test() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
    let client = FileClient::new();

    let result = client.write_to("/tmp/test.txt").from_bytes("bytes").await;
    assert!(result.is_ok());

    let result = client.write_to("/tmp/test.txt").from_stream(ByteStream::from("stream")).await;
    assert!(result.is_ok());

    let result = client.read_from("/tmp/test.txt").as_bytes().await;
    assert!(result.is_ok());

    let result = client.read_from("/tmp/test.txt").as_stream().await;
    assert!(result.is_ok());

    let result = client.copy_from("/tmp/test.txt").copy_to("/tmp/test_copy.txt").await;
    assert!(result.is_ok());

    let result = client.delete("/tmp/test.txt").await;
    assert!(result.is_ok());

    let result = client.delete("/tmp/test_copy.txt").await;
    assert!(result.is_ok());
}