use crate::http::{http_request::HttpRequest, http_response::HttpResponse};

#[derive(Clone)]
pub enum HttpReceiverEvent {
    OnConnection(String, String),
    OnRequest(String, HttpRequest),
    OnResponse(String, HttpResponse),
    OnError(String, String),
}