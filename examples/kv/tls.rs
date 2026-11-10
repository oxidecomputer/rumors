use tokio_rustls::{TlsAcceptor, TlsConnector};

#[derive(Clone)]
pub struct Tls {
    connector: TlsConnector,
    acceptor: TlsAcceptor,
}

use std::sync::Arc;

use anyhow::Result;
use rustls::server::WebPkiClientVerifier;
use rustls::{ClientConfig, RootCertStore, ServerConfig};

use crate::credentials::Credentials;

impl Tls {
    pub fn new(credentials: Credentials) -> Result<Self> {
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

pub const DEFAULT_PORT: u16 = 8081;

use http::uri::Authority;
use rustls::pki_types::ServerName;
use tokio::io::{AsyncBufRead, AsyncWrite, BufReader};
use tokio::net::TcpStream;

use crate::credentials::NETWORK_NAME;

impl Tls {
    pub async fn connect(
        &self,
        address: &Authority,
    ) -> Result<(
        impl AsyncBufRead + Unpin + Send,
        impl AsyncWrite + Unpin + Send,
    )> {
        let host = address.host().trim_start_matches('[').trim_end_matches(']');
        let port = address.port_u16().unwrap_or(DEFAULT_PORT);
        let tcp = TcpStream::connect((host, port)).await?;
        tcp.set_nodelay(true)?;
        let name = ServerName::try_from(NETWORK_NAME)?;
        let (read, write) = tokio::io::split(self.connector.connect(name, tcp).await?);
        Ok((BufReader::new(read), write))
    }

    pub async fn accept(
        &self,
        tcp: TcpStream,
    ) -> Result<(
        impl AsyncBufRead + Unpin + Send,
        impl AsyncWrite + Unpin + Send,
    )> {
        tcp.set_nodelay(true)?;
        let (read, write) = tokio::io::split(self.acceptor.accept(tcp).await?);
        Ok((BufReader::new(read), write))
    }
}
