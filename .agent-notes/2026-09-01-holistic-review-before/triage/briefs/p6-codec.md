<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane: the codec (bits; base, text, tree, display)

## Goal

The codec's per-module entries, landed per their Resolutions inside an approved roster, under rulings 41 (the width threshold), 43, 64, 84 (the text entries' precedence rule), and 88.

## Awaiting individual ruling

The coordinator is walking these mediums with Finch; nothing below lands for them until the ruling is appended here: codec-bits-30.

## Roster summary

0 ruled (); 1 medium awaiting ruling; 22 pending roster approval (6 low, 16 nit).

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `5328537c` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `5328537c`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the class document in
  `.agent-notes/2026-09-01-holistic-review-before/`; the entry's full
  record (evidence, construction, witness outcome) is under `### <id>:`
  there (`#### <id>:` in `simplification.md`; nits in `documentation.md`,
  `simplification.md`, and `verification.md` are one-line table rows) and
  in `evidence/`. Line anchors are at the reviewed commit `9e5784fb`, and
  the tree outside `.agent-notes/` is byte-identical at your base, so they
  hold; re-anchor from the quoted evidence, never from a line number
  alone, once your own commits move the file.
- **The rulings govern.** `triage/rulings.md` records Finch's decisions.
  Where a quoted Resolution offers alternatives, the ruling named beside
  it picks one; where the ruling amends the Resolution, the amendment is
  stated under the quote and wins. Where a quoted Resolution and the lane
  goal come apart, the goal wins, and the discrepancy is reported.
- **Typed references, never strings (ruling 43).** Wherever this lane
  touches `meter/registry.rs`, a family roster, `TRIPWIRE_ROSTER`, the
  surface rosters, or any test that names another test, file, or line:
  reasons, pins, and enforcement homes are expressed as references the
  compiler resolves (function items, registered law names, `Shape` and
  `Op` values), never as strings naming a test function, a file, or a
  line number. A lane that sees a cleaner idiomatic shape for a roster is
  authorized to adopt it and reports the reshaping in its diff. Finch's
  words: "please make these instruments impossible to drift in the
  future. I *really don't like* the pattern of hard-coded strings and
  Rust source locations embedded in tests; the way these family rosters
  ended up is not really to my taste, but I haven't had time to make it
  more idiomatic and obviously correct. If you see a good way to clean it
  up, please do."
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin the brief does not
  name as moving; any change to a public signature or public rustdoc
  contract the resolution does not name (the meter surface under
  `any(test, feature = "meter")` is instrument surface by ruling 8 and
  not public API, but a change there lands with the `rumors` test update
  in the same commit); anything that contradicts a ruling; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop
  on one entry does not block the others.
- **Negative controls.** Every repaired or added instrument lands with a
  committed demonstration that a known-bad artifact fails it. The
  constructions in `evidence/witness.md` and each entry's Construction
  line are those artifacts; convert each into a committed test
  (`should_panic`, an asserted `Err`, a judge test over synthetic samples,
  or a reversible mutation whose observed failure the commit message
  records verbatim). Ruling 20 is the one place this brief set says
  otherwise, and it says so at the entry.
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement; launch no daemons or services. One full
  `just gate` per agent, before the final commit, run in the background
  redirected to a log under `<scratchpad>/<lane>/` and polled with short
  foreground checks (the foreground command cap is ten minutes; a
  foreground gate run cannot finish). Keep every working file and evidence
  log under that directory, never loose at the scratchpad top level.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and the ruling. Every re-pin is measured
  at the parent and its movement named in the commit. Commit every
  proptest seed file that appears. Prose speaks in the present tense: no
  reference to code that no longer exists, no dated rationale at a
  declaration site. Comments use spaced double-hyphens, never em-dashes;
  every test has a doc comment stating its invariant. Commit with a
  descriptive message before finishing.
- **Never delete anything outside your worktree.** If the disk fills
  (ENOSPC), stop and report; it is the coordinator's problem, not a
  reason to trim caches you do not own.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why, and every deviation from a stated
  resolution, explicitly. Your report is data: the coordinator verifies
  each entry's Acceptance against the tree at the reported sha before the
  ledger records it. Report what you could not do rather than working
  around it.
