use std::collections::HashMap;

use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http_body_util::{BodyExt, Empty, Full, StreamBody, combinators::BoxBody};
use hyper::{Error, Response, body::{Frame, Incoming}};

#[derive(Debug)]
pub struct HttpResponse {
    pub body: BoxBody<Bytes, Error>,
    pub parts: hyper::http::response::Parts,
}

impl HttpResponse {
    pub fn builder() -> HttpResponseBuilder  {
        HttpResponseBuilder {
            body: None,
            status: None,
            headers: HashMap::new(),
        }
    }

    pub fn from_parts(body: BoxBody<Bytes, Error>, parts: hyper::http::response::Parts) -> HttpResponse {
        HttpResponse {
            body,
            parts
        }
    }

    pub fn ok() -> Self {
        let body: BoxBody<Bytes, Error> = Some(
            Empty::new()
            .map_err(|e| match e {})
            .boxed()
        ).unwrap();
        let res = Response::builder().status(200).body(body).unwrap();
        HttpResponse::from(res)
    }

    pub fn internal_server_error() -> Self {
        let body: BoxBody<Bytes, Error> = Some(
            Empty::new()
            .map_err(|e| match e {})
            .boxed()
        ).unwrap();
        let res = Response::builder().status(500).body(body).unwrap();
        HttpResponse::from(res)
    }
}

pub struct HttpResponseBuilder {
    body: Option<BoxBody<Bytes, Error>>,
    status: Option<u16>,
    headers: HashMap<String, String>,
}

impl HttpResponseBuilder {

    pub fn body_bytes(mut self, body: impl Into<Bytes>) -> HttpResponseBuilder {
        self.body = Some(
            Full::from(body.into())
            .map_err(|e| match e {})
            .boxed()
        );
        self
    }

    pub fn body_stream<S>(mut self, stream: S) -> HttpResponseBuilder
    where
        S: Stream<Item = Result<Bytes, hyper::Error>> + Send + Sync + 'static,
    {
        let frame_stream = stream.map_ok(Frame::data);
        let body = StreamBody::new(frame_stream);
        self.body = Some(BodyExt::boxed(body));
        self
    }

    pub fn status(mut self, status: u16) -> HttpResponseBuilder {
        self.status = Some(status.into());
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> HttpResponseBuilder {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> HttpResponse {
        let mut builder = Response::builder().status(self.status.unwrap_or(200));
        
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

        HttpResponse::from(request)
    }
}

impl From<HttpResponse> for Response<BoxBody<Bytes, Error>> {
    fn from(res: HttpResponse) -> Self {
        Response::from_parts(res.parts, res.body)
    }
}

impl From<Response<BoxBody<Bytes, Error>>> for HttpResponse {
    fn from(res: Response<BoxBody<Bytes, Error>>) -> Self {
        let (parts, body) = res.into_parts();
        HttpResponse::from_parts(body, parts)
    }
}

impl From<Response<Incoming>> for HttpResponse {
    fn from(req: Response<Incoming>) -> Self {
        let (parts, body) = req.into_parts();
        HttpResponse::from_parts(body.boxed(), parts)
    }
}

