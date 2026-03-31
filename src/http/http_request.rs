use std::collections::HashMap;
use std::marker::PhantomData;

use anyhow::Error;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use http_body_util::{Empty, Full, StreamBody};
use http_body_util::{BodyExt, combinators::BoxBody};
use hyper::HeaderMap;
use hyper::body::Frame;
use hyper::header::HeaderValue;
use hyper::{Request, body::Incoming};

use crate::common::stream::ByteStream;

pub struct Final;
pub struct SetMethod;

#[derive(Debug)]
pub struct HttpRequest {
    body: BoxBody<Bytes, Error>,
    parts: hyper::http::request::Parts,
    params: HashMap<String, String>,
}

impl HttpRequest {

    /// Create a new request using builder.
    pub fn builder() -> HttpRequestBuilder<SetMethod>  {
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
    pub fn body(self) -> ByteStream {
        let stream = self.body.into_data_stream();
        ByteStream::new(stream)
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

impl HttpRequestBuilder<SetMethod> {
    /// Sets the HTTP method to `GET` and assigns the request URI.
    pub fn get(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Final> {
        self.builder = self.builder.method("GET").uri(uri.into());
        HttpRequestBuilder {
            builder: self.builder,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `POST` and assigns the request URI.
    pub fn post(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Final> {
        self.builder = self.builder.method("POST").uri(uri.into());
        HttpRequestBuilder {
            builder: self.builder,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `PUT` and assigns the request URI.
    pub fn put(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Final> {
        self.builder = self.builder.method("PUT").uri(uri.into());
        HttpRequestBuilder {
            builder: self.builder,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `PATCH` and assigns the request URI.
    pub fn patch(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Final> {
        self.builder = self.builder.method("PATCH").uri(uri.into());
        HttpRequestBuilder {
            builder: self.builder,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `DELETE` and assigns the request URI.
    pub fn delete(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Final> {
        self.builder = self.builder.method("DELETE").uri(uri.into());
        HttpRequestBuilder {
            builder: self.builder,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `OPTIONS` and assigns the request URI.
    pub fn options(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Final> {
        self.builder = self.builder.method("OPTIONS").uri(uri.into());
        HttpRequestBuilder {
            builder: self.builder,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `HEAD` and assigns the request URI.
    pub fn head(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Final> {
        self.builder = self.builder.method("OPTIONS").uri(uri.into());
        HttpRequestBuilder {
            builder: self.builder,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `CONNECT` and assigns the request URI.
    pub fn connect(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Final> {
        self.builder = self.builder.method("CONNECT").uri(uri.into());
        HttpRequestBuilder {
            builder: self.builder,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `TRACE` and assigns the request URI.
    pub fn trace(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Final> {
        self.builder = self.builder.method("TRACE").uri(uri.into());
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

    /// Finish the builder and the create the request with a body of bytes in memory.
    pub fn body_bytes(self, body: impl Into<Bytes>) -> anyhow::Result<HttpRequest> {
        let body = Full::from(body.into()).map_err(|e| match e {}).boxed();
        let request = self.builder.body(body)?;
        Ok(HttpRequest::from(request))
    }

    /// Finish the builder and the create the request with a body of bytes as a stream.
    pub fn body_stream(self, stream: impl Stream<Item = Result<Bytes, anyhow::Error>> + Send + Sync + 'static) -> anyhow::Result<HttpRequest> {
        let mapped_stream = stream.map(|res| res.map(Frame::data));
        let body = StreamBody::new(mapped_stream);
        let boxed_body: BoxBody<Bytes, Error> = BodyExt::boxed(body);
        let request: Request<BoxBody<Bytes, Error>> = self.builder.body(boxed_body)?;
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
        let body = body.map_err(|e| anyhow::Error::from(e));
        HttpRequest::from_parts(body.boxed(), parts)
    }
}

