//! Every tunable duration in one place.
//!
//! The constants below are the demo's whole temporal policy: how often we
//! announce ourselves, how long content lives, and how aggressively we
//! gossip. They are deliberately short so the full lifecycle (publish,
//! propagate, expire, evict) plays out within a single demo session.

use std::time::Duration;

/// How often we publish a fresh [`Entry::Presence`](crate::entry::Entry)
/// heartbeat (and redact the previous one). This is also the room's idle
/// gossip budget: every beat advances the causal frontier, and in a full
/// mesh each advance costs every node a (cheap, usually already-converged)
/// session on nearly every link — O(n²) sessions room-wide per beat. 30s
/// keeps a ~100-peer room comfortable while the lifecycle still plays out
/// within a demo session.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// A peer whose newest presence is older than this is considered gone; any
/// peer that notices redacts the stale presence entry. Three missed
/// heartbeats: long enough to ride out gossip latency, short enough to watch
/// eviction happen within a demo session.
pub const PRESENCE_STALE: Duration = Duration::from_secs(90);

/// How long a chat message lives before every holder redacts it. Five
/// minutes: messages visibly vanish within a demo session.
pub const CHAT_TTL: Duration = Duration::from_secs(300);

/// How long join/leave system notices live. Much shorter than chat: the
/// churn makes redaction traffic easy to observe.
pub const SYSTEM_TTL: Duration = Duration::from_secs(15);

/// How often the connector re-sweeps for dialable peers whose backoff has
/// expired; roster changes and finished connections wake it immediately,
/// so this only bounds how stale a backoff expiry can go unnoticed.
pub const REDIAL_SWEEP: Duration = Duration::from_secs(2);

/// How long a dial may take before we give up on the peer for this round.
pub const DIAL_TIMEOUT: Duration = Duration::from_secs(10);

/// Ceiling on the bounded waits around a connection's start: the dialer
/// opening its gossip stream, and the merge dance's fresh stream. The
/// drive itself is unbounded — connections are long-lived by design.
pub const SESSION_TIMEOUT: Duration = Duration::from_secs(30);

/// After a connection ends — failed dial, failed drive, or the peer's own
/// goodbye — leave the peer alone for this long before redialing.
pub const PEER_BACKOFF: Duration = Duration::from_secs(15);

/// The owner's coalescing tick: deferred loss sweeps and view publishes
/// run at most once per tick. Both jobs are O(everything on screen), and
/// session outcomes arrive at mesh rate (every entry anyone originates
/// costs every node a session on nearly every link), so running them per
/// event would bury the owner at scale; per-tick execution caps the work
/// at ten rounds a second regardless of how hot the mesh runs. The UI
/// repaints on [`UI_TICK`] anyway, so the deferral is invisible.
pub const VIEW_COALESCE: Duration = Duration::from_millis(100);

/// UI input poll / render tick.
pub const UI_TICK: Duration = Duration::from_millis(50);

/// How long a message inserted out of causal order (mid-list) keeps its
/// highlight in the UI.
pub const HIGHLIGHT: Duration = Duration::from_millis(1500);
