//! What a parked decoded reply holds.
//!
//! The proxy parks every decoded reply until its stage takes it, and each
//! question's reservation prices the reply it invites by its counts:
//! reactions, listing entries, and supplies. That pricing holds only if a
//! decoded reply's size does not scale with the subtrees it names.
//!
//! Supplied payloads stream through `Convert::assemble` into backend
//! custody while the reply retains one node handle per supplied node, and
//! a maximally disputed reply retains a pure skeleton of at most fan²
//! listing entries, each a radix, a hash, and a grandchild count.
//!
//! While such a reply decodes, its encoding and its decoded skeleton can be
//! resident at once. Both halves are measured here against the figures
//! they are derived from, so neither needs a ceiling of its own: the
//! encoded reply is exactly a fan of full-fan query frames
//! ([`MAX_QUERY_CHILDREN`] of [`FULL_FAN_QUERY_FRAME_LEN`] bytes each), and
//! the decoded skeleton holds exactly fan²
//! listing entries of `size_of::<Listed>()` bytes each.

use crate::tree::mirror::streaming::message::{Grandchildren, Listed};
use std::mem;

use futures::{TryStreamExt, stream};

use crate::{
    message::Message,
    tree::{
        Action, Tree,
        mirror::streaming::{
            Backend, ErasedNode, FAN, Local,
            erased::{Reaction, Reply},
            ledger::buffer,
            remote::codec::{
                self, FULL_FAN_QUERY_FRAME_LEN, MAX_QUERY_CHILDREN, RunBudget, Speaker, Stream,
            },
        },
        typed::{
            self, Hash, HashKey, Prefix,
            height::{Root, UnderRoot},
        },
    },
};

use super::{
    super::{
        LeafEdge, Replay, ReplyLevel, Scope, decode_whole_reply, derive_questions, encode_reply,
    },
    codec, fixture_bounds, hash, runtime, unbounded,
};

/// Leaves committed under the supplied root fan: enough that the fan's
/// child nodes are multi-leaf, so a handle demonstrably covers a subtree
/// larger than itself.
const LEAVES: u64 = 512;

/// A parked decoded reply holds one node handle per supplied node — O(fan)
/// handles independent of how many leaves streamed through assembly into
/// backend custody, never a decoded copy of the subtree.
#[test]
fn parked_supply_reply_holds_handles_not_subtrees() {
    // The handle claim, compiler-checked: a backend node reference is one
    // shared pointer (an `Arc` bump to clone), and the height it sits at
    // decides how many path bytes it hides, so a parked reply holds one
    // reference per supplied node plus its skeleton, and the subtree each
    // names stays in backend custody behind the reference rather than
    // being copied into the reply.
    assert_eq!(
        mem::size_of::<typed::View<UnderRoot>>(),
        mem::size_of::<usize>(),
        "a parked supply must be a shared handle, not an owned subtree",
    );

    let party = before::Party::seed();
    let mut tree = Tree::<()>::default();
    tree.act(&party, (0..LEAVES).map(|v| Action::Insert(Message::new(v))));
    let root = tree
        .root
        .node
        .clone()
        .expect("a populated tree has a root node");

    // The real root fan: version-addressed leaves scatter across first
    // bytes, so the fan is wide and its children are multi-leaf.
    let children: Vec<(u8, typed::Node<UnderRoot>)> = root.into_children().into_iter().collect();
    let expected: Vec<(u8, Hash, usize)> = children
        .iter()
        .map(|(radix, node)| (*radix, node.hash(&HashKey::fixture()), node.len()))
        .collect();
    assert!(
        expected.len() <= FAN,
        "a root fan never exceeds one child per radix byte",
    );
    assert!(
        expected.iter().any(|(_, _, len)| *len > 1),
        "the fixture must supply at least one multi-leaf subtree",
    );
    assert_eq!(
        expected.iter().map(|(_, _, len)| *len).sum::<usize>(),
        LEAVES as usize,
        "the fan partitions every committed leaf",
    );

    let reply = Reply {
        reactions: children
            .into_iter()
            .map(|(radix, node)| Reaction::Supply(radix, <Local as Backend>::erase(node.into())))
            .collect(),
        charge: Default::default(),
        last: true,
        carried: Default::default(),
    };
    let runtime = runtime();
    let scope = Scope::opening_unbounded(&[]);
    let frames = runtime.block_on(async {
        encode_reply(Local, RunBudget::default(), scope.clone(), reply)
            .map_ok(|encoded| encoded.into_parts().0)
            .try_collect::<Vec<_>>()
            .await
            .expect("the local backend is infallible")
    });

    let mut frames = stream::iter(frames);
    let decoded = runtime
        .block_on(decode_whole_reply::<Local, _>(
            &mut LeafEdge::new(),
            Local,
            &fixture_bounds(),
            unbounded(),
            ReplyLevel::Branch,
            &scope.clone(),
            &mut frames,
            codec(),
            Some(&crate::tree::typed::PathKey::fixture()),
        ))
        .expect("a canonical supplied fan decodes");

    // Pure supplies ask nothing: the parked form is the handle vector alone.
    assert!(
        derive_questions(
            ReplyLevel::Branch,
            &scope,
            &mut Replay::default(),
            &decoded.reactions
        )
        .expect("decoding validated positions")
        .is_empty()
    );
    // The parked cost is one handle per supplied node — the fan's width,
    // never the leaf count the run streamed through assembly.
    assert_eq!(decoded.reactions.len(), expected.len());
    assert!(
        decoded.reactions.len() < LEAVES as usize,
        "handle count must not scale with the subtree's leaves",
    );
    for (reaction, (radix, hash, len)) in decoded.reactions.iter().zip(&expected) {
        match reaction {
            Reaction::Supply(actual, node) => {
                assert_eq!(actual, radix);
                let at = Prefix::<Root>::new().push(*actual).erase();
                assert_eq!(
                    node.hash(&at, &HashKey::fixture()),
                    *hash,
                    "assembled custody matches the source"
                );
                assert_eq!(node.len(), *len, "the handle covers the whole subtree");
            }
            _ => panic!("a supplied fan decodes to supply reactions only"),
        }
    }
}

