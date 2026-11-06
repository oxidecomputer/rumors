//! The supply-run batching contract: byte-budget chunking at the encoder.
//!
//! The decoder accepts any batching (its record loop is chunking-agnostic),
//! so every law here is stated against the *encoder*: records accumulate
//! greedily until the frame's wire size — the [`SUPPLY_FRAME_OVERHEAD`]
//! envelope plus the run body — would outgrow the [`RunBudget`], a run never
//! dips below one record, and a run never spans reactions. Round trips are
//! asserted through the same
//! decode → re-encode path the session uses, with the decode side fed
//! deliberately unbatched input to prove batching is the encoder's choice.

use std::pin::pin;

use futures::{StreamExt, TryStreamExt, stream};
use proptest::prelude::*;

use crate::{
    Version, default,
    message::Message,
    tree::{
        mirror::streaming::{
            Backend, Failing, FailingNode, Local,
            erased::{Reaction, Reply},
            ledger::queued,
            remote::{
                Outbound,
                codec::{
                    Flow, Frame, LeafRun, Reaction as WireReaction, RunBudget,
                    SUPPLY_FRAME_OVERHEAD,
                },
                pipe::{PERMIT_BYTES, WirePool},
            },
        },
        typed::{Prefix, height::UnderRoot},
    },
};

use super::{
    super::{LeafEdge, ReplyEncoder, ReplyLevel, Scope, decode_whole_reply, encode_reply},
    LeafCase, codec, colliding_leaves, leaf_run, reply_frames, runtime, separated_leaves,
    unbounded,
};

/// Inclusive bound on leaves per generated run scenario.
const MAX_CASE_LEAVES: usize = 10;

/// Generate a run size and every relevant byte budget for that exact fixture.
fn run_case() -> impl Strategy<Value = (usize, usize)> {
    (1..=MAX_CASE_LEAVES).prop_flat_map(|count| {
        let leaves = colliding_leaves(count);
        let whole_run = SUPPLY_FRAME_OVERHEAD
            + leaves
                .iter()
                .map(|leaf| LeafRun::record_len(&leaf.version, &leaf.message))
                .sum::<usize>();
        (Just(count), 0..whole_run + 1)
    })
}

/// One single-record supply frame per leaf: the least-batched wire form.
fn unbatched_frames(leaves: &[LeafCase]) -> Vec<Frame> {
    reply_frames(
        leaves
            .iter()
            .map(|leaf| WireReaction::Supply(leaf_run(&[(&leaf.version, &leaf.message)]))),
    )
}

/// Decode `frames` as one reply to the opening scope.
fn decode_opening(frames: Vec<Frame>) -> Reply<<Local as Backend>::Erased> {
    runtime().block_on(async {
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
            Some(&crate::tree::typed::PathKey::fixture()),
        )
        .await
        .expect("ascending in-scope leaves assemble")
    })
}

/// Decode `frames` as one reply to the opening scope, then re-encode it
/// under `budget`, returning the emitted wire frames.
fn recode(frames: Vec<Frame>, budget: RunBudget) -> Vec<Frame> {
    let decoded = decode_opening(frames);
    runtime().block_on(async {
        encode_reply(Local, budget, Scope::opening_unbounded(&[]), decoded)
            .map_ok(|encoded| encoded.into_parts().0)
            .try_collect::<Vec<_>>()
            .await
            .expect("the local backend is infallible")
    })
}

/// Split every emitted frame into its supply run, requiring supplies only.
fn runs_of(frames: &[Frame]) -> Vec<&LeafRun> {
    frames
        .iter()
        .map(|frame| match frame {
            Frame::Reaction(WireReaction::Supply(run), _) => run,
            other => panic!("a pure-supply reply emitted {other:?}"),
        })
        .collect()
}

/// The decoded records of `runs`, flattened in wire order.
fn records_of(runs: &[&LeafRun]) -> Vec<(Version, Message)> {
    runs.iter()
        .flat_map(|run| {
            run.records(codec())
                .collect::<Result<Vec<_>, _>>()
                .expect("an encoder-produced run holds canonical records")
        })
        .collect()
}

