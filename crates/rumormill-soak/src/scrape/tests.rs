//! The scrape patterns are pinned against verbatim copies of what the UI
//! and the pre/post-TUI stderr print, so UI copy drift breaks here, not
//! mid-soak.

use super::*;

/// The header regex extracts every counter from a rendered status line,
/// including when a merge notice trails it.
#[test]
fn header_counters_parse() {
    let screen = " rumormill · n042 · net a1b2c3d4 · 101 live · sync 1234✓ 56✗ · merged into a1b2c3d4 \n you: abcdef  (share this id)";
    assert_eq!(
        header_stats(screen),
        Some(HeaderStats {
            net: "a1b2c3d4".into(),
            live: 101,
            sessions_ok: 1234,
            sessions_failed: 56,
        })
    );
}

/// A screen without a rendered header (e.g. captured before the first
/// frame) parses to nothing rather than garbage.
#[test]
fn missing_header_is_none() {
    assert_eq!(header_stats("rumormill: binding the iroh endpoint…"), None);
}

/// The roster count comes from the pane title and is exact: `peers (5)`
/// must not match inside `peers (50)`.
#[test]
fn roster_count_is_exact() {
    let screen = "┌peers (50)┐\n│ n001 (3s)│";
    assert_eq!(roster_count(screen), Some(50));
}

/// The endpoint id is scraped from the verbatim stderr announcement — but
/// only once the line is complete. A chunk boundary mid-id must yield
/// nothing, not a truncated id (the bug that strands every joiner with an
/// unparseable `--peer`).
#[test]
fn endpoint_id_from_stderr() {
    let partial = "rumormill: binding the iroh endpoint…\r\nrumormill: our endpoint id is ybndrf";
    assert_eq!(endpoint_id(partial), None);
    let complete = "rumormill: binding the iroh endpoint…\r\nrumormill: our endpoint id is ybndrfg8ejkmcpqxot1uwisza345h769abc\r\n";
    assert_eq!(
        endpoint_id(complete),
        Some("ybndrfg8ejkmcpqxot1uwisza345h769abc".into())
    );
}

/// Each departure stderr line maps to its variant, and an empty transcript
/// (killed before the departure path) maps to `Unknown`.
#[test]
fn departures_classify() {
    assert_eq!(
        departure("rumormill: retired into abcd1234; id-region reclaimed\r\n"),
        Departure::Retired
    );
    assert_eq!(
        departure("rumormill: the hand-off was interrupted mid-frame; the peer may hold our party"),
        Departure::Uncertain
    );
    assert_eq!(
        departure("rumormill: no peer could absorb us; departing with the region leaked"),
        Departure::Leaked
    );
    assert_eq!(
        departure(
            "rumormill: no live peers; departing without retiring (id-region leaks, which is fine for a demo)"
        ),
        Departure::NoPeers
    );
    assert_eq!(departure(""), Departure::Unknown);
}
