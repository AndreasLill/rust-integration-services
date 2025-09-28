pub enum SmtpMode {
    /// Production SMTP relay with `ESMTP`
    RelayEsmtp,
    /// Production SMTP relay with `STARTTLS`
    RelayStartTls,
    /// Testing SMTP without `ESMTP` or `STARTTLS`
    Testing,
}