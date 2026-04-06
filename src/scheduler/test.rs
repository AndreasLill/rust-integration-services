#[tokio::test]
async fn client_test() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
}