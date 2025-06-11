#[cfg(feature = "http")]
pub mod http_request;
#[cfg(feature = "http")]
pub mod http_response;
#[cfg(feature = "http")]
pub mod http_receiver;
#[cfg(feature = "http")]
pub mod http_sender;

#[cfg(feature = "http")]
#[cfg(test)]
mod test {
    use crate::http::{http_receiver::HttpReceiver, http_request::HttpRequest, http_response::HttpResponse, http_sender::HttpSender};
    use tokio::time::Duration;

    #[tokio::test(start_paused = true)]
    async fn http_receiver_sender() {
        let receiver = HttpReceiver::new("127.0.0.1", 7878)
            .route("GET", "/", |_,_| async {
                HttpResponse::ok().body("Text")
            });

        tokio::spawn(receiver.run());
        tokio::time::advance(Duration::from_millis(100)).await;
        let request = HttpRequest::get();
        let response = HttpSender::new("http://127.0.0.1:7878").send(request).await.unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, "Text");
    }
}