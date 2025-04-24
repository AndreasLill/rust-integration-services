use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use url::Url;

use super::http_client::HttpClient;
use super::http_request::HttpRequest;
use super::http_response::HttpResponse;

type HttpResponseHandler = dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send>> + Send + Sync;

pub struct HttpServer {
    pub ip: String,
    pub port: i32,
    pub routes: HashMap<String, Arc<HttpResponseHandler>>
}

impl HttpServer {
    pub fn new(ip: &str, port: i32) -> Self {
        HttpServer {
            ip: String::from(ip),
            port,
            routes: HashMap::new()
        }
    }

    pub fn route<T, Fut>(mut self, method: &str, route: &str, callback: T) -> Self
    where
        T: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HttpResponse> + Send + 'static,
    {
        self.routes.insert(format!("{}|{}", method, route), Arc::new(move |req| Box::pin(callback(req))));
        self
    }

    pub fn route_proxy(mut self, method: &str, route: &str, url: &str) -> Self {
        let url_parsed = Url::parse(url).unwrap();
        let callback = move |request| {
            let url_cloned = url_parsed.clone();
            async move {
                HttpClient::new(url_cloned, request)
                .send()
                .await
                .unwrap()
            }
        };

        self.routes.insert(format!("{}|{}", method, route), Arc::new(move |req| Box::pin(callback(req))));
        self
    }

    pub async fn start(self) {
        let addr = format!("{}:{}", self.ip, self.port);
        let listener = TcpListener::bind(&addr).await.unwrap();
        println!("HTTP server running on {}", &addr);
        let routes = Arc::new(self.routes.clone());

        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let routes = Arc::clone(&routes);

            tokio::spawn(async move {
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
    
    async fn write_response(stream: &mut TcpStream, mut response: HttpResponse) {
        if response.body.is_empty() {
            stream.write_all(response.to_string().as_bytes()).await.unwrap();
            return;
        }

        response.headers.insert(String::from("Content-Length"), response.body.len().to_string());
        stream.write_all(response.to_string().as_bytes()).await.unwrap();
    }
}