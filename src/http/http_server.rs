use std::{convert::Infallible, panic::AssertUnwindSafe, path::Path, pin::Pin, sync::Arc};

use futures::FutureExt;
use http_body_util::{BodyExt, Full};
use hyper::{body::{Bytes, Incoming}, header::{HeaderName, HeaderValue}, service::service_fn, Request, Response};
use hyper_util::rt::TokioIo;
use matchit::Router;
use rustls::ServerConfig;
use tokio::{net::{TcpListener, TcpStream}, signal::unix::{signal, SignalKind}, task::JoinSet};
use tokio_rustls::TlsAcceptor;

use crate::http::{crypto::Crypto, http_executor::HttpExecutor, http_request::HttpRequest, http_response::HttpResponse};

type RouteCallback = Arc<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync>;

pub struct HttpServer {
    host: String,
    router: Router<RouteCallback>,
    tls_config: Option<ServerConfig>,
}

impl HttpServer {
    /// Creates a new `HttpServer` instance bound to the specified host address. Example: `127.0.0.1:8080`.
    pub fn new(host: impl Into<String>) -> Self {
        HttpServer {
            host: host.into(),
            router: Router::new(),
            tls_config: None,
        }
    }

    /// Enables TLS for incoming connections using the provided server certificate and private key in `.pem` format and
    /// configures the TLS context and sets supported ALPN protocols to allow HTTP/2 and HTTP/1.1.
    pub fn tls(mut self, tls_server_cert_path: impl AsRef<Path>, tls_server_key_path: impl AsRef<Path>) -> Self {
        let mut tls_config = Self::create_tls_config(tls_server_cert_path, tls_server_key_path).expect("Failed to create TLS config");
        tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        self.tls_config = Some(tls_config);
        self
    }

    fn create_tls_config(cert_path: impl AsRef<Path>, key_path: impl AsRef<Path>) -> anyhow::Result<ServerConfig> {
        let certs = Crypto::pem_load_certs(cert_path)?;
        let key = Crypto::pem_load_private_key(key_path)?;

        Crypto::install_crypto_provider()?;
        
        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        Ok(config)
    }

    /// Registers a route with a path, associating it with a handler callback.
    pub fn route<T, Fut>(mut self, path: impl Into<String>, callback: T) -> Self
    where
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HttpResponse> + Send + 'static,
    {
        self.router.insert(path.into(), Arc::new(move |request| Box::pin(callback(request)))).unwrap();
        self
    }

    /// Starts the HTTP server and begins listening for incoming TCP connections (optionally over TLS).
    ///
    /// This method binds to the configured host address and enters a loop to accept new TCP connections.
    /// It also listens for system termination signals (SIGINT, SIGTERM) to gracefully shut down the server.
    pub async fn receive(self) {
        let tls_acceptor = self.tls_config.map(|tls_config| {
            TlsAcceptor::from(Arc::new(tls_config))
        });
        let listener = TcpListener::bind(&self.host).await.expect("Failed to start TCP Listener");
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver");
        let mut receiver_join_set = JoinSet::new();
        let router = Arc::new(self.router);
        
        tracing::trace!("started on {}", self.host);
        loop {
            tokio::select! {
                _ = sigterm.recv() => {
                    drop(listener);
                    break;
                },
                _ = sigint.recv() => {
                    drop(listener);
                    break;
                },
                result = listener.accept() => {
                    let tls_acceptor = tls_acceptor.clone();
                    let router = router.clone();
                    let (tcp_stream, client_addr) = match result {
                        Ok(pair) => pair,
                        Err(err) => {
                            tracing::error!("{:?}", err);
                            continue;
                        },
                    };

                    tracing::trace!("Connection {:?}", client_addr);
                    match tls_acceptor {
                        Some(acceptor) => {
                            receiver_join_set.spawn(Self::tls_connection(acceptor, tcp_stream, router));
                        },
                        None => {
                            receiver_join_set.spawn(Self::tcp_connection(tcp_stream, router));
                        },
                    }
                }
            }
        }

        tracing::trace!("shut down pending...");
        while let Some(_) = receiver_join_set.join_next().await {}
        tracing::trace!("shut down complete");
    }

