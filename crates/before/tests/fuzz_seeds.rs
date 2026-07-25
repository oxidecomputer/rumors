//! Canonicity gate for the committed fuzz seed corpus.
//!
//! The corpus rots silently when the wire format moves: a stale seed
//! decodes as nothing (or as the wrong value) and quietly stops seeding
//! the fuzzer with what it was written to represent. These tests hold
//! `fuzz/seeds/` byte-identical to the live derivation
//! (`tests/support/fuzz_seed_set.rs`) and hold every `fuzz_decode` seed
//! to the decode contract it seeds, so format drift is a red gate with a
//! one-command fix (`cargo run -p before --example fuzz_seeds`).

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use before::{Clock, Party, Version};

#[path = "support/fuzz_seed_set.rs"]
mod fuzz_seed_set;

/// The committed seed root.
fn seeds_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds")
}

/// Every committed seed file is byte-identical to the live derivation:
/// the corpus cannot drift from the wire format without reddening the
/// gate, and the fix is one writer run away.
#[test]
fn committed_seeds_match_the_live_derivation() {
    for seed in fuzz_seed_set::seed_set() {
        let path = seeds_root().join(seed.target).join(seed.name);
        let committed = fs::read(&path)
            .unwrap_or_else(|err| panic!("reading {} failed: {err}", path.display()));
        assert_eq!(
            committed, seed.bytes,
            "{}/{} differs from the live derivation: regenerate with \
             `cargo run -p before --example fuzz_seeds` and commit",
            seed.target, seed.name,
        );
    }
}

/// The seed directories hold exactly the set of record — no strays: a
/// leftover file from a renamed or retired seed would seed the fuzzer
/// with bytes nothing re-derives, which is the rot this suite exists to
/// prevent.
#[test]
fn seed_directories_hold_exactly_the_set_of_record() {
    let mut expected: std::collections::BTreeMap<String, BTreeSet<String>> = Default::default();
    for seed in fuzz_seed_set::seed_set() {
        expected
            .entry(seed.target.to_string())
            .or_default()
            .insert(seed.name.to_string());
    }
    for (target, names) in &expected {
        let dir = seeds_root().join(target);
        let listed: BTreeSet<String> = fs::read_dir(&dir)
            .unwrap_or_else(|err| panic!("listing {} failed: {err}", dir.display()))
            .map(|entry| {
                entry
                    .expect("reading a seed directory entry")
                    .file_name()
                    .into_string()
                    .expect("seed file names are UTF-8")
            })
            .collect();
        assert_eq!(
            &listed, names,
            "fuzz/seeds/{target} holds files outside the set of record (or is missing some)",
        );
    }
}

/// Every `fuzz_decode` seed decodes as the type its name declares and
/// re-encodes byte-identically: the seeds actually exercise the decode
/// paths they were written for.
#[test]
fn decode_seeds_decode_as_named_and_round_trip() {
    for seed in fuzz_seed_set::seed_set() {
        if seed.target != "fuzz_decode" {
            continue;
        }
        let bytes = &seed.bytes;
        let (kind, _) = seed.name.split_once('_').expect("seed names are kind_case");
        match kind {
            "clock" => {
                let clock = Clock::decode(&bytes[..]).expect("a clock seed decodes as a Clock");
                assert_eq!(&clock.encode(), bytes, "clock seed re-encode is not stable");
            }
            "party" => {
                let party = Party::decode(&bytes[..]).expect("a party seed decodes as a Party");
                assert_eq!(&party.encode(), bytes, "party seed re-encode is not stable");
            }
            "version" => {
                let version =
                    Version::decode(&bytes[..]).expect("a version seed decodes as a Version");
                assert_eq!(
                    &version.encode(),
                    bytes,
                    "version seed re-encode is not stable"
                );
            }
            other => panic!("unknown seed kind {other}"),
        }
    }
}
