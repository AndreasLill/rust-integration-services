use std::collections::HashMap;

use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub body: Bytes,
}

impl HttpRequest {
    pub fn new(method: impl AsRef<str>) -> Self {
        HttpRequest {
            method: method.as_ref().to_string(),
            path: String::from("/"),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: Bytes::new(),
        }
    }

    pub fn get() -> Self {
        HttpRequest::new("GET")
    }

    pub fn post() -> Self {
        HttpRequest::new("POST")
    }

    /// Sets the HTTP request body.
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    /// Adds or updates a header in the HTTP request.
    pub fn header<T: AsRef<str>>(mut self, key: T, value: T) -> Self {
        self.headers.insert(key.as_ref().to_string(), value.as_ref().to_string());
        self
    }
}