//! Message sizes: a lognormal distribution set by its mean and coefficient of
//! variation, with a cap.
//!
//! A message's *size* is the bytes its CBOR encoding takes: the byte string's
//! head and its contents. That is the quantity the library charges and sends,
//! so it is the one a user sizing a deployment thinks in.
//!
//! # Why a lognormal, set by mean and CV
//!
//! Two properties of a size distribution move a session's cost, and the two
//! parameters control them separately. The time a large session takes grows
//! with the total bytes it moves, so with the *mean*. Memory and admission
//! depend on the *tail*: a supply is charged its message's size, and the
//! largest messages must fit. The coefficient of variation (standard deviation
//! divided by mean, written CV) sets how heavy that tail is while the mean
//! stays put. A CV of zero is a fixed size: every message has exactly the mean.
//!
//! In log space a lognormal with mean `m` and coefficient of variation `c` has
//! `σ² = ln(1 + c²)` and `μ = ln(m) − σ²/2`.
//!
//! # The cap, and what a redraw is
//!
//! Draws larger than the cap are *redrawn*, never clamped: clamping would pile
//! probability onto the cap and change the tail's shape, while redrawing keeps
//! the distribution's shape below the cap (it is the lognormal conditioned on
//! not exceeding it). A draw that rounds to no bytes at all is redrawn the same
//! way, since the smallest message, an empty byte string, encodes in one byte.
//! Each session records how many draws it redrew, so a run whose cap cuts deep
//! into the tail says so.
//!
//! The default cap is [`DEFAULT_MAX_MESSAGE_SIZE`]; its documentation gives the
//! reason for its value.
//!
//! # Sizes no message can have
//!
//! A CBOR byte string's head grows from one byte to two at 24 content bytes, to
//! three at 256, and to five at 65,536, so a few sizes have no encoding: 25,
//! 258 and 259, and 65,539 to 65,540. A draw that lands on one becomes the
//! largest size below it that has one. This shifts a message by at most two
//! bytes and touches a fraction of draws too small to move any measurement.

use std::fmt;

use anyhow::{Result, bail};
use rand::Rng;
use rand_distr::{Distribution, LogNormal};
use serde::{Deserialize, Serialize};

/// The default cap on a message's encoded size: 64 KiB.
///
/// Gossip carries small records (state updates, membership changes,
/// configuration fragments), and 64 KiB holds the large end of those while
/// staying far below what the library can carry. A message's size there is
/// bounded only by the wire's 32-bit run length, about 4 GiB
/// ([`rumors::MAX_RUN_BUDGET_BYTES`]), and the default target message size,
/// [`rumors::default::TARGET_MESSAGE_SIZE`] (256 KiB), batches three messages
/// at the cap into one wire message, so a capped draw never forces a wire
/// message of its own. Raise it to measure larger payloads.
pub const DEFAULT_MAX_MESSAGE_SIZE: u64 = 64 << 10;

/// The default mean message size: 100 bytes, the size every measurement of the
/// library's model so far has used.
pub const DEFAULT_MEAN_MESSAGE_SIZE: f64 = 100.0;

/// The smallest acceptable chance that one draw lands within bounds.
///
/// A distribution that fails it would spend most of its time redrawing, and its
/// retained part would describe the cap, not the distribution the user asked
/// for; the tool refuses it and says which bound to move.
const MIN_ACCEPTANCE: f64 = 0.5;

/// Draws a cap of this size rejects in a row before the sampler reports that
/// the distribution cannot be met, which with [`MIN_ACCEPTANCE`] happens with
/// probability below `2^-1000`.
const MAX_CONSECUTIVE_REDRAWS: u32 = 1000;

/// A message-size distribution: lognormal by mean and coefficient of variation,
/// capped by redrawing.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SizeDistribution {
    /// The mean encoded size, in bytes, of the uncapped distribution.
    pub mean: f64,
    /// The standard deviation divided by the mean; zero for a fixed size.
    pub cv: f64,
    /// The largest encoded size a message may have; larger draws are redrawn.
    pub max: u64,
}

impl Default for SizeDistribution {
    /// Returns fixed 100-byte messages under the default cap.
    fn default() -> Self {
        Self {
            mean: DEFAULT_MEAN_MESSAGE_SIZE,
            cv: 0.0,
            max: DEFAULT_MAX_MESSAGE_SIZE,
        }
    }
}

impl fmt::Display for SizeDistribution {
    /// Prints `mean 100 B` for a fixed size, else the mean and CV, with the
    /// cap.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.cv == 0.0 {
            write!(f, "{} B", self.mean)
        } else {
            write!(f, "mean {} B, CV {}", self.mean, self.cv)
        }
    }
}

/// The parameters of a lognormal in log space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogSpace {
    /// The mean of the logarithm.
    pub mu: f64,
    /// The standard deviation of the logarithm.
    pub sigma: f64,
}

