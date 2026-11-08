//! The room a question sets aside for its reply's chunks covers every chunk the
//! decoder parks, whatever the peer sends.
//!
//! The decoder returns the room beyond what a reply can still deliver as soon
//! as the reply shows it, so each later chunk must fit what is left. These
//! tests decode replies against a room charged on a ledger of their own and
//! read the ledger's count of bytes items needed beyond the room they were
//! built out of: an oracle independent of how the decoder prices what is left.

use std::{collections::BTreeSet, convert::Infallible, sync::OnceLock};

use futures::stream;
use proptest::prelude::*;

use crate::tree::mirror::streaming::{
    Backend, Local,
    byte_set::ByteSet,
    erased::Reply,
    holding::Holding,
    ledger::{Ledger, ReplyBound},
    materialized::parked_room,
    message::{Grandchildren, LaterMatches, Listed},
    remote::codec::{Flow, Frame, Reaction as WireReaction},
};
use crate::tree::typed::{Hash, Prefix, height::Z};

use super::{
    super::{BoundedPart, DecodeError, LeafEdge, ReplyLevel, Scope, ScopeError, decode_reply},
    LeafCase, codec, fixture_bounds, leaf_run, runtime, unbounded,
};

type E = <Local as Backend>::Erased;

/// What decoding one reply out of its room did: the result, the chunks it
/// delivered, and the bytes those chunks needed beyond the room.
struct Decoded {
    result: Result<(), DecodeError<Infallible>>,
    chunks: Vec<Reply<E>>,
    short: u64,
}

/// Decode `frames` as the reply to the opening question listing `asked`, whose
/// replier's grandchild count of the root is `count`, out of the room such a
/// question sets aside, on a ledger of its own.
fn decode_in_room(asked: &[Listed], count: usize, frames: Vec<Frame>) -> Decoded {
    let scope = Scope::opening(
        asked,
        Grandchildren::new(count).expect("the count is within a node's"),
    );
    let ledger = Ledger::default();
    let room = ledger.charge(
        Holding::ParkedReplies,
        parked_room::<E>(scope.reply_bound(), asked.len()),
    );
    let mut chunks = Vec::new();
    let result = runtime().block_on(async {
        let mut frames = stream::iter(frames);
        decode_reply::<Local, _>(
            &mut LeafEdge::new(),
            Local,
            &fixture_bounds(),
            unbounded(),
            ReplyLevel::Branch,
            &scope,
            &mut frames,
            codec(),
            Some(&crate::tree::typed::PathKey::fixture()),
            room,
            &mut |chunk| chunks.push(chunk),
        )
        .await
    });
    Decoded {
        result,
        chunks,
        short: ledger.short(),
    }
}

/// A child this side lists at `radix`, holding `children` children.
fn ours(radix: u8, children: u8) -> Listed {
    Listed {
        radix,
        hash: Hash::default(),
        grandchildren: Grandchildren::ONE,
        children_less_one: children - 1,
    }
}

/// A peer's listing of `count` children, at ascending radixes.
fn entries(count: usize) -> Vec<Listed> {
    (0..count)
        .map(|radix| {
            ours(
                u8::try_from(radix).expect("a listing holds a fan at most"),
                1,
            )
        })
        .collect()
}

/// A leaf whose keyed path sits under the root at each radix, by radix.
fn leaves_by_radix() -> &'static [LeafCase] {
    static LEAVES: OnceLock<Vec<LeafCase>> = OnceLock::new();
    LEAVES.get_or_init(|| {
        let mut found: Vec<Option<LeafCase>> = vec![None; usize::from(u8::MAX) + 1];
        let mut missing = found.len();
        for value in 0.. {
            let leaf = LeafCase::new(value, 1);
            let slot = &mut found[usize::from(<[u8; 32]>::from(leaf.path())[0])];
            if slot.is_none() {
                *slot = Some(leaf);
                missing -= 1;
                if missing == 0 {
                    break;
                }
            }
        }
        found
            .into_iter()
            .map(|leaf| leaf.expect("every radix was found"))
            .collect()
    })
}

