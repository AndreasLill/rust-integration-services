use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn new() -> Self {
        HttpRequest {
            method: String::new(),
            path: String::new(),
            version: String::new(),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn get() -> Self {
        Self::new().method("GET").path("/")
    }

    pub fn post() -> Self {
        Self::new().method("POST").path("/")
    }

    pub fn method<T: AsRef<str>>(mut self, method: T) -> Self {
        self.method = method.as_ref().to_string();
        self
    }

    pub fn path<T: AsRef<str>>(mut self, path: T) -> Self {
        self.path = path.as_ref().to_string();
        self
    }

    pub fn body(mut self, body: &[u8]) -> Self {
        self.body = body.to_vec();
        self
    }

    pub fn header<T: AsRef<str>>(mut self, key: T, value: T) -> Self {
        self.headers.insert(key.as_ref().to_string(), value.as_ref().to_string());
        self
    }
}