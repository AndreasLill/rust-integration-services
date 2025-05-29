use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::task::JoinSet;
use tokio::sync::mpsc;

use super::http_request::HttpRequest;
use super::http_response::HttpResponse;

type RouteCallback = Arc<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync>;
type OnErrorCallback = Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
type OnStartCallback = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
type OnShutdownCallback = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
type OnRequestCallback = Arc<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
type OnResponseCallback = Arc<dyn Fn(HttpResponse) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

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
    routes: HashMap<String, RouteCallback>,
    control_channel_sender: HttpServerControlChannel,
    control_channel_receiver: mpsc::Receiver<HttpServerControl>,
    callback_on_error: OnErrorCallback,
    callback_on_start: OnStartCallback,
    callback_on_shutdown: OnShutdownCallback,
    callback_on_request: OnRequestCallback,
    callback_on_response: OnResponseCallback,
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
            callback_on_error: Arc::new(|_| Box::pin(async {})),
            callback_on_start: Arc::new(|| Box::pin(async {})),
            callback_on_shutdown: Arc::new(|| Box::pin(async {})),
            callback_on_request: Arc::new(|_| Box::pin(async {})),
            callback_on_response: Arc::new(|_| Box::pin(async {})),
        }
    }

    pub fn on_error<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.callback_on_error = Arc::new(move |error| Box::pin(callback(error)));
        self
    }

    pub fn on_start<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.callback_on_start = Arc::new(move || Box::pin(callback()));
        self
    }

    pub fn on_shutdown<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.callback_on_shutdown = Arc::new(move || Box::pin(callback()));
        self
    }

    pub fn on_request<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.callback_on_request = Arc::new(move |request| Box::pin(callback(request)));
        self
    }

    pub fn on_response<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn(HttpResponse) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.callback_on_response = Arc::new(move |response| Box::pin(callback(response)));
        self
    }

    pub fn route<T, Fut>(mut self, method: &str, route: &str, callback: T) -> Self
    where
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HttpResponse> + Send + 'static,
    {
        self.routes.insert(format!("{}|{}", method.to_uppercase(), route), Arc::new(move |request| Box::pin(callback(request))));
        self
    }

    pub fn get_control_channel(&self) -> HttpServerControlChannel {
        self.control_channel_sender.clone()
    }

    pub async fn start(&mut self) {
        let addr = format!("{}:{}", self.ip, self.port);
        let listener = TcpListener::bind(&addr).await.unwrap();
        let mut join_set = JoinSet::new();
        let routes = Arc::new(self.routes.clone());
        (self.callback_on_start)().await;

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
                    let callback_on_error = Arc::clone(&self.callback_on_error);
                    let callback_on_request = Arc::clone(&self.callback_on_request);
                    let callback_on_response = Arc::clone(&self.callback_on_response);

                    join_set.spawn(async move {
                        let request = match HttpRequest::from_stream(&mut stream).await {
                            Ok(req) => req,
                            Err(err) => {
                                callback_on_error(err.to_string()).await;
                                let response = HttpResponse::internal_server_error();
                                stream.write_all(&response.to_bytes()).await.unwrap();
                                callback_on_response(response.clone()).await;
                                return;
                            }
                        };

                        callback_on_request(request.clone()).await;

                        match routes.get(&format!("{}|{}", &request.method, &request.path)) {
                            None => {
                                let response = HttpResponse::not_found();
                                stream.write_all(&response.to_bytes()).await.unwrap();
                                callback_on_response(response.clone()).await;
                            },
                            Some(callback) => {
                                let mut response = callback(request).await;
                                if !response.body.is_empty() {
                                    response.headers.insert(String::from("Content-Length"), response.body.len().to_string());
                                }
                                stream.write_all(&response.to_bytes()).await.unwrap();
                                callback_on_response(response.clone()).await;
                            }
                        }
                    });
                }
            }
        }

        while let Some(_) = join_set.join_next().await {}
        (self.callback_on_shutdown)().await;
    }
}