//! The pure application state machine.
//!
//! [`AppState`] is the bridge between the rumor set and the screen: every
//! entry the local `Known` observes (originated, gossiped, or bootstrapped)
//! is fed through [`observe`](AppState::observe), which places it in the
//! display structures and returns the [`Effect`]s the caller must apply —
//! redactions to originate and expiries to schedule. The module does no I/O,
//! holds no `Known`, and reads no clock (the caller passes `now`), so the
//! whole lifecycle is testable with synthetic events.
//!
//! Two callbacks rumors does *not* provide shape this module:
//!
//! - Joins observe gains only; a message redacted by a peer simply stops
//!   being live. [`retain_live`](AppState::retain_live) diffs the tracked
//!   key set against the `Known`'s live keys after every join to notice
//!   removals. Ephemerality bounds the live set, so the diff stays cheap.
//! - Redacting locally fires no callback either, so the mutators here
//!   ([`sweep_stale`](AppState::sweep_stale), [`forget`](AppState::forget))
//!   drop their own bookkeeping in the same step that emits the
//!   [`Effect::Redact`].

use std::collections::{BTreeMap, HashMap, HashSet};

use rumors::{Key, Version};

use crate::causal::CausalList;
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
    /// The entry landed mid-list: it was delivered out of causal order and
    /// inserted between messages already on screen. The UI highlights it.
    InsertedMidList {
        /// The channel whose list the entry landed in.
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
    /// Display order, a linear extension of the causal partial order.
    pub list: CausalList<Key, Version>,
    /// Creation metadata, absent until (and unless) the [`Entry::Channel`]
    /// entry is observed — messages may arrive before their channel's
    /// creation entry, since delivery is unordered.
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

    /// Place an ephemeral message (chat or system notice) in its channel.
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
        let state = self.channels.entry(channel.to_string()).or_default();
        let len_before = state.list.len();
        let Some(index) = state.list.insert(key, version.clone()) else {
            return Vec::new(); // duplicate observation
        };
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
        if index < len_before {
            effects.push(Effect::InsertedMidList {
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

    /// Drop every tracked key not in `live` (the `Known`'s live key set,
    /// taken after a join): this is how peer-originated redactions reach the
    /// screen. Returns the dropped keys so the owner can cancel their expiry
    /// timers.
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
            channel.list.remove(key);
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
