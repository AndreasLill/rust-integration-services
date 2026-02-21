use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::Response;

pub struct HttpResponse2 {
    inner: Response<BoxBody<Bytes, Infallible>>,
}

impl HttpResponse2 {
    pub fn ok() -> Self {
        let body = Full::from(Bytes::new()).boxed();
        let res = Response::builder().status(200).body(body).unwrap();
        HttpResponse2::from(res)
    }

    pub fn internal_server_error() -> Self {
        let body = Full::from(Bytes::new()).boxed();
        let res = Response::builder().status(500).body(body).unwrap();
        HttpResponse2::from(res)
    }
}

impl From<HttpResponse2> for Response<BoxBody<Bytes, Infallible>> {
    fn from(res: HttpResponse2) -> Self {
        res.inner
    }
}

impl From<Response<BoxBody<Bytes, Infallible>>> for HttpResponse2 {
    fn from(res: Response<BoxBody<Bytes, Infallible>>) -> Self {
        Self { inner: res }
    }
}
