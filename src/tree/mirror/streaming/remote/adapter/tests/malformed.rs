//! Focused malformed-wire cases which are not naturally height-parametric.

use std::{collections::BTreeMap, convert::Infallible};

use before::Version;
use futures::{TryStreamExt, stream};

use crate::{
    message::Message,
    tree::{
        mirror::streaming::{Backend, Local},
        typed::{
            Path, Prefix,
            height::{S, UnderRoot, UnderUnderRoot, Z},
        },
    },
};

use super::{
    super::{
        DecodeError, EncodeError, Scope, ScopeError, decode_leaf_reply, decode_reply,
        encode_leaf_reply, encode_reply,
    },
    LeafCase, hash, leaf_run, runtime,
};
use crate::tree::mirror::streaming::message::{Reaction, Reply};
use crate::tree::mirror::streaming::remote::codec::{
    DecodeLeafError, End, Flow, Frame, LeafRun, Reaction as WireReaction, RunBudget,
};

/// A nonempty reply must end on its last reaction; a later bare end is ambiguous and invalid.
#[test]
fn bare_end_cannot_follow_reactions() {
    let path = Path::for_leaf(&Version::new(), &[0]);
    let parent = Prefix::<S<Z>>::containing(&path);
    let frames: Vec<Frame<()>> = vec![
        Frame::Reaction(WireReaction::Match, Flow::Continue),
        Frame::End(End::Reply),
    ];

    let error = runtime().block_on(async {
        let mut frames = stream::iter(frames);
        decode_leaf_reply(
            Local,
            u64::MAX,
            Scope::new(parent, &[(0, hash(0))]),
            &mut frames,
        )
        .await
        .err()
        .expect("nonempty replies carry their end on the last reaction")
    });
    assert!(matches!(error, DecodeError::BareEndAfterReaction));
}

/// Exhausting the frame stream without an explicit boundary reports truncation, not a reply.
#[test]
fn stream_exhaustion_before_a_boundary_is_truncation() {
    let path = Path::for_leaf(&Version::new(), &[0]);
    let parent = Prefix::<S<Z>>::containing(&path);
    let mut frames = stream::iter([Frame::<()>::Reaction(WireReaction::Match, Flow::Continue)]);

    let error = runtime().block_on(async {
        decode_leaf_reply(
            Local,
            u64::MAX,
            Scope::new(parent, &[(0, hash(0))]),
            &mut frames,
        )
        .await
        .err()
        .expect("reply without a boundary is truncated")
    });
    assert!(matches!(error, DecodeError::TruncatedReply));
}

/// A match past the question's fan fails at its own frame, in both directions.
///
/// The scope walk is symmetric with the query arm: every positional
/// reaction consumes one listed child, so excess `Match` frames are
/// rejected eagerly — at the offending frame, before the skeleton grows
/// past the fan — rather than after the whole reply decodes.
#[test]
fn an_unpositioned_match_is_rejected_in_both_directions() {
    let path = Path::for_leaf(&Version::new(), &[0]);
    let parent = Prefix::<S<S<Z>>>::containing(&path);
    // One listed child admits one positional reaction; the second Match
    // must fail at its own frame with the reply still unterminated.
    let frames: Vec<Frame<()>> = vec![
        Frame::Reaction(WireReaction::Match, Flow::Continue),
        Frame::Reaction(WireReaction::Match, Flow::Continue),
    ];

    let decode_error = runtime().block_on(async {
        let mut frames = stream::iter(frames);
        decode_reply::<Local, (), Z, _>(
            Local,
            u64::MAX,
            Scope::new(parent, &[(1, hash(1))]),
            &mut frames,
        )
        .await
        .err()
        .expect("a match without a remaining child cannot be scoped")
    });
    assert!(matches!(
        decode_error,
        DecodeError::Scope(ScopeError::UnpositionedMatch)
    ));

    let reply = Reply::<Local, (), S<Z>> {
        replies: vec![Reaction::Match, Reaction::Match],
    };
    let encode_error = runtime().block_on(async {
        encode_reply(
            Local,
            RunBudget::default(),
            Scope::new(parent, &[(1, hash(1))]),
            reply,
        )
        .try_collect::<Vec<_>>()
        .await
        .err()
        .expect("an unpositioned match cannot be put on the wire")
    });
    assert!(matches!(
        encode_error,
        EncodeError::Scope(ScopeError::UnpositionedMatch)
    ));
}

