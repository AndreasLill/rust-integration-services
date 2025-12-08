use url::Url;

#[derive(Debug, Clone)]
pub struct S3ClientConfig {
    endpoint: Option<Url>,
    region: String,
    access_key: String,
    secret_key: String,
}

impl S3ClientConfig {
    pub fn builder() -> S3ClientConfigBuilder {
        S3ClientConfigBuilder {
            endpoint: None,
            region: None,
            access_key: None,
            secret_key: None,
        }
    }

    pub fn endpoint(&self) -> Option<&Url> {
        self.endpoint.as_ref()
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn access_key(&self) -> &str {
        &self.access_key
    }

    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }
}

pub struct S3ClientConfigBuilder {
    endpoint: Option<String>,
    region: Option<String>,
    access_key: Option<String>,
    secret_key: Option<String>,
}

impl S3ClientConfigBuilder {
    pub fn build(self) -> anyhow::Result<S3ClientConfig> {
        let endpoint_url = match self.endpoint {
            Some(url) => Some(Url::parse(&url)?),
            None => None,
        };

        Ok(S3ClientConfig {
            endpoint: endpoint_url,
            region: self.region.unwrap_or(String::from("auto")),
            access_key: self.access_key.unwrap_or(String::from("")),
            secret_key: self.secret_key.unwrap_or(String::from("")),
        })
    }

    /// **Optional**
    /// 
    /// Not required for AWS, but should be set for other S3 solutions.
    pub fn endpoint(mut self, endpoint: impl AsRef<str>) -> Self {
        self.endpoint = Some(endpoint.as_ref().to_owned());
        self
    }

    /// **Optional**
    /// 
    /// Default: `auto`
    pub fn region(mut self, region: impl AsRef<str>) -> Self {
        self.region = Some(region.as_ref().to_owned());
        self
    }

    /// **Optional**
    /// 
    /// Default: empty string
    pub fn access_key(mut self, access_key: impl AsRef<str>) -> Self {
        self.access_key = Some(access_key.as_ref().to_owned());
        self
    }

    /// **Optional**
    /// 
    /// Default: empty string
    pub fn secret_key(mut self, secret_key: impl AsRef<str>) -> Self {
        self.secret_key = Some(secret_key.as_ref().to_owned());
        self
    }
}