#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpClientVersion {
    Auto,
    Http1,
    Http2,
}