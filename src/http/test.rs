use std::{env::home_dir, time::Duration};
use crate::http::{http_client::HttpClient, http_method::HttpMethod, http_server::HttpServer, http_request::HttpRequest, http_response::HttpResponse, http_status::HttpStatus};

#[tokio::test(start_paused = true)]
async fn http_receiver() {
    tokio::spawn(async move {
        HttpServer::new("127.0.0.1:8080")
        .route("/", async move |_request| {
            HttpResponse::ok()
        })
        .receive()
        .await;
    });

    tokio::time::advance(Duration::from_millis(1000)).await;
    let result = HttpClient::default().request(HttpRequest::get()).send("http://127.0.0.1:8080").await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status.code(), 200);
}

/// Create your own certs for testing.
/// 
/// mkcert -install
/// mkcert localhost 127.0.0.1 ::1
#[tokio::test(start_paused = true)]
async fn http_receiver_tls() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
    assert!(home_dir().is_some());
    tokio::spawn(async move {
        let server_cert_path = home_dir().unwrap().join("localhost+2.pem");
        let server_key_path = home_dir().unwrap().join("localhost+2-key.pem");
        HttpServer::new("127.0.0.1:8080")
        .tls(server_cert_path, server_key_path)
        .route("/", async move |_request| {
            HttpResponse::ok()
        })
        .receive()
        .await;
    });

    tokio::time::advance(Duration::from_millis(1000)).await;
    let result = HttpClient::default().request(HttpRequest::get()).send("https://127.0.0.1:8080").await;
    tracing::info!(?result);
    assert!(result.is_ok());
    
    let response = result.unwrap();
    tracing::info!(?response);
    assert_eq!(response.status.code(), 200);
}

#[tokio::test]
async fn http_sender() {
    let result = HttpClient::default().request(HttpRequest::get()).send("http://httpbin.org/get").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn http_sender_tls() {
    let result = HttpClient::default().request(HttpRequest::get()).send("https://httpbin.org/get").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn http_status() {
    assert_eq!(HttpStatus::Ok.code(), 200);
    assert_eq!(HttpStatus::BadRequest.code(), 400);
    assert_eq!(HttpStatus::NotFound.code(), 404);
    assert_eq!(HttpStatus::InternalServerError.code(), 500);
}

#[tokio::test]
async fn http_method() {
    assert_eq!(HttpMethod::Get.as_str(), "GET");
    assert_eq!(HttpMethod::Post.as_str(), "POST");
    assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
}

#[tokio::test]
async fn http_request() {
    let request = HttpRequest::get().with_body("test".as_bytes()).with_header("test", "test");
    assert_eq!(request.method.as_str(), "GET");
    assert_eq!(request.body, "test".as_bytes());
    assert_eq!(request.headers.get("test").unwrap(), "test");
}

#[tokio::test]
async fn http_response() {
    let response = HttpResponse::ok().with_body(b"test").with_header("test", "test");
    assert_eq!(response.status.code(), 200);
    assert_eq!(response.status.text(), "OK");
    assert_eq!(response.body, "test".as_bytes());
    assert_eq!(response.headers.get("test").unwrap(), "test");
}