use crate::smtp::smtp_content_type::SmtpContentType;

pub struct SmtpMessage {
    pub subject: String,
    pub body: String,
    pub content_type: SmtpContentType,
}

impl SmtpMessage {
    pub fn new() -> Self {
        SmtpMessage {
            subject: String::new(),
            body: String::new(),
            content_type: SmtpContentType::TextPlain,
        }
    }

    pub fn with_subject<T: AsRef<str>>(mut self, subject: T) -> Self {
        self.subject = subject.as_ref().to_string();
        self
    }

    pub fn with_body<T: AsRef<str>>(mut self, body: T) -> Self {
        self.body = body.as_ref().to_string();
        self
    }

    pub fn with_content_type(mut self, content_type: SmtpContentType) -> Self {
        self.content_type = content_type;
        self
    }
}