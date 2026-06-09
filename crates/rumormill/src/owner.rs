//! The actor that owns the canonical rumor set.
//!
//! Exactly one task holds the `Known<Entry, Facts>` — the only value that
//! can originate messages and redactions — together with the [`AppState`]
//! display machine and the expiry wheel. Everything else talks to it through
//! a [`Command`] channel and reads back through a
//! [`watch`](tokio::sync::watch)-published [`View`] snapshot.
//!
//! The wiring is deadlock-free by construction: the owner never awaits
//! another task. [`Command::Snapshot`] is answered immediately (a
//! [`rumors`](rumors::Known::rumors) snapshot is a cheap copy-on-write
//! view), join callbacks only touch owner-local state, and publishing uses
//! [`watch::Sender::send_replace`]. Every wait-for edge points from a
//! connection task toward the owner, so the wait-for graph is acyclic.
//!
//! Concurrency follows the crate's snapshot discipline: connection tasks
//! gossip with [`Known<Entry, Rumors>`] snapshots and fold what they learned
//! back via [`Command::JoinBack`]; the owner drives the state machine
//! forward at every join. A lost partition merge arrives as
//! [`Command::Reset`], which swaps the whole world out.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use futures::StreamExt;
use rumors::{Key, Known, Network, Rumors, Version};
use tokio::sync::{mpsc, oneshot, watch};
use tokio_util::time::{DelayQueue, delay_queue};

use crate::entry::{Entry, Millis, PeerId};
use crate::state::{AppState, Effect};
use crate::timers;
use crate::view::{ChannelView, MessageView, PeerView, Stats, View};

/// The channel every room conversation starts in; created at seed time and
/// re-created after a partition merge if the winning network lacks it.
pub const HOME_CHANNEL: &str = "general";

/// A milliseconds-since-epoch clock, injectable so tests can warp wall time
/// (expiry deadlines and staleness sweeps are wall-clock policies).
#[derive(Clone)]
pub struct Clock(Arc<dyn Fn() -> Millis + Send + Sync>);

impl Clock {
    /// The system wall clock.
    pub fn system() -> Self {
        Clock(Arc::new(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is past the epoch")
                .as_millis() as Millis
        }))
    }

    /// An arbitrary clock, for tests.
    #[cfg(test)]
    pub fn from_fn(f: impl Fn() -> Millis + Send + Sync + 'static) -> Self {
        Clock(Arc::new(f))
    }

    /// The current time in epoch milliseconds.
    pub fn now(&self) -> Millis {
        (self.0)()
    }
}

/// Everything the owner can be asked to do. The single entry point to the
/// state machine, so tests drive it synthetically over the same channel
/// production uses.
pub enum Command {
    /// Send a chat line to a channel (from the UI input).
    SendChat {
        /// Target channel.
        channel: String,
        /// Message text.
        body: String,
    },
    /// Create a channel (from the UI `/new`).
    CreateChannel {
        /// The new channel's name.
        name: String,
    },
    /// Add a manual dial target (from the paste dialog or `--peer`).
    AddPeer {
        /// The peer's id.
        peer: PeerId,
    },
    /// Publish a fresh presence heartbeat and sweep stale peers. Produced
    /// internally on a timer; public so tests can fire it at will.
    HeartbeatTick,
    /// An ephemeral entry reached its deadline: redact it. Produced
    /// internally by the expiry wheel; public so tests can fire it at will.
    ExpiryDue {
        /// The expired entry's key.
        key: Key,
    },
    /// Hand out a copy-on-write snapshot for a gossip session.
    Snapshot {
        /// Where to send the snapshot.
        reply: oneshot::Sender<Known<Entry, Rumors>>,
    },
    /// Fold a gossiped snapshot back in, observing everything it learned.
    JoinBack {
        /// The snapshot, as returned by a completed gossip session.
        snapshot: Known<Entry, Rumors>,
    },
    /// We lost a partition merge: adopt the winning universe wholesale.
    Reset {
        /// The freshly bootstrapped rumor set in the winning network.
        known: Box<Known<Entry>>,
        /// Every entry it arrived with, as observed by the bootstrap.
        observed: Vec<(Key, Version, Arc<Entry>)>,
        /// The universe we were in when this merge was lost. The owner
        /// adopts only while it is *still* in that universe: the
        /// session-level verdict is the single arbiter of merges, and this
        /// pins which state it arbitrated. (Comparing event floors again
        /// here would re-litigate the verdict with drifted counts — local
        /// activity keeps advancing between the verdict and this command —
        /// and a wrong decline could leave two partitions never merging.)
        abandoned: Network,
    },
    /// Record a finished gossip session in the status-line counters.
    SessionOutcome {
        /// Whether the session completed.
        ok: bool,
    },
    /// Leave the room: the run loop returns the `Known` for retirement.
    Shutdown,
}

