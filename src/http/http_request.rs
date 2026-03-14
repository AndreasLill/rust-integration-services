use std::collections::HashMap;
use std::marker::PhantomData;

use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http_body_util::{BodyDataStream, Empty, Full, StreamBody};
use http_body_util::{BodyExt, combinators::BoxBody};
use hyper::HeaderMap;
use hyper::body::Frame;
use hyper::header::HeaderValue;
use hyper::{Error, Request, body::Incoming};

pub struct Final;
pub struct SetMethod;
pub struct SetUri;

#[derive(Debug)]
pub struct HttpRequest {
    body: BoxBody<Bytes, Error>,
    parts: hyper::http::request::Parts,
    params: HashMap<String, String>,
}

impl HttpRequest {

    /// Create a new request using builder.
    pub fn builder() -> HttpRequestBuilder<SetUri>  {
        HttpRequestBuilder {
            builder: Request::builder(),
            _state: PhantomData
        }
    }

    /// Create a new request from hyper parts.
    pub fn from_parts(body: BoxBody<Bytes, Error>, parts: hyper::http::request::Parts) -> HttpRequest {
        HttpRequest {
            body,
            parts,
            params: HashMap::new()
        }
    }

    /// Create a new request from hyper parts with params.
    pub fn from_parts_with_params(body: BoxBody<Bytes, Error>, parts: hyper::http::request::Parts, params: HashMap<String, String>) -> HttpRequest {
        HttpRequest {
            body,
            parts,
            params
        }
    }

    /// Returns the boxed body.
    /// 
    /// Used for moving body between requests/responses.
    ///
    /// **This consumes the HttpRequest**
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
    /// **This consumes the HttpRequest**
    pub async fn body_as_bytes(self) -> anyhow::Result<Bytes> {
        Ok(self.body.collect().await?.to_bytes())
    }

    /// Returns the method.
    pub fn method(&self) -> &str {
        self.parts.method.as_str()
    }

    /// Returns the uri host.
    pub fn host(&self) -> Option<&str> {
        self.parts.uri.host()
    }

    /// Returns the uri path.
    pub fn path(&self) -> &str {
        self.parts.uri.path()
    }

    /// Returns the uri port.
    pub fn port(&self) -> Option<u16> {
        self.parts.uri.port_u16()
    }

    /// Returns the uri scheme.
    pub fn scheme(&self) -> Option<&str> {
        self.parts.uri.scheme_str()
    }

    /// Returns all headers.
    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.parts.headers
    }

    /// Returns a hashmap with request params.
    pub fn params(&self) -> &HashMap<String, String> {
        &self.params
    }
}

pub struct HttpRequestBuilder<State> {
    builder: hyper::http::request::Builder,
    _state: PhantomData<State>
}

impl HttpRequestBuilder<SetUri> {
    pub fn uri(mut self, uri: impl Into<String>) -> HttpRequestBuilder<SetMethod> {
        self.builder = self.builder.uri(uri.into());
        HttpRequestBuilder {
            builder: self.builder,
            _state: PhantomData
        }
    }
}

impl HttpRequestBuilder<SetMethod> {
    pub fn method(mut self, method: impl Into<String>) -> HttpRequestBuilder<Final> {
        self.builder = self.builder.method(method.into().as_str());
        HttpRequestBuilder {
            builder: self.builder,
            _state: PhantomData
        }
    }
}

impl HttpRequestBuilder<Final> {

    /// Finish the builder and the create the request with an empty body.
    pub fn body_empty(self) -> anyhow::Result<HttpRequest> {
        let body = Empty::new().map_err(|e| match e {}).boxed();
        let request = self.builder.body(body)?;
        Ok(HttpRequest::from(request))
    }

    /// Finish the builder and the create the request with a boxed body.
    /// 
    /// **Used for moving a body from another request or response.**
    pub fn body_boxed(self, body: BoxBody<Bytes, Error>) -> anyhow::Result<HttpRequest> {
        let request = self.builder.body(body)?;
        Ok(HttpRequest::from(request))
    }

    /// Finish the builder and the create the request with a body of bytes in memory.
    pub fn body_bytes(self, body: impl Into<Bytes>) -> anyhow::Result<HttpRequest> {
        let body = Full::from(body.into()).map_err(|e| match e {}).boxed();
        let request = self.builder.body(body)?;
        Ok(HttpRequest::from(request))
    }

    /// Finish the builder and the create the request with a body of bytes as a stream.
    pub fn body_stream<S>(self, stream: S) -> anyhow::Result<HttpRequest>
    where
        S: Stream<Item = Result<Bytes, hyper::Error>> + Send + Sync + 'static,
    {
        let frame_stream = stream.map_ok(Frame::data);
        let body = StreamBody::new(frame_stream);
        let body_ext = BodyExt::boxed(body);
        let request = self.builder.body(body_ext)?;
        Ok(HttpRequest::from(request))
    }

    /// Add a header to the request.
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

