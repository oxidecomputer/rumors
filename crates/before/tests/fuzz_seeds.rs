//! Canonicity gate for the committed fuzz seed corpus.
//!
//! The corpus rots silently when the wire format moves: a stale seed
//! decodes as nothing (or as the wrong value) and quietly stops seeding
//! the fuzzer with what it was written to represent. These tests hold
//! `fuzz/seeds/` byte-identical to the live derivation
//! (`tests/support/fuzz_seed_set.rs`) and hold every seed to the
//! contract it seeds — the decode targets' round-trips, the
//! differential target's per-genre rejection witnesses, the parse
//! target's display round-trips — so format drift is a red gate with a
//! one-command fix (`cargo run -p before --example fuzz_seeds`).

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use before::{causally::Span, error::Decode, Clock, Party, Rank, Ranked, Version};

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

/// The seed directories hold exactly the set of record — no strays.
///
/// A leftover file from a renamed or retired seed (or a whole directory
/// for a renamed or retired target) would seed the fuzzer with bytes
/// nothing re-derives, which is the rot this suite exists to prevent.
#[test]
fn seed_directories_hold_exactly_the_set_of_record() {
    let mut expected: std::collections::BTreeMap<String, BTreeSet<String>> = Default::default();
    for seed in fuzz_seed_set::seed_set() {
        expected
            .entry(seed.target.to_string())
            .or_default()
            .insert(seed.name.to_string());
    }
    // The root holds exactly one directory per target of record.
    let listed_targets: BTreeSet<String> = fs::read_dir(seeds_root())
        .unwrap_or_else(|err| panic!("listing {} failed: {err}", seeds_root().display()))
        .map(|entry| {
            entry
                .expect("reading the seed root's entry")
                .file_name()
                .into_string()
                .expect("seed directory names are UTF-8")
        })
        .collect();
    let expected_targets: BTreeSet<String> = expected.keys().cloned().collect();
    assert_eq!(
        listed_targets, expected_targets,
        "fuzz/seeds holds directories outside the targets of record (or is missing some)"
    );
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
/// re-encodes byte-identically — or, for the corpus's non-canonical
/// frontier, rejects with exactly its named genre.
///
/// Either way the seeds actually exercise the decode paths they were
/// written for.
#[test]
fn decode_seeds_decode_as_named_and_round_trip() {
    for seed in fuzz_seed_set::seed_set() {
        if seed.target != "fuzz_decode" {
            continue;
        }
        let bytes = &seed.bytes;
        // The rejection witnesses, by full name: each must reject with
        // the exact error its genre pronounces (never fail an earlier
        // parse), so the corpus keeps seeding the validator arm it was
        // written for — the fused span walk cannot reach these arms, so
        // these version-door seeds are their only corpus coverage.
        match seed.name {
            "version_negative_height" => {
                assert!(
                    matches!(Version::decode(&bytes[..]), Err(Decode::NotCanonical)),
                    "a mid-stream negative running height is non-canonical"
                );
                continue;
            }
            "version_zero_sibling" => {
                assert!(
                    matches!(Version::decode(&bytes[..]), Err(Decode::NotCanonical)),
                    "a collapsible sibling pair (zero right delta) is non-canonical"
                );
                continue;
            }
            _ => {}
        }
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
            "rank" => {
                let rank = Rank::decode(&bytes[..]).expect("a rank seed decodes as a Rank");
                assert_eq!(&rank.encode(), bytes, "rank seed re-encode is not stable");
            }
            "ranked" => {
                let key = Ranked::decode(&bytes[..]).expect("a ranked seed decodes as a Ranked");
                assert_eq!(&key.encode(), bytes, "ranked seed re-encode is not stable");
            }
            "span" => {
                let span = Span::decode(&bytes[..]).expect("a span seed decodes as a Span");
                assert_eq!(&span.encode(), bytes, "span seed re-encode is not stable");
            }
            other => panic!("unknown seed kind {other}"),
        }
    }
}

