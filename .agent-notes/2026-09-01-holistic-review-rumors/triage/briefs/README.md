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
| `p1-memwatch.md` | T136 | `tools/memwatch`, the justfile, `.github/workflows/ci.yml`, the `Cargo.toml` `[lib]` comment, `design/rumors-frame-fuzz.md` | small |
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

No lane depends on another's commits, with one exception: `p1-memwatch`
stacks on `p1-gate` (its base is the gate lane's final sha; both edit the
justfile) and launches after gate lands and before `p1-envelope` rebases,
so envelope rebases onto memwatch's sha. Two pairs share a file and should
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

The full landing procedure (worktree, acceptance verification, fresh-eyes rounds, the review packet with its interleaved annotations, stacks, stops, merge, ledger) is `../WORKFLOW.md`; this section is its summary.

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

# P3 lane briefs

Conventions and lints, each with a committed check. Every lane is based
on `<base sha>`, which the coordinator fills at launch: `main` after
every P1 and P2 lane has merged, because the sweeps touch the files all
of them edit. The lanes are governed by rulings T46 through T59 (T27 for
three moot rows), with T141's prose standard (`PROSE.md`) binding each.
The ledger's `lane` column is empty for P3 rows; the `ruling` column is
the roster, and each brief takes the rows of the rulings it names. Every
brief quotes each row's Resolution (and Acceptance, for heading entries)
verbatim, states the goal beside the mechanism, names the exact oracle
whose empty output is the acceptance, the committed check that keeps it
empty, and the known-bad artifact that must fail that check.

| Brief | Rulings | Rows | Touches | Size | Effort |
|---|---|---|---|---|---|
| `p3-lints.md` | T47, T54, T55 | 22 | `Cargo.toml` (`[lints]`), `src/lib.rs` attributes, the four clippy-lint site families, `type_complexity` allows under `streaming/`, `src/error.rs` and the error enums | large | high |
| `p3-modules.md` | T46, T53 | 20 | `AGENTS.md` (one line), `src/tree/typed.rs` comment, six test-module moves, a `tools/` placement check, the justfile | small | medium |
| `p3-seeds.md` | T59 | 5 | `proptest-regressions/**`, `tests/seed_liveness.rs` | small | high |
| `p3-vocabulary.md` | T49, T50, T51, T56, T57 (+T27: swarm-example-4) | 40 | rustdoc and comments across `src`, `tests`, `benches`; fixture renames in `conformance/backend/tests.rs`, `streaming/testing/faulting.rs`, `tests/common`; `AGENTS.md` (one line); `tools/doclint` if T51's check is cheap | large | medium |
| `p3-dashes.md` | T48 (+T27: swarm-example-11) | 20 | a `tools/` em-dash check, the justfile, `//` and `#` comments and assert strings across the rumors paths and the root config files | small | medium |
| `p3-imports.md` | T52 (+T27: swarm-example-25) | 31 | `rustfmt.toml`, the justfile `fmt` recipes, `ci.yml`, every import block in the workspace | medium | medium |
| `p3-prose-pass.md` | T58 | 0 | rustdoc across `src` | medium | medium (its fresh-eyes readers high) |

The rows sum to 138; every P3 id sits in exactly one brief.

## Grouping decisions

- Lanes group by the mechanism each check needs, and the `ruling`
  column's rosters are kept whole. T47, T54, and T55 share one lane
  because all three are lint-attribute edits on the public surface and
  the `missing_docs` sweep opens the same error files T55 edits. T46 and
  T53 share one lane as the two module conventions (one AGENTS.md line
  and one placement check). T52 stands alone: it is the one sweep a
  formatter produces, workspace-wide, redone on every rebase. T48 stands
  alone: its tool is shared with the `before` triage (its rulings 7 and
  54). T49, T50, T56, T57, and T51 share one lane because they rewrite
  the same prose and the same fixtures (`conformance/backend/tests.rs`
  carries both `Knob` and `Dishonest`), and T57's comment text depends on
  T50's sweep. T59 stands alone: its files are disjoint from everything.
- The three T27 rows (`swarm-example-4`, `-11`, `-25`) are moot, disposed
  by the example's deletion in `p1-swarm`; each sits in the lane matching
  its class, and the lane's only act is quoting the file's absence at base.
- `p3-prose-pass.md` carries no rows: TRIAGE.md's P3 table names owner
  decision 13 and T58 rules it as a pass run after the mechanical sweeps,
  so it has a brief; the coordinator may hold it until P3's other lanes
  have merged.
