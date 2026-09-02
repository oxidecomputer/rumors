<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P2 lane: the surface rosters, the registry, and the prose the code contradicts

## Goal

Three instruments attest the crate's public surface and its cost claims
by matching strings against source text: the family-surface rows a gate
never reads, a line-scanning extractor that silently drops `pub const
fn`, an auto-trait hand list thirteen types short, and a registry whose
reason strings name enforcement homes that do not hold what they claim.
Beside them, four pieces of public prose state what the code does not do
(no allocation, a smaller index, a never-larger rank, a hand-pinned arity
cap). Finch's ruling 43 sets the bar for the repairs: instruments that
cannot drift, expressed as references the compiler resolves rather than
strings naming tests, files, or lines, with the rosters made idiomatic
where a cleaner shape exists. The invariant restored: every surface
attestation is mechanical and total in both directions; every reason,
pin, and citation is a typed reference; and no public sentence claims a
bound or an absence the code does not deliver.

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

1. The `syn`-based extractor in `crates/surface-scan` (ruling 47) first,
   since surface-roster-7's binding and surface-roster-23's census pins
   read through it; its fixtures are the entry's shapes plus
   surface-roster-21's and surface-roster-29's if approved.
2. The family-surface binding (ruling 40) and the census auto-trait pins
   (ruling 49), each red-first on the entry's construction.
3. The registry (ruling 43): the reasons corrected toward the code, then
   the reshaping to typed references, with the resolvability check as a
   compile-time fact where the reshaping achieves it and a test where it
   does not. Rebase onto `p1-suites` (ruling 20's registry rows) first.
4. The board's `mechanism()` column dissolved (ruling 48), the arity cap
   exported from the guest (ruling 42).
5. The prose: the elided allocation and size sentences (ruling 44), the
   `Rank` bound derived and pinned by proptest (ruling 46), and the
   cost-prose roster below if approved.

## Members

### surface-roster-28 (medium, correctness): ruling 47, resolution replaced

Resolution: after the two positive arms, a negative catch-all: any line whose trimmed form starts with `pub` and contains ` fn ` that was not classified panics naming file and line ("beyond the line discipline"). Accept `pub const fn` positively at both indents (strip an optional `const ` after `pub `), since it is public surface with the same naming. Fixtures in surface-scan/src/tests.rs: `pub const fn` in an inherent impl is named; an 8-indent `pub fn` inside `mod x { impl T { .. } }` panics. Acceptance: the new fixtures are red on the current extractor and green after; before's and suanpan's totality tests stay green on the tree.
Construction: fixture `impl Thing {\n    pub const fn zero() -> u8 {\n        0\n    }\n}` with `spec(None)`: `extract_public_fns` returns an empty set and does not panic. In suanpan, adding `pub const fn probe() -> u8 { 0 }` inside `impl Accumulator` leaves `claims_are_total_over_the_public_surface` green with no `Accumulator::probe` claim row.

Ruled (47): the line-scanning extractor is replaced by `syn`-based
parsing (`syn` with `full` and `visit`) that enumerates public functions
structurally: `pub fn`, `pub const fn`, `pub async fn`, `pub unsafe fn`,
at any nesting of `mod` and `impl`, with the receiver type resolved from
the `impl` header rather than a bracket scan. The entry's fixtures (a
`const fn` in an inherent impl; an 8-indent `pub fn` inside `mod x { impl
T { .. } }`) become tests that the parser names both; the catch-all panic
is unnecessary and not written. before's and suanpan's totality tests
stay green on the tree, and `surface_coverage.rs:267-270` and suanpan's
`claims/tests.rs:172-195` read the new API.

### suanpan-35 (low, simplification): ruling 47

Resolution: owner decision: a `syn`-based visitor (`full` + `visit`) in surface-scan yielding `#[test]` fn names, `pub fn` items by impl/module context, and method-call receivers by name, replacing the brace-balance convention; or keep the scanners and add a committed check that no witness file contains an unbalanced brace inside a literal or comment. Acceptance: a witness file containing `"}"` in an assert message changes no test's verdict.

Ruled (47): the `syn` visitor, one change with surface-roster-28,
covering the three uses the entry names: `#[test]` fn names, `pub fn`
items by impl and module context, and method-call receivers by name. The
brace-balance convention goes; the acceptance's witness-file case is a
committed fixture.