/// The owner actor. Construct with [`Owner::new`], then drive with
/// [`Owner::run`].
pub struct Owner {
    known: Known<Entry>,
    state: AppState,
    me: PeerId,
    me_display: String,
    name: String,
    clock: Clock,
    expiry: DelayQueue<Key>,
    expiry_keys: HashMap<Key, delay_queue::Key>,
    highlights: HashMap<Key, Instant>,
    dial_targets: Vec<PeerId>,
    merged_notice: Option<String>,
    stats: Stats,
    view_tx: watch::Sender<Arc<View>>,
}

impl Owner {
    /// Build an owner around a freshly seeded `Known` and hand back the view
    /// channel the UI (and the gossip scheduler) will read.
    pub fn new(
        known: Known<Entry>,
        me: PeerId,
        me_display: String,
        name: String,
        clock: Clock,
    ) -> (Self, watch::Receiver<Arc<View>>) {
        let (view_tx, view_rx) = watch::channel(Arc::new(View::default()));
        let owner = Owner {
            known,
            state: AppState::new(),
            me,
            me_display,
            name,
            clock,
            expiry: DelayQueue::new(),
            expiry_keys: HashMap::new(),
            highlights: HashMap::new(),
            dial_targets: Vec::new(),
            merged_notice: None,
            stats: Stats::default(),
            view_tx,
        };
        (owner, view_rx)
    }

    /// Drive the actor until [`Command::Shutdown`], then return the `Known`
    /// (for retirement) and the retire candidates, most recently seen first.
    ///
    /// The heartbeat interval and the expiry wheel are folded into the same
    /// [`Command`] stream the channel feeds, so every state transition flows
    /// through [`handle`](Self::handle).
    pub async fn run(mut self, mut rx: mpsc::Receiver<Command>) -> (Known<Entry>, Vec<PeerId>) {
        // Announce ourselves to the (initially one-node) universe, presence
        // included: the first snapshot handed out must already carry it, and
        // the heartbeat interval below only keeps it fresh.
        let now = self.clock.now();
        self.originate(vec![
            Entry::Channel {
                name: HOME_CHANNEL.into(),
                created_by: self.me,
                at: now,
            },
            Entry::Presence {
                peer: self.me,
                name: self.name.clone(),
                at: now,
            },
            Entry::System {
                channel: HOME_CHANNEL.into(),
                body: format!("{} is online", self.name),
                at: now,
                ttl_ms: timers::SYSTEM_TTL.as_millis() as u64,
            },
        ])
        .await;
        self.publish();

        let mut heartbeat = tokio::time::interval(timers::HEARTBEAT_INTERVAL);
        loop {
            let cmd = tokio::select! {
                Some(cmd) = rx.recv() => cmd,
                _ = heartbeat.tick() => Command::HeartbeatTick,
                // The guard keeps an empty wheel from being polled at all;
                // `DelayQueue` ends its stream when empty rather than
                // registering a waker for future inserts.
                Some(expired) = self.expiry.next(), if !self.expiry.is_empty() => {
                    Command::ExpiryDue { key: expired.into_inner() }
                }
            };
            if matches!(cmd, Command::Shutdown) {
                break;
            }
            self.handle(cmd).await;
            self.publish();
        }
        self.shutdown().await
    }

