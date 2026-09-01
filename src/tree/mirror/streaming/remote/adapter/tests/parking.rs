//! Memory accounting for parked decoded replies.
//!
//! The session's memory model is RAM-sound only because a decoded reply
//! parked between decode and consumption costs O(fan) node *handles* —
//! never a materialized subtree. The one-slot response relay
//! (`proxy/work/queues.rs`) bounds decoded replies to one in flight per
//! stage, and any wire backlog behind that slot sits in the transport's
//! own buffers as encoded bytes; both charges price a parked reply at its
//! decoded size, so that size must not scale with the subtree it names.
//!
//! Supplied payloads stream through `Convert::assemble` into backend
//! custody while the reply retains one pointer-sized handle per supplied
//! node, and a maximally disputed reply retains a pure skeleton of at most
//! fan² `(radix, hash)` entries: ≈ 1.1 MB encoded, ≈ 2.2 MB while the
//! encoded and decoded forms coexist mid-decode. That coexistence
//! transient is the figure the session's memory model charges per parked
//! reply (`streaming/message.rs`), and it is pinned here as a sum of the
//! encoded half, measured off the codec, and the decoded half, derived
//! from `FAN² × size_of::<(u8, Hash)>()`. These tests hold both shapes to
//! that accounting.

use crate::message::{PayloadCodec, PayloadDepthLimit};
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
    super::{Scope, decode_reply, encode_reply},
    hash, runtime, unbounded,
};

/// Leaves committed under the supplied root fan: enough that the fan's
/// child nodes are multi-leaf, so a handle demonstrably covers a subtree
/// larger than itself.
const LEAVES: u64 = 512;

/// The pinned coexistence transient of one maximally disputed reply.
///
/// One encoded and one decoded copy of the fan² skeleton resident at
/// once, plus per-frame signal and count framing, with ~3% headroom.
/// Chosen tight so growth in either half — a wider hash, a larger fan,
/// heavier framing — fails the pin and forces the module doc's charged
/// figure (and `streaming/message.rs`, which states it) to be
/// re-derived rather than silently going stale.
const DISPUTED_REPLY_TRANSIENT_CEILING: usize = 3_570_000;

/// A parked decoded reply holds one pointer-sized node handle per supplied
/// node — O(fan) handles independent of how many leaves streamed through
/// assembly into backend custody, never a decoded copy of the subtree.
#[test]
fn parked_supply_reply_holds_handles_not_subtrees() {
    // The handle claim, compiler-checked: a backend node reference is one
    // shared pointer (an `Arc` bump to clone), so parking a reply costs
    // `replies.len()` pointers plus the skeleton — the subtree's bytes live
    // in backend custody behind the handle.
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
        .root
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
        replies: children
            .into_iter()
            .map(|(radix, node)| Reaction::Supply(radix, <Local as Backend>::erase(node)))
            .collect(),
    };
    let runtime = runtime();
    let scope = Scope::opening(&[]);
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
            Local,
            u64::MAX,
            unbounded(),
            scope,
            &mut frames,
            PayloadCodec::new::<u64>(PayloadDepthLimit::default()),
        ))
        .expect("a canonical supplied fan decodes");

    // Pure supplies ask nothing: the parked form is the handle vector alone.
    assert!(decoded.questions.is_empty());
    // The parked cost is one handle per supplied node — the fan's width,
    // never the leaf count the run streamed through assembly.
    assert_eq!(decoded.reply.replies.len(), expected.len());
    assert!(
        decoded.reply.replies.len() < LEAVES as usize,
        "handle count must not scale with the subtree's leaves",
    );
    for (reaction, (radix, hash, len)) in decoded.reply.replies.iter().zip(&expected) {
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
    let listing: Vec<(u8, Hash)> = (0..FAN)
        .map(|radix| (radix as u8, hash(radix as u8)))
        .collect();
    let reply = Reply::<<Local as Backend>::Erased> {
        replies: (0..FAN).map(|_| Reaction::Query(listing.clone())).collect(),
    };

    let runtime = runtime();
    let scope = Scope::opening(&listing);
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
    // entries. The stream label prices nothing: the frame opener is
    // per-frame and the same width on every stream.
    let stream = Stream::new(11).expect("a valid data stream index");
    let encoded_bytes: usize = frames
        .iter()
        .map(|frame| {
            let mut wire = Vec::new();
            codec::encode(Speaker::Initiator, &(stream, frame.clone()), &mut wire)
                .expect("a canonical disputed frame encodes");
            wire.len()
        })
        .sum();
    let decoded_skeleton = FAN * FAN * mem::size_of::<(u8, Hash)>();
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
            Local,
            u64::MAX,
            unbounded(),
            Scope::opening(&listing),
            &mut frames,
            PayloadCodec::new::<u64>(PayloadDepthLimit::default()),
        ))
        .expect("a canonical maximally disputed reply decodes");

    // Every reaction parks as a listing skeleton; each registers exactly one
    // lower scope for the reply that will answer it.
    assert_eq!(decoded.questions.len(), FAN);
    assert_eq!(decoded.reply.replies.len(), FAN);
    let mut entries = 0_usize;
    for reaction in &decoded.reply.replies {
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
