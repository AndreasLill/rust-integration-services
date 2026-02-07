use crate::http::http_client_version::HttpClientVersion;

pub struct HttpClientConfig {
    pub http_version: HttpClientVersion
}

impl HttpClientConfig {
    pub fn default() -> Self {
        Self {
            http_version: HttpClientVersion::Auto
        }
    }
}