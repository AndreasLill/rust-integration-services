use std::collections::HashMap;

use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http_body_util::{Empty, Full, StreamBody};
use http_body_util::{BodyExt, combinators::BoxBody};
use hyper::body::Frame;
use hyper::{Error, Request, body::Incoming};


#[derive(Debug)]
pub struct HttpRequest {
    pub body: BoxBody<Bytes, Error>,
    pub parts: hyper::http::request::Parts,
}

impl HttpRequest {
    pub fn builder() -> HttpRequestBuilder  {
        HttpRequestBuilder {
            body: None,
            method: None,
            headers: HashMap::new(),
        }
    }

    pub fn from_parts(body: BoxBody<Bytes, Error>, parts: hyper::http::request::Parts) -> HttpRequest {
        HttpRequest {
            body,
            parts
        }
    }
}

pub struct HttpRequestBuilder {
    body: Option<BoxBody<Bytes, Error>>,
    method: Option<String>,
    headers: HashMap<String, String>,
}

impl HttpRequestBuilder {

    pub fn body_bytes(mut self, body: impl Into<Bytes>) -> HttpRequestBuilder {
        self.body = Some(
            Full::from(body.into())
            .map_err(|e| match e {})
            .boxed()
        );
        self
    }

    pub fn body_stream<S>(mut self, stream: S) -> HttpRequestBuilder
    where
        S: Stream<Item = Result<Bytes, hyper::Error>> + Send + Sync + 'static,
    {
        let frame_stream = stream.map_ok(Frame::data);
        let body = StreamBody::new(frame_stream);
        self.body = Some(BodyExt::boxed(body));
        self
    }

    pub fn method(mut self, method: impl Into<String>) -> HttpRequestBuilder {
        self.method = Some(method.into());
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> HttpRequestBuilder {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> HttpRequest {
        let mut builder = Request::builder().method(self.method.unwrap_or(String::from("GET")).as_str());
        
        for (key, value) in self.headers.iter() {
            builder = builder.header(key, value);
        }

        let body = match self.body {
            Some(body) => body,
            None => {
                Empty::new()
                .map_err(|e| match e {})
                .boxed()
            },
        };

        let request = builder.body(body).unwrap();

        HttpRequest::from(request)
    }
}

impl From<HttpRequest> for Request<BoxBody<Bytes, Error>> {
    fn from(req: HttpRequest) -> Self {
        Request::from_parts(req.parts, req.body.boxed())
    }
}

impl From<Request<BoxBody<Bytes, Error>>> for HttpRequest {
    fn from(req: Request<BoxBody<Bytes, Error>>) -> Self {
        let (parts, body) = req.into_parts();
        HttpRequest::from_parts(body, parts)
    }
}

impl From<Request<Incoming>> for HttpRequest {
    fn from(req: Request<Incoming>) -> Self {
        let (parts, body) = req.into_parts();
        HttpRequest::from_parts(body.boxed(), parts)
    }
}

