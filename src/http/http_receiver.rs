use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
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
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_rustls::TlsAcceptor;
use uuid::Uuid;

use crate::utils::error::Error;

use super::http_request::HttpRequest;
use super::http_response::HttpResponse;

pub trait AsyncStream: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin> AsyncStream for T {}

type RouteCallback = Arc<dyn Fn(String, HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync>;

#[derive(Clone)]
pub enum HttpReceiverEventSignal {
    OnConnectionReceived(String, String),
    OnRequestSuccess(String, HttpRequest),
    OnRequestError(String, String),
    OnResponseSuccess(String, HttpResponse),
    OnResponseError(String, String),
}

pub struct TlsConfig {
    cert_path: PathBuf,
    key_path: PathBuf,
}

pub struct HttpReceiver {
    host: String,
    routes: HashMap<String, RouteCallback>,
    event_broadcast: mpsc::Sender<HttpReceiverEventSignal>,
    event_receiver: Option<mpsc::Receiver<HttpReceiverEventSignal>>,
    event_join_set: JoinSet<()>,
    tls_config: Option<TlsConfig>,
}

impl HttpReceiver {
    pub fn new<T: AsRef<str>>(host: T) -> Self {
        let (event_broadcast, event_receiver) = mpsc::channel(128);
        HttpReceiver {
            host: host.as_ref().to_string(),
            routes: HashMap::new(),
            event_broadcast,
            event_receiver: Some(event_receiver),
            event_join_set: JoinSet::new(),
            tls_config: None,
        }
    }

    pub fn route<T, Fut, S>(mut self, method: S, route: S, callback: T) -> Self
    where
        T: Fn(String, HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HttpResponse> + Send + 'static,
        S: AsRef<str>,
    {
        self.routes.insert(format!("{}|{}", method.as_ref().to_uppercase(), route.as_ref()), Arc::new(move |uuid, request| Box::pin(callback(uuid, request))));
        self
    }

    pub fn tls<T: AsRef<Path>>(mut self, cert_path: T, key_path: T) -> Self {
        self.tls_config = Some(TlsConfig {
            cert_path: cert_path.as_ref().to_path_buf(),
            key_path: key_path.as_ref().to_path_buf(),
        });
        self
    }

    pub fn on_event<T, Fut>(mut self, handler: T) -> Self
    where
        T: Fn(HttpReceiverEventSignal) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut receiver = self.event_receiver.unwrap();
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver.");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver.");
        
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

    pub async fn receive(mut self) -> tokio::io::Result<()> {
        let listener = TcpListener::bind(&self.host).await?;
        let tls_acceptor = self.tls_config.as_ref().map(|tls_cert| {
            let config = Arc::new(Self::create_tls_config(&tls_cert.cert_path, &tls_cert.key_path).unwrap());
            TlsAcceptor::from(config)
        });

        let routes = Arc::new(self.routes.clone());
        let mut join_set_main = JoinSet::new();
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;
        
        loop {
            tokio::select! {
                _ = sigterm.recv() => break,
                _ = sigint.recv() => break,
                result = listener.accept() => {
                    let (mut stream, client_addr) = result.unwrap();
                    let routes = Arc::clone(&routes);
                    let event_broadcast = Arc::new(self.event_broadcast.clone());
                    let tls_acceptor = tls_acceptor.clone();
                    let uuid = Uuid::new_v4().to_string();

                    event_broadcast.send(HttpReceiverEventSignal::OnConnectionReceived(uuid.clone(), client_addr.ip().to_string())).await.unwrap();
                    join_set_main.spawn(async move {
                        let mut stream: Box<dyn AsyncStream> = match tls_acceptor {
                            Some(acceptor) => {
                                match Self::is_connection_tls(&stream).await {
                                    Ok(_) => {},
                                    Err(err) => {
                                        event_broadcast.send(HttpReceiverEventSignal::OnRequestError(uuid.clone(), err.to_string())).await.unwrap();
                                        let response = HttpResponse::internal_server_error();
                                        match stream.write_all(&response.to_bytes()).await {
                                            Ok(_) => event_broadcast.send(HttpReceiverEventSignal::OnResponseSuccess(uuid.clone(), response.clone())).await.unwrap(),
                                            Err(err) => event_broadcast.send(HttpReceiverEventSignal::OnResponseError(uuid.clone(), err.to_string())).await.unwrap(),
                                        };
                                        return;
                                    },
                                };

                                match acceptor.accept(&mut stream).await {
                                    Ok(tls_stream) => Box::new(tls_stream),
                                    Err(err) => {
                                        let err = format!("TLS handshake failed: {}", err.to_string());
                                        event_broadcast.send(HttpReceiverEventSignal::OnRequestError(uuid.clone(), err.to_string())).await.unwrap();
                                        let response = HttpResponse::internal_server_error();
                                        match stream.write_all(&response.to_bytes()).await {
                                            Ok(_) => event_broadcast.send(HttpReceiverEventSignal::OnResponseSuccess(uuid.clone(), response.clone())).await.unwrap(),
                                            Err(err) => event_broadcast.send(HttpReceiverEventSignal::OnResponseError(uuid.clone(), err.to_string())).await.unwrap(),
                                        };
                                        return;
                                    },
                                }
                            },
                            None => Box::new(stream),
                        };

                        let request = match HttpRequest::from_stream(&mut stream).await {
                            Ok(request) => request.ip(client_addr.ip().to_string()),
                            Err(err) => {
                                event_broadcast.send(HttpReceiverEventSignal::OnRequestError(uuid.clone(), err.to_string())).await.unwrap();
                                let response = HttpResponse::internal_server_error();
                                match stream.write_all(&response.to_bytes()).await {
                                    Ok(_) => event_broadcast.send(HttpReceiverEventSignal::OnResponseSuccess(uuid.clone(), response.clone())).await.unwrap(),
                                    Err(err) => event_broadcast.send(HttpReceiverEventSignal::OnResponseError(uuid.clone(), err.to_string())).await.unwrap(),
                                };
                                return;
                            }
                        };

                        match routes.get(&format!("{}|{}", &request.method, &request.path)) {
                            None => {
                                event_broadcast.send(HttpReceiverEventSignal::OnRequestSuccess(uuid.clone(), request.clone())).await.unwrap();
                                let response = HttpResponse::not_found();
                                match stream.write_all(&response.to_bytes()).await {
                                    Ok(_) => event_broadcast.send(HttpReceiverEventSignal::OnResponseSuccess(uuid.clone(), response.clone())).await.unwrap(),
                                    Err(err) => event_broadcast.send(HttpReceiverEventSignal::OnResponseError(uuid.clone(), err.to_string())).await.unwrap(),
                                };
                            },
                            Some(callback) => {
                                event_broadcast.send(HttpReceiverEventSignal::OnRequestSuccess(uuid.clone(), request.clone())).await.unwrap();
                                let mut response = callback(uuid.clone(), request).await;
                                if !response.body.is_empty() {
                                    response.headers.insert(String::from("Content-Length"), response.body.len().to_string());
                                }
                                match stream.write_all(&response.to_bytes()).await {
                                    Ok(_) => event_broadcast.send(HttpReceiverEventSignal::OnResponseSuccess(uuid.clone(), response.clone())).await.unwrap(),
                                    Err(err) => event_broadcast.send(HttpReceiverEventSignal::OnResponseError(uuid.clone(), err.to_string())).await.unwrap(),
                                };
                            }
                        }
                    });
                }
            }
        }

        while let Some(_) = join_set_main.join_next().await {}
        while let Some(_) = self.event_join_set.join_next().await {}

        Ok(())
    }

    async fn is_connection_tls(stream: &TcpStream) -> tokio::io::Result<()> {
        let mut peek_buffer = [0u8; 8];
        match stream.peek(&mut peek_buffer).await {
            Ok(len) if len >= 3 => {
                // Check for TLS ClientHello Signature.
                let is_tls_client_sig = peek_buffer[0] == 0x16 && peek_buffer[1] == 0x03 && (0x01..=0x03).contains(&peek_buffer[2]);
                if is_tls_client_sig {
                    return Ok(())
                }
                Err(Error::tokio_io("Non-TLS request on TLS receiver."))
            },
            Ok(_) => Err(Error::tokio_io("Could not determine TLS signature.")),
            Err(err) => Err(err),
        }
    }
    
    fn create_tls_config<T: AsRef<Path>>(cert_path: T, key_path: T) -> std::io::Result<ServerConfig> {
        let cert_file = File::open(cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Error::std_io("Invalid certificate"))?;

        let key_file = File::open(key_path)?;
        let mut key_reader = BufReader::new(key_file);
        let mut keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Error::std_io("Invalid private key"))?;

        let key = keys.pop().unwrap();
        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, PrivateKeyDer::Pkcs8(key))
            .map_err(|err| Error::std_io(err.to_string()))?;

        Ok(config)
    }
}