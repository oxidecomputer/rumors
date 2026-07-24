//! The bench judge's knobs and denominator sidecar, shared by the two
//! judge-facing targets (`benches/board.rs`, `benches/tripwire.rs`).
//!
//! Two environment variables drive the `just bench-judge*` recipes and
//! default to off:
//!
//! - [`SCALE_ENV`]: the input scale (a positive number, or the literal
//!   `record` for the board's acceptance scale
//!   `before::meter::board::RECORD_SCALE`).
//! - [`DENOMS_ENV`]: a path; when set, the harness writes the JSON sidecar
//!   the judge divides by — a configuration stamp plus one denominator
//!   count per cell ID.
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

use std::path::Path;

use before::meter::board;

/// The input-scale environment variable read by [`scale_from_env`].
pub const SCALE_ENV: &str = "BOARD_BENCH_SCALE";

/// The sidecar-path environment variable read by [`write_denoms`].
pub const DENOMS_ENV: &str = "BOARD_BENCH_DENOMS";

/// The source-tip environment variable stamped into the sidecar.
pub const TIP_ENV: &str = "BOARD_BENCH_TIP";

/// The input scale from [`SCALE_ENV`]: unset means the board's
/// seconds-scale default of 1, `record` means `board::RECORD_SCALE`.
pub fn scale_from_env() -> f64 {
    match std::env::var(SCALE_ENV) {
        Err(std::env::VarError::NotPresent) => 1.0,
        Ok(raw) if raw == "record" => board::RECORD_SCALE,
        Ok(raw) => raw.parse().unwrap_or_else(|_| {
            panic!("{SCALE_ENV} must be a positive number or `record`, got {raw:?}")
        }),
        Err(err) => panic!("{SCALE_ENV} is not valid UTF-8: {err}"),
    }
}

/// Write the denominator sidecar to the [`DENOMS_ENV`] path, if set: the
/// configuration stamp, then one `"op/family": bytes` member per cell in
/// the order given.
///
/// Creates the sidecar's parent directory if it does not exist yet: on a
/// fresh target directory the recipes point the sidecar into
/// `target/criterion/`, which criterion only creates later in the run,
/// after this harness-setup write.
pub fn write_denoms<'a>(scale: f64, cells: impl IntoIterator<Item = (&'a str, usize)>) {
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
    json.push_str("  },\n  \"denominator_bytes\": {\n");
    let cells: Vec<(&str, usize)> = cells.into_iter().collect();
    for (i, (id, denom)) in cells.iter().enumerate() {
        assert!(
            id.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-')),
            "cell IDs are plain `op/family` names, got {id:?}"
        );
        let comma = if i + 1 < cells.len() { "," } else { "" };
        json.push_str(&format!("    \"{id}\": {denom}{comma}\n"));
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
