use std::{collections::HashMap, io::Error};

use tokio::{io::{AsyncBufReadExt, AsyncReadExt, BufReader}, net::TcpStream};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub protocol: String,
    pub status: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[allow(dead_code)]
impl HttpResponse {
    pub fn ok() -> Self {
        HttpResponse {
            protocol: String::from("HTTP/1.1"),
            status: String::from("200 OK"),
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    pub fn found() -> Self {
        HttpResponse {
            protocol: String::from("HTTP/1.1"),
            status: String::from("302 Found"),
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    pub fn not_found() -> Self {
        HttpResponse {
            protocol: String::from("HTTP/1.1"),
            status: String::from("404 Not Found"),
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    pub fn internal_server_error() -> Self {
        HttpResponse {
            protocol: String::from("HTTP/1.1"),
            status: String::from("500 Internal Server Error"),
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    pub fn body(mut self, body: &str) -> Self {
        self.body = body.to_string();
        self.headers.insert(String::from("Content-Length"), String::from(body.len().to_string()));
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn to_string(&self) -> String {
        let first_line_str = format!("{} {}", self.protocol, self.status);
        let mut headers_str = String::new();

        for (key, value) in &self.headers {
            headers_str.push_str(&format!("{}: {}\r\n", key, value));
        };

        format!("{}\r\n{}\r\n{}", first_line_str, headers_str, self.body)
    }

    pub async fn from_stream(stream: &mut TcpStream) -> Result<HttpResponse, Error> {
        let mut reader = BufReader::new(stream);
        let mut buffer = String::new();

        reader.read_line(&mut buffer).await?;
        let first_line: Vec<&str> = buffer.split_whitespace().collect();
        let protocol = first_line[0];
        let code = first_line[1];
        let reason = first_line[2];

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

        Ok(HttpResponse {
            protocol: String::from(protocol),
            status: String::from(format!("{} {}", code, reason)),
            headers: headers,
            body: body
        })
    }
}