use std::net::Ipv4Addr;

use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value = "kv-credentials")]
    credentials: PathBuf,
    #[arg(long = "bookmarks", default_value = "kv-bookmarks")]
    bookmarks_dir: PathBuf,
    #[arg(long, default_value_t = SocketAddr::from((Ipv4Addr::UNSPECIFIED, DEFAULT_PORT)))]
    gossip: SocketAddr,
    #[arg(long, default_value_t = SocketAddr::from((Ipv4Addr::LOCALHOST, 8080)))]
    http: SocketAddr,
    #[arg(long = "peer", value_parser = parse_peer)]
    peers: Vec<Authority>,
}
