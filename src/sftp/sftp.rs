use std::sync::Arc;

use russh::keys::{HashAlg, PrivateKeyWithHashAlg};
use russh_sftp::client::SftpSession;

use crate::sftp::{sftp_client_config::SftpClientConfig, ssh_client::SshClient};

pub async fn connect(config: Arc<SftpClientConfig>) -> anyhow::Result<SftpSession> {
    let ssh_config = russh::client::Config::default();
    let ssh = SshClient {};

    tracing::debug!("connecting to {}", config.endpoint);
    let mut session = russh::client::connect(Arc::new(ssh_config), &config.endpoint, ssh).await?;
    
    if let Some(auth) = &config.auth_basic {
        session.authenticate_password(&auth.user, &auth.password).await?;
        tracing::debug!("authenticated using basic");
    }

    if let Some(auth) = &config.auth_private_key {
        let key = russh::keys::load_secret_key(&auth.path, auth.passphrase.as_deref())?;
        let hash_alg = match &key.algorithm() {
            russh::keys::Algorithm::Rsa { .. } => Some(HashAlg::Sha256),
            _ => None,
        };

        let key_with_alg = PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg);
        session.authenticate_publickey(&auth.user, key_with_alg).await?;
        tracing::debug!("authenticated using key");
    }

    let channel = session.channel_open_session().await?;
    channel.request_subsystem(true, "sftp").await?;
    let sftp = SftpSession::new(channel.into_stream()).await?;

    tracing::debug!("connected to {}", config.endpoint);
    Ok(sftp)
}