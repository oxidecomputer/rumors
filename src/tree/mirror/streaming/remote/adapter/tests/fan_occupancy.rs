//! The fan channels' occupancy ceiling: the supply-decode charge premise.
//!
//! Both reply-decode shapes buffer decoded leaf records in a
//! [`FAN`]-slot channel between the reader and the assembler —
//! `decode`'s joined reader/assembler pair and `early_supplies`'
//! jointly driven pair — and the session budget charges that residency
//! flat: `SUPPLY_DECODE_ENVELOPE_BYTES` prices exactly
//! `SUPPLY_RECORDS_PER_STREAM`
//! backend-priced records per reply stream, one full channel plus the
//! record in the reader's hand. The pins here hold that premise against
//! the code through the test-gated `fan_probe` in `decode.rs` (both
//! paths hook the same counter). An eager frame source reaches the
//! `SUPPLY_RECORDS_PER_STREAM` ceiling on each path, demonstrating that the
//! priced regime is real while pinning the maximum residency to the charge.

use futures::{Stream, TryStreamExt, stream};

use before::Version;

use crate::{
    message::Message,
    testing::run_to_quiescence,
    tree::{
        mirror::streaming::{
            Local,
            remote::codec::{Frame, LeafRun, Reaction as WireReaction},
            window::{FAN, SUPPLY_RECORDS_PER_STREAM},
        },
        typed::{Path, Prefix},
    },
};

use super::super::{
    Scope,
    decode::{decode_reply_one_slot, fan_probe},
    decode_reply, early_supplies,
};
use super::{codec, reply_frames, unbounded};

/// Leaf records per supply frame.
const PER_FRAME: usize = 16;

/// `count` unique `u64` leaves, in ascending path order (the wire order
/// the decoder validates).
fn leaves(count: u64) -> Vec<(Version, Message)> {
    let mut leaves: Vec<(Version, Message)> = (0..count)
        .map(|index| {
            let version = Version::try_from(index + 1).expect("small linear versions are valid");
            (version, Message::new(index))
        })
        .collect();
    leaves.sort_by_key(|(version, _)| Path::for_leaf(version));
    leaves
}

/// Chunk leaves into supply frames of [`PER_FRAME`] records each.
fn frames(leaves: &[(Version, Message)]) -> Vec<Frame> {
    reply_frames(leaves.chunks(PER_FRAME).map(|chunk| {
        let mut run = LeafRun::new();
        for (version, message) in chunk {
            run.push(version, message)
                .expect("a test record fits the run framing");
        }
        WireReaction::Supply(run)
    }))
}

/// Decode one pure-supply reply from `input` over the instant in-memory
/// backend, reporting the probe's peak resident record count.
fn peak_occupancy(mut input: impl Stream<Item = Frame> + Unpin) -> usize {
    let runtime = super::runtime();
    fan_probe::reset();
    runtime.block_on(async {
        decode_reply::<Local, _>(
            Local,
            u64::MAX,
            unbounded(),
            Scope::opening(&[]),
            &mut input,
            codec(),
        )
        .await
        .expect("ascending in-scope leaves assemble");
    });
    fan_probe::peak()
}

/// Decode one pure-supply reply through the one-slot test channel.
fn one_slot_peak_occupancy(mut input: impl Stream<Item = Frame> + Unpin) -> usize {
    fan_probe::reset();
    run_to_quiescence(async {
        decode_reply_one_slot::<Local, _>(
            Local,
            u64::MAX,
            unbounded(),
            Scope::opening(&[]),
            &mut input,
            codec(),
        )
        .await
        .expect("ascending in-scope leaves assemble");
    })
    .expect("the reader and assembler reach quiescence");
    fan_probe::peak()
}

/// A one-slot leaf channel completes an eager reply much larger than one fan.
///
/// This is the smallest capacity Tokio permits. Completing four fans of
/// records refutes the premise that this channel must buffer a whole fan for
/// progress; the joined assembler drains each blocked send.
#[test]
fn one_slot_decode_channel_makes_progress() {
    let leaves = leaves(4 * FAN as u64);
    let peak = one_slot_peak_occupancy(stream::iter(frames(&leaves)));
    assert_eq!(
        peak, 2,
        "one queued record plus the reader's current record is the exact ceiling",
    );
}

/// The occupancy ceiling the flat charge rests on: an eager decode
/// reaches exactly [`SUPPLY_RECORDS_PER_STREAM`] resident records and never
/// exceeds it.
///
/// Under an eager frame source (every frame ready — the wire outpaces
/// assembly) and the instant `Local` backend, the reader/assembler
/// channel reaches exactly [`SUPPLY_RECORDS_PER_STREAM`] resident decoded records — one
/// full channel plus the record in the reader's hand. Reaching the
/// ceiling keeps the pin non-vacuous: the regime
/// `SUPPLY_DECODE_ENVELOPE_BYTES` prices is real. Not exceeding it is
/// the charge premise itself, so a widened channel or a new buffer
/// stage on this path fails here instead of silently underpricing every
/// session budget.
#[test]
fn eager_decode_occupancy_pins_the_charged_ceiling() {
    let leaves = leaves(4 * FAN as u64);
    let peak = peak_occupancy(stream::iter(frames(&leaves)));
    assert_eq!(
        peak, SUPPLY_RECORDS_PER_STREAM,
        "peak resident decoded records must equal the charged ceiling: one full \
         fan channel plus the record in the reader's hand, the per-stream shape \
         SUPPLY_DECODE_ENVELOPE_BYTES prices",
    );
}

/// The twin channel rides the same ceiling: `early_supplies`' jointly
/// driven reader/assembler pair reaches exactly [`SUPPLY_RECORDS_PER_STREAM`] resident
/// records under an eager source and never exceeds it.
///
/// The opening-supply path is one of the reply streams the flat charge
/// prices, so its channel must hold the same occupancy premise as
/// `decode`'s; both paths use the same probe.
#[test]
fn eager_early_supplies_ride_the_same_ceiling() {
    let leaves = leaves(4 * FAN as u64);
    let runtime = super::runtime();
    fan_probe::reset();
    runtime.block_on(async {
        let assembled: Vec<_> = early_supplies::<Local, _>(
            Local,
            u64::MAX,
            unbounded(),
            Prefix::new().erase(),
            stream::iter(frames(&leaves)),
            codec(),
        )
        .try_collect()
        .await
        .expect("ascending in-scope leaves assemble");
        assert!(
            !assembled.is_empty(),
            "the eager reply supplies real groups"
        );
    });
    assert_eq!(
        fan_probe::peak(),
        SUPPLY_RECORDS_PER_STREAM,
        "peak resident decoded records on the early-supply path must equal the \
         charged ceiling, the same per-stream shape SUPPLY_DECODE_ENVELOPE_BYTES \
         prices for every reply stream",
    );
}
