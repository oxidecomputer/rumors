//! Source-error propagation across the backend traversal operations used by the adapter.

use std::convert::Infallible;

use futures::{StreamExt, stream};

use crate::tree::{
    mirror::streaming::{
        Backend, Failing, FailingNode, Failure, Local, Operation,
        erased::{Reaction, Reply},
    },
    typed::{
        Prefix,
        height::{Height, S},
    },
};

use super::{
    super::{DecodeError, EncodeError, Scope, decode_reply, encode_reply},
    LeafCase, NodeAt, codec, hash, leaf_run, runtime, unbounded,
};
use crate::tree::mirror::streaming::{
    convert::Convert,
    remote::codec::{End, Flow, Frame, Reaction as WireReaction, RunBudget},
};

/// Exercise every possible injected operation for one non-leaf reply height.
trait FailureHeight: NodeAt {
    /// Exercise every traversal failure point at this reply height.
    fn source_errors_are_atomic(leaf: &LeafCase, runtime: &tokio::runtime::Runtime);
}

/// Walk every injectable child and parent operation for a branch height.
impl<H> FailureHeight for S<H>
where
    H: NodeAt,
    S<H>: Convert,
    S<S<H>>: Height,
{
    fn source_errors_are_atomic(leaf: &LeafCase, runtime: &tokio::runtime::Runtime) {
        let (parent, supply_radix) = Prefix::<Self>::containing(&leaf.path()).pop();
        assert!(
            (2..u8::MAX).contains(&supply_radix),
            "the fixture needs two lower radixes and one higher radix"
        );

        for fail_after in 0..Self::HEIGHT {
            // A failing supply may appear on either side of a positional
            // reaction. Neither ordering may publish the pending frame.
            for (listing, reactions) in [
                (
                    vec![(supply_radix + 1, hash(supply_radix + 1))],
                    vec![
                        Reaction::Supply(
                            supply_radix,
                            <Failing<Local> as Backend>::erase(FailingNode::new(Self::node(leaf))),
                        ),
                        Reaction::Query(Vec::new()),
                    ],
                ),
                (
                    vec![(supply_radix - 1, hash(supply_radix - 1))],
                    vec![
                        Reaction::Query(Vec::new()),
                        Reaction::Supply(
                            supply_radix,
                            <Failing<Local> as Backend>::erase(FailingNode::new(Self::node(leaf))),
                        ),
                    ],
                ),
            ] {
                let backend = Failing::after(Local, fail_after);
                let mut encoded = encode_reply(
                    backend.clone(),
                    RunBudget::default(),
                    Scope::new(parent.erase(), &listing),
                    Reply { reactions },
                );
                let (yielded, error, ended) = runtime.block_on(async {
                    let mut yielded = Vec::new();
                    let error = loop {
                        match encoded.next().await {
                            Some(Ok(frame)) => yielded.push(frame.into_parts()),
                            Some(Err(error)) => break error,
                            None => panic!("the injected encoding failure was not reached"),
                        }
                    };
                    let ended = encoded.next().await.is_none();
                    (yielded, error, ended)
                });
                assert!(
                    yielded.is_empty(),
                    "height {} failure {fail_after} published its pending frame",
                    Self::HEIGHT,
                );
                assert!(ended, "an encoding stream continued after its source error");
                assert_encode_failure(
                    error,
                    Operation::Children {
                        height: Self::HEIGHT - fail_after,
                    },
                );
                assert_history(&backend, fail_after, Self::HEIGHT);
            }

            // Once a frame has been yielded it remains valid, but the frame
            // pending when traversal fails must not escape with a false End.
            let backend = Failing::after(Local, fail_after);
            let lower = [supply_radix - 2, supply_radix - 1];
            let mut encoded = encode_reply(
                backend.clone(),
                RunBudget::default(),
                Scope::new(parent.erase(), &lower.map(|radix| (radix, hash(radix)))),
                Reply {
                    reactions: vec![
                        Reaction::Match,
                        Reaction::Match,
                        Reaction::Supply(
                            supply_radix,
                            <Failing<Local> as Backend>::erase(FailingNode::new(Self::node(leaf))),
                        ),
                    ],
                },
            );
            let (yielded, error) = runtime.block_on(async {
                let first = encoded
                    .next()
                    .await
                    .expect("the first completed frame is published")
                    .expect("the first match needs no backend traversal")
                    .into_parts();
                let error = match encoded.next().await {
                    Some(Err(error)) => error,
                    Some(Ok(_)) => panic!("the pending frame escaped after a source failure"),
                    None => panic!("the injected encoding failure was not reported"),
                };
                assert!(
                    encoded.next().await.is_none(),
                    "the failed stream terminates"
                );
                (first, error)
            });
            assert_eq!(
                yielded,
                (Frame::Reaction(WireReaction::Match, Flow::Continue), None,),
            );
            assert_encode_failure(
                error,
                Operation::Children {
                    height: Self::HEIGHT - fail_after,
                },
            );
            assert_history(&backend, fail_after, Self::HEIGHT);

            let backend = Failing::after(Local, fail_after);
            let sentinel = Frame::End(End::Reply);
            let mut frames = stream::iter([
                Frame::Reaction(
                    WireReaction::Supply(leaf_run(&[(&leaf.version, &leaf.message)])),
                    Flow::End,
                ),
                sentinel.clone(),
            ]);
            let error = runtime
                .block_on(decode_reply::<Failing<Local>, _>(
                    backend.clone(),
                    u64::MAX,
                    unbounded(),
                    Scope::new(parent.erase(), &[]),
                    &mut frames,
                    codec(),
                ))
                .err()
                .expect("the injected decoding failure was not reached");
            assert_decode_failure(
                error,
                Operation::Parent {
                    height: fail_after + 1,
                },
            );
            assert_eq!(
                backend.history(),
                (1..=fail_after + 1)
                    .map(|height| Operation::Parent { height })
                    .collect::<Vec<_>>(),
            );
            assert_eq!(
                runtime.block_on(frames.next()),
                Some(sentinel),
                "decoding consumed the following reply after a backend failure",
            );
        }
    }
}

/// Require exactly the child traversals up to the injected failure.
fn assert_history(backend: &Failing<Local>, fail_after: usize, height: usize) {
    assert_eq!(
        backend.history(),
        (0..=fail_after)
            .map(|step| Operation::Children {
                height: height - step,
            })
            .collect::<Vec<_>>(),
    );
}

/// Require the exact injected operation in an encoder failure.
fn assert_encode_failure(error: EncodeError<Failure<Infallible>>, expected: Operation) {
    match error {
        EncodeError::Backend(Failure::Injected(actual)) => assert_eq!(actual, expected),
        other => panic!("unexpected encode failure: {other:?}"),
    }
}

/// Require the exact injected operation in a decoder failure.
fn assert_decode_failure(error: DecodeError<Failure<Infallible>>, expected: Operation) {
    match error {
        DecodeError::Backend(Failure::Injected(actual)) => assert_eq!(actual, expected),
        other => panic!("unexpected decode failure: {other:?}"),
    }
}

/// Every injectable traversal reports its exact source error. Encoding keeps
/// frames completed before the failure but withholds the pending frame and all
/// later work; decoding leaves the following reply untouched.
#[test]
fn backend_source_failures_are_exhaustive_and_atomic() {
    let leaf = LeafCase::new(0xfeed_face, 7);
    let runtime = runtime();
    for height in 1..32 {
        at_height!(
            height,
            FailureHeight::source_errors_are_atomic(&leaf, &runtime);
            1..32
        );
    }
}
