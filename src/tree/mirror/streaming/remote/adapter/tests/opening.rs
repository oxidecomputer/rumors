//! The distinguished opening reply: the question that rides the greeting,
//! and the early supplies that trail it.
//!
//! No wire frame exists for the opening question: [`opening_parts`]
//! validates the locally produced opening reply and splits it into the
//! listing the root scope derives from and the early supplies that do
//! cross, while [`opening_reply`] replays the peer greeting's listing as
//! the message the responder answers. The two must agree on the scope,
//! since one side builds it from its own message and the other from the
//! listing that crossed the wire.

use crate::message::{PayloadCodec, PayloadDepthLimit};
use before::Version;
use futures::{TryStreamExt, stream};

use crate::message::Message;
use crate::tree::{
    mirror::streaming::{
        Backend, Local,
        erased::{Reaction, Reply},
        remote::codec::{End, Flow, Frame, Reaction as WireReaction},
    },
    typed::{
        self, Prefix,
        height::{Height, S, UnderRoot, Z},
    },
};

/// One erased opening node over the unit payload.
fn erased(node: typed::Node<UnderRoot>) -> <Local as Backend>::Erased {
    <Local as Backend>::erase(node)
}

use super::{
    super::{DecodeError, OpeningError, Scope, early_supplies, opening_parts, opening_reply},
    LeafCase, hash, leaf_run, runtime, unbounded,
};

trait OpeningNode: Height {
    fn node() -> typed::Node<Self>;
}

impl OpeningNode for Z {
    fn node() -> typed::Node<Self> {
        typed::Node::leaf(Version::new(), Message::new(()))
    }
}

impl<H: OpeningNode> OpeningNode for S<H>
where
    S<H>: Height,
{
    fn node() -> typed::Node<Self> {
        typed::Node::beneath(H::node(), 0)
    }
}

/// The local opening's listing derives the same scope the remote side
/// replays from the listing carried in the greeting.
#[test]
fn opening_listing_agrees_with_greeting_replay() {
    let listing = vec![(3, hash(1)), (9, hash(2))];
    let reply = Reply::<<Local as Backend>::Erased> {
        replies: vec![Reaction::Query(listing.clone())],
    };

    let (split, supplies) = opening_parts(reply).expect("canonical opening");
    assert_eq!(split, listing);
    assert!(supplies.is_empty(), "no supplies trailed the question");
    let scope = Scope::opening(&split);

    let (replayed, replayed_scope) = opening_reply::<<Local as Backend>::Erased>(listing.clone());
    assert_eq!(replayed_scope, scope);
    let [Reaction::Query(replayed)] = replayed.replies.as_slice() else {
        panic!("the replayed opening must remain one query")
    };
    assert_eq!(replayed, &listing);
}

/// Early supplies trailing the opening question split off intact, in order.
#[test]
fn opening_supplies_split_off_the_question() {
    let listing = vec![(3, hash(1))];
    let reply = Reply::<<Local as Backend>::Erased> {
        replies: vec![
            Reaction::Query(listing.clone()),
            Reaction::Supply(5, erased(UnderRoot::node())),
            Reaction::Supply(9, erased(UnderRoot::node())),
        ],
    };
    let (split, supplies) = opening_parts(reply).expect("canonical opening with supplies");
    assert_eq!(split, listing);
    let radices: Vec<u8> = supplies
        .iter()
        .map(|reaction| match reaction {
            Reaction::Supply(radix, _) => *radix,
            _ => panic!("only supplies trail the opening question"),
        })
        .collect();
    assert_eq!(radices, [5, 9]);
}

/// An empty carried listing replays the empty-tree initiator's opening: one
/// empty `Query`, meaning "I lack the root, send everything", with a root
/// scope holding no positional children.
#[test]
fn empty_listing_replays_the_empty_opening() {
    let (replayed, mut scope) = opening_reply::<<Local as Backend>::Erased>(Vec::new());
    let [Reaction::Query(listing)] = replayed.replies.as_slice() else {
        panic!("the replayed opening must be one query")
    };
    assert!(listing.is_empty());
    assert_eq!(scope, Scope::opening(&[]));
    assert_eq!(scope.next(), None, "an empty root scope positions nothing");
}

