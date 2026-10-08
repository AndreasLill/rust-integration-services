

use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox, transport::smtp::authentication::Credentials};

use crate::smtp::{smtp_client_config::SmtpClientConfig, smtp_message::SmtpMessage, smtp_security::SmtpSecurity};

pub struct SmtpClient {
    config: SmtpClientConfig,
}

impl SmtpClient {
    pub fn new(config: SmtpClientConfig) -> Self {
        Self {
            config,
        }
    }

    pub async fn send(&self, message: SmtpMessage) -> anyhow::Result<()> {
        let message = self.build_message(message)?;
        let transport = self.build_transport()?;

        match transport.send(message).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(err.to_string())),
        }
    }

    fn build_message(&self, message: SmtpMessage) -> anyhow::Result<Message> {
        let mut builder = Message::builder();
        builder = builder.header(lettre::message::header::ContentType::TEXT_PLAIN);

        for from in message.from.iter() {
            builder = builder.from(Mailbox::new(None, from.parse()?));
        }
        for to in message.to.iter() {
            builder = builder.to(Mailbox::new(None, to.parse()?));
        }
        for cc in message.cc.iter() {
            builder = builder.cc(Mailbox::new(None, cc.parse()?));
        }
        
        Ok(builder.subject(message.subject).body(message.body)?)
    }

    fn build_transport(&self) -> anyhow::Result<AsyncSmtpTransport<Tokio1Executor>> {
        let mut builder = match &self.config.security {
            SmtpSecurity::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.host)?.port(self.config.port),
            SmtpSecurity::StartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.config.host)?.port(self.config.port),
            SmtpSecurity::None => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&self.config.host).port(self.config.port),
        };

        if let (Some(username), Some(password)) = (&self.config.username, &self.config.password) {
            builder = builder.credentials(Credentials::new(username.clone(), password.clone()));
        }

        Ok(builder.build())
    }
}