/// A supply of the one leaf under the root at `radix`.
fn supply(radix: u8) -> WireReaction {
    let leaf = &leaves_by_radix()[usize::from(radix)];
    WireReaction::Supply(leaf_run(&[(&leaf.version, &leaf.message)]))
}

/// `reactions` as one reply's frames, its last carrying the end.
fn frames(reactions: Vec<WireReaction>) -> Vec<Frame> {
    let last = reactions.len().saturating_sub(1);
    reactions
        .into_iter()
        .enumerate()
        .map(|(index, reaction)| {
            let flow = if index == last {
                Flow::End
            } else {
                Flow::Continue
            };
            Frame::Reaction(reaction, flow)
        })
        .collect()
}

/// One reaction of a generated reply.
#[derive(Clone, Debug)]
enum Step {
    /// A supply of one leaf at this radix.
    Supply(u8),
    /// A match at the next position, covering the listed children whose
    /// positions this mask's bits pick, the lowest bit the first position.
    Match(u16),
    /// A query at the next position, listing this many children.
    Query(usize),
}

/// Questions of one to ten children at any radixes, each with one child up to a
/// full fan of them, and any replier's count up to a few fans of fans.
fn questions() -> impl Strategy<Value = (Vec<Listed>, usize)> {
    (
        prop::collection::vec(any::<u8>(), 1..=10),
        prop::collection::vec(1..=u8::MAX, 10),
        1..=4096_usize,
    )
        .prop_map(|(radixes, children, count): (Vec<u8>, Vec<u8>, usize)| {
            // A listing holds each radix once, ascending.
            let radixes: BTreeSet<u8> = radixes.into_iter().collect();
            let asked = radixes
                .into_iter()
                .zip(children)
                .map(|(radix, children)| ours(radix, children))
                .collect();
            (asked, count)
        })
}

/// Replies of one to a dozen reactions of every kind, in any order, at any
/// radixes and with any listing: mostly replies no conforming peer sends.
///
/// Supplies favor the top of the radix range, where a supply ahead of a
/// position leaves the fewest radixes above it, and queries favor empty
/// listings, which no listing count can refuse.
fn steps() -> impl Strategy<Value = Vec<Step>> {
    prop::collection::vec(
        prop_oneof![
            prop_oneof![any::<u8>(), u8::MAX - 15..=u8::MAX].prop_map(Step::Supply),
            any::<u16>().prop_map(Step::Match),
            prop_oneof![Just(0), 1..=usize::from(u8::MAX) + 1].prop_map(Step::Query),
        ],
        1..=12,
    )
}

/// The wire reactions `steps` stand for, against the question listing `asked`.
fn reactions(asked: &[Listed], steps: &[Step]) -> Vec<WireReaction> {
    steps
        .iter()
        .map(|step| match *step {
            Step::Supply(radix) => supply(radix),
            Step::Match(mask) => {
                let covered: ByteSet = asked
                    .iter()
                    .enumerate()
                    .filter(|&(position, _)| mask & (1 << position) != 0)
                    .map(|(_, listed)| listed.radix)
                    .collect();
                WireReaction::Match(LaterMatches::new(covered))
            }
            Step::Query(count) => WireReaction::Query(entries(count)),
        })
        .collect()
}

proptest! {
    /// No chunk the decoder parks needs more than the room its question
    /// set aside leaves for it, whatever reply arrives.
    ///
    /// The decoder returns room as soon as a reply shows it can no longer
    /// use it, so this holds only if every premise that release rests on
    /// is one the decoder checks: a reply it accepts, and every chunk it
    /// delivers before rejecting one, fits the room. The replies here mix
    /// supplies, matches with any covered set and queries with any listing,
    /// in any order and at any radix, against any counts.
    #[test]
    fn no_chunk_needs_more_than_its_room((asked, count) in questions(), steps in steps()) {
        let decoded = decode_in_room(&asked, count, frames(reactions(&asked, &steps)));
        prop_assert_eq!(
            decoded.short,
            0,
            "{} chunks fell {} B short; the decoder returned {:?}",
            decoded.chunks.len(),
            decoded.short,
            decoded.result,
        );
    }
}

