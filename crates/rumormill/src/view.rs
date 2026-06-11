//! Immutable render snapshots.
//!
//! The owner publishes an [`Arc<View>`](std::sync::Arc) through a
//! [`tokio::sync::watch`] channel after every state transition; the UI task
//! renders whatever snapshot is current and keeps only cursor and scroll
//! state of its own. The types here are plain data — no `Rumors`, no iroh —
//! so the renderer needs nothing but this module and ratatui.

use std::time::Instant;

use rumors::Key;

use crate::entry::{Millis, PeerId};

/// Everything the UI renders.
#[derive(Debug, Clone, Default)]
pub struct View {
    /// Our own peer id (the iroh `EndpointId` bytes); shown in the header at
    /// all times so it can be shared.
    pub me: PeerId,
    /// Our z-base-32 endpoint id string, exactly as a peer would paste it.
    pub me_display: String,
    /// Our display name.
    pub name: String,
    /// Short identifier of the universe we currently belong to; changes when
    /// a partition merge resets us into a winning network.
    pub network: String,
    /// Set when the last reset happened: a "merged into …" notice.
    pub merged_notice: Option<String>,
    /// Channels in name order.
    pub channels: Vec<ChannelView>,
    /// Live peers, most recently seen first.
    pub roster: Vec<PeerView>,
    /// Manually added dial targets (paste dialog / --peer) that have not yet
    /// shown up in the roster; the gossip scheduler unions them in.
    pub dial_targets: Vec<PeerId>,
    /// Session counters for the status line.
    pub stats: Stats,
}

/// One channel and its causally ordered messages.
#[derive(Debug, Clone)]
pub struct ChannelView {
    /// The channel name.
    pub name: String,
    /// Messages in display (causal) order.
    pub messages: Vec<MessageView>,
}

/// One rendered message line.
#[derive(Debug, Clone)]
pub struct MessageView {
    /// The message's key: its stable identity across peers. The renderer
    /// styles by `highlight_until` instead; the tests read this to drive
    /// redaction by key.
    #[allow(dead_code)]
    pub key: Key,
    /// The author's id, or `None` for system notices.
    pub author: Option<PeerId>,
    /// The author's display name (resolved from presence, or short hex).
    pub author_name: String,
    /// The text.
    pub body: String,
    /// Wall-clock timestamp, cosmetic.
    pub at: Millis,
    /// While set and in the future, render highlighted: the message was
    /// delivered out of causal order and landed mid-list.
    pub highlight_until: Option<Instant>,
}

/// One roster line.
#[derive(Debug, Clone)]
pub struct PeerView {
    /// The peer's id.
    pub peer: PeerId,
    /// Their display name.
    pub name: String,
    /// Wall-clock time of their newest heartbeat.
    pub last_seen: Millis,
}

/// Gossip session counters for the status line.
#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    /// Entries currently live in the rumor set.
    pub live_entries: usize,
    /// Completed gossip sessions.
    pub sessions_ok: u64,
    /// Failed gossip sessions (dial errors, timeouts, stream errors).
    pub sessions_failed: u64,
    /// Partition merges we lost (and reset through).
    pub merges: u64,
}
