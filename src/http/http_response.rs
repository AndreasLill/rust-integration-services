use std::marker::PhantomData;

use anyhow::Error;
use bytes::Bytes;
use futures::StreamExt;
use http_body_util::{BodyExt, Empty, Full, StreamBody, combinators::BoxBody};
use hyper::{HeaderMap, Response, body::{Frame, Incoming}, header::HeaderValue};

use crate::common::stream::ByteStream;

pub struct Final;
pub struct SetStatus;

#[derive(Debug)]
pub struct HttpResponse {
    body: BoxBody<Bytes, Error>,
    parts: hyper::http::response::Parts,
}

impl HttpResponse {

    /// Create a new response using builder.
    pub fn builder() -> HttpResponseBuilder<SetStatus>  {
        HttpResponseBuilder {
            builder: Response::builder(),
            _state: PhantomData
        }
    }

    /// Create a new response from hyper parts.
    pub fn from_parts(body: BoxBody<Bytes, Error>, parts: hyper::http::response::Parts) -> HttpResponse {
        HttpResponse {
            body,
            parts
        }
    }

    /// Returns the boxed body.
    /// 
    /// Used for moving body between requests/responses.
    ///
    /// **This consumes the HttpResponse**
    pub fn body(self) -> ByteStream {
        let stream = self.body.into_data_stream();
        ByteStream::new(stream)
    }

    /// Returns the status.
    pub fn status(&self) -> u16 {
        self.parts.status.as_u16()
    }

    /// Returns a header by key.
    pub fn header(&self, key: impl AsRef<str>) -> Option<&str> {
        self.parts.headers.get(key.as_ref()).and_then(|v| v.to_str().ok())
    }

    /// Returns all headers.
    pub fn headers(&self) -> impl Iterator<Item = (&str, &str)> {
        self.parts.headers.iter().filter_map(|(k, v)| {
            v.to_str().ok().map(|val| (k.as_str(), val))
        })
    }
}

pub struct HttpResponseBuilder<State> {
    builder: hyper::http::response::Builder,
    _state: PhantomData<State>
}

impl HttpResponseBuilder<SetStatus>  {
    pub fn status(mut self, status: u16) -> HttpResponseBuilder<Final> {
        self.builder = self.builder.status(status);
        HttpResponseBuilder {
            builder: self.builder,
            _state: PhantomData
        }
    }
}

impl HttpResponseBuilder<Final> {

    /// Finish the builder and the create the response with an empty body.
    pub fn body_empty(self) -> anyhow::Result<HttpResponse> {
        let body = Empty::new().map_err(|e| match e {}).boxed();
        let response = self.builder.body(body)?;
        Ok(HttpResponse::from(response))
    }

    /// Finish the builder and the create the response with a boxed body.
    /// 
    /// **Used for moving a body from another request or response.**
    pub fn body_boxed(self, body: BoxBody<Bytes, Error>) -> anyhow::Result<HttpResponse> {
        let response = self.builder.body(body)?;
        Ok(HttpResponse::from(response))
    }

    /// Finish the builder and the create the response with a body of bytes in memory.
    pub fn body_bytes(self, body: impl Into<Bytes>) -> anyhow::Result<HttpResponse> {
        let body = Full::from(body.into()).map_err(|e| match e {}).boxed();
        let response = self.builder.body(body)?;
        Ok(HttpResponse::from(response))
    }

    /// Finish the builder and the create the response with a body of bytes as a stream.
    pub fn body_stream(self, stream: ByteStream) -> anyhow::Result<HttpResponse> {
        let mapped_stream = stream.inner_stream().map(|res| { res.map(Frame::data) });
        let body = StreamBody::new(mapped_stream);
        let boxed_body: BoxBody<Bytes, Error> = BodyExt::boxed(body);
        let response: Response<BoxBody<Bytes, Error>> = self.builder.body(boxed_body)?;
        Ok(HttpResponse::from(response))
    }

    /// Add a header to the response.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.builder = self.builder.header(key.into(), value.into());
        self
    }

    /// Copy headers from another request or response.
    pub fn headers(mut self, headers: &HeaderMap<HeaderValue>) -> Self {
        for (key, value) in headers.iter() {
            self.builder = self.builder.header(key, value);
        }
        self
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
        let body = body.map_err(anyhow::Error::from);
        HttpResponse::from_parts(body.boxed(), parts)
    }
}

