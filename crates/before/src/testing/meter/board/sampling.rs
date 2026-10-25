//! Where each sweep of the measurement ladder sets the families' size knobs.
//!
//! Every family builds its operands from size knobs, each a base size times
//! a sampling scale, at the sampled size and its double. A scaled ladder
//! keeps each knob's position within a doubling period fixed at every
//! sampled size, because a doubling preserves that position. A buffer that
//! grows by doubling holds its peak heap, the old storage beside the new,
//! just past a power of two, so a scaled ladder whose knobs sit elsewhere in
//! the period never samples that peak. [`Sampling::PastPowerOfTwo`] places
//! every knob just past a power of two instead.
//!
//! The past-power sample is judged against the ceilings and floors alone,
//! like the small inputs, and never enters a growth fit: several families
//! align their knobs so that every growth sample compares like with like,
//! and this sample deliberately breaks that alignment.
//!
//! # What the sample reaches
//!
//! The sample places the knobs, not the buffers. A buffer whose length is
//! the knob plus or minus a little, such as a per-level stack, lands just
//! past its doubling. A byte buffer lands there only where a family's bytes
//! grow by a power-of-two factor of its knob, as a spine's four bits per
//! level do; elsewhere it sits at whatever point of its period that factor
//! puts it. A family whose generator rounds its knob up to a power of two
//! gets the next power of two instead, so its past-power sample repeats one
//! of its growth sizes.

#[cfg(test)]
mod tests;

/// How far above a power of two [`Sampling::PastPowerOfTwo`] places a knob.
///
/// Two rather than one: an even knob stays even, so the families whose
/// generators take even counts keep the sample past the power, and a buffer
/// one element shorter than its knob still lands past the power too.
pub const PAST_POWER_OFFSET: usize = 2;

/// Where one sweep sets every family's size knobs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sampling {
    /// Each knob at its base size times this positive scale, rounded, and no
    /// smaller than the board's smallest knob.
    Scaled(f64),
    /// Each knob at [`PAST_POWER_OFFSET`] above the power of two at or above
    /// its base size, so the doubled sample also sits just past a power of
    /// two.
    PastPowerOfTwo,
}

impl Sampling {
    /// Returns the knob for a family's `base` size at this sampling, before
    /// the sample's doubling; never below `min`, the board's smallest knob.
    ///
    /// # Panics
    ///
    /// Panics if the knob does not fit a `usize`.
    pub(super) fn knob(self, base: usize, min: usize) -> usize {
        match self {
            Sampling::Scaled(scale) => {
                let scaled = ((base as f64) * scale).round();
                assert!(
                    scaled < usize::MAX as f64,
                    "amp-board: a scaled knob must fit a usize"
                );
                (scaled as usize).max(min)
            }
            Sampling::PastPowerOfTwo => base
                .max(min)
                .checked_next_power_of_two()
                .and_then(|power| power.checked_add(PAST_POWER_OFFSET))
                .expect("amp-board: a past-power knob must fit a usize"),
        }
    }

    /// Asserts that the sampling can build every family.
    ///
    /// # Panics
    ///
    /// Panics unless a scale is strictly positive and finite.
    pub(super) fn assert_valid(self) {
        if let Sampling::Scaled(scale) = self {
            assert!(
                scale > 0.0 && scale.is_finite(),
                "amp-board: scale must be a positive finite number"
            );
        }
    }

    /// Returns the sampling as one token of the shard protocol, which
    /// [`Sampling::from_token`] reads back exactly.
    ///
    /// A scale travels as its IEEE-754 bit pattern, so no decimal rounding
    /// can move it.
    pub fn token(self) -> String {
        match self {
            Sampling::Scaled(scale) => format!("scale:{:016x}", scale.to_bits()),
            Sampling::PastPowerOfTwo => "past-power-of-two".to_owned(),
        }
    }

    /// Reads a token [`Sampling::token`] wrote, or returns `None` if `token`
    /// is not one.
    pub fn from_token(token: &str) -> Option<Sampling> {
        if token == "past-power-of-two" {
            return Some(Sampling::PastPowerOfTwo);
        }
        let bits = token.strip_prefix("scale:")?;
        if bits.len() != 16 {
            return None;
        }
        u64::from_str_radix(bits, 16)
            .ok()
            .map(|bits| Sampling::Scaled(f64::from_bits(bits)))
    }

    /// Returns the sampling as the fuel board's guest export takes it: a tag
    /// and a word, which [`Sampling::from_words`] reads back exactly.
    ///
    /// The pair is tag 0 with a scale's IEEE-754 bit pattern, or tag 1 with
    /// zero for the past-power sampling.
    pub fn to_words(self) -> (u32, u64) {
        match self {
            Sampling::Scaled(scale) => (0, scale.to_bits()),
            Sampling::PastPowerOfTwo => (1, 0),
        }
    }

    /// Reads a tag and word [`Sampling::to_words`] wrote, or returns `None`
    /// for any other pair.
    pub fn from_words(tag: u32, word: u64) -> Option<Sampling> {
        match (tag, word) {
            (0, bits) => Some(Sampling::Scaled(f64::from_bits(bits))),
            (1, 0) => Some(Sampling::PastPowerOfTwo),
            _ => None,
        }
    }
}

/// Renders a scale as its number and the past-power sampling as `pow2+2`:
/// the label of the sampling in captures and matrix headers.
impl std::fmt::Display for Sampling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sampling::Scaled(scale) => write!(f, "{scale}"),
            Sampling::PastPowerOfTwo => write!(f, "pow2+{PAST_POWER_OFFSET}"),
        }
    }
}
