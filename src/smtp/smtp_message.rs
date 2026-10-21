#[derive(Clone, Debug)]
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

    pub fn builder() -> SmtpMessageBuilder {
        SmtpMessageBuilder::new()
    }
}

pub struct SmtpMessageBuilder {
    from: Vec<String>,
    to: Vec<String>,
    cc: Vec<String>,
    subject: String,
    body: String,
}

impl SmtpMessageBuilder {
    fn new() -> Self {
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

    pub fn build(self) -> anyhow::Result<SmtpMessage> {
        if self.from.is_empty() {
            anyhow::bail!("at least one sender is required")
        }
        if self.to.is_empty() {
            anyhow::bail!("at least one recipient is required")
        }
        if self.subject.is_empty() {
            anyhow::bail!("subject is required")
        }
        if self.body.is_empty() {
            anyhow::bail!("body is required")
        }

        Ok(SmtpMessage {
            from: self.from,
            to: self.to,
            cc: self.cc,
            subject: self.subject,
            body: self.body,
        })
    }
}