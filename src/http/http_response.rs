use std::{collections::HashMap, io::Error};

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub protocol: String,
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[allow(dead_code)]
impl HttpResponse {
    pub fn new() -> Self {
        HttpResponse {
            protocol: String::from("HTTP/1.1"),
            status_code: 0,
            status_text: String::new(),
            headers: HashMap::new(),
            body: String::new(),
        }
    }
    
    pub fn ok() -> Self {
        HttpResponse::new().status(200, "OK")
    }
    
    pub fn found() -> Self {
        HttpResponse::new().status(302, "Found")
    }
    
    pub fn bad_request() -> Self {
        HttpResponse::new().status(400, "Bad Request")
    }
    
    pub fn unauthorized() -> Self {
        HttpResponse::new().status(401, "Unauthorized")
    }
    
    pub fn forbidden() -> Self {
        HttpResponse::new().status(403, "Forbidden")
    }
    
    pub fn not_found() -> Self {
        HttpResponse::new().status(404, "Not Found")
    }
    
    pub fn internal_server_error() -> Self {
        HttpResponse::new().status(500, "Internal Server Error")
    }

    pub fn status(mut self, code: u16, text: &str) -> Self {
        self.status_code = code;
        self.status_text = text.to_string();
        self
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
        let first_line_str = format!("{} {} {}", self.protocol, self.status_code, self.status_text);
        let mut headers_str = String::new();

        for (key, value) in &self.headers {
            headers_str.push_str(&format!("{}: {}\r\n", key, value));
        };

        format!("{}\r\n{}\r\n{}", first_line_str, headers_str, self.body)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_string().into_bytes()
    }

    pub async fn from_stream<S: AsyncRead + Unpin>(stream: &mut S) -> Result<HttpResponse, Error> {
        let mut reader = BufReader::new(stream);
        let mut buffer = String::new();

        reader.read_line(&mut buffer).await?;
        let first_line: Vec<&str> = buffer.split_whitespace().collect();
        let protocol = first_line[0].to_string();
        let status_code = first_line[1].to_string().parse().unwrap();
        let status_text = first_line[2].to_string();

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
            protocol,
            status_code,
            status_text,
            headers,
            body
        })
    }
}