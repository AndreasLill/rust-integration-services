use std::{error::Error, pin::Pin};

use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt};

pub struct ByteStream(
    Pin<Box<dyn Stream<Item = Result<Bytes, Box<dyn Error + Send + Sync>>> + Send + Sync>>
);

impl ByteStream {
    pub fn new<S, E>(stream: S) -> Self
    where 
        S: Stream<Item = Result<Bytes, E>> + Send + Sync + 'static,
        E: Into<Box<dyn Error + Send + Sync>> + 'static,
    {
        let stream = stream.map(|res| res.map_err(Into::into));
        Self(Box::pin(stream))
    }

    pub fn as_stream(self) -> Pin<Box<dyn Stream<Item = Result<Bytes, Box<dyn Error + Send + Sync>>> + Send + Sync>> {
        self.0
    }

    pub async fn as_bytes(self) -> anyhow::Result<Bytes> {
        let mut stream = self.0;
        let mut buffer = BytesMut::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| anyhow::anyhow!(e))?;
            buffer.extend_from_slice(&chunk);
        }

        Ok(buffer.freeze())
    }
}