use std::pin::Pin;

use anyhow::Error;
use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt};

pub struct ByteStream(
    Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send + Sync>>
);

impl ByteStream {
    pub fn new<S, E>(stream: S) -> Self
    where 
        S: Stream<Item = Result<Bytes, E>> + Send + Sync + 'static,
        E: Into<Error> + 'static,
    {
        let stream = stream.map(|res| res.map_err(Into::into));
        Self(Box::pin(stream))
    }

    pub fn inner_stream(self) -> Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send + Sync>> {
        self.0
    }

    pub async fn as_bytes(self) -> anyhow::Result<Bytes> {
        let mut stream = self.0;
        let mut buffer = BytesMut::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.extend_from_slice(&chunk);
        }

        Ok(buffer.freeze())
    }

    pub async fn next(&mut self) -> Option<anyhow::Result<Bytes>> {
        match self.0.next().await {
            Some(Ok(bytes)) => Some(Ok(bytes)),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

impl From<Bytes> for ByteStream {
    fn from(bytes: Bytes) -> Self {
        let stream = futures::stream::once(async move { 
            Ok::<Bytes, Error>(bytes) 
        });

        Self(Box::pin(stream))
    }
}

impl From<Vec<u8>> for ByteStream {
    fn from(v: Vec<u8>) -> Self {
        Self::from(Bytes::from(v))
    }
}

impl From<String> for ByteStream {
    fn from(s: String) -> Self {
        Self::from(Bytes::from(s))
    }
}

impl From<&str> for ByteStream {
    fn from(s: &str) -> Self {
        Self::from(Bytes::copy_from_slice(s.as_bytes()))
    }
}