use lettre::{message::Mailbox, transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::{common::utils, smtp::{smtp_content_type::SmtpContentType, smtp_credentials::SmtpCredentials, smtp_message::SmtpMessage, smtp_mode::SmtpMode}};

pub struct SmtpSender {
    host: String,
    from: Vec<String>,
    to: Vec<String>,
    cc: Vec<String>,
    credentials: Option<SmtpCredentials>,
    mode: SmtpMode,
}

impl SmtpSender {
    pub fn new<T: AsRef<str>>(host: T) -> Self {
        SmtpSender {
            host: host.as_ref().to_string(),
            from: Vec::new(),
            to: Vec::new(),
            cc: Vec::new(),
            credentials: None,
            mode: SmtpMode::RelayEsmtp,
        }
    }

    pub fn mode(mut self, mode: SmtpMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn from<T: AsRef<str>>(mut self, email_address: T) -> Self {
        self.from.push(email_address.as_ref().to_string());
        self
    }

    pub fn to<T: AsRef<str>>(mut self, email_address: T) -> Self {
        self.to.push(email_address.as_ref().to_string());
        self
    }

    pub fn cc<T: AsRef<str>>(mut self, email_address: T) -> Self {
        self.cc.push(email_address.as_ref().to_string());
        self
    }

    pub fn credentials<T: AsRef<str>>(mut self, user: T, password: T) -> Self {
        self.credentials = Some(SmtpCredentials {
            user: user.as_ref().to_string(),
            password: password.as_ref().to_string(),
        });
        self
    }

    pub async fn send(self, message: SmtpMessage) -> anyhow::Result<()> {
        let message = self.build_message(message)?;
        let transport = self.build_transport()?;

        match transport.send(message).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(err.to_string())),
        }
    }

    fn build_message(&self, message: SmtpMessage) -> anyhow::Result<Message> {
        let mut builder = Message::builder();

        builder = match message.content_type {
            SmtpContentType::TextPlain => builder.header(lettre::message::header::ContentType::TEXT_PLAIN),
            SmtpContentType::TextHtml => builder.header(lettre::message::header::ContentType::TEXT_HTML),
        };

        for email in self.from.iter() {
            builder = builder.from(Mailbox::new(None, email.parse()?));
        }
        for email in self.to.iter() {
            builder = builder.to(Mailbox::new(None, email.parse()?));
        }
        for email in self.cc.iter() {
            builder = builder.cc(Mailbox::new(None, email.parse()?));
        }
        
        Ok(builder.subject(message.subject).body(message.body)?)
    }

    fn build_transport(&self) -> anyhow::Result<AsyncSmtpTransport<Tokio1Executor>> {
        let (host, port) = utils::parse_host(&self.host, 25)?;

        let mut builder = match &self.mode {
            SmtpMode::RelayEsmtp => AsyncSmtpTransport::<Tokio1Executor>::relay(host)?.port(port),
            SmtpMode::RelayStartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)?.port(port),
            SmtpMode::Testing => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host).port(port),
        };

        if let Some(creds) = &self.credentials {
            builder = builder.credentials(Credentials::new(creds.user.clone(), creds.password.clone()));
        }

        Ok(builder.build())
    }
}