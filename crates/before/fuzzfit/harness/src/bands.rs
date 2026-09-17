//! Committed fuel bounds for public `before` operations.
//!
//! `bin/calibrate` generates [`BANDS`] from a fixed corpus. Tests then judge
//! fresh programs against the committed values rather than fitting new ones,
//! so a cost change requires an explicit re-pin. Run `just fuzzfit-calibrate`
//! after a deliberate toolchain, implementation, or generator change and
//! review this file like a snapshot.
//!
//! A band is keyed by operation and outcome. Operations that can reject input
//! have separate success and rejection bands because those paths can have
//! different cost laws. For denominator `d`, fuel `f`, and residual
//! `r = log₁₀(f) - (intercept + slope * log₁₀(d))`, a sample is accepted when:
//!
//! ```text
//! -(width_below + ENFORCE_MARGIN_BELOW) <= r
//! r <= width_above + ENFORCE_MARGIN
//! ```
//!
//! The fitted bands apply from `min_denom` upward. Bootstrap operations below
//! that floor use [`SMALL_BANDS`], whose costs are approximately constant over
//! their narrow ranges. A high result flags extra work; a low result flags a
//! dead meter or an unexpectedly constant implementation. [`crate::fit`]
//! explains how the asymmetric widths are fitted.
//!
//! Identity fast paths are excluded because their cost is constant by design;
//! `before`'s `identity_fast_paths` pins them directly. Empty `meet_all` is also
//! unpriced because it has no operand from which to derive a size. All other
//! public operation outcomes are represented here.
//!
//! The pin uses the release Wasm guest, the toolchain in [`PINNED_RUSTC`], and
//! Wasmtime fuel. Calibration also checks fixed reach programs, verifies that
//! the measured floor remains above a no-op, and records which bands the
//! deterministic prefix can refit. These checks keep the tolerances tied to
//! observed behavior without duplicating calibration during every test run.

/// One pinned band (see the module doc for the membership predicate).
#[derive(Debug, Clone, Copy)]
pub struct Band {
    /// The guest kernel this band prices (with `rejected`, the band key).
    pub kernel: &'static str,
    /// Whether this band prices the kernel's rejection arm (`ERR_OP`
    /// outcomes) rather than its success path.
    pub rejected: bool,
    /// Pinned log-log slope.
    pub slope: f64,
    /// Pinned intercept (`log₁₀` fuel at 1 bit).
    pub intercept: f64,
    /// Pinned ceiling width: max positive residual over the calibration
    /// corpus (the regression flag's threshold).
    pub width_above: f64,
    /// Pinned floor width: max negative residual magnitude over the
    /// calibration corpus (the liveness flag's threshold).
    pub width_below: f64,
    /// Smallest calibrated denominator (bits): the judgment floor.
    pub min_denom: u64,
    /// Largest calibrated denominator (bits), recorded for provenance.
    pub max_denom: u64,
    /// Calibration corpus size behind this band.
    pub samples: usize,
    /// Whether the band was constant-classified (slope pinned at 0).
    pub constant: bool,
}

/// Slack beyond each band's fitted ceiling, in `log₁₀` units.
///
/// This absorbs ordinary differences between calibration and enforcement
/// contexts. Calibration checks the fixed reach programs against this margin,
/// while keeping the ceiling tight enough to detect added work.
pub const ENFORCE_MARGIN: f64 = 0.2;

/// Slack beyond each band's fitted floor, in `log₁₀` units.
///
/// The floor distinguishes a live measurement from a no-op while allowing
/// fresh programs to be cheaper than the calibration corpus. Calibration
/// verifies that every fitted floor still clears the no-op level after this
/// margin is subtracted.
pub const ENFORCE_MARGIN_BELOW: f64 = 0.8;

/// The staleness cross-check's prefix length.
///
/// The enforcement suite refits the first `REFIT_PREFIX_PROGRAMS`
/// programs of the deterministic calibration stream
/// (`drive::for_each_deterministic_program`) and compares each covered
/// band key's fresh line against its pin, so a pin the current code would
/// no longer produce fails loud instead of silently drifting. A prefix,
/// because a full-corpus refit would duplicate the calibration sweep
/// inside every suite run.
pub const REFIT_PREFIX_PROGRAMS: usize = 256;

