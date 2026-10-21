use crate::smtp::smtp_security::SmtpSecurity;

/// SMTP client configuration.
///
/// This struct holds the configuration for connecting to an SMTP server.
#[derive(Clone, Debug)]
pub struct SmtpClientConfig {
    pub host: String,
    pub port: u16,
    pub security: SmtpSecurity,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl SmtpClientConfig {
    pub fn builder() -> SmtpClientConfigBuilder {
        SmtpClientConfigBuilder {
            host: None,
            port: None,
            security: None,
            username: None,
            password: None,
        }
    }

    /// Returns the SMTP server host.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the SMTP server port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Returns the security mode.
    pub fn security(&self) -> &SmtpSecurity {
        &self.security
    }

    /// Returns the username, if configured.
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Returns the password, if configured.
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }
}

/// Builder for creating `SmtpClientConfig`.
pub struct SmtpClientConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    security: Option<SmtpSecurity>,
    username: Option<String>,
    password: Option<String>,
}

impl SmtpClientConfigBuilder {
    /// Sets the SMTP server host.
    ///
    /// # Examples
    /// ```ignore
    /// let config = SmtpClientBuilder::new().host("smtp.example.com")?;
    /// ```
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Sets the SMTP server port.
    ///
    /// Common ports:
    /// - 25: Unencrypted SMTP (not recommended)
    /// - 465: SMTP over TLS (implicit TLS)
    /// - 587: SMTP with STARTTLS
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Sets the security mode.
    ///
    /// # Security Recommendations
    /// - Use `SmtpSecurity::Tls` for port 465 (implicit TLS).
    /// - Use `SmtpSecurity::StartTls` for port 587 (explicit TLS).
    /// - Avoid `SmtpSecurity::None` in production.
    pub fn security(mut self, security: SmtpSecurity) -> Self {
        self.security = Some(security);
        self
    }

    /// Sets the username for authentication.
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Sets the password for authentication.
    ///
    /// # Security Warning
    /// Consider using environment variables or a secure vault instead of passing passwords directly.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Consumes the builder and returns the configuration.
    ///
    /// # Errors
    /// Returns an error if any required field (host, port, security) is missing.
    pub fn build(self) -> anyhow::Result<SmtpClientConfig> {
        let host = self.host.ok_or_else(|| anyhow::anyhow!("host is required"))?;
        let port = self.port.ok_or_else(|| anyhow::anyhow!("port is required"))?;
        let security = self.security.ok_or_else(|| anyhow::anyhow!("security is required"))?;

        Ok(SmtpClientConfig {
            host,
            port,
            security,
            username: self.username,
            password: self.password,
        })
    }
}