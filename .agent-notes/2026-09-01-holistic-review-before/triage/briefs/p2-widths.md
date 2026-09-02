<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P2 lane: widths, caps, and the 32-bit boundary

## Goal

`lib.rs` promises correctness for all input sizes, and the review found
five places where a fixed-width cell or a platform-width guard makes that
false: a `u32` link index that panics on a valid multi-GiB decoded input,
a `u32` freeze count behind an expect that asserts rather than argues, a
fold index that silently changes discipline past 2^32 bits, a literal
door quadratic in nesting depth against a documented `O(n)`, `forks`
yielding one share fewer than asked at `u64::MAX`, digit-position
arithmetic that wraps to a silent wrong value on 32-bit release builds,
and a gamma width guard thirty-two values looser than the backend it
guards. The invariant restored: every structure is bounded by allocatable
memory alone; every documented bound is the bound the code has; and every
32-bit boundary has a red-first pin in the `wasm32-pins` guest.

## Ground rules

These apply to every P2 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal the SHA the coordinator names
  at launch, which is `bba0e31a` or a main commit above it (the tree
  outside `.agent-notes/` at `bba0e31a` is byte-identical to the
  reviewed commit `9e5784fb`; a later base carries landed P1 lanes, and
  the coordinator lists which). Run `git -C <worktree> rev-parse HEAD`.
  If HEAD is an ancestor of the named SHA, fast-forward; if it has
  diverged, stop and report. Never call EnterWorktree; operate on the
  worktree through `git -C <path>` and absolute paths, one shell
  invocation at a time.
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
  records verbatim). Ruling 20 waived it for four P1 band docs and nowhere in P2.
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

## Ordering inside the lane

1. The wasm32 pins first, red at your base, in the `wasm32-pins` guest
   and harness: suanpan-24's constructions (the guest gains a suanpan
   export), codec-bits-23's bit-cursor stream, and the `forks(1u64 <<
   32)` pins of clock-17 and party-13 if their roster is approved. The
   leg runs in CI under ruling 16 once `p1-gate` lands; until then run
   `just wasm32-pins` locally, once per pin commit, and say so in the
   commit.
2. suanpan-24 (ruling 39) and codec-bits-23 (ruling 41), each turning
   its pin green.
3. The `u32` caps (ruling 33): `memo.rs`'s link index and `web.rs`'s
   freeze count widened; the fold index's representation chosen by a
   parent measurement of the board's `party_join_all` heap reading;
   `build_unindexed` and the fallback differentials dissolved or
   converted. `p2-cures` rebases its memo-heap representation onto this.
4. The one-pass literal emit (ruling 34).
5. `forks` exactly `k` at `u64::MAX` (ruling 35), with `tests/forks_max.rs`
   re-pinned to the new boundary.

## Members

### suanpan-24 (high, correctness): ruling 39

Resolution: compute every landing position through one checked helper wired to the documented panic (the natural home is suanpan-12's `split_shift`): `digit_shift.checked_add(offset).expect("digit positions fit a usize")`, with `2 * limb_index` via `checked_mul` and `position.checked_add(1)` in `deposit_value`; or compute positions in `u64` and `usize::try_from` once per `add_at`. Restate the `# Panics` text in terms of the landing position (`shift / 32` plus the operand digit's offset). Acceptance: a red-first pin on a 32-bit target (extending crates/before/wasm32-pins' guest, which names suanpan nowhere today, or a new guest) observes the documented panic message for the constructions below; the 64-bit differential and ledger suites unchanged. Construction (32-bit target, release profile): (1) `let mut a = Accumulator::new(); a.add_limbs_shl([0u64, 0, 5], 32 * (u64::from(u32::MAX) - 3));`: `digit_shift = 2^32 - 4` passes `try_from`; limbs 0 and 1 are skipped (`low == high == 0`); limb 2's `low = 5` lands at `2 * 2 + (2^32 - 4) = 2^32`, which wraps to 0, so `a.sign_limbs()` returns `(Greater, vec![5])` and `a.digit_count()` is 1. (2) `a.add_u64(1); a.shl(64); a.shl(32 * (u64::from(u32::MAX) - 1));`: the held digits `[0, 0, 1]` skip offsets 0 and 1 in `fold_accum` and offset 2 wraps, so the value reads as 1. (3) The boundary sibling: at `shift = 32 * (2^32 - 1)`, `resize(pos + 1, 0)` at 1371 computes `usize::MAX + 1`, wraps to `resize(0)`, and `self.digits[pos]` at 1374 panics with an index-out-of-bounds: still the panic class, but not the documented message. In debug builds every case panics with "attempt to add with overflow".

