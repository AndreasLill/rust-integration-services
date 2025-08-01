use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new() -> Self {
        HttpResponse {
            status_code: 0,
            status_text: String::new(),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn status<T: AsRef<str>>(mut self, code: u16, text: T) -> Self {
        self.status_code = code;
        self.status_text = text.as_ref().to_string();
        self
    }

    pub fn header<T: AsRef<str>>(mut self, key: T, value: T) -> Self {
        self.headers.insert(key.as_ref().to_string(), value.as_ref().to_string());
        self
    }

    pub fn body(mut self, body: &[u8]) -> Self {
        self.body = body.to_vec();
        self.headers.insert(String::from("Content-Length"), String::from(body.len().to_string()));
        self
    }
}