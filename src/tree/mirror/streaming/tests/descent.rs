//! An offline model of the descent, checked reply for reply against real
//! sessions.
//!
//! [`predict`] computes every reply a session's two walks build from the
//! replicas' leaves alone, without a tree, a backend, or a channel. Each
//! side contributes its leaves' paths, and for each leaf whether the peer's
//! greeting version leaves it unknown, which is all the deletion filter
//! reads. The suite then runs real sessions, in process and over a pipe,
//! records every reply each walk publishes ([`with_transcript`]), and
//! requires the two to agree exactly: per endpoint and logical stream, the
//! same replies in the same order, each with the same reactions.
//!
//! The model rests on four facts about the protocol:
//!
//! - A node's children, as `children_of` yields them, are the distinct next
//!   bytes of the keys beneath it, whatever its storage compresses.
//! - Two subtrees match exactly when their key sets match, since a node's
//!   hash covers exactly its leaves.
//! - The reply about a node held by both sides holds one reaction per child
//!   either side holds, in radix order: a match where both sides' subtrees
//!   are equal; a query listing the replier's children of the child where
//!   both hold it with different contents; a supply where only the replier
//!   holds it and some leaf beneath it is unknown to the asker, and nothing
//!   where every such leaf is known; and an empty query where only the asker
//!   holds it. The asker answers that empty query with one supply per child
//!   holding a leaf unknown to the replier, or at the last level with the
//!   leaf itself if it is unknown.
//! - Within one logical stream, replies leave in prefix order: each stage
//!   answers the questions of the replies it takes, in the order it asked
//!   them, and each reply's questions in radix order.
//!
//! Because a reply's contents depend only on the two trees, the walk's
//! schedule cannot change them. The suite therefore varies the window and
//! the transport and expects one prediction to hold for all of them; that
//! independence is the property that later lets the model price, offline,
//! every reply a session can invite.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use proptest::prelude::*;

use super::pipe::placed_over_pipe;
use super::skeleton::Party;
use super::{LocalSession, left_initiates};
use crate::tree::Root;
use crate::tree::arb::{
    arb_deep_divergent_pair, arb_divergent_pair, arb_wide_divergent_pair, nth_party,
};
use crate::tree::mirror::contained;
use crate::tree::mirror::streaming::materialized::transcript::{
    Label, Transcript, with_transcript,
};
use crate::tree::mirror::streaming::testing::session::{AMPLE, completed, over_pipe};
use crate::tree::mirror::streaming::window::{Window, WindowConfig};
use crate::tree::typed::{Node as TreeNode, Path};
use crate::tree::{Action, Tree};
use crate::{causally, message::Message};

/// Bytes in a leaf path: one per level of the tree.
const PATH_BYTES: usize = 32;

/// The typed height of a reply's children when the reply is about a node
/// `depth` bytes below the root: the root's reply has children at height
/// 31, and a reply about a leaf's parent has children at height 0.
fn reply_height(depth: usize) -> usize {
    PATH_BYTES - 1 - depth
}

/// One of a side's leaves, as the model sees it.
#[derive(Clone, Copy, Debug)]
struct ModelLeaf {
    /// The leaf's path.
    path: [u8; PATH_BYTES],
    /// Whether the peer's greeting version leaves the leaf unknown, so
    /// that the deletion filter lets it travel.
    unknown: bool,
}

/// `side`'s leaves in path order, each marked by whether `peer`'s greeting
/// version leaves it unknown.
fn model_leaves(side: &Root, peer: &Root) -> Vec<ModelLeaf> {
    let mut leaves: Vec<ModelLeaf> = TreeNode::range_owned(side.node.as_ref(), causally::all())
        .map(|(path, leaf)| ModelLeaf {
            path,
            unknown: !contained(leaf.version(), &peer.ceiling),
        })
        .collect();
    leaves.sort_unstable_by_key(|leaf| leaf.path);
    leaves
}

/// Split `leaves`, which share their first `depth` bytes and are in path
/// order, into runs by their byte at `depth`: the children `children_of`
/// yields for the node they sit under.
fn children(leaves: &[ModelLeaf], depth: usize) -> Vec<(u8, &[ModelLeaf])> {
    let mut runs = Vec::new();
    let mut rest = leaves;
    while let Some(first) = rest.first() {
        let radix = first.path[depth];
        let split = rest.partition_point(|leaf| leaf.path[depth] <= radix);
        let (run, tail) = rest.split_at(split);
        runs.push((radix, run));
        rest = tail;
    }
    runs
}

