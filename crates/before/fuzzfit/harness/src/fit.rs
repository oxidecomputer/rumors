//! Log-log regression over (denominator, fuel) samples: the calibration
//! leg's fitter.
//!
//! Each public operation gets one fit of `log₁₀ fuel` against
//! `log₁₀ denom_bits` by ordinary least squares, with the *residual width*
//! (the maximum absolute residual over the corpus) recorded alongside the
//! line. The width is what makes the line a *band*: enforcement asserts
//! membership in `line ± (width + margin)`, so the fit's looseness is
//! explicit and committed, never implicit in a tolerance constant chosen
//! after the fact.
//!
//! Operations whose sampled denominators span less than a decade cannot
//! support a slope estimate (any line through a point cloud that narrow is
//! an artifact); they classify as *constant* bands — slope 0, the band
//! centered on the mean log-fuel — which is also the honest reading for
//! genuinely O(1) rows.

/// One fitted band over a kernel's samples.
#[derive(Debug, Clone, Copy)]
pub struct Fit {
    /// Fitted log-log slope (0 for constant-classified bands).
    pub slope: f64,
    /// Fitted intercept: `log₁₀ fuel` at `denom_bits = 1`.
    pub intercept: f64,
    /// Maximum absolute residual of the corpus against the line.
    pub width: f64,
    /// Sample count behind the fit.
    pub samples: usize,
    /// Smallest denominator seen (bits).
    pub min_denom: u64,
    /// Largest denominator seen (bits).
    pub max_denom: u64,
    /// Whether the band was constant-classified (denominator span under a
    /// decade).
    pub constant: bool,
}

/// Minimum denominator decades a slope estimate needs; narrower clouds
/// classify constant.
const MIN_DECADES: f64 = 1.0;

/// Fit one kernel's samples. `None` when fewer than 2 samples exist.
pub fn fit(samples: &[(u64, u64)]) -> Option<Fit> {
    if samples.len() < 2 {
        return None;
    }
    let min_denom = samples.iter().map(|&(d, _)| d).min().expect("non-empty");
    let max_denom = samples.iter().map(|&(d, _)| d).max().expect("non-empty");
    let xs: Vec<f64> = samples.iter().map(|&(d, _)| (d as f64).log10()).collect();
    let ys: Vec<f64> = samples
        .iter()
        .map(|&(_, f)| (f.max(1) as f64).log10())
        .collect();
    let n = xs.len() as f64;
    let decades = (max_denom as f64 / min_denom as f64).log10();
    let (slope, intercept) = if decades < MIN_DECADES {
        (0.0, ys.iter().sum::<f64>() / n)
    } else {
        let mx = xs.iter().sum::<f64>() / n;
        let my = ys.iter().sum::<f64>() / n;
        let sxx: f64 = xs.iter().map(|x| (x - mx) * (x - mx)).sum();
        let sxy: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
        let slope = sxy / sxx;
        (slope, my - slope * mx)
    };
    let width = xs
        .iter()
        .zip(&ys)
        .map(|(x, y)| (y - (intercept + slope * x)).abs())
        .fold(0.0f64, f64::max);
    Some(Fit {
        slope,
        intercept,
        width,
        samples: samples.len(),
        min_denom,
        max_denom,
        constant: decades < MIN_DECADES,
    })
}
