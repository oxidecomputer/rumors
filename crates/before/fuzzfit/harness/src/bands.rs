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
        slope: 1.076064,
        intercept: 1.888569,
        width_above: 0.115636,
        width_below: 0.124797,
        min_denom: 128,
        max_denom: 12160,
        samples: 1774,
        constant: false,
    },
    Band {
        kernel: "ff_clock_encode",
        rejected: false,
        slope: 0.325669,
        intercept: 2.128649,
        width_above: 0.210517,
        width_below: 0.237963,
        min_denom: 128,
        max_denom: 12155,
        samples: 1695,
        constant: false,
    },
    Band {
        kernel: "ff_clock_fork",
        rejected: false,
        slope: 0.768938,
        intercept: 1.588151,
        width_above: 0.354303,
        width_below: 0.304429,
        min_denom: 128,
        max_denom: 12356,
        samples: 92501,
        constant: false,
    },
    Band {
        kernel: "ff_clock_from_parts",
        rejected: false,
        slope: 0.000000,
        intercept: 2.431364,
        width_above: 0.000000,
        width_below: 0.000000,
        min_denom: 128,
        max_denom: 12151,
        samples: 1708,
        constant: false,
    },
    Band {
        kernel: "ff_clock_into_parts",
        rejected: false,
        slope: 0.000000,
        intercept: 2.546543,
        width_above: 0.019305,
        width_below: 0.000000,
        min_denom: 128,
        max_denom: 12364,
        samples: 32045,
        constant: false,
    },
    Band {
        kernel: "ff_clock_join",
        rejected: false,
        slope: 1.019724,
        intercept: 2.219037,
        width_above: 0.176510,
        width_below: 1.408795,
        min_denom: 128,
        max_denom: 24155,
        samples: 27149,
        constant: false,
    },
    Band {
        kernel: "ff_clock_join",
        rejected: true,
        slope: 0.993975,
        intercept: 1.210082,
        width_above: 0.008838,
        width_below: 0.012254,
        min_denom: 514,
        max_denom: 24310,
        samples: 270,
        constant: false,
    },
    Band {
        kernel: "ff_clock_own_version",
        rejected: false,
        slope: 1.064947,
        intercept: 2.023844,
        width_above: 0.113113,
        width_below: 0.168994,
        min_denom: 128,
        max_denom: 17153,
        samples: 1584,
        constant: false,
    },
    Band {
        kernel: "ff_clock_recv",
        rejected: false,
        slope: 1.050672,
        intercept: 2.604079,
        width_above: 0.211564,
        width_below: 0.400860,
        min_denom: 128,
        max_denom: 20860,
        samples: 7239,
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
        samples: 4096,
        constant: true,
    },
    Band {
        kernel: "ff_clock_send",
        rejected: false,
        slope: 1.080717,
        intercept: 2.550002,
        width_above: 0.163104,
        width_below: 0.431416,
        min_denom: 128,
        max_denom: 12148,
        samples: 5269,
        constant: false,
    },
    Band {
        kernel: "ff_clock_sync",
        rejected: false,
        slope: 1.014384,
        intercept: 2.253044,
        width_above: 0.147715,
        width_below: 1.508919,
        min_denom: 128,
        max_denom: 24524,
        samples: 3528,
        constant: false,
    },
    Band {
        kernel: "ff_clock_sync",
        rejected: true,
        slope: 0.994031,
        intercept: 1.081871,
        width_above: 0.021249,
        width_below: 0.013849,
        min_denom: 278,
        max_denom: 23106,
        samples: 270,
        constant: false,
    },
    Band {
        kernel: "ff_clock_tick",
        rejected: false,
        slope: 1.086247,
        intercept: 2.514576,
        width_above: 0.259343,
        width_below: 0.336534,
        min_denom: 128,
        max_denom: 12364,
        samples: 313086,
        constant: false,
    },
    Band {
        kernel: "ff_clock_version",
        rejected: false,
        slope: 0.000000,
        intercept: 2.397940,
        width_above: 0.188647,
        width_below: 0.000000,
        min_denom: 128,
        max_denom: 12369,
        samples: 16990,
        constant: false,
    },
    Band {
        kernel: "ff_party_covers",
        rejected: false,
        slope: 0.973132,
        intercept: 1.613910,
        width_above: 0.076811,
        width_below: 1.534395,
        min_denom: 128,
        max_denom: 10428,
        samples: 2157,
        constant: false,
    },
    Band {
        kernel: "ff_party_decode",
        rejected: false,
        slope: 0.996572,
        intercept: 2.030232,
        width_above: 0.026785,
        width_below: 0.041159,
        min_denom: 128,
        max_denom: 5216,
        samples: 1732,
        constant: false,
    },
    Band {
        kernel: "ff_party_encode",
        rejected: false,
        slope: 0.409179,
        intercept: 1.584159,
        width_above: 0.196079,
        width_below: 0.265550,
        min_denom: 128,
        max_denom: 5214,
        samples: 1574,
        constant: false,
    },
    Band {
        kernel: "ff_party_fork",
        rejected: false,
        slope: 0.893441,
        intercept: 1.778878,
        width_above: 0.067157,
        width_below: 0.025098,
        min_denom: 130,
        max_denom: 5120,
        samples: 527,
        constant: false,
    },
    Band {
        kernel: "ff_party_forks",
        rejected: false,
        slope: 0.860137,
        intercept: 2.815826,
        width_above: 0.092889,
        width_below: 0.164285,
        min_denom: 128,
        max_denom: 4760,
        samples: 2501,
        constant: false,
    },
    Band {
        kernel: "ff_party_is_disjoint",
        rejected: false,
        slope: 1.222692,
        intercept: 0.570859,
        width_above: 0.467471,
        width_below: 0.639068,
        min_denom: 128,
        max_denom: 10428,
        samples: 2157,
        constant: false,
    },
    Band {
        kernel: "ff_party_join",
        rejected: false,
        slope: 1.117775,
        intercept: 1.531321,
        width_above: 0.205259,
        width_below: 0.558601,
        min_denom: 128,
        max_denom: 8712,
        samples: 30427,
        constant: false,
    },
    Band {
        kernel: "ff_party_join",
        rejected: true,
        slope: 1.043261,
        intercept: 1.497116,
        width_above: 0.111649,
        width_below: 0.083818,
        min_denom: 172,
        max_denom: 10424,
        samples: 297,
        constant: false,
    },
    Band {
        kernel: "ff_party_seed",
        rejected: false,
        slope: 0.000000,
        intercept: 2.201397,
        width_above: 0.000000,
        width_below: 0.000000,
        min_denom: 8,
        max_denom: 8,
        samples: 521,
        constant: true,
    },
    Band {
        kernel: "ff_party_without",
        rejected: false,
        slope: 0.957014,
        intercept: 1.485454,
        width_above: 0.149290,
        width_below: 0.011291,
        min_denom: 128,
        max_denom: 4870,
        samples: 261,
        constant: false,
    },
    Band {
        kernel: "ff_party_without",
        rejected: true,
        slope: 0.988802,
        intercept: 2.358823,
        width_above: 0.011507,
        width_below: 0.006465,
        min_denom: 144,
        max_denom: 9980,
        samples: 270,
        constant: false,
    },
    Band {
        kernel: "ff_rank_add",
        rejected: false,
        slope: 0.321453,
        intercept: 2.431441,
        width_above: 0.176847,
        width_below: 0.412627,
        min_denom: 128,
        max_denom: 3952,
        samples: 660,
        constant: false,
    },
    Band {
        kernel: "ff_rank_checked_sub",
        rejected: false,
        slope: 0.629627,
        intercept: 1.512266,
        width_above: 0.451012,
        width_below: 0.514770,
        min_denom: 128,
        max_denom: 3872,
        samples: 989,
        constant: false,
    },
    Band {
        kernel: "ff_rank_checked_sub",
        rejected: true,
        slope: -0.015939,
        intercept: 2.410338,
        width_above: 0.456397,
        width_below: 0.060780,
        min_denom: 128,
        max_denom: 1984,
        samples: 117,
        constant: false,
    },
    Band {
        kernel: "ff_rank_cmp",
        rejected: false,
        slope: 0.720010,
        intercept: 1.177341,
        width_above: 0.143081,
        width_below: 1.424501,
        min_denom: 128,
        max_denom: 3872,
        samples: 553,
        constant: false,
    },
    Band {
        kernel: "ff_rank_display",
        rejected: false,
        slope: 0.925215,
        intercept: 1.343483,
        width_above: 0.081215,
        width_below: 0.093414,
        min_denom: 128,
        max_denom: 15640,
        samples: 1283,
        constant: false,
    },
    Band {
        kernel: "ff_version_cmp",
        rejected: false,
        slope: 1.137563,
        intercept: 1.692696,
        width_above: 0.270214,
        width_below: 0.873671,
        min_denom: 128,
        max_denom: 17714,
        samples: 1728,
        constant: false,
    },
    Band {
        kernel: "ff_version_concurrent",
        rejected: false,
        slope: 1.137051,
        intercept: 1.694608,
        width_above: 0.269347,
        width_below: 0.874433,
        min_denom: 128,
        max_denom: 17714,
        samples: 1728,
        constant: false,
    },
    Band {
        kernel: "ff_version_decode",
        rejected: false,
        slope: 1.136660,
        intercept: 1.716744,
        width_above: 0.230378,
        width_below: 0.358231,
        min_denom: 128,
        max_denom: 8656,
        samples: 3168,
        constant: false,
    },
    Band {
        kernel: "ff_version_distance",
        rejected: false,
        slope: 1.145757,
        intercept: 2.011130,
        width_above: 0.278517,
        width_below: 0.366289,
        min_denom: 128,
        max_denom: 8659,
        samples: 1644,
        constant: false,
    },
    Band {
        kernel: "ff_version_encode",
        rejected: false,
        slope: 0.436853,
        intercept: 1.477196,
        width_above: 0.217407,
        width_below: 0.254576,
        min_denom: 128,
        max_denom: 8652,
        samples: 3019,
        constant: false,
    },
    Band {
        kernel: "ff_version_join",
        rejected: false,
        slope: 1.015137,
        intercept: 2.373418,
        width_above: 0.197192,
        width_below: 2.538474,
        min_denom: 128,
        max_denom: 15239,
        samples: 5377,
        constant: false,
    },
    Band {
        kernel: "ff_version_join_all",
        rejected: false,
        slope: 1.151685,
        intercept: 2.459399,
        width_above: 0.337865,
        width_below: 2.165744,
        min_denom: 128,
        max_denom: 187950,
        samples: 4516,
        constant: false,
    },
    Band {
        kernel: "ff_version_lag",
        rejected: false,
        slope: 1.158292,
        intercept: 1.950143,
        width_above: 0.292904,
        width_below: 0.381231,
        min_denom: 128,
        max_denom: 8659,
        samples: 1644,
        constant: false,
    },
    Band {
        kernel: "ff_version_meet",
        rejected: false,
        slope: 1.038081,
        intercept: 2.257775,
        width_above: 0.246949,
        width_below: 2.557752,
        min_denom: 128,
        max_denom: 17157,
        samples: 2796,
        constant: false,
    },
    Band {
        kernel: "ff_version_meet_all",
        rejected: false,
        slope: 1.084694,
        intercept: 2.286868,
        width_above: 0.216397,
        width_below: 1.912347,
        min_denom: 128,
        max_denom: 183640,
        samples: 1536,
        constant: false,
    },
    Band {
        kernel: "ff_version_min_ticks",
        rejected: false,
        slope: 1.066896,
        intercept: 2.455848,
        width_above: 0.232424,
        width_below: 0.418711,
        min_denom: 128,
        max_denom: 8857,
        samples: 6183,
        constant: false,
    },
    Band {
        kernel: "ff_version_project",
        rejected: false,
        slope: 0.884312,
        intercept: 2.500724,
        width_above: 0.525965,
        width_below: 0.368368,
        min_denom: 128,
        max_denom: 53022,
        samples: 4222,
        constant: false,
    },
    Band {
        kernel: "ff_version_rank",
        rejected: false,
        slope: 1.134658,
        intercept: 2.024703,
        width_above: 0.303756,
        width_below: 0.418327,
        min_denom: 128,
        max_denom: 8857,
        samples: 6060,
        constant: false,
    },
    Band {
        kernel: "ff_version_tick",
        rejected: false,
        slope: 0.978597,
        intercept: 2.763407,
        width_above: 0.231054,
        width_below: 0.272187,
        min_denom: 128,
        max_denom: 13827,
        samples: 2554,
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
        intercept: 4.239656,
        width_above: 0.813015,
        width_below: 0.434019,
        min_denom: 10,
        max_denom: 127,
        samples: 1254302,
        constant: true,
    },
    Band {
        kernel: "ff_clock_join",
        rejected: false,
        slope: 0.000000,
        intercept: 4.159633,
        width_above: 0.342412,
        width_below: 0.870713,
        min_denom: 20,
        max_denom: 127,
        samples: 4162,
        constant: true,
    },
    Band {
        kernel: "ff_clock_encode",
        rejected: false,
        slope: 0.000000,
        intercept: 2.726484,
        width_above: 0.302494,
        width_below: 0.136535,
        min_denom: 10,
        max_denom: 127,
        samples: 6448,
        constant: true,
    },
    Band {
        kernel: "ff_clock_decode",
        rejected: false,
        slope: 0.000000,
        intercept: 3.639056,
        width_above: 0.580869,
        width_below: 0.313130,
        min_denom: 16,
        max_denom: 120,
        samples: 6361,
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