/// The opening-supply reply decodes into whole root children, one per
/// version-derived radix group, in ascending radix order.
#[test]
fn opening_supplies_decode_by_radix_group() {
    // Enough cases that at least two distinct first bytes exist; the
    // version-derived paths pick the grouping.
    let mut cases: Vec<LeafCase> = (0..6).map(|i| LeafCase::new(1_000 + i, 1)).collect();
    cases.sort_by_key(LeafCase::path);
    let first_byte = |case: &LeafCase| <[u8; 32]>::from(case.path())[0];
    let mut groups: Vec<Vec<&LeafCase>> = Vec::new();
    for case in &cases {
        match groups.last_mut() {
            Some(group) if first_byte(group[0]) == first_byte(case) => {
                group.push(case);
            }
            _ => groups.push(vec![case]),
        }
    }
    assert!(groups.len() >= 2, "the fixture must span two root children");

    let mut frames: Vec<Frame> = groups
        .iter()
        .map(|group| {
            let records: Vec<_> = group
                .iter()
                .map(|case| (&case.version, &case.message))
                .collect();
            Frame::Reaction(WireReaction::Supply(leaf_run(&records)), Flow::Continue)
        })
        .collect();
    let closing = match frames.pop().expect("at least one group") {
        Frame::Reaction(reaction, _) => Frame::Reaction(reaction, Flow::End),
        end => end,
    };
    frames.push(closing);

    let decoded: Vec<(u8, _)> = runtime()
        .block_on(
            early_supplies::<Local, _>(
                Local,
                u64::MAX,
                unbounded(),
                Prefix::new().erase(),
                stream::iter(frames),
                PayloadCodec::mint::<u64>(PayloadDepthLimit::default()),
            )
            .try_collect(),
        )
        .expect("a canonical opening-supply reply decodes");
    let radices: Vec<u8> = decoded.iter().map(|(radix, _)| *radix).collect();
    let expected: Vec<u8> = groups.iter().map(|group| first_byte(group[0])).collect();
    assert_eq!(radices, expected, "one node per group, in radix order");
    for ((_, node), group) in decoded.iter().zip(&groups) {
        assert_eq!(
            node.len(),
            group.len(),
            "each node holds its group's leaves"
        );
    }
}

/// The opening-supply reply is held to the declared set length record by
/// record: the first record past the allowance fails the decode typed,
/// while the one opening reply is still open.
///
/// The same fixture as the radix-group decode above, under an allowance
/// of one: the eager early path charges at ingress exactly as the
/// per-reply decoder does, so an over-declaring initiator cannot ride
/// the opening stream past its greeting.
#[test]
fn opening_supplies_past_the_declared_set_len_are_rejected() {
    use crate::tree::mirror::streaming::materialized::SupplyLedger;

    let mut cases: Vec<LeafCase> = (0..6).map(|i| LeafCase::new(1_000 + i, 1)).collect();
    cases.sort_by_key(LeafCase::path);
    let records: Vec<_> = cases
        .iter()
        .map(|case| (&case.version, &case.message))
        .collect();
    let frames: Vec<Frame> = vec![Frame::Reaction(
        WireReaction::Supply(leaf_run(&records)),
        Flow::End,
    )];

    let error = runtime()
        .block_on(async {
            early_supplies::<Local, _>(
                Local,
                u64::MAX,
                SupplyLedger::new(1),
                Prefix::new().erase(),
                stream::iter(frames),
                PayloadCodec::mint::<u64>(PayloadDepthLimit::default()),
            )
            .try_collect::<Vec<_>>()
            .await
        })
        .expect_err(
            "undetected over-supply: opening supplies past the declared set \
             length must fail at ingress",
        );
    assert!(
        matches!(error, DecodeError::OverdrawnSupply { declared: 1 }),
        "mistyped over-supply rejection: {error:?}",
    );
}

