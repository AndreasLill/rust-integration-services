#[cfg(feature = "http")]
mod crypto;
#[cfg(feature = "http")]
mod http_executor;
#[cfg(feature = "http")]
pub mod http_receiver;
#[cfg(feature = "http")]
pub mod http_sender;
#[cfg(feature = "http")]
pub mod http_request;
#[cfg(feature = "http")]
pub mod http_response;
#[cfg(feature = "http")]
pub mod http_status;
#[cfg(feature = "http")]
pub mod http_method;

#[cfg(feature = "http")]
#[cfg(test)]
mod test {
    use std::{env::home_dir, time::Duration};

    use crate::http::{http_method::HttpMethod, http_receiver::HttpReceiver, http_request::HttpRequest, http_response::HttpResponse, http_sender::HttpSender, http_status::HttpStatus};

    #[tokio::test(start_paused = true)]
    async fn http_receiver() {
        tokio::spawn(async move {
            HttpReceiver::new("127.0.0.1:8080")
            .route("/", async move |_uuid, _request| {
                HttpResponse::ok()
            })
            .receive()
            .await;
        });

        tokio::time::advance(Duration::from_millis(1000)).await;
        let result = HttpSender::new("http://127.0.0.1:8080").send(HttpRequest::get()).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status.code(), 200);
    }

    /// Create your own certs for testing.
    /// 
    /// sudo dnf install mkcert
    /// 
    /// mkcert -install
    /// 
    /// mkcert localhost 127.0.0.1 ::1
    #[tokio::test(start_paused = true)]
    async fn http_receiver_tls() {
        tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
        assert!(home_dir().is_some());
        tokio::spawn(async move {
            let server_cert_path = home_dir().unwrap().join(".config/rust-integration-services/certs/localhost+2.pem");
            let server_key_path = home_dir().unwrap().join(".config/rust-integration-services/certs/localhost+2-key.pem");
            HttpReceiver::new("127.0.0.1:8080")
            .tls(server_cert_path, server_key_path)
            .route("/", async move |_uuid, _request| {
                HttpResponse::ok()
            })
            .receive()
            .await;
        });

        tokio::time::advance(Duration::from_millis(1000)).await;
        let result = HttpSender::new("https://127.0.0.1:8080").send(HttpRequest::get()).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status.code(), 200);
    }

    #[tokio::test]
    async fn http_sender() {
        let result = HttpSender::new("http://httpbin.org/get").send(HttpRequest::get()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn http_sender_tls() {
        let result = HttpSender::new("https://httpbin.org/get").send(HttpRequest::get()).await;
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
}