use std::collections::HashMap;

use crate::http::http_method::HttpMethod;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn new() -> Self {
        HttpRequest {
            method: HttpMethod::Get,
            path: String::from("/"),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Creates a http request with the method `GET`
    pub fn get() -> Self {
        Self::new().method("GET")
    }

    /// Creates a http request with the method `POST`
    pub fn post() -> Self {
        Self::new().method("POST")
    }

    /// Sets the HTTP request method.
    pub fn method<T: AsRef<str>>(mut self, method: T) -> Self {
        self.method = HttpMethod::from_str(method.as_ref()).expect("Invalid HTTP method");
        self
    }

    /// Sets the HTTP request path.
    pub fn path<T: AsRef<str>>(mut self, path: T) -> Self {
        self.path = path.as_ref().to_string();
        self
    }

    /// Sets the HTTP request body.
    pub fn body(mut self, body: &[u8]) -> Self {
        self.body = body.to_vec();
        self
    }

    /// Adds or updates a header in the HTTP request.
    pub fn header<T: AsRef<str>>(mut self, key: T, value: T) -> Self {
        self.headers.insert(key.as_ref().to_string(), value.as_ref().to_string());
        self
    }
}