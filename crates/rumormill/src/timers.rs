//! Every tunable duration in one place.
//!
//! The constants below are the demo's whole temporal policy: how often we
//! announce ourselves, how long content lives, and how aggressively we
//! gossip. They are deliberately short so the full lifecycle (publish,
//! propagate, expire, evict) plays out within a single demo session.

use std::time::Duration;

/// How often we publish a fresh [`Entry::Presence`](crate::entry::Entry)
/// heartbeat (and redact the previous one).
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

/// A peer whose newest presence is older than this is considered gone; any
/// peer that notices redacts the stale presence entry. Three missed
/// heartbeats: long enough to ride out gossip latency, short enough to watch
/// eviction happen live.
pub const PRESENCE_STALE: Duration = Duration::from_secs(30);

/// How long a chat message lives before every holder redacts it. Five
/// minutes: messages visibly vanish within a demo session.
pub const CHAT_TTL: Duration = Duration::from_secs(300);

/// How long join/leave system notices live. Much shorter than chat: the
/// churn makes redaction traffic easy to observe.
pub const SYSTEM_TTL: Duration = Duration::from_secs(15);

/// Mean of the exponential inter-gossip delay. Gossip initiations form a
/// Poisson process so independent nodes never sync up into a thundering
/// herd; a converged session costs only a handshake, so the mean can be
/// aggressive.
pub const GOSSIP_MEAN_INTERVAL: Duration = Duration::from_millis(500);

/// Bounds on a single sampled gossip delay: the exponential distribution has
/// unbounded support, so clamp the tail (and keep a floor so a tiny sample
/// cannot spin-dial).
pub const GOSSIP_DELAY_MIN: Duration = Duration::from_millis(200);
pub const GOSSIP_DELAY_MAX: Duration = Duration::from_secs(20);

/// How long a dial may take before we give up on the peer for this round.
pub const DIAL_TIMEOUT: Duration = Duration::from_secs(10);

/// Ceiling on one whole gossip session, dial through join-back. A wedged
/// stream should never wedge the scheduler.
pub const SESSION_TIMEOUT: Duration = Duration::from_secs(30);

/// After a failed dial or session, leave the peer alone for this long.
pub const PEER_BACKOFF: Duration = Duration::from_secs(15);

/// UI input poll / render tick.
pub const UI_TICK: Duration = Duration::from_millis(50);

/// How long a message inserted out of causal order (mid-list) keeps its
/// highlight in the UI.
pub const HIGHLIGHT: Duration = Duration::from_millis(1500);