Ruled (39): the second option: positions computed in `u64` and
converted with one `usize::try_from` per `add_at`, wired to the
documented panic, with `# Panics` restated in terms of the landing
position. The red-first pin rides the `wasm32-pins` guest, which gains a
suanpan export for the entry's three constructions; the 64-bit
differential and ledger suites are unchanged. Construction (3), the
boundary sibling that panics with an index-out-of-bounds rather than the
documented message, is covered by the same conversion; assert the
documented message on all three.

### codec-bits-23 (medium, correctness): ruling 41

Resolution: Replace both `usize::try_from(k)` guards with one shared predicate at the backend's cap: with `W = usize::BITS as u64` (dashu's `Word` is `usize`-wide on 32- and 64-bit targets), a `k`-bit mantissa needs `k / W + 1` words and the backend holds at most `usize::MAX / W`, so reject when `k / W >= usize::MAX as u64 / W` (which subsumes the `try_from` failure); state the derivation inline and the dependence on dashu's `Word` width beside the existing "bumping that dependency is a breaking change" rule; amend pins.rs:138-141 to name the borsh path. Pin it under `cfg(target_pointer_width = "32")` (or in the wasm32-pins guest) with a test-only `BitCursor` that yields `2^32 - 32` zero bits then a `1` without materializing them, asserting `Err(Decode::NotCanonical)`. Acceptance: on wasm32 the pin returns `Err(Decode::NotCanonical)` where today it panics with dashu's `assertion failed: self.len < self.capacity`; both arms route through the one predicate (`grep` shows a single `usize::MAX` division site); the 64-bit differential suites are unchanged.
Construction: target wasm32 (`usize` 32 bits, `Word = u32`, `MAX_CAPACITY = 2^27 - 1`). Borsh-deserialize a `Version` from a reader yielding the skyline stream `1` then `k = 2^32 - 32` zero bits then a `1` (about 512 MiB of `0x80, 0x00 ...`): `ReaderCursor::read_int` (borsh_impls.rs:125) falls to `decode_int_from`; `k < 64` is false; `usize::try_from(2^32 - 32)` is `Ok`; `m.set_bit(k)`: `idx = 2^27 - 1`, `Buffer::allocate(2^27)` clamps capacity to `2^27 - 1`, `push(lo)`, `push(hi)`, `push_zeros(2^27 - 3)` passes its `assert!(n <= capacity - len)` exactly, then `push(1 << (k % 32))` fails `assert!(self.len < self.capacity)`. Any `k` in `[2^32 - 32, 2^32 - 1]` works; `k = 2^32 - 33` and below allocate and succeed. The word-parallel path reaches dsi.rs:267 identically given a stream of `2k + 1` live bits.

Ruled (41), Finch's question answered: the existing reject
(`Decode::NotCanonical` for a width the backend cannot hold) is the
mechanism; keep it and derive its threshold from `usize::BITS` exactly as
the resolution states, so both readers route through one predicate and
reject exactly what dashu cannot represent on the running platform. The
wasm32 pin uses a test-only bit cursor yielding `2^32 - 32` zeros then a
`1` without materializing them, red at your base (dashu's assert), green
after (`Err(Decode::NotCanonical)`). `pins.rs:138-141` names the borsh
path.

### inventory-1 (medium, correctness): ruling 33

Resolution: widen the link index so the ledger is bounded only by memory like every other structure on the walk (`Option<NonZeroUsize>` or `NonZeroU64` in `queue`; the const assert at memo.rs:97 moves with it), or keep the cap and add a `# Panics` section to `Version::tick`/`ticks`, `Party::tick`/`ticks`, and `Clock::tick`/`ticks` stating the bound. Acceptance: either the `expect` is gone and the contract is memory alone, or the public docs state the cap. The same u32 shape recurs at web.rs:371 (`expect("freeze count fits u32")`); the sweep estimates its input requirement at roughly 128 GiB, which I did not verify.

Ruled (33): widen; the first branch. The `expect` goes and the
contract is memory alone; no `# Panics` clause is added. The const assert
at `memo.rs:97` moves with the width, and the memo rows' heap columns are
re-pinned with attribution (the cell grows). The `web.rs:371` twin is
skyline-query-24, same ruling.

### skyline-fill-grow-23 (low, correctness): ruling 33

Resolution: Owner's call between (a) widening the index to `Option<NonZeroUsize>` (8 bytes per queue cell on 64-bit, still niche-packed; update the const assert at memo.rs:97 and the "one index-sized cell per site" claim; re-pin the memo rows' heap columns with attribution) so the only cap is allocatable memory, and (b) keeping the cap and stating it as a capacity contract in `fill::tick`/`ticks`'s `# Panics`, with `Version::tick` and `Party::tick` pointing at it. Either way, the `expect` message should name the capped quantity: "nonzero link count fits u32". Acceptance: no `u32::try_from(...).expect` on the ledger path, or the public tick doors' docs name the cap.

