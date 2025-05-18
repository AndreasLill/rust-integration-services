use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use url::Url;

use super::http_client::HttpClient;
use super::http_request::HttpRequest;
use super::http_response::HttpResponse;

type HttpResponseHandler = dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync;

#[derive(Clone)]
pub enum HttpServerControlSignal {
    Shutdown,
}

#[derive(Clone)]
pub enum HttpServerEventSignal {
    OnStart,
    OnShutdown,
    OnRequest(HttpRequest),
    OnResponse(HttpResponse),
}

#[derive(Clone)]
pub struct HttpServerControlChannel {
    control_sender: mpsc::Sender<HttpServerControlSignal>
}

impl HttpServerControlChannel {
    pub async fn shutdown(&self) {
        let _ = self.control_sender.send(HttpServerControlSignal::Shutdown).await;
    }
}

pub struct HttpServer {
    pub ip: String,
    pub port: i32,
    routes: HashMap<String, Arc<HttpResponseHandler>>,
    control_channel_sender: HttpServerControlChannel,
    control_channel_receiver: mpsc::Receiver<HttpServerControlSignal>,
    event_broadcast: broadcast::Sender<HttpServerEventSignal>,
}

impl HttpServer {
    pub fn new(ip: &str, port: i32) -> Self {
        let (control_sender, control_channel_receiver) = mpsc::channel::<HttpServerControlSignal>(16);
        let control_channel_sender = HttpServerControlChannel {
            control_sender,
        };
        let (event_broadcast, _) = broadcast::channel(100);
        HttpServer {
            ip: String::from(ip),
            port,
            routes: HashMap::new(),
            control_channel_sender,
            control_channel_receiver,
            event_broadcast
        }
    }

    pub fn route<T, Fut>(mut self, method: &str, route: &str, callback: T) -> Self
    where
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HttpResponse> + Send + 'static,
    {
        self.routes.insert(format!("{}|{}", method.to_uppercase(), route), Arc::new(move |req| Box::pin(callback(req))));
        self
    }

    pub fn route_proxy(mut self, method: &str, route: &str, url: &str) -> Self {
        let url_parsed = Url::parse(url).unwrap();
        let callback = move |request| {
            let url_cloned = url_parsed.clone();
            async move {
                HttpClient::new(url_cloned)
                .send(request)
                .await
                .unwrap()
            }
        };

        self.routes.insert(format!("{}|{}", method.to_uppercase(), route), Arc::new(move |req| Box::pin(callback(req))));
        self
    }

    pub async fn start(&mut self) {
        let addr = format!("{}:{}", self.ip, self.port);
        let listener = TcpListener::bind(&addr).await.unwrap();
        let _ = self.event_broadcast.send(HttpServerEventSignal::OnStart);

        let routes = Arc::new(self.routes.clone());
        let event_broadcast = Arc::new(self.event_broadcast.clone());
        let mut join_set = JoinSet::new();

        loop {
            tokio::select! {
                event_channel_signal = self.control_channel_receiver.recv() => {
                    match event_channel_signal {
                        Some(HttpServerControlSignal::Shutdown) => break,
                        None => {}
                    }
                }
                result = listener.accept() => {
                    let (mut stream, _) = result.unwrap();
                    let routes = Arc::clone(&routes);
                    let event_broadcast = Arc::clone(&event_broadcast);

                    join_set.spawn(async move {
                        let request = match HttpRequest::from_stream(&mut stream).await {
                            Ok(req) => req,
                            Err(err) => {
                                HttpServer::write_response(&mut stream, HttpResponse::internal_server_error().body(&err.to_string())).await;
                                return;
                            }
                        };

                        let _ = event_broadcast.send(HttpServerEventSignal::OnRequest(request.clone()));
                        match routes.get(&format!("{}|{}", &request.method, &request.path)) {
                            None => {
                                let response = HttpResponse::not_found();
                                HttpServer::write_response(&mut stream, response.clone()).await;
                                println!("Route not found: {} {}", request.method, request.path);
                                let _ = event_broadcast.send(HttpServerEventSignal::OnResponse(response.clone()));
                            },
                            Some(callback) => {
                                let response = callback(request).await;
                                HttpServer::write_response(&mut stream, response.clone()).await;
                                let _ = event_broadcast.send(HttpServerEventSignal::OnResponse(response.clone()));
                            }
                        }
                    });
                }
            }
        }

        while let Some(_) = join_set.join_next().await {}
        let _ = self.event_broadcast.send(HttpServerEventSignal::OnShutdown);
    }

    pub fn get_control_channel(&self) -> HttpServerControlChannel {
        self.control_channel_sender.clone()
    }

    pub fn get_event_broadcast(&self) -> broadcast::Receiver<HttpServerEventSignal> {
        self.event_broadcast.subscribe()
    }
    
    async fn write_response(stream: &mut TcpStream, mut response: HttpResponse) {
        if response.body.is_empty() {
            stream.write_all(response.to_string().as_bytes()).await.unwrap();
            return;
        }

        response.headers.insert(String::from("Content-Length"), response.body.len().to_string());
        stream.write_all(response.to_string().as_bytes()).await.unwrap();
    }
}