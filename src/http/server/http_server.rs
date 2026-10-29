use std::{convert::Infallible, marker::PhantomData, pin::Pin, sync::Arc};

use http_body_util::{BodyExt, combinators::BoxBody};
use hyper::{Request, Response, body::{Bytes, Incoming}, service::service_fn};
use hyper_util::rt::TokioIo;
use matchit::Router;
use tokio::{net::{TcpListener, TcpStream}, signal::unix::{signal, SignalKind}};
use tokio_rustls::TlsAcceptor;

use crate::http::{executor::Executor, http_request::HttpRequest, http_response::HttpResponse, server::http_server_config::HttpServerConfig};

type HttpServerError = anyhow::Error;
type RouteCallback = Arc<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpServerError>> + Send>> + Send + Sync>;
type RequestMiddlewareCallback = Arc<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = Result<HttpRequest, HttpResponse>> + Send>> + Send + Sync>;
type ResponseMiddlewareCallback = Arc<dyn Fn(HttpResponse) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpServerError>> + Send>> + Send + Sync>;

pub struct HttpServer {
    config: HttpServerConfig,
    router: Router<RouteCallback>,
    request_middleware: Vec<RequestMiddlewareCallback>,
    response_middleware: Vec<ResponseMiddlewareCallback>
}

impl HttpServer {
    pub fn builder(config: HttpServerConfig) -> HttpServerBuilder<Before> {
        HttpServerBuilder::new(config)
    }

    /// Run the HTTP server and begins listening for incoming TCP connections (optionally over TLS).
    ///
    /// This method binds to the configured host address and enters a loop to accept new TCP connections.
    /// It also listens for system termination signals (SIGINT, SIGTERM) to gracefully shut down the server.
    pub async fn run(self) {
        let tls_acceptor = self.config.tls_config.map(|tls_config| {
            TlsAcceptor::from(Arc::new(tls_config))
        });

        let host = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&host).await.expect("Failed to start TCP Listener");
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver");
        let router = Arc::new(self.router);
        let request_middleware: Arc<[RequestMiddlewareCallback]> = self.request_middleware.into();
        let response_middleware: Arc<[ResponseMiddlewareCallback]> = self.response_middleware.into();
        
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
                    let request_middleware = request_middleware.clone();
                    let response_middleware = response_middleware.clone();

                    let (tcp_stream, _client_addr) = match result {
                        Ok(pair) => pair,
                        Err(err) => {
                            tracing::error!("{:?}", err);
                            continue;
                        },
                    };

                    match tls_acceptor {
                        Some(acceptor) => {
                            tokio::spawn(Self::tls_connection(acceptor, tcp_stream, router, request_middleware, response_middleware));
                        },
                        None => {
                            tokio::spawn(Self::tcp_connection(tcp_stream, router, request_middleware, response_middleware));
                        },
                    }
                }
            }
        }

        tracing::trace!("Shut down complete");
    }

    async fn tcp_connection(tcp_stream: TcpStream, router: Arc<Router<RouteCallback>>, request_middleware: Arc<[RequestMiddlewareCallback]>, response_middleware: Arc<[ResponseMiddlewareCallback]>) {
        let service = {
            let router = router.clone();
            service_fn(move |req| {
                Self::incoming_request(req, router.clone(), request_middleware.clone(), response_middleware.clone())
            })
        };
        
        let io = TokioIo::new(tcp_stream);
        if let Err(err) = hyper::server::conn::http1::Builder::new().serve_connection(io, service).await {
            tracing::error!("{:?}", err);
        }
    }

    async fn tls_connection(tls_acceptor: TlsAcceptor, tcp_stream: TcpStream, router: Arc<Router<RouteCallback>>, request_middleware: Arc<[RequestMiddlewareCallback]>, response_middleware: Arc<[ResponseMiddlewareCallback]>) {
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
                Self::incoming_request(req, router.clone(), request_middleware.clone(), response_middleware.clone())
            })
        };
        
        let io = TokioIo::new(tls_stream);
        let protocol = io.inner().get_ref().1.alpn_protocol();
        match protocol {
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

    async fn incoming_request(request: Request<Incoming>, router: Arc<Router<RouteCallback>>, request_middleware: Arc<[RequestMiddlewareCallback]>, response_middleware: Arc<[ResponseMiddlewareCallback]>) -> Result<Response<BoxBody<Bytes, anyhow::Error>>, Infallible> {
        let (parts, body) = request.into_parts();
        let path = parts.uri.path().to_owned();
        match router.at(&path) {
            Ok(matched) => {
                let params: Vec<(String, String)> = matched.params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
                let body = body.map_err(anyhow::Error::from);
                let mut req = HttpRequest::from_parts_with_params(body.boxed(), parts, params);

                // Request middlewares
                for handler in request_middleware.iter() {
                    match handler(req).await {
                        Ok(request) => req = request,
                        Err(mut response) => {
                            // Response middlewares
                            for handler in response_middleware.iter() {
                                match handler(response).await {
                                    Ok(res) => response = res,
                                    Err(err) => {
                                        tracing::trace!(?err, "Error in request middleware");
                                        return Ok(Response::from(HttpResponse::builder().status(500).build().unwrap()))
                                    },
                                }
                            }

                            return Ok(Response::from(response))
                        },
                    }
                }

                let callback = matched.value;
                let response = callback(req).await;

                match response {
                    Ok(mut response) => {
                        for handler in response_middleware.iter() {
                                match handler(response).await {
                                    Ok(res) => response = res,
                                    Err(err) => {
                                        tracing::trace!(?err, "Error in response middleware");
                                        return Ok(Response::from(HttpResponse::builder().status(500).build().unwrap()))
                                    },
                                }
                            }

                        Ok(Response::from(response))
                    },
                    Err(err) => {
                        tracing::trace!(?err, "Error in route response");
                        let mut response = HttpResponse::builder().status(500).build().unwrap();

                        for handler in response_middleware.iter() {
                                match handler(response).await {
                                    Ok(res) => response = res,
                                    Err(err) => {
                                        tracing::trace!(?err, "Error in response middleware");
                                        return Ok(Response::from(HttpResponse::builder().status(500).build().unwrap()))
                                    },
                                }
                            }

                        Ok(Response::from(response))
                    },
                }
            },
            Err(_) => {
                let body = body.map_err(anyhow::Error::from);
                let mut req = HttpRequest::from_parts(body.boxed(), parts);

                // Request middlewares
                for handler in request_middleware.iter() {
                    match handler(req).await {
                        Ok(request) => req = request,
                        Err(mut response) => {
                            // Response middlewares
                            for handler in response_middleware.iter() {
                                match handler(response).await {
                                    Ok(res) => response = res,
                                    Err(err) => {
                                        tracing::trace!(?err, "Error in request middleware on 404 path");
                                        return Ok(Response::from(HttpResponse::builder().status(500).build().unwrap()))
                                    },
                                }
                            }

                            return Ok(Response::from(response))
                        },
                    }
                }

                // Return 404 if route was not found.
                let mut response = HttpResponse::builder().status(404).build().unwrap();

                for handler in response_middleware.iter() {
                    match handler(response).await {
                        Ok(res) => response = res,
                        Err(err) => {
                            tracing::trace!(?err, "Error in response middleware on 404 path");
                            return Ok(Response::from(HttpResponse::builder().status(500).build().unwrap()))
                        },
                    }
                }

                Ok(Response::from(response))
            },
        }
    }
}

