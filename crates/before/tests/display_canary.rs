//! A complexity-class canary on big-magnitude decimal rendering.
//!
//! `Display` on a spilled magnitude delegates to `num-bigint`'s
//! binary-to-decimal conversion, whose cost class is set by the dependency
//! floor (0.4.7 ships divide-and-conquer `to_radix_digits`; 0.4.8 fixes its
//! Burnikel–Ziegler division regression; 0.4.6's rendering is quadratic in
//! the magnitude width). That work happens entirely inside `num-bigint`, so
//! the deterministic limb meter cannot observe it — the crate's `Display`
//! records nothing while the conversion runs, and the board's display rows
//! declare that limb column not-applicable for exactly this reason. This
//! canary is therefore a deliberate *wall-clock* assertion (the board's
//! judged wall-exponent leg and its tripwire are the suite's only others):
//! a ratio of two measurements taken in the same process and profile, so
//! machine speed and build profile cancel, sized so the larger interval is
//! seconds-scale against scheduler noise, taken best-of-three, and bounded
//! with slack in both directions. Across the 8× width jump asserted here,
//! the quadratic class measures ~36× (0.4.6, this machine, dev profile)
//! and the divide-and-conquer class ~19×; the ceiling sits at their
//! geometric midpoint.

use std::time::{Duration, Instant};

use before::{meter, Version};

/// The smaller magnitude width, in bits.
///
/// Sized so the larger point renders in about a second and a half under
/// the dev profile: long enough to dwarf timer and scheduler jitter, short
/// enough to keep the whole canary seconds-scale.
const BASE_MAGNITUDE_BITS: usize = 250_000;

/// The width multiplier between the two measured points.
const WIDTH_JUMP: usize = 8;

/// Ceiling on the rendering-time ratio across the width jump, in tenths.
///
/// Measured at these sizes (2026-07-23, aarch64-apple-darwin, dev
/// profile): the quadratic rendering class reads 36.1× (asymptote 64×) and
/// the divide-and-conquer class 18.4–19.4× over three runs. The pin is
/// their geometric midpoint — ~1.35× above the D&C band, ~1.39× below the
/// quadratic reading — so it separates the complexity classes rather than
/// benchmarks the machine.
const RATIO_CEILING_TENTHS: u128 = 260;

/// Render the value `2^bits − 1` (one hugeleaf) to its decimal string and
/// return the wall time, best of three runs.
///
/// Best-of-three keeps the ratio honest under transient scheduler noise:
/// the minimum is the least-disturbed observation of a deterministic
/// computation.
fn render_time(bits: usize) -> Duration {
    let packed = meter::hugeleaf(bits);
    let version = Version::decode(&packed.bytes[..]).expect("hugeleaf is strict normal form");
    (0..3)
        .map(|_| {
            let start = Instant::now();
            let text = version.to_string();
            let elapsed = start.elapsed();
            assert!(
                text.len() > bits / 4,
                "a {bits}-bit magnitude renders at least bits/4 decimal digits"
            );
            elapsed
        })
        .min()
        .expect("three runs yield a minimum")
}

/// Rendering a spilled magnitude stays subquadratic in its width: the wall
/// time of an 8× wider rendering stays under 26× (the quadratic class
/// reads ~36× at these sizes).
///
/// Wall time is otherwise asserted only by the board's judged wall-exponent
/// leg and its tripwire; the module doc carries why it is tolerable here and
/// how the ratio is made robust.
#[test]
fn big_magnitude_rendering_stays_subquadratic() {
    let small = render_time(BASE_MAGNITUDE_BITS);
    let large = render_time(BASE_MAGNITUDE_BITS * WIDTH_JUMP);
    eprintln!(
        "MEASURED display_canary: small={}ms large={}ms ratio={:.2}",
        small.as_millis(),
        large.as_millis(),
        large.as_secs_f64() / small.as_secs_f64(),
    );
    assert!(
        large.as_nanos() * 10 <= small.as_nanos() * RATIO_CEILING_TENTHS,
        "rendering a {WIDTH_JUMP}x wider magnitude took {:?} against {:?}: \
         the ratio exceeds {RATIO_CEILING_TENTHS} tenths, the quadratic-class \
         signature this canary exists to catch",
        large,
        small,
    );
}
