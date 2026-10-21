use std::path::PathBuf;

use crate::sftp::{sftp_auth_basic::SftpAuthBasic, sftp_auth_private_key::SftpAuthPrivateKey};

/// SFTP client configuration holding connection and auth details
pub struct SftpClientConfig {
    pub host: String,
    pub port: u16,
    pub auth_basic: Option<SftpAuthBasic>,
    pub auth_private_key: Option<SftpAuthPrivateKey>,
}

impl SftpClientConfig {
    /// Create a new builder for SftpClientConfig
    pub fn builder() -> SftpClientConfigBuilder {
        SftpClientConfigBuilder::new()
    }
}

/// Builder for SftpClientConfig
pub struct SftpClientConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    auth_basic: Option<SftpAuthBasic>,
    auth_private_key: Option<SftpAuthPrivateKey>,
}

impl SftpClientConfigBuilder {
    pub fn new() -> Self {
        Self {
            host: None,
            port: None,
            auth_basic: None,
            auth_private_key: None,
        }
    }

    /// Set the SFTP server host (required)
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Set the SFTP server port (required)
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Basic authentication using username/password (optional)
    pub fn auth_basic(mut self, user: impl Into<String>, password: impl Into<String>) -> Self {
        self.auth_basic = Some(
            SftpAuthBasic {
                user: user.into(),
                password: password.into(),
            }
        );
        self
    }

    /// Private key authentication with optional passphrase (optional)
    pub fn auth_private_key(mut self, user: impl Into<String>, path: impl Into<PathBuf>, passphrase: impl Into<Option<String>>) -> Self {
        self.auth_private_key = Some(
            SftpAuthPrivateKey {
                user: user.into(),
                path: path.into(),
                passphrase: passphrase.into(),
            }
        );
        self
    }

    /// Build the config, validating required fields
    pub fn build(self) -> anyhow::Result<SftpClientConfig> {
        if self.host.is_none() {
            anyhow::bail!("host is required")
        }
        if self.port.is_none() {
            anyhow::bail!("port is required")
        }

        Ok(SftpClientConfig {
            host: self.host.unwrap(),
            port: self.port.unwrap(),
            auth_basic: self.auth_basic,
            auth_private_key: self.auth_private_key,
        })
    }
}