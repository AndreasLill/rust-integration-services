use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use url::Url;

use super::http_client::HttpClient;
use super::http_request::HttpRequest;
use super::http_response::HttpResponse;

type HttpResponseHandler = dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync;

#[derive(Clone)]
pub enum HttpServerSignal {
    Shutdown,
}

#[derive(Clone)]
pub struct HttpServerEventChannelSender {
    event_sender: mpsc::Sender<HttpServerSignal>
}

impl HttpServerEventChannelSender {
    pub async fn shutdown(&self) {
        let _ = self.event_sender.send(HttpServerSignal::Shutdown).await;
    }
}

pub struct HttpServer {
    pub ip: String,
    pub port: i32,
    routes: HashMap<String, Arc<HttpResponseHandler>>,
    event_channel_sender: HttpServerEventChannelSender,
    event_channel_receiver: mpsc::Receiver<HttpServerSignal>,
}

impl HttpServer {
    pub fn new(ip: &str, port: i32) -> Self {
        let (event_sender, event_channel_receiver) = mpsc::channel::<HttpServerSignal>(16);
        let event_channel_sender = HttpServerEventChannelSender {
            event_sender,
        };
        HttpServer {
            ip: String::from(ip),
            port,
            routes: HashMap::new(),
            event_channel_sender,
            event_channel_receiver,
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
        println!("HTTP server running on {}", &addr);

        let routes = Arc::new(self.routes.clone());
        let mut join_set = JoinSet::new();

        loop {
            tokio::select! {
                event_channel_signal = self.event_channel_receiver.recv() => {
                    match event_channel_signal {
                        Some(HttpServerSignal::Shutdown) => {
                            println!("HTTP server is shutting down.");
                            break;
                        }
                        None => {}
                    }
                }
                result = listener.accept() => {
                    let (mut stream, _) = result.unwrap();
                    let routes = Arc::clone(&routes);

                    join_set.spawn(async move {
                        let request = match HttpRequest::from_stream(&mut stream).await {
                            Ok(req) => req,
                            Err(err) => {
                                HttpServer::write_response(&mut stream, HttpResponse::internal_server_error().body(&err.to_string())).await;
                                return;
                            }
                        };
            
                        match routes.get(&format!("{}|{}", &request.method, &request.path)) {
                            None => {
                                HttpServer::write_response(&mut stream, HttpResponse::not_found()).await;
                                println!("Route not found: {} {}", request.method, request.path);
                            },
                            Some(callback) => {
                                let response = callback(request).await;
                                HttpServer::write_response(&mut stream, response).await;
                            }
                        }
                    });
                }
            }
        }

        println!("HTTP server waiting for ongoing connections to finish...");
        while let Some(_) = join_set.join_next().await {}
        println!("HTTP server shutdown complete.");
    }

    pub fn get_event_channel(&self) -> HttpServerEventChannelSender {
        self.event_channel_sender.clone()
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