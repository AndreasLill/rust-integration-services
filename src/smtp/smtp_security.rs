/// SMTP security modes.
///
/// # Security Recommendations
/// - `Tls`: Use for implicit TLS connections (port 465). Most secure for most cases.
/// - `StartTls`: Use for explicit TLS upgrade (port 587). Common for email providers.
/// - `None`: No encryption. Only use for testing or trusted internal networks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpSecurity {
    /// SMTP with `TLS` (implicit TLS)
    Tls,
    /// SMTP with `STARTTLS` (explicit TLS upgrade)
    StartTls,
    /// SMTP without encryption
    None,
}
