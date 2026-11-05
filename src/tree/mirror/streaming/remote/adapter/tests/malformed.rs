//! Ingress validation: malformed replies and the boundaries they must admit.

use crate::tree::mirror::streaming::message::LaterMatches;
use crate::tree::mirror::streaming::message::listed;
use std::convert::Infallible;

use before::Version;
use futures::{TryStreamExt, stream};
use proptest::prelude::*;

use crate::tree::{
    arb::arb_radixes,
    mirror::streaming::{Backend, Local, byte_set::ByteSet},
    typed::{
        ErasedPrefix, Path, Prefix,
        height::{S, Z},
    },
};

use super::{
    super::{
        DecodeError, EncodeError, LeafEdge, Replay, ReplyLevel, Scope, ScopeError,
        decode_whole_reply, derive_questions, encode_leaf_reply, encode_reply,
    },
    LeafCase, ascending_leaves, codec, colliding_leaves, hash, leaf_run, runtime, unbounded,
};
use crate::tree::mirror::streaming::erased::{Reaction, Reply};
use crate::tree::mirror::streaming::remote::codec::{
    DecodeLeafError, End, Flow, Frame, LeafRun, Reaction as WireReaction, RunBudget,
};

/// A nonempty reply must end on its last reaction; a later bare end is ambiguous and invalid.
#[test]
fn bare_end_cannot_follow_reactions() {
    let path = Path::for_leaf(&Version::new());
    let parent = Prefix::<S<Z>>::containing(&path);
    let frames: Vec<Frame> = vec![
        Frame::Reaction(WireReaction::Match(LaterMatches::NONE), Flow::Continue),
        Frame::End(End::Reply),
    ];

    let error = runtime().block_on(async {
        let mut frames = stream::iter(frames);
        decode_whole_reply(
            &mut LeafEdge::new(),
            Local,
            u64::MAX,
            unbounded(),
            ReplyLevel::Leaf,
            &Scope::unbounded(parent.erase(), &[listed(0, hash(0))]),
            &mut frames,
            codec(),
        )
        .await
        .err()
        .expect("nonempty replies carry their end on the last reaction")
    });
    assert!(
        matches!(error, DecodeError::BareEndAfterReaction),
        "unexpected rejection: {error:?}",
    );
}

/// Exhausting the frame stream without an explicit boundary reports truncation, not a reply.
#[test]
fn stream_exhaustion_before_a_boundary_is_truncation() {
    let path = Path::for_leaf(&Version::new());
    let parent = Prefix::<S<Z>>::containing(&path);
    let mut frames = stream::iter([Frame::Reaction(
        WireReaction::Match(LaterMatches::NONE),
        Flow::Continue,
    )]);

    let error = runtime().block_on(async {
        decode_whole_reply(
            &mut LeafEdge::new(),
            Local,
            u64::MAX,
            unbounded(),
            ReplyLevel::Leaf,
            &Scope::unbounded(parent.erase(), &[listed(0, hash(0))]),
            &mut frames,
            codec(),
        )
        .await
        .err()
        .expect("reply without a boundary is truncated")
    });
    assert!(
        matches!(error, DecodeError::TruncatedReply),
        "unexpected rejection: {error:?}",
    );
}

/// A match past the question's fan fails at its own frame, in both directions.
///
/// The scope walk is symmetric with the query arm: every positional
/// reaction consumes one listed child, so a match after the last is
/// rejected eagerly — at the offending frame, before the skeleton grows
/// past the fan — rather than after the whole reply decodes.
#[test]
fn an_unpositioned_match_is_rejected_in_both_directions() {
    let path = Path::for_leaf(&Version::new());
    let parent = Prefix::<S<S<Z>>>::containing(&path);
    // One listed child admits one positional reaction; the request takes
    // it, so the match must fail at its own frame with the reply still
    // unterminated.
    let frames: Vec<Frame> = vec![
        Frame::Reaction(WireReaction::Query(Vec::new()), Flow::Continue),
        Frame::Reaction(WireReaction::Match(LaterMatches::NONE), Flow::Continue),
    ];

    let decode_error = runtime().block_on(async {
        let mut frames = stream::iter(frames);
        decode_whole_reply::<Local, _>(
            &mut LeafEdge::new(),
            Local,
            u64::MAX,
            unbounded(),
            ReplyLevel::Branch,
            &Scope::unbounded(parent.erase(), &[listed(1, hash(1))]),
            &mut frames,
            codec(),
        )
        .await
        .err()
        .expect("a match without a remaining child cannot be scoped")
    });
    assert_eq!(
        decode_scope_error(decode_error),
        ScopeError::UnpositionedMatch,
    );

    let reply = Reply::<<Local as Backend>::Erased> {
        reactions: vec![
            Reaction::Query(Vec::new()),
            Reaction::Match(LaterMatches::NONE),
        ],
        charge: Default::default(),
        last: true,
        carried: Default::default(),
    };
    let encode_error = runtime().block_on(async {
        encode_reply(
            Local,
            RunBudget::default(),
            Scope::unbounded(parent.erase(), &[listed(1, hash(1))]),
            reply,
        )
        .try_collect::<Vec<_>>()
        .await
        .err()
        .expect("an unpositioned match cannot be put on the wire")
    });
    assert_eq!(
        encode_scope_error(encode_error),
        ScopeError::UnpositionedMatch,
    );
}

