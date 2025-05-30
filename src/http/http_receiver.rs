use std::collections::HashMap;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::task::JoinSet;
use tokio::sync::mpsc;
use tokio::sync::broadcast;

use super::http_request::HttpRequest;
use super::http_response::HttpResponse;

type RouteCallback = Arc<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync>;

#[derive(Clone)]
pub enum HttpReceiverEventSignal {
    OnStart,
    OnShutdown,
    OnShutdownComplete,
    OnRequest(IpAddr, HttpRequest),
    OnRequestError(IpAddr, String),
    OnResponse(IpAddr, HttpResponse),
    OnResponseError(IpAddr, String),
}

#[derive(Clone)]
pub enum HttpReceiverControl {
    Shutdown,
}

#[derive(Clone)]
pub struct HttpReceiverControlChannel {
    control_sender: mpsc::Sender<HttpReceiverControl>
}

impl HttpReceiverControlChannel {
    pub async fn shutdown(&self) {
        let _ = self.control_sender.send(HttpReceiverControl::Shutdown).await;
    }
}

pub struct HttpReceiver {
    pub ip: String,
    pub port: i32,
    routes: HashMap<String, RouteCallback>,
    control_channel_sender: HttpReceiverControlChannel,
    control_channel_receiver: mpsc::Receiver<HttpReceiverControl>,
    event_broadcast: broadcast::Sender<HttpReceiverEventSignal>,
}

impl HttpReceiver {
    pub fn new(ip: &str, port: i32) -> Self {
        let (control_sender, control_channel_receiver) = mpsc::channel::<HttpReceiverControl>(16);
        let (event_broadcast, _) = broadcast::channel(100);
        HttpReceiver {
            ip: String::from(ip),
            port,
            routes: HashMap::new(),
            control_channel_sender: HttpReceiverControlChannel { control_sender },
            control_channel_receiver,
            event_broadcast,
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

    pub fn get_control_channel(&self) -> HttpReceiverControlChannel {
        self.control_channel_sender.clone()
    }

    pub fn get_event_broadcast(&self) -> broadcast::Receiver<HttpReceiverEventSignal> {
        self.event_broadcast.subscribe()
    }

    pub async fn start(&mut self) {
        let addr = format!("{}:{}", self.ip, self.port);
        let listener = TcpListener::bind(&addr).await.unwrap();
        let mut join_set = JoinSet::new();
        let routes = Arc::new(self.routes.clone());
        self.event_broadcast.send(HttpReceiverEventSignal::OnStart).ok();

        loop {
            tokio::select! {
                event_channel_signal = self.control_channel_receiver.recv() => {
                    match event_channel_signal {
                        Some(HttpReceiverControl::Shutdown) => {
                            self.event_broadcast.send(HttpReceiverEventSignal::OnShutdown).ok();
                            break;
                        },
                        None => {}
                    }
                }
                result = listener.accept() => {
                    let (mut stream, client_addr) = result.unwrap();
                    let routes = Arc::clone(&routes);
                    let event_broadcast = Arc::new(self.event_broadcast.clone());
                    let client_addr = Arc::new(client_addr);

                    join_set.spawn(async move {
                        let request = match HttpRequest::from_stream(&mut stream).await {
                            Ok(req) => req,
                            Err(err) => {
                                event_broadcast.send(HttpReceiverEventSignal::OnRequestError(client_addr.ip(), err.to_string())).ok();
                                let response = HttpResponse::internal_server_error();
                                match stream.write_all(&response.to_bytes()).await {
                                    Ok(_) => {
                                        event_broadcast.send(HttpReceiverEventSignal::OnResponse(client_addr.ip(), response.clone())).ok();
                                    },
                                    Err(err) => {
                                        event_broadcast.send(HttpReceiverEventSignal::OnResponseError(client_addr.ip(), err.to_string())).ok();
                                    },
                                };
                                return;
                            }
                        };

                        match routes.get(&format!("{}|{}", &request.method, &request.path)) {
                            None => {
                                event_broadcast.send(HttpReceiverEventSignal::OnRequest(client_addr.ip(), request.clone())).ok();
                                let response = HttpResponse::not_found();
                                match stream.write_all(&response.to_bytes()).await {
                                    Ok(_) => {
                                        event_broadcast.send(HttpReceiverEventSignal::OnResponse(client_addr.ip(), response.clone())).ok();
                                    },
                                    Err(err) => {
                                        event_broadcast.send(HttpReceiverEventSignal::OnResponseError(client_addr.ip(), err.to_string())).ok();
                                    },
                                };
                            },
                            Some(callback) => {
                                event_broadcast.send(HttpReceiverEventSignal::OnRequest(client_addr.ip(), request.clone())).ok();
                                let mut response = callback(request).await;
                                if !response.body.is_empty() {
                                    response.headers.insert(String::from("Content-Length"), response.body.len().to_string());
                                }
                                match stream.write_all(&response.to_bytes()).await {
                                    Ok(_) => {
                                        event_broadcast.send(HttpReceiverEventSignal::OnResponse(client_addr.ip(), response.clone())).ok();
                                    },
                                    Err(err) => {
                                        event_broadcast.send(HttpReceiverEventSignal::OnResponseError(client_addr.ip(), err.to_string())).ok();
                                    },
                                };
                            }
                        }
                    });
                }
            }
        }

        while let Some(_) = join_set.join_next().await {}
        self.event_broadcast.send(HttpReceiverEventSignal::OnShutdownComplete).ok();
    }
}