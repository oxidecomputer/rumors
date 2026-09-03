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
| `p1-proptest-ci.md` | T148 | `src/testing.rs`, every explicit `ProptestConfig` under `src` and `tests`, `tools/caselint`, the justfile, `.github/workflows/ci.yml` | medium |

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

`p1-proptest-ci` launches last, from `main` after every P1 and P2 lane
that touches test files has merged (its sweep routes every explicit
`ProptestConfig`, and its justfile and `ci.yml` edits stack on `p1-gate`
until that lane merges).

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
| `p2-vanish-liveness.md` | T145 | `src/tree/mirror/streaming/remote/proxy/{work.rs,error.rs,work/tests.rs}`, `remote/streams.rs`, `src/peer/gossip.rs` (the post-descent control readers), `tests/common/{fault,sim}.rs`, `tests/disruption.rs` | medium |

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
`p2-vanish-liveness` stacks on `triage/p1-harness-tests` (its base is
that lane's final sha, which carries the vanish fault and the ignored
test) and launches once that branch is complete; it shares
`src/tree/mirror/streaming/remote/proxy/` with `p1-harness-crate`'s test
edits, so whichever of the two lands second rebases.

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

# P4 lane briefs

Pattern sweeps. Every lane is based on `<base sha>`, which the
coordinator fills at launch: `main` after every P3 lane has merged,
because each sweep assumes P3's conventions (the `[lints]` table, the
em-dash check, the vocabulary rulings, `group_imports`, the sibling
test-module placement) and its grep oracles would otherwise re-find
what P3 removes. The lanes are governed by the owner decisions 73
through 78 and 80 through 93 (T86 through T104), the mediums' block
rulings T124 through T131, and T132 for every low and nit; T5, T27,
and T143 dispose the twelve rows already terminal; T141's prose
standard (`PROSE.md`) binds each lane; T151 binds every proptest a
lane adds (no `cases`); T155 governs every manifest commit
(`--locked`). The ledger's `lane` column is empty for P4 rows, so the
roster is the `patterns` column: each lane groups the rows by the
pattern its sweep applies and the oracle that regenerates it. Every
brief quotes each row's Resolution and Acceptance verbatim, states the
goal beside the mechanism, names the exact oracle whose empty output is
the acceptance and the committed check that keeps it empty (or says why
the sweep is a one-time pass with its site list in the report), and
names the known-bad artifact each check must fail.

