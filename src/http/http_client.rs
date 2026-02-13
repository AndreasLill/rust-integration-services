use std::{marker::PhantomData, sync::Arc};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, Uri, Version, body::Incoming, header::{HeaderName, HeaderValue}};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use crate::http::{http_client_config::HttpClientConfig, http_client_version::HttpClientVersion, http_executor::HttpExecutor, http_request::HttpRequest, http_response::HttpResponse};

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
        HttpClient {
            config: Arc::new(HttpClientConfig::new()),
            request: None,
            _state: PhantomData
        }
    }
}

impl HttpClient<HasRequest> {
    /// Sends an HTTP request to the server, automatically selecting the appropriate protocol and transport.
    /// 
    /// If the URL scheme is `"http"`, HTTP/1.1 will be used for the request.
    /// 
    /// If the URL scheme is `"https"`, a secure TLS connection is established and ALPN is used to determine whether to use HTTP/2 or HTTP/1.1 for the request.
    pub async fn send(self, endpoint: impl Into<String>) -> anyhow::Result<HttpResponse> {
        let url = endpoint.into().parse::<Uri>()?;
        let scheme = match url.scheme_str() {
            Some(scheme) => scheme,
            None => return Err(anyhow::anyhow!("URL is missing a scheme.")),
        };

        match scheme {
            "http" => {
                if self.config.http_version == HttpClientVersion::Http2 {
                    return Err(anyhow::anyhow!("https scheme is required for HTTP/2"));
                }
                self.send_tcp(url).await
            },
            "https" => self.send_tls(url).await,
            _ => Err(anyhow::anyhow!("Unsupported scheme: {}", scheme)),
        }
    }

    
    async fn send_tcp(self, url: Uri) -> anyhow::Result<HttpResponse> {
        let host = match url.host() {
            Some(host) => host,
            None => return Err(anyhow::anyhow!("Invalid URL.")),
        };
        let port = url.port_u16().unwrap_or(80);
        
        let stream = TcpStream::connect((host, port)).await?;
        let io = TokioIo::new(stream);
        
        let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await?;
        
        tokio::spawn(async move {
            connection.await
        });
        
        let req = Self::build_http_request(url, self.request.unwrap(), Version::HTTP_11).await?;
        let res = sender.send_request(req).await?;
        let response = Self::build_http_response(res).await?;
        
        Ok(response)
    }
    
    async fn send_tls(self, url: Uri) -> anyhow::Result<HttpResponse> {
        let host = match url.host() {
            Some(host) => host,
            None => return Err(anyhow::anyhow!("Invalid URL.")),
        };
        let port = url.port_u16().unwrap_or(443);
        let domain = rustls::pki_types::ServerName::try_from(host.to_string())?;

        let mut tls_config = self.config.tls_config.clone();

        tls_config.alpn_protocols = match self.config.http_version {
            HttpClientVersion::Auto => vec![b"h2".to_vec(), b"http/1.1".to_vec()],
            HttpClientVersion::Http1 => vec![b"http/1.1".to_vec()],
            HttpClientVersion::Http2 => vec![b"h2".to_vec()],
        };
    
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

        match version {
            Version::HTTP_2 => {
                let io = TokioIo::new(tls_stream);
                let (mut sender, connection) = hyper::client::conn::http2::Builder::new(HttpExecutor).handshake(io).await?;
                
                tokio::spawn(async move {
                    connection.await
                });
                
                let req = Self::build_http_request(url, self.request.unwrap(), Version::HTTP_2).await?;
                let res = sender.send_request(req).await?;
                let response = Self::build_http_response(res).await?;
                
                Ok(response)
            }
            Version::HTTP_11 => {
                let io = TokioIo::new(tls_stream);
                let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await?;
        
                tokio::spawn(async move {
                    connection.await
                });
                
                let req = Self::build_http_request(url, self.request.unwrap(), Version::HTTP_11).await?;
                let res = sender.send_request(req).await?;
                let response = Self::build_http_response(res).await?;
                Ok(response)
            }
            _ => {
                Err(anyhow::anyhow!("Unsupported HTTP version"))
            }
        }
    }

    async fn build_http_request(url: Uri, request: HttpRequest, version: Version) -> anyhow::Result<Request<Full<Bytes>>> {
        let mut req = match version {
            Version::HTTP_2 => {
                Request::builder()
                    .version(version)
                    .method(request.method.as_str())
                    .uri(url.clone())
                    .body(request.body.into())?
            }
            Version::HTTP_11 => {
                let authority = match url.authority() {
                    Some(authority) => authority,
                    None => return Err(anyhow::anyhow!("Invalid URL.")),
                };
                let path = url.path();

                Request::builder()
                    .version(version)
                    .method(request.method.as_str())
                    .uri(path)
                    .header(hyper::header::HOST, authority.as_str())
                    .body(request.body.into())?
            }
            _ => return Err(anyhow::anyhow!("Unsupported HTTP version"))
        };

        for (key, value) in request.headers {
            let header_name = HeaderName::from_bytes(key.as_bytes())?;
            let header_value = HeaderValue::from_str(&value)?;
            req.headers_mut().insert(header_name, header_value);
        }

        Ok(req)
    }

    async fn build_http_response(res: Response<Incoming>) -> anyhow::Result<HttpResponse> {
        let (parts, body) = res.into_parts();
        let mut response = HttpResponse::new(parts.status.as_u16());
        
        for (key, value) in parts.headers {
            if let Some(key) = key {
                response.headers.insert(key.to_string(), value.to_str()?.to_string());
            }
        }
        
        response.body = body.collect().await?.to_bytes();
        
        Ok(response)
    }
}