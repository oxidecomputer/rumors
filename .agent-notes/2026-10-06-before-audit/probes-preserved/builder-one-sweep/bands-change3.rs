//! Committed fuel bounds for the operations in [`crate::ops`].
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
//! their narrow ranges. Other sub-floor samples receive no pointwise fuel
//! judgment. A high result flags extra work; a low result flags a
//! dead meter or an unexpectedly constant implementation. [`crate::fit`]
//! explains how the asymmetric widths are fitted.
//!
//! [`crate::ops::Step::identity`] explains which equality shortcuts are excluded
//! from the fits. The vocabulary covers a subset of the public API; it does not
//! measure malformed decoding or empty folds. See [`crate::strategies`] for
//! the generated inputs and their limits.
//!
//! The pin uses the release Wasm guest, the toolchain in [`PINNED_RUSTC`], and
//! Wasmtime fuel. Calibration reports the fixed reach programs' ceiling excess,
//! the measured floors' distance from a no-op, and prefix-refit divergence.
//! Review these measurements against the enforcement tolerances when re-pinning.

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
    /// Whether the samples had too little size variation to fit a slope.
    /// Such bands use slope zero; a fitted zero slope need not have this
    /// classification.
    pub constant: bool,
}

/// Slack beyond each band's fitted ceiling, in `log₁₀` units.
///
/// This allows differences between calibration and enforcement contexts.
/// Calibration reports the fixed reach programs' ceiling excess for comparison
/// with this margin.
pub const ENFORCE_MARGIN: f64 = 0.2;

/// Slack beyond each band's fitted floor, in `log₁₀` units.
///
/// The floor distinguishes a live measurement from a no-op while allowing
/// fresh programs to be cheaper than the calibration corpus. Calibration
/// reports the narrowest main-band floor's distance from a no-op after this
/// margin is subtracted; review that distance when re-pinning.
pub const ENFORCE_MARGIN_BELOW: f64 = 0.8;

