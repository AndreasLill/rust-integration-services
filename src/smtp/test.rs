use crate::smtp::{smtp_client::SmtpClient, smtp_client_config::SmtpClientConfig, smtp_message::SmtpMessage, smtp_security::SmtpSecurity};

#[tokio::test]
async fn client_test() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    let config = SmtpClientConfig::new("127.0.0.1", 1025, SmtpSecurity::None, None, None);
    let client = SmtpClient::new(config);

    let message = SmtpMessage::new()
    .from("from@email.com")
    .to("to@email.com")
    .cc("cc@email.com")
    .subject("subject")
    .body("body");

    let res = client.send(message).await;

    tracing::info!(?res);
    assert!(res.is_ok());
}