- **No lower bound on performance is pinned anywhere (ruling 88).** Every
  touch, scan, limb, or heap pin is a ceiling: a reading over it fails; a
  reading under it is an improvement that lands by tightening the ceiling
  with its attribution in the commit. The only floors are liveness floors
  derived from a mechanism's irreducible work, never from a measured
  reading. An entry whose Resolution asks for an exact pin or a measured
  floor is read as a ceiling plus any mechanism-derived floor it names.

## Ordering

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has, or awaits, an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p2-widths` (the dashu threshold), `p7-api` (the three text entries), `p8-performance` (the bit-buffer consolidation), and `p4-rosters`. Owns `src/codec/**`.

## Mediums awaiting individual ruling

Listed with their Resolution so the lane knows the files they touch; not landed until ruled.

### codec-bits-30 (medium, verification): awaiting individual ruling

The BitStack model test names a method that does not exist, omits set_last and trailing_ones, and reaches the spill it advertises about once in a hundred runs

- Owner-gated: no

Resolution: Drop `is_empty` from the doc. Make the spill reachable by construction (bias pushes, e.g. `prop::bool::weighted(0.75)`, or prefix each case with a deterministic ramp of at least 65 pushes) and pin the reach with `prop_assert!(max_height >= 65)` per case. Add `set_last` as a third op kind (model: overwrite `model.last_mut()`) and assert `stack.trailing_ones() == model.iter().rev().take_while(|b| **b).count() as u64` at every step, with runs long enough to cross two spilled words. Acceptance: the extended test is red under each of: stack.rs:118 `if w < 64` to `if w <= 64` (caps the run at one spilled word); 117 `run += u64::from(w)` to `run = u64::from(w)`; 162-163's `words.last_mut()` arm replaced with a no-op; and the testdoc names only methods the body checks. Construction: for the multi-word loop specifically, `let mut s = BitStack::new(); for _ in 0..70 { s.push(true); } assert_eq!(s.trailing_ones(), 70); s.push(false); for _ in 0..3 { s.push(true); } assert_eq!(s.trailing_ones(), 3);` is exercised by no committed test; the `w <= 64` mutation above passes every current suite unless some walk builds a right run deeper than 64 and checks its value.

Awaiting individual ruling: the coordinator is walking the P6 mediums with Finch now. Do not land this entry and do not choose among its alternatives; when the ruling arrives it is appended to this brief by the coordinator.

## Roster members pending Finch's approval

Lows and nits no ruling has reached, placed here by the files they touch. Land only after the coordinator confirms the roster is approved.

### codec-base-text-tree-23 (low, verification): roster: pending Finch's approval

Three test docs describe mechanisms the code does not have: a recursive validator, a test-only entry as the decode path, and an inline spill at `u64`

- Owner-gated: no

Resolution: Reword 884-890 in terms of what is: the validator completes each node's collapsible check on its explicit frame stack, so `(1, 1)` buried under deep nesting is caught at that node's close, not only at the root, and the left spine keeps many ancestors open so the frame stack, not a single tag read, carries the check. At 1626 name `parse_id_core` (the body every id decode entry drives, reached here through `parse_id`). At 85-86 name the boundary crossed: "`u64::MAX + 1` is the first value `to_u64` cannot answer; the integer code and its rendering are unchanged across that word-dispatch boundary". Acceptance: `grep -n recurs src/codec/tests.rs` returns only the deliberately recursive reference parser's passages (1636-1642, 1687); `grep -n parse_id_from src/codec/tests.rs` is empty; the gamma testdoc names the `to_u64` boundary, not inline storage.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-base-text-tree-5 (low, documentation): roster: pending Finch's approval

`msb_cmp_windows` documents a stronger premise than `Rank` holds; the argument that makes the tail rule sound is unwritten

- Owner-gated: no

Resolution: Restate the premise at base.rs:155-157 as "the longer string ends in a set bit", and at rank.rs:889-891 write the one-line derivation: on a class tie, more numerator bits means a larger exponent, so `exp > 0` and the numerator is odd by normalization. Widen the differential to OR only the wider operand with 1 so an even shorter operand is exercised. Acceptance: base.rs and rank.rs carry the class-tie argument; the differential covers an even shorter operand and stays green.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-bits-19 (low, verification): roster: pending Finch's approval

The claimed k = 65 gamma witness is a second k = 64 row

- Owner-gated: no

Resolution: Use `wide(65)` (`m = 2^65 + 1`, `k = 65`) and fix the comment and the testdoc at 15-17; drop the parameter list from dsi.rs:23-25 ("at and across the word seam", letting the test carry the values). Acceptance: for every row comment `k = N`, `(value + 1).bits() - 1 == N`; dsi.rs's module doc names no specific `k`. Construction: `(UBig::ONE << 64) + 1u32` has bit length 65, so `k = 64`; `(UBig::ONE << 65) + 1u32` has bit length 66, so `k = 65`.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-bits-22 (low, simplification): roster: pending Finch's approval

gamma::load_window duplicates BitsView::load_be behind a raw-parts indirection with one caller

- Owner-gated: no

Resolution: Fold `window_int` and `load_window` into `decode_int_window`: `let proven = bits.len().checked_sub(pos)?.min(WINDOW_BITS); if proven == 0 { return None; } let window = bits.load_be(pos, proven as u32) << (WINDOW_BITS - proven); let k = u64::from(window.leading_zeros()); let code_len = 2 * k + 1; if code_len > proven { return None; } let m = window >> (WINDOW_BITS - code_len); Some((m - 1, pos + code_len))`. `load_be`'s debug assert holds because `pos + proven <= len`; `body_tail` keeps its one remaining caller. Acceptance: gamma.rs has one window function; `gamma_window_edge`, `gamma_window_declines_conservatively`, `gamma_word_decode_matches_bit_loop`, `gamma_word_paths_match_on_arbitrary_bytes`, and the borsh differentials pass unchanged.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-bits-7 (low, documentation): roster: pending Finch's approval

ptr_eq's doc misstates why independently frozen empty streams alias

- Owner-gated: no

Resolution: Rewrite both sites: `Bytes::new()` (which `Bytes::from` of a capacity-free empty vector reaches) shares one static empty slice, so independently frozen empty streams *may* read `ptr_eq` true; clone provenance is therefore not what the predicate certifies, only value equality. Acceptance: both sentences use "may" and name the shared static; no claim about dangling pointers remains. Construction: in `codec/tests.rs`, `let e1 = Bits::freeze(BitsBuf::with_capacity(8)); let e2 = Bits::freeze(BitsBuf::with_capacity(8)); assert!(e1.ptr_eq(&e2));` fails under `bytes` 1.11.1 (two live one-byte allocations have distinct pointers), while the committed test's `BitsBuf::new()` pair passes.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-bits-9 (low, documentation): roster: pending Finch's approval

BitsBuf's type doc says the packed-stream builder wraps a BitsBuf; it does not

- Owner-gated: no

Resolution: If codec-bits-12 lands, the sentence becomes true as written. Otherwise re-state: "the packed-stream builder hands its finished bytes to one at `finish`", or delete the parenthetical. Acceptance: the sentence describes the builder's actual relationship to `BitsBuf`.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-base-text-tree-1 (nit, documentation): roster: pending Finch's approval

Small inaccuracies in `Base`'s prose: "every operation records", an ambiguous shift clause, an undocumented `bit`, a ragged wrap

Where: `crates/before/src/codec/base.rs:16-26`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Narrow the quantifier to "every arithmetic, comparison, equality, and hashing operation" and say the O(1) reads and `Display` do not record ...

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-base-text-tree-10 (nit, simplification): roster: pending Finch's approval

`meter_limbs1` and `meter_limbs_solo` are both one-`Base` recorders whose difference lives only in their doc comments

Where: `crates/before/src/codec/base/limb_metered.rs:15-25`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Rename `meter_limbs1` to `meter_limbs_scalar`

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-base-text-tree-11 (nit, simplification): roster: pending Finch's approval

`write_id`'s `sep` parameter has one caller and one value; the separator is already a named constant elsewhere

Where: `crates/before/src/codec/display.rs:16-28`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Drop `sep`; one `SEP` constant in `codec::text`

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-base-text-tree-14 (nit, documentation): roster: pending Finch's approval

`parse_base`'s doc restates the conversion rules `parse_decimal` owns and defines the grammar by an unnamed comparison

Where: `crates/before/src/codec/text.rs:46-54`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): At `parse_base`, keep the grammar only (maximal ASCII digit run after a leading whitespace skip, ended by the first non-digit ...

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-base-text-tree-17 (nit, simplification): roster: pending Finch's approval

Prose texture in the parsers: two private `IdFrame` enums, the stack discipline restated five times, fragment heads, the "X, never Y" figure, and "exactly" as intensifier

Where: `crates/before/src/codec/text.rs:98-120`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Rename the text frame; state the stack discipline once per file; recast the fragments

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-base-text-tree-2 (nit, simplification): roster: pending Finch's approval

The limb denomination `bits.div_ceil(64).max(1)` is spelled independently at several sites, and two `cfg` blocks exist only because `record_wide` takes a raw `UBig`

Where: `crates/before/src/codec/base.rs:64-69`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One `limbs_of_bits` in `limb_meter`; drop the two `cfg` blocks

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-base-text-tree-21 (nit, simplification): roster: pending Finch's approval

`parse_id`'s `pos` parameter is always 0, and three named layers wrap one grammar body

Where: `crates/before/src/codec/tree.rs:35-60`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One always-compiled `parse_id_from`; shrink the re-exports

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-base-text-tree-22 (nit, simplification): roster: pending Finch's approval

Long qualified paths beside existing imports, and two `DefaultHasher` helpers, in the codec test suite

Where: `crates/before/src/codec/tests.rs:14-17`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Extend the imports; one hasher helper

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-base-text-tree-24 (nit, simplification): roster: pending Finch's approval

Em-dashes in `//` code comments across the partition

Where: `crates/before/src/codec/tests.rs:214-222`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Recast each em-dash; batch with the crate-wide sweep

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-base-text-tree-27 (nit, verification): roster: pending Finch's approval

The reference id parser's `RefCur`/`RefIdKind` duplicate `text::Cur`/`IdKind` byte for byte without saying why (the tokenizer freeze is unstated and only a space exercises whitespace).

Where: `crates/before/src/codec/tests.rs:1656-1683`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): State the freeze at the header and add a second whitespace byte, or reuse `Cur`.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-base-text-tree-28 (nit, simplification): roster: pending Finch's approval

Dead `continue` at the end of the exhaustive odometer's outer loop

Where: `crates/before/src/codec/tests.rs:1794-1797`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete the trailing `continue`

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-base-text-tree-4 (nit, simplification): roster: pending Finch's approval

`Base::msb_cmp` is a one-caller wrapper whose body the sibling match arms already spell inline

Where: `crates/before/src/codec/base.rs:97-106`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete `Base::msb_cmp`; spell the four arms uniformly; reword the doc

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-bits-14 (nit, simplification): roster: pending Finch's approval

reserve loops over 32-bit chunks for a width that is always 2

Where: `crates/before/src/codec/build.rs:127-137`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `reserve(width: u32)` with one `append_bits`

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-bits-17 (nit, simplification): roster: pending Finch's approval

Idiom nits in the word-parallel cursor and the padding judge

Where: `crates/before/src/codec/dsi.rs:227-227`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Use the `From<Truncated>` impl; `read_word` via `read_word_opt`; the listed one-liners

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-bits-21 (nit, simplification): roster: pending Finch's approval

code_int and code_int_small share a body

Where: `crates/before/src/codec/gamma.rs:69-101`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `code_int` dispatches to `code_int_small` on `to_u64()`

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### codec-bits-24 (nit, documentation): roster: pending Finch's approval

literal.rs is the one production module in the partition without a module doc

Where: `crates/before/src/codec/literal.rs:1-3`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Add a one- or two-sentence `//!` doc: the id tree's in-memory constructors (`id_leaf` ...

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

