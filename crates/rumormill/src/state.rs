//! The pure application state machine.
//!
//! [`AppState`] is the bridge between the rumor set and the screen: every
//! entry the local set observes (originated, gossiped, or bootstrapped)
//! is fed through [`observe`](AppState::observe), which places it in the
//! display structures and returns the [`Effect`]s the caller must apply —
//! redactions to originate and expiries to schedule. The module does no I/O,
//! holds no `Known`, and reads no clock (the caller passes `now`), so the
//! whole lifecycle is testable with synthetic events.
//!
//! **The input contract is causal delivery**: the owner feeds `observe`
//! from a [`CausalMessages`](rumors::CausalMessages) observer, so an entry
//! never arrives before one it causally depends on. Display order is
//! therefore plain arrival order — appending is the whole placement
//! algorithm — and the only ordering question left per arrival is whether
//! it causally follows the channel's tail
//! ([`Effect::ConcurrentArrival`] when it does not).
//!
//! Two signals rumors does *not* deliver shape this module:
//!
//! - Observers see gains only; a message redacted by a peer simply stops
//!   being live. [`retain_live`](AppState::retain_live) diffs the tracked
//!   key set against the set's live keys after every finished session to
//!   notice removals. Ephemerality bounds the live set, so the diff stays
//!   cheap.
//! - Redacting locally is silent too, so the mutators here
//!   ([`sweep_stale`](AppState::sweep_stale), [`forget`](AppState::forget))
//!   drop their own bookkeeping in the same step that emits the
//!   [`Effect::Redact`].

use std::collections::{BTreeMap, HashMap, HashSet};

use rumors::{Key, Version};

use crate::entry::{Entry, Millis, PeerId};
use crate::timers;

/// What the owner must do after a state transition. The state machine never
/// touches the `Known` or any timer directly; it describes the work and the
/// owner applies it.
#[derive(Debug, PartialEq, Eq)]
pub enum Effect {
    /// Redact this key from the rumor set (and thus, contagiously, from
    /// every peer).
    Redact(Key),
    /// Schedule a local expiry: redact `key` once the wall clock reaches
    /// `deadline`.
    Schedule {
        /// The key to redact at the deadline.
        key: Key,
        /// Wall-clock deadline in epoch milliseconds.
        deadline: Millis,
    },
    /// The entry does not causally follow the message displayed above it:
    /// it arrived from a *concurrent* line of history (a gossip burst
    /// delivering messages composed elsewhere, in ignorance of the channel
    /// tail). The UI highlights it as the boundary of merged-in history.
    ConcurrentArrival {
        /// The channel the entry landed in.
        channel: String,
        /// The entry's key.
        key: Key,
    },
}

/// A message placed in a channel: everything the UI needs to render one line.
#[derive(Debug, Clone)]
pub struct MessageInfo {
    /// The channel the message is displayed in.
    pub channel: String,
    /// The author, or `None` for system notices.
    pub author: Option<PeerId>,
    /// The message or notice text.
    pub body: String,
    /// Wall-clock timestamp, cosmetic only.
    pub at: Millis,
}

/// One channel: causally ordered message keys plus creation metadata.
#[derive(Debug, Default)]
pub struct ChannelState {
    /// Display order: plain arrival order, which is a linear extension of
    /// the causal partial order because the owner observes through
    /// [`CausalMessages`](rumors::CausalMessages) — a message is never
    /// delivered before one it causally depends on, so appending preserves
    /// causal consistency with no placement logic at all.
    pub list: Vec<Key>,
    /// The version of the most recently appended message: the causal tail
    /// the next arrival is compared against to detect a
    /// [`ConcurrentArrival`](Effect::ConcurrentArrival).
    tail: Option<Version>,
    /// Creation metadata, absent until (and unless) the [`Entry::Channel`]
    /// entry is observed — messages may arrive before their channel's
    /// creation entry, since concurrent entries carry no delivery order.
    pub created_by: Option<PeerId>,
}

