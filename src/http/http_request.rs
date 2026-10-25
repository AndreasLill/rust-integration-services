use std::borrow::Cow;
use std::marker::PhantomData;

use anyhow::Error;
use bytes::Bytes;
use futures::StreamExt;
use http_body_util::{Empty, StreamBody};
use http_body_util::{BodyExt, combinators::BoxBody};
use hyper::Uri;
use hyper::body::Frame;
use hyper::{Request, body::Incoming};

use crate::common::stream::{ByteStream, IntoByteStream};

pub struct Build;
pub struct Method;

#[derive(Debug)]
pub struct HttpRequest {
    body: BoxBody<Bytes, Error>,
    parts: hyper::http::request::Parts,
    params: Vec<(String, String)>,
}

impl HttpRequest {

    /// Create a new request using builder.
    pub fn builder() -> HttpRequestBuilder<Method>  {
        HttpRequestBuilder::new()
    }

    /// Create a new request from hyper parts.
    pub fn from_parts(body: BoxBody<Bytes, Error>, parts: hyper::http::request::Parts) -> HttpRequest {
        HttpRequest {
            body,
            parts,
            params: Vec::new()
        }
    }

    /// Create a new request from hyper parts with params.
    pub fn from_parts_with_params(body: BoxBody<Bytes, Error>, parts: hyper::http::request::Parts, params: Vec<(String, String)>) -> HttpRequest {
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

    // Returns a param by key.
    pub fn param(&self, key: impl AsRef<str>) -> Option<&str> {
        self.params.iter().find(|(k, _)| k == key.as_ref()).map(|(_, v)| v.as_str())
    }

    /// Returns an iterator with request params.
    pub fn params(&self) -> impl Iterator<Item = (&str, &str)>  {
        self.params.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

pub struct HttpRequestBuilder<State> {
    builder: hyper::http::request::Builder,
    uri_string: Option<String>,
    body: Option<ByteStream>,
    _state: PhantomData<State>
}

impl HttpRequestBuilder<Method> {

    pub fn new() -> HttpRequestBuilder<Method> {
        HttpRequestBuilder {
            builder: Request::builder(),
            uri_string: None,
            body: None,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `GET` and assigns the request URI.
    pub fn get(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Build> {
        self.builder = self.builder.method("GET");
        HttpRequestBuilder {
            builder: self.builder,
            uri_string: Some(uri.into()),
            body: None,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `POST` and assigns the request URI.
    pub fn post(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Build> {
        self.builder = self.builder.method("POST");
        HttpRequestBuilder {
            builder: self.builder,
            uri_string: Some(uri.into()),
            body: None,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `PUT` and assigns the request URI.
    pub fn put(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Build> {
        self.builder = self.builder.method("PUT");
        HttpRequestBuilder {
            builder: self.builder,
            uri_string: Some(uri.into()),
            body: None,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `PATCH` and assigns the request URI.
    pub fn patch(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Build> {
        self.builder = self.builder.method("PATCH");
        HttpRequestBuilder {
            builder: self.builder,
            uri_string: Some(uri.into()),
            body: None,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `DELETE` and assigns the request URI.
    pub fn delete(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Build> {
        self.builder = self.builder.method("DELETE");
        HttpRequestBuilder {
            builder: self.builder,
            uri_string: Some(uri.into()),
            body: None,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `OPTIONS` and assigns the request URI.
    pub fn options(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Build> {
        self.builder = self.builder.method("OPTIONS");
        HttpRequestBuilder {
            builder: self.builder,
            uri_string: Some(uri.into()),
            body: None,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `HEAD` and assigns the request URI.
    pub fn head(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Build> {
        self.builder = self.builder.method("OPTIONS");
        HttpRequestBuilder {
            builder: self.builder,
            uri_string: Some(uri.into()),
            body: None,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `CONNECT` and assigns the request URI.
    pub fn connect(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Build> {
        self.builder = self.builder.method("CONNECT");
        HttpRequestBuilder {
            builder: self.builder,
            uri_string: Some(uri.into()),
            body: None,
            _state: PhantomData
        }
    }

    /// Sets the HTTP method to `TRACE` and assigns the request URI.
    pub fn trace(mut self, uri: impl Into<String>) -> HttpRequestBuilder<Build> {
        self.builder = self.builder.method("TRACE");
        HttpRequestBuilder {
            builder: self.builder,
            uri_string: Some(uri.into()),
            body: None,
            _state: PhantomData
        }
    }
}

impl HttpRequestBuilder<Build> {

    pub fn body(mut self, body: impl IntoByteStream) -> Self {
        self.body = Some(body.into_stream());
        self
    }

    pub fn build(self) -> anyhow::Result<HttpRequest> {
        let request: Request<BoxBody<Bytes, Error>>;

        if let Some(body) = self.body {
            let uri: Uri = self.uri_string.unwrap().parse()?;
            let mapped_stream = body.inner_stream().map(|res| { res.map(Frame::data) });
            let body = StreamBody::new(mapped_stream);
            let boxed_body: BoxBody<Bytes, anyhow::Error> = BodyExt::boxed(body);
            request = self.builder.uri(&uri).header("Host", uri.host().unwrap()).body(boxed_body)?;
        } else {
            let uri: Uri = self.uri_string.unwrap().parse()?;
            let body = Empty::new().map_err(|e| match e {}).boxed();
            request = self.builder.uri(&uri).header("Host", uri.host().unwrap()).body(body)?;
        }

        Ok(HttpRequest::from(request))
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
        let body = body.map_err(anyhow::Error::from);
        HttpRequest::from_parts(body.boxed(), parts)
    }
}

