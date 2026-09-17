//! Ingress validation: malformed replies and the boundaries they must admit.

use std::convert::Infallible;

use before::Version;
use futures::{TryStreamExt, stream};
use proptest::prelude::*;

use crate::tree::{
    mirror::streaming::{Backend, Local},
    typed::{
        Path, Prefix,
        height::{S, Z},
    },
};

use super::{
    super::{
        DecodeError, EncodeError, Scope, ScopeError, decode_leaf_reply, decode_reply,
        encode_leaf_reply, encode_reply,
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
        Frame::Reaction(WireReaction::Match, Flow::Continue),
        Frame::End(End::Reply),
    ];

    let error = runtime().block_on(async {
        let mut frames = stream::iter(frames);
        decode_leaf_reply(
            Local,
            u64::MAX,
            unbounded(),
            Scope::new(parent.erase(), &[(0, hash(0))]),
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
    let mut frames = stream::iter([Frame::Reaction(WireReaction::Match, Flow::Continue)]);

    let error = runtime().block_on(async {
        decode_leaf_reply(
            Local,
            u64::MAX,
            unbounded(),
            Scope::new(parent.erase(), &[(0, hash(0))]),
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
/// reaction consumes one listed child, so excess `Match` frames are
/// rejected eagerly — at the offending frame, before the skeleton grows
/// past the fan — rather than after the whole reply decodes.
#[test]
fn an_unpositioned_match_is_rejected_in_both_directions() {
    let path = Path::for_leaf(&Version::new());
    let parent = Prefix::<S<S<Z>>>::containing(&path);
    // One listed child admits one positional reaction; the second Match
    // must fail at its own frame with the reply still unterminated.
    let frames: Vec<Frame> = vec![
        Frame::Reaction(WireReaction::Match, Flow::Continue),
        Frame::Reaction(WireReaction::Match, Flow::Continue),
    ];

    let decode_error = runtime().block_on(async {
        let mut frames = stream::iter(frames);
        decode_reply::<Local, _>(
            Local,
            u64::MAX,
            unbounded(),
            Scope::new(parent.erase(), &[(1, hash(1))]),
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
        reactions: vec![Reaction::Match, Reaction::Match],
    };
    let encode_error = runtime().block_on(async {
        encode_reply(
            Local,
            RunBudget::default(),
            Scope::new(parent.erase(), &[(1, hash(1))]),
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

/// Prefix-free queries require a remaining positional child in both conversion directions.
#[test]
fn an_unpositioned_query_is_rejected_in_both_directions() {
    let path = Path::for_leaf(&Version::new());
    let parent = Prefix::<S<S<Z>>>::containing(&path);
    let listing = vec![(1, hash(1))];
    let frames: Vec<Frame> = vec![Frame::Reaction(
        WireReaction::Query(listing.clone()),
        Flow::End,
    )];

    let decode_error = runtime().block_on(async {
        let mut frames = stream::iter(frames);
        decode_reply::<Local, _>(
            Local,
            u64::MAX,
            unbounded(),
            Scope::new(parent.erase(), &[]),
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
    };
    let encode_error = runtime().block_on(async {
        encode_reply(
            Local,
            RunBudget::default(),
            Scope::new(parent.erase(), &[]),
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
                vec![(radix, hash(1))]
            } else {
                Vec::new()
            };
            let query_listing = if nonempty {
                vec![(1, hash(2))]
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
            };
            let encoded = runtime().block_on(async {
                encode_leaf_reply(
                    Local,
                    RunBudget::default(),
                    Scope::new(parent.erase(), &scope_listing),
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
                decode_leaf_reply(
                    Local,
                    u64::MAX,
                    unbounded(),
                    Scope::new(parent.erase(), &scope_listing),
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
                    let decoded = decoded.expect("this matrix cell must decode");
                    assert_eq!(
                        decoded.questions,
                        vec![Scope::leaf(parent.push(radix).erase())]
                    );
                    let [Reaction::Query(listing)] = decoded.reply.reactions.as_slice() else {
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
        .block_on(decode_leaf_reply(
            Local,
            u64::MAX,
            unbounded(),
            Scope::new(parent.erase(), &[]),
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
        decode_reply::<Local, _>(
            Local,
            u64::MAX,
            unbounded(),
            Scope::opening(&[]),
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
        decode_leaf_reply(
            Local,
            u64::MAX,
            unbounded(),
            Scope::new(parent.erase(), &[]),
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
        decode_reply::<Local, _>(
            Local,
            u64::MAX,
            unbounded(),
            Scope::opening(&[]),
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
        decode_leaf_reply(
            Local,
            declared,
            unbounded(),
            Scope::new(parent.erase(), &[]),
            &mut input,
            codec(),
        )
        .await
        .expect("a version exactly at the declared bound is admitted");
    });

    let error = runtime().block_on(async {
        let mut input = stream::iter(frames());
        decode_leaf_reply(
            Local,
            declared - 1,
            unbounded(),
            Scope::new(parent.erase(), &[]),
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
        decode_reply::<Local, _>(
            Local,
            u64::MAX,
            SupplyLedger::new(declared),
            Scope::opening(&[]),
            &mut input,
            codec(),
        )
        .await
        .map(|decoded| decoded.reply.reactions.len())
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
        Frame::Reaction(WireReaction::Match, Flow::Continue),
        Frame::Reaction(
            WireReaction::Supply(leaf_run(&[(&leaves[1].version, &leaves[1].message)])),
            Flow::End,
        ),
    ];

    let error = runtime().block_on(async {
        let mut input = stream::iter(frames);
        decode_reply::<Local, _>(
            Local,
            u64::MAX,
            unbounded(),
            Scope::opening(&[(1, hash(1))]),
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
