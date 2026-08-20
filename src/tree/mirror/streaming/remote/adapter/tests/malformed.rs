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
            height::{S, UnderRoot, Z},
        },
    },
};

use super::{
    super::{
        DecodeError, EncodeError, Scope, ScopeError, decode_leaf_reply, decode_reply,
        encode_leaf_reply, encode_reply,
    },
    LeafCase, hash, leaf_run, runtime, unbounded,
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
    let frames: Vec<Frame<()>> = vec![
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
    let path = Path::for_leaf(&Version::new());
    let parent = Prefix::<S<Z>>::containing(&path);
    let mut frames = stream::iter([Frame::<()>::Reaction(WireReaction::Match, Flow::Continue)]);

    let error = runtime().block_on(async {
        decode_leaf_reply(
            Local,
            u64::MAX,
            unbounded(),
            Scope::new(parent.erase(), &[(0, hash(0))]),
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
    let path = Path::for_leaf(&Version::new());
    let parent = Prefix::<S<S<Z>>>::containing(&path);
    // One listed child admits one positional reaction; the second Match
    // must fail at its own frame with the reply still unterminated.
    let frames: Vec<Frame<()>> = vec![
        Frame::Reaction(WireReaction::Match, Flow::Continue),
        Frame::Reaction(WireReaction::Match, Flow::Continue),
    ];

    let decode_error = runtime().block_on(async {
        let mut frames = stream::iter(frames);
        decode_reply::<Local, (), _>(
            Local,
            u64::MAX,
            unbounded(),
            Scope::new(parent.erase(), &[(1, hash(1))]),
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

    let reply = Reply::<<Local as Backend<()>>::Erased> {
        replies: vec![Reaction::Match, Reaction::Match],
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
    assert!(matches!(
        encode_error,
        EncodeError::Scope(ScopeError::UnpositionedMatch)
    ));
}

/// Prefix-free queries require a remaining positional child in both conversion directions.
#[test]
fn an_unpositioned_query_is_rejected_in_both_directions() {
    let path = Path::for_leaf(&Version::new());
    let parent = Prefix::<S<S<Z>>>::containing(&path);
    let listing = vec![(1, hash(1))];
    let frames: Vec<Frame<()>> = vec![Frame::Reaction(
        WireReaction::Query(listing.clone()),
        Flow::End,
    )];

    let decode_error = runtime().block_on(async {
        let mut frames = stream::iter(frames);
        decode_reply::<Local, (), _>(
            Local,
            u64::MAX,
            unbounded(),
            Scope::new(parent.erase(), &[]),
            &mut frames,
        )
        .await
        .err()
        .expect("a query without a child has no derivable scope")
    });
    assert!(matches!(
        decode_error,
        DecodeError::Scope(ScopeError::UnpositionedQuery)
    ));

    let reply = Reply::<<Local as Backend<()>>::Erased> {
        replies: vec![Reaction::Query(listing)],
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
    assert!(matches!(
        encode_error,
        EncodeError::Scope(ScopeError::UnpositionedQuery)
    ));
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
            let expected_frame =
                Frame::Reaction(WireReaction::Query(query_listing.clone()), Flow::End);

            let reply = Reply::<<Local as Backend<()>>::Erased> {
                replies: vec![Reaction::Query(query_listing.clone())],
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
    let path = Path::for_leaf(&Version::new());
    let parent = Prefix::<S<Z>>::containing(&path);
    let mut frames = stream::iter([Frame::<()>::End(End::Stream)]);

    let error = runtime()
        .block_on(decode_leaf_reply(
            Local,
            u64::MAX,
            unbounded(),
            Scope::new(parent.erase(), &[]),
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

fn under_root_pair() -> [(Version, Message, Path); 2] {
    let mut by_radix: BTreeMap<u8, Vec<(Version, Message, Path)>> = BTreeMap::new();
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

/// Consecutive leaves in one version-derived run assemble as one node and reexplode exactly.
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
    let scope = Scope::opening(&[]);

    let reencoded = runtime().block_on(async {
        let mut input = stream::iter(frames.clone());
        let decoded =
            decode_reply::<Local, u64, _>(Local, u64::MAX, unbounded(), scope.clone(), &mut input)
                .await
                .expect("ascending in-scope leaves assemble");
        assert_eq!(decoded.reply.replies.len(), 1);
        let [Reaction::Supply(_, node)] = decoded.reply.replies.as_slice() else {
            panic!("one leaf run must become one supplied node")
        };
        let supplied_prefix = Prefix::<UnderRoot>::containing(&leaves[0].2);
        let rebuilt = Local
            .leaves(
                supplied_prefix,
                <Local as Backend<u64>>::assume::<UnderRoot>(node.clone()),
            )
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
        decode_reply::<Local, u64, _>(
            Local,
            u64::MAX,
            unbounded(),
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
        WireReaction::Supply(leaf_run::<u64>(&[
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
        decode_reply::<Local, u64, _>(
            Local,
            u64::MAX,
            unbounded(),
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
    assert_eq!(source.kind(), std::io::ErrorKind::UnexpectedEof);
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
            WireReaction::Supply(leaf_run::<u64>(&[(&leaf.version, &leaf.message)])),
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

/// A reply streaming past the declared `set_len` fails typed at its first
/// over-declaration record, under node residency independent of the
/// overrun; a declaration exactly covering the stream admits it whole.
///
/// The peer's greeting-declared `set_len` is a premise the session's
/// window solve prices, and the decoder charges it per record before the
/// payload takes backend custody. Metered by the node census (the
/// crate's exact residency shadow): the boundary case pins the meter
/// alive (an admitted stream's every leaf is resident at completion),
/// and the rejection case pins residency equal across a doubled
/// overrun, so custody provably stops at the charge rather than at the
/// reply boundary.
#[test]
fn a_reply_past_the_declared_set_len_fails_at_its_first_over_record() {
    use crate::tree::mirror::streaming::materialized::SupplyLedger;
    use crate::tree::typed::untyped::census;

    const SMALL: u64 = 128;
    const LARGE: u64 = 256;

    /// Decode one whole-root reply of `count` leaves under a declared
    /// allowance of `declared`, returning the outcome and the peak
    /// node-handle residency beyond the pre-decode baseline.
    #[allow(clippy::type_complexity)]
    fn decode_metered(
        count: u64,
        declared: u64,
    ) -> (Result<usize, DecodeError<Infallible>>, usize) {
        let frames = whole_root_supply_reply(&ascending_leaves(count));
        census::reset_peak();
        let (live, _) = census::read();
        let decoded = runtime().block_on(async {
            let mut input = stream::iter(frames);
            decode_reply::<Local, u64, _>(
                Local,
                u64::MAX,
                SupplyLedger::new(declared),
                Scope::opening(&[]),
                &mut input,
            )
            .await
        });
        let (_, peak) = census::read();
        (
            decoded.map(|decoded| decoded.reply.replies.len()),
            peak - live,
        )
    }

    // The no-false-positive boundary, doubling as the meter's liveness
    // floor: a declaration exactly covering the stream admits every
    // record, and every admitted leaf is resident at completion.
    let (admitted, residency) = decode_metered(SMALL, SMALL);
    admitted.expect("a declaration exactly covering the stream admits it");
    assert!(
        residency >= SMALL as usize,
        "the census meter is alive: an admitted {SMALL}-leaf reply holds \
         {residency} resident handles",
    );

    // The rejection: an allowance of one fails at the second record,
    // while the reply is still open.
    let overdrawn = |count: u64| {
        let (result, residency) = decode_metered(count, 1);
        let error = result.expect_err(
            "undetected over-supply: a reply past the declared set length \
             must fail at ingress, at its first over-declaration record",
        );
        assert!(
            matches!(error, DecodeError::OverdrawnSupply { declared: 1 }),
            "mistyped over-supply rejection: {error:?}",
        );
        residency
    };
    let small = overdrawn(SMALL);
    let large = overdrawn(LARGE);
    assert_eq!(
        small, large,
        "residency at rejection is independent of the streamed overrun",
    );
    assert!(
        small < SMALL as usize,
        "custody stops at the charge: {small} resident handles against a \
         {SMALL}-leaf stream",
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
        decode_reply::<Local, u64, _>(
            Local,
            u64::MAX,
            unbounded(),
            Scope::opening(&[(1, hash(1))]),
            &mut input,
        )
        .await
        .err()
        .expect("a keyed supply may occupy only one ascending run")
    });
    assert!(matches!(error, DecodeError::SupplyOrder { .. }));
}
