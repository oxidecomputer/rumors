use rustls::pki_types::{CertificateDer, PrivateKeyDer};

pub struct Credentials {
    pub cert: CertificateDer<'static>,
    pub key: PrivateKeyDer<'static>,
}

mod generate;
mod store;
mod load;

pub use generate::NETWORK_NAME;