/// The latest known heartbeat for one peer.
#[derive(Debug, Clone)]
pub struct PresenceRec {
    /// The key of the presence entry, for supersession and eviction.
    pub key: Key,
    /// The causal version it was observed at, for supersession.
    pub version: Version,
    /// The peer's display name.
    pub name: String,
    /// Wall-clock beat time, for the staleness sweep.
    pub at: Millis,
}

/// The application state: everything on screen, derived purely from observed
/// entries.
#[derive(Debug, Default)]
pub struct AppState {
    /// Channels by name (sorted for display).
    pub channels: BTreeMap<String, ChannelState>,
    /// Render data for every displayed message, across all channels.
    pub messages: HashMap<Key, MessageInfo>,
    /// Latest presence per peer.
    pub presence: HashMap<PeerId, PresenceRec>,
    /// Reverse index: presence entry key to peer, for the removal diff.
    presence_keys: HashMap<Key, PeerId>,
    /// Keys of observed [`Entry::Channel`] entries, for the removal diff.
    /// Losing one forgets creation metadata but keeps the channel: messages
    /// may still reference it.
    channel_keys: HashMap<Key, String>,
}

impl AppState {
    /// An empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply one newly observed entry and return the effects to apply.
    ///
    /// `now` is the caller's wall clock; an ephemeral entry already past its
    /// deadline is not displayed at all, just redacted (idempotent across
    /// peers doing the same).
    pub fn observe(
        &mut self,
        key: Key,
        version: &Version,
        entry: &Entry,
        now: Millis,
    ) -> Vec<Effect> {
        match entry {
            Entry::Chat {
                channel,
                author,
                body,
                sent_at,
                ..
            } => self.place_message(
                key,
                version,
                channel,
                Some(*author),
                body,
                *sent_at,
                entry.expires_at().expect("chat is ephemeral"),
                now,
            ),
            Entry::System {
                channel, body, at, ..
            } => self.place_message(
                key,
                version,
                channel,
                None,
                body,
                *at,
                entry.expires_at().expect("system notices are ephemeral"),
                now,
            ),
            Entry::Presence { peer, name, at } => {
                self.place_presence(key, version, *peer, name, *at)
            }
            Entry::Channel {
                name, created_by, ..
            } => {
                let channel = self.channels.entry(name.clone()).or_default();
                // First creation entry observed wins the metadata; a
                // concurrent duplicate only adds its key to the index.
                channel.created_by.get_or_insert(*created_by);
                self.channel_keys.insert(key, name.clone());
                Vec::new()
            }
        }
    }

    /// Append an ephemeral message (chat or system notice) to its channel.
    ///
    /// Append is all the placement there is: the caller feeds entries in
    /// causal delivery order, so the list is a linear extension of
    /// causality by construction. The only question left is whether the
    /// newcomer causally follows the current tail — if not, it opens a
    /// stretch of merged-in concurrent history, and the UI highlights it.
    #[allow(clippy::too_many_arguments)] // private plumbing shared by two variants
    fn place_message(
        &mut self,
        key: Key,
        version: &Version,
        channel: &str,
        author: Option<PeerId>,
        body: &str,
        at: Millis,
        deadline: Millis,
        now: Millis,
    ) -> Vec<Effect> {
        if deadline <= now {
            // Expired before we ever displayed it; redact rather than show.
            return vec![Effect::Redact(key)];
        }
        if self.messages.contains_key(&key) {
            return Vec::new(); // duplicate observation
        }
        let state = self.channels.entry(channel.to_string()).or_default();
        // Concurrent to (or somehow behind) the tail: news from elsewhere.
        // A successor extends the conversation and is not flagged.
        let concurrent = state
            .tail
            .as_ref()
            .is_some_and(|tail| version.partial_cmp(tail) != Some(std::cmp::Ordering::Greater));
        state.list.push(key);
        state.tail = Some(version.clone());
        self.messages.insert(
            key,
            MessageInfo {
                channel: channel.to_string(),
                author,
                body: body.to_string(),
                at,
            },
        );
        let mut effects = vec![Effect::Schedule { key, deadline }];
        if concurrent {
            effects.push(Effect::ConcurrentArrival {
                channel: channel.to_string(),
                key,
            });
        }
        effects
    }

