#[cfg(feature = "http-hyper")]
pub mod http_sender;
#[cfg(feature = "http-hyper")]
pub mod http_request;
#[cfg(feature = "http-hyper")]
pub mod http_response;

#[cfg(feature = "http-hyper")]
#[cfg(test)]
mod test {
    use crate::http_hyper::{http_request::HttpRequest, http_sender::HttpSender};

    #[tokio::test(start_paused = true)]
    async fn http1_sender() {
        let result = HttpSender::new("https://httpbin.org/get").send(HttpRequest::get()).await;
        assert!(result.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn http1_sender_tls() {
        let result = HttpSender::new("https://httpbin.org/get").send(HttpRequest::get()).await;
        assert!(result.is_ok());
    }
}