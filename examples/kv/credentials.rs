use rustls::pki_types::{CertificateDer, PrivateKeyDer};

pub(crate) struct Credentials {
    pub(crate) cert: CertificateDer<'static>,
    pub(crate) key: PrivateKeyDer<'static>,
}

mod generate;
mod store;
mod load;

pub(crate) use generate::NETWORK_NAME;
