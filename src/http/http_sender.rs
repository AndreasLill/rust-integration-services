use std::sync::Arc;

use http_body_util::{BodyExt, Full};
use hyper::{body::{Bytes, Incoming}, header::{HeaderName, HeaderValue}, Request, Response, Uri, Version};
use hyper_util::rt::TokioIo;
use rustls::{ClientConfig, RootCertStore};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tracing::warn;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::http::{http_executor::HttpExecutor, http_request::HttpRequest, http_response::HttpResponse, http_status::HttpStatus};

pub struct HttpSender {
    url: String,
    root_cert_store: RootCertStore,
    http1_only: bool,
    http2_only: bool,
}

impl HttpSender {
    /// Creates a new `HttpSender` instance with a default set of trusted root CAs.
    /// 
    /// By default, the client trusts the system native root certs in addition to Mozilla root certificates provided by the
    /// [`webpki_roots`](https://docs.rs/webpki-roots) crate.
    /// 
    pub fn new<T: AsRef<str>>(url: T) -> Self {
        let mut root_cert_store = RootCertStore::empty();
        root_cert_store.extend(TLS_SERVER_ROOTS.iter().cloned());
        let native_certs = rustls_native_certs::load_native_certs();
        for cert in native_certs.certs {
            root_cert_store.add(cert).unwrap();
        }
        for error in native_certs.errors {
            warn!("failed to load native cert: {:?}", error);
        }

        HttpSender {
            url: url.as_ref().to_string(),
            root_cert_store,
            http1_only: false,
            http2_only: false,
        }
    }

    /// Force the use of HTTP/1.1.
    pub fn http1_only(mut self) -> Self {
        self.http1_only = true;
        self
    }

    /// Force the use of HTTP/2.
    pub fn http2_only(mut self) -> Self {
        self.http2_only = true;
        self
    }

    /// Sends an HTTP request to the server, automatically selecting the appropriate protocol and transport.
    /// 
    /// If the URL scheme is `"http"`, HTTP/1.1 will be used for the request.
    /// 
    /// If the URL scheme is `"https"`, a secure TLS connection is established and ALPN is used to determine whether to use HTTP/2 or HTTP/1.1 for the request.
    pub async fn send(&self, request: HttpRequest) -> anyhow::Result<HttpResponse> {
        let url = self.url.parse::<Uri>()?;
        let scheme = match url.scheme_str() {
            Some(scheme) => scheme,
            None => return Err(anyhow::anyhow!("URL is missing a scheme.")),
        };

        if self.http1_only && self.http2_only {
            return Err(anyhow::anyhow!("Use of both http1_only and http2_only"))
        }

        match scheme {
            "http" => {
                if self.http2_only {
                    return Err(anyhow::anyhow!("https scheme is required for HTTP/2"));
                }
                self.send_tcp(url, request).await
            },
            "https" => self.send_tls(url, request).await,
            _ => Err(anyhow::anyhow!("Unsupported scheme: {}", scheme)),
        }
    }

    
    async fn send_tcp(&self, url: Uri, request: HttpRequest) -> anyhow::Result<HttpResponse> {
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
        
        let req = Self::build_http_request(url, request, Version::HTTP_11).await?;
        let res = sender.send_request(req).await?;
        let response = Self::build_http_response(res).await?;
        
        Ok(response)
    }
    
    async fn send_tls(&self, url: Uri, request: HttpRequest) -> anyhow::Result<HttpResponse> {
        let host = match url.host() {
            Some(host) => host,
            None => return Err(anyhow::anyhow!("Invalid URL.")),
        };
        let port = url.port_u16().unwrap_or(443);
        let domain = rustls::pki_types::ServerName::try_from(host.to_string())?;
        
        rustls::crypto::ring::default_provider().install_default().map_err(
            |err| anyhow::anyhow!("Failed to install crypto provider {:?}", err)
        )?;
        
        let mut tls_config = ClientConfig::builder()
        .with_root_certificates(self.root_cert_store.clone())
        .with_no_client_auth();

        if self.http1_only {
            tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
        } else if self.http2_only {
            tls_config.alpn_protocols = vec![b"h2".to_vec()];
        } else {
            tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        }
    
        let tcp_stream = TcpStream::connect((host, port)).await?;
        let tls_connector = TlsConnector::from(Arc::new(tls_config));
        let tls_stream = tls_connector.connect(domain, tcp_stream).await?;

        let version = if self.http1_only {
            Version::HTTP_11
        } else if self.http2_only {
            Version::HTTP_2
        } else {
            let protocol = tls_stream.get_ref().1.alpn_protocol();
            match protocol.as_deref() {
                Some(b"h2") => Version::HTTP_2,
                _ => Version::HTTP_11,
            }
        };

        match version {
            Version::HTTP_2 => {
                let io = TokioIo::new(tls_stream);
                let (mut sender, connection) = hyper::client::conn::http2::Builder::new(HttpExecutor).handshake(io).await?;
                
                tokio::spawn(async move {
                    connection.await
                });
                
                let req = Self::build_http_request(url, request, Version::HTTP_2).await?;
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
                
                let req = Self::build_http_request(url, request, Version::HTTP_11).await?;
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
        let mut response = HttpResponse::new();
        response.status = HttpStatus::from_code(parts.status.as_u16())?;
        
        for (key, value) in parts.headers {
            if let Some(key) = key {
                response.headers.insert(key.to_string(), value.to_str()?.to_string());
            }
        }
        
        response.body = body.collect().await?.to_bytes();
        
        Ok(response)
    }
}