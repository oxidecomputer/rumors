//! The fan channels' occupancy ceiling: the supply-decode charge premise.
//!
//! Both reply-decode shapes buffer decoded leaf records in a
//! [`FAN`]-slot channel between the reader and the assembler —
//! `decode`'s joined reader/assembler pair and `early_supplies`'
//! jointly driven pair — and the session budget charges that residency
//! flat: `SUPPLY_DECODE_ENVELOPE_BYTES` prices exactly `FAN + 1`
//! backend-priced records per reply stream, one full channel plus the
//! record in the reader's hand. The pins here hold that premise against
//! the code through the test-gated `fan_probe` in `decode.rs` (both
//! paths hook the same counter), with deterministic counts and no
//! timing anywhere: an eager frame source *reaches* the `FAN + 1`
//! ceiling on each path (the priced regime is real, so the pins cannot
//! pass vacuously) and never exceeds it, and a paced source is the
//! negative control proving the probe reports the regime rather than a
//! constant.

use futures::{Stream, StreamExt, TryStreamExt, stream};

use before::Version;

use crate::{
    message::Message,
    tree::{
        mirror::streaming::{
            Local,
            remote::codec::{Flow, Frame, LeafRun, Reaction as WireReaction},
            window::FAN,
        },
        typed::{
            Path, Prefix,
            height::{UnderRoot, UnderUnderRoot},
        },
    },
};

use super::super::{Scope, decode::fan_probe, decode_reply, early_supplies};

/// Leaf records per supply frame.
const PER_FRAME: usize = 16;

/// `count` unique `u64` leaves, in ascending path order (the wire order
/// the decoder validates).
fn leaves(count: u64) -> Vec<(Version, Message<u64>)> {
    let mut leaves: Vec<(Version, Message<u64>)> = (0..count)
        .map(|index| {
            let version =
                Version::try_from(index % 200 + 1).expect("small linear versions are valid");
            (version, Message::new(index))
        })
        .collect();
    leaves.sort_by_key(|(version, message)| Path::for_leaf(version, message.as_slice()));
    leaves
}

/// Chunk leaves into supply frames of [`PER_FRAME`] records each.
fn frames(leaves: &[(Version, Message<u64>)]) -> Vec<Frame<u64>> {
    let chunks: Vec<&[(Version, Message<u64>)]> = leaves.chunks(PER_FRAME).collect();
    let count = chunks.len();
    chunks
        .into_iter()
        .enumerate()
        .map(|(position, chunk)| {
            let mut run = LeafRun::new();
            for (version, message) in chunk {
                run.push(version, message)
                    .expect("a test record fits the run framing");
            }
            let flow = if position + 1 == count {
                Flow::End
            } else {
                Flow::Continue
            };
            Frame::Reaction(WireReaction::Supply(run), flow)
        })
        .collect()
}

/// Decode one pure-supply reply from `input` over the instant in-memory
/// backend, reporting the probe's peak resident record count.
fn peak_occupancy(mut input: impl Stream<Item = Frame<u64>> + Unpin) -> usize {
    let runtime = super::runtime();
    fan_probe::reset();
    runtime.block_on(async {
        decode_reply::<Local, u64, UnderUnderRoot, _>(
            Local,
            u64::MAX,
            Scope::<UnderRoot>::opening(&[]),
            &mut input,
        )
        .await
        .expect("ascending in-scope leaves assemble");
    });
    fan_probe::peak()
}

/// The occupancy ceiling the flat charge rests on: an eager decode
/// reaches exactly `FAN + 1` resident records and never exceeds it.
///
/// Under an eager frame source (every frame ready — the wire outpaces
/// assembly) and the instant `Local` backend, the reader/assembler
/// channel reaches exactly `FAN + 1` resident decoded records — one
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
        peak,
        FAN + 1,
        "peak resident decoded records must equal the charged ceiling: one full \
         fan channel plus the record in the reader's hand, the per-stream shape \
         SUPPLY_DECODE_ENVELOPE_BYTES prices",
    );
}

/// The twin channel rides the same ceiling: `early_supplies`' jointly
/// driven reader/assembler pair reaches exactly `FAN + 1` resident
/// records under an eager source and never exceeds it.
///
/// The opening-supply path is one of the reply streams the flat charge
/// prices, so its channel must hold the same `FAN + 1` premise as
/// `decode`'s; the probe is the same counter, and the paced negative
/// control below covers its liveness.
#[test]
fn eager_early_supplies_ride_the_same_ceiling() {
    let leaves = leaves(4 * FAN as u64);
    let runtime = super::runtime();
    fan_probe::reset();
    runtime.block_on(async {
        let assembled: Vec<_> = early_supplies::<Local, u64, UnderRoot, _>(
            Local,
            u64::MAX,
            Prefix::new(),
            stream::iter(frames(&leaves)),
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
        FAN + 1,
        "peak resident decoded records on the early-supply path must equal the \
         charged ceiling, the same per-stream shape SUPPLY_DECODE_ENVELOPE_BYTES \
         prices for every reply stream",
    );
}

/// Negative control: the probe is live, not pinned to the ceiling.
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
