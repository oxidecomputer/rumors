//! A nonshrinking population that checks balanced meet folds.
//!
//! One deep carrier is followed by `k − 1` flat versions that all dominate it,
//! so every intermediate meet remains byte-identical to the carrier. A
//! sequential fold would rescan that `d`-byte carrier for every operand and
//! cost `Θ(kd)`. The balanced fold revisits it once per reduction level, giving
//! the declared `O(d log k + k)` model. Absolute ceilings and two diagonal
//! doublings hold the measured work to that model, while the result assertion
//! proves that the population still exercises the intended nonshrinking case.

use before::testing::meter::registry::Shape;
use before::Version;
use suanpan::touch_meter;

/// One n-ary meet run over the shade population `MS(d, k)` through
/// `fold`: total input bytes, the fold model's level count, and
/// the touch counter over the fold body alone.
///
/// Carries the population's semantic leg (the fold returns the
/// carrier, byte for byte) and the one-touch-per-operand-byte
/// liveness floor.
fn run(d: usize, k: usize, fold: fn(Vec<Version>) -> Version) -> Run {
    let population = Shape::MeetShade.versions(d, k);
    let bytes: u64 = population.iter().map(|v| v.encode().len() as u64).sum();
    let carrier = population[0].clone();
    touch_meter::reset();
    let met = fold(population);
    let run = Run {
        bytes,
        levels: (2.0 * k as f64).log2(),
        touches: touch_meter::touches(),
    };
    assert_eq!(
        met, carrier,
        "the shades dominate the carrier everywhere: the meet is the carrier"
    );
    assert!(
        run.touches >= run.bytes,
        "meet fold at {bytes} operand bytes: {} digit touches under \
         the one-per-byte floor: the fold's accumulator work is not metered",
        run.touches,
    );
    run
}

/// One meet-fold run's counters and its model denominators.
struct Run {
    bytes: u64,
    levels: f64,
    touches: u64,
}

/// Assert one counter's model-normalized per-byte cost stays flat
/// (×1.25) across a doubling: `counter / (bytes · log2(2k))` — the
/// declared fold model's constant, as the stagger bands hold it.
fn assert_model_flat(name: &str, small: &Run, large: &Run, counter: fn(&Run) -> u64) {
    let (m1, m2) = (counter(small) as f64, counter(large) as f64);
    let (d1, d2) = (
        small.bytes as f64 * small.levels,
        large.bytes as f64 * large.levels,
    );
    eprintln!(
        "MEASURED meet_fold_{name}: small={m1}/{:.0} large={m2}/{:.0} \
         per_byte_level={:.3} -> {:.3}",
        d1,
        d2,
        m1 / d1,
        m2 / d2,
    );
    assert!(
        m2 * d1 <= m1 * d2 * 1.25,
        "meet_fold_{name}: the model-normalized per-byte cost grew more \
         than x1.25 across the doubling: {m1}/{d1} -> {m2}/{d2}"
    );
}

/// Carrier depth and shade count of the band's small run (the other
/// runs double both, twice).
const MEET_SHADE_SMALL: usize = 512;

/// Absolute touch ceilings for `meet_all` on the shade
/// diagonal, measured ×1.25 at
/// `MS(512, 512), MS(1,024, 1,024), MS(2,048, 2,048)` (the record
/// and every re-pin's movement live in the pin commits).
///
/// The balanced reduction's model-normalized constant is flat
/// across the three scales while the raw per-byte cost grows
/// exactly the documented one-level-per-doubling. A sequential reduction
/// would be quadratic on this diagonal.
const MEET_SHADE_CEILINGS: [u64; 3] = [5_805, 12_850, 28_215];

/// `Version::meet_all` is model-flat on the shade population: the
/// model-normalized per-byte cost stays flat (×1.25) across two
/// diagonal doublings, under absolute
/// pinned ceilings.
///
/// The population keeps the running meet full-size at every
/// combine, so a fold that re-walks its accumulator per operand rather than
/// per counter level reads about twice as much per byte after each
/// doubling.
#[test]
fn meet_all_shade_is_flat_per_unit() {
    // The public entry point, entered as callers do: the population's first
    // element (the carrier) as the receiver, the rest as items.
    let combine: fn(Vec<Version>) -> Version = |mut population| {
        let rest = population.split_off(1);
        let receiver = population.pop().expect("the population is nonempty");
        receiver.meet_all(rest)
    };
    let n = MEET_SHADE_SMALL;
    let runs = [
        run(n, n, combine),
        run(2 * n, 2 * n, combine),
        run(4 * n, 4 * n, combine),
    ];
    for (r, touch) in runs.iter().zip(MEET_SHADE_CEILINGS) {
        eprintln!(
            "MEASURED meet_all_shade: bytes={} touches={}",
            r.bytes, r.touches,
        );
        assert!(
            r.touches <= touch,
            "meet_all_shade: {} touches exceed the pinned ceiling {touch}",
            r.touches,
        );
    }
    for pair in runs.windows(2) {
        assert_model_flat("touches", &pair[0], &pair[1], |r| r.touches);
    }
}
