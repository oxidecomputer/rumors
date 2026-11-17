use anyhow::Result;
use rcgen::{CertificateParams, ExtendedKeyUsagePurpose, KeyPair};
use rustls::pki_types::PrivatePkcs8KeyDer;

use super::Credentials;

pub const NETWORK_NAME: &str = "kv.example.rumors.invalid";

impl Credentials {
    pub fn generate() -> Result<Self> {
        let mut params = CertificateParams::new([NETWORK_NAME.to_owned()])?;
        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ServerAuth,
            ExtendedKeyUsagePurpose::ClientAuth,
        ];

        let key = KeyPair::generate()?;
        let cert = params.self_signed(&key)?;

        Ok(Self {
            cert: cert.der().clone(),
            key: PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
        })
    }
}
