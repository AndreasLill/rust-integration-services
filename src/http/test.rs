use std::{env::home_dir, time::Duration};

use crate::http::{client::http_client::HttpClient, http_request::HttpRequest, http_response::HttpResponse, server::{http_server::HttpServer, http_server_config::HttpServerConfig}};

#[tokio::test(start_paused = true)]
async fn http_server_client() {
    tracing_subscriber::fmt().init();
    tokio::spawn(async move {
        let config = HttpServerConfig::new("127.0.0.1", 8080);
        HttpServer::new(config)
        .route("/", async move |_req| {
            HttpResponse::builder().status(200).body_empty().unwrap()
        })
        .run()
        .await;
    });

    tokio::time::advance(Duration::from_millis(1000)).await;
    let request = HttpRequest::builder().get("http://127.0.0.1:8080").body_empty().unwrap();
    let result = HttpClient::new().send(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    tracing::info!(?response);
    assert_eq!(response.status(), 200);
}

/// Create your own certs for testing.
/// 
/// mkcert -install
/// mkcert -cert-file server.pem -key-file server-key.pem localhost 127.0.0.1
#[tokio::test(start_paused = true)]
async fn http_server_client_tls() {
    tracing_subscriber::fmt().init();
    tokio::spawn(async move {
        let server_cert_path = home_dir().unwrap().join("server.pem");
        let server_key_path = home_dir().unwrap().join("server-key.pem");

        let config = HttpServerConfig::new("127.0.0.1", 8080).tls(server_cert_path, server_key_path);
        HttpServer::new(config)
        .route("/", async move |_req| {
            HttpResponse::builder().status(200).body_empty().unwrap()
        })
        .run()
        .await;
    });

    tokio::time::advance(Duration::from_millis(1000)).await;
    let request = HttpRequest::builder().get("https://127.0.0.1:8080").body_empty().unwrap();
    let result = HttpClient::new().send(request).await;
    assert!(result.is_ok());
    
    let response = result.unwrap();
    tracing::info!(?response);
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn http_client() {
    tracing_subscriber::fmt().init();
    let request = HttpRequest::builder().get("http://httpbin.org/get").body_empty().unwrap();
    let result = HttpClient::new().send(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    tracing::info!(?response);
}

#[tokio::test]
async fn http_client_tls() {
    tracing_subscriber::fmt().init();
    let request = HttpRequest::builder().get("https://api.open-meteo.com/v1/forecast?latitude=59.33&longitude=18.07&current_weather=true").body_empty().unwrap();
    let result = HttpClient::new().send(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    tracing::info!(?response);
}

#[tokio::test]
async fn http_request() {
    let request = HttpRequest::builder().get("https://127.0.0.1").header("key", "value").body_bytes("body").unwrap();
    assert_eq!(request.method(), "GET");
    assert_eq!(request.headers().get("key").unwrap(), "value");
    let body = request.body().as_bytes().await.unwrap();
    assert_eq!(body, "body");
}

#[tokio::test]
async fn http_response() {
    let response = HttpResponse::builder().status(200).header("key", "value").body_bytes("body").unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("key").unwrap(), "value");
    let body = response.body().as_bytes().await.unwrap();
    assert_eq!(body, "body");
}