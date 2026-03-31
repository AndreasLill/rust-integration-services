use std::pin::Pin;

use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt};

pub struct ByteStream(
    Pin<Box<dyn Stream<Item = Result<Bytes, anyhow::Error>> + Send + Sync>>
);

impl ByteStream {
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Bytes, anyhow::Error>> + Send + Sync + 'static
    {
        Self(Box::pin(stream))
    }

    pub fn as_stream(self) -> Pin<Box<dyn Stream<Item = Result<Bytes, anyhow::Error>> + Send + Sync>> {
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
}