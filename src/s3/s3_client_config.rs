#[derive(Debug, Clone)]
pub struct S3ClientConfig {
    pub endpoint: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
}
impl S3ClientConfig {
    pub fn new(endpoint: impl Into<String>) -> Self {
        S3ClientConfig {
            endpoint: endpoint.into(),
            region: String::from("auto"),
            access_key: String::from(""),
            secret_key: String::from("")
        }
    }

    /// **Optional**
    /// 
    /// Default: `auto`
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = region.into();
        self
    }

    /// **Optional**
    /// 
    /// Default: empty string
    pub fn access_key(mut self, access_key: impl Into<String>) -> Self {
        self.access_key = access_key.into();
        self
    }

    /// **Optional**
    /// 
    /// Default: empty string
    pub fn secret_key(mut self, secret_key: impl Into<String>) -> Self {
        self.secret_key = secret_key.into();
        self
    }
}