Ruled (33): option (a), one change with inventory-1 and recursion-5;
the "one index-sized cell per site" claim updated; no capacity contract
is stated in any public tick door's docs.

### recursion-5 (low, correctness): ruling 33

Resolution: Either widen the queue cell to `Option<NonZeroUsize>` (re-pin the affected heap envelopes and the const assert at memo.rs:97), or make the pre-scan launch a fresh scan when the link store would exceed u32 (a scan-span cap mirroring `IdIndex`'s unindexed arm), or rule the corner tolerable and document it at `set_link` with its true reachability (a multi-GiB operand pair and a link store of 2^32 accumulators), replacing the undefined "link-storage contract" at prescan.rs:691-692 with that ruling. Acceptance: either no u32 capacity panic remains on the tick path for any decodable input, or the corner is documented at `set_link` with its reachability and an owner ruling, and prescan.rs cites that ruling.

Ruled (33): the first option (widen); neither the scan-span cap nor
the documented corner. The undefined "link-storage contract" at
`prescan.rs:691-692` is replaced by the sentence that the ledger is
bounded by memory alone.

### skyline-query-24 (low, correctness): ruling 33

Resolution: Make `Reign::epoch` and `epoch()` `usize` (or `u64`), drop the `try_from`/`expect`, and index `refs` with it directly (`Reign` is a heap-owned per-boundary payload, so the width change costs at most a few bytes of alignment per stacked boundary). If the owner instead rules a 2^41-bit operand infeasible, rewrite the message as the argument ("a freeze costs at least 2^9 stream bits, so 2^32 freezes need a 2^41-bit stream"). Either way, reword the doc: "The current epoch: the drifts parked so far (a freeze that finds no drift keeps the epoch)." Acceptance: no `u32` conversion remains on the epoch path, or the expect reads as a proof from a stated bound; the two method docs agree; min_ticks value pins unchanged.
Construction: A right spine of 2^32 + 1 two-leaf blocks whose heights step by `+2^288` then `+1`: after the `+2^288` fold `live.digit_count() = 10` and `int_digits = 10` (no trigger); after the `+1` fold `10 > 1 + 8` fires `EpochLedger::freeze` with nonzero drift, pushing one epoch per block. At the 2^32-th block's leaf, `ledger.epoch()` at query.rs:467 fails the `u32::try_from`. Stream size about 2^32 · 580 bits; construct through `Version::decode` on a 64-bit machine with enough memory rather than running it at test scale.

Ruled (33): `Reign::epoch` and `epoch()` become `usize`, the
`try_from`/`expect` goes, `refs` is indexed directly, and both method
docs are reworded as the resolution states; min_ticks value pins
unchanged.

### party-23 (medium, claim): ruling 33

Resolution: Owner's call between keeping the order at every size (`enum Rights { Narrow(Vec<u32>), Wide(Vec<u64>) }` chosen by `bits.len()`, or `Vec<u64>` unconditionally if the board's `party_join_all` heap reading tolerates it; measure at the parent first), dissolving the fallback arm, `build_unindexed`, and the fallback differentials or converting the latter to a wide-table differential, or carrying the size clause in the island contract and `join_all`'s rustdoc. Acceptance: either `is_disjoint` has no unindexed arm and the deep and arbitrary index differentials pass against a forced wide table, or the public contract names the threshold.
Construction: any accumulator over 2^32 bits with `k` one-byte overlapping probes reads `Θ(k·|self|)` scan bits under the current code where the contract predicts `Θ(k log |self|)`; unaffordable to materialize in a test (as the field doc says), which is why the finding is a contract clause rather than a failing test.

Ruled (33): keep the order at every size. Measure the board's
`party_join_all` heap reading at the parent, then choose between `enum
Rights { Narrow(Vec<u32>), Wide(Vec<u64>) }` by `bits.len()` and
`Vec<u64>` unconditionally by that reading (the narrow-or-wide table if
the unconditional one moves the reading over its ceiling; say which and
show both numbers); dissolve the fallback arm and `build_unindexed`, and
convert the fallback differentials to a forced-wide-table differential so
the wide arm is exercised at test scale. No size clause enters the island
contract or `join_all`'s rustdoc.

### party-11 (medium, claim): ruling 34

