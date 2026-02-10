use std::collections::HashMap;

use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

impl HttpResponse {
    pub fn new(status: u16) -> Self {
        HttpResponse {
            status,
            headers: HashMap::new(),
            body: Bytes::new(),
        }
    }

    pub fn ok() -> Self {
        HttpResponse::new(200)
    }

    pub fn bad_request() -> Self {
        HttpResponse::new(400)
    }

    pub fn unauthorized() -> Self {
        HttpResponse::new(401)
    }

    pub fn not_found() -> Self {
        HttpResponse::new(404)
    }

    pub fn internal_server_error() -> Self {
        HttpResponse::new(500)
    }

    /// Adds or updates a header in the HTTP response.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Sets the HTTP response body and automatically adds a `content-length` header.
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self.headers.insert(String::from("content-length"), String::from(&self.body.len().to_string()));
        self
    }
}