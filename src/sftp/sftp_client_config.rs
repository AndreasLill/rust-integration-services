use std::{marker::PhantomData, path::PathBuf};

use crate::sftp::{sftp_auth_basic::SftpAuthBasic, sftp_auth_private_key::SftpAuthPrivateKey};

pub struct SftpClientConfig {
    pub endpoint: String,
    pub auth_basic: Option<SftpAuthBasic>,
    pub auth_private_key: Option<SftpAuthPrivateKey>,
}

impl SftpClientConfig {
    pub fn builder() -> SftpClientConfigBuilder<SetEndpoint> {
        SftpClientConfigBuilder { 
            endpoint: None,
            auth_basic: None,
            auth_private_key: None,
            _state: PhantomData
        }
    }
}

pub struct SetEndpoint;
pub struct Optional;

pub struct SftpClientConfigBuilder<State> {
    pub endpoint: Option<String>,
    pub auth_basic: Option<SftpAuthBasic>,
    pub auth_private_key: Option<SftpAuthPrivateKey>,
    _state: PhantomData<State>,
}

impl SftpClientConfigBuilder<SetEndpoint> {
    pub fn endpoint(self, endpoint: impl Into<String>) -> SftpClientConfigBuilder<Optional> {
        SftpClientConfigBuilder {
            endpoint: Some(endpoint.into()),
            auth_basic: self.auth_basic,
            auth_private_key: self.auth_private_key,
            _state: PhantomData
        }
    }
}

impl SftpClientConfigBuilder<Optional> {
    /// Basic authentication using password.
    pub fn auth_basic(mut self, user: impl Into<String>, password: impl Into<String>) -> Self {
        self.auth_basic = Some(
            SftpAuthBasic {
                user: user.into(),
                password: password.into(),
            }
        );
        self
    }

    /// Authentication using a private key.
    /// What hash algorithm to choose?
    /// Ed25519 = None
    /// ECDSA = None
    /// RSA = Some(HashAlg::Sha256) or Some(HashAlg::Sha512)
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

    pub fn build(self) -> anyhow::Result<SftpClientConfig> {
        Ok(SftpClientConfig {
            endpoint: self.endpoint.ok_or_else(|| anyhow::anyhow!("Endpoint not found"))?,
            auth_basic: self.auth_basic,
            auth_private_key: self.auth_private_key
        })
    }
}