    /// Apply one command to the state machine.
    async fn handle(&mut self, cmd: Command) {
        let now = self.clock.now();
        match cmd {
            Command::SendChat { channel, body } => {
                self.originate(vec![Entry::Chat {
                    channel,
                    author: self.me,
                    body,
                    sent_at: now,
                    ttl_ms: timers::CHAT_TTL.as_millis() as u64,
                }])
                .await;
            }
            Command::CreateChannel { name } => {
                self.originate(vec![Entry::Channel {
                    name,
                    created_by: self.me,
                    at: now,
                }])
                .await;
            }
            Command::AddPeer { peer } => {
                if peer != self.me && !self.dial_targets.contains(&peer) {
                    self.dial_targets.push(peer);
                }
            }
            Command::HeartbeatTick => {
                // A fresh beat; supersession in the state machine redacts
                // the previous one, so beats never accumulate.
                self.originate(vec![Entry::Presence {
                    peer: self.me,
                    name: self.name.clone(),
                    at: now,
                }])
                .await;
                let effects = self.state.sweep_stale(now);
                self.apply(effects);
            }
            Command::ExpiryDue { key } => {
                self.expiry_keys.remove(&key);
                self.state.forget(&key);
                self.known.redact([key]);
            }
            Command::Snapshot { reply } => {
                // A dropped receiver means the session task died first;
                // nothing to do.
                let _ = reply.send(self.known.rumors());
            }
            Command::JoinBack { snapshot } => self.join_back(snapshot).await,
            Command::Reset {
                known,
                observed,
                abandoned,
            } => self.reset(*known, observed, abandoned).await,
            Command::SessionOutcome { ok } => {
                if ok {
                    self.stats.sessions_ok += 1;
                } else {
                    self.stats.sessions_failed += 1;
                }
            }
            Command::Shutdown => unreachable!("run() intercepts Shutdown"),
        }
    }

    /// Insert entries we author, observing each through the state machine.
    async fn originate(&mut self, entries: Vec<Entry>) {
        let mut observed: Vec<(Key, Version, Arc<Entry>)> = Vec::new();
        self.known
            .message_then(entries, |key, version, entry| {
                observed.push((key, version.clone(), entry.clone()));
                async {}
            })
            .await;
        self.observe_all(observed);
    }

    /// Fold a gossiped snapshot back in: observe its gains, then diff away
    /// its losses (joins observe gains only — a key another peer redacted
    /// simply stops being live, and the diff is how that reaches the
    /// screen).
    async fn join_back(&mut self, snapshot: Known<Entry, Rumors>) {
        let mut observed: Vec<(Key, Version, Arc<Entry>)> = Vec::new();
        if self
            .known
            .join_then(snapshot, |key, version, entry| {
                observed.push((key, version.clone(), entry.clone()));
                async {}
            })
            .await
            .is_err()
        {
            // A snapshot from before a partition-merge reset: it belongs to
            // the universe we abandoned, and `join` hands it back untouched
            // on the network mismatch. Drop it.
            return;
        }
        self.observe_all(observed);
        let live = self.known.iter().map(|(key, _, _)| key).collect();
        for dead in self.state.retain_live(&live) {
            if let Some(slot) = self.expiry_keys.remove(&dead) {
                self.expiry.try_remove(&slot);
            }
            self.highlights.remove(&dead);
        }
    }

    /// Adopt a winning universe after a lost partition merge. The
    /// session-level verdict already arbitrated the merge; here we only
    /// check that the verdict still *applies* — that we are still in the
    /// universe it was computed against. Concurrent sessions can race two
    /// resets: the first to land adopts, the second no longer matches and
    /// is dropped (a future session against that universe re-arbitrates).
    async fn reset(
        &mut self,
        known: Known<Entry>,
        observed: Vec<(Key, Version, Arc<Entry>)>,
        abandoned: Network,
    ) {
        if abandoned != self.known.network() || known.network() == self.known.network() {
            // Stale or out-raced reset. Dropping `known` abandons the party
            // region the winner forked for us — a leak in a universe we are
            // not adopting, which is the acceptable cost of losing the race.
            return;
        }

        // The old universe is gone wholesale: state, timers, highlights.
        self.state = AppState::new();
        self.expiry.clear();
        self.expiry_keys.clear();
        self.highlights.clear();
        self.known = known;
        self.stats.merges += 1;
        self.merged_notice = Some(format!("merged into {}", network_short(&self.known)));

        self.observe_all(observed);

        // Re-announce ourselves in the new universe.
        let now = self.clock.now();
        let mut entries = vec![
            Entry::Presence {
                peer: self.me,
                name: self.name.clone(),
                at: now,
            },
            Entry::System {
                channel: HOME_CHANNEL.into(),
                body: format!("{} joined", self.name),
                at: now,
                ttl_ms: timers::SYSTEM_TTL.as_millis() as u64,
            },
        ];
        if !self.state.channels.contains_key(HOME_CHANNEL) {
            entries.push(Entry::Channel {
                name: HOME_CHANNEL.into(),
                created_by: self.me,
                at: now,
            });
        }
        self.originate(entries).await;
    }

    /// Run a batch of observations through the state machine and apply the
    /// effects.
    fn observe_all(&mut self, observed: Vec<(Key, Version, Arc<Entry>)>) {
        let now = self.clock.now();
        let mut effects = Vec::new();
        for (key, version, entry) in observed {
            effects.extend(self.state.observe(key, &version, &entry, now));
        }
        self.apply(effects);
    }

