use std::path::Path;

use rustls::{ServerConfig};

use crate::http::crypto::Crypto;

pub struct HttpServerConfig {
    pub host: String,
    pub port: u16,
    pub tls_config: Option<ServerConfig>,
}

impl HttpServerConfig {
    pub fn builder() -> HttpServerConfigBuilder {
        HttpServerConfigBuilder::new()
    }
}

pub struct HttpServerConfigBuilder {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub tls_config: Option<ServerConfig>,
}

impl HttpServerConfigBuilder {
    pub fn new() -> Self {
        Self {
            host: None,
            port: None,
            tls_config: None
        }
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Enables TLS for incoming connections using the provided server certificate and private key in `.pem` format and
    /// configures the TLS context and sets supported ALPN protocols to allow HTTP/2 and HTTP/1.1.
    pub fn tls(mut self, tls_server_cert_path: impl AsRef<Path>, tls_server_key_path: impl AsRef<Path>) -> Self {
        let certs = Crypto::pem_load_certs(tls_server_cert_path).expect("Failed to load server cert.");
        let key = Crypto::pem_load_private_key(tls_server_key_path).expect("Failed to load server key.");
        Crypto::install_crypto_provider().expect("Failed to install crypto provider.");

        let mut tls_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .expect("Failed to create tls server config.");

        tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        self.tls_config = Some(tls_config);
        self
    }

    pub fn build(self) -> anyhow::Result<HttpServerConfig> {
        if self.host.is_none() {
            anyhow::bail!("host is required")
        }
        if self.port.is_none() {
            anyhow::bail!("port is required")
        }

        Ok(HttpServerConfig {
            host: self.host.unwrap(),
            port: self.port.unwrap(),
            tls_config: self.tls_config
        })
    }
}