/// Programs in the deterministic prefix used to check each main band's fit.
///
/// Reusing a prefix keeps the check smaller than a full calibration sweep.
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
        slope: 1.075872,
        intercept: 1.896938,
        width_above: 0.115830,
        width_below: 0.124898,
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
        slope: 0.769558,
        intercept: 1.585566,
        width_above: 0.354630,
        width_below: 0.304701,
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
        width_below: -0.000000,
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
        width_below: -0.000000,
        min_denom: 128,
        max_denom: 12364,
        samples: 32045,
        constant: false,
    },
    Band {
        kernel: "ff_clock_join",
        rejected: false,
        slope: 1.018846,
        intercept: 2.229326,
        width_above: 0.175866,
        width_below: 1.416818,
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
        slope: 1.062882,
        intercept: 2.035724,
        width_above: 0.110364,
        width_below: 0.159268,
        min_denom: 128,
        max_denom: 17153,
        samples: 1584,
        constant: false,
    },
    Band {
        kernel: "ff_clock_recv",
        rejected: false,
        slope: 1.056191,
        intercept: 2.609612,
        width_above: 0.230199,
        width_below: 0.402645,
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
        slope: 1.086108,
        intercept: 2.566995,
        width_above: 0.179514,
        width_below: 0.431628,
        min_denom: 128,
        max_denom: 12148,
        samples: 5269,
        constant: false,
    },
    Band {
        kernel: "ff_clock_sync",
        rejected: false,
        slope: 1.013631,
        intercept: 2.263609,
        width_above: 0.147302,
        width_below: 1.518513,
        min_denom: 128,
        max_denom: 24524,
        samples: 3528,
        constant: false,
    },
    Band {
        kernel: "ff_clock_sync",
        rejected: true,
        slope: 0.994077,
        intercept: 1.081755,
        width_above: 0.021239,
        width_below: 0.013857,
        min_denom: 278,
        max_denom: 23106,
        samples: 270,
        constant: false,
    },
    Band {
        kernel: "ff_clock_tick",
        rejected: false,
        slope: 1.100816,
        intercept: 2.496118,
        width_above: 0.250542,
        width_below: 0.317320,
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
        width_below: -0.000000,
        min_denom: 128,
        max_denom: 12369,
        samples: 16990,
        constant: false,
    },
    Band {
        kernel: "ff_party_covers",
        rejected: false,
        slope: 0.973073,
        intercept: 1.614058,
        width_above: 0.076851,
        width_below: 1.534391,
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
        slope: 0.893779,
        intercept: 1.777679,
        width_above: 0.067001,
        width_below: 0.025037,
        min_denom: 130,
        max_denom: 5120,
        samples: 527,
        constant: false,
    },
    Band {
        kernel: "ff_party_forks",
        rejected: false,
        slope: 0.861584,
        intercept: 2.810874,
        width_above: 0.092322,
        width_below: 0.163003,
        min_denom: 128,
        max_denom: 4760,
        samples: 2501,
        constant: false,
    },
    Band {
        kernel: "ff_party_is_disjoint",
        rejected: false,
        slope: 1.222785,
        intercept: 0.570627,
        width_above: 0.467507,
        width_below: 0.639085,
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
        slope: 0.988900,
        intercept: 2.358466,
        width_above: 0.011523,
        width_below: 0.006462,
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
        slope: 0.925683,
        intercept: 1.342260,
        width_above: 0.081396,
        width_below: 0.093625,
        min_denom: 128,
        max_denom: 15640,
        samples: 1283,
        constant: false,
    },
    Band {
        kernel: "ff_version_cmp",
        rejected: false,
        slope: 1.137973,
        intercept: 1.700301,
        width_above: 0.264816,
        width_below: 0.867099,
        min_denom: 128,
        max_denom: 17714,
        samples: 1728,
        constant: false,
    },
    Band {
        kernel: "ff_version_concurrent",
        rejected: false,
        slope: 1.137823,
        intercept: 1.700860,
        width_above: 0.264547,
        width_below: 0.867456,
        min_denom: 128,
        max_denom: 17714,
        samples: 1728,
        constant: false,
    },
    Band {
        kernel: "ff_version_decode",
        rejected: false,
        slope: 1.136454,
        intercept: 1.726818,
        width_above: 0.223509,
        width_below: 0.360033,
        min_denom: 128,
        max_denom: 8656,
        samples: 3168,
        constant: false,
    },
    Band {
        kernel: "ff_version_distance",
        rejected: false,
        slope: 1.150104,
        intercept: 2.004747,
        width_above: 0.281352,
        width_below: 0.366308,
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
        slope: 1.015200,
        intercept: 2.381110,
        width_above: 0.196737,
        width_below: 2.546337,
        min_denom: 128,
        max_denom: 15239,
        samples: 5377,
        constant: false,
    },
    Band {
        kernel: "ff_version_join_all",
        rejected: false,
        slope: 1.150115,
        intercept: 2.471315,
        width_above: 0.334299,
        width_below: 2.173586,
        min_denom: 128,
        max_denom: 187950,
        samples: 4516,
        constant: false,
    },
    Band {
        kernel: "ff_version_lag",
        rejected: false,
        slope: 1.161923,
        intercept: 1.951384,
        width_above: 0.292904,
        width_below: 0.379972,
        min_denom: 128,
        max_denom: 8659,
        samples: 1644,
        constant: false,
    },
    Band {
        kernel: "ff_version_meet",
        rejected: false,
        slope: 1.037924,
        intercept: 2.265532,
        width_above: 0.245608,
        width_below: 2.565081,
        min_denom: 128,
        max_denom: 17157,
        samples: 2796,
        constant: false,
    },
    Band {
        kernel: "ff_version_meet_all",
        rejected: false,
        slope: 1.083349,
        intercept: 2.298895,
        width_above: 0.223097,
        width_below: 1.920767,
        min_denom: 128,
        max_denom: 183640,
        samples: 1536,
        constant: false,
    },
    Band {
        kernel: "ff_version_min_ticks",
        rejected: false,
        slope: 1.084554,
        intercept: 2.453523,
        width_above: 0.231137,
        width_below: 0.419560,
        min_denom: 128,
        max_denom: 8857,
        samples: 6183,
        constant: false,
    },
    Band {
        kernel: "ff_version_project",
        rejected: false,
        slope: 0.888950,
        intercept: 2.492795,
        width_above: 0.526869,
        width_below: 0.362255,
        min_denom: 128,
        max_denom: 53022,
        samples: 4222,
        constant: false,
    },
    Band {
        kernel: "ff_version_rank",
        rejected: false,
        slope: 1.130049,
        intercept: 2.042893,
        width_above: 0.304181,
        width_below: 0.416485,
        min_denom: 128,
        max_denom: 8857,
        samples: 6060,
        constant: false,
    },
    Band {
        kernel: "ff_version_tick",
        rejected: false,
        slope: 1.005371,
        intercept: 2.713095,
        width_above: 0.223349,
        width_below: 0.271688,
        min_denom: 128,
        max_denom: 13827,
        samples: 2554,
        constant: false,
    },
];

/// Bootstrap success paths below their main bands' floors, selected from the
/// executed bootstrap corpus and judged over each band's calibrated span.
///
/// Generated by `just fuzzfit-calibrate` alongside [`BANDS`], from the
/// pooled sub-floor samples of the calibration corpus and the
/// deterministic bootstrap stream — review the diff like a snapshot,
/// commit with a dated movement annotation.
pub const SMALL_BANDS: &[Band] = &[
    Band {
        kernel: "ff_clock_decode",
        rejected: false,
        slope: 0.000000,
        intercept: 3.647260,
        width_above: 0.577377,
        width_below: 0.311401,
        min_denom: 16,
        max_denom: 120,
        samples: 6361,
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
        kernel: "ff_clock_fork",
        rejected: false,
        slope: 0.000000,
        intercept: 3.297508,
        width_above: 0.251372,
        width_below: 0.219414,
        min_denom: 10,
        max_denom: 127,
        samples: 207147,
        constant: true,
    },
    Band {
        kernel: "ff_clock_join",
        rejected: false,
        slope: 0.000000,
        intercept: 4.172079,
        width_above: 0.339109,
        width_below: 0.883159,
        min_denom: 20,
        max_denom: 127,
        samples: 4162,
        constant: true,
    },
    Band {
        kernel: "ff_clock_tick",
        rejected: false,
        slope: 0.000000,
        intercept: 4.251332,
        width_above: 0.804719,
        width_below: 0.431788,
        min_denom: 10,
        max_denom: 127,
        samples: 1254302,
        constant: true,
    },
];