### surface-roster-7 (high, claim): ruling 40

Resolution: minimum, now: restate surface.rs:1013-1015, surface_coverage.rs:28-30, and main.rs:17-20 so the pin is mechanical and the family row is review-maintained, matching tests.rs:334-336; add family rows (or extend existing ones) for `Default` on `Version`/`Rank`/`Ticks`, the `Cow<Version>` conversions, and `error::Overlap`/`TooWide`. Better, as a design round: give each `FAMILY_SURFACE` row a machine-checkable membership (an `impls: &'static [&'static str]` of census-row prefixes or exact rows) and reconcile in surfacecheck both ways, with the three non-impl rows ("unbounded depth", "meter instrumentation plumbing", "error verdict types") excepted by name; `TRAIT_IMPLS` then becomes derived data. Acceptance: no prose attributes the family-row obligation to the gate, and the orphans above carry a disposition; or, under the binding, a synthetic census pin with no covering family row reads red in check/tests.rs and the construction below reads red in `just surface-totality`.
Construction: add `impl core::ops::Neg for Version { type Output = Version; fn neg(self) -> Version { self } }` in src/version.rs and the line `"Version: impl core::ops::arith::Neg for Version",` to `TRAIT_IMPLS`, with no `FAMILY_SURFACE` row. `just surface-totality` and `just test-all` are both green.

Ruled (40): the design round. Each `FAMILY_SURFACE` row carries a
machine-checkable membership; under ruling 43 that membership is a typed
reference (the census row values or a predicate over them), not a list
of string prefixes, if the census type allows it; if the census is
string-keyed at your base, say so and reshape it in this lane rather than
adding another string list. surfacecheck reconciles both ways with the
three non-impl rows excepted by name; `TRAIT_IMPLS` becomes derived data;
the missing rows are added; the three prose sites are restated. Negative
control: the entry's `Neg` construction, red in `just surface-totality`
after the binding, recorded in the commit message.

### surface-roster-23 (medium, claim): ruling 49

Resolution: either (a) stop excluding synthetic impls and pin the `Send`/`Sync`/`Unpin` rows in the census for every reachable type, dissolving the hand list (the JSON already carries them, so the pin becomes mechanical and total); or (b) keep auto_traits.rs, extend it with the thirteen missing types, and have surfacecheck hold it total by asserting every reachable struct and enum has a synthetic impl for each of the three traits. Acceptance: a public type losing `Send` makes `just surface-totality` (a) or `cargo check` (b) red.
Construction: add `_marker: core::marker::PhantomData<*const ()>` to `shape::Plateaus` and initialize it. The crate compiles, `Plateaus` is no longer `Send` or `Sync`, and `just gate` is green.

Ruled (49): option (a). Synthetic impls stop being excluded; the
`Send`, `Sync`, and `Unpin` rows are pinned in the census for every
reachable type; `auto_traits.rs` is deleted. Negative control: the
`PhantomData<*const ()>` construction on `shape::Plateaus`, red in `just
surface-totality`, recorded in the commit message.

### meter-registry-tier2-10 (medium, claim): ruling 43, amended

Resolution: give each family its own accurate reason. Nested-full, mirror-narrow, staircase: "priced by its board column; no tick gate pin exists on this cross" (or add the pins). Cliff-fan, cancelling-chain: name `accum_fan_touches_flat`/`accum_cancelling_touches_flat` and say they price the accumulator's stream, not a `before` operation (or resolve per finding 12). Wide-tooth, jump-comb: either route both runs through `Version::rank` like the weight-comb run and re-rule the coverage answer, or write the internal-entry decision and its reason at the two run fns and quote it. Add a registry-side check that every `reason` naming a test fn resolves (the parity scanner already reads tests/meter.rs). Acceptance: for every `reason` string naming a test, file, or module, a grep of the named site finds the named artifact.
Construction: `grep -nE 'Shape::Dense\.packed1.*NestedFullId|WideTail\.packed2\(1,|IdSpine\.packed_flagged\([A-Z_]+, false\)' crates/before/tests/meter.rs` inside `fn tick_*` bodies finds none; `grep -n 'CliffFan\|cliff_fan' crates/before/src/version/skyline crates/before/src/meter/tier2` finds corpora only; `awk 'NR>=2688&&NR<=2697' crates/before/tests/meter.rs` finds no internal-entry rationale.

