<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: ghosts, contracts, and guideposts

## Goal

Nothing in the tree refers to code that no longer exists; no rustdoc
contract says what the code does not do; no declaration site carries a
dated reading or a hand-maintained count; every guidepost points at
something real and stays small. AGENTS.md's hard rule and Principle 5,
applied file by file across the sites the review listed. Rulings 58,
59, 60, 66, 68, 69, 70, 71, 72, 73, and 74 fix the shape of each
repair; this brief carries them, plus the documentation-class lows and
nits of P4 as a roster pending approval.

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `10cdd255` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `10cdd255`, fast-forward; if it has diverged, stop and report. Never call
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

Prose lanes conflict with code lanes on the same files. This lane
runs after `p1-harness` (the `tests/meter.rs` header and row docs in
envelopes-a-1 are rewritten only once the unified harness exists) and
after `p2-rows` if it has landed (it also touches the header; rebase).
`fuzzfit/harness/src/bands.rs` (fuzzfit-bands-2) and the fuzzfit ops
docs (fuzzfit-strategies-6) belong to `p1-fuzz` until it lands; take
its calibration output rather than re-running calibrate. The judge half
of meter-adequacy-3 belongs to `p1-board`'s file; if the board lane has
not landed, report that half for it and land the doc halves.

Ruling 60's measurement (crate-root-24, crate-root-29, paper-fidelity-1,
paper-fidelity-2) is one run of `examples/space_consumption.rs` on a
quiet machine: check `uptime` first, run once, commit the output, and
disclose the load average in the commit. Never iterate on it.

`p4-structure` edits kernel files this lane edits only in prose; run
the two lanes on disjoint files where possible (this lane does not
touch `operand.rs`, `fill.rs`, `watermark.rs`, `algebra.rs`, `idbits.rs`,
or `admit.rs`; leave their prose to `p4-structure`).

## Hazards and stops

- Ruling 66 restores nothing. If a dangling pointer cannot be deleted
  without leaving a sentence that is false, restate the sentence to
  what the code does; never reconstruct the deleted text.
- Ruling 68 forbids touching the identity-ladder essay (codec-bits-1).
- envelopes-a-1's header never asserts the contract holds today, and
  `lib.rs` never lists exceptions (ruling 1).
- A `just readme` regeneration follows any crate-root rustdoc edit;
  `just readme-check` must pass.

## Members

### version-core-9 (low, claims): ruling 58

`shape`'s closing paragraph argues from "realistically reachable" versions, which the same file's doctest refutes

Resolution: state the price as a function of the input and stop: "Arithmetic you do with the yielded [`Ticks`] is not included: summing rises into a running height costs the running value's width per step, `O(1)` while every height fits a machine word and up to quadratic in the encoded size when heights are wide; the walk itself stays linear regardless." Acceptance: the paragraph names no likelihood, its subject agrees with its predicate, and the island's `O(|self|) to drain` contract is unchanged.

Ruled (58): drop the likelihood framing; the paragraph states what the shape is and how the walk handles it, and the file's doctest stands as the counterexample.

### rank-31 (nit, documentation): ruling 59

The M-definition sentence is hand-copied six times in ranked.rs (eleven crate-wide)

Where: `crates/before/src/version/ranked.rs:101`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Define `M` once in the crate docs' complexity notation (lib.rs already hosts the asymptotic-guarantee section) and have the fuelscape include emit a l ...

Ruled (59): one crate-level complexity-notation section in `lib.rs` defines `M`, `|x|`, and `|iter|`, linked from the fuelscape include; the dashu tier thresholds are named once with the bump note beside them. Other sites link to the section rather than restating.

### skyline-query-28 (nit, claims): ruling 59

Backend multiplication-tier thresholds restated as literals with no anchor to the dependency pin

Resolution: Name them once as a test-module constant (for example `DASHU_05_MUL_TIER_WORDS: [usize; 3] = [24, 96, 4_000]`) with a comment tying them to suanpan's dashu 0.5 pin and the bump procedure; have `dense_factor_tier_legs` and the tier-boundary loop derive their widths from it, and have the integral doc cite the constant by name. Acceptance: one definition site for the thresholds. Construction: Bump dashu-int past a release that moves its Toom-3 threshold: `clustered_charge_agrees_at_backend_tier_boundaries` and `dense_factors_agree_through_the_public_fold_at_tier_boundaries` still pass while straddling no dispatch boundary.

Ruled (59): one crate-level complexity-notation section in `lib.rs` defines `M`, `|x|`, and `|iter|`, linked from the fuelscape include; the dashu tier thresholds are named once with the bump note beside them. Other sites link to the section rather than restating.

### crate-root-24 (medium, claims): ruling 60

"Approximately 100× more space-efficient than a naïve transcription" has no committed measurement

