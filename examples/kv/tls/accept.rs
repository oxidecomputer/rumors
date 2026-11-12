use anyhow::Result;
use tokio::io::{AsyncBufRead, AsyncWrite, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;

use super::Tls;
use super::connect::HANDSHAKE_DEADLINE;

impl Tls {
    pub(crate) async fn accept(
        &self,
        tcp: TcpStream,
    ) -> Result<(
        impl AsyncBufRead + Unpin + Send,
        impl AsyncWrite + Unpin + Send,
    )> {
        tcp.set_nodelay(true)?;
        let handshake = self.acceptor.accept(tcp);
        let (read, write) = tokio::io::split(timeout(HANDSHAKE_DEADLINE, handshake).await??);
        Ok((BufReader::new(read), write))
    }
}
