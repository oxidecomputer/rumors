<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as lane briefs derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. -->

# P1 lane briefs

One brief per lane for the instruments phase of the before and suanpan
triage. Every lane is based on `0fc1921e` (main, the commit recording the
S1 rulings; the tree outside `.agent-notes/` is byte-identical to the
reviewed commit `9e5784fb`, so every line number in the class documents
holds) and is governed by `../rulings.md`. Each brief quotes its members'
Resolution and Acceptance verbatim from the class documents and names the
ruling that governs each, with any amendment stated beside the quote. A
lane agent reads only its brief; the brief carries the ground rules in
full.

Rulings 1 to 28 have individually ruled every high and medium in these
lanes. Lows and nits inside a roster are swept per their entries under the
same rulings; a lane agent that finds a low or nit whose stated resolution
conflicts with a ruling or with a sibling entry reports it rather than
choosing.

## Lanes

| Brief | Rulings | Touches | Size |
|---|---|---|---|
| `p1-gate.md` | 24, 28, 16, 25, 18 (retirement half) | `justfile`, `.github/workflows/ci.yml`, `deny.toml`, `rust-toolchain.toml`, `.cargo/mutants.toml`, `tools/mutantcheck*`, `AGENTS.md`, the six lockfiles, `fuzzfit/harness/src/ops.rs` | medium |
| `p1-survivors.md` | 18 (code half) | `src/codec/build.rs` tests, `src/codec/bits.rs` tests, `src/version/skyline/watermark/tests.rs`, `crates/suanpan/src/accumulator.rs` and its metered tests | small |
| `p1-harness.md` | 4 | `crates/before/tests/meter.rs` | large |
| `p1-board.md` | 11, 12, 13, 9 | `src/meter/board/{judge,ops,floors,ceilings,shard,worst}.rs` and `board/tests.rs`, `worst-cases` pins, two band docs in `tests/meter.rs` | large |
| `p1-fuzz.md` | 14, 15, 17 | `crates/before/fuzzfit/**`, `crates/before/fuzz/**`, `crates/before/wasm32-pins/**`, the `fuzz-build` and `fuzzfit` recipes | large |
| `p1-suites.md` | 19, 20, 21, 22, 27, 10 | `src/meter.rs`, `src/testing/asymptotics.rs`, `src/laws.rs`, `tests/{answer_embedded,fold_skeleton,coincident_span}.rs`, band docs and reset sites in `tests/meter.rs`, `src/meter/registry.rs`, suanpan's metered tests, fuzzfit (ruling 10) | large |

## Grouping decisions

- **Ruling 18 is split across two lanes.** The coordinator's proposal put
  the two surviving-mutant tests (codec-bits-15, skyline-watermark-24)
  and the suanpan exclusion dissolution (suanpan-40) in the gate lane
  with the roster retirement. They are in `p1-survivors.md` instead, for
  two reasons. First, they are library test work (bit-arithmetic model
  tests, a watermark directed pin, exact touch pins in suanpan) with no
  file in common with the gate lane's CI and recipe edits. Second, there
  is an ordering hazard between them: suanpan-40's `read_digits`
  restructure removes two `>>=` codepoints, which changes the listed
  mutant count that the `mutantcheck` gate leg compares against
  `tools/mutantcheck-expected.json`. While that leg exists, the survivors
  lane's gate run would fail on a count it must not edit (the roster is
  retiring, not being re-pinned). So the survivors lane runs after the
  gate lane's retirement commit lands, or rebases onto it before its
  gate run. Splitting them makes that dependency a launch-order fact
  rather than an ordering the one agent has to discover.
- **skyline-query-9 (ruling 10) sits in the suites lane as proposed, and
  the coordinator should consider moving it.** Its instrument is a
  multi-scale fuel fit in the fuzzfit harness, the same harness the fuzz
  lane re-pins under ruling 14. Two lanes adding bands to
  `fuzzfit/harness/src/bands.rs` concurrently will conflict at the pin.
  The brief tells the suites lane to land that entry last and to rebase
  onto `p1-fuzz` if it has landed; moving the entry into `p1-fuzz`
  removes the hazard outright.

## Independence and launch order

`tests/meter.rs` is the shared file. The harness lane rewrites its four
harnesses into one; the board lane edits two band docs in it (envelopes-a-16,
envelopes-b-4); the suites lane touches every reset site in it (ruling 19),
three band docs (ruling 20), and folds three satellite binaries into it
(ruling 22). Run the harness lane first and alone against that file; the
board and suites lanes rebase onto it before their gate runs. If the
harness lane is still running when the others are ready, the others may
land everything outside `tests/meter.rs` and hold their `meter.rs` commits
for the rebase.

Two further orderings from the rulings and TRIAGE.md:

- Ruling 24 (the coverage leg's reproduction) runs before any change to a
  heap pin under coverage. The harness lane's one-time re-measure of the
  three-column tables adds limb, scan, and touch pins to rows that lack
  them; it does not move any heap pin, so it may run concurrently with
  the gate lane. Any heap pin the harness lane finds itself moving is a
  stop.
- Decision 42 (the segments column, ruled in S4) precedes any envelope
  edit that touches segments cells. The harness lane carries the segments
  column through unchanged and re-derives no segments cell; if
  "every column pinned on every row" would require pinning a segments
  cell that is unpinned today, that is a stop.

Recommended order, with waves of at most four builders and the disk
checked before each wave:

1. `p1-gate`, `p1-harness`, `p1-fuzz`
2. `p1-survivors` (after the gate lane's roster retirement lands),
   `p1-board` (rebased onto harness), `p1-suites` (rebased onto harness;
   its ruling 10 entry after `p1-fuzz` lands)

The rumors triage runs lanes in the same workspace. Lanes from the two
plans do not run concurrently against `just gate`; the coordinator
sequences them (ruling 5).

## What the coordinator does with a report

A lane's report is data. For each entry the coordinator runs the
Acceptance against the tree at the reported sha, then writes the sha into
`../ledger.tsv` (the disposition is already `fix` or `fix-amended` from
the ruling; the sha is what makes it terminal). Entries reported as
stopped stay pending and go to Finch as a numbered block. Merge is by
reported sha, never by branch name. Every re-pin in a landed lane is
checked for the attribution the rulings demand: the movement named in the
commit, measured at the parent.
