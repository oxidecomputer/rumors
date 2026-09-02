<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P5 lane: buffers, dates, the oracle fold, and dead machinery

## Goal

No mechanism for accepting a known failure exists, even empty; no
roster carries a date; the oracle's n-ary fold is the sequential
reference the module doc promises and the differential pins only what
the contract states; the Tier 2 prose describes the stored
representation and its formula is right; the fuzz-fit builder keeps no
write-only model; the `_view` operator duality is gone. Rulings 78
(decisions 46 and 47), 79 (decisions 48 and 50), and 80
(fuzzfit-strategies-19, version-core-11) fix the shape; this brief
carries them, plus seven pending lows and nits of the same kind (dead
code and unread counters).

## The retirement discipline

Every dissolution in this lane follows the doctrine's rule for retiring
an instrument: the replacement demonstrates it catches what the
instrument caught before the instrument is deleted. Concretely, each
retirement is one commit series: first the replacement (a unit test,
a fuzz-fit case, a clippy lint confirmed on a fixture, a surfacecheck
reconciliation) landed and shown firing on the artifact the old
instrument existed to catch, with the firing quoted in the commit
message; then the deletion, in a commit that names the replacement.
A retirement whose replacement cannot be shown to fire does not land:
it is reported as a finding about the replacement and the entry stays
open. Deletions leave no trace in the code (no "formerly", no retired
names in prose); the commit message carries the provenance.

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `33779b10` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `33779b10`, fast-forward; if it has diverged, stop and report. Never call
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
- **Prose (ruling 105).** Every paragraph you touch passes the three tests in `PROSE.md` (altitude, concision, legibility); the reviewer applies its checks; the diff is net shorter in prose unless your report says what the additions buy.
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

## Ordering

- version-core-11 lands before span-causally-9 (ruling 76,
  `p4-structure`); if `p4-structure` is already running, this lane
  lands version-core-11 first and notifies the coordinator so that lane
  rebases. This lane owns `src/version.rs`'s `_view` doors and
  `binop_matrix!`, `fold.rs`'s `balanced_fold` signature, and
  `span/algebra.rs`'s `SpanFoldOps` fields for that entry only.
- oracle-laws-2's oracle rewrite and oracle-laws-26's `fold.rs` witness
  land together; `party/tests.rs`'s known-bad variant is re-read
  against the widened differential (the dropped-group variant must
  still be convicted; a variant the new comparison no longer convicts
  is a stop).
- Decision 47's date dissolution edits `registry.rs` and `surface.rs`;
  rebase onto `p1-suites`, `p2-surface`, and `p4-rosters` where they
  have landed.
- Decision 48's covcheck change edits a gate leg's tool and its
  self-test; `just coverage-kernel` must pass on the committed roster
  after the category is gone.
- Decision 50: `tier2.rs`, `testing/compactness.rs`, and the
  registry's Tier 2 prose; if a consumer of the compactness ratio is
  found outside the module, report it and do not dissolve.

## Hazards and stops

- A `remediation`, `EXEMPTIONS`, or `ITEM_EXCEPTIONS` entry that turns
  out to be non-empty at the base SHA is a stop: the entry it holds is
  an untriaged failure and goes to Finch.
- The oracle rewrite must not change any law verdict; the laws suites,
  the exhaustive small scope, and the organic histories stay green.
- fuzzfit-strategies-19: the deterministic corpus is byte-identical
  before and after; a changed byte is a stop.
- Ruling 64's rule applies to every deletion here: no trace in code;
  the commit message carries the reasoning.

## Members

### oracle-laws-2 (medium, simplification): ruling 78

The oracle's `join_all` transcribes production's balanced fold, twice, and the differential pins a hand-back shape the contract declares unspecified while the module doc claims paper transcription

