use std::path::Path;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};

pub struct Crypto;

impl Crypto {
    pub fn pem_load_certs<T: AsRef<Path>>(file_name: T) -> std::io::Result<Vec<CertificateDer<'static>>> {
        let cert_file = std::fs::File::open(file_name)?;
        let mut reader = std::io::BufReader::new(cert_file);
        rustls_pemfile::certs(&mut reader).collect()
    }

    pub fn pem_load_private_key<T: AsRef<Path>>(file_name: T) -> std::io::Result<PrivateKeyDer<'static>> {
        let key_file = std::fs::File::open(file_name)?;
        let mut reader = std::io::BufReader::new(key_file);
        rustls_pemfile::private_key(&mut reader).map(|key| key.unwrap())
    }
}
