use std::time::Duration;

use anyhow::Result;
use http::uri::Authority;
use rustls::pki_types::ServerName;
use tokio::io::{AsyncBufRead, AsyncWrite, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::credentials::NETWORK_NAME;

use super::Tls;

pub(crate) const DEFAULT_PORT: u16 = 8081;

pub(super) const HANDSHAKE_DEADLINE: Duration = Duration::from_secs(10);

impl Tls {
    pub(crate) async fn connect(
        &self,
        address: &Authority,
    ) -> Result<(
        impl AsyncBufRead + Unpin + Send,
        impl AsyncWrite + Unpin + Send,
    )> {
        let host = address.host().trim_start_matches('[').trim_end_matches(']');
        let port = address.port_u16().unwrap_or(DEFAULT_PORT);
        let name = ServerName::try_from(NETWORK_NAME)?;
        let handshake = async {
            let tcp = TcpStream::connect((host, port)).await?;
            tcp.set_nodelay(true)?;
            self.connector.connect(name, tcp).await
        };
        let (read, write) = tokio::io::split(timeout(HANDSHAKE_DEADLINE, handshake).await??);
        Ok((BufReader::new(read), write))
    }
}