proptest! {
    /// For any budget, one reaction's leaves chunk greedily into runs.
    ///
    /// Records accumulate in path order until the next would push the
    /// frame's wire size (envelope plus run body) past the budget; every
    /// run flushed early is full (its successor's first record would not
    /// have fit); no run is empty; only a lone record may push a frame past
    /// the budget. Decoding the chunked frames reproduces the exact leaves,
    /// so chunking is invisible above the wire.
    #[test]
    fn supply_runs_chunk_greedily_by_bytes(
        (count, budget) in run_case(),
    ) {
        let leaves = colliding_leaves(count);
        let frames = recode(unbatched_frames(&leaves), RunBudget::from_bytes(budget));

        // Every frame is a supply run; the reply boundary sits on the last.
        for (position, frame) in frames.iter().enumerate() {
            let expected = if position + 1 == frames.len() {
                Flow::End
            } else {
                Flow::Continue
            };
            prop_assert!(
                matches!(frame, Frame::Reaction(_, flow) if *flow == expected),
                "frame {position} carries the wrong flow"
            );
        }

        let runs = runs_of(&frames);
        for (position, run) in runs.iter().enumerate() {
            let records = run.record_count();
            prop_assert!(records >= 1, "run {position} is empty");
            // The budget bounds the whole wire frame, not just the run body.
            prop_assert!(
                SUPPLY_FRAME_OVERHEAD + run.encoded_len() <= budget || records == 1,
                "run {position} exceeds the budget with {records} records"
            );
            // Greedy: a non-final run flushed only because the next record
            // would have pushed its frame past the budget.
            if position + 1 < runs.len() {
                let (version, message) = runs[position + 1]
                    .records(codec())
                    .next()
                    .expect("a nonempty run yields a first record")
                    .expect("an encoder-produced run holds canonical records");
                prop_assert!(
                    SUPPLY_FRAME_OVERHEAD
                        + run.encoded_len()
                        + LeafRun::record_len(&version, &message)
                        > budget,
                    "run {position} flushed although the next record fit"
                );
            }
        }

        // Chunking loses nothing and reorders nothing.
        let records = records_of(&runs);
        prop_assert_eq!(records.len(), leaves.len());
        for (record, leaf) in records.iter().zip(&leaves) {
            prop_assert_eq!(record.0.as_bytes(), leaf.version.as_bytes());
            prop_assert_eq!(record.1.as_slice(), leaf.message.as_slice());
        }
    }
}

proptest! {
    /// For any budget, the encoder hands each run over as soon as it is
    /// complete, so it never holds a complete run besides the one it is
    /// yielding.
    ///
    /// When it yields a run, it has read from the backend exactly the
    /// records of the runs yielded so far, and one more if leaves remain:
    /// the record that did not fit, which closed the run. An encoder that
    /// held a complete run back would have read at least the next run's
    /// records too.
    #[test]
    fn each_run_is_yielded_once_complete(
        (count, budget) in run_case(),
    ) {
        let leaves = colliding_leaves(count);
        let decoded = decode_opening(unbatched_frames(&leaves));
        let backend = Failing::after(Local, usize::MAX);
        let reply = Reply {
            reactions: decoded
                .reactions
                .into_iter()
                .map(|reaction| match reaction {
                    Reaction::Supply(radix, node) => Reaction::Supply(radix, FailingNode::new(node)),
                    _ => panic!("a pure-supply reply decodes to supplies alone"),
                })
                .collect(),
            charge: Default::default(),
            last: true,
            carried: Default::default(),
        };
        runtime().block_on(async {
            let mut frames = encode_reply(
                backend.clone(),
                RunBudget::from_bytes(budget),
                Scope::opening_unbounded(&[]),
                reply,
            );
            let mut yielded = 0;
            while let Some(encoded) = frames.next().await {
                let (frame, _) = encoded.expect("the wrapped local backend never fails").into_parts();
                let Frame::Reaction(WireReaction::Supply(run), _) = frame else {
                    panic!("a pure-supply reply emitted {frame:?}");
                };
                yielded += run.record_count();
                let closing = usize::from(yielded < count);
                prop_assert_eq!(
                    backend.leaves_yielded(),
                    yielded + closing,
                    "records read when a run of {} was yielded",
                    run.record_count()
                );
            }
            prop_assert_eq!(yielded, count);
            Ok(())
        })?;
    }
}

/// The supply-only reply of `count` colliding leaves, over `backend`'s
/// nodes: one supply reaction per leaf's subtree.
fn supply_reply(count: usize) -> Reply<FailingNode<<Local as Backend>::Erased>> {
    let decoded = decode_opening(unbatched_frames(&colliding_leaves(count)));
    Reply {
        reactions: decoded
            .reactions
            .into_iter()
            .map(|reaction| match reaction {
                Reaction::Supply(radix, node) => Reaction::Supply(radix, FailingNode::new(node)),
                _ => panic!("a pure-supply reply decodes to supplies alone"),
            })
            .collect(),
        charge: Default::default(),
        last: true,
        carried: Default::default(),
    }
}

