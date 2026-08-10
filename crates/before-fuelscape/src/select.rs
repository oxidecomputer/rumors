//! Selecting a roster subset by name: the runner's positional filters.
//!
//! A full survey spends hours; re-measuring one operation's panel after
//! a kernel change should not. The runner therefore accepts positional
//! filter arguments the way `cargo bench` does — each filter selects
//! every operation whose name contains it as a substring — and this
//! module owns those semantics so they are pinned by unit tests rather
//! than remembered by the argument loop.
//!
//! Two invariants close the silent-empty-run hole: with no filters the
//! selection is the whole roster (a bare invocation stays a full
//! survey), and a filter that matches no operation is an error naming
//! the available operations — never an empty selection that measures
//! nothing and exits green.

use crate::ops::OpSpec;

#[cfg(test)]
mod tests;

/// Select the roster rows whose names contain any of `filters` as a
/// substring, preserving roster order.
///
/// No filters selects the whole roster. Each row appears at most once
/// however many filters match it.
///
/// # Errors
///
/// If any filter matches no row — a typo or a stale name — the error
/// names every such filter and lists the roster's operation names. The
/// check is per filter, not on the union, so one misspelled filter is
/// caught even when the others match.
pub fn select<'r>(roster: &'r [OpSpec], filters: &[String]) -> Result<Vec<&'r OpSpec>, NoMatch> {
    let unmatched: Vec<String> = filters
        .iter()
        .filter(|f| !roster.iter().any(|op| op.name.contains(f.as_str())))
        .cloned()
        .collect();
    if !unmatched.is_empty() {
        return Err(NoMatch {
            unmatched,
            available: roster.iter().map(|op| op.name).collect(),
        });
    }
    Ok(roster
        .iter()
        .filter(|op| filters.is_empty() || filters.iter().any(|f| op.name.contains(f.as_str())))
        .collect())
}

/// Format a selection for `--list`: one operation name per line, each
/// line newline-terminated, in roster order.
pub fn listing(selected: &[&OpSpec]) -> String {
    selected.iter().map(|op| format!("{}\n", op.name)).collect()
}

/// A filter that selected nothing: the offending filters and the roster
/// names the caller can choose from.
#[derive(Debug)]
pub struct NoMatch {
    /// The filters that matched no roster row.
    unmatched: Vec<String>,
    /// Every roster row's name, in roster order.
    available: Vec<&'static str>,
}

impl std::fmt::Display for NoMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for filter in &self.unmatched {
            writeln!(f, "filter {filter:?} matches no operation")?;
        }
        writeln!(f, "available operations:")?;
        for name in &self.available {
            writeln!(f, "  {name}")?;
        }
        Ok(())
    }
}

impl std::error::Error for NoMatch {}