/// Whether a subtree holding `leaves` travels in a supply: some leaf in it
/// is unknown to the receiver.
fn travels(leaves: &[ModelLeaf]) -> bool {
    leaves.iter().any(|leaf| leaf.unknown)
}

/// Whether two subtrees hold the same keys, and so match.
fn same_keys(ours: &[ModelLeaf], theirs: &[ModelLeaf]) -> bool {
    ours.len() == theirs.len() && ours.iter().zip(theirs).all(|(a, b)| a.path == b.path)
}

/// Every reply a session builds, per role and reply height, in the order
/// its logical stream carries them.
type Prediction = BTreeMap<(Party, usize), Vec<Vec<Label>>>;

/// One reply the model predicts, before it is sorted into its stream.
struct Predicted {
    /// The side that builds it.
    by: Party,
    /// Its children's typed height, which names its logical stream.
    height: usize,
    /// The prefix that orders it within its stream: the node it is about,
    /// or for a leaf request's reply, the leaf's whole path.
    order: Vec<u8>,
    /// Its reactions, payload-erased.
    labels: Vec<Label>,
}

/// One node whose dispute reply the model still owes.
struct Dispute<'k> {
    /// The replier's leaves beneath the node.
    replier: &'k [ModelLeaf],
    /// The asker's leaves beneath the node.
    asker: &'k [ModelLeaf],
    /// The node's prefix.
    prefix: Vec<u8>,
    /// The side that replies.
    by: Party,
}

/// Predict every reply of a session between `responder` and `initiator`,
/// each side's leaves in path order, whose greeting versions differ.
///
/// The descent iterates over an explicit stack rather than recursing, so
/// the model's own depth never depends on its input.
fn predict(responder: &[ModelLeaf], initiator: &[ModelLeaf]) -> Prediction {
    let mut replies = vec![Predicted {
        by: Party::I,
        height: reply_height(0),
        order: Vec::new(),
        labels: vec![Label::Query(
            children(initiator, 0)
                .iter()
                .map(|(radix, _)| *radix)
                .collect(),
        )],
    }];
    let mut stack = vec![Dispute {
        replier: responder,
        asker: initiator,
        prefix: Vec::new(),
        by: Party::R,
    }];
    while let Some(dispute) = stack.pop() {
        let reply = dispute_reply(dispute, &mut stack, &mut replies);
        replies.push(reply);
    }
    let mut prediction = Prediction::new();
    replies.sort_by(|a, b| a.order.cmp(&b.order));
    for reply in replies {
        prediction
            .entry((reply.by, reply.height))
            .or_default()
            .push(reply.labels);
    }
    prediction
}

/// The reply about one disputed node, pushing the disputes its questions
/// open onto `stack` and the request replies its empty questions invite
/// onto `replies`.
fn dispute_reply<'k>(
    Dispute {
        replier,
        asker,
        prefix,
        by,
    }: Dispute<'k>,
    stack: &mut Vec<Dispute<'k>>,
    replies: &mut Vec<Predicted>,
) -> Predicted {
    let depth = prefix.len();
    let ours = children(replier, depth);
    let theirs = children(asker, depth);
    let mut labels = Vec::new();
    let (mut ours, mut theirs) = (ours.into_iter().peekable(), theirs.into_iter().peekable());
    loop {
        let child = |radix: u8| {
            let mut child = prefix.clone();
            child.push(radix);
            child
        };
        // Which side's next child comes first in radix order; a side with
        // no children left comes last.
        let next = match (ours.peek(), theirs.peek()) {
            (None, None) => break,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some((mine, _)), Some((yours, _))) => mine.cmp(yours),
        };
        match next {
            Ordering::Equal => {
                let (mine, held) = ours.next().expect("peeked");
                let (_, asked) = theirs.next().expect("peeked");
                if same_keys(held, asked) {
                    labels.push(Label::Match);
                } else {
                    // Below a leaf's parent every child is a leaf, and
                    // equal paths are equal leaves, so `depth + 1` is a
                    // real level here.
                    labels.push(Label::Query(
                        children(held, depth + 1)
                            .iter()
                            .map(|(radix, _)| *radix)
                            .collect(),
                    ));
                    stack.push(Dispute {
                        replier: asked,
                        asker: held,
                        prefix: child(mine),
                        by: by.other(),
                    });
                }
            }
            Ordering::Less => {
                let (mine, held) = ours.next().expect("peeked");
                if travels(held) {
                    labels.push(Label::Supply(mine));
                }
            }
            Ordering::Greater => {
                let (yours, asked) = theirs.next().expect("peeked");
                labels.push(Label::Query(Vec::new()));
                replies.push(request_reply(asked, child(yours), by.other()));
            }
        }
    }
    Predicted {
        by,
        height: reply_height(depth),
        order: prefix,
        labels,
    }
}

