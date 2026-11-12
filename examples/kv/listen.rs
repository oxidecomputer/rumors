use std::net::SocketAddr;

use anyhow::{Context, Result};
use tokio::net::TcpListener;

pub(crate) async fn listen(addr: SocketAddr) -> Result<(SocketAddr, TcpListener)> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("listening on {addr}"))?;
    Ok((listener.local_addr()?, listener))
}
