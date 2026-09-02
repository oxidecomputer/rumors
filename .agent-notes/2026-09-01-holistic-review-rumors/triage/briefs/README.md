<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as lane briefs derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. -->

# P1 lane briefs

One brief per lane for the instruments phase. Every lane is based on
`0926fe32` (main, the commit recording the S1 rulings) and is governed by
`../rulings.md`; each brief quotes its members' Resolution and Acceptance
verbatim from the topic documents and names the ruling that governs each,
with any amendment stated. A lane agent reads only its brief; the brief
carries the ground rules in full.

## Lanes

| Brief | Rulings | Touches | Size |
|---|---|---|---|
| `p1-conformance.md` | T6, T18, T26 (tests-resource-link-window-20), T28 (window_corners floors) | `src/conformance/backend*`, `tests/window_census.rs`, `tests/window_corners.rs` | large |
| `p1-causality.md` | T8 | `tests/bookmark_causality.rs`, `tests/bookmark_transmit_window.rs`, `tests/bookmark_attach.rs` | medium |
| `p1-collision-mode.md` | T23 | `src/tree/typed/path.rs`, `src/testing`, the justfile, every suite that pins hash geometry | large |
| `p1-renderer.md` | T5, T9 | `codec/capture.rs` and its tests, `AGENTS.md` | small |
| `p1-envelope.md` | T10, T17, T43 | `window/tests.rs`, `window.rs`, `examples/envelope_sim.rs`, `src/lib.rs`, `results/` | medium |
| `p1-gate.md` | T7, T15, T16, T20, T26 (deps-1, verification-infra-7/-8/-9), T28 (CI and manifest), T29, T30 | `justfile`, `.github/workflows/ci.yml`, `tools/testdoc`, `Cargo.toml`, `src/lib.rs`, `tests/future_size.rs`, `src/peer.rs` rustdoc | medium |
| `p1-harness-crate.md` | T19, T22, T24, T26 (streaming-tests-11, materialized-27, remote-proxy-19) | in-crate suites under `src/tree/mirror/streaming/` | large |
| `p1-harness-tests.md` | T13, T21, T26 (tests-disruption-handshake-7, tests-lifecycle-15), T28 (disruption ranges, PollBudget) | `tests/`, `src/testing/transport.rs`, `src/conformance/link/tests.rs` | medium |
| `p1-swarm.md` | T27 | `examples/swarm*`, `Cargo.toml` | small |

## Grouping decisions

Two entries moved from the grouping the coordinator proposed, both to keep
one file inside one lane:

- `tests-resource-link-window-25` and `-28` (window_corners floors) sit in
  the conformance lane, not the tests-harness lane: `-20` in that lane
  re-measures `window_corners.rs:174`, and `-28`'s budget clause depends on
  that measurement.
- `testing-infra-12` and `conformance-18` (the reordering acceptor) sit in
  the tests-harness lane, not the conformance lane: they touch
  `src/testing/transport.rs` and the link suite, which the backend-suite
  lane never opens.

The swarm deletion stays its own lane despite its size because it is the
one lane whose diff Finch reviews for what it removes rather than what it
adds.

## Independence and launch order

No lane depends on another's commits. Two pairs share a file and should
not run concurrently, or one must rebase onto the other before its gate
run:

- `p1-gate` and `p1-envelope` both edit `src/lib.rs` (crate attributes
  versus the crate doc's envelope figures) and the justfile (recipe legs
  versus none; envelope touches only `window.rs`'s citation). Run gate
  first; envelope rebases.
- `p1-harness-crate` and `p1-collision-mode` both add tests under
  `src/tree/mirror/streaming/` and the collision sweep will mark tests the
  harness lane adds. Run harness-crate first; collision-mode rebases and
  sweeps the result.

Recommended order, with waves of at most four builders and the disk
checked before each wave:

1. `p1-gate`, `p1-swarm`, `p1-renderer`, `p1-conformance`
2. `p1-harness-crate`, `p1-harness-tests`, `p1-causality`, `p1-envelope`
   (envelope after gate lands)
3. `p1-collision-mode` (after harness-crate lands; it is the longest and
   its first-sweep triage produces the most stops)

Cross-lane ordering from TRIAGE.md that these lanes must honor: the
conformance census floor before any resize of `LOCAL_BUDGET`;
`tests-bookmark-12` before `tests-bookmark-9`; `streaming-tests-11` before
any P7 measurement of the capacity suite; T5's re-accept stop before the
renderer fix's snapshots move (they should not move; if they do, that is
the stop).

