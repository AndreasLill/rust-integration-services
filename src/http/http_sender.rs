use std::{path::Path, sync::Arc};

use http_body_util::{BodyExt, Full};
use hyper::{body::{Bytes, Incoming}, header::{HeaderName, HeaderValue}, Request, Response, Uri, Version};
use hyper_util::rt::TokioIo;
use rustls::{ClientConfig, RootCertStore};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::{http::{http_executor::HttpExecutor, http_request::HttpRequest, http_response::HttpResponse, http_status::HttpStatus}, utils::{crypto::Crypto, error::Error, result::ResultDyn}};

pub struct HttpSender {
    root_cert_store: RootCertStore,
}

impl HttpSender {
    /// Creates a new `HttpSender` instance with a default set of trusted root CAs.
    ///
    /// This initializes the internal `RootCertStore` with the Mozilla-recommended `TLS_SERVER_ROOTS` for use in TLS client connections.
    /// 
    /// To use a custom root certificate authority (CA), call the [`tls_root_ca()`] method
    /// before sending the request. This will override the default trust store.
    pub fn new() -> Self {
        let mut root_cert_store = RootCertStore::empty();
        root_cert_store.extend(TLS_SERVER_ROOTS.iter().cloned());

        HttpSender {
            root_cert_store,
        }
    }

    /// Sets a custom root certificate authority (CA) in PEM format for verifying TLS connections.
    /// 
    /// This overrides the default Mozilla trust store used for HTTPS connections.
    pub fn root_ca<T: AsRef<Path>>(mut self, root_ca_path: T) -> Self {
        let certs = Crypto::pem_load_certs(root_ca_path).expect("Could not load Root CA");
        
        self.root_cert_store = RootCertStore::empty();
        for cert in certs {
            self.root_cert_store.add(cert).expect("Could not add root CA to root store");
        }
        self
    }

    /// Sends an HTTP request to the server, automatically selecting the appropriate protocol and transport.
    /// 
    /// If the URL scheme is `"http"`, HTTP/1.1 will be used for the request.
    /// 
    /// If the URL scheme is `"https"`, a secure TLS connection is established and ALPN is used to determine whether to use HTTP/2 or HTTP/1.1 for the request.
    /// 
    /// By default, the client trusts the Mozilla root certificates provided by the
    /// [`webpki_roots`](https://docs.rs/webpki-roots) crate to validate server certificates.
    /// 
    /// To override the default root certificate store and use a custom Certificate Authority (CA),
    /// call [`tls_root_ca()`] before sending the request.
    pub async fn send<T: AsRef<str>>(&self, url: T, request: HttpRequest) -> ResultDyn<HttpResponse> {
        let url = url.as_ref().parse::<Uri>()?;
        let scheme = url.scheme_str().ok_or("URL is missing a scheme.")?;

        match scheme {
            "http" => self.send_http(url, request).await,
            "https" => self.send_https(url, request).await,
            _ => Err(Box::new(Error::tokio_io(format!("Unsupported scheme: {}", scheme))))
        }
    }

    
    async fn send_http(&self, url: Uri, request: HttpRequest) -> ResultDyn<HttpResponse> {
        let host = url.host().ok_or("Invalid URL.")?;
        let port = url.port_u16().unwrap_or(80);
        
        let stream = TcpStream::connect((host, port)).await?;
        let io = TokioIo::new(stream);
        
        let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await.unwrap();
        
        tokio::spawn(async move {
            connection.await
        });
        
        let req = Self::build_http_request(url, request, Version::HTTP_11).await?;
        let res = sender.send_request(req).await?;
        let response = Self::build_http_response(res).await?;
        
        Ok(response)
    }
    
    async fn send_https(&self, url: Uri, request: HttpRequest) -> ResultDyn<HttpResponse> {
        let host = url.host().ok_or("Invalid URL.")?;
        let port = url.port_u16().unwrap_or(443);
        let domain = rustls::pki_types::ServerName::try_from(host.to_string())?;
        
        let mut tls_config = ClientConfig::builder()
        .with_root_certificates(self.root_cert_store.clone())
        .with_no_client_auth();
        tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    
        let tcp_stream = TcpStream::connect((host, port)).await?;
        let tls_connector = TlsConnector::from(Arc::new(tls_config));
        let tls_stream = tls_connector.connect(domain, tcp_stream).await?;
        let protocol = tls_stream.get_ref().1.alpn_protocol();

        match protocol.as_deref() {
            Some(b"h2") => {
                let io = TokioIo::new(tls_stream);
                let (mut sender, connection) = hyper::client::conn::http2::Builder::new(HttpExecutor).handshake(io).await.unwrap();
                
                tokio::spawn(async move {
                    connection.await
                });
                
                let req = Self::build_http_request(url, request, Version::HTTP_2).await?;
                let res = sender.send_request(req).await?;
                let response = Self::build_http_response(res).await?;
                
                Ok(response)
            },
            _ => {
                let io = TokioIo::new(tls_stream);
                let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await.unwrap();

                tokio::spawn(async move {
                    connection.await
                });
                
                let req = Self::build_http_request(url, request, Version::HTTP_11).await?;
                let res = sender.send_request(req).await?;
                let response = Self::build_http_response(res).await?;
                Ok(response)
            }
        }
    }

    async fn build_http_request(url: Uri, request: HttpRequest, version: Version) -> ResultDyn<Request<Full<Bytes>>> {
        match version {
            Version::HTTP_2 => {
                let mut req: Request<Full<Bytes>> = Request::builder()
                    .version(Version::HTTP_2)
                    .method(request.method.as_str())
                    .uri(url.clone())
                    .body(request.body.into())?;

                for (key, value) in request.headers {
                    let header_name = HeaderName::from_bytes(key.as_bytes())?;
                    let header_value = HeaderValue::from_str(&value)?;
                    req.headers_mut().insert(header_name, header_value);
                }

                Ok(req)
            }
            Version::HTTP_11 => {
                let authority = url.authority().ok_or("Invalid URL.")?;
                let path = url.path();

                let mut req: Request<Full<Bytes>> = Request::builder()
                    .version(Version::HTTP_11)
                    .method(request.method.as_str())
                    .uri(path)
                    .header(hyper::header::HOST, authority.as_str())
                    .body(request.body.into())?;

                for (key, value) in request.headers {
                    let header_name = HeaderName::from_bytes(key.as_bytes())?;
                    let header_value = HeaderValue::from_str(&value)?;
                    req.headers_mut().insert(header_name, header_value);
                }

                Ok(req)
            }
            _ => Err(Box::new(Error::std_io("Unsupported HTTP version")))
        }
    }

    async fn build_http_response(res: Response<Incoming>) -> ResultDyn<HttpResponse> {
        let (parts, body) = res.into_parts();
        let mut response = HttpResponse::new();
        response.status = HttpStatus::from_code(parts.status.as_u16())?;
        
        for (key, value) in parts.headers {
            if let Some(key) = key {
                response.headers.insert(key.to_string(), value.to_str()?.to_string());
            }
        }
        
        response.body = body.collect().await?.to_bytes().to_vec();
        
        Ok(response)
    }
}