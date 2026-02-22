use std::{marker::PhantomData, sync::Arc};

use hyper::{Request, Uri, Version};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use crate::http::{client::{http_client_config::HttpClientConfig, http_client_version::HttpClientVersion}, executor::Executor, http_request::HttpRequest, http_response::HttpResponse};

pub struct NoRequest;
pub struct HasRequest;

pub struct HttpClient<State> {
    config: Arc<HttpClientConfig>,
    request: Option<HttpRequest>,
    _state: PhantomData<State>
}

impl HttpClient<NoRequest> {
    pub fn new(config: HttpClientConfig) -> Self {
        Self {
            config: Arc::new(config),
            request: None,
            _state: PhantomData
        }
    }

    pub fn request(&self, request: HttpRequest) -> HttpClient<HasRequest> {
        HttpClient {
            config: self.config.clone(),
            request: Some(request),
            _state: PhantomData
        }
    }
}

impl Default for HttpClient<NoRequest> {
    fn default() -> Self {
        HttpClient::new(HttpClientConfig::new(HttpClientVersion::Auto))
    }
}

impl HttpClient<HasRequest> {
    /// Sends an HTTP request to the server, automatically selecting the appropriate protocol and transport.
    /// 
    /// If the URL scheme is `"http"`, HTTP/1.1 will be used for the request.
    /// 
    /// If the URL scheme is `"https"`, a secure TLS connection is established and ALPN is used to determine whether to use HTTP/2 or HTTP/1.1 for the request.
    pub async fn send(mut self, uri: impl Into<String>) -> anyhow::Result<HttpResponse> {
        let uri = uri.into().parse::<Uri>()?;
        let scheme = match uri.scheme_str() {
            Some(scheme) => scheme,
            None => return Err(anyhow::anyhow!("URL is missing a scheme.")),
        };
        self.request.as_mut().unwrap().parts.uri = uri.clone();

        match scheme {
            "http" => {
                if self.config.http_version == HttpClientVersion::Http2 {
                    return Err(anyhow::anyhow!("https scheme is required for HTTP/2"));
                }
                self.send_tcp().await
            },
            "https" => self.send_tls().await,
            _ => Err(anyhow::anyhow!("Unsupported scheme: {}", scheme)),
        }
    }

    
    async fn send_tcp(self) -> anyhow::Result<HttpResponse> {
        let request = self.request.unwrap();
        let uri = request.parts.uri.clone();

        let host = match uri.host() {
            Some(host) => host,
            None => return Err(anyhow::anyhow!("Invalid URL.")),
        };
        let port = uri.port_u16().unwrap_or(80);
        
        let stream = TcpStream::connect((host, port)).await?;
        let io = TokioIo::new(stream);
        
        let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await?;
        
        tokio::spawn(async move {
            connection.await
        });
        
        let res = sender.send_request(Request::from(request)).await?;
        Ok(HttpResponse::from(res))
    }
    
    async fn send_tls(self) -> anyhow::Result<HttpResponse> {
        let mut request = self.request.unwrap();
        let uri = request.parts.uri.clone();

        let host = match uri.host() {
            Some(host) => host,
            None => return Err(anyhow::anyhow!("Invalid URL.")),
        };
        let port = uri.port_u16().unwrap_or(443);
        let domain = rustls::pki_types::ServerName::try_from(host.to_string())?;

        let tls_config = self.config.tls_config.clone();
        let tcp_stream = TcpStream::connect((host, port)).await?;
        let tls_connector = TlsConnector::from(Arc::new(tls_config));
        let tls_stream = tls_connector.connect(domain, tcp_stream).await?;

        let version = match self.config.http_version {
            HttpClientVersion::Auto => {
                let protocol = tls_stream.get_ref().1.alpn_protocol();
                match protocol.as_deref() {
                    Some(b"h2") => Version::HTTP_2,
                    _ => Version::HTTP_11,
                }
            },
            HttpClientVersion::Http1 => Version::HTTP_11,
            HttpClientVersion::Http2 => Version::HTTP_2,
        };
        request.parts.version = version;

        match version {
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