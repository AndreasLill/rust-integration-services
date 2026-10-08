use russh::keys::PublicKeyOrCertificate;

pub struct SshClient;

impl russh::client::Handler for SshClient {
    type Error = anyhow::Error;

    async fn check_server_key(&mut self, _server_public_key: &PublicKeyOrCertificate) -> Result<bool, anyhow::Error> {
        Ok(true)
    }
}