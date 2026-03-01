use std::marker::PhantomData;

use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http_body_util::{BodyDataStream, BodyExt, Empty, Full, StreamBody, combinators::BoxBody};
use hyper::{Error, Response, body::{Frame, Incoming}};

pub struct Final;
pub struct SetStatus;

#[derive(Debug)]
pub struct HttpResponse {
    body: BoxBody<Bytes, Error>,
    pub parts: hyper::http::response::Parts,
}

impl HttpResponse {
    pub fn builder() -> HttpResponseBuilder<SetStatus>  {
        HttpResponseBuilder {
            builder: Response::builder(),
            body: None,
            _state: PhantomData
        }
    }

    pub fn from_parts(body: BoxBody<Bytes, Error>, parts: hyper::http::response::Parts) -> HttpResponse {
        HttpResponse {
            body,
            parts
        }
    }

    pub fn ok() -> Self {
        let res = HttpResponse::builder().status(200).build().unwrap();
        HttpResponse::from(res)
    }

    pub fn internal_server_error() -> Self {
        let res = HttpResponse::builder().status(500).build().unwrap();
        HttpResponse::from(res)
    }

    /// Returns the boxed body.
    /// 
    /// Used for moving body between requests/responses.
    ///
    /// **This consumes the HttpResponse**
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
    /// **This consumes the HttpResponse**
    pub async fn body_as_bytes(self) -> anyhow::Result<Bytes> {
        Ok(self.body.collect().await?.to_bytes())
    }
}

pub struct HttpResponseBuilder<State> {
    builder: hyper::http::response::Builder,
    body: Option<BoxBody<Bytes, Error>>,
    _state: PhantomData<State>
}

impl HttpResponseBuilder<SetStatus>  {
    pub fn status(mut self, status: u16) -> HttpResponseBuilder<Final> {
        self.builder = self.builder.status(status);
        HttpResponseBuilder {
            builder: self.builder,
            body: self.body,
            _state: PhantomData
        }
    }
}

impl HttpResponseBuilder<Final> {

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

    pub fn build(self) -> anyhow::Result<HttpResponse> {
        let body = match self.body {
            Some(body) => body,
            None => {
                Empty::new()
                .map_err(|e| match e {})
                .boxed()
            },
        };

        let request = self.builder.body(body)?;
        Ok(HttpResponse::from(request))
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

