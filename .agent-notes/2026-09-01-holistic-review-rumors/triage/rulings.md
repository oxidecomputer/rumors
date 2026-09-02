<!-- CAVEAT LECTOR: the ruling texts below are transcribed by Claude from Finch's decisions in triage sessions; the decisions are Finch's, the wording is Claude's unless quoted. Read with the ground rules in ../../README.md. -->

# Rulings

One numbered entry per decision, in the order made. A ruling names the
finding ids or the owner-decision number it disposes, states the decision
positively, and, for `model` and `defer`, names the home where the intent
now lives (a rustdoc sentence, an AGENTS.md line, a design document, a
shadow-tracker issue). `ledger.tsv` cites rulings by number in its
`ruling` column; `ledger.py check` refuses a `model`, `dispute`, `defer`,
or `fix-amended` row that cites none.

Format:

    ## T<n> (<date>): <one-line title>
    Disposes: <owner decision number and/or finding ids>
    Decision: <the ruling, stated positively>
    Home: <where the intent is recorded, for model/defer; "code" for a fix>
    Reasoning: <optional; the why, when it is not obvious from the decision>

Rulings are appended, never edited; a reversal is a new ruling that names
the one it supersedes.

---

## T1 (2026-09-02): The record is the ledger plus this file
Disposes: TRIAGE.md open question 1
Decision: `ledger.tsv` is the record of what was decided and this file is the record of why; the six topic documents stay as written, as evidence. No disposition lines are generated into the documents.
Home: this file and `ledger.tsv`

## T2 (2026-09-02): Highs, mediums, and owner-gated entries are ruled individually
Disposes: TRIAGE.md open question 2
Decision: Every high, every medium, every owner-gated entry, and anything a lane agent flags gets Finch's explicit disposition before it lands. Lows and nits are swept inside lane rosters Finch approves and reviewed as diffs.
Home: this file; TRIAGE.md's session protocol is amended by this ruling (mediums join the individually ruled set).

## T3 (2026-09-02): Instruments first
Disposes: TRIAGE.md open question 3
Decision: P1 runs before P2. P2's three harness-independent members (deps-3, the exactness clamp in mirror-common-8/remote-codec-14, and the documentation half of async-hazards-3) may land in parallel with P1.
Home: TRIAGE.md, Phases.

## T4 (2026-09-02): Nothing is deferred without a per-item ruling
Disposes: TRIAGE.md open question 4
Decision: No entry takes the `defer` disposition by default rule, class, or pattern. Each candidate deferral is put to Finch individually with its proposed home, and the ruling that admits it names that home. The plan's proposed homes (a `design/` document for module-reshaping proposals, the shadow tracker otherwise) are the menu, not the default.
Home: this file; `ledger.py check` continues to refuse a `defer` row citing no ruling.

## T5 (2026-09-02): The renderer-vocabulary re-accept class folds into the format-change class
Disposes: owner decision 81; prose-hygiene-6, verification-infra-15 (fix-amended)
Decision: AGENTS.md keeps one snapshot re-accept class: a deliberate, owner-ruled change named explicitly in the re-accepting commit. The separate renderer-vocabulary class and its hexdump-line-preservation witness are removed from the hard rules. A renderer change that moves `insta` snapshots is re-accepted the same way a wire change is: ruled by Finch on the diff, named in the commit. The remote-capture-atlas-13 fix therefore stops for that ruling when its diff is ready; it does not carry a standing authorization from this ruling.
Home: AGENTS.md, Hard rules (the snapshot paragraph).

## T6 (2026-09-02): The backend conformance suite gets every repair, obligations folded into the checks
Disposes: owner decision 48; conformance-28, -24, -25, -30, -31, -40, -41, -42
Decision: All repairs land in one lane, the census liveness floor first so that it sizes `LOCAL_BUDGET` above the flat decode-fan pre-charge before anything else is resized. The three transcribed obligations (decode-slot padding, residency across the erase/assume re-tag, monotonicity between grid points) become pointwise checks in the suite rather than prose admissions.
Home: code.

