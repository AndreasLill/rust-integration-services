use url::Url;

pub struct S3SenderConfig {
    endpoint: Option<Url>,
    bucket: String,
    region: String,
    access_key: String,
    secret_key: String,
}

impl S3SenderConfig {
    pub fn builder() -> S3SenderConfigBuilder {
        S3SenderConfigBuilder {
            endpoint: None,
            bucket: None,
            region: None,
            access_key: None,
            secret_key: None,
        }
    }

    pub fn endpoint(&self) -> Option<&Url> {
        self.endpoint.as_ref()
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
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

pub struct S3SenderConfigBuilder {
    endpoint: Option<String>,
    bucket: Option<String>,
    region: Option<String>,
    access_key: Option<String>,
    secret_key: Option<String>,
}

impl S3SenderConfigBuilder {
    pub fn build(self) -> anyhow::Result<S3SenderConfig> {
        if self.bucket.is_none() {
            return Err(anyhow::anyhow!("bucket is required"));
        }

        let endpoint_url = match self.endpoint {
            Some(url) => Some(Url::parse(&url)?),
            None => None,
        };

        Ok(S3SenderConfig {
            endpoint: endpoint_url,
            bucket: self.bucket.unwrap(),
            region: self.region.unwrap_or(String::from("auto")),
            access_key: self.access_key.unwrap_or(String::from("")),
            secret_key: self.secret_key.unwrap_or(String::from("")),
        })
    }

    /// **Optional**
    /// 
    /// Not required for AWS, but should be set for other S3 solutions.
    pub fn endpoint<S: AsRef<str>>(mut self, endpoint: S) -> Self {
        self.endpoint = Some(endpoint.as_ref().to_owned());
        self
    }

    /// **Required**
    pub fn bucket<S: AsRef<str>>(mut self, bucket: S) -> Self {
        self.bucket = Some(bucket.as_ref().to_owned());
        self
    }

    /// **Optional**
    /// 
    /// Default: `auto`
    pub fn region<S: AsRef<str>>(mut self, region: S) -> Self {
        self.region = Some(region.as_ref().to_owned());
        self
    }

    /// **Optional**
    /// 
    /// Default: empty string
    pub fn access_key<S: AsRef<str>>(mut self, access_key: S) -> Self {
        self.access_key = Some(access_key.as_ref().to_owned());
        self
    }

    /// **Optional**
    /// 
    /// Default: empty string
    pub fn secret_key<S: AsRef<str>>(mut self, secret_key: S) -> Self {
        self.secret_key = Some(secret_key.as_ref().to_owned());
        self
    }
}