use std::{convert::Infallible, path::Path, pin::Pin, sync::Arc};

use http_body_util::{BodyExt, Full};
use hyper::{body::{Bytes, Incoming}, header::{HeaderName, HeaderValue}, service::service_fn, Request, Response};
use hyper_util::rt::TokioIo;
use matchit::Router;
use rustls::ServerConfig;
use tokio::{net::{TcpListener, TcpStream}, signal::unix::{signal, SignalKind}, sync::mpsc::Sender, task::JoinSet};
use tokio_rustls::TlsAcceptor;
use uuid::Uuid;

use crate::{common::event_handler::EventHandler, http::{crypto::Crypto, http_executor::HttpExecutor, http_method::HttpMethod, http_request::HttpRequest, http_response::HttpResponse}, utils::result::ResultDyn };

type RouteCallback = Arc<dyn Fn(String, HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync>;

#[derive(Clone)]
pub enum HttpReceiverEventSignal {
    OnConnection(String, String),
    OnRequest(String, HttpRequest),
    OnResponse(String, HttpResponse),
    OnError(String, String),
}

pub struct HttpReceiver {
    host: String,
    router: Router<RouteCallback>,
    event_handler: EventHandler<HttpReceiverEventSignal>,
    event_join_set: JoinSet<()>,
    tls_config: Option<ServerConfig>,
}

impl HttpReceiver {
    /// Creates a new `HttpReceiver` instance bound to the specified host address. Example: `127.0.0.1:8080`.
    pub fn new<T: AsRef<str>>(host: T) -> Self {
        HttpReceiver {
            host: host.as_ref().to_string(),
            router: Router::new(),
            event_handler: EventHandler::new(),
            event_join_set: JoinSet::new(),
            tls_config: None,
        }
    }

    /// Enables TLS for incoming connections using the provided server certificate and private key in `.pem` format and
    /// configures the TLS context and sets supported ALPN protocols to allow HTTP/2 and HTTP/1.1.
    pub fn tls<T: AsRef<Path>>(mut self, tls_server_cert_path: T, tls_server_key_path: T) -> Self {
        let mut tls_config = Self::create_tls_config(tls_server_cert_path, tls_server_key_path).expect("Failed to create TLS config");
        tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        self.tls_config = Some(tls_config);
        self
    }

    fn create_tls_config<T: AsRef<Path>>(cert_path: T, key_path: T) -> ResultDyn<ServerConfig> {
        let certs = Crypto::pem_load_certs(cert_path)?;
        let key = Crypto::pem_load_private_key(key_path)?;
        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        Ok(config)
    }

    /// Registers a route with a path, associating it with a handler callback.
    pub fn route<T, Fut, S>(mut self, path: S, callback: T) -> Self
    where
        T: Fn(String, HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HttpResponse> + Send + 'static,
        S: AsRef<str>,
    {
        self.router.insert(path.as_ref(), Arc::new(move |uuid, request| Box::pin(callback(uuid, request)))).expect(&format!("Invalid route path: {}", path.as_ref()));
        self
    }

    /// Registers an asynchronous event handler callback for incoming `HttpReceiverEventSignal`s.
    ///
    /// This sets up a background task that listens for system signals (SIGTERM, SIGINT)
    /// and incoming events from the internal event channel.
    pub fn on_event<T, Fut>(mut self, handler: T) -> Self
    where
        T: Fn(HttpReceiverEventSignal) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.event_join_set = self.event_handler.init(handler);
        self
    }

    async fn incoming_request(req: Request<Incoming>, uuid: String, router: Arc<Router<RouteCallback>>, event_broadcast: Arc<Sender<HttpReceiverEventSignal>>) -> Result<Response<Full<Bytes>>, Infallible> {
        let mut request = Self::build_http_request(req).await;

        match router.at(&request.path) {
            Ok(matched) => {
                request.params = matched.params.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect();
                event_broadcast.send(HttpReceiverEventSignal::OnRequest(uuid.clone(), request.clone())).await.unwrap();
                let callback = matched.value;
                let response = callback(uuid.clone(), request).await;
                let res = Self::build_http_response(response.clone()).await;
                event_broadcast.send(HttpReceiverEventSignal::OnResponse(uuid, response)).await.unwrap();
                Ok(res)
            },
            Err(_) => {
                event_broadcast.send(HttpReceiverEventSignal::OnRequest(uuid.clone(), request.clone())).await.unwrap();
                let response = HttpResponse::not_found();
                let res = Self::build_http_response(response.clone()).await;
                event_broadcast.send(HttpReceiverEventSignal::OnResponse(uuid, response)).await.unwrap();
                Ok(res)
            },
        }
    }

    /// Starts the HTTP server and begins listening for incoming TCP connections (optionally over TLS).
    ///
    /// This method binds to the configured host address and enters a loop to accept new TCP connections.
    /// It also listens for system termination signals (SIGINT, SIGTERM) to gracefully shut down the server.
    pub async fn receive(mut self) {
        let tls_acceptor = self.tls_config.map(|tls_config| {
            TlsAcceptor::from(Arc::new(tls_config))
        });
        let listener = TcpListener::bind(&self.host).await.expect("Failed to start TCP Listener");
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver");
        let mut receiver_join_set = JoinSet::new();
        
        loop {
            tokio::select! {
                _ = sigterm.recv() => break,
                _ = sigint.recv() => break,
                result = listener.accept() => {
                    let (tcp_stream, client_addr) = result.unwrap();
                    let uuid = Uuid::new_v4().to_string();
                    let tls_acceptor = tls_acceptor.clone();
                    let router = Arc::new(self.router.clone());
                    let event_broadcast = Arc::new(self.event_handler.broadcast());
                    
                    event_broadcast.send(HttpReceiverEventSignal::OnConnection(uuid.clone(), client_addr.ip().to_string())).await.unwrap();
                    match tls_acceptor {
                        Some(acceptor) => {
                            receiver_join_set.spawn(Self::tls_connection(acceptor, tcp_stream, uuid, router, event_broadcast));
                        },
                        None => {
                            receiver_join_set.spawn(Self::tcp_connection(tcp_stream, uuid, router, event_broadcast));
                        },
                    }
                }
            }
        }

        while let Some(_) = receiver_join_set.join_next().await {}
        while let Some(_) = self.event_join_set.join_next().await {}
    }

    async fn tcp_connection(tcp_stream: TcpStream, uuid: String, router: Arc<Router<RouteCallback>>, event_broadcast: Arc<Sender<HttpReceiverEventSignal>>) {
        let service = {
            let uuid = uuid.clone();
            let router = router.clone();
            let event_broadcast = event_broadcast.clone();
            service_fn(move |req| {
                Self::incoming_request(req, uuid.clone(), router.clone(), event_broadcast.clone())
            })
        };
        
        let io = TokioIo::new(tcp_stream);
        if let Err(err) = hyper::server::conn::http1::Builder::new().keep_alive(false).serve_connection(io, service).await {
            event_broadcast.send(HttpReceiverEventSignal::OnError(uuid, err.to_string())).await.unwrap();
        }
    }

    async fn tls_connection(tls_acceptor: TlsAcceptor, tcp_stream: TcpStream, uuid: String, router: Arc<Router<RouteCallback>>, event_broadcast: Arc<Sender<HttpReceiverEventSignal>>) {
        let tls_stream = match tls_acceptor.accept(tcp_stream).await {
            Ok(stream) => stream,
            Err(err) => {
                event_broadcast.send(HttpReceiverEventSignal::OnError(uuid, format!("TLS handshake failed: {:?}", err))).await.unwrap();
                return;
            },
        };
        
        let service = {
            let uuid = uuid.clone();
            let router = router.clone();
            let event_broadcast = event_broadcast.clone();
            service_fn(move |req| {
                Self::incoming_request(req, uuid.clone(), router.clone(), event_broadcast.clone())
            })
        };
        
        let io = TokioIo::new(tls_stream);
        let protocol = io.inner().get_ref().1.alpn_protocol();
        match protocol.as_deref() {
            Some(b"h2") => {
                if let Err(err) = hyper::server::conn::http2::Builder::new(HttpExecutor).serve_connection(io, service).await {
                    event_broadcast.send(HttpReceiverEventSignal::OnError(uuid, format!("Connection failed: {:?}", err))).await.unwrap();
                }
            }
            _ => {
                if let Err(err) = hyper::server::conn::http1::Builder::new().keep_alive(false).serve_connection(io, service).await {
                    event_broadcast.send(HttpReceiverEventSignal::OnError(uuid, err.to_string())).await.unwrap();
                }
            }
        }

    }

    async fn build_http_request(req: Request<Incoming>) -> HttpRequest {
        let (parts, body) = req.into_parts();
        let mut request = HttpRequest::new();
        request.method = HttpMethod::from_str(parts.method.as_str()).unwrap();
        request.path = parts.uri.path().to_string();
        for (key, value) in parts.headers {
            if let (Some(key), Ok(value)) = (key, value.to_str()) {
                request.headers.insert(key.to_string(), value.to_string());
            }
        }
        request.body = body.collect().await.unwrap().to_bytes().to_vec();
        request
    }

    async fn build_http_response(res: HttpResponse) -> Response<Full<Bytes>> {
        let mut response: Response<Full<Bytes>> = Response::builder().status(res.status.code()).body(res.body.into()).unwrap();
        for (key, value) in res.headers {
            let header_name = HeaderName::from_bytes(key.as_bytes()).unwrap();
            let header_value = HeaderValue::from_str(&value).unwrap();
            response.headers_mut().insert(header_name, header_value);
        }
        response
    }
}