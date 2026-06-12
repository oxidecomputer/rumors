//! The actor that owns the room's view of the rumor set.
//!
//! One task holds the primary [`Rumors`] handle, the [`CausalMessages`]
//! observer, the [`AppState`] display machine, and the expiry wheel. Everything
//! else talks to it through a [`Command`] channel and reads back through a
//! [`watch`]-published [`View`] snapshot.
//!
//! The wiring is deadlock-free by construction: the owner never awaits another
//! task. [`Command::Handle`] is answered immediately (a [`Rumors`] clone
//! shares the internally-synchronized set), and publishing uses
//! [`watch::Sender::send_replace`]. Every wait-for edge points from a
//! connection task toward the owner, so the wait-for graph is acyclic.
//!
//! Connection tasks gossip on their own [`Rumors`] clones; whatever a
//! session learns lands in the shared set and reaches the owner through its
//! [`CausalMessages`] observer, folded into the same select loop the commands
//! arrive on. Redactions learned from a peer are silent (the leaf is simply
//! gone), so the owner diffs its display state against the live key set
//! whenever a finished session ([`Command::SessionOutcome`]) or a heartbeat
//! has marked a sweep pending. The sweep and the view publish are both
//! O(display state) while session outcomes arrive at mesh rate, so neither
//! runs per event: a coalescing tick ([`timers::VIEW_COALESCE`]) executes
//! whatever is pending at most ten times a second. A lost partition merge
//! arrives as [`Command::Reset`], which swaps the whole world out.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use futures::{FutureExt, StreamExt};
use rumors::{CausalMessages, Key, Network, Peer, Rumors, Version};
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
    /// Hand out a [`Rumors`] clone for a gossip session. The clone shares
    /// the owner's internally-synchronized set, so whatever the session
    /// learns is immediately visible to the owner's observer; there is no
    /// fold-back step.
    Handle {
        /// Where to send the handle.
        reply: oneshot::Sender<Rumors<Entry>>,
    },
    /// We lost a partition merge: adopt the winning universe wholesale.
    Reset {
        /// The freshly bootstrapped rumor set in the winning network. Its
        /// content reaches the display through the owner's fresh observer,
        /// which replays the set from genesis.
        known: Box<Peer<Entry>>,
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
        /// The universe the session's handle belonged to, `None` when no
        /// session was ever established (dial failures). An `ok` outcome
        /// in the *current* universe proves our state — presence included —
        /// has reached a peer; that is the signal that makes a manual dial
        /// target safely droppable (see [`Owner::synced`]). Outcomes from
        /// an abandoned universe must not count: they can arrive queued
        /// behind the [`Reset`](Command::Reset) that abandoned them.
        network: Option<Network>,
    },
    /// Leave the room: the run loop returns the `Peer` for retirement.
    Shutdown,
}

/// The owner actor. Construct with [`Owner::new`], then drive with
/// [`Owner::run`].
pub struct Owner {
    /// The primary actor handle; sessions gossip on clones of it.
    rumors: Rumors<Entry>,
    /// The pull-based observer: every entry that becomes live in the set —
    /// originated here, learned by any session's gossip, or replayed after
    /// a reset — comes through exactly once.
    observer: CausalMessages<Entry>,
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
    /// A session has completed in the *current* universe, proving our
    /// presence has reached at least one peer. Until then, manual dial
    /// targets stay dialable even when they appear in the roster: a node
    /// that just reset may hold the roster without holding a single link
    /// (the smaller-id-dials rule can put every pair's dialing duty on the
    /// *other* side, and the other side cannot know us until we gossip) —
    /// dropping its targets then would strand it in a room that has never
    /// heard of it.
    synced: bool,
    /// State changed since the last publish; the coalescing tick publishes.
    dirty: bool,
    /// A session finished (or a heartbeat fired) since the last loss sweep;
    /// the coalescing tick sweeps before it publishes.
    sweep_pending: bool,
}

