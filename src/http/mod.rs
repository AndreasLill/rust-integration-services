#[cfg(feature = "http")]
pub mod http_request;
#[cfg(feature = "http")]
pub mod http_response;
#[cfg(feature = "http")]
pub mod http_server;
#[cfg(feature = "http")]
pub mod http_client;

#[cfg(feature = "http")]
#[cfg(test)]
mod test {
    use crate::http::{http_client::HttpClient, http_request::HttpRequest, http_response::HttpResponse, http_server::HttpServer};
    use tokio::time::Duration;

    #[tokio::test(start_paused = true)]
    async fn http_server() {
        let mut server = HttpServer::new("127.0.0.1", 7878)
            .route("GET", "/", |_req| async {
                HttpResponse::ok().body("Text")
            });

        let server_control = server.get_control_channel();
        let server_handle = tokio::spawn(async move {
            server.start().await;
        });

        tokio::time::advance(Duration::from_millis(100)).await;
        let response = HttpClient::new("http://127.0.0.1:7878").send(HttpRequest::get()).await.unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, "Text");

        tokio::time::advance(Duration::from_millis(100)).await;
        server_control.shutdown().await;
        server_handle.await.unwrap();
    }
}