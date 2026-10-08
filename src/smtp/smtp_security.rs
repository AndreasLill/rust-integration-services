pub enum SmtpSecurity {
    /// SMTP with `TLS`
    Tls,
    /// SMTP with `STARTTLS`
    StartTls,
    /// SMTP
    None,
}