Resolution: Delete the `validate_id` call in `id_node` (keep the two local collapse checks; validate once in `finish_id` if a belt is wanted) and either restate the bound as `O(n·d)`, `d` the literal's nesting depth, or thread one `IdBuilder` through `PartyLiteral::into_id_bits` (reserve/patch/close per level, one pass, truly `O(n)`). Acceptance: a left-nested literal of depth `d` performs one validation pass in total (scan-meter reading about `2·nodes` bits, not a sum over levels); the `# Complexity` section states the bound the code implements; `parse_bare_notation` and the text round-trip laws stay green.
Construction: under `--features scan-meter`, build `Party::try_from(((((1u8, 0u8), 0u8), 0u8), 0u8))` and its depth-8 and depth-16 extensions, reading `scan_bits()` around each: level `i` copies and re-parses about `2i` bits, so the reading grows about quadratically in `d` for a `2d`-bit result where an `O(n)` door reads linearly.

Ruled (34): the one-pass door: one `IdBuilder` threaded through the
sealed, doc-hidden `PartyLiteral::into_id_bits` (reserve, patch, close
per level), the per-level `validate_id` deleted, `# Complexity` stating
`O(n)` because the code now delivers it. The scan-meter acceptance (a
depth-d left-nested literal reads about `2·nodes` bits) is the pin;
`parse_bare_notation` and the text round-trip laws stay green.

### codec-base-text-tree-13 (medium, claim): ruling 34

Resolution: Owner's choice between (a) restating both rustdocs as `O(n · d)` with d the literal's nesting depth and dropping the per-level `validate_id` (children are normal by construction; the two collapsibility checks are the whole normal-form rule at a node), leaving only the copying; or (b) reshaping the sealed, `#[doc(hidden)]` `PartyLiteral` to emit top-down into one shared `BitsBuf` (reserve the tag, emit children, patch the tag, exactly `text.rs`'s `parse_id_tree` discipline), validating once at the root, and keeping `O(n)`. Recommend (b): the trait is sealed, so it changes no reachable surface, and the claim stays true. Acceptance: under `scan-meter`, `before::meter::scan_bits()` across `Party::try_from(spine(d))` at d = 32 and d = 64 reads a ratio near 2, or the rustdoc says `O(n · d)` and the ratio near 4 is the documented behavior; either way `id_node` no longer re-parses a subtree its own two checks already classified.

Ruled (34): option (b), one change with party-11; the scan-meter
ratio near 2 across d = 32 to 64 is the committed pin.

### clock-14 (low, claim): ruling 34

Resolution: Either restate both bounds as `O(m · d)` with `d` the literal's nesting depth, or have `node` fold children in one pass carrying only the running last height (no `Vec<Base>` per level) so `O(m)` becomes true. Acceptance: the doc names the depth factor, or a scan-meter test over a depth-doubling left-deep literal reads flat per built byte.
Construction: `Version::try_from((0u64, (0u64, (0u64, … (0u64, 0u64, 1u64) …, 2u64), 3u64), 4u64))` nested `d` deep; `node` at level `i` scans a left stream of `i` leaves, total `Σ_{i≤d} i = Θ(d²)` for a `Θ(d)`-leaf stream.

Ruled (34): the second option (one pass carrying only the running
last height, no `Vec<Base>` per level) so `O(m)` becomes true for the
version composer too; the scan-meter test over a depth-doubling
left-deep literal reads flat per built byte.

### skyline-coding-20 (nit, claim): ruling 34

Resolution: state the constant in the `# Complexity` section ("each nesting level of the literal rescans its children, so the constant is the literal's static depth"), or build the literal in one pass by lowering the tuple to an iterative walk that feeds `SkylineBuilder` directly (the parse kernel already does this for text), which also retires `scan()`'s `Vec<Base>`. Acceptance: the `TryFrom<(u64, T, S)>` complexity text matches a stated derivation; if the one-pass builder lands, the literal doctests and the text/tests.rs corpus stay green.

Ruled (34): the one-pass builder (the second option), retiring
`scan()`'s `Vec<Base>`; the literal doctests and the `text/tests.rs`
corpus stay green.

### version-core-16 (nit, claim): ruling 34

Resolution: state `O(m · d)` with `d` the literal's nesting depth (each level re-derives its children's absolute heights from their streams), or restructure `literal::node` to compose without re-scanning. Acceptance: the doc names the depth factor, or `literal::node` no longer re-scans and `descending_literals_build_the_oracle_tree` stays green.

Ruled (34): `literal::node` no longer re-scans (the second option);
`descending_literals_build_the_oracle_tree` stays green.

### clock-3 (medium, claim): ruling 35, resolution replaced