    /// Apply state-machine effects: batch the redactions into one act,
    /// schedule expiries, record highlights.
    fn apply(&mut self, effects: Vec<Effect>) {
        let now = self.clock.now();
        let mut redact = Vec::new();
        for effect in effects {
            match effect {
                Effect::Redact(key) => {
                    self.state.forget(&key);
                    if let Some(slot) = self.expiry_keys.remove(&key) {
                        self.expiry.try_remove(&slot);
                    }
                    redact.push(key);
                }
                Effect::Schedule { key, deadline } => {
                    let delay = std::time::Duration::from_millis(deadline.saturating_sub(now));
                    let slot = self.expiry.insert(key, delay);
                    self.expiry_keys.insert(key, slot);
                }
                Effect::InsertedMidList { key, .. } => {
                    self.highlights
                        .insert(key, Instant::now() + timers::HIGHLIGHT);
                }
            }
        }
        if !redact.is_empty() {
            self.known.redact(redact);
        }
    }

    /// Publish a fresh [`View`] snapshot.
    fn publish(&mut self) {
        let now = Instant::now();
        self.highlights.retain(|_, until| *until > now);

        let channels = self
            .state
            .channels
            .iter()
            .map(|(name, channel)| ChannelView {
                name: name.clone(),
                messages: channel
                    .list
                    .iter()
                    .map(|slot| {
                        let info = &self.state.messages[&slot.key];
                        MessageView {
                            key: slot.key,
                            author: info.author,
                            author_name: match &info.author {
                                Some(peer) => self.state.peer_name(peer),
                                None => "·".to_string(),
                            },
                            body: info.body.clone(),
                            at: info.at,
                            highlight_until: self.highlights.get(&slot.key).copied(),
                        }
                    })
                    .collect(),
            })
            .collect();

        let mut roster: Vec<PeerView> = self
            .state
            .presence
            .iter()
            .map(|(peer, rec)| PeerView {
                peer: *peer,
                name: rec.name.clone(),
                last_seen: rec.at,
            })
            .collect();
        roster.sort_by(|a, b| b.last_seen.cmp(&a.last_seen).then(a.peer.cmp(&b.peer)));

        // Manual targets that have shown up in the roster are now discovered
        // through the synced state itself; stop tracking them separately.
        self.dial_targets
            .retain(|peer| !self.state.presence.contains_key(peer));

        self.stats.live_entries = self.known.len();
        let view = View {
            me: self.me,
            me_display: self.me_display.clone(),
            name: self.name.clone(),
            network: network_short(&self.known),
            merged_notice: self.merged_notice.clone(),
            channels,
            roster,
            dial_targets: self.dial_targets.clone(),
            stats: self.stats,
        };
        self.view_tx.send_replace(Arc::new(view));
    }

    /// Say goodbye and hand the `Known` back for retirement, along with
    /// retire candidates ordered by presence recency.
    async fn shutdown(mut self) -> (Known<Entry>, Vec<PeerId>) {
        let now = self.clock.now();
        // The leave notice and our presence redaction ride out with the
        // retire session's built-in round of reconciliation.
        self.originate(vec![Entry::System {
            channel: HOME_CHANNEL.into(),
            body: format!("{} left", self.name),
            at: now,
            ttl_ms: timers::SYSTEM_TTL.as_millis() as u64,
        }])
        .await;
        if let Some(rec) = self.state.presence.get(&self.me) {
            let key = rec.key;
            self.state.forget(&key);
            self.known.redact([key]);
        }

        let mut candidates: Vec<(Millis, PeerId)> = self
            .state
            .presence
            .iter()
            .filter(|(peer, _)| **peer != self.me)
            .map(|(peer, rec)| (rec.at, *peer))
            .collect();
        candidates.sort_by(|a, b| b.cmp(a));
        (
            self.known,
            candidates.into_iter().map(|(_, peer)| peer).collect(),
        )
    }
}

/// A short, human-scannable identifier for the current universe, derived
/// from the `Network`'s debug form (`Network(<hex>)`).
fn network_short<S>(known: &Known<Entry, S>) -> String {
    let debug = format!("{:?}", known.network());
    let hex = debug.trim_start_matches("Network(").trim_end_matches(')');
    hex.chars().take(8).collect()
}

#[cfg(test)]
mod tests;
