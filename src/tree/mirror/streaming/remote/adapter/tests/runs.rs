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

use futures::{TryStreamExt, stream};
use proptest::prelude::*;

use crate::{
    Version,
    message::Message,
    tree::{
        mirror::streaming::{
            Backend, Local,
            erased::Reaction,
            remote::codec::{
                DEFAULT_TARGET_MESSAGE_SIZE, Flow, Frame, LeafRun, Reaction as WireReaction,
                RunBudget, SUPPLY_FRAME_OVERHEAD,
            },
        },
        typed::{Prefix, height::UnderRoot},
    },
};

use super::{
    super::{Scope, decode_reply, encode_reply},
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

/// Decode `frames` as one reply to the opening scope, then re-encode it
/// under `budget`, returning the emitted wire frames.
fn recode(frames: Vec<Frame>, budget: RunBudget) -> Vec<Frame> {
    let runtime = runtime();
    runtime.block_on(async {
        let mut input = stream::iter(frames);
        let decoded = decode_reply::<Local, _>(
            Local,
            u64::MAX,
            unbounded(),
            Scope::opening(&[]),
            &mut input,
            codec(),
        )
        .await
        .expect("ascending in-scope leaves assemble");
        encode_reply(Local, budget, Scope::opening(&[]), decoded.reply)
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
        RunBudget::from_bytes(DEFAULT_TARGET_MESSAGE_SIZE),
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
        decode_reply::<Local, _>(
            Local,
            u64::MAX,
            unbounded(),
            Scope::opening(&[]),
            &mut input,
            codec(),
        )
        .await
        .expect("the batched frame decodes")
        .reply
    });
    let [Reaction::Supply(_, node)] = reply.replies.as_slice() else {
        panic!("one batched run must decode to one supplied node");
    };
    let prefix = Prefix::<UnderRoot>::containing(&leaves[0].path());
    let rebuilt = runtime.block_on(async {
        <Local as Backend>::leaves(
            Local,
            prefix,
            <Local as Backend>::assume::<UnderRoot>(node.clone()),
        )
        .try_collect::<Vec<_>>()
        .await
        .expect("the local backend is infallible")
    });
    assert_eq!(rebuilt.len(), leaves.len());
}