Resolution: Restore one sentence to the public docs of `Clock::forks`, `Forks`, `Party::forks`, and `party::Forks`: the iterator yields `k` children except at `k == u64::MAX`, where the residual consumes the count's headroom and `u64::MAX - 1` are yielded. Reword "keeps the last share" to "keeps the first (residual) share of the balanced split". Point `tests/forks_max.rs`'s module doc at the public sentence. Acceptance: the four public doc blocks state the boundary and the residual's position; `Forks` no longer says "exactly `k`" unqualified; the pin's "documented behavior" resolves to a public sentence.
Construction: `let mut c = Clock::seed(); assert_eq!(c.forks(u64::MAX).len() as u64, u64::MAX);` fails on a 64-bit host with `u64::MAX - 1`, exactly what tests/forks_max.rs:49-50 asserts, against the `Forks` doc's "exactly `k`".

Ruled (35), the behavior changes: `Clock::forks` and `Party::forks`
yield exactly `k` children for every `k`, `u64::MAX` included. The
resolution's restored boundary sentence is not written; instead the four
public doc blocks keep "exactly `k`" unqualified and true, the parameter
is named `k`, and "keeps the last share" is reworded to name the
residual's true position. `tests/forks_max.rs` pins `forks(u64::MAX)`
yielding `u64::MAX` shares (count them without materializing: the
iterator's own counter, or `len()` on 64-bit) and its module doc names
the public sentence. This is an owner-directed behavior change on the
stable surface: the commit message says so and names ruling 35. If the
count representation cannot hold `u64::MAX` children without a wider
counter, report the design you chose (a `u128` count, or a residual
folded into the last share) with its cost before landing.

Superseded by ruling 82 (`p7-api.md`): `Clock::forks` and `Party::forks` take the count as `usize`, `ExactSizeIterator` is unconditional, and ruling 35's boundary reads at `usize::MAX`. Do not land this entry from this brief; `p7-api` owns the forks change (this lane's `p2-widths` members clock-3, tests-other-17, party-14, api-audit-10, clock-17, and party-13 all move there).

### party-14 (medium, documentation): ruling 35, resolution replaced

Resolution: One sentence on `Forks`' doc, with a pointer from `Party::forks`, and the same on `clock::Forks`/`Clock::forks`: "The count is `k` for every `k < u64::MAX`; at `u64::MAX` it saturates to `u64::MAX − 1`, because the borrowed party must keep one share." (cdad46060's wording is a usable reference.) Acceptance: `cargo doc` renders the saturation clause on both iterators and both methods; tests/forks_max.rs:1 cites public prose that exists.

Ruled (35): no saturation clause is written, because there is no
saturation to document after clock-3's change. `cargo doc` renders
"exactly `k`" on both iterators and both methods, and `tests/forks_max.rs:1`
cites public prose that exists.

Superseded by ruling 82 (`p7-api.md`): `Clock::forks` and `Party::forks` take the count as `usize`, `ExactSizeIterator` is unconditional, and ruling 35's boundary reads at `usize::MAX`. Do not land this entry from this brief; `p7-api` owns the forks change (this lane's `p2-widths` members clock-3, tests-other-17, party-14, api-audit-10, clock-17, and party-13 all move there).

### tests-other-17 (medium, claim): ruling 35, resolution replaced

Resolution: Owner call on the public docs: either restore a one-sentence boundary note in both `forks` rustdocs ("total over every `k`; at `u64::MAX` one share fewer than asked is yielded") or leave the boundary undocumented and have the test doc say it pins a property the public contract leaves implicit. Either way: rename `n` to `k` in both rustdocs (or the reverse in the signatures), and replace "pinned in both profiles" with the property actually held ("total: no panic in any profile"). Acceptance: `Clock::forks`'s rustdoc no longer claims `n` children unconditionally, or the test doc no longer says "documented"; doc and signature agree on the parameter name; no test doc claims a release-profile run the gate does not perform.
Construction: Read clock.rs:166, then run the test's own steps: `Clock::seed().forks(u64::MAX).len() == u64::MAX - 1` (forks_max.rs:49-50). The doc and the pinned reading disagree at the one input the test exists for.

Ruled (35): neither branch of the owner call as written; the boundary
ceases to exist. The rest stands: `n` renamed to `k` in both rustdocs (or
the signatures, whichever the crate's other doors use), "pinned in both
profiles" replaced by the property held ("total: no panic in any
profile"), and no test doc claiming a release-profile run the gate does
not perform.

Superseded by ruling 82 (`p7-api.md`): `Clock::forks` and `Party::forks` take the count as `usize`, `ExactSizeIterator` is unconditional, and ruling 35's boundary reads at `usize::MAX`. Do not land this entry from this brief; `p7-api` owns the forks change (this lane's `p2-widths` members clock-3, tests-other-17, party-14, api-audit-10, clock-17, and party-13 all move there).