impl SizeDistribution {
    /// Checks that the distribution can be sampled as the user meant it.
    ///
    /// # Errors
    ///
    /// If the mean is not a positive, finite number of bytes, the CV is
    /// negative or not finite, the mean exceeds the cap, or fewer than half of
    /// all draws would land within bounds (the error then says whether to raise
    /// the cap or the mean).
    pub fn validate(&self) -> Result<()> {
        if !(self.mean.is_finite() && self.mean >= 1.0) {
            bail!(
                "a mean message size of {} bytes is impossible: the smallest message encodes in 1 byte",
                self.mean
            );
        }
        if !(self.cv.is_finite() && self.cv >= 0.0) {
            bail!(
                "a message-size CV of {} is impossible: it must be zero (a fixed size) or positive",
                self.cv
            );
        }
        if self.mean > self.max as f64 {
            bail!(
                "the mean message size, {} bytes, exceeds the cap of {} bytes: raise the cap with --max-message-size",
                self.mean,
                self.max
            );
        }
        let acceptance = self.acceptance();
        if acceptance < MIN_ACCEPTANCE {
            bail!(
                "only {:.1}% of message sizes drawn at mean {} B and CV {} land between 1 byte and the cap of {} bytes; \
                 raise the cap with --max-message-size or lower the CV",
                acceptance * 100.0,
                self.mean,
                self.cv,
                self.max
            );
        }
        Ok(())
    }

    /// The lognormal's log-space parameters.
    pub fn log_space(&self) -> LogSpace {
        let variance = (1.0 + self.cv * self.cv).ln();
        LogSpace {
            mu: self.mean.ln() - variance / 2.0,
            sigma: variance.sqrt(),
        }
    }

    /// The chance that one draw rounds to a size from 1 byte to the cap.
    pub fn acceptance(&self) -> f64 {
        if self.cv == 0.0 {
            let size = self.mean.round();
            return if size >= 1.0 && size <= self.max as f64 {
                1.0
            } else {
                0.0
            };
        }
        let LogSpace { mu, sigma } = self.log_space();
        let cdf = |x: f64| standard_normal_cdf((x.ln() - mu) / sigma);
        // A draw rounds into bounds when it lies in [0.5, max + 0.5).
        cdf(self.max as f64 + 0.5) - cdf(0.5)
    }

    /// A sampler for this distribution.
    ///
    /// # Panics
    ///
    /// If the distribution fails [`validate`](Self::validate): callers validate
    /// every distribution where the user supplies it.
    pub fn sampler(&self) -> Sampler {
        self.validate()
            .expect("message-size distributions are validated where the user supplies them");
        let LogSpace { mu, sigma } = self.log_space();
        Sampler {
            lognormal: LogNormal::new(mu, sigma)
                .expect("a validated distribution has a finite, non-negative sigma"),
            max: self.max,
            redrawn_above_cap: 0,
            redrawn_below_one_byte: 0,
        }
    }
}

/// The standard normal distribution's cumulative distribution function.
fn standard_normal_cdf(z: f64) -> f64 {
    0.5 * libm::erfc(-z / std::f64::consts::SQRT_2)
}

/// Draws message sizes and counts the draws it had to redraw.
#[derive(Clone, Debug)]
pub struct Sampler {
    /// The uncapped distribution.
    lognormal: LogNormal<f64>,
    /// The cap.
    max: u64,
    /// Draws redrawn because they exceeded the cap.
    pub redrawn_above_cap: u64,
    /// Draws redrawn because they rounded to no bytes.
    pub redrawn_below_one_byte: u64,
}

impl Sampler {
    /// Draws one message size: an encoded size from 1 byte to the cap that a
    /// CBOR byte string can take.
    ///
    /// # Panics
    ///
    /// After [`MAX_CONSECUTIVE_REDRAWS`] redraws in a row, which a validated
    /// distribution reaches with probability below `2^-1000`.
    pub fn draw<R: Rng + ?Sized>(&mut self, rng: &mut R) -> u64 {
        for _ in 0..MAX_CONSECUTIVE_REDRAWS {
            let size = self.lognormal.sample(rng).round();
            if size < 1.0 {
                self.redrawn_below_one_byte += 1;
            } else if size > self.max as f64 {
                self.redrawn_above_cap += 1;
            } else {
                return encodable_size_at_most(size as u64);
            }
        }
        panic!(
            "{MAX_CONSECUTIVE_REDRAWS} message sizes in a row fell outside bounds, \
             which a validated distribution does with probability below 2^-1000"
        )
    }
}

/// The CBOR head bytes of a byte string of `len` content bytes.
pub fn byte_string_head(len: u64) -> u64 {
    match len {
        0..24 => 1,
        24..256 => 2,
        256..65_536 => 3,
        65_536..4_294_967_296 => 5,
        _ => 9,
    }
}

/// The content bytes of the byte string whose encoding takes exactly `size`
/// bytes, if one does.
pub fn content_len(size: u64) -> Option<u64> {
    [1, 2, 3, 5, 9].into_iter().find_map(|head| {
        let len = size.checked_sub(head)?;
        (byte_string_head(len) == head).then_some(len)
    })
}

/// The largest encoded size no larger than `size` that a byte string takes.
///
/// # Panics
///
/// If `size` is zero: no byte string encodes in no bytes.
pub fn encodable_size_at_most(size: u64) -> u64 {
    (1..=size)
        .rev()
        .find(|candidate| content_len(*candidate).is_some())
        .expect("one byte encodes the empty byte string")
}

#[cfg(test)]
mod tests;
