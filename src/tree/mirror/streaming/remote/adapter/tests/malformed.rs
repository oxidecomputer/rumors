//! Focused malformed-wire cases which are not naturally height-parametric.

use std::collections::BTreeMap;

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
        DecodeError, EncodeError, Scope, ScopeError, decode_leaf_reply, decode_reply, encode_reply,
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
    let frames = vec![Frame::Reaction(
        WireReaction::Query(listing.clone()),
        Flow::End(End::Reply),
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
        encode_reply(Local, Scope::new(parent, &[]), reply, End::Reply)
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
            Flow::End(End::Reply),
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

        encode_reply(Local, scope, decoded.reply, End::Reply)
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
            Flow::End(End::Reply),
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
