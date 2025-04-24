use url::Url;

use crate::http::http_request::HttpRequest;
use crate::http::http_server::HttpServer;
use crate::http::http_client::HttpClient;

pub struct Integration;

#[allow(dead_code)]
impl Integration {
    pub fn http_server(ip: &str, port: i32) -> HttpServer {
        return HttpServer::new(ip, port);
    }

    pub fn http_client(url: &str, request: HttpRequest) -> HttpClient {
        return HttpClient::new(Url::parse(url).unwrap(), request);
    }
}