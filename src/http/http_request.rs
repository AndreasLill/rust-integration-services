use std::{collections::HashMap, io::Error};

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, BufReader};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub protocol: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub ip: Option<String>,
}

#[allow(dead_code)]
impl HttpRequest {
    pub fn new() -> Self {
        HttpRequest {
            method: String::new(),
            path: String::new(),
            protocol: String::from("HTTP/1.1"),
            headers: HashMap::new(),
            body: String::new(),
            ip: None,
        }
    }

    /// Builds a default GET request equal to:
    /// 
    /// HttpRequest::new().method("GET").path("/")
    pub fn get() -> Self {
        HttpRequest::new().method("GET").path("/")
    }

    /// Builds a default POST request equal to:
    /// 
    /// HttpRequest::new().method("POST").path("/")
    pub fn post() -> Self {
        HttpRequest::new().method("POST").path("/")
    }

    /// Builds a default PUT request equal to:
    /// 
    /// HttpRequest::new().method("PUT").path("/")
    pub fn put() -> Self {
        HttpRequest::new().method("PUT").path("/")
    }

    /// Builds a default DELETE request equal to:
    /// 
    /// HttpRequest::new().method("DELETE").path("/")
    pub fn delete() -> Self {
        HttpRequest::new().method("DELETE").path("/")
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn body(mut self, body: &str) -> Self {
        self.body = body.to_string();
        self.headers.insert("Content-Length".to_string(), body.len().to_string());
        self
    }

    pub fn method(mut self, method: &str) -> Self {
        self.method = method.to_uppercase();
        self
    }

    pub fn path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }

    pub fn ip(mut self, ip: String) -> Self {
        self.ip = Some(ip);
        self
    }

    /// Returns a HTTP string
    pub fn to_string(&self) -> String {
        let first_line_str = format!("{} {} {}", self.method, self.path, self.protocol);
        let mut headers_str = String::new();

        for (key, value) in &self.headers {
            headers_str.push_str(&format!("{}: {}\r\n", key, value));
        };

        format!("{}\r\n{}\r\n{}", first_line_str, headers_str, self.body)
    }

    pub async fn from_stream<S: AsyncRead + AsyncWrite + Unpin>(stream: &mut S) -> Result<HttpRequest, Error> {
        let mut reader = BufReader::new(stream);
        let mut buffer = String::new();
    
        reader.read_line(&mut buffer).await?;
        let first_line: Vec<&str> = buffer.split_whitespace().collect();
        let method = first_line[0];
        let path = first_line[1];
        let protocol = first_line[2];
    
        let mut header = String::new();
        let mut headers: HashMap<String, String> = HashMap::new();
        loop {
            header.clear();
            reader.read_line(&mut header).await?;
    
            if header.trim().is_empty() {
                break;
            }
    
            if let Some((key, value)) = header.trim().split_once(":") {
                headers.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    
        let mut body = String::new();
        if let Some(content_length) = headers.get("Content-Length") {
            let length = content_length.parse().unwrap_or(0);
            let mut body_bytes = vec![0; length];
            reader.read_exact(&mut body_bytes).await?;
            body = String::from_utf8_lossy(&body_bytes).to_string();
        }
    
        Ok(HttpRequest {
            method: String::from(method),
            path: String::from(path),
            protocol: String::from(protocol),
            headers: headers,
            body: body,
            ip: None,
        })
    }
}