//! rumormill: a TUI chatroom that gossips over iroh, demonstrating the
//! `rumors` crate end to end.
//!
//! Every replicated fact — chat lines, channel creations, presence
//! heartbeats, join/leave notices — rides one `Known<Entry>` rumor set. That
//! is the demo's central trick: peers learn *who else to gossip with* from
//! the same state they are gossiping, so one pasted endpoint id bootstraps a
//! node into the whole room.
//!
//! # What to watch
//!
//! - **Leaderless bootstrap.** Every node seeds its own universe at startup.
//!   When two universes meet, gossip fails symmetrically with
//!   `NetworkMismatch`, which carries the remote's network id and event
//!   floor; both sides apply the same rule (greater `(min_events, network)`
//!   wins — see [`net::decide`]) and the loser resets wholesale, bootstrapping
//!   into the winner. A fresh node is just a tiny partition that always
//!   loses, so "joining" and "partition healing" are the same mechanism.
//! - **Causal display order.** Messages are placed by their causal
//!   [`Version`](rumors::Version); a message that gossip delivers late lands
//!   *mid-list* (briefly highlighted) rather than at the bottom. Concurrent
//!   messages carry no obligation and sit in arrival order.
//! - **Redaction everywhere.** Chat and system notices expire on a TTL
//!   (every holder redacts, idempotently); each heartbeat supersedes and
//!   redacts its predecessor; silent peers have their last presence redacted
//!   by whoever notices. The rumor set stays bounded: only channel
//!   creations are durable.
//! - **The party lifecycle.** Newcomers receive a forked party over the
//!   wire; a graceful quit (`Esc`) retires the party into a live peer so the
//!   id-region is reclaimed rather than leaked.
//!
//! # Smoke script (three terminals)
//!
//! 1. `just rumormill --name a` — note the endpoint id in the header.
//! 2. `just rumormill --name b`, paste A's id into the dialog: B (younger
//!    universe) resets into A's; "b joined" appears on A.
//! 3. `just rumormill --name c`, paste A's id: C discovers B through the
//!    replicated presence and gossips with it directly, though C never
//!    dialed B.
//! 4. `/new dogs` on B: the channel appears everywhere (Tab to switch).
//! 5. `kill -STOP` C, chat on A and B, `kill -CONT` C: the missed messages
//!    land mid-list on C, highlighted.
//! 6. `kill -9` B: within ~30s the survivors drop B from the roster (stale
//!    presence redacted).
//! 7. Wait ~15s after any join/leave: the notice vanishes everywhere (TTL
//!    redaction propagating).
//! 8. `Esc` on A: A retires its party into a survivor and the room outlives
//!    its original seeder.
//!
//! Sending a message to a peer who is offline still works — it propagates
//! through any peers both of you gossip with, whenever they next reconcile.

mod causal;
mod cli;
mod entry;
mod net;
mod owner;
mod state;
mod timers;
mod ui;
mod view;

use anyhow::Context as _;
use clap::Parser as _;
use ratatui::crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use ratatui::crossterm::execute;
use rumors::Known;
use tokio::sync::mpsc;

use crate::owner::{Clock, Command, Owner};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    let name = args.display_name();

    eprintln!("rumormill: binding the iroh endpoint…");
    let endpoint = net::bind().await?;
    endpoint.online().await;
    let me = endpoint.id();
    // Also on stderr (the TUI header repeats it): the id survives in the
    // scrollback for copy-pasting after the alternate screen exits.
    eprintln!("rumormill: our endpoint id is {me}");

    // Our own universe, until we meet a stronger one.
    let known: Known<entry::Entry> = Known::seed();
    let (owner, view_rx) = Owner::new(known, *me.as_bytes(), me.to_string(), name, Clock::system());
    let (cmd_tx, cmd_rx) = mpsc::channel(64);
    let owner_task = tokio::spawn(owner.run(cmd_rx));
    for peer in &args.peers {
        cmd_tx
            .send(Command::AddPeer {
                peer: *peer.as_bytes(),
            })
            .await
            .context("owner task died during startup")?;
    }

    let accept = net::spawn_accept_loop(endpoint.clone(), cmd_tx.clone());
    let scheduler = net::spawn_scheduler(endpoint.clone(), cmd_tx.clone(), view_rx.clone());

    // The terminal. `ratatui::init` installs a restore-on-panic hook;
    // bracketed paste makes a pasted endpoint id arrive as one event.
    let mut terminal = ratatui::init();
    let _ = execute!(std::io::stdout(), EnableBracketedPaste);
    let (input_tx, input_rx) = mpsc::channel(32);
    ui::spawn_input_thread(input_tx);
    let ui_result = ui::run(
        &mut terminal,
        cmd_tx.clone(),
        view_rx.clone(),
        input_rx,
        args.peers.is_empty(),
    )
    .await;
    let _ = execute!(std::io::stdout(), DisableBracketedPaste);
    ratatui::restore();

    // Leave gracefully. Order matters: the owner says goodbye and hands the
    // `Known` back; then the gossip tasks are torn down so their snapshots
    // drop (retire refuses while any snapshot shares the party — the
    // library enforces the exclusivity, and `net::retire` waits it out).
    cmd_tx.send(Command::Shutdown).await.ok();
    let (known, candidates) = owner_task.await.context("owner task panicked")?;
    scheduler.abort();
    accept.abort();
    let _ = scheduler.await;
    let _ = accept.await;

    if candidates.is_empty() {
        eprintln!(
            "rumormill: no live peers; departing without retiring (id-region leaks, which is fine for a demo)"
        );
    } else {
        eprintln!("rumormill: retiring our party into a peer…");
        match net::retire(&endpoint, known, candidates).await {
            net::Departure::Retired { into } => {
                eprintln!(
                    "rumormill: retired into {}; id-region reclaimed",
                    into.fmt_short()
                );
            }
            net::Departure::Uncertain => {
                eprintln!(
                    "rumormill: the hand-off was interrupted mid-frame; the peer may hold our party \
                     (two generals), so we depart without retrying"
                );
            }
            net::Departure::Leaked => {
                eprintln!("rumormill: no peer could absorb us; departing with the region leaked");
            }
        }
    }
    endpoint.close().await;
    ui_result
}