/// The parent of a test question at branch level: a node two levels above
/// the leaves, whose replies may match, query, and request.
fn branch_parent() -> ErasedPrefix {
    Prefix::<S<S<Z>>>::containing(&Path::for_leaf(&Version::new())).erase()
}

/// A question about `parent` listing a child at each of `radixes`.
fn question(parent: ErasedPrefix, radixes: &[u8]) -> Scope {
    let listing: Vec<_> = radixes
        .iter()
        .map(|&radix| listed(radix, hash(radix)))
        .collect();
    Scope::unbounded(parent, &listing)
}

/// Decode `frames` as one reply at `level` to `scope`.
fn decode_frames(
    level: ReplyLevel,
    scope: &Scope,
    frames: Vec<Frame>,
) -> Result<Reply<<Local as Backend>::Erased>, DecodeError<Infallible>> {
    runtime().block_on(async {
        let mut frames = stream::iter(frames);
        decode_whole_reply::<Local, _>(
            &mut LeafEdge::new(),
            Local,
            u64::MAX,
            unbounded(),
            level,
            scope,
            &mut frames,
            codec(),
        )
        .await
    })
}

/// Encode `reactions` as one whole reply at branch level to `scope`.
fn encode_reactions(
    scope: Scope,
    reactions: Vec<Reaction<<Local as Backend>::Erased>>,
) -> Result<Vec<Frame>, EncodeError<Infallible>> {
    let reply = Reply {
        reactions,
        charge: Default::default(),
        last: true,
        carried: Default::default(),
    };
    runtime().block_on(async {
        encode_reply(Local, RunBudget::default(), scope, reply)
            .map_ok(|encoded| encoded.into_parts().0)
            .try_collect()
            .await
    })
}

/// A reply's second match is rejected at its frame, in both directions.
///
/// The reply's first match covers every later listed child that also
/// matches, so a conforming reply carries one match at most; a second is
/// a defect even where a listed child is left for it to take.
#[test]
fn a_second_match_is_rejected_in_both_directions() {
    let scope = question(branch_parent(), &[1, 2]);
    let frames = vec![
        Frame::Reaction(WireReaction::Match(LaterMatches::NONE), Flow::Continue),
        Frame::Reaction(WireReaction::Match(LaterMatches::NONE), Flow::End),
    ];
    let error = decode_frames(ReplyLevel::Branch, &scope, frames)
        .err()
        .expect("a second match cannot decode");
    assert_eq!(decode_scope_error(error), ScopeError::RepeatedMatch);

    let reactions = vec![
        Reaction::Match(LaterMatches::NONE),
        Reaction::Match(LaterMatches::NONE),
    ];
    let error =
        encode_reactions(scope, reactions).expect_err("a second match cannot be put on the wire");
    assert_eq!(encode_scope_error(error), ScopeError::RepeatedMatch);
}

/// Listed radixes, two to eight, and a position among them for a match
/// that is not the last.
fn listing_with_match() -> impl Strategy<Value = (Vec<u8>, usize)> {
    arb_radixes(2..=8).prop_flat_map(|radixes| {
        let radixes: Vec<u8> = radixes.into_iter().collect();
        let positions = 0..radixes.len() - 1;
        (Just(radixes), positions)
    })
}

