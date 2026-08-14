//! The suanpan consumption roster: every property this crate assumes
//! across the accumulator seam, with its consumption sites and the
//! committed witnesses that hold it.
//!
//! Goal: any `suanpan` change that falsifies a property this crate
//! assumes trips a named, committed witness here, instead of silently
//! voiding a guard comment in the skyline watermark or the adequacy of
//! a meter fixture's closed form. The accumulator's public rustdoc is
//! the contract of record; each row below restates, in plain language,
//! the one clause the named consuming code leans on, and cites the
//! witness tests that drive [`suanpan::Accumulator`] through its public
//! API and read that clause back.
//!
//! Mechanism, mirroring suanpan's own claims roster (its
//! `src/claims.rs`): the rows are data, and the binding tests
//! (`consumption/tests.rs`) hold them:
//!
//! - **Row ↔ witness, both directions**: every cited witness exists as
//!   a `#[test]` in its file, and every `#[test]` in the witness file
//!   is cited by some row — a witness can neither rot away under a
//!   live citation nor drift in uncited.
//! - **Row ↔ site**: every consumption site names a live file and an
//!   anchor token it still contains, so a refactor that moves or
//!   renames the consuming code turns the row red instead of
//!   orphaning its prose.
//!
//! The witnesses (`consumption/witnesses.rs`) re-derive every constant
//! from suanpan's public rustdoc — never from this crate's fixture
//! comments — and each carries its adequacy leg in the same test: the
//! known-bad counterpart (clearance one digit short, a top digit below
//! the decision bound, the sign read omitted) failing or reading
//! undecided, so no witness passes vacuously. The touch-metered legs
//! ride the `limb-meter` feature (which lights `suanpan/touch-meter`);
//! the semantic legs run in every test build.

mod tests;
mod witnesses;

/// One consumed property: what this crate assumes of the accumulator,
/// where it assumes it, and the tests that hold the assumption.
pub(crate) struct Consumed {
    /// The property, in plain language, as the consuming code relies
    /// on it.
    pub(crate) property: &'static str,
    /// `(manifest-relative file, anchor)` pairs naming the consuming
    /// code: each anchor is a token the file must contain — an item
    /// name, never a line number.
    pub(crate) sites: &'static [(&'static str, &'static str)],
    /// `(manifest-relative file, #[test] fn name)` pairs, each
    /// required to exist as an attributed test in its file.
    pub(crate) witnesses: &'static [(&'static str, &'static str)],
}

/// The witness file of record: every roster row's tests live here, and
/// the binding holds that file total — each of its `#[test]` fns cited
/// by some row.
pub(crate) const WITNESSES: &str = "src/consumption/witnesses.rs";

/// The skyline watermark, whose latent ladder and undercut propagation
/// consume the sign-read and domination contracts.
const WATERMARK: &str = "src/version/skyline/watermark.rs";

/// The integral sweep, whose segment pricing consumes the scaled-read
/// contract.
const INTEGRAL: &str = "src/version/skyline/query/integral.rs";

/// The consumption roster of record: one row per property this crate
/// assumes across the accumulator seam.
///
/// The binding tests hold every cited witness alive by name, the
/// witness file total over the rows, and every site anchored to live
/// code.
pub(crate) const ROSTER: &[Consumed] = &[
    Consumed {
        property: "After a sign read, digit_count reports the collapsed top: the read folds \
                   a cancelling prefix down and re-deposits it, so a domination floor \
                   derived from the count immediately after rests on an honest top.",
        sites: &[(WATERMARK, "fn decide_undercut_through_latent(")],
        witnesses: &[(
            WITNESSES,
            "sign_collapse_tightens_the_top_and_arms_domination",
        )],
    },
    Consumed {
        property: "sign_dominates_at decides with a decision-bound top exactly two digit \
                   indexes above the floor, and refuses one digit short even with that \
                   top: a guard that skips domination reads under two digits of clearance \
                   skips only reads that could never decide.",
        sites: &[(WATERMARK, "fn propagate(")],
        witnesses: &[(
            WITNESSES,
            "domination_clearance_two_digits_suffice_and_one_short_refuses",
        )],
    },
    Consumed {
        property: "A collapsing sign read can re-deposit its partial below the written \
                   span and lower sign_magnitude_shl's returned shift, so the integral \
                   sweep reads segment mass with no prior sign read and opens each new \
                   segment by buffer replacement.",
        sites: &[
            (INTEGRAL, "fn settle_segment("),
            (INTEGRAL, "fn settle("),
            (INTEGRAL, "fn freeze("),
        ],
        witnesses: &[(
            WITNESSES,
            "collapsing_sign_read_lowers_the_scaled_read_shift",
        )],
    },
    Consumed {
        property: "Magnitude::to_word on Base answers the accumulator's width dispatch at \
                   word scale, zero digit touches for word-held and spilled magnitudes \
                   both: the O(1) dispatch read the small path's cost accounting assumes \
                   free.",
        sites: &[("src/codec/base.rs", "impl suanpan::Magnitude for Base")],
        witnesses: &[(WITNESSES, "base_dispatch_read_touches_no_digits")],
    },
    Consumed {
        property: "A magnitude with top digit 5 at index i decides sign_dominates_at(i - 2) \
                   on its first digit touch, so the seam shapes the meter suite constructs \
                   reach the domination arms their closed forms claim.",
        sites: &[
            ("tests/meter.rs", "fn seam_plunge_ticks("),
            ("tests/meter.rs", "SEAM_CLEARANCE"),
        ],
        witnesses: &[(WITNESSES, "decision_bound_top_decides_on_the_first_touch")],
    },
];
