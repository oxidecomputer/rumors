//! Scaling checks for balanced joins over correlated populations.
//!
//! Each family varies arity, operand size, or both while retaining a known
//! result. Absolute ceilings catch constant-factor drift; adjacent points also
//! check that measured work follows the balanced-fold model rather than a
//! growing sequential accumulator.

use super::uniform_version;
use before::testing::meter;
use before::testing::meter::registry::Shape;
use before::Version;
use suanpan::touch_meter;

/// One balanced `Version::join_all` run over the staggered
/// population: total input bytes, the model's level count, and the
/// three fold counters over the fold body alone.
///
/// Carries two semantic legs — the balanced fold equals the
/// sequential left fold, and both equal the constant-1 Version
/// (every slot owned exactly once) — and the teeth liveness floor:
/// every operand's teeth are folded at least once in its
/// first-level merge, so a touch reading under `n·m` means the
/// fold's accumulator work left the metered representation.
fn version_fold_run(n: usize, m: usize) -> Run {
    let (versions, _) = Shape::StaggerPopulation.population(n, m);
    let mut versions: Vec<Version> = versions.iter().map(meter::Encoding::version).collect();
    let bytes: u64 = versions.iter().map(|v| v.encode().len() as u64).sum();
    let sequential = versions.iter().fold(Version::new(), |acc, v| acc | v);
    let rest = versions.split_off(1);
    let receiver = versions.pop().expect("the population is nonempty");
    touch_meter::reset();
    meter::reset_scan_bits();
    let out = receiver.join_all(rest);
    let run = Run {
        bytes,
        levels: (2.0 * n as f64).log2(),
        touches: touch_meter::touches(),
        scan_bits: meter::scan_bits(),
    };
    assert_eq!(out, sequential, "the balanced fold equals the left fold");
    assert_eq!(
        out,
        uniform_version(1u8),
        "the population's teeth tile the whole domain at height 1"
    );
    assert!(
        run.touches >= (n * m) as u64,
        "join_all over {n}x{m} teeth: {} digit touches under the \
         one-per-tooth floor: the fold's accumulator work is not metered",
        run.touches,
    );
    run
}

/// One balanced `Party::join_all` run over the staggered id
/// population: total input bytes, the model's level count, and the
/// scan counter over the fold body alone.
///
/// The id walk allocates nothing and does no arithmetic, so
/// scanned bits are the only deterministic meter that sees it.
///
/// Carries the seed-reunion semantic leg (the population's slots
/// tile the whole seed region) and the full-examination liveness
/// floor: a scan reading under 8 bits per operand byte means the
/// walk left the metered primitives.
fn party_fold_run(n: usize, m: usize) -> Run {
    let (_, ids) = Shape::StaggerPopulation.population(n, m);
    let mut parties: Vec<before::Party> = ids
        .iter()
        .map(|p| before::Party::decode(&p.bytes[..]).expect("canonical"))
        .collect();
    let bytes: u64 = parties.iter().map(|p| p.encode().len() as u64).sum();
    let rest = parties.split_off(1);
    let mut acc = parties.remove(0);
    touch_meter::reset();
    meter::reset_scan_bits();
    acc.join_all(rest)
        .expect("the staggered slots are pairwise disjoint");
    let run = Run {
        bytes,
        levels: (2.0 * n as f64).log2(),
        touches: touch_meter::touches(),
        scan_bits: meter::scan_bits(),
    };
    assert!(acc.is_seed(), "the staggered slots reunite the seed region");
    assert!(
        run.scan_bits >= 8 * run.bytes,
        "party join_all over {n}x{m} slots: {} scanned bits under the \
         full-examination floor: the walk is not metered",
        run.scan_bits,
    );
    run
}

/// One fold run's counters and its model denominators.
struct Run {
    bytes: u64,
    levels: f64,
    touches: u64,
    scan_bits: u64,
}

/// Assert one counter's model-normalized per-byte cost stays flat
/// (×1.25) across a doubling: `counter / (bytes · log2(2n))` — the
/// declared fold model's constant.
fn assert_model_flat(name: &str, small: &Run, large: &Run, counter: fn(&Run) -> u64) {
    let (m1, m2) = (counter(small) as f64, counter(large) as f64);
    let (d1, d2) = (
        small.bytes as f64 * small.levels,
        large.bytes as f64 * large.levels,
    );
    eprintln!(
        "MEASURED fold_stagger_{name}: small={m1}/{:.0} large={m2}/{:.0} \
         per_byte_level={:.3} -> {:.3}",
        d1,
        d2,
        m1 / d1,
        m2 / d2,
    );
    assert!(
        m2 * d1 <= m1 * d2 * 1.25,
        "fold_stagger_{name}: the model-normalized per-byte cost grew more \
         than x1.25 across the doubling: {m1}/{d1} -> {m2}/{d2}"
    );
}

/// Assert one run's counters against its absolute pinned ceilings,
/// printing the measured line
/// re-pins read from.
fn assert_ceilings(name: &str, run: &Run, ceilings: (u64, u64)) {
    eprintln!(
        "MEASURED fold_stagger_{name}: bytes={} touches={} scan_bits={}",
        run.bytes, run.touches, run.scan_bits,
    );
    let (touch, scan) = ceilings;
    assert!(
        run.touches <= touch,
        "fold_stagger_{name}: {} touches exceed the pinned ceiling {touch}",
        run.touches,
    );
    assert!(
        run.scan_bits <= scan,
        "fold_stagger_{name}: {} scanned bits exceed the pinned ceiling {scan}",
        run.scan_bits,
    );
}

