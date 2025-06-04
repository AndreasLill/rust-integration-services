use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::Error;
use std::io::ErrorKind;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use rustls::pki_types::PrivateKeyDer;
use rustls::ServerConfig;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio::sync::broadcast::Sender;
use tokio::task::JoinSet;
use tokio::sync::broadcast;
use tokio_rustls::TlsAcceptor;

use super::http_request::HttpRequest;
use super::http_response::HttpResponse;

type RouteCallback = Arc<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync>;

#[derive(Clone)]
pub enum HttpReceiverEventSignal {
    OnRequestSuccess(IpAddr, HttpRequest),
    OnRequestError(IpAddr, String),
    OnResponseSuccess(IpAddr, HttpResponse),
    OnResponseError(IpAddr, String),
}

pub struct TlsConfig {
    cert_path: String,
    key_path: String,
}

pub struct HttpReceiver {
    pub ip: String,
    pub port: i32,
    routes: HashMap<String, RouteCallback>,
    event_broadcast: broadcast::Sender<HttpReceiverEventSignal>,
    tls_config: Option<TlsConfig>,
}

impl HttpReceiver {
    pub fn new(ip: &str, port: i32) -> Self {
        let (event_broadcast, _) = broadcast::channel(100);
        HttpReceiver {
            ip: String::from(ip),
            port,
            routes: HashMap::new(),
            event_broadcast,
            tls_config: None,
        }
    }

    pub fn route<T, Fut>(mut self, method: &str, route: &str, callback: T) -> Self
    where
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HttpResponse> + Send + 'static,
    {
        self.routes.insert(format!("{}|{}", method.to_uppercase(), route), Arc::new(move |request| Box::pin(callback(request))));
        self
    }

    pub fn tls(mut self, cert_path: &str, key_path: &str) -> Self {
        self.tls_config = Some(TlsConfig {
            cert_path: cert_path.to_string(),
            key_path: key_path.to_string()
        });
        self
    }

    pub fn get_event_broadcast(&self) -> broadcast::Receiver<HttpReceiverEventSignal> {
        self.event_broadcast.subscribe()
    }

    pub async fn start(self) {
        let addr = format!("{}:{}", self.ip, self.port);
        let listener = TcpListener::bind(&addr).await.unwrap();

        let tls_acceptor = match &self.tls_config {
            Some(tls_cert) => {
                let config = Arc::new(Self::create_tls_config(&tls_cert.cert_path, &tls_cert.key_path).unwrap());
                Some(TlsAcceptor::from(config))
            },
            None => None,
        };

        let routes = Arc::new(self.routes.clone());
        let mut join_set = JoinSet::new();
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        
        loop {
            tokio::select! {
                _ = sigterm.recv() => break,
                _ = sigint.recv() => break,
                result = listener.accept() => {
                    let (stream, client_addr) = result.unwrap();
                    let routes = Arc::clone(&routes);
                    let event_broadcast = Arc::new(self.event_broadcast.clone());
                    let client_addr = Arc::new(client_addr);

                    match tls_acceptor.clone() {
                        Some(tls_acceptor) => join_set.spawn(Self::accept_connection_tls(stream, tls_acceptor, client_addr, routes, event_broadcast)),
                        None => join_set.spawn(Self::accept_connection(stream, client_addr, routes, event_broadcast)),
                    };
                }
            }
        }

        while let Some(_) = join_set.join_next().await {}
    }

    async fn accept_connection<S: AsyncRead + AsyncWrite + Unpin>(mut stream: S, client_addr: Arc<SocketAddr>, routes: Arc<HashMap<String, RouteCallback>>, event_broadcast: Arc<Sender<HttpReceiverEventSignal>>) {
        let request = match HttpRequest::from_stream(&mut stream).await {
            Ok(req) => req,
            Err(err) => {
                event_broadcast.send(HttpReceiverEventSignal::OnRequestError(client_addr.ip(), err.to_string())).ok();
                Self::send_response(stream, client_addr, HttpResponse::internal_server_error(), event_broadcast).await;
                return;
            }
        };

        match routes.get(&format!("{}|{}", &request.method, &request.path)) {
            None => {
                event_broadcast.send(HttpReceiverEventSignal::OnRequestSuccess(client_addr.ip(), request.clone())).ok();
                Self::send_response(stream, client_addr, HttpResponse::not_found(), event_broadcast).await;
            },
            Some(callback) => {
                event_broadcast.send(HttpReceiverEventSignal::OnRequestSuccess(client_addr.ip(), request.clone())).ok();
                let response = callback(request).await;
                Self::send_response(stream, client_addr, response, event_broadcast).await;
            }
        }
    }
    
