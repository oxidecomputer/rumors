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
//! The pin uses the Wasm guest built in the root manifest's `fuel` profile,
//! the toolchain in [`PINNED_RUSTC`], and Wasmtime fuel. Calibration reports
//! the fixed reach programs' ceiling excess, the measured floors' distance
//! from a no-op, and prefix-refit divergence. Review these measurements
//! against the enforcement tolerances when re-pinning.

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
        slope: 1.075099,
        intercept: 1.895135,
        width_above: 0.115532,
        width_below: 0.123867,
        min_denom: 128,
        max_denom: 12160,
        samples: 1774,
        constant: false,
    },
    Band {
        kernel: "ff_clock_encode",
        rejected: false,
        slope: 0.387705,
        intercept: 1.669932,
        width_above: 0.273973,
        width_below: 0.230699,
        min_denom: 128,
        max_denom: 12155,
        samples: 1695,
        constant: false,
    },
    Band {
        kernel: "ff_clock_fork",
        rejected: false,
        slope: 0.693836,
        intercept: 1.844125,
        width_above: 0.306775,
        width_below: 0.288910,
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
        slope: 1.019037,
        intercept: 2.227702,
        width_above: 0.181320,
        width_below: 1.399921,
        min_denom: 128,
        max_denom: 24155,
        samples: 27149,
        constant: false,
    },
    Band {
        kernel: "ff_clock_join",
        rejected: true,
        slope: 0.993939,
        intercept: 1.221853,
        width_above: 0.010743,
        width_below: 0.010846,
        min_denom: 514,
        max_denom: 24310,
        samples: 270,
        constant: false,
    },
    Band {
        kernel: "ff_clock_own_version",
        rejected: false,
        slope: 1.060923,
        intercept: 2.042976,
        width_above: 0.102767,
        width_below: 0.172263,
        min_denom: 128,
        max_denom: 17153,
        samples: 1584,
        constant: false,
    },
    Band {
        kernel: "ff_clock_recv",
        rejected: false,
        slope: 1.047469,
        intercept: 2.639297,
        width_above: 0.239830,
        width_below: 0.405457,
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
        slope: 1.077846,
        intercept: 2.593035,
        width_above: 0.205869,
        width_below: 0.430836,
        min_denom: 128,
        max_denom: 12148,
        samples: 5269,
        constant: false,
    },
    Band {
        kernel: "ff_clock_sync",
        rejected: false,
        slope: 1.008866,
        intercept: 2.280137,
        width_above: 0.146671,
        width_below: 1.520440,
        min_denom: 128,
        max_denom: 24524,
        samples: 3528,
        constant: false,
    },
    Band {
        kernel: "ff_clock_sync",
        rejected: true,
        slope: 0.995329,
        intercept: 1.076426,
        width_above: 0.021635,
        width_below: 0.011971,
        min_denom: 278,
        max_denom: 23106,
        samples: 270,
        constant: false,
    },
    Band {
        kernel: "ff_clock_tick",
        rejected: false,
        slope: 1.089217,
        intercept: 2.536044,
        width_above: 0.248442,
        width_below: 0.317113,
        min_denom: 128,
        max_denom: 12364,
        samples: 313086,
        constant: false,
    },
    Band {
        kernel: "ff_clock_version",
        rejected: false,
        slope: -0.080896,
        intercept: 2.699498,
        width_above: 0.250036,
        width_below: 0.131093,
        min_denom: 128,
        max_denom: 12369,
        samples: 16990,
        constant: false,
    },
    Band {
        kernel: "ff_party_covers",
        rejected: false,
        slope: 0.973118,
        intercept: 1.613884,
        width_above: 0.076848,
        width_below: 1.534332,
        min_denom: 128,
        max_denom: 10428,
        samples: 2157,
        constant: false,
    },
    Band {
        kernel: "ff_party_decode",
        rejected: false,
        slope: 0.998086,
        intercept: 2.036998,
        width_above: 0.028533,
        width_below: 0.042321,
        min_denom: 128,
        max_denom: 5216,
        samples: 1732,
        constant: false,
    },
    Band {
        kernel: "ff_party_encode",
        rejected: false,
        slope: 0.399463,
        intercept: 1.599720,
        width_above: 0.193005,
        width_below: 0.253098,
        min_denom: 128,
        max_denom: 5214,
        samples: 1574,
        constant: false,
    },
    Band {
        kernel: "ff_party_fork",
        rejected: false,
        slope: 0.855583,
        intercept: 1.917486,
        width_above: 0.075419,
        width_below: 0.032837,
        min_denom: 130,
        max_denom: 5120,
        samples: 527,
        constant: false,
    },
    Band {
        kernel: "ff_party_forks",
        rejected: false,
        slope: 0.865463,
        intercept: 2.798340,
        width_above: 0.090371,
        width_below: 0.157439,
        min_denom: 128,
        max_denom: 4760,
        samples: 2501,
        constant: false,
    },
    Band {
        kernel: "ff_party_is_disjoint",
        rejected: false,
        slope: 1.222278,
        intercept: 0.572581,
        width_above: 0.467237,
        width_below: 0.638118,
        min_denom: 128,
        max_denom: 10428,
        samples: 2157,
        constant: false,
    },
    Band {
        kernel: "ff_party_join",
        rejected: false,
        slope: 1.103229,
        intercept: 1.590002,
        width_above: 0.192081,
        width_below: 0.555806,
        min_denom: 128,
        max_denom: 8712,
        samples: 30427,
        constant: false,
    },
    Band {
        kernel: "ff_party_join",
        rejected: true,
        slope: 1.034463,
        intercept: 1.539313,
        width_above: 0.106347,
        width_below: 0.081923,
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
        slope: 0.955639,
        intercept: 1.490290,
        width_above: 0.149576,
        width_below: 0.011579,
        min_denom: 128,
        max_denom: 4870,
        samples: 261,
        constant: false,
    },
    Band {
        kernel: "ff_party_without",
        rejected: true,
        slope: 0.988492,
        intercept: 2.364298,
        width_above: 0.012610,
        width_below: 0.006687,
        min_denom: 144,
        max_denom: 9980,
        samples: 270,
        constant: false,
    },
    Band {
        kernel: "ff_rank_add",
        rejected: false,
        slope: 0.311320,
        intercept: 2.457350,
        width_above: 0.179096,
        width_below: 0.410589,
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
        width_above: 0.445182,
        width_below: 0.520302,
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
        slope: 0.923877,
        intercept: 1.350041,
        width_above: 0.081327,
        width_below: 0.085804,
        min_denom: 128,
        max_denom: 15640,
        samples: 1283,
        constant: false,
    },
    Band {
        kernel: "ff_version_cmp",
        rejected: false,
        slope: 1.135015,
        intercept: 1.703735,
        width_above: 0.277390,
        width_below: 0.861397,
        min_denom: 128,
        max_denom: 17714,
        samples: 1728,
        constant: false,
    },
    Band {
        kernel: "ff_version_concurrent",
        rejected: false,
        slope: 1.135111,
        intercept: 1.703362,
        width_above: 0.277561,
        width_below: 0.861468,
        min_denom: 128,
        max_denom: 17714,
        samples: 1728,
        constant: false,
    },
    Band {
        kernel: "ff_version_decode",
        rejected: false,
        slope: 1.136321,
        intercept: 1.719957,
        width_above: 0.238449,
        width_below: 0.357959,
        min_denom: 128,
        max_denom: 8656,
        samples: 3168,
        constant: false,
    },
    Band {
        kernel: "ff_version_distance",
        rejected: false,
        slope: 1.146732,
        intercept: 2.014257,
        width_above: 0.287578,
        width_below: 0.369697,
        min_denom: 128,
        max_denom: 8659,
        samples: 1644,
        constant: false,
    },
    Band {
        kernel: "ff_version_encode",
        rejected: false,
        slope: 0.404460,
        intercept: 1.561444,
        width_above: 0.197102,
        width_below: 0.240981,
        min_denom: 128,
        max_denom: 8652,
        samples: 3019,
        constant: false,
    },
    Band {
        kernel: "ff_version_join",
        rejected: false,
        slope: 1.009448,
        intercept: 2.401425,
        width_above: 0.190218,
        width_below: 2.464547,
        min_denom: 128,
        max_denom: 15239,
        samples: 5377,
        constant: false,
    },
    Band {
        kernel: "ff_version_join_all",
        rejected: false,
        slope: 1.150476,
        intercept: 2.475649,
        width_above: 0.338889,
        width_below: 2.189399,
        min_denom: 128,
        max_denom: 187950,
        samples: 4516,
        constant: false,
    },
    Band {
        kernel: "ff_version_lag",
        rejected: false,
        slope: 1.158398,
        intercept: 1.959832,
        width_above: 0.301450,
        width_below: 0.377015,
        min_denom: 128,
        max_denom: 8659,
        samples: 1644,
        constant: false,
    },
    Band {
        kernel: "ff_version_meet",
        rejected: false,
        slope: 1.036228,
        intercept: 2.272595,
        width_above: 0.244390,
        width_below: 2.523321,
        min_denom: 128,
        max_denom: 17157,
        samples: 2796,
        constant: false,
    },
    Band {
        kernel: "ff_version_meet_all",
        rejected: false,
        slope: 1.083014,
        intercept: 2.303868,
        width_above: 0.216274,
        width_below: 1.935385,
        min_denom: 128,
        max_denom: 183640,
        samples: 1536,
        constant: false,
    },
    Band {
        kernel: "ff_version_min_ticks",
        rejected: false,
        slope: 1.089189,
        intercept: 2.534172,
        width_above: 0.240747,
        width_below: 0.428722,
        min_denom: 128,
        max_denom: 8857,
        samples: 6183,
        constant: false,
    },
    Band {
        kernel: "ff_version_project",
        rejected: false,
        slope: 0.895368,
        intercept: 2.474670,
        width_above: 0.545415,
        width_below: 0.367180,
        min_denom: 128,
        max_denom: 53022,
        samples: 4222,
        constant: false,
    },
    Band {
        kernel: "ff_version_rank",
        rejected: false,
        slope: 1.128587,
        intercept: 2.050651,
        width_above: 0.310915,
        width_below: 0.415157,
        min_denom: 128,
        max_denom: 8857,
        samples: 6060,
        constant: false,
    },
    Band {
        kernel: "ff_version_tick",
        rejected: false,
        slope: 1.007089,
        intercept: 2.720014,
        width_above: 0.231934,
        width_below: 0.279590,
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
        intercept: 3.649854,
        width_above: 0.577750,
        width_below: 0.310801,
        min_denom: 16,
        max_denom: 120,
        samples: 6361,
        constant: true,
    },
    Band {
        kernel: "ff_clock_encode",
        rejected: false,
        slope: 0.000000,
        intercept: 2.584809,
        width_above: 0.141103,
        width_below: 0.137651,
        min_denom: 10,
        max_denom: 127,
        samples: 6448,
        constant: true,
    },
    Band {
        kernel: "ff_clock_fork",
        rejected: false,
        slope: 0.000000,
        intercept: 3.373172,
        width_above: 0.240776,
        width_below: 0.192759,
        min_denom: 10,
        max_denom: 127,
        samples: 207147,
        constant: true,
    },
    Band {
        kernel: "ff_clock_join",
        rejected: false,
        slope: 0.000000,
        intercept: 4.169904,
        width_above: 0.353712,
        width_below: 0.843978,
        min_denom: 20,
        max_denom: 127,
        samples: 4162,
        constant: true,
    },
    Band {
        kernel: "ff_clock_tick",
        rejected: false,
        slope: 0.000000,
        intercept: 4.254692,
        width_above: 0.827289,
        width_below: 0.422055,
        min_denom: 10,
        max_denom: 127,
        samples: 1254302,
        constant: true,
    },
];