Resolution: Either commit the measurement (extend `examples/space_consumption.rs` to record the oracle values' in-memory or boxed-node encoded size beside `encode().len()` at each checkpoint, and quote the observed ratio with its scenario) or drop the multiplier and say what the README supports. Acceptance: the figure is reproduced by a committed example or CSV the docs name, or is absent; `just readme` regenerated. Construction: Compute both sizes for one population: `Clock::encode().len()` against the oracle tree's size under any naive spelling (boxed nodes with u64 counts, or the paper's Appendix A bits). No committed program does this, so the ratio is unchecked; if it is not about 100 at the paper's parameters the sentence is false as written.

Ruled (60): the front page cites the committed run with its denominator; the multiplier is replaced by the measured figure.

### crate-root-29 (medium, claims): ruling 60

The space-efficiency figures name parameters no committed measurement uses

Resolution: Re-denominate the paragraph to the committed artifact's parameters and quantities (stamp bytes at 128 entities after 100k/25k iterations, static versus dynamic), or commit the run that produces the 100-party / 10^6-event figures (the example takes the population and iteration budget) and cite it; state the N and N² fits as slopes over the committed columns with their band. Acceptance: every number in lib.rs:312-319 is readable from results/space_consumption/space.csv (or a newly committed CSV) at the parameters the prose names, and the growth laws are stated as fits over committed columns.

Ruled (60): measure and commit. `examples/space_consumption.rs` emits the oracle tree's boxed footprint beside `encode().len()`; the run is made once on a quiet machine (check load first; disclose if run under load) and committed; the front page and paper-fidelity paragraphs cite it with the denominator named, replacing the unsourced multiplier and the "100 parties, 1,000,000 events" figure.

### paper-fidelity-1 (medium, claims): ruling 60

Space Efficiency paragraph quotes figures no committed measurement produced

Resolution: re-denominate the paragraph against the committed run (populations 4-128, the paper's two regimes, stamp sizes at the committed iteration counts, the two fitted exponents), or commit the run that produces the quoted figures (extend `examples/space_consumption.rs` with a party/version split and a 100-party, 1,000,000-event checkpoint, regenerate `space.csv`) and let the prose cite those rows. `build.rs` already binds the figure to `results/` (lines 93 and 194); the prose numbers have no such binding. Acceptance: every number in the paragraph is traceable to a row of a committed CSV or to a formula stated beside it with its validity band; `README.md` regenerates to match.

Ruled (60): the space paragraph is re-denominated to the committed run; the deleted formulas are not restored.

### paper-fidelity-2 (medium, claims): ruling 60

"100× more space-efficient than a naïve transcription" has no committed measurement and no stated denominator

Resolution: name the denominator in the sentence and commit the measurement behind the ratio (for example, have `examples/space_consumption.rs` also report the oracle tree's heap footprint per stamp, or add a compactness test asserting the ratio band on the committed families), or drop the ratio and cite the committed Appendix A comparison. Acceptance: the sentence's ratio and its denominator are both produced by a committed artifact the sentence can name.

Ruled (60): measure and commit. `examples/space_consumption.rs` emits the oracle tree's boxed footprint beside `encode().len()`; the run is made once on a quiet machine (check load first; disclose if run under load) and committed; the front page and paper-fidelity paragraphs cite it with the denominator named, replacing the unsourced multiplier and the "100 parties, 1,000,000 events" figure.

### party-8 (medium, documentation): ruling 66

`join_all`'s `# Errors` contract promises the overlapping inputs back; the fold hands back coalesced unions

Resolution: Rewrite: on `Err`, `self` has absorbed some inputs (possibly none); the returned parties are unions of the remaining inputs, each containing at least one input that overlapped `self` or another input; no region is lost (`self` joined with the returned parties covers the original region plus every input's); which inputs are absorbed and how the rest are grouped is unspecified. Acceptance: the `# Errors` text describes union hand-back and region conservation, and `join_all_agrees_with_oracle_on_aliased_coalesced_group` reads as an instance of it rather than an exception.

Ruled (66): delete the dangling pointer, or restate the sentence so it points at nothing missing. The deletions in a6dcfbb4, b3f09baa0, 20c0515a, and bbb9f802 stand as Finch's intent; no substance is restored and nothing is drafted for Finch to edit.

### span-causally-33 (medium, documentation): ruling 66

The SAT/NP-completeness motivation is stated three ways, once logically inverted, never argued

Resolution: state the claim once, in causally.rs's Polarity section, in the direction that holds with a one-line sketch. The refutation pass's sketch: SAT (in its monotone form, every clause all-positive or all-negative, still NP-complete) reduces to mixed-polarity emptiness: take one region per variable and the span `[⊥, all ones]`; an all-positive clause is one `Down` hole `v <= h` with `h` zero exactly on the clause's regions (it subtracts the assignments violating the clause), an all-negative clause one `Up` hole `h' <= v` with `h'` one exactly on its regions; the clamp minus the holes is nonempty iff the formula is satisfiable, so exact `coverage` for a mixed query is at least as hard as SAT. Then make query.rs:26-30 and polarity.rs:209-212 one sentence each linking there; replace "footgun"/"famously"/"silently exponential" with the mechanism; align "linear time" with span-causally-24. Acceptance: exactly one site states the hardness claim with its direction and argument; `grep -rn 'NP-complete\|SAT problem' crates/before/src` returns one definitional hit plus links; "non-polynomial" is gone.

Ruled (66): delete the dangling pointer, or restate the sentence so it points at nothing missing. The deletions in a6dcfbb4, b3f09baa0, 20c0515a, and bbb9f802 stand as Finch's intent; no substance is restored and nothing is drafted for Finch to edit.

### party-7 (low, documentation): ruling 66

Public rustdoc slips in `party.rs`: `n` in prose against `k` in signatures, a missing period, "logarithmic factor" for an additive term, and a `# Warning` that is `Clock::decode`'s text verbatim

Resolution: prose to `k` at party.rs:184, 239 and clock.rs:166 (do not rename the parameters back); add the period at 184; 242-245 "grows by only `O(log k)` bits over the party it was split from"; rewrite 605-610 in terms of `Party` ("Decoding creates a second holder of the share the bytes name, circumventing the compiler-enforced `!Clone` linearity; treat an encode/decode boundary as a *move* of the [`Party`], and the same for any [`Clock`](crate::Clock) built from it"; the e546b6d5e text is a usable reference). Acceptance: every `Party`/`Clock` doc names the parameter its signature declares; the `forks` sentence agrees with the island contract; the warning on `Party::decode` names `Party`.

Ruled (66): delete the dangling pointer, or restate the sentence so it points at nothing missing. The deletions in a6dcfbb4, b3f09baa0, 20c0515a, and bbb9f802 stand as Finch's intent; no substance is restored and nothing is drafted for Finch to edit.

### span-causally-34 (low, documentation): ruling 66

`Coverage`'s docs do not state exactness, and `laws.rs` cites a precision contract on them that does not exist and describes an incompleteness the code does not have

Resolution: restore one sentence on `Coverage` (db9dfa3e's "The verdict is exact for every constructible query" is a restoration candidate; today's wording: "Each verdict is exact: `Partial` means at least one covered version is admitted and at least one is not"); rewrite laws.rs:1036-1038 to say the law pins soundness and that completeness is pinned by `coverage_is_exact_on_the_two_party_grid`. Acceptance: `grep -rn 'cannot be complete' crates/before/src` returns nothing; `Coverage`'s rustdoc states exactness; both agree with `refine_partial`'s doc.

Ruled (66): delete the dangling pointer, or restate the sentence so it points at nothing missing. The deletions in a6dcfbb4, b3f09baa0, 20c0515a, and bbb9f802 stand as Finch's intent; no substance is restored and nothing is drafted for Finch to edit.

### span-causally-38 (low, documentation): ruling 66

"there is deliberately no `Eq`; see the module docs" points at module docs that no longer discuss `Eq`

Resolution: record the decision once, either restored to causally.rs's module doc (where both pointers say it is) or on `Query`'s type doc with the pointers repointed; state the mechanism (non-unique normal forms under conjunction order and inert degenerate holes). Acceptance: `grep -n 'Eq' crates/before/src/causally.rs` (or query.rs's type doc) finds the decision the two comments cite.

Ruled (66): delete the dangling pointer, or restate the sentence so it points at nothing missing. The deletions in a6dcfbb4, b3f09baa0, 20c0515a, and bbb9f802 stand as Finch's intent; no substance is restored and nothing is drafted for Finch to edit.

### span-causally-5 (low, documentation): ruling 66

Public rustdoc typos, a doubled word, a garbled sentence, and a ghost parameter name

Resolution: "the two `hi` endpoints, where each comparison costs:"; drop the duplicated "as" at three sites; "The smallest [`Span`] containing two versions is [`span`](Version::span) (`v ^ w`)."; "of [`Span`]s grant them"; "arbitrary"; "intractable"; make `toward`'s doc use `s` and `t` throughout ("nothing in the causal future of `t` (including `t` itself). Equivalent to `after(s) & until(t)`."). Acceptance: the greps return nothing and the doc letters equal the signature's.

Ruled (66): delete the dangling pointer, or restate the sentence so it points at nothing missing. The deletions in a6dcfbb4, b3f09baa0, 20c0515a, and bbb9f802 stand as Finch's intent; no substance is restored and nothing is drafted for Finch to edit.

### api-audit-3 (medium, documentation): ruling 68

The crate guidepost names an `implementation` module and a "Law of Disjointness" that no longer exist

Resolution: point the model reference at the crate docs' "Safety rules" section (Causal Singularity, Identity Linearity), and either delete the `implementation` pointer or name where the design essay now lives (`version/skyline.rs` and `testing/validation_index.rs` are the surviving homes). Acceptance: every module and heading named in crates/before/AGENTS.md resolves to an existing item or heading.

Ruled (68): `crates/before/AGENTS.md` toward reality, compact and drift-proof. Do not add a pointer to the essay (codec-bits-1 is deferred); do not add hand-maintained enumerations.

### board-frame-5 (low, documentation): ruling 68

The root doc re-narrates each submodule's mechanism, and the copies have drifted

Resolution: State the map rule inline at the top of board.rs ("this doc orients; each mechanism is derived once, in the submodule named"), keep the criterion, the three-axis map, and the profile of record here, and replace each re-narrated derivation with a one-sentence pointer and rustdoc link to its owning submodule; where a sentence must appear twice, one copy is a link. Acceptance: no sentence of more than about ten words appears verbatim in two files of the board module (a `grep -F` of each root-doc sentence against the submodules finds only itself); denomination, declared models, tiling, and the bench mirror each have exactly one derivation site.

Ruled (68): fix `crates/before/AGENTS.md` toward the tree, kept compact and drift-proof (it names structure and points at the docs of record; it restates no enumerable fact). Finch's words: "Please keep agent instructions compact and drift-proof."

### codec-bits-1 (low, documentation): ruling 68

The identity-ladder essay decides for operations outside the module, without saying it is their home

Resolution: Either state in the essay's first paragraph that it is the ladder's single home for rung policy and reduce the call sites to a pointer ("rung choice: see `codec::bits`"), or keep the bullets as criteria only (free insurance; pays where the replaced walk is expensive and equality is common; not where the fallback is itself a cheap scan and unequal is the common case; not on linear predicates) and drop the named operations, letting each site carry its rationale as today. Acceptance: the per-operation rung rationale appears exactly once in the crate, and wherever it lives names itself as the home.

Ruled (68): deferred, home: Finch's own rewrite. Do not move, edit, or rewrite the identity-ladder essay. Finch's words: "I will re-write the essay in my own words, some day. Do not rewrite it." Nothing to do in this lane; listed so the lane does not touch it.

Ledger note: essay untouched; home: Finch's own rewrite

### recursion-4 (low, documentation): ruling 68

recurse.rs presents the descend! roster as the inventory of all remaining depth recursion

Resolution: Restate recurse.rs:9-14 as the rule with its bound classes: library code never recurses on depth; test code may recurse when bounded by the oracle envelope, a log of input size, or a named constant; `descend!` is for the test walks that can meet oracle-envelope depths on frames heavier than the oracle's own. Say plainly that the three named sites are the macro's users, or replace the roster with a mechanical check (the surface-scan tooling flagging self-recursive functions outside `descend!` and failing on an unlisted one). Mirror the change in AGENTS.md:32-36. Acceptance: recurse.rs and AGENTS.md state the policy as a rule with its bound classes, and any roster that remains is labelled as the `descend!` user list or is enforced by a committed check.

Ruled (68): fix `crates/before/AGENTS.md` toward the tree, kept compact and drift-proof (it names structure and points at the docs of record; it restates no enumerable fact). Finch's words: "Please keep agent instructions compact and drift-proof."

### skyline-watermark-8 (low, documentation): ruling 68

compacting()'s doc carries measured counterfactual ratios and misstates the saving's mechanism

Resolution: reword the mechanism as "compaction retires a word-scale boundary's accumulator to the pool at the push, so a stack of word-scale boundaries circulates one digit buffer instead of holding one per entry, and an undercut consumes each word boundary by one O(1) fold; the `skyline_min_ticks_ascend` row is the enforcing envelope, and deleting compaction trips both its heap and touch ceilings". Either drop the ratios (they live in cedb6015) and re-point .cargo/mutants.toml:74-75 and tests/meter.rs:7137-7139 at the row itself, or make the demonstration committed (a `#[cfg(test)]` constructor toggle and a red-first assertion that the un-compacted web exceeds the row's ceilings), at which point the numbers live in that test. Name `memo_resolution_cost` at 227. Acceptance: no measured ratio at the declaration site or a committed test producing it; the mechanism sentence names the digit buffer; every cited module name resolves by grep.

Ruled (68): excise the dated ratios from `MinWeb::compacting`'s doc. The `.cargo/mutants.toml` citation of them is moot once the gate lane retires the roster (ruling 18); if the roster is still present at your base, leave that file alone.

### tests-other-4 (nit, documentation): ruling 68

"answer-embedded" names two different claims

Where: `crates/before/tests/answer_embedded.rs:1-2`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Rename the file and its doc to the mechanism it attacks ("wide-base tiny-tail and wide-ladder folds", or "answer-width coupling") ...

Ruled (68): split the `answer-embedded` name into its two claims as the entry states.

### skyline-fill-grow-17 (low, documentation): ruling 69

Kernel-doc test and envelope citations resolve today but no gate leg checks them

Resolution: Owner's call between (a) extending citecheck's extraction to backticked identifiers in `//`/`///`/`//!` comments under `src/version/skyline/**` that match a collected test's final segment or an envelope name in tests/meter.rs, and (b) reducing kernel-doc citations to the owning module (`tests/meter.rs`'s tick envelopes) so renames inside cannot orphan them. Acceptance: either the citecheck gate leg fails on a deliberate local rename of a cited kernel-doc test (demonstrated once), or no production doc in the partition names an individual test function.

Ruled (69): extend `tools/citecheck` to backticked identifiers under `src/version/skyline/**` that match a collected test or envelope name. Citations stay names; no rustdoc links are introduced.

### testing-diff-gen-22 (medium, documentation): ruling 70

The asymptotics pins cite rustdoc sentences that no longer exist; the claims of record are the fuelscape roster's `contract` strings, and the Ω floor is not a public claim

Resolution: Re-word the module doc and each failure message to name the authoring site (the operation's `contract` field in `crates/before-fuelscape/src/ops.rs`, regenerated into `fuelscape/<op>.json` and rendered into the section by build.rs) and quote the contract's actual wording. For the Ω floor, owner call: add the lower bound to the public contracts if it is meant to be promised (the style rule keeps Ω out of headlines and in prose), otherwise re-scope the three `mul_bound_*` pins' docs to the private derivation in `query.rs`/`integral.rs` with the invariant restated inline. Acceptance: every quoted sentence in asymptotics.rs appears verbatim in the artifact it names; every failure message names a file and field that exists at HEAD.

Ruled (70): the floor stays private; the `mul_bound_*` pins' docs are re-scoped to the derivation. No public rustdoc states a lower bound.

### benches-examples-18 (high, documentation): ruling 71

code_study.rs cites a deleted module and essay as its reason to exist

Resolution: rewrite lines 5-10 and 55-57 in the present tense without the ghost: what the study measures (the two emission classes priced closed-form on the realistic and adversarial corpora), that it is the workload-side instrument for the crate's integer-code choice, and that a re-run reproduces the committed histograms. Decide its status explicitly: either a tiny-parameter smoke leg (`RUNS=1`, adversarial corpus only) so the reconciliation pin stays live, or a doc sentence and a justfile note declaring it a manual instrument. If the owner considers the gamma question closed for good, dissolve the example, its `[[example]]` entry, and `study_family_versions` together. Fix crates/before/AGENTS.md:6 ("the public `implementation` module for the design essay") in the same pass. Acceptance: `grep -rn 'before::implementation\|Small values\|committed figures' crates/before` is empty; every link in the file names an item in the tree; either a gate leg runs the walker or the doc states it is manual. Construction: `grep -rn 'pub mod implementation' crates/before/src` returns nothing; `git show --stat 22cdfbe1 | grep implementation.rs` shows the deletion.

Ruled (71): delete `examples/code_study.rs`, its `[[example]]` entry in `Cargo.toml`, and `study_family_versions` together; fix the AGENTS.md pointer in the same commit (coordinate with api-audit-3 in this lane). The rewrite alternatives are struck.

### envelopes-a-1 (high, documentation): ruling 71

File header and a dozen test docs describe the pre-flag-day implementation and contradict the pins beside them

Resolution: Rewrite lines 4-11 to state the suite's present role: the contract is `lib.rs:350-353`'s; five operations the claims document demonstrates over their documented bounds (`Version::join`'s re-anchor cascade, skyline-coding-9; `Ranked::cmp`'s settle, rank-33; the masked comparison's `peek_flip` term, skyline-sweep-place-masked-5; `Query::coverage`'s per-hole sweep, span-causally-36; `tick`'s memo-family heap, skyline-fill-grow-2) are defects under repair, not exceptions (owner ruling 1, 2026-09-02, `triage/rulings.md`): name them here as under repair with their finding ids, so the list empties as the fixes land, and never in `lib.rs`; and each row pins the current measured cost at ×1.25 so a regression fails and an improvement re-pins. Replace "Today's implementation is far from that" and its recursion-and-transcode narrative with that named list, never with a sentence asserting that the contract holds. Replace the meter enumerations at 13 and 357 with non-counting phrasing ("the deterministic meters", "under every meter"). Re-state each row doc in terms of the mechanism its table row's trailing comment already names: 428-429 (the validator's bit stack, drop "today"), 440-441 (the iterative sweep, zero segments), 465-466 (drop "today"), 983-985 and 996-997 (one wide root decode, or two, linear), the header 1400-1408 and the decoder docs at 1500-1501, 1517-1519, 1533-1534, 1548-1549, 1563-1564 (validate plus one exactly-sized copy, same scan reading as the validate row by construction), and 1896-1898 (the public operator's result, not an oracle; see envelopes-a-14 for the value-leg consequence). The assert messages' "transcoded"/"the transcode round-trips" (1429, 1447, 1463, 1481, 1497, 1512, 1530, 1545, 1560, 1575) legitimately name `Packed::version`'s construction-language transcode and may stay. Acceptance: `grep -nE 'far from that|Three deterministic|both meters|today|recursion-frame|per-frame|transcode back|materializ|packed-form oracle' crates/before/tests/meter.rs` over lines 1-5305 returns nothing but the construction-language sites; each decoder doc agrees with the table comment at 293-295.

Ruled (71, under ruling 1): rewrite the header to state that the contract is `lib.rs:350-353`'s; name the operations still under repair by finding id (skyline-coding-9, rank-33, skyline-sweep-place-masked-5, span-causally-36, skyline-fill-grow-2, less any whose cure has landed at your base) so the list empties as fixes land; never write a sentence asserting the contract holds today, and never list exceptions in `lib.rs`. Restate each row doc from the mechanism its own table comment names. Lands after the harness unification (`p1-harness`) and after `p2-rows` if it has touched the header; rebase, do not merge.

### envelopes-b-27 (high, documentation): ruling 71

`span_shares_the_crossing_folds` documents a limb leg the body does not have and narrates its removal

Resolution: rewrite the doc to the touch leg alone (the fused hull folds each crossing into one shared difference; a two-accumulator spelling reads the composed folds back; decode sharing is pinned by the scan identity in `span_fuses_the_pair_walk`); delete the "once rode" sentence, stating only the present fact that word-scale crossings enter no limb denomination, so the pin has no limb leg. Acceptance: the doc names exactly the legs the body asserts; `grep -n 'once rode\|limb leg pins' crates/before/tests/meter.rs` is empty.

Ruled (71): rewrite to the touch leg alone; the limb leg is not restored.

### party-3 (high, documentation): ruling 71

Prose refers to code that no longer exists: `EvNode`, `IdLit`, a removed `compare` op, an absent oracle note, a former encoding, and a "recursive form" on a loop

Resolution: idbits.rs:35 drop the `EvNode` clause or re-state positively ("the shape the id walks match on"); move the party.rs:802-808 block onto `pub trait PartyLiteral` and write `PartyLiteral` (or "a literal leaf") for `IdLit`; delete `compare` from ops.rs:2 and idbits.rs:2 (party-2 rewrites those lines anyway) and rewrite tests.rs:368 over the ops that exist (`is_disjoint == false`, `sum == None`, `join == Err`); at diff.rs:65-66 either state the linearity note inline (the `diff` method doc is its natural home: a general meet can synthesize a region shared with a third live party, a difference cannot) or drop the pointer; idbits.rs:64 "exactly as they would if `0` were stored as a leaf"; split.rs:19 "The cursor form of `oracle::Party::split` (the paper's `split`)". Acceptance: `grep -rn 'EvNode\|IdLit\|compare\b' crates/before/src/party crates/before/src/idbits.rs` returns no prose hits; the private-items rustdoc shows the literal-door text on `PartyLiteral`; diff.rs:65-66 links to text that exists; `grep -n 'recursive form' crates/before/src/party/ops/split.rs` is empty.

Ruled (71): as stated.

### surface-roster-20 (high, documentation): ruling 71

Ghost and temporal references: a deleted API name in a doc example, a pointer to a note that does not exist, a scan that no longer exists, and "none at this tip"

Resolution: extract.rs:8: `causally::since` for a public-module free function (and `causally::Query::contains` for a method on a type inside a public module). surface_coverage.rs:165-166: delete the parenthetical. tests.rs:133-134: "named non-test helpers declared under `src/` are absent from ...". check.rs:37: state the rule without the temporal clause, or dissolve with surface-roster-18. Acceptance: `grep -rn 'causally::Range' crates` is empty; `grep -rn 'coverage note\|bare-name scan' crates/before/src/testing` is empty; `grep -n 'at this tip' crates/before/surfacecheck` is empty.

Ruled (71): as stated.

### board-families-floors-judge-1 (medium, documentation): ruling 72

Base-size docs quote probe-build readings the owner's excision ruling classifies as excised; one is labeled "committed" against its source, two have no committed kernel

Resolution: Apply the 500d4d09 treatment to the six docs: keep the design argument (which band's small run the base matches, why the pair straddles the regime, the mod-32 remainder alignment), restate each known-bad separation as a class ("the known-bad settle reads a quadratic's ~×2 per byte per doubling"), cite the committed `_reads_superlinear` kernel by name where one exists, and for weight comb and freeze parade either write "a local probe build" as the band doc does or commit the kernel. Replace the literals 500 and 256 at 289-290 with the two `WIDE_ARMING_SMALL` names. For HUGELEAF, state the backend-regime boundary as the constraint (both probes on one side of the parser's algorithm switch) without the measured exponents. Acceptance: `grep -nE '×[0-9]\.[0-9]{2,3}|e 1\.41|~1× to ~4×' crates/before/src/meter/board/family.rs` returns nothing; "committed" is not applied to a probe-build measurement; every cited kernel name matches a row in tests/superlinear_tripwires.rs.

Ruled (72, with ruling 20): apply commit 500d4d09's treatment; for the weight-comb and freeze-parade docs state the mechanism with no probe citation and no committed kernel (ruling 20), rather than either alternative the Resolution offers.

### board-families-floors-judge-8 (medium, documentation): ruling 72

"deterministic-liveness" floors are undefined, spelled with "today" in six rendered strings, and contradicted by the module's opening sentence

Resolution: Define the two kinds once, by contrast, at the top of the module doc (a contract floor, derived from mandatory work and sound for every conforming implementation; a kernel-pinned floor, derived from the shipped kernel's mechanism and lowered deliberately by a re-representation, committed so state migrating off the meter trips red), and fix the opening sentence here and board.rs:88-91 to admit both. Delete "today" from the six `WHY_` strings and currency.rs:139: the trailing "would lower this floor deliberately" clause already carries the mutability. Consider anchoring the kind to an identifier (a `FloorKind` field on `Liveness::Floor`, or one shared prefix constant) so the legend's tag is not free prose. Acceptance: `grep -nw today crates/before/src/meter/board` returns nothing; the module doc defines both kinds before using either; board.rs's liveness section no longer says "never from how it does it" without qualification.

Ruled (72): define the two floor kinds by contrast at the module top and carry the kind as a typed `FloorKind` on the liveness floor that the legend renders from; fix the opening sentence here and in `board.rs`; delete "today" from the six strings and `currency.rs:139`.

### board-ops-render-29 (medium, documentation): ruling 72

A test doc quotes a dated measurement with its history and cites a design document from code

Resolution: Delete the bracketed measurement and the "§3 entry" sentence (the paragraph already states the mechanism: a linear discipline reads ×2.00 across the joint doubling, the ceiling adds rounding headroom, a per-input re-walk reads ~×4); re-word 862-863 to describe the probe's readings without "were red". Acceptance: `grep -n 'design doc\|§\|Measured\|landed\|were red' crates/before/src/meter/board/*.rs` returns nothing.

Ruled (72): delete the bracketed measurement, the design-doc sentence, and the "were red" narration; the mechanism sentence stays. prose-hygiene-3 is the same edit.

### clock-25 (medium, documentation): ruling 72

Test prose describes retired code or contradicts the code beside it at four sites

Resolution: For 791-840, either dissolve the section (334-339 already pins decoded-component equality over the same world population, and `decoded_seed_version_encodes_canonically` is its seed point case) or re-denominate it in present terms: `Clock::decode` adopts two byte slices of one read buffer as each component's storage, so each extracted component must itself be canonical storage and must re-encode and re-decode unchanged; drop the offset narrative. For 303-305, lead with the invariant ("after impl-driven `fork`/`join`/`sync`, `as_bytes` is byte-identical to `encode` and both decode to the value: no operation may leave stale bits in storage"). Reword 941 to "Party and Version render `Debug` as `Display`; `Clock`'s `Debug` is the struct form". Delete 1002 and compare the `Result`s directly. Acceptance: `git grep -n 'non-byte-aligned\|once left\|has no .PartialEq' crates/before/src/clock` is empty; the four comments match clock.rs:51, 757-760, and 916-923; `just test-all` green.

Ruled (72): dissolve the canonicity section (the sibling test already pins decoded-component equality over the same population); fix the three contradicting comments as stated.

### fresh-eyes-1 (medium, documentation): ruling 72

Crate docs and README name PartialEq as the causal ordering

Resolution: Change `PartialEq` to `PartialOrd` at lib.rs:277, then `just readme` so README.md:281 follows. Acceptance: `grep -n 'PartialEq.*causal' crates/before/src/lib.rs crates/before/README.md` returns nothing and `just readme-check` passes.

Ruled (72): as stated.

### prose-hygiene-4 (medium, documentation): ruling 72

Opaque roster IDs PROG-5 / COV-7 in the fuzz workspace

Resolution: delete the three tags. Acceptance: `grep -rn -E '\b[A-Z]{2,4}-[0-9]+\b'` over the fuzz workspace returns only UTF-8 and license identifiers.

Ruled (72): as stated.

### fuelscape-render-9 (medium, documentation): ruling 73

Format-version docs narrate the layout they replaced

Resolution: keep the first sentence and, if a second is wanted, state the invariant positively without "moved" or "Version N": "each operation document carries its own measurement commit; the index carries only the run parameters every document shares". Acceptance: neither constant's doc names a prior layout.

Ruled (73): as stated.

### fuzzfit-bands-2 (medium, documentation): ruling 73

Pin-time measurements hand-transcribed into prose have rotted; the judgment constants' evidence is printed, never committed or asserted

Resolution: Have `calibrate` emit the evidence it already computes into the generated region as one constant, e.g. `pub const PIN_EVIDENCE: PinEvidence` with fields `corpus_programs`, `corpus_steps`, `nop_fuel`, `shape_max_excess` (key, case), `refit_max_divergence` (key), `floor_min_gap` (key), `replay_ceiling_excess` (key, depth), and the uncovered keys with their reasons. Make the docs of `ENFORCE_MARGIN`, `ENFORCE_MARGIN_BELOW`, `REFIT_TOLERANCE`, and `SLOPE_ALLOWANCE` cite those fields by name instead of by number, and delete the numeric narrative from the head (the small-band numbers duplicate `SMALL_BANDS` outright; the slope-by-slope commentary at bands.rs:98-125 belongs in the re-pin commit, as bands.rs:127 itself says). Add one enforcement test asserting the orderings the docs claim: `ENFORCE_MARGIN > PIN_EVIDENCE.replay_ceiling_excess`, `PIN_EVIDENCE.floor_min_gap > 0.0`, `SLOPE_ALLOWANCE > PIN_EVIDENCE.shape_max_excess`, `REFIT_TOLERANCE > PIN_EVIDENCE.refit_max_divergence`. Acceptance: after `just fuzzfit-calibrate` at HEAD, no decimal literal above the splice marker duplicates a value the generated region or calibrate's stderr carries; the ordering test exists and passes; grep finds one shape-leg maximum in the tree, generated.

Ruled (73): typed `PIN_EVIDENCE` emitted by calibrate, docs citing fields, the ordering test. `bands.rs` is owned by `p1-fuzz` until it lands; rebase onto it and take its one calibration run's output rather than running calibrate again.

### fuzzfit-strategies-6 (medium, documentation): ruling 73

`ops.rs` claims one op per public operation and a one-to-one guest mirror, and asserts `Rank` has no packed codec

Resolution: rewrite ops.rs:3-4 to state the actual relation ("one op per operation the fuzz-fit bands price; each op calls one guest kernel by name; the guest additionally exports the kernels the fuelscape atlas measures, which no strategy reaches"); delete "and no packed codec" at 148 (or add `RankEncode`/`RankDecode` under finding 7 and snapshot ranks through `encode()`); reword lib.rs:4-5 and enforce.rs:441 to name the priced subset; extend the scope section at strategies.rs:41-62 with the omitted surfaces and the reason. Acceptance: ops.rs:3-4 and 147-148 make no claim rank.rs or the guest's export list contradicts, and the scope section names every public surface the vocabulary omits.

Ruled (73): correct the claims and snapshot ranks through `encode()` (the `RankEncode`/`RankDecode` route), coordinated with the fuzz lane's ruling 14 vocabulary binding; if `p1-fuzz` has not landed, land the prose half and report the snapshot half for that lane.

### meter-adequacy-3 (medium, documentation): ruling 73

The exponent instruments admit an n log n regression at every committed family size, and MAX_SCALING_EXPONENT's rustdoc says the opposite

Resolution: re-state the constant's rustdoc to what it excludes: polynomial superlinearity, with the crossover stated (an `n log n` cost over the x8 ladder fits slope `1 + 1/(ln 2 · log2 n)` ≈ 1.11 at 4 KiB, so a log factor is excluded only below ~256 B). Add the same sentence to the envelope suite's flatness convention at tests/meter.rs:2342-2347. If a log factor is meant to be excluded on some rows, that needs a many-point trend over a wider span or a closed-form witness per row, which is an owner decision about cost. Acceptance: the rustdoc states the admitted class; optionally a unit test in `src/meter/board/tests.rs` feeding `trend` the four points `(4096·2^l, 4096·2^l·(12+l)·c)` and asserting the slope is under `MAX_SCALING_EXPONENT`, documenting the admitted class in code.

Ruled (73, amended): the board excludes a log factor. On the acceptance ladder, hold every deterministic currency (touch, scan, limb; heap on the allowance-subtracted residual of ruling 11) to an affine model fitted on the two smallest points and asserted at the larger points with a tolerance derived from rounding and the O(1) setup term alone; commit an n log n ladder as the known-bad that reads red. The slope ceiling stays as the coarse first leg. The two-point envelope bands' flatness convention states that they admit a logarithmic factor and that the board excludes it for every operation with a board row. Finch's words: "Can we make it so that we *do* exclude n log n?" This lands in the board's judge (`p1-board` owns `judge.rs`); coordinate: if `p1-board` has not landed at your base, land the doc halves here and report the judge half for the board lane.

### meter-core-4 (medium, documentation): ruling 74

The `cliff_comb` and `wide_tooth_comb` funding arguments describe the construction coding as "this coding" and the stored delta coding as a hypothetical

Resolution: Rewrite the two rationales in terms of the stored skyline: each crossing is a 3-bit (comb) or about `2w + 3`-bit (wide tooth) delta code while the carry spans `k` (or `k - w`) bits, and the balanced signed-digit accumulator is what keeps validation, sweep, and emit linear per stored bit; state that the construction spells the tooth magnitude per tooth only as a building convenience and is not the operand's size. Delete "the leaf-delta representation question" and "under today's coding". Hand tests/meter.rs:7-9 and 1067-1070 to the envelope partition for the same re-denomination. Acceptance: `grep -n "representation question\|under today's coding" crates/before/src/meter.rs` is empty; both docs name the stored per-crossing code width and the accumulator as the mechanism.

Ruled (74): rewrite both rationales to the stored delta coding and the accumulator; the two matching lines in `tests/meter.rs` belong to envelopes-a-1 in this lane.

### prose-hygiene-3 (medium, documentation): ruling 74

Design-doc citation from code in the board tests, and the doc has moved

Resolution: delete the clause after the semicolon; if the cure's rationale matters to the test, state it in one sentence here. Acceptance: no in-scope code or rustdoc contains "design doc" or a section-number pointer.

Ruled (74): `dup` of board-ops-render-29; the same clause. Nothing separate to do; confirm the grep in this entry's Acceptance after board-ops-render-29 lands.

Ledger note: same edit as board-ops-render-29

### skyline-coding-14 (medium, documentation): ruling 74

ghost reference to the retired `implementation` essay in a test doc

Resolution: restate the counterfactual in terms of what is: "a different integer code whose zero costs two bits (ζ₂, for instance) would turn every collapse check into a no-op"; excise the two out-of-partition sites in the same pass. Acceptance: `grep -rn 'implementation\` essay\|before::implementation' crates/before` returns nothing; the testdoc still names the two-bit-zero counterfactual.

Ruled (74): restate the counterfactual without the essay; excise the out-of-partition citations in the same pass (AGENTS.md's is api-audit-3 here).

## Roster members approved (ruling 104)

Lows and nits approved as this lane's roster by ruling 104. Land each per its quoted Resolution and Acceptance, swept with the ruled members; report rather than choose if a Resolution conflicts with a ruling or offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89).

### api-audit-4 (low, documentation): roster: approved (ruling 104)

Crate docs attribute the causal ordering to PartialEq instead of PartialOrd

Resolution: replace `PartialEq` with `PartialOrd` in the sentence, then `just readme` to regenerate crates/before/README.md. Acceptance: lib.rs:277 and README.md:281 name `PartialOrd`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### benches-examples-7 (low, documentation): roster: approved (ruling 104)

Judge constants, measured exponents, and a hand cell count restated as literals at declaration sites

Resolution: cite `MAX_WALL_SCALING_EXPONENT`, `MAX_TEXT_SCALING_EXPONENT`, and `MIN_JUDGED_MEDIAN_NANOS` by name at sidecar.rs:38-39, board.rs:64-65, tripwire.rs:11 and :30; replace "~200 cells" with "the whole shape × operation product"; for the measured exponents, either state at sidecar.rs:62-85 that they are the declared model's evidence recorded at the declaration and drop the duplicate in the roster notes, or keep them in one home. Acceptance: `grep -n '1\.3\|1\.7\|10 µs\|~200 cells' crates/before/benches` returns only name-cites or nothing; the measured figures appear in exactly one committed place, or the declaration states why two.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-families-floors-judge-12 (low, documentation): roster: approved (ruling 104)

The same two derivations are restated up to seven times across floors.rs and operand.rs

Resolution: Give each derivation one home (the constructor whose `why` string it justifies: `limb_stream`, `touch_pair_fold`) and have the sibling constructors and operand.rs point at it by name; keep the module doc's prose statement (the link constraint) and the rendered strings to the one-line mechanism. Acceptance: `grep -c 'stores its width once' crates/before/src/meter/board/*.rs` is at most 2 (the module doc and the constructor).

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-families-floors-judge-2 (low, documentation): roster: approved (ruling 104)

The `version2` slot doc enumerates two pair shapes; four build arms fill the slot

Resolution: State the structure, not the roster: "except where a build arm fills it with the pairing the shape was constructed around; the post-pass leaves such a pairing in place". Acceptance: the doc names no shapes.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-frame-2 (low, documentation): roster: approved (ruling 104)

The root doc states a two-point exponent formula the judge does not use and counts "all four counters" on a five-currency axis

Resolution: At :69-73 state the estimator once ("the log-log least-squares slope of the counter against the denominator over every measured point; through two points that is the log ratio") and point at the exponent-policy section. At :123 write "invisible to every deterministic counter". Acceptance: the criterion section and the exponent-policy section name the same estimator; no numeral in board.rs restates the arity of `ByCurrency` (`grep -n -w four board.rs` shows only the ladder's four points).

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-frame-4 (low, documentation): roster: approved (ruling 104)

Two sites prescribe a "dated owner rationale" at the declaring constants; no constant carries a date, and the doctrine forbids one there

Resolution: "with an owner-ratified rationale at the declaring constant, its readings in the pin commit" at both sites. Acceptance: `grep -n dated crates/before/src/meter/board.rs crates/before/src/meter/board/ceilings.rs` returns only the registry-row sentence at board.rs:240.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-frame-7 (low, documentation): roster: approved (ruling 104)

The fold-model roster is stated three ways: two rows (cell.rs), three rows (ceilings.rs), four rows carry `with_fold_arity`

Resolution: In cell.rs drop the count ("`Some` on the n-ary fold rows, where it drives the declared fold scan model"). In ceilings.rs either add `version_span_all` or replace the enumeration with "the n-ary fold rows (every row built with `with_fold_arity`)" so the code is the roster. Acceptance: neither file counts or enumerates the fold rows, or the ceilings.rs list matches `grep -n with_fold_arity ops.rs`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-ops-render-17 (low, documentation): roster: approved (ruling 104)

shard.rs `# Panics` sections are incomplete, the rustdoc allow is module-wide, the merge output is a positional four-tuple, and the bit-pattern parse repeats where a helper exists

Resolution: Write "a strictly positive finite number" in the three `# Panics` sections; add the unframed-rationale clause to `emit_shard` (or move `assert_unframed` to a unit test over the floors constants, since every rationale is a `&'static str` constant); move the allow onto `run_acceptance` as an outer attribute; introduce `struct MeasuredCell { op, family, s1, s2 }`; add `fn opt_bits(text, line) -> Option<f64>`. Acceptance: each public `# Panics` lists every panic path; the file has no inner `#![allow]`; no four-tuple destructuring of merge output.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-ops-render-3 (low, documentation): roster: approved (ruling 104)

A dated rationale justifies the magnitude shapes' `designed` arm

Resolution: State the invariant positively, for example "The magnitude shapes stress every group but Rank: the rank rows' mismatch pair is built from the spine families, so these shapes are not its adversary." Acceptance: no "predate" or "were never" in the arm's comment.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-ops-render-30 (low, documentation): roster: approved (ruling 104)

The acceptance criterion's doc and two kernel docs cite a retired determinism tripwire

Resolution: In ceilings.rs drop "under the determinism tripwire" (if the owner wants the property re-instrumented, the smoke suite's cross-shard byte-identity test is the live witness to cite). In fill.rs:150 and fill/tests.rs:14 re-denominate the coverage sentence onto what exists: the board's acceptance ladder and the envelope suite's pinned scales. Acceptance: `grep -rn 'determinism tripwire' crates/before/src` returns nothing; `just doclint` and `just citecheck` stay clean.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### clock-2 (low, documentation): roster: approved (ruling 104)

Rustdoc names parameters the signatures do not have, plus three typos

Resolution: Rewrite the docs to the signatures' names (`k`, `iter`) so the 2efff149 convention holds in prose too; fix the three typos; sweep `Party::forks` (party.rs:239, "Splits `n` balanced shares") in the same pass. Acceptance: every backticked parameter name in clock.rs and forks.rs rustdoc appears in the corresponding signature; `git grep -n iteratatively crates/before` is empty.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### clock-20 (low, documentation): roster: approved (ruling 104)

Deleted production API names (`has_seen`, `happens_before`) presented as "the clock observers"

Resolution: Restate the doc in terms of what is compared: "Version order on production clocks matches the oracle: `>=` against a received version, strict `<` between clocks, and `concurrent` against the oracle's `concurrent_with`." Either call `oa.has_seen(&msg_oracle)`/`oa.happens_before(ob)` so the oracle observers are the reference, or drop their names. Rewrite 618 as "`>=` against a deep version lowers to a deep `causal_cmp`". Acceptance: no `Clock` test doc or comment names `has_seen`/`happens_before` unless the body calls the oracle method of that name.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### clock-4 (low, documentation): roster: approved (ruling 104)

`join_all`'s Errors section reads as returning input clocks, and "Unreachable" presumes linearity without saying so

Resolution: Add to `# Errors`: "A handed-back `Clock` may be the merge of several inputs that coalesced before the overlap was detected; its party covers exactly those inputs' regions and its version is their join." Change "Unreachable for clocks descended from one seed" to "Unreachable for clocks descended from one seed and handled linearly (the crate's safety rules)" at 239 and 350. Mirror the first sentence on `Party::join_all` (party.rs:312-315, outside this partition). Acceptance: the two `# Errors` sections state that hand-backs can be coalesced groups and name the linearity precondition; the law `clock_join_all_accepts_iff_parties_pairwise_disjoint` and the differential at tests.rs:78-93 remain the enforcement. Construction: feed `join_all` the order `[a, b, alias(a), c, d, e]` over five forks as tests.rs:78-93 does; `Err(back)` has `back.len() == 1` and `back[0].party()` covers the regions of `alias(a)`, `c`, `d`, and `e`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### codec-base-text-tree-3 (low, documentation): roster: approved (ruling 104)

`parse_decimal`'s rustdoc pins a probe measurement ("parse exponent 1.49") that no committed instrument holds

Resolution: Replace the bracketed clause with the instrument by name: "whose divide-and-conquer parser is subquadratic in the digit count; the bench judge's text-ceiling parse cells (`version_parse_trailing/hugeleaf`, `version_parse_noncanon/hugeleaf`, `clock_parse_trailing/hugeleaf`) hold the class." Keep the rest. Acceptance: `grep -rn '1\.49\|dependency-selection' crates/before/src` is empty; the paragraph names the bench judge as the sole authority for the class.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### codec-base-text-tree-9 (low, documentation): roster: approved (ruling 104)

The `limb_meter` module doc enumerates record sites by name and the list is stale; the materialization convention has no home

Resolution: At `record_wide`, state the convention once: one value-width count wherever a wide value is materialized from bits, bytes, or text, because the backend touches every limb of the result and a meter that missed it would let a decoder build arbitrarily wide values while reading zero. In the module doc, state the two rules (operand widths per `Base` operation; one value-width per materialized wide value) without naming modules. Have base.rs:78-79 and 136-139 cite `record_wide` rather than "the wide-gamma decode". Acceptance: the module doc names no recording module; `record_wide`'s doc states the convention; base.rs's two materialization docs point at `record_wide`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### codec-bits-16 (low, documentation): roster: approved (ruling 104)

cursor.rs misdescribes SliceCursor's consumers and read_int's overrides

Resolution: Replace 71-78 with the contract every override must honor (accept and reject on exactly the inputs the per-bit loop does; the mechanism belongs at each override, where `SliceCursor::read_int` and `DsiCursor::read_int` already state theirs). Rewrite 110-113 to name the actual consumers: the per-bit primitive under `decode_int` and the masked id leaf cursor; the id parsers and skyline kernels read through `DsiCursor`. Optionally make `read_int` a required method and move the one-line default into `BitwiseReaderCursor`. Acceptance: no sentence in cursor.rs names a consumer that does not construct a `SliceCursor` or counts implementors; `grep -n 'Both cursors' crates/before/src/codec/cursor.rs` is empty.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### codec-bits-20 (low, documentation): roster: approved (ruling 104)

Ghost references to the retired store_be emit path

Resolution: "emitted as two word appends (`BitsBuf::push_bits`)" at gamma.rs:15; re-denominate both test mentions to `BitsBuf::push_bits`. Acceptance: `grep -rn store_be crates/before` returns nothing; gamma.rs:15 matches `encode_int`'s word arm.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### codec-bits-26 (low, documentation): roster: approved (ruling 104)

scan.rs's record-site roster has drifted

Resolution: State the rule: records fire at the packed-stream primitives (builder appends and splices, cursor bit, unary, and code reads), and any kernel that examines packed bits outside those primitives records at its own site at the width it examined. If a roster is wanted, put it in a test that greps for `record_bits` call sites. Collapse the `seal_padding` consumer list to one sentence at `seal_padding` and let codec.rs point there. Acceptance: scan.rs's module doc names no file or type as a record site; the `seal_padding` enumeration appears at most once.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### codec-bits-6 (low, documentation): roster: approved (ruling 104)

The u64-width refrain is restated at entries that neither convert nor compute, and the read_gamma rejection is stated twice

Resolution: Delete the refrain at the six non-arithmetic sites, or reduce each to one clause pointing at buf.rs "# Widths"; keep the ledger where a conversion or wrap-freedom argument sits (bits.rs:156-159, buf.rs:56-58, build.rs:43-47, cursor.rs:58-60, dsi.rs:56-59 and 113-115, gamma.rs:214-216, scan.rs:63-65, stack.rs:39-42). Reduce dsi.rs:205-208 to "(`read_gamma` is refused: module doc)". Acceptance: `grep -rn 'every size on every target' crates/before/src/codec` hits no entry whose body has no `as u64`, `usize::try_from`, or width arithmetic; the `read_gamma` rejection is argued once in dsi.rs.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### crate-root-1 (low, documentation): roster: approved (ruling 104)

AGENTS.md sends readers to a "Law of Disjointness" and an `implementation` module that do not exist

Resolution: Point at what exists: the crate docs' "Safety rules" section, and `version/skyline.rs` (which line 7 already names) for the coding and kernels; fix or remove the `before::implementation` link in code_study.rs, naming the module doc that now carries the "Small values over large" trade. Acceptance: every module and rule name AGENTS.md cites resolves in src; `grep -rn 'before::implementation\|Law of Disjointness' crates/before` is empty.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### crate-root-10 (low, documentation): roster: approved (ruling 104)

before's borsh tests cite a rumors module (`crate::bookmark`) and rumors' protocol

Resolution: Drop the `reclaim`/`bookmark` clause and the "gossip protocol" clause; keep the mechanism sentences. Acceptance: `grep -rn 'bookmark\|gossip protocol' crates/before/src` is empty.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### crate-root-16 (low, documentation): roster: approved (ruling 104)

`Parse`'s doc omits `Ticks` and asserts paper notation for a decimal parse

Resolution: Add [`Ticks`](crate::Ticks) and qualify: "Party, Version, and Clock parse the paper's notation; Ticks parses a decimal count. Every parser strictly rejects non-canonical input." (Or drop the list: "into one of the crate's `FromStr` types".) Acceptance: the doc names every `FromStr` impl whose `Err = Parse`, or names none by type.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### crate-root-18 (low, documentation): roster: approved (ruling 104)

The closing drain hands back coalesced groups, and the public `# Errors` prose promises each input is merged or handed back

Resolution: Amend both `# Errors` sections to state the union contract ("a handed-back element may be the union of several inputs whose regions could not be folded in; the handed-back regions together are exactly the unmerged inputs' regions"), and add one sentence to fold.rs's drain paragraph saying the drain may refuse a coalesced group. Acceptance: a committed unit test constructs the case below and asserts `err.len() == 1` with `err[0]` equal to the union `c | d`; the public prose matches the test. Construction: `let mut s = Party::seed(); let mut x = s.fork(); let mut b = x.fork(); let d = b.fork(); let a = x; let c = a.dangerously_alias();` then `s.join_all([a, b, c, d]).unwrap_err()`: `a` and `b` coalesce at weight 1; `c` waits at weight 0 and coalesces with `d`; `combine(ab, cd)` overlaps on `(0, (1, 0))` so both stay on the stack; the drain joins `ab` into `s` and refuses `cd`, so the error is `vec![cd]` with `cd == "(0, (1, (0, 1)))"`, a party that was never an input.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### crate-root-20 (low, documentation): roster: approved (ruling 104)

fold.rs's hand-maintained caller list omits `Span`'s fold

Resolution: State the two caller shapes without naming them: "Receiver-seeded callers never see `None`; seedless callers restore their operator's identity over it." Acceptance: fold.rs names no specific caller, or `git grep balanced_reduce` matches the list exactly.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### crate-root-27 (low, documentation): roster: approved (ruling 104)

Safety rule 2 says bytes are "exactly one hole" in linearity; party.rs names three doors

Resolution: State the three doors as party.rs does (bytes via `decode`, serde, and borsh; text notation via `FromStr` and literals; `dangerously_alias`), each creating a second holder of an identity, and keep the bytes paragraph as the one that is easy to trip unknowingly; `just readme`. Acceptance: rule 2 names every public constructor that yields a `Party`/`Clock` sharing identity with an existing handle; no sentence asserts a count of holes.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### crate-root-28 (low, documentation): roster: approved (ruling 104)

Crate-docs accuracy slips: `PartialEq` where `PartialOrd` is meant, the `'static`-only `Deserialize`, and the unmentioned `surface` module

Resolution: `[`PartialOrd`]` at 277 (add "its `PartialEq` is byte equality on the canonical encoding" if wanted); append "`Deserialize` yields the owned forms `Ranked<'static>` and `Span<'static>`" to the serde bullet; add the operation roster (`surface`) to the instrument bullet; `just readme`. Acceptance: 277 names `PartialOrd`; the feature list names every `pub mod` a feature enables and states the lifetime of the deserialized view types.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### crate-root-31 (low, claims): roster: approved (ruling 104)

"Every operation is verified differentially against" both references overstates the roster

Resolution: "Every operation with a counterpart in the paper is verified differentially against its naive recursive implementation and a nondeterministic function-space semantics; operations with no reference (the codecs, text, the n-ary folds, borrowing mechanics) are pinned on production by algebraic laws, round-trip and strict-rejection batteries, and format goldens; the `surface` roster records which leg each door carries." Acceptance: the Testing paragraph's quantifier matches the `Leg` variants the roster assigns; `just readme`. Construction: Textual: list the surface.rs rows whose `prod_tree`/`prod_fs` legs are `Leg::Excluded(_)`; each is an operation the sentence claims is differentially verified against both references and is not.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### envelopes-a-10 (low, documentation): roster: approved (ruling 104)

Door rows print the generator's construction-language size as the operand's input size

Resolution: Pass the bytes of the operand the operation reads: `v.encode().len()` (or `encoded_bits().div_ceil(8)`) for version operands, the id bytes for parties (which are stored as built). Acceptance: every `input_bytes` printed for a version operand equals that operand's encoded byte length; `p.bytes.len()`/`ev.bytes.len()` appears in the range only for `Party` operands and the canary.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### envelopes-a-20 (low, documentation): roster: approved (ruling 104)

Principle 5 residue: history at declaration sites, hand-copied scenario sizes, a stale "thin margin" claim, and an inline measured ratio

Resolution: Restate each declaration positively (3546: "the measured record ×1.25 at both scales"; 2745-2746: drop the clause; 1372-1376: "`Sum` accepts any order; high-first makes every later add a shifted word, so it is the pinned order"; 3674: drop "The review's residual risk:"; 5076-5077: cite the mutants roster entry instead, see envelopes-a-22); replace size literals with the constant's name or the structural phrase; at 1656-1660 either state the row's present role plainly (an ordinary ×1.25 ceiling whose heap reading depends on the locked `dashu-int` allocation policy) or, if the change-detector role is wanted, re-measure and re-pin the ceiling thin again in a commit that says so; at 2365-2367 either state "kept as an order-of-magnitude calibration, band ×1 to ×2" at the site or drop the sentence. Acceptance: no "review", "truing", "retired", "was the", "at pin time", "deliberately thin", or size literal with a named constant remains in lines 1-5305; the 1.6 sentence names itself a calibration with a band or is gone.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### envelopes-a-5 (low, documentation): roster: approved (ruling 104)

"Only ever tightened" is contradicted by the tables' own re-denomination clause and by the pin history

Resolution: Replace "only ever tightened" in the five preambles (once, after envelopes-a-4) with the rule applied: a ceiling moves down on remeasure; it moves up only with an attributed mechanism or re-denomination named in the pin commit; drift inside the ceiling leaves the older ceiling standing. Acceptance: the stated rule is one every commit in `git log -p -- crates/before/tests/meter.rs` satisfies.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### envelopes-b-12 (low, documentation): roster: approved (ruling 104)

Past-tense and provenance narration at three sites: a bracketed uncommitted demonstration, a `//` provenance block under a `///` doc, and "the old" check

Resolution: at 7137-7140 replace the bracket with the present-tense fact ("the delete-field mutant of `MinWeb::compacting` reads over both ceilings under the campaign of record, .cargo/mutants.toml"); merge 7551-7554 into the `///` block; replace "the old first check alone" and "the old floor-first check" with "the two-check composition's first check" (the shape built at 9906-9912). Acceptance: no bracketed history, no `//` continuation of a `///` doc, and no "the old" in the range.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fresh-eyes-7 (low, documentation): roster: approved (ruling 104)

Projection docs spell an owned-operand `/` that does not compile

Resolution: Write `&v / &p` at own.rs:12 and `&s / &p` yielding `(&lo / &p) <= (&hi / &p)` at span.rs:70-71. Acceptance: every spelling of the projection operator in rustdoc takes a borrowed left operand.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuelscape-pipeline-21 (low, documentation): roster: approved (ruling 104)

Stale hand-maintained count: "767 canonical members at exactly 2 bytes" is the pre-marker-padding window; the current window holds 433

Resolution: Delete the number; if a size rationale is wanted, "a few hundred members: enough categories for the chi-square, small enough to enumerate". Acceptance: no literal member count remains in sample/tests.rs.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuelscape-pipeline-3 (low, documentation): roster: approved (ruling 104)

History and roadmap in module prose: a split rule "unchanged", a "long-term fix" not built

Resolution: Drop ", unchanged" at plan.rs:188; delete count.rs:37-39 and file the persistence idea in the shadow tracker if it is still wanted (a one-line negative-space statement, "tables are rebuilt per run", may stay). Acceptance: `grep -n 'unchanged\|long-term fix\|Not built here'` over the two files returns nothing.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzz-guests-pins-20 (low, documentation): roster: approved (ruling 104)

`ff_regs_reserve`'s doc narrates an incident and cites a seed path that does not exist

Resolution: Keep the mechanism sentences (272-274 and 277-279 without the parenthetical); delete the "caught exactly that" sentence and the path; if a pointer is wanted, cite the harness constant that enforces the reserve (`REGS_RESERVE` and its const assert in `harness/src/wasm.rs`). Acceptance: the doc names no path and no past event.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-bands-20 (low, documentation): roster: approved (ruling 104)

The seed-location sentence is a ghost of the pre-anchor layout

Resolution: "writes a seed to `proptest-regressions/enforce.txt` at the package root (the `tests/main.rs` anchor)"; fix guest/src/lib.rs:276-277 to `harness/proptest-regressions/enforce.txt`. Acceptance: `grep -rn 'next to this binary\|enforce.proptest-regressions' crates/before/fuzzfit` is empty; both sentences name the path `tests/seed_liveness.rs` resolves.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-bands-21 (low, documentation): roster: approved (ruling 104)

The `PROPTEST_CASES` override the suite documents is discarded by `ProptestConfig::with_cases`

Resolution: Either delete the override sentence (the justfile already states 48), or honor it: `ProptestConfig { cases: std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(SENTRY_CASES), ..ProptestConfig::default() }` with `const SENTRY_CASES: u32 = 48` (which also anchors the other hand-written 48s in this file; see finding 26). Acceptance: the doc describes what the config does; if an override is kept, `PROPTEST_CASES=4 PROPTEST_VERBOSE=1 cargo nextest run ... fuel_stays_in_the_pinned_bands` reports 4 cases. Construction: Run the sentry with `PROPTEST_CASES=1 PROPTEST_VERBOSE=1` in the fuzzfit workspace: 48 cases execute.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-bands-24 (low, documentation): roster: approved (ruling 104)

"the demonstrations ledger" resolves to nothing in the tree; the decision to keep the reach demonstrations in git history is recorded only in the design note

Resolution: Replace "the demonstrations ledger's business" with the decision in the tree's own terms: "the reach families' and the escalation replays' business; the known-bad reconstructions that accepted each reach genre are recorded at their pin commits, and the defenses they forced (`ESCALATION_REPLAYS`, the outcome-keyed bands, `REFIT_COVERAGE`, the committed seeds) are what stand here". Acceptance: the phrase names tree artifacts or git history explicitly; no rustdoc in the partition points at a design document.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-bands-26 (low, documentation): roster: approved (ruling 104)

Hand-maintained counts and derived probabilities across the harness prose

Resolution: Name the constants (`pub const SENTRY_CASES: u32 = 48`, `pub const CORPUS_OF_RECORD: usize = 4096` with `calibrate` defaulting to it) and cite them by name; replace "once in 137" with the structure ("Escalation carries weight 1 against 8 for every other family"); drop the 20,048 parenthetical (the const assertion is the statement) and the host count; replace "the seven single-operand rows" with the structural description. Acceptance: changing `any_family`'s weights, `ESCALATION_BUDGET`, or the sentry case count leaves no numeral in prose to update; `grep -rn '137\|20,048\|seven single' harness/` is empty.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-strategies-17 (low, documentation): roster: approved (ruling 104)

Prose restates draw ranges and roster ratios the code owns, one of them off by the jitter term

Resolution: 117-119: drop the "top out at 8" clause (the cap exists precisely so the shift does not depend on the ranges) and state "a capped tooth's base count is 2¹⁰ − 1, plus the family's jitter"; 98: "the family's full depth draw (up to [`ESCALATION_MAX_DEPTH`])"; 292: "Universe count (clamped by `build`)" or a shared `UNIVERSES` range constant; 301: "the small-band kernels ([`crate::bands::SMALL_BAND_KERNELS`])"; 1975-1976: "weighted 1 against every other family's 8", with enforce.rs:402 and 425 expressed as the weight ratio or computed in the message. Acceptance: no literal in these docs duplicates a value that also appears in `any_family` or a constant, and the tick bound matches the construction.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-strategies-3 (low, documentation): roster: approved (ruling 104)

The identity-routing argument is stated in full three times and overstates the canonical-equality rung as O(1)

Resolution: keep the full argument on `Step::identity` (the predicate is defined there), reworded to "settled by an equality rung (clone identity or one byte compare), not by the walk whose size law the band fits"; reduce drive.rs:57-66 to one line citing `Step::identity` and bands.rs:91-96 to one sentence with a link; drop the "the driver's sampling carries the argument" pointer. Acceptance: `grep -rn 'decoration-wide\|walked cloud' harness/src` returns one site and no site says O(1) of the byte-compare rung.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### inventory-3 (low, documentation): roster: approved (ruling 104)

AGENTS.md points readers at the retired `implementation` module

Resolution: remove the `implementation` clause, or point at where the essay's content lives now (22cdfbe1 says the `Span` type docs carry the operation table, algebra, and wire form). Acceptance: every module the guidepost names exists in lib.rs's module list.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### meter-registry-tier2-6 (low, documentation): roster: approved (ruling 104)

Measured tripwire readings (×1.50, ×1.74) and a constant's value are restated in family docs

Resolution: replace the readings with the kernel names (`absolute_position_accounting_reads_superlinear_on_freeze_position`, `span_promotion_accounting_reads_superlinear_on_rearm_spine`), as the DenseSuffix and PlateauPuncture rows do, and "16 group parties" with "`WEAVE_GROUPS` group parties". Acceptance: `grep -nE '×1\.[0-9]+|16 group' crates/before/src/meter/registry.rs` returns nothing.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### module-graph-5 (low, documentation): roster: approved (ruling 104)

AGENTS.md points at a public `implementation` module that no longer exists

Resolution: Drop the clause, or point it at wherever the design essay's content now lives (the 22cdfbe1 message says the module was retired, not moved, so dropping is the default). Acceptance: every path and module AGENTS.md names resolves in the tree.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### paper-fidelity-4 (low, documentation): roster: approved (ruling 104)

AGENTS.md points paper-readers at a retired `implementation` module and an unnamed "Law of Disjointness"

Resolution: in AGENTS.md, name the safety rules as the crate docs do and point at where the design content lives now (`version/skyline.rs`'s module doc is `pub` only under `test`/`meter`, version.rs:26-29, so say so or point at the crate docs); in `build/tests.rs:394-395`, state the ζ₂ alternative inline ("a code whose zero costs two bits") instead of citing the deleted essay. Acceptance: `grep -rn 'implementation' crates/before/AGENTS.md crates/before/src` returns no reference to a module or essay, and every rule name in AGENTS.md appears in `lib.rs`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### paper-fidelity-7 (low, documentation): roster: approved (ruling 104)

Party::join_all's Errors doc promises input parties back; the fold hands back coalesced unions

Resolution: rephrase as clock.rs does: the returned parties are the overlapping inputs' regions, possibly coalesced into unions of inputs that were disjoint among themselves; every input's region is either merged or present in the union of the returned parties. Doc change only. Acceptance: the `# Errors` text is true of the committed `join_all_agrees_with_oracle_on_aliased_coalesced_group` case.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### party-2 (low, documentation): roster: approved (ruling 104)

Hand-maintained operation and caller rosters have drifted

Resolution: Replace each roster with the mechanism it stands for: idbits.rs:1-3 "shared by the party operations in `party::ops` and by the event-side `fill`/`grow` walks"; idbits.rs:66-70 "a second cursor over a range is formed only deliberately, by `IdReader::at` from a recorded position, and each such site bounds its re-read where it lives"; idbits.rs:129-131 "a look at the current node, leaving the cursor in place"; idbits.rs:175-176 "for capacity hints and verbatim splice ranges"; idbits.rs:207-209 "every packed-tree skip in the crate routes through it"; ops.rs:6 "Every cursor operation is `O(n + m)` in its inputs (`index` documents the fold-only search term)"; ops.rs:1-2 drop the operation list; tests.rs:1-2 name the categories without "all". Acceptance: no module or item doc in the partition enumerates a caller or operation set that grep shows to be incomplete; the `compare` ghost (party-3) is gone from the same lines.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### rank-1 (low, documentation): roster: approved (ruling 104)

Hand-maintained literals and counts in prose that nothing enforces

Resolution: Either pin each family's ratio in `rank_encoding_size_is_provenance_linear` as a ceiling with slack (wide counter at or under 0.7, say) and have rank.rs:37-38 and version/tests.rs:1187-1190 cite the pin by name, or delete the raw figures and keep only the enforced 1.0 statement. Rewrite 543 as "Two clauses per operand, both width facts"; 920-921 as "The reference forms mirror Base's own Add matrix". Derive ~604 MB once in num.rs's module doc ("9/64 · 2^32 bytes") and refer to "the fraction-form capacity crossing" at the other three sites. Replace the assert message with one computed from the constants: `format!("131 bits exceeds the {TEST_CEILING_BITS}-bit test ceiling")`, or assert `wide.bits() > TEST_CEILING_BITS`. Acceptance: no numeric literal in the partition's prose duplicates an enforced constant or an unenforced measurement; `python3 -c 'print(((5<<128)+1).bit_length())'` prints 131 and matches the message.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### rank-24 (low, documentation): roster: approved (ruling 104)

"historical" as dated rationale at five declaration sites, once explaining a metered no-op shift whose live rationale exists only in git

Resolution: Restate each site positively and undated: num.rs:208-211 "The base arm can only shrink, so it stays canonical without re-dispatch. The shift runs even at zero so its width-scale limb record is emitted: the board's rank_encode limb floor asserts that the encode walk reads every limb of the numerator, and that record rides the shift."; num.rs:42-44 "Base-arm operations are Base's own metered methods"; num.rs:281 see rank-25; num.rs:319-320 "Below the ceiling this is Base::from_be_bytes then the metered sub-byte shift"; num.rs:405-407 "so the conversion never fires and every Base numerator passes through unchanged, unmetered"; rank.rs:946-947 "at Base's shift-and-add cost"; version/tests.rs:1393 likewise. Acceptance: `grep -n historical` over the partition and version/tests.rs is empty, and the shift-by-zero's rationale is stated at num.rs:206-212.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### rank-6 (low, documentation): roster: approved (ruling 104)

checked_sub and saturating_sub docs name a parameter the signatures do not have

Resolution: Rename the parameter to `rhs` (matches `Add` and the docs; parameter names are not part of the API) or change the docs to `other`; replace "handled arm by arm" with "matched on". Acceptance: parameter names in the two signatures equal the names in their docs; "arm" in rank.rs refers only to `Num`'s arms.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### rumors-dependence-6 (low, documentation): roster: approved (ruling 104)

Three sites cite the retired `implementation` module

Resolution: excise or re-point each citation to where the material lives now, in the present tense (the crate docs for the model; `version/skyline.rs` for the stored coding and its integer-code trade, if the "Small values over large" argument survives there; otherwise drop the reference). Acceptance: `grep -rn implementation crates/before --include='*.rs' --include='*.md'` finds no reference to a module or essay of that name.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-coding-22 (low, documentation): roster: approved (ruling 104)

the overlay-advance law has a third generic statement; overlay.rs's "exactly two generic faces" is a stale hand-maintained count

Resolution: required: correct overlay.rs:12-17 to name the statements that exist and why each does (two overlay faces; shape's `Refine` face, which folds exhaustion in; admit's fallible restatement), or drop the count and state the structure. Optional: implement `CursorSet` for `[W; N]` over `Refine` (priority `0..N`, depth `0` when done, step = `advance`) plus an all-done check at src/shape.rs:284 and :359, and delete `advance_refinement`. Acceptance: `just gate` clean; the public shape iterators' snapshot and differential tests pass; overlay.rs's count matches `grep -rn 'tied boundaries close to one shared flip level' crates/before/src`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-coding-26 (low, documentation): roster: approved (ruling 104)

dated incident narration and a meter-denominated constant at declaration sites in text.rs

Resolution: lines 158-161: "never holds an old and a new buffer at once during growth, so a deep left-full shape's peak transient is one chunk of slack, and a chunk never moves once allocated" (naming the board row that pins it is fine). Lines 147-150: derive 64 from a domain quantity (entry size against the per-level transient target) or state plainly that it is a tuning value whose live pin is the named board row. Acceptance: neither doc uses past-tense incident language; `PARKED_CHUNK`'s doc names a derivation or the committed pin that moves if the value changes.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-coding-31 (low, documentation): roster: approved (ruling 104)

`validate_from`'s error list omits the `Decode::Io` arm the borsh cursor surfaces

Resolution: add the arm in admit.rs's words and, optionally, convert the inline "Errors:" sentence to a `# Errors` list matching admit.rs:261-272. Acceptance: `validate_from`'s doc names `Truncated`, `NotCanonical`, and `Io` with the slice-cursor caveat.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-fill-grow-1 (low, documentation): roster: approved (ruling 104)

Hand-quoted measurements and history language in the fill module's Cost section

Resolution: Replace both brackets with the enforced statement by instrument name: the board's tick cells under `MAX_SCALING_EXPONENT`, and `tests/meter.rs`'s `width_circulation_cost` and `memo_resolution_cost` modules with their liveness floors. Drop "e 1.00", "exponent 1.00 with flat constants", and "every refuted discipline"; name `memo_resolution_cost` rather than "memo modules". At fill/tests.rs:1302-1304 state the enforced band (`b1 + 4·bitlen(k + 1) + 8`), not the observed reading. Acceptance: `grep -n '\[measured' fill.rs fill/tests.rs` returns nothing; every cited instrument name resolves.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-fill-grow-3 (low, documentation): roster: approved (ruling 104)

The `# Testing` sections paraphrase their sibling tests.rs module docs

Resolution: Replace each `# Testing` section with one sentence pointing at the sibling `tests.rs` module doc as the description of record. Cut the heights paragraph's restatement of the ledger-link fact to a pointer at the intro's statement. Before landing, check the cut sentences against the Wave 7 praised-sentence list if the owner still holds it. Acceptance: fill.rs and grow.rs each carry a one-sentence `# Testing`; nothing the tests.rs module docs say is restated in the kernels.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-fill-grow-7 (low, documentation): roster: approved (ruling 104)

`FillWalk`'s doc calls the id reader "the recursion argument" of an iterative walk

Resolution: "The `&mut` [`IdReader`] threads alongside as [`walk`](Self::walk)'s second cursor, exactly as the packed walks thread theirs." Acceptance: `grep -n recursion fill.rs` returns nothing.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-query-22 (low, documentation): roster: approved (ruling 104)

Hand-maintained caller and client counts, one already false in the test build

Resolution: State the contract, not the tally: "Callers pass nonzero counts; a zero `digits` operand is a no-op by the empty digit walk" (web.rs:128); "callers skip zero-valued factors at the sign reads that price them" (integral.rs:473); "The caller gates the call" (1046); "held once for its clients" (web.rs:20); "one watermark read serving the settle and the banked window" (901). Acceptance: the grep returns nothing over the partition.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-query-30 (low, documentation): roster: approved (ruling 104)

Eight adequacy test docs carry bracketed measured readings the sibling envelope suite keeps in pin commits

Resolution: Move the eight bracketed records to the pin commits and keep the derivation sentence ("the floor sits between linear and the measured growth; the record lives in the pin commit"). Acceptance: `grep -n 'measured in the dev profile' tests.rs` returns nothing and each floor's doc still says what the floor sits between.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-query-32 (low, documentation): roster: approved (ruling 104)

"retired" and "fell into" narrate history where the siblings state the present

Resolution: tests.rs:2433 and 2446: "refuted". integral.rs:144: "the hole a composed form falls into: the meet's emission re-codes one operand's width into switch jumps that the integral then evicts at the other operand's cheap codes". Acceptance: `grep -n -E '\bretired\b|fell into'` over the partition matches only the `Close::Retired` variant.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-sweep-place-masked-36 (low, documentation): roster: approved (ruling 104)

Hand-maintained caller rosters and counts that have already drifted

Resolution: sweep.rs:191-193, "Every comparison walk folds one sign per elementary interval into this pair ..." with no list; overlay.rs:233, replace the census with the requirement ("so a client's fold must be commutative"); masked.rs:20, "plus a constant number of accumulators" or name them structurally (one difference, at most one height integrator per masked side, a per-block net during a skip). Acceptance: no client list on `Directions`; the priority doc states the commutativity requirement; the masked module doc states no number that `block_skip`'s `net` falsifies.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-watermark-4 (low, documentation): roster: approved (ruling 104)

Hand-maintained 'two' restates FOLLOWER_SLOTS, once inaccurately

Resolution: 93-95 "Only the fill walk installs any (`FOLLOWER_SLOTS` of them, for the relations named in `fill.rs`); the min-ticks fold installs none and pays one `Option` check per slot at each arm, undercut, park, and collapse." 147-148 "Follower slots the web carries; a const assert beside the fill walk's slot constants binds the two rosters." 236 `followers: [const { None }; FOLLOWER_SLOTS]` (stable on the pinned 1.97.1 toolchain). Acceptance: widening `FOLLOWER_SLOTS` needs no prose edit and no change at 236; the cost statement matches the code paths that iterate the slots.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### span-causally-16 (low, documentation): roster: approved (ruling 104)

`OwnSpan::to_span` cites a monotonicity argument "the type's docs carry" that no type's docs carry

Resolution: state the argument inline ("projection is a join homomorphism, so `lo <= hi` gives `lo / p <= hi / p`; the `projection_monotone_in_version` law pins it") and optionally add the monotonicity sentence to `OwnVersion`'s docs; disambiguate version.rs:1675-1676. Acceptance: the pointer resolves (`grep -ni monoton` finds the argument at the cited site), and the `Div` comment's "it" names `min_ticks`.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### span-causally-37 (low, documentation): roster: approved (ruling 104)

`Query::into_owned` is documented `O(1)` but rebuilds the hole `Vec`

Resolution: "`O(1)` per stored bound (one allocation for the hole list): owned versions move, borrowed ones clone by sharing their stored buffers." Alternatively settle the holes in place (`for hole in &mut holes { hole.at = Cow::Owned(...) }`) and keep the per-bound wording. Acceptance: the complexity sentence names the per-bound denominator.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suanpan-7 (low, documentation): roster: approved (ruling 104)

Hand-maintained restatements of enumerable facts, two already stale

Resolution: (1) "the representation and the cost arguments"; (2) add `sign_limbs` beside `sign_magnitude`, or restate structurally ("every `&self` method: the two O(1) probes, the three O(held digits) readouts, and clone"); (3) drop the number ("the workspace's pinned dashu-int; bumping it is a breaking change") or pin it mechanically. `just readme` afterwards. Acceptance: no numeral or hand list in these sentences that the code can change without touching the prose.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suanpan-tests-11 (low, documentation): roster: approved (ruling 104)

the exhaustive ledger testdoc carries hand-maintained tallies, a wall-time figure, a dated anecdote, and ragged wrapping

Resolution: rewrite as "Exhaustive over every schedule of at most `LEDGER_DEPTH` ops drawn from the alphabet, each state checked once: word-scale deltas, recentering `u64::MAX` deltas, ..." and drop the state count, the seconds, and the depth-7 sentence (git history holds them; a depth bump is a deliberate commit that can cite its own run). Acceptance: no numeral in the doc duplicates a constant, no sentence reports a past run, and the paragraph reflows without a one-word line.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suite-economics-5 (low, documentation): roster: approved (ruling 104)

Wall-time claims and measurement narratives in test prose are unenforced, and exhaustive_small's is contradicted by an order of magnitude

Resolution: state the mechanism that bounds the cost and drop the seconds. exhaustive.rs:37-39 and :73-75 keep the corpus sizes (256 ids, 691 events) and name the rayon pool; ledger.rs:212-214 keeps the schedule space and drops the timing and the pin-time note; exhaustive/tests.rs:427-436 keeps the order of magnitude ("budget upwards of an hour, run detached") and the reason a strided sample under-prices it (the structurally similar pairs concentrate near the diagonal) and drops the two-runs narrative; :442-443 names the nextest profile's terminate budget without restating its number. Acceptance: no second-count or "at pin time" narrative remains in the cited passages while the corpus sizes and the schedule count do.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### surface-roster-2 (low, documentation): roster: approved (ruling 104)

The roster's module doc states the meter-feature fact three times, the surface-totality paragraph recurs at four sites, and two counts are hand-maintained

Resolution: keep one meter sentence in surface.rs (the 11-15 sentence carries the argument), delete 3-5 and 16-17; let surface_coverage.rs own the enforcement explanation and main.rs own the gate's, with the other sites pointing by name; replace "seven families" with "the families in [`Exclusion::FAMILIES`]" and "two known-public items" with "the anchors". Acceptance: the meter sentence appears once in surface.rs; the totality mechanism is described in full at one site and referenced by name elsewhere; no numeral restates an enumerable list in the partition's docs.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### testing-diff-gen-10 (low, documentation): roster: approved (ruling 104)

The generators module doc enumerates contents and has rotted; "All trees" is contradicted by `deep_left_spine_party`

Resolution: State the structure, not the roster: three sections named as the file's headers name them, each with a one-sentence purpose; qualify "All trees" with the bit-built exception or drop "All". Acceptance: the module doc names sections, not functions, and every sentence in it is true of the file.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### testing-oracles-1 (low, documentation): roster: approved (ruling 104)

Hand-maintained enumerations of suites and corpus importers have drifted

Resolution: In testing.rs, either add `fuelscape_islands` with its one-clause purpose or replace the list with the structural statement (scaffolding modules are `pub(crate)`, suite modules are private, each module's own doc states what it catches) and let the `mod` list be the roster. In exhaustive.rs, drop the importer list and keep the sentence that kernel suites import the corpus, with the one example. Acceptance: every `mod` declared in testing.rs is named in its doc or the doc no longer enumerates; exhaustive.rs names no subset of the importers.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### testing-oracles-19 (low, documentation): roster: approved (ruling 104)

The deep-enumeration docs carry a run chronology and per-leg nanosecond pricing beyond the budget-and-machine annotation the owner ruled on

Resolution: Trim both passages to the ruled contract: the budget (hour-scale, run detached), the machine and profile annotation, the structural reason for the leg split stated as a relative present-tense fact, and the reason strided extrapolation fails. Remove the aborted-run chronology, the cache-tiled mention, and the ns/pair figures; if a completed run exists, state its wall time, and if none does, say so plainly (the history pass reports no completed run on record). If the owner wants the chronology kept, state at the site why. Acceptance: neither file mentions "cache-tiled", "45-minute", "31 minutes", or ns/pair figures; budget and machine annotation remain; the decision record is unchanged.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### tests-other-15 (low, documentation): roster: approved (ruling 104)

Prose that narrates history or restates enumerable facts

Resolution: Delete the "Demonstrated: ... 197 items" sentence (the mechanism is already stated around it) and write "empty: `before` re-exports no foreign surface" for lines 18 and 27; cite the constant by name ("the judge's text ceiling, `MAX_TEXT_SCALING_EXPONENT` in tools/benchjudge") without the literal; `const FORKS: usize = 16;` used by the loop and the doc; "The version pins state that model ... A clock witness pins that violation. Every pin is built from ...". Acceptance: no numeral count of items, tests, or constants in these four files' prose that is not the name of an enforced home.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### version-core-10 (low, documentation): roster: approved (ruling 104)

`join_view`'s doc calls the byte-compare rung `O(1)`; the codec's own ladder prices it by the shared prefix

Resolution: "two short-circuits: canonical equality (clone identity in `O(1)`, else one early-exiting byte compare bounded by the shorter operand and absorbed by the sweep it precedes) and the `O(1)` lattice identity `0 ∨ v = v`"; `meet_view` (908-911) inherits the wording through "The dual short-circuits apply". Acceptance: the `join_view`/`meet_view` docs no longer call the byte compare `O(1)`, and their pricing matches bits.rs and the `distance`/`lag` comments.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### version-core-35 (low, documentation): roster: approved (ruling 104)

The deep-spine pin's assert message points at "the query depth-guard size prose", which no longer exists

Resolution: name the actual consumer in both the doc and the message: the `query::integral` funding argument's premise that every depth is paid at least one stored bit (which a 3-bit rate implies); or, if no prose depends on the 3-bit figure, restate the pin as pinning the grammar's per-level cost and drop the re-derive instruction. Acceptance: the assert message and doc cite a prose location that exists, by module and premise, or the pin's purpose is restated without a pointer.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### api-audit-16 (nit, documentation): roster: approved (ruling 104)

error::Overlap's summary names only Clock::sync; Clock::sync_all returns it too

Where: `crates/before/src/error.rs:5-5`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "...during [`Clock::sync`] or [`Clock::sync_all`]." Acceptance: both origins named.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### api-audit-18 (nit, documentation): roster: approved (ruling 104)

Parameter names drift between signature and prose (n vs k; rhs vs other; version vs other)

Where: `crates/before/src/clock.rs:107-113`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): one letter per concept (`k` for counts, `other` for the second operand) in both prose and signatures

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### api-audit-22 (nit, documentation): roster: approved (ruling 104)

Hand-maintained item count in a test module doc

Where: `crates/before/tests/foreign_reexport.rs:12-14`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): drop the number ("reads the same item count")

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-families-floors-judge-9 (nit, documentation): roster: approved (ruling 104)

Three floors.rs sentences disagree with the code or number they describe

Where: `crates/before/src/meter/board/floors.rs:23-26`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): (a) State the structure ("not-applicable where the contract forces no metered stream work ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### board-frame-12 (nit, documentation): roster: approved (ruling 104)

Two measured readings are quoted in ceilings.rs prose against the file's own rule

Where: `crates/before/src/meter/board/ceilings.rs:223-224`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Keep the mechanism (the pair's denominator barely moves; the log factor's marginal) and excise the numbers or move them to the pin commits

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### clock-1 (nit, documentation): roster: approved (ruling 104)

Safety rule cited by ordinal beside its name

Where: `crates/before/src/clock.rs:57-58`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Drop the parenthetical at clock.rs:57, party.rs:68, and lib.rs:270; cite by name, or by the anchor `[Safety rules](crate#safety-rules)`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### clock-21 (nit, documentation): roster: approved (ruling 104)

Pointer-only section headers with a hand-maintained population count

Where: `crates/before/src/clock/tests.rs:541-547`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Fold the three pointers (16-22, 543-547, 915-918) into one short "what lives elsewhere" paragraph at the top of the file naming the laws without the c ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### codec-base-text-tree-26 (nit, documentation): roster: approved (ruling 104)

Hand-maintained counts: "the 256 uniform-random vectors" and "the five marker-padded wire types"

Where: `crates/before/src/codec/tests.rs:1227-1228`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "The uniform-random vectors in `clock::tests::decode_never_panics` are a thin panic net"; "The family spans the marker-padded wire types (`Party` ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### codec-bits-2 (nit, claims): roster: approved (ruling 104)

The ladder essay's "order of magnitude" ratio is a number no instrument measures

Resolution: Restate as mechanism ("byte-parallel, with no decode per bit") or cite a bench cell comparing a `canonical_eq` miss to `causal_cmp` on byte-equal operands. Acceptance: the sentence makes no quantitative claim or names a committed measurement.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### crate-root-14 (nit, documentation): roster: approved (ruling 104)

`Overlap`'s first sentence names only `Clock::sync`; `sync_all` also returns it

Where: `crates/before/src/error.rs:5-5`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "Two parties were not disjoint when synchronizing clocks ([`Clock::sync`], [`Clock::sync_all`])." Acceptance: the doc names every public producer of ` ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### envelopes-b-24 (nit, documentation): roster: approved (ruling 104)

The width-circulation header restates the file doc's two-genre paragraph

Where: `crates/before/tests/meter.rs:8661-8668`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): delete 8661-8668 and let the module's constants cite the file doc's genres by name

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fresh-eyes-10 (nit, documentation): roster: approved (ruling 104)

Parameter named `k` in signatures, `n` in prose, across ticks and forks

Where: `crates/before/src/clock.rs:107-113`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Use `k` (the signatures' letter) in every sentence and table row listed

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fresh-eyes-13 (nit, documentation): roster: approved (ruling 104)

The error module's summary line is a question, and Overlap's doc names one of its two producers

Where: `crates/before/src/error.rs:1-5`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A summary that informs, such as "Error types: decode and parse failures, overlapping parties, crossed spans, and out-of-range tick counts." ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuelscape-pipeline-27 (nit, documentation): roster: approved (ruling 104)

The stratified-arity argument is stated three times and the totality chain four times

Where: `crates/before-fuelscape/src/ops.rs:61-68`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Keep the arity argument on the `Inputs::VersionSlice` declaration and have plan.rs cite it in one clause ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuelscape-render-33 (nit, claims): roster: approved (ruling 104)

nit: the justfile's head-weight figure for the injected header has drifted

Resolution: drop the figure ("carry the header's inert weight") or tie it to the file ("the size of docs/fuelscape-header.html"). Acceptance: the comment carries no byte figure, or the figure is derived.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### fuzzfit-bands-13 (nit, documentation): roster: approved (ruling 104)

`POOL_SLOTS`'s doc names a fallback the pooling allocator does not have

Where: `crates/before/fuzzfit/harness/src/wasm.rs:49-57`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Reword: "The pool has no fallback: exceeding it fails `Instance::new` with wasmtime's concurrency-limit error ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### gate-legs-13 (nit, documentation): roster: approved (ruling 104)

Hand-maintained counts and dated measurements in verification prose

Where: `justfile:580-583`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Name the constants instead of their values in the justfile (`ProptestConfig::with_cases` in enforce.rs, `REFIT_PREFIX_PROGRAMS` ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### inventory-13 (nit, documentation): roster: approved (ruling 104)

`Party::forks(u64::MAX)` yields one share fewer than the public doc promises

Where: `crates/before/src/party/forks.rs:111-114`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): add the saturation clause to the `Forks`, `Party::forks`, and

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### inventory-14 (nit, documentation): roster: approved (ruling 104)

`add_at` doc claims "any i128 magnitude" but the carry arithmetic overflows near the extremes

Where: `crates/suanpan/src/accumulator.rs:1333-1334`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): state the actual precondition (`\

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### paper-fidelity-12 (nit, documentation): roster: approved (ruling 104)

"PartialEq describes a causal ordering" should read PartialOrd

Where: `crates/before/src/lib.rs:277-279`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): replace `PartialEq` with `PartialOrd`; regenerate the README

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### party-21 (nit, claims): roster: approved (ruling 104)

Index doc claims a wall-time measurement no committed artifact holds

Resolution: Delete the clause, or add `party_join_all` to the bench-judge roster (`tools/benchjudge-expected.json`) and cite that cell by name. Acceptance: every measurement claim in the module doc names a committed instrument. Construction: none needed beyond the grep: the claim names no artifact, and `tools/benchjudge-expected.json` has no party entry.

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### rumors-dependence-4 (nit, documentation): roster: approved (ruling 104)

`dangerously_alias`'s "dropped without further use" forbids the read-only uses before's own examples and laws make

Where: `crates/before/src/party.rs:514-520`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): reword the Warning on `Party::dangerously_alias` and `Clock::dangerously_alias` to name the mechanism: while one copy is live ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-coding-19 (nit, documentation): roster: approved (ruling 104)

`expect` message names the wrong artifact

Where: `crates/before/src/version/skyline/encode.rs:36-36`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "a canonical packed preorder stream parses cleanly"

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-coding-24 (nit, documentation): roster: approved (ruling 104)

hand-maintained "depth-2" in two testdocs duplicates `EV_SMALL_DEPTH`

Where: `crates/before/src/version/skyline/tests.rs:477-478`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "of any normal form to the small-scope depth" (477) and "every normal-form tree to the small-scope depth" (570)

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-coding-30 (nit, documentation): roster: approved (ruling 104)

the reset-versus-compensating-subtraction argument is restated at full length three times

Where: `crates/before/src/version/skyline/text.rs:540-549`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): keep the module-doc statement; at 540-549 leave only "A reset, not a compensating subtraction: the module doc's exact-top argument" ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-coding-34 (nit, documentation): roster: approved (ruling 104)

walk.rs's module doc enumerates a client roster that has rotted

Where: `crates/before/src/version/skyline/walk.rs:4-6`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): keep the structural clause and drop the enumeration, or make it explicitly non-exhaustive

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-fill-grow-20 (nit, documentation): roster: approved (ruling 104)

`expand_subtree`'s `# Panics` attributes an unrepresentable state to a normal-form violation

Where: `crates/before/src/version/skyline/fill/fuse.rs:380-385`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Restate the `# Panics` and the assert message: an `Internal` tag has a present child by the coding (`00` is the terminal) ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-fill-grow-5 (nit, documentation): roster: approved (ruling 104)

`# Panics` on tick and ticks documents an empty-id state the `Party` type excludes

Where: `crates/before/src/version/skyline/fill.rs:205-208`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete the empty-id sentences from `tick`, `ticks`, and `grow::emit`'s docs; keep `emit`'s debug assert

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### skyline-sweep-place-masked-34 (nit, documentation): roster: approved (ruling 104)

The debug-assert rationale paragraph is copied six times across sweep, masked, and place

Where: `crates/before/src/version/skyline/sweep.rs:123-127`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): keep every assert and each site's first sentence (the site-specific control-flow fact); delete the "keeps that argument loud .. ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### span-causally-18 (nit, documentation): roster: approved (ruling 104)

A testdoc hardcodes "depth 2" for a constant defined elsewhere

Where: `crates/before/src/span/tests.rs:608-609`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "The small-scope sweep is exhaustive to `EV_SMALL_DEPTH`; ..."

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### span-causally-7 (nit, documentation): roster: approved (ruling 104)

The `v + w` absence argument is written out three times in `algebra.rs`

Where: `crates/before/src/span/algebra.rs:39-43`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): keep the module-doc statement; reduce 672-675 to a pointer ("a version-pair `+` is deliberately absent ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suanpan-27 (nit, documentation): roster: approved (ruling 104)

Incident history in roster prose ("twice found wrong in review")

Where: `crates/suanpan/src/claims.rs:12-14`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): at both sites, "the table is committed data held to the roster; an edit to either without the other is a named failure"

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suanpan-6 (nit, documentation): roster: approved (ruling 104)

The register-metering sentence says every absorbed delta or shift counts one touch; zero deltas and `shl(0)` count none

Where: `crates/suanpan/src/lib.rs:277-279`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): amend here and at touch_meter.rs:9-12: "a nonzero delta, a sign query, a negation ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### suanpan-tests-22 (nit, documentation): roster: approved (ruling 104)

witness docs narrate the mutation history in the past tense, three times

Where: `crates/suanpan/src/accumulator/tests/witnesses.rs:6-9`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): module doc: "each witness pins a corner whose mutation passes every other committed test". Test docs (:28-33 ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### testing-diff-gen-13 (nit, documentation): roster: approved (ruling 104)

`skip_stress_pair` cites a "bounded lazy-skip" that no longer exists in either `is_disjoint` kernel

Where: `crates/before/src/testing/generators.rs:206-208`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Re-denominate: "drives the per-level dominated-subtree skip (`IdReader::skip`) in the lockstep disjointness walk to Θ(scale) skips of O(1) subtrees ea ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### testing-oracles-6 (nit, documentation): roster: approved (ruling 104)

The bridge's impl-to-oracle section comment restates the module doc, repeats the recursion caveat four times, and coins "master harness"

Where: `crates/before/src/testing/bridge.rs:97-107`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Reduce the section comment to the banner plus one sentence saying `to_oracle_*` is the inverse of `from_oracle_*` ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### version-core-3 (nit, documentation): roster: approved (ruling 104)

Small typographic and consistency slips across the partition

Where: `crates/before/src/version.rs:320-321`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): (a) delete one blank; (b) `= a`; (c) "so materialization is kept explicit"; (d) `&v / &p` ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### version-core-30 (nit, documentation): roster: approved (ruling 104)

Hand-maintained pair counts in test names, docs, and a const doc

Where: `crates/before/src/version/tests.rs:840-855`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): name the bounds (`const RANK_CMP_SWEEP_PAIRS: u32 = 25_000;` and its wide-arm twin), cite them by name in the docs ...

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

### version-core-6 (nit, documentation): roster: approved (ruling 104)

Ghost name `alice` in the `span` example

Where: `crates/before/src/version.rs:581-581`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `// concurrent to a's line`

Roster note: approved by ruling 104; lands per the quoted Resolution and Acceptance, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives or conflicts with a ruling.

