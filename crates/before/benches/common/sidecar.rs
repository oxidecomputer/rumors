//! The bench judge's knobs and denominator sidecar, shared by the two
//! judge-facing targets (`benches/board.rs`, `benches/tripwire.rs`).
//!
//! Three environment variables drive the `just bench-judge*` recipes and
//! default to off:
//!
//! - [`SCALE_ENV`]: the input scale (a positive number, or the literal
//!   `acceptance` for the board ladder's top sampling scale
//!   `before::meter::board::LADDER_TOP_SCALE`).
//! - [`MODE_ENV`]: the product slice (`pinned`, the default, or `full`;
//!   `board::BenchMode`).
//! - [`DENOMS_ENV`]: a path; when set, the harness writes the JSON sidecar
//!   the judge divides by — a configuration stamp plus one object per cell
//!   ID carrying the denominator count and the cell's ceiling class.
//!
//! The stamp binds the sidecar to the bench run that wrote it, so
//! `tools/benchjudge` can refuse a sidecar/baseline pair assembled from
//! different runs (exit 2) instead of silently re-scoring: the resolved
//! scale, the build profile (`dev` under debug assertions, `optimized`
//! otherwise — criterion benches build optimized under cargo's bench
//! profile), the sampling mode (`quick` when a `--sample-size` override is
//! on the command line, the recipes' reduced-sampling convention; `record`
//! at criterion's full-sampling default), and the source tip ([`TIP_ENV`],
//! which the recipes set to `git rev-parse HEAD`; written as JSON `null`
//! when unset).
//!
//! Each cell entry carries the cell's [`Ceiling`] class beside its
//! denominator bytes: the ceiling a cell is judged at is a property of the
//! cell, declared here in bench code at the cell's definition site — never
//! by the judge's roster, whose entries are expectations only.

use std::path::Path;

use before::meter::board;

/// The ceiling class a bench cell declares for the judge's exponent fit.
///
/// `tools/benchjudge` maps each class to its ceiling constant (general 1.3,
/// text 1.7; both derivations live at the constants). The text class exists
/// for conversion-dominated rendering only ([`TEXT_CEILING_CELLS`]):
/// binary→decimal conversion is honestly superlinear, so the general
/// ceiling would read the honest class red.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ceiling {
    /// The default class, judged at the judge's general ceiling.
    General,
    /// The text-conversion class, judged at the judge's text ceiling.
    Text,
}

impl Ceiling {
    /// The class's sidecar spelling, the judge's `ceiling` vocabulary.
    fn as_str(self) -> &'static str {
        match self {
            Ceiling::General => "general",
            Ceiling::Text => "text",
        }
    }
}

/// The cell IDs declared [`Ceiling::Text`]: the judge-only wide-display
/// pair and the board's hugeleaf display pair.
///
/// The wide-display pair (`version_display_wide`, `display_schoolbook`)
/// times conversion at conversion-dominated widths by construction
/// (`benches/board.rs` documents the pair). The hugeleaf display pair
/// (`version_display`, `clock_display` on the maximal-bits-per-node
/// family) declares the same model: display renders text, and at hugeleaf
/// widths the render is conversion-dominated, so the
/// superlinear-but-subquadratic conversion class is the intended cost of
/// exactly these rows — measured exponents 1.39/1.42 (quick sampling,
/// bench profile) against the 1.7 text ceiling, with a quadratic render
/// still reading red there.
///
/// [`write_denoms`] asserts every declaration against this set, and
/// `tests/bench_judge_roster.rs` pins the set itself — so widening the
/// text class is a two-site edit whose diff a reviewer sees, never a
/// one-character class swap at a cell.
pub const TEXT_CEILING_CELLS: [&str; 4] = [
    "version_display_wide/hugeleaf",
    "display_schoolbook/hugeleaf",
    "version_display/hugeleaf",
    "clock_display/hugeleaf",
];

/// The input-scale environment variable read by [`scale_from_env`].
pub const SCALE_ENV: &str = "BOARD_BENCH_SCALE";

/// The bench-mode environment variable read by [`mode_from_env`].
pub const MODE_ENV: &str = "BOARD_BENCH_MODE";

/// The sidecar-path environment variable read by [`write_denoms`].
pub const DENOMS_ENV: &str = "BOARD_BENCH_DENOMS";

/// The source-tip environment variable stamped into the sidecar.
pub const TIP_ENV: &str = "BOARD_BENCH_TIP";

/// The input scale from [`SCALE_ENV`]: unset means the board's
/// seconds-scale base of 1, `acceptance` means `board::LADDER_TOP_SCALE`.
pub fn scale_from_env() -> f64 {
    match std::env::var(SCALE_ENV) {
        Err(std::env::VarError::NotPresent) => 1.0,
        Ok(raw) if raw == "acceptance" => board::LADDER_TOP_SCALE,
        Ok(raw) => raw.parse().unwrap_or_else(|_| {
            panic!("{SCALE_ENV} must be a positive number or `acceptance`, got {raw:?}")
        }),
        Err(err) => panic!("{SCALE_ENV} is not valid UTF-8: {err}"),
    }
}

