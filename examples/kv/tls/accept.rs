use anyhow::Result;
use tokio::io::{AsyncBufRead, AsyncWrite, BufReader};
use tokio::net::TcpStream;

use super::Tls;

impl Tls {
    pub(crate) async fn accept(
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
