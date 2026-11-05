//! The hop rule: a session's serialized hop count, predicted from the two
//! replicas' leaf paths without running the session.
//!
//! # Setting
//!
//! The rule describes a session on a connection whose one-way delay is the
//! unit of time, where computation takes no time, which the paused clock of
//! the tracer provides. It assumes two more things, which [`super`]'s
//! property asserts on every case:
//!
//! - Neither side's memory budget holds back a question or an answer: both
//!   sides report zero
//!   [`budget_waits`](rumors::SessionStats::budget_waits).
//! - The transport's in-flight window never fills, so no write waits for
//!   the peer to read.
//!
//! The rule says nothing about a session whose budget holds work back:
//! there a reply's chunk waits for deeper work to free memory, and that
//! wait can cost hops the rule does not count.
//!
//! # Terms
//!
//! The *depth* of a node is the length of its prefix in path bytes: the
//! root is at depth 0, its children at depth 1, and leaves at depth 32. The
//! protocol descends this trie one byte per level. The tree compresses
//! single-child chains internally, but a session sees a compressed node as
//! a node with one child at every byte it spans.
//!
//! An *exclusive leaf* is a leaf path that one replica holds and the other
//! lacks. Its *holder* is the replica that has it. A leaf can be exclusive
//! because its holder inserted it or because the other side redacted it.
//! The rule treats both cases alike, because it reads only which side holds
//! which paths.
//!
//! The *parting depth* `d` of an exclusive leaf is one more than the longest
//! prefix its path shares with any path the other replica holds. It is the
//! shallowest depth at which the leaf's prefix names a node only its holder
//! has, and it lies in `1..=32`. When the other replica is empty, `d` is 1.
//!
//! The *initiator* is the replica with fewer live messages; between equal
//! counts, it is the one whose latest version has the greater canonical
//! encoding. The other replica is the *responder*.
//!
//! Every question about a node lists the asker's children of that node, so
//! a question about a node at depth `k - 1` *lists depth* `k`. The
//! initiator's root question rides its greeting and lists depth 1. The
//! responder answers it, asking its own questions about depth-1 nodes, which
//! list depth 2. Questions alternate sides from then on, so the initiator
//! lists the odd depths and the responder the even ones.
//!
//! # The rule
//!
//! When the two replicas' latest versions are equal, the session ends at
//! the greeting exchange and takes 3 hops.
//!
//! Otherwise the session takes
//!
//! ```text
//! hops = 3 + max(1, max over exclusive leaves of d + [the holder lists depth d])
//! ```
//!
//! where `[P]` is 1 when `P` holds and 0 otherwise. That is: a session costs
//! three hops of setup and closing, plus one hop per level its deepest reply
//! sits at, and the responder's reply to the opening question always sits at
//! depth 1.
//!
//! The floor of 1 matters only for replicas whose versions differ while
//! their leaf sets are equal, such as two replicas that each inserted and
//! then redacted a message. They take 4 hops.
//!
//! # Why the rule holds
//!
//! The hops of a divergent session are these:
//!
//! 1. Both preambles cross in the first hop, and both greetings in the
//!    second; the initiator's greeting carries its root listing.
//! 2. The reply about the depth-`k` children of a node leaves at hop `k + 1`
//!    and lands at hop `k + 2`. Each question a reply asks ends one of the
//!    reply's chunks, and the receiver answers it as soon as that chunk
//!    arrives, so each level of questions costs exactly one hop.
//! 3. Each side writes its completion marker once its own work ends, so the
//!    last marker lands one hop after the last reply does.
//!
//! The session therefore takes `3 + D` hops, where `D` is the depth of its
//! deepest reply. The responder's opening reply puts `D` at 1 or more.
//!
//! A question about a node exists in two cases. In a *dispute*, both sides
//! hold the node, with different digests. A node's digest is a function of
//! the leaf paths beneath it, keyed by the network's secret, so digests
//! differ exactly when some leaf beneath the node is exclusive (barring a
//! collision of the keyed digest, which no one without the secret can
//! search for). In a *request*, the replier lacks a node the asker listed,
//! and asks for the whole node with an empty listing. A child that the
//! replier holds and the asker did not list is supplied outright, and a
//! supply asks nothing.
//!
//! Follow one exclusive leaf with parting depth `d`. Both sides hold its
//! ancestors at depths `0..d`, and each of those ancestors has the leaf
//! beneath it on one side only, so each is disputed. The question about the
//! ancestor at depth `d - 1` therefore draws a reply about depth `d`. At
//! depth `d`, only the holder has the leaf's node:
//!
//! - If the lacking side lists depth `d`, its listing omits the node, and
//!   the holder supplies it in the reply about depth `d`, which asks
//!   nothing more.
//! - If the holder lists depth `d`, the lacking side finds a listed node it
//!   lacks. Redaction leaves no tombstones, so it cannot tell a node whose
//!   leaves it redacted from one it never saw, and it requests the node.
//!   The holder's reply about depth `d + 1` answers the request, with a
//!   supply, or with nothing when the lacking side's version shows it
//!   redacted those leaves. Either way that reply crosses the wire.
//!
//! Every disputed node lies on the path of some exclusive leaf, above its
//! parting depth, so the deepest reply sits at the largest
//! `d + [the holder lists depth d]`, or at depth 1 when no leaf is
//! exclusive.
//!
//! At depth 32 the request is for a single leaf, and its answer travels on
//! the leaf stream rather than a deeper one. It still leaves one hop after
//! the request arrives, so the formula holds at every depth.
//!
//! # Message sizes
//!
//! Neither payload sizes nor the target message size enter the rule. A
//! reply ends a chunk at each question, and a supply run splits into
//! frames at the session's target message size. While admission never
//! waits, a side sends all of a reply's chunks and frames at the instant
//! the question they answer arrives, so splitting adds bytes but no hops.

