use url::Url;

use crate::s3::s3_auth::S3Auth;

pub struct S3SenderConfig {
    endpoint: Option<Url>,
    bucket: String,
    region: String,
    auth: S3Auth
}

impl S3SenderConfig {
    pub fn builder() -> S3SenderConfigBuilder {
        S3SenderConfigBuilder {
            endpoint: None,
            bucket: None,
            region: None,
            auth: None,
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

    pub fn auth(&self) -> &S3Auth {
        &self.auth
    }
}

pub struct S3SenderConfigBuilder {
    endpoint: Option<String>,
    bucket: Option<String>,
    region: Option<String>,
    auth: Option<S3Auth>
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
            auth: self.auth.unwrap_or(S3Auth::None)
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
    /// Default: `S3Auth::None`
    pub fn auth(mut self, auth: S3Auth) -> Self {
        self.auth = Some(auth);
        self
    }
}