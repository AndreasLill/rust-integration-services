use crate::smtp::smtp_security::SmtpSecurity;

pub struct SmtpClientConfig {
    pub host: String,
    pub port: u16,
    pub security: SmtpSecurity,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl SmtpClientConfig {
    pub fn new(host: impl Into<String>, port: u16, security: SmtpSecurity, username: Option<String>, password: Option<String>) -> Self {
        Self {
            host: host.into(),
            port: port,
            security: security,
            username: username,
            password: password
        }
    }
}