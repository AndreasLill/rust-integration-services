#[derive(Debug, Clone)]
pub struct S3ClientConfig {
    pub host: String,
    pub port: u16,
    pub region: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}

impl S3ClientConfig {
    pub fn builder() -> S3ClientConfigBuilder {
        S3ClientConfigBuilder::new()
    }
}

pub struct S3ClientConfigBuilder {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub region: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}

impl S3ClientConfigBuilder {
    pub fn new() -> Self {
        Self {
            host: None,
            port: None,
            region: None,
            access_key: None,
            secret_key: None,
        }
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    pub fn access_key(mut self, access_key: impl Into<String>) -> Self {
        self.access_key = Some(access_key.into());
        self
    }

    pub fn secret_key(mut self, secret_key: impl Into<String>) -> Self {
        self.secret_key = Some(secret_key.into());
        self
    }

    pub fn build(self) -> anyhow::Result<S3ClientConfig> {
        if self.host.is_none() {
            anyhow::bail!("host is required")
        }
        if self.port.is_none() {
            anyhow::bail!("port is required")
        }

        Ok(S3ClientConfig {
            host: self.host.unwrap(),
            port: self.port.unwrap(),
            region: self.region,
            access_key: self.access_key,
            secret_key: self.secret_key
        })
    }
}