/// Every `fuzz_decode_differential` seed exercises the case its name declares.
///
/// The accept-frontier seeds decode and round-trip, and each rejection
/// witness rejects with the exact genre whose precedence the
/// differential oracle guards — so the committed corpus keeps seeding
/// the genre seams it was written for, and a reintroduced
/// verdict-before-padding ordering reddens this gate directly.
#[test]
fn differential_seeds_exercise_their_genre_seams() {
    for seed in fuzz_seed_set::seed_set() {
        if seed.target != "fuzz_decode_differential" {
            continue;
        }
        let bytes = &seed.bytes;
        match seed.name {
            "span_ordered" => {
                let span = Span::decode(&bytes[..]).expect("the ordered span seed decodes");
                assert_eq!(&span.encode(), bytes, "span seed re-encode is not stable");
            }
            "ranked_nested" => {
                let key = Ranked::decode(&bytes[..]).expect("the ranked seed decodes");
                assert_eq!(&key.encode(), bytes, "ranked seed re-encode is not stable");
            }
            "postcard_span" => {
                let payload: Vec<u8> =
                    postcard::from_bytes(bytes).expect("the frame carries a byte payload");
                let span = Span::decode(&payload[..]).expect("the framed payload is a span");
                assert_eq!(
                    span.encode(),
                    payload,
                    "the payload is the canonical encoding"
                );
            }
            "span_crossed" => {
                assert!(
                    matches!(Span::decode(&bytes[..]), Err(Decode::NotCanonical)),
                    "a crossed pair is the canonical spelling of no span"
                );
            }
            "span_crossed_padding" => {
                assert!(
                    matches!(Span::decode(&bytes[..]), Err(Decode::TrailingBits)),
                    "malformed padding outranks the refuted pair verdict"
                );
            }
            "span_negative_join" => {
                assert!(
                    matches!(Span::decode(&bytes[..]), Err(Decode::NotCanonical)),
                    "a whole negative-height join rejects as the validator would"
                );
            }
            "span_trailing" => {
                assert!(
                    matches!(Span::decode(&bytes[..]), Err(Decode::TrailingBits)),
                    "a spurious byte past the composite is the trailing genre"
                );
            }
            "ranked_mismatched" => {
                assert!(
                    matches!(Ranked::decode(&bytes[..]), Err(Decode::NotCanonical)),
                    "a rank prefix the version does not measure is non-canonical"
                );
            }
            "span_coincident" => {
                let span = Span::decode(&bytes[..]).expect("the coincident composite decodes");
                assert_eq!(
                    span.lo(),
                    span.hi(),
                    "the coincident seed's endpoints are one version"
                );
                assert_eq!(&span.encode(), bytes, "span seed re-encode is not stable");
            }
            other => panic!("unknown differential seed {other}"),
        }
    }
}

/// Every `fuzz_parse` seed parses as named and round-trips through its display.
///
/// The seeds actually exercise the text parsers they were written for,
/// and the wide seed really is wide (a magnitude past `u64::MAX`, the
/// tier random text never reaches).
#[test]
fn parse_seeds_parse_as_named_and_round_trip() {
    let mut saw_wide = false;
    for seed in fuzz_seed_set::seed_set() {
        if seed.target != "fuzz_parse" {
            continue;
        }
        let text = std::str::from_utf8(&seed.bytes).expect("parse seeds are UTF-8");
        match seed.name {
            "clock_display" => {
                let clock: Clock = text.parse().expect("the clock seed parses");
                assert_eq!(clock.to_string(), text, "clock display round-trip");
            }
            "version_nested_text" => {
                let version: Version = text.parse().expect("the nested version seed parses");
                assert_eq!(version.to_string(), text, "version display round-trip");
            }
            "party_nested_text" => {
                let party: Party = text.parse().expect("the party seed parses");
                assert_eq!(party.to_string(), text, "party display round-trip");
            }
            "version_wide" => {
                let version: Version = text.parse().expect("the wide version seed parses");
                assert_eq!(version.to_string(), text, "wide display round-trip");
                // A version leaf displays as its bare magnitude; 21+ digits
                // is past u64::MAX (20 digits), i.e. the wide-gamma tier.
                saw_wide |= text.len() >= 21;
            }
            other => panic!("unknown parse seed {other}"),
        }
    }
    assert!(saw_wide, "no fuzz_parse seed reaches the wide-decimal tier");
}

