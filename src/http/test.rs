use std::{env::home_dir, time::Duration};

use http_body_util::BodyExt;

use crate::http::{client::http_client::HttpClient, http_request::HttpRequest, http_request_2::HttpRequest2, http_response::HttpResponse, http_response_2::HttpResponse2, server::{http_server::HttpServer, http_server_config::HttpServerConfig}};

#[tokio::test(start_paused = true)]
async fn http_server_client() {
    tracing_subscriber::fmt().init();
    tokio::spawn(async move {
        let config = HttpServerConfig::new("127.0.0.1", 8080);
        HttpServer::new(config)
        .route("/", async move |_req| {
            HttpResponse2::ok()
        })
        .receive()
        .await;
    });

    tokio::time::advance(Duration::from_millis(1000)).await;
    let result = HttpClient::default().request(HttpRequest2::builder().build()).send("http://127.0.0.1:8080").await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, 200);
}

/// Create your own certs for testing.
/// 
/// mkcert -install
/// mkcert -cert-file server.pem -key-file server-key.pem localhost 127.0.0.1
#[tokio::test(start_paused = true)]
async fn http_server_client_tls() {
    tracing_subscriber::fmt().init();
    assert!(home_dir().is_some());
    tokio::spawn(async move {
        let server_cert_path = home_dir().unwrap().join("server.pem");
        let server_key_path = home_dir().unwrap().join("server-key.pem");

        let config = HttpServerConfig::new("127.0.0.1", 8080).tls(server_cert_path, server_key_path);
        HttpServer::new(config)
        .route("/", async move |_req| {
            HttpResponse2::ok()
        })
        .receive()
        .await;
    });

    tokio::time::advance(Duration::from_millis(1000)).await;
    let result = HttpClient::default().request(HttpRequest2::builder().build()).send("https://127.0.0.1:8080").await;
    tracing::info!(?result);
    assert!(result.is_ok());
    
    let response = result.unwrap();
    tracing::info!(?response);
    assert_eq!(response.status, 200);
}

#[tokio::test]
async fn http_client() {
    let result = HttpClient::default().request(HttpRequest2::builder().build()).send("http://httpbin.org/get").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn http_client_tls() {
    let result = HttpClient::default().request(HttpRequest2::builder().build()).send("https://httpbin.org/get").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn http_request_2() {
    let request = HttpRequest2::builder().header("key", "value").body_bytes("body").build();
    assert_eq!(request.parts.method.as_str(), "GET");
    assert_eq!(request.parts.headers.get("key").unwrap(), "value");
    let body = request.body.collect().await.unwrap().to_bytes();
    assert_eq!(body, "body");
}

#[tokio::test]
async fn http_request() {
    let request = HttpRequest::get().body("test").header("test", "test");
    assert_eq!(request.method.as_str(), "GET");
    assert_eq!(request.body, "test");
    assert_eq!(request.headers.get("test").unwrap(), "test");
}

#[tokio::test]
async fn http_response() {
    let response = HttpResponse::ok().body("test").header("test", "test");
    assert_eq!(response.status, 200);
    assert_eq!(response.body, "test");
    assert_eq!(response.headers.get("test").unwrap(), "test");
}