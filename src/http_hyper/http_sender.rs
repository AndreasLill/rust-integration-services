use std::sync::Arc;

use http_body_util::{BodyExt, Full};
use hyper::{body::{Bytes, Incoming}, header::{HeaderName, HeaderValue}, Request, Response, Uri};
use hyper_util::rt::TokioIo;
use rustls::{ClientConfig, RootCertStore};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::{http_hyper::{http_request::HttpRequest, http_response::HttpResponse}, utils::{error::Error, result::ResultDyn}};

pub struct HttpSender {
    url: String,
}

impl HttpSender {
    pub fn new<T: AsRef<str>>(url: T) -> Self {
        HttpSender {
            url: url.as_ref().to_string(),
        }
    }

    pub async fn send(self, request: HttpRequest) -> ResultDyn<HttpResponse> {
        let url = self.url.parse::<Uri>()?;
        let scheme = url.scheme_str().ok_or("URL is missing a scheme.")?;

        match scheme {
            "http" => Self::send_http(url, request).await,
            "https" => Self::send_https(url, request).await,
            _ => Err(Box::new(Error::tokio_io(format!("Unsupported scheme: {}", scheme))))
        }
    }

    async fn send_http(url: Uri, request: HttpRequest) -> ResultDyn<HttpResponse> {
        let host = url.host().ok_or("Invalid URL.")?;
        let port = url.port_u16().unwrap_or(80);
        let authority = url.authority().ok_or("Invalid URL.")?;
        let path = url.path();

        let stream = TcpStream::connect((host, port)).await?;
        let io = TokioIo::new(stream);

        let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await.unwrap();

        tokio::spawn(async move {
            connection.await
        });

        let mut req: Request<Full<Bytes>> = Request::builder()
            .method(request.method.as_str())
            .uri(path)
            .header(hyper::header::HOST, authority.as_str())
            .body(request.body.into())?;

        for (key, value) in request.headers {
            let header_name = HeaderName::from_bytes(key.as_bytes())?;
            let header_value = HeaderValue::from_str(&value)?;
            req.headers_mut().insert(header_name, header_value);
        }

        let res = sender.send_request(req).await?;
        let response = Self::build_http_response(res).await?;

        Ok(response)
    }

    async fn send_https(url: Uri, request: HttpRequest) -> ResultDyn<HttpResponse> {
        let host = url.host().ok_or("Invalid URL.")?;
        let port = url.port_u16().unwrap_or(443);
        let domain = rustls::pki_types::ServerName::try_from(host.to_string())?;
        let authority = url.authority().ok_or("Invalid URL.")?;
        let path = url.path();

        let mut root_store = RootCertStore::empty();
        root_store.extend(TLS_SERVER_ROOTS.iter().cloned());

        let tls_config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

        let tcp_stream = TcpStream::connect((host, port)).await?;
        let tls_connector = TlsConnector::from(Arc::new(tls_config));
        let tls_stream = tls_connector.connect(domain, tcp_stream).await?;

        let io = TokioIo::new(tls_stream);
        let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await.unwrap();

        tokio::spawn(async move {
            connection.await
        });

        let mut req: Request<Full<Bytes>> = Request::builder()
            .method(request.method.as_str())
            .uri(path)
            .header(hyper::header::HOST, authority.as_str())
            .body(request.body.into())?;

        for (key, value) in request.headers {
            let header_name = HeaderName::from_bytes(key.as_bytes())?;
            let header_value = HeaderValue::from_str(&value)?;
            req.headers_mut().insert(header_name, header_value);
        }

        let res = sender.send_request(req).await?;
        let response = Self::build_http_response(res).await?;

        Ok(response)
    }

    async fn build_http_response(res: Response<Incoming>) -> ResultDyn<HttpResponse> {
        let mut response = HttpResponse::new();
        response.status_code = res.status().as_u16();
        response.status_text = res.status().canonical_reason().unwrap_or("").to_string();

        for (key, value) in res.headers() {
            match value.to_str() {
                Ok(value) => {
                    response.headers.insert(key.as_str().to_string(), value.to_string());
                },
                Err(err) => return Err(Box::new(err)),
            }
        }

        response.body = res.collect().await?.to_bytes().to_vec();

        Ok(response)
    }
}