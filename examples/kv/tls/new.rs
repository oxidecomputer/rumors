use tokio_rustls::{TlsAcceptor, TlsConnector};

#[derive(Clone)]
struct Tls {
    connector: TlsConnector,
    acceptor: TlsAcceptor,
}

use rustls::server::WebPkiClientVerifier;
use rustls::{ClientConfig, RootCertStore, ServerConfig};

impl Tls {
    fn new(credentials: Credentials) -> Result<Self> {
        let Credentials { cert, key } = credentials;

        let mut roots = RootCertStore::empty();
        roots.add(cert.clone())?;
        let roots = Arc::new(roots);

        let client = ClientConfig::builder()
            .with_root_certificates(roots.clone())
            .with_client_auth_cert(vec![cert.clone()], key.clone_key())?;

        let verifier = WebPkiClientVerifier::builder(roots).build()?;
        let server = ServerConfig::builder()
            .with_client_cert_verifier(verifier)
            .with_single_cert(vec![cert], key)?;

        Ok(Self {
            connector: Arc::new(client).into(),
            acceptor: Arc::new(server).into(),
        })
    }
}