### api-audit-10 (low, documentation): ruling 35

Resolution: one sentence in `Party::forks`, `Clock::forks`, and both `Forks` type docs ("`k == u64::MAX` saturates: `u64::MAX - 1` shares are yielded, the residual taking the last slot"), after which tests/forks_max.rs's "documented behavior" becomes true; or count in `u128` internally so `k` shares are always yielded. Acceptance: the public `forks` docs state the corner, or `forks(u64::MAX).len()` equals `u64::MAX` on 64-bit.

Ruled (35): the second alternative in the resolution (`k` shares are
always yielded), so the acceptance is `forks(u64::MAX).len()` equal to
`u64::MAX` on 64-bit. Decision 4's `ExactSizeIterator` question (clock-17,
party-13) is separate and below.

Superseded by ruling 82 (`p7-api.md`): `Clock::forks` and `Party::forks` take the count as `usize`, `ExactSizeIterator` is unconditional, and ruling 35's boundary reads at `usize::MAX`. Do not land this entry from this brief; `p7-api` owns the forks change (this lane's `p2-widths` members clock-3, tests-other-17, party-14, api-audit-10, clock-17, and party-13 all move there).

### clock-17 (medium, correctness): roster: approved (ruling 104)

Resolution: Owner's call among (a) dropping `ExactSizeIterator` from `Forks`/`party::Forks`/`Split` and keeping the accurate `size_hint` (`len()` disappears; iter.rs:16's doc example uses it); (b) taking the count as `usize`; (c) documenting under `# Panics` that `len()` panics past `usize` on narrow targets, or clamping the reserved count at construction and documenting the saturation beside the existing `u64::MAX` one. Whichever is chosen, add a red-first pin to `wasm32-pins` calling `.len()` and `size_hint()` on `forks(1u64 << 32)`. Acceptance: a committed wasm32 pin exercises the boundary under the chosen contract; the rustdoc of `Clock::forks`, `Forks`, `Party::forks`, and `party::Forks` states what happens past `usize`.
Construction: on any 32-bit target (the pinned wasm32 guest): `let mut c = before::Clock::seed(); let it = c.forks(1u64 << 32); let _ = it.len();`. `usize::try_from(4294967297).ok()` is `None`, so `size_hint` is `(usize::MAX, None)` and the default `len` panics on `assert_eq!(None, Some(usize::MAX))`. On 64-bit the same call returns `4294967296`.

Roster note: decision 4 (README), not yet ruled: `Forks::len()` panics
on 32-bit targets past `usize`. The plan's recommendation was `# Panics`
now plus the `forks(1u64 << 32)` wasm32 pin, with the API question
(dropping `ExactSizeIterator`, or a `usize` count) queued for a breaking
release. Ruling 35 makes the count exactly `k` at every `k`, which does
not change the 32-bit `len()` question. Land the red-first wasm32 pin in
step 1 only if Finch approves; edit no public doc or impl until decision
4 is ruled.

Superseded by ruling 82 (`p7-api.md`): `Clock::forks` and `Party::forks` take the count as `usize`, `ExactSizeIterator` is unconditional, and ruling 35's boundary reads at `usize::MAX`. Do not land this entry from this brief; `p7-api` owns the forks change (this lane's `p2-widths` members clock-3, tests-other-17, party-14, api-audit-10, clock-17, and party-13 all move there).

### party-13 (medium, correctness): roster: approved (ruling 104)

Resolution: Owner's choice among: document a `# Panics` on `Forks`/`clock::Forks` for `len()` past `usize::MAX` shares on 32-bit targets; override `len()` to saturate at `usize::MAX` (documented as the one place the count is inexact); or stop implementing `ExactSizeIterator` (an API removal, least attractive). Whichever is chosen, add a wasm32 pin. Nit alongside: `size_hint` converts `remaining` twice; compute `usize::try_from(self.remaining)` once. Acceptance: a committed wasm32-pins test exercises `len()` on `forks(u64::from(u32::MAX) + 1)` under the chosen contract; tests/forks_max.rs loses its `64-bit test host` caveat or states why it remains.
Construction: on any 32-bit target: `let mut p = Party::seed(); let it = p.forks(u64::from(u32::MAX) + 1); let _ = it.len();`. After the residual is drawn, `remaining` is `2^32 + 1`, `size_hint` is `(usize::MAX, None)`, and the default `len()`'s `assert_eq!(upper, Some(lower))` fails.

Roster note: the `Party` twin of clock-17 under decision 4; the same
pin and the same wait. The `size_hint` nit (one `usize::try_from`) may
land with the pin if approved.