    /// Place a heartbeat: the dominating presence per peer wins and the
    /// loser is redacted, so the rumor set never accumulates stale beats.
    fn place_presence(
        &mut self,
        key: Key,
        version: &Version,
        peer: PeerId,
        name: &str,
        at: Millis,
    ) -> Vec<Effect> {
        let rec = PresenceRec {
            key,
            version: version.clone(),
            name: name.to_string(),
            at,
        };
        match self.presence.get(&peer) {
            None => {
                self.presence.insert(peer, rec);
                self.presence_keys.insert(key, peer);
                Vec::new()
            }
            Some(existing) => {
                // Newer wins by causality; concurrent beats (possible after
                // a partition merge) tie-break on (at, key), which both
                // sides resolve identically.
                let incoming_wins = match version.partial_cmp(&existing.version) {
                    Some(std::cmp::Ordering::Greater) => true,
                    Some(_) => false,
                    None => (at, key) > (existing.at, existing.key),
                };
                if incoming_wins {
                    let loser = existing.key;
                    self.presence_keys.remove(&loser);
                    self.presence_keys.insert(key, peer);
                    self.presence.insert(peer, rec);
                    vec![Effect::Redact(loser)]
                } else {
                    vec![Effect::Redact(key)]
                }
            }
        }
    }

    /// Drop every tracked key not in `live` (the set's live keys, taken
    /// after a finished session): this is how peer-originated redactions
    /// reach the screen. Returns the dropped keys so the owner can cancel
    /// their expiry timers.
    pub fn retain_live(&mut self, live: &HashSet<Key>) -> Vec<Key> {
        let dead: Vec<Key> = self
            .messages
            .keys()
            .chain(self.presence_keys.keys())
            .chain(self.channel_keys.keys())
            .filter(|k| !live.contains(*k))
            .copied()
            .collect();
        for key in &dead {
            self.forget(key);
        }
        dead
    }

    /// Drop one key from whatever display structure holds it. Used when the
    /// owner redacts locally (expiry, staleness) and by the removal diff;
    /// unknown keys are a no-op, mirroring redaction's idempotence.
    pub fn forget(&mut self, key: &Key) {
        if let Some(info) = self.messages.remove(key)
            && let Some(channel) = self.channels.get_mut(&info.channel)
        {
            channel.list.retain(|k| k != key);
        }
        if let Some(peer) = self.presence_keys.remove(key) {
            // Only drop the roster entry if it still points at this key; a
            // newer beat may have superseded it already.
            if self.presence.get(&peer).is_some_and(|rec| rec.key == *key) {
                self.presence.remove(&peer);
            }
        }
        self.channel_keys.remove(key);
    }

    /// Evict peers whose newest heartbeat is older than
    /// [`timers::PRESENCE_STALE`]: drop them from the roster and redact
    /// their presence so we stop gossiping them. Any peer may run this
    /// sweep; concurrent eviction is idempotent, and a revived peer simply
    /// publishes a fresh beat.
    pub fn sweep_stale(&mut self, now: Millis) -> Vec<Effect> {
        let threshold = timers::PRESENCE_STALE.as_millis() as Millis;
        let stale: Vec<Key> = self
            .presence
            .values()
            .filter(|rec| rec.at.saturating_add(threshold) <= now)
            .map(|rec| rec.key)
            .collect();
        stale
            .iter()
            .map(|key| {
                self.forget(key);
                Effect::Redact(*key)
            })
            .collect()
    }

    /// The display name for a peer, falling back to a short hex of its id.
    pub fn peer_name(&self, peer: &PeerId) -> String {
        match self.presence.get(peer) {
            Some(rec) => rec.name.clone(),
            None => hex::encode(&peer[..4]),
        }
    }
}

#[cfg(test)]
mod tests;
