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
    use crate::http::{http_client::HttpClient, http_request::HttpRequest, http_response::HttpResponse, http_server::{HttpServer, HttpServerEventSignal}};
    use tokio::time::Duration;
    use url::Url;

    #[tokio::test(start_paused = true)]
    async fn http_server() {
        let ip = "127.0.0.1";
        let port = 7878;

        let mut server = HttpServer::new(ip, port)
            .route("GET", "/", |_req| async {
                HttpResponse::ok().body("Text")
            });

        let server_control_channel = server.get_control_channel();
        let mut server_event_broadcast = server.get_event_broadcast();

        let event_handle = tokio::spawn(async move {
            while let Ok(event) = server_event_broadcast.recv().await {
                match event {
                    HttpServerEventSignal::OnStart => println!("HTTP server started."),
                    HttpServerEventSignal::OnShutdown => println!("HTTP server shutdown."),
                    HttpServerEventSignal::OnRequest(request) => println!("{:#?}", request),
                    HttpServerEventSignal::OnResponse(response) => println!("{:#?}", response),
                }
            }
        });

        let handle = tokio::spawn(async move {
            server.start().await;
        });

        tokio::time::advance(Duration::from_millis(100)).await;
        let response = HttpClient::new(Url::parse(format!("http://{}:{}", ip, port).as_str()).unwrap()).send(HttpRequest::get()).await.unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, "Text");

        tokio::time::advance(Duration::from_millis(100)).await;
        server_control_channel.shutdown().await;
        event_handle.await.unwrap();
        handle.await.unwrap();
    }
}