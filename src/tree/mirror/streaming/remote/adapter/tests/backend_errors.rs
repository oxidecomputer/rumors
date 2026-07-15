//! Source-error propagation across every backend operation reachable by the adapter.

use std::convert::Infallible;

use futures::{StreamExt, stream};

use crate::tree::{
    mirror::streaming::{
        Failing, FailingNode, Failure, Leaf, Local, Operation,
        message::{Reaction, Reply},
    },
    typed::{
        self, Prefix,
        height::{Height, S, Z},
    },
};

use super::{
    super::{DecodeError, EncodeError, Scope, decode_reply, encode_reply},
    LeafCase, hash, runtime,
};
use crate::tree::mirror::streaming::{
    convert::Convert,
    remote::codec::{End, Flow, Frame, Reaction as WireReaction},
};

/// Construct the same one-leaf subtree at any concrete reply height.
trait BackendHeight: Convert {
    fn node(leaf: &LeafCase) -> typed::Node<u64, Self>;
}

impl BackendHeight for Z {
    fn node(leaf: &LeafCase) -> typed::Node<u64, Self> {
        <typed::Node<u64, Z> as Leaf<u64>>::leaf(leaf.version.clone(), leaf.message.clone())
    }
}

impl<H> BackendHeight for S<H>
where
    H: BackendHeight,
    S<H>: Convert,
{
    fn node(leaf: &LeafCase) -> typed::Node<u64, Self> {
        let path: [u8; 32] = leaf.path().into();
        typed::Node::beneath(H::node(leaf), path[31 - H::HEIGHT])
    }
}

/// Exercise every possible injected operation for one non-leaf reply height.
trait FailureHeight: BackendHeight {
    fn source_errors_are_atomic(leaf: &LeafCase, runtime: &tokio::runtime::Runtime);
}

impl<H> FailureHeight for S<H>
where
    H: BackendHeight,
    S<H>: Convert,
    S<S<H>>: Height,
{
    fn source_errors_are_atomic(leaf: &LeafCase, runtime: &tokio::runtime::Runtime) {
        let (parent, supply_radix) = Prefix::<Self>::containing(&leaf.path()).pop();
        let positional_radix = if supply_radix < u8::MAX {
            supply_radix + 1
        } else {
            supply_radix - 1
        };
        let listing = [(positional_radix, hash(positional_radix))];

        for fail_after in 0..Self::HEIGHT {
            let backend = Failing::after(Local, fail_after);
            let supply = FailingNode::new(Self::node(leaf));
            let replies = if supply_radix < u8::MAX {
                vec![
                    Reaction::Supply(supply_radix, supply),
                    Reaction::Query(Vec::new()),
                ]
            } else {
                vec![
                    Reaction::Query(Vec::new()),
                    Reaction::Supply(supply_radix, supply),
                ]
            };
            let mut encoded = encode_reply(
                backend.clone(),
                Scope::new(parent, &listing),
                Reply { replies },
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
                "height {} failure {fail_after} published a frame or question",
                Self::HEIGHT,
            );
            assert!(ended, "an encoding stream continued after its source error");
            assert_encode_failure(
                error,
                Operation::Children {
                    height: Self::HEIGHT - fail_after,
                },
            );
            assert_eq!(
                backend.history(),
                (0..=fail_after)
                    .map(|step| Operation::Children {
                        height: Self::HEIGHT - step,
                    })
                    .collect::<Vec<_>>(),
            );

            let backend = Failing::after(Local, fail_after);
            let sentinel = Frame::End(End::Reply);
            let mut frames = stream::iter([
                Frame::Reaction(
                    WireReaction::Supply(leaf.version.clone(), leaf.message.clone()),
                    Flow::End,
                ),
                sentinel.clone(),
            ]);
            let error = runtime
                .block_on(decode_reply::<Failing<Local>, u64, H, _>(
                    backend.clone(),
                    Scope::new(parent, &[]),
                    &mut frames,
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

fn assert_encode_failure(error: EncodeError<Failure<Infallible>>, expected: Operation) {
    assert!(matches!(
        error,
        EncodeError::Backend(Failure::Injected(actual)) if actual == expected
    ));
}

fn assert_decode_failure(error: DecodeError<Failure<Infallible>>, expected: Operation) {
    assert!(matches!(
        error,
        DecodeError::Backend(Failure::Injected(actual)) if actual == expected
    ));
}

/// Recurse from a runtime height to its type-level backend-error test.
macro_rules! dispatch_height {
    ($height:expr, $leaf:expr, $runtime:expr; $type:ty, $number:expr; _ $($rest:tt)*) => {
        if $height == $number {
            <$type as FailureHeight>::source_errors_are_atomic($leaf, $runtime)
        } else {
            dispatch_height!($height, $leaf, $runtime; S<$type>, $number + 1; $($rest)*)
        }
    };
    ($height:expr, $leaf:expr, $runtime:expr; $type:ty, $number:expr;) => {
        panic!("reply height {} is outside the backend-error matrix", $height)
    };
}

macro_rules! at_height {
    ($height:expr, $leaf:expr, $runtime:expr) => {
        dispatch_height!($height, $leaf, $runtime; S<Z>, 1;
            _ _ _ _ _ _ _ _
            _ _ _ _ _ _ _ _
            _ _ _ _ _ _ _ _
            _ _ _ _ _ _ _
        )
    };
}

/// Every reachable explode/assemble operation reports its exact injected
/// source error and makes no frame, question, partial reply, or later call
/// observable after the failure.
#[test]
fn backend_source_failures_are_exhaustive_and_atomic() {
    let leaf = LeafCase::new(0xfeed_face, 7);
    let runtime = runtime();
    for height in 1..32 {
        at_height!(height, &leaf, &runtime);
    }
}
