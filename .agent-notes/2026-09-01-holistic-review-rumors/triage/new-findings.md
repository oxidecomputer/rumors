<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch during the rumors triage of 2026-09-02; findings surfaced by lanes and fresh-eyes reviews that no roster entry covers, each with its disposition or the ruling that gave it one; not authored, audited, or endorsed by Finch. Read with the ground rules in ../README.md. -->

# Findings without an entry

A finding a lane or reviewer noticed that the review's roster does not
carry gets a row here the day it is found, with its evidence and its
disposition: a ruling that homes it, a lane that lands it, or a stated
reason it changes nothing. Nothing is mentioned and left.

| found by | finding | evidence | disposition |
|---|---|---|---|
| p1-harness-tests lane | After a complete handshake, a responder whose peer dies before opening its first data stream parks in the data-stream accept and never consults control EOF. Within the link contract (the caller owns the timeout); a cheap death signal ignored. | the lane's probe log (`STUCK ... reads: 6 ... connects: 0, accepts: 0`, no EOF line) | T143: recorded as an open item for the mirror protocol's lanes; the in-process vanish fault is the instrument that reaches it, committed ignored if it fails on the current tree. |
| gate lane, fresh-eyes round 2 | `tools/doclint` walks its five roots with a bare `rglob("*.rs")` and no ignore set, so untracked build trees under `crates/` (`crates/before/fuzz/target`, `surfacecheck/target`, `wasm32-pins/target`, restored from cache in CI before `just ci` runs it) can change its verdict: the same class `verification-infra-9` fixed for testdoc. | `tools/doclint:370-373` at `b1a12f97`; `ci.yml` cache-restore steps precede the lint | Open. Candidate for the P3 lints lane (`p3-lints`), which already owns doclint's roster; the fix is testdoc's: the same ignore set, or a walk over `git ls-files`. |
