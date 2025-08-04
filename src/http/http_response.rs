use std::collections::HashMap;

use crate::http::http_status::HttpStatus;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: HttpStatus,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new() -> Self {
        HttpResponse {
            status: HttpStatus::Ok,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Creates a http response with status 200
    pub fn ok() -> Self {
        HttpResponse::new().status(200)
    }
    
    /// Creates a http response with status 302
    pub fn found() -> Self {
        HttpResponse::new().status(302)
    }
    
    /// Creates a http response with status 400
    pub fn bad_request() -> Self {
        HttpResponse::new().status(400)
    }
    
    /// Creates a http response with status 401
    pub fn unauthorized() -> Self {
        HttpResponse::new().status(401)
    }
    
    /// Creates a http response with status 403
    pub fn forbidden() -> Self {
        HttpResponse::new().status(403)
    }
    
    /// Creates a http response with status 404
    pub fn not_found() -> Self {
        HttpResponse::new().status(404)
    }
    
    /// Creates a http response with status 500
    pub fn internal_server_error() -> Self {
        HttpResponse::new().status(500)
    }

    /// Sets the HTTP response status code.
    pub fn status(mut self, code: u16) -> Self {
        self.status = HttpStatus::from_code(code).expect("Invalid HTTP status code");
        self
    }

    /// Adds or updates a header in the HTTP response.
    pub fn header<T: AsRef<str>>(mut self, key: T, value: T) -> Self {
        self.headers.insert(key.as_ref().to_string(), value.as_ref().to_string());
        self
    }

    /// Sets the HTTP response body and automatically adds a `content-length` header.
    pub fn body(mut self, body: &[u8]) -> Self {
        self.body = body.to_vec();
        self.headers.insert(String::from("content-length"), String::from(body.len().to_string()));
        self
    }
}