/// The reply `by` builds to an empty question about the node at `prefix`,
/// under which it holds `leaves`: one supply per child that travels, or at
/// the last level the leaf itself if it travels.
fn request_reply(leaves: &[ModelLeaf], prefix: Vec<u8>, by: Party) -> Predicted {
    let depth = prefix.len();
    let (height, labels) = if depth == PATH_BYTES {
        let radix = prefix[PATH_BYTES - 1];
        let labels = if travels(leaves) {
            vec![Label::Supply(radix)]
        } else {
            Vec::new()
        };
        (reply_height(PATH_BYTES - 1), labels)
    } else {
        let labels = children(leaves, depth)
            .into_iter()
            .filter(|(_, child)| travels(child))
            .map(|(radix, _)| Label::Supply(radix))
            .collect();
        (reply_height(depth), labels)
    };
    Predicted {
        by,
        height,
        order: prefix,
        labels,
    }
}

/// The model's prediction for a session between `left` and `right`, or
/// `None` when their greeting versions agree and no descent opens.
fn predict_pair(left: &Root, right: &Root) -> Option<Prediction> {
    if left.ceiling == right.ceiling {
        return None;
    }
    let (initiator, responder) = if left_initiates(left, right) {
        (left, right)
    } else {
        (right, left)
    };
    Some(predict(
        &model_leaves(responder, initiator),
        &model_leaves(initiator, responder),
    ))
}

/// A transcript's replies per endpoint and logical stream, the endpoint
/// named by its work identity.
fn recorded(transcript: &Transcript) -> BTreeMap<usize, BTreeMap<usize, Vec<Vec<Label>>>> {
    let mut endpoints: BTreeMap<usize, BTreeMap<usize, Vec<Vec<Label>>>> = BTreeMap::new();
    for sent in transcript.sent() {
        endpoints
            .entry(sent.work)
            .or_default()
            .entry(sent.height)
            .or_default()
            .push(sent.labels.clone());
    }
    endpoints
}

/// The prediction's replies for one role, per logical stream.
fn role(prediction: &Prediction, party: Party) -> BTreeMap<usize, Vec<Vec<Label>>> {
    prediction
        .iter()
        .filter(|((by, _), _)| *by == party)
        .map(|((_, height), replies)| (*height, replies.clone()))
        .collect()
}

/// Require `transcript` to hold exactly the replies the model predicts for
/// a session between `left` and `right`.
///
/// The transcript names endpoints by work identity, which says nothing
/// about roles, so the check takes the two endpoints as an unordered pair:
/// one must have built exactly the initiator's predicted replies and the
/// other exactly the responder's.
fn assert_matches(left: &Root, right: &Root, transcript: &Transcript) -> Result<(), TestCaseError> {
    let endpoints = recorded(transcript);
    let Some(prediction) = predict_pair(left, right) else {
        prop_assert!(
            endpoints.is_empty(),
            "equal greeting versions open no descent"
        );
        return Ok(());
    };
    let expected = [role(&prediction, Party::I), role(&prediction, Party::R)];
    let actual: Vec<_> = endpoints.into_values().collect();
    prop_assert!(
        actual.len() == 2
            && (actual[..] == expected[..]
                || [&actual[1], &actual[0]] == [&expected[0], &expected[1]]),
        "the model predicts every reply both walks build:\npredicted {expected:#?}\nrecorded {actual:#?}"
    );
    Ok(())
}

