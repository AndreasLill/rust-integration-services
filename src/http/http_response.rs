
use std::{borrow::Cow, marker::PhantomData};

use anyhow::Error;
use bytes::Bytes;
use futures::StreamExt;
use http_body_util::{BodyExt, Empty, StreamBody, combinators::BoxBody};
use hyper::{Response, body::{Frame, Incoming}};

use crate::common::stream::{ByteStream, IntoByteStream};

pub struct Build;
pub struct Status;

#[derive(Debug)]
pub struct HttpResponse {
    body: BoxBody<Bytes, Error>,
    parts: hyper::http::response::Parts,
}

impl HttpResponse {

    /// Create a new response using builder.
    pub fn builder() -> HttpResponseBuilder<Status> {
        HttpResponseBuilder::new()
    }

    /// Create a new response from hyper parts.
    pub fn from_parts(body: BoxBody<Bytes, Error>, parts: hyper::http::response::Parts) -> HttpResponse {
        HttpResponse {
            body,
            parts
        }
    }

    /// Returns the body.
    pub fn body(self) -> ByteStream {
        let stream = self.body.into_data_stream();
        ByteStream::new(stream)
    }

    /// Returns the status.
    pub fn status(&self) -> u16 {
        self.parts.status.as_u16()
    }

    /// Returns the raw bytes of a header value by key, or `None` if the header is not present.
    pub fn header(&self, key: impl AsRef<str>) -> Option<&[u8]> {
        self.parts.headers.get(key.as_ref()).map(|value| value.as_bytes())
    }

    /// Returns a header value as a string by key, or `None` if the header is not present.
    pub fn header_str(&self, key: impl AsRef<str>) -> Option<Cow<'_, str>> {
        self.parts.headers.get(key.as_ref()).map(|value| value.as_bytes()).map(String::from_utf8_lossy)
    }

    /// Returns an iterator over all headers, yielding each header name and its value as a byte slice.
    pub fn headers(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.parts.headers.iter().map(|(key, value)| {(key.as_str(), value.as_bytes())})
    }
}

pub struct HttpResponseBuilder<State> {
    builder: hyper::http::response::Builder,
    body: Option<ByteStream>,
    _state: PhantomData<State>
}

impl HttpResponseBuilder<Status>  {
    pub fn new() -> HttpResponseBuilder<Status> {
        HttpResponseBuilder {
            builder: Response::builder(),
            body: None,
            _state: PhantomData,
        }
    }

    pub fn status(mut self, status: u16) -> HttpResponseBuilder<Build> {
        self.builder = self.builder.status(status);
        HttpResponseBuilder {
            builder: self.builder,
            body: None,
            _state: PhantomData
        }
    }
}

impl HttpResponseBuilder<Build> {
    /// Set the response body.
    pub fn body(mut self, body: impl IntoByteStream) -> Self {
        self.body = Some(body.into_stream());
        self
    }

    /// Finish the builder and the create the response.
    pub fn build(self) -> anyhow::Result<HttpResponse> {
        let response: Response<BoxBody<Bytes, Error>>;

        if let Some(body) = self.body {
            let mapped_stream = body.inner_stream().map(|res| { res.map(Frame::data) });
            let body = StreamBody::new(mapped_stream);
            let boxed_body: BoxBody<Bytes, Error> = BodyExt::boxed(body);
            response = self.builder.body(boxed_body)?;
        } else {
            let body = Empty::new().map_err(|e| match e {}).boxed();
            response = self.builder.body(body)?;
        }

        Ok(HttpResponse::from(response))
    }

    /// Add a header to the request.
    pub fn header(mut self, key: impl AsRef<str>, value: impl AsRef<[u8]>) -> Self {
        self.builder = self.builder.header(key.as_ref(), value.as_ref());
        self
    }

    /// Copy headers from another request or response.
    pub fn headers<I, K, V>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<[u8]>,
    {
        for (key, value) in headers {
            self.builder = self.builder.header(key.as_ref(), value.as_ref());
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

