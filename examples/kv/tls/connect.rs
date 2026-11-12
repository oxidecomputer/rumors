use anyhow::Result;
use http::uri::Authority;
use rustls::pki_types::ServerName;
use tokio::io::{AsyncBufRead, AsyncWrite, BufReader};
use tokio::net::TcpStream;

use crate::credentials::NETWORK_NAME;

use super::Tls;

pub(crate) const DEFAULT_PORT: u16 = 8081;

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
        let tcp = TcpStream::connect((host, port)).await?;
        tcp.set_nodelay(true)?;
        let name = ServerName::try_from(NETWORK_NAME)?;
        let (read, write) = tokio::io::split(self.connector.connect(name, tcp).await?);
        Ok((BufReader::new(read), write))
    }
}
