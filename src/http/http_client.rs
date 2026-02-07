use std::sync::Arc;

use rustls::{ClientConfig, RootCertStore};
use tracing::warn;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::http::{crypto::Crypto, http_client_config::HttpClientConfig, http_client_request::HttpClientRequest, http_request::HttpRequest};

pub struct HttpClient {
    config: Arc<HttpClientConfig>,
    tls_config: Arc<ClientConfig>,
}

impl HttpClient {
    /// Creates a new `HttpClient` instance with a default set of trusted root CAs.
    /// 
    /// By default, the client trusts the system native root certs in addition to Mozilla root certificates provided by the
    /// [`webpki_roots`](https://docs.rs/webpki-roots) crate.
    /// 
    pub fn new(config: HttpClientConfig) -> Self {
        let mut root_cert_store = RootCertStore::empty();
        root_cert_store.extend(TLS_SERVER_ROOTS.iter().cloned());
        let native_certs = rustls_native_certs::load_native_certs();
        for cert in native_certs.certs {
            root_cert_store.add(cert).unwrap();
        }
        for error in native_certs.errors {
            warn!("failed to load native cert: {:?}", error);
        }

        if let Err(error) = Crypto::install_crypto_provider() {
            warn!("failed to install crypto provider: {:?}", error);
        }

        let tls_config = ClientConfig::builder()
        .with_root_certificates(root_cert_store.clone())
        .with_no_client_auth();

        Self {
            config: Arc::new(config),
            tls_config: Arc::new(tls_config),
        }
    }

    pub fn default() -> Self {
        Self::new(HttpClientConfig::default())
    }

    pub fn request(&self, request: HttpRequest) -> HttpClientRequest {
        HttpClientRequest::new(self.config.clone(), self.tls_config.clone(), request)
    }
}