    async fn accept_connection_tls(mut stream: TcpStream, tls_acceptor: TlsAcceptor, client_addr: Arc<SocketAddr>, routes: Arc<HashMap<String, RouteCallback>>, event_broadcast: Arc<Sender<HttpReceiverEventSignal>>) {
        let mut peek_buffer = [0u8; 8];
        match stream.peek(&mut peek_buffer).await {
            Ok(len) if len >= 3 => {
                // Check for TLS ClientHello Signature.
                let is_tls_client_sig = peek_buffer[0] == 0x16 && peek_buffer[1] == 0x03 && (0x01..=0x03).contains(&peek_buffer[2]);
                if !is_tls_client_sig {
                    event_broadcast.send(HttpReceiverEventSignal::OnRequestError(client_addr.ip(), "Non-TLS request on TLS receiver.".to_string())).ok();
                    Self::send_response(stream, client_addr, HttpResponse::internal_server_error(), event_broadcast).await;
                    return;
                }
            },
            Ok(_) => {
                event_broadcast.send(HttpReceiverEventSignal::OnRequestError(client_addr.ip(), "Could not determine TLS signature.".to_string())).ok();
                Self::send_response(stream, client_addr, HttpResponse::internal_server_error(), event_broadcast).await;
                return;
            },
            Err(err) => {
                event_broadcast.send(HttpReceiverEventSignal::OnRequestError(client_addr.ip(), err.to_string())).ok();
                Self::send_response(stream, client_addr, HttpResponse::internal_server_error(), event_broadcast).await;
                return;
            }
        }

        match tls_acceptor.accept(&mut stream).await {
            Ok(tls_stream) => Self::accept_connection(tls_stream, client_addr, routes, event_broadcast).await,
            Err(err) => {
                let err = format!("TLS handshake failed: {}", err.to_string());
                event_broadcast.send(HttpReceiverEventSignal::OnRequestError(client_addr.ip(), err)).ok();
                Self::send_response(stream, client_addr, HttpResponse::internal_server_error(), event_broadcast).await;
                return;
            }
        };
    }


    async fn send_response<S: AsyncRead + AsyncWrite + Unpin>(mut stream: S, client_addr: Arc<SocketAddr>, mut response: HttpResponse, event_broadcast: Arc<Sender<HttpReceiverEventSignal>>) {
        if !response.body.is_empty() {
            response.headers.insert(String::from("Content-Length"), response.body.len().to_string());
        }
        match stream.write_all(&response.to_bytes()).await {
            Ok(_) => event_broadcast.send(HttpReceiverEventSignal::OnResponseSuccess(client_addr.ip(), response.clone())).ok(),
            Err(err) => event_broadcast.send(HttpReceiverEventSignal::OnResponseError(client_addr.ip(), err.to_string())).ok(),
        };
    }
    
    fn create_tls_config(cert_path: &str, key_path: &str) -> std::io::Result<ServerConfig> {
        let cert_file = File::open(cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid certificate"))?;

        let key_file = File::open(key_path)?;
        let mut key_reader = BufReader::new(key_file);
        let mut keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid private key"))?;

        let key = keys.pop().unwrap();
        let config = ServerConfig::builder()
            .with_no_client_auth() // Adjust if client auth is needed
            .with_single_cert(certs, PrivateKeyDer::Pkcs8(key))
            .map_err(|err| Error::new(ErrorKind::InvalidData, err))?;

        Ok(config)
    }
}