/// An empty opening-supply reply — the whole early set pruned away —
/// decodes to no supplies at all.
#[test]
fn empty_opening_supply_reply_decodes_to_nothing() {
    let frames: Vec<Frame> = vec![Frame::End(End::Reply)];
    let decoded: Vec<(u8, _)> = runtime()
        .block_on(
            early_supplies::<Local, _>(
                Local,
                u64::MAX,
                unbounded(),
                Prefix::new().erase(),
                stream::iter(frames),
                PayloadCodec::mint::<u64>(PayloadDepthLimit::default()),
            )
            .try_collect(),
        )
        .expect("an empty batch decodes cleanly");
    assert!(decoded.is_empty());
}

/// The opening-supply stream carries exactly one reply: frames after its
/// end are rejected, not absorbed into a phantom second reply.
#[test]
fn second_opening_supply_reply_is_rejected() {
    let frames: Vec<Frame> = vec![Frame::End(End::Reply), Frame::End(End::Reply)];
    let error = runtime()
        .block_on(async {
            early_supplies::<Local, _>(
                Local,
                u64::MAX,
                unbounded(),
                Prefix::new().erase(),
                stream::iter(frames),
                PayloadCodec::mint::<u64>(PayloadDepthLimit::default()),
            )
            .try_collect::<Vec<_>>()
            .await
        })
        .expect_err("a second reply on the opening-supply stream is invalid");
    assert!(matches!(error, DecodeError::ExtraOpeningReply));
}

/// Positional reactions are unrepresentable in the opening-supply grammar;
/// an in-process one is rejected as unpositioned.
#[test]
fn positional_reaction_in_opening_supplies_is_rejected() {
    let frames: Vec<Frame> = vec![Frame::Reaction(WireReaction::Match, Flow::End)];
    let error = runtime()
        .block_on(async {
            early_supplies::<Local, _>(
                Local,
                u64::MAX,
                unbounded(),
                Prefix::new().erase(),
                stream::iter(frames),
                PayloadCodec::mint::<u64>(PayloadDepthLimit::default()),
            )
            .try_collect::<Vec<_>>()
            .await
        })
        .expect_err("the opening supplies admit no positional reaction");
    assert!(matches!(
        error,
        DecodeError::Scope(super::super::ScopeError::UnpositionedMatch)
    ));
}

/// Every semantic opening shape is either the canonical query-then-supplies
/// form or its exact typed rejection.
#[test]
fn opening_rejections_are_exhaustive() {
    let empty = Reply::<<Local as Backend>::Erased> {
        replies: Vec::new(),
    };
    assert_eq!(opening_parts(empty).err(), Some(OpeningError::Empty));

    for count in 1..=3 {
        let reply = Reply::<<Local as Backend>::Erased> {
            replies: (0..count).map(|_| Reaction::Match).collect(),
        };
        assert_eq!(opening_parts(reply).err(), Some(OpeningError::NotQuery));
    }

    let supplied = Reply::<<Local as Backend>::Erased> {
        replies: vec![Reaction::Supply(0, erased(UnderRoot::node()))],
    };
    assert_eq!(opening_parts(supplied).err(), Some(OpeningError::NotQuery));

    // A non-supply reaction anywhere behind the question is rejected at
    // its whole-reply position.
    let trailing = Reply::<<Local as Backend>::Erased> {
        replies: vec![
            Reaction::Query(vec![(3, hash(1))]),
            Reaction::Supply(5, erased(UnderRoot::node())),
            Reaction::Match,
        ],
    };
    assert_eq!(
        opening_parts(trailing).err(),
        Some(OpeningError::NotSupply { index: 2 })
    );
}
