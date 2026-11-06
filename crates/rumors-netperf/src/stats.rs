//! Medians and their distribution-free confidence intervals.
//!
//! Wall-time measurements are skewed (a session can be slowed by
//! interference, never sped up by it) and few per point, so the report
//! summarizes each point by its median and an interval between two order
//! statistics. For *n* samples from any continuous distribution, the
//! interval from the *k*-th smallest to the *k*-th largest contains the
//! median with probability `1 − 2·P(X < k)` for `X ~ Binomial(n, 1/2)`,
//! whatever the distribution. No resampling is involved, and the coverage
//! is exact: with five samples, the interval from the smallest to the
//! largest covers the median with probability 1 − 2/32 = 93.75%.

/// The coverage the report asks of every median's interval.
pub const CONFIDENCE: f64 = 0.90;

/// A median with its order-statistic interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Summary {
    /// The sample median.
    pub median: f64,
    /// The interval's lower end.
    pub low: f64,
    /// The interval's upper end.
    pub high: f64,
    /// The probability the interval covers the population median.
    pub coverage: f64,
    /// The samples summarized.
    pub n: usize,
}

/// `P(X ≤ k)` for `X ~ Binomial(n, 1/2)`.
pub fn binomial_half_cdf(n: usize, k: usize) -> f64 {
    let ln_half_n = n as f64 * 0.5f64.ln();
    let ln_choose = |i: usize| {
        libm::lgamma(n as f64 + 1.0)
            - libm::lgamma(i as f64 + 1.0)
            - libm::lgamma((n - i) as f64 + 1.0)
    };
    (0..=k.min(n))
        .map(|i| (ln_choose(i) + ln_half_n).exp())
        .sum::<f64>()
        .min(1.0)
}

/// The median of `samples` and the narrowest order-statistic interval that
/// covers it with probability at least `confidence`; `None` for no samples.
///
/// When even the full range cannot reach `confidence`, the interval is the
/// full range, with the coverage it has.
pub fn summarize(samples: &[f64], confidence: f64) -> Option<Summary> {
    let mut sorted: Vec<f64> = samples.iter().copied().filter(|x| x.is_finite()).collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let median = if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    };
    // The interval [x(k), x(n−k+1)], 1-indexed, misses the median exactly
    // when at most k − 1 samples fall on one side of it.
    let coverage = |k: usize| 1.0 - 2.0 * binomial_half_cdf(n, k - 1);
    let mut k = 1;
    while k < n.div_ceil(2) && coverage(k + 1) >= confidence {
        k += 1;
    }
    Some(Summary {
        median,
        low: sorted[k - 1],
        high: sorted[n - k],
        coverage: coverage(k).max(0.0),
        n,
    })
}

#[cfg(test)]
mod tests;