## T7 (2026-09-02): `future_size` runs under the dev profile like every other test
Disposes: owner decision 42; tests-wire-format-26 (with verification-infra-1 and suite-economics-1 as cross-references), tests-wire-format-25
Decision: The `cfg(not(debug_assertions))` gate is lifted; the three budgets are re-measured under the dev profile and pinned there; a first-run failure is a finding to triage, never a reason to gate the binary back out. The module doc is restated against `Reconciliation::reconcile` in the same change.
Home: code.

## T8 (2026-09-02): The bookmark-causality harness gets all four repairs
Disposes: owner decision 49; tests-bookmark-9, -12, -11, -8
Decision: A per-network redaction ledger with a survival check (the recycle oracle); a fault-regime flag that asserts `Ok` wherever a session cannot legitimately fail; a seeded RNG threaded through `World` via `seed_rng`, with generated networks asserted pairwise distinct; and the `join!`-over-owned-links experiment to restore the stall detector, falling back to a per-session timeout that names the step if EOF does not propagate. Ordering: -12 before -9, since the oracle repair is observable only once the harness stops swallowing errors.
Home: code.

## T9 (2026-09-02): The renderer's injectivity pin is an inverse parser
Disposes: owner decision 51; remote-capture-atlas-17
Decision: The committed pin parses a rendering back to the bytes it was rendered from and asserts the round trip, over the generated corpus; this is chosen over the cheaper leaf-mutation property because it also catches the float-width and trailing-byte survivors. It lands beside the remote-capture-atlas-13 renderer fix and is the demonstration that the fix restores injectivity.
Home: code.

## T10 (2026-09-02): The envelope certificate moves into `window/tests.rs`; the simulator's copies retire
Disposes: owner decision 43; benches-envelope-32, benches-envelope-28, benches-envelope-29, streaming-backend-window-32
Decision: The exact-Chernoff oracle becomes a differential proptest in `window/tests.rs` over the shipped `occupied`, `jointly_occupied`, `children_quantile`, and `stage_population`, extended to asymmetric `(A, B)`. Once that proptest demonstrably fails on a lowered shipped quantile (the committed negative control), the example's integer copies of the envelopes, its "landed" flat baseline, and its `L(N)` section are deleted. `window.rs` cites the proptest as its certificate.
Home: code.

## T11 (2026-09-02): No rumors mutation campaign
Disposes: owner decision 44; verification-infra-3 (dispute)
Decision: A mutation campaign over `rumors` is not a verification instrument of record. No recipe, no run, no cadence. The existing campaign notes stand as history in git and `.agent-notes/`; the `.cargo/mutants.toml` roster is untouched by this ruling.
Home: this file.
Reasoning: Finch's call ("don't worry about it"), recorded as a dispute of the finding's premise that the campaign is owed a confirming re-run.

## T12 (2026-09-02): The coverage pin stays scoped to `before`
Disposes: owner decision 45; verification-infra-4 (dispute)
Decision: `covcheck` keeps its current scope. Rumors is not added to the coverage judgment.
Home: this file.

## T13 (2026-09-02): The overlap shadow gets a validity meta-test and nothing else
Disposes: owner decision 50; tests-observation-28 (fix-amended), tests-common-12 (fix-amended)
Decision: `arb_overlap_schedule_with_shadow` is exposed and the overlap twin of `shadow_predicts_live_state` lands in `tests/shadow_validity.rs`. The executor's observed-guard stays a guard: no assertion, no skip count, no ceiling, and the shadow's fork point stays at `Open`. The docs at the `Open` variant and the module doc are restated to describe the Open-time fork as the model and the guard as the consequence of the model and the protocol forking at different points.
Home: code.

## T14 (2026-09-02): The rumors fuzz target is deferred; its home is the design document
Disposes: owner decision 46; verification-infra-5 (defer)
Decision: No fuzz target lands in this triage. `design/rumors-frame-fuzz.md` is the home of the deferred work; it already states the target's specification. No tracker issue is filed.
Home: `design/rumors-frame-fuzz.md`.

