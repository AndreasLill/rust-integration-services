use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::sync::mpsc;

use super::http_request::HttpRequest;
use super::http_response::HttpResponse;

type RouteCallback = dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync;
type OnStartCallback = dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;
type OnShutdownCallback = dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;
type OnRequestCallback = dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;
type OnResponseCallback = dyn Fn(HttpResponse) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;

#[derive(Clone)]
pub enum HttpServerControl {
    Shutdown,
}

#[derive(Clone)]
pub struct HttpServerControlChannel {
    control_sender: mpsc::Sender<HttpServerControl>
}

impl HttpServerControlChannel {
    pub async fn shutdown(&self) {
        let _ = self.control_sender.send(HttpServerControl::Shutdown).await;
    }
}

pub struct HttpServer {
    pub ip: String,
    pub port: i32,
    routes: HashMap<String, Arc<RouteCallback>>,
    control_channel_sender: HttpServerControlChannel,
    control_channel_receiver: mpsc::Receiver<HttpServerControl>,
    on_start: Arc<OnStartCallback>,
    on_shutdown: Arc<OnShutdownCallback>,
    on_request: Arc<OnRequestCallback>,
    on_response: Arc<OnResponseCallback>,
}

impl HttpServer {
    pub fn new(ip: &str, port: i32) -> Self {
        let (control_sender, control_channel_receiver) = mpsc::channel::<HttpServerControl>(16);
        HttpServer {
            ip: String::from(ip),
            port,
            routes: HashMap::new(),
            control_channel_sender: HttpServerControlChannel { control_sender },
            control_channel_receiver,
            on_start: Arc::new(|| Box::pin(async {})),
            on_shutdown: Arc::new(|| Box::pin(async {})),
            on_request: Arc::new(|_| Box::pin(async {})),
            on_response: Arc::new(|_| Box::pin(async {})),
        }
    }

    pub fn on_start<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_start = Arc::new(move || Box::pin(callback()));
        self
    }

    pub fn on_shutdown<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_shutdown = Arc::new(move || Box::pin(callback()));
        self
    }

    pub fn on_request<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_request = Arc::new(move |request| Box::pin(callback(request)));
        self
    }

    pub fn on_response<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn(HttpResponse) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_response = Arc::new(move |response| Box::pin(callback(response)));
        self
    }

    pub fn route<T, Fut>(mut self, method: &str, route: &str, callback: T) -> Self
    where
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HttpResponse> + Send + 'static,
    {
        self.routes.insert(format!("{}|{}", method.to_uppercase(), route), Arc::new(move |req| Box::pin(callback(req))));
        self
    }

    pub async fn start(&mut self) {
        let addr = format!("{}:{}", self.ip, self.port);
        let listener = TcpListener::bind(&addr).await.unwrap();

        let callback_on_start = Arc::clone(&self.on_start);
        tokio::spawn(async move {
            callback_on_start().await;
        });

        let routes = Arc::new(self.routes.clone());
        let mut join_set = JoinSet::new();

        loop {
            tokio::select! {
                event_channel_signal = self.control_channel_receiver.recv() => {
                    match event_channel_signal {
                        Some(HttpServerControl::Shutdown) => break,
                        None => {}
                    }
                }
                result = listener.accept() => {
                    let (mut stream, _) = result.unwrap();
                    let routes = Arc::clone(&routes);
                    let callback_on_request = Arc::clone(&self.on_request);
                    let callback_on_response = Arc::clone(&self.on_response);

                    join_set.spawn(async move {
                        let request = match HttpRequest::from_stream(&mut stream).await {
                            Ok(req) => req,
                            Err(err) => {
                                HttpServer::write_response(&mut stream, HttpResponse::internal_server_error().body(&err.to_string())).await;
                                return;
                            }
                        };

                        let request_clone = request.clone();
                        tokio::spawn(async move {
                            callback_on_request(request_clone).await;
                        });

                        match routes.get(&format!("{}|{}", &request.method, &request.path)) {
                            None => {
                                let response = HttpResponse::not_found();
                                HttpServer::write_response(&mut stream, response.clone()).await;

                                let response_clone = response.clone();
                                tokio::spawn(async move {
                                    callback_on_response(response_clone).await;
                                });
                            },
                            Some(callback) => {
                                let response = callback(request).await;
                                HttpServer::write_response(&mut stream, response.clone()).await;

                                let response_clone = response.clone();
                                tokio::spawn(async move {
                                    callback_on_response(response_clone).await;
                                });
                            }
                        }
                    });
                }
            }
        }

        while let Some(_) = join_set.join_next().await {}

        let callback_on_shutdown = Arc::clone(&self.on_shutdown);
        tokio::spawn(async move {
            callback_on_shutdown().await;
        });
    }

    pub fn get_control_channel(&self) -> HttpServerControlChannel {
        self.control_channel_sender.clone()
    }
    
    async fn write_response(stream: &mut TcpStream, mut response: HttpResponse) {
        if !response.body.is_empty() {
            response.headers.insert(String::from("Content-Length"), response.body.len().to_string());
        }
        stream.write_all(response.to_string().as_bytes()).await.unwrap();
    }
}