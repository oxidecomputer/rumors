//! Checks the supply decoder's bounded channel occupancy, and its reuse
//! across replies.
//!
//! The reader and assembler communicate through a [`FAN`]-slot channel. An
//! eager source must reach, but never exceed, `FAN + 1` resident records: one
//! full channel plus the record held by the reader. A paced source confirms
//! that the probe measures actual occupancy rather than returning a constant.
//! One edge carries every reply a decode pump decodes, so a sequence of
//! replies through one edge must decode each reply as a fresh edge would.

use crate::tree::mirror::streaming::message::{Listed, listed};
use futures::{Stream, StreamExt, stream};
use proptest::{collection::vec, prelude::*};

use before::Version;

use crate::{
    message::Message,
    testing::run_to_quiescence,
    tree::{
        mirror::streaming::{
            Backend, FAN, Local,
            erased::{Reaction, Reply},
            remote::codec::{Frame, LeafRun, Reaction as WireReaction},
        },
        typed::Path,
    },
};

use super::super::{
    LeafEdge, ReplyLevel, Scope,
    decode::{SUPPLY_RECORDS_PER_STREAM, fan_probe},
    decode_whole_reply,
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
    // A path is a keyed hash of its version, so compute each once: a key
    // recomputed per comparison hashes each leaf about 2·log₂(count) times.
    leaves.sort_by_cached_key(|(version, _)| Path::for_fixture_leaf(version));
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
        .expect("ascending in-scope leaves assemble");
    });
    fan_probe::peak()
}

/// Decode one pure-supply reply through the one-slot test channel.
fn one_slot_peak_occupancy(mut input: impl Stream<Item = Frame> + Unpin) -> usize {
    fan_probe::reset();
    run_to_quiescence(async {
        decode_whole_reply::<Local, _>(
            &mut LeafEdge::one_slot(),
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
        .expect("ascending in-scope leaves assemble");
    })
    .expect("the reader and assembler reach quiescence");
    fan_probe::peak()
}

/// A one-slot leaf channel completes an eager reply much larger than one fan.
///
/// One slot is the smallest capacity a protocol edge accepts: the streaming
/// channel rejects a zero capacity. Completing four fans of records refutes
/// the premise that this channel must buffer a whole fan for progress; the
/// joined assembler drains each blocked send.
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

/// The radix and node hash of each supply in a pure-supply reply, in order.
fn supplies(reply: &Reply<<Local as Backend>::Erased>) -> Vec<Listed> {
    reply
        .reactions
        .iter()
        .map(|reaction| match reaction {
            Reaction::Supply(radix, node) => listed(*radix, node.hash()),
            _ => panic!("a pure-supply reply decodes to supplies only"),
        })
        .collect()
}

/// Decode consecutive pure-supply replies of the given sizes from one frame
/// stream, each through `edge` if given, else through a fresh edge.
fn decode_sequence(counts: &[u64], mut edge: Option<LeafEdge<Local>>) -> Vec<Vec<Listed>> {
    let mut input = stream::iter(counts.iter().flat_map(|&count| frames(&leaves(count))));
    run_to_quiescence(async {
        let mut decoded = Vec::new();
        for _ in counts {
            let mut fresh = LeafEdge::new();
            let reply = decode_whole_reply::<Local, _>(
                edge.as_mut().unwrap_or(&mut fresh),
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
            .expect("ascending in-scope leaves assemble");
            decoded.push(supplies(&reply));
        }
        decoded
    })
    .expect("the reader and assembler reach quiescence")
}

proptest! {
    /// Consecutive replies decoded through one edge each decode exactly as
    /// through a fresh edge, at the production fan and at one slot.
    ///
    /// Reply sizes span empty replies, replies within one fan, and replies
    /// several fans long, so a reply's closing marker lands both in a
    /// partly filled edge and behind a full one; an assembler reading past
    /// the marker, or stopping short of it, shifts a supply between
    /// replies and fails the comparison.
    #[test]
    fn one_edge_decodes_each_reply_of_a_sequence_as_a_fresh_edge_does(
        counts in vec(0..=3 * FAN as u64, 1..4),
        one_slot in any::<bool>(),
    ) {
        let shared = if one_slot { LeafEdge::one_slot() } else { LeafEdge::new() };
        prop_assert_eq!(decode_sequence(&counts, Some(shared)), decode_sequence(&counts, None));
    }
}