proptest! {
    /// With a send pool, each run the encoder yields holds a permit for
    /// exactly its buffer and its node in the lane, and the encoder holds no
    /// other permit.
    ///
    /// The permit is whole permits, or the whole pool if that is less; at
    /// each yield the pool holds back nothing but the run in hand.
    ///
    /// The pool holds one or two runs' worth at the run budget, and the
    /// budget ranges over every size the fixture's records can fill, zero
    /// included, where every record outgrows the pool's runs and takes the
    /// whole pool. The test drops each run as the multiplexer would once it
    /// is written, so the encoder never waits on a run the test holds.
    #[test]
    fn each_run_holds_a_permit_for_exactly_its_bytes(
        (count, budget) in run_case(),
        runs in 1usize..=2,
    ) {
        let pool = WirePool::sending(budget, runs);
        let node = queued::<Outbound>();
        let mut encoder = ReplyEncoder::branch(
            Failing::after(Local, usize::MAX),
            RunBudget::from_bytes(budget),
            Scope::opening_unbounded(&[]),
        )
        .pooled(pool.clone());
        let reply = supply_reply(count);
        runtime().block_on(async {
            let mut frames = pin!(encoder.chunk(reply));
            while let Some(encoded) = frames.next().await {
                let encoded = encoded.expect("the wrapped local backend never fails");
                let permitted = encoded.permitted();
                prop_assert_eq!(
                    pool.available() + permitted,
                    pool.bytes(),
                    "the encoder holds only the permit it yields"
                );
                let (frame, _) = encoded.into_parts();
                let Frame::Reaction(WireReaction::Supply(run), _) = frame else {
                    panic!("a pure-supply reply emitted {frame:?}");
                };
                let whole = (run.capacity() + node).div_ceil(PERMIT_BYTES) * PERMIT_BYTES;
                prop_assert_eq!(permitted, whole.min(pool.bytes()));
            }
            prop_assert_eq!(pool.available(), pool.bytes(), "every permit returned");
            Ok(())
        })?;
    }
}

/// A single record larger than the budget ships alone in its own run,
/// exceeding the budget: the minimum-one-record rule keeps every leaf
/// shippable under any setting, including a zero budget.
#[test]
fn an_oversized_record_ships_alone() {
    let leaves = colliding_leaves(3);
    let frames = recode(unbatched_frames(&leaves), RunBudget::from_bytes(0));

    let runs = runs_of(&frames);
    assert_eq!(runs.len(), leaves.len(), "each record rides its own run");
    for run in &runs {
        assert_eq!(run.record_count(), 1);
        assert!(
            run.encoded_len() > 0,
            "a lone record still exceeds budget 0"
        );
    }
    assert_eq!(records_of(&runs).len(), leaves.len());
}

/// Runs never span reactions: two supplied subtrees whose records would
/// comfortably share one run still ship as two runs, because batching scope
/// is one reaction's leaf enumeration.
#[test]
fn runs_never_span_reactions() {
    let leaves = separated_leaves();
    let frames = recode(
        unbatched_frames(&leaves),
        RunBudget::from_bytes(default::TARGET_MESSAGE_SIZE),
    );

    let runs = runs_of(&frames);
    assert_eq!(runs.len(), 2, "two reactions must emit two runs");
    for run in &runs {
        assert_eq!(run.record_count(), 1);
    }
}

/// The largest generated run fits one frame at its exact encoded size, and
/// decoding that frame reconstructs every leaf.
#[test]
fn a_batched_run_round_trips_the_reply() {
    let leaves = colliding_leaves(MAX_CASE_LEAVES);
    let records: Vec<_> = leaves
        .iter()
        .map(|leaf| (&leaf.version, &leaf.message))
        .collect();
    let expected = Frame::Reaction(WireReaction::Supply(leaf_run(&records)), Flow::End);
    let budget = match &expected {
        Frame::Reaction(WireReaction::Supply(run), _) => {
            RunBudget::from_bytes(SUPPLY_FRAME_OVERHEAD + run.encoded_len())
        }
        _ => unreachable!("the expected frame is a supply"),
    };
    let frames = recode(unbatched_frames(&leaves), budget);
    assert_eq!(frames, [expected], "the exact wire size admits one run");

    let runtime = runtime();
    let reply = runtime.block_on(async {
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
            Some(&crate::tree::typed::PathKey::fixture()),
        )
        .await
        .expect("the batched frame decodes")
    });
    let [Reaction::Supply(_, node)] = reply.reactions.as_slice() else {
        panic!("one batched run must decode to one supplied node");
    };
    let prefix = Prefix::<UnderRoot>::containing(&leaves[0].path());
    let rebuilt = runtime.block_on(async {
        <Local as Backend>::leaves(
            Local,
            prefix,
            <Local as Backend>::assume(&prefix, node.clone()),
        )
        .try_collect::<Vec<_>>()
        .await
        .expect("the local backend is infallible")
    });
    assert_eq!(rebuilt.len(), leaves.len());
}