/// Carve the next length-prefixed chunk off a `fuzz_laws` seed, exactly as
/// the target's framing does (part of the wire contract the seed set
/// documents).
fn laws_chunk<'d>(data: &mut &'d [u8]) -> &'d [u8] {
    let Some((&len, rest)) = data.split_first() else {
        *data = &[];
        return &[];
    };
    let split = (len as usize).min(rest.len());
    let (bytes, tail) = rest.split_at(split);
    *data = tail;
    bytes
}

/// Carve one list script — `[arity: u8][pool indices]` — off a `fuzz_laws`
/// seed, exactly as the target's framing does.
///
/// Asserts the seed's bytes are in-band as written — the arity below the
/// target's fold and every index below its pool — so no seed byte silently
/// aliases a smaller value than it was written to represent.
fn laws_script(name: &str, data: &mut &[u8], pool: usize) -> usize {
    const ARITY_SPAN: usize = 18; // the target's arity band, per its framing
    let (&arity, rest) = data
        .split_first()
        .unwrap_or_else(|| panic!("{name}: input exhausted before a list script's arity byte"));
    *data = rest;
    assert!(
        usize::from(arity) < ARITY_SPAN,
        "{name}: script arity {arity} is out of the target's arity band"
    );
    for _ in 0..arity {
        let (&index, rest) = data
            .split_first()
            .unwrap_or_else(|| panic!("{name}: input exhausted inside a list script"));
        *data = rest;
        assert!(
            usize::from(index) < pool,
            "{name}: script index {index} is outside its pool of {pool}"
        );
    }
    usize::from(arity)
}

/// Every `fuzz_laws` seed decodes positionally per the target's framing —
/// three versions, two parties, a clock, then the three variadic list
/// scripts (version, party, clock pools), nothing left over.
///
/// So no chunk silently falls back to a default and stops representing the
/// value it was written for. And the corpus keeps its deliberate tails: the
/// wide-gamma seed's first version is a magnitude past `u64::MAX` (a
/// 21+-digit leaf, the decode tier random bytes essentially never reach),
/// some version script crosses the balanced counter's merged–merged carry
/// inside the first octave (arity 4..=9), and some crosses the second
/// octave (arity 15+).
#[test]
fn laws_seeds_decode_per_framing_and_stay_wide() {
    let mut saw_wide = false;
    let mut saw_carry = false;
    let mut saw_second_octave = false;
    for seed in fuzz_seed_set::seed_set() {
        if seed.target != "fuzz_laws" {
            continue;
        }
        let mut data = &seed.bytes[..];
        let data = &mut data;
        let versions: Vec<Version> = (0..3)
            .map(|position| {
                Version::decode(laws_chunk(data)).unwrap_or_else(|err| {
                    panic!(
                        "{}: version chunk {position} fails decode: {err}",
                        seed.name
                    )
                })
            })
            .collect();
        for position in 0..2 {
            Party::decode(laws_chunk(data)).unwrap_or_else(|err| {
                panic!("{}: party chunk {position} fails decode: {err}", seed.name)
            });
        }
        Clock::decode(laws_chunk(data))
            .unwrap_or_else(|err| panic!("{}: clock chunk fails decode: {err}", seed.name));
        let version_arity = laws_script(seed.name, data, 4);
        laws_script(seed.name, data, 3);
        laws_script(seed.name, data, 3);
        assert!(
            data.is_empty(),
            "{}: bytes past the framed chunks and scripts",
            seed.name
        );

        // A version leaf displays as its bare magnitude; 21+ digits is past
        // u64::MAX (20 digits), i.e. the wide-gamma decode tier.
        saw_wide |= versions[0].to_string().len() >= 21;
        saw_carry |= (4..=9).contains(&version_arity);
        saw_second_octave |= version_arity >= 15;
    }
    assert!(saw_wide, "no fuzz_laws seed reaches the wide-gamma tier");
    assert!(
        saw_carry,
        "no fuzz_laws seed crosses the merged–merged carry in the first octave"
    );
    assert!(
        saw_second_octave,
        "no fuzz_laws seed crosses the second arity octave"
    );
}
