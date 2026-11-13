//! An eventually consistent key-value store, replicated by causal gossip.

mod table;
mod replica;
mod credentials;
mod tls;
mod node;
mod api;
mod cli;
mod log_to_stderr;
mod listen;

use std::error::Error;

use anyhow::Result;
use clap::Parser;
use tracing::{error, info};

use crate::api::router;
use crate::cli::Cli;
use crate::credentials::Credentials;
use crate::listen::listen;
use crate::log_to_stderr::log_to_stderr;
use crate::node::Node;
use crate::tls::Tls;

#[tokio::main]
async fn main() -> Result<()> {
    log_to_stderr();

    let options = Cli::parse();

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