| Brief | Rulings | Rows | Touches | Size | Effort |
|---|---|---|---|---|---|
| `p4-tallies.md` | T98, T104, T129, T131, T132 (T112's two rows named) | 33 (+2 named) | rustdoc and comments across `src`, `tests`, `benches`; `.config/nextest.toml`, `ci.yml`; one `const _` in `src/conformance/link.rs` | large | medium |
| `p4-v1-residue.md` | T82, T124, T125, T126, T128, T129, T130, T131, T132 | 31 | rustdoc and testdocs across `src` and `tests`; `src/protocol.rs` (the `repr`, the `Default`), `tests/handshake.rs`; `src/tree/traverse.rs` (`mod act` private) | large | medium |
| `p4-narrative.md` | T128, T130, T132 | 20 (+1 routed finding) | rustdoc and comments in the tree, streaming, remote, session, and the suites | medium | medium |
| `p4-docshape.md` | T132 | 24 | `tools/doclint` (a heading-vocabulary rule), rustdoc across `src` and `tests` | medium | medium |
| `p4-placement.md` | T85, T92, T93, T95, T97, T99, T100, T102, T103, T128, T132 (T96's row named) | 24 (+1 named) | a new public explanation module beside `reconciliation`; `src/peer.rs`, `src/reconciliation.rs`, `src/lib.rs`, `window.rs`, `backend.rs`, the error docs, `src/snapshot.rs` and `src/tree.rs` (T97), `codec/signal.rs` and `src/observe.rs` (T99), `src/link.rs`, `src/link/routed.rs`, `tests/cbor_evolution.rs`, one justfile comment | large | high |
| `p4-leftovers.md` | T89, T90, T91, T132 (T84's and T115's rows named) | 31 (+3 named) | derives, bounds, and attributes across `src`; `Cargo.toml` and `Cargo.lock` (four manifest commits); `tools/digestshare`; braces in eleven test files; `tests/common/mod.rs`'s allow | large | high |
| `p4-widths.md` | T85, T87, T88, T126, T129, T132 | 27 | `src/tree/typed/*` (`PATH_LEN`), `src/link.rs`, `codec/signal.rs`, `window.rs`, `src/testing.rs`, test-side constants across the suites, `benches/branch_hash.rs` | large | high |
| `p4-core-simplify.md` | T125, T128, T132 | 26 | `src/rumors/*`, `src/peer/gossip.rs`, `src/bookmark.rs`, `src/message.rs`, `src/tree.rs`, `src/tree/typed/*`, `src/link/routed/router.rs`, `src/conformance/*`, `src/testing/transport.rs`, `benches/support`, `tests/common/schedule` | large | high |
| `p4-streaming-simplify.md` | T86, T128, T132 (T63's row named; T86's cross-reference) | 22 (+2 named) | `streaming/channel.rs`, `erased.rs`, `materialized*`, `backend/*`, `window.rs`, `protocol.rs`, `streaming.rs`, `framing.rs`; `Cargo.toml` (`tokio-stream` leaves) | large | high |
| `p4-remote-simplify.md` | T119, T126, T132 (T63's row named) | 23 (+1 named) | `remote/proxy/{start,state,work,work/pump,work/encode}.rs`, `remote/adapter/{decode,encode}.rs`, `remote/streams.rs`, `remote/codec/{greeting,frame,tests}.rs`, `remote.rs`, the capture renderer | large | high |
| `p4-testdocs.md` | T85, T124, T126, T128, T129, T130, T132 | 32 | test docs and bodies across `src/**/tests.rs` and `tests/` | large | high |
| `p4-negative-controls.md` | T126, T130, T132 | 12 | new tests beside twelve detectors in `src` and `tests` | medium | high |
| `p4-oracles.md` | T126, T128, T130, T132 | 16 | the proxy tests, `streaming/tests/stats.rs`, `src/tree/arb.rs`, nine integration suites | medium | high |
| `p4-drivers.md` | T101, T128, T130, T132 | 16 | `src/tests.rs`, the proxy tests, `tests/{changes,latency_link,gossip_when}.rs`, the streaming, window, handshake, and codec tests; `Cargo.toml` (`pollster` leaves if no site survives) | medium | high |
| `p4-bookmark-example.md` | T94 | 1 | `src/bookmark.rs` or a new module, `Cargo.toml` (a feature), `src/tutorial.rs`, three test bookmarks | small | high |

The rows sum to 359: 338 landed by the lane that lists them, nine named
in a lane and verified at base because another ruling's lane lands them
(T63: mirror-common-10, remote-adapter-streams-19; T84: link-5; T96:
async-hazards-4; T112: suite-economics-3, tree-core-24; T115:
benches-envelope-22, tests-disruption-handshake-28; T86's
cross-reference materialized-10), and twelve already terminal in the
ledger (the seven `swarm-example-*` rows under T27, prose-hygiene-6 and
verification-infra-15 under T5, tests-disruption-handshake-4 as a model
under T143, inventory-5 and inventory-9 as dups). Every P4 id sits in
exactly one brief or in that terminal list.

## Grouping decisions

- Prose rows group by the pattern a grep regenerates, crate-wide, since
  a prose sweep's cost is the census and its hunks are small: five
  lanes (tallies; the V1 and wire-respelling residue; incident
  narrative with roster tags; the four doc-shape patterns; public
  altitude with the owner's placement rulings). Code rows group by the
  module family the refactor edits, since a refactor's oracle is the
  suites and its hazard is the merge, so file-disjointness governs:
  three refactor lanes (core, streaming, remote) plus two crate-wide
  mechanical code sweeps whose oracles are greps and `const`
  assertions (leftovers, widths). Test rows group by the shape of the
  repair: docs against bodies, a demonstration per detector,
  whole-root oracles with redaction in every population, and the
  poller with family claims.
- Rulings are kept whole across lanes wherever a ruling names one
  mechanism: T82's seven sites in one lane, T119's five, T87's two,
  T88, T93, T95, T103, T104, T101. The block approvals (T124 through
  T131, "each lands per its entry") are split by pattern, which is what
  they approve.
- Six rows sit outside their document's pattern lane on purpose:
  materialized-11 (a documentation "two homes" entry whose fix is the
  handshake refactor) rides `p4-streaming-simplify`; prose-hygiene-1
  and tree-typed-19 (simplification "erasure residue" whose fix is
  prose over the same `traverse.rs` sites as tree-core-26) ride
  `p4-v1-residue`; remote-adapter-streams-29 (a testdoc row whose fix
  is the epoch proptest beside tests-observation-24) rides `p4-widths`;
  tests-bookmark-14 (a simplification row that deletes vacuous asserts)
  rides `p4-testdocs`; link-2 (incident narrative under T100) rides
  `p4-placement` with T100's other row.
- The `internal_level` doc claim from `new-findings.md` (no ledger row)
  rides `p4-narrative`, the prose lane over `materialized/`.
- `p4-bookmark-example` is one row on its own because its base differs:
  T94 builds against T62's trait shape and T69's suite, both P6.

## Open before launch

Places where a ruling and a row's disposition disagree, where a
ruling's mechanism cannot be executed as worded, or where a brief adds
something no ruling names; each brief marks its item, and none is
resolved in the briefs.

1. T82's Disposes names module-graph-4, prose-hygiene-2, and
   mirror-common-12; the ledger cites T132 for all three. Bookkeeping:
   recommend the ledger's `ruling` column read T82 for them.
2. Nine P4-phase rows are homed by rulings in other phases' lanes (the
   list above). The briefs verify them at base and land nothing;
   recommend a `phase!` note re-phasing each row to its lane so
   `ledger.py check` counts them where they land.
3. T94 (fresh-eyes-8, owner decision 84, phase P4) depends on T62 and
   T69 (P6): `p4-bookmark-example`'s base is main after those merge.
   Recommend it launch with or after the P6 bookmark lane.
4. T97 (`Snapshot::hash` and `MERKLE_HASH_LEN` leave the surface) and
   T99 (one election enum) are public-surface edits ruled under P4's
   decisions 86 and 88 but shaped like P6's pass; `p4-placement` lands
   them and stops if P6 has moved `Snapshot` (T60, T61) or `observe.rs`
   (T66, T74) at base. Recommend they ride P6's lane if it launches
   first.
5. T87 rules that the codec owns the stream count and the link cites
   it; link-3's and remote-codec-3's Resolutions say the reverse. The
   widths brief follows the ruling and reports the discrepancy.
   remote-codec-3's fan half (`MAX_QUERY_CHILDREN` from `FAN`) is T120,
   P8, and is not landed.
6. streaming-tests-23's Resolution proposes
   `ProptestConfig::with_cases(32)`; T151 forbids it. `p4-drivers`
   takes the default count and reports the site.
7. T125 orders api-core-33 with T74 (P6); `p4-core-simplify` lands it
   alone if T74 is absent at base and reports.
8. module-graph-2's acceptance names `analyze.py`, which lives in this
   note's `evidence/` tree, not in `tools/`: T86 has no standing check.
   Recommend accepting the quoted run, or ruling the script into
   `tools/` as a gate leg.
9. `p4-docshape` adds a `tools/doclint` heading-vocabulary rule (a new
   gate check no ruling names; self-tested, with `# Cancellation` as
   its known-bad). Recommend approving it, or reducing it to a one-time
   grep.
10. `p4-drivers` removes `pollster` from `[dev-dependencies]` if no
    site survives: a dependency removal no ruling names. Recommend
    approving it, since the absent dependency is the strongest check.
11. tests-disruption-handshake-10's equality between the two hop
    instruments waits on the shared fixture (tests-disruption-handshake-31,
    T131, the P5 harness lane); `p4-oracles` lands the floor cell now
    and reports the equality as waiting.
12. Four rows' sites may be gone at base: benches-envelope-30
    (`examples/envelope_sim.rs`, deleted by T43 once `p1-envelope`
    lands), prose-hygiene-12 and tests-resource-link-window-4
    (`tests/async_wire.rs`, deleted by T131 in P5), suite-economics-9
    (the memwatch header, T136), and the three capture-renderer rows
    (remote-capture-atlas-11, -16, -19) after T138/T140. Each brief
    says: land if present, else quote the absence; the ledger then
    needs a disposition (`dup` of the deleting ruling) for any that
    turn out absent.

## Independence and launch order

At `cb79d712` (main at drafting) the unmerged `triage/*` branches with
a nonempty diff are `p1-proptest-ci` (the proxy and streaming test
files, `tests/{bookmark_when,disruption,observe,party_conservation,session_overlap,session_stats,wire_legibility}.rs`,
justfile, `ci.yml`, `tools/caselint`), `p2-commit-path` (`src/tree.rs`,
`traverse/*`, `typed/{node,prefix,untyped,untyped/fan}.rs`, `arb.rs`,
`batch.rs`, `peer.rs`, `gossip.rs`, `rumors.rs`, `snapshot.rs`,
`lib.rs`), `p2-link` (`src/link/routed/*`, `tests/routed_link.rs`,
`tests/common/{tcp,routed_tcp}.rs`), and `p2-vanish-liveness` (on the
merged harness-tests tip, so its diff against main is that lane's
history until it is rebased; its own work touches the proxy's `work.rs`,
`error.rs`, `streams.rs`, `proxy/tests.rs`, `gossip_when.rs`,
`src/testing.rs`). Not yet launched: `p1-envelope`, `p2-peer`, the seven
P3 lanes. Every P4 lane collides with at least one of these (from the
members' cited sites; `p2-vanish-liveness` counted by its own scope):

- `p4-tallies`: commit-path (`peer.rs`, `gossip.rs`, `tree.rs`,
  `arb.rs`), proptest-ci (`ci.yml`, justfile), vanish-liveness
  (`nextest.toml`, `conformance/link/tests.rs`, `src/testing.rs`).
- `p4-v1-residue`: commit-path (twelve files under `src/tree`, `peer.rs`,
  `gossip.rs`), link (`endpoint.rs`, `tests/common/tcp.rs`,
  `routed_link.rs`), vanish-liveness (`src/testing.rs`, `tcp_link.rs`).
- `p4-narrative`: commit-path (`arb.rs`, `tree/tests.rs`,
  `traverse/unknown/tests.rs`, `typed/*`), proptest-ci (`capacity.rs`,
  `disruption.rs`, `session_overlap.rs`), vanish-liveness
  (`bookmark_causality.rs`, `common/overlap.rs`, `disruption.rs`).
- `p4-docshape`: commit-path (`gossip.rs`, `rumors.rs`, `typed/*`), link
  (`endpoint.rs`, `router.rs`, `routed/tests.rs`).
- `p4-placement`: commit-path (`README.md`, `lib.rs`, `peer.rs`,
  `rumors.rs`, `snapshot.rs`, `typed/node.rs`), link (`routed.rs`),
  vanish-liveness (`src/testing.rs`, `transport.rs`), proptest-ci
  (justfile).
- `p4-leftovers`: commit-path (`gossip.rs`, `tree.rs`, `traverse/*`,
  `typed/*`), link (`endpoint.rs`, `header.rs`, `routed_link.rs`),
  proptest-ci (justfile, four suites), vanish-liveness
  (`conformance/link/tests.rs`, `common/{fault,sim}.rs`, `disruption.rs`,
  `pairwise.rs`).
- `p4-widths`: commit-path (`peer.rs`, `typed/prefix.rs`), proptest-ci
  (`failures.rs`, `capacity.rs`, `observe.rs`, `session_stats.rs`),
  vanish-liveness (`src/testing.rs`, `gossip_when.rs`).
- `p4-core-simplify`: commit-path (nine files), link (`router.rs`),
  vanish-liveness (`conformance/link/tests.rs`, `src/testing.rs`,
  `transport.rs`), proptest-ci (`bookmark_when.rs`).
- `p4-streaming-simplify`: commit-path (`lib.rs`, `gossip.rs`),
  proptest-ci (`capacity.rs`), vanish-liveness (`transport.rs`).
- `p4-remote-simplify`: vanish-liveness (the proxy's production code
  and `proxy/tests.rs`), proptest-ci (`proxy/tests.rs`), commit-path
  (`gossip.rs`, `typed/prefix.rs`).
- `p4-testdocs`: commit-path (eight files), proptest-ci
  (`proxy/tests.rs`, three suites), link (`routed/tests.rs`),
  vanish-liveness (`src/testing.rs`, `proxy/tests.rs`,
  `bookmark_causality.rs`, `shadow_validity.rs`).
- `p4-negative-controls`: commit-path (`lib.rs`, `gossip.rs`,
  `typed/*`), proptest-ci (`failures.rs`, `wire_legibility.rs`),
  vanish-liveness (`conformance/link/tests.rs`, `gossip_when.rs`).
- `p4-oracles`: proptest-ci (`proxy/tests.rs`, `stats.rs`,
  `bookmark_when.rs`, `party_conservation.rs`, `session_stats.rs`),
  commit-path (`rumors.rs`, `snapshot.rs`, `tree.rs`, `arb.rs`,
  `typed/node.rs`), link (`routed_link.rs`), vanish-liveness
  (`proxy/tests.rs`, `common/sim.rs`, `gossip_when.rs`).
- `p4-drivers`: vanish-liveness (seven files), link (`routed/*/tests.rs`,
  `routed_link.rs`), proptest-ci (`proxy/tests.rs`, `capacity.rs`),
  commit-path (`batch.rs`, `rumors.rs`, `tree/tests.rs`).
- `p4-bookmark-example`: proptest-ci (`bookmark_when.rs`); and P6.

So P4 launches only after those merges and after P3, from `main`, and
every "surveyed at" count in the briefs is re-derived by grep at that
base. Within P4, waves of at most four builders, prose first (grep
oracles, small hunks, cheap rebases), then the two mechanical code
sweeps, then the refactors, then the test lanes, each later lane
rebasing onto the earlier merges before its gate run:

1. `p4-v1-residue`, `p4-tallies`, `p4-narrative`, `p4-docshape`
   (pairwise they share files but not hunks; merge in that order, each
   rebasing onto the previous).
2. `p4-placement` (after docshape: the error docs), `p4-leftovers`,
   `p4-widths` (after leftovers: both edit `src/tree/typed`).
3. `p4-core-simplify`, `p4-streaming-simplify`, then
   `p4-remote-simplify` (after placement for T99's enum and after
   streaming for the one `start.rs` line).
4. `p4-testdocs`, then `p4-negative-controls` (after remote-simplify:
   remote-adapter-streams-3 before remote-adapter-tests-15, T126),
   `p4-oracles`, and `p4-drivers` (both after testdocs: `faults.rs` and
   `arb.rs`).

`p4-bookmark-example` launches after the P6 lane carrying T62 and T69
merges. P5's harness lane (T115, T131) follows `p4-leftovers` and
`p4-widths`, which both edit `tests/common`; the rest of P5 follows P4
as TRIAGE.md orders.
