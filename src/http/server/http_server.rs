use std::{convert::Infallible, pin::Pin, sync::Arc};

use http_body_util::{BodyExt, combinators::BoxBody};
use hyper::{Request, Response, body::{Bytes, Incoming}, service::service_fn};
use hyper_util::rt::TokioIo;
use matchit::Router;
use tokio::{net::{TcpListener, TcpStream}, signal::unix::{signal, SignalKind}};
use tokio_rustls::TlsAcceptor;

use crate::http::{executor::Executor, http_request::HttpRequest, http_response::HttpResponse, server::{http_server_builder::{Before, HttpServerBuilder}, http_server_config::HttpServerConfig}};

pub(crate) type HttpServerError = anyhow::Error;
pub(crate) type RouteCallback = Arc<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpServerError>> + Send>> + Send + Sync>;
pub(crate) type RequestMiddlewareCallback = Arc<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = Result<HttpRequest, HttpResponse>> + Send>> + Send + Sync>;
pub(crate) type ResponseMiddlewareCallback = Arc<dyn Fn(HttpResponse) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpServerError>> + Send>> + Send + Sync>;

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

    pub(crate) fn new(config: HttpServerConfig, router: Router<RouteCallback>, request_middleware: Vec<RequestMiddlewareCallback>, response_middleware: Vec<ResponseMiddlewareCallback>) -> Self {
        Self {
            config,
            router,
            request_middleware,
            response_middleware
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
                // Matched route with path
                let params: Vec<(String, String)> = matched.params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
                let body = body.map_err(anyhow::Error::from);
                let request = HttpRequest::from_parts_with_params(body.boxed(), parts, params);
                let response = Self::route_matched(request, request_middleware, response_middleware, matched.value).await;

                Ok(Response::from(response))
            },
            Err(_) => {
                // Error: path not found 404
                let body = body.map_err(anyhow::Error::from);
                let request = HttpRequest::from_parts(body.boxed(), parts);
                let response = Self::route_not_found(request, request_middleware, response_middleware).await;

                Ok(Response::from(response))
            },
        }
    }

    async fn route_matched(mut request: HttpRequest, request_middleware: Arc<[RequestMiddlewareCallback]>, response_middleware: Arc<[ResponseMiddlewareCallback]>, callback: &RouteCallback) -> HttpResponse {
        // Request middlewares
        request = match Self::run_request_middleware(request, request_middleware.clone(), response_middleware.clone()).await {
            Ok(req) => req,
            Err(res) => {
                return res
            },
        };

        let response = callback(request).await;

        // Route callback response
        match response {
            Ok(mut response) => {
                // Response middlewares before route response
                response = match Self::run_response_middleware(response, response_middleware).await {
                    Ok(res) => res,
                    Err(err) => {
                        tracing::trace!(?err, "Error in response middleware");
                        return HttpResponse::builder().status(500).build().unwrap()
                    },
                };

                // Everything went OK
                response
            },
            Err(err) => {
                tracing::trace!(?err, "Error in route response");
                let mut response = HttpResponse::builder().status(500).build().unwrap();

                // Response middlewares before route error response
                response = match Self::run_response_middleware(response, response_middleware).await {
                    Ok(res) => res,
                    Err(err) => {
                        tracing::trace!(?err, "Error in response middleware");
                        return HttpResponse::builder().status(500).build().unwrap()
                    },
                };

                // Route error response
                response
            },
        }
    }

    async fn route_not_found(request: HttpRequest, request_middleware: Arc<[RequestMiddlewareCallback]>, response_middleware: Arc<[ResponseMiddlewareCallback]>) -> HttpResponse {
        // Request middlewares, the returned request is not used, but useful for logging
        match Self::run_request_middleware(request, request_middleware.clone(), response_middleware.clone()).await {
            Ok(req) => req,
            Err(res) => {
                return res
            },
        };

        // Return 404 if route was not found.
        let mut response = HttpResponse::builder().status(404).build().unwrap();

        // Response middlewares before route response on 404 path
        response = match Self::run_response_middleware(response, response_middleware).await {
            Ok(res) => res,
            Err(err) => {
                tracing::trace!(?err, "Error in response middleware on 404 path");
                return HttpResponse::builder().status(500).build().unwrap()
            },
        };

        response
    }

    async fn run_request_middleware(mut request: HttpRequest, request_middleware: Arc<[RequestMiddlewareCallback]>, response_middleware: Arc<[ResponseMiddlewareCallback]>) -> Result<HttpRequest, HttpResponse> {
        for handler in request_middleware.iter() {
            match handler(request).await {
                Ok(req) => request = req,
                Err(mut response) => {
                    // Response middlewares in case of error in request middlewares
                    response = match Self::run_response_middleware(response, response_middleware).await {
                        Ok(res) => res,
                        Err(err) => {
                            tracing::trace!(?err, "Error in response middleware inside request middleware error response");
                            return Err(HttpResponse::builder().status(500).build().unwrap())
                        },
                    };

                    return Err(response)
                },
            }
        }

        Ok(request)
    }

    async fn run_response_middleware(mut response: HttpResponse, response_middleware: Arc<[ResponseMiddlewareCallback]>) -> Result<HttpResponse, HttpServerError> {
        for handler in response_middleware.iter() {
            match handler(response).await {
                Ok(res) => response = res,
                Err(err) => return Err(err),
            }
        }

        Ok(response)
    }
}
