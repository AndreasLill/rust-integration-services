use std::collections::HashMap;
use std::marker::PhantomData;

use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http_body_util::{Empty, Full, StreamBody};
use http_body_util::{BodyExt, combinators::BoxBody};
use hyper::body::Frame;
use hyper::{Error, Request, body::Incoming};

pub struct Final;
pub struct SetMethod;

#[derive(Debug)]
pub struct HttpRequest {
    pub body: BoxBody<Bytes, Error>,
    pub parts: hyper::http::request::Parts,
}

impl HttpRequest {
    pub fn builder() -> HttpRequestBuilder<SetMethod>  {
        HttpRequestBuilder {
            body: None,
            method: None,
            headers: HashMap::new(),
            _state: PhantomData
        }
    }

    pub fn from_parts(body: BoxBody<Bytes, Error>, parts: hyper::http::request::Parts) -> HttpRequest {
        HttpRequest {
            body,
            parts
        }
    }

    pub fn get() -> HttpRequest {
        HttpRequest::builder().method("GET").build()
    }

    pub fn post() -> HttpRequest {
        HttpRequest::builder().method("POST").build()
    }
}

pub struct HttpRequestBuilder<State> {
    body: Option<BoxBody<Bytes, Error>>,
    method: Option<String>,
    headers: HashMap<String, String>,
    _state: PhantomData<State>
}

impl HttpRequestBuilder<SetMethod> {

    pub fn method(self, method: impl Into<String>) -> HttpRequestBuilder<Final> {
        HttpRequestBuilder {
            body: self.body,
            method: Some(method.into()),
            headers: self.headers,
            _state: PhantomData
        }
    }
}

impl HttpRequestBuilder<Final> {

    pub fn body_bytes(mut self, body: impl Into<Bytes>) -> Self {
        self.body = Some(
            Full::from(body.into())
            .map_err(|e| match e {})
            .boxed()
        );
        self
    }

    pub fn body_stream<S>(mut self, stream: S) -> Self
    where
        S: Stream<Item = Result<Bytes, hyper::Error>> + Send + Sync + 'static,
    {
        let frame_stream = stream.map_ok(Frame::data);
        let body = StreamBody::new(frame_stream);
        self.body = Some(BodyExt::boxed(body));
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
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