/// The bench mode from [`MODE_ENV`]: unset or `pinned` means the
/// rule-derived subset, `full` the whole shape × operation product (the
/// mode for final verdicts).
///
/// Judge runs must pair like with like: a lo/hi baseline pair recorded in
/// different modes covers different cell sets, and the judge treats the
/// asymmetry as missing cells rather than silently judging the
/// intersection.
pub fn mode_from_env() -> board::BenchMode {
    match std::env::var(MODE_ENV) {
        Err(std::env::VarError::NotPresent) => board::BenchMode::Pinned,
        Ok(raw) if raw == "pinned" => board::BenchMode::Pinned,
        Ok(raw) if raw == "full" => board::BenchMode::Full,
        Ok(raw) => panic!("{MODE_ENV} must be `pinned` or `full`, got {raw:?}"),
        Err(err) => panic!("{MODE_ENV} is not valid UTF-8: {err}"),
    }
}

/// Write the denominator sidecar to the [`DENOMS_ENV`] path, if set: the
/// configuration stamp, then one cell object (denominator bytes plus
/// ceiling class) per cell in the order given.
///
/// # Panics
///
/// Panics when a cell's declared [`Ceiling`] disagrees with membership in
/// [`TEXT_CEILING_CELLS`]: the text-class set is pinned, so widening it
/// takes an edit to the pinned constant, never a lone class argument.
///
/// Creates the sidecar's parent directory if it does not exist yet: on a
/// fresh target directory the recipes point the sidecar into
/// `target/criterion/`, which criterion only creates later in the run,
/// after this harness-setup write.
pub fn write_denoms<'a>(scale: f64, cells: impl IntoIterator<Item = (&'a str, usize, Ceiling)>) {
    let path = match std::env::var(DENOMS_ENV) {
        Err(std::env::VarError::NotPresent) => return,
        Ok(path) => path,
        Err(err) => panic!("{DENOMS_ENV} is not valid UTF-8: {err}"),
    };
    let mut json = String::from("{\n  \"stamp\": {\n");
    json.push_str(&format!("    \"scale\": {scale:?},\n"));
    json.push_str(&format!("    \"profile\": \"{}\",\n", profile()));
    json.push_str(&format!(
        "    \"sampling\": \"{}\",\n",
        sampling_from_args()
    ));
    json.push_str(&format!("    \"tip\": {}\n", tip_from_env()));
    json.push_str("  },\n  \"cells\": {\n");
    let cells: Vec<(&str, usize, Ceiling)> = cells.into_iter().collect();
    for (i, (id, denom, ceiling)) in cells.iter().enumerate() {
        assert!(
            id.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-')),
            "cell IDs are plain `op/family` names, got {id:?}"
        );
        assert_eq!(
            *ceiling == Ceiling::Text,
            TEXT_CEILING_CELLS.contains(id),
            "{id}: the text-ceiling set is pinned as TEXT_CEILING_CELLS; \
             declare the class there and at the cell together"
        );
        let comma = if i + 1 < cells.len() { "," } else { "" };
        json.push_str(&format!(
            "    \"{id}\": {{ \"denominator_bytes\": {denom}, \"ceiling\": \"{}\" }}{comma}\n",
            ceiling.as_str()
        ));
    }
    json.push_str("  }\n}\n");
    if let Some(parent) = Path::new(&path).parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|err| {
            panic!("creating the sidecar directory {parent:?} failed: {err}")
        });
    }
    std::fs::write(&path, json)
        .unwrap_or_else(|err| panic!("writing the denominator sidecar {path:?} failed: {err}"));
}

/// The build profile as the harness can observe it: `dev` under debug
/// assertions, `optimized` otherwise.
fn profile() -> &'static str {
    if cfg!(debug_assertions) {
        "dev"
    } else {
        "optimized"
    }
}

/// The sampling mode, read off this process's own criterion arguments:
/// `quick` when a `--sample-size` override is present (the recipes'
/// reduced-sampling convention), `record` at criterion's full default.
fn sampling_from_args() -> &'static str {
    let quick =
        std::env::args().any(|arg| arg == "--sample-size" || arg.starts_with("--sample-size="));
    if quick {
        "quick"
    } else {
        "record"
    }
}

/// The [`TIP_ENV`] stamp value as a JSON fragment: a quoted hash when the
/// recipe set it, `null` otherwise.
fn tip_from_env() -> String {
    match std::env::var(TIP_ENV) {
        Err(std::env::VarError::NotPresent) => "null".to_string(),
        Ok(tip) => {
            assert!(
                !tip.is_empty()
                    && tip
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')),
                "{TIP_ENV} must be a plain revision string, got {tip:?}"
            );
            format!("\"{tip}\"")
        }
        Err(err) => panic!("{TIP_ENV} is not valid UTF-8: {err}"),
    }
}
