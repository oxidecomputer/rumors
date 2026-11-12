//! An eventually consistent key-value store, replicated by causal gossip.

mod table;
mod replica;
mod credentials;
mod tls;
mod node;
mod api;
mod cli;

use std::env;
use std::error::Error;
use std::io::{self, IsTerminal};

use anyhow::{Context, Result};
use clap::Parser;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

use crate::api::router;
use crate::cli::Cli;
use crate::credentials::Credentials;
use crate::node::Node;
use crate::tls::Tls;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with_writer(io::stderr)
        .with_ansi(
            io::stderr().is_terminal()
                && env::var_os("NO_COLOR").is_none_or(|value| value.is_empty()),
        )
        .init();

    let options = Cli::parse();

    let listen = async |addr| {
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("listening on {addr}"))?;
        Ok::<_, anyhow::Error>((listener.local_addr()?, listener))
    };
    let (gossip_addr, gossip) = listen(options.gossip).await?;
    let (http_addr, http) = listen(options.http).await?;

    let tls = Tls::new(Credentials::load_or_create(&options.credentials)?)?;
    let node = Node::new(tls, options.bookmarks_dir, options.peers);
    node.start(gossip).await?;

    info!(gossip = %gossip_addr, http = %http_addr, "listening");
    axum::serve(http, router(node.clone()))
        .with_graceful_shutdown(async {
            if let Err(e) = tokio::signal::ctrl_c().await {
                error!(
                    error = &e as &dyn Error,
                    "cannot wait for Ctrl-C, so shutting down"
                );
            }
        })
        .await?;

    node.shutdown().await;
    Ok(())
}