/// The bands' base population: 64 operands of 64 teeth (the board's
/// stagger family at scale 1.0); each axis doubles its own knob
/// twice from here.
const STAGGER_SMALL: usize = 64;

/// Absolute touch and scan ceilings for the version fold's
/// arity axis, measured ×1.25 at
/// `(n, m) = (64, 64), (128, 64), (256, 64)`.
const VERSION_ARITY_CEILINGS: [(u64, u64); 3] = [
    (214_954, 959_793),
    (537_433, 2_462_273),
    (1_310_392, 6_132_833),
];

/// Absolute ceilings for the version fold's size axis, measured
/// ×1.25 at `(n, m) = (64, 64), (64, 128), (64, 256)`.
const VERSION_SIZE_CEILINGS: [(u64, u64); 3] = [
    (214_954, 959_793),
    (429_994, 1_919_798),
    (860_074, 3_839_803),
];

/// Absolute ceilings for the party fold's arity axis, measured
/// ×1.25 (scan is the fold's only live counter; the touch leg asserts
/// that the id walk does no accumulator work).
const PARTY_ARITY_CEILINGS: [u64; 3] = [1_781_690, 4_034_330, 9_075_610];

/// Absolute ceilings for the party fold's size axis, measured
/// ×1.25.
const PARTY_SIZE_CEILINGS: [u64; 3] = [1_781_690, 3_892_090, 8_437_970];

/// The version fold's model-normalized cost stays flat across two
/// arity doublings at fixed operand size.
///
/// `join_all` pays the declared `O(D log 2k)` and nothing more
/// when every reduction merge swells to the sum of its inputs.
///
/// The raw per-byte cost on this axis legitimately grows one
/// level's worth per doubling (the documented log factor); the
/// band divides it out and holds the model's constant, so a
/// reduction whose merges re-walk more than the swollen overlay —
/// a growing-accumulator regression, a per-level re-scan — reads
/// over the bound while the model's own growth passes exactly.
#[test]
fn fold_version_stagger_arity_axis_is_flat_per_unit() {
    let m = STAGGER_SMALL;
    let runs = [
        version_fold_run(m, m),
        version_fold_run(2 * m, m),
        version_fold_run(4 * m, m),
    ];
    for (run, ceilings) in runs.iter().zip(VERSION_ARITY_CEILINGS) {
        assert_ceilings("version_arity", run, ceilings);
    }
    for pair in runs.windows(2) {
        assert_model_flat("version_arity_touches", &pair[0], &pair[1], |r| r.touches);
        assert_model_flat("version_arity_scan_bits", &pair[0], &pair[1], |r| {
            r.scan_bits
        });
    }
}

/// The version fold's model-normalized cost stays flat across two
/// operand-size doublings at fixed arity.
///
/// At a fixed level count the fold is linear in the population's
/// input bytes, however large each swollen intermediate grows.
#[test]
fn fold_version_stagger_size_axis_is_flat_per_unit() {
    let n = STAGGER_SMALL;
    let runs = [
        version_fold_run(n, n),
        version_fold_run(n, 2 * n),
        version_fold_run(n, 4 * n),
    ];
    for (run, ceilings) in runs.iter().zip(VERSION_SIZE_CEILINGS) {
        assert_ceilings("version_size", run, ceilings);
    }
    for pair in runs.windows(2) {
        assert_model_flat("version_size_touches", &pair[0], &pair[1], |r| r.touches);
        assert_model_flat("version_size_scan_bits", &pair[0], &pair[1], |r| {
            r.scan_bits
        });
    }
}

/// Scan cost follows the fold model as the number of parties doubles twice.
///
/// Operand size stays fixed. The regions remain interleaved until the last
/// merge level, while arithmetic counters remain zero.
#[test]
fn fold_party_stagger_arity_axis_is_flat_per_unit() {
    let m = STAGGER_SMALL;
    let runs = [
        party_fold_run(m, m),
        party_fold_run(2 * m, m),
        party_fold_run(4 * m, m),
    ];
    for (run, ceiling) in runs.iter().zip(PARTY_ARITY_CEILINGS) {
        assert_ceilings("party_arity", run, (0, ceiling));
    }
    for pair in runs.windows(2) {
        assert_model_flat("party_arity_scan_bits", &pair[0], &pair[1], |r| r.scan_bits);
    }
}

/// The party fold's model-normalized scan cost stays flat across
/// two operand-size doublings at fixed arity, and the id walk
/// forces no accumulator work at any scale (touches stay at zero).
#[test]
fn fold_party_stagger_size_axis_is_flat_per_unit() {
    let n = STAGGER_SMALL;
    let runs = [
        party_fold_run(n, n),
        party_fold_run(n, 2 * n),
        party_fold_run(n, 4 * n),
    ];
    for (run, ceiling) in runs.iter().zip(PARTY_SIZE_CEILINGS) {
        assert_ceilings("party_size", run, (0, ceiling));
    }
    for pair in runs.windows(2) {
        assert_model_flat("party_size_scan_bits", &pair[0], &pair[1], |r| r.scan_bits);
    }
}
