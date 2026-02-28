use std::sync::Arc;

use hyper::{Request, Version};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use crate::http::{client::http_client_config::HttpClientConfig, executor::Executor, http_request::HttpRequest, http_response::HttpResponse};

pub struct HttpClient {
    config: Arc<HttpClientConfig>,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            config: Arc::new(HttpClientConfig::new()),
        }
    }

    /// Sends an HTTP request to the server, automatically selecting the appropriate protocol and transport.
    /// 
    /// ALPN is used to determine whether to use HTTP/2 or HTTP/1.1 for the request.
    pub async fn send(self, request: HttpRequest) -> anyhow::Result<HttpResponse> {

        let scheme = match request.parts.uri.scheme_str()  {
            Some(scheme) => scheme,
            None => return Err(anyhow::anyhow!("URL is missing a scheme.")),
        };

        match request.parts.uri.scheme_str().unwrap_or_default() {
            "http" => self.send_tcp(request).await,
            "https" => self.send_tls(request).await,
            _ => Err(anyhow::anyhow!("Unsupported scheme: {}", scheme)),
        }
    }

    
    async fn send_tcp(self, request: HttpRequest) -> anyhow::Result<HttpResponse> {
        let host = match request.parts.uri.host() {
            Some(host) => host,
            None => return Err(anyhow::anyhow!("Invalid URL.")),
        };

        let port = request.parts.uri.port_u16().unwrap_or(80);
        
        let stream = TcpStream::connect((host, port)).await?;
        let io = TokioIo::new(stream);
        
        let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await?;
        
        tokio::spawn(async move {
            connection.await
        });
        
        let res = sender.send_request(Request::from(request)).await?;
        Ok(HttpResponse::from(res))
    }
    
    async fn send_tls(self, mut request: HttpRequest) -> anyhow::Result<HttpResponse> {
        let host = match request.parts.uri.host() {
            Some(host) => host,
            None => return Err(anyhow::anyhow!("Invalid URL.")),
        };

        let port = request.parts.uri.port_u16().unwrap_or(443);
        let domain = rustls::pki_types::ServerName::try_from(host.to_string())?;

        let tls_config = self.config.tls_config.clone();
        let tcp_stream = TcpStream::connect((host, port)).await?;
        let tls_connector = TlsConnector::from(Arc::new(tls_config));
        let tls_stream = tls_connector.connect(domain, tcp_stream).await?;

        let protocol = tls_stream.get_ref().1.alpn_protocol();
        request.parts.version = match protocol.as_deref() {
            Some(b"h2") => Version::HTTP_2,
            _ => Version::HTTP_11,
        };

        match request.parts.version {
            Version::HTTP_2 => {
                let io = TokioIo::new(tls_stream);
                let (mut sender, connection) = hyper::client::conn::http2::Builder::new(Executor).handshake(io).await?;
                
                tokio::spawn(async move {
                    connection.await
                });
                
                let res = sender.send_request(Request::from(request)).await?;
                Ok(HttpResponse::from(res))
            }
            Version::HTTP_11 => {
                let io = TokioIo::new(tls_stream);
                let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await?;
        
                tokio::spawn(async move {
                    connection.await
                });
                
                let res = sender.send_request(Request::from(request)).await?;
                Ok(HttpResponse::from(res))
            }
            _ => {
                Err(anyhow::anyhow!("Unsupported HTTP version"))
            }
        }
    }
}