/// Prefix-free queries require a remaining positional child in both conversion directions.
#[test]
fn an_unpositioned_query_is_rejected_in_both_directions() {
    let path = Path::for_leaf(&Version::new(), &[0]);
    let parent = Prefix::<S<S<Z>>>::containing(&path);
    let listing = vec![(1, hash(1))];
    let frames: Vec<Frame<()>> = vec![Frame::Reaction(
        WireReaction::Query(listing.clone()),
        Flow::End,
    )];

    let decode_error = runtime().block_on(async {
        let mut frames = stream::iter(frames);
        decode_reply::<Local, (), Z, _>(Local, u64::MAX, Scope::new(parent, &[]), &mut frames)
            .await
            .err()
            .expect("a query without a child has no derivable scope")
    });
    assert!(matches!(
        decode_error,
        DecodeError::Scope(ScopeError::UnpositionedQuery)
    ));

    let reply = Reply::<Local, (), S<Z>> {
        replies: vec![Reaction::Query(listing)],
    };
    let encode_error = runtime().block_on(async {
        encode_reply(Local, RunBudget::default(), Scope::new(parent, &[]), reply)
            .try_collect::<Vec<_>>()
            .await
            .err()
            .expect("an unpositioned query cannot be put on the wire")
    });
    assert!(matches!(
        encode_error,
        EncodeError::Scope(ScopeError::UnpositionedQuery)
    ));
}