Superseded by ruling 82 (`p7-api.md`): `Clock::forks` and `Party::forks` take the count as `usize`, `ExactSizeIterator` is unconditional, and ruling 35's boundary reads at `usize::MAX`. Do not land this entry from this brief; `p7-api` owns the forks change (this lane's `p2-widths` members clock-3, tests-other-17, party-14, api-audit-10, clock-17, and party-13 all move there).

### clippy-pedantic-2 (low, correctness): roster: approved (ruling 104)

Resolution: assert on the untruncated position before the cast: `assert!(at < self.len(), "patch position {at} is past the output");` and then `let offset = (at - committed) as u32;` is exact because `at - committed < staged_len`. Apply the same reorder to `bit_at`'s `debug_assert!` (321-322). Acceptance: the construction below panics with the documented message.
Construction: in a codec unit test, `let mut b = PackedBuilder::with_capacity(0); b.push_bit(false); b.patch_bit(1u64 << 32, true);`. Today `committed == 0`, `offset == 0 < staged_len == 1`, no panic, and `finish()` carries a set bit the caller never wrote; after the fix the call panics as documented.

Roster note: `patch_bit` truncates the offset to `u32` before the
assert that implements its documented panic, so a position past 2^32
silently patches the wrong bit; assert on the untruncated position first.
The same reorder for `bit_at`. Lands with the width work if approved.

### inventory-7 (nit, correctness): roster: approved (ruling 104)

Resolution: compare at u64 width first (`assert!(at - committed < u64::from(self.staged_len), ...)`) and cast after; `bit_at` (321-322) and `read_bits` (302) can take the same shape. Acceptance: the assert's operand is never narrowed before the comparison.

Roster note: the same site as clippy-pedantic-2 (compare at `u64`
width first, cast after; `read_bits` and `bit_at` take the same shape);
one change with it if approved.

### codec-bits-8 (nit, correctness): roster: approved (ruling 104)

Resolution: Either add an arm rejecting the corner (`pos == 0 && !bytes.is_empty()` is `TrailingBits`, with the doc sentence "the empty stream's only spelling is the empty buffer"), or narrow the contract to `pos >= 1 || bytes.is_empty()` under `# Panics` with a `debug_assert!`. Either way add the totality law to `codec/tests.rs`: for random `(bytes, pos)`, `require_marker_padding(b, p).is_ok()` implies `padding_is_canonical(&Bits::from_canonical(b))`. Acceptance: `require_marker_padding(&[0x80], 0)` is no longer `Ok(())`, and the proptest is committed.
Construction: `assert!(require_marker_padding(&[0x80], 0).is_ok())` passes today; `padding_is_canonical(&Bits::from_canonical(Bytes::from_static(&[0x80])))` trips the debug assertion, and in release yields a `Bits` with `len() == 0` that is byte-unequal to `Bits::empty()`.

Roster note: `require_marker_padding(&[0x80], 0)` accepts a spelling
of the empty stream that `padding_is_canonical` rejects; add the
rejecting arm and the committed totality proptest. Lands if approved.

### fuzz-guests-pins-27 (low, correctness): roster: approved (ruling 104)

Resolution: Take `n_bytes: u64`, compute `k` in `u64`, place the bits with the existing `set_bit` helper (lines 54-56 and `synth_rank:172` hand-roll the `0x80 >> (pos % 8)` it names; move `set_bit` and `fill_ones` above their first use), and convert to `usize` only for the allocation via `usize::try_from`. Make synthesis fallible in-band: `Vec::try_reserve_exact` and `checked_mul`/`checked_add` returning distinct negative codes, so a trap is a `before` trap by construction and the pins' "the probe backtrace attributes..." sentences become unnecessary. Acceptance: `call1("pin_version_decode", 1 << 30)` returns a negative synthesis code, never a synthesizer trap; `grep -c '0x80 >>' guest/src/lib.rs` is 1.
Construction: `call1("pin_version_decode", 1_073_741_824)`: `4 * n` overflows at line 52 under `overflow-checks = true`; the harness reports `Trapped(UnreachableCodeReached)`, indistinguishable from the pinned terminal.

Roster note: the wasm32 guest's `synth_version` does 32-bit `usize`
arithmetic that wraps seven bytes above the terminal pin and shares the
trap channel with the pins' own terminals; positions in `u64`, fallible
synthesis with distinct negative codes. The guest is this lane's file in
step 1; lands with the pins if approved.

### fuzz-guests-pins-15 (low, correctness): roster: approved (ruling 104)

Resolution: Read `let base = HEAP.current_usage();` before the reset and assert on `HEAP.peak_usage().saturating_sub(base)`, the transient quantity the doc names. Acceptance: the asserted quantity is zero for an empty body regardless of process baseline; the doc sentence and the arithmetic agree.
Construction: Hold 900 MiB before the fuzz loop (or grow the live corpus to that size), then run an input whose body allocates 200 MiB: the cap trips with no amplification present.