## What the coordinator does with a report

The full landing procedure (worktree, acceptance verification, fresh-eyes rounds, the pull request and its annotated self-review, stacks, stops, merge, ledger) is `../WORKFLOW.md`; this section is its summary.

A lane's report is data. For each entry the coordinator runs the
Acceptance against the tree at the reported sha, then writes `fix` with
the sha into `../ledger.tsv`. Entries reported as stopped stay `open` and
go to Finch as a numbered block. Merge is by reported sha, never by
branch name.

# P2 lane briefs

Production correctness. Every lane is based on `62b30e1f` (main at the
S2 commit) and governed by rulings T31 through T45. `deps-3` (the docs.rs
fix) is in `p1-gate.md` and is not repeated.

| Brief | Rulings | Touches | Size |
|---|---|---|---|
| `p2-commit-path.md` | T34, T39 (+tree-typed-30), T38, T36, T42 | `src/tree.rs`, `src/tree/traverse/{act,join}.rs`, `src/tree/typed/{prefix,untyped/fan}.rs`, `src/batch.rs`, `src/peer/gossip.rs` (commit closure), `src/lib.rs` payload section, `benches/in_memory.rs` | large |
| `p2-codec.md` | T33, T41 | `src/tree/mirror/framing.rs` and tests, `remote/codec/decode/async_io.rs` and tests | small |
| `p2-link.md` | T31, T44, T45 | `src/link/routed/` and its tests, `tests/common/{routed_tcp,tcp}.rs`, `tests/routed_link.rs` | medium |
| `p2-peer.md` | T37, T32, T35 | `src/peer.rs`, `src/peer/gossip.rs`, `src/bookmark.rs`, `src/rumors/changes.rs`, `src/batch.rs` (no-party arm) | medium |
| `p2-walk.md` | T40 | `src/tree/mirror/streaming/materialized/` and its tests; one prose sweep across `src/` | medium |

## Grouping decisions

`api-core-2` (the no-party arm in `Batch::commit`) sits in `p2-peer`, not
`p2-commit-path`, because T37 dissolves the arm by making the party total,
and that redesign lives in `Peer::retire`. Both lanes edit `src/batch.rs`
and `src/peer/gossip.rs`: commit-path touches only the `send_if_modified`
closures' shape (the pre-image `Option` and the sink), peer touches only
the party field and its arms. They must not run concurrently; see the
order. The "aborts typed" prose sweep rides with `p2-walk` because it is
prose-only and that lane is the one T40 governs; it touches test files in
`remote/`, so it lands after any P1 lane editing those files.

## Dependencies on P1 and launch order

- `p2-codec` and `p1-harness-crate` both edit the codec decode tests; the
  brief says land codec first or rebase onto harness-crate's sha.
- `p2-commit-path` and `p2-peer` share two files; run commit-path first,
  peer rebases onto its sha.
- `p2-link` and `p2-walk` are independent of everything.
- P2's acceptance tests are judged by P1's repaired harnesses only where
  they run through them: none of the five lanes runs through the
  streaming stall probes or the causality harness, so P2 may start in
  parallel with P1 wave 1 (T3).

Order: `p2-codec`, `p2-link`, `p2-walk`, `p2-commit-path` as one wave of
four (with P1 wave 1 already running, check the disk and cap concurrent
builders at four total); `p2-peer` after `p2-commit-path` lands.
