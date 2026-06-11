//! The actor that owns the room's view of the rumor set.
//!
//! One task holds the primary [`Broadcast`] handle, the [`Messages`]
//! observer, the [`AppState`] display machine, and the expiry wheel.
//! Everything else talks to it through a [`Command`] channel and reads back
//! through a [`watch`]-published [`View`] snapshot.
//!
//! The wiring is deadlock-free by construction: the owner never awaits
//! another task. [`Command::Handle`] is answered immediately (a
//! [`Broadcast`] clone shares the internally-synchronized set), and
//! publishing uses [`watch::Sender::send_replace`]. Every wait-for edge
//! points from a connection task toward the owner, so the wait-for graph is
//! acyclic.
//!
//! Connection tasks gossip on their own [`Broadcast`] clones; whatever a
//! session learns lands in the shared set and reaches the owner through its
//! [`Messages`] observer, folded into the same select loop the commands
//! arrive on. Redactions learned from a peer are silent (the leaf is simply
//! gone), so the owner diffs its display state against the live key set
//! after every finished session ([`Command::SessionOutcome`]) and on every
//! heartbeat. A lost partition merge arrives as [`Command::Reset`], which
//! swaps the whole world out.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use futures::{FutureExt, StreamExt};
use rumors::{Broadcast, Key, Known, Messages, Network, Version};
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
    /// Hand out a [`Broadcast`] clone for a gossip session. The clone shares
    /// the owner's internally-synchronized set, so whatever the session
    /// learns is immediately visible to the owner's observer; there is no
    /// fold-back step.
    Handle {
        /// Where to send the handle.
        reply: oneshot::Sender<Broadcast<Entry>>,
    },
    /// We lost a partition merge: adopt the winning universe wholesale.
    Reset {
        /// The freshly bootstrapped rumor set in the winning network. Its
        /// content reaches the display through the owner's fresh observer,
        /// which replays the set from genesis.
        known: Box<Known<Entry>>,
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
    /// The primary actor handle; sessions gossip on clones of it.
    broadcast: Broadcast<Entry>,
    /// The pull-based observer: every entry that becomes live in the set —
    /// originated here, learned by any session's gossip, or replayed after
    /// a reset — comes through exactly once.
    observer: Messages<Entry>,
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
        let broadcast = known.broadcast();
        let observer = broadcast.messages();
        let owner = Owner {
            broadcast,
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
        };
        (owner, view_rx)
    }

    /// Drive the actor until [`Command::Shutdown`], then return the `Known`
    /// (for retirement) and the retire candidates, most recently seen first.
    ///
    /// The heartbeat interval, the expiry wheel, and the [`Messages`]
    /// observer are folded into the same select loop the channel feeds, so
    /// every state transition flows through [`handle`](Self::handle) or
    /// [`observe_all`](Self::observe_all).
    pub async fn run(mut self, mut rx: mpsc::Receiver<Command>) -> (Known<Entry>, Vec<PeerId>) {
        /// One turn of the owner loop: either a command or an observation.
        enum Turn {
            Cmd(Command),
            Observed((Key, Version, Arc<Entry>)),
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
                Turn::Cmd(cmd) => self.handle(cmd),
                Turn::Observed(observed) => self.observe_all(vec![observed]),
            }
            self.publish();
        }
        self.shutdown().await
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
                self.sweep_losses();
            }
            Command::ExpiryDue { key } => {
                self.expiry_keys.remove(&key);
                self.state.forget(&key);
                self.broadcast.redact(key);
            }
            Command::Handle { reply } => {
                // A dropped receiver means the session task died first;
                // nothing to do.
                let _ = reply.send(self.broadcast.clone());
            }
            Command::Reset { known, abandoned } => self.reset(*known, abandoned),
            Command::SessionOutcome { ok } => {
                if ok {
                    self.stats.sessions_ok += 1;
                } else {
                    self.stats.sessions_failed += 1;
                }
                // The session may have learned peer redactions, which are
                // silent (the leaf is simply gone): diff the display state
                // against the live set.
                self.sweep_losses();
            }
            Command::Shutdown => unreachable!("run() intercepts Shutdown"),
        }
    }

    /// Insert entries we author, then drain the observer inline so the
    /// state machine sees them before the next publish.
    fn originate(&mut self, entries: Vec<Entry>) {
        {
            let mut batch = self.broadcast.batch();
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
        let snapshot = self.broadcast.snapshot();
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
    fn reset(&mut self, known: Known<Entry>, abandoned: Network) {
        if abandoned != self.broadcast.network() || known.network() == self.broadcast.network() {
            // Stale or out-raced reset. Dropping `known` abandons the party
            // region the winner forked for us — a leak in a universe we are
            // not adopting, which is the acceptable cost of losing the race.
            return;
        }

        // The old universe is gone wholesale: state, timers, highlights,
        // handle, observer. Stale `Broadcast` clones still inside session
        // tasks keep talking to the abandoned set until those sessions end;
        // their universe loses every future merge verdict, so nothing they
        // do can leak back in.
        self.state = AppState::new();
        self.expiry.clear();
        self.expiry_keys.clear();
        self.highlights.clear();
        self.broadcast = known.broadcast();
        // A fresh observer replays the adopted universe from genesis; the
        // inline drain below runs it through the state machine so the merge
        // lands on screen atomically with the network switch.
        self.observer = self.broadcast.messages();
        self.stats.merges += 1;
        self.merged_notice = Some(format!(
            "merged into {}",
            network_short(self.broadcast.network())
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
                Effect::InsertedMidList { key, .. } => {
                    self.highlights
                        .insert(key, Instant::now() + timers::HIGHLIGHT);
                }
            }
        }
        if !redact.is_empty() {
            let mut batch = self.broadcast.batch();
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

        self.stats.live_entries = self.broadcast.len();
        let view = View {
            me: self.me,
            me_display: self.me_display.clone(),
            name: self.name.clone(),
            network: network_short(self.broadcast.network()),
            merged_notice: self.merged_notice.clone(),
            channels,
            roster,
            dial_targets: self.dial_targets.clone(),
            stats: self.stats,
        };
        self.view_tx.send_replace(Arc::new(view));
    }

    /// Say goodbye, reclaim the `Known` from the broadcast generation, and
    /// hand it back for retirement along with retire candidates ordered by
    /// presence recency.
    async fn shutdown(mut self) -> (Known<Entry>, Vec<PeerId>) {
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
            self.broadcast.redact(key);
        }

        let mut candidates: Vec<(Millis, PeerId)> = self
            .state
            .presence
            .iter()
            .filter(|(peer, _)| **peer != self.me)
            .map(|(peer, rec)| (rec.at, *peer))
            .collect();
        candidates.sort_by(|a, b| b.cmp(a));

        // Reunite resolves once every session's handle clone has dropped
        // (the caller tears the gossip tasks down alongside us); we are the
        // only reuniter, so the `Known` always comes back to us.
        let known = self
            .broadcast
            .reunite()
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
