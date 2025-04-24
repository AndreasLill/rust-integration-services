pub mod integration;
pub mod http;

#[cfg(test)]
mod tests {
    use crate::{http::{http_request::HttpRequest, http_response::HttpResponse}, integration::Integration};
    use tokio::time::Duration;

    #[tokio::test(start_paused = true)]
    async fn http_server() {
        let handle = tokio::spawn(async move {
            Integration::http_server("127.0.0.1", 7878)
            .route("GET", "/", |_req| async {
                HttpResponse::ok().body("Text")
            })
            .start()
            .await;
        });
        tokio::time::advance(Duration::from_millis(100)).await;

        let response = Integration::http_client("http://127.0.0.1:7878", HttpRequest::get()).send().await.unwrap();
        assert_eq!(response.status, "200 OK");
        assert_eq!(response.body, "Text");
        handle.abort();
    }
}