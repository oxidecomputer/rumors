//! Pure parsers over scraped terminal content.
//!
//! Everything here matches text the rumormill UI or its pre/post-TUI stderr
//! actually prints; the patterns are pinned by the unit tests in `tests.rs`,
//! so a UI copy change breaks loudly here instead of silently in a soak run.

use std::sync::OnceLock;

use regex::Regex;

/// The status-line counters from the TUI header:
/// `· net {net} · {live} live · sync {ok}✓ {failed}✗`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderStats {
    /// Short universe identifier (8 hex chars).
    pub net: String,
    /// Entries currently live in the rumor set.
    pub live: u64,
    /// Completed gossip sessions.
    pub sessions_ok: u64,
    /// Failed gossip sessions.
    pub sessions_failed: u64,
}

fn header_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"net ([0-9a-f]+) · (\d+) live · sync (\d+)✓ (\d+)✗").expect("static regex")
    })
}

/// Parse the header counters out of a screen scrape, if the header is
/// rendered and parseable.
pub fn header_stats(screen: &str) -> Option<HeaderStats> {
    let caps = header_re().captures(screen)?;
    Some(HeaderStats {
        net: caps[1].to_string(),
        live: caps[2].parse().ok()?,
        sessions_ok: caps[3].parse().ok()?,
        sessions_failed: caps[4].parse().ok()?,
    })
}

/// The roster size from the roster pane title, `peers (N)`.
pub fn roster_count(screen: &str) -> Option<usize> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"peers \((\d+)\)").expect("static regex"));
    re.captures(screen)?[1].parse().ok()
}

/// The endpoint id from the pre-TUI stderr announcement,
/// `rumormill: our endpoint id is {id}`.
///
/// The trailing line terminator is required: the scrape polls a stream
/// that arrives in arbitrary chunks, and without the anchor a poll landing
/// mid-line would match (and hand out) a truncated id.
pub fn endpoint_id(raw: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE
        .get_or_init(|| Regex::new(r"our endpoint id is ([a-z0-9]+)[\r\n]").expect("static regex"));
    Some(re.captures(raw)?[1].to_string())
}

/// How a node's departure went, per the stderr lines `main.rs` prints after
/// the TUI exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Departure {
    /// `retired into {peer}; id-region reclaimed`.
    Retired,
    /// `the hand-off was interrupted mid-frame` (two generals).
    Uncertain,
    /// `no peer could absorb us; departing with the region leaked`.
    Leaked,
    /// `no live peers; departing without retiring`.
    NoPeers,
    /// None of the departure lines appeared: the process was killed or
    /// crashed before reaching the departure path.
    Unknown,
}

/// Classify a finished node's departure from its raw transcript tail.
pub fn departure(raw: &str) -> Departure {
    if raw.contains("id-region reclaimed") {
        Departure::Retired
    } else if raw.contains("hand-off was interrupted") {
        Departure::Uncertain
    } else if raw.contains("no peer could absorb us") {
        Departure::Leaked
    } else if raw.contains("departing without retiring") {
        Departure::NoPeers
    } else {
        Departure::Unknown
    }
}

#[cfg(test)]
mod tests;