/// The frames of a reply that requests every listed child before
/// `position`, then matches there, covering `later`, and ends.
fn match_after_requests(position: usize, later: ByteSet) -> Vec<Frame> {
    let mut frames: Vec<Frame> = (0..position)
        .map(|_| Frame::Reaction(WireReaction::Query(Vec::new()), Flow::Continue))
        .collect();
    frames.push(Frame::Reaction(
        WireReaction::Match(LaterMatches::new(later)),
        Flow::End,
    ));
    frames
}

proptest! {
    /// A match covering a radix that is not a listed child after its own
    /// is rejected at its frame, in both directions, naming that radix,
    /// whether the radix is unlisted, the match's own, or before it.
    ///
    /// Every other radix the match covers is a valid later child, so the
    /// one defect is what fails the reply.
    #[test]
    fn a_cover_of_anything_but_a_later_listed_child_is_rejected(
        (radixes, position) in listing_with_match(),
        unlisted in any::<u8>(),
        earlier in any::<prop::sample::Index>(),
        use_unlisted in any::<bool>(),
    ) {
        let bad = if use_unlisted && !radixes.contains(&unlisted) {
            unlisted
        } else {
            radixes[earlier.index(position + 1)]
        };
        let mut later: ByteSet = radixes[position + 1..].iter().copied().collect();
        later.insert(bad);
        let scope = question(branch_parent(), &radixes);
        let error = decode_frames(ReplyLevel::Branch, &scope, match_after_requests(position, later))
            .err()
            .expect("a cover of anything but a later listed child cannot decode");
        prop_assert_eq!(decode_scope_error(error), ScopeError::UnlistedCover { radix: bad });

        let mut reactions: Vec<_> = (0..position).map(|_| Reaction::Query(Vec::new())).collect();
        reactions.push(Reaction::Match(LaterMatches::new(later)));
        let error = encode_reactions(scope, reactions)
            .expect_err("a cover of anything but a later listed child cannot be put on the wire");
        prop_assert_eq!(encode_scope_error(error), ScopeError::UnlistedCover { radix: bad });
    }

    /// A reply may end on its match while listed children remain after it,
    /// when the match covers every one of them.
    ///
    /// The end is checked, never inferred, and the reply decodes to exactly
    /// its frames' reactions, the match carrying its cover.
    #[test]
    fn a_reply_may_end_before_the_children_its_match_covers(
        (radixes, position) in listing_with_match(),
    ) {
        let later: ByteSet = radixes[position + 1..].iter().copied().collect();
        let scope = question(branch_parent(), &radixes);
        let reply = decode_frames(ReplyLevel::Branch, &scope, match_after_requests(position, later))
            .expect("a match covering every child left may end the reply");
        prop_assert_eq!(reply.reactions.len(), position + 1);
        let Some(Reaction::Match(covered)) = reply.reactions.last() else {
            return Err(TestCaseError::fail("the reply ends on its match"));
        };
        prop_assert_eq!(covered.set(), later);
        let questions = derive_questions(
            ReplyLevel::Branch,
            &scope,
            &mut Replay::default(),
            &reply.reactions,
        )
        .expect("decoding validated positions");
        prop_assert_eq!(questions.len(), position);
    }

    /// A reply that ends while a listed child after its last frame is
    /// neither answered nor covered is rejected at its end, naming the
    /// first such child: whether the reply ends on a match that leaves one
    /// out, or is empty.
    #[test]
    fn a_reply_ending_before_an_uncovered_child_is_rejected(
        (radixes, position) in listing_with_match(),
        omitted in any::<prop::sample::Index>(),
        empty in any::<bool>(),
    ) {
        let scope = question(branch_parent(), &radixes);
        let (frames, first) = if empty {
            (vec![Frame::End(End::Reply)], radixes[0])
        } else {
            let left = &radixes[position + 1..];
            let first = left[omitted.index(left.len())];
            let later: ByteSet = left.iter().copied().filter(|radix| *radix != first).collect();
            (match_after_requests(position, later), first)
        };
        let error = decode_frames(ReplyLevel::Branch, &scope, frames)
            .err()
            .expect("a reply leaving a listed child uncovered cannot decode");
        prop_assert_eq!(decode_scope_error(error), ScopeError::UncoveredEnd { radix: first });
    }

    /// A supply at a radix the reply's match covers is rejected at its
    /// frame: the covered child is one both sides hold.
    ///
    /// The supplied leaf sits under the question's node, at the radix the
    /// match covers, after a match at a smaller listed radix: the first leaf
    /// from `value` on whose radix leaves room for one.
    #[test]
    fn a_supply_at_a_covered_radix_is_rejected(value in any::<u64>(), ticks in 1..=8_u8) {
        let (leaf, parent, radix) = (0..)
            .map(|step| LeafCase::new(value.wrapping_add(step), ticks))
            .find_map(|leaf| {
                let (parent, radix) = Prefix::<Z>::containing(&leaf.path()).pop();
                (radix > 0).then_some((leaf, parent, radix))
            })
            .expect("leaf radixes are uniform, so one above zero comes quickly");
        let scope = question(parent.erase(), &[radix - 1, radix]);
        let frames = vec![
            Frame::Reaction(
                WireReaction::Match(LaterMatches::new(std::iter::once(radix).collect())),
                Flow::Continue,
            ),
            Frame::Reaction(
                WireReaction::Supply(leaf_run(&[(&leaf.version, &leaf.message)])),
                Flow::End,
            ),
        ];
        let error = decode_frames(ReplyLevel::Leaf, &scope, frames)
            .err()
            .expect("a supply at a covered radix cannot decode");
        prop_assert_eq!(decode_scope_error(error), ScopeError::CoveredSupply { radix });
    }
}