/// A query listing more than the replier's count leaves beside the child counts
/// of the children the reply matched and covered fails at its frame.
///
/// Those children's listings never come, so their counts are counted as listed.
///
/// The question lists children at radixes 253, 254 and 255, the first two with
/// 200 children each, and the replier counts 200 grandchildren. The reply
/// matches 253 covering 254, which leaves nothing for a listing, and then
/// queries 255 listing 200 children.
#[test]
fn a_listing_past_what_matched_children_leave_fails() {
    let asked = [ours(253, 200), ours(254, 200), ours(255, 1)];
    let decoded = decode_in_room(
        &asked,
        200,
        frames(vec![
            WireReaction::Match(LaterMatches::new(std::iter::once(254).collect())),
            WireReaction::Query(entries(200)),
        ]),
    );
    assert!(
        matches!(
            decoded.result,
            Err(DecodeError::ExceedsBound {
                part: BoundedPart::Listing,
                bound: 200
            })
        ),
        "unexpected result: {:?}",
        decoded.result,
    );
    assert_eq!(decoded.short, 0, "no chunk falls short of its room");
}

/// A positional reaction below a supply's radix fails at its frame: a reply's
/// reactions ascend by radix.
///
/// The question lists children at radixes 10 and 20. The reply supplies a node
/// at radix 255 first, which leaves no radix above it for a later reaction,
/// then requests 10 and matches 20; the request's chunk holds the supply and
/// the request.
#[test]
fn a_position_below_a_supply_fails() {
    let asked = [ours(10, 1), ours(20, 1)];
    let decoded = decode_in_room(
        &asked,
        200,
        frames(vec![
            supply(u8::MAX),
            WireReaction::Query(Vec::new()),
            WireReaction::Match(LaterMatches::NONE),
        ]),
    );
    assert!(
        matches!(
            decoded.result,
            Err(DecodeError::Scope(ScopeError::Unordered {
                after: u8::MAX,
                radix: 10
            }))
        ),
        "unexpected result: {:?}",
        decoded.result,
    );
    assert_eq!(decoded.short, 0, "no chunk falls short of its room");
}

/// A reply to a leaf request that matches or asks fails at its frame, and parks
/// nothing beyond the request's room.
///
/// The request reserved room for one supplied leaf, and its record lists no
/// position: the reply supplies the leaf by its radix, or omits it. An empty
/// query and a match, the only positional reactions a reply about a leaf's
/// parent can carry, each fail as having no position to take.
#[test]
fn a_leaf_request_reply_takes_no_position() {
    let leaf = &leaves_by_radix()[0];
    let scope = Scope::leaf(Prefix::<Z>::containing(&leaf.path()).erase());
    assert_eq!(scope.reply_bound(), ReplyBound::LEAF);
    for (reaction, expected) in [
        (
            WireReaction::Query(Vec::new()),
            ScopeError::UnpositionedQuery,
        ),
        (
            WireReaction::Match(LaterMatches::NONE),
            ScopeError::UnpositionedMatch,
        ),
    ] {
        let ledger = Ledger::default();
        let room = ledger.charge(
            Holding::ParkedReplies,
            parked_room::<E>(ReplyBound::LEAF, 0),
        );
        let result = runtime().block_on(async {
            let mut frames = stream::iter(frames(vec![reaction]));
            decode_reply::<Local, _>(
                &mut LeafEdge::new(),
                Local,
                &fixture_bounds(),
                unbounded(),
                ReplyLevel::Leaf,
                &scope,
                &mut frames,
                codec(),
                Some(&crate::tree::typed::PathKey::fixture()),
                room,
                &mut |_| {},
            )
            .await
        });
        assert!(
            matches!(result, Err(DecodeError::Scope(error)) if error == expected),
            "expected {expected:?}, got {result:?}",
        );
        assert_eq!(ledger.short(), 0, "no chunk falls short of its room");
    }
}
