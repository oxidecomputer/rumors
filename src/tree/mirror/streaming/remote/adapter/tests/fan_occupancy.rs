//! Checks the supply decoder's bounded channel occupancy.
//!
//! The reader and assembler communicate through a [`FAN`]-slot channel. An
//! eager source must reach, but never exceed, `FAN + 1` resident records: one
//! full channel plus the record held by the reader. A paced source confirms
//! that the probe measures actual occupancy rather than returning a constant.

use futures::{Stream, StreamExt, TryStreamExt, stream};

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
use super::{codec, reply_frames, unbounded, uniform_version};

/// Leaf records per supply frame.
const PER_FRAME: usize = 16;

/// `count` unique `u64` leaves, in ascending path order (the wire order
/// the decoder validates).
fn leaves(count: u64) -> Vec<(Version, Message)> {
    let mut leaves: Vec<(Version, Message)> = (0..count)
        .map(|index| {
            let version = uniform_version(index + 1);
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

/// An eager ordinary decode reaches exactly `FAN + 1` resident records.
///
/// All frames are immediately ready and the backend is instantaneous, so the
/// reader runs as far ahead as the channel permits. Equality makes the bound
/// both strict and non-vacuous.
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

/// Eager opening-supply decoding reaches the same `FAN + 1` ceiling.
///
/// This separately checks the jointly driven opening path.
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

/// A paced source stays below the ceiling, proving the probe is live.
///
/// A paced source (one frame per poll cycle, so the assembler keeps up)
/// holds peak occupancy at the reader's per-cycle intake, far under
/// `FAN + 1`: the sub-ceiling reading proves the eager pin's figure is
/// measured rather than an instrument artifact, and that occupancy
/// tracks reader-ahead — not channel capacity — when the wire is the
/// slower side.
#[test]
fn paced_decode_stays_under_the_ceiling() {
    let leaves = leaves(4 * FAN as u64);
    let paced = Box::pin(stream::iter(frames(&leaves)).then(|frame| async move {
        tokio::task::yield_now().await;
        frame
    }));
    let peak = peak_occupancy(paced);
    assert!(peak >= 1, "records flowed through the probe");
    assert!(
        peak <= 2 * PER_FRAME,
        "paced peak {peak} stays at per-cycle intake, far under the {} ceiling",
        FAN + 1,
    );
}
