use std::marker::PhantomData;

use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http_body_util::{BodyDataStream, Empty, Full, StreamBody};
use http_body_util::{BodyExt, combinators::BoxBody};
use hyper::body::Frame;
use hyper::{Error, Request, body::Incoming};

pub struct Final;
pub struct SetMethod;
pub struct SetUri;

#[derive(Debug)]
pub struct HttpRequest {
    body: BoxBody<Bytes, Error>,
    pub parts: hyper::http::request::Parts,
}

impl HttpRequest {
    pub fn builder() -> HttpRequestBuilder<SetUri>  {
        HttpRequestBuilder {
            builder: Request::builder(),
            body: None,
            _state: PhantomData
        }
    }

    pub fn from_parts(body: BoxBody<Bytes, Error>, parts: hyper::http::request::Parts) -> HttpRequest {
        HttpRequest {
            body,
            parts
        }
    }

    /// Returns the boxed body.
    /// 
    /// Used for moving body between requests/responses.
    ///
    /// **This consumes the HttpRequest**
    pub fn body_as_boxed(self) -> BoxBody<Bytes, Error> {
        self.body
    }

    /// Returns the body data stream.
    ///
    /// **This consumes the HttpRequest**
    pub fn body_as_stream(self) -> BodyDataStream<BoxBody<Bytes, Error>> {
        self.body.into_data_stream()
    }

    /// Returns the body as bytes.
    /// 
    /// **This consumes the HttpRequest**
    pub async fn body_as_bytes(self) -> anyhow::Result<Bytes> {
        Ok(self.body.collect().await?.to_bytes())
    }
}

pub struct HttpRequestBuilder<State> {
    builder: hyper::http::request::Builder,
    body: Option<BoxBody<Bytes, Error>>,
    _state: PhantomData<State>
}

impl HttpRequestBuilder<SetUri> {
    pub fn uri(mut self, uri: impl Into<String>) -> HttpRequestBuilder<SetMethod> {
        self.builder = self.builder.uri(uri.into());
        HttpRequestBuilder {
            builder: self.builder,
            body: self.body,
            _state: PhantomData
        }
    }
}

impl HttpRequestBuilder<SetMethod> {
    pub fn method(mut self, method: impl Into<String>) -> HttpRequestBuilder<Final> {
        self.builder = self.builder.method(method.into().as_str());
        HttpRequestBuilder {
            builder: self.builder,
            body: self.body,
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
        self.builder = self.builder.header(key.into(), value.into());
        self
    }

    pub fn build(self) -> anyhow::Result<HttpRequest> {
        let body = match self.body {
            Some(body) => body,
            None => {
                Empty::new()
                .map_err(|e| match e {})
                .boxed()
            },
        };

        let request = self.builder.body(body)?;
        Ok(HttpRequest::from(request))
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