/// Largest allowed [`crate::fit::line_divergence`] between the prefix
/// refit and the pin, in `log₁₀` units.
///
/// The deterministic prefix is smaller than the calibration corpus, so sparse
/// outcomes fit less precisely. This tolerance makes the comparison a stale-pin
/// detector; the committed bands remain the criterion of record.
pub const REFIT_TOLERANCE: f64 = 0.7;

/// Look up the pinned band for one band key (kernel × outcome).
pub fn band_for(kernel: &str, rejected: bool) -> Option<&'static Band> {
    BANDS
        .iter()
        .find(|b| b.kernel == kernel && b.rejected == rejected)
}

/// One step's verdict against a band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Within the band: the pinned claim held for this step.
    InBand,
    /// Above the band: a regression flag (more work than the pinned law
    /// predicts for this size).
    Above,
    /// Below the band: less work than the calibrated executions, indicating a
    /// dead meter or an unmeasured path.
    Below,
    /// Below the calibrated floor: not judged (the line extrapolates the
    /// constant-overhead regime downward there).
    BelowFloor,
}

/// Judge one measured step against a band (the enforcement predicate; see
/// the module doc for its definition).
pub fn judge_against(band: &Band, denom_bits: u64, fuel: u64) -> Verdict {
    if denom_bits < band.min_denom {
        return Verdict::BelowFloor;
    }
    let predicted = band.intercept + band.slope * (denom_bits as f64).log10();
    let actual = (fuel.max(1) as f64).log10();
    if actual > predicted + band.width_above + ENFORCE_MARGIN {
        Verdict::Above
    } else if actual < predicted - band.width_below - ENFORCE_MARGIN_BELOW {
        Verdict::Below
    } else {
        Verdict::InBand
    }
}

/// Kernels with success bands for operands below the general fit floor.
///
/// The size-law legs are structurally out of range below
/// [`crate::fit::FIT_FLOOR_BITS`] — the point leg returns
/// [`Verdict::BelowFloor`], the shape leg buckets only floored samples,
/// and the refit fitter drops sub-floor samples. This list keeps those small
/// operand sizes under an explicit judgment.
/// The committed expectation list is this constant: a calibration that
/// stops producing a small band for any kernel here fails the
/// enforcement suite by name.
pub const SMALL_BAND_KERNELS: &[&str] = &[
    "ff_clock_tick",
    "ff_clock_join",
    "ff_clock_encode",
    "ff_clock_decode",
];

/// Look up the pinned small-operand band for one band key.
pub fn small_band_for(kernel: &str, rejected: bool) -> Option<&'static Band> {
    SMALL_BANDS
        .iter()
        .find(|b| b.kernel == kernel && b.rejected == rejected)
}

/// Judge one sub-floor step against its small-operand band.
///
/// Returns `Some((band, verdict))` when a small band prices this key
/// and the step lands inside the band's calibrated span, `None` when
/// the step stays structurally unjudged (no small band, or outside the
/// span).
///
/// Constant bands apply only over their calibrated interval. Beyond
/// `max_denom`, ordinary input-proportionate growth may begin.
pub fn judge_small(
    kernel: &str,
    rejected: bool,
    denom_bits: u64,
    fuel: u64,
) -> Option<(&'static Band, Verdict)> {
    let band = small_band_for(kernel, rejected)?;
    if denom_bits < band.min_denom || denom_bits > band.max_denom {
        return None;
    }
    Some((band, judge_against(band, denom_bits, fuel)))
}

/// The toolchain that pinned the constants below, as `rustc --version`
/// reports it.
///
/// Generated by `just fuzzfit-calibrate` alongside [`BANDS`]: guest
/// codegen (and so every fuel constant) is a function of this compiler,
/// and the suite asserts the building toolchain matches, so a toolchain
/// bump reads red until the bands are re-pinned. wasmtime (the fuel
/// schedule's other half) is pinned exactly by the workspace
/// `Cargo.lock`; bumping it there is likewise a re-pin event.
pub const PINNED_RUSTC: &str = "rustc 1.97.1 (8bab26f4f 2026-07-14)";

