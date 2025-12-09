use std::path::Path;

use rustls_pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};

pub struct Crypto;

impl Crypto {
    pub fn pem_load_certs(file_name: impl AsRef<Path>) -> anyhow::Result<Vec<CertificateDer<'static>>> {
        let certs = CertificateDer::pem_file_iter(file_name.as_ref())?
        .map(|res| {
            let cert = res?;
            Ok(cert.into_owned())
        })
        .collect::<Result<Vec<CertificateDer<'static>>, rustls_pki_types::pem::Error>>()?;

        Ok(certs)
    }

    
    pub fn pem_load_private_key(file_name: impl AsRef<Path>) -> anyhow::Result<PrivateKeyDer<'static>> {
        let keys: Vec<PrivateKeyDer<'static>> = PrivateKeyDer::pem_file_iter(file_name.as_ref())?
        .map(|res| {
            let key = res?;
            Ok(key.clone_key())
        })
        .collect::<Result<_, rustls_pki_types::pem::Error>>()?;

        keys.into_iter().next().ok_or_else(|| anyhow::anyhow!("no private keys found"))
    }
}
