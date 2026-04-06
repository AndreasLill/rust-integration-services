use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct S3ClientConfig {
    pub endpoint: String,
    pub region: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}
impl S3ClientConfig {
    pub fn builder() -> S3ClientConfigBuilder<SetEndpoint> {
        S3ClientConfigBuilder {
            endpoint: None,
            region: None,
            access_key: None,
            secret_key: None,
            _state: PhantomData
        }
    }
}

pub struct SetEndpoint;
pub struct Optional;

pub struct S3ClientConfigBuilder<State> {
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    _state: PhantomData<State>,
}

impl S3ClientConfigBuilder<SetEndpoint> {
    pub fn endpoint(self, endpoint: impl Into<String>) -> S3ClientConfigBuilder<Optional> {
        S3ClientConfigBuilder {
            endpoint: Some(endpoint.into()),
            region: self.region,
            access_key: self.access_key,
            secret_key: self.secret_key,
            _state: PhantomData
        }
    }
}

impl S3ClientConfigBuilder<Optional> {
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
        Ok(S3ClientConfig {
            endpoint: self.endpoint.ok_or_else(|| anyhow::anyhow!("Endpoint not found"))?,
            region: self.region,
            access_key: self.access_key,
            secret_key: self.secret_key
        })
    }
}