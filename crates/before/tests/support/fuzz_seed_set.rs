//! The fuzz seed corpus of record: one derivation, two consumers.
//!
//! `fuzz/seeds/<target>/<name>` holds canonical encodings that seed the
//! libFuzzer corpus. Hand-authored seed bytes rot silently when the wire
//! format moves — a stale seed fails no gate, it just stops representing
//! the values it was written for and quietly degrades the corpus. This
//! module is the cure: [`seed_set`] derives every seed from the live
//! public API, the `fuzz_seeds` example writes the files, and the
//! `fuzz_seeds` integration test re-derives and byte-compares the
//! committed directory (names exact, no strays), so a format change
//! turns corpus rot into a red gate with a one-command fix.
//!
//! Shared by `#[path]` inclusion from the example (the writer) and the
//! integration test (the checker), so the two cannot drift from each
//! other; both build against the public API only.

use before::{Clock, Count, Party, Ranked, Span, Version};

// The writer example uses only the encoding half; the test and detached fuzz
// workspace use the parsers and target list from the same file.
#[allow(dead_code)]
#[path = "fuzz_input.rs"]
pub mod input;

use input::{encode_laws, encode_operations, Operations, Target};

/// Returns `2^exponent` as an unbounded tick count.
fn power_of_two(exponent: u32) -> Count {
    let mut count = Count::from(1u8);
    for _ in 0..exponent {
        count = &count + &count;
    }
    count
}

/// A uniform version at `count`.
fn uniform(count: impl Into<Count>) -> Version {
    let party = Party::seed();
    let mut version = Version::new();
    party.ticks(&mut version, count);
    version
}

/// One committed seed file: its fuzz target, file name, and exact bytes.
pub struct Seed {
    /// The fuzz target directory under `fuzz/seeds/`.
    pub target: Target,
    /// The file name inside the target directory.
    pub name: &'static str,
    /// The file's exact bytes.
    pub bytes: Vec<u8>,
}