/// All eight leaf-query paths pin validity, error precedence, framing, and publication.
#[test]
fn leaf_query_matrix_is_exhaustive() {
    let path = Path::for_leaf(&Version::new(), &[0]);
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
            let expected_frame =
                Frame::Reaction(WireReaction::Query(query_listing.clone()), Flow::End);

            let reply = Reply::<Local, (), Z> {
                replies: vec![Reaction::Query(query_listing.clone())],
            };
            let encoded = runtime().block_on(async {
                encode_leaf_reply(
                    Local,
                    RunBudget::default(),
                    Scope::new(parent, &scope_listing),
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
                    assert_eq!(question, &Some(Scope::leaf(parent.push(radix))));
                }
            }
            checked += 1;

            let decoded = runtime().block_on(async {
                let mut frames = stream::iter([expected_frame]);
                decode_leaf_reply(
                    Local,
                    u64::MAX,
                    Scope::new(parent, &scope_listing),
                    &mut frames,
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
                    assert_eq!(decoded.questions, vec![Scope::leaf(parent.push(radix))]);
                    let [Reaction::Query(listing)] = decoded.reply.replies.as_slice() else {
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
    let path = Path::for_leaf(&Version::new(), &[0]);
    let parent = Prefix::<S<Z>>::containing(&path);
    let mut frames = stream::iter([Frame::<()>::End(End::Stream)]);

    let error = runtime()
        .block_on(decode_leaf_reply(
            Local,
            u64::MAX,
            Scope::new(parent, &[]),
            &mut frames,
        ))
        .err()
        .expect("stream control must be consumed below the adapter");
    assert!(matches!(error, DecodeError::UnexpectedStreamEnd));
}

fn encode_scope_error(error: EncodeError<Infallible>) -> ScopeError {
    match error {
        EncodeError::Scope(error) => error,
        EncodeError::Backend(error) => match error {},
        EncodeError::Record(error) => panic!("expected a scope error, got {error}"),
    }
}

fn decode_scope_error(error: DecodeError<Infallible>) -> ScopeError {
    match error {
        DecodeError::Scope(error) => error,
        other => panic!("expected a scope error, got {other:?}"),
    }
}

fn under_root_pair() -> [(Version, Message<u64>, Path); 2] {
    let mut by_radix: BTreeMap<u8, Vec<(Version, Message<u64>, Path)>> = BTreeMap::new();
    for value in 0..u64::MAX {
        let leaf = LeafCase::new(value, value as u8 % 4);
        let path = leaf.path();
        let bytes: [u8; 32] = path.into();
        let group = by_radix.entry(bytes[0]).or_default();
        group.push((leaf.version, leaf.message, path));
        if group.len() == 2 {
            group.sort_by_key(|(_, _, path)| *path);
            return group.clone().try_into().expect("two colliding radices");
        }
    }
    unreachable!("the finite radix alphabet forces a collision")
}

/// Consecutive leaves in one content-derived run assemble as one node and reexplode exactly.
#[test]
fn a_multi_leaf_run_is_one_supplied_subtree() {
    let leaves = under_root_pair();
    let frames = vec![
        Frame::Reaction(
            WireReaction::Supply(leaf_run(&[(&leaves[0].0, &leaves[0].1)])),
            Flow::Continue,
        ),
        Frame::Reaction(
            WireReaction::Supply(leaf_run(&[(&leaves[1].0, &leaves[1].1)])),
            Flow::End,
        ),
    ];
    let scope = Scope::<UnderRoot>::opening(&[]);

    let reencoded = runtime().block_on(async {
        let mut input = stream::iter(frames.clone());
        let decoded = decode_reply::<Local, u64, UnderUnderRoot, _>(
            Local,
            u64::MAX,
            scope.clone(),
            &mut input,
        )
        .await
        .expect("ascending in-scope leaves assemble");
        assert_eq!(decoded.reply.replies.len(), 1);
        let [Reaction::Supply(_, node)] = decoded.reply.replies.as_slice() else {
            panic!("one leaf run must become one supplied node")
        };
        let supplied_prefix = Prefix::<UnderRoot>::containing(&leaves[0].2);
        let rebuilt = Local
            .leaves(supplied_prefix, node.clone())
            .try_collect::<Vec<_>>()
            .await
            .expect("the local backend is infallible");
        assert_eq!(rebuilt.len(), 2);

        encode_reply(Local, RunBudget::default(), scope, decoded.reply)
            .map_ok(|encoded| encoded.into_parts().0)
            .try_collect::<Vec<_>>()
            .await
            .expect("rebuilt subtree reexplodes")
    });
    // Re-encoding batches the whole reaction's leaves into one default-budget
    // run: the canonical wire form, regardless of how the input was chunked.
    let batched = vec![Frame::Reaction(
        WireReaction::Supply(leaf_run(&[
            (&leaves[0].0, &leaves[0].1),
            (&leaves[1].0, &leaves[1].1),
        ])),
        Flow::End,
    )];
    assert_eq!(reencoded, batched);
}

/// Leaf ordering is enforced between records inside one run, not only across frames.
///
/// The existing ordering rejections all place their two records in separate
/// single-record frames; here one supply run carries both records with the
/// second preceding the first in content-path order, and the decoder must
/// still report `LeafOrder`.
#[test]
fn leaf_order_is_enforced_within_one_run() {
    let leaves = under_root_pair();
    let frames = vec![Frame::Reaction(
        WireReaction::Supply(leaf_run(&[
            (&leaves[1].0, &leaves[1].1),
            (&leaves[0].0, &leaves[0].1),
        ])),
        Flow::End,
    )];

    let error = runtime().block_on(async {
        let mut input = stream::iter(frames);
        decode_reply::<Local, u64, UnderUnderRoot, _>(
            Local,
            u64::MAX,
            Scope::opening(&[]),
            &mut input,
        )
        .await
        .err()
        .expect("descending records within one run violate leaf ordering")
    });
    let DecodeError::LeafOrder { previous, current } = error else {
        panic!("expected LeafOrder, got {error:?}");
    };
    assert_eq!(previous, <[u8; 32]>::from(leaves[1].2));
    assert_eq!(current, <[u8; 32]>::from(leaves[0].2));
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
        decode_leaf_reply(Local, u64::MAX, Scope::new(parent, &[]), &mut input)
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

/// The run body of a single zero-length record: one bare record header.
const ZERO_LENGTH_RECORD_RUN: [u8; 4] = [0, 0, 0, 0];

/// A zero-length record passes structural validation but fails canonically.
///
/// A `00000000` record header inside a run chains exactly, so the wire
/// accepts the run's structure; the record's empty body cannot hold a
/// version, so the reply decoder reports `DecodeError::Record` carrying the
/// version decoder's `UnexpectedEof`.
#[test]
fn a_zero_length_record_fails_as_a_version_decode_error() {
    let run = LeafRun::<u64>::from_encoded(ZERO_LENGTH_RECORD_RUN.to_vec())
        .expect("a zero-length record header chains structurally");
    assert_eq!(run.record_count(), 1);
    let frames = vec![Frame::Reaction(WireReaction::Supply(run), Flow::End)];

    let error = runtime().block_on(async {
        let mut input = stream::iter(frames);
        decode_reply::<Local, u64, UnderUnderRoot, _>(
            Local,
            u64::MAX,
            Scope::opening(&[]),
            &mut input,
        )
        .await
        .err()
        .expect("an empty record body cannot decode a version")
    });
    let DecodeError::Record(DecodeLeafError::Version(source)) = error else {
        panic!("expected a version decode error, got {error:?}");
    };
    assert_eq!(source.kind(), borsh::io::ErrorKind::UnexpectedEof);
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
        decode_leaf_reply(Local, declared, Scope::new(parent, &[]), &mut input)
            .await
            .expect("a version exactly at the declared bound is admitted");
    });

    let error = runtime().block_on(async {
        let mut input = stream::iter(frames());
        decode_leaf_reply(Local, declared - 1, Scope::new(parent, &[]), &mut input)
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

/// `count` distinct leaves in ascending content-path order, all inside
/// the whole-root opening scope: the shape of one reply streaming an
/// arbitrary volume of supplies.
fn ascending_leaves(count: u64) -> Vec<LeafCase> {
    let mut cases: Vec<LeafCase> = (0..count)
        .map(|value| LeafCase::new(value, value as u8 % 4))
        .collect();
    cases.sort_by_key(LeafCase::path);
    cases
}

/// One whole-root reply supplying every leaf in `cases`, as a single
/// ascending run.
fn whole_root_supply_reply(cases: &[LeafCase]) -> Vec<Frame<u64>> {
    let records: Vec<_> = cases
        .iter()
        .map(|case| (&case.version, &case.message))
        .collect();
    vec![Frame::Reaction(
        WireReaction::Supply(leaf_run(&records)),
        Flow::End,
    )]
}

/// Known-bad baseline for reply-ingress supply accounting: the decoder
/// takes custody of every supplied record in a still-open reply, bounded
/// by nothing the peer declared.
///
/// The peer's greeting-declared `set_len` is a premise the session's
/// window solve prices, yet no charge sits on this decode path: a reply
/// streaming any number of leaves decodes to completion, and decoded
/// node residency grows record for record with the stream. Metered by
/// the node census (the crate's exact residency shadow) across two
/// stream sizes, so the growth reads as a slope, not a point. A
/// conforming decoder must instead fail typed at the first record past
/// the declaration; this pin holds the bad baseline until that charge
/// flips it.
#[test]
fn decoded_supply_residency_is_unbounded_by_any_declaration() {
    use crate::tree::typed::untyped::census;

    const SMALL: u64 = 128;
    const LARGE: u64 = 256;

    /// Peak node-handle residency beyond the pre-decode baseline while
    /// one whole-root reply of `count` leaves decodes to completion.
    fn residency_beyond_baseline(count: u64) -> usize {
        let frames = whole_root_supply_reply(&ascending_leaves(count));
        census::reset_peak();
        let (live, _) = census::read();
        let decoded = runtime()
            .block_on(async {
                let mut input = stream::iter(frames);
                decode_reply::<Local, u64, UnderUnderRoot, _>(
                    Local,
                    u64::MAX,
                    Scope::opening(&[]),
                    &mut input,
                )
                .await
            })
            .expect("no declaration bounds this decode path today");
        let (_, peak) = census::read();
        drop(decoded);
        peak - live
    }

    let small = residency_beyond_baseline(SMALL);
    let large = residency_beyond_baseline(LARGE);
    assert!(
        small >= SMALL as usize,
        "every streamed record takes custody: {small} resident handles \
         for a {SMALL}-leaf reply",
    );
    assert!(
        large >= LARGE as usize,
        "every streamed record takes custody: {large} resident handles \
         for a {LARGE}-leaf reply",
    );
    assert!(
        large - small >= (LARGE - SMALL) as usize,
        "residency grows record for record with the stream \
         ({small} -> {large} handles for {SMALL} -> {LARGE} leaves): \
         nothing bounds a still-open reply",
    );
}

/// Interrupting a supply run finalizes its radix, so later resumption is rejected as reordering.
#[test]
fn a_supply_run_cannot_resume_after_another_reaction() {
    let leaves = under_root_pair();
    // The interrupting Match consumes the scope's one listed child, so it
    // is positionally valid and the failure isolates the supply
    // resumption itself.
    let frames = vec![
        Frame::Reaction(
            WireReaction::Supply(leaf_run(&[(&leaves[0].0, &leaves[0].1)])),
            Flow::Continue,
        ),
        Frame::Reaction(WireReaction::Match, Flow::Continue),
        Frame::Reaction(
            WireReaction::Supply(leaf_run(&[(&leaves[1].0, &leaves[1].1)])),
            Flow::End,
        ),
    ];

    let error = runtime().block_on(async {
        let mut input = stream::iter(frames);
        decode_reply::<Local, u64, UnderUnderRoot, _>(
            Local,
            u64::MAX,
            Scope::opening(&[(1, hash(1))]),
            &mut input,
        )
        .await
        .err()
        .expect("a keyed supply may occupy only one ascending run")
    });
    assert!(matches!(error, DecodeError::SupplyOrder { .. }));
}