- Rows a P6 ruling owns are named as such in the briefs and not landed
  here: the `Iter` re-export (T60, in the lints lane's rows), the codec
  constructors' narrowing (T63), the twelve `Debug` impls (T84), the
  `warm_caches` docs (T96).

## Open before launch

Places where a ruling and a row's disposition disagree, or where a
ruling's mechanism cannot be executed as worded; each brief marks its
item as a stop, and none is resolved in the briefs.

1. `module-graph-14` (T53 row, P3, `fix`): T53 says the five inline
   production modules it names are "a separate P5 question and untouched
   here". The modules brief stops before touching it.
2. `tests-lifecycle-18` (T59 row, P3, `fix`, medium): its Resolution
   deletes `tests/async_wire.rs`, which T131 lands in the P5 tests lane;
   T59 disposes only the row's "consequence" (the four seeds re-homing in
   that commit). The seeds brief does not delete the binary.
3. `api-core-17` (T51): the row's three named sites are the
   `warm_caches` docs T96 deletes, and T51's wording ("every public
   type's and module's rustdoc is imperative, matching `Peer`, `Rumors`,
   `Bootstrap`") does not match the tree, where those type docs open as
   noun phrases and the mixed mood is in method docs. PROSE.md reads T51
   as one mood per item kind. The vocabulary brief stops before rewriting
   a type doc.
4. T47's "landed at `warn`, swept to zero, then promoted": under the
   recipes' `-D warnings` a `warn` entry in `[lints]` already fails the
   gate, so no lint can sit in the table with a nonzero remainder, and
   the three rustc lints' remainders are partly P6's (T60, T63, T84). The
   lints brief enumerates and stops on those three with a recommendation.
5. T53's check, read literally ("no `.rs` file contains an inline
   `cfg(test)` module body"), fires on `src/tree.rs`'s
   `#[cfg(test)] pub(crate) mod meter {`, which the same ruling leaves to
   P5. The modules brief checks by module name (`tests`/`test`) and asks
   which reading is meant.
6. `inventory-18` and the `header.rs` site of `clippy-pedantic-1` (T47
   rows) are link-25's, ruled T44 and landed by `p2-link`; the lints
   brief verifies them at base rather than landing them.
7. Cross-triage: the em-dash tool is one workspace check by the `before`
   triage's rulings 7 and 54 (whichever lane lands first ships it; the
   other sweep runs against it), and `p3-imports`' formatter run moves
   76 files under `crates/` (dry run at `6c90bd7d`) that the in-flight
   `before` lanes also edit. Both go through `.agent-notes/merge-queue.md`.

## Independence and launch order

Every unmerged `triage/*` branch at `6c90bd7d` collides with at least one
P3 lane: `p1-gate` (justfile, `ci.yml`, `Cargo.toml`, `src/lib.rs`,
`tools/testdoc`, five `.rs` files), `p1-memwatch` (justfile, `ci.yml`,
`Cargo.toml`), `p1-swarm` (`Cargo.toml`, `Cargo.lock`, `examples/swarm*`),
`p1-renderer` (`AGENTS.md`, `Cargo.toml`, justfile, `capture.rs` and its
tests, `tools/digestshare`, the snapshots), `p1-causality`
(`tests/bookmark_*.rs`, `tests/common/flaky.rs`), `p1-harness-tests`
(`src/testing.rs`, `src/testing/transport.rs`, `src/conformance/link/tests.rs`,
`tests/common/*`, `tests/disruption.rs`, `tests/gossip_when.rs`,
`tests/multi_peer.rs`, `tests/pairwise.rs`, `proptest-regressions/disruption.txt`),
`p2-link` (`src/link/routed/*`, `proptest-regressions/link/**`). The
lanes not yet started (`p1-harness-crate`, `p1-envelope`,
`p1-collision-mode`, `p2-codec`, `p2-commit-path`, `p2-walk`) overlap the
sweeps as well. So P3 launches only after the last P1 and P2 merge, from
`main`, and the briefs' "surveyed at `6c90bd7d`" counts are re-derived by
grep at that base.

Within P3, waves of at most four builders, hand-heavy lanes first and
formatter-shaped lanes last so a rebase redoes only what is cheap:

1. `p3-modules`, `p3-seeds`, `p3-lints` (seeds is file-disjoint from
   both; lints and modules overlap in `adversarial.rs`, so lints rebases
   onto modules' sha before its gate run).
2. `p3-vocabulary`, after modules (the AGENTS.md line) and lints (its
   new rustdoc is T51's input) have merged.
3. `p3-dashes`, then `p3-imports` (its formatter commit is separate,
   announced in the merge queue, and redone on every rebase).
4. `p3-prose-pass`, after `p3-imports` has merged.

P6 waits on `p3-lints`' enumeration (T47: the rustc lints list P6's
mechanical half); nothing in P4 or P5 waits on a P3 lane beyond the
rulings themselves, which are already recorded.
