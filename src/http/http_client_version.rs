#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpClientVersion {
    Auto,
    ForceHttp1,
    ForceHttp2,
}