use crate::http::{http_request::HttpRequest, http_response::HttpResponse};

#[derive(Clone)]
pub enum HttpReceiverEvent {
    Connection {
        uuid: String,
        ip: String
    },
    Request {
        uuid: String,
        request: HttpRequest
    },
    Response {
        uuid: String,
        response: HttpResponse
    },
    Error {
        uuid: String,
        error: String
    },
}