Roster note: the fuzz heap cap asserts on absolute peak, not the
transient peak its doc names; read the baseline before the reset and
assert on the difference. The cap itself stays flat (ruling 15). Lands
if approved.

### testing-oracles-4 (low, correctness): roster: approved (ruling 104)

Resolution: Add `debug_assert!(t.is_normal(), …)` at `from_oracle_version` and `from_oracle_party`, plus `debug_assert!(!t.is_empty(), …)` on the party door (both oracle types expose `is_normal`; `oracle::Party::is_empty` exists at oracle/party.rs:30), and reword lines 10 and 23-24 to state the precondition as the caller's. Acceptance: a test lowering `oracle::Party::Node(Arc::new(Leaf(false)), Arc::new(Leaf(false)))` panics at the door instead of yielding a `Party` equal to the seed's.
Construction: In any unit test with `pub(crate)` access: `from_oracle_party(&oracle::Party::Node(Arc::new(oracle::Party::Leaf(false)), Arc::new(oracle::Party::Leaf(false))))` returns bits `00`, equal to `from_oracle_party(&oracle::Party::Leaf(true))`. For versions: take the left-descent candidate of `all_inflations(&oracle::Party::Leaf(true), &oracle::Version::node(0u64, oracle::Version::leaf(1u64), oracle::Version::leaf(2u64)))` (a non-normal `Node(0, Leaf 2, Leaf 2)`), lower it with `from_oracle_version`, and observe it is not equal to `from_oracle_version(&that.normalized_for_test())`.

Roster note: the oracle-to-impl doors lower a non-normal or empty
oracle tree to a wrong `Party` with no check; `debug_assert!` on
normality and non-emptiness at both doors, the precondition stated as the
caller's. Lands if approved.

### skyline-sweep-place-masked-3 (low, correctness): roster: approved (ruling 104)

Resolution: delete the two `debug_assert_ne!` lines (249 and 258) and keep the contract as written. If the owner prefers the guard, amend the Panics section to say a negative running height trips a debug assertion, and either way extend `collapsible_sibling_pair_sweeps_without_panicking` (or add a sibling) with the negative-height witness so the sentence's second case is pinned. Acceptance: a debug-profile test feeding the witness below through `masked::causal_cmp` and `masked::eq`, in both operand positions, returns without panicking.

Roster note: two `debug_assert_ne!` lines panic in debug builds on
input the Panics contract says sweeps silently. Cited under decision 70
(guards that recompute what committed tests hold, a P4 ruling); do not
land until that decision is ruled, then delete the two lines and extend
the collapsible-pair test with the negative-height witness, or amend the
Panics section, per the ruling.

### skyline-query-13 (low, claim): roster: approved (ruling 104)

Resolution: Restate the premise at both sites in the codec's terms: the shift is bounded by the stream's bit length, itself bounded by allocatable memory, under 2^35 bits on a 32-bit target, so a digit position stays under 2^30; on 64-bit targets the documented panic is unreachable. Drop "multiple". Acceptance: the grep returns nothing and both comments name the bound bits.rs states.
Construction: Textual: the premise is contradicted by bits.rs:117-119 and buf.rs:25-31 as written. For the runtime side, on a 64-bit target a right spine of 2^31 unit leaves (about 1.6 GiB of stream) passes `Version::from_bits` and every shift in `rank` and `min_ticks` still fits, which is the correct argument the comment should carry.

Roster note: a shift panic-freedom comment argued from a storage cap
the codec denies; restate the premise at both sites in the codec's terms
(a digit position stays under 2^30 on 32-bit; unreachable on 64-bit).
Prose only; lands if approved.

## Hazards and stops

- `forks` at `u64::MAX` is a public behavior change ruling 35 directs; any
  other public signature or contract movement is a stop.
- The memo rows' heap columns move with the widened link index; measure
  at the parent and attribute. Any other pin moving is a stop.
- `memo.rs` is shared with `p2-cures` (ruling 2's representation); land
  the widening first so that lane rebases onto it.
- The wasm32 pins are red-first: a pin that reads green at your base is a
  finding about the construction, not a pin to keep.
- `wasm32-pins` is `p1-fuzz`'s workspace in P1 (the discriminator and
  the terminal-pin renames); if that lane has landed, rebase onto it so
  the harness reads trap genres through the discriminator; if not, the
  guest export and the harness test follow the existing `Outcome` shape
  and say so.
- No bench judge; the board's `party_join_all` reading for party-23 is a
  deterministic counter from `just amp-board-acceptance`.