/// The pinned bands.
///
/// Generated by `just fuzzfit-calibrate` — review the diff like a
/// snapshot, commit with a dated movement annotation.
pub const BANDS: &[Band] = &[
    Band {
        kernel: "ff_clock_decode",
        rejected: false,
        slope: 1.042764,
        intercept: 1.961709,
        width_above: 0.113759,
        width_below: 0.132722,
        min_denom: 128,
        max_denom: 12160,
        samples: 1469,
        constant: false,
    },
    Band {
        kernel: "ff_clock_encode",
        rejected: false,
        slope: 0.324264,
        intercept: 2.127748,
        width_above: 0.191734,
        width_below: 0.229845,
        min_denom: 128,
        max_denom: 12155,
        samples: 1427,
        constant: false,
    },
    Band {
        kernel: "ff_clock_fork",
        rejected: false,
        slope: 0.848437,
        intercept: 1.556456,
        width_above: 0.326882,
        width_below: 0.339624,
        min_denom: 128,
        max_denom: 12234,
        samples: 95834,
        constant: false,
    },
    Band {
        kernel: "ff_clock_from_parts",
        rejected: false,
        slope: 0.000000,
        intercept: 2.431364,
        width_above: 0.000000,
        width_below: -0.000000,
        min_denom: 128,
        max_denom: 12151,
        samples: 1635,
        constant: false,
    },
    Band {
        kernel: "ff_clock_into_parts",
        rejected: false,
        slope: 0.000000,
        intercept: 2.546543,
        width_above: 0.019305,
        width_below: -0.000000,
        min_denom: 128,
        max_denom: 12234,
        samples: 34198,
        constant: false,
    },
    Band {
        kernel: "ff_clock_join",
        rejected: false,
        slope: 0.998468,
        intercept: 2.268268,
        width_above: 0.135981,
        width_below: 1.196063,
        min_denom: 128,
        max_denom: 24065,
        samples: 27985,
        constant: false,
    },
    Band {
        kernel: "ff_clock_join",
        rejected: true,
        slope: 1.488793,
        intercept: -0.243603,
        width_above: 0.967142,
        width_below: 0.667957,
        min_denom: 129,
        max_denom: 24310,
        samples: 421,
        constant: false,
    },
    Band {
        kernel: "ff_clock_own_version",
        rejected: false,
        slope: 1.069636,
        intercept: 1.891834,
        width_above: 0.115426,
        width_below: 0.150984,
        min_denom: 128,
        max_denom: 13458,
        samples: 1019,
        constant: false,
    },
    Band {
        kernel: "ff_clock_recv",
        rejected: false,
        slope: 1.047285,
        intercept: 2.506641,
        width_above: 0.308530,
        width_below: 0.537756,
        min_denom: 128,
        max_denom: 20860,
        samples: 5566,
        constant: false,
    },
    Band {
        kernel: "ff_clock_seed",
        rejected: false,
        slope: 0.000000,
        intercept: 2.227887,
        width_above: 0.000000,
        width_below: 0.000000,
        min_denom: 8,
        max_denom: 8,
        samples: 4565,
        constant: true,
    },
    Band {
        kernel: "ff_clock_send",
        rejected: false,
        slope: 1.080457,
        intercept: 2.477080,
        width_above: 0.209551,
        width_below: 0.363735,
        min_denom: 128,
        max_denom: 12148,
        samples: 3398,
        constant: false,
    },
    Band {
        kernel: "ff_clock_sync",
        rejected: false,
        slope: 1.012054,
        intercept: 2.151295,
        width_above: 0.302247,
        width_below: 1.069405,
        min_denom: 128,
        max_denom: 24394,
        samples: 1702,
        constant: false,
    },
    Band {
        kernel: "ff_clock_sync",
        rejected: true,
        slope: 1.377784,
        intercept: 0.102708,
        width_above: 0.500653,
        width_below: 1.032758,
        min_denom: 128,
        max_denom: 23106,
        samples: 428,
        constant: false,
    },
    Band {
        kernel: "ff_clock_tick",
        rejected: false,
        slope: 1.066778,
        intercept: 2.518810,
        width_above: 0.248513,
        width_below: 0.364683,
        min_denom: 128,
        max_denom: 12234,
        samples: 316304,
        constant: false,
    },
    Band {
        kernel: "ff_clock_version",
        rejected: false,
        slope: 0.000000,
        intercept: 2.397940,
        width_above: 0.221153,
        width_below: -0.000000,
        min_denom: 128,
        max_denom: 12239,
        samples: 12271,
        constant: false,
    },
    Band {
        kernel: "ff_party_covers",
        rejected: false,
        slope: 1.017781,
        intercept: 1.846113,
        width_above: 0.088467,
        width_below: 1.523615,
        min_denom: 128,
        max_denom: 10316,
        samples: 2372,
        constant: false,
    },
    Band {
        kernel: "ff_party_decode",
        rejected: false,
        slope: 0.973486,
        intercept: 2.029079,
        width_above: 0.027874,
        width_below: 0.039380,
        min_denom: 128,
        max_denom: 5168,
        samples: 1782,
        constant: false,
    },
    Band {
        kernel: "ff_party_encode",
        rejected: false,
        slope: 0.408195,
        intercept: 1.590225,
        width_above: 0.254817,
        width_below: 0.267794,
        min_denom: 128,
        max_denom: 5160,
        samples: 1636,
        constant: false,
    },
    Band {
        kernel: "ff_party_fork",
        rejected: false,
        slope: 0.888539,
        intercept: 1.936730,
        width_above: 0.070282,
        width_below: 0.027805,
        min_denom: 130,
        max_denom: 5120,
        samples: 669,
        constant: false,
    },
    Band {
        kernel: "ff_party_forks",
        rejected: false,
        slope: 0.892139,
        intercept: 2.232931,
        width_above: 0.098463,
        width_below: 0.286358,
        min_denom: 128,
        max_denom: 4760,
        samples: 1396,
        constant: false,
    },
    Band {
        kernel: "ff_party_is_disjoint",
        rejected: false,
        slope: 1.215406,
        intercept: 0.909012,
        width_above: 0.480334,
        width_below: 1.076111,
        min_denom: 128,
        max_denom: 10316,
        samples: 2372,
        constant: false,
    },
    Band {
        kernel: "ff_party_join",
        rejected: false,
        slope: 1.258628,
        intercept: 1.496206,
        width_above: 0.350765,
        width_below: 0.897242,
        min_denom: 128,
        max_denom: 8622,
        samples: 33343,
        constant: false,
    },
    Band {
        kernel: "ff_party_join",
        rejected: true,
        slope: 1.696084,
        intercept: -0.420939,
        width_above: 0.972662,
        width_below: 0.528437,
        min_denom: 128,
        max_denom: 10316,
        samples: 438,
        constant: false,
    },
    Band {
        kernel: "ff_party_seed",
        rejected: false,
        slope: 0.000000,
        intercept: 2.201961,
        width_above: 0.041077,
        width_below: 0.000564,
        min_denom: 8,
        max_denom: 8,
        samples: 369,
        constant: true,
    },
    Band {
        kernel: "ff_party_without",
        rejected: false,
        slope: 0.782902,
        intercept: 2.314482,
        width_above: 0.094888,
        width_below: 0.045958,
        min_denom: 128,
        max_denom: 4870,
        samples: 324,
        constant: false,
    },
    Band {
        kernel: "ff_party_without",
        rejected: true,
        slope: 1.369260,
        intercept: 1.182955,
        width_above: 0.426357,
        width_below: 0.289841,
        min_denom: 134,
        max_denom: 9980,
        samples: 342,
        constant: false,
    },
    Band {
        kernel: "ff_rank_add",
        rejected: false,
        slope: 0.345953,
        intercept: 2.575215,
        width_above: 0.158355,
        width_below: 0.249622,
        min_denom: 128,
        max_denom: 3920,
        samples: 620,
        constant: false,
    },
    Band {
        kernel: "ff_rank_checked_sub",
        rejected: false,
        slope: 0.677853,
        intercept: 1.503029,
        width_above: 0.468442,
        width_below: 0.336929,
        min_denom: 128,
        max_denom: 3920,
        samples: 880,
        constant: false,
    },
    Band {
        kernel: "ff_rank_checked_sub",
        rejected: true,
        slope: -0.130850,
        intercept: 2.774849,
        width_above: 0.532499,
        width_below: 0.139284,
        min_denom: 128,
        max_denom: 1952,
        samples: 118,
        constant: false,
    },
    Band {
        kernel: "ff_rank_cmp",
        rejected: false,
        slope: 0.734923,
        intercept: 1.265285,
        width_above: 0.178565,
        width_below: 1.487634,
        min_denom: 128,
        max_denom: 3920,
        samples: 499,
        constant: false,
    },
    Band {
        kernel: "ff_rank_display",
        rejected: false,
        slope: 0.936139,
        intercept: 1.272334,
        width_above: 0.109999,
        width_below: 0.065678,
        min_denom: 128,
        max_denom: 15848,
        samples: 1226,
        constant: false,
    },
    Band {
        kernel: "ff_version_cmp",
        rejected: false,
        slope: 1.128005,
        intercept: 1.541184,
        width_above: 0.361605,
        width_below: 0.961437,
        min_denom: 128,
        max_denom: 17400,
        samples: 2247,
        constant: false,
    },
    Band {
        kernel: "ff_version_concurrent",
        rejected: false,
        slope: 1.126046,
        intercept: 1.545980,
        width_above: 0.352899,
        width_below: 0.960476,
        min_denom: 128,
        max_denom: 17400,
        samples: 2247,
        constant: false,
    },
    Band {
        kernel: "ff_version_decode",
        rejected: false,
        slope: 1.101449,
        intercept: 1.783297,
        width_above: 0.303163,
        width_below: 0.360463,
        min_denom: 128,
        max_denom: 8768,
        samples: 3084,
        constant: false,
    },
    Band {
        kernel: "ff_version_distance",
        rejected: false,
        slope: 1.177596,
        intercept: 1.846553,
        width_above: 0.307104,
        width_below: 0.403961,
        min_denom: 128,
        max_denom: 6174,
        samples: 2224,
        constant: false,
    },
    Band {
        kernel: "ff_version_encode",
        rejected: false,
        slope: 0.426330,
        intercept: 1.510380,
        width_above: 0.339012,
        width_below: 0.254732,
        min_denom: 128,
        max_denom: 8767,
        samples: 2937,
        constant: false,
    },
    Band {
        kernel: "ff_version_join",
        rejected: false,
        slope: 0.996252,
        intercept: 2.273630,
        width_above: 0.243653,
        width_below: 2.339654,
        min_denom: 128,
        max_denom: 15575,
        samples: 5419,
        constant: false,
    },
    Band {
        kernel: "ff_version_join_all",
        rejected: false,
        slope: 1.123978,
        intercept: 2.475601,
        width_above: 0.333039,
        width_below: 2.213928,
        min_denom: 128,
        max_denom: 187770,
        samples: 3705,
        constant: false,
    },
    Band {
        kernel: "ff_version_lag",
        rejected: false,
        slope: 1.103047,
        intercept: 2.010897,
        width_above: 0.294135,
        width_below: 0.379510,
        min_denom: 128,
        max_denom: 6174,
        samples: 2224,
        constant: false,
    },
    Band {
        kernel: "ff_version_meet",
        rejected: false,
        slope: 0.991377,
        intercept: 2.267561,
        width_above: 0.246181,
        width_below: 2.444125,
        min_denom: 128,
        max_denom: 17157,
        samples: 3064,
        constant: false,
    },
    Band {
        kernel: "ff_version_meet_all",
        rejected: false,
        slope: 1.100015,
        intercept: 2.103468,
        width_above: 0.269385,
        width_below: 1.328208,
        min_denom: 128,
        max_denom: 131930,
        samples: 1197,
        constant: false,
    },
    Band {
        kernel: "ff_version_min_ticks",
        rejected: false,
        slope: 1.095726,
        intercept: 2.406490,
        width_above: 0.261864,
        width_below: 0.465707,
        min_denom: 128,
        max_denom: 8767,
        samples: 3050,
        constant: false,
    },
    Band {
        kernel: "ff_version_project",
        rejected: false,
        slope: 0.864276,
        intercept: 2.445508,
        width_above: 0.580713,
        width_below: 0.399235,
        min_denom: 128,
        max_denom: 52428,
        samples: 4202,
        constant: false,
    },
    Band {
        kernel: "ff_version_rank",
        rejected: false,
        slope: 1.137634,
        intercept: 1.909928,
        width_above: 0.339661,
        width_below: 0.408965,
        min_denom: 128,
        max_denom: 8767,
        samples: 6320,
        constant: false,
    },
    Band {
        kernel: "ff_version_tick",
        rejected: false,
        slope: 0.988270,
        intercept: 2.690843,
        width_above: 0.226701,
        width_below: 0.553137,
        min_denom: 128,
        max_denom: 13827,
        samples: 2593,
        constant: false,
    },
];