pub struct Before;
pub struct Route;
pub struct After;

pub struct HttpServerBuilder<State> {
    config: HttpServerConfig,
    router: Router<RouteCallback>,
    request_middleware: Vec<RequestMiddlewareCallback>,
    response_middleware: Vec<ResponseMiddlewareCallback>,
    _state: PhantomData<State>
}

impl HttpServerBuilder<Before> {
    pub fn new(config: HttpServerConfig) -> Self {
        Self {
            config: config,
            router: Router::new(),
            request_middleware: Vec::new(),
            response_middleware: Vec::new(),
            _state: PhantomData
        }
    }

    /// Add a middleware to the request pipeline.
    ///
    /// - Return `HttpRequest` to continue to the next middleware or route handler
    /// - Return `HttpResponse` to short-circuit the pipeline and respond immediately
    ///
    /// Response will still be processed by the response `after` pipeline, if any.
    pub fn request_middleware<T, Fut>(mut self, callback: T) -> Self
    where 
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<HttpRequest, HttpResponse>> + Send + 'static
    {
        let callback = Arc::new(callback);
        self.request_middleware.push(Arc::new(move |request| {
            let callback = Arc::clone(&callback);
            Box::pin(async move {
                callback(request).await
            })
        }));
        self
    }

    /// Registers a route with a path, associating it with a handler callback.
    pub fn route<T, Fut>(mut self, path: impl Into<String>, callback: T) -> HttpServerBuilder<Route>
    where
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<HttpResponse, HttpServerError>> + Send + 'static,
    {
        self.router.insert(path.into(), Arc::new(move |request| Box::pin(callback(request)))).unwrap();
        HttpServerBuilder {
            config: self.config,
            router: self.router,
            request_middleware: self.request_middleware,
            response_middleware: self.response_middleware,
            _state: PhantomData
        }
    }
}

impl HttpServerBuilder<Route> {
    /// Registers a route with a path, associating it with a handler callback.
    pub fn route<T, Fut>(mut self, path: impl Into<String>, callback: T) -> Self
    where
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<HttpResponse, HttpServerError>> + Send + 'static,
    {
        self.router.insert(path.into(), Arc::new(move |request| Box::pin(callback(request)))).unwrap();
        self
    }

    /// Add a middleware to the response pipeline.
    ///
    /// This middleware runs after a response is produced and can modify the response before it is sent to the client.
    /// 
    /// It does not affect request execution flow.
    pub fn response_middleware<T, Fut>(mut self, callback: T) -> HttpServerBuilder<After>
    where 
        T: Fn(HttpResponse) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<HttpResponse, HttpServerError>> + Send + 'static,
    {
        self.response_middleware.push(Arc::new(move |response| Box::pin(callback(response))));
        HttpServerBuilder {
            config: self.config,
            router: self.router,
            request_middleware: self.request_middleware,
            response_middleware: self.response_middleware,
            _state: PhantomData
        }
    }

    pub fn build(self) -> HttpServer {
        HttpServer {
            config: self.config,
            router: self.router,
            request_middleware: self.request_middleware,
            response_middleware: self.response_middleware
        }
    }
}

impl HttpServerBuilder<After> {
    pub fn build(self) -> HttpServer {
        HttpServer {
            config: self.config,
            router: self.router,
            request_middleware: self.request_middleware,
            response_middleware: self.response_middleware
        }
    }
}