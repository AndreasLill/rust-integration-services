use rustls::{ClientConfig, RootCertStore};
use webpki_roots::TLS_SERVER_ROOTS;

use crate::http::{client::http_client_version::HttpClientVersion, crypto::Crypto};

pub struct HttpClientConfig {
    pub http_version: HttpClientVersion,
    pub tls_config: ClientConfig,
}

impl HttpClientConfig {
    /// Creates a new instance with a default set of trusted root CAs.
    /// 
    /// By default, the client trusts the system native root certs in addition to Mozilla root certificates provided by the
    /// [`webpki_roots`](https://docs.rs/webpki-roots) crate.
    /// 
    pub fn new() -> Self {
        let mut root_cert_store = RootCertStore::empty();
        root_cert_store.extend(TLS_SERVER_ROOTS.iter().cloned());
        let native_certs = rustls_native_certs::load_native_certs();
        for cert in native_certs.certs {
            root_cert_store.add(cert).unwrap();
        }
        for error in native_certs.errors {
            tracing::warn!("failed to load native cert: {:?}", error);
        }

        if let Err(error) = Crypto::install_crypto_provider() {
            tracing::warn!("failed to install crypto provider: {:?}", error);
        }

        let tls_config = ClientConfig::builder()
        .with_root_certificates(root_cert_store.clone())
        .with_no_client_auth();

        HttpClientConfig {
            http_version: HttpClientVersion::Auto,
            tls_config,
        }
    }

    pub fn http_version(mut self, version: HttpClientVersion) -> Self {
        self.http_version = version;
        self
    }
}