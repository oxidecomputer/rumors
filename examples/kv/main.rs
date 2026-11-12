//! An eventually consistent key-value store, replicated by causal gossip.

include!("table.rs");
include!("replica.rs");
include!("credentials.rs");
include!("tls.rs");
include!("node.rs");
include!("api.rs");
include!("cli.rs");

#[tokio::main]
async fn main() -> Result<()> {
    let Cli {
        credentials,
        bookmarks_dir,
        gossip,
        http,
        peers,
    } = Cli::parse();

    let listen = async |addr| {
        TcpListener::bind(addr)
            .await
            .with_context(|| format!("listening on {addr}"))
    };
    let gossip = listen(gossip).await?;
    let http = listen(http).await?;

    let tls = Tls::new(Credentials::load_or_create(&credentials)?)?;
    let node = Node::new(tls, bookmarks_dir, peers);
    node.start(gossip).await?;

    axum::serve(http, router(node.clone()))
        .with_graceful_shutdown(async {
            if let Err(e) = tokio::signal::ctrl_c().await {
                eprintln!("cannot wait for Ctrl-C, so shutting down: {e}");
            }
        })
        .await?;

    node.shutdown().await;
    Ok(())
}
