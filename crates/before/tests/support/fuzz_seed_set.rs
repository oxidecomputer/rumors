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

use before::{causally::Span, Clock, Party, Ranked, Version};

/// One committed seed file: its fuzz target, file name, and exact bytes.
pub struct Seed {
    /// The fuzz target directory under `fuzz/seeds/`.
    pub target: &'static str,
    /// The file name inside the target directory.
    pub name: &'static str,
    /// The file's exact bytes.
    pub bytes: Vec<u8>,
}

/// Every seed file of record, derived from the live API.
///
/// The `fuzz_decode` seeds are canonical encodings of a small family of
/// known values (the seed clock, a forked pair, split parties, a nested
/// version); the `fuzz_decode_ops` seeds are decode-then-operate scripts
/// in that target's framing (flavour byte, length-prefixed value bytes,
/// one op per trailing byte). Deterministic: no randomness, no clocks.
pub fn seed_set() -> Vec<Seed> {
    let mut seeds = Vec::new();

    // The seed clock, and a forked pair with one tick each: the smallest
    // canonical clock, and two siblings whose parties are proper halves.
    let mut a = Clock::seed();
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "clock_seed",
        bytes: a.encode(),
    });
    let mut b = a.fork();
    a.tick();
    b.tick();
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "clock_forked_a",
        bytes: a.encode(),
    });
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "clock_forked_b",
        bytes: b.encode(),
    });

    // Parties: the whole interval, a proper half, and a quarter nested a
    // level deeper.
    let mut whole = Clock::seed();
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "party_seed",
        bytes: whole.party().encode(),
    });
    let mut half = whole.fork();
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "party_split",
        bytes: whole.party().encode(),
    });
    let quarter = half.fork();
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "party_nested",
        bytes: quarter.party().encode(),
    });

    // Versions: the empty version, and a nested tree built from a forked
    // history (concurrent ticks joined through sync).
    seeds.push(Seed {
        target: "fuzz_decode",
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
        target: "fuzz_decode",
        name: "version_nested",
        bytes: x.version().encode(),
    });
    // The decode target's non-canonical frontier: one committed witness
    // per skyline-validator arm, driven through the version door (the
    // span differential's fused admission walk subsumes both genres
    // under its dominance refutation, so only a version-kind seed
    // reaches these arms). Neither stream is derivable from the API —
    // no encode produces them — so the bytes carry their derivations.
    //
    // A running height that dips negative mid-stream: root internal
    // `0`, left leaf `1` with height gamma(0) `1`, right leaf `1` with
    // delta zigzag(-1) `010`, one zero padding bit — 0b0111_0100.
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "version_negative_height",
        bytes: vec![0x74],
    });
    // A collapsible sibling pair (zero right delta): root internal `0`,
    // left leaf `1` with height gamma(5) `00110`, right leaf `1` with
    // delta zigzag(0) `1` — nine live bits, 0b0100_1101 then `1` and
    // seven zero padding bits.
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "version_zero_sibling",
        bytes: vec![0x4D, 0x80],
    });
    // `y` exists to nest `z`'s party a level deeper; its version stays
    // empty and needs no seed of its own.
    let _ = y.version();
    let newer = x.version().clone();

    // Ranks, ranked keys, and spans: canonical encodings of the remaining
    // wire types, so the decode target's corpus reaches every roster row.
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "rank_nested",
        bytes: newer.rank().encode(),
    });
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "ranked_nested",
        bytes: Ranked::from(&newer).encode(),
    });
    let span_ordered = Span::new(&older, &newer)
        .expect("one history's versions are ordered")
        .encode();
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "span_ordered",
        bytes: span_ordered.clone(),
    });

    // Differential-target seeds: the accept frontier plus one committed
    // witness per rejection genre whose *precedence* the differential
    // oracle guards. Every fuzz run replays the seed corpus first, so a
    // reintroduced genre-ordering defect (a pair verdict pronounced before
    // the padding check) crashes the very first smoke run.
    seeds.push(Seed {
        target: "fuzz_decode_differential",
        name: "span_ordered",
        bytes: span_ordered.clone(),
    });
    seeds.push(Seed {
        target: "fuzz_decode_differential",
        name: "ranked_nested",
        bytes: Ranked::from(&newer).encode(),
    });
    // The postcard frame of a span: the committed serde tests pin the
    // typed payload byte-identical to `encode()` inside the format's
    // plain byte-sequence framing, so the frame derives from the
    // encoding alone (no serde feature needed here).
    seeds.push(Seed {
        target: "fuzz_decode_differential",
        name: "postcard_span",
        bytes: postcard::to_allocvec(&span_ordered)
            .expect("postcard serialization to a Vec is infallible"),
    });
    // A strictly crossed pair: the join strictly below the meet.
    seeds.push(Seed {
        target: "fuzz_decode_differential",
        name: "span_crossed",
        bytes: [newer.encode(), older.encode()].concat(),
    });
    // The minimal coincident composite (two empty versions): the accept
    // path that dispatches the span storage dedup — the admission
    // walk's Equal verdict — plus both static stream buffers, through
    // every differential arm.
    seeds.push(Seed {
        target: "fuzz_decode_differential",
        name: "span_coincident",
        bytes: [Version::new().encode(), Version::new().encode()].concat(),
    });
    // A crossed pair whose join also carries a set padding bit: the
    // structural genre (TrailingBits) must outrank the pair verdict.
    let mut padded_empty = Version::new().encode();
    padded_empty[0] |= 0x04;
    seeds.push(Seed {
        target: "fuzz_decode_differential",
        name: "span_crossed_padding",
        bytes: [older.encode(), padded_empty].concat(),
    });
    // A join whose running height dips negative: root internal `0`, left
    // leaf `1` with height gamma(0) `1`, right leaf `1` with delta
    // zigzag(-1) `010`, one zero padding bit — 0b0111_0100. Not derivable
    // from the API (no encode produces it); it seeds the one documented
    // fused/composed genre divergence, the height-dip subsumption.
    seeds.push(Seed {
        target: "fuzz_decode_differential",
        name: "span_negative_join",
        bytes: [Version::new().encode(), vec![0x74]].concat(),
    });
    // A complete span followed by a spurious byte: the borsh prefix read
    // accepts and leaves a remainder; the whole-slice decode must reject.
    seeds.push(Seed {
        target: "fuzz_decode_differential",
        name: "span_trailing",
        bytes: [span_ordered, vec![0x00]].concat(),
    });
    // A rank prefix the version does not measure: well-formed components
    // no encode ever pairs.
    seeds.push(Seed {
        target: "fuzz_decode_differential",
        name: "ranked_mismatched",
        bytes: [newer.rank().encode(), older.encode()].concat(),
    });

    // Text-parse seeds: the display notation of known values, including
    // the nested tuple form and wide decimal magnitudes.
    seeds.push(Seed {
        target: "fuzz_parse",
        name: "clock_display",
        bytes: x.to_string().into_bytes(),
    });
    seeds.push(Seed {
        target: "fuzz_parse",
        name: "version_nested_text",
        bytes: newer.to_string().into_bytes(),
    });
    seeds.push(Seed {
        target: "fuzz_parse",
        name: "party_nested_text",
        bytes: quarter.party().to_string().into_bytes(),
    });

    // Decode-then-ops scripts, in fuzz_decode_ops framing: flavour byte,
    // 1-byte length prefix, the value bytes, then the op script. The
    // framing and the op indices below are a wire contract with
    // `fuzz/fuzz_targets/fuzz_decode_ops.rs` (its `run` carves the value,
    // its `drive_clock` op table is the `% 8` dispatch the script bytes
    // select from); a change on either side means regenerating the seeds.
    let mut clock = Clock::seed();
    let mut sibling = clock.fork();
    clock.tick();
    sibling.tick();
    clock
        .sync(&mut sibling)
        .expect("forked clocks are disjoint");
    let clock_bytes = clock.encode();
    let len = u8::try_from(clock_bytes.len()).expect("seed clocks encode within one length byte");

    // Flavour 0: drive the clock op set (tick, fork, join, sync, send/recv,
    // compare, fused multi-tick — one full lap of the script's op table).
    let mut ops = vec![0u8, len];
    ops.extend_from_slice(&clock_bytes);
    ops.extend_from_slice(&[0, 1, 3, 5, 2, 4, 6, 7]);
    seeds.push(Seed {
        target: "fuzz_decode_ops",
        name: "clock_then_ops",
        bytes: ops,
    });

    // Flavour 1: compare against, then receive, a canonical message (the
    // sibling's version, concurrent to the clock's own history).
    let mut msg = vec![1u8, len];
    msg.extend_from_slice(&clock_bytes);
    msg.extend_from_slice(&sibling.version().encode());
    seeds.push(Seed {
        target: "fuzz_decode_ops",
        name: "clock_then_msg",
        bytes: msg,
    });

    // Law-target inputs, in fuzz_laws framing: six length-prefixed chunks —
    // `[len: u8][bytes]` each — decoded positionally as three Versions, two
    // Parties, and a Clock, then three list scripts — `[arity: u8][pool
    // indices: one byte per element]`, versions (pool of 4) then parties
    // (pool of 3) then clocks (pool of 3) — feeding the variadic law
    // groups. The framing is a wire contract with
    // `fuzz/fuzz_targets/fuzz_laws.rs` (its `chunk` carves the values in
    // this order, its `picks` reads the scripts); a change on either side
    // means regenerating the seeds.
    let laws_chunks =
        |versions: [&Version; 3], parties: [&Party; 2], clock: &Clock, scripts: [&[u8]; 3]| {
            let mut bytes = Vec::new();
            let mut push = |encoded: Vec<u8>| {
                let len =
                    u8::try_from(encoded.len()).expect("seed values encode within one length byte");
                bytes.push(len);
                bytes.extend_from_slice(&encoded);
            };
            for version in versions {
                push(version.encode());
            }
            for party in parties {
                push(party.encode());
            }
            push(clock.encode());
            for script in scripts {
                let arity = u8::try_from(script.len())
                    .expect("seed list scripts stay within one arity byte");
                bytes.push(arity);
                bytes.extend_from_slice(script);
            }
            bytes
        };

    // A live family: the synced clock's nested version, the sibling's
    // concurrent version, the empty version, and the two disjoint sibling
    // parties around the clock itself. The scripts cross the balanced
    // counter's first-octave boundaries: an arity-5 version list (the
    // merged–merged carry at four, then the drain) with a repeat and the
    // empty version, and arity-4 party and clock lists whose repeats are
    // aliases (the refusal arm).
    seeds.push(Seed {
        target: "fuzz_laws",
        name: "laws_family",
        bytes: laws_chunks(
            [clock.version(), sibling.version(), &Version::new()],
            [clock.party(), sibling.party()],
            &clock,
            [&[0, 1, 2, 3, 0], &[0, 1, 2, 0], &[0, 1, 2, 0]],
        ),
    });

    // Wide-gamma bases: values past `u64::MAX` open their gamma codes with
    // a 64+-zero unary prefix — about `2^-64` per random byte stream — so
    // random fuzzing essentially never reaches the wide-value decode tier.
    // These seeds fix that thin tail. The parties nest a level deeper than
    // the family seed's, and the clock pairs a wide history with a quarter
    // share.
    let wide_leaf: Version = "340282366920938463463374607431768211456" // 2^128
        .parse()
        .expect("a bare wide integer parses as a version leaf");
    let wide_nested: Version = "(18446744073709551616, 1, (0, 2, 0))" // 2^64 base at the root
        .parse()
        .expect("a nested wide event tree parses");
    let mut quarter_owner = Clock::seed();
    let mut half = quarter_owner.fork();
    let quarter = half.fork();
    // The second-octave scripts: arities 17/16/15 (a lone input under a
    // weight-4 carry; the full carry; a four-group drain), cycling their
    // pools so wide-gamma values ride through every combine weight.
    let deep_versions: Vec<u8> = (0..17u8).map(|i| i % 4).collect();
    let deep_parties: Vec<u8> = (0..16u8).map(|i| i % 3).collect();
    let deep_clocks: Vec<u8> = (0..15u8).map(|i| i % 3).collect();
    seeds.push(Seed {
        target: "fuzz_laws",
        name: "laws_wide_gamma",
        bytes: laws_chunks(
            [&wide_leaf, &wide_nested, clock.version()],
            [half.party(), quarter.party()],
            &Clock::from_parts(
                quarter_owner.party().dangerously_alias(),
                wide_nested.clone(),
            ),
            [&deep_versions, &deep_parties, &deep_clocks],
        ),
    });
    // `quarter_owner` exists to nest the parties; only its party is read.
    let _ = quarter_owner.version();

    // The wide tier for the rank-bearing and text decoders: rank streams
    // with mantissas past the machine word, a span whose endpoints carry
    // wide bases, and the wide decimal display (which parses as both a
    // version leaf and a tick count) — shapes random bytes essentially
    // never reach (the same ~2^-64 unary-prefix argument as the laws
    // seeds above).
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "rank_wide",
        bytes: wide_nested.rank().encode(),
    });
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "ranked_wide",
        bytes: Ranked::from(&wide_nested).encode(),
    });
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "span_wide",
        bytes: Span::new(&wide_nested, &wide_leaf)
            .expect("the nested wide tree sits below the 2^128 leaf")
            .encode(),
    });
    seeds.push(Seed {
        target: "fuzz_parse",
        name: "version_wide",
        bytes: wide_leaf.to_string().into_bytes(),
    });

    seeds
}