/// Every seed file of record, derived from the live API.
///
/// The decode seeds are canonical encodings and deliberate rejection
/// witnesses. The operation and law seeds use the framing shared with their
/// target bodies. Derivation is deterministic: it uses neither randomness nor
/// wall-clock time.
pub fn seed_set() -> Vec<Seed> {
    let mut seeds = Vec::new();

    // The seed clock, and a forked pair with one tick each: the smallest
    // canonical clock, and two siblings whose parties are proper halves.
    let mut a = Clock::seed();
    seeds.push(Seed {
        target: Target::Decode,
        name: "clock_seed",
        bytes: a.encode(),
    });
    let mut b = a.fork();
    a.tick();
    b.tick();
    seeds.push(Seed {
        target: Target::Decode,
        name: "clock_forked_a",
        bytes: a.encode(),
    });
    seeds.push(Seed {
        target: Target::Decode,
        name: "clock_forked_b",
        bytes: b.encode(),
    });

    // Parties: the whole interval, a proper half, and a quarter nested a
    // level deeper.
    let mut whole = Clock::seed();
    seeds.push(Seed {
        target: Target::Decode,
        name: "party_seed",
        bytes: whole.party().encode(),
    });
    let mut half = whole.fork();
    seeds.push(Seed {
        target: Target::Decode,
        name: "party_split",
        bytes: whole.party().encode(),
    });
    let quarter = half.fork();
    seeds.push(Seed {
        target: Target::Decode,
        name: "party_nested",
        bytes: quarter.party().encode(),
    });

    // Versions: the empty version, and a nested tree built from a forked
    // history (concurrent ticks joined through sync).
    seeds.push(Seed {
        target: Target::Decode,
        name: "version_seed",
        bytes: Version::new().encode(),
    });
    let mut x = Clock::seed();
    let mut y = x.fork();
    let mut z = y.fork();
    x.tick();
    // A strictly ordered pair from one history: `older` (one tick) sits
    // strictly below `newer` (the sync-joined nested tree). The rank,
    // ranked-key, span, and differential seeds below are all built from it.
    let older = x.version().clone();
    z.tick();
    z.tick();
    x.sync(&mut z).expect("forked clocks are disjoint");
    seeds.push(Seed {
        target: Target::Decode,
        name: "version_nested",
        bytes: x.version().encode(),
    });
    // The decode target's non-canonical frontier: one committed witness
    // per skyline-validator arm, driven through the version entry point (the
    // span differential's fused admission walk subsumes both cases
    // under its dominance refutation, so only a version-kind seed
    // reaches these arms). Neither stream is derivable from the API —
    // no encode produces them — so the bytes carry their derivations.
    //
    // A running height that dips negative mid-stream: root internal
    // `0`, left leaf `1` with height gamma(0) `1`, right leaf `1` with
    // delta zigzag(-1) `010`, then the padding marker — 0b0111_0101.
    seeds.push(Seed {
        target: Target::Decode,
        name: "version_negative_height",
        bytes: vec![0x75],
    });
    // A collapsible sibling pair (zero right delta): root internal `0`,
    // left leaf `1` with height gamma(5) `00110`, right leaf `1` with
    // delta zigzag(0) `1` — nine live bits, 0b0100_1101 then `1`, the
    // padding marker, and six zeros.
    seeds.push(Seed {
        target: Target::Decode,
        name: "version_zero_sibling",
        bytes: vec![0x4D, 0xC0],
    });
    // `y` exists to nest `z`'s party a level deeper; its version stays
    // empty and needs no seed of its own.
    let _ = y.version();
    let newer = x.version().clone();

    // Ranks, ranked keys, and spans: canonical encodings of the remaining
    // wire types, so the decode target's corpus reaches every supported type.
    seeds.push(Seed {
        target: Target::Decode,
        name: "rank_nested",
        bytes: newer.rank().encode(),
    });
    seeds.push(Seed {
        target: Target::Decode,
        name: "ranked_nested",
        bytes: Ranked::from(&newer).encode(),
    });
    let span_ordered = Span::new(&older, &newer)
        .expect("one history's versions are ordered")
        .encode();
    seeds.push(Seed {
        target: Target::Decode,
        name: "span_ordered",
        bytes: span_ordered.clone(),
    });

    // Differential-target seeds: the accept frontier plus one committed
    // witness per rejection class whose *precedence* the differential
    // oracle guards. Every fuzz run replays the seed corpus first, so a
    // incorrect error ordering (a pair verdict pronounced before
    // the padding check) crashes the very first smoke run.
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "span_ordered",
        bytes: span_ordered.clone(),
    });
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "ranked_nested",
        bytes: Ranked::from(&newer).encode(),
    });
    // The postcard frame of a span: the committed serde tests pin the
    // typed payload byte-identical to `encode()` inside the format's
    // plain byte-sequence framing, so the frame derives from the
    // encoding alone (no serde feature needed here).
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "postcard_span",
        bytes: postcard::to_allocvec(&span_ordered)
            .expect("postcard serialization to a Vec is infallible"),
    });
    // A strictly crossed pair: the join strictly below the meet.
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "span_crossed",
        bytes: [newer.encode(), older.encode()].concat(),
    });
    // The minimal coincident composite (two empty versions): the accept
    // path that dispatches the span storage dedup — the admission
    // walk's Equal verdict — plus both static stream buffers, through
    // every differential arm.
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "span_coincident",
        bytes: [Version::new().encode(), Version::new().encode()].concat(),
    });
    // A crossed pair whose join also carries a set padding bit: the
    // structural error (`TrailingBits`) must outrank the pair verdict.
    let mut padded_empty = Version::new().encode();
    padded_empty[0] |= 0x04;
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "span_crossed_padding",
        bytes: [older.encode(), padded_empty].concat(),
    });
    // A join whose running height dips negative: root internal `0`, left
    // leaf `1` with height gamma(0) `1`, right leaf `1` with delta
    // zigzag(-1) `010`, then the padding marker — 0b0111_0101. Not
    // derivable from the API (no encode produces it); it seeds the one
    // documented fused/composed divergence, the height-dip
    // subsumption.
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "span_negative_join",
        bytes: [Version::new().encode(), vec![0x75]].concat(),
    });
    // A complete span followed by a spurious byte: the borsh prefix read
    // accepts and leaves a remainder; the whole-slice decode must reject.
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "span_trailing",
        bytes: [span_ordered, vec![0x00]].concat(),
    });
    // A rank prefix the version does not measure: well-formed components
    // no encode ever pairs.
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "ranked_mismatched",
        bytes: [newer.rank().encode(), older.encode()].concat(),
    });
    // A canonical encoding cut exactly at a flush byte boundary: the live
    // bits of the uniform version at 7 (leaf flag `1`, gamma(7) `0001000`)
    // fill its first byte, so its whole `1000_0000` padding byte is the
    // second — and the first byte alone is a complete tree whose required
    // padding is missing entirely. It seeds the truncation agreement
    // between the reader and slice entry points (`UnexpectedEof` is exactly raw
    // `Truncated`).
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "version_flush_cut",
        bytes: uniform(7u8).encode()[..1].to_vec(),
    });
    // Count's transport-only representation is a canonical sequence of
    // least-significant-first u64 limbs. Wide and redundant-zero sequences
    // exercise the accept and canonicality arms in both formats.
    let wide_limbs = vec![0_u64, 0, 1];
    let redundant_zero = vec![0_u64];
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "count_borsh_wide",
        bytes: borsh::to_vec(&wide_limbs).expect("serializing a Vec cannot fail"),
    });
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "count_borsh_redundant_zero",
        bytes: borsh::to_vec(&redundant_zero).expect("serializing a Vec cannot fail"),
    });
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "count_postcard_wide",
        bytes: postcard::to_allocvec(&wide_limbs).expect("serializing a Vec cannot fail"),
    });
    seeds.push(Seed {
        target: Target::DecodeDifferential,
        name: "count_postcard_redundant_zero",
        bytes: postcard::to_allocvec(&redundant_zero).expect("serializing a Vec cannot fail"),
    });

    // Decode-then-operate inputs. The shared writer fixes the framing; these
    // bytes choose one complete lap of the script dispatch and one concurrent
    // message reception.
    let mut clock = Clock::seed();
    let mut sibling = clock.fork();
    clock.tick();
    sibling.tick();
    let clock_branch = clock.version().clone();
    let sibling_branch = sibling.version().clone();

    seeds.push(Seed {
        target: Target::DecodeOperations,
        name: "clock_then_ops",
        bytes: encode_operations(
            Operations::Script,
            &clock.encode(),
            &[0, 1, 3, 5, 2, 4, 6, 7],
        ),
    });
    seeds.push(Seed {
        target: Target::DecodeOperations,
        name: "clock_then_msg",
        bytes: encode_operations(
            Operations::Message,
            &clock.encode(),
            &sibling.version().encode(),
        ),
    });

    // Synchronize the pair for the clock-valued law input while retaining the
    // concurrent branch versions above for the version-valued laws.
    clock
        .sync(&mut sibling)
        .expect("forked clocks are disjoint");

    // A live family: two concurrent branch versions, the empty version, and
    // the two disjoint parties of a synchronized clock pair. A five-item
    // version list crosses the balanced fold's first regrouping boundary and
    // repeats the empty version. The four-item party and clock lists repeat
    // aliases, which exercises overlap rejection.
    seeds.push(Seed {
        target: Target::Laws,
        name: "laws_family",
        bytes: encode_laws(
            [
                &clock_branch.encode(),
                &sibling_branch.encode(),
                &Version::new().encode(),
            ],
            [clock.party().as_bytes(), sibling.party().as_bytes()],
            &clock.encode(),
            [&[0, 1, 2, 3, 0], &[0, 1, 2, 0], &[0, 1, 2, 0]],
        ),
    });

    // Wide-gamma bases: values past `u64::MAX` open their gamma codes with
    // a 64+-zero unary prefix — about `2^-64` per random byte stream — so
    // random fuzzing essentially never reaches the wide-value decode tier.
    // These seeds fix that thin tail. The parties nest a level deeper than
    // the family seed's, and the clock pairs a wide history with a quarter
    // share.
    let wide_leaf = uniform(power_of_two(128));
    let mut wide_owner = Party::seed();
    let mut wide_right = wide_owner.fork();
    let wide_tail = wide_right.fork();
    let mut wide_nested = Version::new();
    let wide_base = power_of_two(64);
    wide_owner.ticks(&mut wide_nested, &wide_base + Count::from(1u8));
    wide_right.ticks(&mut wide_nested, &wide_base + Count::from(2u8));
    wide_tail.ticks(&mut wide_nested, wide_base);
    let mut quarter_owner = Clock::seed();
    let mut half = quarter_owner.fork();
    let quarter = half.fork();
    // Lengths 15, 16, and 17 cross the fold's second power-of-two boundary.
    // Cycling the pools carries wide values through each regrouping case.
    let deep_versions: Vec<u8> = (0..17u8).map(|i| i % 4).collect();
    let deep_parties: Vec<u8> = (0..16u8).map(|i| i % 3).collect();
    let deep_clocks: Vec<u8> = (0..15u8).map(|i| i % 3).collect();
    seeds.push(Seed {
        target: Target::Laws,
        name: "laws_wide_gamma",
        bytes: encode_laws(
            [
                &wide_leaf.encode(),
                &wide_nested.encode(),
                &clock.version().encode(),
            ],
            [half.party().as_bytes(), quarter.party().as_bytes()],
            &Clock::from_parts(
                quarter_owner.party().dangerously_alias(),
                wide_nested.clone(),
            )
            .encode(),
            [&deep_versions, &deep_parties, &deep_clocks],
        ),
    });
    // `quarter_owner` exists to nest the parties; only its party is read.
    let _ = quarter_owner.version();

    // The wide tier for the rank-bearing decoders: rank streams
    // with mantissas past the machine word, a span whose endpoints carry
    // wide bases — shapes random bytes essentially never reach (the same
    // ~2^-64 unary-prefix argument as the laws seeds above).
    seeds.push(Seed {
        target: Target::Decode,
        name: "rank_wide",
        bytes: wide_nested.rank().encode(),
    });
    seeds.push(Seed {
        target: Target::Decode,
        name: "ranked_wide",
        bytes: Ranked::from(&wide_nested).encode(),
    });
    seeds.push(Seed {
        target: Target::Decode,
        name: "span_wide",
        bytes: Span::new(&wide_nested, &wide_leaf)
            .expect("the nested wide tree sits below the 2^128 leaf")
            .encode(),
    });
    seeds
}