/// The pinned small-operand bands: one constant-classified band per
/// [`SMALL_BAND_KERNELS`] entry (success arm), judged below the fit
/// floor over each band's own calibrated span.
///
/// Generated by `just fuzzfit-calibrate` alongside [`BANDS`], from the
/// pooled sub-floor samples of the calibration corpus and the
/// deterministic bootstrap stream — review the diff like a snapshot,
/// commit with a dated movement annotation.
pub const SMALL_BANDS: &[Band] = &[
    Band {
        kernel: "ff_clock_tick",
        rejected: false,
        slope: 0.000000,
        intercept: 4.200028,
        width_above: 0.810810,
        width_below: 0.443316,
        min_denom: 10,
        max_denom: 127,
        samples: 1304457,
        constant: true,
    },
    Band {
        kernel: "ff_clock_join",
        rejected: false,
        slope: 0.000000,
        intercept: 4.138107,
        width_above: 0.307092,
        width_below: 0.586170,
        min_denom: 20,
        max_denom: 127,
        samples: 4509,
        constant: true,
    },
    Band {
        kernel: "ff_clock_encode",
        rejected: false,
        slope: 0.000000,
        intercept: 2.726782,
        width_above: 0.301789,
        width_below: 0.135717,
        min_denom: 10,
        max_denom: 127,
        samples: 6214,
        constant: true,
    },
    Band {
        kernel: "ff_clock_decode",
        rejected: false,
        slope: 0.000000,
        intercept: 3.651783,
        width_above: 0.603659,
        width_below: 0.338971,
        min_denom: 16,
        max_denom: 120,
        samples: 6164,
        constant: true,
    },
];