Ruled (43): option 1 of the resolution's three (the accurate reasons:
nested-full, mirror-narrow, and staircase priced by their board columns
with no tick gate pin on their cross; cliff-fan and cancelling-chain
naming `accum_fan_touches_flat` and `accum_cancelling_touches_flat` as
accumulator-stream pins; wide-tooth and jump-comb routed through public
`Version::rank` like the weight-comb run). Then the amendment, Finch's
direction quoted in the ground rules: the registry's `reason` strings,
and the pin and enforcement-home citations beside them, become typed
references the compiler resolves (function items, registered law names,
`Shape` and `Op` values), so that a reason naming a test that does not
exist is a compile error, not a grep finding; the resolvability check the
resolution asks for is then a compile-time fact, and a test remains only
for what the type system cannot express. You are authorized to reshape
the family rosters to whatever idiomatic form makes them obviously
correct; report the reshaping in the diff. The cliff-fan caveat (the
tier2 corpus at `tier2/tests.rs:531`) is settled by reading that site.

### board-ops-render-12 (medium, correctness): ruling 48

Resolution: Delete the "mirroring the tags a red-buffer triage entry commits" clause. Then decide the tag's fate on present-tense grounds: either dissolve `mechanism()` and the `mech[...]` column (the `<- {reasons}` list already names every red leg), or keep it as a reader aid and have judge.rs carry the kind as data (a `RedKind { Exponent, Constant, Floor }` beside each label in `CellResult.red`, with `Display` producing today's text) so render classifies by type. The tests that match labels by string (`vec!["limb exponent"]`, `SCAN_FLOOR_TRIP`) compare variants or `to_string()` afterwards. Acceptance: `git grep -n -i 'red-buffer' -- crates` is empty; either `mechanism` is gone or a unit test asserts `mechanism(&[SCAN_FLOOR_TRIP]) == "floor"` and `mechanism(&["segments count"]) == "constant"`, with no `contains(` in the classifier.

Ruled (48): the red-buffer clause deleted and the column dissolved:
`mechanism()` and the `mech[...]` render column go; the `<- {reasons}`
list is the record of every red leg. The tests that matched labels by
string are removed with it or, where they assert a red leg, compare the
judge's typed result (ruling 43).

### fuelscape-pipeline-28 (medium, claim): ruling 42, amended

Resolution: Add a boundary test beside the parity tests: build one `Guest`, load `COMBINE_ARITY_CAP` one-byte canonical versions, call `ff_shape_combine(0, COMBINE_ARITY_CAP)` and assert `ret >= 0`, load one more and call with `COMBINE_ARITY_CAP + 1`, asserting `ret == -1`. That pins host cap equal to guest cap at the boundary independently of the smoke's random draws (alternatively export the cap from the guest and derive the host constant from it). Rewrite lines 200-202 to name that test. Acceptance: changing either constant alone (and the guest's `dispatch!` list) fails `just fuelscape-test` by name.
Construction: Lower the guest constant at fuzzfit/guest/src/lib.rs:791 to 8 and trim `dispatch!` to `0..=8`; rebuild the guest. `just fuelscape-test` passes (every smoke arity is at most 8). `just fuelscape --max-bytes 32 shape_combine` then panics with "shape_combine: guest kernel reported -1 at size 32 sample N" on the first sample whose arity draw exceeds 8. Dually, raise the host constant to 32 with the guest at 16: same gate pass, same survey panic.

Ruled (42): the alternative in the resolution's parenthesis: the
guest exports its arity cap and the host constant derives from that
export; no boundary test against a second constant, since there is no
second constant. Lines 200-202 are rewritten to name the export. The
negative control is structural: `grep -n 'COMBINE_ARITY_CAP = ' crates/
before-fuelscape` finds no literal.

### party-1 (medium, claim): ruling 44, resolution replaced