/// A maximally disputed reply parks a bounded skeleton and no payload, and
/// both its halves are exactly the figures they are derived from.
///
/// The skeleton is fan queries of fan listing entries each, every count at
/// its widest. Its encoding is exactly a fan of frames, each a full-fan
/// query frame ([`FULL_FAN_QUERY_FRAME_LEN`]). Its decoded form holds
/// exactly fan² listing entries, at exactly
/// their size, so the two resident at once mid-decode hold that sum and no
/// more.
#[test]
fn maximally_disputed_reply_parks_bounded_skeleton() {
    let listing: Vec<Listed> = (0..FAN)
        .map(|radix| Listed {
            radix: radix as u8,
            hash: hash(radix as u8),
            grandchildren: Grandchildren::MAX,
            children_less_one: 0,
        })
        .collect();
    let reply = Reply::<<Local as Backend>::Erased> {
        reactions: (0..FAN).map(|_| Reaction::Query(listing.clone())).collect(),
        charge: Default::default(),
        last: true,
        carried: Default::default(),
    };

    let runtime = runtime();
    let scope = Scope::opening_unbounded(&listing);
    let frames = runtime.block_on(async {
        encode_reply(Local, RunBudget::default(), scope.clone(), reply)
            .map_ok(|encoded| encoded.into_parts().0)
            .try_collect::<Vec<_>>()
            .await
            .expect("the local backend is infallible")
    });

    // The encoded half, measured off the codec: every frame's exact wire
    // bytes. Which logical stream carries the frames changes nothing: every
    // frame's opener names its stream in an item of one fixed width.
    let stream = Stream::new(11).expect("a valid logical stream index");
    let encoded_bytes: usize = frames
        .iter()
        .map(|frame| {
            let mut wire = Vec::new();
            codec::encode(Speaker::Initiator, &(stream, frame.clone()), &mut wire)
                .expect("a canonical disputed frame encodes");
            wire.len()
        })
        .sum();
    assert_eq!(
        encoded_bytes,
        MAX_QUERY_CHILDREN * FULL_FAN_QUERY_FRAME_LEN,
        "a maximally disputed reply encodes as a fan of full-fan query frames",
    );

    let mut frames = stream::iter(frames);
    let decoded = runtime
        .block_on(decode_whole_reply::<Local, _>(
            &mut LeafEdge::new(),
            Local,
            &fixture_bounds(),
            unbounded(),
            ReplyLevel::Branch,
            &Scope::opening_unbounded(&listing),
            &mut frames,
            codec(),
            Some(&crate::tree::typed::PathKey::fixture()),
        ))
        .expect("a canonical maximally disputed reply decodes");

    // Every reaction parks as a listing skeleton; each registers exactly one
    // lower scope for the reply that will answer it.
    let questions = derive_questions(
        ReplyLevel::Branch,
        &Scope::opening_unbounded(&listing),
        &mut Replay::default(),
        &decoded.reactions,
    )
    .expect("decoding validated positions");
    assert_eq!(questions.len(), FAN);
    assert_eq!(decoded.reactions.len(), FAN);
    // The decoded half, measured off the parked listings at their
    // capacities.
    let mut entries = 0_usize;
    let mut skeleton_bytes = 0_usize;
    for reaction in &decoded.reactions {
        match reaction {
            Reaction::Query(nested) => {
                entries += nested.len();
                skeleton_bytes += buffer(nested);
            }
            _ => panic!("a maximally disputed reply parks queries only"),
        }
    }
    assert_eq!(
        entries,
        FAN * FAN,
        "the parked skeleton is exactly fan² listing entries",
    );
    assert_eq!(
        skeleton_bytes,
        FAN * FAN * mem::size_of::<Listed>(),
        "the parked skeleton holds its entries at exactly their size",
    );
}