Resolution: Decide first whether the fold's retention and drain discipline is a pinned behavior. If not: replace both oracle `join_all` bodies with the sequential reference (test each input against the fixed accumulator; refused inputs hand back individually in feed order; `for other in inputs { if let Err(back) = self.join(other) { overlapping.push(back) } }`), and widen the `Err` arm of both `assert_join_all_matches_recursive_oracle` harnesses to the contract: same verdict, same final accumulator, and equal region union (party) or region union plus history join (clock) of the hand-backs, not element-wise order. That comparison still pins the `IdIndex` accept seam (a wrongly accepted or refused input moves the accumulator and the union) and still convicts the dropped-group variant (on `[a, b, alias(a), c, d, e]` the variant returns `Ok` and loses `c`, so verdict and union both differ); restate `join_all_differential_convicts_the_dropped_group_oracle`'s doc accordingly. If the discipline is wanted pinned: say so in both `# Errors` sections and pin it once at `fold.rs` over a plain payload type (see oracle-laws-26), then retire the oracle copies per the retirement discipline. Do not route the oracle through `crate::fold`: b9f6af2d7 retired exactly that shape. In either branch, amend oracle.rs:6-9 so it no longer claims paper transcription for `join_all`. Acceptance: no weight-stack loop remains outside `fold.rs` and the known-bad test variant; the surface roster rows for `Party::join_all` and `Clock::join_all` still cite live bindings (citecheck green); `just gate` clean.

Ruled (78, decision 46): the n-ary fold's hand-back grouping and order are unspecified, as the public `# Errors` sections say. The oracle's `join_all` on both oracle types becomes the sequential reference (feed each input to a fixed accumulator; refused inputs handed back individually in feed order); the differential compares verdict, final accumulator, and the hand-backs' region union (plus history join for clocks), never element-wise order; `fold.rs` gets its own retention-arm witness (oracle-laws-26 is its home). The Resolution's 'pinned discipline' branch is struck.

### meter-registry-tier2-13 (low, verification): ruling 78

A tautological board-roster equality and a weak, duplicated date predicate with an under-stated doc