impl Owner {
    /// Build an owner around a freshly seeded `Peer` and hand back the view
    /// channel the UI (and the connector) will read.
    pub fn new(
        known: Peer<Entry>,
        me: PeerId,
        me_display: String,
        name: String,
        clock: Clock,
    ) -> (Self, watch::Receiver<Arc<View>>) {
        let (view_tx, view_rx) = watch::channel(Arc::new(View::default()));
        let rumors = known.into_rumors();
        let observer = rumors.causal_messages();
        let owner = Owner {
            rumors,
            observer,
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
            synced: false,
            dirty: false,
            sweep_pending: false,
        };
        (owner, view_rx)
    }

    /// Drive the actor until [`Command::Shutdown`], then return the `Peer`
    /// (for retirement) and the retire candidates, most recently seen first.
    ///
    /// The heartbeat interval, the expiry wheel, the coalescing tick, and
    /// the [`CausalMessages`] observer are folded into the same select loop
    /// the channel feeds, so every state transition flows through
    /// [`handle`](Self::handle) or [`observe_all`](Self::observe_all).
    pub async fn run(mut self, mut rx: mpsc::Receiver<Command>) -> (Peer<Entry>, Vec<PeerId>) {
        /// One turn of the owner loop: a command, an observation, or the
        /// coalescing tick that executes deferred sweeps and publishes.
        enum Turn {
            Cmd(Command),
            Observed((Key, Version, Arc<Entry>)),
            Refresh,
        }

        // Announce ourselves to the (initially one-node) universe, presence
        // included: the first handle given out must already carry it, and
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
        ]);
        self.publish();

        let mut heartbeat = tokio::time::interval(timers::HEARTBEAT_INTERVAL);
        let mut refresh = tokio::time::interval(timers::VIEW_COALESCE);
        loop {
            // The select borrows the wheel and the observer; bundling its
            // outcome into a `Turn` ends those borrows before `self` is
            // borrowed again to act on it.
            let turn = {
                let expiry = &mut self.expiry;
                let observer = &mut self.observer;
                tokio::select! {
                    Some(cmd) = rx.recv() => Turn::Cmd(cmd),
                    _ = heartbeat.tick() => Turn::Cmd(Command::HeartbeatTick),
                    _ = refresh.tick() => Turn::Refresh,
                    // The guard keeps an empty wheel from being polled at all;
                    // `DelayQueue` ends its stream when empty rather than
                    // registering a waker for future inserts.
                    Some(expired) = expiry.next(), if !expiry.is_empty() => {
                        Turn::Cmd(Command::ExpiryDue { key: expired.into_inner() })
                    }
                    // Entries learned by sessions gossiping on their handle
                    // clones; our own originations are drained inline by
                    // `originate`, so this arm sees only what peers taught us.
                    Some(observed) = observer.next() => Turn::Observed(observed),
                }
            };
            match turn {
                Turn::Cmd(Command::Shutdown) => break,
                Turn::Cmd(cmd) => {
                    self.handle(cmd);
                    self.dirty = true;
                }
                Turn::Observed(observed) => {
                    self.observe_all(vec![observed]);
                    self.dirty = true;
                }
                Turn::Refresh => self.refresh(),
            }
        }
        self.shutdown().await
    }

    /// One coalescing tick: run the deferred loss sweep, then publish a
    /// fresh view if any turn changed state since the last one. Sweep
    /// before publish, so a view never shows a key the sweep is about to
    /// drop.
    fn refresh(&mut self) {
        if self.sweep_pending {
            self.sweep_pending = false;
            self.sweep_losses();
        }
        if self.dirty {
            self.dirty = false;
            self.publish();
        }
    }

    /// Apply one command to the state machine.
    fn handle(&mut self, cmd: Command) {
        let now = self.clock.now();
        match cmd {
            Command::SendChat { channel, body } => {
                self.originate(vec![Entry::Chat {
                    channel,
                    author: self.me,
                    body,
                    sent_at: now,
                    ttl_ms: timers::CHAT_TTL.as_millis() as u64,
                }]);
            }
            Command::CreateChannel { name } => {
                self.originate(vec![Entry::Channel {
                    name,
                    created_by: self.me,
                    at: now,
                }]);
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
                }]);
                let effects = self.state.sweep_stale(now);
                self.apply(effects);
                self.sweep_pending = true;
            }
            Command::ExpiryDue { key } => {
                self.expiry_keys.remove(&key);
                self.state.forget(&key);
                self.rumors.redact(key);
            }
            Command::Handle { reply } => {
                // A dropped receiver means the session task died first;
                // nothing to do.
                let _ = reply.send(self.rumors.clone());
            }
            Command::Reset { known, abandoned } => self.reset(*known, abandoned),
            Command::SessionOutcome { ok, network } => {
                if ok {
                    self.stats.sessions_ok += 1;
                    // Only a session in the universe we are still in counts
                    // as proof of reachability; see the field docs.
                    if network == Some(self.rumors.network()) {
                        self.synced = true;
                    }
                } else {
                    self.stats.sessions_failed += 1;
                }
                // The session may have learned peer redactions, which are
                // silent (the leaf is simply gone): mark the live-set diff
                // pending. Deferred, not run inline — outcomes arrive at
                // mesh rate, and the diff walks the whole live set.
                self.sweep_pending = true;
            }
            Command::Shutdown => unreachable!("run() intercepts Shutdown"),
        }
    }

    /// Insert entries we author, then drain the observer inline so the
    /// state machine sees them before the next publish.
    fn originate(&mut self, entries: Vec<Entry>) {
        {
            let mut batch = self.rumors.batch();
            for entry in entries {
                batch.send(entry);
            }
        }
        self.drain_observer();
    }

    /// Pull everything the observer has pending — without blocking — and
    /// run it through the state machine.
    fn drain_observer(&mut self) {
        let mut observed: Vec<(Key, Version, Arc<Entry>)> = Vec::new();
        while let Some(Some(item)) = self.observer.next().now_or_never() {
            observed.push(item);
        }
        self.observe_all(observed);
    }

    /// Diff the display state against the live key set: observers see gains
    /// only — a key another peer redacted simply stops being live, and this
    /// diff is how that reaches the screen.
    fn sweep_losses(&mut self) {
        let snapshot = self.rumors.snapshot();
        let live = snapshot.iter().map(|(key, _, _)| key).collect();
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
    fn reset(&mut self, known: Peer<Entry>, abandoned: Network) {
        if abandoned != self.rumors.network() || known.network() == self.rumors.network() {
            // Stale or out-raced reset. Dropping `known` abandons the party
            // region the winner forked for us — a leak in a universe we are
            // not adopting, which is the acceptable cost of losing the race.
            crate::trace::trace(|| {
                format!(
                    "reset declined: abandoned {abandoned:?}, adopted-candidate {:?}, current {:?}",
                    known.network(),
                    self.rumors.network()
                )
            });
            return;
        }
        crate::trace::trace(|| {
            format!(
                "reset adopted: {:?} -> {:?}",
                self.rumors.network(),
                known.network()
            )
        });

        // The old universe is gone wholesale: state, timers, highlights,
        // handle, observer. Stale `Rumors` clones still inside connection
        // tasks keep talking to the abandoned set only until the publish
        // below reaches them: every drive watches `View::universe` and
        // tears down when its handle no longer matches (net.rs). Nothing a
        // stale drive does in the meantime can leak back in — its universe
        // loses every future merge verdict.
        self.state = AppState::new();
        self.expiry.clear();
        self.expiry_keys.clear();
        self.highlights.clear();
        // Nobody in the adopted universe has heard from us yet; manual
        // dial targets become load-bearing again until one session lands.
        self.synced = false;
        self.rumors = known.into_rumors();
        // A fresh observer replays the adopted universe from genesis; the
        // inline drain below runs it through the state machine so the merge
        // lands on screen atomically with the network switch.
        self.observer = self.rumors.causal_messages();
        self.stats.merges += 1;
        self.merged_notice = Some(format!(
            "merged into {}",
            network_short(self.rumors.network())
        ));

        self.drain_observer();

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
        self.originate(entries);
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
                Effect::ConcurrentArrival { key, .. } => {
                    self.highlights
                        .insert(key, Instant::now() + timers::HIGHLIGHT);
                }
            }
        }
        if !redact.is_empty() {
            let mut batch = self.rumors.batch();
            for key in redact {
                batch.redact(key);
            }
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
                    .map(|&key| {
                        let info = &self.state.messages[&key];
                        MessageView {
                            key,
                            author: info.author,
                            author_name: match &info.author {
                                Some(peer) => self.state.peer_name(peer),
                                None => "·".to_string(),
                            },
                            body: info.body.clone(),
                            at: info.at,
                            highlight_until: self.highlights.get(&key).copied(),
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

        // Manual targets that have shown up in the roster are discovered
        // through the replicated state itself — but only once one of our
        // sessions has actually carried our presence into this universe.
        // Before that, the roster is adopted hearsay: it lists peers who
        // have never heard of us, and the pair rule may oblige none of
        // them to dial us (see the `synced` field docs).
        if self.synced {
            self.dial_targets
                .retain(|peer| !self.state.presence.contains_key(peer));
        }

        // One consistent snapshot serves both gauges.
        let snapshot = self.rumors.snapshot();
        self.stats.live_entries = snapshot.len();
        let view = View {
            me: self.me,
            me_display: self.me_display.clone(),
            name: self.name.clone(),
            network: network_short(snapshot.network()),
            universe: Some(snapshot.network()),
            merged_notice: self.merged_notice.clone(),
            channels,
            roster,
            dial_targets: self.dial_targets.clone(),
            stats: self.stats,
        };
        self.view_tx.send_replace(Arc::new(view));
    }

    /// Say goodbye, reclaim the unique `Peer` from the `Rumors` clones, and
    /// hand it back for retirement along with retire candidates ordered by
    /// presence recency.
    async fn shutdown(mut self) -> (Peer<Entry>, Vec<PeerId>) {
        let now = self.clock.now();
        // The leave notice and our presence redaction ride out with the
        // retire session's built-in round of reconciliation.
        self.originate(vec![Entry::System {
            channel: HOME_CHANNEL.into(),
            body: format!("{} left", self.name),
            at: now,
            ttl_ms: timers::SYSTEM_TTL.as_millis() as u64,
        }]);
        if let Some(rec) = self.state.presence.get(&self.me) {
            let key = rec.key;
            self.state.forget(&key);
            self.rumors.redact(key);
        }

        let mut candidates: Vec<(Millis, PeerId)> = self
            .state
            .presence
            .iter()
            .filter(|(peer, _)| **peer != self.me)
            .map(|(peer, rec)| (rec.at, *peer))
            .collect();
        candidates.sort_by(|a, b| b.cmp(a));

        // `try_into_peer` resolves once every session's handle clone has
        // dropped (the caller tears the gossip tasks down alongside us); we
        // are the only reuniter, so the `Peer` always comes back to us.
        let known = self
            .rumors
            .try_into_peer()
            .await
            .expect("the owner is the sole reuniter");
        (
            known,
            candidates.into_iter().map(|(_, peer)| peer).collect(),
        )
    }
}

/// A short, human-scannable identifier for a universe, derived from the
/// `Network`'s debug form (`Network(<hex>)`).
fn network_short(network: Network) -> String {
    let debug = format!("{network:?}");
    let hex = debug.trim_start_matches("Network(").trim_end_matches(')');
    hex.chars().take(8).collect()
}

#[cfg(test)]
mod tests;