    async fn tcp_connection(tcp_stream: TcpStream, router: Arc<Router<RouteCallback>>) {
        let service = {
            let router = router.clone();
            service_fn(move |req| {
                Self::incoming_request(req, router.clone())
            })
        };
        
        let io = TokioIo::new(tcp_stream);
        if let Err(err) = hyper::server::conn::http1::Builder::new().serve_connection(io, service).await {
            tracing::error!("{:?}", err);
        }
    }

    async fn tls_connection(tls_acceptor: TlsAcceptor, tcp_stream: TcpStream, router: Arc<Router<RouteCallback>>) {
        let tls_stream = match tls_acceptor.accept(tcp_stream).await {
            Ok(stream) => stream,
            Err(err) => {
                tracing::error!("TLS handshake failed {:?}", err);
                return;
            },
        };
        
        let service = {
            let router = router.clone();
            service_fn(move |req| {
                Self::incoming_request(req, router.clone())
            })
        };
        
        let io = TokioIo::new(tls_stream);
        let protocol = io.inner().get_ref().1.alpn_protocol();
        match protocol.as_deref() {
            Some(b"h2") => {
                if let Err(err) = hyper::server::conn::http2::Builder::new(HttpExecutor).serve_connection(io, service).await {
                    tracing::error!("TLS handshake failed {:?}", err);
                }
            }
            _ => {
                if let Err(err) = hyper::server::conn::http1::Builder::new().keep_alive(false).serve_connection(io, service).await {
                    tracing::error!("{:?}", err);
                }
            }
        }
    }

    async fn incoming_request(req: Request<Incoming>, router: Arc<Router<RouteCallback>>) -> Result<Response<Full<Bytes>>, Infallible> {
        let mut request = match Self::build_http_request(req).await {
            Ok(req) => req,
            Err(err) => {
                tracing::error!("{:?}", err);
                return Ok(Self::hyper_internal_server_error_response())
            },
        };

        match router.at(&request.path) {
            Ok(matched) => {
                request.params = matched.params.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect();
                let callback = matched.value;
                let callback_fut = callback(request);
                let result = AssertUnwindSafe(callback_fut).catch_unwind().await;
                let response = match result {
                    Ok(res) => res,
                    Err(err) => {
                        tracing::error!("{:?}", err);
                        HttpResponse::internal_server_error()
                    }
                };

                let hyper_res = match Self::build_http_response(response.clone()).await {
                    Ok(res) => res,
                    Err(err) => {
                        tracing::error!("{:?}", err);
                        return Ok(Self::hyper_internal_server_error_response())
                    },
                };

                Ok(hyper_res)
            },
            Err(_) => {
                let response = HttpResponse::not_found();
                let hyper_res = match Self::build_http_response(response.clone()).await {
                    Ok(res) => res,
                    Err(err) => {
                        tracing::error!("{:?}", err);
                        return Ok(Self::hyper_internal_server_error_response())
                    },
                };

                Ok(hyper_res)
            },
        }
    }

    async fn build_http_request(req: Request<Incoming>) -> anyhow::Result<HttpRequest> {
        let (parts, body) = req.into_parts();
        let mut request = HttpRequest::new(parts.method.as_str().to_string());
        request.path = parts.uri.path().to_string();
        for (key, value) in parts.headers.iter() {
            if let Ok(value) = value.to_str() {
                request.headers.insert(key.to_string(), value.to_string());
            }
        }
        request.body = body.collect().await?.to_bytes();
        Ok(request)
    }

    async fn build_http_response(res: HttpResponse) -> anyhow::Result<Response<Full<Bytes>>> {
        let mut response: Response<Full<Bytes>> = Response::builder().status(res.status).body(res.body.into())?;
        for (key, value) in res.headers.iter() {
            let header_name = HeaderName::from_bytes(key.as_bytes())?;
            let header_value = HeaderValue::from_bytes(&value.as_bytes())?;
            response.headers_mut().insert(header_name, header_value);
        }
        Ok(response)
    }

    fn hyper_internal_server_error_response() -> Response<Full<Bytes>> {
        Response::builder().status(500).body(Full::new(Bytes::new())).unwrap()
    }
}