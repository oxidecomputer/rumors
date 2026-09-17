//! Behavioral specification for adapting wire frames into protocol replies.
//!
//! The suite checks valid and malformed replies, batching, bounded parking,
//! and the reader/assembler occupancy limit across every protocol height.

use std::{collections::BTreeMap, ops::Range};

use before::{Party, Version};

use crate::{
    message::{Message, PayloadCodec, PayloadDepthLimit},
    tree::{
        mirror::streaming::{
            convert::Convert,
            materialized::SupplyLedger,
            remote::codec::{End, Flow, Frame, LeafRun, Reaction as WireReaction},
        },
        typed::{
            self, Hash, Path,
            hash::MERKLE_HASH_LEN,
            height::{S, Z},
        },
    },
};

/// Dispatch a runtime height to its concrete marker type.
macro_rules! at_height {
    ($height:expr, $trait:ident::$method:ident($($argument:expr),*); $start:literal..$end:literal) => {
        seq_macro::seq!(N in $start..$end {
            match $height {
                #(N => <crate::tree::typed::height::H~N as $trait>::$method($($argument),*),)*
                _ => panic!("reply height {} is outside {}..{}", $height, $start, $end),
            }
        })
    };
}

mod backend_errors;
mod fan_occupancy;
mod malformed;
mod opening;
mod parking;
mod properties;
mod runs;

/// Build a visibly synthetic Merkle hash for scope fixtures.
fn hash(byte: u8) -> Hash {
    Hash([byte; MERKLE_HASH_LEN])
}

/// A set-length allowance no fixture here can exhaust, for tests whose
/// subject is not the ingress supply charge.
fn unbounded() -> SupplyLedger {
    SupplyLedger::new(u64::MAX)
}

/// Construct the payload codec shared by adapter fixtures.
fn codec() -> PayloadCodec {
    PayloadCodec::new::<u64>(PayloadDepthLimit::default())
}

/// Build a version with the same event count everywhere.
fn uniform_version(ticks: u64) -> Version {
    let mut version = Version::new();
    Party::seed().ticks(&mut version, ticks);
    version
}

/// Build a supply run from borrowed leaf records, in the given order.
fn leaf_run(records: &[(&Version, &Message)]) -> LeafRun {
    let mut run = LeafRun::new();
    for (version, message) in records {
        run.push(version, message)
            .expect("a test record fits the run framing");
    }
    run
}

/// Frame one reply's reactions with the canonical final marker.
fn reply_frames(reactions: impl IntoIterator<Item = WireReaction>) -> Vec<Frame> {
    let reactions: Vec<_> = reactions.into_iter().collect();
    if reactions.is_empty() {
        return vec![Frame::End(End::Reply)];
    }
    let last = reactions.len() - 1;
    reactions
        .into_iter()
        .enumerate()
        .map(|(position, reaction)| {
            Frame::Reaction(
                reaction,
                if position == last {
                    Flow::End
                } else {
                    Flow::Continue
                },
            )
        })
        .collect()
}

/// Find `count` leaves which share a root radix, ordered by path.
fn colliding_leaves(count: usize) -> Vec<LeafCase> {
    let mut by_radix: BTreeMap<u8, Vec<LeafCase>> = BTreeMap::new();
    for value in 0..u64::MAX {
        let leaf = LeafCase::new(value, value as u8 % 4);
        let radix = <[u8; 32]>::from(leaf.path())[0];
        let group = by_radix.entry(radix).or_default();
        group.push(leaf);
        if group.len() == count {
            group.sort_by_key(LeafCase::path);
            return std::mem::take(group);
        }
    }
    unreachable!("the finite radix alphabet forces {count} collisions")
}

/// Find two leaves with distinct root radixes, ordered by path.
fn separated_leaves() -> [LeafCase; 2] {
    let first = LeafCase::new(0, 0);
    let first_radix = <[u8; 32]>::from(first.path())[0];
    for value in 1..u64::MAX {
        let leaf = LeafCase::new(value, value as u8 % 4);
        if <[u8; 32]>::from(leaf.path())[0] != first_radix {
            let mut pair = [first, leaf];
            pair.sort_by_key(LeafCase::path);
            return pair;
        }
    }
    unreachable!("version-derived paths span more than one root radix")
}

/// Build leaves for `values` with fixed ticks, ordered by content path.
fn ascending_leaves(values: Range<u64>, ticks: u8) -> Vec<LeafCase> {
    let mut leaves: Vec<_> = values.map(|value| LeafCase::new(value, ticks)).collect();
    leaves.sort_by_key(LeafCase::path);
    leaves
}

/// Construct the single-threaded runtime used to drive adapter futures.
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("test runtime")
}

/// A deterministic leaf fixture with its payload and derived version path.
#[derive(Clone, Debug)]
struct LeafCase {
    value: u64,
    version: Version,
    message: Message,
}

impl LeafCase {
    /// Construct a leaf whose low 56 value bits and ticks select its version.
    fn new(value: u64, ticks: u8) -> Self {
        Self {
            value,
            version: uniform_version(value.wrapping_shl(8) | u64::from(ticks)),
            message: Message::new(value),
        }
    }

    /// Return the content path derived from this leaf's version.
    fn path(&self) -> Path {
        Path::for_leaf(&self.version)
    }
}

/// Build the same one-leaf subtree at any adapter height.
trait NodeAt: Convert {
    /// Build this height's subtree around `leaf`.
    fn node(leaf: &LeafCase) -> typed::Node<Self>;
}

/// A leaf is already a height-zero subtree.
impl NodeAt for Z {
    fn node(leaf: &LeafCase) -> typed::Node<Self> {
        typed::Node::leaf(leaf.version.clone(), leaf.message.clone())
    }
}

/// Wrap the lower subtree in the branch selected by the leaf's path.
impl<H> NodeAt for S<H>
where
    H: NodeAt,
    S<H>: Convert,
{
    fn node(leaf: &LeafCase) -> typed::Node<Self> {
        let path: [u8; 32] = leaf.path().into();
        typed::Node::beneath(H::node(leaf), path[31 - H::HEIGHT])
    }
}