/// Prefix-free queries require a remaining positional child in both conversion directions.
#[test]
fn an_unpositioned_query_is_rejected_in_both_directions() {
    let path = Path::for_leaf(&Version::new());
    let parent = Prefix::<S<S<Z>>>::containing(&path);
    let listing = vec![listed(1, hash(1))];
    let frames: Vec<Frame> = vec![Frame::Reaction(
        WireReaction::Query(listing.clone()),
        Flow::End,
    )];

    let decode_error = runtime().block_on(async {
        let mut frames = stream::iter(frames);
        decode_whole_reply::<Local, _>(
            &mut LeafEdge::new(),
            Local,
            u64::MAX,
            unbounded(),
            ReplyLevel::Branch,
            &Scope::unbounded(parent.erase(), &[]),
            &mut frames,
            codec(),
        )
        .await
        .err()
        .expect("a query without a child has no derivable scope")
    });
    assert_eq!(
        decode_scope_error(decode_error),
        ScopeError::UnpositionedQuery,
    );

    let reply = Reply::<<Local as Backend>::Erased> {
        reactions: vec![Reaction::Query(listing)],
        charge: Default::default(),
        last: true,
        carried: Default::default(),
    };
    let encode_error = runtime().block_on(async {
        encode_reply(
            Local,
            RunBudget::default(),
            Scope::unbounded(parent.erase(), &[]),
            reply,
        )
        .try_collect::<Vec<_>>()
        .await
        .err()
        .expect("an unpositioned query cannot be put on the wire")
    });
    assert_eq!(
        encode_scope_error(encode_error),
        ScopeError::UnpositionedQuery,
    );
}