## T15 (2026-09-02): The coverage legs join `just all`; the header names `ci` as the CI job's recipe
Disposes: owner decision 47; verification-infra-2
Decision: `just all` runs the coverage legs, so the local ladder cannot pass while CI's coverage job fails. The justfile header is restated so `ci` is described as the recipe the CI job runs, not as a mirror of every check. The gate is unchanged.
Home: code (the justfile).

## T16 (2026-09-02): `sync_memory_budget`'s rustdoc stops quoting the tradeoff probe
Disposes: owner decision 53; verification-infra-14 (fix-amended), tests-resource-link-window-18 (fix-amended)
Decision: No recipe is added for `tests/tradeoff_probe.rs`. The public rustdoc of `Peer::sync_memory_budget` stops quoting a measurement nothing enforces; the probe remains a hand-run instrument and its own module doc says so. The justfile's opening claim that every artifact has a recipe is restated to admit hand-run probes, in the same change.
Home: code.

## T17 (2026-09-02): The operating-envelope figures are derived by a committed test; `results/` is excised
Disposes: owner decision 54; verification-infra-6
Decision: A committed test derives the crate doc's operating-envelope figures from the pinned per-message wire law; the crate doc cites the constant by name with its validity band. The `results/` directory is deleted; its history stays in git.
Home: code.

## T18 (2026-09-02): `window_census` takes the census lock
Disposes: owner decision 55; tests-resource-link-window-19
Decision: `window_census` guards its peak measurement with the same lock `decode_alloc.rs` uses, so `cargo test` is a supported entry for it as well.
Home: code.

## T19 (2026-09-02): Cancellation and wide-window coverage: two of three instruments
Disposes: owner decision 56 (correctness open question 14, verification open question 20)
Decision: Two instruments land: a walk-tier proptest that cancels the `mirror` future at a drawn poll count and asserts the invariants that must survive cancellation, and a wide-window arm in `streaming_matches_join_oracle`. The deterministic cancel-at-every-poll-prefix sweep over a serving bootstrap and a retirement is declined.
Home: code.

## T20 (2026-09-02): The docs.rs fix lands with a `--cfg docsrs` leg in both `gate` and `ci`
Disposes: owner decision 57; deps-3
Decision: The `doc_cfg` configuration is corrected, and a nightly `--cfg docsrs` rustdoc leg is added to `gate` and to `ci`, so the docs.rs build path is exercised before every commit.
Home: code.

## T21 (2026-09-02): `ReorderingAcceptor` gets its demonstration; the conformance duplicate unifies onto it
Disposes: owner decision 60; testing-infra-12, conformance-18
Decision: A unit-level test demonstrates that the patient wait yields a batch of two accepts. The proxy-tier `== 0` reordering assertion stays as previously ruled. Conformance's `ReversingAcceptor` is replaced by the shared `testing::ReorderingAcceptor` provided the conformance verdicts hold under the patient wait; if they do not, that is a finding to report, not a reason to keep the duplicate silently.
Home: code.

