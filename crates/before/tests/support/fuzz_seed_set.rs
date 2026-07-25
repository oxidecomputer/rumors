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

use before::{Clock, Version};

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
    z.tick();
    z.tick();
    x.sync(&mut z).expect("forked clocks are disjoint");
    seeds.push(Seed {
        target: "fuzz_decode",
        name: "version_nested",
        bytes: x.version().encode(),
    });
    // `y` exists to nest `z`'s party a level deeper; its version stays
    // empty and needs no seed of its own.
    let _ = y.version();

    // Decode-then-ops scripts, in fuzz_decode_ops framing: flavour byte,
    // 1-byte length prefix, the value bytes, then the op script. The
    // framing and the op indices below are a wire contract with
    // `fuzz/fuzz_targets/fuzz_decode_ops.rs` (its `run` carves the value,
    // its `drive_clock` op table is the `% 7` dispatch the script bytes
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
    // compare — one full lap of the script's op table).
    let mut ops = vec![0u8, len];
    ops.extend_from_slice(&clock_bytes);
    ops.extend_from_slice(&[0, 1, 3, 5, 2, 4, 6]);
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

    seeds
}
