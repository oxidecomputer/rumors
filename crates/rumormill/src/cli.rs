//! Command-line arguments.

use clap::Parser;
use iroh::EndpointId;

/// A TUI chatroom that gossips over iroh, built on the `rumors` crate.
///
/// Every node seeds its own universe at startup; point any two at each other
/// (paste a peer id into the dialog, or pass `--peer`) and the younger
/// partition resets into the older one. From a single contact, the rest of
/// the room is discovered through the replicated state itself.
#[derive(Debug, Parser)]
#[command(name = "rumormill", version, about)]
pub struct Args {
    /// Display name; defaults to $USER.
    #[arg(long, short)]
    pub name: Option<String>,

    /// A peer to dial at startup (repeatable). When given, the connect
    /// dialog is skipped.
    #[arg(long = "peer", value_name = "ENDPOINT_ID")]
    pub peers: Vec<EndpointId>,
}

impl Args {
    /// The display name to announce.
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .or_else(|| std::env::var("USER").ok())
            .unwrap_or_else(|| "anon".to_string())
    }
}
