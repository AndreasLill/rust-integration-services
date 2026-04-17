use std::{collections::HashMap, convert::Infallible, pin::Pin, sync::Arc};

use futures::FutureExt;
use http_body_util::{BodyExt, combinators::BoxBody};
use hyper::{Request, Response, body::{Bytes, Incoming}, service::service_fn};
use hyper_util::rt::TokioIo;
use matchit::Router;
use tokio::{net::{TcpListener, TcpStream}, signal::unix::{signal, SignalKind}};
use tokio_rustls::TlsAcceptor;

use crate::http::{executor::Executor, http_request::HttpRequest, http_response::HttpResponse, server::http_server_config::HttpServerConfig};

type RouteCallback = Arc<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync>;
type BeforeCallback = Arc<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = BeforeResult> + Send>> + Send + Sync>;
type AfterCallback = Arc<dyn Fn(HttpResponse) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync>;
type ErrorCallback = Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync>;

pub struct HttpServer {
    config: HttpServerConfig,
    router: Router<RouteCallback>,
    before: Vec<BeforeCallback>,
    after: Vec<AfterCallback>,
    on_error: Option<ErrorCallback>,
}

impl HttpServer {
    pub fn builder(config: HttpServerConfig) -> HttpServerBuilder {
        HttpServerBuilder {
            config,
            router: Router::new(),
            before: Vec::new(),
            after: Vec::new(),
            on_error: None,
        }
    }

    /// Run the HTTP server and begins listening for incoming TCP connections (optionally over TLS).
    ///
    /// This method binds to the configured host address and enters a loop to accept new TCP connections.
    /// It also listens for system termination signals (SIGINT, SIGTERM) to gracefully shut down the server.
    pub async fn run(self) {
        let tls_acceptor = self.config.tls_config.map(|tls_config| {
            TlsAcceptor::from(Arc::new(tls_config))
        });

        let host = format!("{}:{}", self.config.ip, self.config.port);
        let listener = TcpListener::bind(&host).await.expect("Failed to start TCP Listener");
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver");
        let router = Arc::new(self.router);
        let before: Arc<[BeforeCallback]> = self.before.into();
        let after: Arc<[AfterCallback]> = self.after.into();
        let on_error = self.on_error;
        
        tracing::trace!("Started on {}", &host);
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
                    let before = before.clone();
                    let after = after.clone();
                    let on_error = on_error.clone();
                    let (tcp_stream, _client_addr) = match result {
                        Ok(pair) => pair,
                        Err(err) => {
                            tracing::error!("{:?}", err);
                            continue;
                        },
                    };

                    match tls_acceptor {
                        Some(acceptor) => {
                            tokio::spawn(Self::tls_connection(acceptor, tcp_stream, router, before, after, on_error));
                        },
                        None => {
                            tokio::spawn(Self::tcp_connection(tcp_stream, router, before, after, on_error));
                        },
                    }
                }
            }
        }

        tracing::trace!("Shut down complete");
    }

    async fn tcp_connection(tcp_stream: TcpStream, router: Arc<Router<RouteCallback>>, before: Arc<[BeforeCallback]>, after: Arc<[AfterCallback]>, on_error: Option<ErrorCallback>) {
        let service = {
            let router = router.clone();
            service_fn(move |req| {
                Self::incoming_request(req, router.clone(), before.clone(), after.clone(), on_error.clone())
            })
        };
        
        let io = TokioIo::new(tcp_stream);
        if let Err(err) = hyper::server::conn::http1::Builder::new().serve_connection(io, service).await {
            tracing::error!("{:?}", err);
        }
    }

    async fn tls_connection(tls_acceptor: TlsAcceptor, tcp_stream: TcpStream, router: Arc<Router<RouteCallback>>, before: Arc<[BeforeCallback]>, after: Arc<[AfterCallback]>, on_error: Option<ErrorCallback>) {
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
                Self::incoming_request(req, router.clone(), before.clone(), after.clone(), on_error.clone())
            })
        };
        
        let io = TokioIo::new(tls_stream);
        let protocol = io.inner().get_ref().1.alpn_protocol();
        match protocol.as_deref() {
            Some(b"h2") => {
                if let Err(err) = hyper::server::conn::http2::Builder::new(Executor).serve_connection(io, service).await {
                    tracing::error!("TLS handshake failed {:?}", err);
                }
            }
            _ => {
                if let Err(err) = hyper::server::conn::http1::Builder::new().serve_connection(io, service).await {
                    tracing::error!("{:?}", err);
                }
            }
        }
    }

    async fn incoming_request(request: Request<Incoming>, router: Arc<Router<RouteCallback>>, before: Arc<[BeforeCallback]>, after: Arc<[AfterCallback]>, on_error: Option<ErrorCallback>) -> Result<Response<BoxBody<Bytes, anyhow::Error>>, Infallible> {
        let result = std::panic::AssertUnwindSafe(Self::inner_request(request, router, before, after)).catch_unwind().await;
        match result {
            Ok(response) => response,
            Err(err) => {
                let error = if let Some(s) = err.downcast_ref::<String>() {
                    s.as_str()
                } else if let Some(s) = err.downcast_ref::<&str>() {
                    s
                } else {
                    "Unknown panic!"
                };

                let response = match on_error {
                    Some(handler) => {
                        handler(error.to_string()).await
                    },
                    None => {
                        tracing::error!("{:?}", error);
                        HttpResponse::builder().status(500).body_empty().unwrap()
                    },
                };

                Ok(Response::from(response))
            }
        }
    }

    async fn inner_request(request: Request<Incoming>, router: Arc<Router<RouteCallback>>, before: Arc<[BeforeCallback]>, after: Arc<[AfterCallback]>) -> Result<Response<BoxBody<Bytes, anyhow::Error>>, Infallible> {
        let path = request.uri().path().to_owned();
        match router.at(&path) {
            Ok(matched) => {
                let mut params: HashMap<String, String> = HashMap::with_capacity(matched.params.len());
                for (k, v) in matched.params.iter() {
                    params.insert(k.to_owned(), v.to_owned());
                }
                let (parts, body) = request.into_parts();
                let body = body.map_err(|e| anyhow::Error::from(e));
                let mut req = HttpRequest::from_parts_with_params(body.boxed(), parts, params);

                for handler in before.iter() {
                    match handler(req).await {
                        BeforeResult::Next(request) => {
                            req = request;
                        },
                        BeforeResult::Response(response) => {
                            let mut response = response;

                            for handler in after.iter() {
                                response = handler(response).await;
                            }

                            return Ok(Response::from(response))
                        },
                    }
                }

                let callback = matched.value;
                let mut response = callback(req).await;

                for handler in after.iter() {
                    response = handler(response).await;
                }

                Ok(Response::from(response))
            },
            Err(_) => {
                let response = HttpResponse::builder().status(404).body_empty().unwrap();
                Ok(Response::from(response))
            },
        }
    }
}

