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

## T31 (2026-09-02): A pooling `Dial` is a first-class deployment shape
Disposes: owner decision 36; link-28, link-14 (fix)
Decision: Recovered pooled connections get their own bound, applied before `READY`, so a stalled fresh header can never evict an admitted idle connection. `set_nodelay(true)` in the routed TCP guidance and harnesses, and one sentence on `Conn`, land in the same change. The constructed in-memory eviction test pins the old failure.
Home: code.

## T32 (2026-09-02): The `Bookmark` doc states the checkpoint reclaim rule; a test pins it
Disposes: owner decision 37; session-bookmark-21 (fix)
Decision: The public `Bookmark` doc is reworded to the code's rule: reclaim happens at the first unsuppressed pre-session checkpoint after dominance. The constructed scenario lands as a test pinning that timing.
Home: code.

## T33 (2026-09-02): The exactness clamp lives in `resume_payload`
Disposes: owner decision 38; mirror-common-8, remote-codec-14 (fix)
Decision: `resume_payload` clamps every read so no iteration fills past `len`, and its doc states the one remaining precondition (`payload.len() <= len`). The demonstrated fixtures at both altitudes land as the tests, through the kept sync-oracle differential (T24).
Home: code.

## T34 (2026-09-02): No user destructor runs under the watch write lock
Disposes: owner decision 39; async-hazards-3 (fix-amended: full evacuation)
Decision: At both commit sites the pre-image root leaves the `send_if_modified` closure through an `Option` and drops after the lock is released; the act and join walks push every discarded incoming value (a causally-skipped action's message, a duplicate or deletion-filtered incoming subtree) into a sink that is returned from the closure and dropped after the lock as well. The public payload guidance states that destructors never run under the replica's lock. The demonstrated hang (a `redact` whose payload destructor calls `snapshot()`) lands as the pin and must complete. The measurement of a large `send_all` under the lock stays with owner decision 64 in P7.
Home: code.
Reasoning: Finch proposed the `Option` mechanism; the sink covers the two mid-walk last-handle drops of incoming values, which the `Option` alone does not reach.

## T35 (2026-09-02): Content changes always advance the frontier; a party-only change wakes no one
Disposes: owner decision 40 (correctness open questions 15 and 17)
Decision: A content change without a frontier advance is impossible: every action in `Tree::act` ticks the party before it is applied (forgets included), a gained leaf carries a version outside the local ceiling (a leaf inside it that we lack was redacted, and deletion honoring refuses it), and a shed leaf requires the peer's ceiling to contain the redaction tick, which we cannot already hold because holding it means we already shed the leaf. The `Changes` field doc states this argument in one sentence. The wake rule at both sites is: a party-only change (take or fork) notifies no watcher; content processing is independent of the party, and the user never reasons about it. `gossip.rs`'s take-or-fork site adopts `bookmark_update`'s rule. The lane verifies that no consumer relied on the party-only wake (the retire, bootstrap, and bookmark suites are the check) and stops if one did.
Home: code (the `Changes` field doc; one comment at the shared wake rule).
Reasoning: Finch: "the frontier always updates when content changes" (verified against `Tree::act` and the join's deletion honoring); "we shouldn't wake anyone when the party only changes."

## T36 (2026-09-02): `ErasedPrefix::assume`'s length check is a release assert
Disposes: owner decision 41 (correctness open question 16)
Decision: The O(1) length-versus-height check in `ErasedPrefix::assume` is a release-profile `assert!` carrying the proof, so a cross-height re-tag panics as programmer error in every profile instead of misplacing a leaf silently.
Home: code.

## T37 (2026-09-02): The in-flight party lives in the retire future; `Inner.party` is not an `Option`
Disposes: owner decision 94; api-core-2 (fix-amended: the redesign lands now)
Decision: `Peer::retire` holds the party it is moving inside the retire future rather than leaving a `None` behind in `Inner`, so `Inner.party` becomes a plain `Party`, `Batch::commit`'s no-party arm and its sibling unreachable arm disappear, and no release path can drop a batch silently. This lands in P2, not as a later design item.
Home: code.

## T38 (2026-09-02): The leaf stores its action's version; the running join is dissolved
Disposes: owner decision 95; tree-core-30 (fix-amended)
Decision: `Z::act` stores the applied action's own version at the leaf and the per-key running join is removed: the ceiling observer already joins every effectual action's version, and under `Tree::act` the versions at one key form a causally ascending chain (each action ticks the party in specification order; an insert's path is fresh from its post-tick version, so only forgets of an existing path share a key), so the join at a key equals the last version and a surviving leaf has exactly one. `react`'s doc states that it is `act`'s commit section over versioned actions and that its contract assumes the ascending per-key order `act` guarantees. The two constructions in tree-core-30 land as tests of the stated contract (the first now yields the insert's version; the second is documented as outside the contract or made to hold, the lane reports which). Simplifications the removal exposes are reported, not landed unbidden.
Home: code.
Reasoning: Finch: "the running join is the same as taking the last, in all production code ... the version uniquely keys the path ... some simplification can be done here." Verified at `Tree::act`.

## T39 (2026-09-02): The single-sort tier of `act` lands in P2
Disposes: tree-core-27 (fix-amended: the single-sort tier only; the slice-recursion redesign stays with owner decision 64 in P7), tree-typed-30's fan-doc contradiction rides along
Decision: `act` sorts its action list once at entry and merges on reassembly, replacing the per-height sort and re-materialization. It lands beside the destructor evacuation (T34), since both touch the commit path, and is measured with the existing benches at the parent commit before the change is credited. The deeper redesign is ruled in P7.
Home: code.

## T40 (2026-09-02): The walk's end legs share the `Resolver`'s classifier; "aborts typed" leaves the prose
Disposes: materialized-14 (fix-amended); a crate-wide vocabulary ruling
Decision: The lane first resolves the recorded opening-leg liveness gap (a violation raised before the opening's first yield must return an error over a wire rather than stall), then routes the terminal and opening legs through `Resolver::react` so one classifier serves every height, falling back to per-arm reclassification only if the ends genuinely differ (reported, not assumed). The early-supply structural checks and the trailing-reply check land either way, each malformed shape pinned by a committed injection that fails on the current arms; `absorb` moves the accepted leaf instead of cloning it. Vocabulary: the phrase family "fails typed", "aborts typed", and kin is excised crate-wide in favor of "returns an error"; the sweep runs its regenerating grep to zero in the commit that lands it.
Home: code; the vocabulary rule joins the P3 convention sweeps.

## T41 (2026-09-02): The async opener read surfaces a recorded transport failure in wire order
Disposes: remote-codec-11 (fix)
Decision: The bulk opener read keeps a pending-failure slot; the bytes in hand are parsed, and the first item that needs more bytes returns the recorded failure as a read error at that part rather than re-reading the transport. The non-sticky failing async reader fixture (one byte, then an error, then end-of-stream) pins that both decoders classify it identically; the sticky-error and full-delivery cases are unchanged. Lands beside the exactness clamp (T33).
Home: code.

## T42 (2026-09-02): `warm_caches` forces the memos it lists
Disposes: tree-core-8 (fix)
Decision: `warm_caches` forces `version_bytes` (which forces the bounds span, so the separate ceiling and floor calls go), its doc names the memos it forces rather than claiming "every", `Snapshot::warm_caches` mirrors the wording, and a `cfg(test)` probe on the node pins that the memo is populated after warming.
Home: code.

## T43 (2026-09-02): `examples/envelope_sim.rs` is deleted
Disposes: benches-envelope-34, benches-envelope-31 (fix-amended: resolved by deletion); amends T10
Decision: Once the differential proptest of T10 demonstrably fails on a lowered shipped quantile, the whole example is deleted, its `[[example]]` entry and every recipe and prose reference with it. Nothing of its Monte Carlo tiers is kept; the certificate of record is the in-tree proptest. The P1 envelope brief's "empty example" stop is resolved by this ruling.
Home: code.

## T44 (2026-09-02): The advertised name travels as a validated-length newtype
Disposes: link-25 (fix-amended)
Decision: The routed link's advertised name is carried as a newtype whose constructor is the one place its length is checked against `MAX_ADDR_LEN`, so `link_header` takes the type and writes its length without a cast or a guard; the `debug_assert!` and the `as u8` both go. The endpoint's construction-time validation moves into that constructor.
Home: code.

## T45 (2026-09-02): The router's read-id counter wraps
Disposes: link-29 (fix)
Decision: `next_id` advances with `wrapping_add(1)`, with a one-line comment that at most `pending_headers` ids are live at once, so distinctness among live entries is all the counter owes.
Home: code.

## T46 (2026-09-02): Pub-in-private is the recorded convention under private modules
Disposes: owner decision 1; inventory-10, module-graph-12, tree-typed-1, streaming-backend-window-2, materialized-6, mirror-common-20, session-bookmark-44, api-audit-14, remote-codec-21, inventory-14, inventory-19, session-bookmark-17, link-22, tree-core-25 (model, except where an entry's remainder is a doc-voice fix)
Decision: No `unreachable_pub` lint. Under a private module, `pub` and `pub(crate)` are both admitted and carry no API meaning; the convention is stated in one AGENTS.md line, and the comment at `typed.rs:19-22` is reconciled with it. The remaining halves of these entries (rustdoc on unreachable items written in library-user voice, such as `Message`) are re-voiced for the maintainer in the module lanes.
Home: AGENTS.md (one line); `src/tree/typed.rs`'s module comment.

## T47 (2026-09-02): A `[lints]` table with seven lints
Disposes: owner decision 2; clippy-pedantic open question; api-audit-12, api-audit-7, api-core-34, tree-core-5, link-5 (the mechanical halves)
Decision: `Cargo.toml` gains a `[lints]` table enabling `clippy::elidable_lifetime_names`, `clippy::redundant_closure_for_method_calls`, `clippy::manual_let_else`, `clippy::match_wildcard_for_single_variants`, `missing_debug_implementations`, `unnameable_types`, and `missing_docs`, each landed at `warn`, swept to zero, then promoted so it rides `-D warnings`. `unnameable_types` and `missing_docs` enumerate P6's mechanical half; their sweeps land with the API pass where the fix is an API decision (decision 15) and in P3 where it is not.
Home: `Cargo.toml` `[lints]`.

## T48 (2026-09-02): Em-dashes leave non-doc comments; a gate check holds it
Disposes: owner decision 3; prose-hygiene-9, prose-hygiene-8, and the eighteen per-partition nit rows of the em-dash class (fix)
Decision: One mechanical sweep rewrites every em-dash in `//` and `#` comments and in assert strings, using the greps prose-hygiene-9 records; a `tools/` check on U+2014 in non-doc comment lines and string literals of `.rs`, justfile, `.toml`, and `.yml` files joins `just gate` in the same series, at zero. Rustdoc em-dashes are decision 13's question and untouched here.
Home: code (`tools/`, wired into the gate).

## T49 (2026-09-02): "seam" and "knob" are swept out entirely
Disposes: owner decision 4; prose-hygiene-10, prose-hygiene-11 (the knob half), mirror-common-34, conformance-22, session-bookmark-5, fresh-eyes-11, link-13, streaming-backend-window-3, tree-core-12, tests-bookmark-22, tests-resource-link-window-6, testing-infra-6 (the seam and knob halves) (fix)
Decision: Neither word survives anywhere in the tree, the height-erasure site included: every use is rewritten as the mechanism it names ("boundary", "setting", or the specific thing). The sweep runs prose-hygiene-10's regenerating greps to zero in its commit; no standing check.
Home: code.

## T50 (2026-09-02): "honest" means the trust premise and nothing else
Disposes: owner decision 5; conformance-32, remote-proxy-tests-14, tests-common-23, session-bookmark-16, tests-resource-link-window-6, testing-infra-6, tests-observation-19, mirror-common-36, tests-wire-format-15, remote-codec-2, swarm-example-4 (moot), benches-envelope-26 (moot), and the "honest" halves of tree-core-12, streaming-backend-window-3, remote-proxy-22, remote-adapter-streams-11 (fix)
Decision: One prose commit reserves "honest" for the authenticated-honest-peer trust premise. Every other use becomes the accuracy term it means (accurate, complete, delivered, well-formed, self-consistent), and the fixtures are renamed: `HONEST_LEN`, `Dishonest`, `Knob.honest`, `is_honest_error`, `GreetingLie`, and their kin. "lie", "lied", "deceived", and "malicious" become "misdeclared" or the mechanism they name. The regenerating grep runs to zero in the commit.
Home: code.

## T51 (2026-09-02): Public rustdoc opens in the imperative
Disposes: owner decision 6; api-core-17 (fix-amended)
Decision: The first sentence of every public type's and module's rustdoc is imperative, matching `Peer`, `Rumors`, `Bootstrap`, and the observers; `Batch`, `Snapshot`, `Network`, and every other third-person opener are rewritten, and the three sentences that appear in both moods are unified. `tools/doclint` gains the mood check if it can be expressed cheaply; otherwise the sweep is one-time.
Home: code.

## T52 (2026-09-02): `group_imports` on the pinned nightly
Disposes: owner decision 7; tests-wire-format-10 and the import-hygiene pattern's 22 sites, api-core-31, materialized-9, mirror-common-7, mirror-common-18, remote-adapter-streams-18, remote-capture-atlas-26, remote-codec-13, session-bookmark-41, testing-infra-3, tests-common-26, tests-wire-format-24, tree-core-19, tree-typed-4, tree-typed-15, clippy-pedantic-5, clippy-pedantic-15, inventory-15, module-graph-10, remote-adapter-tests-5, remote-proxy-tests-3, remote-proxy-11, session-bookmark-2, tests-common-1, tests-lifecycle-12, tests-observation-21, tests-resource-link-window-17, tests-disruption-handshake-29, streaming-tests-21, remote-capture-atlas-30, swarm-example-25 (moot) (fix)
Decision: `rustfmt.toml` gains `group_imports = "StdExternalCrate"`, and `fmt`/`fmt-check` run on the pinned nightly toolchain the gate already names. The first run is the sweep; the qualified-path stragglers are merged into their imports by hand in the same commit.
Home: `rustfmt.toml`; the justfile.

## T53 (2026-09-02): Test modules live in sibling files, without exception, and a check holds it
Disposes: owner decision 8; module-graph-6 (with mirror-common-22, suite-economics-11 as cross-references), streaming-backend-window-14, testing-infra-5, tree-core-25 (the test-block half); documentation open question 22 (declined) (fix)
Decision: All six inline `#[cfg(test)] mod tests {}` blocks move to sibling `tests.rs` files, scaffolding files included; a `tools/` check that no `.rs` file contains an inline `cfg(test)` module body joins `just gate`. The five inline production modules named in module-graph-14 are a separate P5 question and untouched here.
Home: code (`tools/`, wired into the gate).

## T54 (2026-09-02): One module-wide `type_complexity` allow; the item-level copies go
Disposes: owner decision 9; inventory-3, materialized-8 (the allow half), mirror-common-29 (fix)
Decision: The module-wide allow at the streaming module root stays, with its reason stated once (the phase-schedule types); the twelve item-level allows and `protocol.rs`'s inner allow are deleted.
Home: code.

## T55 (2026-09-02): The `#[non_exhaustive]` rule lives in `error.rs`; the growing seven open
Disposes: owner decision 10; session-bookmark-46, remote-codec-18, remote-capture-atlas-35, remote-adapter-streams-22, api-audit-10, link-17, and the exhaustiveness half of materialized-17 (fix)
Decision: `src/error.rs`'s module doc states the rule: an error taxonomy that grows with enforcement is `#[non_exhaustive]`; a wire-grammar or contract-outcome enum is closed. `EncodeError`, `SendError`, the adapter's `EncodeError<E>`, `EncodeErrorKind`, `DecodeLeafError`, `LeafRunError`, and `DecodeSignalError` become open; every deliberately closed public enum carries one comment naming the rule. `EndpointError` and `LinkError` are classified under the same rule in the same commit.
Home: `src/error.rs` module doc.

## T56 (2026-09-02): "V2" stays only where the wire number is the subject
Disposes: owner decision 11; tests-common-7, session-bookmark-12 (fix)
Decision: Prose that describes behavior drops "V2"; prose whose subject is the wire dialect number keeps it, as do identifiers and wire constants. One sweep by that rule.
Home: code.

## T57 (2026-09-02): The illumos lint-allow comments stay, restated platform-free
Disposes: owner decision 12; remote-proxy-22, remote-adapter-streams-11, inventory-13 (fix-amended)
Decision: The nine allows stay at their sites. Each comment is restated without naming a platform the tree does not build for and without "honest" (T50): the allow exists because a target the crate supports flags the item under `-D warnings`. AGENTS.md gains one line recording that the illumos build is a hand run on ox-east-1.
Home: code; AGENTS.md (one line).

## T58 (2026-09-02): A dedicated prose pass on intensifier "genuine(ly)" and rustdoc em-dash density
Disposes: owner decision 13; prose-hygiene open questions 2 and 3 (fix)
Decision: One prose pass, run with fresh-eyes readers per the documentation change policy, removes every pure-intensifier "genuine(ly)" (contrastive uses stay) and rewrites em-dash-heavy rustdoc toward colons, semicolons, and sentence breaks wherever the dash carries no emphasis. It lands after the mechanical P3 sweeps so it reads the tree they leave.
Home: code.

## T59 (2026-09-02): Orphaned seeds are disposed by named commits; `seed_liveness` matches parameters
Disposes: owner decision 14; tests-observation-38, tests-lifecycle-1, tests-common-32, streaming-tests-20, tests-lifecycle-18's consequence (fix)
Decision: Each orphaned seed line (the awaiting-disposition `cc` in `shadow_validity.txt`, the deleted-property line in `retire.txt`, the two `faults.txt` lines naming values no strategy generates) is removed in a commit that names the deleted or changed property, or its comment corrected where the seed still replays; the `async_wire` seeds re-home into `pairwise.txt` in the commit that deletes that binary; "minted" comments are restated in the present tense; `tests/seed_liveness.rs` is extended to match each seed's shrink-note parameter names against the live `proptest!` signature it anchors to.
Home: code.

## T60 (2026-09-02): `Snapshot`'s iterator is a nameable public type
Disposes: owner decision 15; tree-core-5, fresh-eyes-4, inventory-1, api-audit-2 (the Iter half), api-core-34 (fix)
Decision: `Iter` is re-exported at the crate root, `Snapshot::iter` returns it by name, and `IntoIterator for &Snapshot<T>` names the same type. The simplification document's opposite proposal (delete the `IntoIterator` impl) is declined. `unnameable_types` (T47) holds the class thereafter.
Home: code.

## T61 (2026-09-02): `Snapshot` equality is documented as network, live set, and frontier
Disposes: owner decision 16; api-core-35 (fix-amended)
Decision: The `PartialEq` impl stays as it is. The type doc states that `==` compares the network, the live set, and the causal frontier, and contrasts it with `hash()`, which excludes the frontier.
Home: code (the `Snapshot` rustdoc).

## T62 (2026-09-02): `Bookmark::store` takes owned bytes; `BookmarkError` folds into `Bookmark`
Disposes: owner decision 17; session-bookmark-23, api-audit-4 (fix)
Decision: In one pre-release edit, `store` takes the already-encoded record as `Vec<u8>` by value under the same commit-iff-`Ok` obligation, `Serialized` is deleted, `type Error` moves onto `Bookmark` and the `BookmarkError` trait is deleted, and `load` stays reader-shaped. Every generic bound `B: BookmarkError` becomes `B: Bookmark`.
Home: code.

## T63 (2026-09-02): The `rumors::error` pass lands whole, R2 reopened
Disposes: owner decision 18; api-audit-13, api-audit-14 (the error half), remote-codec-30, remote-codec-28, remote-proxy-2, remote-proxy-3, remote-codec-9, remote-adapter-streams-19, mirror-common-10, api-core-7, mirror-common-15, fresh-eyes-10 (fix)
Decision: The codec, adapter, stream, and proxy taxonomies are diagnostic surface: constructors and schedule helpers become `pub(crate)`; `signal::StreamError` is renamed; the two `Display` impls stop routing through `Debug`; a concrete `MirrorError` is constructed at `streaming_error` so the `Infallible` arms and the dead `PayloadDepthMismatch` arm disappear; `GreetingError::Order` is deleted; `RemoteError` states which variants diagnose the local participant and which the peer; `DecodeErrorKind` gains typed `Head { part, source }` and `InvalidListing(ListingIssue)`; `ReplyFrame`'s constructors become infallible and `ReplyFrameError` is deleted; `Preamble::decode` takes `[u8; V2_PREAMBLE_LEN]` and the two defensive `PreambleDefect` variants dissolve, which reopens ruling R2 on the fact that `Staged::buf` is now that array (the commit names the reopening); the flat generic `Error<B>` shape is kept and the reason recorded in `Error::widen`'s doc.
Home: code.

## T64 (2026-09-02): `seed_rng` and `warm_caches` are gated on `test-internals`
Disposes: owner decision 19; api-audit-15, api-core-12, inventory-4, benches-envelope-6, tests-lifecycle-10, deps-5, async-hazards-4 (the gating half) (fix); the `Network`-taking constructor is not added (model until a user asks)
Decision: Both hooks move behind `any(test, feature = "test-internals")` like their siblings; a private inner function keeps `seed` calling `seed_rng`. No documented `Network`-taking constructor is added; if an application test suite needs deterministic network ids, that becomes a fresh API proposal carrying the two-universes hazard. `warm_caches` stays a bench hook with no documented name. `Snapshot::warm_caches`, which has no caller, is deleted (api-core-12).
Home: code; this file for the declined constructor.

## T65 (2026-09-02): `Snapshot` gains `versions()` and `contains()`; `Rumors` reads through a snapshot
Disposes: owner decision 21; benches-envelope-19, api-audit open question on direct readers (fix)
Decision: `Snapshot::versions()` enumerates versions without touching payloads and `Snapshot::contains(&Version)` is a membership test; `Rumors` keeps no readers of its own, and its type doc states that reading goes through `snapshot()`.
Home: code.

## T66 (2026-09-02): Observability additions: the session outcome hook, frame and window-stall counters, settings readback
Disposes: owner decision 22 (three of four parts); session-bookmark-38, remote-adapter-streams-21, materialized-2, api-audit-11, remote-codec-5 (fix)
Decision: `SessionObserver` gains a `finished` hook carrying the session's outcome; `SessionStats` gains `frames_sent`, `frames_received`, and a `window_stalls` readout (zero versus nonzero is the claim it must support); `Peer` and `Rumors` gain getters for `sync_memory_budget`, `target_message_size`, and `payload_depth_limit` and show them in `Debug` output; the effective (saturated) run budget gets a getter and `MAX_RUN_BUDGET_BYTES` is re-exported. The fourth part (the dialer's `Token`, router eviction counters) is ruled separately.
Home: code.

## T67 (2026-09-02): `Rumors::send` returns the `Version` it stamped
Disposes: owner decision 20; tests-common-2, tests-lifecycle-31 (fix-amended); supersedes the recorded ruling at `src/rumors.rs` ("Where the version comes from") for the single-message method
Decision: `send` returns `Version` by value: the type is backed by a refcounted buffer, so the return is an O(1) clone and never touches the payload. A borrowed return is not possible, since the version lives behind the replica's write lock. `send_all` and `Batch` are unchanged (batching still breaks the one-to-one correspondence, and the doc keeps that half of the argument). The harness's six recovery idioms consolidate onto the return value.
Home: code (the `send` rustdoc records the reversal's reason).

## T68 (2026-09-02): The dialer sees its token; router counters follow T31
Disposes: owner decision 22, part four; link-21, link-16 (fix)
Decision: `Endpoint::link` returns `(LinkInfo<D::Addr>, RoutedLink<D>)`, mirroring `Incoming::accept`. After the pooling bound (T31) lands, the endpoint exposes atomic counters for the router events a committed known-bad test moves (the pending-header eviction and the stream-queue overflow at minimum); a counter no test moves is not added.
Home: code.

## T69 (2026-09-02): A `conformance::bookmark` suite ships; a hang under `check` is documented
Disposes: owner decision 23; conformance-1, conformance-6 (fix); conformance-37 is ruled separately
Decision: `conformance::bookmark` validates a caller's `Bookmark` implementation against the commit-iff-`Ok` clause and its siblings, ships with a negative control (a bookmark that commits a partial frame on `Err`) and a paragraph naming what the suite cannot see. `check`'s rustdoc states that a hang under the caller's timeout names no check. The suite lands after T62 reshapes the trait.
Home: code.

## T70 (2026-09-02): `Joined::Bailed` and `Joined::Failed` return the whole builder
Disposes: owner decision 24; fresh-eyes-9 (fix)
Decision: Both outcomes carry the `BookmarkedBootstrap<T, B>` back, so the retry the type's doc promises is one call.
Home: code.

## T71 (2026-09-02): The cancellation probe needs no connect backlog
Disposes: conformance-37 (fix-amended)
Decision: The link conformance suite's cancellation probe is rewritten to poll the peer's acceptor while its two connects complete, so a conforming link with no backlog passes; no requirement is added to the suite's preconditions and no clause to the link contract.
Home: code.

## T72 (2026-09-02): Payload bounds live on the type definitions
Disposes: owner decision 25; api-audit-3, api-core-8 (fix)
Decision: `T: Send + Sync + 'static` is stated once, on the definitions of `Peer`, `Rumors`, `Snapshot`, and the observers; every per-method restatement is deleted, and the crate doc's "demanded once" sentence becomes true as written.
Home: code.

## T73 (2026-09-02): `pub use ::before;` stays, with its reason at the site
Disposes: owner decision 26; deps-6 (model)
Decision: The whole-crate re-export is the version-pinning path for the ITC types and stays; one comment at the `src/lib.rs` re-export records that reason.
Home: `src/lib.rs` (the comment at the re-export).

## T74 (2026-09-02): `observe` accumulates observers
Disposes: owner decision 27; session-bookmark-40 (fix-amended)
Decision: A second `Peer::observe` or `Bootstrap::observe` adds an observer rather than replacing the first; every registered observer receives every callback, in registration order, and the builder docs say so. The shipped tracing adapter is unaffected.
Home: code.

## T75 (2026-09-02): `latest` and `earliest` keep their names, with both definitions stated
Disposes: owner decision 28; tree-core-6 (fix-amended)
Decision: The tree-level docs and the public accessors state that `latest` is the causal ceiling of every action, redactions included, and `earliest` is the floor of the live leaves; the names stay.
Home: code.

## T76 (2026-09-02): `IoFault` becomes an enum carrying the unit only where bytes move
Disposes: owner decision 29; testing-infra-8 (fix)
Decision: The fault injector's type is an enum whose byte-moving surfaces carry the unit and whose others carry none.
Home: code.

## T77 (2026-09-02): Both bootstraps return a typed `Joined`
Disposes: owner decision 30; the tests-disruption-handshake partition's asymmetry item (fix-amended)
Decision: `Bootstrap::join` returns a typed `Joined` like `BookmarkedBootstrap::join`, replacing `Result<Option<Peer>>`; the seven double-`expect` test sites collapse onto the typed outcome. The prior commit's recorded rationale for the asymmetry is superseded by this ruling.
Home: code.

## T78 (2026-09-02): Causal delivery order is arbitrary among concurrent messages; the tests pin the single-pass order
Disposes: owner decision 31; tests-observation-3 (fix-amended)
Decision: The public `CausalMessages` contract stands as written. The private field doc qualifies "deterministic" to "within one ingested backlog". The two tests are re-labeled as pins of the single-pass staging order (rank, then canonical bytes), and a new test constructs the cross-session inversion (a concurrent, lower-ranked message ingested in a later session is delivered after higher-ranked messages from an earlier pass) so that the arbitrary-order clause is pinned as real.
Home: code.
Reasoning: Finch: "You can go backwards in the ordering if a concurrent (but lesser-ranked) message arrives in a subsequent gossip session." Verified: `staged` is documented as the residue of a single ingest.

## T79 (2026-09-02): A publication-preparation phase, P9
Disposes: owner decision 32; deps-12 (fix-amended, moved to P9)
Decision: Publication preparation is its own phase, after the triage phases: every crate in the workspace is licensed MPL-2.0 with correct license headers on every source file; every package carries `publish = false` until Finch lifts it; manifests gain the metadata a clean `cargo publish` needs (description, license, repository, and whatever else the dry run demands) so publishing later is one flag flip; and the workspace is rearranged as publication requires. Nothing is published. The phase is planned as a design document before it runs.
Home: TRIAGE.md (the P9 section); the crate-description sentences are Finch's prose and are approved by him before they land.

## T80 (2026-09-02): `Link` publishes its fields; `LinkParts` dissolves
Disposes: owner decision 33; link-9 (fix)
Decision: `Link`'s fields become public and `LinkParts` is deleted, the decorate-and-rebuild sites moving mechanically onto the fields.
Home: code.

## T81 (2026-09-02): Counts stay `usize` with a runtime check
Disposes: owner decision 34; link-20 (model)
Decision: `Config`'s counts and `memory_with_capacity` keep `usize` parameters checked at the boundary, consistent with the crate's precedent; the precedent is stated once at `Config`.
Home: `Config`'s rustdoc.

## T82 (2026-09-02): `VersionMismatch.local_protocol` stays the enum; the "select" prose is rewritten
Disposes: owner decision 35; api-audit-5, api-core-4, api-core-21, fresh-eyes-2, module-graph-4, prose-hygiene-2, mirror-common-12 (fix)
Decision: The field keeps the `Protocol` enum. Every public site that tells the user to "select" a protocol (the error table's remedy row, the two `Display` strings, `peer.rs`, `protocol.rs`'s "Selectable", the handshake twin) is rewritten to describe the dialect the crate speaks. The vestigial `#[repr(u16)]` and the `Default` derive on `Protocol` are deleted.
Home: code.

## T83 (2026-09-02): Generic wrappers carry no derive-added bounds
Disposes: api-core-5, api-audit-1, tree-core-2 (fix)
Decision: The derives on `Error<B>`, `Snapshot<T>`, `Tree<T>`, `Retire<T, B>`, `Unbookmarked<T, B>`, and `Joined<T, B>` are replaced by manual impls that bound only what the representation inspects, following the rule the crate already states at `bootstrap.rs`; a compile test clones and compares a `Snapshot` over a non-`Clone` payload and converts an `Error` over a non-`Debug` bookmark into `Box<dyn Error>`. The rendered impl headers are the acceptance.
Home: code.

## T84 (2026-09-02): Three strictly widening additions
Disposes: api-core-1, api-core-30, link-5 (fix); link-6 (dup: T80's published fields make the getter unnecessary)
Decision: `redact_all` accepts `I::Item: Borrow<Version>`; the four small `Copy` enums gain `Hash` and `Protocol` gains `PartialOrd, Ord`; the twelve public types without `Debug` gain manual impls printing what is type-agnostic through `finish_non_exhaustive`, with `PartialEq, Eq` on `SessionState`, `Config`, `LinkInfo` and `PartialOrd, Ord` on `Token`. No `Link::session()` getter: `Link`'s fields are public under T80.
Home: code.

## T85 (2026-09-02): Error-variant docs; two `testing` exports; `assert_parent_early` dissolves
Disposes: remote-codec-19 (fix, the prose half; the constructor narrowing is T63), tests-common-6 (fix), materialized-22 (fix-amended: dissolve)
Decision: Every public error variant gets one doc line stating what separates it from its neighbors, and the two docs naming private items are rewritten. `rumors::testing` exports `leaf_path(&Version)` and `decode_bookmark_record(&[u8])`, delegating to the crate's own derivations, and the harness's transcriptions are deleted. `assert_parent_early` and its test are removed, the d5/d6 design-space record left to the model, and `assert_parent_last`'s doc stops pointing at it.
Home: code.

## T86 (2026-09-02): `DEFAULT_TARGET_MESSAGE_SIZE` stays in `codec::budget`, threaded into the walk
Disposes: owner decision 73; module-graph-2 (the constant half), materialized-10 (dup) (fix)
Decision: The constant stays where the frame constants derive it; the walk receives the target at its start instead of importing it from `remote`, which removes that edge of the streaming core's cycle. Its type is unchanged.
Home: code.

## T87 (2026-09-02): The codec owns the stream count; the link cites it by name
Disposes: owner decision 74; link-3, remote-codec-3 (the count half) (fix)
Decision: The stream count is derived at compile time in the codec from `STREAMED_HEIGHT_COUNT` and `STREAM_HEIGHT_STRIDE`; `link::STREAM_COUNT` cites that derivation by name rather than restating the literal, and the pin test that compared two literals becomes a check of the derivation.
Home: code.

## T88 (2026-09-02): `SCOPE_FIXED_BYTES` and `LEAF_REQUEST_BYTES` derive from `size_of`
Disposes: owner decision 75; streaming-backend-window-26 (fix)
Decision: The `Query` and `Resolution` types are named, both constants derive from `size_of`, and if `SCOPE_ENVELOPE_BYTES` moves as a result it is re-pinned in the same commit with the layout attribution stated.
Home: code.

## T89 (2026-09-02): The criterion series ids drop `V2`
Disposes: owner decision 76; benches-envelope-2 (fix)
Decision: The series are renamed now; saved local baselines are discarded; the two unreachable latency-group arms go in the same change.
Home: code.

## T90 (2026-09-02): The `encoded_bits` assertions and the `meter` dev-feature go
Disposes: owner decision 77; tests-observation-35 (fix); supersedes the ruling recorded in 05d87e1b
Decision: Both `encoded_bits` assertions and the `meter` dev-feature are deleted: `Party`'s byte-level equality already implies the size equality they checked. The commit names the superseded ruling.
Home: code.

## T91 (2026-09-02): `tempfile` becomes a dev-dependency
Disposes: owner decision 78; tests-wire-format-21 (fix)
Decision: One manifest line; the transitive presence in the lockfile is made explicit.
Home: code.

## T92 (2026-09-02): The sizing guide moves to a docs-only explanation module
Disposes: owner decision 80; api-audit-9, api-core-15, fresh-eyes-3, api-audit-8 (fix)
Decision: The operator sizing guide and its table leave `Peer::sync_memory_budget`'s rustdoc for a public explanation module beside `reconciliation`; the setter keeps its contract and one link. No public page cites a test file, a test function, a `cfg(test)` constant, or an internal cost function. The earlier rulings that placed the guide on the setter are superseded.
Home: code.

## T93 (2026-09-02): The CBOR evolution rules stay out of the crate doc; the tests are reworded
Disposes: owner decision 82; tests-wire-format-9, tests-wire-format-14 (fix-amended)
Decision: The removal in 3d16765f9 stands as a ruling: the crate doc does not promise unknown-field skipping or `#[serde(default)]` semantics. `cbor_evolution.rs`'s test docs stop claiming the crate documents them and state what each test pins; the serializer-only test is routed through `Peer`.
Home: code.

## T94 (2026-09-02): A file-backed `Bookmark` ships behind a feature
Disposes: owner decision 84; fresh-eyes-8 (fix-amended)
Decision: The crate ships a minimal atomic file-backed `Bookmark` implementation behind a cargo feature (name proposed by the lane, e.g. `fs`), written against the T62 trait shape, with the trait's `# Examples` block pointing at it. It is validated by the `conformance::bookmark` suite of T69.
Home: code.

## T95 (2026-09-02): The outside-the-network adversary is stated explicitly; no off-model label
Disposes: owner decision 83; session-bookmark-34, tree-typed-3 (fix-amended)
Decision: The reconciliation page and the crate docs' trust-model section state the one non-member adversary the model admits: an actor outside the gossip network who can inject messages through a peer's application surface and cause partitions, and who thereby steers which versions are created and merged, and so the paths, indirectly and within bounds. The 24-byte width's 2^96 offline birthday floor is stated as the in-model bound against that actor; no "off-model" label is applied to it. `hash.rs` keeps the type-local accident bound (2^-192 per interior comparison) and cites the page for the rest; its "author of message content" bullet, which prices nobody, is deleted. The wording is Finch's trust model and goes to him for read before it lands.
Home: code (`src/reconciliation.rs`, `src/lib.rs`); this file records the model.
Reasoning: Finch: "The adversary exists outside the gossip network, but may arbitrarily inject messages and cause partitions. This indirectly gives them the ability to manipulate paths into the tree, but not totally arbitrarily."

## T96 (2026-09-02): Memos warm eagerly; the first greeting never hashes a cold tree
Disposes: owner decision 85; async-hazards-4 (fix-amended); amends T42 and T64 for `warm_caches`
Decision: After every commit (outside the lock, per T34) and at every install point that creates a tree (bootstrap install, a rebuilt set), the memos of the new spine are forced, so the greeting does O(fan) work inside a poll. `warm_caches` becomes redundant and is deleted from every handle (superseding T42's body change and T64's gating of it); the benches call nothing. A test pins that a greeting on a freshly installed tree forces no memo. Rides the P2 commit-path lane after T34.
Home: code.

## T97 (2026-09-02): `Snapshot::hash` and `MERKLE_HASH_LEN` leave the public surface
Disposes: owner decision 86; tree-typed-2 (fix-amended)
Decision: `Snapshot::hash` (and `Tree::hash`'s public face) is gated on `any(test, feature = "test-internals")`; the crate root stops re-exporting `MERKLE_HASH_LEN`, which becomes crate-private. Users compare snapshots with `==` (T61) or by readout. The crate's tests and benches keep using the accessor under the feature. The reconciliation page's "twenty-four-byte digests" section stays as the wire argument and no longer links a public constant.
Home: code.

## T98 (2026-09-02): Hop bands stay; recorded measurements leave the comments
Disposes: owner decision 87; tests-resource-link-window-24 (fix)
Decision: `window_corners` keeps its headroom bands; the exact hop counts recorded in comments beside them are deleted.
Home: code.

## T99 (2026-09-02): One enum names the election
Disposes: owner decision 88; remote-codec-32 (fix-amended)
Decision: `Speaker` and `observe::Role` become one public enum; the other is deleted (or is an alias if a name must survive at both paths), and every site uses the one.
Home: code.

## T100 (2026-09-02): The two owner passages in `link.rs` are restated
Disposes: owner decision 89; link-2, link-15 (fix)
Decision: The pooled-flow-control paragraph is restated in the present tense naming the pin that holds it; `Dial::recycle`'s doc says the ready byte has an unspecified value rather than exposing `READY`.
Home: code.

## T101 (2026-09-02): The reordering tripwire is one deterministic case, documented as unreachable
Disposes: owner decision 90; remote-proxy-tests-6 (fix)
Decision: The `REORDER_BATCH` and helper docs state that no inversion is reachable under the joined-endpoint driver and why (accepts complete one at a time over the synchronous in-memory link, so a batch never forms); the property reduces to one deterministic case over `early_first_child_dispute_pair` asserting `reordered == 0`, the tripwire for a driver change. The prior ruling keeping the `== 0` assertion (cbc4a0aa, T21) stands.
Home: code.

## T102 (2026-09-02): The `Backend` family is internal, documented for the maintainer, with no mention of the future
Disposes: owner decision 91; streaming-backend-window-1, the inventory open question (fix-amended)
Decision: The `Backend` trait family's docs address the crate's maintainer, state that the boundary is crate-internal, and describe `Local` as the implementation; they say nothing about future implementations or publication. No trait-shape pass is scheduled: the boundary is always internal, and the next implementation shapes the trait when it arrives.
Home: code (the backend module doc).
Reasoning: Finch: "It is always going to be internal, but there will be a second implementation when persistent storage arrives. Do not mention the future in the docs."

## T103 (2026-09-02): Window-module prose placement, all three
Disposes: owner decision 92; streaming-backend-window-24, streaming-backend-window-10, streaming-backend-window-27 (fix)
Decision: The flushed-question derivation moves onto `queues::local_questions`; `Local::assemble`'s run buffer is priced in prose; `DEFAULT_SYNC_MEMORY_BUDGET` takes the proposed first sentence.
Home: code.

## T104 (2026-09-02): The reconciliation page cites `STREAM_COUNT` by name
Disposes: owner decision 93; fresh-eyes-1, session-bookmark-35 (fix)
Decision: The stream-bound derivation on the public reconciliation page states the structure (data streams per direction beside one control stream) and cites `STREAM_COUNT` by name, with no literal counts.
Home: code.

## T105 (2026-09-02): Ruling B2 is reopened; the version atom is hand-parsed
Disposes: owner decision 62; remote-codec-24 (fix); supersedes ruling B2's decline of enforcement
Decision: Every supplied record's version atom is parsed with the head primitive the greeting already uses, deleting the per-leaf 4 KiB stack zero and `Vec` allocation and judging the head spelling (the canonicality gap closes). The commit names ruling B2 as reopened on the cost B2 did not have before it and names the two pins it flips. Measured with the decode allocation meters at the parent commit before crediting.
Home: code.

## T106 (2026-09-02): The decode channel's liveness claim is constructed, then the comments state amortization
Disposes: owner decision 63; remote-adapter-streams-6, remote-proxy-27 (fix-amended)
Decision: A committed sub-FAN run constructs the case first. `FAN` is required for liveness in the assembler; it is not believed required at the decode channel, where its value is amortization. If the construction completes, both comments are rewritten to state that: the assembler's requirement and the channel's amortization rationale, distinctly. The pull-based reader stays a measure-first experiment gated on a persistent backend; the walk-side allocation meter (decision 68) is the instrument for remote-proxy-27's per-reply cost.
Home: code.
Reasoning: Finch: "FAN is required for liveness in the assembler; I don't think it's required here, but amortizing it helps efficiency."

## T107 (2026-09-02): The slice-recursion redesign of `act` lands now
Disposes: owner decision 64; tree-core-27 (the redesign tier; T39 landed the single sort), tree-core-29 (fix)
Decision: `act` recurses on slices of the one sorted action list rather than re-materializing per height, and the no-op commit keeps its root spine and memos (tree-core-29). Measured with `benches/in_memory.rs` and the per-commit allocation count at the parent before crediting; the send_all-under-lock figure is taken once after T34 and this land, and recorded in the commit.
Home: code.

## T108 (2026-09-02): `multi_peer` fuses its implied checks; `sanity`'s subsumed test goes
Disposes: owner decision 65; tests-lifecycle-9, tests-lifecycle-29, suite-economics-5 (fix)
Decision: The two implied checks fuse into the canonical-map test with distinct messages; `sanity.rs`'s panic-freedom property, strictly subsumed by `multi_peer`, is deleted; the module doc states the one-test-per-invariant policy and the sampling-breadth trade. The `window_corners` and `window_census` halves of suite-economics-5 ride with the P1 conformance lane's entries.
Home: code.

## T109 (2026-09-02): The capacity test's width is measured once at four parents
Disposes: owner decision 67; suite-economics-7 (fix-amended)
Decision: After the P1 harness lane makes the stall verdict honest (streaming-tests-11), one measured run at four parents decides: the width shrinks if the verdict holds, or the test's doc gains the sentence stating why 32 parents are needed. One run, load reported.
Home: code.

## T110 (2026-09-02): No new allocation meters; the message residency slack is fixed
Disposes: owner decision 68; materialized-30 (model: no walk-side meter), api-core-10 (fix), the commit-path meter half of tree-core-27 (model)
Decision: No walk-side, residency, or commit-path allocation meter is added now. The message residency slack itself (each stored message retaining its encoding vector's power-of-two spare capacity plus a shared header) is fixed per api-core-10's resolution. Owner decision 68's acceptance instrument for remote-proxy-27 and streaming-backend-window-9 is therefore the existing benches, measured at the parent.
Home: this file for the declined meters; code for the fix.
Reasoning: Finch: "None now. I would like to fix the message residency slack, though."

## T111 (2026-09-02): Typed-tree trades are constructed and measured; the leaf preimage's heap vector goes now
Disposes: owner decision 69; tree-typed-6 (fix: the leaf half now, the branch half measured), tree-typed-23 (fix-amended: measure first), the tree-typed open question on `Leaf::into_node` (measure first)
Decision: The leaf hash preimage's heap `Vec` is deleted outright. The inline-prefix `ArrayVec`, the branch preimage buffer, and the supply path's per-leaf `Leaf::into_node` allocation are each constructed and measured with `benches/in_memory.rs`, `benches/branch_hash.rs`, `benches/gossip_fixed.rs`, and the node census before landing; each lands only on a measured improvement with its committed baseline updated, and a regression is a finding.
Home: code.

## T112 (2026-09-02): The geometry search takes a checked hint
Disposes: owner decision 66; suite-economics-2, suite-economics-3, tree-core-24 (fix)
Decision: `HINT_ATTEMPT`, measured once by running the search under SHA3-256, is tried first: the fixture hashes only that window's leaves and evaluates the geometry predicate, falling back to the full scan (with its loud exhaustion failure) if the hint fails. A committed test pins that the hint satisfies the predicate and that a wrong hint falls through to the scan. The prose number in `ATTEMPTS`'s doc is deleted; the guard's description matches its spelling.
Home: code.

## T113 (2026-09-02): api-core-10's acceptance is a direct assertion, not a meter
Disposes: api-core-10 (fix-amended); amends T110
Decision: The encoding is stored at exact length with no header allocation (`into_boxed_slice` or equivalent). Acceptance: a committed test asserts every stored message's serialized cache has zero spare capacity, and `benches/in_memory.rs` `batch_insert` is neutral or better at the parent. No standing residency meter.
Home: code.

## T114 (2026-09-02): Bench economics, all three
Disposes: owner decision 70; the benches-envelope open questions (fix)
Decision: The gossip benches report `Throughput::Bytes` from `Gossiped.stats` beside elements; `just all` runs a short smoke of every bench binary; the comment at `CAPACITY` states why `Wire` and `DelayedWire` use different pipe sizes.
Home: code.

## T115 (2026-09-02): `tests/common` and `benches/support` become a path dev-dependency crate
Disposes: owner decision 71; suite-economics-8, tests-common-8, tests-disruption-handshake-28, benches-envelope-22 (fix-amended)
Decision: The shared test harness and bench support move into a workspace crate consumed as a path dev-dependency, so they compile once; the window family and the five single-test binaries fold into fewer binaries with their committed seeds re-homed (T59); the schedule-engine suites keep their binaries. `cargo build --tests --timings` is measured once at the parent and once after, and the two figures are recorded in the commit, as the measurement of a change already decided rather than its gate.
Home: code.

## T116 (2026-09-02): The schedule-executor properties get an explicit, larger case budget
Disposes: owner decision 72; the tests-lifecycle partition's case-budget item (fix-amended)
Decision: The schedule-executor binaries set a `ProptestConfig` explicitly, with a case count above proptest's default; the lane measures the per-binary wall time at the default and at the proposed count on a quiet machine, and the count lands with that measurement and its reason in the module doc. The number is proposed by the lane and approved by Finch before it is committed.
Home: code.
Reasoning: Finch: "And probably make it more cases!"

## T117 (2026-09-02): Height-indexed chain traits for `define_peer!`, one experiment
Disposes: owner decision 96; the mirror-common open question (fix-amended: an experiment)
Decision: One branch derives the 16-deep and 15-deep bound chains from height-indexed chain traits; it is abandoned only on measured compile-time or diagnostic-quality evidence, reported with the figures, never on anticipated complexity.
Home: code (or this file, if abandoned with its evidence).

## T118 (2026-09-02): One instrumentation shape: the proxy's `Progress`
Disposes: owner decision 97; the remote-proxy and materialized open questions (fix)
Decision: The walk adopts the proxy's zero-sized `Progress` passed by value; its `cfg(test)` parameters and `trace_id` field are removed.
Home: code.

## T119 (2026-09-02): The greeting-derived premises become a named bundle after the hand-off collapses
Disposes: owner decision 98; remote-proxy-4, remote-adapter-streams-4, remote-adapter-tests-2, remote-proxy-24 (fix)
Decision: After remote-proxy-7 collapses the proxy's handshake-to-session hand-off, the set length, version bytes, and digest travel as one named struct with documented fields, replacing the loose parameters and the forty test re-spellings; the inline "one premise per argument" rationale is superseded by the field list.
Home: code.

## T120 (2026-09-02): `MAX_QUERY_CHILDREN` is defined from `FAN`
Disposes: owner decision 99; remote-codec-3 (the fan half) (fix)
Decision: One constant is defined from the other, with a sentence stating why the wire keeps its own name for the radix fan.
Home: code.

## T121 (2026-09-02): The `streaming` path level is flattened into `mirror`
Disposes: owner decision 100; module-graph-9 (fix-amended)
Decision: `tree::mirror::streaming::*` moves up into `tree::mirror`; the level and its name go, and `mirror.rs`'s module doc describes the mechanism. A mechanical move, landed as its own commit so the diff is a rename.
Home: code.

## T122 (2026-09-02): The focused conformance checks loosen their bounds; the eightfold shape stays
Disposes: owner decision 101; conformance-7 (fix-amended)
Decision: The focused checks take the looser bounds they need (non-breaking); the eight public signatures keep their shape, and one comment at the module states why (the reason f5039abb recorded in its message).
Home: code.

## T123 (2026-09-02): Two wall times recorded; `sha3` at opt-level 2 in the dev profile
Disposes: owner decision 102; the tests-observation overlap-sweep item and remote-capture-atlas-28 (fix)
Decision: The total overlap sweep and the bounded corpus manifest each get one measured wall time stated as a band in the test's module doc, load reported in the commit. `Cargo.toml` sets `[profile.dev.package.sha3] opt-level = 2`, which serves the corpus test and the geometry fixture.
Home: code.

## T124 (2026-09-02): Tree-core mediums land as stated
Disposes: tree-core-1, tree-core-11, tree-core-16, tree-core-23, tree-core-22, tree-core-34 (fix)
Decision: Each lands per its entry's Resolution and Acceptance. Ordering: tree-core-11 after T38 (the leaf level it documents changes there); tree-core-16 with T34 (the evacuation rewrites the same comments). tree-core-23's two out-of-partition sites ride with it.
Home: code.

## T125 (2026-09-02): Tree-typed and crate-root mediums land as stated
Disposes: tree-typed-5, prose-hygiene-1, tree-typed-19, api-core-25, api-core-22, api-core-33, module-graph-1 (fix); inventory-9 (dup of tree-typed-5)
Decision: Each lands per its entry. Ordering: api-core-25's reunion tests before api-core-22's `Extant` redesign; api-core-33's shared `Channel` with T74's observer registration change; prose-hygiene-1's `future_size` half is T7's.
Home: code.

## T126 (2026-09-02): Remote adapter, codec, proxy, and capture mediums land as stated
Disposes: remote-adapter-streams-3, remote-adapter-streams-5, remote-adapter-tests-10, remote-adapter-tests-20, remote-adapter-tests-17, remote-adapter-tests-15, remote-adapter-tests-22, remote-codec-27, remote-proxy-1, remote-proxy-tests-25, remote-proxy-tests-26, remote-proxy-tests-7, remote-capture-atlas-2 (fix); inventory-5 (dup of remote-codec-27)
Decision: Each lands per its entry. Ordering: remote-adapter-streams-3 before remote-adapter-tests-15, so the recognizer-differential family pins one loop at two entries; remote-proxy-1 rides with T63's error pass; remote-codec-27 moves no wire byte (the greeting snapshot is the check).
Home: code.

## T127 (2026-09-02): The `LocalSession` builder lands with the three-way outcome in the P1 harness lane
Disposes: streaming-tests-3 (fix); amends the P1 harness-crate brief
Decision: The P1 harness-crate lane builds the `LocalSession` builder and its `Outcome` type together with streaming-tests-11's three-way stall verdict, consolidating the nine construction sites while the probes are rewritten. The brief's "do not build it here" note is withdrawn.
Home: code; `triage/briefs/p1-harness-crate.md`.

## T128 (2026-09-02): Materialized, window, conformance, streaming-test, and link mediums land as stated
Disposes: materialized-18, materialized-20, materialized-13, materialized-34, streaming-backend-window-19, deps-2, streaming-backend-window-37, conformance-9, streaming-tests-18, streaming-tests-19, streaming-tests-23, streaming-tests-26, streaming-tests-27, streaming-tests-5, async-hazards-2, link-26 (fix)
Decision: Each lands per its entry. Ordering: materialized-18 with T63's error pass; deps-2's full form (the production receiver wrapper) so `tokio-stream` leaves the manifest, which T86's cycle work depends on; materialized-20's out-of-partition sites ride with it (streaming-tests-8, prose-hygiene-4 agree on keeping `B5`); streaming-tests-19's terminal-phase failure, if any, is reported as a finding.
Home: code.

## T129 (2026-09-02): Scaffolding, bench, suite-economics, and session-bookmark mediums land as stated
Disposes: testing-infra-16, testing-infra-21, testing-infra-22, testing-infra-2, testing-infra-20, benches-envelope-10, benches-envelope-8, benches-envelope-13, suite-economics-4, tests-resource-link-window-1, session-bookmark-10, session-bookmark-22 (fix)
Decision: Each lands per its entry. benches-envelope-13's visit counter is admitted as a pin on the pruning claim (a correctness-shaped property), distinct from the allocation meters T110 declined. session-bookmark-22 lands with T62's trait edit. testing-infra-20 deletes the in-crate fuse; `tests/common/fault.rs` follows only once its severed-direction capability has a home in `IoPlan`, reported by the lane.
Home: code.

## T130 (2026-09-02): Integration-test mediums, first half, land as stated
Disposes: tests-resource-link-window-16, tests-bookmark-21 (option (a)), tests-bookmark-6, tests-disruption-handshake-14, tests-disruption-handshake-15, tests-disruption-handshake-22, tests-lifecycle-21, tests-observation-17, tests-observation-25, tests-common-25, tests-wire-format-27, prose-hygiene-3, tests-observation-15 (the same defect), tests-common-11, tests-disruption-handshake-4, tests-disruption-handshake-9, tests-lifecycle-19 (fix)
Decision: Each lands per its entry. tests-common-25's new fault-plan fields are drawn after `windows` so every committed disruption seed keeps regenerating its prefix; tests-disruption-handshake-4's separate question (a redaction step in the child plan) is a lane-reported item, not ruled here.
Home: code.

## T131 (2026-09-02): Integration-test mediums, second half, land as stated; `async_wire.rs` is deleted
Disposes: tests-lifecycle-22, tests-observation-18, tests-resource-link-window-29, tests-wire-format-16, tests-wire-format-3, tests-wire-format-6, tests-bookmark-3, tests-disruption-handshake-31, tests-lifecycle-3, tests-observation-2, tests-observation-32, tests-resource-link-window-22, tests-resource-link-window-3 (fix)
Decision: Each lands per its entry. The harness consolidation entries (tests-bookmark-3, tests-lifecycle-3, tests-observation-2, tests-observation-32, tests-resource-link-window-22, tests-disruption-handshake-31) land in the shared harness crate of T115, the harness lane before the suite lanes. `tests/async_wire.rs` is deleted with its `String` payload riding `pairwise`'s union property and its four seeds re-homed into `pairwise.txt` in the same commit (T59).
Home: code.

## T132 (2026-09-02): `api-core-18` lands as stated; every remaining low and nit roster is approved
Disposes: api-core-18 (fix); every open low and nit row in P4, P5, P6, and P7 (fix)
Decision: Each open low and nit entry lands per its own Resolution and Acceptance inside its pattern sweep (P4), module lane (P5), the error pass (the two P6 nits), or the performance lane (P7). Finch reviews each lane's diff; a lane agent that must deviate from a stated resolution stops and reports, and the entry stays open until ruled.
Home: code.

## T133 (2026-09-02): Lanes land through pull requests under a separate Claude identity
Disposes: the landing workflow for every `fix` row
Decision: Every lane lands as a GitHub pull request opened by a separate Claude identity (a GitHub App or machine user Finch provisions; `GITHUB-APP-SETUP.md`), never under Finch's account. The pull request body carries the CAVEAT LECTOR header, the goal, the rulings, and an acceptance table; a single `COMMENT` review annotates every changed region, nits included, from the implementing agent's own annotation file, in its own words. Fresh-eyes reviewers read each diff cold before the pull request opens, and their findings return to the implementer as repairs; their reports are never the annotations. Pull requests are capped at one logical unit of review and stacked where the briefs name a dependency; a pull request with an open stop stays a draft under the `triage:stop` label. Merges are Finch's, by rebase-merge, and the ledger's `sha` column is written only from merged commits whose acceptance the coordinator ran. No lane pushes or opens a pull request until the identity exists. The procedure is `WORKFLOW.md`.
Home: `triage/WORKFLOW.md`, `triage/GITHUB-APP-SETUP.md`.
