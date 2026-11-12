use std::fmt::Display;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use anyhow::{Result, ensure};
use clap::Parser;
use http::uri::Authority;

use crate::tls::DEFAULT_GOSSIP_PORT;

const DEFAULT_HTTP_PORT: u16 = 8000;

#[derive(Parser)]
pub(crate) struct Cli {
    #[arg(long, default_value = "kv-credentials")]
    pub(crate) credentials: PathBuf,
    #[arg(long = "bookmarks", default_value = "kv-bookmarks")]
    pub(crate) bookmarks_dir: PathBuf,
    #[arg(
        long,
        value_parser = parse_gossip,
        default_value_t = SocketAddr::from((Ipv4Addr::UNSPECIFIED, DEFAULT_GOSSIP_PORT)),
    )]
    pub(crate) gossip: SocketAddr,
    #[arg(
        long,
        value_parser = parse_http,
        default_value_t = SocketAddr::from((Ipv4Addr::LOCALHOST, DEFAULT_HTTP_PORT)),
    )]
    pub(crate) http: SocketAddr,
    #[arg(long = "peer", value_parser = parse_peer)]
    pub(crate) peers: Vec<Authority>,
}

fn with_host(address: &str, host: impl Display) -> String {
    match address.strip_prefix(':') {
        Some(port) => format!("{host}:{port}"),
        None if address.bytes().all(|byte| byte.is_ascii_digit()) => format!("{host}:{address}"),
        None => address.to_owned(),
    }
}

fn parse_gossip(address: &str) -> Result<SocketAddr> {
    Ok(with_host(address, Ipv4Addr::UNSPECIFIED).parse()?)
}

fn parse_http(address: &str) -> Result<SocketAddr> {
    Ok(with_host(address, Ipv4Addr::LOCALHOST).parse()?)
}

fn parse_peer(address: &str) -> Result<Authority> {
    let address = with_host(address, "localhost");
    let authority: Authority = address.parse()?;
    let host = authority.host();
    let understood = match authority.port_u16() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    };
    ensure!(
        understood == address,
        "expected HOST, HOST:PORT, PORT, or :PORT"
    );
    Ok(authority)
}