/// The band keys the pin-time prefix refit covered: the staleness
/// cross-check's committed expectation list.
///
/// Generated by `just fuzzfit-calibrate` alongside [`BANDS`]: every key
/// listed here had, at pin time, a prefix refit whose classification
/// matched its pin. The enforcement suite requires each listed key to
/// still fit, still match its pin's classification, and still agree
/// within [`REFIT_TOLERANCE`] — so coverage decay, a classification flip
/// (the reach-regression tell), and line drift each fail by name instead
/// of hollowing the check out silently. Keys not listed are outside the
/// staleness detector's reach at pin time; calibration prints them for
/// the re-pinner to review.
pub const REFIT_COVERAGE: &[(&str, bool)] = &[
    ("ff_clock_decode", false),
    ("ff_clock_encode", false),
    ("ff_clock_fork", false),
    ("ff_clock_from_parts", false),
    ("ff_clock_into_parts", false),
    ("ff_clock_join", false),
    ("ff_clock_join", true),
    ("ff_clock_own_version", false),
    ("ff_clock_recv", false),
    ("ff_clock_seed", false),
    ("ff_clock_send", false),
    ("ff_clock_sync", false),
    ("ff_clock_sync", true),
    ("ff_clock_tick", false),
    ("ff_clock_version", false),
    ("ff_party_covers", false),
    ("ff_party_decode", false),
    ("ff_party_encode", false),
    ("ff_party_fork", false),
    ("ff_party_forks", false),
    ("ff_party_is_disjoint", false),
    ("ff_party_join", false),
    ("ff_party_join", true),
    ("ff_party_seed", false),
    ("ff_party_without", false),
    ("ff_party_without", true),
    ("ff_rank_add", false),
    ("ff_rank_checked_sub", false),
    ("ff_rank_checked_sub", true),
    ("ff_rank_cmp", false),
    ("ff_rank_display", false),
    ("ff_version_cmp", false),
    ("ff_version_concurrent", false),
    ("ff_version_decode", false),
    ("ff_version_distance", false),
    ("ff_version_encode", false),
    ("ff_version_join", false),
    ("ff_version_join_all", false),
    ("ff_version_lag", false),
    ("ff_version_meet", false),
    ("ff_version_meet_all", false),
    ("ff_version_min_ticks", false),
    ("ff_version_project", false),
    ("ff_version_rank", false),
    ("ff_version_tick", false),
];
