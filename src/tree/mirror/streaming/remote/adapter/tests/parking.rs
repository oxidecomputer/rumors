//! What a parked decoded reply holds.
//!
//! The proxy parks every decoded reply this side can have invited at a
//! level, up to the in-flight count, until its stage takes it, and the
//! window prices each parked reply by its counts: reactions, listing
//! entries, and supplies. That pricing holds only if a decoded reply's size
//! does not scale with the subtrees it names.
//!
//! Supplied payloads stream through `Convert::assemble` into backend
//! custody while the reply retains one node handle per supplied node, and
//! a maximally disputed reply retains a pure skeleton of at most fan²
//! listing entries, each a radix, a hash, and a grandchild count. The coexistence of an encoded reply and
//! that decoded skeleton is pinned here by
//! [`DISPUTED_REPLY_TRANSIENT_CEILING`]: the encoded half is measured from
//! the codec, and the decoded half is derived from
//! `FAN² × size_of::<Listed>()`.

use crate::tree::mirror::streaming::message::{Listed, listed};
use std::mem;

use futures::{TryStreamExt, stream};

use crate::{
    message::Message,
    tree::{
        Action, Tree,
        mirror::streaming::{
            Backend, Local,
            erased::{Reaction, Reply},
            remote::codec::{self, RunBudget, Speaker, Stream},
            window::FAN,
        },
        typed::{self, Hash, height::UnderRoot},
    },
};

use super::{
    super::{LeafEdge, ReplyLevel, Scope, decode_reply, derive_questions, encode_reply},
    codec, hash, runtime, unbounded,
};

/// Leaves committed under the supplied root fan: enough that the fan's
/// child nodes are multi-leaf, so a handle demonstrably covers a subtree
/// larger than itself.
const LEAVES: u64 = 512;

/// The pinned coexistence transient of one maximally disputed reply.
///
/// One encoded and one decoded copy of the fan² skeleton resident at
/// once, plus per-frame signal and count framing, with ~3% headroom over
/// the measured 3,796,480 bytes.
/// Chosen tight so growth in either half — a wider hash, a larger fan,
/// heavier framing — fails the pin and forces the charge to be reconsidered.
const DISPUTED_REPLY_TRANSIENT_CEILING: usize = 3_910_000;

/// A parked decoded reply holds one node handle per supplied node — O(fan)
/// handles independent of how many leaves streamed through assembly into
/// backend custody, never a decoded copy of the subtree.
#[test]
fn parked_supply_reply_holds_handles_not_subtrees() {
    // The handle claim, compiler-checked: a backend node reference is one
    // shared pointer (an `Arc` bump to clone), so a parked reply holds one
    // reference per supplied node plus its skeleton, and the subtree each
    // names stays in backend custody behind the reference rather than being
    // copied into the reply.
    assert_eq!(
        mem::size_of::<typed::Node<UnderRoot>>(),
        mem::size_of::<usize>(),
        "a parked supply must be a shared handle, not an owned subtree",
    );

    let party = before::Party::seed();
    let mut tree = Tree::<()>::new();
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
        .map(|(radix, node)| (*radix, node.hash(), node.len()))
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
            .map(|(radix, node)| Reaction::Supply(radix, <Local as Backend>::erase(node)))
            .collect(),
        charge: Default::default(),
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
        .block_on(decode_reply::<Local, _>(
            &mut LeafEdge::new(),
            Local,
            u64::MAX,
            unbounded(),
            ReplyLevel::Branch,
            &scope.clone(),
            &mut frames,
            codec(),
        ))
        .expect("a canonical supplied fan decodes");

    // Pure supplies ask nothing: the parked form is the handle vector alone.
    assert!(
        derive_questions(ReplyLevel::Branch, &scope, &decoded)
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
                assert_eq!(node.hash(), *hash, "assembled custody matches the source");
                assert_eq!(node.len(), *len, "the handle covers the whole subtree");
            }
            _ => panic!("a supplied fan decodes to supply reactions only"),
        }
    }
}

/// A maximally disputed reply parks a bounded skeleton and no payload.
///
/// The skeleton is fan queries of fan listing entries each, and its
/// coexistence transient — the encoded reply and the decoded skeleton
/// resident at once, mid-decode — stays within the charged figure the
/// module doc states, pinned as measured encoded bytes plus the derived
/// decoded size so growth in either half fails loudly.
#[test]
fn maximally_disputed_reply_parks_bounded_skeleton() {
    let listing: Vec<Listed> = (0..FAN)
        .map(|radix| listed(radix as u8, hash(radix as u8)))
        .collect();
    let reply = Reply::<<Local as Backend>::Erased> {
        reactions: (0..FAN).map(|_| Reaction::Query(listing.clone())).collect(),
        charge: Default::default(),
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

    // The coexistence transient, held to the module doc's charged figure:
    // the encoded half measured off the codec (every frame's exact wire
    // bytes), the decoded half derived from the skeleton's in-memory
    // entries. Which logical stream carries the frames changes nothing:
    // every frame's opener names its stream in an item of one fixed width.
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
    let decoded_skeleton = FAN * FAN * mem::size_of::<Listed>();
    let transient = encoded_bytes + decoded_skeleton;
    eprintln!(
        "disputed-reply transient: {encoded_bytes} B encoded + \
         {decoded_skeleton} B decoded = {transient} B",
    );
    assert!(
        transient <= DISPUTED_REPLY_TRANSIENT_CEILING,
        "the coexistence transient (encoded {encoded_bytes} B plus decoded \
         skeleton {decoded_skeleton} B = {transient} B) must stay within \
         the memory model's charged figure of \
         {DISPUTED_REPLY_TRANSIENT_CEILING} B",
    );

    let mut frames = stream::iter(frames);
    let decoded = runtime
        .block_on(decode_reply::<Local, _>(
            &mut LeafEdge::new(),
            Local,
            u64::MAX,
            unbounded(),
            ReplyLevel::Branch,
            &Scope::opening_unbounded(&listing),
            &mut frames,
            codec(),
        ))
        .expect("a canonical maximally disputed reply decodes");

    // Every reaction parks as a listing skeleton; each registers exactly one
    // lower scope for the reply that will answer it.
    let questions = derive_questions(
        ReplyLevel::Branch,
        &Scope::opening_unbounded(&listing),
        &decoded,
    )
    .expect("decoding validated positions");
    assert_eq!(questions.len(), FAN);
    assert_eq!(decoded.reactions.len(), FAN);
    let mut entries = 0_usize;
    for reaction in &decoded.reactions {
        match reaction {
            Reaction::Query(nested) => entries += nested.len(),
            _ => panic!("a maximally disputed reply parks queries only"),
        }
    }
    assert_eq!(
        entries,
        FAN * FAN,
        "the parked skeleton is exactly fan² listing entries",
    );
}