/// The in-process session's transcript between `left` and `right` at
/// `window`.
fn local_transcript(left: Root, right: Root, window: WindowConfig) -> Transcript {
    let mut outcome = LocalSession::new(left, right)
        .window(window)
        .transcript()
        .run();
    let transcript = outcome
        .transcript
        .take()
        .expect("the session was asked to record its transcript");
    outcome.sides();
    transcript
}

/// The wire session's transcript between `left` and `right`, both
/// endpoints at `window`, with leaf-path derivation following the pair's
/// placement when `placed`.
fn wire_transcript(left: Root, right: Root, window: WindowConfig, placed: bool) -> Transcript {
    let (outcome, transcript) = with_transcript(|| {
        if placed {
            placed_over_pipe((left, right), [window; 2], AMPLE)
        } else {
            over_pipe::<()>((left, right), [window; 2], AMPLE)
        }
    });
    completed(outcome.expect("the session completes under the closed-world poller"));
    transcript
}

/// The windows the suite runs every pair at: the one-slot floor and a
/// uniform window wide enough to pipeline every fixture here.
fn arb_window() -> impl Strategy<Value = WindowConfig> {
    prop_oneof![
        Just(WindowConfig::FLOOR),
        Just(WindowConfig::Fixed(Window::uniform(64))),
    ]
}

proptest! {
    /// The model predicts every reply both walks build, reaction for
    /// reaction, in process and over a pipe, at any window.
    ///
    /// The pairs are version-addressed, with shared history and redaction,
    /// which the deletion filter prunes.
    #[test]
    fn model_predicts_every_reply(
        (left, right) in prop_oneof![arb_divergent_pair(), arb_wide_divergent_pair()],
        window in arb_window(),
    ) {
        let local = local_transcript(left.clone(), right.clone(), window);
        assert_matches(&left, &right, &local)?;
        let wire = wire_transcript(left.clone(), right.clone(), window, false);
        assert_matches(&left, &right, &wire)?;
    }

    /// The model predicts every reply of sessions over pairs placed below a
    /// shared prefix of drawn length, whose disputes reach every depth of
    /// the tree, in process and over a pipe.
    #[test]
    fn model_predicts_every_deep_reply(
        (left, right) in arb_deep_divergent_pair(),
        window in arb_window(),
    ) {
        let local = local_transcript(left.clone(), right.clone(), window);
        assert_matches(&left, &right, &local)?;
        let wire = wire_transcript(left.clone(), right.clone(), window, true);
        assert_matches(&left, &right, &wire)?;
    }
}

/// Messages both forks of [`model_predicts_forked_replicas`] share.
const FORK_SHARED: usize = 2_000;

/// Messages each fork of [`model_predicts_forked_replicas`] sends that the
/// other lacks.
const FORK_OWN: usize = 2_000;

/// Two forks of one network: both hold [`FORK_SHARED`] messages and each
/// then sends [`FORK_OWN`] the other lacks, one of them redacting every
/// third shared message.
fn forked_replicas() -> (Root, Root) {
    let seed = nth_party(0);
    let mut base = Tree::<()>::new();
    base.act(
        &seed,
        (0..FORK_SHARED).map(|_| Action::Insert(Message::new(()))),
    );
    let shared: Vec<_> = base
        .iter()
        .map(|(version, _)| Path::for_leaf(version))
        .collect();
    let fork = |index: usize, redact: bool| {
        let party = nth_party(index);
        let mut tree = base.clone();
        tree.act(
            &party,
            (0..FORK_OWN).map(|_| Action::Insert(Message::new(()))),
        );
        if redact {
            tree.act(
                &party,
                shared.iter().step_by(3).map(|path| Action::Forget(*path)),
            );
        }
        tree.root
    };
    (fork(1, true), fork(2, false))
}

/// The model predicts every reply of sessions between two forks, in
/// process and over a pipe.
///
/// The forks are large enough that every depth-1 node is disputed, and one
/// fork's redactions reach the deletion filter.
#[test]
fn model_predicts_forked_replicas() {
    let (left, right) = forked_replicas();
    let window = WindowConfig::Fixed(Window::uniform(64));
    let local = local_transcript(left.clone(), right.clone(), window);
    assert_matches(&left, &right, &local).expect("the model predicts the local session");
    let wire = wire_transcript(left.clone(), right.clone(), window, false);
    assert_matches(&left, &right, &wire).expect("the model predicts the wire session");
}
