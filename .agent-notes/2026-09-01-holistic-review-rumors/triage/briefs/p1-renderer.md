<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the capture renderer's injectivity

## Goal

The wire snapshot discipline rests on the capture renderer being
injective: two different wire byte strings render differently, so a
snapshot diff is a wire diff. At the reviewed commit the renderer collapses
any container-shaped or protocol-tagged map key to a bare `…`, so two
supply records differing only inside a tuple key render identically
(demonstrated), and no committed test samples the claim. Ruling T5 also
folds AGENTS.md's renderer-vocabulary re-accept class into the one
format-change class, because its witness names hexdump lines the corpus no
longer has. The invariant restored: the renderer is injective, a committed
proptest holds it so, and the hard rule that governs re-accepts is one a
reviewer can apply.

## Ground rules

These apply to every P1 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `0926fe32` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `0926fe32`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the topic document in
  `.agent-notes/2026-09-01-holistic-review-rumors/`; the entry's full record
  (evidence, construction, demonstration) is under `### <id>:` there and in
  `evidence/`. Line anchors are at the reviewed commit `9e5784fb`; re-anchor
  from the quoted evidence, never from the line numbers.
- **Goal beside mechanism.** Where a quoted resolution and the goal above
  come apart, the goal wins, and the discrepancy is reported.
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin; any change to a public
  signature or public rustdoc contract the resolution does not name;
  anything that contradicts a ruling in `triage/rulings.md`; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop on
  one entry does not block the others.
- **Negative controls.** Every repaired instrument lands with a committed
  demonstration that a known-bad artifact fails it. The constructions in
  `evidence/witness.md` and each entry's Construction line are those
  artifacts; convert each into a committed test (`should_panic`, an asserted
  `Err`, or a reversible mutation whose observed failure the commit message
  records verbatim).
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement. One full `just gate` before each commit,
  run in the background redirected to a log under
  `<scratchpad>/p1-renderer/`, polled with short foreground checks (the
  foreground command cap is ten minutes). Keep every working file under that
  directory.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and ruling. Commit every proptest seed
  file that appears. Prose speaks in the present tense: no reference to code
  that no longer exists, no dated rationale at a declaration site. Comments
  use spaced double-hyphens, never em-dashes; every test has a doc comment
  stating its invariant. Never delete anything outside your worktree; if the
  disk fills, stop and report.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why. Your report is data: the coordinator
  verifies each entry's Acceptance against the tree at the reported sha
  before the ledger records it. Report what you could not do rather than
  working around it.

## Ordering inside the lane

1. `remote-capture-atlas-17` first: the proptest is committed failing on
   the current tree (it is the negative control), then
2. `remote-capture-atlas-13` makes it pass, in the next commit, then
3. the AGENTS.md rewording (`prose-hygiene-6`, `verification-infra-15`).

## Members

### remote-capture-atlas-17 (medium): ruling T9

Resolution: Add a proptest with a `Node`-shaped strategy (bounded depth and size; uints, nints, bytes, text, arrays, maps with arbitrary keys, tags including the protocol's named tags and 24/63 over byte strings), encoded through `cbor::write_head`. Either (a) write a small inverse parser from the rendering's grammar back to bytes and assert round-trip, which pins injectivity outright, or (b) the cheaper mutation form: generate an item, mutate exactly one scalar leaf anywhere (map keys included), and `prop_assert_ne!` the two renderings through `render_item`; cover `Naming::Listing` and the `"listing"` key context so `render_listing` is exercised. Acceptance: the proptest is committed with a doc comment stating the injectivity invariant; it fails on the current tree and passes once finding 13 is fixed; any shrunk seed rides along in `proptest-regressions`.

Amendment (T9): form (a), the inverse parser, is the ruling; form (b) is
not landed. The witness pass's form-(b) run and its seed
(`evidence/witness.md`, section `remote-capture-atlas-17`) show the shape
of the failing case.

### remote-capture-atlas-13 (high): ruling T5

Resolution: Never elide. When `scalar(key)` is `None`, render the entry in block form: a `key =>` line preceded by `render_node(key, Naming::Plain, deeper, depth + 1, out)` (or a `/ key /`-annotated block), then the value block or inline value. Apply the same at line 523 (a listing key that is not a uint is already off-grammar; render it fully and let a key-shape verdict carry the diagnosis, see finding 14). `Node` does not retain its byte span, so block-form keys are the local fix; routing container-keyed maps through `fallback` with exact bytes is the alternative if span retention is added. Then tighten the module doc's fallback list to match. Acceptance: the construction below fails before the fix and passes after; `grep -n '"…"' capture.rs` is empty; the existing snapshots are byte-identical (`just test-all`).

"Finding 14" is `remote-capture-atlas-14`, a simplification entry outside
this lane; do not land its key-shape verdict here. The entry's own
verification found no committed snapshot containing `…`, so the fix should
move no pin; the acceptance requires `just test-all` to show every
snapshot byte-identical. If any snapshot moves, that is the T5 stop: do not
re-accept, report the moved files.

### prose-hygiene-6 (medium) and verification-infra-15 (low): ruling T5, resolutions replaced

Both entries' resolutions (restate the witness in the render's own terms;
give it a mechanical form as a `tools/snapwitness`) are superseded by T5:
the renderer-vocabulary re-accept class is removed from AGENTS.md's hard
rules, leaving one class, the deliberate, owner-ruled, named re-accept. No
tool is written. The edit is to the snapshot paragraph of AGENTS.md's Hard
rules only: delete the sentences from "One further sanctioned re-accept
class" through the renderer-change attribution clause, and leave the
bookmark fixture re-pin class and everything else as written. The bookmark
pins' hex-line sentence at `src/bookmark/format/tests.rs` is accurate and
stays.

Acceptance: AGENTS.md names no hexdump witness and no renderer-vocabulary
class; the remaining paragraph reads as one rule; `just gate` is clean.

## Hazards and stops

- AGENTS.md is Finch's prose. The edit is the deletion T5 authorizes and
  nothing else; report the exact diff.
- A `Node`-shaped strategy that reaches the protocol's named tags must not
  make `render_item` panic on well-formed input; a panic found by the
  generator is a finding, reported as a stop, not caught by narrowing the
  strategy.
