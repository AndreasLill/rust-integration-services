use std::io::{Error, ErrorKind};

use tokio::{io::AsyncWriteExt, net::TcpStream};
use url::Url;

use super::{http_request::HttpRequest, http_response::HttpResponse};

#[allow(dead_code)]
pub struct HttpClient {
    url: Url,
    request: HttpRequest,
}

impl HttpClient {
    pub fn new(url: Url, request: HttpRequest) -> Self {
        HttpClient { 
            url,
            request
        }
    }

    pub async fn send(&mut self) -> Result<HttpResponse, Error> {
        let host = self.url.host_str().unwrap();
        let port = self.url.port_or_known_default().unwrap();
        let addr = format!("{}:{}", host, port);
        
        self.request.headers.insert(String::from("Host"), String::from(host));
        self.request.path = self.url.path().to_string();
        
        match self.url.scheme() {
            "http" => {
                let mut stream = TcpStream::connect(addr).await?;
                stream.write_all(self.request.to_string().as_bytes()).await?;

                let response = HttpResponse::from_stream(&mut stream).await?;
                Ok(response)
            }
            "https" => {
                Err(Error::new(ErrorKind::Other, format!("Scheme '{}' not supported.", self.url.scheme())))
            }
            _ => {
                Err(Error::new(ErrorKind::Other, format!("Scheme '{}' not supported.", self.url.scheme())))
            }
        }
    }
}