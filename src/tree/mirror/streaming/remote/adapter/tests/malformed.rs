//! Focused malformed-wire cases which are not naturally height-parametric.

use std::{collections::BTreeMap, convert::Infallible};

use before::Version;
use futures::{StreamExt, TryStreamExt, stream};

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
    LeafCase, hash, runtime,
};
use crate::tree::mirror::streaming::message::{Reaction, Reply};
use crate::tree::mirror::streaming::remote::codec::{End, Flow, Frame, Reaction as WireReaction};

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
        decode_leaf_reply(Local, Scope::new(parent, &[(0, hash(0))]), &mut frames)
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
        decode_leaf_reply(Local, Scope::new(parent, &[(0, hash(0))]), &mut frames)
            .await
            .err()
            .expect("reply without a boundary is truncated")
    });
    assert!(matches!(error, DecodeError::TruncatedReply));
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
        decode_reply::<Local, (), Z, _>(Local, Scope::new(parent, &[]), &mut frames)
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
        encode_reply(Local, Scope::new(parent, &[]), reply)
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
                encode_leaf_reply(Local, Scope::new(parent, &scope_listing), reply)
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
                decode_leaf_reply(Local, Scope::new(parent, &scope_listing), &mut frames).await
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
            WireReaction::Supply(leaves[0].0.clone(), leaves[0].1.clone()),
            Flow::Continue,
        ),
        Frame::Reaction(
            WireReaction::Supply(leaves[1].0.clone(), leaves[1].1.clone()),
            Flow::End,
        ),
    ];
    let scope = Scope::<UnderRoot>::opening(&[]);

    let reencoded = runtime().block_on(async {
        let mut input = stream::iter(frames.clone());
        let decoded =
            decode_reply::<Local, u64, UnderUnderRoot, _>(Local, scope.clone(), &mut input)
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

        encode_reply(Local, scope, decoded.reply)
            .map_ok(|encoded| encoded.into_parts().0)
            .try_collect::<Vec<_>>()
            .await
            .expect("rebuilt subtree reexplodes")
    });
    assert_eq!(reencoded, frames);
}

/// Interrupting a supply run finalizes its radix, so later resumption is rejected as reordering.
#[test]
fn a_supply_run_cannot_resume_after_another_reaction() {
    let leaves = under_root_pair();
    let frames = vec![
        Frame::Reaction(
            WireReaction::Supply(leaves[0].0.clone(), leaves[0].1.clone()),
            Flow::Continue,
        ),
        Frame::Reaction(WireReaction::Match, Flow::Continue),
        Frame::Reaction(
            WireReaction::Supply(leaves[1].0.clone(), leaves[1].1.clone()),
            Flow::End,
        ),
    ];

    let error = runtime().block_on(async {
        let mut input = stream::iter(frames);
        decode_reply::<Local, u64, UnderUnderRoot, _>(Local, Scope::opening(&[]), &mut input)
            .await
            .err()
            .expect("a keyed supply may occupy only one ascending run")
    });
    assert!(matches!(error, DecodeError::SupplyOrder { .. }));
}