use std::cmp::{Ordering, Reverse};

use rumors::Rumors;

/// One side of a session, by the tracer's label for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The side the tracer labels `'A'`.
    A,
    /// The side the tracer labels `'B'`.
    B,
}

impl Side {
    /// The tracer's label for this side.
    pub fn label(self) -> char {
        match self {
            Side::A => 'A',
            Side::B => 'B',
        }
    }

    /// The other side.
    pub fn other(self) -> Side {
        match self {
            Side::A => Side::B,
            Side::B => Side::A,
        }
    }
}

/// One replica as the rule reads it: its leaf paths and its latest version.
pub struct Replica {
    /// The replica's live leaf paths, in ascending order.
    paths: Vec<[u8; 32]>,
    /// The canonical encoding of the replica's latest version, which its
    /// greeting carries and the role election compares.
    latest: Vec<u8>,
}

impl Replica {
    /// Read the leaf paths and latest version of `rumors`'s current
    /// snapshot.
    pub fn of<T: Send + Sync + 'static>(rumors: &Rumors<T>) -> Self {
        let snapshot = rumors.snapshot();
        let mut paths: Vec<[u8; 32]> = snapshot
            .versions()
            .map(|version| rumors::testing::leaf_path(&snapshot, version))
            .collect();
        paths.sort_unstable();
        Replica {
            paths,
            latest: snapshot.latest().as_bytes().to_vec(),
        }
    }

    /// The longest prefix, in bytes, that `path` shares with any path this
    /// replica holds; zero when this replica is empty.
    ///
    /// In sorted order the paths sharing the longest prefix with `path` are
    /// its neighbors, so the two paths around its insertion point decide it.
    fn longest_shared_prefix(&self, path: &[u8; 32]) -> usize {
        let at = self.paths.partition_point(|held| held < path);
        let before = at.checked_sub(1).map(|i| &self.paths[i]);
        let after = self.paths.get(at);
        [before, after]
            .into_iter()
            .flatten()
            .map(|held| shared_prefix(held, path))
            .max()
            .unwrap_or(0)
    }
}

/// The length, in bytes, of the longest common prefix of two paths.
fn shared_prefix(a: &[u8; 32], b: &[u8; 32]) -> usize {
    a.iter().zip(b).take_while(|(a, b)| a == b).count()
}

/// The exclusive leaf that sets a session's deepest reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Critical {
    /// The leaf's path.
    pub path: [u8; 32],
    /// The side that holds the leaf.
    pub holder: Side,
    /// The leaf's parting depth.
    pub depth: usize,
    /// Whether the holder lists the parting depth, which costs the lacking
    /// side's request and one more hop.
    pub holder_lists: bool,
}

impl Critical {
    /// The depth of the reply this leaf draws: its parting depth, or one
    /// deeper when the holder lists that depth.
    fn reply_depth(&self) -> usize {
        self.depth + usize::from(self.holder_lists)
    }
}

/// What the rule predicts for a session between two replicas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prediction {
    /// The session's serialized hop count.
    pub hops: u64,
    /// The initiator, or `None` when equal versions end the session at the
    /// greeting, before roles are elected.
    pub initiator: Option<Side>,
    /// The exclusive leaf that sets the deepest reply, the first in path
    /// order among equals, or `None` when no leaf is exclusive.
    pub critical: Option<Critical>,
}

/// The hops of the setup and closing: preambles, greetings, and the
/// completion markers.
const SETUP_AND_CLOSING_HOPS: u64 = 3;

/// The depth of the responder's reply to the opening question, the
/// shallowest reply a divergent session has.
const OPENING_REPLY_DEPTH: usize = 1;

/// Predict the session between replica `a`, on side [`Side::A`], and
/// replica `b`, on side [`Side::B`], by the rule in the module
/// documentation.
pub fn predict(a: &Replica, b: &Replica) -> Prediction {
    if a.latest == b.latest {
        return Prediction {
            hops: SETUP_AND_CLOSING_HOPS,
            initiator: None,
            critical: None,
        };
    }
    let initiator = match a.paths.len().cmp(&b.paths.len()) {
        Ordering::Less => Side::A,
        Ordering::Greater => Side::B,
        Ordering::Equal if a.latest > b.latest => Side::A,
        Ordering::Equal => Side::B,
    };
    let replica = |side| match side {
        Side::A => a,
        Side::B => b,
    };
    let lister = |depth: usize| {
        if depth % 2 == 1 {
            initiator
        } else {
            initiator.other()
        }
    };

    let critical = [Side::A, Side::B]
        .into_iter()
        .flat_map(|holder| {
            let (held, other) = (replica(holder), replica(holder.other()));
            held.paths
                .iter()
                .filter(|path| other.paths.binary_search(path).is_err())
                .map(move |path| {
                    let depth = other.longest_shared_prefix(path) + 1;
                    Critical {
                        path: *path,
                        holder,
                        depth,
                        holder_lists: lister(depth) == holder,
                    }
                })
        })
        .max_by_key(|leaf| (leaf.reply_depth(), Reverse(leaf.path)));

    let deepest_reply = critical
        .map_or(OPENING_REPLY_DEPTH, |leaf| leaf.reply_depth())
        .max(OPENING_REPLY_DEPTH);
    Prediction {
        hops: SETUP_AND_CLOSING_HOPS + deepest_reply as u64,
        initiator: Some(initiator),
        critical,
    }
}
