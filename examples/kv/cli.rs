use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use anyhow::{Result, ensure};
use clap::Parser;
use http::uri::Authority;

use crate::tls::DEFAULT_PORT;

#[derive(Parser)]
pub struct Cli {
    #[arg(long, default_value = "kv-credentials")]
    pub credentials: PathBuf,
    #[arg(long = "bookmarks", default_value = "kv-bookmarks")]
    pub bookmarks_dir: PathBuf,
    #[arg(long, default_value_t = SocketAddr::from((Ipv4Addr::UNSPECIFIED, DEFAULT_PORT)))]
    pub gossip: SocketAddr,
    #[arg(long, default_value_t = SocketAddr::from((Ipv4Addr::LOCALHOST, 8080)))]
    pub http: SocketAddr,
    #[arg(long = "peer", value_parser = parse_peer)]
    pub peers: Vec<Authority>,
}

fn parse_peer(address: &str) -> Result<Authority> {
    let address = match address.strip_prefix(':') {
        Some(port) => format!("localhost:{port}"),
        None => address.to_owned(),
    };
    let authority: Authority = address.parse()?;
    let host = authority.host();
    let understood = match authority.port_u16() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    };
    ensure!(understood == address, "expected HOST, HOST:PORT, or :PORT");
    Ok(authority)
}
