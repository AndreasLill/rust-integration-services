use std::pin::Pin;

use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt};

pub struct ByteStream {
    data: Pin<Box<dyn Stream<Item = Result<Bytes, anyhow::Error>> + Send + Sync>>,
}

impl ByteStream {
    pub fn new(data: Pin<Box<dyn Stream<Item = Result<Bytes, anyhow::Error>> + Send + Sync>>) -> Self {
        ByteStream {
            data
        }
    }

    pub fn as_stream(self) -> Pin<Box<dyn Stream<Item = Result<Bytes, anyhow::Error>> + Send + Sync>> {
        self.data
    }

    pub async fn as_bytes(self) -> anyhow::Result<Bytes> {
        let mut stream = self.data;
        let mut buffer = BytesMut::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.extend_from_slice(&chunk);
        }

        Ok(buffer.freeze())
    }
}