/// Result type used internally by the request middleware pipeline.
///
/// This type is typically not used directly by end users.
///
/// Instead, middleware can simply return either:
/// - `HttpRequest` to continue processing
/// - `HttpResponse` to short-circuit the pipeline
///
/// The conversion is handled automatically via `Into<BeforeResult>`.
pub enum BeforeResult {
    Next(HttpRequest),
    Response(HttpResponse),
}

impl From<HttpRequest> for BeforeResult {
    fn from(req: HttpRequest) -> Self {
        BeforeResult::Next(req)
    }
}

impl From<HttpResponse> for BeforeResult {
    fn from(res: HttpResponse) -> Self {
        BeforeResult::Response(res)
    }
}

pub struct HttpServerBuilder {
    config: HttpServerConfig,
    router: Router<RouteCallback>,
    before: Vec<BeforeCallback>,
    after: Vec<AfterCallback>,
    on_error: Option<ErrorCallback>,
}

impl HttpServerBuilder {
    /// Add a middleware to the request pipeline.
    ///
    /// - Return `HttpRequest` to continue to the next middleware or route handler
    /// - Return `HttpResponse` to short-circuit the pipeline and respond immediately
    ///
    /// Response will still be processed by the response `after` pipeline, if any.
    pub fn before<T, Fut, R>(mut self, callback: T) -> Self
    where 
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: Into<BeforeResult> + 'static,
    {
        let callback = Arc::new(callback);
        self.before.push(Arc::new(move |request| {
            let callback = Arc::clone(&callback);
            Box::pin(async move {
                callback(request).await.into()
            })
        }));
        self
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

    /// Add a middleware to the response pipeline.
    ///
    /// This middleware runs after a response is produced and can modify the response before it is sent to the client.
    /// 
    /// It does not affect request execution flow.
    pub fn after<T, Fut>(mut self, callback: T) -> Self
    where 
        T: Fn(HttpResponse) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HttpResponse> + Send + 'static,
    {
        self.after.push(Arc::new(move |response| Box::pin(callback(response))));
        self
    }

    /// Registers a global error handler for the HTTP server.
    /// 
    /// The handler receives an error string and return `HttpResponse` to allow full control over how errors are translated into a response.
    /// 
    /// Only one error handler is supported, and registering multiple will overwrite the previous one.
    pub fn on_error<T, Fut>(mut self, callback: T) -> Self
    where 
        T: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HttpResponse> + Send + 'static,
    {
        self.on_error = Some(Arc::new(move |err| Box::pin(callback(err))));
        self
    }

    pub fn build(self) -> HttpServer {
        HttpServer {
            config: self.config,
            router: self.router,
            before: self.before,
            on_error: self.on_error,
            after: self.after
        }
    }
}