Resolution: Restate both contracts as "`O(|self| + |other|)`; transient state is two bits per queued ancestor pair" (the same wording fits the board's `party_disjoint`/`party_covers` heap floors, which already treat heap as structurally near-zero rather than absent). Acceptance: the rendered `# Complexity` text on `Party::covers` and `Party::is_disjoint` no longer contradicts the `ID_COVERS`/`ID_DISJOINT` heap pins; no envelope or band moves.
Construction: the `IdSpine` divert pair already used by `id_covers_envelope`/`id_disjoint_envelope` (tests/meter.rs:6135-6161): one both-present node queues one right pair, allocating the `BitsBuf`'s byte vector; the pinned ceiling of 10 bytes is the measured 8 × 1.25. Any pair with a both-present node reaches the `push` at compare.rs:163-164.

Ruled (44), Finch's words: "Get rid of all the claims about allocation
that aren't grounded in reality; don't restate them, just elide them."
The "no allocation" sentence is deleted from both islands and the
rendered `# Complexity` keeps `O(|self| + |other|)`; the "two bits per
queued ancestor pair" restatement is not written. No envelope or band
moves.

### party-22 (medium, claim): ruling 44, resolution replaced

Resolution: State the bound the code has: "one machine word (32 bits) per both-present node of the indexed operand: up to eight times the operand's bit length (a both-present node and its forced subtree cost at least four bits), `O(|self|)` and freed with the fold", and add the pending stack's per-level cost if the sentence is meant as the space envelope. Acceptance: the smallest witness (`(1, (1, 0))`, 8 bits of operand, one 32-bit entry) satisfies the stated bound; no prose in the partition claims the index is smaller than its operand.
Construction: `Party::try_from((1u8, (1u8, 0u8)))` encodes as `11 00 10 00` (8 bits) with one both-present node; `IdIndex::build` allocates `vec![0u32; 1]` (32 bits). The left comb `L_0 = (1, 0)`, `L_{k+1} = (L_k, 1)` is `4d + 4` bits with `d` both-present nodes, so the table is `32d` bits and the ratio approaches 8.

Ruled (44): the "strictly smaller than the operand" sentence is
deleted; no replacement bound is written. The `O(|self|)` clause and
"freed with the fold" stay if they are already there.

### party-9 (medium, claim): ruling 37 and 44, resolution replaced

Resolution: Restate: "each region costs amortized `O(1)`; transient state is two bits per open ancestor (one machine word per 64 levels), nothing per region." If the owner wants the drain priced, a `tests/meter.rs` sweep row over the `IdSpine` party (scan = every tag once, heap = the two stacks' words) is a closed-form pin. Acceptance: the doc states the bit-per-level transient; if a row is added, its scan reading equals the party's packed bits and its heap reading equals `2·⌈depth/64⌉·8` bytes plus allocator slack.
Construction: `shape_party(Shape::LeftSpine, 200)` (testing/generators.rs) drained under the `PeakAlloc` meter used by `tests/meter.rs`: two `BitStack` spills of one word each (`path` and `right_present`), so peak heap is nonzero.

Ruled (44): "nothing allocates" is elided from `Party::shape`, not
restated; the row that prices the drain is `p2-rows`' (crate-root-37),
and the transient's size is stated only where that row pins it.

### rank-4 (medium, claim): ruling 46, amended

Resolution: Restate the bold sentence as the bound the paragraph argues (a small constant multiple of the version's packed size, and often exponentially smaller), and state the denominator the pin measures (encoded rank bits against packed version bits at the tested scales). If a byte-size bound is wanted as a contract, derive it (per level at least two topology/payload bits against 9/8 fraction bits; a b-bit counter costs 2b+1 gamma bits against b + 2 log b) with the small-scale exception stated, and pin it at two scales. Acceptance: the public doc no longer asserts an unqualified "never"; a committed test asserts the small-scale witness (`"(0, 1, 0)".parse::<Version>().unwrap().encode().len() == 1` alongside the existing 1/2 golden), and the provenance pin's doc names its denominator and scales.

Ruled (46), Finch's words: "Derive and pin (using a proptest) an exact
upper bound." The unqualified "never larger" sentence is replaced by a
derived exact upper bound on the rank encoding's size in terms of the
version's packed size (the per-level accounting the resolution sketches:
topology and payload bits against fraction bits; a b-bit counter's gamma
cost against its packed cost), with the small-scale behavior stated, and
a proptest over generated versions asserting the bound. The small-scale
witness is committed with the corrected length (`"(0, 1, 0)"` encodes to
two bytes and its rank to two, per the witness pass; the resolution's
`== 1` is wrong). The provenance pin's doc names its denominator and
scales. The proptest's shrunk seed, if one appears, is committed.

### surface-roster-21 (low, correctness): roster: pending Finch's approval

Resolution: retire the line scan (surface-roster-9), or state at tests/doc_hidden.rs (beside the roster) that a hidden inherent `pub fn` in a `SURFACE_SOURCES` file cannot satisfy both totality checks and is therefore not a shape the crate admits. Acceptance: the prose exists, or only one extractor remains.
Construction: add `#[doc(hidden)] pub fn probe(&self) {}` inside `impl Party` in src/party.rs. `roster_is_total_over_the_public_fn_surface` fails naming `Party::probe` as unrostered; add the row, and `just surface-totality` fails naming `Party::probe` as orphaned.

Roster note: the two totality jaws contradict each other on a
`#[doc(hidden)]` inherent `pub fn`; with the line scan replaced by the
parser (ruling 47) the question becomes whether the parser reports hidden
items, which one policy at `tests/doc_hidden.rs` settles. Lands with
step 1 if approved.

### surface-roster-29 (low, correctness): roster: pending Finch's approval

Resolution: track the previous character and do not count a `>` preceded by `-` as a close (or skip `->` as a unit); add a fixture `impl<F: FnMut() -> u8> Thing<F> {\n    pub fn poke(&self) {}\n}` extracting as `Thing::poke`. Acceptance: the fixture is red on the current parser (it yields `u8::poke`) and green after.
Construction: the fixture above with `spec(None)`: `extract_public_fns` returns `{"u8::poke"}`.

Roster note: the `->` arrow mis-parsed as a closing angle bracket;
dissolved by the parser (ruling 47), whose fixtures include the entry's
`impl<F: FnMut() -> u8> Thing<F>`. Lands with step 1 if approved.

### crate-root-17 (low, claim): roster: pending Finch's approval

Resolution: "so each input passes through `O(log k)` combines, each pairing two groups holding equally many inputs; because the groups at any counter level partition the inputs, one level's combines cost `O(D)` in total packed size and the whole fold `O(D log k)`." Acceptance: the cost argument mentions input-count balance and per-level partition only; no claim about operand packed-size ratio remains.
Construction: Two inputs, a one-leaf version and `Shape::Dense.packed1(125_000)`: `balanced_reduce` performs exactly one combine whose operands differ in packed size by about five orders of magnitude, contradicting the "bounded factor" clause while the `O(D log k)` bound holds trivially at `k = 2`.

Roster note: `fold.rs` states an operand-size balance the counter
does not provide; the cost argument is restated on input-count balance
and per-level partition. Prose toward the code; lands if approved.

### paper-fidelity-5 (low, claim): roster: pending Finch's approval

Resolution: replace the partner-ratio clause with the per-input participation bound, and state (or cite) the output-size bound the roster contracts rest on. Acceptance: the paragraph's every clause is true under the crate's byte-size denomination, and the `log k` contract's two premises are both stated.

Roster note: the same `fold.rs` paragraph's partner-size clause,
false in the byte-size denomination; one change with crate-root-17 if
approved.

### skyline-coding-2 (low, claim): roster: pending Finch's approval

Resolution: reword skyline.rs:119-121 to encode.rs:14-15's bound ("transient state is one `Base` per open subtree, bounded by the packed input's depth and magnitudes"), or make the transcoder push the node's base rather than the running sum so the stack holds Θ(input) bits and the sentence becomes true. Acceptance: the two docs state the same bound; if the delta-stack rewrite lands, the length-agreement and round-trip tests in skyline/tests.rs stay green.
Construction: under `limb-meter`, transcode `Shape::Bigroot.packed2(b, d)` for (b, d) = (2048, 2048) and (4096, 4096) and read `meter::limb_ops()` per input bit; the per-bit cost roughly doubles where a stream-priced walk would stay flat.

Roster note: the transcoder cost sentence overstates on Bigroot;
reword to `encode.rs`'s bound (the first option). Lands if approved.

### skyline-coding-16 (nit, claim): roster: pending Finch's approval

Resolution: "a bounded number of reallocations, never correctness", or derive the output bound (each elementary interval's code is at most the wider input code at that boundary plus a constant) and size the capacity to it. Acceptance: the comment states only what is argued.

Roster note: "costing one reallocation" is unargued; state "a bounded
number of reallocations". Lands if approved.

### skyline-sweep-place-masked-14 (nit, claim): roster: pending Finch's approval

Resolution: write the comparison in bits scanned ("|v| + |s| + |e| bits against the composition's 2|v| + |s| + |e|") and keep one O() for the order. Acceptance: no O() expression in the partition carries a numeric constant.

Roster note: a numeric constant inside big-O; write the comparison in
bits scanned. Lands if approved.

### version-core-23 (low, claim): roster: pending Finch's approval

Resolution: write `Sum` as `O(N + k)` for `k` summands, "amortized: each carry clears bits an earlier summand set", or define `N` as `Σ(1 + ‖nᵢ‖)`. Acceptance: the stated `Sum` bound is nonzero for every nonempty iterator and names its amortization.

Roster note: `Ticks`' `Sum` bound omits the per-summand term; write
`O(N + k)` with the amortization named. Lands if approved.

### codec-bits-27 (nit, claim): roster: pending Finch's approval

Resolution: "`push`, `pop`, `last`, and `set_last` are O(1); the run scans (`trailing_ones`, `all_set`) are priced where they are declared." Acceptance: the type doc names no operation as O(1) that is not.

Roster note: `BitStack`'s doc claims every operation is O(1); the run
scans are priced where declared. Lands if approved.

### codec-bits-10 (nit, claim): roster: pending Finch's approval

Resolution: State what the code does ("a request that does not fit `usize` allocates nothing up front; callers pass hints bounded by their operands' live lengths"), or clamp the hint if the no-op semantics are wanted. Acceptance: both sentences match the code's behavior on both target widths.

Roster note: `with_capacity`'s "allocates nothing up front" holds
only where `usize::try_from` fails; state what the code does. Lands if
approved.

### suanpan-4 (low, claim): roster: pending Finch's approval

Resolution: at both sites, "a nonzero partial decides within one step over a zero digit, so a fold never walks into a certified run while carrying value". Acceptance: both sentences mention the zero digit. Construction (a witness worth adding to witnesses.rs if the small-partial descent is not already pinned): build digits `[1 at index k, -(2^32 - 1) at each of k-1..1]` via `sub_magnitude_shl` for `i in 1..k` then `add_magnitude_shl(&UBig::ONE, 32 * k)`; `sign()` descends k digits with partial exactly 1 at every step, deciding only at digit 0; every step is over a nonzero digit, refuting the clause, and no certified run is entered, so the conclusion stands.

Roster note: "a nonzero partial decides within one step" is false in
general; both sites say "over a zero digit". Lands if approved.

### rank-23 (nit, claim): roster: pending Finch's approval

Resolution: "held to the real backend from above by the wasm32 boundary pins: a decode one fraction group past the capacity succeeds, which it could not if the ceiling routed that width to the backend (the pins assert values, not arms; the lower side is not load-bearing)". Acceptance: the sentence claims only what a pin observes.

Roster note: the module doc attributes arm placement to pins that
assert values only; claim only what a pin observes. Lands if approved.

### crate-root-6 (low, claim): roster: pending Finch's approval

Resolution: Give the widget data a short denominator phrase beside `size_measure` (or derive one from its first clause) and interpolate it into the summary and noscript strings; this is the same change the note's open item 4 wants for the x-axis caption, done once for all three readers. Acceptance: the rendered summary for `rank_add` names the versions' packed bytes and for `party_fromstr` the value's packed bytes; the constant phrase no longer appears in build.rs.
Construction: Open the rendered docs for `Rank::add` with JavaScript disabled: the noscript text reads "in total input bytes" while the dataset's x-axis is the packed bytes of two versions the ranks were derived from.

Roster note: island summaries say "in total input bytes" while the
datasets carry a per-operation `size_measure`; a denominator phrase
interpolated from the data. Touches `build.rs` and the widget strings;
lands if approved.

## Hazards and stops

- Public rustdoc changes only where a ruling above names them (the elided
  sentences, the derived `Rank` bound, `# Complexity` text for the literal
  doors is `p2-widths`'); any other public contract movement is a stop.
- The `syn` rewrite changes what the surface tests see; a public fn the
  parser reports that the line scan did not is a finding to report (and
  a row to add), never a reason to exempt it.
- `registry.rs` is `p1-suites`' first (ruling 20's rows); rebase before
  the registry commits.
- The roster reshaping ruling 43 authorizes is reported in full in the
  diff; it changes instrument surface (ruling 8), so a `rumors` test that
  reads the registry is updated in the same commit.
- `.cargo/mutants.toml` and the mutantcheck leg are gone at any base after
  `p1-gate`; if they exist at yours, nothing here edits them.
