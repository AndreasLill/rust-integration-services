#[derive(Debug, Clone)]
pub struct TrackingInfo {
    pub uuid: Option<String>,
    pub ip: Option<String>,
}

impl TrackingInfo {
    pub fn new() -> Self {
        TrackingInfo {
            uuid: None,
            ip: None,
        }
    }

    pub fn uuid(mut self, uuid: String) -> Self {
        self.uuid = Some(uuid);
        self
    }

    pub fn ip(mut self, ip: String) -> Self {
        self.ip = Some(ip);
        self
    }
}