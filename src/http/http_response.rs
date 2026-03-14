use std::marker::PhantomData;

use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http_body_util::{BodyDataStream, BodyExt, Empty, Full, StreamBody, combinators::BoxBody};
use hyper::{Error, HeaderMap, Response, body::{Frame, Incoming}, header::HeaderValue};

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
    pub fn body(self) -> BoxBody<Bytes, Error> {
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
    /// **This consumes the HttpResponse**
    pub async fn body_as_bytes(self) -> anyhow::Result<Bytes> {
        Ok(self.body.collect().await?.to_bytes())
    }

    /// Returns the status.
    pub fn status(&self) -> u16 {
        self.parts.status.as_u16()
    }

    /// Returns all headers.
    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.parts.headers
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
    pub fn body_stream<S>(self, stream: S) -> anyhow::Result<HttpResponse>
    where
        S: Stream<Item = Result<Bytes, hyper::Error>> + Send + Sync + 'static,
    {
        let frame_stream = stream.map_ok(Frame::data);
        let body = StreamBody::new(frame_stream);
        let body_ext = BodyExt::boxed(body);
        let response = self.builder.body(body_ext)?;
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
        HttpResponse::from_parts(body.boxed(), parts)
    }
}