Resolution: rename the first test to what it holds (`board_columns_declare_nonzero_reach`), drop the equality and the unreachable arm; extract one `is_iso_date` predicate (or lift surfacecheck's month/day check) and fix the second test's doc to name both ruling kinds. If finding 9 resolves by deleting `decided`, the date test goes with it. Acceptance: no assertion in registry/tests.rs restates a registry function's body; one date predicate; each test doc matches its body.

Ruled (78, decision 47): the `decided` fields and `REGISTRY_RATIFIED` dissolve with their parsers and exemptions; git holds the dates. The Resolution's 'keep with one typed parse' alternative is struck.

### meter-registry-tier2-9 (low, simplification): ruling 78

Dated rulings (`decided`, `REGISTRY_RATIFIED`) are dated rationale at declaration sites, enforced by a shape-only date test

Resolution: rule once for the registry and surfacecheck. If the registry is a decision record: say so in one sentence of the module doc and lift surfacecheck's date parse into a shared helper. Otherwise: drop `decided` and `REGISTRY_RATIFIED`, keep `reason`, fold the non-empty-reason check into `band_citations_are_unique_and_nonempty`, and delete the date test. Acceptance: either the module doc names the decision-record exemption and one typed date parse is shared, or `grep -nE '20[0-9]{2}-[0-9]{2}-[0-9]{2}' crates/before/src/meter/registry*` is empty.

Ruled (78, decision 47): the `decided` fields and `REGISTRY_RATIFIED` dissolve with their parsers and exemptions; git holds the dates. The Resolution's 'keep with one typed parse' alternative is struck.

### oracle-laws-26 (low, simplification): ruling 78

The fold's retention-arm witness lives beside the laws while `fold.rs` has no tests

Resolution: Add `fold/tests.rs` with one witness of the retention arm over integers (a combiner that refuses a marked pair), plus the dropped-group known-bad variant from party/tests.rs:182-227 rewritten over the same combiner and held convicted there; then reduce this test to the one clause the laws module owns (the hand-back contains a coalesced group, the justification for stating conservation over unions), or move that clause beside the fold witness and delete this test; widen or honor the module doc at laws/tests.rs:1-3. This is the natural home for the pin oracle-laws-2's second branch asks for. Acceptance: `fold.rs` has `mod tests;` with the retention-arm witness and its known-bad conviction; laws/tests.rs contains only collection-level pins or its doc says otherwise.

Ruled (78, decision 46): the n-ary fold's hand-back grouping and order are unspecified, as the public `# Errors` sections say. The oracle's `join_all` on both oracle types becomes the sequential reference (feed each input to a fixed accumulator; refused inputs handed back individually in feed order); the differential compares verdict, final accumulator, and the hand-backs' region union (plus history join for clocks), never element-wise order; `fold.rs` gets its own retention-arm witness (oracle-laws-26 is its home). The Resolution's 'pinned discipline' branch is struck.

### prose-hygiene-7 (low, documentation): ruling 78

Ruling dates embedded as code data, consumed only by a date-shape check

Resolution: owner ruling. Keep: say once at each `decided` field's doc that the registry is an embedded decision record and dates are part of its schema. Dissolve: drop the `decided` fields, `REGISTRY_RATIFIED`, the two shape checks, and rewrite the fourteen "dated reason/ruling/exception" phrases listed above to "reason of record". Acceptance: either the field's doc states the decision-record rationale, or `grep -rn -w -i dated` over before/src and surfacecheck returns nothing.

Ruled (78, decision 47): the `decided` fields and `REGISTRY_RATIFIED` dissolve with their parsers and exemptions; git holds the dates. The Resolution's 'keep with one typed parse' alternative is struck.

### surface-roster-17 (low, simplification): ruling 78

Exception rulings carry enforced dates at the declaration site, validated by one of two divergent validators, under a name defined nowhere

Resolution: owner's ruling, applied uniformly. Preferred: drop `decided` from `Exception` and from the registry's `EnvelopeOnly`/`Unbanded`, delete the `dated` closure and the `undated`/`misdashed`/`unmonthed` fixtures, and let `git log -L` carry the when. If dated rulings stay: record the convention once in the project instructions as a deliberate exception to Principle 5, host one `is_yyyy_mm_dd` in surface-scan used by both sites, and replace "the registry exemption-list discipline" with a pointer to that site. Either way, name the `20` (surface-roster-26). Acceptance: `grep -rn decided crates/before/surfacecheck crates/before/src/meter` is empty, or exactly one date validator exists and the convention is recorded where the phrase is defined.

Ruled (78, decision 47): the `decided` fields and `REGISTRY_RATIFIED` dissolve with their parsers and exemptions; git holds the dates. The Resolution's 'keep with one typed parse' alternative is struck.

### meter-adequacy-12 (nit, simplification): ruling 78

Registry and surfacecheck rulings carry `decided` dates whose only consumers are format tests

Where: `crates/before/src/meter/registry.rs:1073-1097`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Only a date-shape format test reads the `decided` fields. `git log -S` on the reason strings; delete the fields, `REGISTRY_RATIFIED`, and the two format tests.

Ruled (78, decision 47): the `decided` fields and `REGISTRY_RATIFIED` dissolve with their parsers and exemptions; git holds the dates. The Resolution's 'keep with one typed parse' alternative is struck.

### meter-registry-tier2-14 (high, documentation): ruling 79

`tier2.rs` describes the stored coding as a candidate and the deleted construction-language coding as "today's"; the `Tier2Size` formula is wrong against `encoded_bits`

Resolution: rewrite against today's code. The module is the independent sizer of the stored skyline coding, computed from the construction-language stream (the min-lifted packed preorder form `meter::Packed::as_bits` and `testing::bridge::packed_bits_of` produce), never from a stored stream; list the coding's terms; name its consumers (length agreement in `version/skyline/tests.rs`, the kernel emission length pins in this module's tests, the plain-sweep pin, and the compactness ratio against the construction-language size `Packed::bits` if that suite stays); state the independence rule beside `zigzag`/`gamma_bits` ("re-derived here on purpose: sharing them with `version::skyline` would make length agreement check nothing"). In `Tier2Size`, replace "today's encoded size ... `encoded_bits - nodes`" with "the construction-language size (`Packed::bits`) minus `nodes`". Fix the `expect`/`assert` messages at 73 and 92 to name the construction-language stream. In the tests, rename `*_matches_current_size` to `*_matches_packed_spelling_size` (the term the file already uses at 118, 135, 163, 183) and `..._while_current_is_quadratic` to `..._while_the_packed_spelling_is_quadratic`; re-state 5-7, 231-233, 705-709; end 236 by naming that the production validator's `suanpan::Accumulator` is the cliff-free design (skyline.rs:102-110). Owner-gated suggestion: rename the module (`meter::sizer` or `meter::skyline_size`) and `Tier2Size` with it. Acceptance: `grep -nE "today|current|would (have|be)|representation decision|canonical Version" crates/before/src/meter/tier2.rs crates/before/src/meter/tier2/tests.rs` returns only lines whose referent is unambiguous and true of the stored coding; the `Tier2Size` doc names `Packed::bits`; the independence rule appears beside the duplicate helpers.

Ruled (79, decision 50): re-denominate `tier2.rs`'s prose to the stored representation and correct `Tier2Size`'s formula; `tier2_size` stays as the independent sizer under the 2026-07-24 keep; `testing::compactness`'s envelope is dissolved unless the lane names a consumer of its ratio outside the module, in which case it reports back instead of dissolving. `meter::tier2` is not renamed.

### prose-hygiene-2 (medium, simplification): ruling 79

Tier 2 compactness apparatus describes a representation decision that has been taken

Resolution: owner call on the instrument: dissolve `meter::tier2` and `testing::compactness` (the claim is settled and the stored coding is the measured one), or re-denominate them as a stored-coding-versus-per-node-coding size bound with the per-node coding named for what it is (`testing::bridge::packed_bits_of`) and every "today"/"decision" phrase removed. Either way rewrite tier2.rs:1-19 and 23-32, compactness.rs:1-20, 31-44, 59-67 and 115-125, tier2/tests.rs:223-236 and 705-709, and meter.rs:307-309 in the present tense over what is. Acceptance: `grep -rn -i "today\|decision" crates/before/src/meter/tier2* crates/before/src/testing/compactness*` returns nothing, and the module docs state which two codings are compared and why the comparison is kept.

Ruled (79, decision 50): re-denominate `tier2.rs`'s prose to the stored representation and correct `Tier2Size`'s formula; `tier2_size` stays as the independent sizer under the 2026-07-24 keep; `testing::compactness`'s envelope is dissolved unless the lane names a consumer of its ratio outside the module, in which case it reports back instead of dissolving. `meter::tier2` is not renamed.

### testing-diff-gen-17 (medium, documentation): ruling 79

`compactness.rs` speaks from before the flag day: "today's"/"decision-era"/"adoption turns on", a `current_bits` doc that names the wrong quantity, and measurement-sweep readings in prose

Resolution: Rewrite the module doc, both constant docs, `Sample`, `check_sample`'s comment and messages, and compactness/tests.rs:1-10 in the present tense: the skyline coding's bit length is at most twice the min-lifted packed preorder reference (plus 0 bits per node) on every family; the reference is the oracle lowering's packed stream; the comb is the tightness witness. Rename `current_bits` to `reference_bits`. Keep the derivation (each stored base charged at most twice, O(1) bits per gamma merge) and the committed tightness test; move the sweep record and family maxima out of prose (they live in f2d0011b and the design note). The note's own present-tense sentence ("skyline ≤ 2× the packed-era coding outright on every sample", §11) is the ready replacement. Acceptance: no occurrence of "today", "current coding", "adoption", "decision-critical", "decision-era", "Provenance: measured", or "Ratio record" in compactness.rs or compactness/tests.rs; every figure remaining in prose is one a committed assertion pins.

Ruled (79, decision 50): re-denominate `tier2.rs`'s prose to the stored representation and correct `Tier2Size`'s formula; `tier2_size` stays as the independent sizer under the 2026-07-24 keep; `testing::compactness`'s envelope is dissolved unless the lane names a consumer of its ratio outside the module, in which case it reports back instead of dissolving. `meter::tier2` is not renamed.

### tools-14 (medium, verification): ruling 79

covcheck's `remediation` disposition is a mechanism for accepting known failures, empty today

Resolution: delete `remediation` from `dispositions` and the docstring; move the self-test fixtures at 264, 272, 282, 289 to `unreachable` or `panic-arm`; restate justfile:1006-1009 as "curated (panic-arm or unreachable, its argument at the entry) or a failure until a directed test lands" and 1024-1026 accordingly. If the owner keeps the category, record the ruling positively at line 77 as a deliberate exception. Acceptance: an entry carrying `"disposition": "remediation"` fails with the "never invent a category" message; `covcheck --self-test` passes with two dispositions; `just coverage-kernel` still passes on the committed roster.

Ruled (79, decision 48): the buffer is deleted with its supporting code. covcheck keeps `panic-arm` and `unreachable`, each with its argument at the entry, and its self-test refuses `remediation` by name; a reachable uncovered kernel line is red until a directed test lands. The Resolution's 'if the owner keeps the category' branch is struck.

### gate-legs-9 (low, simplification): ruling 79

Two rosters carry a class for accepting known failures (one now empty, one holding only a demonstration)

Resolution: Drop "remediation" from covcheck's disposition set and from the justfile's GOAL sentence (a reachable uncovered kernel line then blocks until a directed test lands or it is curated as panic-arm or unreachable). Redefine benchjudge's `red` class as the required-red known-bad tripwires and strike the "owned reds await their cures" sentence, so an unexpected red has exactly two exits: a cure, or a sidecar-declared model. Acceptance: covcheck's self-test pins that "remediation" is refused, and benchjudge's docstring and the roster notes describe `red` only as the liveness tripwire set.

Ruled (79, decision 48): the buffer is deleted with its supporting code. covcheck keeps `panic-arm` and `unreachable`, each with its argument at the entry, and its self-test refuses `remediation` by name; a reachable uncovered kernel line is red until a directed test lands. The Resolution's 'if the owner keeps the category' branch is struck.

### meter-adequacy-8 (low, simplification): ruling 79

The bench roster's `red` class is documented as a buffer for owned reds awaiting cures, a shape the roster outgrew

Resolution: rename the class to what it holds (`tripwire`), or route the schoolbook cell through the existing `--expect-red` path in its own invocation so the roster needs no red class; re-state benchjudge:65-69 and the roster notes so no text describes reds awaiting cures; fix "board-red riders" at bench_judge_roster.rs:47 to the constant's name. The self-test's laundering pins (benchjudge:857-884) hold under either name. Acceptance: no prose in the judge or roster describes a queue of expected failures.

Ruled (79, decision 48): the buffer is deleted with its supporting code. covcheck keeps `panic-arm` and `unreachable`, each with its argument at the entry, and its self-test refuses `remediation` by name; a reachable uncovered kernel line is red until a directed test lands. The Resolution's 'if the owner keeps the category' branch is struck.

### surface-roster-18 (low, simplification): ruling 79

`ITEM_EXCEPTIONS` is an empty per-item exception list with a full category, render block, `--list` disposition, census count, and tests behind it

Resolution: delete `ITEM_EXCEPTIONS` and its supporting code (the parameter, `excepted_items`, `Findings::dead_item_exceptions` and its render block, the `--list` branch, the census-line count, and the item halves of `exceptions_excuse_their_scope`, `dead_and_shadowing_exceptions_read_red`, `malformed_exceptions_read_red`, `committed_exceptions_are_well_formed`); re-adding the mechanism with its first inhabitant is the reviewed event. If kept, reword the doc without the temporal clause. Acceptance: `grep -rn ITEM_EXCEPTIONS crates/before/surfacecheck` is empty and `reconcile_with` has six parameters; or the doc states the rule with no "at this tip".

Ruled (79, decision 48): the buffer is deleted with its supporting code. covcheck keeps `panic-arm` and `unreachable`, each with its argument at the entry, and its self-test refuses `remediation` by name; a reachable uncovered kernel line is red until a directed test lands. The Resolution's 'if the owner keeps the category' branch is struck.

### testing-diff-gen-31 (low, simplification): ruling 79

`EXEMPTIONS` in `fuelscape_islands.rs` is an empty acceptance buffer

Resolution: Delete `EXEMPTIONS`, the exemption loop (63-72), and the `exempt` set; assert `emitted == included` as two `BTreeSet<String>`s so the messages at 77-80 and 85 become the two set-difference reports. Reintroduce a roster the day the first reviewed exemption exists, with its reason, as the bespoke-genre roster does. Acceptance: the test body compares two sets; no exemption mechanism and no "Currently empty" in the file.

Ruled (79, decision 48): the buffer is deleted with its supporting code. covcheck keeps `panic-arm` and `unreachable`, each with its argument at the entry, and its self-test refuses `remediation` by name; a reachable uncovered kernel line is red until a directed test lands. The Resolution's 'if the owner keeps the category' branch is struck.

### fuzzfit-strategies-19 (medium, simplification): ruling 80

The builder's `Ty`/`slots` liveness model is write-only

Resolution: replace `slots: Vec<Ty>` with a `next_reg: Reg` counter, delete `enum Ty`, make `alloc()` take no argument, remove every `self.slots[..] = Ty::Dead` line (keeping the informative comments such as `// may underflow` where they explain why a destination is not pooled), and rewrite the struct doc to "emits ops and enforces the budget unconditionally; the mirror owns well-formedness (`programs_are_well_formed`)"; fix sanity.rs:41-43 to state what the test checks. (The alternative, making the model live with a slot-type assertion in each emitter, samples no space `programs_are_well_formed` cannot, so dissolution is the doctrinal answer.) Acceptance: `grep -n 'Ty\b\|slots' strategies.rs` returns only the counter; `generation_is_deterministic` and `programs_are_well_formed` stay green with the deterministic corpus byte-identical (register numbering unchanged); no doc names a builder liveness model.

Ruled (80): dissolve the write-only model: a `next_reg` counter replaces `slots: Vec<Ty>`, `enum Ty` and every `Ty::Dead` write go, the struct and sanity-test docs say the mirror owns well-formedness. The deterministic corpus stays byte-identical; a changed byte is a stop.

### version-core-11 (medium, simplification): ruling 80

The `_view` join/meet doors, the three-strategy operator macro, and `balanced_fold`'s `view` parameter are Batch-era machinery: every caller passes a `Version`

Resolution: make `join_view`/`meet_view` take `&Version` and be one-liners (`*self = Self::join_refs(self, other)`), or delete them and write the assignment at the call sites. Collapse `binop_matrix!` to `span_matrix!`'s shape: one arm for the four value cells using `Borrow::borrow` plus one `*Assign` arm. Drop the `view` parameter from `balanced_fold` (the `Merged × Input` arm becomes `a = refs(&a, b.borrow())`). In span/algebra.rs, `SpanFoldOps` loses `lo_view`/`hi_view` and the `_core` kernels call the `_refs` forms. Delete the three "lockstep" sentences and re-word 784-791. Rename the `join_view_*`/`meet_view_*` rows at tests/meter.rs:10757-10776 (they keep pinning the same rungs through `|=`/`&=`) and re-state coverage.rs:8-10's "the same `join_view`/`meet_view` emitters". Acceptance: `just gate` green; `empty_operands_answer_without_a_walk`'s scan pins unchanged; `grep -rn lockstep crates/before/src` returns nothing; `binop_matrix!` has one value-cell arm and one assign arm.

Ruled (80): collapse the `_view` duality: `join_view`/`meet_view` become one-liners over the `_refs` ladders (or go), `binop_matrix!` collapses to one value arm and one assign arm, `balanced_fold` loses its `view` parameter, `SpanFoldOps` loses `lo_view`/`hi_view`, the lockstep sentences go, the meter rows are renamed with their pins unchanged. This lands before span-causally-9 (ruling 76, `p4-structure`), which it shrinks.

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104, placed here by the files they touch and the kind of dissolution. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### envelopes-b-17 (low, simplification): roster: approved (ruling 104)

`scatter_population` re-implements the board's private scatter constructor

Resolution: expose the board's scatter builder through the registry as the stagger and shade populations are exposed (a `Shape::Scatter.population(n)` door or a `FamilyId::Scatter` accessor on the `meter` instrument surface) and call it from `fold_version_scatter_envelope` and `fold_party_scatter_envelope`; delete `scatter_population`. Acceptance: one scatter constructor under crates/before, reachable from both the board and tests/meter.rs.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names and the retirement discipline. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### rank-26 (low, simplification): roster: approved (ruling 104)

Wide-arm limb metering has no observer: sixteen meter_wide hooks feed a counter no committed check reads while the arm is engaged

Resolution: Either make the hooks live: one `#[cfg(all(test, feature = "limb-meter"))]` unit pin under `ceiling::force(TEST_CEILING_BITS)` that resets the meter, decodes a stream whose numerator crosses the ceiling, and asserts `limb_ops() >= bits.div_ceil(64)` (the floor derived from irreducible work: every limb of the value is materialized), plus the same for one `+` on the accumulate route; or remove `meter_wide` and its call sites and state in the Metering section that wide-arm cost is priced by memory and suanpan's digit-touch meter. Acceptance: a committed test fails when any `meter_wide` call is deleted, or `meter_wide` is gone and the module doc's Metering section describes what remains.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names and the retirement discipline. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-watermark-28 (low, simplification): roster: approved (ruling 104)

Two of three emit-traffic counters are recorded but never read, and the snapshot doc overstates what they sum to

Resolution: either (a) pin `dominated_above` on a committed family that provably routes emissions through the return-early arm (the dominated-undercut family's own sites may yield a derivable count; state the derivation in the floor doc) and fold `Undecided` into a single fallback cell, or (b) reduce `EmitTraffic` to the one enforced cell. In both cases reword 44-45 to "the emission-path domination reads". Acceptance: every field of `EmitTraffic` is read by at least one committed floor or band, or the struct has one field; the doc names the emission path. Construction: delete the `record(Decision::DominatedAbove)` call at watermark.rs:966 and run the full suite: nothing turns red.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names and the retirement discipline. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### testing-oracles-9 (low, simplification): roster: approved (ruling 104)

`Dyadic`'s hand-written `PartialEq`/`Eq`/`PartialOrd`/`Ord` are dead code, and the `Ord` doc's overflow premise names the wrong bound

Resolution: Delete the four impls, keeping `#[derive(Clone, Copy, Debug)]`. If an ordering is wanted later, state the bound as `fs_grid`'s `g < 64` (exponent at most 64 after `center`). Acceptance: `grep -n 'impl .* for Dyadic' crates/before/src/testing/semantic_oracle.rs` is empty and the test target compiles.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names and the retirement discipline. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### crate-root-22 (nit, simplification): roster: approved (ruling 104)

`balanced_reduce`'s `debug_assert` checks a property its own closures make impossible

Where: `crates/before/src/fold.rs:97-100`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Nothing: the two literal closures above the assert make the asserted property impossible. Delete the assert.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names and the retirement discipline. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### oracle-laws-24 (nit, simplification): roster: approved (ruling 104)

`law_names_are_unique_across_groups` restates a guarantee `laws!` gives at compile time, and its doc overstates its role

Where: `crates/before/src/laws/tests.rs:9-21`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Only a hand-written group bypassing `laws!` that registers a foreign fn under another law's name; `laws!` makes registered names unique at compile time. Dissolve the test, or re-document the one door it guards.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names and the retirement discipline. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-fill-grow-32 (nit, simplification): roster: approved (ruling 104)

`Cost::deepen`'s test-seam parameter is spelled three ways across its callers

Where: `crates/before/src/version/skyline/grow.rs:131-149`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A sanctioned, documented test seam (`Cost::deepen`'s `ceiling`), spelled three ways across its callers. One spelling: delete the oracle's one-argument wrapper so every caller shows the seam.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names and the retirement discipline. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

