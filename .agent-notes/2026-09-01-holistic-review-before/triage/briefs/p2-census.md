<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from ruling 120 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P2 lane: the surface census dissolves into `cargo public-api`

## Goal

The library's public surface is guarded against unreviewed change by a
maintained tool and a committed snapshot, and by nothing we own. Today
three thousand lines of hand tooling (a census over rustdoc JSON, a
`syn`-based extractor cross-checking it, a hand roster of every public
method with per-row dispositions, a coverage suite reconciling the two
both ways, a pinned rustdoc-types format with its own bump procedure)
guard the same hazard and generate their own maintenance cascade; the
review found sixteen defects in them. The invariant restored: a public
item entering, leaving, or changing signature reads red in the gate until
a commit deliberately re-accepts the snapshot and names the change; no
hand-maintained enumeration of the public surface exists anywhere; the
instruments that need the surface as data derive it from the tool's
output.

## Ground rules

The standard ground rules of the P1 briefs apply (base verification,
`git -C` and absolute paths, never EnterWorktree, negative controls
committed with the red quoted, one commit per unit, prose in the present
tense with no reference to code that no longer exists, spaced
double-hyphens in comments, resource discipline, never delete outside the
worktree, report what you could not do). Specific to this lane:

- **Base.** Worktree `/Users/oxide/src/before-p2-census`, branch
  `before/p2-census`, HEAD `1b6df6fd` (the `p1-fuzz` code tip; its
  fuzzfit tiling test is one consumer you rewire). Verify before starting.
- **Commits** are `git -c commit.gpgsign=false commit` (the signing agent
  is down; re-signed at merge). Never push.
- **The tool.** `cargo public-api` (crates.io `cargo-public-api`), being
  installed on this Mac and on the box by the coordinator; if
  `cargo public-api --version` is absent when you need it, install it
  yourself with `cargo install cargo-public-api --locked` (approved). It
  needs a nightly toolchain for rustdoc JSON; use the justfile's pinned
  `nightly_toolchain` (`cargo +nightly-... public-api` or the tool's
  `--toolchain` flag), never a floating nightly. Pin the tool's version
  beside the toolchain pins (the justfile's header names the nightly and
  why; the tool's version and its bump procedure join it, and AGENTS.md's
  install list gains it): the snapshot is compared textually, so a tool
  upgrade that changes the rendering is a deliberate re-accept.
- **The snapshot** is one committed text file under `crates/before/`
  (the tool's own output; name it plainly), produced by a recipe and
  diffed by the gate leg that replaces `surface-totality`: red on any
  difference. Include hidden items and auto-trait impls (the tool's flags
  for `#[doc(hidden)]` and blanket/auto impls; rulings 49 and 91 want
  them visible). Feature set: the public surface under default features
  and, if the tool supports it in one run, the `meter` feature's
  instrument surface as a second snapshot; if not, default features only,
  and say so. The negative control: add a throwaway `pub fn` to
  `lib.rs`, show the leg red with the tool's diff line quoted, remove it.
- **Dissolve, do not port.** Delete `crates/before/surfacecheck` (and its
  lockfile, its `cargo audit` line, its entries in the justfile's
  detached-workspace lists and the `lockcheck` roster), the `surface-scan`
  crate and its workspace dependency, `crates/before/src/surface.rs`,
  `crates/before/src/testing/surface_coverage.rs` and its tests, the
  `surface-totality` and rustdoc-JSON recipes and their CI steps, the
  `rustdoc-types` pin and the justfile header's paragraph about its
  format, and every prose site that describes the roster or the census
  (grep `METHOD_SURFACE`, `FAMILY_SURFACE`, `surfacecheck`, `surface
  roster`, `census`, `surface-totality`, `surface-scan` across the
  workspace, `.github/` and root docs included; `.agent-notes/` is exempt).
  Where the deleted code held knowledge that is still true and useful
  (the exclusion families' reasons for deliberately untested items), that
  is ruling 98's deferred coverage floor: record it in the report, do not
  keep a roster for it.
- **Rewire the two consumers.** `crates/before-fuelscape/src/ops/tests.rs`
  and the fuzzfit harness's exemption tiling test
  (`crates/before/fuzzfit/harness/src/ops.rs` `EXEMPTIONS` and
  `tests/sanity.rs`) walk `METHOD_SURFACE` today. They derive the method
  list from the tool's snapshot instead, through one small adapter that
  parses the snapshot's lines into (type, method) rows (the snapshot is a
  committed file both detached workspaces can read by path; or read the
  rustdoc JSON directly if that is cleaner), with a test that the adapter
  reads the committed snapshot and finds the methods the tests need. The
  `diff_ops` totality pin in `crates/before/src/testing/diff_ops/tests.rs`
  also names `METHOD_SURFACE`: rewire or dissolve it by the same rule (a
  descriptor roster held total against the tool's list, or nothing).
  Every consumer's red-first control stays red-first: for the tiling
  test, a method present in the snapshot with neither an op nor an
  exemption still fails by name.
- **What the snapshot must carry so the consumers work**: check early
  (before deleting anything) that the tool's output lists every inherent
  method the consumers roster today; if something the roster had is not
  in the tool's output (a hidden item without the flag, a macro-generated
  impl), report it before proceeding rather than keeping a hand list for
  it.
- **Verification.** On the box through the wrapper with the two caps, no
  `pset-run`, logs under `<scratchpad>/p2-census/`, polled inside your
  turn: `cargo check --locked --workspace --all-targets` in both feature
  sets, `just fuelscape-test`, `just fuzzfit` (the tiling test), the new
  snapshot leg green and red-first, then one `just gate` under the shared
  mutex (`fuzz` red on the box is the port and counts clean when sole).
  The tool's own run needs the nightly with `rust-src`? Check what it
  needs and add it to `rust-toolchain.toml`'s components only if the
  pinned nightly lacks it. Lockfiles: deleting `surfacecheck` removes one
  of the six lockfiles `lockcheck` counts; update its expectations and
  prose (state the structure, not the count).
- **Annotations.** `.agent-notes/2026-09-01-holistic-review-before/triage/annotations/p2-census.tsv`
  (new; `path\tline\tentry\truling\tnote`, one row per changed region
  against base `1b6df6fd`, entry `ruling-120` where no ledger id applies);
  check with `review.py packet --base 1b6df6fd` until 0 unannotated, 0
  stale; commit last.

## Ordering inside the lane

1. Install and probe the tool: produce the snapshot at the base, confirm
   it lists every method the consumers need; report any gap before step 2.
2. The snapshot, its recipe, the gate leg replacing `surface-totality`,
   the version pin, the red-first control.
3. The adapter and the two rewired consumers (plus `diff_ops`' pin), each
   with its red-first control.
4. The dissolution: `surfacecheck`, `surface-scan`, `surface.rs`,
   `surface_coverage`, recipes, CI, lockcheck, prose.
5. Verification, annotations, report.

## Report

The new tip; per step the commit shas and decisive evidence verbatim
(the tool's version; the red diff line on the planted `pub fn`; the
adapter test's output; each consumer's red-first line; the gate's
verdict); every item the tool's output lacks against the old roster;
what ruling 98's deferred coverage floor still needs; the line count
removed and added; anything you disagree with in this brief, with
evidence.
