use std::collections::HashMap;

use bytes::Bytes;

use crate::http::http_method::HttpMethod;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub body: Bytes,
}

impl HttpRequest {
    pub fn new() -> Self {
        HttpRequest {
            method: HttpMethod::Get,
            path: String::from("/"),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: Bytes::new(),
        }
    }

    /// Creates a http request with the method `GET`
    pub fn get() -> Self {
        Self::new().with_method("GET")
    }

    /// Creates a http request with the method `POST`
    pub fn post() -> Self {
        Self::new().with_method("POST")
    }

    /// Sets the HTTP request method.
    pub fn with_method<T: AsRef<str>>(mut self, method: T) -> Self {
        self.method = HttpMethod::from_str(method.as_ref()).expect("Invalid HTTP method");
        self
    }

    /// Sets the HTTP request path.
    pub fn with_path<T: AsRef<str>>(mut self, path: T) -> Self {
        self.path = path.as_ref().to_string();
        self
    }

    /// Sets the HTTP request body.
    pub fn with_body(mut self, body: &[u8]) -> Self {
        self.body = Bytes::copy_from_slice(body);
        self
    }

    /// Adds or updates a header in the HTTP request.
    pub fn with_header<T: AsRef<str>>(mut self, key: T, value: T) -> Self {
        self.headers.insert(key.as_ref().to_string(), value.as_ref().to_string());
        self
    }
}