//! The population atlas: per-operation heatmaps of deterministic fuel
//! against exact input size, sampled uniformly from each size's whole
//! canonical input space.
//!
//! The crate's other instruments enforce points and slopes: envelope pins
//! bound chosen adversarial families, the fuzz-fit bands bound fuzzed
//! shapes, the amplification board prices the committed worst cases. None
//! of them shows a human the *distribution* of work over a declared input
//! measure. The atlas is that instrument: for each public operation it
//! renders `p(fuel | size)` as a log-log heatmap — one column per input
//! size, one conditional fuel distribution per column — so early-exit
//! strata, log-factor banding, and spread are visible to the eye, with the
//! committed adversarial families overlaid as marked points on the same
//! axes. One canvas shows the bulk cloud and the adversarial frontier.
//!
//! **The atlas is an audit view, not enforcement.** Its committed checks
//! are the sampler-correctness pins and a pipeline smoke test, nothing
//! else: no fuel threshold, percentile gate, or band is ever minted from
//! atlas data — the envelope suite and the fuzz-fit bands own enforcement.
//!
//! **The measure, and what it cannot see.** Inputs are drawn uniformly
//! from the set of canonical inputs whose packed encoding is exactly `n`
//! bytes (the crate's denominator of record), by counting-guided
//! generation over the codec grammars ([`count`], [`sample`]). Uniform
//! sampling audits the *bulk*: it shows where the mass of the input space
//! sends the operation. Engineered adversarial corners are measure-zero
//! out there — no uniform sample will ever hit one — which is exactly why
//! the committed family generators are overlaid as explicit points rather
//! than trusted to appear in the cloud, and why the board and the
//! envelopes (not the atlas) carry the adversarial verdicts.
//!
//! Fuel is wasmtime instruction-count metering through the fuzz-fit
//! guest — the same currency, guest, and driver as the band enforcement —
//! chosen for totality: no unmetered path can read as zero. Every cell's
//! RNG is seeded from (operation, size, sample index), so a run is exactly
//! reproducible and no entropy comes from time or the OS.

pub mod count;
pub mod enumerate;
pub mod sample;