## T22 (2026-09-02): The proxy harness runs production's arrangement by default; the `Connect` impls stay
Disposes: owner decision 61; remote-proxy-tests-24, remote-proxy-tests-5 (fix); the impls' "no production caller" claim (model)
Decision: The harness default becomes production's arrangement and the seven hand-rolled two-proxy topologies consolidate onto `harness::drive`. The remote proxy's `Connect` and `CompleteConnect` impls are kept as the alternative arrangement, with a doc at the impls stating that they exist for the harness's wire-path arrangement and have no production caller.
Home: code (the impls' rustdoc).

## T23 (2026-09-02): Streams two and above are reached through the collision-schedule test mode
Disposes: remote-proxy-tests-10 (fix-amended), with tests-wire-format-7 and tests-wire-format-18
Decision: Deep tree geometry is unreachable through the public API under blake3-derived leaf paths, so the gap is closed by implementing the collision-schedule test mode designed in `.agent-notes/2026-08-21-collision-schedule-test-mode/`: an injective, collision-planning leaf-path schedule behind `test-internals`, activated by an environment variable, with `assume_blake3()` marks carrying a rationale for every test that pins hash-derived geometry, and a first-sweep triage in which every failure resolves to a bug or a marked assumption. The acceptance for remote-proxy-tests-10 is a frame census under the mode observing frames on logical streams with index two or greater in each direction with roots equal to the oracle. The note's four open questions (variable name and seed format, seed sweep, gate versus `ci` tier, partial clusters) are put to Finch when the lane reaches them.
Home: code; the design note is the record of the design.
Reasoning: Finch: "This is impossible unless you inject a different hash function which engineers deep collisions."

## T24 (2026-09-02): The sync decoder oracle stays, documented, with its duplicated fragments dissolved
Disposes: owner decision 58; remote-codec-8, remote-codec-10
Decision: The synchronous decoder remains the oracle side of the codec differential. Its struct gains a doc stating that role. The fragments copied between it and the async reader (the over-budget gate, `record_prefix`, EOF classification) become one definition both use, and a listing-head fixture is driven through the async path so the differential sees listing defects. Supersedes the "delete" answer given earlier in the same session, on the blast radius: `decode_both`'s ten sites, the error atlas's read witnesses, and remote-codec-14's acceptance all rest on the oracle.
Home: code.

## T25 (2026-09-02): The review README points at the triage record
Disposes: TRIAGE.md open question 5
Decision: One sentence under the README's "Reading the note" names `TRIAGE.md` and `triage/` as the disposition record.
Home: `README.md` of this note.

## T26 (2026-09-02): P1 entries confirmed to land per their stated resolution and acceptance
Disposes: streaming-tests-11, tests-disruption-handshake-7, materialized-27, tests-resource-link-window-20, tests-wire-format-25, deps-1, tests-lifecycle-15, remote-proxy-19, verification-infra-7, verification-infra-8, verification-infra-9, remote-proxy-tests-5, tests-bookmark-11, tests-bookmark-8 (fix); owner decision 59 needs no ruling (the fix lands regardless of severity)
Decision: Each lands exactly as its entry's Resolution states and is judged by its Acceptance. A lane agent that must deviate stops and reports; the entry stays open until ruled.
Home: code.

## T27 (2026-09-02): The swarm example is deleted
Disposes: owner decision 79 (moot); every `swarm-example-*` entry (fix-amended: resolved by deletion)
Decision: `examples/swarm.rs` and `examples/swarm/tests.rs` are deleted, along with the `[[example]]` entry in `Cargo.toml` and every recipe, workflow step, and prose reference to the example. The crate carries no showcase example of that shape; the measurement it offered is not replaced by this ruling.
Home: code.
Reasoning: Finch: "The swarm example should be deleted."

## T28 (2026-09-02): The P1 lane roster is approved
Disposes: deps-7, verification-infra-12, verification-infra-13, deps-8, deps-9, tests-resource-link-window-25, tests-resource-link-window-28, tests-disruption-handshake-3, tests-disruption-handshake-17, testing-infra-4, verification-infra-10 (fix)
Decision: These land in the P1 lane per each entry's stated Resolution, judged by its Acceptance, and are reviewed as the lane's diff. streaming-tests-28 (owner decision 52) is not in this ruling; it awaits its own.
Home: code.

## T29 (2026-09-02): The Lean wedge literal stays a human-checked transcription
Disposes: owner decision 52; streaming-tests-28 (model)
Decision: `lean_wedge_literal()` remains a hand transcription of `Mux.wedge`. No gate leg compares the two. The doc at the literal states that the transcription is human-checked against `Instances.lean` whenever either side changes, and why that discipline is acceptable: the literal is twelve scopes, the generator pin and the session pin hold the Rust side rigid, and a Lean edit to the witness is itself an owner-level change to the theorem's subject.
Home: the rustdoc on `lean_wedge_literal` in `src/tree/mirror/streaming/tests/wedge.rs`.

## T30 (2026-09-02): `testdoc` recognizes `#[pollster::test]`
Disposes: tests-observation-37 (fix, entry of record); remote-proxy-tests-27, tests-disruption-handshake-33 (dup)
Decision: `tools/testdoc`'s attribute pattern recognizes `#[pollster::test]`, and a committed probe demonstrates that an undocumented pollster test fails the gate.
Home: code.
