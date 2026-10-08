#[derive(Clone)]
pub struct SmtpMessage {
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub body: String,
}

impl SmtpMessage {
    pub fn new() -> Self {
        Self {
            from: Vec::new(),
            to: Vec::new(),
            cc: Vec::new(),
            subject: String::new(),
            body: String::new(),
        }
    }

    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from.push(from.into());
        self
    }

    pub fn to(mut self, to: impl Into<String>) -> Self {
        self.to.push(to.into());
        self
    }

    pub fn cc(mut self, cc: impl Into<String>) -> Self {
        self.cc.push(cc.into());
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }
}