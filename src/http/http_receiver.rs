use std::{convert::Infallible, path::Path, pin::Pin, sync::Arc};

use http_body_util::{BodyExt, Full};
use hyper::{body::{Bytes, Incoming}, header::{HeaderName, HeaderValue}, service::service_fn, Request, Response};
use hyper_util::rt::TokioIo;
use matchit::Router;
use rustls::ServerConfig;
use tokio::{net::TcpListener, net::TcpStream, signal::unix::{signal, SignalKind}, sync::mpsc::{self, Sender}, task::JoinSet};
use tokio_rustls::TlsAcceptor;
use uuid::Uuid;

use crate::{http::{crypto::Crypto, http_executor::HttpExecutor, http_method::HttpMethod, http_request::HttpRequest, http_response::HttpResponse}, utils::result::ResultDyn };

type RouteCallback = Arc<dyn Fn(String, HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync>;

#[derive(Clone)]
pub enum HttpReceiverEventSignal {
    OnConnectionOpened(String, String),
    OnRequest(String, HttpRequest),
    OnResponse(String, HttpResponse),
    OnConnectionFailed(String, String),
}

pub struct HttpReceiver {
    host: String,
    router: Router<RouteCallback>,
    event_broadcast: mpsc::Sender<HttpReceiverEventSignal>,
    event_receiver: Option<mpsc::Receiver<HttpReceiverEventSignal>>,
    event_join_set: JoinSet<()>,
    tls_config: Option<ServerConfig>,
}

impl HttpReceiver {
    /// Creates a new `HttpReceiver` instance bound to the specified host address. Example: `127.0.0.1:8080`.
    pub fn new<T: AsRef<str>>(host: T) -> Self {
        let (event_broadcast, event_receiver) = mpsc::channel(128);
        HttpReceiver {
            host: host.as_ref().to_string(),
            router: Router::new(),
            event_broadcast,
            event_receiver: Some(event_receiver),
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
        let mut receiver = self.event_receiver.unwrap();
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver");
        
        self.event_join_set.spawn(async move {
            loop {
                tokio::select! {
                    _ = sigterm.recv() => break,
                    _ = sigint.recv() => break,
                    event = receiver.recv() => {
                        match event {
                            Some(event) => handler(event).await,
                            None => break,
                        }
                    }
                }
            }
        });
        
        self.event_receiver = None;
        self
    }

    async fn incoming_request(req: Request<Incoming>, uuid: String, router: Arc<Router<RouteCallback>>, event_broadcast: Arc<Sender<HttpReceiverEventSignal>>) -> Result<Response<Full<Bytes>>, Infallible> {
        let request = Self::build_http_request(req).await;
        event_broadcast.send(HttpReceiverEventSignal::OnRequest(uuid.clone(), request.clone())).await.unwrap();

        match router.at(&request.path) {
            Ok(matched) => {
                let callback = matched.value;
                let response = callback(uuid.clone(), request).await;
                let res = Self::build_http_response(response.clone()).await;
                event_broadcast.send(HttpReceiverEventSignal::OnResponse(uuid, response)).await.unwrap();
                Ok(res)
            },
            Err(_) => {
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
        let mut join_set = JoinSet::new();
        
        loop {
            tokio::select! {
                _ = sigterm.recv() => break,
                _ = sigint.recv() => break,
                result = listener.accept() => {
                    let (tcp_stream, client_addr) = result.unwrap();
                    let uuid = Uuid::new_v4().to_string();
                    let event_broadcast = Arc::new(self.event_broadcast.clone());
                    let tls_acceptor = tls_acceptor.clone();
                    let router = Arc::new(self.router.clone());
                    
                    event_broadcast.send(HttpReceiverEventSignal::OnConnectionOpened(uuid.clone(), client_addr.ip().to_string())).await.unwrap();
                    match tls_acceptor {
                        Some(acceptor) => {
                            join_set.spawn(Self::tls_connection(acceptor, tcp_stream, uuid, router, event_broadcast));
                        },
                        None => {
                            join_set.spawn(Self::tcp_connection(tcp_stream, uuid, router, event_broadcast));
                        },
                    }
                }
            }
        }

        while let Some(_) = join_set.join_next().await {}
        while let Some(_) = self.event_join_set.join_next().await {}
    }

    async fn tcp_connection(tcp_stream: TcpStream, uuid: String, router: Arc<Router<RouteCallback>>, event_broadcast: Arc<Sender<HttpReceiverEventSignal>>) {
        let uuid_clone = uuid.clone();
        let event_broadcast_clone = event_broadcast.clone();
        let io = TokioIo::new(tcp_stream);
        let service = service_fn(move |req| {
            Self::incoming_request(req, uuid_clone.to_owned(), router.clone(), event_broadcast_clone.to_owned())
        });
        
        if let Err(err) = hyper::server::conn::http1::Builder::new().serve_connection(io, service).await {
            event_broadcast.send(HttpReceiverEventSignal::OnConnectionFailed(uuid, err.to_string())).await.unwrap();
        }
    }

    async fn tls_connection(tls_acceptor: TlsAcceptor, tcp_stream: TcpStream, uuid: String, router: Arc<Router<RouteCallback>>, event_broadcast: Arc<Sender<HttpReceiverEventSignal>>) {
        let tls_stream = match tls_acceptor.accept(tcp_stream).await {
            Ok(stream) => stream,
            Err(err) => {
                event_broadcast.send(HttpReceiverEventSignal::OnConnectionFailed(uuid, format!("TLS handshake failed: {:?}", err))).await.unwrap();
                return;
            },
        };
        
        let uuid_clone = uuid.clone();
        let event_broadcast_clone = event_broadcast.clone();
        let service = service_fn(move |req| {
            Self::incoming_request(req, uuid_clone.to_owned(), router.clone(), event_broadcast_clone.to_owned())
        });
        
        let io = TokioIo::new(tls_stream);
        let protocol = io.inner().get_ref().1.alpn_protocol();

        match protocol.as_deref() {
            Some(b"h2") => {
                if let Err(err) = hyper::server::conn::http2::Builder::new(HttpExecutor).serve_connection(io, service).await {
                    event_broadcast.send(HttpReceiverEventSignal::OnConnectionFailed(uuid, format!("Connection failed: {:?}", err))).await.unwrap();
                }
            }
            _ => {
                if let Err(err) = hyper::server::conn::http1::Builder::new().serve_connection(io, service).await {
                    event_broadcast.send(HttpReceiverEventSignal::OnConnectionFailed(uuid, err.to_string())).await.unwrap();
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