use crate::smtp::{smtp_client::SmtpClient, smtp_client_config::SmtpClientConfig, smtp_message::SmtpMessage, smtp_security::SmtpSecurity};

#[tokio::test]
async fn test_smtp_client() {
    let config = SmtpClientConfig::builder()
    .host("127.0.0.1")
    .port(1025)
    .security(SmtpSecurity::None)
    .build()
    .unwrap();
    let client = SmtpClient::new(config);

    let message = SmtpMessage::builder()
    .from("from@email.com")
    .to("to@email.com")
    .cc("cc@email.com")
    .subject("subject")
    .body("body")
    .build()
    .unwrap();

    let res = client.send(message).await;

    assert!(res.is_ok());
}

#[test]
fn test_smtp_config_builder_success() {
    let config = SmtpClientConfig::builder()
        .host("smtp.example.com")
        .port(587)
        .security(SmtpSecurity::StartTls)
        .username("user@example.com")
        .password("password123")
        .build()
        .unwrap();

    assert_eq!(config.host(), "smtp.example.com");
    assert_eq!(config.port(), 587);
    assert_eq!(config.security(), &SmtpSecurity::StartTls);
    assert_eq!(config.username(), Some("user@example.com"));
    assert_eq!(config.password(), Some("password123"));
}

#[test]
fn test_smtp_builder_missing_host() {
    let result = SmtpClientConfig::builder()
        .port(587)
        .security(SmtpSecurity::StartTls)
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("host is required"));
}

#[test]
fn test_smtp_builder_missing_port() {
    let result = SmtpClientConfig::builder()
        .host("smtp.example.com")
        .security(SmtpSecurity::StartTls)
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("port is required"));
}

#[test]
fn test_smtp_builder_missing_security() {
    let result = SmtpClientConfig::builder()
        .host("smtp.example.com")
        .port(587)
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("security is required"));
}

#[test]
fn test_smtp_config_getters() {
    let config = SmtpClientConfig::builder()
        .host("smtp.example.com")
        .port(25)
        .security(SmtpSecurity::None)
        .username("user@example.com")
        .password("password123")
        .build()
        .unwrap();

    assert_eq!(config.host(), "smtp.example.com");
    assert_eq!(config.port(), 25);
    assert_eq!(config.security(), &SmtpSecurity::None);
    assert_eq!(config.username(), Some("user@example.com"));
    assert_eq!(config.password(), Some("password123"));
}

#[test]
fn test_smtp_message() {
    let message = SmtpMessage::builder()
    .from("from@example.com")
    .to("to@example.com")
    .subject("subject")
    .body("body")
    .build();

    assert!(message.is_ok());
}

#[test]
fn test_smtp_message_missing_from() {
    let message = SmtpMessage::builder()
        .to("to@example.com")
        .subject("subject")
        .body("body")
        .build();
    
    assert!(message.is_err());
}

#[test]
fn test_smtp_message_missing_to() {
    let message = SmtpMessage::builder()
        .from("from@example.com")
        .subject("subject")
        .body("body")
        .build();
    
    assert!(message.is_err());
}

#[test]
fn test_smtp_message_missing_subject() {
    let message = SmtpMessage::builder()
        .from("from@example.com")
        .to("to@example.com")
        .body("body")
        .build();
    
    assert!(message.is_err());
}

#[test]
fn test_smtp_message_missing_body() {
    let message = SmtpMessage::builder()
        .from("from@example.com")
        .to("to@example.com")
        .subject("subject")
        .build();
    
    assert!(message.is_err());
}