/// All eight leaf-query paths pin validity, error precedence, framing, and publication.
#[test]
fn leaf_query_matrix_is_exhaustive() {
    let path = Path::for_leaf(&Version::new());
    let parent = Prefix::<S<Z>>::containing(&path);
    let radix = 3;
    let mut checked = 0;

    for positioned in [false, true] {
        for nonempty in [false, true] {
            let scope_listing = if positioned {
                vec![listed(radix, hash(1))]
            } else {
                Vec::new()
            };
            let query_listing = if nonempty {
                vec![listed(1, hash(2))]
            } else {
                Vec::new()
            };
            let expected_error = if nonempty {
                Some(ScopeError::NonemptyLeafQuery)
            } else if !positioned {
                Some(ScopeError::UnpositionedQuery)
            } else {
                None
            };
            let expected_frame: Frame =
                Frame::Reaction(WireReaction::Query(query_listing.clone()), Flow::End);

            let reply = Reply::<<Local as Backend>::Erased> {
                reactions: vec![Reaction::Query(query_listing.clone())],
                charge: Default::default(),
                last: true,
                carried: Default::default(),
            };
            let encoded = runtime().block_on(async {
                encode_leaf_reply(
                    Local,
                    RunBudget::default(),
                    Scope::unbounded(parent.erase(), &scope_listing),
                    reply,
                )
                .map_ok(|encoded| encoded.into_parts())
                .try_collect::<Vec<_>>()
                .await
            });
            match expected_error {
                Some(expected) => {
                    let error = encoded.expect_err("this matrix cell must reject");
                    assert_eq!(encode_scope_error(error), expected);
                }
                None => {
                    let encoded = encoded.expect("this matrix cell must encode");
                    let [(frame, question)] = encoded.as_slice() else {
                        panic!("a leaf query encodes as exactly one frame")
                    };
                    assert_eq!(frame, &expected_frame);
                    assert_eq!(question, &Some(Scope::leaf(parent.push(radix).erase())));
                }
            }
            checked += 1;

            let decoded = runtime().block_on(async {
                let mut frames = stream::iter([expected_frame]);
                decode_whole_reply(
                    &mut LeafEdge::new(),
                    Local,
                    u64::MAX,
                    unbounded(),
                    ReplyLevel::Leaf,
                    &Scope::unbounded(parent.erase(), &scope_listing),
                    &mut frames,
                    codec(),
                )
                .await
            });
            match expected_error {
                Some(expected) => {
                    let error = decoded.err().expect("this matrix cell must reject");
                    assert_eq!(decode_scope_error(error), expected);
                }
                None => {
                    let reply = decoded.expect("this matrix cell must decode");
                    let questions = derive_questions(
                        ReplyLevel::Leaf,
                        &Scope::unbounded(parent.erase(), &scope_listing),
                        &mut Replay::default(),
                        &reply.reactions,
                    )
                    .expect("decoding validated positions");
                    assert_eq!(questions, vec![Scope::leaf(parent.push(radix).erase())]);
                    let [Reaction::Query(listing)] = reply.reactions.as_slice() else {
                        panic!("the decoded reaction must remain a query")
                    };
                    assert!(listing.is_empty());
                }
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 8);
}

/// Transport stream-end control is rejected if it leaks past demultiplexing.
#[test]
fn stream_end_is_not_a_protocol_reply() {
    let path = Path::for_leaf(&Version::new());
    let parent = Prefix::<S<Z>>::containing(&path);
    let mut frames = stream::iter([Frame::End(End::Stream)]);

    let error = runtime()
        .block_on(decode_whole_reply(
            &mut LeafEdge::new(),
            Local,
            u64::MAX,
            unbounded(),
            ReplyLevel::Leaf,
            &Scope::unbounded(parent.erase(), &[]),
            &mut frames,
            codec(),
        ))
        .err()
        .expect("stream control must be consumed below the adapter");
    assert!(
        matches!(error, DecodeError::UnexpectedStreamEnd),
        "unexpected rejection: {error:?}",
    );
}

/// Extract a scope error from the infallible local encoder.
fn encode_scope_error(error: EncodeError<Infallible>) -> ScopeError {
    match error {
        EncodeError::Scope(error) => error,
        EncodeError::Backend(error) => match error {},
        EncodeError::Record(error) => panic!("expected a scope error, got {error}"),
    }
}

/// Extract a scope error from the infallible local decoder.
fn decode_scope_error(error: DecodeError<Infallible>) -> ScopeError {
    match error {
        DecodeError::Scope(error) => error,
        other => panic!("expected a scope error, got {other:?}"),
    }
}

/// Leaf ordering is enforced between records inside one run, not only across frames.
///
/// The existing ordering rejections all place their two records in separate
/// single-record frames; here one supply run carries both records with the
/// second preceding the first in content-path order, and the decoder must
/// still report `LeafOrder`.
#[test]
fn leaf_order_is_enforced_within_one_run() {
    let leaves = colliding_leaves(2);
    let frames = vec![Frame::Reaction(
        WireReaction::Supply(leaf_run(&[
            (&leaves[1].version, &leaves[1].message),
            (&leaves[0].version, &leaves[0].message),
        ])),
        Flow::End,
    )];

    let error = runtime().block_on(async {
        let mut input = stream::iter(frames);
        decode_whole_reply::<Local, _>(
            &mut LeafEdge::new(),
            Local,
            u64::MAX,
            unbounded(),
            ReplyLevel::Branch,
            &Scope::opening_unbounded(&[]),
            &mut input,
            codec(),
        )
        .await
        .err()
        .expect("descending records within one run violate leaf ordering")
    });
    let DecodeError::LeafOrder { previous, current } = error else {
        panic!("expected LeafOrder, got {error:?}");
    };
    assert_eq!(previous, <[u8; 32]>::from(leaves[1].path()));
    assert_eq!(current, <[u8; 32]>::from(leaves[0].path()));
}

/// Reply scope is enforced between records inside one run, not only across frames.
///
/// A single supply run whose first record sits inside the reply's scope and
/// whose second escapes it must be rejected as `LeafOutsideScope` on that
/// second record, not silently absorbed with its in-scope sibling.
#[test]
fn leaf_scope_is_enforced_within_one_run() {
    let inside = LeafCase::new(0, 0);
    let parent = Prefix::<Z>::containing(&inside.path()).pop().0;
    let outside = (1..u64::MAX)
        .map(|value| LeafCase::new(value, 0))
        .find(|candidate| Prefix::<Z>::containing(&candidate.path()).pop().0 != parent)
        .expect("content paths do not all share one leaf parent");
    let frames = vec![Frame::Reaction(
        WireReaction::Supply(leaf_run(&[
            (&inside.version, &inside.message),
            (&outside.version, &outside.message),
        ])),
        Flow::End,
    )];

    let error = runtime().block_on(async {
        let mut input = stream::iter(frames);
        decode_whole_reply(
            &mut LeafEdge::new(),
            Local,
            u64::MAX,
            unbounded(),
            ReplyLevel::Leaf,
            &Scope::unbounded(parent.erase(), &[]),
            &mut input,
            codec(),
        )
        .await
        .err()
        .expect("a record escaping the reply scope must fail")
    });
    let DecodeError::LeafOutsideScope { expected, actual } = error else {
        panic!("expected LeafOutsideScope, got {error:?}");
    };
    assert_eq!(expected, parent.as_bytes().to_vec());
    assert_eq!(actual, <[u8; 32]>::from(outside.path()));
}

/// The run body of a single empty-content record: the embedded-sequence
/// tag wrapping an empty byte string.
const ZERO_LENGTH_RECORD_RUN: [u8; 3] = [0xd8, 0x3f, 0x40];

/// An empty-content record passes structural validation but fails
/// canonically.
///
/// A record whose byte string is empty chains exactly, so the wire
/// accepts the run's structure; the empty content cannot hold a tagged
/// version, so the reply decoder reports `DecodeError::Record` carrying
/// the version decoder's `UnexpectedEof`.
#[test]
fn a_zero_length_record_fails_as_a_version_decode_error() {
    let run = LeafRun::from_encoded(ZERO_LENGTH_RECORD_RUN.to_vec())
        .expect("a zero-length record header chains structurally");
    assert_eq!(run.record_count(), 1);
    let frames = vec![Frame::Reaction(WireReaction::Supply(run), Flow::End)];

    let error = runtime().block_on(async {
        let mut input = stream::iter(frames);
        decode_whole_reply::<Local, _>(
            &mut LeafEdge::new(),
            Local,
            u64::MAX,
            unbounded(),
            ReplyLevel::Branch,
            &Scope::opening_unbounded(&[]),
            &mut input,
            codec(),
        )
        .await
        .err()
        .expect("an empty record body cannot decode a version")
    });
    let DecodeError::Record(DecodeLeafError::Version(source)) = error else {
        panic!("expected a version decode error, got {error:?}");
    };
    assert!(
        matches!(
            source,
            crate::tree::mirror::streaming::remote::codec::VersionDecodeError::TagHead(
                crate::tree::mirror::cbor::HeadError::Truncated
            )
        ),
        "unexpected version rejection: {source:?}",
    );
}

/// The declared version bound admits exactly the versions it covers.
///
/// A supplied version encoding exactly at the peer's declared
/// `max_version_bytes` decodes, and the same record under a declaration
/// one byte smaller is rejected as `OversizedVersion` carrying both the
/// declaration and the offending encoding's size.
#[test]
fn a_version_over_the_declared_bound_is_rejected() {
    let leaf = LeafCase::new(0, 0);
    let parent = Prefix::<Z>::containing(&leaf.path()).pop().0;
    let declared = leaf.version.as_bytes().len() as u64;
    let frames = || {
        vec![Frame::Reaction(
            WireReaction::Supply(leaf_run(&[(&leaf.version, &leaf.message)])),
            Flow::End,
        )]
    };

    runtime().block_on(async {
        let mut input = stream::iter(frames());
        decode_whole_reply(
            &mut LeafEdge::new(),
            Local,
            declared,
            unbounded(),
            ReplyLevel::Leaf,
            &Scope::unbounded(parent.erase(), &[]),
            &mut input,
            codec(),
        )
        .await
        .expect("a version exactly at the declared bound is admitted");
    });

    let error = runtime().block_on(async {
        let mut input = stream::iter(frames());
        decode_whole_reply(
            &mut LeafEdge::new(),
            Local,
            declared - 1,
            unbounded(),
            ReplyLevel::Leaf,
            &Scope::unbounded(parent.erase(), &[]),
            &mut input,
            codec(),
        )
        .await
        .err()
        .expect("a version over the declared bound must be rejected")
    });
    let DecodeError::OversizedVersion {
        declared: bound,
        actual,
    } = error
    else {
        panic!("expected OversizedVersion, got {error:?}");
    };
    assert_eq!(bound, declared - 1);
    assert_eq!(actual as u64, declared);
}

/// One whole-root reply supplying every leaf in `cases`, as a single
/// ascending run.
fn whole_root_supply_reply(cases: &[LeafCase]) -> Vec<Frame> {
    let records: Vec<_> = cases
        .iter()
        .map(|case| (&case.version, &case.message))
        .collect();
    vec![Frame::Reaction(
        WireReaction::Supply(leaf_run(&records)),
        Flow::End,
    )]
}

/// Decode a whole-root supply under the peer's declared set length.
fn decode_with_set_len(count: u64, declared: u64) -> Result<usize, DecodeError<Infallible>> {
    use crate::tree::mirror::streaming::materialized::SupplyLedger;

    let frames = whole_root_supply_reply(&ascending_leaves(0..count, 0));
    runtime().block_on(async {
        let mut input = stream::iter(frames);
        decode_whole_reply::<Local, _>(
            &mut LeafEdge::new(),
            Local,
            u64::MAX,
            SupplyLedger::new(declared),
            ReplyLevel::Branch,
            &Scope::opening_unbounded(&[]),
            &mut input,
            codec(),
        )
        .await
        .map(|reply| reply.reactions.len())
    })
}

proptest! {
    /// The peer's declared set length is the exact admission boundary for a reply.
    #[test]
    fn supplied_record_count_respects_the_declared_set_len(
        declared in 1u64..64,
        excess in 1u64..64,
    ) {
        prop_assert!(
            decode_with_set_len(declared, declared).is_ok(),
            "a declaration must admit exactly that many supplied records",
        );
        let rejected = matches!(
            decode_with_set_len(declared + excess, declared),
            Err(DecodeError::OverdrawnSupply { declared: actual }) if actual == declared
        );
        prop_assert!(rejected, "a reply above the declaration must be rejected as overdrawn");
    }
}

/// Interrupting a supply run finalizes its radix, so later resumption is rejected as reordering.
#[test]
fn a_supply_run_cannot_resume_after_another_reaction() {
    let leaves = colliding_leaves(2);
    // The interrupting Match consumes the scope's one listed child, so it
    // is positionally valid and the failure isolates the supply
    // resumption itself.
    let frames = vec![
        Frame::Reaction(
            WireReaction::Supply(leaf_run(&[(&leaves[0].version, &leaves[0].message)])),
            Flow::Continue,
        ),
        Frame::Reaction(WireReaction::Match(LaterMatches::NONE), Flow::Continue),
        Frame::Reaction(
            WireReaction::Supply(leaf_run(&[(&leaves[1].version, &leaves[1].message)])),
            Flow::End,
        ),
    ];

    let error = runtime().block_on(async {
        let mut input = stream::iter(frames);
        decode_whole_reply::<Local, _>(
            &mut LeafEdge::new(),
            Local,
            u64::MAX,
            unbounded(),
            ReplyLevel::Branch,
            &Scope::opening_unbounded(&[listed(1, hash(1))]),
            &mut input,
            codec(),
        )
        .await
        .err()
        .expect("a keyed supply may occupy only one ascending run")
    });
    let expected = <[u8; 32]>::from(leaves[0].path())[0];
    match error {
        DecodeError::SupplyOrder { previous, radix } => {
            assert_eq!(previous, expected);
            assert_eq!(radix, expected);
        }
        other => panic!("expected SupplyOrder, got {other:?}"),
    }
}
