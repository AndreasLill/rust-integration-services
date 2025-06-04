use std::sync::Arc;
use rustls::{ClientConfig, RootCertStore};
use tokio::{io::AsyncWriteExt, net::TcpStream};
use tokio::io::Error;
use tokio::io::ErrorKind;
use tokio_rustls::TlsConnector;
use url::Url;
use webpki_roots::TLS_SERVER_ROOTS;

use super::{http_request::HttpRequest, http_response::HttpResponse};

#[allow(dead_code)]
pub struct HttpSender {
    url: Url,
}

impl HttpSender {
    pub fn new(url: &str) -> Self {
        let url = Url::parse(url).expect("Invalid URL!");
        HttpSender { 
            url,
        }
    }

    /// Send a request to the url.
    /// 
    /// Path and host header will be set automatically from the url.
    pub async fn send(&mut self, mut request: HttpRequest) -> tokio::io::Result<HttpResponse> {
        let host = self.url.host_str().unwrap();
        let port = self.url.port_or_known_default().unwrap();
        let addr = format!("{}:{}", host, port);
        
        request.headers.insert(String::from("Host"), String::from(host));
        request.path = self.url.path().to_string();
        
        match self.url.scheme() {
            "http" => {
                let mut stream = TcpStream::connect(&addr).await?;
                stream.write_all(request.to_string().as_bytes()).await?;

                let response = HttpResponse::from_stream(&mut stream).await?;
                Ok(response)
            }
            "https" => {
                let mut root_store = RootCertStore::empty();
                root_store.extend(TLS_SERVER_ROOTS.iter().cloned());

                let config = ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();

                let connector = TlsConnector::from(Arc::new(config));
                let stream = TcpStream::connect(&addr).await?;

                let domain = rustls::pki_types::ServerName::try_from(host)
                .map_err(|_| tokio::io::Error::new(ErrorKind::Other, "Invalid DNS name."))?
                .to_owned();

                let mut tls_stream = connector.connect(domain, stream).await?;
                tls_stream.write_all(request.to_string().as_bytes()).await?;

                let response = HttpResponse::from_stream(&mut tls_stream).await?;
                Ok(response)
            }
            _ => {
                Err(Error::new(ErrorKind::Other, format!("Scheme '{}' not supported.", self.url.scheme())))
            }
        }
    }
}