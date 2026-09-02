# Documentation and comments

This document collects every finding of primary class *documentation* from the holistic review of `before` and `suanpan` at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764. It answers one question: where is the prose inaccurate, at the wrong altitude, too long, too short, unanchored, or in the default dialect? Three surfaces are kept apart wherever a finding touches them: public rustdoc (what a library user reads), maintainer prose (module docs on private items, `//` comments, manifest and workflow comments, agent-facing guideposts), and test doc comments (which the crate's own `AGENTS.md` holds to "their incorrectness is a bug in the test"). Ids are `<partition or sweep key>-<n>`; the full record of each finding, with the lens reports it was merged from and the refutation and history passes that settled it, lives in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md` beside this file. The severity scale is the finalizers': **high** for a breach of a hard rule in the root or crate `AGENTS.md` (prose naming code that no longer exists, a design document cited from code) or a public contract false as written; **medium** for prose a maintainer or user would act on and be wrong (a mechanism described backwards, a test doc that does not state what the test asserts, a definition the code contradicts); **low** for inaccurate, stale, or misplaced prose with a bounded blast radius; **nit** for register, typography, a stray count, or a coinage. Provenance: **demonstrated** (a constructed test ran), **executed** (a run settled it), **verified** (mechanically checked or re-derived: grep, git, a hand computation), **assessed** (read).

Counts: 399 entries, of which 6 high, 29 medium, 197 low, and 167 nit. Under the review's scale rule, every high, medium, and low entry appears below in the finalizers' template, verbatim; nits appear per module in a compact table (id, anchor, claim, resolution in a few words) that points at the evidence file for the full record. Nothing is dropped and no anchor, evidence, verdict, severity, or provenance has been changed. Where a finding is reported by more than one partition or sweep, every id is kept and the duplicates are named in a *Cross-references* paragraph at the end of the module section, so one edit can be checked against every report that asked for it. Anything I add beyond the finalizers' text is marked *Synthesis note*.

Two provenance notes on this assembly. First, the working tree's HEAD at assembly time was 7440d1a3, two commits past the briefed 9e5784fb; both are agent-note commits and `git diff --stat 9e5784fb HEAD -- crates/before crates/suanpan crates/surface-scan crates/before-fuelscape justfile tools .cargo .github` is empty, so the sixteen anchors I re-read against the tree (iter.rs:9-10, error.rs:1 and :5, lib.rs:277, AGENTS.md:5-7, tests/meter.rs:7-8, build.rs:1-22, floors.rs:5-7, party/forks.rs:80, clock/tests.rs:1258, tier2.rs:10-11, sweep.rs:56-57, extract.rs:8, idbits.rs:35, shape.rs:83, causally/query.rs:26-27) are the briefed commit's bytes; every one matched its entry's excerpt. Second, no witness construction in `witness/results.md` targets a documentation finding: the 117 witness ids and the 399 documentation ids are disjoint, and none of the 399 ids is mentioned anywhere in that file, so every documentation entry's provenance is what its own Provenance line says.

## Highest-value items

1. The resource-envelope suite's file header still says the implementation is "far from" the linear contract that `lib.rs` declares a hard guarantee, and a dozen row docs describe recursion frames, quadratic path sums, and a transcoding decoder that the pinned constants beside them refute; a re-pinner reading them is pre-authorized to accept the regression the rows exist to refuse (envelopes-a-1; the same header is prose-hygiene-1).
2. `meter/tier2.rs` describes the stored skyline coding as a candidate and the deleted construction-language coding as "today's", and its `Tier2Size` formula is wrong against `encoded_bits`; the same pre-flag-day framing runs through `testing/compactness.rs` (testing-diff-gen-17) and the `Packed` and `cliff_comb` docs in `meter.rs` (meter-core-3, meter-core-4), so a maintainer reading the meter surface alone is told a decision made in July is still pending (meter-registry-tier2-14).
3. The retired `before::implementation` essay is still cited from the crate guidepost, an example's reason to exist, and a test doc; eight ids from five partitions and sweeps report the same ghost, and `cargo doc` never visits the example so no lint can see it (benches-examples-18; also api-audit-3, skyline-coding-14, crate-root-1, inventory-3, module-graph-5, paper-fidelity-4, rumors-dependence-6).
4. Six sites in the party kernels name code that does not exist: an `EvNode` type, an `IdLit` trait (the doc block also sits on the wrong item), a removed `compare` operation, an oracle note that was never written, the pre-pruning encoding, and a "recursive form" on a loop (party-3).
5. The surface-roster machinery teaches its naming scheme with `causally::Range::since`, a type the crate no longer has, and carries a pointer to a coverage note that does not exist, a "bare-name scan" that never did, and an emptiness dated to "this tip" (surface-roster-20).
6. `span_shares_the_crossing_folds` documents a limb leg its body does not have and narrates the leg's removal; a test doc that does not state what the test asserts, plus a ghost reference (envelopes-b-27).
7. `Party::join_all`'s public `# Errors` promises the overlapping inputs back, but the balanced counter hands back coalesced unions that equal no input; the laws and the partition's own tests state the true contract, region conservation, and the public prose should too (party-8; the same contract at clock-4, crate-root-18, paper-fidelity-7).
8. `Version::decode`, `Party::decode`, and `Clock::decode` read to end and reject any spare byte as `TrailingBits`, and none of the three says so or carries a `# Errors` section, while `Span::decode`, `Rank::decode`, and `Ranked::decode` carry complete ones; the decode trio are the entries most likely to meet untrusted bytes (fresh-eyes-2; version-core-14, api-audit-8, clock-11).
9. Four public docs promise "exactly `k`" shares from `forks`; `Forks::new` saturates `k + 1`, so `forks(u64::MAX)` yields one fewer, and the only statements of the boundary are a `//` comment and a test file that calls it "the documented behavior" (party-14; api-audit-10, inventory-13).
10. `shape.rs` defines a *plateau* as one maximal constant run of the step function, but the walk yields the canonical coding's leaves, which can be adjacent and equal across a subtree boundary; a renderer trusting the definition draws a boundary the function does not have (crate-root-38).
11. The `causally` docs state the SAT/NP-completeness motivation three ways, once logically inverted ("reduces to SAT, and is therefore NP-complete (non-polynomial)"), and never argue it; the same owner hand-edit commits deleted the `Coverage` exactness sentence and the no-`Eq` rationale that two pointers still cite (span-causally-33; span-causally-34, span-causally-38, fresh-eyes-8, api-audit-9).
12. The asymptotics pins quote rustdoc sentences that no longer exist and tell a failing maintainer to edit a `# Complexity` section that is now an `include_str!` of a fuelscape island authored in `before-fuelscape/src/ops.rs`; the fuzz-fit bands head carries hand-transcribed measurements the generator can never refresh (fuzzfit-bands-2), and `sweep.rs` names as its verdict oracle the `PartialOrd` that is the sweep itself (skyline-sweep-place-masked-33) (testing-diff-gen-22).

## Crate-wide patterns

The 399 entries fall into a small number of recurring patterns. Each paragraph below names the pattern, the rule it breaches, and every id that instantiates it, so a single crate-wide pass can be checked against the whole roster rather than one partition. The prose-hygiene sweep's census findings, which are crate-wide by construction, appear here in full rather than under the module their first anchor happens to sit in.

**Prose from before the flag day.** Commit faf3cd0a (2026-07-25) made the skyline coding the stored form. Several files still describe the previous world as the present: the envelope suite's header and row docs (envelopes-a-1, prose-hygiene-1), the tier-2 sizer and its tests (meter-registry-tier2-14), the compactness probe (testing-diff-gen-17), the `Packed` type and the comb generators' funding arguments (meter-core-3, meter-core-4), the sweep's "stored-form comparison" oracle (skyline-sweep-place-masked-33), the fill walk's "recursion argument" (skyline-fill-grow-7), the door rows' construction-language denominator (envelopes-a-10), and the transcoder's `expect` message (skyline-coding-19). The 205a361da ghost sweep and the d2a9d04e dated-notes excision each covered part of the surface; the remainder is listed here.

**Ghost references to deleted code and retired conventions.** The root and crate `AGENTS.md` forbid prose naming code that no longer exists. Sites: the `implementation` essay (benches-examples-18, api-audit-3, skyline-coding-14, crate-root-1, inventory-3, module-graph-5, paper-fidelity-4, rumors-dependence-6); `EvNode`, `IdLit`, `compare`, an absent oracle note, and a "recursive form" (party-3); `causally::Range::since`, a coverage note, a bare-name scan (surface-roster-20); `store_be` (codec-bits-20); the determinism tripwire (board-ops-render-30); `has_seen`/`happens_before` (clock-20); the bit-packed clock framing (clock-25); a limb leg (envelopes-b-27); `crate::bookmark` (crate-root-10); the "transient fixed-width working form" in the crates.io description (crate-root-2); `h34_decode_never_panics` (fuzz-guests-pins-2, gate-legs-11); the pre-anchor seed path (fuzzfit-bands-20, fuzz-guests-pins-20); "the demonstrations ledger" (fuzzfit-bands-24); the bounded lazy-skip (testing-diff-gen-13); a `skyline_oracle` (envelopes-a-1); the deleted `Coverage` and no-`Eq` decisions two pointers still cite (span-causally-34, span-causally-38, fresh-eyes-8, api-audit-9); a monotonicity argument "the type's docs carry" (span-causally-16); a "depth-guard size prose" (version-core-35); `alice` (version-core-6); "the old" and "retired" (envelopes-b-12, skyline-query-32, prose-hygiene-8); "unchanged" and "replaced" (fuelscape-pipeline-3, testing-oracles-29); the summary-merge sentence (testing-diff-gen-22).

**Public contracts contradicted by the code.** These are the entries a user would act on: `forks` promises exactly `k` (party-14, api-audit-10, inventory-13); `join_all`'s `# Errors` promises inputs back (party-8, clock-4, crate-root-18, paper-fidelity-7); a plateau is "one maximal constant run" (crate-root-38); `Query::into_owned` is `O(1)` (span-causally-37); `PartialEq` "describes a causal ordering" (fresh-eyes-1, crate-root-28, api-audit-4, paper-fidelity-12); `Parse` omits `Ticks` and asserts paper notation for a decimal (crate-root-16); `Overlap` names one of two producers (crate-root-14, api-audit-16, fresh-eyes-13); `# Panics` promises a panic on any non-canonical stream at six internal entries where only unreadable bits panic (skyline-fill-grow-4, skyline-sweep-place-masked-13); `Rank` "has no packed codec" (fuzzfit-strategies-6); the byte-compare rung is `O(1)` (version-core-10, fuzzfit-strategies-3); `PROPTEST_CASES` overrides the sentry (fuzzfit-bands-21); the exact-touch sentence is not exact at zero (suanpan-6); `add_at` takes "any `i128` magnitude" (inventory-14); `MAX_SCALING_EXPONENT` excludes "a real log factor" (meter-adequacy-3); `dangerously_alias`'s second copy "must be dropped without further use" (rumors-dependence-4); the `/` operator spelled on an owned left operand (fresh-eyes-7, version-core-3).

**Missing or non-uniform hazard sections.** `# Errors` covers about half the fallible public entries; the decode trio and every `FromStr`/`TryFrom` lack it (fresh-eyes-2, api-audit-8, version-core-14, clock-11, api-audit-21). `# Panics` sections are incomplete or copied (board-ops-render-17, skyline-sweep-place-masked-9, skyline-coding-35, skyline-fill-grow-5, skyline-fill-grow-20), and `validate_from`'s error list omits the `Io` arm (skyline-coding-31).

**Measured readings and history at declaration sites.** The crate's own convention (ceilings.rs:56-62, tests/meter.rs:245-257) is that readings live in pin commits. Readings and incident narration survive at: board-families-floors-judge-1, board-frame-12, board-ops-render-29 (with prose-hygiene-3), codec-base-text-tree-3, envelopes-a-5, envelopes-a-20, envelopes-b-12, skyline-fill-grow-1, skyline-query-30, skyline-watermark-8, meter-registry-tier2-6, rank-1, rank-24, fuzzfit-bands-2, fuzzfit-bands-13, fuzz-guests-pins-20, skyline-coding-26, suanpan-tests-11, suanpan-tests-22, suanpan-27, testing-oracles-19, suite-economics-5, tests-other-15, gate-legs-13, board-ops-render-3 (a dated rationale without a date), and the "dated owner rationale" prescriptions the excision commit itself retired (board-frame-4, prose-hygiene-6). The `decided:` date fields are the one place dates are code data (prose-hygiene-7; also surface-roster-17 in the scaffolding class).

**Hand-maintained counts and rosters.** "No hand-maintained counts" is breached wherever prose restates an enumerable fact: "all four counters" and the fold-row roster (board-frame-2, board-frame-7), the `version2` slot's shape list (board-families-floors-judge-2), the scan-NA list (board-families-floors-judge-9), "second safety rule" (clock-1), "three law populations" (clock-21), "the 256 vectors" and "five wire types" (codec-base-text-tree-26), "exactly one hole" (crate-root-27), fold.rs's caller list (crate-root-20), cursor and record-site rosters (codec-bits-16, codec-bits-26, codec-base-text-tree-9), idbits and ops rosters (party-2), walk.rs's clients (skyline-coding-34), "exactly two generic faces" (skyline-coding-22), "depth-2" and "depth 2" (skyline-coding-24, span-causally-18), "two" follower slots (skyline-watermark-4), `Directions`' clients and "three accumulators" (skyline-sweep-place-masked-36), "both callers" (skyline-query-22), the 767 members (fuelscape-pipeline-21), ~137 and 20,048 (fuzzfit-bands-26, fuzzfit-strategies-17), 0.56 and ~604 MB (rank-1), 25,000 in a test name (version-core-30), "16 group parties" (meter-registry-tier2-6), "seven families" and "two anchors" (surface-roster-2), suite and importer lists (testing-oracles-1, testing-diff-gen-10), the 197 items (tests-other-15, api-audit-22), 1,948,716 states (suanpan-tests-11), "both cost arguments" and the `&self` list (suanpan-7), the fuzzfit head's 49 keys (prose-hygiene-15, gate-legs-13), "~200 cells" (benches-examples-7), and the validation index's "four instruments" (testing-oracles-29).

**One argument written several times.** A derivation with many homes drifts: the stream-codes limb argument at seven sites (board-families-floors-judge-12), the board root doc re-narrating each submodule (board-frame-5), the overlay `# Panics` paragraph seven times (skyline-sweep-place-masked-9), the debug-assert rationale six times (skyline-sweep-place-masked-34), walk.rs's `# Panics` five times (skyline-coding-35), the reset-versus-subtraction argument three times (skyline-coding-30), the `v + w` absence three times (span-causally-7), the identity-routing paragraph three times (fuzzfit-strategies-3), `M`'s definition eleven times (rank-31), the u64-width refrain at six non-arithmetic sites (codec-bits-6), the `# Testing` paraphrases (skyline-fill-grow-3), the two-genre paragraph (envelopes-b-24), the stratified-arity and totality chains (fuelscape-pipeline-27), the surface-totality explanation at four sites (surface-roster-2), and the bridge's section comment (testing-oracles-6).

**Vocabulary without an anchor.** The vocabulary rule wants every coined term tied to an identifier or defined once by contrast. Undefined across the crate: *door* (crate-root-21, codec-bits-5, rank-13, skyline-coding-1, oracle-laws-19, span-causally-17, testing-diff-gen-24, version-core-17, fresh-eyes-4; counts range from 208 to 279 lines depending on the grep's scope), *seam* and *genre* (codec-bits-5, skyline-query-1, skyline-sweep-place-masked-29, envelopes-a-18, envelopes-b-3, span-causally-22 in a public `# Errors`), *knob* and *luck-proof* (prose-hygiene-11, meter-core-1), *keystone* (clock-24, codec-base-text-tree-25, testing-oracles-12, testing-diff-gen-24, surface-roster-25), *pincer/jaw* (tests-other-12, surface-roster-25), *sentry* (fuzzfit-bands-1, fuzzfit-strategies-15), *deterministic-liveness* (board-families-floors-judge-8), *forest parent* (skyline-fill-grow-22), *GREEN PIN* (envelopes-b-3), `\[derived\]` tags (board-frame-11, suanpan-3), *currency* in three senses (skyline-coding-1, skyline-sweep-place-masked-29, codec-bits-5), *arming* in two (skyline-query-16), *re-arm* and *fill phase* (skyline-watermark-25), *answer-embedded* in two (tests-other-4), and the fuzzfit harness's *rung*, *battery*, *mirror*, *ladder* (fuzzfit-strategies-15). Opaque roster ids: PROG-5/COV-7 (fuzz-guests-pins-2, deps-11, gate-legs-11, module-graph-6, prose-hygiene-4) and `d1_` (surface-roster-15).

### prose-hygiene-11: Unanchored coinages: door (279), knob (107), seam (276), luck-proof (1)
- Where: crates/before/src/meter.rs:24-27 (related: crates/before/src/meter/registry.rs:63, 87-90, 273-281, 565, 585; crates/before/src/codec/bits.rs:264; crates/before/src/codec/tree.rs:25; crates/before/src/party.rs:14, 872-873; crates/before/src/meter.rs:2667-2683; crates/before/src/testing/asymptotics.rs:117; crates/before/src/span/tests.rs:169, 908)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (recounted; searched for italic definitions `*door*`, `*knob*`, `*seam*` in before/src and suanpan/src: none; searched identifiers: `seam_rung`, `seam_wide`, `seam_plunge*`, `seam_stop*`, `SeamPlunge*`, `SeamStop*`, `door_scan_bits`, two `*_doors_*` test names; read the seam-shape docs at meter.rs:2667-2700 and registry.rs:183-196); executed: no
- Verification: confirmed, with one refinement: the `seam_*` and `Seam*` identifiers name shape families, and their docs use "the seam shapes" and "propagate-seam" without ever saying what a seam is, so the identifiers do not anchor the term; "kernel-seam probe" (21 uses) is a second sense with no definition either; history: no-rationale-found
- Owner-gated: yes: with door at 279 lines and seam at 276, the choice between one definition site and a plain-term sweep is the owner's

Four recurring terms have neither a defining identifier nor a
define-once-by-contrast sentence, against the anchoring rule; the style
tables list door, knob, and seam as default-dialect for entry point,
parameter, and boundary.

Evidence:

        24	//! additionally earns a column on the amplification board ([`board`]) only when
        25	//! it is a whole-surface adversary rather than a kernel-seam probe (the
        26	//! criterion, each family's coverage answer, and the luck-proof touch list sit
        27	//! on the registry's [`FamilyId`](crate::meter::registry::FamilyId)).

    registry.rs:
        63	//! Two seams stay pinned by tests instead of types, each a deliberate,
       273	    /// One size knob to one packed shape.

    codec/bits.rs:
       264	    /// The byte decode doors' entry: a door walks its whole input buffer as

    meter.rs:
      2673	/// Every stacked boundary and every dying residue the seam shapes mint holds

Resolution: per term, either define once at first use in the owning module
(registry.rs for knob and both senses of seam; codec.rs or the lib.rs
implementation essay for door) or replace with the plain term where the count
is small (luck-proof: name the touch list's property; seam outside the shape
names: boundary). Acceptance: each surviving term has an italic definition
site the module doc links, or the greps return only the shape-family
identifiers.

**"mint" for constructing a value.** The one word the writing rule bans outright appears in the Quickstart, public module docs, assert messages, and as the identifier `Reign::mint`. Module sites: crate-root-26, api-audit-19, fresh-eyes-4, clock-27, party-5, board-frame-18, board-ops-render-8, envelopes-b-3, meter-core-1, meter-registry-tier2-1, skyline-coding-7, skyline-fill-grow-21 (three senses, one of them watermark.rs's latent-register term), skyline-watermark-1, span-causally-27, oracle-laws-22, fuzz-guests-pins-21, fuelscape-render-1, testing-diff-gen-5, testing-oracles-29, suanpan-tests-17, version-core-17. The counts differ by scope (82 lines over every in-scope surface; 56 to 58 over `crates/before/src` alone), which is why the census entry below is the one to work from.

### prose-hygiene-5: "mint" for constructing a value at 82 sites, including the identifier Reign::mint
- Where: crates/before/src/version/skyline/query/web.rs:190-190 (related: web.rs:300, 311, 330; crates/before/src/lib.rs:49 and its derived crates/before/README.md:53; crates/before/src/party.rs:15, 872; crates/before/src/version/skyline.rs:7; crates/before/src/meter.rs:18, 1383, 2673; crates/before/src/meter/board/family.rs:1123; the full list of 82 lines is reproducible with `grep -rn -i -E '\bmint(ed|s|ing)?\b'` over the in-scope surfaces)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (recounted: 82 lines, 32 in non-test rustdoc; read the sites quoted); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The writing-style rule is unconditional ("Never write *mint* for constructing
a value — in code, prose, or comments"); the word appears in the crate's
front-page example, in public module docs, in assert messages, and as a
private constructor name.

Evidence:

       190	    fn mint(sign: Sign, offset: &Base, epoch: u32) -> Reign {

    lib.rs:
        49	//! // New participants fork off a live clock, never mint themselves.

    party.rs:
       872	/// Mints identity exactly as the `u8` literal door does — a test and

    skyline.rs:
         7	//! module's cursor vocabulary mints the term). Topology plus absolute leaf

    family.rs:
      1123	        "the disjoint-mount adapter must mint a disjoint pair"

Resolution: rename `Reign::mint` to `Reign::new` (private; no API movement)
and replace every prose, comment and message use with construct, create,
build, or the specific operation (fork, split, emit, allocate); rerun `just
readme` so README.md:53 follows lib.rs:49. Acceptance: the grep above returns
nothing across the in-scope surfaces.

**Moralized code and register transplants.** "honest", "genuine", "real", "earns", "buys", "backstop", "launder", "mandate", "load-bearing" stand where a property or mechanism is meant. Sites: board-frame-18, board-ops-render-8, codec-bits-3, codec-base-text-tree-25, envelopes-a-18, envelopes-b-3, rank-13, skyline-fill-grow-14, skyline-query-12, skyline-sweep-place-masked-29, skyline-watermark-1, meter-core-1, meter-registry-tier2-1, oracle-laws-22, fuzz-guests-pins-17, fuzzfit-bands-1, fuzzfit-strategies-15, fuelscape-render-1, surface-roster-25, testing-oracles-12, testing-diff-gen-5, tests-other-8, suanpan-tests-17, suanpan-19, party-5, version-core-17, clock-24. Several partitions note that "honest" is also the owner's own idiom in recorded rulings, so the sweep wants an owner-confirmed word list (see Open questions).

### prose-hygiene-10: Moralized code: "honest" (234) and "genuine" (65) in place of the property that holds
- Where: crates/before/src/meter/board/floors.rs:5-5 (related: per-file counts of `honest`: tests/meter.rs 29, board/ceilings.rs 25, board/floors.rs 18, board/cell.rs 15, board/tests.rs 13, fuzzfit bands.rs 10, party/tests.rs 8, board/ops.rs 7, board/family.rs 7, board.rs 6; crates/before/tests/amp_board_smoke.rs:231-232; crates/before/src/meter/board/currency.rs:140; crates/before/src/testing/compactness.rs:43; justfile:837; .cargo/mutants.toml:93)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (recounted with `\bhonest(ly|y)?\b` and `\bgenuine(ly)?\b`; per-file tallies recomputed; sites quoted were read); executed: no
- Verification: confirmed (my count for the honest family is 234 because it includes "honesty"; the sweep's 213 excluded it); history: no-rationale-found
- Owner-gated: no

"Honest floor", "honest improvement", "honest reading", "honestly refuses",
"genuinely quadratic" stand where a property is meant (derived from
irreducible work; attributed; non-vacuous; a class regression), and one test
binds a variable `honest`.

Evidence:

         5	//! A floor states the least a watching counter can honestly read, derived from

    currency.rs:
       140	/// its honest floor is zero, and a zero floor asserts nothing.

    amp_board_smoke.rs:
       231	    let honest = in_process_spawn(1, &heap)(SMOKE_SCALE).expect("in-process capture succeeds");

    justfile:
       837	# honest tree and fails on any unexpected red OR unexpected green.

    mutants.toml:
        93	    # carries the argument), and no honest meter band can price a zero

Resolution: sweep both words: "honest floor" to "the floor" (its derivation
is stated beside it), "honest improvement" to "an attributed improvement",
"honest reading/tree" to "unmutated" or "within the model", "genuinely
quadratic" to "quadratic"; rename the `honest` binding to `captures`.
Acceptance: the two greps return nothing outside quoted test names.

**Relative-time words and history narration.** "today", "currently", "none at this tip", "historical", "retired", "the old", "once": board-families-floors-judge-8, board-frame-18, envelopes-a-1, envelopes-a-20, envelopes-b-12, meter-core-4, rank-24, skyline-query-32, skyline-coding-26, fuzzfit-strategies-17, fuelscape-render-9, surface-roster-20, tests-other-15, testing-diff-gen-5, testing-diff-gen-17, suanpan-tests-22, suanpan-27, meter-registry-tier2-14.

### prose-hygiene-9: "today"/"currently"/"none at this tip" as relative-time naming of the present implementation
- Where: crates/before/src/meter/board/floors.rs:173-177 (related: floors.rs:182, 220, 233, 265, 291; crates/before/src/meter/board/currency.rs:139; crates/before/fuzzfit/harness/src/strategies.rs:117; crates/before-fuelscape/src/compact.rs:42; crates/before/src/testing/fuelscape_islands.rs:19; crates/before/surfacecheck/src/check.rs:37; crates/before/tests/foreign_reexport.rs:18; crates/before/AGENTS.md:34)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for the terms outside tier2/compactness: 24 lines; each listed site read in context); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Liveness-floor strings, a policy doc, a guard doc, the guidepost, and three
empty-roster notes mark the present with "today"/"currently". In each, the
sentence already says what holds and what change would move it, so the word
is either redundant or a hand-maintained state note ("currently empty",
"none at this tip") that rots when the literal changes.

Evidence:

       173	pub(super) const WHY_LIMB_RANK_ENCODE: &str =
       174	    "deterministic-liveness: the encoder extracts and biases the integral part through \
       175	     one arithmetic pass over the numerator today, one op per 64 numerator bits; a \
       176	     pure bit-walk emission (riding the bias as a carry) would lower this floor \
       177	     deliberately";

    currency.rs:
       139	/// NA on every cell today: the target is walks that never grow the stack, so

    fuelscape_islands.rs:
        19	/// Currently empty: every measured operation's island reaches the

    check.rs:
        37	/// Per-item exceptions: none at this tip. An entry here is a deliberate,

    foreign_reexport.rs:
        18	//! library source (today: none), so adding one is a reviewable diff

    AGENTS.md:
        34	  heap before a deep input can overflow — today those are only test

Resolution: delete "today" in the six floors.rs strings, currency.rs:139,
strategies.rs:117, compact.rs:42 and AGENTS.md:34; in the three empty-roster
docs keep only the sentence stating what an entry means and let the empty
literal speak. Acceptance: `grep -rn -i -E '\btoday\b|currently empty|none at this tip'`
over the listed files returns nothing.

### prose-hygiene-8: Historical narration at declaration sites ("retired", "the old", "before this binding existed")
- Where: crates/before/src/testing/validation_index.rs:101-102 (related: crates/before/tests/meter.rs:2744-2746, 9880-9887; crates/before/src/version/skyline/query/tests.rs:2433-2435, 2446-2447; crates/before/src/meter/board/tests.rs:1009-1010; crates/suanpan/src/claims.rs:12-14)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read each site in a ten-line window; `git log -S` on each phrase); executed: no
- Verification: confirmed; history: no-rationale-found (introduced across 2026-07-24 to 2026-07-29: 8b1610d46, 016b91c4a, 669cf3103, a4cc1cf35, e2f4e2a5e, 9e7b7ce33; none records a reason to describe the present by its predecessor)
- Owner-gated: no

Several docs describe committed known-bad kernels or current checks by what
they replaced rather than by what they are; the suanpan bullet carries a
review-incident count as rationale.

Evidence:

       101	//! committed known-bad kernels (schoolbook converters, sequential-reduce
       102	//! folds, retired quadratic walks) held red beside the green pins. What

    tests/meter.rs:
      2745	    /// tightened record that retired the frozen-width-per-tooth
      2746	    /// quadratic baseline.
      9880	    /// resolution and strictly under the floor-first two-check shape it
      9881	    /// replaces; against the old *first check alone* the earlier bail

    query/tests.rs:
      2446	    /// The retired per-digit charge: one `parked`-wide product per
      2447	    /// balanced digit of the mass.

    claims.rs:
        12	//!   row must be named by some claim — the table was twice found wrong
        13	//!   in review before this binding existed, so it is held to the roster

Resolution: name each known-bad kernel by mechanism ("the per-digit
schoolbook charge", "the frozen-width-per-tooth kernel", "quadratic
re-walks") and describe each current check without its predecessor; delete
"the table was twice found wrong in review before this binding existed" (the
bullet already states the binding's purpose). Acceptance: `grep -rn -E
'\bretired\b|\bthe old\b|\bit replaces\b|before this binding'` over the
listed files returns nothing.

### prose-hygiene-6: Prose still describes owner rationales and re-pin annotations as dated after the dated-notes excision
- Where: crates/before/src/meter/board.rs:170-172 (related: crates/before/src/meter/board/ceilings.rs:233-235; crates/before/fuzzfit/harness/src/bands.rs:5-8, 325-326, 924-925; crates/before/fuzzfit/harness/src/bin/calibrate.rs:373-374, 384-385)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the sites; `grep -rn -E '20[0-9]{2}-[0-9]{2}-[0-9]{2}'` over crates/before/src/meter/board/ finds no date; read d2a9d04e's message and its ceilings.rs hunks, which replace the dated rationales with undated "(owner-ratified: ...)" text); executed: no
- Verification: confirmed (new finding, not in the sweep's report); history: no-rationale-found (d2a9d04e rewrote the declaring constants and collapsed bands.rs's dated re-pin ledger but left these sentences)
- Owner-gated: no

The board module doc and the ceilings header say each declared model carries
"a dated owner rationale committed at the declaring constant"; the declaring
constants read "(owner-ratified: ...)" with no date, as the excision commit
intended. The bands.rs re-pin instruction, and the calibrate.rs template that
regenerates two copies of it, tell the re-pinner to "commit with a dated
movement annotation", the convention the same commit retired in favour of
"movement between pins belongs to the re-pinning commit".

Evidence:

       170	//! Some cells are judged against a **declared model** — a ratified cost law
       171	//! derived at the cell, with a dated owner rationale committed at the declaring
       172	//! constant — in place of one global ceiling, because the global form is

    ceilings.rs:
       119	/// model. The ceiling is the worst honest reading ×1.25, rounded up
       120	/// (owner-ratified: the family-stated ceilings' margin convention; the
       121	/// reading lives in the pin commit), so a kernel that re-reads digit state
       233	// Some cells carry a *declared model* in place of one global ceiling: a
       234	// ratified cost law, derived and priced at the cell with a dated owner
       235	// rationale, that the readings must match — the global ceiling would otherwise

    bands.rs:
         7	//! `just fuzzfit-calibrate`, review the diff like a snapshot, and commit
         8	//! with a dated movement annotation.

    calibrate.rs:
       373	/// Generated by `just fuzzfit-calibrate` — review the diff like a
       374	/// snapshot, commit with a dated movement annotation.

Resolution: in board.rs:171 and ceilings.rs:234 write "with the owner
ratification stated at the declaring constant"; in calibrate.rs:373-374 and
384-385 (the source of bands.rs:325-326 and 924-925) and in bands.rs:7-8
write "commit with the movement and its attribution in the commit message",
then run `just fuzzfit-calibrate` so the generated copies follow.
Acceptance: `grep -rn -w dated crates/before/src/meter/board crates/before/fuzzfit`
returns nothing.

**Em-dashes in `//` comments.** The owner's register rule reserves true em-dashes for rendered prose; 374 `//` lines across 76 files under `crates/before/src` carry one, and every partition that counted its own sites concluded that a partition-local fix would leave the crate inconsistent. Sites by partition: board-ops-render-5 (20), codec-bits-4 (13), rank-15 (19), skyline-coding-5 (24 plus one assert string), skyline-fill-grow-10 (62), skyline-query-4 (41 plus one assert string), skyline-sweep-place-masked-16 (25), skyline-watermark-10 (8, with a mutants-roster re-pin cascade), span-causally-2 (15), party-5 (24), version-core-17 (35), meter-core-1 (7), fuzz-guests-pins-7 (6), fuelscape-render-25 (22 plus three literal `\u2014` escapes), testing-diff-gen-5, tests-other-8, suanpan-13 (including panic strings), envelopes-a-18 and envelopes-b-3 in passing. The prose-hygiene sweep proposes a `tools/` lint leg for U+2014 outside rustdoc and Markdown (its open question 7); the decision is the owner's (see Open questions).

**Parameter names that drift between signature and prose.** `n` in prose against `k` in signatures on `ticks` and `forks`, `rhs` against `other` on `Rank`, `version` against `other` on `Version`: clock-2, party-7, rank-6, api-audit-18, fresh-eyes-10, span-causally-5 (`after(p)` for a parameter named `s`).

**Residual dialect tells and hand counts from the census.** Two prose-hygiene nits round out the census; their full records are in `evidence/sweeps/prose-hygiene.md`.

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| prose-hygiene-15 | `crates/before/fuzzfit/harness/src/bands.rs:76-77` | Hand-maintained counts in doc comments | bands.rs: state the structure (one key per kernel plus one per | `evidence/sweeps/prose-hygiene.md` |
| prose-hygiene-16 | `crates/before/src/meter/board/floors.rs:136-136` | Residual dialect tells: load-bearing, earns, backstop, surface-as-verb, flavour, story, dial | replace per the style tables in one sweep: load-bearing to "the | `evidence/sweeps/prose-hygiene.md` |

## Crate root and public types: crate root (lib.rs, error.rs, iter.rs, Cargo.toml, build.rs, AGENTS.md, README)

28 findings (0 high, 2 medium, 13 low, 13 nit). Full records: `evidence/partitions/crate-root.md`, `evidence/sweeps/api-audit.md`, `evidence/sweeps/deps.md`, `evidence/sweeps/fresh-eyes.md`, `evidence/sweeps/inventory.md`, `evidence/sweeps/module-graph.md`, `evidence/sweeps/paper-fidelity.md`, `evidence/sweeps/prose-hygiene.md`, `evidence/sweeps/rumors-dependence.md`.

### api-audit-3: The crate guidepost names an `implementation` module and a "Law of Disjointness" that no longer exist
- Where: crates/before/AGENTS.md:5-7 (related: crates/before/src/lib.rs:218-256)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn 'mod implementation' crates/before/src`: no hits; `grep 'Law of Disjointness' crates/before/src/lib.rs`: no hits; `git show --stat 22cdfbe1` lists `crates/before/src/implementation.rs | 270 -------------` and its lib.rs hunk removes `pub mod implementation;`; `git show a431eaf1d` removes the line `//! Interval tree clocks are correct only under the Law of Disjointness: no` and adds the Causal Singularity and Identity Linearity rules); executed: no
- Verification: confirmed; history: deliberate-but-expired: `f204638d` (2026-07-27, "fix ghost path in the crate guidepost") deliberately pointed the guidepost at the then-new public `implementation` module; `a431eaf1d` (2026-08-04) renamed the model section to "Safety rules" and `22cdfbe1` (2026-08-17) deleted the module, neither touching AGENTS.md
- Owner-gated: no

The first file a contributor reads sends them to a module that was deleted and
to a section heading that was renamed. Both the root and crate AGENTS.md forbid
references to code that no longer exists.

Evidence:

         5	crate docs for the model (`Party`/`Version`/`Clock`, the Law of
         6	Disjointness) and the public `implementation` module for the design essay,
         7	`version/skyline.rs` for the stored coding and its operation kernels, and

Resolution: point the model reference at the crate docs' "Safety rules" section (Causal Singularity, Identity Linearity), and either delete the `implementation` pointer or name where the design essay now lives (`version/skyline.rs` and `testing/validation_index.rs` are the surviving homes). Acceptance: every module and heading named in crates/before/AGENTS.md resolves to an existing item or heading.

### fresh-eyes-1: Crate docs and README name PartialEq as the causal ordering
- Where: crates/before/src/lib.rs:277-279 (related: crates/before/README.md:281-283 (derived), crates/before/src/version.rs:58-60, crates/before/src/lib.rs:30)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read); executed: no
- Verification: confirmed; history: no-rationale-found (a431eaf1d "Doc editing" introduced the sentence)
- Owner-gated: no

The "Comparing and ordering versions" section says `Version`'s `PartialEq` describes the causal ordering. The ordering is `PartialOrd`; the type table one screen above (lib.rs:30) and the `Version` type docs (version.rs:58) say so correctly, and the derived README repeats the slip.

Evidence:

       277	//! [`Version`]'s [`PartialEq`] describes a causal ordering: `a <= b` tests
       278	//! containment of history, and two versions with no containing order are
       279	//! [`concurrent`](Version::concurrent). Three tools extend it:

        58	/// Comparison is **partial** ([`PartialOrd`], not [`Ord`]): two distinct
        59	/// versions can be [`concurrent`](Version::concurrent), and then `a < b`, `a ==
        60	/// b`, and `a > b` are all false.

Resolution: Change `PartialEq` to `PartialOrd` at lib.rs:277, then `just readme` so README.md:281 follows. Acceptance: `grep -n 'PartialEq.*causal' crates/before/src/lib.rs crates/before/README.md` returns nothing and `just readme-check` passes.

### crate-root-1: AGENTS.md sends readers to a "Law of Disjointness" and an `implementation` module that do not exist
- Where: crates/before/AGENTS.md:5-6 (related: crates/before/examples/code_study.rs:6, crates/before/src/lib.rs:224-231, crates/before/src/lib.rs:415-454)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `implementation\b` and `Law of Disjointness` over crates/before; `ls crates/before/src` shows no implementation.rs; lib.rs module list read at 415-454); executed: no
- Seen by: structure, prose; refutation: confirmed; history: contradicts-hard-rule (the rule was renamed at a431eaf1d, the module retired at 22cdfbe1; AGENTS.md last touched 04b4d1b7, between the two)
- Owner-gated: no

The guidepost every agent reads first points at a rule name lib.rs no longer uses (the safety rules are "Causal Singularity" and "Identity Linearity", lib.rs:224 and 231) and at a public module that no longer exists; `examples/code_study.rs:6` links `before::implementation` as well. The root AGENTS.md hard rule forbids references to code that no longer exists.

Evidence:

     5  crate docs for the model (`Party`/`Version`/`Clock`, the Law of
     6  Disjointness) and the public `implementation` module for the design essay,

    code_study.rs:
     6  //! [`implementation`](before::implementation) essay's "Small values over

Resolution: Point at what exists: the crate docs' "Safety rules" section, and `version/skyline.rs` (which line 7 already names) for the coding and kernels; fix or remove the `before::implementation` link in code_study.rs, naming the module doc that now carries the "Small values over large" trade. Acceptance: every module and rule name AGENTS.md cites resolves in src; `grep -rn 'before::implementation\|Law of Disjointness' crates/before` is empty.

### crate-root-2: Cargo description names a "transient fixed-width working form" the crate no longer has
- Where: crates/before/Cargo.toml:5-5 (related: crates/before/results/benchmarks/README.md:4, crates/before/src/lib.rs:333-340)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep -rni 'working form' over crates/before: Cargo.toml:5, results/benchmarks/README.md:4, and benches/board.rs:185 in an unrelated sense; nothing in src); executed: no
- Seen by: structure, prose, claims; refutation: confirmed; history: contradicts-hard-rule (faf3cd0a6 deleted `version/working.rs`, whose module doc was "The transient fixed-width working form for event mutation"; 1af119c1f re-touched the line and left the phrase)
- Owner-gated: yes (crates.io-facing wording)

The one line every crates.io visitor reads describes a design element that was deleted; the crate docs now describe fused single-pass kernels over the packed form (lib.rs:336-338). `results/benchmarks/README.md:4` carries the same ghost outside this partition.

Evidence:

     5  description = "Interval Tree Clocks (Almeida, Baquero & Fonte, 2008): packed bit-stream storage, transient fixed-width working form, linear-typed API."

Resolution: Restate against today's design, for example "Interval Tree Clocks (Almeida, Baquero & Fonte, 2008): canonical packed bit-stream storage, fused streaming kernels, linear-typed API"; fix results/benchmarks/README.md:4 in the same pass. Acceptance: `grep -rni 'working form' crates/before --include='*.toml' --include='*.md'` is empty.

### crate-root-4: build.rs's module doc and the manifest comment describe only the fuelscape job; the "holds no constants" clause is contradicted by the v3 banner literal
- Where: crates/before/build.rs:1-22 (related: crates/before/Cargo.toml:16-19, crates/before/build.rs:93-100, crates/before/build.rs:114-115, crates/before/build.rs:183-197, crates/before/build.rs:272-278, crates/before-fuelscape/src/compact.rs:82)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (build.rs read in full; `grep -n FORMAT_VERSION crates/before-fuelscape/src/compact.rs` gives line 82 `const FORMAT_VERSION: u32 = 3;`); executed: no
- Seen by: structure, prose; refutation: confirmed; history: deliberate-but-expired (the doc and Cargo.toml:16-19 are from 0a8884ee4 and were accurate then; 2efff1498 added the figure job without amending either; 6f63edb7 bumped the banner from v2 to v3 in both the tuple and the message)
- Owner-gated: no

The doc's Inputs and Outputs (3-15) name `fuelscape/` and `docs/` and `$OUT_DIR/fuelscapes/...`, but `main` also reads `results/space_consumption/itc_space_consumption.svg` (93), writes `$OUT_DIR/space_consumption.svg` (95-99), and under `BEFORE_REGEN_DOC_FIGURE` writes into the source tree (187-189); Cargo.toml:16-19 has the same blind spot. Line 18-19 says the script "holds no constants the widget or compactor also hold", yet `check_banner` holds the format version `3` twice (as a value and inside the message string), a number the compactor owns as `FORMAT_VERSION`. `theme_svg`'s first sentence (114-115) names only the rustdoc target though it also derives the README copy.

Evidence:

     9  //! Outputs: `$OUT_DIR/fuelscapes/<op>.html` — one single-line
    10  //! `<details>` island per operation, pulled into a `# Complexity`
    ...
    17  //! This script is a pure formatter: it re-bins nothing, computes no
    18  //! statistics, and holds no constants the widget or compactor also
    19  //! hold. Every failure here is a defect in the committed repository
    ...
   273      assert_eq!(
   274          (doc["format"].as_str(), doc["version"].as_u64()),
   275          (Some(expected), Some(3)),
   276          "{file}: not a {expected} v3 document"
   277      );

Resolution: Add the figure job to Inputs (the results/ SVG and the committed README SVG) and Outputs (`$OUT_DIR/space_consumption.svg`; the opt-in source-tree write); mirror it in Cargo.toml:16-19; widen `theme_svg`'s first sentence to both targets. Name the banner version once (`const WIDGET_DATA_VERSION: u64 = 3;` used in the tuple and interpolated into the message) and amend lines 18-19 to say the format banner is the one deliberately shared constant, the consumer's pin on the compactor's number. Acceptance: every path `main` reads or writes appears in the module doc; `grep -n 'Some(3)\|v3' crates/before/build.rs` finds only the constant's definition and its interpolation.

### crate-root-16: `Parse`'s doc omits `Ticks` and asserts paper notation for a decimal parse
- Where: crates/before/src/error.rs:94-98 (related: crates/before/src/version/ticks.rs:205-212, crates/before/src/error.rs:47-51)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (ticks.rs:205-206 `impl FromStr for Ticks { type Err = Parse;` parsing ASCII digits; error.rs:49 itself parses a `Ticks` in the `TooWide` example); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (the producer list is from dc88e755be; `Ticks` gained `FromStr` at 56a08f907; ebcf0087f7 re-edited the second sentence afterwards without adding it)
- Owner-gated: no

Public error docs are where a user learns which operations can produce the error; the list misses one producer, and "the original paper's notation" is false for the decimal count.

Evidence:

    94  /// Why a string or Rust literal failed to parse into a [`Party`](crate::Party),
    95  /// [`Version`](crate::Version), or [`Clock`](crate::Clock).
    96  ///
    97  /// Parsing uses the original paper's notation and strictly rejects
    98  /// non-canonical input.

Resolution: Add [`Ticks`](crate::Ticks) and qualify: "Party, Version, and Clock parse the paper's notation; Ticks parses a decimal count. Every parser strictly rejects non-canonical input." (Or drop the list: "into one of the crate's `FromStr` types".) Acceptance: the doc names every `FromStr` impl whose `Err = Parse`, or names none by type.

### crate-root-26: "mint" for constructing a value, in the Quickstart and crate-wide
- Where: crates/before/src/lib.rs:49-49 (related: crates/before/src/serde_impls/tests.rs:111, crates/before/src/party.rs:15, crates/before/src/party.rs:872, crates/before/src/version/skyline.rs:7, and roughly twenty further sites listed by `grep -rni '\bmint' crates/before/src`)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep; in-partition sites lib.rs:49 and serde_impls/tests.rs:111; crate-wide hits in laws.rs, meter.rs, version.rs, causally/conjunction.rs, party.rs, skyline.rs and its submodules, meter/board/*, including a function named `Reign::mint` in skyline/query/web.rs); executed: no
- Seen by: prose, structure, correctness; refutation: confirmed (a crate-wide sweep, not a partition fix); history: contradicts-hard-rule (the ban is in the owner's live writing-style.md; every site predates its adoption)
- Owner-gated: no

The owner's vocabulary rule: never write "mint" for constructing a value. The Quickstart site is the crate's first code sample and flows into the derived README.

Evidence:

    49  //! // New participants fork off a live clock, never mint themselves.

    serde_impls/tests.rs:
   110  /// the self-describing (number-array) paths, for every rejection genre the raw
   111  /// decodes mint — trailing bytes on each type, the rank-mismatch composite

Resolution: lib.rs:49 "// New participants fork off a live clock; nothing creates a second seed."; serde_impls/tests.rs:111 "every rejection genre the raw decodes produce"; sweep the remaining sites (party.rs:15 "which create a second holder", :872 "Creates identity exactly as ..."; rename `Reign::mint` if the owner wants the rule to reach identifiers). Acceptance: `grep -rni '\bmint' crates/before/src` is empty, or lists only identifiers the owner exempts.

### crate-root-27: Safety rule 2 says bytes are "exactly one hole" in linearity; party.rs names three doors
- Where: crates/before/src/lib.rs:236-238 (related: crates/before/src/party.rs:13-17, crates/before/src/party.rs:534, crates/before/src/party.rs:784, crates/before/src/clock.rs:890, crates/before/src/clock.rs:942)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`pub fn dangerously_alias` at party.rs:534 and clock.rs:890; `impl FromStr for` Party at party.rs:784 and Clock at clock.rs:942; party.rs:13-17 read); executed: no
- Seen by: prose, correctness; refutation: confirmed, low (the bytes hole is the one a user trips unknowingly; the other two are labeled dangerous or as fresh-universe doors at their sites, so the omission misdirects an auditor rather than a user); history: no-rationale-found (the owner's a431eaf1d sentence postdates both other doors and party.rs's three-door enumeration)
- Owner-gated: no

The crate-level rule is the hazard map a user reads first; a reviewer auditing an application for linearity on its strength checks only decode sites. party.rs's own module doc lists the serialization door, the text/literal door, and `dangerously_alias`. Hand-maintained counts ("exactly one") rot.

Evidence:

   236  //!    [`!Clone`](Clone). This leaves exactly one hole: bytes. A
   237  //!    serialized state sidesteps the type system, and
   238  //!    [`decode`](Clock::decode) cannot tell the latest state from a stale

    party.rs:
    13  //! merely (mutably) borrows. The type system enforces that linearity up to the
    14  //! documented escape hatches: the serialization and text/literal doors, which
    15  //! mint a second holder from bytes or notation, and
    16  //! [`dangerously_alias`](Party::dangerously_alias), the deliberate in-memory
    17  //! duplication.

Resolution: State the three doors as party.rs does (bytes via `decode`, serde, and borsh; text notation via `FromStr` and literals; `dangerously_alias`), each creating a second holder of an identity, and keep the bytes paragraph as the one that is easy to trip unknowingly; `just readme`. Acceptance: rule 2 names every public constructor that yields a `Party`/`Clock` sharing identity with an existing handle; no sentence asserts a count of holes.

### crate-root-28: Crate-docs accuracy slips: `PartialEq` where `PartialOrd` is meant, the `'static`-only `Deserialize`, and the unmentioned `surface` module
- Where: crates/before/src/lib.rs:277-277 (related: crates/before/src/lib.rs:30, crates/before/src/lib.rs:392-393, crates/before/src/lib.rs:398-403, crates/before/src/lib.rs:441-442, crates/before/src/serde_impls.rs:89, crates/before/src/serde_impls.rs:109, crates/before/src/borsh_impls.rs:240, crates/before/src/borsh_impls.rs:275)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (lib.rs:30's types table names `PartialOrd` for `<`, `<=`, `concurrent`; serde_impls.rs:89 `impl<'de> Deserialize<'de> for Ranked<'static>` and :109 likewise for `Span<'static>`, borsh at 240 and 275; `pub mod surface` gated by `meter` at lib.rs:441-442); executed: no
- Seen by: structure, prose, correctness; refutation: confirmed, low; history: no-rationale-found (line 277 was introduced whole by a431eaf1d, a slip against the line-30 table; the instrument bullet predates `pub mod surface`)
- Owner-gated: no

Three sentences a library user reads to choose a trait bound, a feature, or a field type are wrong or incomplete: `PartialEq` on `Version` is byte equality and the causal ordering belongs to `PartialOrd`; the serde bullet does not say `Deserialize` yields only the owned `Ranked<'static>`/`Span<'static>` (a user deriving `Deserialize` on a struct holding `Span<'a>` hits this without warning); the `meter` bullet omits the `surface` roster the same feature gates.

Evidence:

   277  //! [`Version`]'s [`PartialEq`] describes a causal ordering: `a <= b` tests
    ...
   392  //! - **`serde`:** `Serialize`/`Deserialize` for [`Party`], [`Version`],
   393  //!   [`Clock`], [`Rank`], [`Ranked`], and [`Span`].

Resolution: `[`PartialOrd`]` at 277 (add "its `PartialEq` is byte equality on the canonical encoding" if wanted); append "`Deserialize` yields the owned forms `Ranked<'static>` and `Span<'static>`" to the serde bullet; add the operation roster (`surface`) to the instrument bullet; `just readme`. Acceptance: 277 names `PartialOrd`; the feature list names every `pub mod` a feature enables and states the lifetime of the deserialized view types.

### api-audit-4: Crate docs attribute the causal ordering to PartialEq instead of PartialOrd
- Where: crates/before/src/lib.rs:277-279 (related: lib.rs:30, version.rs:58-60, crates/before/README.md:281)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read lib.rs:25-35 and 275-279, version.rs:50-60, and the comparison matrix at version.rs:1713-1760; `grep 'describes a causal ordering' crates/before/README.md` hits line 281, so the derived README carries the same sentence); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`a <= b` is `PartialOrd`; the types table at lib.rs:30 and the `Version` docs at
version.rs:58 name it correctly. The crate deliberately splits equality (a byte
compare) from ordering (the causal sweep), so naming the wrong trait here
misdirects exactly the reader who goes looking for the impl.

Evidence:

       277	//! [`Version`]'s [`PartialEq`] describes a causal ordering: `a <= b` tests
       278	//! containment of history, and two versions with no containing order are
       279	//! [`concurrent`](Version::concurrent). Three tools extend it:

Resolution: replace `PartialEq` with `PartialOrd` in the sentence, then `just readme` to regenerate crates/before/README.md. Acceptance: lib.rs:277 and README.md:281 name `PartialOrd`.

### fresh-eyes-3: The serde representation is undocumented (JSON emits a numeric byte array)
- Where: crates/before/src/lib.rs:392-393 (related: crates/before/src/serde_impls.rs:20-24, 33-37, 46-50, 64-68; crates/before/src/lib.rs:263-264; crates/before/README.md:386-387 (derived))
- Class / severity / confidence: documentation / low / high
- Provenance: verified (the sweep's run2.log lines 33-34: `serde_json version: [41,42,91,23,118,92,74,75,128]`, `serde_json party: [32]`; this pass read serde_impls.rs: every `Serialize` impl calls `serialize_bytes(&self.encode())` and every `Deserialize` goes through `<Vec<u8>>::deserialize`, and `is_human_readable` appears nowhere in the crate); executed: yes, by the sweep's scratch run, matched to its log
- Verification: confirmed; history: no-rationale-found (no agent note or commit records a decision on the human-readable form)
- Owner-gated: no for stating the representation; yes for changing it

The `serde` feature docs say which types implement the traits and (at lib.rs:263-264) that serde serializes "through the same encodings"; a serde_json user gets each byte as a JSON number, several times the wire size, and cannot learn that without running the code.

Evidence:

       392	//! - **`serde`:** `Serialize`/`Deserialize` for [`Party`], [`Version`],
       393	//!   [`Clock`], [`Rank`], [`Ranked`], and [`Span`].

        20	impl Serialize for Party {
        21	    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        22	        s.serialize_bytes(&self.encode())
        23	    }
        24	}

Resolution: State in the feature docs that every type serializes as the bytes of its canonical encoding via `serialize_bytes`, so binary formats carry the wire bytes and human-readable formats carry a sequence of integers; then `just readme`. Whether to branch on `Serializer::is_human_readable()` and emit the paper notation is an owner decision (see Open questions). Acceptance: the `serde` bullet names the representation.

### inventory-3: AGENTS.md points readers at the retired `implementation` module
- Where: crates/before/AGENTS.md:5-7 (related: crates/before/src/lib.rs:415-454)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'mod implementation' crates/before/src` returns nothing; `find crates/before -name 'implementation*'` returns nothing; `git log -S'pub mod implementation' -- crates/before/src/lib.rs` shows 67970b75 adding it and 22cdfbe1 removing it, whose message reads "The design-essay implementation module is retired."); executed: yes (the greps and git log settle it)
- Verification: confirmed; history: no-rationale-found (a miss in 22cdfbe1's doc sweep)
- Owner-gated: no

The crate guidepost routes readers to a public `implementation` module for
the design essay; lib.rs declares no such module.

Evidence:

         5	crate docs for the model (`Party`/`Version`/`Clock`, the Law of
         6	Disjointness) and the public `implementation` module for the design essay,
         7	`version/skyline.rs` for the stored coding and its operation kernels, and

Resolution: remove the `implementation` clause, or point at where the essay's
content lives now (22cdfbe1 says the `Span` type docs carry the operation
table, algebra, and wire form). Acceptance: every module the guidepost names
exists in lib.rs's module list.

### module-graph-5: AGENTS.md points at a public `implementation` module that no longer exists
- Where: crates/before/AGENTS.md:5-7 (related: crates/before/src/lib.rs:415-454)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'design essay' crates/before/src crates/before/AGENTS.md crates/before/README.md` hits only AGENTS.md:6; `git log -S'mod implementation' -- crates/before/src/lib.rs` names 22cdfbe1 as the removal, whose message says "The design-essay implementation module is retired."); executed: yes: the two git and grep commands above
- Verification: confirmed; history: already-known: the removing commit records the retirement; the guidepost was not updated.
- Owner-gated: no

The guidepost's orientation sentence sends a reader to a module lib.rs no longer declares, in the
one place a cold reader looks first.

Evidence:

         5	crate docs for the model (`Party`/`Version`/`Clock`, the Law of
         6	Disjointness) and the public `implementation` module for the design essay,
         7	`version/skyline.rs` for the stored coding and its operation kernels, and

Resolution: Drop the clause, or point it at wherever the design essay's content now lives (the
22cdfbe1 message says the module was retired, not moved, so dropping is the default). Acceptance:
every path and module AGENTS.md names resolves in the tree.

### paper-fidelity-4: AGENTS.md points paper-readers at a retired `implementation` module and an unnamed "Law of Disjointness"
- Where: crates/before/AGENTS.md:3-7 (related: crates/before/src/version/skyline/build/tests.rs:394, crates/before/src/lib.rs:224,231,425-433, crates/before/src/version.rs:26-29)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for `mod implementation` and `Law of Disjointness` across `crates/before`; `git log -S 'pub mod implementation'` and the two resulting commits read; lib.rs's public API list read); executed: no
- Verification: confirmed and expanded to a second site; history: deliberate-but-expired (f204638d, 2026-07-27, set this pointer to the then-new public `implementation` module; 22cdfbe1, 2026-08-17, deleted `src/implementation.rs` with the message "The design-essay implementation module is retired." and did not relocate it: that commit changes `skyline.rs` by 6 lines and `lib.rs` by 7)
- Owner-gated: no

The guidepost's orientation sentence names a public `implementation` module that no longer exists and a "Law of Disjointness" no source file uses (the crate docs call the two rules **Causal Singularity** and **Identity Linearity**). A test comment in `skyline/build/tests.rs` cites the same essay. This is the crate's own hard rule ("Nothing in the codebase refers to code that no longer exists") breached at the first paragraph an agent reads, and the same sentence was repaired once before for a different ghost path.

Evidence:

    crates/before/AGENTS.md:
         5	crate docs for the model (`Party`/`Version`/`Clock`, the Law of
         6	Disjointness) and the public `implementation` module for the design essay,

    crates/before/src/version/skyline/build/tests.rs:
       394	/// independently; a different integer code (the `implementation` essay
       395	/// contemplates ζ₂, whose zero costs two bits) would silently turn every

    crates/before/src/lib.rs:
       224	//! 1. **Causal Singularity.** A system of clocks has one [`Clock::seed`] (or
       231	//! 2. **Identity Linearity.** Advancing a [`Clock`] or [`Party`] (by

Resolution: in AGENTS.md, name the safety rules as the crate docs do and point at where the design content lives now (`version/skyline.rs`'s module doc is `pub` only under `test`/`meter`, version.rs:26-29, so say so or point at the crate docs); in `build/tests.rs:394-395`, state the ζ₂ alternative inline ("a code whose zero costs two bits") instead of citing the deleted essay. Acceptance: `grep -rn 'implementation' crates/before/AGENTS.md crates/before/src` returns no reference to a module or essay, and every rule name in AGENTS.md appears in `lib.rs`.

### rumors-dependence-6: Three sites cite the retired `implementation` module
- Where: crates/before/AGENTS.md:5-6 (related: crates/before/examples/code_study.rs:5-7, crates/before/src/version/skyline/build/tests.rs:394-395)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git show 22cdfbe1a --stat` lists `crates/before/src/implementation.rs | 270 -------------`, its message reads "The design-essay implementation module is retired.", and `lib.rs` declares no such module today); executed: no
- Verification: found during this pass, outside the dependence lens, recorded here so it has a disposition; history: deliberate-but-expired (the module was retired in 22cdfbe1a; the citations were not excised)
- Owner-gated: no

`crates/before/AGENTS.md` sends the reader to "the public `implementation` module for the design essay"; `examples/code_study.rs` carries an intra-doc link to `before::implementation`; a test doc in `version/skyline/build/tests.rs` cites "the `implementation` essay". No module of that name exists (the `pub mod` declarations in `lib.rs` are `causally`, `error`, `iter`, `shape`, `oracle`, `meter`, `surface`, `laws`). This breaches before's hard rule that nothing in the codebase refers to code that no longer exists, at the guidepost the AGENTS.md itself is.

Evidence:

    crates/before/AGENTS.md:
         5	crate docs for the model (`Party`/`Version`/`Clock`, the Law of
         6	Disjointness) and the public `implementation` module for the design essay,

    crates/before/examples/code_study.rs:
         5	//! This is the instrument behind the crate docs' integer-code figures (the
         6	//! [`implementation`](before::implementation) essay's "Small values over
         7	//! large" trade): the constants below are the as-run parameters of the

    crates/before/src/version/skyline/build/tests.rs:
       394	/// independently; a different integer code (the `implementation` essay
       395	/// contemplates ζ₂, whose zero costs two bits) would silently turn every

Resolution: excise or re-point each citation to where the material lives now, in the present tense (the crate docs for the model; `version/skyline.rs` for the stored coding and its integer-code trade, if the "Small values over large" argument survives there; otherwise drop the reference). Acceptance: `grep -rn implementation crates/before --include='*.rs' --include='*.md'` finds no reference to a module or essay of that name.

**Nits (13), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| crate-root-13 | `crates/before/src/error.rs:1-1` | `error` module's listing sentence is a joke, and three error types derive `Default` nothing defaults | A first sentence such as "The error types every fallible operation returns.", with the joke kept as a second line if wanted ... | `evidence/partitions/crate-root.md` |
| crate-root-14 | `crates/before/src/error.rs:5-5` | `Overlap`'s first sentence names only `Clock::sync`; `sync_all` also returns it | "Two parties were not disjoint when synchronizing clocks ([`Clock::sync`], [`Clock::sync_all`])." Acceptance: the doc names every public producer of ` ... | `evidence/partitions/crate-root.md` |
| crate-root-23 | `crates/before/src/iter.rs:9-10` | Unclosed backtick in the `Clock::forks` intra-doc link | `//! [`Clock::forks`](crate::Clock::forks).` Acceptance: the rendered `before::iter` page shows both links as code spans. | `evidence/partitions/crate-root.md` |
| crate-root-30 | `crates/before/src/lib.rs:350-358` | Register and redundancy nits in the crate-level docs and neighbors | 350 "The operations hold their bounds on every input shape."; 357 "with the bound holding on the rest"; 381 "Consequently" ... | `evidence/partitions/crate-root.md` |
| api-audit-5 | `crates/before/src/iter.rs:9-10` | iter module doc has an unclosed code span; the Clock::forks link text carries a literal backtick | add the missing backtick | `evidence/sweeps/api-audit.md` |
| api-audit-16 | `crates/before/src/error.rs:5-5` | error::Overlap's summary names only Clock::sync; Clock::sync_all returns it too | "...during [`Clock::sync`] or [`Clock::sync_all`]." Acceptance: both origins named. | `evidence/sweeps/api-audit.md` |
| api-audit-19 | `crates/before/src/lib.rs:49-49` | Vocabulary tells in public rustdoc: "mint" for constructing values, "honest" for exact | "create"/"build" for mint; "exact"/"the price of exactness" for honest, at the public sites first | `evidence/sweeps/api-audit.md` |
| api-audit-20 | `crates/before/src/error.rs:1-1` | The error module's first sentence is a rhetorical question | a descriptive sentence, e.g. "The error types: decode, parse, overlap, crossed-span ... | `evidence/sweeps/api-audit.md` |
| deps-12 | `crates/before/Cargo.toml:13-14` | docs.rs metadata enables no feature, so the serde and borsh impls the crate docs advertise never render there | add `features = ["serde", "borsh"]` to the docs.rs table (not | `evidence/sweeps/deps.md` |
| deps-15 | `crates/before/Cargo.toml:74-84` | manifest feature summaries lag their module docs: limb-meter lights two counters, touch-meter counts more than the manifest says | one clause each ("...and a second column counting | `evidence/sweeps/deps.md` |
| fresh-eyes-13 | `crates/before/src/error.rs:1-5` | The error module's summary line is a question, and Overlap's doc names one of its two producers | A summary that informs, such as "Error types: decode and parse failures, overlapping parties, crossed spans, and out-of-range tick counts." ... | `evidence/sweeps/fresh-eyes.md` |
| paper-fidelity-12 | `crates/before/src/lib.rs:277-279` | "PartialEq describes a causal ordering" should read PartialOrd | replace `PartialEq` with `PartialOrd`; regenerate the README | `evidence/sweeps/paper-fidelity.md` |
| prose-hygiene-13 | `crates/before/src/lib.rs:350-357` | Contract paragraph intensifiers and a significance adverb in public rustdoc | "The operations in this crate are hardened against pathological | `evidence/sweeps/prose-hygiene.md` |

**Cross-references.** crate-root-1, api-audit-3, inventory-3, module-graph-5, paper-fidelity-4, and rumors-dependence-6 report the same `AGENTS.md:5-7` ghost (the `implementation` module and the "Law of Disjointness"); benches-examples-18 and skyline-coding-14 are the two other citation sites of the same essay. The severities differ by site (the guidepost at low and medium, the example's reason to exist at high, the test doc at medium); one pass closes all eight. crate-root-28, api-audit-4, fresh-eyes-1, and paper-fidelity-12 are one sentence at lib.rs:277 reported four times at four severities; the fix is one word plus `just readme`. crate-root-23 and api-audit-5 are the same stray backtick at iter.rs:10. crate-root-13, crate-root-14, api-audit-20, api-audit-16, and fresh-eyes-13 are the same two lines of error.rs. crate-root-26, api-audit-19, and fresh-eyes-4 are the public "mint" sites; the census is prose-hygiene-5. crate-root-2's "working form" ghost recurs at results/benchmarks/README.md:4 (benches-examples). crate-root-4 and fuelscape-render-29 anchor the same build.rs module doc from two partitions; the fuelscape entry adds the validator drift and the banner literals. crate-root-24, crate-root-25, and crate-root-29 (the claim findings) carry the crate docs' unbacked 100×, "asymptotically linear", and 100-party figures that the register nits in crate-root-30 sit beside.

## Crate root and public types: clock

18 findings (0 high, 1 medium, 6 low, 11 nit). Full records: `evidence/partitions/clock.md`, `evidence/sweeps/api-audit.md`, `evidence/sweeps/fresh-eyes.md`, `evidence/sweeps/paper-fidelity.md`.

### clock-25: Test prose describes retired code or contradicts the code beside it at four sites
- Where: crates/before/src/clock/tests.rs:791-840 (related: crates/before/src/clock/tests.rs:297-309, 941-947, 1002-1004; crates/before/src/clock.rs:51, 733, 757-760, 819-821)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git log -S'no bit-level packing' -- crates/before/src/clock.rs` gives 32a655438 (2026-06-18), whose message states the encoding became the byte concatenation of `Party.encode() ++ Version.encode()`; `git log -S'non-byte-aligned'` on tests.rs bottoms out at the 2026-06-01 rename, so the comments predate the change; clock.rs:51 derives `PartialEq`; tests.rs:944-947 pins `Clock`'s `Debug` as the struct form); executed: no
- Seen by: structure, prose, correctness (each site by at least two lenses); refutation: confirmed (with the note that `encoding_views_agree_over_impl_history` at 334-336 subsumes the component-equality assertions, since `Eq` is byte equality); history: contradicts-hard-rule for 791-840, 297-309, and 172-174; deliberate-but-expired for 1002 (accurate for two hours until 46184a6a added the derive); the "Debug is the same as Display" comment at 941 was scoped to id/ev from birth at a519aa88 and never re-worded
- Owner-gated: no

The "decoded-component canonicity (regression)" section and its two test docs describe a bit-packed framing in which the version "begins at a generally non-byte-aligned bit offset" and `Clock::encode` "re-aligns each component"; at HEAD `Clock::encode_to` byte-concatenates two independently padded encodings and `Clock::decode` adopts byte-aligned slices of one buffer, the opposite of the copy-down-to-bit-0 the comment prescribes, so the guarded failure class is structurally unreachable and the two stated invariants are false. Three further comments contradict the code: 303-305 narrates a past bug ("once left stale bits") rather than stating the invariant it protects; 941 says "Debug is the same as Display" above an assertion that `Clock`'s `Debug` is `Clock { party: 1, version: 0 }`; 1002 says "Clock has no `PartialEq`" beside `#[derive(PartialEq, Eq, Hash)]`. The project's hard rule forbids prose describing code that no longer exists, and a test's doc comment is its statement of record: an incorrect one is a bug in the test.

Evidence:

       793	// `Clock::encode` lays the id directly before the event, so the event begins at
       794	// a generally non-byte-aligned bit offset. A `decode` that extracts the event
       795	// as an offset slice of the clock's buffer (rather than copying it down to
       796	// bit 0) leaves the recovered `Version`'s packed stream non-canonical:
       798	// Whole-clock round-trips hide this, because `Clock::encode` re-aligns each
       799	// component as it copies it in; the bug only shows when a component
       802	/// The seed's id is two bits, so its event starts at a non-byte-aligned offset.

    (clock.rs, today's framing)
       757	        // The clock's bytes are the byte-aligned [`Party`] encoding followed by
       758	        // the byte-aligned [`Version`] encoding. Each part is independently
       759	        // canonical and the party is self-delimiting (a decoder parses its id
       760	        // to find the split), so the two concatenate with no bit-level packing.

       303	    /// `fork`/`join`/`sync` *and* reads `as_bytes` — the exact seam where a
       304	    /// normalizing `join` once left stale bits in the stored buffer, so that

       941	    // Debug is the same as Display.
       944	    assert_eq!(
       945	        format!("{:?}", Clock::seed()),
       946	        "Clock { party: 1, version: 0 }"

      1002	    // Clock has no `PartialEq`, so compare the error directly.

    (clock.rs)
        51	#[derive(PartialEq, Eq, Hash)]

Resolution: For 791-840, either dissolve the section (334-339 already pins decoded-component equality over the same world population, and `decoded_seed_version_encodes_canonically` is its seed point case) or re-denominate it in present terms: `Clock::decode` adopts two byte slices of one read buffer as each component's storage, so each extracted component must itself be canonical storage and must re-encode and re-decode unchanged; drop the offset narrative. For 303-305, lead with the invariant ("after impl-driven `fork`/`join`/`sync`, `as_bytes` is byte-identical to `encode` and both decode to the value: no operation may leave stale bits in storage"). Reword 941 to "Party and Version render `Debug` as `Display`; `Clock`'s `Debug` is the struct form". Delete 1002 and compare the `Result`s directly. Acceptance: `git grep -n 'non-byte-aligned\|once left\|has no .PartialEq' crates/before/src/clock` is empty; the four comments match clock.rs:51, 757-760, and 916-923; `just test-all` green.

### clock-2: Rustdoc names parameters the signatures do not have, plus three typos
- Where: crates/before/src/clock.rs:107-113 (related: crates/before/src/clock.rs:126, 159, 166, 192, 338, 341, 377, 543; crates/before/src/clock/tests.rs:1232; crates/before/src/party.rs:239)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (signatures read at 126, 192, 377; `git grep -n iteratatively crates/before` returns only clock.rs:341; `git log -1 2efff149` records the rename of counts to `k` and fold inputs to `iter`); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: deliberate-but-expired (2efff149 renamed the parameters so `n` means bytes alone; the prose was not swept)
- Owner-gated: no

Public rustdoc describes `ticks` and `forks` in terms of `n` while the parameter is `k` (the `ticks` doc uses both names for one quantity), and `sync_all` speaks of `others` while the parameter is `iter`; "iteratatively" is misspelled in rendered rustdoc, "and returning" at 543 breaks the parallel with "and returns" at 479, and "balanced- forked" at tests.rs:1232 is a split word. A doc that names a parameter the signature lacks costs the reader a resolution step for no information, and the crate's own convention (counts are `k`, `n` is bytes) is contradicted at the sites that should exhibit it.

Evidence:

       107	    /// Advances this [`Clock`] by `n` events for its own [`Party`], returning
       112	    /// The count `k` is any unsigned number, since all can be converted into
       126	    pub fn ticks(&mut self, k: impl Into<Ticks>) -> &Version {
       159	    /// Splits `n` balanced child clocks off this [`Clock`], as a lazy
       192	    pub fn forks(&mut self, k: u64) -> Forks<'_> {
       338	    /// Reconciles this [`Clock`] with every *disjoint* clock in `others`,
       341	    /// Prefer this to iteratatively calling [`sync`](Clock::sync), as this is
       377	    pub fn sync_all<'a, I>(&mut self, iter: I) -> Result<&Version, Overlap>
       543	    /// version, without marking an event, and returning the new version.
      1232	/// Build the scenario orbits' fixed population: `n` clocks balanced- forked

Resolution: Rewrite the docs to the signatures' names (`k`, `iter`) so the 2efff149 convention holds in prose too; fix the three typos; sweep `Party::forks` (party.rs:239, "Splits `n` balanced shares") in the same pass. Acceptance: every backticked parameter name in clock.rs and forks.rs rustdoc appears in the corresponding signature; `git grep -n iteratatively crates/before` is empty.

### clock-4: `join_all`'s Errors section reads as returning input clocks, and "Unreachable" presumes linearity without saying so
- Where: crates/before/src/clock.rs:232-240 (related: crates/before/src/clock.rs:345-351; crates/before/src/fold.rs:31-36; crates/before/src/clock/tests.rs:53-56, 71-74, 91, 511-515; crates/before/src/party.rs:308-318)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read fold.rs:31-36 and the partition's tests that reach both errors with seed-descended clocks through `dangerously_alias`); executed: no
- Seen by: prose (coalesced hand-back); refutation: confirmed, and raised the "Unreachable" point as new; history: no-rationale-found
- Owner-gated: no

Two imprecisions in one public contract. First, "Returns the clocks whose parties *overlapped*" reads as elements of `iter`, but under the fold's retention policy a newer group that already coalesced stays on the stack and, if it later fails against `self`, is handed back as one `Clock` whose party is the union of several inputs and whose version is their join; the partition's own test doc states this outcome, the public contract does not, so `Err(v).len()` is not a count of rejected inputs and a handed-back clock's party may overlap none of `self`'s region. Second, "Unreachable for clocks descended from one seed" (here and on `sync_all` at 350-351) silently presumes the linearity rule: the partition's tests reach both errors with seed-descended clocks via `dangerously_alias`. The contract should name its precondition.

Evidence:

       234	    /// Returns the clocks whose parties *overlapped* and so could not be folded
       235	    /// in, dropping nothing: every input's party region and version are either
       236	    /// merged into `self` or handed back. In case of partial error, the set of
       237	    /// [`Clock`]s which are absorbed vs. handed back is unspecified.
       238	    ///
       239	    /// Unreachable for clocks descended from one [`seed`](Clock::seed): their
       240	    /// parties are pairwise disjoint.

    (fold.rs)
        31	/// - a failed combine (`Err((older, newer))`) retains the older group
        32	///   on the stack at its weight; a *lone* newer input (weight 0) is
        33	///   handed to `rejected`, while a newer group that already coalesced
        34	///   stays on the stack unmerged at the same weight — dropping nothing,

    (tests.rs)
        73	/// combine, then coalesces d∪e into it, so the hand-back is the four-input
        74	/// group and the accumulator absorbs only a∪b — with each input carrying a

Resolution: Add to `# Errors`: "A handed-back `Clock` may be the merge of several inputs that coalesced before the overlap was detected; its party covers exactly those inputs' regions and its version is their join." Change "Unreachable for clocks descended from one seed" to "Unreachable for clocks descended from one seed and handled linearly (the crate's safety rules)" at 239 and 350. Mirror the first sentence on `Party::join_all` (party.rs:312-315, outside this partition). Acceptance: the two `# Errors` sections state that hand-backs can be coalesced groups and name the linearity precondition; the law `clock_join_all_accepts_iff_parties_pairwise_disjoint` and the differential at tests.rs:78-93 remain the enforcement.
Construction: feed `join_all` the order `[a, b, alias(a), c, d, e]` over five forks as tests.rs:78-93 does; `Err(back)` has `back.len() == 1` and `back[0].party()` covers the regions of `alias(a)`, `c`, `d`, and `e`.

### clock-8: `sync_all` claims "maximally balanced, and therefore minimally large" with no derivation
- Where: crates/before/src/clock.rs:341-343 (related: crates/before/src/party/forks.rs:4-5, 16-17)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (party/forks.rs derives minimal depth `⌈log₂ k⌉` and nothing else; no share-size pin exists for `forks`); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The balanced split promises minimal depth, which party/forks.rs derives. "Minimally large" asserts a size optimum the crate neither derives nor pins, and a share's encoded size depends on the union's shape, not only on the split's depth. Space claims in this crate are hard guarantees; a superlative with no argument behind it is a claim the owner cannot defend and an instrument cannot check.

Evidence:

       341	    /// Prefer this to iteratatively calling [`sync`](Clock::sync), as this is
       342	    /// more efficient, and re-splits the inner [`Party`] of each [`Clock`] to
       343	    /// be maximally balanced, and therefore minimally large.

    (party/forks.rs)
         4	//! Both rest on `Split`, which lazily divides a region into `k` shares of
         5	//! minimal-depth (`⌈log₂ k⌉`) id tree, emitting one share per step. This is the

Resolution: State what is derived: "Prefer this to iterated `sync`: one balanced fold does the joins, and the union is re-split into shares of minimal depth (`⌈log₂ k⌉`) rather than the linear spine iterated `sync` builds." Acceptance: the sentence states the minimal-depth property and no size superlative.

### clock-11: The fallible construction doors carry no `# Errors` section while the joins do
- Where: crates/before/src/clock.rs:765-787 (related: crates/before/src/clock.rs:925-951, 954-983, 199-202; crates/before/src/party.rs:623; crates/before/src/version.rs:1110)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '# Errors' crates/before/src` outside tests: clock.rs:199, 232, 303, 345; party.rs:280, 310; span.rs:120; span/wire.rs:52, 79; rank.rs:403, 435; ranked.rs:241; admit.rs:261; none on any `decode`, `FromStr`, or `TryFrom`; 165 `# Panics` sections); executed: no
- Seen by: prose, correctness; refutation: confirmed; history: no-rationale-found (no convention, lint, or note requires the sections; the present ones arrived with features)
- Owner-gated: no

Within one file `join`, `join_all`, `sync`, and `sync_all` name their failure arms under `# Errors`, while `decode` (`Decode::{Io, Truncated, TrailingBits, NotCanonical}`), `FromStr` (`Parse::{Syntax, NotCanonical, Anonymous}`), and `TryFrom` fold theirs into a summary clause. Hazards belong under uniform sections so a user scanning for what can go wrong finds them in one place; here the convention covers half the fallible surface of one type.

Evidence:

       765	    /// Decodes a [`Clock`] from a reader of canonical bytes, strictly rejecting
       766	    /// malformed or non-canonical input.
       787	    pub fn decode<R: Read>(mut reader: R) -> Result<Self, Decode> {

    (contrast)
       199	    /// # Errors
       200	    ///
       201	    /// If the two clocks' [`Party`]s overlap, `self` is unmodified and `other`
       202	    /// is handed back in the error.

Resolution: Add `# Errors` to the three clock doors naming the variants and their triggers (decode: `Io` from the reader, `Truncated` for exhausted input including the flush-id cut, `TrailingBits` for a spurious remainder, `NotCanonical` for a collapsible id pair or an invalid version stream, all checked before any byte is adopted; text and literal doors: `Syntax`, `NotCanonical`, `Anonymous`), and sweep `Party`/`Version`'s doors in the same pass. Acceptance: every public `-> Result<_, _>` item on `Clock`, `Party`, and `Version` has an `# Errors` section.

### clock-20: Deleted production API names (`has_seen`, `happens_before`) presented as "the clock observers"
- Where: crates/before/src/clock/tests.rs:172-174 (related: crates/before/src/clock/tests.rs:186-188, 617-619; crates/before/src/oracle/clock.rs:123-129)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git grep -n 'has_seen\|happens_before' crates/before/src` hits only these two test sites, oracle/clock.rs:123-127, and oracle/tests.rs:401); executed: no
- Seen by: prose; refutation: confirmed; history: contradicts-hard-rule (`has_seen`/`happens_before`/`concurrent_with` were public `Clock` methods until a8c4d395 removed them; the test doc predates the removal and the comment at 618 postdates it by ten minutes)
- Owner-gated: no

The doc says "The clock observers match the oracle's: `has_seen` …, `happens_before` …", but `Clock` has no such methods, and the body calls neither of the oracle's `has_seen`/`happens_before` either: it compares `>=` and `<` on versions against the oracle's `>=`/`<`, and only `concurrent_with` is invoked. The root AGENTS.md forbids deleted API names in any prose; here they send the reader looking for an API that does not exist and describe a differential the body does not run.

Evidence:

       172	    /// The clock observers match the oracle's: `has_seen` is `msg <= version`,
       173	    /// `happens_before` is the strict causal order, and `concurrent_with` is
       174	    /// incomparability.
       186	        prop_assert_eq!(ia.version() >= msg, oa.version() >= msg_oracle);
       187	        prop_assert_eq!(ia.version() < ib.version(), oa.version() < ob.version());
       188	        prop_assert_eq!(ia.version().concurrent(ib.version()), oa.concurrent_with(ob));

       618	    // `has_seen` lowers to a deep `causal_cmp` against the version, and the

Resolution: Restate the doc in terms of what is compared: "Version order on production clocks matches the oracle: `>=` against a received version, strict `<` between clocks, and `concurrent` against the oracle's `concurrent_with`." Either call `oa.has_seen(&msg_oracle)`/`oa.happens_before(ob)` so the oracle observers are the reference, or drop their names. Rewrite 618 as "`>=` against a deep version lowers to a deep `causal_cmp`". Acceptance: no `Clock` test doc or comment names `has_seen`/`happens_before` unless the body calls the oracle method of that name.

### paper-fidelity-11: the paper's peek and anonymous stamp have no named counterpart in the public docs
- Where: crates/before/src/clock.rs:629-633 (related: crates/before/src/clock.rs:427-432,452-457,479-481, crates/before/src/party.rs:763-765, crates/before/examples/space_consumption.rs:24-32, crates/before/src/laws.rs:3002)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (grep for `peek` and `anonymous` across `crates/before/src`; the `Clock::version`/`send`/`recv`/`absorb` docs and the Quickstart read); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The crate models the paper's anonymous stamp `(0, e)` as a bare `Version` and `peek` as `Clock::version`, and forbids an anonymous `Party`. The public docs describe this model fully in the crate's own vocabulary (`send` is "named for the case where another party will recv", `absorb` learns history "without marking an event"), but the paper's names appear only in an example's mapping table and in test-only law docs; `anonymous` appears publicly only in parse-rejection notes. A reader arriving from the paper cannot find where `peek` and anonymous stamps went.

Evidence:

       629	    /// The current state of the [`Clock`], as a [`Version`].
       630	    ///
       631	    /// # Complexity
       632	    ///
       633	    /// `O(1)`.

    crates/before/src/party.rs:
       763	/// Parses paper notation (`0 | 1 | (i1, i2)`), strictly rejecting
       764	/// non-normal-form input and the anonymous identity `0` (a standalone `Party`
       765	/// must be a nonzero share).

Resolution: one sentence at `Clock::version` or in the crate docs' "Replicating clocks between processes": a bare `Version` is the paper's anonymous stamp `(0, e)`; `version()` is `peek`, `send` is event-then-peek, `absorb`/`|=` is the anonymous join, `recv` is join-then-event; no anonymous `Party` exists, which makes `event`'s `i ≠ 0` precondition structural. Acceptance: `grep -n peek crates/before/src/lib.rs crates/before/src/clock.rs` hits public rustdoc.

**Nits (11), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| clock-1 | `crates/before/src/clock.rs:57-58` | Safety rule cited by ordinal beside its name | Drop the parenthetical at clock.rs:57, party.rs:68, and lib.rs:270; cite by name, or by the anchor `[Safety rules](crate#safety-rules)` | `evidence/partitions/clock.md` |
| clock-5 | `crates/before/src/clock.rs:257` | The `result_large_err` allows are live at exactly the threshold, and only one of the two sites says why | Move the rationale to `join_all` (the site sync_all cites), stating the mechanism in one line: "the combine closure's `Err` is two `Clock`s ... | `evidence/partitions/clock.md` |
| clock-6 | `crates/before/src/clock.rs:262-274` | `join_all`'s maintainer comment re-derives `Party::join_all`'s argument instead of stating the delta | Reduce to the delta: `Party::join_all`'s fold carrying versions; the accept test indexes `self`'s party; the combine is `Clock::join` ... | `evidence/partitions/clock.md` |
| clock-15 | `crates/before/src/clock.rs:1049-1058` | The shared operator doc says "in either operand order" on the `\|=` cells, where order is fixed | Give the `as_clock` arm its own doc sentence ("`clock \|= version`: merge a received `Version` in place, without marking an event ... | `evidence/partitions/clock.md` |
| clock-18 | `crates/before/src/clock/tests.rs:16-22` | Pointer comment claims "every feed order"; the law drivers sample pool-indexed picks | "driven at boundary-band arities with pool-indexed picks (repeats and arbitrary orders arise from the draws)" ... | `evidence/partitions/clock.md` |
| clock-21 | `crates/before/src/clock/tests.rs:541-547` | Pointer-only section headers with a hand-maintained population count | Fold the three pointers (16-22, 543-547, 915-918) into one short "what lives elsewhere" paragraph at the top of the file naming the laws without the c ... | `evidence/partitions/clock.md` |
| clock-23 | `crates/before/src/clock/tests.rs:555-557` | Test doc cites AGENTS.md (a reverse citation) | Delete the parenthetical (naturally done while rewriting the headline under clock-22) | `evidence/partitions/clock.md` |
| clock-24 | `crates/before/src/clock/tests.rs:612` | Unanchored coinages and significance adverbs in maintainer prose | "keystone invariant" → "the invariant `Eq`/`Hash` rest on"; "text mirror" → "the text round-trip"; drop "genuinely", "really", and the " ... | `evidence/partitions/clock.md` |
| clock-27 | `crates/before/src/clock/tests.rs:1257-1259` | Banned "mints" in an orbit test doc | "iterated re-partitioning of an idle region adds no bytes, with no transient and no ratchet" | `evidence/partitions/clock.md` |
| api-audit-18 | `crates/before/src/clock.rs:107-113` | Parameter names drift between signature and prose (n vs k; rhs vs other; version vs other) | one letter per concept (`k` for counts, `other` for the second operand) in both prose and signatures | `evidence/sweeps/api-audit.md` |
| fresh-eyes-10 | `crates/before/src/clock.rs:107-113` | Parameter named `k` in signatures, `n` in prose, across ticks and forks | Use `k` (the signatures' letter) in every sentence and table row listed | `evidence/sweeps/fresh-eyes.md` |

**Cross-references.** clock-2, api-audit-18, and fresh-eyes-10 are the `n`/`k` parameter drift (party-7 has the party side). clock-4 shares the `join_all` hand-back contract with party-8, crate-root-18, and paper-fidelity-7. clock-11 shares the missing `# Errors` family with fresh-eyes-2, api-audit-8, and version-core-14. clock-17 (the API class) is the 32-bit `ExactSizeIterator::len` panic that clock-25's siblings touch. clock-23's `AGENTS.md` back-pointer dissolves with clock-22 (another class), the depth-100k headline rewrite. paper-fidelity-11's request for the paper's `peek` and anonymous-stamp names lands on `Clock::version`.

## Crate root and public types: party (party.rs, party/, idbits.rs)

19 findings (1 high, 2 medium, 12 low, 4 nit). Full records: `evidence/partitions/party.md`, `evidence/sweeps/api-audit.md`, `evidence/sweeps/fresh-eyes.md`, `evidence/sweeps/inventory.md`, `evidence/sweeps/paper-fidelity.md`, `evidence/sweeps/recursion.md`, `evidence/sweeps/rumors-dependence.md`.

### party-3: Prose refers to code that no longer exists: `EvNode`, `IdLit`, a removed `compare` op, an absent oracle note, a former encoding, and a "recursive form" on a loop
- Where: crates/before/src/idbits.rs:32-36 (related: crates/before/src/idbits.rs:2, crates/before/src/idbits.rs:61-64, crates/before/src/party.rs:802-809, crates/before/src/party/ops.rs:1-2, crates/before/src/party/tests.rs:368, crates/before/src/party/ops/diff.rs:63-66, crates/before/src/party/ops/split.rs:19)
- Class / severity / confidence: documentation / high / high
- Provenance: verified (`grep -rn EvNode crates/before/src` returns idbits.rs:35 only, and `git grep -c EvNode 782064269^` shows the grow.rs and grow/tests.rs occurrences that 782064269 removed; `grep -rn IdLit crates/` returns party.rs:806 only, the trait is `PartyLiteral` at party.rs:817, and the `///` block at 802-808 is attached to `mod sealed` at 809; `git log -S'fn compare' -- crates/before/src/party crates/before/src/idbits.rs` names 7139904bc; `grep -rn BitAnd crates/before/src` finds no prose note in the oracle, only `impl BitAnd<Version> for Version` at oracle/version.rs:454; split.rs:45-53 is a loop); executed: no
- Seen by: prose, structure, correctness (nit), claims; refutation: confirmed (its item 3, the `IdReader::at` pointer, is disputed and dropped); history: contradicts hard rule (each expiry commit named above)
- Owner-gated: no

Six sites breach the root and crate AGENTS.md hard rule "Nothing in the codebase refers to code that no longer exists": an event-side type `EvNode` (removed 782064269), a trait `IdLit` (the trait is `PartyLiteral`, and the doc block describing it sits on `mod sealed`, so it renders on the wrong item), an id operation `compare` listed twice and used as `compare == None` (removed 7139904bc), a pointer to "the note on the absent `BitAnd for Clock`" in an oracle module that contains no such note, a past-tense description of the encoding the pruned form replaced ("as they did when `0` was a real leaf in the stream"), and `split` described as "The recursive form" of the oracle when `build_split` has been a loop since 32a655438 (the sibling docs at sum.rs:17 and compare.rs:11 say "The cursor form"). Each sends a maintainer to a name or place that is not there.

Evidence:

        32	/// A decoded id node: the empty `0` leaf, the full `1` leaf, or an internal
        33	/// node tagged with which of its children are present.
        34	///
        35	/// The id-side analogue of the event side's `EvNode` — the clean shape the
        36	/// operations match on (the paper's id grammar `i ::= 0 | 1 | (i1, i2)`).

       806	/// literals. Unlike the public `TryFrom`, an `IdLit` leaf of `0` is allowed (it
       807	/// is a valid *sub-tree*); the anonymous check happens only once the whole id
       808	/// is assembled (see [`finish_id`]).
       809	mod sealed {

       368	// the overlap arms (`compare == None`, `sum == None`, `join == Err`) that the

        65	    /// it linearity-safe where a general id *meet* is not (see the note on the
        66	    /// absent `BitAnd for Clock` in [`oracle`](crate::oracle)): carving a

        64	///   exactly as they did when `0` was a real leaf in the stream.

        19	    /// The recursive form of `oracle::Party::split` (the paper's `split`).

Resolution: idbits.rs:35 drop the `EvNode` clause or re-state positively ("the shape the id walks match on"); move the party.rs:802-808 block onto `pub trait PartyLiteral` and write `PartyLiteral` (or "a literal leaf") for `IdLit`; delete `compare` from ops.rs:2 and idbits.rs:2 (party-2 rewrites those lines anyway) and rewrite tests.rs:368 over the ops that exist (`is_disjoint == false`, `sum == None`, `join == Err`); at diff.rs:65-66 either state the linearity note inline (the `diff` method doc is its natural home: a general meet can synthesize a region shared with a third live party, a difference cannot) or drop the pointer; idbits.rs:64 "exactly as they would if `0` were stored as a leaf"; split.rs:19 "The cursor form of `oracle::Party::split` (the paper's `split`)". Acceptance: `grep -rn 'EvNode\|IdLit\|compare\b' crates/before/src/party crates/before/src/idbits.rs` returns no prose hits; the private-items rustdoc shows the literal-door text on `PartyLiteral`; diff.rs:65-66 links to text that exists; `grep -n 'recursive form' crates/before/src/party/ops/split.rs` is empty.

### party-8: `join_all`'s `# Errors` contract promises the overlapping inputs back; the fold hands back coalesced unions
- Where: crates/before/src/party.rs:310-315 (related: crates/before/src/laws.rs:2402-2416, crates/before/src/fold.rs:31-36, crates/before/src/party/tests.rs:89-103)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read party.rs:312-315 against laws.rs:2406-2408 "the closing drain legitimately hands back *coalesced* groups, byte-distinct from every input", fold.rs:31-36's retention policy, and tests.rs:93-100's four-input hand-back `alias∪c∪d∪e`; `git show 4f12b8218:crates/before/src/party.rs` shows the earlier text carried the coalescing clause); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (b3f09baa0, the owner's doc pass, replaced the clause without a message)
- Owner-gated: no

The public sentence says the returned parties are "the parties which *overlapped*" and that "every input [`Party`] is either merged into `self` or handed back", but the balanced counter returns unions of several inputs, some of which overlapped nothing, whenever a coalesced group later fails a combine; the partition's own test and the laws roster both state this as correct behavior. A caller who treats each returned `Party` as one offending input (counting them, or retrying `join` with "the overlapping ones") has a wrong model. The true invariant is region conservation (`party_join_all_err_conserves_the_region_union`), and the prose should state it.

Evidence:

       310	    /// # Errors
       311	    ///
       312	    /// Returns the parties which *overlapped* and so could not be folded in,
       313	    /// dropping nothing: every input [`Party`] is either merged into `self` or
       314	    /// handed back. In case of partial error, the set of parties which are
       315	    /// absorbed vs. handed back is unspecified.

Resolution: Rewrite: on `Err`, `self` has absorbed some inputs (possibly none); the returned parties are unions of the remaining inputs, each containing at least one input that overlapped `self` or another input; no region is lost (`self` joined with the returned parties covers the original region plus every input's); which inputs are absorbed and how the rest are grouped is unspecified. Acceptance: the `# Errors` text describes union hand-back and region conservation, and `join_all_agrees_with_oracle_on_aliased_coalesced_group` reads as an instance of it rather than an exception.

### party-14: `Forks` promises "exactly `k`" shares; at `k == u64::MAX` it yields one fewer, and only a `//` comment and a test file say so
- Where: crates/before/src/party/forks.rs:78-80 (related: crates/before/src/party.rs:239-240, crates/before/src/clock/forks.rs:9, crates/before/src/clock.rs:166, crates/before/src/party/forks.rs:111-114, crates/before/tests/forks_max.rs:1-11)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -n 'saturat\|u64::MAX'` over party.rs, party/forks.rs, clock.rs, clock/forks.rs finds only forks.rs:111 (a `//` comment) and :114 (the `saturating_add`); no `///` or `//!` line mentions the boundary; `git show cdad46060:crates/before/src/party/forks.rs` shows the public clause "(at the one saturating input `n == u64::MAX`, one fewer — see [`Party::forks`])" that b3f09baa0 removed); executed: no
- Seen by: prose, correctness; refutation: confirmed (severity lowered to low by the refutation because the behavior is owner-ruled and pinned; kept at medium here because the public contract is false at an input the code handles deliberately and the pin's module doc points at "the documented behavior" that no longer exists); history: no rationale found for the removal (owner's doc passes b3f09baa0, a431eaf1)
- Owner-gated: no

The public rustdoc of `Forks`, `Party::forks`, `clock::Forks`, and `Clock::forks` promises `k` shares without qualification; `Forks::new` saturates `k + 1`, so `forks(u64::MAX)` yields `u64::MAX − 1`. The boundary is stated in a non-doc comment and in `tests/forks_max.rs`, whose module doc calls it "the documented behavior". Correct at all scales, for all inputs: the likelihood of `u64::MAX` carries no weight, and the contract as written is false at one input while a committed pin protects the behavior the contract contradicts.

Evidence:

        78	/// A lazy iterator of balanced [`Party`] shares, returned by [`Party::forks`].
        79	///
        80	/// Yields exactly `k` disjoint shares produced one at a time. The party it

       111	        // The count saturates: at `k == u64::MAX` the residual's headroom is
       112	        // spent and the iterator yields one share fewer than asked.

         1	//! `forks(u64::MAX)`: the documented behavior at the split count's
         2	//! saturation boundary, pinned in both profiles.

Resolution: One sentence on `Forks`' doc, with a pointer from `Party::forks`, and the same on `clock::Forks`/`Clock::forks`: "The count is `k` for every `k < u64::MAX`; at `u64::MAX` it saturates to `u64::MAX − 1`, because the borrowed party must keep one share." (cdad46060's wording is a usable reference.) Acceptance: `cargo doc` renders the saturation clause on both iterators and both methods; tests/forks_max.rs:1 cites public prose that exists.

### party-2: Hand-maintained operation and caller rosters have drifted
- Where: crates/before/src/idbits.rs:1-3 (related: crates/before/src/idbits.rs:66-70, crates/before/src/idbits.rs:129-131, crates/before/src/idbits.rs:175-176, crates/before/src/idbits.rs:207-209, crates/before/src/party/ops.rs:1-2, crates/before/src/party/ops.rs:6-7, crates/before/src/party/tests.rs:1-2)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn skip_subtree` shows callers in idbits.rs, diff.rs:368/371, grow.rs:301, meter/board/family.rs:1199; `grep -rn 'IdReader::at('` shows sum_split.rs:176/198-199, split.rs:116, fill.rs:524; `peek` callers at sum.rs:38-39, sum_split.rs:80, split.rs:22, build.rs:89; `bits()` callers at split.rs:26, sum_split.rs:114-115/172, build.rs:94); executed: no
- Seen by: prose, claims (the `grow`-only sentence), structure (item 5), refutation (three new sites); refutation: confirmed; history: contradicts doctrine (drift traceable: 7139904bc, a99e8c8f7, e5151f9bd, c7c9d3e8f, 33d37fb0f, 29d3c8f27, 522705cff)
- Owner-gated: no

Seven module- and item-level docs enumerate the operations or callers that use a thing, and each list is now incomplete or wrong: the idbits roster omits `diff`/`covers`/`sum_split`; `IdReader::peek`'s doc names `fill` as its user while four party kernels peek; `IdReader::bits`'s doc names `sum`/`diff` capacity hints while `split`, `sum_split`, and `copy_reader` call it for splice ranges; "The only operation that reads a tree twice is `grow`" omits `fill`'s memoized pre-scan, `IdIndex::build`'s two passes, and the two `IdReader::at` probes in `split`/`sum_split`; `skip_subtree`'s doc names two runners while `diff::consume` and the board's family generator also call it; ops.rs promises `O(n + m)` for "Every operation" while its own `index` submodule documents an extra `B log n` term, and lists two predicates that emit nothing under a "mutate by re-emission" rationale; tests.rs:1-2 says the tests are "all differential against the oracle" while the file holds orbit pins, scan floors, and constructed byte-checks. Doctrine: no hand-maintained enumerations of module contents or callers; state the structure, not the tally.

Evidence:

         1	//! A read-only cursor over the packed id encoding, shared by the party
         2	//! operations (`split`/`sum`/`is_disjoint`/`compare`) and the event operations
         3	//! (`fill`/`grow` walk the packed id alongside the working event tree).

        66	/// Not `Copy` or `Clone`: a cursor is single-use. Advancing it consumes the
        67	/// stream, so a stale or duplicated cursor (a re-scan, which would break the
        68	/// `O(n + m)` bound) cannot be formed by accident. The only operation that
        69	/// reads a tree twice is `grow` (on the event side), which rebuilds a fresh
        70	/// cursor from the source per pass.

       207	/// The single shared spelling of this scan: [`IdReader::skip`] runs it on the
       208	/// packed id encoding, and the skyline `grow` walks run it to skip event
       209	/// subtrees (one topology flag plus one skipped payload code per node).

         6	//! the *absence* of a child, never a node. Every operation is `O(n + m)` in its
         7	//! inputs, with no re-scan to find a right child, and none recurses — a deep

Resolution: Replace each roster with the mechanism it stands for: idbits.rs:1-3 "shared by the party operations in `party::ops` and by the event-side `fill`/`grow` walks"; idbits.rs:66-70 "a second cursor over a range is formed only deliberately, by `IdReader::at` from a recorded position, and each such site bounds its re-read where it lives"; idbits.rs:129-131 "a look at the current node, leaving the cursor in place"; idbits.rs:175-176 "for capacity hints and verbatim splice ranges"; idbits.rs:207-209 "every packed-tree skip in the crate routes through it"; ops.rs:6 "Every cursor operation is `O(n + m)` in its inputs (`index` documents the fold-only search term)"; ops.rs:1-2 drop the operation list; tests.rs:1-2 name the categories without "all". Acceptance: no module or item doc in the partition enumerates a caller or operation set that grep shows to be incomplete; the `compare` ghost (party-3) is gone from the same lines.

### party-5: Register and vocabulary sweep items: `mint`, em-dashes in `//` comments, `honest`, `tripwire`, a `d_` prefix, a dependent first sentence
- Where: crates/before/src/party.rs:13-17 (related: crates/before/src/party.rs:872, crates/before/src/party/ops/sum_split.rs:166, crates/before/src/party/tests.rs:1042, crates/before/src/party/tests.rs:30-32, crates/before/src/party/tests.rs:323, crates/before/src/party/tests.rs:430, crates/before/src/party/tests.rs:584, crates/before/src/party/tests.rs:993; em-dash `//` sites: party.rs:337, 344, 398, 399, 635, 638; forks.rs:51, 108, 109, 193; diff.rs:83, 84, 118, 119; index.rs:182, 187; compare.rs:19, 37; tests.rs:69, 70, 74, 365, 366, 1040)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn -i '\bmint'` over the partition returns exactly the four sites; `grep -n '^\s*//[^/!].*—'` over the twelve files returns exactly the 24 sites listed; `grep -n -i 'honest\|tripwire' tests.rs` returns lines 32, 61, 121, 177, 181, 251, 259, 279, 430, 584, 746; `grep -rn 'fn d_' crates/before/src` returns tests.rs:323 alone); executed: no
- Seen by: structure, prose; refutation: confirmed; history: contradicts writing-style rules (~/.claude/writing-style.md:149-151, 170, 326-330, 406) that postdate every site; crate-wide counts: `mint` 58, em-dash-in-`//` 374, `honest` 159, `tripwire` 75
- Owner-gated: no

Four uses of `mint` for constructing a value (the one banned word); 24 `//` comments carrying true em-dashes where the register rule wants ` -- `; `honest` eight times for "non-duplicated" or "defect-free" and `tripwire` twice (430, 584) for tests no committed known-bad fails (177 uses it correctly for the `surface_coverage` roster); a lone `d_` test-name prefix whose only sibling migrated to `laws.rs` at 86dd53a71; and a test doc whose first sentence ("The invariant holds for both halves ...") depends on the previous test's doc. These are instances of crate-wide sweeps, listed here so the partition's sites are in one place; the `mint` rule is the one with a stated ban.

Evidence:

        14	//! documented escape hatches: the serialization and text/literal doors, which
        15	//! mint a second holder from bytes or notation, and

       872	/// Mints identity exactly as the `u8` literal door does — a test and

       166	#[allow(clippy::type_complexity)] // two optional bit ranges: an inline pair over a minted name

        30	/// Aliased inputs stay best-effort: a duplicated share collides on its way in
        31	/// and is handed back whole (nothing panics, nothing is dropped), while the
        32	/// honest copy of every share still reunites the seed region.

       323	    fn d_fork_join_roundtrip(ops in world_strategy(), i in 0usize..64) {

       993	    /// The invariant holds for both halves produced by `fork` (the split path),

Resolution: party.rs:15 "create a second holder", party.rs:872 "Creates identity exactly as", sum_split.rs:166 "the inline pair reads better than a coined name" (party-6 removes the allow anyway), tests.rs:1042 "add more than its one tree level"; ` -- ` at the 24 `//` sites; "the original" / "the defect-free transcription" / "the sublinear arm" for `honest`; "The deterministic companion to" (430) and "witnesses and floors" (584) for `tripwire`; rename `d_fork_join_roundtrip` to `fork_join_roundtrip_matches_oracle`; tests.rs:993 "`as_bytes` equals `encode` for both halves produced by `fork`". Acceptance: `grep -rni '\bmint' crates/before/src/party.rs crates/before/src/party/ crates/before/src/idbits.rs` empty; `grep -n '^\s*//[^/!].*—'` over the partition empty; every `tripwire` in tests.rs names a committed known-bad artifact.

### party-7: Public rustdoc slips in `party.rs`: `n` in prose against `k` in signatures, a missing period, "logarithmic factor" for an additive term, and a `# Warning` that is `Clock::decode`'s text verbatim
- Where: crates/before/src/party.rs:184-184 (related: crates/before/src/party.rs:206, crates/before/src/party.rs:239-245, crates/before/src/party.rs:274, crates/before/src/party.rs:605-610, crates/before/src/clock.rs:166, crates/before/src/clock.rs:192, crates/before/src/clock.rs:768-772, crates/before/src/party/forks.rs:16-17, crates/before-fuelscape/src/ops.rs:772)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the six lines against clock.rs:768-772, which the warning matches word for word; `git show 2efff1498` confirms the `n -> k` parameter rename with its rationale "counts renamed to k so n means bytes alone"; `git show e546b6d5e:crates/before/src/party.rs` shows the earlier Party-specific warning text that b3f09baa0 replaced); executed: no
- Seen by: prose, claims ("logarithmic factor"); refutation: confirmed; history: the `k` side is deliberate (2efff1498), the prose `n` is the un-renamed remainder; the `Clock` warning replaced a Party-specific one in the owner's doc pass (b3f09baa0) with no message
- Owner-gated: no

Four public-doc accuracy items on one type. `ticks` and `forks` document `n` while their signatures take `k` (the rename to `k` is the deliberate side: "so `n` means bytes alone"); `ticks`' first sentence has no terminal period; `forks` says each share "increases in size by only a logarithmic factor" when the growth is an additive `2·⌈log₂(k+1)⌉` bits (forks.rs:16-17 and the island contract `k (|self| + log k)` both spell it additively); and `Party::decode`'s `# Warning` names `Clock` in every sentence and `Party` in none, being the clock.rs:770-772 text copied whole. The same `n`/`k` mismatch sits at clock.rs:166/192.

Evidence:

       184	    /// Advances `version` by `n` events for this [`Party`]

       206	    pub fn ticks(&self, version: &mut Version, k: impl Into<Ticks>) {

       242	    /// Unlike repeatedly calling [`fork`](Party::fork), which deepens its
       243	    /// representation into a biased linear tree (see its warning), every
       244	    /// resultant [`Party`] produced here increases in size by only a
       245	    /// logarithmic factor.

       607	    /// Serializing a [`Clock`](crate::Clock) circumvents its otherwise
       608	    /// compiler-enforced `!Clone` linearity. Deserializing one can violate
       609	    /// causality. Treat serialization/deserialization boundaries as *moves* of
       610	    /// the [`Clock`](crate::Clock).

Resolution: prose to `k` at party.rs:184, 239 and clock.rs:166 (do not rename the parameters back); add the period at 184; 242-245 "grows by only `O(log k)` bits over the party it was split from"; rewrite 605-610 in terms of `Party` ("Decoding creates a second holder of the share the bytes name, circumventing the compiler-enforced `!Clone` linearity; treat an encode/decode boundary as a *move* of the [`Party`], and the same for any [`Clock`](crate::Clock) built from it"; the e546b6d5e text is a usable reference). Acceptance: every `Party`/`Clock` doc names the parameter its signature declares; the `forks` sentence agrees with the island contract; the warning on `Party::decode` names `Party`.

### party-10: Maintainer-facing "single gate"/"single point" overclaims and small slips in `party.rs`, `sum.rs`, and `compare.rs`
- Where: crates/before/src/party.rs:791-800 (related: crates/before/src/party.rs:45, crates/before/src/party.rs:233-237, crates/before/src/party.rs:298-306, crates/before/src/party.rs:453, crates/before/src/party.rs:458-465, crates/before/src/party.rs:646, crates/before/src/party.rs:673-676, crates/before/src/party.rs:712-714, crates/before/src/party/forks.rs:113, crates/before/src/party/ops/sum.rs:11-13, crates/before/src/party/ops/sum_split.rs:85-90, crates/before/src/party/ops/compare.rs:11-12, crates/before/reference/itc2008.md:182, crates/before/src/oracle/party.rs:145)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn finish_id` lists callers at party.rs:787, 866, 889, 911 only; `fork`, `join`, `sum_split` call `from_bits` directly and `without` re-spells `id_is_empty`; `grep -n -i disjoint reference/itc2008.md` returns one line, inside the invariant paragraph, and oracle/party.rs:145 defines `is_disjoint`); executed: no
- Seen by: structure, prose; refutation: confirmed; history: `finish_id`'s claim expired at f0b837336 (`without` inlined the check from birth); sum.rs:11 expired at c7c9d3e8f; `mem::swap` predates `forks` (14d36c62d used `mem::replace` from birth); compare.rs:11's paper attribution is a choice with no stated reason (517c7ebeb)
- Owner-gated: no

Six maintainer-facing statements are false or imprecise, and two of them are exactly the kind a reviewer relies on to skip auditing other paths. `finish_id` is "The single gate through which every parsed/built top-level `Party` passes", but only the text and literal doors call it; `fork`/`join`/`sum_split` freeze through `from_bits`, and `without` duplicates the emptiness test inline. `sum` is "the single point of overlap detection", but `sum_split` detects overlap on the spine itself (its own doc says so). `anonymous()` cites `mem::swap` while the one use is `mem::replace`. The operations table names `b` in the call and `q` in the meaning. The `without` example compares `to_string()`s where `Party: Eq + Debug` makes `assert_eq!` on values direct. `is_disjoint` is attributed to "the paper's region-disjointness test" when the paper states disjointness as an invariant (`∀i1 ≠ i2. i1 · i2 = 0`), not an operation, and the reference the siblings cite by name is `oracle::Party::is_disjoint`.

Evidence:

       791	/// Wrap validated id bits as a `Party`, rejecting the anonymous (empty)
       792	/// identity. The single gate through which every parsed/built top-level `Party`
       793	/// passes.

       459	        let bits = self.view().diff(other.view());
       460	        if codec::id_is_empty(codec::built_view(&bits)) {

        11	    /// This is the single point of overlap detection: callers (`Party::join`)

       646	    /// Internal and transient only (i.e. for use in `mem::swap`) and *never* a

        45	/// | [`p.is_disjoint(&b)`](Party::is_disjoint)              | whether `p` and `q` share no region, hence may safely interact            |

       453	/// assert_eq!(p.without(&q).unwrap().to_string(), keep.to_string());

        11	    /// The cursor form of the paper's region-disjointness test, on the shared
        12	    /// lockstep predicate walk ([`lockstep_holds`]).

Resolution: Extract `fn nonempty(bits: codec::BitsBuf) -> Option<Party>` (the emptiness gate plus `from_bits`); `finish_id` becomes `nonempty(bits).ok_or(Parse::Anonymous)` and `without` becomes `nonempty(self.view().diff(other.view()))`; describe it as the gate for every top-level `Party` built from possibly-empty bits (the kernels that prove non-emptiness structurally freeze through `from_bits`). sum.rs:11: "the point of overlap detection for `join`: a successful `sum` is the disjointness proof (`sum_split` detects it the same way on the spine)". party.rs:646 `mem::replace`; party.rs:45 `q` in both columns; party.rs:453 `assert_eq!(p.without(&q).unwrap(), keep);`; compare.rs:11 "The cursor form of `oracle::Party::is_disjoint` (the paper's disjointness invariant `i1 · i2 = 0`, as a test)". Acceptance: `grep -n 'id_is_empty' crates/before/src/party.rs` shows one site; `grep -n 'single gate\|single point'` over party.rs and sum.rs returns claims true of the code; every "form of" line in party/ops names the oracle function it mirrors.

### party-15: `Forks`' Complexity section promises a per-step cost that is never stated
- Where: crates/before/src/party/forks.rs:86-92 (related: crates/before/src/party.rs:258, crates/before/src/clock.rs:178, crates/before/src/clock/forks.rs:18-19, crates/before/src/party/forks.rs:48-63)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read); executed: no
- Seen by: prose; refutation: confirmed (the proposed bound is plausible from `Split::next` but its constants were not derived); history: no rationale found (b3f09baa0's wording)
- Owner-gated: no

`Party::forks` and `Clock::forks` defer with "see [`Forks`] for the per-step and early-drop costs"; `Forks` gives the early-drop cost and describes a step only as "proportionate to its share of the drain", which is not a bound a caller can plan against. Public rustdoc carries complexity where it routes a decision, and lazy iteration exists so a caller can bound per-step latency; a pointer that lands on a non-answer costs two reads for nothing.

Evidence:

        90	/// Each `next` costs proportionate to its share of the drain; an early drop
        91	/// rejoins the unclaimed remainder in `O(|p| log k)`, with `|p|` the borrowed
        92	/// party's size in bytes.

       258	    /// Shares are built on demand; see [`Forks`] for the per-step and early-drop costs.

Resolution: State the per-step bound the code has: each `next` performs at most `⌈log₂ k⌉` forks along the spine of one pending region, each `O(|p| + log k)` bits, so a step is `O(|p| log k)` worst case and `O(|p| + log k)` amortized over a full drain; verify the constants against `Split::next` (forks.rs:48-63) before landing. Acceptance: a reader following the pointer from `Party::forks` or `Clock::forks` finds a per-step bound in `Forks`' Complexity section.

### party-17: `Open` token doc claims an unclosed node "cannot compile"; `#[must_use]` is a warn-level lint on discarded expressions, and the module destructures the token itself
- Where: crates/before/src/party/ops/build.rs:36-38 (related: crates/before/src/party/ops/build.rs:222, crates/before/src/party/ops/build.rs:262, crates/before/src/party/ops/build.rs:302, crates/before/src/lib.rs:412-413, justfile:127)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (lib.rs carries only `#![forbid(unsafe_code)]` and `#![warn(missing_docs)]`; the gate's `cargo clippy ... -- -D warnings` rejects a bare discarded `open()` but `let _o = b.open();` compiles silently; the destructure/reconstruct sites are build.rs:222, 262, 302); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate but expired (1d77f4c82 introduced the claim for a design where every walk closed via `close_node` with the token in hand; 7b11b3ea's `IdSkylineBuilder` bypasses the token onto `PosStack`)
- Owner-gated: no

Documentation altitude: a described type-level guarantee must be one the compiler gives. `#[must_use]` fires `unused_must_use` (warn by default) only when the value is discarded as a statement; reuse is prevented by move semantics, not the borrow checker; and `IdSkylineBuilder` destructures `Open(at)` onto `PosStack` and rebuilds `Open(self.tags.pop())` at close, so "closed exactly once" is a convention this module keeps, not a property the type enforces. A maintainer trusting the sentence would expect the compiler to catch a missing `close_node`; it will not.

Evidence:

        36	/// `!Clone` and `#[must_use]`: the token must be closed exactly once, and the
        37	/// borrow checker stops it being reused or dropped silently — so an open with
        38	/// no matching close cannot compile.

       222	            let Open(at) = self.out.open();

       302	                    kind = self.out.close_node(Open(self.tags.pop()), left, kind);

Resolution: Rewrite to what holds: "The token is `!Clone` and `#[must_use]`, so a discarded `open()` result warns and a token cannot be closed twice by accident; `IdSkylineBuilder` stores the position on `PosStack` and reconstructs the token at close, so the pairing there is kept by `close_up`'s stack discipline, not by the type." Alternatively have `PosStack` hand back `Open` tokens behind a method so the reconstruction is confined to one place. Acceptance: the `Open` doc makes no claim of a compile error; a reader can find where the token discipline is bypassed from the doc alone.

### party-20: Two `IdLeafCursor` types walk the same id coding; the decision to keep them separate is recorded only in an agent note
- Where: crates/before/src/party/ops/diff.rs:229-244 (related: crates/before/src/version/skyline/overlay.rs:16-20, crates/before/src/version/skyline/overlay.rs:471-485, crates/before/src/version/skyline/overlay.rs:583-614, crates/before/src/party/ops/diff.rs:414-437, crates/before/src/version/skyline/shape.rs:78-85, crates/before/src/version/skyline/masked.rs:209-211, crates/before/src/version/skyline/query.rs:502)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep 'impl PlateauCursor for'` returns three impls: diff.rs:381, overlay.rs:411, overlay.rs:555; the two `step` bodies implement the same pop-trailing-rights/flip/pop-right-present law with the same `unreachable!` text; the survey's §8 amendment at `.agent-notes/2026-07-29-fold-unification-survey/fold-unification-survey.md:499-509` records "Phase C: measured at the boundary and dropped, by this survey's own criterion"); executed: no
- Seen by: structure, correctness, claims; refutation: confirmed (unification feasible); history: already known and dropped on record (the merged cursor was judged not smaller or clearer: ~12 lines of residual flip bookkeeping against different descent policies, plus a re-pin bill)
- Owner-gated: no

The duplication three lenses reported is a decided drop, not a fresh defect: the survey measured the merge and rejected it. What the tree does not carry is that decision. `diff`'s cursor doc names the event-side `LeafCursor` as its sibling and never mentions the existing id cursor of the same name one module away, so the next reader re-derives the same finding (three lenses did). Code may not cite `.agent-notes`, so the rationale must be restated inline. overlay.rs:17's "the two cursor instances" is a module-scoped count (deliberately trimmed to the module's own two at c6ba2208) that a crate-wide reader takes as a crate-wide count; saying whose count it is closes that.

Evidence:

       229	/// A cursor at the current item of one packed id, read as a boolean
       230	/// skyline.
       231	///
       232	/// The id-side sibling of the event sweep's leaf cursor: the tag stream is
       233	/// consumed forward at most once, the root-to-item path is the only per-depth

        16	//! inside each slot's step; the boundary bookkeeping below is their shared
        17	//! correctness argument. Above them sit the two cursor instances.

Resolution: At diff.rs:229-244 add one paragraph: "A second id cursor beside [`overlay::IdLeafCursor`], deliberately: that cursor settles eagerly inside its step (its plateaus are the stored regions), this one defers settlement so the covered-block scans can skip what the sweep never visits; the shared flip bookkeeping is a dozen lines, and a merge would wrap an eager adapter around the settle-driven cursor or import block machinery into the shape and masked walks." At overlay.rs:17 "Above them sit this module's two cursor instances" (or name the third with its home). Acceptance: a reader of either cursor's doc can find the other and the reason they are two.

### party-27: `split`'s spine walk and output copies sit outside the scan meter; the exemption lives only in a `tests/meter.rs` comment, and `ops.rs` points at a rationale `build_split` does not carry
- Where: crates/before/src/party/ops/split.rs:45-53 (related: crates/before/src/party/ops.rs:38-40, crates/before/src/party/ops/split.rs:38-44, crates/before/src/party/ops/sum_split.rs:123-126, crates/before/src/party/ops/sum_split.rs:205-223, crates/before/src/codec/buf.rs:164-173, crates/before/src/codec/buf.rs:346-369, crates/before/src/codec/scan.rs:9-11, crates/before/tests/meter.rs:6397-6410)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'record_bits\|scan'` over split.rs and sum_split.rs returns only doc prose; `BitsBuf::push` and `extend_from_view` record nothing; the only statement of the exemption is tests/meter.rs:6400-6405; split.rs:38-44 describes the splice and never says why the builder is not used); executed: no
- Seen by: structure, claims ([57]); refutation: confirmed; history: deliberate and holds (fc862595d: "the scan pin records the split kernel's deliberately raw path, so wiring it into the metered primitives is a deliberate re-pin"), stated only in the test and in history
- Owner-gated: no (routing the walk through the meter would be, and is left as an open question)

`build_split` reads each spine tag with raw `bits.bit()` and writes both halves through `extend_from_view`/`BitsBuf::push`, none of which records, so `Party::fork`'s scan reading is a constant independent of spine depth; `sum_split`'s spine pushes and `half`/`splice` copies are outside the meter the same way. That is a recorded decision, but the record lives in one envelope's comment: nothing at the code says so, `scan.rs`'s coverage list reads as total, and ops.rs:40 sends the reader to `build_split` "for why it does not use the builder", where the doc states a property (verbatim copies of already-normal ranges) but never names the builder or the meter. Comments state what the code cannot show, at the code that creates the blind spot.

Evidence:

        45	fn build_split(bits: BitsView<'_>, start: u64) -> (BitsBuf, BitsBuf) {
        46	    let mut pos = start;
        47	    let (prefix_end, kind) = loop {
        48	        match (bits.bit(pos), bits.bit(pos + 1)) {
        49	            (false, false) => break (pos, SpineEnd::Terminal), // the `1` leaf
        50	            (true, true) => break (pos, SpineEnd::Branch),     // both-present branch
        51	            _ => pos += 2, // unary: descend the single present child (at pos + 2)
        52	        }
        53	    };

        38	//! normal form. Output is built by [`build::IdBuilder`] (`sum`), the
        39	//! leaf-driven [`build::IdSkylineBuilder`] (`diff`), or by direct bit-splice
        40	//! (`split`); see `split`'s `build_split` for why it does not use the builder.

      6400	/// The split kernel builds both halves by raw bit-slice writes and walks
      6401	/// the spine by raw indexing — deliberately outside the scan primitives —

Resolution: State the exemption where it lives: at `build_split` ("the halves are verbatim slices of already-normal ranges plus one retagged node, so no tag is reserved, patched, or collapsed and the builder's placeholder discipline buys nothing; the spine read and the copies are deliberately outside the scan meter, whose fork envelope pins the raw path's near-zero reading"), at `sum_split::half`/`splice`, and as an explicit bullet in scan.rs:9-11's coverage list naming the two kernels whose reads and writes it does not count. Acceptance: `grep -n 'outside the scan' crates/before/src/party/ops/split.rs crates/before/src/codec/scan.rs` hits; ops.rs:40's pointer lands on the answer it promises.

### api-audit-10: forks(u64::MAX) yields one share fewer than asked; the public docs say "exactly `k`" and the test calls this "documented"
- Where: crates/before/src/party.rs:239-276 (related: clock.rs:159-194, party/forks.rs:78-80, party/forks.rs:105-118, clock/forks.rs:7-9, tests/forks_max.rs:1-11)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `Forks::new`, `Split::new`, `Split::size_hint`, tests/forks_max.rs in full, and all four public docs); executed: no
- Verification: confirmed, with two more sites: the `Forks` type docs at party/forks.rs:80 and clock/forks.rs:9 both say "Yields exactly `k`", and tests/forks_max.rs:1-2 calls the saturation "the documented behavior", which nothing public documents; history: no-rationale-found
- Owner-gated: no

`Forks::new` splits `k.saturating_add(1)` ways and hands the first leaf to the
borrowed party, so at `k == u64::MAX` the iterator reports and yields
`u64::MAX - 1` shares. The saturation is the benign choice and is pinned by
tests/forks_max.rs, but the public contract at four sites promises `n`/`k`
shares unconditionally. Reading `len()` at that input costs O(1), so this is
not the infeasible-work corner the doctrine tolerates undocumented.

Evidence:

       239	    /// Splits `n` balanced shares off this [`Party`], as a lazy
       240	    /// [`ExactSizeIterator`].

        80	/// Yields exactly `k` disjoint shares produced one at a time. The party it

       111	        // The count saturates: at `k == u64::MAX` the residual's headroom is
       112	        // spent and the iterator yields one share fewer than asked.
       113	        let whole = mem::replace(party, Party::anonymous());
       114	        let mut split = Split::new(whole, k.saturating_add(1));

         1	//! `forks(u64::MAX)`: the documented behavior at the split count's
         2	//! saturation boundary, pinned in both profiles.

Resolution: one sentence in `Party::forks`, `Clock::forks`, and both `Forks` type docs ("`k == u64::MAX` saturates: `u64::MAX - 1` shares are yielded, the residual taking the last slot"), after which tests/forks_max.rs's "documented behavior" becomes true; or count in `u128` internally so `k` shares are always yielded. Acceptance: the public `forks` docs state the corner, or `forks(u64::MAX).len()` equals `u64::MAX` on 64-bit.

### api-audit-15: Party::decode's Warning is Clock::decode's text and never names Party
- Where: crates/before/src/party.rs:605-610 (related: clock.rs:768-772, party.rs:10-17)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both sites side by side); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Evidence:

       605	    /// # Warning
       606	    ///
       607	    /// Serializing a [`Clock`](crate::Clock) circumvents its otherwise
       608	    /// compiler-enforced `!Clone` linearity. Deserializing one can violate
       609	    /// causality. Treat serialization/deserialization boundaries as *moves* of
       610	    /// the [`Clock`](crate::Clock).

Resolution: reword in terms of `Party` (the module doc at party.rs:10-17 already carries the right sentence). Acceptance: the warning names `Party`.

### fresh-eyes-4: 'mint' and the 'door' metaphor in public rustdoc
- Where: crates/before/src/party.rs:872-873 (related: crates/before/src/party.rs:850, crates/before/src/lib.rs:49, crates/before/README.md:53 (derived))
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read; `grep -rnw 'mint\|mints\|door\|doors'` over the crate located the public sites and roughly two hundred private-prose uses of `door`); executed: no
- Verification: confirmed; history: no-rationale-found (e546b6d5e introduced "identity-minting door" deliberately and uses the terms throughout its message, but records no argument for the words themselves and anchors them nowhere)
- Owner-gated: no

Public rustdoc uses "mint" for constructing a value and promotes "door" to jargon without anchoring it to an identifier or defining it by contrast. At party.rs:850 the plain phrase already sits beside the coinage.

Evidence:

       850	/// Like every literal door, this *creates* identity tied to no existing handle.

       872	/// Mints identity exactly as the `u8` literal door does — a test and
       873	/// fresh-universe door ([Safety rules](crate#safety-rules)).

        49	//! // New participants fork off a live clock, never mint themselves.

Resolution: At party.rs:872-873 write "Creates identity exactly as the `u8` literal does: a constructor for tests and fresh universes ([Safety rules](crate#safety-rules))"; at party.rs:850 drop "Like every literal door,"; at lib.rs:49 write "never create themselves", then `just readme`. The private-prose uses of `door` are a separate question for the owner (below). Acceptance: `grep -rnw 'mint\|mints\|door' crates/before/src/party.rs crates/before/src/lib.rs` finds no line beginning `///` or `//!`.

### paper-fidelity-7: Party::join_all's Errors doc promises input parties back; the fold hands back coalesced unions
- Where: crates/before/src/party.rs:312-315 (related: crates/before/src/party.rs:353-366, crates/before/src/clock.rs:234-237, crates/before/src/fold.rs:31-36, crates/before/src/laws.rs:2406-2409, crates/before/src/party/tests.rs:95-105)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (the counter traced by hand on the construction below against `balanced_try_fold` and `Party::join_all`'s drain; corroborated by the committed test's doc comment at party/tests.rs:95-103, which states a four-input coalesced hand-back); executed: no
- Verification: confirmed; history: already-known (laws.rs:2406-2409 and party/tests.rs:95-103 state the coalesced hand-back; only the public prose lags)
- Owner-gated: no

The public `# Errors` prose says every input `Party` is either merged or handed back. Under aliasing, a coalesced group that fails its weight-level combine stays on the stack (fold.rs:33-36), and the closing drain (party.rs:362-366) hands that union back as one `Party` equal to no input. `Clock::join_all` phrases the same contract as regions and versions handed back, which is accurate. Reachable only when the safety rules are violated, but that is exactly the case the Errors section describes.

Evidence:

       312	    /// Returns the parties which *overlapped* and so could not be folded in,
       313	    /// dropping nothing: every input [`Party`] is either merged into `self` or
       314	    /// handed back. In case of partial error, the set of parties which are
       315	    /// absorbed vs. handed back is unspecified.

    crates/before/src/clock.rs:
       234	    /// Returns the clocks whose parties *overlapped* and so could not be folded
       235	    /// in, dropping nothing: every input's party region and version are either
       236	    /// merged into `self` or handed back. In case of partial error, the set of

    crates/before/src/laws.rs:
      2407	    /// union), never byte identity: the closing drain legitimately
      2408	    /// hands back *coalesced* groups, byte-distinct from every input,

Resolution: rephrase as clock.rs does: the returned parties are the overlapping inputs' regions, possibly coalesced into unions of inputs that were disjoint among themselves; every input's region is either merged or present in the union of the returned parties. Doc change only. Acceptance: the `# Errors` text is true of the committed `join_all_agrees_with_oracle_on_aliased_coalesced_group` case.

Construction: `let mut whole = Party::seed(); let mut p = whole.fork(); let q = p.fork();` (whole = [0,½), p = [½,¾), q = [¾,1)); `let p3 = p.dangerously_alias(); let p4 = q.dangerously_alias(); let err = whole.join_all([p, q, p3, p4]).unwrap_err();`. Counter: p enters at weight 0; q merges to A = [½,1) at weight 1; p3 enters at weight 0; p4 merges with it to B = [½,1) at weight 1; combine(A, B) fails and both stay at weight 1; the drain joins A into `whole` and fails on B, so `err == [B]`, and B equals no input.

**Nits (4), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| fresh-eyes-12 | `crates/before/src/party.rs:605-610` | Party::decode's warning is written about Clock | Restate in terms of `Party`: "Serializing a [`Party`] circumvents its otherwise compiler-enforced `!Clone` linearity ... | `evidence/sweeps/fresh-eyes.md` |
| inventory-13 | `crates/before/src/party/forks.rs:111-114` | `Party::forks(u64::MAX)` yields one share fewer than the public doc promises | add the saturation clause to the `Forks`, `Party::forks`, and | `evidence/sweeps/inventory.md` |
| recursion-7 | `crates/before/src/party/ops/split.rs:19-19` | Doc wording inverts iterative and recursive at three sites | split.rs:19 -> "The cursor form of `oracle::Party::split`"; | `evidence/sweeps/recursion.md` |
| rumors-dependence-4 | `crates/before/src/party.rs:514-520` | `dangerously_alias`'s "dropped without further use" forbids the read-only uses before's own examples and laws make | reword the Warning on `Party::dangerously_alias` and `Clock::dangerously_alias` to name the mechanism: while one copy is live ... | `evidence/sweeps/rumors-dependence.md` |

**Cross-references.** party-14, api-audit-10, and inventory-13 are the `forks(u64::MAX)` contract; party-13 (correctness class) is the 32-bit `len()` panic beside it. party-8 and paper-fidelity-7 are the same `# Errors` paragraph at party.rs:312-315 (clock-4 and crate-root-18 the clock and fold sides). party-7, api-audit-15, and fresh-eyes-12 are the `Party::decode` warning written about `Clock`; party-7 and api-audit-18 the `n`/`k` drift. party-5 and fresh-eyes-4 share the party.rs:15 and :872 "mint" sites. party-3 and recursion-7 both name split.rs:19's "recursive form". party-20's two `IdLeafCursor`s are a decided drop recorded only in an agent note. rumors-dependence-4's `dangerously_alias` overshoot is the one entry here from the dependence sweep.

## Crate root and public types: version core (version.rs, own.rs, ticks.rs, hull_traffic.rs)

19 findings (0 high, 1 medium, 11 low, 7 nit). Full records: `evidence/partitions/version-core.md`, `evidence/sweeps/api-audit.md`, `evidence/sweeps/fresh-eyes.md`, `evidence/sweeps/inventory.md`.

### fresh-eyes-2: Version, Party, and Clock decode do not state the whole-input contract and lack # Errors sections
- Where: crates/before/src/version.rs:1095-1110 (related: crates/before/src/clock.rs:765-787, crates/before/src/party.rs:602-623, crates/before/src/error.rs:77-85; the complete sections to copy from: crates/before/src/span/wire.rs:79-89, crates/before/src/version/rank.rs:435-445, crates/before/src/version/ranked.rs:241-247)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (the sweep's run2.log lines 46-50: `Version::decode(value ++ extra) = Err(TrailingBits)`, `Clock::decode(clock ++ version) = Err(TrailingBits)`, `Version::decode(&[]) = Err(Truncated)`, `Clock::decode(party bytes) = Err(Truncated)`; this pass read the three bodies: each calls `read_to_end` then `require_marker_padding`); executed: yes, by the sweep's scratch run, matched to its log
- Verification: reframed: the sweep's resolution also named `Span::decode`, which already carries a complete `# Errors` section (span/wire.rs:79-89); the gap is exactly the three sites here; history: no-rationale-found
- Owner-gated: no for the documentation; the prefix-decoding entry is an owner-gated suggestion

`Version::decode`, `Party::decode`, and `Clock::decode` take a `Read`, consume it to end, and reject any byte past the value as `Decode::TrailingBits`; none of the three docs says so, and none has a `# Errors` section, while `Span::decode`, `Rank::decode`, and `Ranked::decode` carry complete ones. The whole-input rule is discoverable today only on the `Decode::TrailingBits` variant.

Evidence:

      1095	    /// Decodes a [`Version`] from a reader of canonical bytes.
      1096	    ///
      1097	    /// # Complexity
      1098	    ///
      1099	    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_decode.html"))]
      1100	    ///
      1101	    /// Strict validation is one pass over the stream, and the result reuses the read buffer.
      ...
      1110	    pub fn decode<R: Read>(mut reader: R) -> Result<Self, Decode> {
      1111	        let mut buf = Vec::new();
      1112	        reader.read_to_end(&mut buf).map_err(Decode::Io)?;

        79	    /// # Errors
        80	    ///
        81	    /// - [`Decode::Truncated`]: the bytes end before the composite does —
        ...
        89	    /// - [`Decode::Io`]: the reader itself fails.

Resolution: Add a `# Errors` section to `Version::decode`, `Party::decode`, and `Clock::decode` in the form `Span::decode` uses, and state in each that the reader is read to end and must hold exactly one value. Owner-gated suggestion, separately: a prefix-decoding entry (for example `decode_prefix(&[u8]) -> Result<(Self, &[u8]), Decode>`) for callers who frame values without a length prefix; the borsh feature already relies on the encodings being prefix-free. Acceptance: `grep -c '# Errors' crates/before/src/version.rs` is at least 1 and each of the three `decode` docs names `TrailingBits` for spurious input.

### version-core-7: `span_all` advises against an operation that does not exist and links `span` to `Version::meet`
- Where: crates/before/src/version.rs:603-604 (related: crates/before/src/version.rs:526-527, 638-651)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'Version::meet)ing'` matches only 603; `git blame -L 603,604` attributes both lines to b3f09baa0, the docs-pass commit; 526-527 is the `meet_all` sentence it was copied from); executed: no
- Seen by: structure, prose, correctness; refutation: confirmed; history: slip (copy of `meet_all`'s sentence with the verb swapped and the link target left)
- Owner-gated: no

`span` returns a `Span`, not a `Version`, so there is no "iteratively spanning versions one-at-a-time" for a caller to prefer this over, and the link text `span` lands on `Version::meet`. Public rustdoc serves the library user; a comparison target that cannot be written plus a wrong link cost the reader the contract they came for.

Evidence:

       603	    /// Prefer this to iteratively [`span`](Version::meet)ing [`Version`]s
       604	    /// one-at-a-time, as it is more efficient.

Resolution: name the true alternative, which 638-651 already describes: "Prefer this to computing [`meet_all`](Self::meet_all) and [`join_all`](Self::join_all) separately: one balanced fold carries both endpoints and reads each input once." Drop the `Version::meet` link. Acceptance: the sentence names an operation a caller could write instead, and every intra-doc link in the `span_all` doc resolves to the item its text names.

### version-core-10: `join_view`'s doc calls the byte-compare rung `O(1)`; the codec's own ladder prices it by the shared prefix
- Where: crates/before/src/version.rs:856-858 (related: crates/before/src/version.rs:385-390, 908-911; crates/before/src/codec/bits.rs:22-26, 414-420)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (bits.rs:419 is `a.ptr_eq(b) || a.as_raw_slice() == b.as_raw_slice()`; bits.rs:22-24 prices a miss as "an early-exiting byte compare over the operands' shared prefix"; version.rs:390 prices the same rung correctly for `distance`); executed: no
- Seen by: prose, claims; refutation: confirmed, with the refinement that slice `==` compares lengths first, so a miss on different-length streams is `O(1)` and the `O(min(|a|,|b|))` case is equal-length streams sharing a long prefix; history: the sentence entered in 58a37d80d when the doors became private methods; an inaccuracy, not a decision
- Owner-gated: no

A private complexity statement that contradicts the module it delegates to is a maintainer trap in a crate that treats complexity claims as hard guarantees; the `distance` comment twenty lines away already has it right, so the two should agree.

Evidence:

       856	    /// Before the merge sweep, two `O(1)` short-circuits settle the cases
       857	    /// canonical form makes immediate: trivial equality (`a ∨ a = a`, a no-op,
       858	    /// decided by a byte compare of the two unique streams) and the lattice

Resolution: "two short-circuits: canonical equality (clone identity in `O(1)`, else one early-exiting byte compare bounded by the shorter operand and absorbed by the sweep it precedes) and the `O(1)` lattice identity `0 ∨ v = v`"; `meet_view` (908-911) inherits the wording through "The dual short-circuits apply". Acceptance: the `join_view`/`meet_view` docs no longer call the byte compare `O(1)`, and their pricing matches bits.rs and the `distance`/`lag` comments.

### version-core-12: `span_refs`'s doc claims its rungs match `join_refs`'s order; `join_refs` tests the empty rungs the other way round
- Where: crates/before/src/version.rs:950-952 (related: crates/before/src/version.rs:868-871, 889-899, 916-919, 934-945, 968-983)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the five ladders: `join_refs` tests `b` empty at 893 before `a` at 896; `meet_refs` at 938/941 and `span_refs` at 974/979 test `a` first; `join_view` at 868/871 mirrors `join_refs` and `meet_view` at 916/919 mirrors `meet_refs`); executed: no
- Seen by: prose; refutation: confirmed (value-irrelevant: after the equal rung at most one operand is empty); history: the claim was false from its first commit (70eb67ab1); the join/meet asymmetry has a readable logic (each `_view` tests its no-op rung first, and each `_refs` mirrors its own `_view`), so aligning `join_refs` would break the join_view/join_refs lockstep
- Owner-gated: no

A maintenance directive that the code already violates teaches the next reader to distrust the others; the fix that keeps every stated invariant is to restate the doc, not to reorder the rungs. (If version-core-11 lands, the `_view` forms vanish and the three `_refs` ladders can simply share one order.)

Evidence:

       950	    /// The first three rungs are [`meet_refs`](Self::meet_refs) and
       951	    /// [`join_refs`](Self::join_refs)'s in the same order — keep the three in
       952	    /// lockstep — each settling both endpoints at once, and the equal rung's

Resolution: "the same three rungs; after the equal rung at most one operand is empty, so the two empty rungs' order is free", and drop "keep the three in lockstep". Acceptance: no doc in the file claims an order the three `_refs` functions do not share.

### version-core-14: `Version::decode` and the text/literal constructors have no `# Errors` section; `Rank::decode` and `Ranked::decode` carry itemized ones
- Where: crates/before/src/version.rs:1095-1110 (related: crates/before/src/version.rs:1440-1459, 1461-1478, 1480-1511; crates/before/src/version/ticks.rs:166-190, 199-213; crates/before/src/version/rank.rs:435-445; crates/before/src/version/ranked.rs:241; crates/before/src/party.rs:605-623, out of partition)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '# Errors' crates/before/src`: 13 sites in clock.rs, span.rs, party.rs, span/wire.rs, ranked.rs, rank.rs, admit.rs, none in version.rs, ticks.rs, or own.rs; rank.rs:435-445 itemizes `Truncated`/`TrailingBits`/`NotCanonical`/`Io` with triggers; literal.rs:18 `leaf` returns `BitsBuf`, so `TryFrom<u64>` at 1473-1478 never fails); executed: no
- Seen by: prose; refutation: confirmed; history: accretion across two authoring eras (the section entered with the rank wire form, f0f3a2aed; version.rs never carried one)
- Owner-gated: no

Documentation altitude: hazards belong under uniform `# Panics`/`# Errors` sections so a user finds every return arm; the crate adopted that convention for the rank doors, so the version doors' omission reads as a gap. `FromStr for Version`, `TryFrom<(u64, T, S)>`, `FromStr for Ticks`, and `TryFrom<&Ticks> for u64` state rejections in running prose or not at all, and `TryFrom<u64> for Version` is spelled fallible while never failing, with no note saying so.

Evidence:

      1095	    /// Decodes a [`Version`] from a reader of canonical bytes.
      1096	    ///
      1097	    /// # Complexity
      1098	    ///
      1099	    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_decode.html"))]
      1100	    ///
      1101	    /// Strict validation is one pass over the stream, and the result reuses the read buffer.

Resolution: add `# Errors` to `Version::decode` mirroring `Rank::decode`'s shape (`Decode::Truncated`, `Decode::NotCanonical`, `Decode::TrailingBits`, `Decode::Io`, each with its trigger drawn from `validate_prefix` and `require_marker_padding`); `# Errors` naming `Parse::Syntax`/`Parse::NotCanonical` on the two `FromStr` impls and the node literal; on `TryFrom<u64>`, "Never fails; the fallible spelling lets leaves compose with node literals, whose `TryFrom<T, Error = Parse>` bound it satisfies." Acceptance: every fallible public entry in version.rs, own.rs, and ticks.rs has an `# Errors` section naming each variant it can return, and the infallible `TryFrom<u64>` says so.

### version-core-17: Register and vocabulary rules that postdate the prose: `mints`, moralized and dated adjectives, unanchored `door`, em-dashes in `//` comments
- Where: crates/before/src/version.rs:1624-1625 (related: `door`: version.rs:1003, hull_traffic.rs:13, tests.rs:1353, 1357, 1516, 1590, 1641, 1644, 1669, 1681; `honest`/`historical`/`truthful`/`real`: tests.rs:1107, 1349, 1355, 1392-1393, ticks/tests.rs:84; em-dashes on `//` lines: version.rs 25, 123, 156, 385, 386, 429, 430, 640, 646, 1122, 1351, 1622, 1717; own.rs 77, 78; own/tests.rs 40, 62; tests.rs 454, 455, 536, 788, 991, 1005, 1157, 1165, 1204, 1349, 1350, 1355, 1651, 1669, 1671, 1799, 1869, 1902)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep per file: `mint` once in the partition (version.rs:1624) and 43 times in crates/before/src; `honest`/`historical`/`truthful` 4 lines in the partition and `honest` 146 crate-wide; `door(s)` 10 lines in the partition and 208 crate-wide, with no definition in lib.rs, any AGENTS.md, or validation_index.rs; em-dashes on `//` (non-doc) lines 35 in the partition and 374 across 76 files crate-wide; no assert, expect, or panic message in the partition carries an em-dash); executed: no
- Seen by: prose ([17], [29], [30], [31]); refutation: [17], [29], [31] confirmed, [30] reframed as crate-wide; history: each rule entered the owner's chezmoi-managed doctrine on 2026-08-10 (55281a3), after most of this prose; `door` is listed in the writing-style lexicon as the plain-term case "entry point"
- Owner-gated: yes (a crate-wide sweep, and `door` needs a ruling: define once or replace)

Four tells the review standard names, all present in the partition, none partition-local: "mint" for constructing a value is banned outright; "honestly reachable", "honest coordinate", "real capacity", "real groups" moralize code and "the historical numerator path" dates it (the file's own present-tense name is "backend-arm", 1362, 1414); "door" is a metaphor promoted to jargon with no definition site; and the owner's comment-register rule asks for colons or spaced double-hyphens in `//` comments, with true em-dashes reserved for rendered prose. Partition-local fixes would leave the crate inconsistent, so the disposition is one mechanical sweep plus one ruling on `door`.

Evidence:

      1622	// operand type — a `Span`, not a `Version` — so the family has no assigning
      1623	// form (nothing of the receiver's type to assign back) and no owned-operand
      1624	// strategy: every cell reads both operands in place and mints the endpoints
      1625	// owned, exactly as the named method does.

      1349	// The wide arm is honestly reachable only past the backend's capacity —

Resolution: (1) "returns the endpoints owned"; (2) "reachable only past the backend's capacity", "the production coordinate is 2⁶⁴ − 64 bits", "whose capacity is astronomically higher", "the backend-arm path", "the exact-size length stays exact", "half a GiB of groups"; (3) define `door` once (lib.rs or AGENTS.md: a public method or trait impl through which a value enters or leaves the crate) or replace per site with "public method" / "codec entry" / "fold"; (4) on `//` lines only, replace ` — ` with `: ` or `; ` where it introduces an apposition and with a parenthetical or a new sentence where it brackets one. Book all four as one crate-wide pass. Acceptance: `grep -n -i 'mint\|honest\|historical\|truthful'` over the partition returns nothing; a single definition site for `door` exists or the term is gone; `grep` for `—` on lines matching `^\s*//[^/!]` over the partition returns nothing.

### version-core-18: "so it is not monotone under `<=`" reads as a claim about projection, which the same paragraph calls a lattice homomorphism
- Where: crates/before/src/version.rs:1675-1676 (related: crates/before/src/version.rs:1669-1674; crates/before/src/version/tests.rs:2161-2166)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both sites; 1673-1674 state projection is "a homomorphism of both join and meet", hence monotone); executed: no
- Seen by: prose; refutation: confirmed; history: both sentences written in 472d646e2 with the same construction; a grammatical slip
- Owner-gated: no

The grammatical subject of "it is not monotone" is "Projection", so the sentence asserts a falsehood two lines after the mathematics that refutes it; the intended claim is that `min_ticks` is not monotone under `<=` (a sub-version can have a larger floor). The test doc at tests.rs:2161 ("Projection can *raise* `min_ticks`: it is not monotone under `<=`.") has the same construction, and test docs are held to "must be accurate".

Evidence:

      1675	// Projection can still raise `min_ticks` (carving one broad tick into
      1676	// disjoint peaks), so it is not monotone under `<=`.

Resolution: name the subject: "Projection can still raise `min_ticks` (carving one broad tick into disjoint peaks): `min_ticks` is not monotone under `<=`, though projection itself is." Same edit at tests.rs:2161. Acceptance: neither sentence can be read as "projection is not monotone".

### version-core-22: `Ticks` type doc: unclosed parenthesis, a semicolon splice with a verb-agreement slip, the text-I/O bound stated twice, and "then ascending"
- Where: crates/before/src/version/ticks.rs:18-30 (related: crates/before/src/version/ticks.rs:41-48, 91-92)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the sites); executed: no
- Seen by: structure ([9]), prose ([27]); refutation: confirmed; history: (a) and (d) are the owner's hand edit 925b9973 (2026-08-19), which replaced a balanced parenthesis with the current unclosed one and wrote "then ascending"; (c) is accretion across 56a08f90, 3bba6cbb, and the same hand edit; none deliberate
- Owner-gated: no (syntax slips; the owner authored them, the fix is uncontroversial)

Four prose defects in the first thing a user of `Ticks` reads: (a) line 19 opens "total (" and never closes it, running three clauses together; (b) 26-29 splice with "; and consumed by" and mismatch "each of which take"; (c) 41-43 and 45-48 state the same text-I/O bound with the same reason, the second paragraph alone carrying the space term; (d) 91-92 "least significant first, then ascending" says one thing twice.

Evidence:

        18	/// Event counts have no ceiling, so the count is unbounded rather than any
        19	/// fixed-width integer: every conversion *into* it is total ([`From`] on
        20	/// unsigned machine integers, and every conversion *out* is explicit
        21	/// about width: `TryFrom<&Ticks> for u64` answers the machine-range case
        ...
        26	/// This type is produced by [`Version::min_ticks`](crate::Version::min_ticks);
        27	/// and consumed by [`Version::ticks`](crate::Version::ticks),
        28	/// [`Party::ticks`](crate::Party::ticks), and
        29	/// [`Clock::ticks`](crate::Clock::ticks), each of which take `impl
        ...
        41	/// Construction is `O(1)`; comparison and hashing `O(‖n‖)`; addition `O(‖a‖ +
        42	/// ‖b‖)`, `Sum` `O(N)`; text I/O is superlinear but subquadratic in the count's
        43	/// width (because it requires decimal conversion).
        ...
        91	    /// The count's base-2^64 "digits", i.e. its *limbs*, least significant
        92	    /// first, then ascending.

Resolution: (a) close the parenthesis after "unsigned machine integers)" and start a new sentence for the conversions out; (b) "produced by `min_ticks` and consumed by `Version::ticks`, `Party::ticks`, and `Clock::ticks`, each of which takes `impl Into<Ticks>`"; (c) drop the text-I/O clause from 41-43 and keep 45-48; (d) "least significant first". Acceptance: the doc parses as English with balanced parentheses and states each bound once.

### version-core-25: The `From<u8..u128> for Ticks` impls carry no rustdoc; the doc sits on the macro, and `From<usize>` alone is documented
- Where: crates/before/src/version/ticks.rs:151-164 (related: crates/before/src/version/ticks.rs:192-197)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (the `///` line precedes `macro_rules!`; the expansion at 155-159 carries no `#[doc]`; rustdoc attaches a doc comment to the item it precedes, here the private macro); executed: no (a docs build was not permitted)
- Seen by: prose; refutation: confirmed; history: the macro and its doc line date to 56a08f90; no commit addresses where the doc lands
- Owner-gated: no

The type doc promises `From` on unsigned machine integers is total and `O(1)`, so each impl should say so where the user sees it, as the `usize` one already does.

Evidence:

       151	/// A count from a machine integer: total, `O(1)`.
       152	macro_rules! ticks_from_unsigned {
       153	    ($($t:ty),*) => {
       154	        $(
       155	            impl From<$t> for Ticks {
       ...
       192	/// A count from a machine size: total, `O(1)`.
       193	impl From<usize> for Ticks {

Resolution: move the doc inside the expansion (`$( #[doc = "A count from a machine integer: total, `O(1)`."] impl From<$t> for Ticks { ... } )*`), or fold `usize` into the macro so all six are documented identically. Acceptance: every `From<_> for Ticks` impl shows the same one-line doc in rendered rustdoc.

### version-core-35: The deep-spine pin's assert message points at "the query depth-guard size prose", which no longer exists
- Where: crates/before/src/version/tests.rs:2375-2401 (related: crates/before/src/version/skyline/query/integral.rs:172-174)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for `depth-guard`, `depth guard`, `bits per level`, `depth-derived` over crates/before/src and tests outside this file: no such prose; the nearest live consumer is integral.rs:172-174, "each digit position of a window is a depth the stream's topology paid at least one bit for"; `git show --stat 6323d6677` touched query.rs and tests.rs); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (6ea3a18a2 added the pin to guard a `# Panics` sentence in query.rs; 6323d6677 widened the exponent to u64, excised that prose "everywhere it propagated", and re-denominated the doc comment as the grammar's exchange rate, but missed the assert message)
- Owner-gated: no

An assert message is prose the maintainer reads at the moment of failure; a pointer to an argument that cannot be found leaves them without the derivation they need (no ghost references). The pin protects a live premise (integral.rs prices each depth at "at least one bit", which the 3-bit rate over-satisfies) but names it by a name that does not exist.

Evidence:

      2381	/// the grammar ever admits a cheaper per-level spelling, this pin moves and any
      2382	/// prose pricing depth in input bytes must be re-derived with it.
      ...
      2399	        "the deep spine's marginal level cost moved off 3 bits: re-derive \
      2400	         the query depth-guard size prose from the new grammar"

Resolution: name the actual consumer in both the doc and the message: the `query::integral` funding argument's premise that every depth is paid at least one stored bit (which a 3-bit rate implies); or, if no prose depends on the 3-bit figure, restate the pin as pinning the grammar's per-level cost and drop the re-derive instruction. Acceptance: the assert message and doc cite a prose location that exists, by module and premise, or the pin's purpose is restated without a pointer.

### api-audit-8: `# Errors` sections cover about half the fallible public entries
- Where: crates/before/src/version.rs:1095-1110 (related: party.rs:558-574, party.rs:602-623, party.rs:763-789, party.rs:863-908, clock.rs:742-756, clock.rs:765-787, clock.rs:925-952, clock.rs:971, version.rs:1031-1047, version.rs:1071-1093, version.rs:1440-1511, version/ranked.rs:185, version/ranked.rs:234, version/ticks.rs:185-190, version/ticks.rs:199-213)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '# Errors' crates/before/src` excluding test and instrument modules: clock.rs:199,232,303,345; party.rs:280,310; span.rs:120; version/rank.rs:403,435; version/ranked.rs:241; span/wire.rs:52,79. Cross-checked against every `Result`-returning `pub fn`, `FromStr`, and `TryFrom` in the public defining files); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The section exists for `Span::new`, `Clock::join/join_all/sync/sync_all`,
`Party::join/join_all`, `Span::encode_to/decode`, `Rank::encode_to/decode`, and
`Ranked::decode`, and it is absent from `Version::decode`, `Party::decode`,
`Clock::decode`, the `encode_to` trio, `Version::encode_rank_to`,
`Ranked::encode_to/encode_rank_to`, `TryFrom<&Ticks> for u64`, and every
`FromStr`/`TryFrom` impl (which describe rejection in prose without the
section). The decode trio are the entries most likely to meet untrusted bytes.

Evidence:

      1095	    /// Decodes a [`Version`] from a reader of canonical bytes.
      1096	    ///
      1097	    /// # Complexity
      1098	    ///
      1099	    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_decode.html"))]
      1100	    ///
      1101	    /// Strict validation is one pass over the stream, and the result reuses the read buffer.

Resolution: add `# Errors` to the listed sites, naming the `Decode`/`Parse`/`TooWide` variants each can return; `Rank::decode` (rank.rs:435-445) and `Span::decode` (span/wire.rs:79-85) are the variant-by-variant template, and `Rank::encode_to` (rank.rs:403-406) the one-line template for writers. Acceptance: every `pub fn` or trait impl returning `Result` carries a `# Errors` section.

### fresh-eyes-7: Projection docs spell an owned-operand `/` that does not compile
- Where: crates/before/src/version/own.rs:12 (related: crates/before/src/span.rs:69-71; correct spellings at crates/before/src/version/own.rs:1, crates/before/src/version.rs:703, crates/before/src/span.rs:47, crates/before/src/version.rs:1703)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (the sweep's check1.log lines 64 and 77: `error[E0369]: cannot divide 'Version' by '&before::Party'` and `cannot divide 'Span<'_>' by '&before::Party'`; this pass read the only `Div<&Party>` impl for versions, on `&'a Version` at version.rs:1703); executed: yes, by the sweep's compile probes, matched to its log
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`OwnVersion`'s first sentence writes `v / &p` and the `Span` algebra section writes `s / &p`, but `Div<&Party>` exists only for `&Version` and `&Span`. The module doc one line above (own.rs:1) and the `Span` table row (span.rs:47) have it right.

Evidence:

         1	//! [`OwnVersion`]: the lazy projection view `&v / &p`.
        ...
        12	/// The projection of a [`Version`] by a [`Party`]: `v / &p`.

        69	/// Projection applies [`Version::project`] pointwise to the low and high ends
        70	/// of the span: for a given [`Span`] `s`, `s / &p` yields the span `(lo / &p)
        71	/// <= (hi / &p)`.

Resolution: Write `&v / &p` at own.rs:12 and `&s / &p` yielding `(&lo / &p) <= (&hi / &p)` at span.rs:70-71. Acceptance: every spelling of the projection operator in rustdoc takes a borrowed left operand.

**Nits (7), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| version-core-3 | `crates/before/src/version.rs:320-321` | Small typographic and consistency slips across the partition | (a) delete one blank; (b) `= a`; (c) "so materialization is kept explicit"; (d) `&v / &p` ... | `evidence/partitions/version-core.md` |
| version-core-6 | `crates/before/src/version.rs:581-581` | Ghost name `alice` in the `span` example | `// concurrent to a's line` | `evidence/partitions/version-core.md` |
| version-core-30 | `crates/before/src/version/tests.rs:840-855` | Hand-maintained pair counts in test names, docs, and a const doc | name the bounds (`const RANK_CMP_SWEEP_PAIRS: u32 = 25_000;` and its wide-arm twin), cite them by name in the docs ... | `evidence/partitions/version-core.md` |
| api-audit-17 | `crates/before/src/version.rs:603-604` | Doc-link and spelling slips in public rustdoc | fix the link target to `Version::span` and the five typos | `evidence/sweeps/api-audit.md` |
| api-audit-21 | `crates/before/src/version.rs:1473-1478` | TryFrom<u64> for Version can never fail, and its docs do not say so | one sentence in the impl doc: "Never fails; the `Result` is the shape the nested `(n, left ... | `evidence/sweeps/api-audit.md` |
| fresh-eyes-11 | `crates/before/src/version.rs:603-604` | Typos and wrong link targets in public rustdoc | version.rs:603 link to `Version::span`; forms.rs:232 `t` for `e`, forms.rs:234 `after(s) & until(t)`; clock.rs:341 "iteratively" ... | `evidence/sweeps/fresh-eyes.md` |
| inventory-10 | `crates/before/src/version.rs:603-604` | Two doc slips: `span_all` links "span" to `Version::meet`; "seach" typo | link to `Version::span`; "seach" to "each" and "endpoint" to | `evidence/sweeps/inventory.md` |

**Cross-references.** version-core-7, api-audit-17, fresh-eyes-11, and inventory-10 are the `span_all` link to `Version::meet` at version.rs:603 (fresh-eyes-11 and api-audit-17 also collect the same typos that span-causally-5 and clock-2 report at their homes). version-core-14, api-audit-8, and fresh-eyes-2 are the decode trio's missing `# Errors`; api-audit-21 is the never-failing `TryFrom<u64>` inside it. version-core-3(d) and fresh-eyes-7 are the `v / &p` spelling at own.rs:12. version-core-18 and api-audit-9 read the same "not monotone" sentence at version.rs:1675-1676. version-core-5 (another class) is the subadditivity lemma the folds' space bounds rest on, derived only in test prose.

## Crate root and public types: rank (rank.rs, rank/num.rs, ranked.rs)

12 findings (0 high, 0 medium, 7 low, 5 nit). Full records: `evidence/partitions/rank.md`.

### rank-1: Hand-maintained literals and counts in prose that nothing enforces
- Where: crates/before/src/version/rank.rs:36-39 (related: rank.rs:543, rank.rs:920-921, rank.rs:623, rank.rs:799-804, crates/before/src/version/rank/num.rs:12, crates/before/src/version/rank/num/tests.rs:189, crates/before/src/version/tests.rs:1187-1190)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for each literal; hand arithmetic for the assert message: `vec![1, 0, 5, 0]` strips to three limbs with top limb 5, three bits wide, so 2·64 + 3 = 131 bits; the only enforced number is the `encoded_bits <= input_bits` assert at version/tests.rs:1240-1241); executed: no
- Seen by: prose, correctness, claims, structure; refutation: confirmed (the `dsi-bitstream 0.10` item dropped: it names the minor line evaluated); history: no rationale found (0.56 entered in 02f6180f as a design record quoting the measurement)
- Owner-gated: no

The module doc argues a design choice from a measured constant (0.56) that no test enforces (the pin is 1.0), the provenance test's doc lists five measured ratios its body never checks, `Three clauses` introduces a predicate with two clauses per operand, `The four reference forms` counts `impl Add` blocks by hand, `~604 MB` is written at four sites with its derivation (`9/64 · exp` bytes) only in the wasm32 guest, and a test assert message states a false width. Principle 5: no hand-maintained counts or dated measurement literals; a number that matters lives in a mechanically enforced place the prose cites by name.

Evidence:

        36	//!    width *again* and drive the worst committed provenance family
        37	//!    (the lone wide counter, measured at 0.56 encoded bits per
        38	//!    packed input bit) up against the 1.0-per-family
        39	//!    provenance-linearity pin; delta's `N + O(log N)` is what keeps

       543	/// Three clauses, all width facts: each exponent gap must fit the

       920	// what makes [`Version::distance`](crate::Version::distance) a metric. The four
       921	// reference forms mirror [`Base`]'s own `Add` matrix so callers need not place

    (num/tests.rs)
       189	    assert!(wide.is_wide(), "129 bits exceeds the 96-bit test ceiling");

Resolution: Either pin each family's ratio in `rank_encoding_size_is_provenance_linear` as a ceiling with slack (wide counter at or under 0.7, say) and have rank.rs:37-38 and version/tests.rs:1187-1190 cite the pin by name, or delete the raw figures and keep only the enforced 1.0 statement. Rewrite 543 as "Two clauses per operand, both width facts"; 920-921 as "The reference forms mirror Base's own Add matrix". Derive ~604 MB once in num.rs's module doc ("9/64 · 2^32 bytes") and refer to "the fraction-form capacity crossing" at the other three sites. Replace the assert message with one computed from the constants: `format!("131 bits exceeds the {TEST_CEILING_BITS}-bit test ceiling")`, or assert `wide.bits() > TEST_CEILING_BITS`. Acceptance: no numeric literal in the partition's prose duplicates an enforced constant or an unenforced measurement; `python3 -c 'print(((5<<128)+1).bit_length())'` prints 131 and matches the message.

### rank-5: The exp field doc states the bound for one construction path
- Where: crates/before/src/version/rank.rs:255-257 (related: rank.rs:577-582, rank.rs:1020-1028, rank.rs:788)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (rank.rs:788 `let exp = frac_len;` is the decoder's exponent, counted from groups read; the two-case bound is spelled at 579-582 and 1023-1027); executed: no
- Seen by: prose, correctness; refutation: confirmed; history: deliberate but expired (18206f21 wrote the field doc when the tree fold was the only producer; f0f3a2ae added the decoder and 6323d667 re-derived its bound in the commit message but left the field doc)
- Owner-gated: no

The field is the invariant's home, and both panic arguments (accumulate's and sum_ranks') rest on it, yet it states the bound for version-derived ranks only while a decoded rank's exponent is bounded by fraction bits read and `Add`/`Sum` carry the operands' maximum. The full two-case bound is then re-derived at two consumer sites instead.

Evidence:

       255	    /// The (binary) exponent of the denominator `2^exp`. Bounded by the
       256	    /// event tree's depth, since each level halves the interval width.
       257	    exp: u64,

       580	/// any honest exponent: a decoded exponent is counted from fraction bits
       581	/// actually read, under 2³⁵ from a whole 32-bit address space, and a
       582	/// version-derived exponent is bounded by its tree's stored bit length.

Resolution: On the field: "Bounded by bits already resident: a version-derived exponent by its tree's stored bit length (each level halves the interval), a decoded exponent by the fraction bits actually read, and a sum by its operands' maximum; under 2^35 on a 32-bit target, which keeps every usize-indexed digit position (suanpan's documented panic) unreachable." Reduce 579-582 and 1023-1027 to a citation of the field's bound. Acceptance: the two-case bound appears once, on `exp`; the two consumers cite it.

### rank-6: checked_sub and saturating_sub docs name a parameter the signatures do not have
- Where: crates/before/src/version/rank.rs:278-298 (related: rank.rs:332-337, rank.rs:354)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (lines 278, 298, 332-333, 337, 354 read); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (14d36c62 named it `rhs`; 8d8a06e2 renamed the signature to `other` and left the docs)
- Owner-gated: no

Both public docs describe `self - rhs` while the parameter is `other`, and saturating_sub's doc says "handled arm by arm" where "arm" everywhere else in this module means a `Num` storage arm.

Evidence:

       278	    /// The difference `self - rhs`, or [`None`] when `rhs` exceeds `self`.
      ...
       298	    pub fn checked_sub(&self, other: &Rank) -> Option<Rank> {

       332	    /// The difference `self - rhs`, or [`Rank::ZERO`] when `rhs` exceeds
       333	    /// `self`.
      ...
       337	    /// remaining" rather than be handled arm by arm.

Resolution: Rename the parameter to `rhs` (matches `Add` and the docs; parameter names are not part of the API) or change the docs to `other`; replace "handled arm by arm" with "matched on". Acceptance: parameter names in the two signatures equal the names in their docs; "arm" in rank.rs refers only to `Num`'s arms.

### rank-9: Rank::decode's "# Decoded size" section describes the encoded size and cites text that lives on encode
- Where: crates/before/src/version/rank.rs:428-433 (related: rank.rs:372-376)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: the suffix-safety paragraph is at 372-376 on `encode`; nothing above 428 in `decode`'s doc mentions it); executed: no
- Seen by: prose, structure, correctness, claims; refutation: confirmed; history: no rationale found (added fresh on `decode` in 8d8a06e2, cross-reference dangling at birth)
- Owner-gated: no

A first-time reader of `decode` finds a heading about decoded size, a fact about encoded size, and a dangling "above". The 9/8 bound is a property of the encoding and belongs on `encode` or the type's `# Complexity`, which already introduces `‖r‖`.

Evidence:

       428	    /// # Decoded size
       429	    ///
       430	    /// The serialized representation of a [`Rank`] is at most `9⁄8 · ‖r‖ +
       431	    /// O(log ‖r‖)` bits: one bit per integral bit, nine bits per eight
       432	    /// fractional bits (this is required to keep distinct ranks' encodings
       433	    /// prefix-free, providing the above generalized suffix-safety).

Resolution: Move the bound onto `Rank::encode` after its suffix-safety paragraph as "# Encoded size", or into the type-level `# Complexity` beside the `‖r‖` definition; delete the section from `decode`. Acceptance: `decode`'s doc has no size section; the 9/8 bound appears once, following the suffix-safety text it references.

### rank-12: from_num's doc names the decoder as a producer it does not serve
- Where: crates/before/src/version/rank.rs:514-519 (related: rank.rs:813-817)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'from_num(' crates/before/src` lists exactly rank.rs:511, 609, 1049 as callers; the decoder constructs `Rank { num, exp }` directly at 817 behind the debug_assert at 813-816); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (inaccurate from birth: the decoder already bypassed normalization one commit before 79a944ab wrote this doc)
- Owner-gated: no

A reader auditing the canonical-form invariant (structural `Eq`/`Hash` rest on it) is told there is one normalization funnel when the decoder is a second, independent guarantor of the same invariant, relying on strict minimal packing. That bypass is deliberate and deserves to be named rather than hidden by an inaccurate list.

Evidence:

       516	    /// The shared normalization every raw `(numerator, exponent)`
       517	    /// producer — the folds, the decoder, the accumulator readout — lands
       518	    /// through, which also re-dispatches the stripped numerator onto its
       519	    /// canonical arm.

       817	    Ok(Rank { num, exp })

Resolution: "The shared normalization the folds and the accumulator readout land through, which also re-dispatches the stripped numerator onto its canonical arm. The decoder is the one producer that bypasses it: strict minimal packing already guarantees an odd numerator whenever `exp > 0` (its debug_assert states the premise), so it constructs the normalized value directly." Acceptance: the doc's producer list equals the call graph and the decoder's bypass is stated at `from_num` or at line 817.

### rank-19: Public rustdoc names internals and privately defined terms
- Where: crates/before/src/version/rank.rs:879 (related: rank.rs:884, rank.rs:1064-1069, crates/before/src/version/ranked.rs:243, ranked.rs:376)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read; "magnitude class" is defined only in the `//` comment at 884, invisible to rustdoc readers of 879; "genre" has no definition in error.rs per the history pass's grep); executed: no
- Seen by: prose; refutation: confirmed ("schoolbook long division" dropped as an established term); history: no rationale found (879 and 376 are owner-authored, 2efff149 and b5a81583)
- Owner-gated: no

Public `# Complexity` and `# Errors` sections use maintainer vocabulary: "magnitude classes" (defined in a private comment), "the big-integer backend's capacity", "One fused signed rank co-sweep", "Each component's own genres". AGENTS.md: public rustdoc must not refer to functionality invisible to someone not reading the source. The user-relevant facts are plain: comparison is O(1) when the two ranks' `floor(log2)` differ; decimal rendering is superlinear and, on 32-bit targets past ~2^32-bit numerators, quadratic; `Ranked` comparison is one linear walk with no `Rank` built; the error variants are those of `Rank::decode` and `Version::decode`.

Evidence:

       879	/// Unequal magnitude classes settle in `O(1)`:

      1064	/// Superlinear, subquadratic in the rank's width: decimal conversion. (A
      1065	/// numerator wider than the big-integer backend's capacity — reachable
      1066	/// only on 32-bit targets, from hundreds of megabytes of decoded input —
      1067	/// renders by schoolbook long division instead, quadratic in the width:

    (ranked.rs)
       243	    /// Each component's own genres ([`Rank::decode`]'s and
      ...
       376	/// One fused signed rank co-sweep over the two viewed versions:

Resolution: 879 "Ranks whose integer parts of log2 differ settle in O(1):"; 1064-1069 "Superlinear, subquadratic in the rank's width (decimal conversion); on 32-bit targets, numerators above ~2^32 bits, reachable only from hundreds of megabytes of decoded input, render quadratically."; ranked.rs:376 "One walk over both versions, no Rank materialized:" (see rank-33 for the bound itself); ranked.rs:243 "Each component's own variants". Acceptance: no public doc in the partition uses "backend", "magnitude class", "co-sweep", "signed", or "genre" without defining it in the same public doc.

### rank-24: "historical" as dated rationale at five declaration sites, once explaining a metered no-op shift whose live rationale exists only in git
- Where: crates/before/src/version/rank/num.rs:206-222 (related: num.rs:42-44, num.rs:281, num.rs:319-320, num.rs:405-407, crates/before/src/version/rank.rs:946-947, crates/before/src/version/tests.rs:1393)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n historical` over the partition and version/tests.rs: rank.rs:946; num.rs:210, 281, 319, 406; tests.rs:1393; num.rs:42-44 is the same rationale without the word); executed: no
- Seen by: structure, prose, claims; refutation: confirmed; history: deliberate and holds for the mechanism, dated for the prose (cfa7c7ed restored the whole `(num >> exp) + 1` spelling because a width-guard early exit skipped the shift's width-scale limb record and read the board's `rank_encode` limb floor from below; the floor is a liveness floor asserting the encode walk reads every limb)
- Owner-gated: no

A reader who never saw the single-arm numerator gets no information from "historical" (Principle 5: dated rationale at a declaration site is a ghost reference; provenance lives in git). At 208-212 the word also carries a real, live rationale that is stated nowhere in the tree: the shift-by-zero is executed so the shift's limb record is emitted, because the board's `rank_encode` limb floor asserts that the encode walk reads the numerator and the record rides the shift. That converts to "state the rationale at the site"; the option of adding an early return and re-pinning would re-open the verdict cfa7c7ed cured.

Evidence:

       208	            // The base arm can only shrink, so it stays canonical with no
       209	            // re-dispatch — including the shift-by-zero spelling, which
       210	            // keeps this arm's cost and metering exactly the historical
       211	            // numerator path's.
       212	            Num::Base(base) => Num::Base(base >> n),

    (rank.rs)
       946	        // historical cost, and a 32-bit target's wide sums (a gap at or

    (cfa7c7ed, commit message)
    one that skipped the shift's width-scale limb record, reading the
    amp board's rank_encode limb floors from below (the floor was right:
    the walk genuinely reads the numerator; the record rides the shift).

Resolution: Restate each site positively and undated: num.rs:208-211 "The base arm can only shrink, so it stays canonical without re-dispatch. The shift runs even at zero so its width-scale limb record is emitted: the board's rank_encode limb floor asserts that the encode walk reads every limb of the numerator, and that record rides the shift."; num.rs:42-44 "Base-arm operations are Base's own metered methods"; num.rs:281 see rank-25; num.rs:319-320 "Below the ceiling this is Base::from_be_bytes then the metered sub-byte shift"; num.rs:405-407 "so the conversion never fires and every Base numerator passes through unchanged, unmetered"; rank.rs:946-947 "at Base's shift-and-add cost"; version/tests.rs:1393 likewise. Acceptance: `grep -n historical` over the partition and version/tests.rs is empty, and the shift-by-zero's rationale is stated at num.rs:206-212.

**Nits (5), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| rank-3 | `crates/before/src/version/rank.rs:152-166` | Copyedit slips in the public Rank and Ranked type docs | "strictly monotone in the causal order"; "so any tiebreak between equal ranks extends the causal order to a total one" ... | `evidence/partitions/rank.md` |
| rank-11 | `crates/before/src/version/rank.rs:493-509` | The same shouted warning appears verbatim on raw_parts and from_raw; the invariant it guards is never stated positively | State once, on `from_raw`: "Crate-private, and must stay so together with `raw_parts`: every public construction path bounds `exp` by bits already res ... | `evidence/partitions/rank.md` |
| rank-13 | `crates/before/src/version/rank.rs:549-552` | Register and vocabulary: "honest", "loud", "rent", "sliver", "door", "two-ways pin", "rank-class" | Replace each modifier with the mechanism it abbreviates: rank.rs:550 "and the gap clause holds for every u64 exponent on a 64-bit target" ... | `evidence/partitions/rank.md` |
| rank-15 | `crates/before/src/version/rank.rs:621` | Em-dashes in // comments (19 lines); none in assert or expect messages | Replace with colons, semicolons, or parentheses at the listed lines, as part of a crate-wide pass | `evidence/partitions/rank.md` |
| rank-31 | `crates/before/src/version/ranked.rs:101` | The M-definition sentence is hand-copied six times in ranked.rs (eleven crate-wide) | Define `M` once in the crate docs' complexity notation (lib.rs already hosts the asymptotic-guarantee section) and have the fuelscape include emit a l ... | `evidence/partitions/rank.md` |

**Cross-references.** rank-6 and api-audit-18 share the `rhs`/`other` drift. rank-3, fresh-eyes-11, api-audit-17, and prose-hygiene-13 all touch rank.rs:198's "never is larger". rank-31's eleven copies of `M` and rank-19's public use of "magnitude class" both resolve through the crate-level notation section proposed in Open questions 14. rank-33 (the claim finding) is the `Ranked::cmp` complexity contract rank-19's rewording defers to.

## Crate root and public types: span and causally

22 findings (0 high, 1 medium, 11 low, 10 nit). Full records: `evidence/partitions/span-causally.md`, `evidence/sweeps/api-audit.md`, `evidence/sweeps/clippy-pedantic.md`, `evidence/sweeps/fresh-eyes.md`.

### span-causally-33: The SAT/NP-completeness motivation is stated three ways, once logically inverted, never argued
- Where: crates/before/src/causally/query.rs:26-30 (related: crates/before/src/causally.rs:78-83; crates/before/src/causally/polarity.rs:209-212)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read the three sites); executed: no
- Seen by: prose; refutation: confirmed (adds a correct sketch: monotone SAT reduces to mixed-polarity emptiness); history: no rationale found (the inverted direction and the "footgun"/"famously"/"non-polynomial" register are the owner's own words from a6dcfbb4; the text they replaced, db9dfa3e's "deciding whether it empties a span encodes satisfiability", had the correct direction)
- Owner-gated: no (doc accuracy), but the text is owner-authored

The public `Query` doc says deciding span overlap for arbitrary queries "reduces to the SAT problem, and is therefore NP-complete (non-polynomial)": a problem that reduces to SAT is in NP (an upper bound), not thereby NP-complete, and NP-complete does not mean non-polynomial. causally.rs:79-83 says "equivalent to", polarity.rs:209-212 "reduces to", and none of the three gives the reduction; three paraphrases that disagree in strength are the tell that none is the statement of record. A hardness claim in public docs is a claim like any other and needs its argument. The causally.rs site also carries register transplants ("powerful hidden footgun", "famously", "silently exponential") where no adversary exists, and the typo of span-causally-5.

Evidence:

        26	/// Not all queries may be combined, because exactly deciding [`Span`]-overlap
        27	/// for entirely arbitrary queries reduces to the SAT problem, and is therefore
        28	/// NP-complete (non-polynomial). The [`Polarity`] restriction enforced by the
        29	/// types of [`Query`] ensures that only linear-time decidable queries are
        30	/// expressible.

    (causally.rs)
        79	//! `&`, permitting this carries a powerful hidden footgun: exactly deciding the
        80	//! [`coverage`](Query::coverage) of a [`Span`] against a freely constructed
        81	//! [`Query`] with arbitary negation is equivalent to the famously NP-complete
        82	//! SAT problem: exposing this interface would make it easy to express silently
        83	//! exponential queries.

Resolution: state the claim once, in causally.rs's Polarity section, in the direction that holds with a one-line sketch. The refutation pass's sketch: SAT (in its monotone form, every clause all-positive or all-negative, still NP-complete) reduces to mixed-polarity emptiness: take one region per variable and the span `[⊥, all ones]`; an all-positive clause is one `Down` hole `v <= h` with `h` zero exactly on the clause's regions (it subtracts the assignments violating the clause), an all-negative clause one `Up` hole `h' <= v` with `h'` one exactly on its regions; the clamp minus the holes is nonempty iff the formula is satisfiable, so exact `coverage` for a mixed query is at least as hard as SAT. Then make query.rs:26-30 and polarity.rs:209-212 one sentence each linking there; replace "footgun"/"famously"/"silently exponential" with the mechanism; align "linear time" with span-causally-24. Acceptance: exactly one site states the hardness claim with its direction and argument; `grep -rn 'NP-complete\|SAT problem' crates/before/src` returns one definitional hit plus links; "non-polynomial" is gone.

### span-causally-4: `Span::dominance`/`precedence` comments explain the fast path in the consumer's vocabulary ("compressed-subtree classification")
- Where: crates/before/src/span.rs:313-314 (related: crates/before/src/span.rs:377-379)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'compressed-subtree\|compressed subtree' crates/before/src` returns exactly these two sites; nothing in before names nodes or subtrees); executed: no
- Seen by: structure, prose; refutation: confirmed (corrects the cite: the abstraction-boundary rule is the root AGENTS.md "Writing style" section, not crates/before/AGENTS.md); history: no rationale found (4874f527 added the rung to serve rumors' classifier and wrote the motivation in rumors' terms)
- Owner-gated: no

Both coarsening fast paths close their rationale with a sentence about "compressed-subtree classification" and "a node whose version bounds coincide", which are the rumors tree's concepts. The root AGENTS.md asks documentation to respect abstraction boundaries; a maintainer of before has no node or subtree to map these to, and the paragraph above each sentence already states the in-crate mechanism (one single-bound placement instead of the fused walk reading one shared buffer twice).

Evidence:

       313	        // This is the compressed-subtree classification fast path: a node whose
       314	        // version bounds coincide is classified against one stream, not two.

       377	        // This is the compressed-subtree classification fast path, mirrored: a
       378	        // node whose version bounds coincide is classified against one stream,
       379	        // not two.

Resolution: delete both sentences, or restate in before's terms ("a caller holding many coincident spans classifies each against one stream"). Acceptance: the grep returns nothing.

### span-causally-5: Public rustdoc typos, a doubled word, a garbled sentence, and a ghost parameter name
- Where: crates/before/src/span.rs:413-415 (related: crates/before/src/span/algebra.rs:754-755, 785-786, 814-815, 899-902; crates/before/src/causally.rs:58-59, 81; crates/before/src/causally/conjunction.rs:5-6; crates/before/src/causally/forms.rs:231-234, 253)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (each excerpt read at the cited line; grep for `seach\|arbitary\|intractible\|of a \[`Span`\]s` and for line-final `taken as` locates exactly these sites; `toward`'s signature at forms.rs:253 is `toward(s, t)`); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: no rationale found (`after(p)` is a rename residue from a6dcfbb4, which renamed the parameter and rewrote the prose but left the equivalence line; the algebra.rs and span.rs items are from 22cdfbe1; "arbitary"/"intractible" from a6dcfbb4; "of a [`Span`]s" from bbb9f802)
- Owner-gated: no

Public rustdoc is the contract the reader came for, and these sit under `# Complexity` and on operator impls. Eight sites: span.rs:414 "the two `hi` endpoint, where seach comparison"; algebra.rs:754-755, 785-786, 814-815 "taken as / as its coincident point span" (the doubled word renders three times on the `Span` page); algebra.rs:900-902 "The smallest span containing [`Span`] two versions is"; causally.rs:59 "of a [`Span`]s"; causally.rs:81 "arbitary"; conjunction.rs:6 "intractible" (private module doc); forms.rs:234 "Equivalent to `after(p) & until(t)`" for a function whose parameters are `s` and `t` and whose prose uses `s` and `e`.

Evidence:

       413	    /// A [`Span`] requires two causal comparisons: one to compare the two `lo`
       414	    /// endpoints and a second to compare the two `hi` endpoint, where seach
       415	    /// comparison costs:

    (algebra.rs, likewise 785-786 and 814-815)
       754	    /// The right operand is anything [`Into`] a [`Span`]; a [`Version`] is taken as
       755	    /// as its coincident point span.

    (algebra.rs)
       900	    /// which a version-pair `+` would conceptually contradict. The smallest
       901	    /// span containing [`Span`] two versions is [`span`](Version::span) (`v ^
       902	    /// w`).

    (causally/forms.rs)
       231	/// Everything in the causal future of `s` (including `s` itself) but nothing in
       232	/// the causal future of `e` (including `e` itself).
       233	///
       234	/// Equivalent to `after(p) & until(t)`.
       253	pub fn toward<'a>(s: impl Into<Cow<'a, Version>>, t: impl Into<Cow<'a, Version>>) -> Query<'a, Up> {

Resolution: "the two `hi` endpoints, where each comparison costs:"; drop the duplicated "as" at three sites; "The smallest [`Span`] containing two versions is [`span`](Version::span) (`v ^ w`)."; "of [`Span`]s grant them"; "arbitrary"; "intractable"; make `toward`'s doc use `s` and `t` throughout ("nothing in the causal future of `t` (including `t` itself). Equivalent to `after(s) & until(t)`."). Acceptance: the greps return nothing and the doc letters equal the signature's.

### span-causally-13: Maintainer docs in `algebra.rs` give the wrong reason for the missing identities and a clone-discipline sentence the code contradicts
- Where: crates/before/src/span/algebra.rs:1014-1016 (related: crates/before/src/span/algebra.rs:1118-1120, 1041-1043, 346-348, 364)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (re-derived both identities; read the fold doc against the filter body); executed: no
- Seen by: prose, correctness, claims; refutation: confirmed (severity lowered from medium: private macro docs; the public impl doc at 1041-1043 says only "(union has no identity span)", which is correct); history: no rationale found (both identity paragraphs are the owner's own text, 20c0515a; the clone sentence and the clone-on-entry filter landed together in ed1b3c8b)
- Owner-gated: no

Three maintainer sentences state an argument the code does not support. (1) The `span_union_fold!` doc says union has no identity because "the version lattice has no top": an identity `e` for `+` needs `lo_a & lo_e = lo_a` for all `a` (so `lo_e` is a top) and `hi_a | hi_e = hi_a` (so `hi_e` is the bottom), i.e. the crossed pair `(top, bottom)`, which is no `Span` whether or not a top exists; the missing top is not the operative reason. (2) The `span_intersect_fold!` doc says "no span is covered by every span": the intersection identity is the span that covers every span, `(bottom, top)`, so the direction is inverted, and there the missing top is the actual reason. (3) `fold_endpoints`'s doc says inputs "are cloned only at their first combine", but the dedup filter clones every admitted input's endpoints on entry (line 364, refcount bumps).

Evidence:

      1014	/// The receiver is [`Option`] because union has no identity: the version
      1015	/// lattice has no top, so an empty iterator has no non-empty hull. `None`
      1016	/// means exactly "no spans came", never an empty union. The item shapes

      1118	/// [`None`] covers both an empty iterator (intersection has no identity: the
      1119	/// version lattice has no top, so no span is covered by every span) and a
      1120	/// nonempty family sharing no version — the two ways there is no product.

       346	    /// so the fold is never empty. Inputs enter untouched and are cloned only
       347	    /// at their first combine, and every clone of a stored version is a
       348	    /// refcount bump, never a byte copy.
       364	                    last = Some((s.lo().clone(), s.hi().clone()));

Resolution: union: "union has no identity span: the identity would be the empty set of versions, and every [`Span`] is nonempty (`lo <= hi`)". Intersection: "intersection has no identity: the identity would be the span covering every span, `[bottom, top]`, and the version lattice has no top." Fold doc: "inputs are borrowed into the counter; the dedup filter holds one refcount pair, and every clone of a stored version is a refcount bump, never a byte copy." Acceptance: the three sentences read true against the code; "covered by" at 1119 becomes "covers".

### span-causally-16: `OwnSpan::to_span` cites a monotonicity argument "the type's docs carry" that no type's docs carry
- Where: crates/before/src/span/own.rs:264-270 (related: crates/before/src/version/own.rs:12-46; crates/before/src/version.rs:700-724, 1669-1676; crates/before/src/laws.rs:2620-2628, 2688-2690)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -ni monoton crates/before/src/version/own.rs` returns nothing; read `OwnVersion`'s type doc and `Version::project`'s doc, which state only that a projection is a sub-version; the argument lives at the `Div` impl comment, version.rs:1669-1674, and the `projection_monotone_in_version` law, laws.rs:2623); executed: no
- Seen by: prose, correctness; refutation: confirmed; history: no rationale found (born dangling in ed1b3c8b)
- Owner-gated: no

`to_span` builds through `Span::owned` with no validation, so the `lo <= hi` invariant rests on monotonicity of projection, and the doc points at an argument that is not where it says. The argument that exists (projection is a join and meet homomorphism, so `a <= b`, i.e. `a | b == b`, gives `a/p | b/p == b/p`, i.e. `a/p <= b/p`) lives at the `Div` impl comment and the law. One caution when repointing: version.rs:1675-1676 reads "Projection can still raise `min_ticks` ..., so it is not monotone under `<=`", whose "it" a reader takes as the projection; that sentence should say `min_ticks` is what fails to be monotone.

Evidence:

       264	    /// Materializes the projected span: the explicit, eager form of
       265	    /// this view.
       266	    ///
       267	    /// One [`OwnVersion::to_version`] per endpoint; the projection is
       268	    /// monotone (the type's docs carry the argument), so the
       269	    /// projected pair is ordered and the construction revalidates
       270	    /// nothing.

Resolution: state the argument inline ("projection is a join homomorphism, so `lo <= hi` gives `lo / p <= hi / p`; the `projection_monotone_in_version` law pins it") and optionally add the monotonicity sentence to `OwnVersion`'s docs; disambiguate version.rs:1675-1676. Acceptance: the pointer resolves (`grep -ni monoton` finds the argument at the cited site), and the `Div` comment's "it" names `min_ticks`.

### span-causally-22: "Structural genres" in `Span::decode`'s public `# Errors` is undefined at the user's altitude
- Where: crates/before/src/span/wire.rs:91-92 (related: crates/before/src/span/wire.rs:79-90; crates/before/src/error.rs:68-92; crates/before/src/span/tests.rs:448-463)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -ni genre crates/before/src/error.rs crates/before/src/lib.rs` returns nothing; `Decode`'s variant docs never use the word); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (the public sentence inherited the wire module's private vocabulary from 39f5d591's "structural genres win")
- Owner-gated: no

Public rustdoc names nothing the API does not reach: the user reaches `Decode::Truncated`, `Decode::TrailingBits`, and `Decode::NotCanonical`, not a "genre" (crate-internal dialect, 210 occurrences, defined only in a private codec doc). The precedence rule holds and is pinned by `span_decode_structural_genres_outrank_the_pair_verdict`; it deserves stating in the user's vocabulary.

Evidence:

        91	    /// On an input defective several ways at once, the components'
        92	    /// structural genres win.

Resolution: "On an input defective several ways at once, a component's own [`Truncated`](Decode::Truncated) or [`TrailingBits`](Decode::TrailingBits) is reported before the pair's [`NotCanonical`](Decode::NotCanonical), exactly as decoding the two components separately would." Acceptance: the public doc names the variants; "genre" appears in wire.rs only in private comments or not at all.

### span-causally-27: Banned vocabulary: "mint"/"minting" for constructing a hole
- Where: crates/before/src/causally/conjunction.rs:29-37 (related: crates/before/src/party.rs:15, 872)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rni '\bmint'` over the thirteen partition files returns exactly conjunction.rs:30 and :37); executed: no
- Seen by: prose; refutation: confirmed; history: already-known (2c73d032 purged the word from the rumors crate as an owner ruling; the before crate was not swept, and these two sites are the owner's own text from 20c0515a, written before the ruling)
- Owner-gated: no

The review standard names "mint" as never to be written for constructing a value, and the ruling on record already removed it from the sibling crate.

Evidence:

        29	    /// Each operand's holes are already a pairwise-unabsorbed antichain
        30	    /// (constructors mint at most one hole; every multi-hole query came
        31	    /// through this merge), so same-side pairs are never compared — only the

        36	    /// fall into rides through inert, subtracting nothing on every path,
        37	    /// rather than minting a corner case here.

Resolution: line 30: "(constructors add at most one hole; ...)"; line 37: "rather than introducing a corner case here." Acceptance: `grep -rni '\bmint' crates/before/src/causally crates/before/src/span` returns nothing (party.rs is outside this partition).

### span-causally-34: `Coverage`'s docs do not state exactness, and `laws.rs` cites a precision contract on them that does not exist and describes an incompleteness the code does not have
- Where: crates/before/src/causally/query.rs:50-59 (related: crates/before/src/causally/query.rs:143-171; crates/before/src/laws.rs:1029-1039; crates/before/src/causally/tests.rs:222-323)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `Coverage`'s docs in full, which state only the three verdicts' meanings; read `refine_partial`'s doc, "the exact emptiness decision"; read `coverage_is_exact_on_the_two_party_grid`, which asserts every verdict equals the brute-force census; `git show db9dfa3e:crates/before/src/causally/query.rs` carries "The verdict is **exact** for every constructible query" at its lines 61-62); executed: no
- Seen by: correctness, claims; refutation: confirmed (re-derived exactness: for `Down` the clamp is nonempty iff `clamped_lo <= clamped_hi`, and then `clamped_hi` is admitted iff no hole subtracts it, since every `v` in the clamp has `v <= clamped_hi <= hole.at`; dually for `Up`); history: no rationale found (a6dcfbb4 deleted the exactness sentence from `Coverage` and from `coverage`'s doc; b3f09baa introduced the laws.rs sentence the next day citing a precision contract that never existed anywhere in the tree and asserting an incompleteness the code has never had)
- Owner-gated: no

A user reading `Coverage`'s rustdoc cannot tell whether `Partial` means "genuinely mixed" or "undecided"; the code makes it the former (the clamp refinement exists to buy exactly that), and the docs are silent. The `coverage_bounds_membership` law's doc says "the [`Coverage`] docs carry the precision contract, including why `Empty` cannot be complete", which is a ghost reference in both halves: no such text exists, and `Empty` is complete. Prose speaks in the present tense.

Evidence:

        50	/// How much of a [`Span`]'s segment a [`Query`] admits.
        51	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        52	pub enum Coverage {
        53	    /// Every version the span covers is admitted by the query.
        54	    Full,
        55	    /// Some covered versions are admitted and some are not.
        56	    Partial,
        57	    /// No version the span covers is admitted by the query.
        58	    Empty,
        59	}

    (laws.rs)
      1036	    /// arms are exercised against genuinely interior points. `Partial` promises
      1037	    /// nothing pointwise: the [`Coverage`] docs carry the precision contract,
      1038	    /// including why `Empty` cannot be complete.

Resolution: restore one sentence on `Coverage` (db9dfa3e's "The verdict is exact for every constructible query" is a restoration candidate; today's wording: "Each verdict is exact: `Partial` means at least one covered version is admitted and at least one is not"); rewrite laws.rs:1036-1038 to say the law pins soundness and that completeness is pinned by `coverage_is_exact_on_the_two_party_grid`. Acceptance: `grep -rn 'cannot be complete' crates/before/src` returns nothing; `Coverage`'s rustdoc states exactness; both agree with `refine_partial`'s doc.

### span-causally-37: `Query::into_owned` is documented `O(1)` but rebuilds the hole `Vec`
- Where: crates/before/src/causally/query.rs:176-194 (related: crates/before/src/causally/query.rs:38; crates/before/src/causally/convert.rs:36)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read the body); executed: no
- Seen by: prose, claims; refutation: confirmed; history: no rationale found (db9dfa3e read "`O(1)` per stored bound"; a6dcfbb4 dropped "per stored bound")
- Owner-gated: no

The body allocates a fresh `Vec` and maps every hole into it: O(#holes) refcount bumps plus one allocation, and the hole count is unbounded. The `Clone` impl's doc (line 38, "`O(1)` per bound") and convert.rs:36 ("`O(1)` per stored bound") have the accurate denominator; `into_owned` dropped it. Asymptotic claims are hard guarantees in this crate.

Evidence:

       176	    /// # Complexity
       177	    ///
       178	    /// `O(1)`: owned versions move, borrowed ones clone by sharing their stored
       179	    /// buffers.

       184	            holes: self
       185	                .holes
       186	                .into_iter()
       187	                .map(|hole| Hole {
       188	                    at: Cow::Owned(hole.at.into_owned()),
       189	                    strict: hole.strict,
       190	                })
       191	                .collect(),

Resolution: "`O(1)` per stored bound (one allocation for the hole list): owned versions move, borrowed ones clone by sharing their stored buffers." Alternatively settle the holes in place (`for hole in &mut holes { hole.at = Cow::Owned(...) }`) and keep the per-bound wording. Acceptance: the complexity sentence names the per-bound denominator.

### span-causally-38: "there is deliberately no `Eq`; see the module docs" points at module docs that no longer discuss `Eq`
- Where: crates/before/src/causally/query.rs:211-212 (related: crates/before/src/laws.rs:807-809; crates/before/src/causally.rs:1-139; crates/before/src/causally/conjunction.rs:61)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'Eq\b'` over causally.rs and causally/*.rs, excluding `Ordering::Equal`, returns only the `Coverage` derive at query.rs:51 and this pointer; laws.rs:809 repeats the pointer; `git show db9dfa3e:crates/before/src/causally.rs` lines 60-80 carry the deleted "# Deliberately absent" section with the `Eq` bullet); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: deliberate-but-expired (the pointer was true when written; the owner's a6dcfbb4 deleted the section while re-wrapping and keeping the pointer)
- Owner-gated: no

A pointer to a decision recorded nowhere sends the maintainer on a search that ends nowhere, and a maintainer who wants to add `PartialEq` finds no stated objection. The reason holds and is written nowhere: the hole antichain is stored in construction order (`kept.append(&mut added)` at conjunction.rs:61, so `a & b` and `b & a` differ structurally), and a degenerate hole rides inert beside `all()` (`degenerate_holes_are_inert`), so structurally different normal forms denote one predicate and structural equality would be neither semantic equality nor a useful approximation. The deleted text is a restoration candidate.

Evidence:

       211	// Debug renders the module's own expression vocabulary, the only structural
       212	// window into a query (there is deliberately no `Eq`; see the module docs), so

    (laws.rs)
       807	    /// query admitting exactly itself. Behavioral equations only: a query's
       808	    /// observation surface is membership, deliberately not identity
       809	    /// (`causally`'s module docs carry the no-`Eq` decision).

    (causally.rs at db9dfa3e, deleted by a6dcfbb4)
        73	//! - **`Eq`, `Hash`, and a wire form.** A query is an ephemeral
        74	//!   filter, not a value: two queries built differently may denote
        75	//!   the same predicate. Observe queries behaviorally, through
        76	//!   [`contains`](Query::contains) and [`coverage`](Query::coverage).

Resolution: record the decision once, either restored to causally.rs's module doc (where both pointers say it is) or on `Query`'s type doc with the pointers repointed; state the mechanism (non-unique normal forms under conjunction order and inert degenerate holes). Acceptance: `grep -n 'Eq' crates/before/src/causally.rs` (or query.rs's type doc) finds the decision the two comments cite.

### api-audit-9: Public docs point at arguments that live nowhere public (OwnSpan monotonicity; Query's missing Eq)
- Where: crates/before/src/span/own.rs:267-270 (related: span/own.rs:11-37, version/own.rs:12-28, version.rs:1669-1676, causally/query.rs:211-215, causally.rs:1-139)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both `Own*` type docs, version.rs:1660-1711, causally.rs in full; `grep -n 'Eq' crates/before/src/causally.rs` matches only `Ordering::Equal` at line 163; `grep -rn monoton` over the span/version files hits only own.rs:268, version.rs:356, version.rs:1676); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`OwnSpan::to_span` says the type's docs carry the monotonicity argument;
neither `OwnSpan`'s (span/own.rs:11-37) nor `OwnVersion`'s (version/own.rs:12-28)
does. The only nearby argument is a private comment whose closing sentence
reads "so it is not monotone under `<=`", with `min_ticks` as its true subject,
so a reader following the pointer meets an apparent contradiction. Separately,
query.rs says "there is deliberately no `Eq`; see the module docs", and the
`causally` module docs never mention it.

Evidence:

       267	    /// One [`OwnVersion::to_version`] per endpoint; the projection is
       268	    /// monotone (the type's docs carry the argument), so the
       269	    /// projected pair is ordered and the construction revalidates
       270	    /// nothing.

      1675	// Projection can still raise `min_ticks` (carving one broad tick into
      1676	// disjoint peaks), so it is not monotone under `<=`.

       211	// Debug renders the module's own expression vocabulary, the only structural
       212	// window into a query (there is deliberately no `Eq`; see the module docs), so

Resolution: state once, in `OwnVersion`'s public docs, that projection is a homomorphism of join and meet and therefore order-preserving (`a <= b` implies `a/p <= b/p`), and point `to_span` there; reword version.rs:1675-1676 so `min_ticks` is the stated subject ("`min_ticks` is not monotone under projection"); add the reason `Query` has no `PartialEq` to `Query`'s type docs and fix the query.rs pointer. Acceptance: each pointer resolves to a paragraph that states the argument.

### fresh-eyes-8: Query has no equality; the public docs do not say so and the private pointer to the rationale dangles
- Where: crates/before/src/causally/query.rs:20-30 (related: crates/before/src/causally/query.rs:211-215, crates/before/src/causally/query.rs:1-8, crates/before/src/causally.rs:1-169)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (the sweep's check1.log line 51: `error[E0369]: binary operation '==' cannot be applied to type 'Query<'_, before::causally::Down>'`; this pass grepped `Eq`, `equal`, and `equalit` across `causally.rs` and `causally/*.rs`: the only hits are the `Coverage` derive at query.rs:51, the comment at query.rs:212, and unrelated uses of "equal"); executed: yes, by the sweep's compile probe, matched to its log
- Verification: reframed: the sweep found the rationale hidden in a private comment; this pass finds that the comment's "see the module docs" points at prose that does not exist in either query.rs's or causally.rs's module doc; history: no-rationale-found (db9dfa3ed introduced the comment; its message does not carry the argument either)
- Owner-gated: no

`Query` is `Clone` and `Debug` but not `PartialEq`. The deliberate absence is stated only in a private comment, and that comment defers to module docs that never take the subject up, so the reason survives nowhere in the tree.

Evidence:

        20	/// A causal filter on [`Version`]s and [`Span`]s within a restricted [`Query`]
        21	/// language.
        22	///
        23	/// Queries are composed from the atomic queries in this module, which may be
        24	/// negated with `!` and combined with `&`.

       211	// Debug renders the module's own expression vocabulary, the only structural
       212	// window into a query (there is deliberately no `Eq`; see the module docs), so
       213	// failures and logs read as an expression denoting the same predicate.

Resolution: Add one sentence to the `Query` type docs stating that queries carry no equality (two structurally different queries can denote one predicate, so `==` would not mean what a reader expects) and that `Debug` renders the normal form, and either state the rationale at query.rs:212 inline or point the comment at the sentence that now holds it. Acceptance: `grep -n 'see the module docs' crates/before/src/causally/query.rs` either returns nothing or the module doc it names contains the word `Eq`.

**Nits (10), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| span-causally-2 | `crates/before/src/span.rs:176-179` | Em-dashes inside `//` comments at fifteen partition sites | recast each with a colon, semicolon, or parentheses. The crate-wide sweep (374 lines) is a separate prose-pass decision; see the open questions | `evidence/partitions/span-causally.md` |
| span-causally-7 | `crates/before/src/span/algebra.rs:39-43` | The `v + w` absence argument is written out three times in `algebra.rs` | keep the module-doc statement; reduce 672-675 to a pointer ("a version-pair `+` is deliberately absent ... | `evidence/partitions/span-causally.md` |
| span-causally-14 | `crates/before/src/span/own.rs:88-90` | `OwnSpan`'s composed two-comparison verdicts are a deliberate design whose rationale lives only in history | one sentence on the type or the `place` doc: the verdicts compose two masked comparisons (no fused masked placement walk exists ... | `evidence/partitions/span-causally.md` |
| span-causally-17 | `crates/before/src/span/tests.rs:149-151` | "door" is used as jargon for constructors and entry points without a definition | in this partition, the plain noun ("constructor", "entry point", "`span_all`"). Crate-wide ... | `evidence/partitions/span-causally.md` |
| span-causally-18 | `crates/before/src/span/tests.rs:608-609` | A testdoc hardcodes "depth 2" for a constant defined elsewhere | "The small-scope sweep is exhaustive to `EV_SMALL_DEPTH`; ..." | `evidence/partitions/span-causally.md` |
| span-causally-19 | `crates/before/src/span/verdict.rs:69-85` | `Dominance` and `Precedence` docs drift in vocabulary between mirrored variants | one noun ("version", matching the parameter name) and one relation phrase ("concurrent to", the crate's public term) in both enums | `evidence/partitions/span-causally.md` |
| span-causally-20 | `crates/before/src/span/wire.rs:40-41` | `wire.rs` calls the endpoints "the meet" and "the join" where the rest of `Span` says `lo`/`hi` and `meet`/`join` name the operators | `lo`/`hi` (or "the lower endpoint") in wire.rs and in the span/tests.rs genre strings ... | `evidence/partitions/span-causally.md` |
| span-causally-29 | `crates/before/src/causally/conjunction.rs:150-154` | `conjoin!` gives every `&` cell one doc line and the hole-bearing island, including the seven hole-free atom cells | let the macro take a per-group doc string (atom x atom, atom x polar, polar x polar) so each cell states its output polarity ... | `evidence/partitions/span-causally.md` |
| span-causally-30 | `crates/before/src/causally/forms.rs:19-32` | `Floor`/`Ceiling` public docs state their predicate with the private field name `at` | `s <= v` / `v <= e` (matching `after(s)` / `before(e)`), or "bound <= v" | `evidence/partitions/span-causally.md` |
| clippy-pedantic-5 | `crates/before/src/span.rs:36-37` | 87 doc links spelled `[`name`]`(args)`` render as two adjacent code spans with the link on only the first | mechanical pass: `[`Span::new(lo, hi)`](Span::new)`, | `evidence/sweeps/clippy-pedantic.md` |

**Cross-references.** span-causally-34, span-causally-38, fresh-eyes-8, and api-audit-9 report the decisions a6dcfbb4 deleted (the `Coverage` exactness sentence; the no-`Eq` rationale) that two pointers still cite; span-causally-16 and api-audit-9 the `to_span` monotonicity pointer. span-causally-5, fresh-eyes-11, and api-audit-17 collect the same public typos. span-causally-27, prose-hygiene-5, and the party sites are the "mint" census. span-causally-24 and span-causally-36 (the claim findings) are the multi-hole cost contract that span-causally-33's "linear time" clause should align with. clippy-pedantic-5's split code-span links are anchored at span.rs:36-37 but recur across meter/registry.rs (68 of its 87 hits).

## The skyline coding: coding (skyline.rs, admit, build, decode and encode, emit, literal, validate, text, shape, walk)

14 findings (0 high, 1 medium, 3 low, 10 nit). Full records: `evidence/partitions/skyline-coding.md`.

### skyline-coding-14: ghost reference to the retired `implementation` essay in a test doc
- Where: crates/before/src/version/skyline/build/tests.rs:394-396 (related: crates/before/AGENTS.md:6; crates/before/examples/code_study.rs:6; both outside this partition)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep for `mod implementation`, `before::implementation`, and the phrase across crates/before finds only the three citing sites); executed: no
- Seen by: prose, correctness, claims; refutation: confirmed (three lenses, one finding); history: contradicts-hard-rule (67970b75 added the module; 22cdfbe1 retired it and left the citations standing)
- Owner-gated: no

crates/before/AGENTS.md's hard rule and Principle 5: nothing in the codebase refers to code that no longer exists; provenance lives in git. The citation sits in the doc of the pin whose purpose is to name what it guards. Severity medium rather than high because the blast radius is one parenthetical and the fix is mechanical; the weightier occurrence is the guidepost at AGENTS.md:6, outside this partition.

Evidence:

    build/tests.rs
    394	/// independently; a different integer code (the `implementation` essay
    395	/// contemplates ζ₂, whose zero costs two bits) would silently turn every
    396	/// collapse check into a no-op — this pin turns that into a red test.

Resolution: restate the counterfactual in terms of what is: "a different integer code whose zero costs two bits (ζ₂, for instance) would turn every collapse check into a no-op"; excise the two out-of-partition sites in the same pass. Acceptance: `grep -rn 'implementation\` essay\|before::implementation' crates/before` returns nothing; the testdoc still names the two-bit-zero counterfactual.

### skyline-coding-22: the overlay-advance law has a third generic statement; overlay.rs's "exactly two generic faces" is a stale hand-maintained count
- Where: crates/before/src/version/skyline/shape.rs:176-198 (related: crates/before/src/version/skyline/shape.rs:17-18; crates/before/src/version/skyline/overlay.rs:12-17 and 268-286; crates/before/src/version/skyline/admit.rs:340-347; crates/before/src/shape.rs:284-287 and 357-359)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for the tie-assert string: admit.rs:370 and 382, overlay.rs:176, 190, and 283, shape.rs:195; both shape call sites read); executed: no
- Seen by: structure; refutation: reframed (the count correction is the solid part; folding `advance_refinement` into `CursorSet` would need an all-done guard at both callers because `advance_set` steps the deepest slot unconditionally); history: no rationale (e30d659de made the count true; 46eb64f97 added shape.rs's statement without saying why `advance_set` was not used)
- Owner-gated: no

Principle 5: a hand-maintained count that the tree has outgrown. `advance_refinement<W: Refine>` is the N-ary law with done-ness folded in, `admit::advance` is the documented fallible restatement, and overlay.rs still says the law is stated in exactly two generic faces; each statement carries its own copy of the tie assert.

Evidence:

    shape.rs
    17	//! walk whose depth the flip level reaches. [`advance_refinement`] states
    18	//! that law once over [`Refine`], for any arity and either walk kind;

    176	pub(crate) fn advance_refinement<W: Refine>(walks: &mut [W]) -> bool {

    191	    let flip = walks[deepest].advance();
    192	    for (slot, walk) in walks.iter_mut().enumerate() {
    193	        if slot != deepest && !walk.done() && walk.depth() >= flip {
    194	            let tied = walk.advance();
    195	            debug_assert_eq!(tied, flip, "tied boundaries close to one shared flip level");

    overlay.rs
    12	//! boundary carrying the cursor's own crossing payload. The overlay-advance law
    13	//! is stated (and debug-asserted) in exactly two generic faces — the binary
    14	//! [`advance`], which hands each crossing to the caller's fold, and the N-ary
    15	//! [`advance_set`] over a walk's whole [`CursorSet`], which folds crossings

Resolution: required: correct overlay.rs:12-17 to name the statements that exist and why each does (two overlay faces; shape's `Refine` face, which folds exhaustion in; admit's fallible restatement), or drop the count and state the structure. Optional: implement `CursorSet` for `[W; N]` over `Refine` (priority `0..N`, depth `0` when done, step = `advance`) plus an all-done check at src/shape.rs:284 and :359, and delete `advance_refinement`. Acceptance: `just gate` clean; the public shape iterators' snapshot and differential tests pass; overlay.rs's count matches `grep -rn 'tied boundaries close to one shared flip level' crates/before/src`.

### skyline-coding-26: dated incident narration and a meter-denominated constant at declaration sites in text.rs
- Where: crates/before/src/version/skyline/text.rs:147-161 (related: none)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale (742a02fc1 wrote both while curing the display heap reds; no commit chooses to keep incident narration at the declaration)
- Owner-gated: no

Principle 5: dated rationale at a declaration site is a ghost reference in disguise; state the invariant positively and leave when-and-why to git. Principle 3: a constant whose only stated justification is fitting a meter's allowance is the calibration-constant tell; if the board's allowance moves, this prose rots silently.

Evidence:

    text.rs
    147	/// How many parked entries one [`ParkedStack`] chunk holds: small enough that a
    148	/// shallow walk's single chunk sits inside the board's flat heap allowance,
    149	/// large enough that the chunk spine stays negligible.
    150	const PARKED_CHUNK: usize = 64;

    158	/// a doubling `Vec`, never holds an old and a new buffer at once during growth:
    159	/// that realloc coexistence spike is exactly what pushed the deep left-full
    160	/// shapes over the board's heap ceiling, and a chunk never moves once
    161	/// allocated.

Resolution: lines 158-161: "never holds an old and a new buffer at once during growth, so a deep left-full shape's peak transient is one chunk of slack, and a chunk never moves once allocated" (naming the board row that pins it is fine). Lines 147-150: derive 64 from a domain quantity (entry size against the per-level transient target) or state plainly that it is a tuning value whose live pin is the named board row. Acceptance: neither doc uses past-tense incident language; `PARKED_CHUNK`'s doc names a derivation or the committed pin that moves if the value changes.

### skyline-coding-31: `validate_from`'s error list omits the `Decode::Io` arm the borsh cursor surfaces
- Where: crates/before/src/version/skyline/validate.rs:65-68 (related: crates/before/src/version/skyline/admit.rs:271-272; crates/before/src/borsh_impls.rs:88-97 and 141; crates/before/src/error.rs:91; crates/before/src/version/skyline.rs:218-220)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (borsh_impls.rs:88-92 `impl<R: Read> BitCursor for ReaderCursor` with `type Error = Decode`, :97 `.map_err(Decode::Io)?`, :141 calls `validate_from(&mut cursor)`; error.rs:91 `Io(io::Error)`); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (complete when only the slice cursor existed; faf3cd0a introduced `ReaderCursor` and routed the borsh leg here; admit.rs, written later, lists the arm from birth)
- Owner-gated: no

A maintainer-facing contract must state every return arm; the function is generic over `C: BitCursor` with `Decode: From<C::Error>`, the borsh `ReaderCursor` returns `Decode::Io` on a failed read, and admit.rs documents that arm for the identical bound.

Evidence:

    validate.rs
    65	/// Returns with the cursor just past the tree. Errors: running out of bits
    66	/// mid-tree or mid-code is [`Decode::Truncated`]; a collapsible sibling pair
    67	/// (an internal node's two leaf children with a zero right delta) or a delta
    68	/// driving the running leaf height negative is [`Decode::NotCanonical`].

    admit.rs
    271	/// - [`Decode::Io`]: the cursor's own reads fail (the wire-side
    272	///   cursor's genre; a slice cursor reports truncation instead).

Resolution: add the arm in admit.rs's words and, optionally, convert the inline "Errors:" sentence to a `# Errors` list matching admit.rs:261-272. Acceptance: `validate_from`'s doc names `Truncated`, `NotCanonical`, and `Io` with the slice-cursor caveat.

**Nits (10), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| skyline-coding-1 | `crates/before/src/version/skyline.rs:55-55` | "currency" collides with the board's defined term; "arm" carries three senses; "door" is undefined | skyline.rs:55 and walk.rs:203: "the sign-magnitude type" / "as `Signed` values". shape.rs:110: "setting the pending rise" ... | `evidence/partitions/skyline-coding.md` |
| skyline-coding-3 | `crates/before/src/version/skyline.rs:123-145` | two inventories of one test suite, already drifting | keep the inventory in the tests module doc (where a new test is added) and reduce the kernel module's section to the invariants the tests protect plus ... | `evidence/partitions/skyline-coding.md` |
| skyline-coding-5 | `crates/before/src/version/skyline.rs:239-239` | em-dashes in 24 line comments and one assert message | replace each with a colon, semicolon, or restructured sentence; the assert message becomes "the adequacy witness went green: the kernel no longer demo ... | `evidence/partitions/skyline-coding.md` |
| skyline-coding-7 | `crates/before/src/version/skyline/admit.rs:256-257` | "mints" for constructing an error value | "and returns [`Decode::NotCanonical`] for a [`Refuted`] verdict only after they pass" | `evidence/partitions/skyline-coding.md` |
| skyline-coding-19 | `crates/before/src/version/skyline/encode.rs:36-36` | `expect` message names the wrong artifact | "a canonical packed preorder stream parses cleanly" | `evidence/partitions/skyline-coding.md` |
| skyline-coding-24 | `crates/before/src/version/skyline/tests.rs:477-478` | hand-maintained "depth-2" in two testdocs duplicates `EV_SMALL_DEPTH` | "of any normal form to the small-scope depth" (477) and "every normal-form tree to the small-scope depth" (570) | `evidence/partitions/skyline-coding.md` |
| skyline-coding-25 | `crates/before/src/version/skyline/text.rs:30-35` | text.rs module doc misplaces the second pin and misstates the kernels' visibility; a testdoc possessive | "pinned twice: by the wide-arming flatness band in `tests/meter.rs` ..., and by the committed schoolbook kernel in this module's tests (under `limb-me ... | `evidence/partitions/skyline-coding.md` |
| skyline-coding-30 | `crates/before/src/version/skyline/text.rs:540-549` | the reset-versus-compensating-subtraction argument is restated at full length three times | keep the module-doc statement; at 540-549 leave only "A reset, not a compensating subtraction: the module doc's exact-top argument" ... | `evidence/partitions/skyline-coding.md` |
| skyline-coding-34 | `crates/before/src/version/skyline/walk.rs:4-6` | walk.rs's module doc enumerates a client roster that has rotted | keep the structural clause and drop the enumeration, or make it explicitly non-exhaustive | `evidence/partitions/skyline-coding.md` |
| skyline-coding-35 | `crates/before/src/version/skyline/walk.rs:74-79` | a `# Panics` paragraph copied five times in walk.rs while claiming to be "stated once there" | state it once in walk.rs's module doc and reduce each function's section to `# Panics` plus one line ("Canonical input required ... | `evidence/partitions/skyline-coding.md` |

**Cross-references.** skyline-coding-14 is the third citation site of the `implementation` essay (with benches-examples-18 and the `AGENTS.md` cluster under crate root). skyline-coding-22's "exactly two generic faces" and party-20's "two cursor instances" are both overlay.rs:12-17 counts. skyline-coding-35's copied `# Panics` paragraph and skyline-sweep-place-masked-9's are the same paragraph in two modules. skyline-coding-19's `expect` message is the same flag-day residue as meter-registry-tier2-14's messages at tier2.rs:73 and :92.

## The skyline coding: fill and grow

14 findings (0 high, 1 medium, 6 low, 7 nit). Full records: `evidence/partitions/skyline-fill-grow.md`.

### skyline-fill-grow-24: The Counter widths section says the fill walk's `depth` "stays `usize`"; fill.rs declares it `u64`, and the crate argues one width three ways
- Where: crates/before/src/version/skyline/fill/prescan.rs:32-47 (related: crates/before/src/version/skyline/fill.rs:433-438; crates/before/src/version/skyline/grow.rs:517-519; crates/before/src/codec/stack.rs:37-45)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git blame -L 45,47 prescan.rs`: 3b883dd3 and 4bc9df64, 2026-08-13; `git blame -L 435,438 fill.rs`: 05d87e1b, 2026-08-19; `git log -S'stays \`usize\`' -- prescan.rs` returns 3b883dd3 only; `git log -S'let mut depth = 0u64;' -- fill.rs` returns 05d87e1b only; `git log --oneline` places 05d87e1b at position 139 and 3b883dd3 at 233, so the doc predates the change it contradicts); executed: no
- Seen by: prose (18), correctness (34), claims (40); refutation: confirmed; history: deliberate-but-expired at 05d87e1b (the widening did not revisit the prescan sentence)
- Owner-gated: no

The section contrasts the pre-scan's `u64` site-nesting counters against the fill walk's `depth`, which it says "stays `usize`, with its width argument stated at its declaration". fill.rs:438 declares `let mut depth = 0u64;` with a `u64` argument, so the contrast is false and a maintainer auditing integer widths is sent to verify a `usize` argument that does not exist. Principle 5: prose states what IS. The same section also argues `u64` from "2^64 sequential increments" while fill.rs:435-437 and codec/stack.rs:39-42 argue the same walk-surface width from allocatable memory: three rationales for one denomination.

Evidence:

        34	//! The recorder's site-nesting counters ([`run`](PreScan::run)'s `level`,
        35	//! `head_level`, [`SuspendedLevel`]'s `level`) are `u64` because reaching
        36	//! that cap would take 2^64 sequential increments: unreachable on every
        45	//! ([`Memo::set_link`]). The fill walk's near-synonymous `depth` (the
        46	//! [`run`](PreScan::run) doc contrasts the two notions) stays `usize`,
        47	//! with its width argument stated at its declaration.
    (fill.rs)
       435	        // `u64`, the walk surface's depth denomination: every open frame
       436	        // holds transient bits in real memory, so the count is bounded by
       437	        // allocatable memory, far below any `u64` wrap on every target.
       438	        let mut depth = 0u64;
    (codec/stack.rs)
        39	    /// `u64`, the walks' depth denomination: a stack this deep occupies
        40	    /// real memory (its words), so the height is bounded by allocatable

Resolution: Rewrite prescan.rs:32-47 to cite the convention by name and drop the contrast: the site-nesting counters are `u64` like every depth on the walk surface (codec/stack.rs's depth denomination), and the one width contract that differs in kind is the ledger's `u32` link index. Have fill.rs:435-437 and grow.rs:517-518 cite the same convention rather than re-deriving it. Acceptance: no sentence in prescan.rs names `usize` for the fill walk's depth; the width rationale for walk depths appears once (codec/stack.rs) and is cited elsewhere.

### skyline-fill-grow-1: Hand-quoted measurements and history language in the fill module's Cost section
- Where: crates/before/src/version/skyline/fill.rs:65-80 (related: crates/before/src/version/skyline/fill/tests.rs:1302-1304, crates/before/src/meter/board/ceilings.rs:57-69, crates/before/tests/meter.rs:8413-8432)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both sites; `grep -rn '\[measured' src` locates every bracketed reading in the crate; read ceilings.rs:57-69 and `memo_resolution_cost::assert_flat`); executed: no
- Seen by: prose (20), correctness (37), claims (41); refutation: confirmed (41 adds the fill/tests.rs site); history: deliberate-but-expired (written in f1e0e08b; the crate adopted the opposite convention in 500d4d09, whose sweep covered meter surfaces only)
- Owner-gated: no

Two bracketed readings ("e 1.00", "exponent 1.00") and the phrase "every refuted discipline" sit in production rustdoc. No committed check holds an exponent of 1.00: the board judges `MAX_SCALING_EXPONENT = 1.15`, and `memo_resolution_cost` judges ×2.5 per doubling, so a family drifting to 1.10 passes every gate while the prose keeps asserting 1.00. The crate's own meter prose states the rule this breaks: readings live in pin commits, never in prose. fill/tests.rs:1302-1304 quotes "+24 bits over 4096 ticks" where the assertion at 1354 is `b1 + 4 * logk + 8` (60 bits at k = 4096). Principle 5: a number that matters lives in a mechanically enforced place that prose cites by name; "refuted discipline" is provenance for git.

Evidence:

        65	//! Scan: `O(n + m)` bits in the two packed streams [measured: e 1.00 on every
        66	//! committed board family at both scales]. The walk consumes every position
        76	//! Limb: accumulator digit touches are amortized linear in the two packed
        77	//! streams [measured: exponent 1.00 with flat constants across the committed
        78	//! families — the `width_circulation_cost` and memo modules of
        79	//! `tests/meter.rs` name each family, state its shape, and pin the readings
        80	//! that separate this from every refuted discipline].
    (fill/tests.rs)
      1302	    /// [measured: the envelope holds with zero slack at the log term on the
      1303	    /// committed families over 512 ticks, and the fixed-pair orbit freezes at
      1304	    /// +24 bits over 4096 ticks].
    (ceilings.rs)
        57	// at the release profile of record. The readings themselves live in the pin
        58	// commits (`git log -S` the constant), never in this prose: a quoted reading
        69	pub const MAX_SCALING_EXPONENT: f64 = 1.15;

Resolution: Replace both brackets with the enforced statement by instrument name: the board's tick cells under `MAX_SCALING_EXPONENT`, and `tests/meter.rs`'s `width_circulation_cost` and `memo_resolution_cost` modules with their liveness floors. Drop "e 1.00", "exponent 1.00 with flat constants", and "every refuted discipline"; name `memo_resolution_cost` rather than "memo modules". At fill/tests.rs:1302-1304 state the enforced band (`b1 + 4·bitlen(k + 1) + 8`), not the observed reading. Acceptance: `grep -n '\[measured' fill.rs fill/tests.rs` returns nothing; every cited instrument name resolves.

### skyline-fill-grow-3: The `# Testing` sections paraphrase their sibling tests.rs module docs
- Where: crates/before/src/version/skyline/fill.rs:136-150 (related: crates/before/src/version/skyline/fill.rs:36-41, crates/before/src/version/skyline/fill.rs:57-61, crates/before/src/version/skyline/grow.rs:76-89, crates/before/src/version/skyline/fill/tests.rs:3-22, crates/before/src/version/skyline/grow/tests.rs:1-17)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (side-by-side read of fill.rs:138-150 against fill/tests.rs:3-22 and grow.rs:76-89 against grow/tests.rs:1-17); executed: no
- Seen by: prose (22); refutation: reframed (the memo appears once per cost axis, which the Cost structure requires; the excess is the two `# Testing` sections and the intro/heights overlap); history: no-rationale-found (Wave 7's consolidation ran under a praised-sentence guard whose list is not in the tree)
- Owner-gated: no

fill.rs:138-150 and grow.rs:78-89 restate, nearly clause for clause, what fill/tests.rs:3-22 and grow/tests.rs:1-17 say about themselves. The gate's `testdoc` reads tests.rs, not the kernel, so the kernel copy rots when the suite changes. The intro (36-41) and the heights paragraph (57-61) both state that no minimum is materialized and each travels as one ledger link. Every sentence competes with the contract the reader came for.

Evidence:

       138	//! Two committed differentials pin the fused walk directly to the recursive
       139	//! oracle, and they are the entire pin of the flag seam: `tick` byte-identical
       140	//! to the oracle's `event`, and the changed flag ≡ (the oracle's `fill` moved
    (fill/tests.rs)
         3	//! Two committed differentials are the entire pin of the fused walk and its
         4	//! changed flag, both total by canonical uniqueness: [`tick`] must equal the
         5	//! recursive oracle's `event` byte for byte, and the walk's changed flag must

Resolution: Replace each `# Testing` section with one sentence pointing at the sibling `tests.rs` module doc as the description of record. Cut the heights paragraph's restatement of the ledger-link fact to a pointer at the intro's statement. Before landing, check the cut sentences against the Wave 7 praised-sentence list if the owner still holds it. Acceptance: fill.rs and grow.rs each carry a one-sentence `# Testing`; nothing the tests.rs module docs say is restated in the kernels.

### skyline-fill-grow-4: `# Panics` promises a panic on any non-canonical stream; the code panics only on unreadable bits
- Where: crates/before/src/version/skyline/fill.rs:202-205 (related: crates/before/src/version/skyline/fill.rs:241-244, crates/before/src/version/skyline/fill.rs:290-292, crates/before/src/version/skyline/grow.rs:256-258, crates/before/src/version/skyline/grow.rs:274-276, crates/before/src/version/skyline/grow.rs:339-341, crates/before/src/version/skyline/sweep.rs:87-93, crates/before/src/version/skyline/walk.rs:74-79)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read every panic site on these paths: fill.rs:634, 641; grow.rs:260, 265, 283-284, 352; fuse.rs:231 are `.expect("canonical skyline bits")` on decode failures, and the `unreachable!` sites are not reached by a structurally well-formed stream; read the adopted precise form at sweep.rs:87-93 and walk.rs:74-79); executed: no
- Seen by: prose (19); refutation: confirmed (traced node[node[5,5],7] under id (0,1): the release build returns a stream); history: deliberate-but-expired (a736ef14 adopted the truncation/malformation-vs-silent form for walk.rs and overlay.rs, pointing at `causal_cmp`; that pass did not reach fill.rs, fuse.rs, grow.rs)
- Owner-gated: no

Six internal entries say "Panics if the event operand is not a canonical skyline stream". The only panics are on codes the cursor cannot decode; a structurally well-formed but non-canonical stream (an equal-sibling pair, a non-minimal topology) passes through the verbatim paths (`Out::note_match`, `copy_subtree`'s block skip, grow's `feed_subtree`/`continue_verbatim`) unrepaired and yields an unspecified stream. The crate states the accurate contract once at `causal_cmp` and cites it from walk.rs. A `# Panics` section is a contract; promising detection the code does not perform invites a maintainer to rely on `tick` as a validator.

Evidence:

       202	/// # Panics
       203	///
       204	/// Panics if the event operand is not a canonical skyline stream — run
       205	/// [`validate`](fn@super::validate) first on untrusted bytes. The id must own
    (grow.rs)
       256	    /// # Panics
       257	    ///
       258	    /// Panics if the stream is not a canonical skyline encoding.
    (fill.rs, the only panic on the path)
       633	    fn read_flag(&mut self) -> bool {
       634	        self.cursor.read_bit().expect("canonical skyline bits")
    (walk.rs, the adopted form)
        76	    /// The stream must be canonical. The violations this walk structurally
        77	    /// notices — truncation, malformation — panic; the rest walk silently
        78	    /// with an unspecified result (the contract of
        79	    /// [`causal_cmp`](super::sweep::causal_cmp), stated once there).

Resolution: Reword the six sites to walk.rs's form: the operand must be canonical (every stored `Version` is; run `validate` on untrusted bytes); truncation and malformation panic; a well-formed non-canonical stream yields an unspecified result, per `causal_cmp`'s statement. Acceptance: no `# Panics` in fill.rs, fuse.rs, or grow.rs claims a panic for non-canonical input as such; each states the precondition and the malformed-stream panic separately.

Construction: Hand-build node[node[leaf 5, leaf 5], leaf 7] in the skyline coding (fill/tests.rs's `prescan_raise_shapes` helpers `nd`/`lf`/`pk` build such streams) and call `fill::tick(view, &"(0, 1)".parse().unwrap())` in a release build. The walk copies the left region verbatim, declines the right-full raise (7 > 5), reports `Unchanged`, and `grow::emit` splices the equal pair through `continue_verbatim`. Assert the call returns and `validate(out)` is `Err`; in a debug build note whether a builder `debug_assert!` fires instead. Either outcome contradicts "Panics if ... not canonical".

### skyline-fill-grow-7: `FillWalk`'s doc calls the id reader "the recursion argument" of an iterative walk
- Where: crates/before/src/version/skyline/fill.rs:343-345 (related: crates/before/src/version/skyline/fill.rs:417-431, crates/before/src/version/skyline/fill/prescan.rs:62-64)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (history pass: `git log -S'the recursion argument' -- fill.rs` returns 78206426, the recursive fusion; 05bd2b16 converted the walk to explicit stacks; fill.rs:420 opens "Iterative"); executed: no
- Seen by: prose (27, one item of the texture sweep); refutation: confirmed; history: expired at 05bd2b16
- Owner-gated: no

The sentence describes the recursive implementation the walk replaced; the method's own doc three screens down opens "Iterative". Principle 5: no ghost references to removed code. Separated from the texture sweep (skyline-fill-grow-14) because this one is a factual ghost, not register.

Evidence:

       343	/// The fill walk: input cursor, relative-height state, the fused changed-flag
       344	/// output, and the route probe. The `&mut` [`IdReader`] threads alongside as
       345	/// the recursion argument, exactly as the packed walks thread theirs.
       420	    /// Iterative: the loop alternates a *descend* phase (process the subtree at

Resolution: "The `&mut` [`IdReader`] threads alongside as [`walk`](Self::walk)'s second cursor, exactly as the packed walks thread theirs." Acceptance: `grep -n recursion fill.rs` returns nothing.

### skyline-fill-grow-17: Kernel-doc test and envelope citations resolve today but no gate leg checks them
- Where: crates/before/src/version/skyline/fill.rs:1099-1101 (related: crates/before/src/version/skyline/fill.rs:78-80, 92-94, 212-214, 234; crates/before/src/version/skyline/fill/prescan.rs:482-483; tools/citecheck:8-13, 77-81)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (the lenses' grep confirmed every cited name exists at HEAD: the `tick_*_envelope` fns and envelope names in tests/meter.rs, `width_circulation_cost`, `tick_is_ticks_one`, `ticks_one_is_tick`, `fill_is_idempotent`; I read tools/citecheck's docstring and its fixed haystack constants `SURFACE`, `DIFF_OPS`, `SURFACE_COVERAGE`, `LAWS`); executed: no
- Seen by: prose (24); refutation: confirmed (the bare-backtick names are not intra-doc links, so rustdoc never resolves them either); history: no-rationale-found (citecheck scopes itself to the roster, the bespoke tiling, and the tripwires; kernel-doc citations are an unexamined boundary)
- Owner-gated: yes (a gate-scope decision)

Production docs cite tests and envelopes by bare identifier: `tick_ownership_hole`/`tick_ownership_comb` (92-94), `tick_is_ticks_one`/`ticks_one_is_tick` (212-214), `fill_is_idempotent` (234), `tick_collapse_hole`/`tick_raise_hole` (1099-1101), `width_circulation_cost` (78), `tick_copy_hole` (prescan.rs:483). `tools/citecheck` resolves citations against the nextest inventory but its haystack is fixed to the roster files; `doclint` checks layout and `testdoc` checks doc presence. A rename of any cited test leaves a ghost reference no gate leg sees, which is exactly the rot citecheck's own docstring calls "the expensive kind".

Evidence:

      1099	            // The `tick_collapse_hole` and `tick_raise_hole` envelopes pin
      1100	            // the block side engaging on deep ranges, one per arm of this
      1101	            // scan (descend-site collapse, ascend-site raise).
    (tools/citecheck)
        78	SURFACE = "src/surface.rs"
        79	DIFF_OPS = "src/testing/diff_ops.rs"
        80	SURFACE_COVERAGE = "src/testing/surface_coverage.rs"
        81	LAWS = "src/laws.rs"

Resolution: Owner's call between (a) extending citecheck's extraction to backticked identifiers in `//`/`///`/`//!` comments under `src/version/skyline/**` that match a collected test's final segment or an envelope name in tests/meter.rs, and (b) reducing kernel-doc citations to the owning module (`tests/meter.rs`'s tick envelopes) so renames inside cannot orphan them. Acceptance: either the citecheck gate leg fails on a deliberate local rename of a cited kernel-doc test (demonstrated once), or no production doc in the partition names an individual test function.

### skyline-fill-grow-21: "mint"/"minted" at eight sites, in three senses
- Where: crates/before/src/version/skyline/fill/memo.rs:4-4 (related: crates/before/src/version/skyline/fill/prescan.rs:378-380; crates/before/src/version/skyline/fill.rs:816-818; crates/before/src/version/skyline/fill/tests.rs:753, 894, 957, 1268, 1298; crates/before/src/version/skyline/watermark.rs:303, 350, 361)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i mint` over the partition returns exactly these eight sites; watermark.rs:303/350/361 define the latent-register sense); executed: no
- Seen by: prose (21); refutation: confirmed; history: contradicts the owner's global writing-style rule (never write "mint" for constructing a value), first tracked 2026-08-19 after every cited site; memo.rs:4's "minted in" is the coining-a-term sense the owner's own Wave 7 ruling used
- Owner-gated: yes (the watermark sense is a crate coinage defined outside this partition)

Three senses: "defined" (memo.rs:4), "allocated"/"produced" (prescan.rs:380 "nothing is minted per resolve"; tests.rs:1268 "minting content no operand paid for"; tests.rs:1298 "the orbit mints expansions"), and watermark.rs's name for a boundary moving into the latent register (fill.rs:817; tests.rs:753, 894, 957). The second sense is the banned one outright; the third collides with it on the same page, so a reader of fill.rs:817 cannot tell which is meant without opening watermark.rs.

Evidence:

    (memo.rs)
         4	//! A left-full site (minted in [`fill`](super)'s module doc: an id node whose
    (prescan.rs)
       378	        // chain_span := (min − m_last) + (m_last − m_first) = min − m_first;
       379	        // the keeper dies into it (its buffer is re-armed for the outer level
       380	        // below — nothing is minted per resolve).
    (fill.rs)
       816	            // the anchor must be exact: a latent parked by a nested site's
       817	            // close retires here (its one death, funded by the mint the input's
       818	            // re-widening climb paid for); the consume cycle's arm has already

Resolution: memo.rs:4 "defined in"; prescan.rs:380 "nothing is allocated per resolve"; tests.rs:1268 "producing content"; tests.rs:1298 "the orbit expands each id site at most once". For the watermark sense (fill.rs:817; tests.rs:753, 894, 957), either link the term to its definition at first use or rename it with watermark.rs (owner's call, outside this partition). Acceptance: `grep -in mint` over the partition returns only sites that link to the watermark definition, or nothing.

**Nits (7), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| skyline-fill-grow-5 | `crates/before/src/version/skyline/fill.rs:205-208` | `# Panics` on tick and ticks documents an empty-id state the `Party` type excludes | Delete the empty-id sentences from `tick`, `ticks`, and `grow::emit`'s docs; keep `emit`'s debug assert | `evidence/partitions/skyline-fill-grow.md` |
| skyline-fill-grow-10 | `crates/before/src/version/skyline/fill.rs:456-459` | Em-dashes in `//` comments (62 sites in the partition) | If the owner rules for the double-hyphen in this crate, one mechanical pass over `//` (not `///`/`//!`) lines replacing ` — ` with ` -- ` or a colon ... | `evidence/partitions/skyline-fill-grow.md` |
| skyline-fill-grow-14 | `crates/before/src/version/skyline/fill.rs:895-897` | Register texture that names no mechanism | 897 drop "genuinely"; 1018 "their size"; memo.rs:88 "charge the heap meter for"; fuse.rs:36 "the route fold's cost"; grow.rs:36 drop "simply" ... | `evidence/partitions/skyline-fill-grow.md` |
| skyline-fill-grow-20 | `crates/before/src/version/skyline/fill/fuse.rs:380-385` | `expand_subtree`'s `# Panics` attributes an unrepresentable state to a normal-form violation | Restate the `# Panics` and the assert message: an `Internal` tag has a present child by the coding (`00` is the terminal) ... | `evidence/partitions/skyline-fill-grow.md` |
| skyline-fill-grow-22 | `crates/before/src/version/skyline/fill/memo.rs:21-25` | "forest parent" and "site forest" are used as terms without a definition | One sentence before the bullets at memo.rs:17: the sites a scan records nest (a site can sit inside another's sibling range) ... | `evidence/partitions/skyline-fill-grow.md` |
| skyline-fill-grow-28 | `crates/before/src/version/skyline/fill/prescan.rs:691-693` | `pop_site`'s comment gives the wrong reason the slot cast is lossless | "A slot is a `queue` index pushed as `u64` at `push_site`, so the round trip to `usize` is exact." At memo.rs:137: `expect("nonzero link count fits u3 ... | `evidence/partitions/skyline-fill-grow.md` |
| skyline-fill-grow-29 | `crates/before/src/version/skyline/fill/tests.rs:1005-1005` | Two line-wrap typos split compound modifiers | "nested-full-sibling id"; "pending-sibling path bits" | `evidence/partitions/skyline-fill-grow.md` |

**Cross-references.** skyline-fill-grow-4 and skyline-sweep-place-masked-13 are the same over-promising `# Panics` form; walk.rs:74-79 is the adopted statement both should cite. skyline-fill-grow-21's watermark sense of "mint" is defined at watermark.rs:303/350/361 (skyline-watermark-1). skyline-fill-grow-1's bracketed readings and skyline-query-30's are the same genre the 500d4d09 sweep excised from meter surfaces. board-ops-render-30 re-denominates fill.rs:150 and fill/tests.rs:14 (the retired determinism tripwire). skyline-fill-grow-24's `usize`/`u64` width contrast points at codec/stack.rs:39-42 as the one convention site.

## The skyline coding: comparison kernels (sweep, place and filter, masked, overlay, signed)

13 findings (0 high, 1 medium, 3 low, 9 nit). Full records: `evidence/partitions/skyline-sweep-place-masked.md`.

### skyline-sweep-place-masked-33: sweep.rs names `Version`'s `PartialOrd` as the verdict oracle, but that is the sweep itself
- Where: crates/before/src/version/skyline/sweep.rs:56-58 (related: crates/before/src/version/skyline/sweep.rs:84-85; crates/before/src/version.rs:1730-1731; crates/before/src/version/skyline/sweep/tests.rs:1-10, 45)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep: `stored-form comparison` occurs only at sweep.rs:56 and :84; version.rs:1730-1731 (and the `&` variants at 1740-1751) implement `partial_cmp` as `skyline::sweep::causal_cmp`; `git show 66b5dc661:crates/before/src/version.rs` has `partial_cmp` calling `self.view().causal_cmp(o.view())` at lines 770-771; sweep/tests.rs:45 computes `want` from `to_oracle_version(a).partial_cmp(&to_oracle_version(b))`); executed: no
- Seen by: prose, claims; refutation: confirmed; history: deliberate but expired (true at 66b5dc66; faf3cd0a made `partial_cmp` the sweep two days later; 73624b27 re-anchored the tests to the recursive oracle the same day but touched sweep/tests.rs only)
- Owner-gated: no

The Testing section and `causal_cmp`'s doc name "the stored-form comparison ([`Version`]'s `PartialOrd`)" as the differential oracle. Today `Version`'s `partial_cmp` is `skyline::sweep::causal_cmp`, so the sentence says the function is tested against itself; the tests use the recursive oracle through the bridge. The phrase names an implementation since retired, breaching the hard rule that nothing refers to code that no longer exists, and it hides the strongest fact about this suite: the oracle shares no cursor, delta, or accumulator with the sweep.

Evidence:

        56	//! The stored-form comparison ([`Version`](crate::Version)'s `PartialOrd`) is
        57	//! the verdict oracle: differential tests pin all four entry points against it

        84	/// verdict matches the stored-form comparison exactly (the module doc's
        85	/// differential suite pins all four outcomes).

    version.rs
      1730	                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
      1731	                    skyline::sweep::causal_cmp(self.view().live(), o.view().live())

Resolution: re-denominate both sentences against what is: the recursive oracle (`oracle::Version`, the paper transcription, reached through `testing::bridge::to_oracle_version`) is the verdict witness, over the exhaustive small scope, the generator families, and the organic histories, as sweep/tests.rs:1-10 already says. Drop "stored-form comparison" from sweep.rs. Acceptance: `grep -n 'stored-form comparison' sweep.rs` returns nothing; the Testing section names the oracle sweep/tests.rs calls.

### skyline-sweep-place-masked-13: `OpenedPair::open` and `BoundSide::open` overstate their Panics relative to the walk they wrap
- Where: crates/before/src/version/skyline/overlay.rs:697-699 (related: crates/before/src/version/skyline/place.rs:147-149; crates/before/src/version/skyline/overlay.rs:331-336; crates/before/src/version/skyline/masked/tests.rs:25-72)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate but expired (the short form was the original contract wording; a736ef14 and 0a534fe0 replaced it with the precise noticed/silent split on the cursor methods and missed these two constructors)
- Owner-gated: no

Both constructors say they panic if a stream is not a canonical skyline encoding. They call `LeafCursor::open`, whose Panics section says only structurally noticed violations panic and the rest walk silently, and `collapsible_sibling_pair_sweeps_without_panicking` drives a non-canonical stream through `OpenedPair::open` (via `Walk::open`) without a panic. The module now carries two incompatible spellings of one contract.

Evidence:

       697	    /// # Panics
       698	    ///
       699	    /// Panics if either stream is not a canonical skyline encoding.

    place.rs
       147	    /// # Panics
       148	    ///
       149	    /// Panics if the stream is not a canonical skyline encoding.

Resolution: replace both with a citation of the one statement: "[`LeafCursor::open`]'s canonical-stream contract, on each stream." Acceptance: neither constructor claims an unconditional panic on non-canonical input.

### skyline-sweep-place-masked-25: `coverage`'s `finish` doc states a discipline its hole emptiness arms do not keep
- Where: crates/before/src/version/skyline/place/filter.rs:463-476 (related: crates/before/src/version/skyline/place/filter.rs:410-425, 482-492)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read the walk arms and `finish`); executed: no
- Seen by: correctness; refutation: confirmed; history: already known (ed77a9f7 recorded the decision: "finish's emptiness arms legitimately read settled pairs' relations", choosing the comment over making `Pair::relation` refuse when not live; aa7c96a0's doc rewrite narrowed the paragraph to required pairs and dropped that clause, producing the mismatch)
- Owner-gated: no

The doc says only a pair alive at exhaustion answers by its decided relation and that the stale-direction agreement "is not part of the contract". For `NotBefore`/`NotStrictlyBefore` the walk settles `hi` alone when `hi <= bound` is refuted (411-413) while `lo` stays live, so the side survives to `finish`, whose emptiness arms read `side.hi.pair.relation()` unguarded (482, 487-488). The answer is right because refutation is permanent, which is precisely the argument the doc disclaims. The recorded decision is that the emptiness arms read settled pairs by design, so the fix is to restore that clause, not to add guards.

Evidence:

       465	/// Division of labor with the walk: a settled *required* pair's refutation
       466	/// already lives in `full_possible`, so the `!live` guards on the two
       467	/// required arms keep `finish` from consulting a settled pair's stale
       468	/// `directions`. The stale directions would happen to agree (a
       469	/// settle-direction refutation is permanent), but that agreement is not part
       470	/// of the contract: only a pair alive at exhaustion answers by its decided
       471	/// relation. A hole needs no guard on its fullness endpoint: a surviving

       482	        let (lo, hi) = (side.lo.pair.relation(), side.hi.pair.relation());
       ...
       487	            Demand::NotBefore => matches!(hi, Some(Ordering::Less | Ordering::Equal)),
       488	            Demand::NotStrictlyBefore => hi == Some(Ordering::Less),

Resolution: rewrite the paragraph to say that the hole emptiness arms read their emptiness endpoint whether or not it is settled, relying on the permanence of refutation (a settled emptiness endpoint has its emptying direction refuted, so `false` is the decided answer), and that the `!live` guards on the two required arms exist because those arms' refutations already live in `full_possible`. Acceptance: the doc and the six emptiness arms agree on the argument each uses; `filter_coverage_matches_the_composed_sweeps` and `filter_coverage_organic_witnesses` stay green.

### skyline-sweep-place-masked-36: Hand-maintained caller rosters and counts that have already drifted
- Where: crates/before/src/version/skyline/sweep.rs:191-193 (related: crates/before/src/version/skyline/overlay.rs:230-233; crates/before/src/version/skyline/masked.rs:19-20, 323, 335; crates/before/src/version/skyline/emit.rs:211; crates/before/src/version/skyline/place/filter.rs:91-94)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: emit.rs:211 `let mut directions = Directions::new();` and filter.rs:93 `directions: Directions,` are clients the roster omits; masked.rs:323 and :335 `let mut net = Accumulator::new();`); executed: no
- Seen by: prose; refutation: confirmed (the masked count is defensible as "at most three persistent", which is exactly the rot the doctrine forbids); history: deliberate but expired (bb6f083d's roster was accurate for a day; emit.rs adopted `Directions` in e4d4817f the next day and filter's `Pair` in db9dfa3e; c6ba2208 already set the house precedent of trimming client inventories: "each client module names what it consumes")
- Owner-gated: no

`Directions`' doc enumerates its clients as "this module's sweep, the masked co-walk, the placement walk's bound sides"; emit.rs and filter.rs's `Pair` are a fourth and fifth client. overlay.rs:233 asserts "which no current client does", a census of callers. masked.rs:19-20 counts the transient state as "three accumulators" while `block_skip` allocates a fourth (`net`) per block. Principle 5: no hand-maintained counts or caller rosters; two of these three have already rotted.

Evidence:

       191	/// Every comparison walk — this module's sweep, the masked co-walk, the
       192	/// placement walk's bound sides — folds one sign per elementary interval into
       193	/// this pair and asks its question of the survivors.

    overlay.rs
       230	    /// Semantically the choice is free only because every client algebra folds
       231	    /// commutative sums — any order yields the same fold values; a client with
       232	    /// a non-commutative fold would make the tie-break part of its answer,
       233	    /// which no current client does. The order is contract, not convenience: a

    masked.rs
        19	//! slot's step folds). Nothing recurses, and the transient state is the
        20	//! cursors' path bits plus three accumulators.

Resolution: sweep.rs:191-193, "Every comparison walk folds one sign per elementary interval into this pair ..." with no list; overlay.rs:233, replace the census with the requirement ("so a client's fold must be commutative"); masked.rs:20, "plus a constant number of accumulators" or name them structurally (one difference, at most one height integrator per masked side, a per-block net during a skip). Acceptance: no client list on `Directions`; the priority doc states the commutativity requirement; the masked module doc states no number that `block_skip`'s `net` falsifies.

**Nits (9), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| skyline-sweep-place-masked-1 | `crates/before/src/version/skyline/masked.rs:110-111` | Rustdoc link syntax inside a `//` comment, and a ragged module-doc wrap | write `// (Directions::relation's map).` and rewrap lines 38-46 to the module's width | `evidence/partitions/skyline-sweep-place-masked.md` |
| skyline-sweep-place-masked-6 | `crates/before/src/version/skyline/masked.rs:384-384` | `unreachable!` messages that are slot counts, not proofs | at all four sites, "slot indices come only from `priority`, which names the constants above" | `evidence/partitions/skyline-sweep-place-masked.md` |
| skyline-sweep-place-masked-8 | `crates/before/src/version/skyline/overlay.rs:71-71` | Colon-fronted fragments open body paragraphs | rewrite as sentences ("The bounds are derived rather than measured: ..."; "A *plateau* is one maximal constant run ..." ... | `evidence/partitions/skyline-sweep-place-masked.md` |
| skyline-sweep-place-masked-9 | `crates/before/src/version/skyline/overlay.rs:331-336` | One Panics paragraph copied seven times in overlay.rs, each copy saying it is stated once elsewhere | state the contract once on each cursor struct's doc (`LeafCursor` at 302, `IdLeafCursor` at 460) and reduce every method's section to one line citing  ... | `evidence/partitions/skyline-sweep-place-masked.md` |
| skyline-sweep-place-masked-16 | `crates/before/src/version/skyline/place.rs:258-260` | Em-dashes in `//` comments at 25 sites | rewrite the 25 sites with colons, semicolons, parentheses, or spaced double-hyphens; the grep above enumerates them | `evidence/partitions/skyline-sweep-place-masked.md` |
| skyline-sweep-place-masked-24 | `crates/before/src/version/skyline/place/filter.rs:152-154` | Read-order prose names a write-sequence effect that independent accumulators cannot have | at filter.rs:152-154, :318-319, and place.rs:459-463, replace "fixes the accumulator write sequence" / "so every question's accumulator traffic is ide ... | `evidence/partitions/skyline-sweep-place-masked.md` |
| skyline-sweep-place-masked-29 | `crates/before/src/version/skyline/signed.rs:1-3` | Texture coinages used as jargon: currency, face, genre, "block consume", "real", point "tripwire" | currency to "representation" or "exchange form" (signed.rs already anchors "exchange pair/shape" to `Signed`); face to "form" or "entry point" ... | `evidence/partitions/skyline-sweep-place-masked.md` |
| skyline-sweep-place-masked-30 | `crates/before/src/version/skyline/signed.rs:12-14` | signed.rs says "the tests pin the bijection", but the bijection test lives in skyline/tests.rs | move `zigzag_is_a_bijection_without_negative_zero` (skyline/tests.rs:300-331) into signed/tests.rs, or ... | `evidence/partitions/skyline-sweep-place-masked.md` |
| skyline-sweep-place-masked-34 | `crates/before/src/version/skyline/sweep.rs:123-127` | The debug-assert rationale paragraph is copied six times across sweep, masked, and place | keep every assert and each site's first sentence (the site-specific control-flow fact); delete the "keeps that argument loud .. ... | `evidence/partitions/skyline-sweep-place-masked.md` |

**Cross-references.** skyline-sweep-place-masked-33's "stored-form comparison" oracle is flag-day residue of the same family as envelopes-a-1 and meter-registry-tier2-14. skyline-sweep-place-masked-13 pairs with skyline-fill-grow-4; -9 with skyline-coding-35; -34 with the owner rulings aac1bc04/faa31262/b0194c6a on the asserts themselves. skyline-sweep-place-masked-29's "currency" collision is also skyline-coding-1's and codec-bits-5's. skyline-sweep-place-masked-24 and -25 concern the filter walk whose cost contract is skyline-sweep-place-masked-21 (the claim finding).

## The skyline coding: query (query, integral, web)

14 findings (0 high, 0 medium, 7 low, 7 nit). Full records: `evidence/partitions/skyline-query.md`, `evidence/sweeps/recursion.md`.

### skyline-query-10: `mass_split`'s doc overstates the right half's bound in the clamped case
- Where: crates/before/src/version/skyline/query/integral.rs:292-294 (related: query/integral.rs:301-304; query/tests.rs:1334-1335)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (hand computation of the construction against integral.rs:302-303); executed: no
- Seen by: correctness; refutation: confirmed, and adds that tests.rs:1334-1335 repeats the clause while the test asserts only nonempty halves and the depth bound; history: no-rationale-found (982bd260 derived "right half <= floor(M/2)" without the clamp case)
- Owner-gated: no

The doc says "the right half is at most half the node's mass", but when no prefix inside `(lo, hi)` reaches the target the `.min(hi - 1)` clamp makes the right half a single heavy leaf exceeding half. The pinned depth bound survives (that half is one leaf and the recursion ends there), and the preceding clause ("within half the node's plus one leaf's") is accurate; only this lemma is false, and a maintainer re-deriving the bound from the doc would reach a false step (Principle 5).

Evidence:

    292  /// internal node's range; the returned `mid` satisfies `lo < mid < hi`. Each
    293  /// half's mass stays within half the node's plus one leaf's: the right half is
    294  /// at most half the node's mass, and the left half exceeds half only by mass

    (tests.rs:1334-1335)
    1334      /// naive recursive reference expanding the same rule: the right half
    1335      /// never exceeds half the node's mass, and the left half exceeds it only

Resolution: Amend both sentences: "the right half is at most half the node's mass unless the clamp made it a single leaf, in which case it is that leaf and the recursion ends there". Acceptance: the doc's statement holds on masses `[1, 1, 100]`.
Construction: `mass_split(&[0, 1, 2, 102], 0, 3)`: `target = (0 + 102).div_ceil(2) = 51`; `prefix[1..3] = [1, 2]`, `partition_point(p < 51) = 2`; `lo + 1 + 2 = 3`, clamped by `min(hi - 1 = 2)` to `2`; the right half `[2, 3)` has mass 100 > 51.

### skyline-query-16: `Arming` (a ledger entry) collides with the watermark web's "arming" of a range, with no contrast drawn
- Where: crates/before/src/version/skyline/query/integral.rs:584-587 (related: query.rs:108, 131; query/web.rs:30, 302-303; watermark.rs:105)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (grep: watermark.rs:105 is the section header "# The arming paths"; `struct Arming` at integral.rs:587); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found (the web sense predates; `struct Arming` arrived in ceb9f330; the lexicon re-ruling in a736ef14 did not notice the collision)
- Owner-gated: no

In `integral.rs` and the `query.rs` module doc an "arming" is one promotion recorded in the ledger; in `web.rs` and `watermark.rs` "arming" is pushing a pending range's boundary onto the web. The two senses live under one parent module and neither definition site mentions the other, so a reader who learns the ledger sense reads `web.rs`'s "range arms above it" with the wrong referent. The vocabulary rule asks for a definition by contrast where a neighbor shares the word.

Evidence:

    584  /// One promotion, recorded at its freeze and settled once at the sweep's close:
    585  /// the promoted parked component and the window of interval mass that separates
    586  /// it from the previous promotion.
    587  pub(super) struct Arming {

    (web.rs:30)  //! range arms above it — rides the interrupting boundary as its payload and

Resolution: Either one sentence of contrast at `Arming`'s definition ("an arming here is a ledger entry; the watermark web's arming of a pending range is unrelated") and the mirror sentence in web.rs's module doc, or rename the struct to `Promotion` (its own doc's noun; the field is already `promotions: Vec<Arming>`), leaving the registry family names untouched. Acceptance: each module's first use of "arming" is unambiguous to a reader of that module alone.

### skyline-query-22: Hand-maintained caller and client counts, one already false in the test build
- Where: crates/before/src/version/skyline/query/web.rs:128-129 (related: query/web.rs:20; query/integral.rs:473-474, 901, 1046; query/tests.rs:1447, 1627, 1645, 1660, 1965, 1983, 2560)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `both callers|the one caller|both its clients|both consumers` over the four files; grep of `mul_into(` shows two production callers and seven in tests.rs); executed: no
- Seen by: structure, prose, correctness; refutation: confirmed; history: no-rationale-found (web.rs:128 and integral.rs:473 were written in aa7c96a0 two days after the doctrine line naming "both callers" entered the dotfiles; 1046 in 9c7b6999 the same day; 901 and web.rs:20 predate the rule)
- Owner-gated: no

Doctrine names "both callers" as the forbidden example of a hand-maintained count. `mul_into`'s "Both callers hand in nonzero counts" is already false: the adequacy kernels call it with segment and position masses (tests.rs:1645-1651 passes a possibly-zero segment) and rely on the zero-operand fall-through.

Evidence:

    128      // Both callers hand in nonzero counts (a settled reign counts at least
    129      // its one close; the ledger settle skips zero suffixes), and a zero

    (integral.rs:473-474)
    473      // clusters, so the loop is the no-op it should be, and both callers
    474      // (the segment settles and the aggregate merges) already skip
    (integral.rs:901)      /// both consumers, priced by the segment's depth variation.
    (integral.rs:1046)     // debt. The one caller tests this immediately above the call, so the
    (web.rs:20)            //! held once for both its clients; this module drives it through [`ReignWeb`]

Resolution: State the contract, not the tally: "Callers pass nonzero counts; a zero `digits` operand is a no-op by the empty digit walk" (web.rs:128); "callers skip zero-valued factors at the sign reads that price them" (integral.rs:473); "The caller gates the call" (1046); "held once for its clients" (web.rs:20); "one watermark read serving the settle and the banked window" (901). Acceptance: the grep returns nothing over the partition.

### skyline-query-29: Test comment attributes production stack safety to `crate::recurse::descend!`, which production code does not use
- Where: crates/before/src/version/skyline/query/tests.rs:1165-1168 (related: crates/before/AGENTS.md:31-37; query.rs:212-221, 440-471)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'descend!' crates/before/src` excluding tests.rs files, src/testing/, and src/recurse.rs returns nothing); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found (inaccurate when written in 013334f2, not expired)
- Owner-gated: no

The production folds are iterative (`rank` and `min_ticks` are `loop`/`while` walks; the settle tree runs on an explicit stack), and AGENTS.md states that `descend!` guards only test surfaces. A reader following this parenthetical looks for a guard in `query.rs` that is not there (Principle 5).

Evidence:

    1165      // The recursive oracle and its bridge are test-only plain recursion on tree
    1166      // depth, and the dense masses here run the spine thousands of levels deep —
    1167      // the production folds are stack-safe (`crate::recurse::descend!`), so the
    1168      // headroom is for the witnesses, not the code under test.

Resolution: "the production folds are iterative (the crate's recursion rule: depth lives on explicit stacks), so the headroom is for the witnesses, not the code under test." Acceptance: the comment names no `descend!` and the grep stays empty.

### skyline-query-30: Eight adequacy test docs carry bracketed measured readings the sibling envelope suite keeps in pin commits
- Where: crates/before/src/version/skyline/query/tests.rs:1491-1493 (related: query/tests.rs:1802-1804, 1829-1832, 2165-2167, 2192-2195, 2404-2407, 2636-2639, 2675-2678; tests/meter.rs:5222-5225)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `measured in the dev profile` finds the eight sites; tests/meter.rs:5222-5225 read); executed: no
- Seen by: correctness, claims; refutation: confirmed, severity low -> nit; history: deliberate-but-expired (d2a9d04e kept the brackets and stripped their dates; 500d4d09 then ruled that readings live in pin commits and swept the meter surface, but its sweep did not reach query/tests.rs)
- Owner-gated: no

Each tripwire doc records touch or limb counts and byte sizes from one profile run to justify its floor as "midway between linear and the measured growth". The floors are enforced; the tallies are prose restatements that rot with any generator or accumulator change, while the `eprintln!("MEASURED ...")` lines already print the live reading. The crate's own convention of record for the same kind of number (tests/meter.rs:5222-5225) is "the record and every re-pin's movement live in the pin commits" (Principle 5: dated measurement reports are not exempt).

Evidence:

    1491      /// [measured in the dev profile, exact counters: touches 124,368 -> 372,859
    1492      /// across FP(1,000) -> FP(2,000), packed 73,328B -> 146,579B: per-byte
    1493      /// growth x1.50.]

    (meter.rs:5223-5224)
    5223      /// wide-arming family, measured ×1.25 (the record and every
    5224      /// re-pin's movement live in the pin commits).

Resolution: Move the eight bracketed records to the pin commits and keep the derivation sentence ("the floor sits between linear and the measured growth; the record lives in the pin commit"). Acceptance: `grep -n 'measured in the dev profile' tests.rs` returns nothing and each floor's doc still says what the floor sits between.

### skyline-query-32: "retired" and "fell into" narrate history where the siblings state the present
- Where: crates/before/src/version/skyline/query/tests.rs:2446-2447 (related: query/tests.rs:2433, 1388, 1517, 1854, 2007, 2220; query/integral.rs:144-146)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `\bretired\b|fell into` over the four files; the sibling demonstrators say "refuted" at the related sites); executed: no
- Seen by: prose; refutation: confirmed (`Close::Retired` is live domain vocabulary in a different sense and stays); history: contradicts-hard-rule for "retired" (root AGENTS.md's ghost-reference rule of 2026-07-21 predates 016b91c4's "The retired per-digit charge"; the kernel was the shipped settle move until that commit, so "retired" does the work of "superseded"); "fell into" narrates a past incident but the composed form still exists as the test oracle, so that half is a wording fix
- Owner-gated: no

"Retired" says this kernel was formerly the shipped path, which is provenance that lives in git; the sibling demonstrators use the present-tense "refuted". This is a breach of the ghost-reference hard rule by its letter (no "formerly"/"superseded"); its purpose (no dangling references to deleted code) is not harmed because the kernel exists as the demonstrator, which is why the severity is low rather than high. integral.rs:144 narrates the composed form's failure in the past tense.

Evidence:

    2446      /// The retired per-digit charge: one `parked`-wide product per
    2447      /// balanced digit of the mass.

    (tests.rs:2433)      // `integral` module doc's settle bound). This kernel keeps the retired
    (integral.rs:144-146)
    144  //! operand that did not deposit — the hole the composed form fell into, where
    145  //! the meet's emission re-coded one operand's width into switch jumps that
    146  //! the integral then evicted at the other operand's cheap codes, priced by a

Resolution: tests.rs:2433 and 2446: "refuted". integral.rs:144: "the hole a composed form falls into: the meet's emission re-codes one operand's width into switch jumps that the integral then evicts at the other operand's cheap codes". Acceptance: `grep -n -E '\bretired\b|fell into'` over the partition matches only the `Close::Retired` variant.

### recursion-3: Test comment credits production folds to descend!; the fat-stack thread is missing from the oracle envelope's bound list
- Where: crates/before/src/version/skyline/query/tests.rs:1165-1172 (related: crates/before/src/oracle.rs:25-27, crates/before/src/version/skyline/query/integral.rs:1037-1038, crates/before/src/recurse.rs:9-11)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for `descend!` shows no production use; recurse.rs:118 gates the macro `#[cfg(test)]`; `git blame` dates the comment to d3a029d411, 2026-08-06, after the guard became test-only in 1ddb5a483, 2026-07-31); executed: no
- Verification: confirmed; history: no-rationale-found (the comment was inaccurate when written)
- Owner-gated: no

The comment says the production folds are stack-safe via
`crate::recurse::descend!`, but the folds are iterative on explicit stacks
(integral.rs:1038 says so) and `descend!` is `cfg(test)` and appears in no
production code. The same test bounds the recursive oracle by spawning a
256 MiB stack thread, a mechanism oracle.rs's list of harness input bounds
does not name, so the envelope claim "it is the harnesses that bound their
inputs" does not describe this leg. A comment naming the wrong mechanism sends
a reader auditing the recursion rule to look for `descend!` in the folds and
find none.

Evidence:

      1165	    // The recursive oracle and its bridge are test-only plain recursion on tree
      1166	    // depth, and the dense masses here run the spine thousands of levels deep —
      1167	    // the production folds are stack-safe (`crate::recurse::descend!`), so the
      1168	    // headroom is for the witnesses, not the code under test.
      1169	    let body = std::thread::Builder::new()
      1170	        .stack_size(256 << 20)
      1171	        .spawn(dense_factor_tier_legs)
      1172	        .expect("the fat-stack witness thread spawns");

    integral.rs:
      1038	    /// iterative on explicit stacks per the crate's recursion rule, and it

    oracle.rs:
        25	//! the definition it exists to transcribe. Each oracle-facing suite therefore
        26	//! carries its own input bound (generator recursion caps, enumeration depth
        27	//! constants, op-trace length caps, family scale caps), and depth-stress

Resolution: Reword the comment: the production folds are iterative on explicit
stacks (integral.rs's settle), so the headroom is for the recursive oracle and
bridge witnesses only. Add the fat-stack witness thread to oracle.rs:25-27's
list of bound kinds, or cap this leg's tree so the default test stack suffices
and drop the thread. Acceptance: the comment names the iterative folds, and
oracle.rs's list covers every mechanism an oracle-facing harness uses.

**Nits (7), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| skyline-query-1 | `crates/before/src/version/skyline/query.rs:121-123` | Unanchored crate-dialect terms "seam" and "genre" in this partition's prose | Rule once at crate scope. If the words stay, define each once by contrast at one home (the crate's vocabulary section or the first use) and link to it ... | `evidence/partitions/skyline-query.md` |
| skyline-query-4 | `crates/before/src/version/skyline/query.rs:388-390` | Em-dashes in `//` comments and in one assert message | Rule once at crate scope; if the doctrine applies, sweep `//` comments to ` -- ` or restructure with a colon ... | `evidence/partitions/skyline-query.md` |
| skyline-query-11 | `crates/before/src/version/skyline/query/integral.rs:343` | `clusters`' "ascending" precondition must be strict; the gap subtraction underflows on a repeated index | Say "strictly ascending" at 324-325, 441, and 603, and extend `charge_digits`' debug_assert loop to check each index exceeds its predecessor | `evidence/partitions/skyline-query.md` |
| skyline-query-12 | `crates/before/src/version/skyline/query/integral.rs:356-359` | Moralized and significance wording: "honest", "real", "is the point" | Name the property at each site; drop "is the point" and state the claim; restate integral.rs:272 as the work bound | `evidence/partitions/skyline-query.md` |
| skyline-query-19 | `crates/before/src/version/skyline/query/integral.rs:980-981` | "sound" used loosely for "correct only when" and "holds" | integral.rs:980 "Correct only immediately after ..."; tests.rs:1418 and 1524 "the identity ... holds" | `evidence/partitions/skyline-query.md` |
| skyline-query-20 | `crates/before/src/version/skyline/query/integral.rs:1044-1051` | Comment says the emptiness check is not re-taken here, directly above a debug_assert that re-takes it | "The caller gates the call on a non-empty ledger; this restates the precondition in debug builds." Acceptance: comment and code agree on whether the c ... | `evidence/partitions/skyline-query.md` |
| skyline-query-26 | `crates/before/src/version/skyline/query/tests.rs:341` | A fullwidth left parenthesis (U+FF08) in the `zero_drift_heights` doc | Replace `（` with `(` | `evidence/partitions/skyline-query.md` |

**Cross-references.** skyline-query-29 and recursion-3 are the same comment at query/tests.rs:1165-1168 crediting production folds to `descend!`. prose-hygiene-5's `Reign::mint` identifier is web.rs:190 in this module. skyline-query-16's `Arming` collision is with watermark.rs:105 (skyline-watermark). skyline-query-30's bracketed readings and skyline-fill-grow-1's are one genre. skyline-query-22's "both callers" is the doctrine's own named example of a hand count.

## The skyline coding: watermark and the traffic counters

10 findings (0 high, 0 medium, 3 low, 7 nit). Full records: `evidence/partitions/skyline-watermark.md`.

### skyline-watermark-4: Hand-maintained 'two' restates FOLLOWER_SLOTS, once inaccurately
- Where: crates/before/src/version/skyline/watermark.rs:93-95 (related: watermark.rs:147-149, 236; fill.rs:182-188)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read; the slot-iterating sites are `park` 367-375, `resolve_latent` 418-426, `drop_below` 530-540, `push_boundary` 639-644, the dominated arm 975-977, and `close`'s assert 328; `undercuts_here` 452-463 touches no slot); executed: no
- Seen by: prose, claims; refutation: confirmed (with the fuller site list); history: contradicts-hard-rule (CLAUDE.md "No hand-maintained counts", 2026-08-10; the prose is c29bd2b3's, `FOLLOWER_SLOTS` arrived the same day in a736ef14 without re-denominating it)
- Owner-gated: no

Doctrine: no hand-maintained counts; a number that matters lives in a mechanically enforced place the prose cites by name. `FOLLOWER_SLOTS` is that place, and the prose has already drifted: a min-ticks leaf that neither arms nor undercuts passes through no follower loop, so "two `None` checks per event" is false.

Evidence:

        93	//! the relation and never reads it. Only the fill walk installs any (two, for
        94	//! relations named in `fill.rs`; the min-ticks fold installs none and pays two
        95	//! `None` checks per event).
       147	/// Follower slots the web carries (the fill walk's two relations; a const
       148	/// assert beside the fill walk's slot constants binds the two rosters).
       236	            followers: [None, None],

Resolution: 93-95 "Only the fill walk installs any (`FOLLOWER_SLOTS` of them, for the relations named in `fill.rs`); the min-ticks fold installs none and pays one `Option` check per slot at each arm, undercut, park, and collapse." 147-148 "Follower slots the web carries; a const assert beside the fill walk's slot constants binds the two rosters." 236 `followers: [const { None }; FOLLOWER_SLOTS]` (stable on the pinned 1.97.1 toolchain). Acceptance: widening `FOLLOWER_SLOTS` needs no prose edit and no change at 236; the cost statement matches the code paths that iterate the slots.

### skyline-watermark-8: compacting()'s doc carries measured counterfactual ratios and misstates the saving's mechanism
- Where: crates/before/src/version/skyline/watermark.rs:245-256 (related: watermark.rs:220-227; tests/meter.rs:7125-7140, 8363; .cargo/mutants.toml:70-76; crates/suanpan/src/accumulator.rs:103-155)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified for the ratios' provenance (`git show cedb6015`: "623,408 B heap (x1.41) and 32,027 touches (x2.0)" measured under a manual swap; `compacting()` is the only min-ticks constructor at query/web.rs:231, so nothing committed reproduces them) and for the roster cross-reference (.cargo/mutants.toml:74-75 "the compacting doc carries the measured margins"); assessed for the layout point (Accumulator's fields at accumulator.rs:103-155 are `Option<i128>`, `Vec<i64>`, two `usize`, `BTreeMap`; `Boundary` is a by-value enum inside `Entry<P>`, so both variants occupy the same inline size; `size_of` not measured); executed: no
- Seen by: prose ([19]), correctness ([29]), claims ([34]); refutation: confirmed; history: deliberate-and-holds for placement (cedb6015 put the ratios in; 9aec9aa2 adjudicated the delete-field mutant "in the doc's favor"; the mutants roster reads them), so the relocation is owner-gated
- Owner-gated: yes: the ratios are the recorded adequacy evidence for a mutant cargo-mutants cannot filter, and .cargo/mutants.toml:74-75 cites the doc as carrying them

Prose speaks in the present tense: the ×1.41 and ×2.0 figures are a dated measurement of an implementation that does not exist in the tree, quoted at a declaration site, and re-pinning `skyline_min_ticks_ascend` silently invalidates them. The mechanism sentence is imprecise in a way the next optimizer would trip on: `Word(u64)` and `Wide(Accumulator)` occupy the same inline footprint, so the saving is the retired accumulator's digit buffer circulating through the pool (and the O(1) word fold at propagate), not "an inline word ... instead of an accumulator entry". Separately, `new()`'s doc at 226-227 cites "the `width_circulation_cost` and memo modules"; the module is `memo_resolution_cost` (tests/meter.rs:8363).

Evidence:

       246	    /// boundaries stack, in two currencies: per-boundary transient storage
       247	    /// (an inline word per stacked difference instead of an accumulator
       248	    /// entry) and undercut propagation (a residue consumes each word
       253	    /// un-compacted storage reads ×1.41 that row's pinned peak heap and
       254	    /// ×2.0 its pinned touches, over both ceilings. Shapes whose stacked

Resolution: reword the mechanism as "compaction retires a word-scale boundary's accumulator to the pool at the push, so a stack of word-scale boundaries circulates one digit buffer instead of holding one per entry, and an undercut consumes each word boundary by one O(1) fold; the `skyline_min_ticks_ascend` row is the enforcing envelope, and deleting compaction trips both its heap and touch ceilings". Either drop the ratios (they live in cedb6015) and re-point .cargo/mutants.toml:74-75 and tests/meter.rs:7137-7139 at the row itself, or make the demonstration committed (a `#[cfg(test)]` constructor toggle and a red-first assertion that the un-compacted web exceeds the row's ceilings), at which point the numbers live in that test. Name `memo_resolution_cost` at 227. Acceptance: no measured ratio at the declaration site or a committed test producing it; the mechanism sentence names the digit buffer; every cited module name resolves by grep.

### skyline-watermark-13: push_boundary's doc says only the pushed-above arm constructs the payload; the undercut arm does too
- Where: crates/before/src/version/skyline/watermark.rs:628-631 (related: watermark.rs:553-555, 663-668; query/web.rs:308-315)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read: line 664 is `on_die(payload());`; `arm_at_height`'s doc at 553-555 names both constructing arms; the min-ticks payload closure at query/web.rs:309-313 swaps the reigning record, so the call does work); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found (inaccurate from c29bd2b3)
- Owner-gated: no

Private rustdoc must be accurate against today's code: a maintainer trusting this sentence would take the `Less` arm's `payload()` for a redundant construction and remove it, leaving the dead record reigning in the min-ticks fold.

Evidence:

       628	    /// residue). Only the pushed-above arm mints the payload; an arming
       629	    /// undercut's payload dies by `on_die` before the residue drives outward,
       630	    /// and an exact meet touches no payload at all — the reigning state
       631	    /// continues.
       663	            Ordering::Less => {
       664	                on_die(payload());

Resolution: "Two arms construct the payload: the pushed-above arm stacks it beside the new difference, and an arming undercut hands it straight to `on_die` before the residue drives outward; an exact meet touches no payload at all — the reigning state continues." Acceptance: the `push_boundary` and `arm_at_height` docs agree and name the same two arms as lines 655 and 664.

**Nits (7), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| skyline-watermark-1 | `crates/before/src/version/skyline/watermark.rs:44-1144` | Vocabulary sweep: 'mint' for constructing a value, moralized 'honest', shouted 'MOVES', 'genuinely' | 66 "defines"; 129, 553, 628 "constructs"; 303 "creating it"; 350 "the fresh-latent move"; 361 "a fresh latent finds them `m`-exact" ... | `evidence/partitions/skyline-watermark.md` |
| skyline-watermark-2 | `crates/before/src/version/skyline/watermark.rs:55-70` | The emission bullet of the cost discipline is one sixteen-line sentence with rewrap residue | split into three sentences (the amortized sign read against the anchor; the O(1) latent decision by top-index domination with comparable scales foldin ... | `evidence/partitions/skyline-watermark.md` |
| skyline-watermark-5 | `crates/before/src/version/skyline/watermark.rs:113-115` | Two maintainer docs misattribute which operand a fold reads or a reader mutates | 115 and 586 "the offset `gap_old − below` costs one fold of `below`'s width, which the caller priced and which survives as the new `gap`" ... | `evidence/partitions/skyline-watermark.md` |
| skyline-watermark-9 | `crates/before/src/version/skyline/watermark.rs:282-290` | fold_height's armed guard has its why only in .cargo/mutants.toml | add one clause to the doc: "An unarmed web skips the fold: its `gap` is replaced wholesale at the first arming ... | `evidence/partitions/skyline-watermark.md` |
| skyline-watermark-10 | `crates/before/src/version/skyline/watermark.rs:491-811` | Em-dashes in // line comments at eight sites | replace each with a colon, semicolon, or sentence break. Seven of the eight sit above line 790 ... | `evidence/partitions/skyline-watermark.md` |
| skyline-watermark-15 | `crates/before/src/version/skyline/watermark.rs:850-853` | compact's 'anything wider can never fit' overstates suanpan's collapse bound | "The width test reads the digit count alone: a difference spelled in more than two digits after its sign read is at least `2^64 − 2^32` (the dominatio ... | `evidence/partitions/skyline-watermark.md` |
| skyline-watermark-25 | `crates/before/src/version/skyline/pool_traffic.rs:4-18` | Vocabulary collisions: 're-arm' for lease and 'fill phase' beside the fill walk | watermark.rs:82 "return to a pool and are leased again cleared"; pool_traffic.rs:5 "and leases from it (`MinWeb::lease`)" ... | `evidence/partitions/skyline-watermark.md` |

**Cross-references.** skyline-watermark-8's ratios are cited by `.cargo/mutants.toml:74-75` and tests/meter.rs:7137-7139 (envelopes-b-12), so the three move together; the owner decision is Open questions 16. skyline-watermark-10's em-dash sweep moves the line-pinned exclusion `watermark.rs:790:43` and should land with skyline-watermark-14 (another class). skyline-watermark-25's "fill phase" recurs at tests/meter.rs:9434-9452 and src/meter.rs:2824; its "re-arm" collides with the promotion re-arm family (src/meter.rs:1683-1802) and with skyline-query-16's ledger sense. envelopes-b's open question 8 relayed the compacting() ratios; open question 7 relayed meter-core-10's `arming_train` off-by-one.

## The codec: bits (bits, buf, build, code, cursor, dsi, gamma, int, literal, scan, stack)

11 findings (0 high, 0 medium, 8 low, 3 nit). Full records: `evidence/partitions/codec-bits.md`.

### codec-bits-1: The identity-ladder essay decides for operations outside the module, without saying it is their home
- Where: crates/before/src/codec/bits.rs:8-55 (related: crates/before/src/version.rs:93, 431, 995, 1006, 1225)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (read bits.rs:8-55; `grep -n '\brung' crates/before/src/version.rs` shows the sites stating their own choice; `git log -1 306e2de0` message); executed: no
- Seen by: structure [6], prose [19]; refutation: confirmed; history: deliberate-and-holds (306e2de0: "The codec::bits module doc now carries the whole ladder rationale (which rung belongs where, and why)"), the rationale lives only in history
- Owner-gated: yes (a recorded design decision on where the policy lives)

The storage module's doc names which operations in `version.rs` and `party.rs` take which rung and says each site states its choice; the sites do (version.rs:93, 431, 995, 1006, 1225), so the per-operation outcomes are written twice with no mechanical tie between the copies, and nothing in `bits.rs` says the essay is the designated home. A rung adopted or dropped at a call site leaves the essay wrong with nothing to catch it (Principle 5: enumerations the code can change without touching the prose; documentation altitude: a module states its own contract).

Evidence:

        14	//! walking. The decision rule, applied per call site (each site cites its law
        15	//! and states its choice):
    ...
        27	//!   arithmetic and allocating walks take it: join/meet/span (an
        28	//!   emission plus its buffers), `distance`/`lag` (accumulator folds
        29	//!   and `Base` products), `Ranked`'s total order (the rank co-sweep).

Resolution: Either state in the essay's first paragraph that it is the ladder's single home for rung policy and reduce the call sites to a pointer ("rung choice: see `codec::bits`"), or keep the bullets as criteria only (free insurance; pays where the replaced walk is expensive and equality is common; not where the fallback is itself a cheap scan and unequal is the common case; not on linear predicates) and drop the named operations, letting each site carry its rationale as today. Acceptance: the per-operation rung rationale appears exactly once in the crate, and wherever it lives names itself as the home.

### codec-bits-5: Door, seam, gate: three unanchored words for one boundary; "denomination" and "currency" each carry two senses
- Where: crates/before/src/codec/bits.rs:107-119 (related: crates/before/src/codec/bits.rs:5-6, 90, 135; crates/before/src/codec.rs:46; crates/before/src/codec/buf.rs:26, 30; crates/before/src/codec/cursor.rs:44, 93; crates/before/src/codec/dsi.rs:61, 68-70; crates/before/src/codec/stack.rs:4-6, 39, 192; crates/before/src/clock.rs:836; crates/before/src/meter/board/currency.rs)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (per-file counts over the partition: "door" 16, "seam" 16, "gate" 1; `grep -in door crates/before/src/lib.rs` is empty; the "denominat" sites read); executed: no
- Seen by: prose [20]; refutation: confirmed, with a correction to the evidence (the non-codec "denominate" sites read "denominate readings in exact encoded bit lengths", the unit-of-measure sense, not bytes-per-cost); history: no-rationale-found (no definition site exists anywhere; the vocabulary predates the writing doctrine's capture)
- Owner-gated: yes (crate-wide vocabulary; the definition site is the owner's call)

`Bits::freeze` is "the seam" (5-6), "the single gate" (108-109), and "the door" (119) within one file; neither "door" nor "seam" is anchored to an identifier or defined by contrast anywhere in the crate. "denomination" in this partition means the integer width a count is expressed in (bits.rs:90, 135; buf.rs:26, 30; cursor.rs:44, 93; dsi.rs:61; stack.rs:39), while clock.rs:836 and its siblings use it for the unit a reading is expressed in; "currency" at stack.rs:5 and 192 means the unit a transient is priced in, while `meter/board/currency.rs` names one deterministic meter. The vocabulary rule: a coined term is an identifier or is defined once by contrast; a reader meeting three words for one thing must decide whether they differ.

Evidence:

       107	    /// Freeze a built stream into the at-rest form, canonicalizing its storage:
       108	    /// the single gate between the mutable build-side world and the shared
       109	    /// frozen one.
    ...
       119	    /// buffer is allocatable — the door imposes no bound of its own.
    (bits.rs:5-6)
         5	//! build-side form lives in the sibling `buf` module; [`Bits::freeze`] is
         6	//! the seam between the two.

Resolution: Define "door" and "seam" once by contrast in `codec.rs`'s module doc (a door admits untrusted bytes or text into a stored value and owns their validation: decode, parse, literal; a seam is a hand-off between two in-crate representations at which an invariant is established: freeze, `built_view`, `into_base`); retire "gate" in the boundary sense (`just gate` already owns the word). Write "width" or "`u64`" for the integer-width sense of "denomination" and "unit" for stack.rs's "currency". Acceptance: `codec.rs` defines the two terms; "gate" as a boundary noun does not appear in the partition; "denominat" in `codec/` refers to a unit of measure or does not appear.

### codec-bits-6: The u64-width refrain is restated at entries that neither convert nor compute, and the read_gamma rejection is stated twice
- Where: crates/before/src/codec/bits.rs:117-119 (related: crates/before/src/codec/bits.rs:135-138, 173-175; crates/before/src/codec/build.rs:230-232; crates/before/src/codec/cursor.rs:44-45; crates/before/src/codec/gamma.rs:150-152; crates/before/src/codec/dsi.rs:14-18, 205-208; crates/before/src/codec/buf.rs:23-34)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read every restatement site and checked its body for width arithmetic or a `usize` conversion); executed: no
- Seen by: prose [13]; refutation: confirmed; history: deliberate-and-holds in part (7ea3df58 and 05d87e1b record the per-site ledger, and the owner's writing doctrine restates a breakable rationale at every site capable of breaking it), so the finding narrows to sites with nothing to break
- Owner-gated: no

"Exact at every size on every target" is restated at `freeze` (117-119), `from_canonical` (135-138), `live` (173-175), `finish` (230-232), the trait's `position` (44-45), and `decode_int_window` (150-152), whose bodies hold no width arithmetic and no `usize` conversion: the signature already says `u64`, and the argument's homes (buf.rs "# Widths", and the arithmetic sites such as `Bits::len`, `BitsBuf::live`, `PackedBuilder::len`, `read_unary`) carry it. `dsi.rs` states why `read_gamma` is refused at 14-18 and again at 205-208. Documentation altitude: never document what the types prevent; every sentence competes with the contract the reader came for.

Evidence:

       117	    /// Exact at every size on every target: lengths and positions are `u64`
       118	    /// on both sides of this seam, so an emission is storable whenever its
       119	    /// buffer is allocatable — the door imposes no bound of its own.
    ...
       135	    /// Exact at every size on every target: the stored form denominates its
       136	    /// bit positions in `u64` ([`len`](Self::len), [`live`](Self::live)), so
       137	    /// any buffer the validator admits is adoptable whole — the door imposes
       138	    /// no bound of its own.
    (dsi.rs)
       205	    /// - the composed unary-prefix + mantissa read for machine-word
       206	    ///   codes (`k < 64`) — `dsi-bitstream`'s own `read_gamma` is
       207	    ///   unusable here because its supported range caps at `u64` while
       208	    ///   this coding has no value cap;

Resolution: Delete the refrain at the six non-arithmetic sites, or reduce each to one clause pointing at buf.rs "# Widths"; keep the ledger where a conversion or wrap-freedom argument sits (bits.rs:156-159, buf.rs:56-58, build.rs:43-47, cursor.rs:58-60, dsi.rs:56-59 and 113-115, gamma.rs:214-216, scan.rs:63-65, stack.rs:39-42). Reduce dsi.rs:205-208 to "(`read_gamma` is refused: module doc)". Acceptance: `grep -rn 'every size on every target' crates/before/src/codec` hits no entry whose body has no `as u64`, `usize::try_from`, or width arithmetic; the `read_gamma` rejection is argued once in dsi.rs.

### codec-bits-7: ptr_eq's doc misstates why independently frozen empty streams alias
- Where: crates/before/src/codec/bits.rs:209-211 (related: crates/before/src/codec/tests.rs:168-172, 183-188; bytes-1.11.1 src/bytes.rs:138-143, 960-971, 995-1002)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read `bytes` 1.11.1: `From<Vec<u8>>` routes `len == cap` through `From<Box<[u8]>>`, which returns `Bytes::new()`, a static `EMPTY` slice, for an empty slice; a vector with `len != cap` takes the `Shared` path and keeps its own pointer); executed: no
- Seen by: prose [18]; refutation: confirmed, adding the test doc at codec/tests.rs:168-172 as a second site; history: no-rationale-found (f9973730 asserted the mechanism in its message and doc)
- Owner-gated: no

The pointer two empty freezes share is `Bytes::new()`'s static empty slice, not a dangling zero-byte allocation; and only a capacity-free empty vector reaches it. `Bits::freeze(BitsBuf::with_capacity(n))` for `n > 0` on an empty buffer hands `Bytes::from` a vector with `len 0 != cap`, which keeps its own heap pointer, so two such freezes read `ptr_eq` false. The safety conclusion (a rung derives only equality) holds; the mechanism a maintainer would reason from is wrong on both counts (statement faithfulness).

Evidence:

       209	    /// provenance is the *production* source of sharing, not the predicate's
       210	    /// meaning: every zero-byte allocation carries the same dangling pointer,
       211	    /// so two independently frozen **empty** streams also read `ptr_eq` true. A
    (codec/tests.rs)
       168	/// substitute for it. The empty stream is the deliberate exception the
       169	/// predicate's docs carry: every zero-byte allocation shares one dangling
       170	/// pointer, so two independent empty freezes read `ptr_eq` true *without* clone

Resolution: Rewrite both sites: `Bytes::new()` (which `Bytes::from` of a capacity-free empty vector reaches) shares one static empty slice, so independently frozen empty streams *may* read `ptr_eq` true; clone provenance is therefore not what the predicate certifies, only value equality. Acceptance: both sentences use "may" and name the shared static; no claim about dangling pointers remains.
Construction: in `codec/tests.rs`, `let e1 = Bits::freeze(BitsBuf::with_capacity(8)); let e2 = Bits::freeze(BitsBuf::with_capacity(8)); assert!(e1.ptr_eq(&e2));` fails under `bytes` 1.11.1 (two live one-byte allocations have distinct pointers), while the committed test's `BitsBuf::new()` pair passes.

### codec-bits-9: BitsBuf's type doc says the packed-stream builder wraps a BitsBuf; it does not
- Where: crates/before/src/codec/buf.rs:41-43 (related: crates/before/src/codec/build.rs:48-58, 233-240)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the `PackedBuilder` struct and `finish`); executed: no
- Seen by: structure [1]; refutation: confirmed; history: no-rationale-found (the sentence and the builder that contradicts it landed together in 83e61b4d)
- Owner-gated: no

`PackedBuilder` (build.rs:48-58) has fields `bytes: Vec<u8>`, `staged: u64`, `staged_len: u32` and constructs a `BitsBuf` only at `finish` via `from_raw_parts` (233-240). The parenthetical tells a maintainer to look for `BitsBuf` methods to explain builder behavior that lives in a parallel implementation (Principle 5: prose states what is). Independently actionable whether or not codec-bits-12 is taken.

Evidence:

        41	/// Every emitter and builder writes into one of these (the crate's
        42	/// packed-stream builder wraps one with the metered move set); a finished
        43	/// stream freezes into the at-rest `Bits` at the storage seam. The module doc

Resolution: If codec-bits-12 lands, the sentence becomes true as written. Otherwise re-state: "the packed-stream builder hands its finished bytes to one at `finish`", or delete the parenthetical. Acceptance: the sentence describes the builder's actual relationship to `BitsBuf`.

### codec-bits-16: cursor.rs misdescribes SliceCursor's consumers and read_int's overrides
- Where: crates/before/src/codec/cursor.rs:71-78 (related: crates/before/src/codec/cursor.rs:110-113; crates/before/src/codec/tree.rs:35-39; crates/before/src/codec/dsi.rs:218-289; crates/before/src/borsh_impls.rs:110-126; crates/before/src/version/skyline/overlay.rs:504; crates/before/src/borsh_impls/tests.rs:311)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'impl.*BitCursor for'`: cursor.rs:104, dsi.rs:166, borsh_impls.rs:88, plus the test-only borsh_impls/tests.rs:311; `grep -rn 'fn read_int'`: the default at cursor.rs:79 and overrides at cursor.rs:123, dsi.rs:218, borsh_impls.rs:110; tree.rs:36 opens `super::DsiCursor::new_at(bits, pos)`; production `SliceCursor::new` sites are gamma.rs:117 and overlay.rs:504); executed: no
- Seen by: structure [4], prose [15]; refutation: confirmed; history: deliberate-but-expired for 110-113 (b3f09baa wrote it when tree.rs opened a `SliceCursor`; 5d167a63 moved `parse_id` onto `DsiCursor` and left the comment), no-rationale-found for 74 ("Both" was already an undercount when written: `DsiCursor` has overridden `read_int` since 3e5b95df23)
- Owner-gated: no

The trait doc counts two overriding cursors where three production implementors override (`SliceCursor`, `DsiCursor`, `ReaderCursor`) and says an override routes through `gamma::decode_int_window`, which `DsiCursor` does not use (it composes a table tier, a word arm, and a wide arm of its own); the default body serves only the test-only `BitwiseReaderCursor`. The `read_bit` comment at 110-113 says `SliceCursor` sits under the id-tree parsers; `parse_id` reads through `DsiCursor`, and `SliceCursor`'s production users are `decode_int` and the masked walks' `IdLeafCursor`. A maintainer pricing scan-meter work for the id parsers would look at the wrong cursor (Principle 5: no ghost references, no hand-maintained counts).

Evidence:

        71	    /// The provided default is the per-bit loop ([`decode_int_from`]); a cursor
        72	    /// with cheap access to its byte-backed window overrides it to route
        73	    /// through the word decoder ([`gamma::decode_int_window`]), which reads a
        74	    /// whole code in `O(1)` words. Both cursors override: [`SliceCursor`]
    ...
       110	        // One live bit scanned: this cursor is the sequential read primitive
       111	        // under the id-tree parsers and the per-bit gamma decode path, so the
       112	        // scan meter records here once for both. The skyline kernels read
       113	        // through `DsiCursor`, which carries its own records.

Resolution: Replace 71-78 with the contract every override must honor (accept and reject on exactly the inputs the per-bit loop does; the mechanism belongs at each override, where `SliceCursor::read_int` and `DsiCursor::read_int` already state theirs). Rewrite 110-113 to name the actual consumers: the per-bit primitive under `decode_int` and the masked id leaf cursor; the id parsers and skyline kernels read through `DsiCursor`. Optionally make `read_int` a required method and move the one-line default into `BitwiseReaderCursor`. Acceptance: no sentence in cursor.rs names a consumer that does not construct a `SliceCursor` or counts implementors; `grep -n 'Both cursors' crates/before/src/codec/cursor.rs` is empty.

### codec-bits-20: Ghost references to the retired store_be emit path
- Where: crates/before/src/codec/gamma.rs:13-16 (related: crates/before/src/codec/tests.rs:386-395, 514-519; crates/before/src/codec/gamma.rs:39-45)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn store_be crates/before --include='*.rs'` returns only codec/tests.rs:389 and 517; `git show 83e61b4d` removed the `store_be` calls and its message says the gamma word path emits through `BitsBuf::push_bits`; `encode_int`'s word arm is two `push_bits` appends at gamma.rs:43-44); executed: no
- Seen by: structure [5]; refutation: confirmed, adding gamma.rs:15's "emitted with one store" as a third site; history: contradicts-hard-rule (root AGENTS.md: nothing in the codebase refers to code that no longer exists)
- Owner-gated: no

Two test comments name a `store_be` helper that no longer exists, and gamma.rs's module doc describes the word path as "emitted with one store", the shape of the deleted `bitvec` store; the path is two `push_bits` appends. A hard-rule breach with a trivial fix: a reader hunting for the fast path under test greps for a name that is not there.

Evidence:

        13	//! Both directions keep the coding's cost word-scale: the stream is
        14	//! byte-backed, so a whole code is decoded from one 64-bit window
        15	//! ([`decode_int_window`]) and emitted with one store, with per-bit loops as
        16	//! the fallback — and, on decode, the sole arbiter of every reject.
    (codec/tests.rs)
       389	// `gamma::decode_int_window` / `store_be`, and the word-parallel cursor's
       517	    /// Holds for every value — `u64`-range codes (the `store_be` path) and

Resolution: "emitted as two word appends (`BitsBuf::push_bits`)" at gamma.rs:15; re-denominate both test mentions to `BitsBuf::push_bits`. Acceptance: `grep -rn store_be crates/before` returns nothing; gamma.rs:15 matches `encode_int`'s word arm.

### codec-bits-26: scan.rs's record-site roster has drifted
- Where: crates/before/src/codec/scan.rs:9-17 (related: crates/before/src/party/ops/diff.rs:312, 352, 363; crates/before/src/party/ops/index.rs:77, 91, 200, 282; crates/before/src/version/skyline/grow.rs:295, 302; crates/before/src/codec.rs:46-49, 63-65; crates/before/src/codec/buf.rs:380-382)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rc record_bits crates/before/src`: idbits.rs 3, party/ops/index.rs 4, party/ops/diff.rs 3, codec/dsi.rs 8, codec/build.rs 7, codec/cursor.rs 2, version/skyline/grow.rs 2; index.rs:282 records 32 bits per binary-search step; grow.rs:295 and 302 record 2); executed: no
- Seen by: prose [16]; refutation: confirmed; history: no-rationale-found (the roster was the initial inventory in 197dfd9e16, extended once for `DsiCursor`; later record sites in grow.rs, diff.rs, and index.rs landed without touching it)
- Owner-gated: no

The module doc enumerates where `record_bits` fires and omits three files that record directly (`party/ops/diff.rs`, `party/ops/index.rs`, `version/skyline/grow.rs`), and attributes builder writes to "`party::ops`' builder" although `PackedBuilder` serves the skyline builder too. The `seal_padding` consumer list is stated twice (codec.rs:46-49 and buf.rs:380-382). Hand-maintained caller rosters rot silently, and this one has; a meter's doc should state the rule for where records happen, so a reviewer can judge a new kernel against it.

Evidence:

         9	//! - id tag reads and skip steps (`idbits::IdReader`), 2 bits per node;
        10	//! - id-builder bit writes and verbatim splice lengths
        11	//!   (`party::ops`' builder);
        12	//! - event topology cursor advances and gamma code-skips (the skyline
        13	//!   walks' word-parallel `codec::DsiCursor` — unary runs and code
        14	//!   skips record their full bit widths, however the reads batch);
        15	//! - every sequential decoder/validator bit read (`codec::SliceCursor`
        16	//!   and `codec::DsiCursor`, which carry `decode`, the gamma decoder,
        17	//!   and the skyline validator/decoder cursors).

Resolution: State the rule: records fire at the packed-stream primitives (builder appends and splices, cursor bit, unary, and code reads), and any kernel that examines packed bits outside those primitives records at its own site at the width it examined. If a roster is wanted, put it in a test that greps for `record_bits` call sites. Collapse the `seal_padding` consumer list to one sentence at `seal_padding` and let codec.rs point there. Acceptance: scan.rs's module doc names no file or type as a record site; the `seal_padding` enumeration appears at most once.

**Nits (3), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| codec-bits-3 | `crates/before/src/codec/bits.rs:50-50` | Register tells: "honest", "genuinely", and "real" where the anchored term is "live" | "Two instruments pin the ladder"; "a consumer with wide arithmetic"; "live stream" / "live input" / "allocated memory" at the "real" sites | `evidence/partitions/codec-bits.md` |
| codec-bits-4 | `crates/before/src/codec/bits.rs:60-60` | Em-dashes in `//` comments, and a doc line broken mid-clause | Replace the em-dashes in `//` comments with ` -- `; reflow dsi.rs:295-298 | `evidence/partitions/codec-bits.md` |
| codec-bits-24 | `crates/before/src/codec/literal.rs:1-3` | literal.rs is the one production module in the partition without a module doc | Add a one- or two-sentence `//!` doc: the id tree's in-memory constructors (`id_leaf` ... | `evidence/partitions/codec-bits.md` |

**Cross-references.** codec-bits-9's false "wraps a `BitsBuf`" sentence becomes true if codec-bits-12 (the simplification finding) lands. codec-bits-16's stale consumer roster and codec-bits-26's record-site roster are the module's two hand-maintained caller lists; codec-base-text-tree-9 is the third in the codec. codec-bits-5's "door/seam/gate" is the crate-wide vocabulary question (Open questions 1 and 2); codec-bits-1 asks where the identity-ladder policy lives (306e2de0 says bits.rs; the file does not).

## The codec: base, text, tree, display

7 findings (0 high, 0 medium, 3 low, 4 nit). Full records: `evidence/partitions/codec-base-text-tree.md`.

### codec-base-text-tree-3: `parse_decimal`'s rustdoc pins a probe measurement ("parse exponent 1.49") that no committed instrument holds
- Where: crates/before/src/codec/base.rs:71-83 (related: tools/benchjudge-expected.json (notes), crates/before/tests/meter.rs:55-60)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '1\.49\|dependency-selection' src tests tools`: base.rs:75, an unrelated ×1.49 floor at tests/meter.rs:8309, and the agent note `.agent-notes/2026-07-22-before-adversarial-resource-amplification/…md:423`; `tools/benchjudge-expected.json` notes record the parse trio at "measured e 1.28/1.33/1.30" under the 1.7 text ceiling); executed: no
- Seen by: prose, claims; refutation: confirmed; history: deliberate but expired (54b68b4f made the bench judge the class judge the same day f3da6377 placed the number)
- Owner-gated: no

The bracketed measurement and its source ("the dependency-selection probe") are dated rationale at a declaration site whose only provenance is an agent note; the committed instrument for the conversion class, which the same paragraph names, records different exponents for the same quantity. Principle 5 (no dated rationale or design-doc citations at code sites) and Principle 8 (a transcribed number is a hypothesis, not a measurement).

Evidence:

        73	    /// The radix conversion is delegated whole to the backend, whose
        74	    /// divide-and-conquer parser is subquadratic in the digit count
        75	    /// \[measured — the dependency-selection probe: parse exponent 1.49
        76	    /// over doubling digit counts\]. The conversion therefore runs inside
        77	    /// the dependency, below the limb shim, so this records one
        78	    /// width-proportional limb count for the materialized value — the
        79	    /// same convention as the wide-gamma decode — and the bench judge's
        80	    /// time leg is what judges the conversion's complexity class.

Resolution: Replace the bracketed clause with the instrument by name: "whose divide-and-conquer parser is subquadratic in the digit count; the bench judge's text-ceiling parse cells (`version_parse_trailing/hugeleaf`, `version_parse_noncanon/hugeleaf`, `clock_parse_trailing/hugeleaf`) hold the class." Keep the rest. Acceptance: `grep -rn '1\.49\|dependency-selection' crates/before/src` is empty; the paragraph names the bench judge as the sole authority for the class.

### codec-base-text-tree-5: `msb_cmp_windows` documents a stronger premise than `Rank` holds; the argument that makes the tail rule sound is unwritten
- Where: crates/before/src/codec/base.rs:148-159 (related: crates/before/src/version/rank.rs:520-534, crates/before/src/version/rank.rs:813-816, crates/before/src/version/rank.rs:883-906, crates/before/src/version/rank.rs:244-248, crates/before/src/version/rank/num/tests.rs:103-120)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read rank.rs `from_num` and the `debug_assert!` at 813-816; read the `Ord` impl at 883-906; read the differential at num/tests.rs:110-120); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (the premise was overstated from d8f91040 onward; `from_raw` already shifted by `tz.min(exp)`)
- Owner-gated: no

The tail rule ("the longer string is the larger value") is justified by "the caller's normalization invariant that the strings end in a set bit (an odd numerator)". `Rank`'s actual invariant is `exp == 0 || num.bit(0)`: an integral rank such as 2 is stored as numerator 2, exponent 0, an even numerator. The kernel is still correct for `Rank` because it runs only on a class tie (`bits(num) - exp` equal), where a longer numerator forces a larger exponent, hence `exp > 0`, hence oddness of the longer string, which is all the tail rule needs; that derivation appears nowhere, and rank.rs:889-893 repeats the overstatement. The differential ORs both operands with 1, so it never exercises an even operand. Statement faithfulness: a proof resting on a premise the caller does not hold is one a future reader will either distrust or extend wrongly.

Evidence:

       154	/// cost is O(shared-prefix limbs) with zero allocation. When every shared
       155	/// window agrees, the longer bit string is the larger value: this rides on
       156	/// the caller's normalization invariant that the strings end in a set bit
       157	/// (an odd numerator), so the longer string's extension is nonzero. The

    (rank.rs)
       813	    debug_assert!(
       814	        exp == 0 || num.bit(0),
       815	        "a nonempty fraction ends in its last set bit, so the numerator is odd"
       816	    );

       889	        // shift; its longer-string-wins tail rule is sound because
       890	        // normalization keeps numerators odd (the longer string ends in a set
       891	        // bit). The order is exact at any magnitude — a false tie here would

    (num/tests.rs)
       112	        // Odd operands: the tail rule's normalization premise (the
       113	        // stored numerator invariant).
       114	        let (a, b) = (a | UBig::ONE, b | UBig::ONE);

Resolution: Restate the premise at base.rs:155-157 as "the longer string ends in a set bit", and at rank.rs:889-891 write the one-line derivation: on a class tie, more numerator bits means a larger exponent, so `exp > 0` and the numerator is odd by normalization. Widen the differential to OR only the wider operand with 1 so an even shorter operand is exercised. Acceptance: base.rs and rank.rs carry the class-tie argument; the differential covers an even shorter operand and stays green.

Construction: Rank 2 (num 2, exp 0) versus Rank 5/2 (num 5, exp 1) tie at class 2; the first windows `[1<<63]` versus `[5<<61]` decide `Less` inside the window, correct. The tail rule is reached only across different widths, where the longer operand has `exp > 0`; no counterexample exists for `Rank`. The finding is that the written premise does not say this.

### codec-base-text-tree-9: The `limb_meter` module doc enumerates record sites by name and the list is stale; the materialization convention has no home
- Where: crates/before/src/codec/base/limb_meter.rs:6-16 (related: crates/before/src/codec/base/limb_meter.rs:38-42, crates/before/src/codec/dsi.rs:282-285, crates/before/src/version/skyline/query/integral.rs:352-376, crates/before/src/version/skyline/query/integral.rs:388-392, crates/before/src/codec/base.rs:132-139)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'record_wide\|limb_meter::record(' src` outside `codec/base`: dsi.rs:285, gamma.rs:262, num.rs:66, integral.rs:370-372 and :390); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate but expired (the roster was complete at 43486366; 3e5b95df and 14259c1f added recorders without touching this doc)
- Owner-gated: no

The doc names `codec::gamma` and `version::rank::num` as the recorders outside `Base`, but `codec::dsi` and `skyline::query::integral` also record. Meanwhile the convention every materializing site cites ("one width-proportional limb count for the materialized value") is stated by analogy at base.rs:78-79, base.rs:136-139, and integral.rs:355-356, but not at `record_wide`, whose doc is one line. Principle 5: no hand-maintained caller enumerations; the rule the sites follow belongs once, where the recorder lives.

Evidence:

         6	//! The proxy counted here is the operands' 64-bit limb counts per `Base`
         7	//! operation — arithmetic, comparison, equality, and hashing all record before
         8	//! they run, and the wide-gamma decode in `codec::gamma` records one
         9	//! value-width count per decoded value — so amortized-linear algorithms count

        12	//! not any particular storage: the rank numerator's wide arm
        13	//! (`version::rank::num`, magnitudes past the backend's capacity on
        14	//! 32-bit targets) records its operations' operand and materialization

        38	/// Record the limb width of a raw `UBig` working value.
        39	pub(crate) fn record_wide(n: &dashu_int::UBig) {

    (dsi.rs)
       284	        #[cfg(feature = "limb-meter")]
       285	        super::limb_meter::record_wide(&m);

Resolution: At `record_wide`, state the convention once: one value-width count wherever a wide value is materialized from bits, bytes, or text, because the backend touches every limb of the result and a meter that missed it would let a decoder build arbitrarily wide values while reading zero. In the module doc, state the two rules (operand widths per `Base` operation; one value-width per materialized wide value) without naming modules. Have base.rs:78-79 and 136-139 cite `record_wide` rather than "the wide-gamma decode". Acceptance: the module doc names no recording module; `record_wide`'s doc states the convention; base.rs's two materialization docs point at `record_wide`.

**Nits (4), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| codec-base-text-tree-1 | `crates/before/src/codec/base.rs:16-26` | Small inaccuracies in `Base`'s prose: "every operation records", an ambiguous shift clause, an undocumented `bit`, a ragged wrap | Narrow the quantifier to "every arithmetic, comparison, equality, and hashing operation" and say the O(1) reads and `Display` do not record ... | `evidence/partitions/codec-base-text-tree.md` |
| codec-base-text-tree-14 | `crates/before/src/codec/text.rs:46-54` | `parse_base`'s doc restates the conversion rules `parse_decimal` owns and defines the grammar by an unnamed comparison | At `parse_base`, keep the grammar only (maximal ASCII digit run after a leading whitespace skip, ended by the first non-digit ... | `evidence/partitions/codec-base-text-tree.md` |
| codec-base-text-tree-25 | `crates/before/src/codec/tests.rs:1225-1238` | Register words: "honest", "genre", "keystone", "real", "major finding", a caps `WITNESS` label, and a "Test-only" mislabel | "so the recorded cost counts exactly the window pairs the scan compared"; "class" or "kind" for "genre" ... | `evidence/partitions/codec-base-text-tree.md` |
| codec-base-text-tree-26 | `crates/before/src/codec/tests.rs:1227-1228` | Hand-maintained counts: "the 256 uniform-random vectors" and "the five marker-padded wire types" | "The uniform-random vectors in `clock::tests::decode_never_panics` are a thin panic net"; "The family spans the marker-padded wire types (`Party` ... | `evidence/partitions/codec-base-text-tree.md` |

**Cross-references.** codec-base-text-tree-3's "1.49" is the same genre as board-families-floors-judge-1 and codec-bits' bracketed readings. codec-base-text-tree-5's overstated premise recurs at rank.rs:889-891 (rank partition). codec-base-text-tree-25's "Test-only" label on `limb_meter` contradicts Cargo.toml:115-117's `required-features` for the board example (crate-root-4 and deps-15 read the same manifest comments). codec-base-text-tree-26's "256 vectors" transcribes proptest's default case count.

## Cross-cutting: fold, shape, recurse, serde and borsh

7 findings (0 high, 1 medium, 5 low, 1 nit). Full records: `evidence/partitions/crate-root.md`, `evidence/sweeps/recursion.md`.

### crate-root-38: A plateau is documented as a "maximal constant run" but the walk yields canonical leaves, which can be adjacent and equal
- Where: crates/before/src/shape.rs:83-94 (related: crates/before/src/shape.rs:11-15, crates/before/src/shape.rs:115-116, crates/before/src/version/skyline.rs:4-7, crates/before/src/version/skyline.rs:69-73, crates/before/src/version/skyline/shape.rs:33-36, crates/before/src/party.rs:473)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (skyline.rs:69-73 defines canonical form as forbidding only equal *sibling* leaves and calls a zero delta between non-sibling leaves "two plateaus of equal height separated by a subtree boundary"; skyline/shape.rs:33-36 returns `None` for a zero delta, which arises exactly between adjacent equal-height leaves; the constructed example below was checked by hand against the normalization rule, not run); executed: no
- Seen by: prose; refutation: confirmed (and the same conflation stands at skyline.rs:5, overlay.rs:99, and party.rs:473 outside this partition); history: no-rationale-found (46eb64f9 wrote "maximal constant run"; the owner's same-day 98e5b3b4 removed Region's copy of the definition but left Plateau's and the module bullets; the file contradicts itself at 93-94)
- Owner-gated: no

The module doc (11-15) and `Plateau`'s doc (83-84) define an item as one maximal constant run of the step function; `Plateau::rise`'s doc (93-94) then admits that two equal-height plateaus can be adjacent. Both cannot hold. The items are the leaves of the canonical coding, and canonical form forbids only equal sibling leaves, so a maximal constant run can span several items. A renderer that trusts the definition draws a boundary the function does not have; a consumer counting runs gets the wrong count. The same holds for `Region` (115-116) and `Party::shape`.

Evidence:

    83  /// A *plateau* is one maximal constant run of the version's step
    84  /// function. A shape walk yields plateaus left to right; see the
    ...
    93      /// left edge. `None` occurs mid-stream too: two equal-height
    94      /// plateaus separated by a subtree boundary are a real shape.

    version/skyline.rs:
    71  //!   what collapse removes. A zero delta between *non-sibling* consecutive
    72  //!   leaves is a real, canonical shape: two plateaus of equal height
    73  //!   separated by a subtree boundary.

Resolution: Define the item by the coding: "A *plateau* is one leaf of the version's canonical coding: a dyadic interval on which the step function is constant. Canonical form merges equal sibling leaves, so adjacent plateaus differ in height except across a subtree boundary, where `rise: None` marks a level step." Apply the same correction to the `Region` sentences (14-15, 115-116), the module bullets (11-12), and, outside this partition, skyline.rs:4-5, overlay.rs:99, and party.rs:473. Keep "equal iff plateau sequences equal" (it holds for the leaf sequence). Acceptance: no sentence in shape.rs calls an item a maximal constant run; a doctest parses `(0, (0, 0, 1), (0, 1, 0))` and asserts four plateaus with the third's `rise == None`.
Construction: `let v: Version = "(0, (0, 0, 1), (0, 1, 0))".parse().unwrap(); let p: Vec<_> = v.shape().collect();` The tree is canonical (no equal sibling leaves; no liftable minimum), the step function is 0, 1, 1, 0 over quarters (three maximal runs), and the walk yields four `Plateau { depth: 2 }` items, the third with `rise: None`. For parties, `((0, 1), (1, 0))` yields four regions with the middle two both `owned: true`.

### crate-root-10: before's borsh tests cite a rumors module (`crate::bookmark`) and rumors' protocol
- Where: crates/before/src/borsh_impls/tests.rs:224-226 (related: crates/before/src/borsh_impls/tests.rs:261-263)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'bookmark\|gossip protocol' crates/before/src` returns only lines 225 and 263; no bookmark module exists under crates/before/src); executed: no
- Seen by: structure, prose, correctness; refutation: confirmed; history: no-rationale-found (written into this file at b7fb409f0, a rumors-feature commit, when `bookmark` was already the root crate's module; cross-crate from birth)
- Owner-gated: no

`crate::bookmark` resolves to nothing in `before` (rustdoc never compiles `cfg(test)` module docs, so no gate catches the dangling link), and the test's motivation is stated in a consumer's vocabulary. The invariant the test protects stands on its own in before's terms (lines 228-234: a normalizing `join` sheds bits the freeze must seal behind canonical padding).

Evidence:

   224  /// Regression: a `Party` grown by [`join`](Party::join) — the operation
   225  /// [`reclaim`](crate::bookmark) drives on a reboot — must survive the borsh
   226  /// wire round-trip.
    ...
   263      /// through borsh — the on-wire form the gossip protocol ships.

Resolution: Drop the `reclaim`/`bookmark` clause and the "gossip protocol" clause; keep the mechanism sentences. Acceptance: `grep -rn 'bookmark\|gossip protocol' crates/before/src` is empty.

### crate-root-18: The closing drain hands back coalesced groups, and the public `# Errors` prose promises each input is merged or handed back
- Where: crates/before/src/fold.rs:38-40 (related: crates/before/src/party.rs:312-315, crates/before/src/party.rs:353-366, crates/before/src/clock.rs:234-237, crates/before/src/laws.rs:2406-2410)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read; the construction below was traced by hand against party.rs:353-366, not run); executed: no
- Seen by: correctness; refutation: reframed from correctness to documentation precision (no region is lost; the clause already says the absorbed/handed-back split is unspecified; reachable only through aliased input); history: deliberate-and-holds for the mechanism (fold.rs:33-36 documents the over-full slot; laws.rs:2406-2410 pins conservation over region unions and states that "an element-wise identity clause would reject correct behavior"), so the finding narrows to the prose
- Owner-gated: no

On aliased input a refused group is a union of several inputs, and `Party::join_all`/`Clock::join_all` push that union to the returned `Vec`. The `# Errors` sections say "every input [`Party`] is either merged into `self` or handed back", which the fused element does not satisfy element-wise; the laws state the correct contract (the union of handed-back regions equals the union of the unmerged inputs' regions). Judge by the clause, not the likelihood of aliasing.

Evidence:

    38  /// The returned groups are in stack order, oldest (heaviest) first, for
    39  /// the caller's closing drain: a left-to-right fold over them keeps
    40  /// every combine's left operand the older group.

    party.rs:
   312      /// Returns the parties which *overlapped* and so could not be folded in,
   313      /// dropping nothing: every input [`Party`] is either merged into `self` or
   314      /// handed back. In case of partial error, the set of parties which are
   315      /// absorbed vs. handed back is unspecified.

Resolution: Amend both `# Errors` sections to state the union contract ("a handed-back element may be the union of several inputs whose regions could not be folded in; the handed-back regions together are exactly the unmerged inputs' regions"), and add one sentence to fold.rs's drain paragraph saying the drain may refuse a coalesced group. Acceptance: a committed unit test constructs the case below and asserts `err.len() == 1` with `err[0]` equal to the union `c | d`; the public prose matches the test.
Construction: `let mut s = Party::seed(); let mut x = s.fork(); let mut b = x.fork(); let d = b.fork(); let a = x; let c = a.dangerously_alias();` then `s.join_all([a, b, c, d]).unwrap_err()`: `a` and `b` coalesce at weight 1; `c` waits at weight 0 and coalesces with `d`; `combine(ab, cd)` overlaps on `(0, (1, 0))` so both stay on the stack; the drain joins `ab` into `s` and refuses `cd`, so the error is `vec![cd]` with `cd == "(0, (1, (0, 1)))"`, a party that was never an input.

### crate-root-20: fold.rs's hand-maintained caller list omits `Span`'s fold
- Where: crates/before/src/fold.rs:86-90 (related: crates/before/src/span/algebra.rs:339-341, crates/before/src/span/algebra.rs:369)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `balanced_reduce` outside fold.rs: version.rs:653, :806, and span/algebra.rs:369 inside `fold_endpoints`, the four span operators' shared fold); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found (b3f09baa0 both wrote the list and added the span caller, so it was incomplete from birth)
- Owner-gated: no

No hand-maintained enumerations of callers; the list has already drifted from the code.

Evidence:

    86  /// [`balanced_try_fold`] with every input accepted and the closing
    87  /// drain folded in: the receiver-seeded version folds
    88  /// (`Version::join_all`, `Version::meet_all`, `Version::span_all`)
    89  /// never see the `None`, and the seedless `Sum`/`FromIterator` doors
    90  /// restore the join's identity (the empty version) over it.

Resolution: State the two caller shapes without naming them: "Receiver-seeded callers never see `None`; seedless callers restore their operator's identity over it." Acceptance: fold.rs names no specific caller, or `git grep balanced_reduce` matches the list exactly.

### crate-root-33: `RED_ZONE`'s derivation reasons about a release-profile measurement for a constant that compiles only under `cfg(test)`
- Where: crates/before/src/recurse.rs:43-49 (related: crates/before/src/recurse.rs:38-39, crates/before/src/recurse.rs:50-51, crates/before/src/recurse.rs:100, justfile:109, justfile:114, crates/before/src/meter/tests.rs:392-432, crates/before/src/clock/tests.rs deep_tree_stack_safety)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read; `STRIDE`, `RED_ZONE`, and `grow` are `#[cfg(test)]` at 38, 50, 100; the gate's test recipes run `cargo nextest run --workspace` without `--release`, so the committed exercises of the guard run under the dev profile the derivation does not describe); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (dfb247c36 set the constant from release frame sizes when the guard ran in production traversals; 05bd2b16d moved it under `cfg(test)`; b3f09baa0 re-wrapped the comment without re-denominating)
- Owner-gated: no

An approximation survives in prose only with its validity band stated; this band is stated for a profile the constant never runs under (dev-profile frames are considerably larger), so the "8x cushion" is not the cushion the code runs with. The operative proofs are the depth-200k dive and the depth-100k clock test, which the comment does not cite.

Evidence:

    43  /// Sized from a frame-size measurement (aarch64 release): the heaviest
    44  /// traversal frame is roughly 0.5 KiB/level — established by per-level
    45  /// stack-pointer deltas and cross-checked against each recursive function's
    46  /// prologue `sub sp`. With [`STRIDE`] = 64 the inter-probe burst is therefore
    47  /// well under 32 KiB, so 256 KiB leaves roughly an 8x cushion — ample headroom

Resolution: Restate the premise for the profile the guard runs in, or replace the derivation with the enforcement: "`STRIDE` × the largest test frame must stay under `RED_ZONE`; `meter::tests::stack_segment_meter_counts_deterministically_and_resets` (depth 200 000) and `clock::tests::deep_tree_stack_safety` are the committed proofs that the pair holds on every target the gate runs." Acceptance: the comment names the profile it reasons about and the committed tests that hold the constant to it.

### recursion-4: recurse.rs presents the descend! roster as the inventory of all remaining depth recursion
- Where: crates/before/src/recurse.rs:9-14 (related: crates/before/AGENTS.md:32-36, crates/before/src/codec/tests.rs:1691-1705, crates/before/src/testing/shape_rows.rs:121-127, crates/before/src/testing/shape_rows.rs:199-206, crates/before/src/version/skyline/tests.rs:349-363, crates/before/src/testing/grow_brute_force.rs:164-170, crates/before/src/testing/generators.rs:79-107, crates/before/src/testing/semantic_oracle.rs:610-630)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (each related site read at its self-call lines; `descend!` grep confirms exactly three files use the macro; commit 1ddb5a483's message states the list's intent); executed: no
- Verification: reframed: the list is accurate as the roster of `descend!` users (bridge, grow reference probe, segment-liveness dive; commit 1ddb5a483: "recurse.rs's exception inventory now names all three test-surface descend! users"), but the sentence frames it as "where the remaining depth recursion lives", and AGENTS.md states a rule ("A walk that must recurse routes each recursive call through `crate::recurse::descend!`") that the test surface does not follow; history: deliberate-and-holds for the roster, no-rationale-found for the framing
- Owner-gated: no

The module doc says the remaining depth recursion lives in the bridge plus two
witnesses, yet the test surface holds many further functions that recurse on
oracle tree depth or text nesting without `descend!`, each bounded by the
oracle envelope, a log of size, or a named constant: `ref_parse_id_node`,
`oracle_plateaus::walk`, `party_as_steps`, `inverted_flag_stream::walk`,
`best_inflation`, `bushy_version_with`/`bushy_party`, `min_ticks::rec`. As a
statement of where recursion lives the sentence is false today; as a roster it
is hand-maintained and will drift. AGENTS.md:32-36 repeats the framing and
states the guard as a rule for every recursive walk.

Evidence:

         9	//! Every library traversal is iterative: depth lives on explicit heap stacks,
        10	//! never the call stack, so the guard machinery compiles only for the test
        11	//! surface, where the remaining depth recursion lives: the differential oracle
        12	//! bridge (`testing::bridge`), whose walks mirror the paper's recursive trees,
        13	//! plus the test-local recursive witnesses beside it (the grow suite's
        14	//! reference cost probe, the meter suite's segment-liveness dive).

    AGENTS.md:
        32	  `version/skyline/fill.rs`). A walk that must recurse routes each recursive
        33	  call through `crate::recurse::descend!`, which grows the stack onto the
        34	  heap before a deep input can overflow — today those are only test
        35	  surfaces: the oracle bridge and the test-local recursive witnesses beside
        36	  it (`recurse.rs`'s module doc holds the inventory and the keep decision).

    shape_rows.rs:
       126	                walk(l, &offset, depth + 1, out);
       127	                walk(r, &offset, depth + 1, out);

    codec/tests.rs:
      1701	            let left = ref_parse_id_node(cur, bits)?;

    semantic_oracle.rs:
       629	        let l = rec(e, 2 * k, level + 1, g, &off2);
       630	        let r = rec(e, 2 * k + 1, level + 1, g, &off2);

Resolution: Restate recurse.rs:9-14 as the rule with its bound classes:
library code never recurses on depth; test code may recurse when bounded by
the oracle envelope, a log of input size, or a named constant; `descend!` is
for the test walks that can meet oracle-envelope depths on frames heavier than
the oracle's own. Say plainly that the three named sites are the macro's
users, or replace the roster with a mechanical check (the surface-scan tooling
flagging self-recursive functions outside `descend!` and failing on an
unlisted one). Mirror the change in AGENTS.md:32-36. Acceptance: recurse.rs
and AGENTS.md state the policy as a rule with its bound classes, and any roster
that remains is labelled as the `descend!` user list or is enforced by a
committed check.

**Nits (1), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| crate-root-21 | `crates/before/src/fold.rs:89-89` | "door" is crate-wide jargon that no site defines | Define once where the entries live (codec.rs's or lib.rs's private docs): "A *door* is a public entry through which a value enters or leaves the proce ... | `evidence/partitions/crate-root.md` |

**Cross-references.** crate-root-38 (plateau) also stands at skyline.rs:4-5, overlay.rs:99, and party.rs:473. crate-root-18 is the fold side of the `join_all` contract (party-8, clock-4, paper-fidelity-7). crate-root-21's "door" definition site is the crate-wide question (Open questions 1). crate-root-33's `RED_ZONE` derivation and recursion-4's inventory framing are the two recurse.rs prose items; recursion-2 (bridge) and testing-oracles-3 (another class) are the mechanism they describe. crate-root-10's `crate::bookmark` link is the one before-side reference into rumors. crate-root-32 (another class) is the segments counter whose "measured fact" recurse.rs describes.

## suanpan (the crate and its test suites)

19 findings (0 high, 0 medium, 6 low, 13 nit). Full records: `evidence/partitions/suanpan-tests.md`, `evidence/partitions/suanpan.md`, `evidence/sweeps/api-audit.md`, `evidence/sweeps/inventory.md`, `evidence/sweeps/paper-fidelity.md`.

### suanpan-tests-2: park_extreme_negative_digit's stated reason for two deposits is false: one deposit of −(2^33 − 1) lands in the zone
- Where: crates/suanpan/src/accumulator/tests.rs:100-116 (related: crates/suanpan/src/accumulator.rs:34-39, crates/suanpan/src/accumulator.rs:1218-1238, crates/suanpan/src/accumulator.rs:1369-1380, crates/suanpan/src/magnitude.rs:46-49)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (trace by reading: `to_word` is `Some` for 2^33 − 1; `add_shifted_word` reaches `add_at`; `add_at` keeps `|total| < LAZY_LIMIT = 2^33`); executed: no
- Seen by: structure-prose [1]; refutation: confirmed; history: no-rationale-found (the sentence was false at inception; the later spill is the step that does the work)
- Owner-gated: no

The doc says a single deposit of the full value would recenter. Against the code, `sub_magnitude_shl(&UBig::from((1u64 << 33) - 1), 32 * index)` takes the word path (`u64::try_from` succeeds), reaches `add_at` with `value = −(2^33 − 1)`, and `total.abs() = 2^33 − 1 < LAZY_LIMIT` selects the in-zone arm: no recenter. Comments state what the code cannot show and must be true; a maintainer reading this shared helper learns a wrong fact about where the zone closes.

Evidence:

       104	/// Two deposits of `−2^32` and `−(2^32 − 1)` land in one digit because
       105	/// each intermediate total stays inside the zone; a single deposit of
       106	/// the full value would recenter. This is the construction behind the
    ...
       113	    acc.spill();
       114	    acc.sub_magnitude_shl(&UBig::from(1u64 << 32), 32 * index);
       115	    acc.sub_magnitude_shl(&UBig::from((1u64 << 32) - 1), 32 * index);

    accumulator.rs:
        39	const LAZY_LIMIT: i128 = 1 << (DIGIT_BITS + 1);
      1375	            if total.abs() < LAZY_LIMIT {

Resolution: collapse to one call after the spill (`acc.sub_magnitude_shl(&UBig::from((1u64 << 33) - 1), 32 * index);`) and restate the doc: the zone is open at 2^33, so the extreme digit lands in one deposit; the spill first keeps the register from holding the value exactly instead of as a digit. Optionally add `debug_assert_eq!(acc.digits[index as usize], -((1i64 << 33) - 1))` so the helper pins its own postcondition. Acceptance: `witnesses.rs` and `differential.rs` pass unchanged with the one-call helper and the postcondition assert.
Construction: make the one-call change and run the accumulator tests; a green run demonstrates the single deposit does not recenter, refuting the sentence.

### suanpan-tests-11: the exhaustive ledger testdoc carries hand-maintained tallies, a wall-time figure, a dated anecdote, and ragged wrapping
- Where: crates/suanpan/src/accumulator/tests/ledger.rs:212-216 (related: crates/suanpan/src/accumulator/tests/ledger.rs:174-179)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (the tally Σ_{k=1..6} 11^k = 1,948,716 checked by hand; git blame attributes the text to 023eff6e, which moved it from the introducing commit 5f16e2e5c whose message carries the same numbers); executed: no
- Seen by: structure-prose [3], blind-spots [25]; refutation: confirmed (and noted the raggedness at :214-216); history: no-rationale-found (the dated-notes excision d2a9d04e matched only calendar dates, so these lines survived by pattern, not by ruling)
- Owner-gated: no

The doc restates `LEDGER_OPS` and `LEDGER_DEPTH` as "11-op" and "≤ 6", tallies the state count, quotes a dev-profile wall time, and records that a depth-7 sweep "also passed once, at pin time". All four rot silently when either constant moves; the timing is machine-dependent narration; "at pin time" is history whose evidence is a commit. Prose speaks in the present tense, with no hand-maintained counts. The structure is already stated at `LEDGER_DEPTH`'s own doc (:176-178). Lines 214-216 also wrap with "recentering" alone on a line, the same excision residue as suanpan-tests-13.

Evidence:

       212	/// Exhaustive over all 11-op schedules of length ≤ 6 (1,948,716
       213	/// states, each checked once; ~4 s dev — the length-≤ 7 sweep's
       214	/// 21.4M states also passed once, at pin time): word-scale deltas,
       215	/// recentering
       216	/// `u64::MAX` deltas, one-limb jumps to digits 3 and 7 in both

Resolution: rewrite as "Exhaustive over every schedule of at most `LEDGER_DEPTH` ops drawn from the alphabet, each state checked once: word-scale deltas, recentering `u64::MAX` deltas, ..." and drop the state count, the seconds, and the depth-7 sentence (git history holds them; a depth bump is a deliberate commit that can cite its own run). Acceptance: no numeral in the doc duplicates a constant, no sentence reports a past run, and the paragraph reflows without a one-word line.

### suanpan-5: `reserve_digits` is absent from the crate page
- Where: crates/suanpan/src/lib.rs:225-233 (related: crates/suanpan/src/accumulator.rs:79-80, 681-702; crates/suanpan/src/claims.rs:322-326)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -c reserve_digits crates/suanpan/src/lib.rs` is 0); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-and-holds for the table omission (the table is denominated in digit touches, lib.rs:200, and 8da57920 landed the method with `table_cost: None`); no rationale for the memory paragraph's silence
- Owner-gated: no

The type doc sends readers to the crate page as the overview, and the memory paragraph describes exactly the buffer growth `reserve_digits` shapes ("A shifted entry point grows the digit buffer to cover the shifted position") without naming the hint. Documentation altitude: the crate page is where a user learns an operation exists.

Evidence:

       225	//! Digit touches are shift-independent; memory is not. A shifted entry point
       226	//! grows the digit buffer to cover the shifted position, so memory is O(shift /
       227	//! 32) plus the operand's own digits (the zero-run ledger adds at most one
       228	//! entry per write that lands above the held top, bounded by half the held
       229	//! digit positions). The *written span* is every digit from the lowest position

Resolution: one sentence in this paragraph: "A caller that knows the scale its writes will reach can pre-size the buffer once with [`reserve_digits`](Accumulator::reserve_digits)." The table stays as is (no digit-touch axis). Acceptance: `grep reserve_digits lib.rs` finds it; `cost_table_rows_bind_to_the_roster` untouched; README re-derived.

### suanpan-7: Hand-maintained restatements of enumerable facts, two already stale
- Where: crates/suanpan/src/lib.rs:296-307 (related: crates/suanpan/src/accumulator.rs:72-73, 1027; Cargo.toml:50)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (git blame: accumulator.rs:73 dates to 456c5e9f, when there were two arguments; lib.rs:27 moved to "three" in bf95fdfe; 8da57920 added `sign_limbs` and extended the table row (222) and seam paragraph (239, 247) but not the `&self` list; root Cargo.toml:50 pins dashu-int 0.5); executed: no
- Seen by: prose (17), claims (44, 52); refutation: confirmed all three legs; history: deliberate-but-expired for "both" and the `&self` list; deliberate-and-holds for the version sentence's purpose (7ab518ce), with the number as residue
- Owner-gated: no

Three prose restatements of facts the code owns, Principle 5 (state the structure, not the tally). (1) accumulator.rs:73 says the crate page carries "both cost arguments"; lib.rs:27 names three. (2) The Interop list of `&self` reads omits `sign_limbs` (`pub fn sign_limbs(&self)`, accumulator.rs:1027), the readout the same page motivates for the 32-bit case, so a user behind a shared reference is told it is unavailable. (3) The dashu-int version is restated by hand.

Evidence:

       296	//! [`UBig`] is `dashu_int::UBig` (compiled against `dashu-int` 0.5; bumping
       ...
       301	//! every amortized-O(1) sign query takes `&mut self`, so the value reads
       302	//! available behind a shared reference are
       303	//! [`is_literally_zero`](Accumulator::is_literally_zero),
       304	//! [`digit_count`](Accumulator::digit_count), the O(held digits)
       305	//! [`sign_magnitude`](Accumulator::sign_magnitude) (and its scaled twin
       306	//! [`sign_magnitude_shl`](Accumulator::sign_magnitude_shl)), and a
       307	//! [`clone`](Clone::clone) — wrap in a lock for shared sign reads. It is

    accumulator.rs:
        72	/// held value to a normalized magnitude. The crate docs carry the
        73	/// representation and both cost arguments. Sign queries take `&mut self`

Resolution: (1) "the representation and the cost arguments"; (2) add `sign_limbs` beside `sign_magnitude`, or restate structurally ("every `&self` method: the two O(1) probes, the three O(held digits) readouts, and clone"); (3) drop the number ("the workspace's pinned dashu-int; bumping it is a breaking change") or pin it mechanically. `just readme` afterwards. Acceptance: no numeral or hand list in these sentences that the code can change without touching the prose.

### suanpan-32: Roster data states mechanisms and labels the code does not have
- Where: crates/suanpan/src/claims.rs:118-121 (related: 313-320, 89, 397-405; crates/suanpan/src/accumulator.rs:89, 163-171, 942-948, 1286-1291, 1493-1497; crates/suanpan/src/claims/tests.rs:299-303)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (git: `git show 1c1cb67d -- crates/suanpan/src/accumulator.rs` replaces `digits: vec![0]` with `quick: Some(0)` and `digits: Vec::new()`; `git log -S'allocates the one-digit buffer'` resolves to 9e7b7ce3 (2026-07-29), before 1c1cb67d (2026-08-04); accumulator.rs:89 is `#[derive(Debug, Clone)]` and 1493-1497 a hand-written `impl Default`); executed: no
- Seen by: structure (10), prose (15), claims (43), correctness (38); refutation: confirmed, reframing 15 (the `is_literally_zero` reason is accurate in the meter's read-modify-write denomination); history: deliberate-but-expired (both reasons landed against the pre-register bodies; the `Default` label was inaccurate from birth)
- Owner-gated: no

In this roster an exclusion reason is the claim's only evidence (claims.rs:65-66), so a false mechanism is a claim with false evidence, and a row label is committed data the tests byte-compare. `new()` allocates nothing (the one-digit allocation lives in `enter_digit_engine`, 1286-1291); `digit_count`'s register arm is a `leading_zeros`/`div_ceil`, not "one field read plus an increment"; `Default` is hand-written, not derived. Principle 5. The binding test holds reasons to a 20-character floor (tests.rs:299-303), which is why these passed (suanpan-37).

Evidence:

       118	    constant(
       119	        "Accumulator::new",
       120	        "allocates the one-digit buffer: word-scale, no input axis to measure against",
       121	    ),
       316	        evidence: Evidence::Excluded(
       317	            "one field read plus an increment; the exact-top maintenance it rests on is \
        89	    "Accumulator Clone / Debug / Default (derived surface)",

    accumulator.rs:
       163	    pub fn new() -> Accumulator {
       164	        Accumulator {
       165	            quick: Some(0),
       166	            digits: Vec::new(),
      1493	impl Default for Accumulator {

Resolution: rewrite the `new` reason ("builds a register-held zero: no allocation, no digit, no input axis to measure against"), the `digit_count` reason to cover both tiers, and the label to "(trait surface)" at 89 and 398 with the exclusion text at 400-404 adjusted. Acceptance: each sentence matches the current body it describes; `cited_witnesses_exist` and `claims_are_total_over_the_public_surface` still pass.

### paper-fidelity-9: suanpan's lazy-zone amortization is asserted, not derived, under a page that promises every argument "in full"
- Where: crates/suanpan/src/lib.rs:25-29 (related: crates/suanpan/src/lib.rs:66-82, crates/suanpan/src/lib.rs:103-126, crates/suanpan/src/lib.rs:150-161)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (re-derived the amortization from the recentering rule stated at lines 70-71; compared the rigor of the three sections); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The sign-fold section states its stopping bound and why it holds (lines 103-113) and the ledger section gives an explicit credit argument (lines 150-161). The lazy-zone section gives two facts and then asserts the amortized conclusion ("So sustained carry traffic thins out geometrically…", "never more than those writes prepaid"), so a reader checking the O(1)-per-machine-word bound has to supply the potential.

Evidence:

        25	//! Every cost this page quotes holds on adversarial input sequences — the
        26	//! amortized bounds are worst-case over the whole sequence, not average-case
        27	//! claims — and every one is *derived*: the three arguments that carry them
        28	//! (the lazy zone, the collapsing sign fold, the zero-run ledger) are below, in
        29	//! full.
        76	//! against the inflow the digit above needs before it carries on. So sustained
        77	//! carry traffic thins out geometrically with height, and the total carry work
        78	//! is dominated by the deltas that entered below. The write bounds are
        79	//! amortized: a single call can be caught repaying a run of digits that earlier
        80	//! writes parked near the zone's edge, but never more than those writes prepaid

The argument that closes it, in two sentences: take Φ = Σ_{i≥1} |dᵢ| / 2³² (digit 0 excluded). A machine-word deposit touches digit 0 and raises Φ by at most about 1 (its carry into digit 1 is at most about 2³² + 2); every further carry step out of a digit `i ≥ 1` requires `|dᵢ + c| ≥ 2³³` and leaves `|r| < 2³¹`, so it lowers Φ by at least about 0.5 (digit 1, after a word-scale carry) or 1.5 (higher digits, whose incoming carry is a handful of units) while depositing at most a few units above; every carry touch is therefore prepaid, and a wide operand's per-limb contributions each add less than 1 to Φ, giving O(limbs).

Resolution: add the potential (as above) to the lazy-zone section, or soften "in full" to "in outline" for that one argument. Acceptance: each of the three named arguments states the quantity it amortizes against.

**Nits (13), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| suanpan-tests-14 | `crates/suanpan/src/accumulator/tests/metered.rs:69-71` | two doc fragments left by the dated-note excision: "Green pin: an" and a verbless "Measured (...)" | "An alternating shifted pair costs its operand, not the zero run under it: exact totals ... | `evidence/partitions/suanpan-tests.md` |
| suanpan-tests-17 | `crates/suanpan/src/accumulator/tests/metered.rs:337-337` | moralized and register-transplant vocabulary: "honest" (5), "real fold", "minted", and "tripwire" for criteria with no committed known-bad | metered.rs:337 "the difference is operand − receiver"; :678-679 drop the comment (the `#[allow]` says why) or "a named alias would only add a name to  ... | `evidence/partitions/suanpan-tests.md` |
| suanpan-tests-20 | `crates/suanpan/src/accumulator/tests/metered.rs:719-726` | domination_reads' doc names sign_dominates_at for a loop that calls sign_dominates_word, and overclaims one touch per later read | state the shape: after the first collapse a decided top answers every later read in one touch; the thousand reads go through `sign_dominates_word` ... | `evidence/partitions/suanpan-tests.md` |
| suanpan-tests-22 | `crates/suanpan/src/accumulator/tests/witnesses.rs:6-9` | witness docs narrate the mutation history in the past tense, three times | module doc: "each witness pins a corner whose mutation passes every other committed test". Test docs (:28-33 ... | `evidence/partitions/suanpan-tests.md` |
| suanpan-3 | `crates/suanpan/src/lib.rs:72-72` | Opaque `\[derived\]` tags in the crate page | delete both; if the intent is to mark which paragraphs the table's "derived above" (line 200) points at, say it in words | `evidence/partitions/suanpan.md` |
| suanpan-6 | `crates/suanpan/src/lib.rs:277-279` | The register-metering sentence says every absorbed delta or shift counts one touch; zero deltas and `shl(0)` count none | amend here and at touch_meter.rs:9-12: "a nonzero delta, a sign query, a negation ... | `evidence/partitions/suanpan.md` |
| suanpan-13 | `crates/suanpan/src/accumulator.rs:603-603` | Em-dashes in `//` comments, a TOML comment, and panic/error strings | a colon or semicolon at each site (e.g. 603 "keeps both free: no rebuild of a"; tests.rs:291 "no longer holds the #[test] fn `{witness}` ... | `evidence/partitions/suanpan.md` |
| suanpan-15 | `crates/suanpan/src/accumulator.rs:786-794` | `sign_dominates_at`'s public doc carries the margin proof at user altitude | move 786-794's inequality chain to the `SIGN_DECIDED` doc (45-50) or a `//` comment above 826 ... | `evidence/partitions/suanpan.md` |
| suanpan-19 | `crates/suanpan/src/accumulator.rs:992-992` | Vocabulary tells: moralized code, a significance adverb, a mechanism-less "silently", three spellings of one term, colon-fronted labels and antitheses | drop "honest" at 992 ("one spelling of the value, not a normal form" already says it) and "genuinely" at tests.rs:321 ... | `evidence/partitions/suanpan.md` |
| suanpan-23 | `crates/suanpan/src/accumulator.rs:1473-1474` | Headroom comments state loose bit counts with an underived `33` | "At most 32 + 31 bits" and "At most 64 + 31 bits" | `evidence/partitions/suanpan.md` |
| suanpan-27 | `crates/suanpan/src/claims.rs:12-14` | Incident history in roster prose ("twice found wrong in review") | at both sites, "the table is committed data held to the roster; an edit to either without the other is a named failure" | `evidence/partitions/suanpan.md` |
| api-audit-23 | `crates/suanpan/src/lib.rs:244-248` | suanpan states there is no from-value constructor without saying why | add the clause that names what the absence serves, or add the constructor if no reason survives | `evidence/sweeps/api-audit.md` |
| inventory-14 | `crates/suanpan/src/accumulator.rs:1333-1334` | `add_at` doc claims "any i128 magnitude" but the carry arithmetic overflows near the extremes | state the actual precondition (`\|value\| < 2^127 - 2^33`, or the | `evidence/sweeps/inventory.md` |

**Cross-references.** suanpan-tests-17 and suanpan-19 list the same "honest" sites from two partitions; suanpan-13's em-dashes include the panic strings that the register rule addresses first. suanpan-tests-11 and suite-economics-5 both cite ledger.rs:212-214's wall time and pin-time anecdote. suanpan-27 and prose-hygiene-8 both cite claims.rs:12-14. suanpan-7's `sign_limbs` omission and suanpan-5's `reserve_digits` omission are the two crate-page gaps. api-audit-23 asks why there is no from-value constructor; paper-fidelity-9 asks for the lazy-zone potential the page promises "in full". inventory-14's `add_at` domain and suanpan-23's headroom arithmetic are neighbours in accumulator.rs.

## The instruments: oracle and laws

6 findings (0 high, 0 medium, 5 low, 1 nit). Full records: `evidence/partitions/oracle-laws.md`, `evidence/sweeps/paper-fidelity.md`, `evidence/sweeps/recursion.md`.

### oracle-laws-9: The oracle test module doc says trees are never fabricated directly; three suites draw generated or literal trees
- Where: crates/before/src/oracle/tests.rs:5-7 (related: crates/before/src/oracle/tests.rs:437-443, 469-488, 499-506, 518-519, 556-559, 575-578, 593-596, 625-628, 722, 742, 759)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the module doc against the draws and literals listed); executed: no
- Seen by: instrument-correctness [41]; refutation: confirmed; history: deliberate-but-expired (true at b3320b385 apart from the paper literals; e27835ac6 and f0b83733 added the generator-driven suites; b3f09baa0 re-touched the sentence without re-truing it)
- Owner-gated: no

Prose speaks in the present tense and describes what is: the grow-optimality suite draws `arb_oracle_party_nonempty`/`arb_oracle_version`, the `covers`/`without` suites draw `arb_oracle_party`, and the paper worked examples build `V::Node`/`Party::Node` literals with `Arc::new`; a reader auditing the suite's input space from this header gets a false map.

Evidence:

         5	//! Values are generated via operations from a seed (always valid, normal-form,
         6	//! and — for populations — pairwise party-disjoint), never by fabricating
         7	//! trees directly, which might violate normality.

       437	    let left = V::Node(2u64.into(), Arc::new(leaf(1)), Arc::new(leaf(0))); // (2,1,0), already normal

       557	        id in arb_oracle_party_nonempty(),
       558	        e in arb_oracle_version(),

Resolution: Restate the header: values come from seed-derived op traces (pairwise party-disjoint populations), from the normalizing arbitrary generators, or from paper literals whose normality the test asserts or which are already normal. Acceptance: the module doc names all three input sources the file uses.

### oracle-laws-19: "door" is an undefined coinage used 41 times in laws.rs
- Where: crates/before/src/laws.rs:1723-1724 (related: crates/before/src/laws.rs:341-342, 1782-1783, 2327, 3265; crates/before/src/oracle/version.rs:370 and 375; 237 uses across `crates/before/src`)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -c door` laws.rs = 41, lib.rs = 0, crate-wide 237; the owner's lexicon at `~/.claude/writing-style.md:383` lists `| door | entry point |`); executed: no
- Seen by: structure-prose [22]; refutation: confirmed (severity lowered: vocabulary only, crate-wide, owner's call); history: contradicts-hard-rule (writing-style.md:164-169, the anchoring rule)
- Owner-gated: no

Every coined term must be anchored to an identifier or defined once by contrast; "door" ("join doors", "n-ary door", "validating door", "materialization doors", "without doors") is neither an identifier nor defined anywhere in the crate, and a new maintainer must infer from context whether a door is a method, an operator impl, a trait impl, or a constructor. The lexicon lists it as a case where the plain term is strictly clearer.

Evidence:

      1723	    /// The seedless iterator join doors (`Sum` and `FromIterator`, owned and
      1724	    /// borrowed) against their sequential pair-operator oracle, and their

Resolution: Replace with the plain term at each site ("entry point", or the method or trait name: "the `Sum` and `FromIterator` impls", "the n-ary method"), or, if the word is kept, define it once in the crate-level docs and link that definition from laws.rs's module doc. The partition-local edit is mechanical; the crate-wide decision is Finch's (see open questions). Acceptance: `grep -c door crates/before/src/laws.rs` returns 0, or one definition site exists and laws.rs links it.

### oracle-laws-22: Prose tells: the banned "minted", "THE LAW", moralized "real"/"honest", significance adverbs, the undefined "under mass" and "faces", and temporal "still works"/"survive here"
- Where: crates/before/src/laws.rs:2956-2956 (related: crates/before/src/laws.rs:3236, 3473, 2913, 2472, 1036, 2354, 2318, 2781, 2309, 3258, 3352; crates/before/src/laws/tests.rs:69-74, 81, 107; crates/before/src/oracle/version.rs:28, 388-389, 472-473; crates/before/src/oracle/tests.rs:544; crates/before/src/oracle.rs:40)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep over the partition files for each word; `~/.claude/writing-style.md:170` bans "mint" outright, :326-330 asks for the property in place of "real"/"genuine"/"honest", :404-409 lists red/green, tripwire, and "keeps it honest"; "under mass" has seven uses crate-wide and no definition; `git log -S'impl DivAssign' -- version.rs` shows production's by-value `Div`/`DivAssign` removed in 6c88c2ad0 on the day oracle/version.rs:473 was written); executed: no
- Seen by: scaffolding [7], structure-prose [27, 28]; refutation: confirmed (one citation corrected: laws.rs:2382 reads "genuine shares", not "genuinely"); history: contradicts-hard-rule for "mint", "real"/"honest"/"genuinely", and "red"; the "survive here" clause is intelligible only relative to a production removal, so it refers to code that no longer exists
- Owner-gated: no

The vocabulary rule: never write "mint" for constructing a value (here stretched to recording a tick, where production's own docs say "without marking an event"); describe code by the property that holds rather than "real", "honest", "genuinely"; anchor every coinage ("under mass", "faces" for the two operand types of one law). "THE LAW" in capitals is emphasis without a semantic distinction. "still works" and "survive here" imply a history the present-tense rule excludes. On "tripwire" at laws/tests.rs:69: the crate defines the word broadly at surface_coverage.rs:71-92 to include liveness anchors, so the site matches the crate's definition and conflicts with the lexicon's ("a test a known-bad implementation fails"); the dispute is with the crate-local definition, and "deterministic witness" is right at this site either way.

Evidence:

      2956	    /// anonymous join with no event minted, `sync` reconciles a fork,
      3236	    /// `absorb` is the anonymous join with no event minted: the version
      2913	    /// THE LAW of the rank wire form: byte-wise lexicographic order on
      2472	    /// `tick`'s inflation is real *within* the region (§4: `f · i ⊐ 0`): the
      2318	    /// the refusal arm under mass, with two and more distinct

    oracle/version.rs:
        28	/// truncation point. Literal/`u64` construction still works via
       388	    /// [`Version::ticks`](crate::Version::ticks) is the definitionally
       389	    /// honest loop — `O(n · tree)`, fit for small `n` only (the module
       473	// by-value and assign forms survive here for the oracle's own ergonomics.

    laws/tests.rs:
        69	/// discipline-level drop would lose — the deterministic tripwire beside
        70	/// the pool-driven law family, red the day the fold misroutes that group
        71	/// (dropping it outright flips the verdict, which the acceptance laws
        72	/// police; dropping it only when the rejection channel is already

Resolution: "no event minted" -> "without marking an event" (production's phrasing at clock.rs:494) at 2956, 3236, 3473; "THE LAW of the rank wire form" -> "The rank wire form's defining law"; "is real *within*" -> "is strict within"; "the definitionally honest loop" -> "the literal loop"; drop "genuinely" at laws.rs:1036, 2354, laws/tests.rs:74, oracle/tests.rs:544; "under mass" -> "on most samples" or define it once in testing/generators.rs where it originates; "faces" -> "the party and clock instances"; "still works" -> "works"; "survive here for" -> "exist for"; "not real public API" (oracle.rs:40) -> "a test and bench reference, not supported API"; rewrite laws/tests.rs:67-76 as mechanism ("This feed order makes the closing drain hand back a coalesced group; the conservation laws hold on it, and a fold that dropped that group would fail them while passing the acceptance laws' `Err` clauses"). Acceptance: `grep -n -i 'minted\|THE LAW\|honest\|genuinely\|under mass\|survive here\|still works\|red the day\|police' crates/before/src/laws.rs crates/before/src/laws/tests.rs crates/before/src/oracle crates/before/src/oracle.rs` returns nothing.

### paper-fidelity-6: laws.rs header attributes the crate's extensions to the paper and cites §2-§4
- Where: crates/before/src/laws.rs:15-20 (related: crates/before/src/testing/semantic_oracle.rs:11, crates/before/reference/itc2008.md:52,86,119,144, crates/before/src/laws.rs:536,882)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (paper section headings and §3 lines 100-143 read; grep of the paper for `meet|lattice|distribut|greatest lower`, which hits only the join-semilattice sentence at line 119); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The header says the algebraic laws "transcribe the ITC algebra (…, §2-§4)" and lists a distributive lattice under `|`/`&` and a rank valuation. The paper's §2 is "Related Work"; the algebra is in §3-§5; and the paper requires only a join semilattice (line 119: "the order must form a join semilattice"), defining no meet, no distributivity, and no rank. `meet_distributes_over_join` (laws.rs:882) and `rank_is_a_valuation` (laws.rs:536) are the crate's own. The same off-by-one appears in `semantic_oracle.rs:11` ("closure combinator (§2-3)") beside a first line that correctly says §4.

Evidence:

        15	//! The algebraic laws transcribe the ITC algebra (Almeida, Baquero & Fonte
        16	//! 2008, §2–§4): versions form a distributive lattice under `|`/`&` whose
        17	//! partial order is causality, ids form a partial commutative monoid under
        18	//! disjoint join with `fork` as its splitting inverse, events inflate strictly
        19	//! and only within the owned region, and `rank` is a strictly monotone
        20	//! valuation. The representational laws pin the crate's own contracts: the

    reference/itc2008.md:
        52	## 2 Related Work
       119	must be defined for all pairs; i.e. the order must form a join semilattice. In causal histories the join

Resolution: cite §3-§4 (and §5 for the trees) and split the sentence into the paper's algebra (join semilattice whose order is causality; ids under disjoint sum with fork as split; event as strict inflation within the id) and the crate's extensions (meet and the distributive lattice, rank as valuation and metric, projection, span, causally); fix `semantic_oracle.rs:11` to §4. Acceptance: every property the header attributes to the paper appears in §3-§5 of the transcription.

### paper-fidelity-8: the oracle's grow defines two arms the paper does not, undocumented at the oracle
- Where: crates/before/src/oracle/version.rs:268-289 (related: crates/before/reference/itc2008.md:536-543, crates/before/src/version/skyline/grow.rs:42-48, crates/before/src/testing/grow_brute_force.rs:55-58, crates/before/src/oracle/tests.rs:556-559, crates/before/src/oracle.rs:22-25)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (the oracle's arms read against the paper's five `grow` equations; grow.rs and grow_brute_force.rs read at the cited lines); executed: no
- Verification: confirmed; history: already-known at the impl (grow.rs:47-48 asserts the arm unreachable) and the brute force (grow_brute_force.rs:57), not at the oracle
- Owner-gated: no

The paper's `grow` (lines 536-543) has no `grow(1, (n, el, er))` and no `grow(0, e)`: `fill(1, e) = max(e)` collapses any fully owned event node before `event` reaches `grow`, and the `i ≠ 0` precondition plus the `(0, ir)`/`(il, 0)` arms keep `grow` off empty ids. The oracle adds both arms with a one-line doc comment, while `oracle.rs:22` makes transcription fidelity the module's purpose, and `grow_cost_is_globally_minimal` drives `grow` over arbitrary `(id, e)` pairs, so the brute-force pins depend on the arms.

Evidence:

       268	    /// `grow(id, self)` → (tree, cost).
       269	    fn grow(&self, id: &Party) -> (Version, Cost) {
       270	        match (id, self) {
       271	            (Party::Leaf(true), Version::Leaf(n)) => (Version::Leaf(n + 1u64), (0, 0)),
       272	            (Party::Leaf(true), Version::Node(n, el, er)) => {
       287	            (Party::Leaf(false), _) => {
       288	                (self.clone(), (RouteCost::INFEASIBLE, RouteCost::INFEASIBLE))

    reference/itc2008.md:
       536	grow(1, n) = (n + 1, 0),
       537	grow(i, n) = (e′, c + N), where (e′, c) = grow(i, (n, 0, 0)),
       539	grow((0, ir), (n, el, er)) = ((n, el, e′r), cr + 1), where (e′r, cr) = grow(ir, er),
       540	grow((il, 0), (n, el, er)) = ((n, e′l, er), cl + 1), where (e′l, cl) = grow(il, el),
       541	grow((il, ir), (n, el, er)) = { ((n, e′l, er), cl + 1) if cl < cr,

Resolution: a short paragraph on `grow` naming both extensions: the `1`-over-node arm is the paper's `(il, ir)` rule read on the unnormalized `(1, 1)`, present so the optimality proptests can quantify over arbitrary pairs; the empty-id arm is the infeasible sentinel; `event` reaches neither. Acceptance: a paper-reader diffing the oracle against §5.3.4 finds every untranscribed arm named with its reason.

**Nits (1), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| recursion-10 | `crates/before/src/oracle.rs:16-18` | Oracle prose says "boxed" trees and clones; the trees are Arc and clones are refcount bumps | oracle.rs:17-18 -> "the derived `Drop` of the `Arc`-linked trees" | `evidence/sweeps/recursion.md` |

**Cross-references.** oracle-laws-19 is the "door" census for laws.rs (41 uses). oracle-laws-22's "minted" for recording a tick is the same word the census tracks; its "tripwire" dispute is with the crate-local definition at surface_coverage.rs:71-92. paper-fidelity-6 (the §2-§4 attribution) and paper-fidelity-8 (the two extra `grow` arms) are the paper-fidelity sweep's oracle-side entries; recursion-10's "boxed" trees is the same file's representation slip. oracle-laws-1 and oracle-laws-2 (other classes) carry the oracle `Clock`'s false mirror claim and the hand-copied fold.

## The instruments: meter core (meter.rs and its tests)

5 findings (0 high, 2 medium, 2 low, 1 nit). Full records: `evidence/partitions/meter-core.md`.

### meter-core-3: `Packed` documents one coding but carries two; event-shape `bytes` are the construction language, which `Version::decode` cannot accept
- Where: crates/before/src/meter.rs:85-96 (related: meter.rs:6-9, 29-31, 116-121, 2286-2288; encode.rs:9-17, 38-41; meter/tests.rs:30-37, 1207-1219; tests/meter.rs:328-346, 433-434, 446, 459, 472, 1067-1070, 1075, 1089, 8397)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (encode.rs read: the transcoder inverts the flag and re-codes leaves as absolute-then-zigzag deltas; `check_version` decodes `v.encode()`, never `p.bytes`; construction-versus-stored sizes derived by hand: dense(1000) 4004 vs 3006 bits, cliff_comb(1000, 1000) 2,010,002 vs 14,002 bits); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate but expired (written in 6014124ad when the construction language was the stored coding; faf3cd0a made the skyline the stored coding and added `Packed::version`; d800957e re-edited the sentence without correcting it; the 205a361da ghost sweep did not touch meter.rs)
- Owner-gated: no for the doc; yes for the type split below (the meter surface is public under a feature)

The `Packed` doc's first sentence, and lines 87-88, say `bytes` "is what `decode` accepts and `encode` reproduces". For every event-shape generator the bytes are the construction language: flag `1` = internal (the skyline flags `0` internal), one `gamma(base)` per node, which `encode_bits` transcodes rather than decodes. Only id shapes' bytes decode directly. The same type carries both kinds with no discrimination, so `Shape::IdSpine.packed_flagged(5, false).version()` compiles and feeds an id wire to the event transcoder. `bits` is the construction stream's length, which differs asymptotically from the stored size (the module's own `plateau_puncture` doc at 2286-2288 and the pin at tests.rs:1207-1219 say so). Downstream, tests/meter.rs denominates the skyline `cmp_*`/`join_*`/`tick_dense` rows by `p.bytes.len()` while the kernel reads `version_of(&p)`, so the printed `MEASURED` input for `cmp_cliff` is about 143 times the operand the sweep read, and the `heap_meter_floor_on_decode_dense` floor asserts `peak >= p.bytes.len()` on the premise that "the decoded version owns a copy of the packed bits", whereas the version owns 3006 stored bits against 4004 construction bits and the floor passes only because `encode_bits` allocates `with_capacity(bits.len())` (encode.rs:24). Those downstream sites belong to the envelope partition; the false contract that leads there is this type's doc. This breaches the documentation-accuracy rule (a first sentence that is false for half the values the type holds) and, downstream, the doctrine that a liveness floor derives from irreducible work.

Evidence:

        85	/// A generator's output: canonical packed bytes plus the exact bit length.
        86	///
        87	/// `bytes` is what `decode` accepts and `encode` reproduces
        88	/// (marker-padded to a byte boundary); `bits` is the live bit length
        89	/// before that padding, so tests can pin the closed-form size of each
        90	/// shape.

    encode.rs
        38	        // The construction language flags `1` internal; the skyline stream
        39	        // flags `0` internal (`1` leaf), so the flag inverts at this transcode
        40	        // boundary.
        41	        out.push(!internal);

    tests/meter.rs
       330	/// The decoded version owns a copy of the packed bits, so the one big
       341	        peak >= p.bytes.len(),

Resolution: Restate the `Packed` doc as it is: for id shapes `bytes` is the crate codec and decodes directly; for event shapes `bytes` is the construction language and `version()` is the door to the stored form, whose wire then round-trips; `bits` is the construction stream's live length (the denominator the size pins use), not the stored size. Say in the module doc (6-9, 29-31) which coding each closed form denominates. Owner-gated option: split `Packed` into an id type and an event type (or an enum) so `version()` exists only for event shapes and the registry's `Builder` arms carry the distinction. Hand the envelope partition the two consequences: denominate skyline rows by `version_of(&p).encode().len()` as the `decode_*` rows and `tick_run` already do, and derive the decode-dense floor from the stored size. Acceptance: no sentence in meter.rs claims `Version::decode` accepts an event generator's `bytes`; the `Packed` doc names the two codings and which one each field measures; if split, `Shape::IdSpine.packed_flagged(..).version()` does not compile.

Construction: `assert!(Version::decode(&dense(3).bytes[..]).is_ok())` fails (the stream's first bit is `1`, which the skyline reads as a leaf flag over a topology that does not parse as one leaf); `assert_eq!(dense(1000).bits as u64, dense(1000).version().encoded_bits())` fails with 4004 against 3006.

### meter-core-4: The `cliff_comb` and `wide_tooth_comb` funding arguments describe the construction coding as "this coding" and the stored delta coding as a hypothetical
- Where: crates/before/src/meter.rs:220-224 (related: meter.rs:307-309; skyline.rs:102-110, 114; tests/meter.rs:7-9, 1067-1070)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (skyline.rs:102-114 read; the two passages read; stored-versus-construction per-crossing widths derived by hand: 3 bits per crossing stored, `2k + 1` per tooth constructed); executed: no
- Seen by: instrument-correctness; refutation: confirmed (git log -S dates the phrase to 7c3677192, 2026-07-23, before the storage flip); history: deliberate but expired (accurate when written; faf3cd0a decided the question; the 205a361da ghost sweep did not cover meter.rs or tests/meter.rs)
- Owner-gated: no

The comb's doc says that in "this coding" each tooth stores its own `gamma(2^k - 1)` so operations stay linear per input bit, and that "a delta coding of the same tree stores 3-bit ±1 codes per crossing instead, which is what makes this the separating family for the leaf-delta representation question". The stored skyline is that delta coding: skyline.rs:102-106 says the comb's payload stream is 3-bit codes on a `2^k` carry boundary so a plain running height pays `Theta(W^2)`, and skyline.rs:114 says "The stored form *is* this coding". On the operand the kernels read, the comb is the adversary the balanced accumulator exists for, not a funded control, and the representation question is decided. `wide_tooth_comb` makes the same claim with a temporal marker ("under today's coding", 308). Prose must speak in the present tense and a rationale the design rests on must describe the coding in use; a maintainer reading this learns the opposite of what the kernel doc states.

Evidence:

       220	/// crossing. In this coding each tooth stores its own `gamma(2^k − 1)` — `2k +
       221	/// 1` bits — so every crossing is paid for by a comparably-wide input code and
       222	/// operations stay linear per input bit; a delta coding of the same tree stores
       223	/// 3-bit `±1` codes per crossing instead, which is what makes this the
       224	/// separating family for the leaf-delta representation question.

       307	/// normalized region pays O(delta limbs). Each tooth stores `gamma(2^k − 2^w)`
       308	/// — `2k − 1` bits — so under today's coding every crossing is paid for by a
       309	/// comparably-wide input code.

    skyline.rs
       102	//! The accumulator choice is load-bearing, not an optimization: on the boundary
       103	//! comb (`meter::cliff_comb`) the payload stream is 3-bit `±1` codes sitting
       104	//! exactly on a `2^k` carry boundary, so a plain big-integer running height
       114	//! The stored form *is* this coding, so neither byte-level entry point

Resolution: Rewrite the two rationales in terms of the stored skyline: each crossing is a 3-bit (comb) or about `2w + 3`-bit (wide tooth) delta code while the carry spans `k` (or `k - w`) bits, and the balanced signed-digit accumulator is what keeps validation, sweep, and emit linear per stored bit; state that the construction spells the tooth magnitude per tooth only as a building convenience and is not the operand's size. Delete "the leaf-delta representation question" and "under today's coding". Hand tests/meter.rs:7-9 and 1067-1070 to the envelope partition for the same re-denomination. Acceptance: `grep -n "representation question\|under today's coding" crates/before/src/meter.rs` is empty; both docs name the stored per-crossing code width and the accumulator as the mechanism.

### meter-core-9: `factor_digit` and `dense_factor` docs differ from their code in the mixing input and a forced bit
- Where: crates/before/src/meter.rs:2325-2334 (related: meter.rs:2348-2350, 2360-2362)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (lines read; the `| 2` is what keeps digit 0 nonzero when the stream digit is 1 after `& !1`); executed: no
- Seen by: instrument-correctness, scaffolding, structure-prose; refutation: confirmed; history: no rationale found (both mismatches born together in 013334f2)
- Owner-gated: no

These are the committed content streams the plateau-puncture incompressibility argument rests on, and both docs must let a reader re-derive the digits exactly. `factor_digit`'s doc says "the SplitMix64 finalizer over `seed ⊕ i`" while the code mixes `seed ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15)`; `dense_factor`'s doc lists three forced bits while the code also forces bit 1 of digit 0 set, which is what keeps the digit nonzero and even together, and the doc's "every base-2^32 digit nonzero" rests on it unstated.

Evidence:

      2325	/// Digit `i` of the deterministic pseudorandom content stream `seed`: the
      2326	/// SplitMix64 finalizer over `seed ⊕ i`, truncated to one base-2^32 digit, zero
      2334	    let mut z = seed ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15);

      2348	/// The top bit is forced set (exact width), the top digit's bit 30 forced clear
      2349	/// (never an all-ones digit), and bit 0 forced clear (so `+ 1` never carries
      2350	/// past digit 0 — the gamma code of the value stays at its closed-form width).
      2361	            digit = (digit & !1) | 2;

Resolution: State the mix as the finalizer over `seed ⊕ (i · φ)` naming the golden-ratio constant, and add the fourth forcing to `dense_factor`'s list: "bit 0 forced clear and bit 1 forced set, so digit 0 is even and nonzero". Acceptance: both docs match the code line for line.

### meter-core-10: `arming_train`'s band is documented as `32w + ⌈log₂ n⌉ + 2` but computed as `32w + bitlen(n) + 2`, and the test re-spells the code instead of the doc
- Where: crates/before/src/meter.rs:2470-2472 (related: meter.rs:2490, 1679-1681; meter/tests.rs:1244-1245, 1253-1259)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (lines read; `bitlen(n) = floor(log2 n) + 1` exceeds `ceil(log2 n)` by one at every power of two: `n = 1` gives 1 vs 0, `n = 2` gives 2 vs 1, `n = 4` gives 3 vs 2, and the test table includes `n = 1, 2, 4`); executed: no
- Seen by: adequacy; refutation: confirmed; history: no rationale found (all three spellings born together in e695d5cf)
- Owner-gated: no

Prose states what is: a reader deriving the closed form from the doc gets a different bit count than the generator emits at `n ∈ {1, 2, 4}`. The test passes only because line 1259 recomputes the band as `(usize::BITS - n.leading_zeros()) as usize`, an inline copy of the `bitlen` it already imports (tests.rs:11), so the pin mirrors the implementation rather than the documented derivation and cannot catch the divergence it exists to guard.

Evidence:

      2470	/// every dense window to its right. All wide leaves live in one gamma band
      2471	/// (`band = 32w + ⌈log₂ n⌉ + 2` headroom bits over the swings and kickers), so
      2472	/// the packed size is the closed form `n(g(2·band + 132) + 8·band + 16) + 2`
      2490	    let band = 32 * w + bitlen(n) + 2;

    tests.rs
      1259	        let band = 32 * w + (usize::BITS - n.leading_zeros()) as usize + 2;

Resolution: State the band as `32w + bitlen(n) + 2` (equivalently `32w + ⌊log₂ n⌋ + 3`) in both docs, and have the test call `bitlen(n)` or, better, assert the doc's own spelling as an independent expression. Acceptance: the doc formula evaluated by hand at `n = 1, 2, 4` equals the generator's `band`; tests.rs:1259 no longer re-spells `bitlen`.

**Nits (1), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| meter-core-1 | `crates/before/src/meter.rs:18-27` | Module-doc and comment prose: unanchored coinage, duplicated sentence, history in prose, "mint", em-dashes in `//` comments | Point line 26 at the registry's own phrasing ("the roster entries the compiler cannot force ... | `evidence/partitions/meter-core.md` |

**Cross-references.** meter-core-3 and meter-core-4 are the flag-day family (with meter-registry-tier2-14, testing-diff-gen-17, envelopes-a-1); the envelope consequences (denominating skyline rows by construction-language bytes; the decode-dense floor's premise) are envelopes-a-10 and envelopes-a-14. meter-core-1's "luck-proof" is prose-hygiene-11's fourth term; the list it names is registry.rs:572-583. meter-core-10's `arming_train` off-by-one was relayed by envelopes-b's open question 7. meter-core-11 (another class) is the segments column the module doc presents as live.

## The instruments: the family registry and tier2

8 findings (1 high, 0 medium, 4 low, 3 nit). Full records: `evidence/partitions/meter-registry-tier2.md`, `evidence/sweeps/prose-hygiene.md`.

### meter-registry-tier2-14: `tier2.rs` describes the stored coding as a candidate and the deleted construction-language coding as "today's"; the `Tier2Size` formula is wrong against `encoded_bits`
- Where: crates/before/src/meter/tier2.rs:1-32 (related: tier2.rs:53-56, 73, 89-93, 104-120; crates/before/src/meter/tier2/tests.rs:5-7, 106, 121, 197, 231-236, 705-709; crates/before/src/version.rs:1151-1153; crates/before/src/version/skyline.rs:26-27, 114-129; crates/before/src/version/skyline/tests.rs:3-7; crates/before/src/testing/compactness.rs:1-9, 78-80 (out of partition))
- Class / severity / confidence: documentation / high / high
- Provenance: verified (read version.rs:1151-1153 (`encoded_bits` is `self.0.len()`, the stored skyline length), skyline.rs:26-27 ("This coding is the stored and wire form"), tests.rs:145 (asserts `v.encoded_bits() == 11` with the message "the stored coding is Tier 2 itself"), compactness.rs:78-80 and :97 (computes the stored-base bits from `packed.len()`, not `encoded_bits`); `grep -rnE 'Tier [0-9]'` over crates/before/src and crates/suanpan/src finds only "Tier 2"; `git log -- crates/before/src/meter/tier2.rs`: 3e1a1631 create (2026-07-23), faf3cd0a flag day (2026-07-25), b3f09baa docs WIP, 5d167a63); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed, plus two new sites (the `expect`/`assert` messages at 73 and 89-93 still say "canonical Version" though the argument has been a construction-language bit view since faf3cd0a); history: contradicts the root AGENTS.md hard rule (nothing refers to code that no longer exists); the framing was written while the decision was pending, faf3cd0a changed only intra-doc links and the input type in tier2.rs and partially re-denominated tier2/tests.rs, and b3f09baa re-wrapped the lines
- Owner-gated: no for the prose correction toward the code (sanctioned); renaming the module (public under the `meter` feature) is the owner's

The module doc says `tier2_size` computes what a Version "would have" under the Tier 2 coding "if re-encoded", that its topology bits are "exactly as today", that "The compactness ratio between this size and today's encoded size is the evidence the representation decision turns on", and that deltas are gamma-coded "exactly like today's stored bases". The `Tier2Size` doc says "today's encoded size is `nodes` flag bits plus its stored gamma codes, so the stored-base code bits are exactly `encoded_bits - nodes`". The stored coding has been the skyline coding, which is Tier 2, since faf3cd0a; `Version::encoded_bits()` is the stored skyline length, so `encoded_bits - nodes` is the first-leaf plus delta bits, not any stored-base bits (compactness.rs:97 computes the quantity correctly from `packed.len()`). A maintainer reading this module alone is told a decision is pending that was made in July and that the production coding is the one that was deleted. The module also never states its live role, the independent second implementation of the coding that every emitted stream's length is pinned against (skyline.rs:125-129; skyline/tests.rs:3-7), so its duplicate `zigzag`/`gamma_bits` read as accidental. The `# Panics` input description ("the packed form") is ambiguous now that the stored form is also a packed bit stream: `tier2_size` reads one gamma base per node and panics (`BitsView::bit` asserts at bits.rs:309, `decode_int` at 73) on a stored skyline stream. "Tier 2" is a design-note tier label with no Tier 1 anywhere in the tree. The same drift runs through the tests: "the hand-derived current bit count" (5-7), `single_small_leaf_matches_current_size`, `single_big_leaf_matches_current_size`, `cliff_comb_tier2_size_is_linear_while_current_is_quadratic` (where "current" is `packed.bits`, the construction-language length), "Under today's coding the same tree pays `2k + 1` stored bits per crossing" (231-233; under the stored coding it pays 3, which is the point of 188-190), and "10 bits against today's 14" (705-706).

Evidence:

         1	//! The exact encoded size a [`Version`](crate::Version) would have under the
         2	//! Tier 2 coding: preorder topology bits plus delta-coded absolute leaf values.
         4	//! A measurement tool, not a codec: [`tier2_size`] computes, bit-exactly, how
         5	//! large a canonical [`Version`](crate::Version) would be if re-encoded as its
         6	//! preorder topology (one flag bit per node, exactly as today) plus its leaf
        10	//! compactness ratio between this size and today's encoded size is the evidence
        11	//! the representation decision turns on, so the walk here is written for
        29	/// leaf. The parts are exposed separately so the compactness suite can charge
        30	/// the delta stream against today's stored bases (today's encoded size is
        31	/// `nodes` flag bits plus its stored gamma codes, so the stored-base code bits
        32	/// are exactly `encoded_bits - nodes`).
        73	        let (base, next) = codec::decode_int(bits, pos).expect("canonical Version parses cleanly");
        92	        "canonical Version walk consumes every packed bit"

    tier2/tests.rs:
       145	    assert_eq!(v.encoded_bits(), 11, "the stored coding is Tier 2 itself");
       197	fn cliff_comb_tier2_size_is_linear_while_current_is_quadratic() {
       231	/// `Θ(W²)` total in wire bits `W`. Under today's coding the same tree pays
       232	/// `2k + 1` stored bits per crossing (the envelope suite pins those operations

Resolution: rewrite against today's code. The module is the independent sizer of the stored skyline coding, computed from the construction-language stream (the min-lifted packed preorder form `meter::Packed::as_bits` and `testing::bridge::packed_bits_of` produce), never from a stored stream; list the coding's terms; name its consumers (length agreement in `version/skyline/tests.rs`, the kernel emission length pins in this module's tests, the plain-sweep pin, and the compactness ratio against the construction-language size `Packed::bits` if that suite stays); state the independence rule beside `zigzag`/`gamma_bits` ("re-derived here on purpose: sharing them with `version::skyline` would make length agreement check nothing"). In `Tier2Size`, replace "today's encoded size ... `encoded_bits - nodes`" with "the construction-language size (`Packed::bits`) minus `nodes`". Fix the `expect`/`assert` messages at 73 and 92 to name the construction-language stream. In the tests, rename `*_matches_current_size` to `*_matches_packed_spelling_size` (the term the file already uses at 118, 135, 163, 183) and `..._while_current_is_quadratic` to `..._while_the_packed_spelling_is_quadratic`; re-state 5-7, 231-233, 705-709; end 236 by naming that the production validator's `suanpan::Accumulator` is the cliff-free design (skyline.rs:102-110). Owner-gated suggestion: rename the module (`meter::sizer` or `meter::skyline_size`) and `Tier2Size` with it. Acceptance: `grep -nE "today|current|would (have|be)|representation decision|canonical Version" crates/before/src/meter/tier2.rs crates/before/src/meter/tier2/tests.rs` returns only lines whose referent is unambiguous and true of the stored coding; the `Tier2Size` doc names `Packed::bits`; the independence rule appears beside the duplicate helpers.

### meter-registry-tier2-1: Vocabulary tells in the registry prose: "mint" for construction, "earns", "sentinel", "honest", "luck", "mandate", "tombstone"
- Where: crates/before/src/meter/registry.rs:20-31 (related: registry.rs:43, 566, 572, 584, 636, 675, 734, 748, 762, 776, 794, 813, 832, 883, 887, 920, 1036, 1076; crates/before/src/meter/tier2/tests.rs:163)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -nwE` over the four partition files; every site listed was read); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: contradicts the owner's writing-style rule for "mint" (the rest is register guidance); the only "mint" purge (2c73d032) was scoped to the rumors crate
- Owner-gated: no

The registry is public rustdoc under the `meter` feature and uses "mint"/"minted" for constructing a value at three sites, which the owner's vocabulary rule forbids outright; "earns a column" (five sites), "honest-less-work witness", and "never mandate" are register transplants where no economy exists; "sentinel" in a watchman sense (seven family docs) is a coinage the roster already names as "probe"; "found by luck" and "coalescing luck" name an unchecked step instead of the mechanism; "tombstone" is a metaphor with no mechanism and collides with the rumors crate's redaction rule. One curly apostrophe sits in tier2/tests.rs:163.

Evidence:

        20	//!   or downstream crate — can mint an adversarial shape except through
        31	//!   // The registry door: the same comb, minted through its Shape row.
       572	/// compiler cannot force, in the order it is otherwise found by luck: the
       675	/// difference is minted at every consume and popped at every close — the
       883	    /// zero, so the pair is also the touch floor's honest-less-work witness
       887	    /// implementation, never mandate.
       920	    /// live records, the live-anchored followers' tombstone.

Resolution: "build"/"built" for mint; "has a column"/"gets no column" for earns; "probe" for sentinel; "minimal-work witness" and "never a requirement" at 883/887; name the mechanism at 920 ("the shape that retires every live-anchored follower"); "otherwise unchecked" at 572 and "the adjacent-slot coalescing that index order would allow" at 636; a straight apostrophe at tests.rs:163. Acceptance: `grep -nwiE 'mint|minted|earns?|sentinel|honest|luck|mandate|tombstone' crates/before/src/meter/registry.rs` returns nothing, and `grep -n "’" crates/before/src/meter/tier2/tests.rs` is empty.

### meter-registry-tier2-6: Measured tripwire readings (×1.50, ×1.74) and a constant's value are restated in family docs
- Where: crates/before/src/meter/registry.rs:741-745 (related: registry.rs:756-759, 609-610; crates/before/src/version/skyline/query/tests.rs:1487-1493, 1798-1804; crates/before/src/meter/board/family.rs:378)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read query/tests.rs 1484-1510 and 1795-1821: the asserted floors are `* 125` and `* 136`, the ×1.50/×1.74 figures are bracketed measurement notes; `WEAVE_GROUPS: usize = 16` at family.rs:378); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed; history: introduced at cc84df7e; the 500d4d09 sweep that excised measured snapshots from meter prose reworded the weight-comb/freeze-parade readings but left these
- Owner-gated: no

The FreezePos and PromoRearm docs quote the known-bad kernels' measured per-byte growth; the committed tripwires enforce floors of 1.25 and 1.36, so the quoted numbers are snapshots no test compares and will rot on the next re-measure (the notes say "measured in the dev profile"). Unlike DenseSuffix (803-805) and PlateauPuncture (843-846), these rows do not name the tripwire functions the roster holds live. The Weave doc restates `WEAVE_GROUPS` as "16 group parties". Principle 5: a number that matters lives in a mechanically-enforced place that prose cites by name.

Evidence:

       741	    /// while the family's positions compact to O(1) digits. The committed
       742	    /// known-bad kernel reads ×1.50 per byte across the doubling on this shape
       743	    /// (the query fold's adequacy tripwire); the anchored-segment discipline
       756	    /// to O(1) balanced terms. The committed known-bad kernel reads ×1.74 per
       609	    /// The weave fold population: the leaves of one balanced fork tree dealt
       610	    /// round-robin among 16 group parties (the board's weave-group constant),

Resolution: replace the readings with the kernel names (`absolute_position_accounting_reads_superlinear_on_freeze_position`, `span_promotion_accounting_reads_superlinear_on_rearm_spine`), as the DenseSuffix and PlateauPuncture rows do, and "16 group parties" with "`WEAVE_GROUPS` group parties". Acceptance: `grep -nE '×1\.[0-9]+|16 group' crates/before/src/meter/registry.rs` returns nothing.

### meter-registry-tier2-21: `grid_version::build` recurses on depth outside the `recurse.rs` inventory of test-local recursive witnesses
- Where: crates/before/src/meter/tier2/tests.rs:468-484 (related: crates/before/src/recurse.rs:9-14; crates/before/AGENTS.md:32-36; `descend!` sites: testing/bridge.rs:56-57, 150-151; version/skyline/grow/tests.rs:125, 172, 177; meter/tests.rs:417)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (read recurse.rs:1-20 and AGENTS.md:26-37; `grep -rn 'descend!('` over crates/before/src lists the bridge, the grow suite's probe, and the meter suite's dive, matching the inventory; `build` at 469-477 recurses without `descend!` and is not listed); executed: no
- Seen by: adequacy; refutation: confirmed; history: contradicts the letter of the AGENTS.md hard rule (a walk that must recurse routes through `descend!`; the inventory in recurse.rs names today's recursive surfaces); `build` (eec23b79) predates the inventory (1ddb5a48) by eight days
- Owner-gated: no

The grid builder recurses on `log2(values.len())` without `descend!`, and recurse.rs's inventory (which AGENTS.md points at as holding "the inventory and the keep decision") names only the bridge, the grow suite's reference cost probe, and the meter suite's segment-liveness dive. Depth is bounded by the tests' own constants (at most 64 cells, depth 6), so the rule's goal is met; the inventory is a hand-maintained enumeration that has drifted (Principle 5), and the rule's letter is not followed at this site.

Evidence:

       466	/// canonical whatever the values. Recursive over the grid's `O(log)` depth
       467	/// (test-only; the measured paths are iterative).
       468	fn grid_version(values: &[Base]) -> Version {
       469	    fn build(values: &[Base]) -> oracle::Version {
       470	        match values {
       471	            [v] => oracle::Version::leaf(v.clone()),
       472	            _ => {
       473	                let (l, r) = values.split_at(values.len() / 2);
       474	                oracle::Version::node(0u64, build(l), build(r))

Resolution: build the grid iteratively by pairing bottom-up (which removes the recursion and the inventory question), or route the two recursive calls through `descend!` and add the site to recurse.rs's inventory with the bounded-depth note. Acceptance: recurse.rs's inventory names every test-local recursive fn, or `build` is iterative.

### prose-hygiene-7: Ruling dates embedded as code data, consumed only by a date-shape check
- Where: crates/before/src/meter/registry.rs:1102-1103 (related: registry.rs:1078-1079, 1094-1095, the ten `decided:` literals at 1859, 1864, 1893, 1923, 1945 and surfacecheck check.rs:60, 68, 77, 86; the nine `decided: REGISTRY_RATIFIED` uses; crates/before/src/meter/registry/tests.rs:204-224; crates/before/surfacecheck/src/check.rs:33-34, 259-273; the prose describing the schema at registry.rs:42, 50, 58, 566, 585, 1035, 1039, 1074, 1089, 1960, board.rs:240, check.rs:7, 149, main.rs:16)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (`grep -w decided` over before/src, surfacecheck/src and tests: every read of the field is a shape check; read both checks and the excision commit's message); executed: no
- Verification: confirmed; history: already-known (d2a9d04e's message reports the machinery to the owner verbatim: "the enforced dated-exception machinery ... requires field and test changes to dissolve — a design round, not a prose sweep")
- Owner-gated: yes: dissolving the field is a schema and test change the owner has already been asked to rule on

Every ruling carries a `decided: "YYYY-MM-DD"` and the registry a
`REGISTRY_RATIFIED` constant; the only readers check that the string is
shaped like a date. `git log -S` on any reason string yields the ruling date
exactly.

Evidence:

      1102	/// The date the registry's rulings were ratified as the rows of record.
      1103	const REGISTRY_RATIFIED: &str = "2026-07-29";

    registry/tests.rs:
       215	                decided.len() == 10 && decided.chars().filter(|&c| c == '-').count() == 2,
       216	                "{family:?}'s ruling date `{decided}` is not a YYYY-MM-DD date"

    check.rs:
        33	    /// The date of the ruling of record (YYYY-MM-DD).
        34	    pub decided: &'static str,

Resolution: owner ruling. Keep: say once at each `decided` field's doc that
the registry is an embedded decision record and dates are part of its
schema. Dissolve: drop the `decided` fields, `REGISTRY_RATIFIED`, the two
shape checks, and rewrite the fourteen "dated reason/ruling/exception"
phrases listed above to "reason of record". Acceptance: either the field's
doc states the decision-record rationale, or `grep -rn -w -i dated` over
before/src and surfacecheck returns nothing.

**Nits (3), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| meter-registry-tier2-2 | `crates/before/src/meter/registry.rs:46-50` | The module doc calls the band-to-family link compiler-checked; the compiler checks only band-to-Shape | reword to "the band-to-shape link is a compiler-checked construction site; the band-to-family link is the spec's `Bands` roster ... | `evidence/partitions/meter-registry-tier2.md` |
| meter-registry-tier2-4 | `crates/before/src/meter/registry.rs:100-174` | Shape notation letters collide: B, F, W, A each name two shapes | give the later coinages distinct abbreviations (the two-letter style the newer families already use: `MB`, `MF`, `DR` ... | `evidence/partitions/meter-registry-tier2.md` |
| meter-registry-tier2-5 | `crates/before/src/meter/registry.rs:384-386` | `wrong_door` points at the variant doc instead of naming the accessor; the door argument is stated twice | give `Builder` an `fn accessor(&self) -> &'static str` and have `wrong_door` print "{self:?} builds through {right}, not {called}" ... | `evidence/partitions/meter-registry-tier2.md` |

**Cross-references.** meter-registry-tier2-14 is the flag-day family's high entry (with meter-core-3/-4, testing-diff-gen-17, envelopes-a-1). meter-registry-tier2-6's measured readings and family.rs's (board-families-floors-judge-1) are the same 500d4d09 residue. meter-registry-tier2-21 belongs with recursion-4's inventory framing. prose-hygiene-7's `decided` dates are this module's schema (Open questions 6). meter-registry-tier2-1's "mint"/"earns"/"sentinel" are census sites.

## The instruments: the amplification board (frame; families, floors, judge; ops, render, shards, tests)

30 findings (0 high, 5 medium, 18 low, 7 nit). Full records: `evidence/partitions/board-families-floors-judge.md`, `evidence/partitions/board-frame.md`, `evidence/partitions/board-ops-render.md`, `evidence/sweeps/meter-adequacy.md`, `evidence/sweeps/prose-hygiene.md`.

### board-families-floors-judge-1: Base-size docs quote probe-build readings the owner's excision ruling classifies as excised; one is labeled "committed" against its source, two have no committed kernel
- Where: crates/before/src/meter/board/family.rs:27-38 (related: family.rs:236-240, 250-254, 272-276, 288-292, 306-317; ceilings.rs:56-62; tests/meter.rs:4551, 5220, 5596; tests/superlinear_tripwires.rs:27-68)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git show 500d4d09 -- family.rs` touched only the FREEZE_POS, PROMO_REARM, and RevealComb passages; `git blame` puts every cited range at b3f09baa09, 2026-08-06, five days before the sweep; the tripwire roster and tests/meter.rs lines read); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed (partial sweep, not a policy exemption); history: deliberate-but-expired
- Owner-gated: no (the ruling exists; this applies it)

Commit 500d4d09 ("excise measured snapshots from meter prose") rules that probe-build readings are excised and known-bad separations restated as complexity classes, and names board/family.rs among the swept surfaces; six constant docs survived it: HUGELEAF (29-35: "~1× to ~4×", "e 1.41"), WEIGHT_COMB (238: ×1.93), FREEZE_PARADE (252: ×1.91), DENSE_SUFFIX (274: ×1.96), WIDE_ARMING (290-291: ~×1.9, ×2.00, plus the copied literals 500 and 256), PLATEAU_PUNCTURE (311-317: ×1.879, ×1.579, ×1.555, ×1.91, ×1.65). The weight-comb line calls its source "the band ceiling doc's committed probe-build measurement" where tests/meter.rs:4551 says "a local probe build", and the tripwire roster has no weight-comb or freeze-parade kernel, so those two readings have no committed instrument behind them. Principle 5 (dated measurement reports are not exempt) and Principle 8 (a number you were handed is a hypothesis).

Evidence:

        29	/// Sized so the level doubling stays inside one backend decimal-conversion
        30	/// regime: the backend's divide-and-conquer parser switches algorithm between
        31	/// 16,000 and 20,000 value bits (its parse transient steps from ~1× to ~4× the
        32	/// value bytes there, by measurement), and a probe pair straddling that switch
        33	/// reads the step as a heap exponent — a 16,000-bit base fits e 1.41 on the

       237	/// of the `skyline_flatness` weight-comb band's small run: with certificate
       238	/// consumption disabled, rank reads ×1.93 per-byte growth across this regime's
       239	/// doubling (the band ceiling doc's committed probe-build measurement), so the

    tests/meter.rs:
      4551	    /// consumption disabled (a local probe build whose scans step

    ceilings.rs:
        56	// Several ceilings below argue their calibration from the worst honest reader
        57	// at the release profile of record. The readings themselves live in the pin
        58	// commits (`git log -S` the constant), never in this prose: a quoted reading
        59	// would keep asserting itself as present-tense fact while headroom absorbed

Resolution: Apply the 500d4d09 treatment to the six docs: keep the design argument (which band's small run the base matches, why the pair straddles the regime, the mod-32 remainder alignment), restate each known-bad separation as a class ("the known-bad settle reads a quadratic's ~×2 per byte per doubling"), cite the committed `_reads_superlinear` kernel by name where one exists, and for weight comb and freeze parade either write "a local probe build" as the band doc does or commit the kernel. Replace the literals 500 and 256 at 289-290 with the two `WIDE_ARMING_SMALL` names. For HUGELEAF, state the backend-regime boundary as the constraint (both probes on one side of the parser's algorithm switch) without the measured exponents. Acceptance: `grep -nE '×[0-9]\.[0-9]{2,3}|e 1\.41|~1× to ~4×' crates/before/src/meter/board/family.rs` returns nothing; "committed" is not applied to a probe-build measurement; every cited kernel name matches a row in tests/superlinear_tripwires.rs.

### board-families-floors-judge-8: "deterministic-liveness" floors are undefined, spelled with "today" in six rendered strings, and contradicted by the module's opening sentence
- Where: crates/before/src/meter/board/floors.rs:5-8 (related: floors.rs:41-42, 64-69, 171-183, 216-221, 230-266, 287-293; board.rs:88-91, 102-104; currency.rs:118-119, 138-140; operand.rs:17)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn 'deterministic-liveness' crates/before/src` finds uses in floors.rs, currency.rs:119, operand.rs:17 and no definition; `grep -nw today floors.rs` gives 175, 182, 220, 233, 265, 291, all inside `WHY_` strings; board.rs:88-91 read); executed: no
- Seen by: scaffolding, structure-prose (and the "today" half of the scaffolding vocabulary sweep); refutation: confirmed; history: deliberate-and-holds (both floor classes are intended and the class's purpose is stated at board.rs:102-104 and floors.rs:64-69; only the definition and the opening sentence are missing)
- Owner-gated: no (a reconciliation of two in-code statements, not a design change)

The module opens by stating every floor is derived from what the operation must do, "never from how it does it" (board.rs:88-91 says the same), yet nine floors are pinned to the shipped kernel's mechanism under the tag "deterministic-liveness", a coinage defined nowhere, and six of their rendered legend strings carry the relative date "today". The distinction between a floor no conforming implementation can undercut and a floor pinned to the current kernel is the first thing a maintainer reading a floor trip needs, and the file's first sentence tells them the second kind does not exist. Principle 5 (dated rationale at a declaration site) and the vocabulary rule (a coined term is anchored to an identifier or defined once by contrast).

Evidence:

         5	//! A floor states the least a watching counter can honestly read, derived from
         6	//! what the operation must do, never from how it does it; the board module
         7	//! doc's Liveness floors section carries the criterion and what a trip means.

        41	//! - **Touch** floors are deterministic-liveness declarations, like the
        42	//!   fork rows' heap floor, at three derivations. The single-operand

       173	pub(super) const WHY_LIMB_RANK_ENCODE: &str =
       174	    "deterministic-liveness: the encoder extracts and biases the integral part through \
       175	     one arithmetic pass over the numerator today, one op per 64 numerator bits; a \
       176	     pure bit-walk emission (riding the bias as a carry) would lower this floor \
       177	     deliberately";

Resolution: Define the two kinds once, by contrast, at the top of the module doc (a contract floor, derived from mandatory work and sound for every conforming implementation; a kernel-pinned floor, derived from the shipped kernel's mechanism and lowered deliberately by a re-representation, committed so state migrating off the meter trips red), and fix the opening sentence here and board.rs:88-91 to admit both. Delete "today" from the six `WHY_` strings and currency.rs:139: the trailing "would lower this floor deliberately" clause already carries the mutability. Consider anchoring the kind to an identifier (a `FloorKind` field on `Liveness::Floor`, or one shared prefix constant) so the legend's tag is not free prose. Acceptance: `grep -nw today crates/before/src/meter/board` returns nothing; the module doc defines both kinds before using either; board.rs's liveness section no longer says "never from how it does it" without qualification.

### board-ops-render-29: A test doc quotes a dated measurement with its history and cites a design document from code
- Where: crates/before/src/meter/board/tests.rs:489-505 (related: tests.rs:862-863; ceilings.rs:54-62)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -n 'design doc\|§\|Measured\|landed' src/meter/board/*.rs` returns only tests.rs:490 and 505, so the module is clean after the deletion; `git log -S'33,036'` attributes the number to a99e8c8f7, the cure commit; ceilings.rs:56-62 read); executed: no
- Seen by: scaffolding [12], adequacy [22], structure-prose [28], instrument-correctness [52]; refutation: confirmed (adding tests.rs:862-863 "the readings that were red under the flat ceilings" as the same genre); history: contradicts-hard-rule (the global doctrine's "Never reference design documents in code comments or rustdoc" and Principle 5; the tree's own convention at ceilings.rs:56-62 set by 500d4d09, whose grep sweep of board/tests.rs missed the bracketed form; the cited document was moved to .agent-notes at 15bd905e0, so the pointer now resolves to an LLM-written note)
- Owner-gated: no

`join_all_overlap_upfront_test_reads_flat`'s doc records a past reading and the landing of a re-pin as history and cites "the design doc's §3 entry". The crate's own rule (ceilings.rs:56-62) is that readings live in pin commits, never in prose, because "a quoted reading would keep asserting itself as present-tense fact while headroom absorbed the drift"; the doctrine forbids design-doc citations from code. Lines 862-863 narrate history in the same vein.

Evidence:

       490	/// \[Measured ×2.00, 33,036 → 66,060 bits — the re-pin landed with the per-call
       491	/// index.\]

       504	/// board's `party_join_all_overlap` row carries the same reading at the scales
       505	/// of record; the cure's decision record lives in the design doc's §3 entry.

    ceilings.rs:
        57	// at the release profile of record. The readings themselves live in the pin
        58	// commits (`git log -S` the constant), never in this prose: a quoted reading

Resolution: Delete the bracketed measurement and the "§3 entry" sentence (the paragraph already states the mechanism: a linear discipline reads ×2.00 across the joint doubling, the ceiling adds rounding headroom, a per-input re-walk reads ~×4); re-word 862-863 to describe the probe's readings without "were red". Acceptance: `grep -n 'design doc\|§\|Measured\|landed\|were red' crates/before/src/meter/board/*.rs` returns nothing.

### meter-adequacy-3: The exponent instruments admit an n log n regression at every committed family size, and MAX_SCALING_EXPONENT's rustdoc says the opposite
- Where: crates/before/src/meter/board/ceilings.rs:64-69 (related: crates/before/src/meter/board/judge.rs:37-54, crates/before/src/meter/board/ceilings.rs:338-358, crates/before/src/meter/board/ceilings.rs:445-467, crates/before/src/meter/board/family.rs:18-19, crates/before/src/meter/board/family.rs:89-92, crates/before/src/meter/board/family.rs:490-494, crates/before/tests/meter.rs:2342-2347, crates/before/tests/meter.rs:2392-2408)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (Python reproduction of `judge.rs::trend`'s least-squares on synthetic ladders at the committed sizes); executed: yes: a scratch Python program computing the log-log slope of `c·n·log2(n)` over the x8 ladder (levels 0..3 from `family.rs::build`'s `<< level`, `LADDER_TOP_SCALE` 4) at 4 KiB base reads 1.107 (bytes as the log argument) or 1.088 (bits), at 1 KiB base 1.126 / 1.100; the slope reaches 1.15 only below a ~256 B base; `log2(2n)/log2(n)` at the 66 KB flatness streams is 1.0625
- Verification: confirmed; history: no-rationale-found (the constant's doc has stated this since the board landed; the fold ceiling's doc at 342-346 already derives the log factor's marginal as `1 + log2(log2(2k2)/log2(2k1))/log2(D2/D1)`, which is the same arithmetic and reaches 1.15 only because its `k` is small)
- Owner-gated: no

The board's exponent ceiling (1.15 over an x8 ladder) and the envelope
suite's flatness convention (x1.25 across one doubling) both admit a
logarithmic factor at every committed operand size: a per-node `BTreeMap`
probe or a binary search per element reads slope ~1.09-1.13 on the board
and ~1.06 in the flatness bands. What these legs pin is polynomial
superlinearity. That is a defensible scope, but the constant's rustdoc
asserts it excludes "a real log factor at these input sizes", and no
instrument in the suite refutes a log factor's presence (the asymptotics
pins assert its presence where it is documented).

Evidence:

        64	/// Green requires every meter's scaling exponent at or below this.
        65	///
        66	/// The contract is amortized-linear; 1.15 leaves room for measurement noise
        67	/// (allocator rounding, `Vec` doubling) without admitting a real log factor at
        68	/// these input sizes.
        69	pub const MAX_SCALING_EXPONENT: f64 = 1.15;

    crates/before/src/meter/board/family.rs
        18	/// Dense event spine depth at scale 1.0 (packed size ~4 KiB).
        19	const DENSE_BASE_DEPTH: usize = 8_000;
       ...
       491	        let size = |base: usize| -> usize {
       492	            let scaled = ((base as f64) * scale).round() as usize;
       493	            scaled.max(MIN_SIZE_PARAM) << level
       494	        };

    crates/before/tests/meter.rs
      2342	    /// Slack numerator over the small-scale cost (denominator
      2343	    /// [`SLACK_DEN`]): the ×1.25 flatness convention.
      2344	    const SLACK_NUM: u64 = 5;

Resolution: re-state the constant's rustdoc to what it excludes:
polynomial superlinearity, with the crossover stated (an `n log n` cost
over the x8 ladder fits slope `1 + 1/(ln 2 · log2 n)` ≈ 1.11 at 4 KiB, so a
log factor is excluded only below ~256 B). Add the same sentence to the
envelope suite's flatness convention at tests/meter.rs:2342-2347. If a log
factor is meant to be excluded on some rows, that needs a many-point trend
over a wider span or a closed-form witness per row, which is an owner
decision about cost. Acceptance: the rustdoc states the admitted class;
optionally a unit test in `src/meter/board/tests.rs` feeding `trend` the
four points `(4096·2^l, 4096·2^l·(12+l)·c)` and asserting the slope is
under `MAX_SCALING_EXPONENT`, documenting the admitted class in code.

### prose-hygiene-3: Design-doc citation from code in the board tests, and the doc has moved
- Where: crates/before/src/meter/board/tests.rs:505-505 (related: .agent-notes/2026-07-22-before-adversarial-resource-amplification/, .agent-notes/2026-08-19-design-directory-migration/README.md)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read 495-512; `git log -S` on the phrase; located the cited document with grep over `.agent-notes/`); executed: no
- Verification: reframed: the citation breaches the rule, and its target no longer sits where "the design doc" implies. The adversarial-resource-amplification design document was moved from `design/` into `.agent-notes/2026-07-22-before-adversarial-resource-amplification/` by the 2026-08-19 migration; the root `design/` holds only `rumors-frame-fuzz.md`; history: no-rationale-found (introduced in d3de44396 on 2026-07-26)
- Owner-gated: no

A test doc comment sends the reader to "the design doc's §3 entry" for the
cure's decision record instead of stating the mechanism, and the pointer now
resolves to an agent note.

Evidence:

       504	/// board's `party_join_all_overlap` row carries the same reading at the scales
       505	/// of record; the cure's decision record lives in the design doc's §3 entry.

Resolution: delete the clause after the semicolon; if the cure's rationale
matters to the test, state it in one sentence here. Acceptance: no in-scope
code or rustdoc contains "design doc" or a section-number pointer.

### board-families-floors-judge-2: The `version2` slot doc enumerates two pair shapes; four build arms fill the slot
- Where: crates/before/src/meter/board/family.rs:413-415 (related: family.rs:679, 717, 751, 761)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the four `data.version2 = Some(..)` arms; `git log -S'jump-pair, concurrent-pair'` traces the phrase to d88be7d9, 2026-07-27, before the tooth-tail and dense-suffix arms landed); executed: no
- Seen by: adequacy; refutation: confirmed; history: deliberate-but-expired
- Owner-gated: no

The slot doc names "the pair shapes (jump-pair, concurrent-pair)" as the only arms that fill `version2` themselves; the `DenseSuffix` arm (717) and the `ToothTail` arm (761) also set it. A hand-maintained enumeration of a fact the code changes without touching the prose (Principle 5), already rotted.

Evidence:

       413	    /// Derived uniformly by the post-pass — except on the pair shapes
       414	    /// (jump-pair, concurrent-pair), whose build arms fill it with the pairing
       415	    /// the shape was constructed around and the post-pass leaves in place.

       717	                data.version2 = Some(Shape::DenseSuffixMate.packed2(p, p).version().encode());

       761	                data.version2 = Some(b.version().encode());

Resolution: State the structure, not the roster: "except where a build arm fills it with the pairing the shape was constructed around; the post-pass leaves such a pairing in place". Acceptance: the doc names no shapes.

### board-families-floors-judge-12: The same two derivations are restated up to seven times across floors.rs and operand.rs
- Where: crates/before/src/meter/board/floors.rs:159-162 (related: floors.rs:27-40, 49-60, 237-245, 302-308, 458-478, 494-504, 579-584, 773-781; operand.rs:47-56, 127-137)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read); executed: no
- Seen by: structure-prose (and the judge.rs half of the same lens finding, handled in finding 23); refutation: confirmed; history: partially deliberate (50a017d0 restated the touch derivations in the module doc because private intra-doc links have no path in the public build; the constructor-level repeats have no rationale)
- Owner-gated: no

The stream-codes-not-tree-values limb argument ("a plateau of equal wide leaves stores its width once") is written at floors.rs:27-40, 159-162, 302-308, 579-584, 773-781 and operand.rs:47-56, 127-137; the pair-fold max-not-sum argument at floors.rs:49-60, 237-245, 458-478, 494-504. Doc altitude: every sentence competes with the contract the reader came for, and a derivation with seven homes drifts (one site says "provably need not", another "legitimately"). The module-doc copy is deliberate (a link-check constraint); the constructor and operand.rs copies are not.

Evidence:

       159	const WHY_LIMB_STREAM: &str = "every payload code of the stored stream wider than the \
       160	     machine-word bound must be decoded limb by limb: one op per 64 code bits (the stream's \
       161	     own codes, not the decoded tree's values — a plateau of equal wide leaves stores its \
       162	     width once)";

        31	//!   stored payload code wider than [`MACHINE_WORD_MAGNITUDE_BITS`](super::ceilings::MACHINE_WORD_MAGNITUDE_BITS) — a
        32	//!   plateau of equal wide leaves stores its width once and steps by
        33	//!   unit deltas after, and a conforming walk provably need not

Resolution: Give each derivation one home (the constructor whose `why` string it justifies: `limb_stream`, `touch_pair_fold`) and have the sibling constructors and operand.rs point at it by name; keep the module doc's prose statement (the link constraint) and the rendered strings to the one-line mechanism. Acceptance: `grep -c 'stores its width once' crates/before/src/meter/board/*.rs` is at most 2 (the module doc and the constructor).

### board-families-floors-judge-17: The tick rows' 8-bits-per-byte scan floor is derived from a single examination that records fewer bits than the floor, and nothing states why these rows alone floor at the full rate
- Where: crates/before/src/meter/board/floors.rs:782-793 (related: floors.rs:226-229, 773-775; ceilings.rs:56-62, 129-145; board.rs:106-113; codec/scan.rs:9-17; codec/build.rs:83, 95, 113, 128, 145, 180; version.rs:1129-1131)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (scan.rs's recording rules and build.rs's six `record_bits` sites read; the size law at version.rs:1129-1131; `git show -s 500d4d09` lists "the tick walk's 2-5x floor margin" among kept calibrations; 3b2d00c9's message and bd75d025's rewording per the history pass); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: reframed (the derivation wording is imprecise by at most 16 bits per pair; the shipped fill walk clears the floor because the builder records emitted writes; the 2-5x aside is a deliberately kept calibration, so that part is refuted); history: deliberate-and-holds for both rates, with no stated reason why the tick rows differ
- Owner-gated: yes (the eighth-versus-full choice is measurement policy)

WHY_SCAN_TICK_WALK derives 8 bits per input byte as "the walk's irreducible single examination", but a single examination through the metered cursors records the live bits (`encoded_bits()`), which are strictly fewer than 8 times `encode().len()` for every operand (the marker bit and padding: 1 to 8 bits short per operand). The floor holds because the codec builder also records every emitted bit, which the derivation does not name; a metered read-only pass would sit under it. Separately, every other full-examination row floors at `SCAN_FLOOR_BITS_PER_INPUT_BYTE = 1`, defended at board.rs:106-113 as the vacuity-detection eighth, and neither constant's doc says why the tick rows alone carry the full rate (my reading: because the fill walk emits output whose writes are recorded, roughly doubling a read-only walk's count, so the full rate is only sound where emission is metered). The 2-5x margin at 773-775 is a kept calibration by the owner's ruling, but ceilings.rs:56-62 states the no-readings convention without the kept-calibration class the ruling names, so a reader cannot tell the reading is sanctioned.

Evidence:

       227	const WHY_SCAN_TICK_WALK: &str = "the paired fill walk examines every topology bit and payload \
       228	     code of both operands at least once: 8 bits per input byte, the walk's irreducible single \
       229	     examination";

       787	        scan: Liveness::Floor {
       788	            min: (packed_bytes as u64).saturating_mul(TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE),
       789	            why: WHY_SCAN_TICK_WALK,
       790	        },

    ceilings.rs:
       132	/// One bit per byte is an eighth of the stored bits: far below any honest full
       133	/// walk (measured ~8 bits per byte across the board), and far above a counter
       134	/// that has stopped watching (which reads ~0).
       135	pub const SCAN_FLOOR_BITS_PER_INPUT_BYTE: f64 = 1.0;

    version.rs:
      1129	    /// The exact length in bits of [`encode`](Self::encode) before its
      1130	    /// padding — the marker bit and zero-pad to the byte boundary, so
      1131	    /// `encode().len()` is `(encoded_bits() + 1).div_ceil(8)`.

Resolution: Either floor the tick rows at the exact single examination (`version.encoded_bits() + party.encoded_bits()`, still eight times the universal floor) and restate WHY_SCAN_TICK_WALK in live bits, or keep 8 per byte and name the recorded emission in the derivation. State at `SCAN_FLOOR_BITS_PER_INPUT_BYTE` (or `TICK_WALK_SCAN_FLOOR_BITS_PER_BYTE`) why the tick rows alone take the full rate. Add the kept-calibration class to the ceilings.rs:56-62 convention header so the 2-5x margin reads as sanctioned. Acceptance: the WHY string derives the number it states; the two scan-floor constants' docs explain their relation; the convention header names what readings may remain.
Construction: For any cross bundle, `8 * (v.encode().len() + p.encode().len()) - (v.encoded_bits() + p.encoded_bits())` is at least 2; a metered read-only pass (a `DsiCursor` skipping each code once over `v` plus an `IdReader` over `p`, no builder) records exactly the live-bit sum and reads below the committed floor.

### board-families-floors-judge-18: `trend`'s docstring claims a lumpy counter "errs red, never green"; a counter dark at the larger point reads green
- Where: crates/before/src/meter/board/judge.rs:33-36 (related: judge.rs:37-54)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (my transcription of `trend` run in python: `trend([(100,5),(200,0)]) = -2.322`, `trend([(100,0),(200,5)]) = +2.322`; no cargo run); executed: no (a python transcription, not the Rust function)
- Seen by: adequacy; refutation: confirmed; history: no-rationale-found (added by 248d5539 as a doc clarification with no analysis of the direction)
- Owner-gated: no

The clamp `max(m, 1)` steepens the fit only when the zero is at the smaller denominator; a counter reading nonzero at the small size and zero at the large size yields a negative slope (green on the exponent leg) and a zero constant (green), so only a declared floor catches it. The following clause already hands vacuously quiet counters to the floors, which limits the exposure to NA-declared columns, but the universal "never green" is false as stated (statement faithfulness).

Evidence:

        33	/// variance) score 0. A sparse, lumpy counter therefore errs red, never
        34	/// green: its clamped zeros steepen the fit toward the exponent ceiling (a
        35	/// conservative false red to triage), and a vacuously quiet counter is the
        36	/// liveness floors' business, not the trend's.

Resolution: State the direction-dependence: a zero at a smaller point steepens the fit (a conservative false red); a zero at a larger point flattens it and reads green, which is why every judged column carries a liveness declaration. Optionally pin `trend(&[(100, 5), (200, 0)]) < 0.0` in a one-line judge test. Acceptance: the docstring states the direction-dependence.
Construction: `assert!(trend(&[(100, 5), (200, 0)]) < 0.0)`; through `evaluate` with all-NA floors and `limb: Some(60)` then `Some(0)` over denominators 100 -> 200, `red` is empty.

### board-families-floors-judge-22: Constants are judged at each window's larger size only; board.rs says "per size across the ladder"
- Where: crates/before/src/meter/board/judge.rs:256-263 (related: judge.rs:315-337, 381-387, 414-418; board.rs:69-71, 222-224)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: `per_unit` is computed from `m2`/`s2` alone and the declared heap/limb ceilings apply to that one value at 342 and 352, while floors (381-387) and the capacity band (325-336) check both samples); executed: no
- Seen by: adequacy; refutation: confirmed; history: deliberate-and-holds (board.rs:69-71 states "a per-denominator-byte constant at the larger scale"; 9e36dd28's "per size" meant per window; the reason for excluding the smaller size is stated nowhere)
- Owner-gated: no for the doc fix; yes if both sizes are to be judged

`judge_window` computes `per_unit` from `s2` alone, so across the acceptance ladder the constant legs and the declared heap/limb ceilings are judged at two of the four sizes; the board module doc states that "constants, declared-model bands, and liveness floors stay judged per size across the ladder", which is true for bands and floors and false for constants. Prose states what IS; judging at the larger size only may be deliberate (fixed overhead inflates per-byte constants at the smaller size), in which case the doc should say so.

Evidence:

       256	        let per_unit = match c {
       257	            Currency::Heap => {
       258	                m2.saturating_sub(HEAP_FLAT_ALLOWANCE_BYTES as u64) as f64 / s2.denom_bytes as f64
       259	            }
       260	            Currency::Segments => m2 as f64,
       261	            Currency::Limb => m2 as f64 / s2.limb_denom as f64,
       262	            Currency::Scan | Currency::Touch => m2 as f64 / s2.denom_bytes as f64,
       263	        };

    board.rs:
       222	//! super-linearity bends every point and still reads red; constants,
       223	//! declared-model bands, and liveness floors stay judged per size across the
       224	//! ladder. A bare single-scale run fits its own window's two points and

Resolution: Correct board.rs:222-224 to "constants at each window's larger size; bands and floors at every size" and state the reason the smaller size is excluded; or (owner's call) judge `per_unit` at both samples of each window. Acceptance: board.rs and judge.rs agree on which sizes carry the constant legs.
Construction: Through `evaluate`, `sample(n, limb = 10*128*n)` (over the 128/B ceiling) paired with `sample(2n, limb = 2n)` reads no "limb constant" red today, since only `s2` is judged.

### board-frame-2: The root doc states a two-point exponent formula the judge does not use and counts "all four counters" on a five-currency axis
- Where: crates/before/src/meter/board.rs:69-73 (related: crates/before/src/meter/board.rs:122-124, crates/before/src/meter/board.rs:43, crates/before/src/meter/board.rs:214-217, crates/before/src/meter/board/judge.rs:37-54, crates/before/src/meter/board/currency.rs:29-40)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read judge.rs:37-54, one log-log least-squares slope over all points; `git log -S'invisible to all four' -- board.rs` gives 54b68b4fd (2026-07-24); `git log -S'Touch,' -- currency.rs` gives 15c7a4acc (2026-07-26, "the touch currency joins the axis as field five"), which also wrote "five deterministic meters" at board.rs:43); executed: no
- Seen by: structure-prose, instrument-correctness (also the counts half of scaffolding's vocabulary item); refutation: confirmed (attribution of the "four counters" sentence corrected to 54b68b4fd); history: deliberate-but-expired (the formula is from 7d81a248a when the board fitted two points; 9e36dd280 replaced the estimator and added the exponent-policy section without touching :69; the count predates the fifth currency)
- Owner-gated: no

Two sentences on the module's front page contradict the code and the same doc's later sections (Principle 5: prose states what IS; no hand-maintained counts). Line 69 gives the exponent as the two-point ratio while `judge::trend` is a least-squares slope over every measured point, as the exponent-policy section at :214-217 says; line 123 says a machine-word quadratic is "invisible to all four counters" while the axis has five (currency.rs:29-40) and :43 of the same doc says five.

Evidence:

        69	//! Per meter the board derives a **scaling exponent** `log(m₂/m₁) / log(n₂/n₁)`
        70	//! (`n` = the cell's denominator bytes, below — every exponent, on every
        71	//! column) and a **per-denominator-byte constant** at the larger scale (the one

    [board.rs:122-124]
       122	//! space: a kernel doing quadratic work in plain machine-word arithmetic (no
       123	//! allocation, no recursion, no metered reads) is invisible to all four
       124	//! counters and visible only to a clock — but the clock lives where timing

    [judge.rs:37, 52-53]
        37	pub(super) fn trend(points: &[(usize, u64)]) -> f64 {
        52	    let sxy: f64 = xy.iter().map(|(x, y)| (x - mean_x) * (y - mean_y)).sum();
        53	    sxy / sxx

Resolution: At :69-73 state the estimator once ("the log-log least-squares slope of the counter against the denominator over every measured point; through two points that is the log ratio") and point at the exponent-policy section. At :123 write "invisible to every deterministic counter". Acceptance: the criterion section and the exponent-policy section name the same estimator; no numeral in board.rs restates the arity of `ByCurrency` (`grep -n -w four board.rs` shows only the ladder's four points).

### board-frame-4: Two sites prescribe a "dated owner rationale" at the declaring constants; no constant carries a date, and the doctrine forbids one there
- Where: crates/before/src/meter/board.rs:170-172 (related: crates/before/src/meter/board/ceilings.rs:233-235, crates/before/src/meter/board/ceilings.rs:56-62, crates/before/src/meter/board.rs:240-241)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n dated` over board.rs and board/*.rs hits :171, :240, ceilings.rs:234; `grep -E '20[0-9]{2}-[01][0-9]'` over the same files returns nothing; `git log -1 d2a9d04e` is "dated-notes excision: history lives in git, prose states what is" (2026-07-31)); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: deliberate-but-expired (669cf3103 introduced both the prescription and dated rationales; d2a9d04e excised the dates and left the two sentences)
- Owner-gated: no

The declared-models mechanism is described as committing "a dated owner rationale" at the declaring constant, at board.rs:170-172 and again in the ceilings.rs comment block at :233-235. The constants say "owner-ratified" without dates, and ceilings.rs:56-62 states the actual discipline (readings in pin commits). The prescription is a ghost of a retired convention, and the convention it prescribes is the one Principle 5 excises ("dated rationale at a declaration site"). board.rs:240's "dated envelope-only ruling on its registry row" is accurate (registry.rs carries `decided:` fields) and stays.

Evidence:

       170	//! Some cells are judged against a **declared model** — a ratified cost law
       171	//! derived at the cell, with a dated owner rationale committed at the declaring
       172	//! constant — in place of one global ceiling, because the global form is

    [ceilings.rs:233-235]
       233	// Some cells carry a *declared model* in place of one global ceiling: a
       234	// ratified cost law, derived and priced at the cell with a dated owner
       235	// rationale, that the readings must match — the global ceiling would otherwise

Resolution: "with an owner-ratified rationale at the declaring constant, its readings in the pin commit" at both sites. Acceptance: `grep -n dated crates/before/src/meter/board.rs crates/before/src/meter/board/ceilings.rs` returns only the registry-row sentence at board.rs:240.

### board-frame-5: The root doc re-narrates each submodule's mechanism, and the copies have drifted
- Where: crates/before/src/meter/board.rs:253-259 (related: crates/before/src/meter/board/export.rs:1-11, crates/before/src/meter/board.rs:152-166 with cell.rs:4-95, crates/before/src/meter/board.rs:168-187 with ceilings.rs:1-50 and ceilings.rs:231-242, crates/before/src/meter/board.rs:245-251 with coverage.rs:43-54 and coverage/tests.rs:30-37)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn -F 'wall-time mirror rides the same axes'` gives board.rs:253 and export.rs:3; the other triplications read side by side; the two drifts in board-frame-2 and board-frame-7 are copies that moved apart); executed: no
- Seen by: structure-prose; refutation: confirmed; history: deliberate-and-holds as a design (a05918df7: "one-paragraph summaries ... each pointing at the owning submodule", with the audit criterion that every parent line "survives verbatim or is one of the audited rewrites"), but the rationale lives only in the commit message and the verbatim-copy criterion is what drifted
- Owner-gated: yes (reshapes ~260 lines of public rustdoc under the `meter` feature)

The bench-mirror paragraph at board.rs:253-256 is verbatim export.rs:3-6; the tiling explanation appears in board.rs, coverage.rs, and coverage/tests.rs; the declared-models mechanism is derived in board.rs, the ceilings.rs module doc, and the ceilings.rs comment block; denomination is derived in board.rs and cell.rs. The design (a summary per submodule pointing at the owner) is recorded and sound, but the criterion that copies survive verbatim is what produced two independent drifts ("all four counters"; "the two n-ary fold rows" versus three versus four). This is an undocumented deliberate choice whose cost has materialized (Principle 5; documentation altitude: each mechanism derived in exactly one place).

Evidence:

       253	//! The wall-time mirror rides the same axes: the bench suite's criterion IDs
       254	//! are exactly the board's op × family cell names ([`bench_cells`] is the
       255	//! board's own table), so board coverage is bench coverage cell for cell, with
       256	//! no second enumeration (the `export` module derives the judged subset). Which

    [export.rs:3-6]
         3	//! The wall-time mirror rides the same axes: the bench suite's criterion IDs
         4	//! are exactly the board's op × family cell names ([`bench_cells`] is the
         5	//! board's own table), so board coverage is bench coverage cell for cell, with
         6	//! no second enumeration. Wall benching pays criterion's warmup and sampling

Resolution: State the map rule inline at the top of board.rs ("this doc orients; each mechanism is derived once, in the submodule named"), keep the criterion, the three-axis map, and the profile of record here, and replace each re-narrated derivation with a one-sentence pointer and rustdoc link to its owning submodule; where a sentence must appear twice, one copy is a link. Acceptance: no sentence of more than about ten words appears verbatim in two files of the board module (a `grep -F` of each root-doc sentence against the submodules finds only itself); denomination, declared models, tiling, and the bench mirror each have exactly one derivation site.

### board-frame-7: The fold-model roster is stated three ways: two rows (cell.rs), three rows (ceilings.rs), four rows carry `with_fold_arity`
- Where: crates/before/src/meter/board/ceilings.rs:9-10 (related: crates/before/src/meter/board/cell.rs:115-117, crates/before/src/meter/board/ops.rs:808-869, crates/before/src/meter/board/ops.rs:1361-1392)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n with_fold_arity ops.rs` gives :821, :846, :869, :1392, whose rows are `version_join_all` (:808), `version_meet_all` (:826), `version_span_all` (:851), `party_join_all` (:1361)); executed: no
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (a4cc1cf35 declared the model on two rows and wrote "two"; a5cb08156 added `version_meet_all`; 70eb67ab1 added `version_span_all` and updated neither list)
- Owner-gated: no

The declared-models section is the disclosure record of which cells read green under a model rather than the global bound, and it omits `version_span_all`; cell.rs's field doc counts two. Two independent restatements of one roster drifted in two different ways (Principle 5: no hand-maintained enumerations of callers).

Evidence:

         9	//! - **The fold rows** (`version_join_all`, `version_meet_all`,
        10	//!   `party_join_all`): the

    [cell.rs:115-117]
       115	    /// The fold rows' operand count at this scale: `Some` on the two n-ary fold
       116	    /// rows only, where it drives the declared fold scan model (the `ceilings`
       117	    /// module's declared-models section).

Resolution: In cell.rs drop the count ("`Some` on the n-ary fold rows, where it drives the declared fold scan model"). In ceilings.rs either add `version_span_all` or replace the enumeration with "the n-ary fold rows (every row built with `with_fold_arity`)" so the code is the roster. Acceptance: neither file counts or enumerates the fold rows, or the ceilings.rs list matches `grep -n with_fold_arity ops.rs`.

### board-frame-9: `MAX_HEAP_BYTES_PER_INPUT_BYTE` carries no derivation
- Where: crates/before/src/meter/board/ceilings.rs:71-73 (related: crates/before/src/meter/board/ceilings.rs:56-62, crates/before/src/meter/board/ceilings.rs:85-94)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (`git log -S'MAX_HEAP_BYTES_PER_INPUT_BYTE: f64 = 16'` returns only the board's introduction and the module split; the file's other global ceilings each argue a calibration or mechanism); executed: no
- Seen by: adequacy; refutation: confirmed; history: no-rationale-found (7d81a248a states "heap 16 B/B over an 8 KiB flat allowance" with no derivation while deriving limb in the same sentence)
- Owner-gated: no

Every other global ceiling in the file states its calibrating reader and margin or its mechanism; the heap constant states only what it requires. Under the file's own convention (ceilings.rs:56-62: derive at the constant, readings in the pin commit) an underived ceiling cannot be re-derived when its worst reader moves, and a reader cannot tell whether 16 is slack over a measured reader or an operationalization of the crate docs' auxiliary-space promise.

Evidence:

        71	/// Green requires peak transient heap at most this many bytes per packed input
        72	/// byte, over the flat allowance.
        73	pub const MAX_HEAP_BYTES_PER_INPUT_BYTE: f64 = 16.0;

Resolution: State the derivation: the calibrating reader at the release profile and the margin convention, or that the constant operationalizes the crate-level "small constant multiple" promise at a stated multiple, with the reading left to the pin commit. Acceptance: the constant's doc names its calibrating reader or its derivation from the crate-level promise.

### board-frame-10: `TICKS_BOARD_COUNT`'s one-line proof names the wrong leg
- Where: crates/before/src/meter/board/ceilings.rs:151-158 (related: crates/before/src/meter/board/ops.rs:409-432, crates/before/src/meter/board/judge.rs:256-263)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read ops.rs:409-432: the row is input-denominated at `n` and runs `v.ticks(&party, TICKS_BOARD_COUNT)`; an implementation iterating c single ticks does c·O(n) work, leaving the exponent in n unchanged and multiplying every per-byte constant by c); executed: no
- Seen by: adequacy; refutation: confirmed; history: no-rationale-found (from 56a08f907, reflowed unchanged)
- Owner-gated: no

A count fixed across scales multiplies per-byte work by a constant; what fires is the scan, limb, and touch constant legs (512 × ~8 scan bits per byte is far over the 96 ceiling), not the scaling exponent. Every pinned constant's doc is its one-line proof; naming the exponent leg misdirects the reader deciding which leg to strengthen.

Evidence:

       153	/// Fixed so the cell's judged axis is the packed input alone: the count's whole
       154	/// contribution is the boundary codes' gamma width (the flatness rows of
       155	/// `tests/meter.rs` pin that axis point to point), and 512 sits far enough past
       156	/// the single tick that an implementation iterating even a fraction of the
       157	/// count would blow the scaling ceiling rather than hide in a constant.
       158	pub const TICKS_BOARD_COUNT: u64 = 512;

Resolution: "...an implementation iterating even a fraction of the count multiplies every per-byte constant by that fraction of 512, far over the scan, limb, and touch ceilings, so it cannot hide in headroom." Acceptance: the sentence names the constant ceilings as the leg that fires.

### board-frame-11: Escaped-bracket citation tags (`\[derived\]`, `\[the ... in the test suite\]`) are an undefined convention that names no test
- Where: crates/before/src/meter/board/ceilings.rs:172-173 (related: crates/before/src/meter/board/ceilings.rs:182-183, crates/before/src/meter/board/ceilings.rs:207, crates/before/src/meter/board/ceilings.rs:227-228, crates/before/src/meter/board/cell.rs:20, crates/before/src/meter/board/cell.rs:28-29)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (`grep -n -F '\['` over the partition gives exactly these six sites plus board/tests.rs:490 outside it; no definition of the convention exists in the crate); executed: no
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (the tags are a design note's evidence vocabulary, `[derived]` paired with `[measured]`; 500d4d094 excised every `\[measured` tag from ceilings.rs and left `\[derived`, so the tag no longer distinguishes anything, and its only definition is in a note code may not cite)
- Owner-gated: no

Six sites carry bracketed tags rendered literally. `\[derived\]` tells the reader nothing the surrounding derivation does not; the test citations ("the chunked tripwire in the test suite", "the schoolbook and delegating-parser pins") name no test function, so a rename cannot break them and a reader cannot grep to them (Principle 5: no opaque roster markup; every coined marker defined once).

Evidence:

       172	/// quadratic exponent against `n_io` \[the chunked tripwire in the test
       173	/// suite pins both halves\]; the exponent leg is what excludes it. What κ

    [ceilings.rs:207]
       207	/// denominator, read green. The ceiling closes it \[derived\], and its basis is

    [cell.rs:20]
        20	//!   there \[the committed chunked tripwire\].

Resolution: Delete the `\[derived\]` tags; replace each test citation with the test's function name in backticks (the chunked-schoolbook, schoolbook, delegating-parser, and sub-scaling tests in board/tests.rs). Acceptance: `grep -rn -F '\[' crates/before/src/meter/board/` returns nothing; every test cited in ceilings.rs and cell.rs prose is a function `grep -n 'fn <name>'` finds.

### board-frame-17: The heterogeneous `clock | version` joins cite `clock_hash`, a row that prices a byte compare
- Where: crates/before/src/meter/board/coverage.rs:218-221 (related: crates/before/src/meter/board/coverage.rs:6-7, crates/before/src/meter/board/coverage.rs:114, crates/before/src/meter/board/coverage.rs:217, crates/before/src/clock.rs:1013-1046, crates/before/src/meter/board/coverage/tests.rs:53-55)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (every `clock_join_matrix!` arm runs `self.version |= r.borrow()` or `r.version |= self.borrow()` (clock.rs:1016, :1030, :1043); coverage.rs:6-7 says these operators fold through the recv row's join-assign; :114 cites `version_join` for `Clock::absorb`, the named equivalent; :217 already prices `Clock Eq / Hash` with `clock_hash`; coverage/tests.rs:53-55 asserts only that a cited row exists); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (669cf3103 attached `clock_hash` to this claim to satisfy the new no-orphan leg when Clock had no Eq/Hash surface row; 551f4df15 added that row and priced it with `clock_hash`, leaving this citation redundant)
- Owner-gated: no

The priced table is the durable record of which mechanism prices which surface row, and the tiling test verifies existence only, so a wrong citation is the cheapest passing artifact (Principle 6). No hashing occurs in these impls; the `Span::contains` entry at :151-153 shows the right form when a second row is cited deliberately (an inline reason).

Evidence:

       218	    (
       219	        "Clock | Version and Version | Clock (heterogeneous joins, |=)",
       220	        &["clock_recv", "clock_hash"],
       221	    ),

    [clock.rs:1041-1044]
      1041	        impl BitOrAssign<$rhs> for $lhs {
      1042	            fn bitor_assign(&mut self, r: $rhs) {
      1043	                self.version |= r.borrow();
      1044	            }

    [coverage/tests.rs:53-55]
        53	        for row in *rows {
        54	            assert!(ops.contains(*row), "{op}: cites unknown board row {row}");
        55	        }

Resolution: Replace `clock_hash` with `version_join_assign` (the `|=` the impls run) or `version_join` as the `Clock::absorb` entry does; `clock_recv` may stay as the module doc's stated mechanism (recv is absorb plus tick). Acceptance: the entry cites only rows whose mechanism the impls execute; `board_coverage_tiles_the_public_surface` stays green.
Construction: Change `clock_hash` here to any other live row name (e.g. `rank_decode`) and run the coverage tests: the tiling test still passes, showing it cannot distinguish a right citation from a wrong one.

### board-ops-render-1: The row table's module doc states an absolute the declared-model attachments break
- Where: crates/before/src/meter/board/ops.rs:3-5 (related: ops.rs:391, 427, 800, 902, 1047, 1586, 1710, 1770; board.rs:25-29)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'f\.kind' ops.rs` returns exactly the eight sites); executed: no
- Seen by: scaffolding [1], adequacy [23], instrument-correctness [55]; refutation: reframed (the consults attach judgment models after the cell is prepared and never gate applicability, so the product argument holds; the defect is the sentence); history: deliberate-but-expired (true at 3eadcb107, falsified by 669cf3103 two days later without amendment)
- Owner-gated: no

The module doc's first sentence promises that a row prepares its cell "never from the shape's identity", but eight rows match on `f.kind` to attach owner-declared per-cell models (`with_declared_heap`, `with_capacity_model`, `with_declared_limb`). The attachments do not change which shapes a row reaches, so the product argument the sentence defends still holds; the sentence is what is wrong (Principle 5: prose states what is).

Evidence:

         3	//! Each row declares the bundle slots its signature consumes and prepares its
         4	//! cell from them alone — never from the shape's identity — so a row reaches
         5	//! every shape that supplies its operands.

       391	                    return Some(if matches!(f.kind, FamilyId::AscendCliff) {
       392	                        cell.with_declared_heap(ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE)

Resolution: Name the exception where the rule is stated: "...never from the shape's identity, except to attach an owner-declared per-cell judgment model (the `ceilings` module's declared-models section), which by construction is a statement about one named cell and never about reach." Apply the same amendment to board.rs:25-29. Relocating the declarations onto the bundle (a `declared` slot the family builder fills, as `output_dominated` is) is an optional design proposal; the declared constants differ per operation (the tick trio versus `version_min_ticks`), so a per-family slot is not obviously cleaner. Acceptance: the two docs and the code agree; a reader of ops.rs:3-5 is told where identity is consulted and why.

### board-ops-render-3: A dated rationale justifies the magnitude shapes' `designed` arm
- Where: crates/before/src/meter/board/ops.rs:113-115 (related: none)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); executed: no
- Seen by: structure-prose [32]; refutation: confirmed; history: contradicts-hard-rule (Part II under Principle 5: "Dated rationale at a declaration site is the same failure in disguise"; the comment is from 3eadcb107 and survived d2a9d04e's dated-notes sweep because it carries no calendar date)
- Owner-gated: no

The arm's comment explains the arm by chronology (which family came first) rather than by the present-tense fact it encodes (which groups those shapes stress).

Evidence:

       113	        // The magnitude shapes predate the rank rows' mismatch pair and
       114	        // were never its designed adversary.
       115	        FamilyId::Bigroot | FamilyId::Hugeleaf | FamilyId::Cliff => group != OpGroup::Rank,

Resolution: State the invariant positively, for example "The magnitude shapes stress every group but Rank: the rank rows' mismatch pair is built from the spine families, so these shapes are not its adversary." Acceptance: no "predate" or "were never" in the arm's comment.

### board-ops-render-17: shard.rs `# Panics` sections are incomplete, the rustdoc allow is module-wide, the merge output is a positional four-tuple, and the bit-pattern parse repeats where a helper exists
- Where: crates/before/src/meter/board/shard.rs:136-138 (related: shard.rs:65-67, 196-201, 231, 235, 279-281, 322-329, 385, 395-399, 404, 532-535, 556, 567, 584; render.rs:165-170, 193; measure.rs:115)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the cited lines: `emit_shard`'s `# Panics` names index and scale only while `assert_unframed` (196-201, called at 231 and 235), `measure_cell`'s applicability expect (render.rs:193), and `assert_honest_text` in `measure` (measure.rs:115) are also reachable; `assert_scale` requires finiteness, which the "strictly positive" wording at 138, 398, and 534 omits; the `#![allow(rustdoc::private_intra_doc_links)]` at 67 is module-wide although only `run_acceptance`'s doc (556, 567) links private items; `merge_samples` returns `Vec<(&'static str, &'static str, Sample, Sample)>` destructured positionally at 385 and 584; `heap_model` and `declared_heap` parsing (322-329) repeat `(text != "-").then(|| from_bits(text, line))` beside the `opt_number` helper at 279-281); executed: no
- Seen by: structure-prose [44]; refutation: confirmed, adding the finiteness omission; history: no-rationale-found
- Owner-gated: no

`emit_shard`, `run`, and `merge_samples` are public and their uniform `# Panics` sections must be complete: each says "strictly positive" where the guard also requires finite (an infinite scale is strictly positive and panics), and `emit_shard` omits the framing-byte assertion on a rationale containing a tab or newline and the panics reachable through `measure_cell`. The lint allow at 67 keeps the detector off for the whole module although one item needs it. A named record beats a four-tuple read by position, and an `opt_bits` helper would parallel `opt_number`.

Evidence:

       136	/// # Panics
       137	///
       138	/// Panics unless `index < count` and `scale` is strictly positive.

       196	fn assert_unframed(text: &str) {
       197	    assert!(
       198	        !text.contains('\t') && !text.contains('\n'),

Resolution: Write "a strictly positive finite number" in the three `# Panics` sections; add the unframed-rationale clause to `emit_shard` (or move `assert_unframed` to a unit test over the floors constants, since every rationale is a `&'static str` constant); move the allow onto `run_acceptance` as an outer attribute; introduce `struct MeasuredCell { op, family, s1, s2 }`; add `fn opt_bits(text, line) -> Option<f64>`. Acceptance: each public `# Panics` lists every panic path; the file has no inner `#![allow]`; no four-tuple destructuring of merge output.

### board-ops-render-24: tests.rs prose carries a partial hand-maintained inventory and two counts its bodies contradict
- Where: crates/before/src/meter/board/tests.rs:1-12 (related: tests.rs:104-105 versus 109-114; tests.rs:860-861 versus 910, 921, 932, 953, 964; measure.rs:114-116)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the whole file: the header names the κ tripwires, the `n_io` exponent tripwire, the floors tripwire, and the join_all pin, and omits the exponent-guard, acceptance-trend, fold-model, capacity-model, bench-rider, and worst-map tests at 650-1284; counted five `evaluate` calls in the fold-model test against "Three probes"; counted four hand-picked shapes against "Every family's"); executed: no
- Seen by: structure-prose [35], structure-prose [36]; refutation: confirmed; history: "Three probes" was accurate at a4cc1cf35 and 29d3c8f2 added the two search probes the same day without amending it; the header was already partial when 89cf4c8d created the file
- Owner-gated: no

Every test's doc comment must state its invariant accurately and no prose may hand-maintain a count or an inventory the code can change without touching it (Principle 5). The module doc enumerates roughly half the file's tests; `declared_fold_model_admits_the_log_factor_and_rejects_quadratic` says three probes go through `evaluate` where five do; `rendered_text_is_honest_and_padding_trips` says "Every family's rendered text" where the body checks four shapes (the every-family property is enforced per board text cell at measure.rs:114-116, not here).

Evidence:

       860	/// Three probes through [`evaluate`], all at the benign control's committed
       861	/// arity pair (k 256 -> 512 over a x2.19 denominator): the pre-declaration

       104	/// Every family's rendered text sits under the output-honesty ceiling, and the
       105	/// ceiling is tight enough that a text stream padded past

Resolution: Rewrite the header as the file's structure, not a tally: derivation pins (radix units, mandatory limbs, output honesty), known-bad probes through `evaluate`/`evaluate_acceptance` that must read red on exactly one leg, pins that tie tables to live axes (bench riders, the ranking pin), and the argmax kernel and near-tie rendering tests. Rewrite 860 to name the probes without a numeral; rewrite 104 as "Four representative shapes render under the ceiling", pointing at the per-cell assertion. Acceptance: no numeral count of probes in a test doc; the header names genres, not tests; adding a probe test requires no header edit.

### board-ops-render-30: The acceptance criterion's doc and two kernel docs cite a retired determinism tripwire
- Where: crates/before/src/meter/board/ceilings.rs:460-462 (related: version/skyline/fill.rs:150; version/skyline/fill/tests.rs:14; shard.rs:570-612; board.rs:115-120)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'determinism tripwire' src` returns exactly ceilings.rs:462, fill.rs:150, fill/tests.rs:14; `git show -s 289e14a4`: "board: retire the determinism scaffolding and the serial reference path"; `run_acceptance` (shard.rs:570-612) read in full performs no determinism check); executed: no
- Seen by: instrument-correctness [49]; refutation: confirmed; history: contradicts-hard-rule (root AGENTS.md: nothing refers to code that no longer exists; the tripwire landed at c15daaab4/67b031aa9 and 289e14a45 retired both halves without sweeping these three sites)
- Owner-gated: no

Note: the anchors live outside this partition's five files; the claim concerns `run_acceptance`, which is in shard.rs. Deduplicate against the ceilings and skyline partitions at merge.

`LADDER_TOP_SCALE`'s doc states that campaign acceptance runs "under the determinism tripwire"; commit 289e14a4 removed both halves of that tripwire (the in-process double measurement and the gate's cross-process compare), and `run_acceptance` performs no determinism check. fill.rs and fill/tests.rs cite "the board's determinism tripwire" as a large-operand coverage instrument in the same way. board.rs:115-120 already states the determinism property as a design fact without the tripwire.

Evidence:

    ceilings.rs:
       460	/// tops out there. **Campaign acceptance is every cell green across the whole
       461	/// ladder, one acceptance invocation measuring all of it under the
       462	/// determinism tripwire**, with each exponent judged as one trend over the

    fill.rs:
       150	//! board's determinism tripwire.

Resolution: In ceilings.rs drop "under the determinism tripwire" (if the owner wants the property re-instrumented, the smoke suite's cross-shard byte-identity test is the live witness to cite). In fill.rs:150 and fill/tests.rs:14 re-denominate the coverage sentence onto what exists: the board's acceptance ladder and the envelope suite's pinned scales. Acceptance: `grep -rn 'determinism tripwire' crates/before/src` returns nothing; `just doclint` and `just citecheck` stay clean.

**Nits (7), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| board-families-floors-judge-9 | `crates/before/src/meter/board/floors.rs:23-26` | Three floors.rs sentences disagree with the code or number they describe | (a) State the structure ("not-applicable where the contract forces no metered stream work ... | `evidence/partitions/board-families-floors-judge.md` |
| board-frame-12 | `crates/before/src/meter/board/ceilings.rs:223-224` | Two measured readings are quoted in ceilings.rs prose against the file's own rule | Keep the mechanism (the pair's denominator barely moves; the log factor's marginal) and excise the numbers or move them to the pin commits | `evidence/partitions/board-frame.md` |
| board-frame-14 | `crates/before/src/meter/board/cell.rs:45-48` | "The tripwire pair below" points at tests that live in another file | Name the two tests in backticks | `evidence/partitions/board-frame.md` |
| board-frame-18 | `crates/before/src/meter/board/coverage.rs:435` | Vocabulary tells across the frame's prose: "mint", an unanchored "honest" in four senses, register transplants, "genre", "today", a past-tense justification, a duplicated sentence | Schedule a crate-wide prose pass. In it: "construct"/"build" for mint; keep "output honesty" anchored to `assert_honest_text` and replace the other se ... | `evidence/partitions/board-frame.md` |
| board-ops-render-5 | `crates/before/src/meter/board/ops.rs:327-327` | Em-dashes in plain `//` comments at twenty sites | Replace with colons, semicolons, or spaced `--` in those twenty lines | `evidence/partitions/board-ops-render.md` |
| board-ops-render-8 | `crates/before/src/meter/board/ops.rs:701-702` | Moralized and overloaded vocabulary: unanchored "honest", two meanings of "diagonal", three referents for "seam", and "mints" | Replace unanchored "honest" with the mechanism ("the input-byte denominator, which the output-honesty assertion makes the smaller of the two" ... | `evidence/partitions/board-ops-render.md` |
| board-ops-render-11 | `crates/before/src/meter/board/render.rs:26-29` | `Summary.red`'s doc calls every red an amplification finding | "Cells with at least one red leg: an exponent or constant over its bound, a reading outside a declared model's band ... | `evidence/partitions/board-ops-render.md` |

**Cross-references.** board-frame-4 and prose-hygiene-6 are the same "dated owner rationale" prescriptions at board.rs:170-172 and ceilings.rs:233-235 (prose-hygiene-6 adds the fuzzfit re-pin template). board-ops-render-29 and prose-hygiene-3 are the same design-doc citation at board/tests.rs:505. board-families-floors-judge-8 and prose-hygiene-9 are the six "today" strings in floors.rs; prose-hygiene-10's per-file "honest" counts start with the board. meter-adequacy-3's `MAX_SCALING_EXPONENT` doc sits beside board-frame-8 (another class), the log-factor admission. board-frame-2, board-frame-7, and board-ops-render-24 are the frame's hand counts. board-ops-render-30's ghost tripwire reaches into fill.rs (skyline-fill-grow).

## The instruments: the surface roster (surface.rs, surface_coverage, surfacecheck, surface-scan)

7 findings (1 high, 0 medium, 3 low, 3 nit). Full records: `evidence/partitions/surface-roster.md`, `evidence/sweeps/clippy-pedantic.md`.

### surface-roster-20: Ghost and temporal references: a deleted API name in a doc example, a pointer to a note that does not exist, a scan that no longer exists, and "none at this tip"
- Where: crates/before/surfacecheck/src/extract.rs:7-8 (related: crates/before/src/testing/surface_coverage.rs:164-166, crates/before/src/testing/surface_coverage/tests.rs:133-134, crates/before/surfacecheck/src/check.rs:37, crates/before/src/causally/forms.rs:172)
- Class / severity / confidence: documentation / high / high
- Provenance: verified (`grep -rn 'causally::Range\|struct Range\|enum Range' crates/before/src crates/before/surfacecheck/src` hits only extract.rs:8; db9dfa3e "before: replace causally::Range with the polar Query algebra" touched census.rs and auto_traits.rs and not extract.rs; `pub fn since` is a free function at causally/forms.rs:172 and the roster row is `causally::since`; `grep -rn 'coverage note' crates/before/src` hits only meter/board/coverage.rs:21, which concerns error-path dispositions, and tests.rs:70-94 carries no such note; `grep -rn 'bare-name scan'` hits only tests.rs:134); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: contradicts-hard-rule for the Range example (live when written, a ghost since db9dfa3e); the coverage-note pointer has pointed at nothing since 67956d1f; "old bare-name scan" was a ghost from birth (4b136e2b); "none at this tip" survived the dated-notes sweep because d2a9d04e did not touch check.rs
- Owner-gated: no

The root AGENTS.md hard rule: "Nothing in the codebase refers to code that no longer exists — no 'formerly', 'superseded', 'was removed', no deleted API names in any prose." `causally::Range::since` names a type the crate no longer has, in the sentence meant to teach the naming scheme. The other three sites are Principle 5 defects of the same family: `SURFACE_SOURCES`'s doc tells the maintainer to update "the roster test's coverage note", which does not exist; the haystack test's doc explains its probes by "the old bare-name scan"; `ITEM_EXCEPTIONS`'s doc dates its emptiness to the present. The severity follows the rubric for a hard-rule breach; each fix is a phrase.

Evidence:

         7	//! which is the roster's own naming (`Party::seed` for a root re-export,
         8	//! `causally::Range::since` inside a public module). The `paths` table's

    surface_coverage.rs:
       164	/// The public-API source files of record. A new public module with
       165	/// inherent methods must be added here (and the roster test's coverage
       166	/// note updated), which is itself a reviewed diff.

    surface_coverage/tests.rs:
       133	/// Two directions. Negative: named non-test helpers — declared `fn`s the
       134	/// old bare-name scan accepted — are absent from

    check.rs:
        37	/// Per-item exceptions: none at this tip. An entry here is a deliberate,

Resolution: extract.rs:8: `causally::since` for a public-module free function (and `causally::Query::contains` for a method on a type inside a public module). surface_coverage.rs:165-166: delete the parenthetical. tests.rs:133-134: "named non-test helpers declared under `src/` are absent from ...". check.rs:37: state the rule without the temporal clause, or dissolve with surface-roster-18. Acceptance: `grep -rn 'causally::Range' crates` is empty; `grep -rn 'coverage note\|bare-name scan' crates/before/src/testing` is empty; `grep -n 'at this tip' crates/before/surfacecheck` is empty.

### surface-roster-2: The roster's module doc states the meter-feature fact three times, the surface-totality paragraph recurs at four sites, and two counts are hand-maintained
- Where: crates/before/src/surface.rs:1-22 (related: crates/before/src/testing/surface_coverage.rs:22-30, crates/before/src/testing/surface_coverage.rs:62-64, crates/before/surfacecheck/src/main.rs:5-22, crates/before/surfacecheck/src/check.rs:11-13, justfile:906-918)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read all sites; `Exclusion::FAMILIES` has seven entries at surface.rs:157-165; `ANCHORS` has two at check.rs:96); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (surface.rs:3-5 was prepended in ebe66966 beside the original 16-17; d60b7570 rewrote the totality explanation in parallel at four sites, which is the drift vector that produced surface-roster-7)
- Owner-gated: no

Lines 3-5, 11-15, and 16-17 each say the module is public under `meter` so instrument crates bind to the same roster. The surface-totality mechanism is described in full at surface.rs:1010-1015, surface_coverage.rs:22-30, main.rs:5-22, and justfile:906-918; two of the four copies carry the family-row overclaim (surface-roster-7). surface_coverage.rs:63 counts "seven families" by hand (restating `Exclusion::FAMILIES.len()`), and check.rs:12-13 says "two known-public items must be present by name" (restating `ANCHORS.len()`). Documentation altitude: every sentence competes with the contract the reader came for; Principle 5: no hand-maintained counts.

Evidence:

         3	//! Public under the `meter` feature (with the other instrument-facing data) so
         4	//! external instrument crates can bind their coverage tables to the same
         5	//! roster.
    ...
        16	//! Public under the `meter` feature (the instrument crates' feature) and never
        17	//! part of a production build.

    surface_coverage.rs:
        62	//! An excluded leg carries a variant of the typed [`crate::surface::Exclusion`]
        63	//! vocabulary — seven families, each variant's documentation defending its

    check.rs:
        12	//! that returns nothing (or the wrong tree) cannot pass, because two
        13	//! known-public items must be present by name; the censuses need no

Resolution: keep one meter sentence in surface.rs (the 11-15 sentence carries the argument), delete 3-5 and 16-17; let surface_coverage.rs own the enforcement explanation and main.rs own the gate's, with the other sites pointing by name; replace "seven families" with "the families in [`Exclusion::FAMILIES`]" and "two known-public items" with "the anchors". Acceptance: the meter sentence appears once in surface.rs; the totality mechanism is described in full at one site and referenced by name elsewhere; no numeral restates an enumerable list in the partition's docs.

### surface-roster-15: `d1_seeds_stay_committed` carries an opaque prefix defined nowhere
- Where: crates/before/src/testing/surface_coverage/tests.rs:355-359 (related: crates/before/src/testing/surface_coverage.rs:77, crates/before/src/testing/surface_coverage.rs:115)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '\bd1_\|\bD1\b' crates/before/src` hits only the test name and its two prose mentions); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (the name arrived in 67956d1f with no definition of D1 in that diff or any note)
- Owner-gated: no

The test and its two mentions use `d1` for the prod↔tree leg; every other site names the leg in words, and `D1` is defined nowhere in the crate. Doctrine under Principle 5: no opaque roster IDs in code or prose; a tag that outlives its roster is self-invented jargon.

Evidence:

       355	/// The prod↔tree adequacy seeds stay committed: the two fold-mutation
       356	/// witnesses replay through the `join_all` differentials from these
       357	/// files on every run, and this pin makes stripping them a red diff.
       358	#[test]
       359	fn d1_seeds_stay_committed() {

Resolution: rename to `fold_seeds_stay_committed` and update surface_coverage.rs:77 and 115. Acceptance: `grep -rn 'd1_' crates/before/src` returns nothing; the coverage tests pass.

### surface-roster-16: The validation index promises every guarding instrument and omits surfacecheck, citecheck, and the two hidden-surface pins
- Where: crates/before/src/testing/validation_index.rs:1-32 (related: crates/before/surfacecheck/src/main.rs:1-28, tools/citecheck:1-34, crates/before/tests/doc_hidden.rs:1-11, crates/before/tests/foreign_reexport.rs:1-21)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i 'surfacecheck\|rustdoc JSON\|citecheck\|doc_hidden\|foreign_reexport\|totality'` on the 182-line index returns nothing; the index does cover external instruments, e.g. before-fuelscape at 132); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no-rationale-found (the index predates all four instruments and was edited eight times afterwards without gaining entries)
- Owner-gated: no

The index describes itself as "every instrument that guards this crate, what failure class each one catches that the others cannot" and gives the roster one entry, but has no entry for surfacecheck (items in unlisted files, feature-gated trees, trait impls), citecheck (cited tests the runner never collects), tests/doc_hidden.rs (hidden items rustdoc JSON omits), or tests/foreign_reexport.rs (a dependency's surface published through `pub use`). A maintainer orienting cold would not learn that the roster's totality has a second, stronger enforcement or that citation liveness is judged outside the crate.

Evidence:

         1	//! The validation index: every instrument that guards this crate, what
         2	//! failure class each one catches that the others cannot, and where it
         3	//! lives.

Resolution: one entry each, stating what it alone catches as a constructible input. If the line scan is retired (surface-roster-9), restate the roster entry's "held equal, name for name, to the `pub fn` surface extracted from source" against rustdoc JSON. Acceptance: `grep -n 'surfacecheck\|citecheck\|doc_hidden\|foreign_reexport' crates/before/src/testing/validation_index.rs` returns one entry each.

**Nits (3), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| surface-roster-25 | `crates/before/surfacecheck/src/main.rs:19-20` | Metaphors promoted to jargon without an anchor: "jaw"/"pincer", "honest reading", "earns", "the seal", "keystone", "a real roster row" | rewrite as mechanism: "the census that pins each impl `FAMILY_SURFACE` disposes"; "two public spellings are two rows of surface" (drop "honest") ... | `evidence/partitions/surface-roster.md` |
| surface-roster-27 | `crates/before/surfacecheck/src/main.rs:118-133` | The clean-sweep census line mixes item counts with exception-entry counts, so its arithmetic identity does not hold | report the parenthetical as item counts (items under item exceptions, items under module exceptions ... | `evidence/partitions/surface-roster.md` |
| clippy-pedantic-6 | `crates/before/src/surface.rs:319-321` | Six items whose first doc paragraph runs three to four lines, so the module listing shows a block instead of a name | insert a blank `///` line after the first sentence at each site; | `evidence/sweeps/clippy-pedantic.md` |

**Cross-references.** surface-roster-16, fuzz-guests-pins-24, tests-other-1, and testing-oracles-29 are the validation index's totality claim from four partitions; module-graph-11 adds that the page never renders. surface-roster-25 and tests-other-12 are the pincer/jaw metaphor across surfacecheck and the roster pins. surface-roster-15's `d1_` prefix is the crate's other opaque roster id (with PROG-5/COV-7). surface-roster-2's totality story at four sites is what produced surface-roster-7 (another class). clippy-pedantic-6's long first paragraphs anchor at surface.rs:319-321.

## The instruments: the test harness (testing/: bridge, semantic oracle, exhaustive, algebraic laws, validation index, diff_ops, generators, compactness, asymptotics, fuelscape islands)

19 findings (0 high, 2 medium, 11 low, 6 nit). Full records: `evidence/partitions/testing-diff-gen.md`, `evidence/partitions/testing-oracles.md`, `evidence/sweeps/module-graph.md`, `evidence/sweeps/recursion.md`, `evidence/sweeps/suite-economics.md`.

### testing-diff-gen-17: `compactness.rs` speaks from before the flag day: "today's"/"decision-era"/"adoption turns on", a `current_bits` doc that names the wrong quantity, and measurement-sweep readings in prose
- Where: crates/before/src/testing/compactness.rs:1-9 (related: crates/before/src/testing/compactness.rs:31-57, crates/before/src/testing/compactness.rs:59-68, crates/before/src/testing/compactness.rs:78-84, crates/before/src/testing/compactness.rs:91, crates/before/src/testing/compactness/tests.rs:1-10, crates/before/src/testing/compactness/tests.rs:30-32, 41-42, 58-59, 70-72, crates/before/src/meter/tier2.rs:1-19)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git log --date=short -- crates/before/src/testing/compactness.rs`: f2d0011b 2026-07-23 lands the suite, faf3cd0a 2026-07-25 is the flag day; `current_bits = packed.len()` where `packed = packed_bits_of(&to_oracle_version(version))` at 81-84 and `bridge::packed_bits_of` (bridge.rs:62-68) emits the min-lifted stream; tier2/tests.rs:145 asserts "the stored coding is Tier 2 itself"; tests.rs:44 runs `world_strategy_up_to(120)` against prose "trace lengths up to 400"; `arb_comb_params` tops out at 200/256 against prose "2048-bit teeth and 1024 pairs"; the one committed tightness point is `comb(1024, 1024)` at tests.rs:93); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: already-known for the dissolution branch (the compactness probes are a recorded defended keep, `.agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:1756-1760`, reaffirmed at faf3cd0a, so that branch is dropped); contradicts-hard-rule for the prose (root AGENTS.md: nothing refers to code that no longer exists; CLAUDE.md Principle 5: dated measurement reports are re-denominated or excised when the code they cite is gone; the d2a9d04e sweep removed calendar dates only)
- Owner-gated: no (a prose correction toward the code; the keep is already ruled, and where the 2x claim should live is an open question below)

The skyline is the stored coding. What the instrument measures today is the stored skyline stream's bit length against the min-lifted packed preorder stream re-derived through the oracle lowering, a fixed reference; the measurement is right and its labels are wrong. `Sample.current_bits` is documented as "Today's live encoded bit length" and is the reference stream's length. The module frames itself as pre-decision evidence, the two `f64` constants carry hand-counted sweep provenance (sample counts, family maxima) that the committed tests do not reproduce, and compactness/tests.rs opens with a "Ratio record of the measurement sweep" quoting 1.9966, 1.9633, 1.50, 30-40%, 0.75, 0.7502, of which only the 1024×1024 comb figures are asserted (tests.rs:94-97). The sibling asymptotics module had exactly this class of reading excised at 4797009a under the rule that a reading not pinned by an adjacent assertion is falsified silently.

Evidence:

         3	//! The Tier 2 coding stores preorder topology plus delta-coded absolute leaf
         4	//! values ([`crate::meter::tier2`]); the claim its adoption turns on is that
         5	//! its coded size never exceeds ~2x today's size plus O(1) bits per node.
       ...
        34	/// Provenance: measured (~13k samples: 5000 arbitrary trees,
        35	/// ~7800 organic-history versions at trace lengths up to 400, the
       ...
        64	    /// Today's live encoded bit length of the same version.
        65	    pub(crate) current_bits: u64,
       ...
        78	    // The decision-era "current" coding is the min-lifted packed preorder
        79	    // stream (one gamma-coded base per node), re-derived through the
        80	    // oracle lowering; the stored coding is Tier 2 itself.
       ...
        91	         (ratio {ratio:.4}): decision-critical, pin this witness: {tier2:?}",

    compactness/tests.rs:
         4	//! Ratio record of the measurement sweep (thousands of samples
         5	//! per random family; the deterministic grids in full): the global maximum
         6	//! ratio is 1.9966 at the alternating comb with 2048-bit teeth and 1024

Resolution: Rewrite the module doc, both constant docs, `Sample`, `check_sample`'s comment and messages, and compactness/tests.rs:1-10 in the present tense: the skyline coding's bit length is at most twice the min-lifted packed preorder reference (plus 0 bits per node) on every family; the reference is the oracle lowering's packed stream; the comb is the tightness witness. Rename `current_bits` to `reference_bits`. Keep the derivation (each stored base charged at most twice, O(1) bits per gamma merge) and the committed tightness test; move the sweep record and family maxima out of prose (they live in f2d0011b and the design note). The note's own present-tense sentence ("skyline ≤ 2× the packed-era coding outright on every sample", §11) is the ready replacement. Acceptance: no occurrence of "today", "current coding", "adoption", "decision-critical", "decision-era", "Provenance: measured", or "Ratio record" in compactness.rs or compactness/tests.rs; every figure remaining in prose is one a committed assertion pins.

### testing-diff-gen-22: The asymptotics pins cite rustdoc sentences that no longer exist; the claims of record are the fuelscape roster's `contract` strings, and the Ω floor is not a public claim
- Where: crates/before/src/testing/asymptotics.rs:1-28 (related: crates/before/src/testing/asymptotics.rs:42-48, crates/before/src/testing/asymptotics.rs:65-68, crates/before/src/testing/asymptotics.rs:234-237, crates/before/src/testing/asymptotics.rs:383-384, crates/before/src/version.rs:289-293, crates/before/src/version.rs:471-475, crates/before/src/version.rs:1410-1412, crates/before-fuelscape/src/ops.rs:331, crates/before-fuelscape/src/ops.rs:360, crates/before-fuelscape/src/ops.rs:630, crates/before/build.rs:201-236)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn 'summary-merge\|grows faster than the operand'` over `crates/before/src` outside `testing/` hits only a comment at board/ops.rs:1044; `O(D log k)` outside `testing/` appears only in meter/board sources; `Ω(M` outside `testing/` appears only in private module docs (query.rs:112, integral.rs:240, registry.rs:843, meter.rs:2269); version.rs:291, 473, 1412 are `include_str!` of islands, and version.rs:293 says only that `M` is the multiplication complexity; build.rs:201-236 renders the island from the JSON `contract` and `claim`; `git show b3f09baa -- version.rs` removed "pays a summary-merge cost that grows faster than the" and added "Superlinear but subquadratic time"); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (the prose binding chosen at 0a5bdaeb expired in three steps: b3f09baa removed the Display sentence, 2efff149 re-denominated the fold bound and made claims O-only, and b5a81583/2efff149 moved the wording into the roster; the fuelscape note's "nothing the asymptotics pins describe changes" is true of the bound and false of the quoted wording)
- Owner-gated: yes (whether rank/distance/lag/Ranked publicly promise the Ω lower bound is a documentation-contract decision)

Prose speaks in the present tense: a quotation of text not in the tree is a ghost reference, and a pin whose failure message names the wrong document to update does not bind the documentation it exists to bind. The rendered `# Complexity` text for `Display` is "superlinear, subquadratic time; `O(|self|)` space" (ops.rs:331); for the fold doors "`O((|self| + |iter|) log k)` time, `k` the operand count" (ops.rs:630); for rank "`O(M(|self|) · log |self|)` time" (ops.rs:360), with no Ω statement. A maintainer told to "update its `# Complexity` section" opens version.rs and finds an include.

Evidence:

         1	//! Liveness pins for the documented asymptotics: the growth behaviors
         2	//! the public rustdoc's `# Complexity` sections claim, held alive
         3	//! against the deterministic meters.
       ...
        44	/// `Display` limb work on the wide left-full shape grows super-linearly
        45	/// across a doubling, which is exactly what the rustdoc's "summary-merge
        46	/// cost that grows faster than the operand" sentence describes. When the
       ...
       236	         the documented `O(D log k)` overstates for this door, so update \
       237	         its `# Complexity` section and this pin together"
       ...
       383	/// through an independent backend multiplication — the value
       384	/// structure behind the `Ω(M(|v|))` floor the rank rustdoc states:

Resolution: Re-word the module doc and each failure message to name the authoring site (the operation's `contract` field in `crates/before-fuelscape/src/ops.rs`, regenerated into `fuelscape/<op>.json` and rendered into the section by build.rs) and quote the contract's actual wording. For the Ω floor, owner call: add the lower bound to the public contracts if it is meant to be promised (the style rule keeps Ω out of headlines and in prose), otherwise re-scope the three `mul_bound_*` pins' docs to the private derivation in `query.rs`/`integral.rs` with the invariant restated inline. Acceptance: every quoted sentence in asymptotics.rs appears verbatim in the artifact it names; every failure message names a file and field that exists at HEAD.

Construction: Change `fuelscape/version_display.json`'s `contract` to "linear time" and leave version.rs untouched: the rendered rustdoc now contradicts the pin, yet no `.rs` `# Complexity` section changed, so the failure message's instruction cannot be followed as written.

### testing-diff-gen-3: `party_shape` descriptor's doc claims the anonymous id is in its population; no driver feeds it
- Where: crates/before/src/testing/diff_ops.rs:553-554 (related: crates/before/src/testing/diff_ops/tests.rs:258-266, crates/before/src/testing/diff_ops/tests.rs:400-402, crates/before/src/party.rs:643-654)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (tests.rs:263 `fn $driver(a in arb_oracle_party_nonempty())` for the `(party)` arm; tests.rs:401 `assert_diff_ops!(super::$group, $env.p[0]);` from live clocks; party.rs:646-647 "never a publicly constructible value"); executed: no
- Seen by: adequacy; refutation: confirmed; history: no-rationale-found (the sentence landed at 46eb64f9 after efc0f5f6 had already restricted the `(party)` driver to non-empty ids)
- Owner-gated: no

Every test's doc comment must state its population accurately. `PARTY_SOLO`'s arbitrary driver draws `arb_oracle_party_nonempty()` and its organic arm feeds live clocks' ids, so `Leaf(false)` never reaches `party_shape_matches_the_oracle`; the anonymous `Party` is also `pub(crate)`-only. The sentence describes neither the drivers' population nor a public state.

Evidence:

       553	    /// as a tree, against the geometric lift. The anonymous id is in the
       554	    /// population: its walk is the single unowned whole-interval region.

Resolution: Drop the sentence; or, if the anonymous walk is meant to be covered, register a `party_shape` spelling in a group whose driver admits the anonymous id and say the value is crate-internal. Acceptance: the descriptor's doc describes only inputs its drivers produce.

### testing-diff-gen-5: Prose sweep: em-dashes in line comments and one assert message, "mint", "honest", temporal and anticipatory phrasing
- Where: crates/before/src/testing/diff_ops/tests.rs:86-87 (related: crates/before/src/testing/diff_ops/tests.rs:193, 196, 408, 475-476, 531, 544; crates/before/src/testing/diff_ops.rs:27, 34; crates/before/src/testing/generators.rs:228; crates/before/src/testing/grow_brute_force.rs:55; crates/before/src/testing/asymptotics.rs:46-47, 78, 449; crates/before/src/testing/compactness.rs:43; crates/before/src/testing/fuelscape_islands.rs:19)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `^\s*//[^/!].*—` over the thirteen files and grep -w for `mint|honest|honestly`; every cited line read); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: contradicts-hard-rule (writing-style.md:149-152 dashes in comments, :170 "mint", :326-330 moralized code; CLAUDE.md "colons (or semicolons) over em-dashes in log messages and comments")
- Owner-gated: no

The owner's writing rules name each of these: the spaced double-hyphen (or a colon) is the dash of code comments; "mint" is never written for constructing a value; "real"/"genuine"/"honest" are replaced by the property that holds; prose speaks in the present tense without anticipating a named future change. Em-dashes in `//` comments: diff_ops/tests.rs:193, 196, 408, 475, 476; generators.rs:228; grow_brute_force.rs:55; asymptotics.rs:78, 449; and inside the assert message at diff_ops/tests.rs:87. "mint a synonym": diff_ops/tests.rs:531, 544. "states honestly": diff_ops.rs:34. "the honest constant": compactness.rs:43. "Currently empty": fuelscape_islands.rs:19. "When the render-merge cure lands": asymptotics.rs:46-47. "where a body per population was three independent spellings" (diff_ops.rs:27) is a hand count of a replaced design.

Evidence:

        86	            "{name}: derived from the descriptor table AND rostered as \
        87	             bespoke — the tiling sides must stay disjoint; remove one"
       ...
       531	// mint a synonym per signature to appease the lint.

    diff_ops.rs:
        34	//! The table covers what a value-returning descriptor states honestly. The

    compactness.rs:
        43	/// case and re-pin the honest constant.

    asymptotics.rs:
        46	/// cost that grows faster than the operand" sentence describes. When the
        47	/// render-merge cure lands this pin reads red, and the rustdoc and this

Resolution: Mechanical sweep: colons or semicolons for the listed em-dashes (`///` and `//!` doc comments keep theirs); "coin" or "add" for "mint"; "states" for "states honestly" and "the measured constant" for "the honest constant"; "a cure that removes the merge flips this pin" for the anticipatory sentence; drop "Currently"; drop the "three independent spellings" count. Acceptance: `grep -nE '^\s*//[^/!].*—'` over the partition returns nothing; no `mint`, `honest`, or `Currently` in the partition.

### testing-diff-gen-10: The generators module doc enumerates contents and has rotted; "All trees" is contradicted by `deep_left_spine_party`
- Where: crates/before/src/testing/generators.rs:1-24 (related: crates/before/src/testing/generators.rs:263-283, crates/before/src/testing/generators.rs:368-495)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (line 1 says "in two families"; the third section header "variadic-law families" is at 368; `deep_left_spine_party` at 274-283 builds packed bits with a flat loop); executed: no
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired ("in two families" predates 6ed90b93's variadic section; "All trees" predates d8cd87d7d's bit-built spine)
- Owner-gated: no

Prose speaks in the present tense and does not hand-maintain enumerations of module contents. The doc lists functions by name and says "two families"; the file has three sections and omits `shape_version_wide`, `bushy_expand_party`, `arb_shape`, `arb_oracle_party_nonempty`, and the four family strategies. "All trees are built via the oracle's normalizing constructors" is false for the bit-built deep spine.

Evidence:

         1	//! Input generators for the property tests, in two families:
       ...
        18	//! All trees are built via the oracle's normalizing constructors (`O(1)` per
        19	//! node), then lowered to the impl with [`super::bridge`].

Resolution: State the structure, not the roster: three sections named as the file's headers name them, each with a one-sentence purpose; qualify "All trees" with the bit-built exception or drop "All". Acceptance: the module doc names sections, not functions, and every sentence in it is true of the file.

### testing-diff-gen-24: "door" is a crate-wide coinage never defined; "seam", "keystone", and "vehicle" point at things not named
- Where: crates/before/src/testing/asymptotics.rs:18-28 (related: crates/before/src/testing/diff_ops.rs:14, crates/before/src/testing/diff_ops.rs:725, crates/before/src/testing/diff_ops.rs:839, crates/before/src/testing/diff_ops/tests.rs:23, crates/before/src/testing/diff_ops/tests.rs:58, crates/before/src/testing/diff_ops/tests.rs:144, crates/before/src/meter/registry.rs:13, crates/before/src/laws/tests.rs:29, crates/before/src/testing/semantic_oracle/tests.rs:157)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (`grep -c -w 'door\|doors'`: 43 in asymptotics.rs, 2 in diff_ops/tests.rs, 1 in diff_ops.rs; a grep for a definitional form across `crates/before/AGENTS.md`, `lib.rs`, and `validation_index.rs` finds only uses; registry.rs:13 "the one public door"); executed: no
- Seen by: structure-prose; refutation: confirmed; history: contradicts-hard-rule (writing-style.md:164-169: every coinage is an identifier or a term defined once by contrast; "door" entered through commit messages and is now relied on)
- Owner-gated: yes (vocabulary choice)

"door" is the axis the asymptotics pins are organized on ("one pin per public door") and is anchored to no identifier and defined nowhere. "The seam this pin defends is a drift" (diff_ops/tests.rs:58) misuses seam. "the keystone replay" (diff_ops.rs:725, 839) names `replay_matches_across_references` only at semantic_oracle/tests.rs:157. "vehicle" (diff_ops/tests.rs:23, 529, 542) is metaphor for `assert_diff_ops!`.

Evidence:

        18	//! - **Fold doors** (`scan-meter`): one pin per public door whose
        19	//!   rustdoc claims the balanced reduction's `O(D log k)`, each measured
        20	//!   at its own door — the doors share the balanced core, but a door's

    diff_ops/tests.rs:
        58	/// The seam this pin defends is a drift, not an error: a pointwise pure

    diff_ops.rs:
       725	    /// maximum the keystone replay exercises through `join`/`send`.

Resolution: Define "door" once where the pins are organized (asymptotics.rs module doc: a *door* is one public method an operation is reachable through; the pins bind each separately because a door's wiring can drop a factor the shared core keeps) and cite that definition from registry.rs and laws/tests.rs, or replace with "public method"/"entry point". Name `replay_matches_across_references` where "keystone" is used; "the hole this pin closes is a drift" for the seam sentence; "the assertion macro" for "vehicle". Acceptance: every use of "door" resolves to one definition or is gone; "keystone" is followed by the test name on first use in each file.

### testing-oracles-1: Hand-maintained enumerations of suites and corpus importers have drifted
- Where: crates/before/src/testing.rs:10-20 (related: crates/before/src/testing.rs:46, crates/before/src/testing/exhaustive.rs:32-35, crates/before/src/span/tests.rs, crates/before/src/version/skyline/tests.rs)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (compared the doc's suite list against the `mod` declarations at testing.rs:41-50; `grep -rl 'all_normal_ids\|all_normal_events' crates/before/src` lists span/tests.rs and version/skyline/tests.rs beyond the six suites exhaustive.rs names; `git show --stat b5a81583` adds `mod fuelscape_islands;` without touching the doc); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (the inventories were written at ea0b1a8c and drifted at b5a81583 and ed1b3c8b; version/skyline/tests.rs was already an importer when the list was written)
- Owner-gated: no

The module doc enumerates the cross-cutting suites and omits `fuelscape_islands`, which line 46 declares; exhaustive.rs enumerates the kernel suites that import the corpus and omits two importers. Principle 5 forbids hand-maintained enumerations of module contents or callers because they rot silently, and both have.

Evidence:

        10	//! The cross-cutting suites: exhaustive small-scope enumeration
        11	//! ([`exhaustive`]), the function-space semantic oracle ([`semantic_oracle`]),
        12	//! the algebraic-law harness ([`algebraic_laws`] — a thin binding; the named
        46	mod fuelscape_islands;

    exhaustive.rs:
        32	//! necessarily outside the small scope: the kernel test suites import the
        33	//! corpus and run their own operation-specific sweeps over it (the skyline
        34	//! query, sweep, emit, fill, grow, and text suites all do — e.g. the pair
        35	//! queries' `exhaustive_small_scope_pairs_agree`). Two variants:

Resolution: In testing.rs, either add `fuelscape_islands` with its one-clause purpose or replace the list with the structural statement (scaffolding modules are `pub(crate)`, suite modules are private, each module's own doc states what it catches) and let the `mod` list be the roster. In exhaustive.rs, drop the importer list and keep the sentence that kernel suites import the corpus, with the one example. Acceptance: every `mod` declared in testing.rs is named in its doc or the doc no longer enumerates; exhaustive.rs names no subset of the importers.

### testing-oracles-16: Two test docs say the random `fork` refines up to two levels per call; the code, the module doc, and the equality pin say one
- Where: crates/before/src/testing/semantic_oracle/tests.rs:533-536 (related: crates/before/src/testing/semantic_oracle/tests.rs:287-288, crates/before/src/testing/semantic_oracle.rs:360-362, crates/before/src/testing/semantic_oracle.rs:371-384, crates/before/src/testing/semantic_oracle/tests.rs:612-624)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`fork` at 375-384 sets `level = res` when the region has two or more pieces, else `res + 1`, and both children are built at `level`, so resolution grows at most one level per call; the module doc at 360-362 states exactly that and `fork_chain_raises_resolution_one_level_per_fork` asserts `id_res(&kept.id) == k` after `k` forks; `git show 7487be16:crates/itc/src/testing/semantic_oracle.rs` line 212 reads `(res + 1).min(GRID_N)` and that commit's tests.rs already carries both "two levels" phrases); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (wrong at introduction; never revisited)
- Owner-gated: no

AGENTS.md holds every test doc's invariant statement to accuracy ("their incorrectness is a bug in the test"). The `grid_cap_is_never_reached` doc makes a two-level rate the reason the function space's resolution "is the binding constraint", and `fork_partitions` scans at `id_depth + 3` under the same premise, one level finer than needed. Both contradict the module doc beside them and the deterministic pin whose whole purpose is the one-level rate.

Evidence:

       533	/// This covers *both* the oracle's tree depth and the function space's probed
       534	/// resolution — the random `fork` refines up to two levels per call (vs. the
       535	/// paper's one), so its resolution can run ahead of the oracle's, and it is the
       536	/// binding constraint.
       287	        // Children refine ≤ 2 levels below the id's depth; scan deep enough to resolve them.
       288	        let g = (id_depth(&p) + 3).min(GRID_N);

    semantic_oracle.rs:
       360	/// Dealing out *existing* pieces adds no new boundary, so resolution grows only
       361	/// on the bisection of an indivisible piece — exactly the paper's rate (≤ 1
       362	/// level per fork). That is the one concession to a *finite* comparison grid:
       375	    let level = if owned_cells(i, res).len() >= 2 {
       376	        res
       377	    } else {
       383	        res + 1
       384	    };

Resolution: Restate both as one level (children carry a ceiling at most one level below the id's resolution); at 287-288 use `fs_grid(&[id_depth(&p) + 1])` with the one-level reason; if testing-oracles-17 rewrites the sweep's doc, the 533-536 paragraph is replaced there. Acceptance: no "two levels" or "≤ 2 levels" remains in the file; the three statements of the rate agree; `fork_partitions` and the chain pin stay green.

### testing-oracles-19: The deep-enumeration docs carry a run chronology and per-leg nanosecond pricing beyond the budget-and-machine annotation the owner ruled on
- Where: crates/before/src/testing/exhaustive.rs:53-59 (related: crates/before/src/testing/exhaustive/tests.rs:427-436, .agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:597-612)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both passages; `git show d2a9d04e` on both files removes only the calendar date from each; the decision record at 609-612 reads "its budget and machine annotation the contract for whoever runs it"; `grep -rn tiled crates/before/src` hits only tests.rs:429); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (severity lowered because the ruling retains budget and machine annotation); history: deliberate-and-holds for the annotation (ruling #53, preserved by the d2a9d04e sweep); the chronology and pricing exceed what the ruling names, and the reason for keeping them is stated nowhere in the tree; "cache-tiled" names an experimental run that never landed, so the no-ghost rule is not triggered
- Owner-gated: no

Principle 5 sends dated measurement reports and incident chronology to git and decision records. Ruling #53 keeps "budget and machine annotation" on the test; the two passages additionally recount two aborted runs (a 45-minute cap, a 31-minute profile), a cache-tiled variant not in the tree, and machine-bound ns/pair figures per leg, which is the history of the measurement rather than its result. The decision record already holds that history. The present-tense contract (hour-scale; run detached with `cargo test` because nextest terminates at its slow-timeout budget; verdict legs only at the deep bound because they allocate nothing; strided samples undershoot because the expensive pairs are structurally similar trees near the diagonal) is correct and should stay.

Evidence:

        53	//! structural compare — which prices at roughly seven eighths of the pair
        54	//! worker (measured on a stride-sampled quarter of the deep corpus,
        55	//! aarch64-apple-darwin, 16 cores, release: the difference leg
        56	//! ~66 and the join leg ~12 of a ~91 ns/pair wall worker), and at the full

    exhaustive/tests.rs:
       427	/// about a minute. Measured state (aarch64-apple-darwin, 16
       428	/// cores, release, quiet machine): two fully parallel runs were stopped at a
       429	/// 45-minute cap, one row-major and one cache-tiled, the row-major one
       430	/// profiled still inside the verdict pair product at 31 minutes — budget
       431	/// upwards of an hour and run it detached. Sampled-corpus extrapolation

    decision record:
       609	  robustness); every oracle-facing suite bounded. DECIDED (owner):
       610	  the hour-scale verdict-pair totality test stays, and stays out
       611	  of the gate — `#[ignore]`d, run detached on demand, its budget
       612	  and machine annotation the contract for whoever runs it.

Resolution: Trim both passages to the ruled contract: the budget (hour-scale, run detached), the machine and profile annotation, the structural reason for the leg split stated as a relative present-tense fact, and the reason strided extrapolation fails. Remove the aborted-run chronology, the cache-tiled mention, and the ns/pair figures; if a completed run exists, state its wall time, and if none does, say so plainly (the history pass reports no completed run on record). If the owner wants the chronology kept, state at the site why. Acceptance: neither file mentions "cache-tiled", "45-minute", "31 minutes", or ns/pair figures; budget and machine annotation remain; the decision record is unchanged.

### testing-oracles-29: Validation index prose: a ghost reference to replaced bodies, "mint", "honest", a self-description hedge, and a hand count
- Where: crates/before/src/testing/validation_index.rs:42-45 (related: crates/before/src/testing/validation_index.rs:5-6, crates/before/src/testing/validation_index.rs:75, crates/before/src/testing/validation_index.rs:142, crates/before/src/testing/validation_index.rs:178, crates/before/src/testing/validation_index.rs:182)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read each site; the "four instruments" at 75 enumerates four enforcing mechanisms and the fifth bold entry, the atlas, is described at 139-142 as enforcing nothing, so the count is consistent today); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed (the count item reframed as a hand count that is defensible as written); history: contradicts-hard-rule for "replaced" (entered at 5a3f9cb1, after the d2a9d04e ghost sweep; the bodies were deleted the next day in 223795c1) and for "mint"/"honest" (the writing-style rule postdates the prose)
- Owner-gated: no

The root AGENTS.md hard rule forbids referring to code that no longer exists ("The hand-written bodies it replaced"); the vocabulary rule bans "mint" for constructing a value and moralized words like "honest" where no adversary exists; "in the spirit of a documentation-only module" hedges a description of a module that is one; "four instruments" is a hand count that rots on the next addition even though it is consistent today.

Evidence:

        42	//! op-trace values, or the reverse. The hand-written bodies it replaced
        43	//! could not prevent one: each chose its own population, so coverage was
        44	//! a product nobody enumerated. Here the population belongs to the driver
        45	//! and the operation to the descriptor, and the two meet by construction.
         5	//! This page is a map for a maintainer orienting cold, in the spirit of
         6	//! a documentation-only module: it holds no code. Two questions organize
        75	//! Cost claims are guarded by four instruments in a deliberate layering:
       142	//! read the roster but never mint a threshold.
       178	//!   or an honest input legitimately did less work than the floor's
       182	//!   change, whichever direction is honest.

Resolution: 42-45: state the property positively ("A body that chose its own population would make coverage a product nobody enumerated; here the population belongs to the driver and the operation to the descriptor, so the two meet by construction."). 5-6: "This module is documentation only: it holds no code." 75: "guarded by a deliberate layering of instruments". 142: "never set a threshold". 178: "a legitimate input did less work than the floor's premise". 182: "whichever direction the code supports". Acceptance: none of "replaced", "mint", "honest", "in the spirit of", or "four instruments" remains in the file.

### module-graph-11: The validation index and the whole `testing` tree are invisible to both rustdoc passes
- Where: crates/before/src/testing.rs:50 (related: crates/before/src/testing/validation_index.rs:1-13, crates/before/src/lib.rs:453-454, justfile:256-270, crates/before/src/recurse.rs:30-31, crates/before/Cargo.toml:44)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read the module declarations and both docs recipes; `grep -c '\[\`'` counts 8 intra-doc link sites in validation_index.rs); executed: no
- Verification: confirmed, with one refinement on the resolution: a rustdoc pass with `--cfg test` is not a drop-in fix, because the cfg(test) tree uses dev-dependencies (`stacker` in recurse.rs:30-31 and 103, `proptest` in the suites) that a `cargo doc` lib build does not link; history: no-rationale-found.
- Owner-gated: no

`testing` is `#[cfg(test)]` and neither `docs` nor `docs-internal` sets that cfg, so the page that
calls itself "a map for a maintainer orienting cold" is never rendered and none of its intra-doc
links are ever resolved by rustdoc; `docs-internal`'s stated purpose (catching stale links in
private modules) does not reach this tree. `pub mod` inside a private cfg(test) module is visibility
that reaches no reader.

Evidence:

        50	pub mod validation_index;
    --- lib.rs:453-454 ---
       453	#[cfg(test)]
       454	mod testing;
    --- justfile:269-270 ---
       269	docs-internal:
       270	    RUSTDOCFLAGS="-D warnings --html-in-header {{ justfile_directory() }}/crates/before/docs/fuelscape-header.html" cargo doc --workspace --all-features --no-deps --document-private-items --target-dir target/doc-internal
    --- validation_index.rs:5-6 ---
         5	//! This page is a map for a maintainer orienting cold, in the spirit of
         6	//! a documentation-only module: it holds no code.

Resolution: Either move `validation_index` under the meter-gated tree (for example
`crate::meter::validation`), where `docs --all-features` renders it and checks its links (links
into `testing::*` would then need to become plain text or point at rendered items), or state at the
top of validation_index.rs that it is source-only and drop the `pub`. Acceptance: every intra-doc
link in the index is checked by a gate leg, or the file says it is source-only.

### recursion-2: Bridge id walks are unguarded; ev walks pass depth 0 to descend!, defeating its amortization
- Where: crates/before/src/testing/bridge.rs:29-60 (related: crates/before/src/testing/bridge.rs:109-172, crates/before/src/recurse.rs:9-14, crates/before/src/recurse.rs:22-28, crates/before/src/recurse.rs:88-93, crates/before/src/recurse.rs:111-117, crates/before/AGENTS.md:32-36, crates/before/src/party/tests.rs:832-834)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (bridge.rs and recurse.rs read in full; `0.is_multiple_of(64)` is true by the definition of `is_multiple_of`); executed: no
- Verification: confirmed; history: no-rationale-found (commit 1ddb5a483 names the bridge as a `descend!` user without qualification; nothing records the id-side omission or the constant depth)
- Owner-gated: no

`emit_id` and `read_id` recurse on oracle id depth with no guard, while
`emit_ev` and `read_ev` route through `descend!` with the literal depth 0, for
which `should_grow(0)` is true at every call: the headroom probe and the
closure frame the module doc says the macro avoids run on every level rather
than once per `STRIDE`, so the `STRIDE`/`RED_ZONE` derivation at
recurse.rs:33-51 does not describe these callers. recurse.rs:9-14 and
AGENTS.md:32-36 name "the oracle bridge" as the guarded surface without this
asymmetry; only party/tests.rs:832-834 acknowledges that the id-side bridge is
plain recursion. Test-only, and every bridge input is bounded by the oracle's
own recursive `Drop`, so no overflow is reachable that the oracle would not
also hit; the cost is prose that does not match the mechanism.

Evidence:

        41	            emit_id(out, l);
        42	            emit_id(out, r);

        56	            descend!(0, emit_ev(out, l));
        57	            descend!(0, emit_ev(out, r));

       118	        let (l, np) = read_id(bits, next);

       150	        let (l, after_l) = descend!(0, read_ev(bits, pos + 1, prev));
       151	        let (r, after_r) = descend!(0, read_ev(bits, after_l, prev));

    recurse.rs:
        91	pub(crate) fn should_grow(depth: usize) -> bool {
        92	    depth.is_multiple_of(STRIDE)
        93	}

       115	/// only every [`STRIDE`] levels is the call routed through [`grow`]. Use at
       116	/// each recursive call site: `descend!(depth + 1, self.rec(child_args, depth +
       117	/// 1))`.

    party/tests.rs:
       832	    /// Scales the plain-recursive oracle (and the id-side bridge) can walk on
       833	    /// the test stack; the ladder's top is deliberately beyond it.
       834	    const ORACLE_SCALE_MAX: usize = 4096;

Resolution: Pick one policy and state it at bridge.rs: either thread a depth
through all four walks and call `descend!(depth + 1, ...)` at each site
(`grow/tests.rs:125-180` is the in-crate pattern), or document that the bridge
is deliberately unguarded because the oracle's derived `Drop` bounds every
input it can meet, and drop the two ev-side `descend!` pairs that argument
makes decorative. Update recurse.rs:9-14 and AGENTS.md:32-36 to match.
Acceptance: bridge.rs's four recursive walks share one documented guard
policy, and recurse.rs and AGENTS.md describe it accurately.

### suite-economics-5: Wall-time claims and measurement narratives in test prose are unenforced, and exhaustive_small's is contradicted by an order of magnitude
- Where: crates/before/src/testing/exhaustive.rs:37-39 (related: exhaustive.rs:73-75; crates/before/src/testing/exhaustive/tests.rs:427-436 and 442-443; .config/nextest.toml:26; crates/suanpan/src/accumulator/tests/ledger.rs:212-214)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (run2.log: `PASS [ 12.914s] (7/7) before testing::exhaustive::tests::exhaustive_small`, load 5.3 rising to 19.9 from its own rayon pool; the sweep's run: 8.372 s at load 5 to 19; suanpan's ledger 2.545 s in run2 and 1.633 s in the sweep); executed: yes (run2)
- Verification: confirmed for exhaustive_small; reframed for the rest: the ledger and exhaustive_deep passages are measurement reports at declaration sites, not contradicted claims, and the amp_board_smoke site is dropped (see Dropped); history: no-rationale-found; exhaustive.rs:44-45 deliberately delegates "the measured state and how to run it" to the exhaustive_deep doc comment, which explains the narrative's placement but not its dated form
- Owner-gated: no

exhaustive.rs states twice that the small-scope cross-product is "well under a second"; in the dev profile the gate runs, it takes 8 to 13 s wall on a 16-core rayon pool. exhaustive/tests.rs:427-436 narrates two stopped runs and a profile as the deep variant's budget, and :442-443 restates nextest.toml:26's `60s` x `3` as "180 seconds"; ledger.rs:212-214 records "~4 s dev" and a one-time pass of the length-7 sweep "at pin time". Under Principle 5 a second-count in a doc is a number the code can change without touching the prose, and measurement reports live in commits, not declaration sites.

Evidence:

        37	//! - [`exhaustive_small`] runs every check in the normal gate at
        38	//!   [`ID_SMALL_DEPTH`] / [`EV_SMALL_DEPTH`] (256 ids, 691 events); the full op
        39	//!   cross-product is well under a second.

    (crates/before/src/testing/exhaustive/tests.rs)
       442	/// (`cargo test`, not nextest: the workspace's nextest profile terminates
       443	/// any test at 180 seconds, which this enumeration exceeds).

    (crates/suanpan/src/accumulator/tests/ledger.rs)
       212	/// Exhaustive over all 11-op schedules of length ≤ 6 (1,948,716
       213	/// states, each checked once; ~4 s dev — the length-≤ 7 sweep's
       214	/// 21.4M states also passed once, at pin time): word-scale deltas,

Resolution: state the mechanism that bounds the cost and drop the seconds. exhaustive.rs:37-39 and :73-75 keep the corpus sizes (256 ids, 691 events) and name the rayon pool; ledger.rs:212-214 keeps the schedule space and drops the timing and the pin-time note; exhaustive/tests.rs:427-436 keeps the order of magnitude ("budget upwards of an hour, run detached") and the reason a strided sample under-prices it (the structurally similar pairs concentrate near the diagonal) and drops the two-runs narrative; :442-443 names the nextest profile's terminate budget without restating its number. Acceptance: no second-count or "at pin time" narrative remains in the cited passages while the corpus sizes and the schedule count do.

**Nits (6), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| testing-diff-gen-13 | `crates/before/src/testing/generators.rs:206-208` | `skip_stress_pair` cites a "bounded lazy-skip" that no longer exists in either `is_disjoint` kernel | Re-denominate: "drives the per-level dominated-subtree skip (`IdReader::skip`) in the lockstep disjointness walk to Θ(scale) skips of O(1) subtrees ea ... | `evidence/partitions/testing-diff-gen.md` |
| testing-oracles-6 | `crates/before/src/testing/bridge.rs:97-107` | The bridge's impl-to-oracle section comment restates the module doc, repeats the recursion caveat four times, and coins "master harness" | Reduce the section comment to the banner plus one sentence saying `to_oracle_*` is the inverse of `from_oracle_*` ... | `evidence/partitions/testing-oracles.md` |
| testing-oracles-12 | `crates/before/src/testing/semantic_oracle/tests.rs:157-157` | "keystone" is used six times before any definition and as a first sentence that says nothing; "honest" and "genuine" moralize the correct reference | Open the keystone doc with the invariant ("After one op trace, every ordered pair of final clocks has the same comparison descriptor under all three r ... | `evidence/partitions/testing-oracles.md` |
| testing-oracles-18 | `crates/before/src/testing/exhaustive.rs:6-9` | The event corpus is the normalization closure over `{0, 1, 2}`, not "normal-form trees with bases in `{0, 1, 2}`" | State it as "the normal forms of every raw tree over the alphabet `{0, 1, 2}` (lifting and collapse carry bases up to `2(d + 1)`)" at both sites ... | `evidence/partitions/testing-oracles.md` |
| testing-oracles-21 | `crates/before/src/testing/exhaustive/tests.rs:49-51` | "At the bottom of this file" is no longer true, and "180 seconds" recomputes a number the nextest profile owns | "in the intrinsic symmetry laws section of this file"; "the workspace's nextest profile terminates slow tests at its `slow-timeout` budget ... | `evidence/partitions/testing-oracles.md` |
| testing-oracles-26 | `crates/before/src/testing/algebraic_laws/tests.rs:358-369` | The organic drive pairs versions with foreign clocks' regions without saying whether that is the intended regime | One comment above the arms (or on `Organic`) stating the regime, for example that versions meet foreign live regions here because the owner pairing is ... | `evidence/partitions/testing-oracles.md` |

**Cross-references.** testing-diff-gen-17 and meter-registry-tier2-14 are the compactness/tier2 flag-day pair; testing-diff-gen-22's asymptotics pins and fuzzfit-bands-2 are the "instrument prose points at wording that no longer exists" pair. testing-oracles-29, surface-roster-16, fuzz-guests-pins-24, tests-other-1, and module-graph-11 are the validation index. recursion-2 (the bridge's guard asymmetry) and testing-oracles-3 (another class) describe the same mechanism; recursion-4 and crate-root-33 the recurse.rs prose. suite-economics-5 and testing-oracles-19/-21 cite the same exhaustive docs. testing-diff-gen-24 and testing-oracles-12 both carry "keystone" (with clock-24, codec-base-text-tree-25, surface-roster-25). testing-diff-gen-13's lazy-skip ghost points at compare.rs (party).

## The instruments: the envelopes (tests/meter.rs)

15 findings (2 high, 1 medium, 7 low, 5 nit). Full records: `evidence/partitions/envelopes-a.md`, `evidence/partitions/envelopes-b.md`, `evidence/sweeps/prose-hygiene.md`.

### envelopes-a-1: File header and a dozen test docs describe the pre-flag-day implementation and contradict the pins beside them
- Where: crates/before/tests/meter.rs:4-11 (related: 13, 357, 428-429, 440-441, 465-466, 983-985, 996-997, 1400-1408, 1500-1501, 1517-1519, 1533-1534, 1548-1549, 1563-1564, 1896-1898; crates/before/src/lib.rs:350-358; crates/before/src/version/skyline.rs:231-274; crates/before/src/version/skyline/decode.rs:9-20; crates/before/src/version.rs:889-899)
- Class / severity / confidence: documentation / high / high
- Provenance: verified (read every cited line against the row constants at 262-300 and the kernels in skyline.rs, decode.rs, version.rs; `git log -S'far from that' -- crates/before/tests/meter.rs` returns only 0d1ea4905 (2026-07-22); `faf3cd0a` is dated 2026-07-25); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (all four); history: deliberate-but-expired (true at 0d1ea490/563e491e, expired at faf3cd0a, passed over by three later prose sweeps)
- Owner-gated: no

The header says the implementation is far from the O(n + m) contract while `lib.rs:350-358` states that contract as a hard guarantee whose violation is a bug; the row docs attribute costs to recursion frames and per-frame path sums on rows whose pins read zero segments and linear limb counts (`CMP_BIGROOT`'s 783 equals `DECODE_BIGROOT`'s 783, one wide root decode); the skyline-codec section and five decoder docs describe a transcode into a quadratically larger packed form, while the same table's comment at 293-295, `decode.rs`, and the pins (`SKYLINE_DECODE_CLIFF` 2_250 against `SKYLINE_VALIDATE_CLIFF` 1_770, one exactly-sized copy) say decode is validate plus one copy; and `skyline_oracle`'s doc names a "packed-form oracle" whose implementation was deleted at the flag day (the public `|` routes to `emit::join`, version.rs:899). Every test's doc comment states the invariant of record and must be accurate; the crate's hard rule forbids prose naming code that no longer exists; and a claim contradicted by the committed numbers beside it is the one kind of prose a re-pinner cannot afford, because it pre-authorizes the regression the row exists to refuse.

Evidence:

         4	//! The contract this suite is driving toward: no operation materializes
         5	//! transient state asymptotically larger than its packed operands, and every
         6	//! operation is amortized O(n + m) in the packed input bits — with no bound
         7	//! on value magnitude, tree depth, or encoded size. Today's implementation
         8	//! is far from that — several operations amplify their input by large
         9	//! constants or worse — so every scenario here pins the *current* measured
        10	//! cost, with ×1.25 slack, as a ceiling. A regression fails loudly now; each
        11	//! improvement tightens a committed number.

        13	//! Three deterministic meters, asserted together per scenario:

       357	/// Run one scenario body under both meters and assert its envelope.

       440	/// Comparing the dense spine against the empty version stays within its
       441	/// envelope (the recursion-frame cost: heap stays flat, segments do not).

       983	/// Comparing bigroot against the empty version stays within its envelope
       984	/// (today the worst amplifier: per-frame owned path sums, quadratic in the
       985	/// root magnitude × depth).

      1404	// input bytes; the decoder rows add the transcode back to the packed
      1405	// form, whose materialized heights and floors are priced by that packed
      1406	// output (on the comb it is quadratically larger than the skyline input,
      1407	// so no transcode can be skyline-linear; the validator is the piece that
      1408	// carries the wire-bit-linear claim).

      1517	/// The packed output stores a fresh `gamma(2^k − 1)` per tooth, so the
      1518	/// materialized heights and floors are output-sized — quadratically above
      1519	/// the skyline input, linearly within the packed form being rebuilt.

      1896	/// One family shape and the packed-form oracle's answer against the

    The same file's table comment, and the kernel:

       293	    // Skyline decoder rows: validation plus the wrap into storage — the
       294	    // stored coding is the skyline stream itself, so decode materializes
       295	    // nothing beyond the copy and stays priced by the wire input.

    decode.rs:
        14	pub(crate) fn decode_bits(bits: BitsView<'_>) -> Result<Version, Decode> {
        15	    validate_bits(bits)?;
        18	    let mut copy = crate::codec::BitsBuf::with_capacity(bits.len() + 1);
        19	    crate::codec::extend_from_view(&mut copy, bits, 0, bits.len());
        20	    Ok(Version::from_bits(copy))

Resolution: Rewrite lines 4-11 to state the suite's present role: the contract is `lib.rs:350-353`'s; five operations the claims document demonstrates over their documented bounds (`Version::join`'s re-anchor cascade, skyline-coding-9; `Ranked::cmp`'s settle, rank-33; the masked comparison's `peek_flip` term, skyline-sweep-place-masked-5; `Query::coverage`'s per-hole sweep, span-causally-36; `tick`'s memo-family heap, skyline-fill-grow-2) are defects under repair, not exceptions (owner ruling 1, 2026-09-02, `triage/rulings.md`): name them here as under repair with their finding ids, so the list empties as the fixes land, and never in `lib.rs`; and each row pins the current measured cost at ×1.25 so a regression fails and an improvement re-pins. Replace "Today's implementation is far from that" and its recursion-and-transcode narrative with that named list, never with a sentence asserting that the contract holds. Replace the meter enumerations at 13 and 357 with non-counting phrasing ("the deterministic meters", "under every meter"). Re-state each row doc in terms of the mechanism its table row's trailing comment already names: 428-429 (the validator's bit stack, drop "today"), 440-441 (the iterative sweep, zero segments), 465-466 (drop "today"), 983-985 and 996-997 (one wide root decode, or two, linear), the header 1400-1408 and the decoder docs at 1500-1501, 1517-1519, 1533-1534, 1548-1549, 1563-1564 (validate plus one exactly-sized copy, same scan reading as the validate row by construction), and 1896-1898 (the public operator's result, not an oracle; see envelopes-a-14 for the value-leg consequence). The assert messages' "transcoded"/"the transcode round-trips" (1429, 1447, 1463, 1481, 1497, 1512, 1530, 1545, 1560, 1575) legitimately name `Packed::version`'s construction-language transcode and may stay. Acceptance: `grep -nE 'far from that|Three deterministic|both meters|today|recursion-frame|per-frame|transcode back|materializ|packed-form oracle' crates/before/tests/meter.rs` over lines 1-5305 returns nothing but the construction-language sites; each decoder doc agrees with the table comment at 293-295.

### envelopes-b-27: `span_shares_the_crossing_folds` documents a limb leg the body does not have and narrates its removal
- Where: crates/before/tests/meter.rs:10261-10292 (related: 10156-10202, 10293-10327)
- Class / severity / confidence: documentation / high / high
- Provenance: verified (read the doc and body; `git log -S'No limb leg'` -> e4c9b083e, whose message says "the span crossing-fold pin retires its limb undercut" and whose diff adds the "No limb leg ... once rode" comment without touching the doc paragraph at 10265-10269); executed: no
- Seen by: adequacy, structure-prose; refutation: confirmed (downgraded to low as a doc-only fix); history: contradicts the root AGENTS.md hard rule ("Nothing in the codebase refers to code that no longer exists")
- Owner-gated: no

The doc promises "the two meter faces of the fusion, one leg each" and devotes a paragraph to a limb leg with an "unfused hull" witness; the body has only the touch leg and says so in a comment that explains what the removed leg "once rode". This is a test doc that does not state the invariant the test asserts (a claim contradicted) and a ghost reference to a retired assertion, the root AGENTS.md hard rule. The severity follows the rule breached; the fix is a doc edit.

Evidence:

     10261	    /// GREEN PIN: the fused hull decodes the pair once at arithmetic
     10262	    /// width, and folds each crossing into ONE shared running
     10263	    /// difference — the two meter faces of the fusion, one leg each.
     10264	    ///
     10265	    /// The limb leg pins the decode sharing at arithmetic width: each
     10266	    /// wide-gamma decode records one value-width limb count, and the
     10267	    /// composed emitters decode every operand twice. Its witness is an
     10268	    /// unfused hull that decodes per emission; it is blind to the
     10269	    /// accumulator, whose folds record no limb ops.
    ...
     10286	        // No limb leg: word-scale crossings never enter the limb
     10287	        // denomination, so the arithmetic-width undercut that once rode
     10288	        // the composed emissions' duplicated zigzag work has no margin
     10289	        // left to read. Decode sharing is pinned structurally by the

Resolution: rewrite the doc to the touch leg alone (the fused hull folds each crossing into one shared difference; a two-accumulator spelling reads the composed folds back; decode sharing is pinned by the scan identity in `span_fuses_the_pair_walk`); delete the "once rode" sentence, stating only the present fact that word-scale crossings enter no limb denomination, so the pin has no limb leg. Acceptance: the doc names exactly the legs the body asserts; `grep -n 'once rode\|limb leg pins' crates/before/tests/meter.rs` is empty.

### prose-hygiene-1: tests/meter.rs narrates the pre-skyline recursive, quadratic implementation as the present
- Where: crates/before/tests/meter.rs:7-11 (related: crates/before/tests/meter.rs:428-429, 440-441, 465-466, 983-985; crates/before/tests/meter.rs:263, 269; crates/before/src/lib.rs:350-358)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read the module doc, `Envelope`, `envelope()`, the envelope table 245-301, the scenario docs; cross-read lib.rs:342-358 and version.rs:1730-1741); executed: no
- Verification: confirmed; history: no-rationale-found (the module doc entered in 434863669 on 2026-07-22 and the scenario docs in 0d1ea4905 the same day; the flag day faf3cd0a landed 2026-07-25; neither doc was rewritten)
- Owner-gated: no

The envelope suite's module doc says the implementation is "far from" its
contract, and the `cmp_*`/`decode_*`/`tick_*` scenario docs describe
recursion frames growing stack segments and a quadratic per-frame comparison.
The envelope table beside them pins `segments = 0` on every row and names the
iterative sweep, and the crate docs promise every asymptotic claim as a hard
guarantee.

Evidence:

         7	//! on value magnitude, tree depth, or encoded size. Today's implementation
         8	//! is far from that — several operations amplify their input by large
         9	//! constants or worse — so every scenario here pins the *current* measured
        10	//! cost, with ×1.25 slack, as a ceiling. A regression fails loudly now; each
        11	//! improvement tightens a committed number.

       440	/// Comparing the dense spine against the empty version stays within its
       441	/// envelope (the recursion-frame cost: heap stays flat, segments do not).

       983	/// Comparing bigroot against the empty version stays within its envelope
       984	/// (today the worst amplifier: per-frame owned path sums, quadratic in the
       985	/// root magnitude × depth).

       263	    pub const CMP_DENSE: Envelope = envelope(30_720, 0, 0, 0); // the iterative sweep over the Bytes-backed at-rest form (OpenedPair states the pair walk's opening move once); word-valued payloads keep the limb column at zero
       269	    pub const CMP_BIGROOT: Envelope = envelope(40_340, 0, 783, 469); // the iterative sweep over the Bytes-backed at-rest form; the wide root's decode is the limb record

       351	//! pathological input shapes. Any asymptotic claim is a hard guarantee that the
       352	//! operation will perform in time proportionate to that bound, for all input

Resolution: rewrite lines 4-11 to state what the suite pins (exact counter
envelopes at measured ×1.25, the limb floors, the committed known-bad
kernels) without the status narrative; restate the scenario docs at 428-429,
440-441, 465-466 and 983-985 from the envelope-table comments (iterative
sweep; validate-plus-wrap decode; the fused tick), dropping "today" and every
recursion or quadratic description. Acceptance: no scenario doc in the file
describes a mechanism its own pinned row contradicts, and `grep -n today
crates/before/tests/meter.rs` returns nothing.

### envelopes-a-5: "Only ever tightened" is contradicted by the tables' own re-denomination clause and by the pin history
- Where: crates/before/tests/meter.rs:245-250 (related: 1174-1189, 1637-1641, 1865-1869, 2139-2141)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git log -S'query_envelope(     4_485' -- crates/before/tests/meter.rs` returns f75964702 (2026-07-28), which raised `SKYLINE_RANK_WIDE_TOOTH`'s heap ceiling from 3_095 to 4_485 for an attributed mechanism change; read the five preambles); executed: no
- Seen by: adequacy; refutation: confirmed; history: deliberate-but-expired (the monotone rule is 0d1ea490's; c48c7f0e added the re-denomination amendment to the rank table only; f7596470 raised two ceilings outright)
- Owner-gated: no

Each table says its ceilings are only ever tightened, with the older ceiling standing when a remeasure rises inside it; the rank table then sanctions rises for re-denomination, and the history shows ceilings raised past the old value with an attribution. The rule in force is "a rise is a re-pin attributed in its commit", not monotonicity; a stated rule the history does not obey is one a re-pinner cannot rely on.

Evidence:

       245	// The envelope table: pinned ceiling = measured ×1.25, rounded up
       246	// (aarch64-apple-darwin, dev profile, three identical runs), and only ever
       247	// tightened: where a remeasure rises while staying inside an existing
       248	// ceiling (the spilled-magnitude heap cells, which carry the backend's
       249	// `len/8 + 2` words of growth headroom per heap allocation), the older,
       250	// tighter ceiling stands. The trailing comment on each line states the

      1186	// A re-denomination of a column — the same work newly counted at the
      1187	// metered seam (`Base::trailing_zeros`, widening shifts) — is a
      1188	// sanctioned rise under the tightening rule, recorded in its pin commit,
      1189	// never a weakening.

Resolution: Replace "only ever tightened" in the five preambles (once, after envelopes-a-4) with the rule applied: a ceiling moves down on remeasure; it moves up only with an attributed mechanism or re-denomination named in the pin commit; drift inside the ceiling leaves the older ceiling standing. Acceptance: the stated rule is one every commit in `git log -p -- crates/before/tests/meter.rs` satisfies.

### envelopes-a-10: Door rows print the generator's construction-language size as the operand's input size
- Where: crates/before/tests/meter.rs:446-448 (related: 459, 472, 488, 508, 534, 559, 583, 607, 629, 650, 670, 686, 705, 990, 1005, 1042, 1075, 1089, 1284, 1298, 1317, 2111; 1406-1407; crates/before/src/meter.rs:85-121)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read `Packed` at src/meter.rs:85-97 and `Packed::version` at 116-121: `bytes` is the min-lifted packed preorder construction stream that `version()` transcodes; the refutation pass's run log shows `rank_dense` printing `input_bytes=62501` and `skyline_rank_dense` printing `input_bytes=46876` for the same operand, `rank_bigroot` 15002 against 13752); executed: no (the log was read; the finding rests on reading)
- Seen by: scaffolding, adequacy; refutation: confirmed; history: deliberate-but-expired (at 0d1ea490 `p.bytes` was the stored form; the flag day changed the decode rows to `wire.len()` and left the cmp/join/tick/rank rows)
- Owner-gated: no

`input_bytes` is printed on every MEASURED line, appears in every failure message, and is what a re-pinner reads amplification ratios from; on these rows it is the size of an artifact the operation never sees. On the dense spine it overstates the stored operand by about a third; on the cliff comb the file's own header at 1406-1407 says the construction stream is quadratically larger than the stored one. Denominate resource amplification precisely and state the denominator with every claim. The flatness runs already use `v.encode().len()`.

Evidence:

       446	    let r = metered("cmp_dense", p.bytes.len(), &envelope::CMP_DENSE, || {

    src/meter.rs:
       116	    /// Lift an event-shape generator's output into a stored [`Version`](crate::Version),
       117	    /// transcoding the construction language (a min-lifted packed preorder
       118	    /// stream) into the skyline coding the version stores.
       119	    pub fn version(&self) -> crate::Version {
       120	        crate::Version::from_bits(skyline::encode_bits(self.as_bits()))
       121	    }

Resolution: Pass the bytes of the operand the operation reads: `v.encode().len()` (or `encoded_bits().div_ceil(8)`) for version operands, the id bytes for parties (which are stored as built). Acceptance: every `input_bytes` printed for a version operand equals that operand's encoded byte length; `p.bytes.len()`/`ev.bytes.len()` appears in the range only for `Party` operands and the canary.

### envelopes-a-18: Unanchored coinages and significance refrains at maintainer altitude
- Where: crates/before/tests/meter.rs:3084-3102 (related: 37-46, 210, 3051, 3596, 3670, 4070, 4126, 4682, 5154 ("genre"); 108, 124, 524, 576, 3099, 3326 ("freight"); 3092, 3117, 3258, 3264 ("daylight", beside `SEAM_CLEARANCE`); "funded"/"funding" at 17 sites; "honest" at 18 sites; "never decoration" at 3566, 3782, 4863, 4957; "Semantics first:" at 3183, 3334, 3446; 3546 ("truings"); crates/before/src/version/skyline/overlay.rs:76-84)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (awk counts over lines 1-5305: honest 18, genre 10, freight 6, daylight 4, funded 15 plus funding 2, "never decoration" 4, "Semantics first" 3, truing 1); executed: no
- Seen by: structure-prose; refutation: confirmed (noting "genre" is defined by contrast at 37-46 for the two lower-bound kinds, and later uses are a different sense; "funded" has an anchor in suanpan's potential argument but none here, where overlay.rs defines "priced by")
- Owner-gated: no

Every coined term is anchored to an identifier or defined once by contrast; a metaphor that exists only as texture fails. "genre" is defined at 37-46 for two lower-bound kinds and then reused for families ("close-reveal genre", "many-freezes genre", "arming genre"); "daylight" shadows the identifier `SEAM_CLEARANCE`; "freight" and "funded" stand for per-leaf register work and for "priced by", the convention overlay.rs defines; "honest" moralizes code eighteen times where the sentence already says whether an improvement is genuine or a meter is dead; "so this band is never decoration" and "Semantics first:" restate what the cited kernel name and the value-leg assert already prove.

Evidence:

      3091	    // annihilation), so the guards' clearance line itself — hops decided
      3092	    // at exactly two digits of daylight, in both directions — is reached
      3098	    // control whose run difference isolates the hops from the shared
      3099	    // consume/arm freight. The clearance band moves only the residue's

        24	//!   bypass any allocator meter; the segment counter is the honest stand-in

      3782	    /// failing on this family, so this band is never decoration.

Resolution: "genre" outside 37-46 to "kind" or "family"; "freight" to "per-leaf register work"; "daylight" to "digit clearance" (matching `SEAM_CLEARANCE`); "funded width" to "the width its own code paid for" or "priced by" per overlay.rs; "honest improvement" to "an improvement", "honest stand-in" to "the stand-in"; delete the "never decoration" and "Semantics first:" sentences or fold their fact into the preceding clause. Acceptance: `grep -c 'honest\|freight\|daylight\|never decoration\|Semantics first\|truing'` over lines 1-5305 is 0; "genre" appears only in the file doc's contrast definition; "funded" is gone or defined once beside "priced by".

### envelopes-a-20: Principle 5 residue: history at declaration sites, hand-copied scenario sizes, a stale "thin margin" claim, and an inline measured ratio
- Where: crates/before/tests/meter.rs:3546-3547 (related: 1372-1376, 2745-2746, 3674-3677, 5070-5078; 1656-1660; 2365-2367; size literals at 661, 867, 1748, 1912, 1932-1933, 2066, 2135, 2164, 2184, 2203, 2259, 2280, 3151, 3307, 3424)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read each site; `git show 500d4d09 -- crates/before/tests/meter.rs` shows the excised record for `SKYLINE_CMP_WIDE_TOOTH`'s heap: "1_032 (dashu-int backend) -> 1_224 (the zero-run ledger's map node) -> 1_064 (OpenedPair ...)" against the 1_250 ceiling, so the last recorded basis sits about 17% under the ceiling, ordinary ×1.25 headroom rather than a thin margin; the current basis was not re-measured here); executed: no
- Seen by: structure-prose, instrument-correctness, adequacy, scaffolding; refutation: confirmed ([38], [39], [53]; [9]'s manifest-pin leg refuted); history: [38] contradicts the root AGENTS.md hard rule (2745-2746 names a retired accounting; 1372-1376 narrates a retired fold; 3674 cites "The review"; 3546-3547 is residue of the dated-ledger excision d2a9d04e); [39] no-rationale-found; [53] deliberate-and-holds (500d4d09 kept "~1.6 touches per delta" as an order-of-magnitude calibration; the rationale lives only in that commit); [9] deliberate-but-expired (the change-detector role was written for a 1_032-under-1_050 margin at 35fc5ab5)
- Owner-gated: no

Prose speaks in the present tense; history lives in git. "Two independent truings", "the tightened record that retired the frozen-width-per-tooth quadratic baseline", "was the adversarial arm", "The review's residual risk", and "[measured under the live mutation, same harness, at pin time]" narrate how a constant got its value rather than what it asserts, and "which review?" is unanswerable from the tree. The scenario-size literals ("125k", "250k-deep", "40k-bit", "k = 1,024") restate constants the code can change without touching the prose. The `SKYLINE_CMP_WIDE_TOOTH` comment assigns a special change-detector role to a "deliberately thin heap margin" that the last recorded basis shows to be ordinary slack. The "~1.6 touches per delta" sentence was kept deliberately as a calibration but does not say so or state a band, and nothing catches it going stale.

Evidence:

      3546	    // Ceilings: the element-wise tightest of two independent truings,
      3547	    // held green by the run below.

      2745	    /// tightened record that retired the frozen-width-per-tooth
      2746	    /// quadratic baseline.

      3674	    /// The review's residual risk: `Θ(k)` freezes where one operand's

      1656	    // SKYLINE_CMP_WIDE_TOOTH's deliberately thin heap margin is a
      1657	    // change-detector on the backend's and the accumulator's allocation
      1658	    // policies: the committed Cargo.lock (dashu-int 0.5.0 exact) is what
      1659	    // makes the measurement deterministic, and a cargo update to any other
      1660	    // 0.5.x is a deliberate re-measure event, not noise.

      2365	    /// fails loudly here instead. The metered accumulator measures about
      2366	    /// 1.6 touches per delta on this comb, so the one-touch floor is
      2367	    /// comfortable.

      1932	/// The whole output collapses to one leaf through 125k absorb steps
      1933	/// around a held 125k-bit code, so this row is linear only because absorb

Resolution: Restate each declaration positively (3546: "the measured record ×1.25 at both scales"; 2745-2746: drop the clause; 1372-1376: "`Sum` accepts any order; high-first makes every later add a shifted word, so it is the pinned order"; 3674: drop "The review's residual risk:"; 5076-5077: cite the mutants roster entry instead, see envelopes-a-22); replace size literals with the constant's name or the structural phrase; at 1656-1660 either state the row's present role plainly (an ordinary ×1.25 ceiling whose heap reading depends on the locked `dashu-int` allocation policy) or, if the change-detector role is wanted, re-measure and re-pin the ceiling thin again in a commit that says so; at 2365-2367 either state "kept as an order-of-magnitude calibration, band ×1 to ×2" at the site or drop the sentence. Acceptance: no "review", "truing", "retired", "was the", "at pin time", "deliberately thin", or size literal with a named constant remains in lines 1-5305; the 1.6 sentence names itself a calibration with a band or is gone.

### envelopes-b-3: Coined labels and register transplants: `GREEN PIN`, `mandate`, `mint`, two senses of `genre`, moralized and economic vocabulary
- Where: crates/before/tests/meter.rs:5685-5689 (related: 8917, 9106, 9531, 9572, 9652, 9699, 9725, 9828, 9874, 9955, 10034, 10138, 10204, 10223, 10261, 10389, 10436, 10487 (`GREEN PIN`); 8735, 9203 (`mint`); 6832; 6404; 5751, 9380; 6689, 6733; 5536, 5675, 8161, 8262; the file doc at 35-38)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep over 5306-10808: `GREEN PIN` 18, `RED PIN` 0 in the whole file and no definition anywhere under crates/before; `genre` 22; `honest` 11; `mint` at 8735 and 9203; `git show 03f78744c:crates/before/tests/meter.rs` carries `RED PIN:` at 3474 and 3528 beside `GREEN PIN:` at 3589, and `99d302083`'s tree keeps only the GREEN label); executed: no
- Seen by: structure-prose, scaffolding; refutation: confirmed (and added the two `mint` sites); history: deliberate-but-expired for `GREEN PIN` (one half of a RED/GREEN pair whose RED half every cure removed); no rationale for the rest
- Owner-gated: no

`GREEN PIN:` prefixes eighteen doc first sentences and is defined nowhere; the contrast that gave it meaning (the `RED PIN` labels on committed-failing pins) is gone, and later modules reused the label for relational identities with no red counterpart. `genre` is used for cost-mechanism classes ("the settle's densified-image span genre") while the file doc defines it only for the two lower-bound kinds. "mint"/"minted" is used for producing a value at 8735 and 9203. `mandate`/`mandatory` (5688, 5764), `honest` (11 uses), "not a vibe" (6832), "is the point" (6404), "kills"/"killed" (5751, 9380), "load-bearing" (6689, 6733), and "never decoration" (four uses) perform significance rather than state mechanism. The vocabulary rule: every coined term is anchored to an identifier or defined once by contrast; a doc's first sentence stands alone in a listing.

Evidence:

      5685	    /// Carries the `min_ticks` closed form (`s · x + 1` over the
      5686	    /// committed factors) as the generator's semantic leg, the
      5687	    /// exact-rank leg (the answer is the product `2·x·y + 1` — the
      5688	    /// `Ω(M(|v|))` mandate's witness), and the
      5689	    /// one-touch-per-operand-byte liveness floor.
    ...
      8735	    /// consume-minted width-b boundary difference parks in the latent
    ...
      9531	    /// GREEN PIN: on a full sweep (no demand settles before
      9532	    /// exhaustion), the fused membership walk scans exactly the

Resolution: delete the `GREEN PIN:` prefixes (each sentence already states the invariant); replace `genre` outside the file doc's floor/tripwire contrast with `class` or define the second sense once; `mandate` to `lower bound`; `consume-minted` to `consume-time`, `mints` to `produces`; drop "not a vibe", "is the point", "kills"; "honest improvement" to "improvement" where the dead-meter contrast is already stated. Acceptance: `grep -c 'GREEN PIN' crates/before/tests/meter.rs` reads 0; `grep -n 'mint' crates/before/tests/meter.rs` is empty; `genre` appears only in its defined sense.

### envelopes-b-6: The fork row's docs contradict each other on whether a cost record exists, and name no instrument for the split's spine walk
- Where: crates/before/tests/meter.rs:6395-6419 (related: 6410; crates/before/fuzzfit/harness/src/bands.rs:557; crates/before/src/meter/board/ops.rs:1304; crates/before/src/party/ops/split.rs:7)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git blame` resolves 6395 and 6416-6419 to fc862595d9; grep finds `kernel: "ff_party_fork"` at fuzzfit bands.rs:557 and the `party_fork` board cell at ops.rs:1304; split.rs:7 claims `O(|self|)`); executed: no
- Seen by: structure-prose, adequacy (and scaffolding's finding 9, whose circularity objection is answered by the inline rationale); refutation: confirmed; history: no rationale found (both sentences date from the same commit)
- Owner-gated: no

The section header calls `fork_env::ID_FORK` "the split kernel's committed cost record" while the test doc twenty lines later says fork has "no committed cost record"; both were written together, so the second was true only until the first landed. The row's scan column pins the split's absence from the metered primitives by stated design (6400-6405, fc862595d9), which is fine, but nothing in the row says which instrument does price the `O(|self|)` spine walk: it is the fuzz-fit fuel band `ff_party_fork` and the board's `party_fork` cell, neither named here, so the deliberate hole reads as an accidental one. Prose speaks in the present tense, and a row that looks like the instrument should point at the instrument.

Evidence:

      6395	// ─── fork envelope (the split kernel's committed cost record) ───────────────
    ...
      6416	/// Fork is the one id operation with no committed cost record: its halves
      6417	/// materialize (the heap column prices them), its spine walk is iterative
      6418	/// (zero segments), and its writes are raw (the scan pin above). The
      6419	/// rejoin closes the semantic leg: fork then join is the identity.

Resolution: restate 6416-6419 positively ("Fork's cost has no counter of its own here: the halves materialize (heap), the spine walk is iterative (zero segments), the writes are raw (the scan pin above); the walk's linearity is priced by the fuzz-fit `ff_party_fork` band and the board's `party_fork` cell"). Acceptance: the header and the test doc agree, and the row names the instrument that prices the walk.

### envelopes-b-12: Past-tense and provenance narration at three sites: a bracketed uncommitted demonstration, a `//` provenance block under a `///` doc, and "the old" check
- Where: crates/before/tests/meter.rs:7137-7140 (related: 7551-7554, 9881-9887; .cargo/mutants.toml:70-76)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git log -S'demonstrated under the live'` -> 500d4d094; `git show 4874f527b9:crates/before/tests/meter.rs` shows the `//` block at 7551-7554 was a dated measurement ledger ("536 -> 352 (2026-07-30 ...)") that 500d4d094 re-worded in place without changing the comment form; 9aec9aa2 (2026-08-18) recorded the compacting delete-field mutant CAUGHT by this row and .cargo/mutants.toml:70-76 states it; e2f4e2a5e dissolved the range walk the "old" checks at 9881-9885 refer to); executed: no
- Seen by: adequacy, structure-prose, scaffolding; refutation: confirmed for the bracket and the split comment, reframed for the compaction-off demonstration (the separator exists as a documented cargo-mutants disposition outside the gate); history: deliberate-but-expired at all three sites
- Owner-gated: no

The ascend row's doc narrates a one-off hand experiment in brackets ("demonstrated under the live swap, same harness") when a committed present-tense fact is available: the mutants campaign of record found `MinWeb::compacting`'s delete-field mutant killed by exactly this row (.cargo/mutants.toml:70-76). `JOIN_EQUAL_OPERANDS_PEAK` carries a `///` doc followed by a `//` block continuing the same rationale, the residue of a provenance ledger whose readings were excised. `dominance_bails_at_the_refuted_start` compares against "the old *first check alone*" and "the old floor-first check", a composition the test constructs live (9906-9912), so "old" narrates the dissolved range walk rather than naming what is measured. Prose speaks in the present tense; provenance lives in git.

Evidence:

      7137	/// accumulator hop. With compaction deleted the same body reads over
      7138	/// both the heap and touch ceilings \[demonstrated under the live
      7139	/// swap, same harness\], so this row is the measured basis
      7140	/// `MinWeb::compacting` cites.
    ...
      7548	/// Peak-heap ceiling for the equal-operands join fold: the fold's own
      7549	/// machinery (the counter's group vec, the dedup adapter's held clone),
      7550	/// none of it proportional to the operands.
      7551	// The equality rung's hand-back is an O(1) refcount bump, so the fold's
      7552	// peak is its size-independent machinery alone — the flatness leg below
      7553	// is the proof. Ceiling 1.25x the measurement of record (the reading
      7554	// lives in the pin commit).
    ...
      9881	    /// replaces; against the old *first check alone* the earlier bail
    ...
      9884	    /// (`probe < lo`, comparable) is where the bail changes class: the
      9885	    /// old floor-first check could confirm `Greater` only at

Resolution: at 7137-7140 replace the bracket with the present-tense fact ("the delete-field mutant of `MinWeb::compacting` reads over both ceilings under the campaign of record, .cargo/mutants.toml"); merge 7551-7554 into the `///` block; replace "the old first check alone" and "the old floor-first check" with "the two-check composition's first check" (the shape built at 9906-9912). Acceptance: no bracketed history, no `//` continuation of a `///` doc, and no "the old" in the range.

**Nits (5), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| envelopes-b-9 | `crates/before/tests/meter.rs:6851-6854` | The "expansion rows" header sits above the hole and masked rows it does not describe | move the comment to precede `TICK_EXPAND_SPINE` at 6861 and give the hole/masked rows a one-line header of their own | `evidence/partitions/envelopes-b.md` |
| envelopes-b-24 | `crates/before/tests/meter.rs:8661-8668` | The width-circulation header restates the file doc's two-genre paragraph | delete 8661-8668 and let the module's constants cite the file doc's genres by name | `evidence/partitions/envelopes-b.md` |
| envelopes-b-29 | `crates/before/tests/meter.rs:10436-10446` | `span_decode_shares_the_second_payload_decode`'s doc says the gap is "exactly" one check per leaf delta; the test asserts only `>=` | either assert the gap (count the second component's leaf deltas through the public shape iterators and `assert_eq!(fused ... | `evidence/partitions/envelopes-b.md` |
| envelopes-b-30 | `crates/before/tests/meter.rs:10649-10651` | `distinct_buffers_keep_the_walked_paths_covered`'s doc lists operations the body does not exercise | drop "/distance/lag" or point at `metric_fast_paths_skip_the_fold` as the test that pins them | `evidence/partitions/envelopes-b.md` |
| prose-hygiene-14 | `crates/before/tests/meter.rs:208-210` | "improvement tripwire" names the benign trigger, not the failure the floor detects | rename the genre to "bypass floor" (the file's own phrase at | `evidence/sweeps/prose-hygiene.md` |

**Cross-references.** envelopes-a-1 and prose-hygiene-1 are the same file header and row docs. envelopes-a-10 and meter-core-3 are the construction-language denominator from the two sides. envelopes-b-12's compaction-off bracket, skyline-watermark-8's ratios, and `.cargo/mutants.toml:70-76` move together (Open questions 16). envelopes-b-3's `GREEN PIN` labels and envelopes-a-18's "genre" are the file's coinages; prose-hygiene-14's "improvement tripwire" rename is the third. envelopes-b-6's fork row names no instrument; the pricing instruments are `ff_party_fork` (fuzzfit) and `party_fork` (board). envelopes-b-27's ghost limb leg is the range's one high entry.

## The instruments: other suites (tests/*.rs)

8 findings (0 high, 1 medium, 4 low, 3 nit). Full records: `evidence/partitions/tests-other.md`, `evidence/sweeps/api-audit.md`.

### tests-other-11: The dominance test's doc says "read strictly more"; its body asserts only inequality
- Where: crates/before/tests/coincident_span.rs:91-93 (related: crates/before/tests/coincident_span.rs:6-8, crates/before/tests/coincident_span.rs:117-129)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read); executed: no
- Seen by: structure-prose; refutation: confirmed; history: as-born in 47b03e89, whose own message records the early-exit fact ("equality, since the fused walk's dominance early-exit can legitimately read fewer bits"); the doc sentence is a copy of the place test's
- Owner-gated: no

Every test's doc comment states its invariant in English and must be accurate; its incorrectness is a bug in the test. Here the doc (and the module doc at lines 6-8) says distinct-buffer coincident endpoints "read strictly more", while the body's own comment explains the fused walk's early exit can read fewer and asserts `assert_ne!`. The module doc's "so a lost rung and a dead scan meter both read red" also overclaims for this leg: the dead meter is caught by the separate `collapsed > 0` floor at 103-106, not by the direction.

Evidence:

        91	/// `Span::dominance` on a clone-coincident span reads exactly the
        92	/// collapsed containment's scan; coincident endpoints in distinct
        93	/// buffers take the fused walk and read strictly more.
       ...
       120	    // The fused walk's dominance early-exit can read *fewer* bits than
       121	    // the collapsed containment on a refuting probe, so the walking leg
       122	    // pins divergence, not direction: distinct buffers must not read
       123	    // scan-identical to the collapsed form.
       124	    assert_ne!(
       125	        walked, collapsed,

Resolution: Re-state the test doc: "coincident endpoints in distinct buffers take the fused walk and read a different scan count (its early exit may read fewer, so only divergence is pinned)". Scope the module doc's "strictly more" to `place` and the `contains` argument rung, naming dominance as the divergence-only leg. Acceptance: every doc sentence in the file matches the assertion form beneath it.

### tests-other-1: The validation index omits every instrument in this partition but the bench-judge roster
- Where: crates/before/src/testing/validation_index.rs:1-3 (related: crates/before/tests/verdict_matrix.rs:1-9, crates/before/tests/coincident_span.rs:1-11, crates/before/tests/superlinear_tripwires.rs:1-16)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (per-stem grep over validation_index.rs: only `bench_judge_roster` matches, at line 113); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no rationale found (the index's last edit, ba124e1d on 2026-08-12, predates the verdict matrix; earlier edits were targeted re-denominations, not totality passes)
- Owner-gated: no

The index promises "every instrument that guards this crate" and sets the bar for a new instrument as "a failure class no row below already catches", but of the twelve test binaries in this partition only `bench_judge_roster` is named; the verdict matrix (a semantic instrument with a stated failure class), the coincident-rung witnesses, the two flatness criteria, and the four roster pins have no row. Principle 3's own procedure cannot be applied to instruments the map does not list.

Evidence:

         1	//! The validation index: every instrument that guards this crate, what
         2	//! failure class each one catches that the others cannot, and where it
         3	//! lives.

Resolution: Add a semantic row for the verdict matrix (its class: a kernel-local verdict inversion on adversarial shapes the law populations under-hit; the twins as adequacy), a resource row for the deep-skeleton and answer-embedded criteria, and a short paragraph on the roster pins (`_reads_superlinear`, `_reads_inverted`, doc-hidden, foreign re-export, or their successors under tests-other-13 and tests-other-16) and the coincident-rung witnesses. Acceptance: every `crates/before/tests/*.rs` binary is named in the index or covered by a genre row that names its file.

### tests-other-5: Generator and support docs disagree with their code in three small places
- Where: crates/before/tests/answer_embedded.rs:9-11 (related: crates/before/tests/answer_embedded.rs:68-69, crates/before/tests/answer_embedded.rs:77-79, crates/before/tests/support/fuzz_seed_set.rs:31-35, crates/before/tests/amp_board_smoke.rs:137-138)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); executed: no
- Seen by: instrument-correctness (answer_embedded, seed_set), instrument-correctness (amp_board_smoke isolation wording); refutation: confirmed; history: the `seed_set` doc was complete when written (bb6ea8b7 seeded only two targets) and expired when 778c89f5 and 23e46a7c added three more; the `step_by(2)` mismatch is as-born in 4398dcd4
- Owner-gated: no

Three docs state something their code does not do. `wt`'s docs say `n` forked parties tick once each, but the loop ticks every other party (`step_by(2)`), so `n/2` ticks land on `n` leaves. `seed_set`'s doc enumerates the `fuzz_decode` and `fuzz_decode_ops` seeds as if they were the corpus, while the function derives seeds for five targets. `shard_protocol_round_trips` says every judged quantity is a counter "over state each shard owns privately", but the in-process spawner runs every shard in one process against one global `PeakAlloc`; byte-identity holds because each shard resets the peak, not because the state is private. Every test doc states its invariant in English and must be accurate.

Evidence:

         9	//! - **wide-base tiny-tail** `WT(n, w)`: one `ticks(seed, 10^w)` base
        10	//!   raise over the whole id space, then `n` forked parties tick once
        11	//!   each on alternating leaves. Every leaf height and every subtree
       ...
        77	    for p in parties.iter().step_by(2) {
        78	        v.tick(p);
        79	    }

    (fuzz_seed_set.rs:31-35)
        31	/// The `fuzz_decode` seeds are canonical encodings of a small family of
        32	/// known values (the seed clock, a forked pair, split parties, a nested
        33	/// version); the `fuzz_decode_ops` seeds are decode-then-operate scripts
        34	/// in that target's framing (flavour byte, length-prefixed value bytes,
        35	/// one op per trailing byte). Deterministic: no randomness, no clocks.

    (amp_board_smoke.rs:137-138)
       137	/// byte-identity is how it asserts, since every judged quantity is a
       138	/// deterministic counter over state each shard owns privately. Three

Resolution: answer_embedded.rs:9-11 and 68-69: "then the even-indexed of `n` forked leaves tick once" (or tick every leaf and re-derive the grid). fuzz_seed_set.rs:31-35: describe the corpus by genre for all five targets, or name none. amp_board_smoke.rs:137-138: "a deterministic counter reset per shard, under one-scenario-per-process isolation". Acceptance: each doc sentence is true of the code beneath it.

### tests-other-12: "pincer" and "jaw" are an unanchored metaphor used as jargon across four roster pins
- Where: crates/before/tests/doc_hidden.rs:1-8 (related: crates/before/tests/doc_hidden.rs:49, crates/before/tests/foreign_reexport.rs:3, crates/before/tests/foreign_reexport.rs:7, crates/before/tests/foreign_reexport.rs:104, crates/before/tests/foreign_reexport.rs:126, crates/before/tests/superlinear_tripwires.rs:14, crates/before/tests/verdict_matrix.rs:1215, crates/before/surfacecheck/src/main.rs:19)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep `pincer|\bjaws?\b` over crates/before src, surfacecheck/src, and tests: only the listed sites; no definition anywhere in src); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (entered with the r141 witness commits 8409fe55 and 9daec56e and propagated by imitation)
- Owner-gated: no

"The totality pincer", "both jaws of the pincer", "the missing jaw", "the jaw that makes deleting ... a reviewable diff" appear in four test files and once in surfacecheck's main.rs, but the term is defined nowhere and "jaw" is never introduced by contrast. The metaphor rewrites as mechanism without loss: the two surface-totality checks (the rustdoc-JSON census in `surfacecheck` and the in-tree roster scan in `surface_coverage`). A reader of superlinear_tripwires.rs meets "the missing jaw" with no pincer in sight.

Evidence:

         1	//! The `#[doc(hidden)]` roster: every hidden public item is pinned by
         2	//! name, so hiding surface from the totality pincer is tamper-evident.
       ...
         7	//! named source files — a hidden public item is reachable API that both
         8	//! jaws of the pincer structurally miss. This pin closes that channel:

Resolution: Replace each use with the mechanism ("invisible to both totality checks: the rustdoc-JSON census omits hidden items and the roster scan reads only named files"), or define the term once where the two checks are described and cite that site. Acceptance: `grep -rn 'pincer\|\bjaw' crates/before` returns nothing, or every hit follows one definition site.

### tests-other-15: Prose that narrates history or restates enumerable facts
- Where: crates/before/tests/foreign_reexport.rs:12-14 (related: crates/before/tests/foreign_reexport.rs:18, crates/before/tests/foreign_reexport.rs:27, crates/before/tests/bench_judge_roster.rs:96, crates/before/tests/fold_skeleton.rs:8, crates/before/tests/fold_skeleton.rs:52, crates/before/tests/stale_state.rs:9, crates/before/tests/stale_state.rs:13-14, tools/benchjudge:138, crates/before/benches/common/sidecar.rs:38-39)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for `\b197\b` over surfacecheck/src, tools/, and .agent-notes finds no pin of the figure; tools/benchjudge:138 is `MAX_TEXT_SCALING_EXPONENT = 1.7`; stale_state.rs holds exactly four tests); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: the 197 is transcribed from 9daec56e's commit message; the dated-notes sweep d2a9d04e removed calendar dates only and never touched foreign_reexport.rs
- Owner-gated: no

Principle 5: prose speaks in the present tense, with no hand-maintained counts and no incident narration in the tree. `foreign_reexport.rs` records a past experiment with a count pinned nowhere ("Demonstrated: ... reads the same 197 items") and two git-register markers ("today: none", "empty at this tip"); `bench_judge_roster.rs:96` restates the judge's text ceiling as a literal that lives in tools/benchjudge; `fold_skeleton.rs:8` says "sixteen" beside a literal `16`; `stale_state.rs` counts its own tests three times ("Three pins", "The fourth witness", "All four").

Evidence:

        12	//! source files. Demonstrated: with `pub use bytes::Bytes;` added at
        13	//! the crate root, the surface-totality leg reads the same 197 items
        14	//! and exits clean. `pub extern crate <dep>` and a `pub type` alias of

    (bench_judge_roster.rs:96)
        96	/// The text ceiling (1.7) exists for conversion-dominated text IO only —

    (stale_state.rs:9, 13-14)
         9	//! how version-vector-style callers work. Three pins state that model.
        13	//! restored party overlaps its own descendant. The fourth witness pins
        14	//! that violation. All four are built from individually documented

Resolution: Delete the "Demonstrated: ... 197 items" sentence (the mechanism is already stated around it) and write "empty: `before` re-exports no foreign surface" for lines 18 and 27; cite the constant by name ("the judge's text ceiling, `MAX_TEXT_SCALING_EXPONENT` in tools/benchjudge") without the literal; `const FORKS: usize = 16;` used by the loop and the doc; "The version pins state that model ... A clock witness pins that violation. Every pin is built from ...". Acceptance: no numeral count of items, tests, or constants in these four files' prose that is not the name of an enforced home.

**Nits (3), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| tests-other-4 | `crates/before/tests/answer_embedded.rs:1-2` | "answer-embedded" names two different claims | Rename the file and its doc to the mechanism it attacks ("wide-base tiny-tail and wide-ladder folds", or "answer-width coupling") ... | `evidence/partitions/tests-other.md` |
| tests-other-8 | `crates/before/tests/bench_judge_roster.rs:7-8` | Register transplants and dash register across the partition | "launder" -> "misclassify as expected"; `honest` -> `untampered`/`intact`; drop "genuine(ly)/real(ly)" or state the mechanism ... | `evidence/partitions/tests-other.md` |
| api-audit-22 | `crates/before/tests/foreign_reexport.rs:12-14` | Hand-maintained item count in a test module doc | drop the number ("reads the same item count") | `evidence/sweeps/api-audit.md` |

**Cross-references.** tests-other-1 is the validation index (with surface-roster-16, fuzz-guests-pins-24, testing-oracles-29, module-graph-11). tests-other-15 and api-audit-22 both cite foreign_reexport.rs:12-14's 197 items; tests-other-15 also collects bench_judge_roster.rs:96's 1.7 (benches-examples-7 names the same constant from the bench side). tests-other-12 and surface-roster-25 are the pincer metaphor. tests-other-4's "answer-embedded" collision is Open questions 20. tests-other-11's dominance doc is the one medium here; coincident_span.rs's sibling legs are in the verification-gap class.

## The instruments: benches and examples

7 findings (1 high, 0 medium, 4 low, 2 nit). Full records: `evidence/partitions/benches-examples.md`, `evidence/sweeps/paper-fidelity.md`.

### benches-examples-18: code_study.rs cites a deleted module and essay as its reason to exist
- Where: crates/before/examples/code_study.rs:5-10 (related: crates/before/examples/code_study.rs:55-57, 316-322; crates/before/src/meter/board/family.rs:1255-1262; crates/before/src/meter/board.rs:294; crates/before/Cargo.toml:119-125; justfile:99, 258, 270; crates/before/AGENTS.md:6 (outside this partition, the same ghost); .agent-notes/2026-07-27-before-constants-frontier/before-constants-frontier.md:277-313)
- Class / severity / confidence: documentation / high / high
- Provenance: verified (`grep -rn 'mod implementation\|before::implementation\|Small values' crates/before` excluding .agent-notes hits only code_study.rs:6; `git log -S'pub mod implementation' -- crates/before/src/lib.rs` shows 67970b75 (added 2026-07-27) and 22cdfbe1 (removed 2026-08-17, "The design-essay implementation module is retired"); justfile:258 and :270 are the only `cargo doc` invocations and pass no `--examples`, so rustdoc's broken-link lint never visits example docs; `just check` (justfile:99, `--all-targets`) compiles the example but nothing runs it; `study_family_versions` is consumed by this example alone and its own doc says "Nothing on the board consumes it"); executed: no
- Seen by: scaffolding [4], adequacy [15], structure-prose [26], instrument-correctness [51]; refutation: confirmed; history: contradicts the hard rule (22cdfbe1 deleted lib.rs's pointer but not this one); the instrument's standing IS recorded, in a note only: constants-frontier §3.1 "Ruling: keep gamma. Revisit trigger: ... re-running the committed study binary at the paper's weights"
- Owner-gated: no (the prose fix; the study's disposition is an open question below)

Root AGENTS.md hard rule: nothing in the codebase refers to code that no longer exists. The module doc's stated purpose links `before::implementation` and quotes a passage ("Small values over large") that exist nowhere in the tree, and lines 55-57 call the constants "the as-run parameters of the crate docs' committed figures" though the crate docs carry no integer-code figures. With the figures gone, the only in-tree answer to "why does this exist" is the example itself (Principle 3); the ruling that keeps it lives in a note the code may not cite. The reconciliation pin at 316-322 is dark: compiled by `just check`, run by nothing.

Evidence:

         5	//! This is the instrument behind the crate docs' integer-code figures (the
         6	//! [`implementation`](before::implementation) essay's "Small values over
         7	//! large" trade): the constants below are the as-run parameters of the
         8	//! measurement quoted there. It changes nothing and asserts its own
         9	//! taxonomy exactly (the reconciliation pin below), so a re-run at other
        10	//! parameters is safe and cheap.
        55	// ─── realistic-simulation parameters ────────────────────────────────────────
        56	// Reduced relative to the `space_consumption` example's defaults; these are
        57	// the as-run parameters of the crate docs' committed figures.

    family.rs:
      1258	/// A measure-only study surface for offline payload analysis — the `code_study`
      1259	/// example re-parses these stored streams into per-class integer histograms to
      1260	/// price candidate integer codes on the adversarial corpus. Nothing on the
      1261	/// board consumes it.

Resolution: rewrite lines 5-10 and 55-57 in the present tense without the ghost: what the study measures (the two emission classes priced closed-form on the realistic and adversarial corpora), that it is the workload-side instrument for the crate's integer-code choice, and that a re-run reproduces the committed histograms. Decide its status explicitly: either a tiny-parameter smoke leg (`RUNS=1`, adversarial corpus only) so the reconciliation pin stays live, or a doc sentence and a justfile note declaring it a manual instrument. If the owner considers the gamma question closed for good, dissolve the example, its `[[example]]` entry, and `study_family_versions` together. Fix crates/before/AGENTS.md:6 ("the public `implementation` module for the design essay") in the same pass. Acceptance: `grep -rn 'before::implementation\|Small values\|committed figures' crates/before` is empty; every link in the file names an item in the tree; either a gate leg runs the walker or the doc states it is manual.
Construction: `grep -rn 'pub mod implementation' crates/before/src` returns nothing; `git show --stat 22cdfbe1 | grep implementation.rs` shows the deletion.

### benches-examples-3: Bench doc comments misstate operand ownership: `recv` borrows, `Version` is `Clone`
- Where: crates/before/benches/clock.rs:167-168 (related: crates/before/benches/clock.rs:185-187, 199-201; crates/before/benches/common/mod.rs:22-24; crates/before/src/clock.rs:473; crates/before/src/oracle/clock.rs:140; crates/before/src/version.rs:91-94)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (src/clock.rs:473 `pub fn recv(&mut self, version: &Version) -> &Version`; src/oracle/clock.rs:140 `pub fn receive(&mut self, msg: Version)`; src/version.rs:94 `#[derive(Clone, Eq)] pub struct Version` with the comment that a clone shares the buffer; benches/clock.rs:185 passes `msg` through setup by reference with no clone, while the oracle arm at 199 clones); executed: no
- Seen by: scaffolding [11], adequacy [22], structure-prose [31]; refutation: confirmed; history: the recv doc expired at dc88e755 (the call was updated to `recv`, the doc was not); the "not `Clone`" phrase was inaccurate for Version from the benches' first commit, its referent having always been Party/Clock
- Owner-gated: no

Every bench doc comment must be accurate to its body. `bench_receive` says the message "is consumed" and "clones cheaply in setup", which is true only of the oracle arm; common/mod.rs says the impl is decoded because it "is not `Clone`", which is true of Party and Clock but false of Version, and hides the actual reason decode is used for versions (each iteration gets a distinct buffer, which the ptr_eq rung makes observable).

Evidence:

       167	/// `receive`: merge an incoming message, then tick. The message is consumed; the clock is
       168	/// mutated. Both operands fresh per iteration (the message clones cheaply in setup).

    common/mod.rs:
        22	//! Generation lives outside the timed region; the benches only clone (oracle)
        23	//! or `decode` (impl, which is not `Clone`) a prebuilt template to get a fresh
        24	//! value per iteration.

Resolution: clock.rs: "The clock is mutated and rebuilt per iteration; the impl borrows the message (`recv(&Version)`), the oracle consumes a clone made in setup." common/mod.rs:22-24: "or `decode` (impl: `Party` and `Clock` are not `Clone`, and a `Version` clone shares its buffer, so decode gives each iteration a distinct one)". Acceptance: both comments match the signatures they describe; no bench doc claims Version is not Clone.

### benches-examples-7: Judge constants, measured exponents, and a hand cell count restated as literals at declaration sites
- Where: crates/before/benches/common/sidecar.rs:62-85 (related: crates/before/benches/common/sidecar.rs:36-43; crates/before/benches/board.rs:53-68, 91-96; crates/before/benches/tripwire.rs:8-14, 27-32; tools/benchjudge:121, 138, 160; tools/benchjudge-expected.json:2)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (the constants live as `MAX_WALL_SCALING_EXPONENT = 1.3` (benchjudge:121), `MAX_TEXT_SCALING_EXPONENT = 1.7` (:138), and `MIN_JUDGED_MEDIAN_NANOS` (:160); the same measured figures recur in the roster's `notes` at benchjudge-expected.json:2); executed: no
- Seen by: scaffolding [8], structure-prose [37], scaffolding [11 part]; refutation: confirmed; history: the measured exponents were placed deliberately (514cbcea, b1c07fe1) as the declared model's evidence per the house pattern of disclosing a declared model on its row face, a rationale that lives only in commit messages and the amplification note; the constant literals and the "~200 cells" count have no recorded rationale
- Owner-gated: no

Principle 5: a number that matters lives in one mechanically enforced place that prose cites by name. "general 1.3, text 1.7", "the 1.3 ceiling", and "the judge's 10 µs floor" are literals that rot when the judge's constants move; "the full surface is ~200 cells" is a hand-maintained count. The measured exponents (1.39/1.42, 1.28/1.33/1.30, "e ≈ 1.5", "e ≈ 2.0") appear both here and in the roster notes, so two homes hold one measurement. Since placing them at the declaration is a deliberate pattern, the fix is to say so at the site or to keep one home, not to delete them unexamined.

Evidence:

        73	/// (`version_display`, `clock_display`) renders binary→decimal —
        74	/// measured exponents 1.39/1.42. Inbound, the hugeleaf parse trio
        75	/// (`version_parse_trailing`, `version_parse_noncanon`,
        76	/// `clock_parse_trailing`) converts hugeleaf-width decimal literals
        77	/// decimal→binary on the way to the placed defect — measured exponents
        78	/// 1.28/1.33/1.30.
        79	/// (Both measurements: quick sampling, bench profile, against the 1.7
        80	/// text ceiling; a quadratic conversion still reads red there.)

    board.rs:
        95	/// full surface is ~200 cells — the committed windows are what keeps a

Resolution: cite `MAX_WALL_SCALING_EXPONENT`, `MAX_TEXT_SCALING_EXPONENT`, and `MIN_JUDGED_MEDIAN_NANOS` by name at sidecar.rs:38-39, board.rs:64-65, tripwire.rs:11 and :30; replace "~200 cells" with "the whole shape × operation product"; for the measured exponents, either state at sidecar.rs:62-85 that they are the declared model's evidence recorded at the declaration and drop the duplicate in the roster notes, or keep them in one home. Acceptance: `grep -n '1\.3\|1\.7\|10 µs\|~200 cells' crates/before/benches` returns only name-cites or nothing; the measured figures appear in exactly one committed place, or the declaration states why two.

### benches-examples-22: perf_probe.rs frames itself as a one-off for a past investigation and documents a feature flag that is always on
- Where: crates/before/examples/perf_probe.rs:1-9 (related: crates/before/examples/perf_probe.rs:112, 318; crates/before/Cargo.toml:49; crates/before/benches/common/mod.rs:30; .agent-notes/2026-08-04-perf-probe/README.md:15-17)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (b606f2ad's message assigns the standing role: "the probe remains the sampling profiler's op-isolation loop"; Cargo.toml:49 `before = { workspace = true, features = ["oracle", "meter"] }` is a self dev-dependency, so `oracle` is enabled for every dev target, which is why benches/common/mod.rs:30 imports `before::oracle` unconditionally under the flagless `just bench-build`); executed: no
- Seen by: scaffolding [1 part], structure-prose [27 part], [39], instrument-correctness [53 part]; refutation: confirmed; history: the file's existence is deliberate-and-holds (b606f2ad; note README:15-17); line 1's framing predates that role and contradicts it; the `--features oracle` note was moot from the start (6e69b427: "a self dev-dependency turns it on so `cargo bench` builds with the oracle exposed without needing `--features oracle`")
- Owner-gated: no (whether criterion's `--profile-time` should replace the file is an open question below)

Principle 5: "One-off profiling harness for the perf-probe investigation" is history at a declaration site, and b606f2ad gave the file a present-tense role it does not state. The `#[cfg(feature = "oracle")]` gates at 112 and 318 and the usage line's `--features oracle` describe a switch cargo's feature unification has already thrown.

Evidence:

         1	//! One-off profiling harness for the perf-probe investigation.
         2	//!
         3	//! Rebuilds the bench suite's corpus shapes through the public API,
         4	//! then spins each hot operation in its own `#[inline(never)]` loop so
         5	//! a sampling profiler attributes cycles per operation. Run with
         6	//! `--features oracle` to also print the oracle's timings for the same
         7	//! plans (ratio anchor only).
         8	//!
         9	//! Usage: cargo run -p before --profile bench --example perf_probe --features oracle [n]

    Cargo.toml (crates/before):
        49	before = { workspace = true, features = ["oracle", "meter"] }

Resolution: rewrite the module doc as what the file is ("Profiler harness: one `#[inline(never)]` loop per hot operation over the bench corpus, so a sampling profiler attributes cycles per operation; the wall-time record is the criterion suite"); drop the cfg gates and the `--features oracle` note. Acceptance: no "one-off" or "investigation" framing in the file; `cargo check -p before --example perf_probe` with no feature flags builds and the oracle loops are unconditional.

### benches-examples-26: The space-consumption results README lists six CSV columns for an eight-column file; the figure of record predates the marker-padding change to `encode().len()`
- Where: crates/before/results/space_consumption/README.md:8-9 (related: crates/before/examples/space_consumption.rs:50, 168; crates/before/scripts/plot_space_consumption.py:52-53; crates/before/src/lib.rs:329; crates/before/build.rs:93-99; crates/before/src/version.rs:1131; crates/before/src/clock.rs:827)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`head -1 space.csv` is `scenario,entities,iteration,mean_bits,std_bits,mean_bytes,std_bytes,runs`; the example writes that header at :50 and :168; the data commit 4e7f2d3d is 2026-07-27; d800957e (2026-08-06) changed the size law to `encode().len() == (encoded_bits() + 1).div_ceil(8)` and touched nothing under results/; the plot reads `mean_bytes` by name; build.rs:93-99 inlines the SVG into lib.rs:329); executed: no
- Seen by: scaffolding [10], adequacy [25]; refutation: confirmed; history: no rationale (stale since dc88e755, five hours after the README was written; survived two later README edits)
- Owner-gated: no for the column list; the re-measure is the owner's call (long-running at paper parameters)

Prose contradicting code, and a public crate-docs figure whose byte column describes a wire one revision behind (at most +1 B per stamp, about +1/8 B on average, visible only on the 4-replica curves). Measurements bind to their run.

Evidence:

         8	- `space.csv` — raw measurements (100 runs, paper parameters). Columns:
         9	  `scenario,entities,iteration,mean_bytes,std_bytes,runs`.

Resolution: fix the column list now (or point at the example's `# Output` section as the column reference). Re-run `cargo run --release --example space_consumption` and the plot at the next convenient point, or state in the results README the commit the data was collected at. Acceptance: the README's column list equals `head -1 space.csv`; the README names the data's commit; after a re-run, the 4-replica final byte means in the README table match the new CSV.

**Nits (2), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| benches-examples-23 | `crates/before/examples/space_consumption.rs:31-31` | An unescaped `\|` inside a code span splits the API-mapping table row | escape it: `clock \\|= v` | `evidence/partitions/benches-examples.md` |
| paper-fidelity-14 | `crates/before/results/space_consumption/README.md:8-9` | the results README's CSV column list omits the bit columns the file carries | update the column list or point at the example's `# Output` section | `evidence/sweeps/paper-fidelity.md` |

**Cross-references.** benches-examples-18 is the high entry of the `implementation` ghost family (see crate root). benches-examples-26 and paper-fidelity-14 are the same results README column list. benches-examples-7's judge constants recur at bench_judge_roster.rs:96 (tests-other-15). benches-examples-22's "one-off" framing is a question of the file's standing (its open question 2). benches-examples-12 (another class) is the presize A/B leg the alloc cfg comments describe.

## The instruments: fuzz targets, the fuzz-fit guest, and the wasm32 pins

14 findings (0 high, 1 medium, 10 low, 3 nit). Full records: `evidence/partitions/fuzz-guests-pins.md`, `evidence/sweeps/deps.md`, `evidence/sweeps/gate-legs.md`, `evidence/sweeps/module-graph.md`, `evidence/sweeps/prose-hygiene.md`.

### prose-hygiene-4: Opaque roster IDs PROG-5 / COV-7 in the fuzz workspace
- Where: crates/before/fuzz/Cargo.toml:1-1 (related: crates/before/fuzz/Cargo.toml:48; crates/before/fuzz/README.md:1)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn -E 'PROG-5|COV-7'` over the whole repository including `.agent-notes/`: the three sites are the only occurrences); executed: no
- Verification: confirmed; history: no-rationale-found (the roster these tags index exists nowhere in the tree, the resurrected design notes included)
- Owner-gated: no

Three comments carry tags from a roster no reader of the tree can resolve.
The invariant at line 48 is already named in plain language in the same
sentence.

Evidence:

         1	# Standalone fuzz workspace (PROG-5 / COV-7). The empty `[workspace]` table detaches
        48	# keystone invariant (COV-7) inline — an accepted value re-encodes stably and decodes

    README.md:
         1	# `before` fuzz targets (PROG-5 / COV-7)

Resolution: delete the three tags. Acceptance: `grep -rn -E '\b[A-Z]{2,4}-[0-9]+\b'`
over the fuzz workspace returns only UTF-8 and license identifiers.

### fuzz-guests-pins-2: Opaque roster IDs and a ghost test name in the fuzz manifest and README
- Where: crates/before/fuzz/Cargo.toml:1-1 (related: crates/before/fuzz/Cargo.toml:48, crates/before/fuzz/README.md:1, crates/before/fuzz/README.md:24-25, crates/before/src/clock/tests.rs:766-773)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rnE 'PROG-[0-9]|COV-[0-9]'` over crates/, justfile, tools/ hits exactly the three sites; `grep -rn h34` over crates/before hits only README.md:25; the live test is `fn decode_never_panics` at clock/tests.rs:773, whose doc at 766-767 states the `decode(b) == Ok(x) ⟹ is_normal(x)` implication the README describes); executed: no
- Seen by: scaffolding [2],[3]; adequacy [27]; structure-prose [48],[49]; instrument-correctness [65],[66]; refutation: confirmed; history: contradicts-hard-rule (the tags were born with the itc-era plan in 5a2679c9a, the plan retired in 7384d0450, and the test-name scrub 90f903c33 never touched fuzz/)
- Owner-gated: no

`PROG-5` and `COV-7` are plan tags defined nowhere in the tree, and `clock::tests::h34_decode_never_panics` names a test that does not exist; the live test is `clock::tests::decode_never_panics`. Hard rules: no opaque roster IDs in code or prose; nothing refers to code that no longer exists.

Evidence:

    (Cargo.toml:1)  # Standalone fuzz workspace (PROG-5 / COV-7). The empty `[workspace]` table detaches
    (Cargo.toml:48) # keystone invariant (COV-7) inline — an accepted value re-encodes stably and decodes
    (README.md:1)   # `before` fuzz targets (PROG-5 / COV-7)
    (README.md:24-25)
      structural `is_normal`-on-accept form of the same invariant is checked by
      the in-tree proptest `clock::tests::h34_decode_never_panics`.

Resolution: Delete the three parenthetical tags (the surrounding sentences already name the invariant in plain words) and cite `clock::tests::decode_never_panics`. Acceptance: `grep -rnE 'PROG-[0-9]|COV-[0-9]|h34_' crates/before` is empty and the cited test name resolves to a `fn`.

### fuzz-guests-pins-3: The fuzz run commands are spelled three ways, two have drifted, and the README misstates the gate's toolchain
- Where: crates/before/fuzz/Cargo.toml:3-9 (related: crates/before/fuzz/README.md:6-9, crates/before/fuzz/README.md:54-60, crates/before/fuzz/README.md:72-74, justfile:42-54, justfile:366-368, justfile:468, justfile:555-559)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (compared the three command lists; read the gate stream roster and the `fuzz-build` recipe); executed: no
- Seen by: scaffolding [4]; adequacy [27]; structure-prose [50]; refutation: confirmed, and raised the README:8-9 claim as new; history: deliberate-but-expired (the manifest header was accurate before fd5c92bf5 made seeding a named-directory contract and c6fac5c7d added `--target` to the justfile alone)
- Owner-gated: no

The manifest header's commands pass no seed directory, so following them runs unseeded (README.md:72-74 says seeds load only when named); neither the manifest nor the README passes `--target`, which justfile:42-47 explains a prebuilt cargo-fuzz needs; and README.md:8-9 says the gate does not need nightly or libFuzzer, which is false: `fuzz-build` is a gate leg (justfile:468) and runs `cargo +nightly fuzz build` (justfile:368). AGENTS.md names the justfile as the source of truth for verification; the duplicates are where drift lives, and justfile:51-52 still cross-refers to the manifest header for the smoke duration.

Evidence:

    3	# tries to build it (it needs nightly + libFuzzer). Build/run with cargo-fuzz only:
    4	#   cargo +nightly fuzz build
    5	#   cargo +nightly fuzz run fuzz_decode -- -max_total_time=20
    (README.md:8-9)
    gate never tries to build it. Fuzzing needs a nightly toolchain and libFuzzer; the gate
    does not.
    (justfile:368)     {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}

Resolution: Replace Cargo.toml:3-9 with one pointer to `just fuzz-build` / `just fuzz`; in the README keep only the crash-reproduction line the recipe does not cover and point at the recipe for the rest; correct README.md:8-9 to say the gate builds the targets on nightly and only the smoke runs at `just all` cadence; point justfile:51-52 at the README or drop the cross-reference. Acceptance: one seeded, `--target`-bearing spelling of the invocation remains (the justfile); the README's gate sentence agrees with justfile:468.

### fuzz-guests-pins-8: `fuzz_decode_ops`'s module doc and flavour-1 comment describe only flavour 0's framing
- Where: crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:11-15 (related: crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:48-60, crates/before/fuzz/Cargo.toml:68-70, crates/before/fuzz/README.md:36-39, crates/before/tests/support/fuzz_seed_set.rs:29-35, crates/before/tests/support/fuzz_seed_set.rs:290-299)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the two arms against `fuzz_seed_set.rs`'s `clock_then_msg` spelling: `[1u8, len] ++ clock_bytes ++ sibling.version().encode()`); executed: no
- Seen by: scaffolding [17]; structure-prose [46]; instrument-correctness [72]; adequacy [27] (the Cargo.toml:68 ride-along); refutation: confirmed; history: wrong from birth (5a2679c9a), kept through two later doc passes
- Owner-gated: no

The module doc says the remainder after the value chunk "is the op script (one op per byte)" and calls the framing a wire contract with the seed set; in flavour 1 the arm decodes a `Clock` from the chunk and the whole remainder as a `Version`, and its own comment at line 48 says it decodes "a Version (message)" where line 50 decodes a `Clock`. Cargo.toml:68 ("a `Clock` (or `Version`)"), README.md:36-37, and `fuzz_seed_set.rs:33-35` repeat the one-flavour description. A doc that declares itself a wire contract must state both flavours; a maintainer regenerating seeds from it would spell flavour 1 wrong.

Evidence:

    11	//! The first byte selects the value flavour, the next length-prefixed chunk is
    12	//! the value's bytes, and the remainder is the op script (one op per byte).
    13	//! This framing and the op table below are a wire contract with the committed
    14	//! seed corpus: `tests/support/fuzz_seed_set.rs` spells seeds in exactly this
    15	//! shape, so a change here means regenerating the seeds with it.
    48	        // Decode a Version (message) and exercise the version-facing ops.
    49	        _ => {
    50	            let Ok(mut clock) = Clock::decode(value_bytes) else {

Resolution: Module doc: "flavour 0: the remainder is an op script, one op per byte, over the decoded clock; flavour 1: the remainder is a `Version` message the decoded clock compares against and receives". Line 48: "Decode a Clock, then compare against and receive a Version decoded from the remainder." Mirror the two-flavour sentence in `fuzz_seed_set.rs:33-35`, Cargo.toml:68, and README.md:36-37. Acceptance: every description of the framing names both flavours and matches both `match` arms.

### fuzz-guests-pins-20: `ff_regs_reserve`'s doc narrates an incident and cites a seed path that does not exist
- Where: crates/before/fuzzfit/guest/src/lib.rs:272-279 (related: crates/before/fuzzfit/harness/tests/main.rs:1-7, crates/before/fuzzfit/harness/proptest-regressions/enforce.txt, crates/before/fuzzfit/harness/src/wasm.rs:28-47)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`ls crates/before/fuzzfit/harness/tests` shows enforce.rs, main.rs, sanity.rs; `find crates/before/fuzzfit -name '*proptest-regressions*'` returns only `harness/proptest-regressions`, holding enforce.txt); executed: no
- Seen by: scaffolding [16]; adequacy [28]; structure-prose [43]; instrument-correctness [67]; refutation: confirmed; history: deliberate-but-expired (ce3664dd9 centralized seed persistence and did not update this doc)
- Owner-gated: no

The doc narrates a false above-band flag and points at `harness/tests/enforce.proptest-regressions`; the committed seed lives at `harness/proptest-regressions/enforce.txt`. Prose speaks in the present tense: the invariant (no reallocation inside a measured window; at most one fresh slot per `put`) is the useful sentence, the history is git's, and the path is a ghost reference.

Evidence:

    272	/// Without the reservation, a measured kernel whose `put` lands on a `Vec`
    273	/// doubling boundary pays an O(file) reallocation inside its fuel window —
    274	/// register-machine bookkeeping billed to a public operation. The
    275	/// enforcement suite caught exactly that as a false above-band flag on
    276	/// `ff_party_seed` (the committed seed in
    277	/// `harness/tests/enforce.proptest-regressions` replays it); with the file
    278	/// pre-reserved to the program budget, `put` fills at most one fresh slot
    279	/// per call, O(1) forever.

Resolution: Keep the mechanism sentences (272-274 and 277-279 without the parenthetical); delete the "caught exactly that" sentence and the path; if a pointer is wanted, cite the harness constant that enforces the reserve (`REGS_RESERVE` and its const assert in `harness/src/wasm.rs`). Acceptance: the doc names no path and no past event.

### fuzz-guests-pins-21: "mint" for constructing values at five sites
- Where: crates/before/fuzzfit/guest/src/lib.rs:454-456 (related: crates/before/fuzzfit/guest/src/lib.rs:881, crates/before/fuzzfit/guest/src/lib.rs:899, crates/before/fuzzfit/guest/src/lib.rs:1592, crates/before/fuzzfit/guest/src/lib.rs:1657)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -niE '\bmint'` over the partition's ten .rs files hits only the fuzz-fit guest at 454, 455, 881, 899, 1592, 1657); executed: no
- Seen by: structure-prose [42]; instrument-correctness [71]; refutation: confirmed; history: contradicts-hard-rule (writing-style: never write "mint" for constructing a value)
- Owner-gated: no

The one vocabulary rule the brief states as an outright "never". At 881, 899, 1592, and 1657 the intended content is "the operands are borrowed and the endpoints are freshly allocated (owned)", which the plain words say better.

Evidence:

    454	/// The text door mints the clock's party from the literal; the atlas
    455	/// only replays text a staged clock rendered, so no minted party ever
    456	/// meets a live handle.

Resolution: 454-456: "The text door constructs the clock's party from the literal; ... so no such party ever meets a live handle." 881, 899, 1592, 1657: "the operands are read in place and the endpoints are freshly allocated (owned)". Acceptance: `grep -in '\bmint' crates/before/fuzzfit/guest/src/lib.rs` returns nothing.

### fuzz-guests-pins-24: The validation index has no row for the fuzz targets, the heap cap, or the wasm32 pins
- Where: crates/before/src/testing/validation_index.rs:1-3 (related: crates/before/src/testing/validation_index.rs:55, crates/before/src/testing/validation_index.rs:121, crates/before/src/testing/validation_index.rs:169, justfile:464-471)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i 'fuzz|wasm32|32-bit|heap cap|libfuzzer'` over the index hits only line 55 (laws "shared with the fuzz targets"), 121 (the fuzz-fit bands row), and 169 ("fuzz seeds" under the wire-format pins)); executed: no
- Seen by: scaffolding [11]; refutation: confirmed; history: no-rationale-found (the index was created after the fuzz targets and never gained a row; wasm32-pins landed three weeks later)
- Owner-gated: no

The index claims totality ("every instrument that guards this crate"), yet the five libFuzzer targets (hostile-byte inputs no generator produces; the heap cap as their resource side) and the wasm32 execution leg (seams that exist only under a 32-bit `usize`, which `wasm-check` compiles but never runs) have no row, although both run in the gate's stream roster.

Evidence:

    1	//! The validation index: every instrument that guards this crate, what
    2	//! failure class each one catches that the others cannot, and where it
    3	//! lives.

Resolution: Add two rows, each with its failure class as a constructible input and its recipe: the fuzz targets and their heap cap; the wasm32 pins. Acceptance: every leg in the gate's stream list (justfile:464-471) has a row naming what it alone catches.

### fuzz-guests-pins-36: One join-emit pin's doc gives a different size for the same operand than its sibling
- Where: crates/before/wasm32-pins/harness/tests/pins.rs:616-618 (related: crates/before/wasm32-pins/harness/tests/pins.rs:603, crates/before/wasm32-pins/guest/src/lib.rs:251-262, crates/before/wasm32-pins/guest/src/lib.rs:276-291)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (arithmetic from `synth_two_leaf_left`: live length `4k + 5` is 400_000_005 bits at k = 10^8, about 50 MB; the right operand at j = 2_047_483_647 is `2j + 5` = 4_094_967_299 bits, 512 MB, which is 488 MiB); executed: no
- Seen by: instrument-correctness [69]; refutation: confirmed; history: no-rationale-found (the two docs were written in different commits; "~25 MB" matches only the left leaf's gamma code, not the operand)
- Owner-gated: no

Line 603 says "~50 MB" for `synth_two_leaf_left(100_000_000)`; line 616 says "~25 MB" for the same operand, and "~488 MB" where the unit is MiB. The project holds test doc comments to correctness ("their incorrectness is a bug in the test").

Evidence:

    616	/// A join of two valid operands (~25 MB and ~488 MB) emits an output of
    617	/// 4294967299 live bits — 536870913 finished bytes, one byte past the
    618	/// 2^29-byte coordinate where a 32-bit `usize` runs out of bit positions.

Resolution: "~50 MB and ~512 MB" (or "~48 MiB and ~488 MiB"). Acceptance: the two join-emit docs quote the same size for the same operand in the same unit.

### deps-11: the fuzz workspace manifest and README carry opaque roster IDs (PROG-5 / COV-7) and restate build commands the justfile supersedes
- Where: crates/before/fuzz/Cargo.toml:1-9 (related: crates/before/fuzz/Cargo.toml:47-48, crates/before/fuzz/README.md:1 and 14 and 55-60, justfile:42-54 and 368 and 555-559)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep of `(PROG|COV)-[0-9]+` across the tree including .agent-notes: exactly three sites, all in the fuzz workspace, and no definition anywhere; read fuzz/Cargo.toml and fuzz/README.md in full); executed: no
- Verification: confirmed; history: no-rationale-found (the IDs resolve to nothing, including the agent notes)
- Owner-gated: no

The manifest header and README title name `PROG-5 / COV-7`, and Cargo.toml
calls the round-trip property "the keystone invariant (COV-7)"; the tags
resolve to nothing in the tree or the notes. The same header and the README's
Run section give `cargo +nightly fuzz build` / `fuzz run` commands that omit
the `--target {{ host_triple }}` the justfile explains is required with a
prebuilt cargo-fuzz and use floating `nightly` where the recipes use the
dated pin; justfile:51-52 in turn hand-syncs the 20-second smoke default
"with the guidance in crates/before/fuzz/Cargo.toml".

Evidence:

    crates/before/fuzz/Cargo.toml
         1	# Standalone fuzz workspace (PROG-5 / COV-7). The empty `[workspace]` table detaches
         ...
         4	#   cargo +nightly fuzz build
         5	#   cargo +nightly fuzz run fuzz_decode -- -max_total_time=20
         ...
        47	# Decode-only target: feed arbitrary bytes to every top-level `decode`. Asserts the
        48	# keystone invariant (COV-7) inline — an accepted value re-encodes stably and decodes
    crates/before/fuzz/README.md
         1	# `before` fuzz targets (PROG-5 / COV-7)
        14	rustup toolchain install nightly
        55	cargo +nightly fuzz build   # build all targets
    justfile
        51	# Default fuzz smoke duration per target, in seconds (matches the guidance in
        52	# crates/before/fuzz/Cargo.toml).
       368	    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}

No opaque roster IDs (they outlive their roster; these already have) and no
restated enumerable facts: the invocation of record is `just fuzz-build` /
`just fuzz`, and both prose copies have diverged from it.

Resolution: drop the IDs and name the property ("the decode round-trip
invariant: an accepted value re-encodes stably and decodes back to itself");
replace the command listings in Cargo.toml:3-9 and README:50-64 with a
pointer to the two recipes; let the README's prerequisites name the pinned
nightly via the justfile rather than `rustup toolchain install nightly`; drop
the justfile:51-52 cross-reference once the manifest no longer carries the
number. Acceptance: grep of `(PROG|COV)-[0-9]+` over the tree is empty; the
fuzz workspace's prose names no cargo-fuzz command line.

### gate-legs-11: The fuzz workspace's prose carries opaque roster tags, floating-nightly instructions, a ghost test name, and a hand-duplicated duration
- Where: crates/before/fuzz/Cargo.toml:1-9 (related: crates/before/fuzz/Cargo.toml:48, crates/before/fuzz/README.md:1, crates/before/fuzz/README.md:13-16, crates/before/fuzz/README.md:25, crates/before/fuzz/README.md:54-60, crates/before/src/clock/tests.rs:772-773, justfile:51-54)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both files in full; grep for `(COV|PROG)-[0-9]+` across the in-scope crates, tools, justfile, and .cargo finds only these three sites; grep for `h34` across crates/before finds only README.md:25, while the live test is `decode_never_panics` at clock/tests.rs:773); executed: no
- Verification: confirmed, with one ghost reference added; history: no-rationale-found
- Owner-gated: no

The fuzz manifest and README carry "PROG-5 / COV-7" roster tags, instruct `cargo +nightly fuzz build` and `rustup toolchain install nightly` while the recipes of record use the dated `nightly_toolchain` and `--target {{ host_triple }}`, and the README names a test `clock::tests::h34_decode_never_panics` that does not exist (the live test is `clock::tests::decode_never_panics`), which breaks the root AGENTS.md hard rule against references to code that no longer exists. The justfile also restates the 20 s smoke duration by hand as matching the manifest's guidance.

Evidence:

         1	# Standalone fuzz workspace (PROG-5 / COV-7). The empty `[workspace]` table detaches
         4	#   cargo +nightly fuzz build
        48	# keystone invariant (COV-7) inline — an accepted value re-encodes stably and decodes

    crates/before/fuzz/README.md
         1	# `before` fuzz targets (PROG-5 / COV-7)
        14	rustup toolchain install nightly
        25	  the in-tree proptest `clock::tests::h34_decode_never_panics`.
        55	cargo +nightly fuzz build   # build all targets

    crates/before/src/clock/tests.rs
       772	    #[test]
       773	    fn decode_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..512)) {

    justfile
        51	# Default fuzz smoke duration per target, in seconds (matches the guidance in
        52	# crates/before/fuzz/Cargo.toml).

Resolution: Delete the tags; fix the test name to `clock::tests::decode_never_panics`; replace the manual command lists with `just fuzz-build` / `just fuzz` (keeping the seed-corpus explanation); drop the duration duplication from the justfile comment or make the manifest defer to the recipe. Acceptance: no `PROG-`/`COV-` tag remains in the in-scope tree, every test name in the fuzz README resolves against `cargo nextest list -p before --all-features`, and the fuzz prose names the dated toolchain or the recipe rather than `+nightly`.

### module-graph-6: The fuzz workspace's manifest and README carry opaque roster IDs and a stale gate claim
- Where: crates/before/fuzz/Cargo.toml:1-3 (related: crates/before/fuzz/Cargo.toml:47-49, crates/before/fuzz/README.md:1, crates/before/fuzz/README.md:6-8, justfile:363-368, justfile:468, crates/before/AGENTS.md:14-15, crates/before/fuzzfit/Cargo.toml:1-5, crates/before/surfacecheck/Cargo.toml:1-6)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the manifest header, README head, the gate-streams recipe, and the crate AGENTS.md; `grep -rn 'COV-[0-9]\|PROG-[0-9]'` over crates/, justfile, tools, .cargo finds only these two files); executed: no
- Verification: confirmed, and the README added as a second site with the same IDs and the same claim; history: no-rationale-found.
- Owner-gated: no

"PROG-5" and "COV-7" resolve to nothing in the tree. "the before clippy/nextest gate never tries to
build it" is true of those two legs and false of the gate: `just gate` runs `fuzz-build` in its
fuzz stream, and the crate's AGENTS.md says so.

Evidence:

         1	# Standalone fuzz workspace (PROG-5 / COV-7). The empty `[workspace]` table detaches
         2	# this crate from the parent `rumors` workspace, so the before clippy/nextest gate never
         3	# tries to build it (it needs nightly + libFuzzer). Build/run with cargo-fuzz only:
    --- fuzz/Cargo.toml:48 ---
        48	# keystone invariant (COV-7) inline — an accepted value re-encodes stably and decodes
    --- fuzz/README.md:1 ---
         1	# `before` fuzz targets (PROG-5 / COV-7)
    --- justfile:468 ---
       468	    start_stream fuzz         10 fuzz-build
    --- crates/before/AGENTS.md:14-15 ---
        14	`just gate` before every commit (it compiles this crate's fuzz targets, which
        15	no workspace-wide build reaches), `just all` for the full sweep (this crate's

Resolution: Rewrite both headers in the surfacecheck manifest's form: detached so workspace-wide
cargo invocations never compile it; the gate reaches it through `just fuzz-build` (and `just fuzz`
at `all` cadence); drop the ID tags and keep the invariant in words (line 48 already spells it
out). Acceptance: no roster ID remains in the tree; the header names the recipes that reach it.

**Nits (3), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| fuzz-guests-pins-7 | `crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:296-296` | Em-dashes inside `//` comments at six sites | Batch into a crate-wide sweep rather than fixing these six alone; per site, a colon, semicolon, or parenthetical | `evidence/partitions/fuzz-guests-pins.md` |
| fuzz-guests-pins-17 | `crates/before/fuzzfit/guest/src/lib.rs:84-87` | Moralized qualifiers ("honest", "real") at ten sites | Batch with the crate-wide prose pass; substitute the property at each site | `evidence/partitions/fuzz-guests-pins.md` |
| fuzz-guests-pins-23 | `crates/before/fuzzfit/guest/src/lib.rs:987-988` | `ff_party_forks`'s doc says the kernel "replaces `src`"; it mutates `src` in place | "(the source in `src` keeps its remainder share; the iterator borrows it)" | `evidence/partitions/fuzz-guests-pins.md` |

**Cross-references.** fuzz-guests-pins-2, deps-11, gate-legs-11, module-graph-6, and prose-hygiene-4 are the PROG-5/COV-7 tags and the fuzz manifest's stale commands, reported by one partition and four sweeps; gate-legs-11 and fuzz-guests-pins-2 add the ghost `h34_` test name; module-graph-6 adds the false "the gate never tries to build it" claim (fuzz-guests-pins-3 has the README's version of it). fuzz-guests-pins-20 and fuzzfit-bands-20 are the same stale seed path from the guest and harness sides. fuzz-guests-pins-24 is the validation index. fuzz-guests-pins-33 and -35 (other classes) are the memory-terminal pins whose genre the prose leaves unsettled.

## The instruments: fuzzfit (the harness: bands, fit, curve, wasm, strategies, ops, drive)

14 findings (0 high, 2 medium, 8 low, 4 nit). Full records: `evidence/partitions/fuzzfit-bands.md`, `evidence/partitions/fuzzfit-strategies.md`.

### fuzzfit-bands-2: Pin-time measurements hand-transcribed into prose have rotted; the judgment constants' evidence is printed, never committed or asserted
- Where: crates/before/fuzzfit/harness/src/bands.rs:84-89 (related: bands.rs:74-79, bands.rs:98-100, bands.rs:106-107, bands.rs:116, bands.rs:161-173, bands.rs:175-194, bands.rs:207-218, crates/before/fuzzfit/harness/src/curve.rs:61-71, crates/before/fuzzfit/harness/src/bin/calibrate.rs:218-343, crates/before/fuzzfit/harness/src/bin/calibrate.rs:345-353)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git show e7a4b7b0 -- bands.rs`: every hunk sits at line 318 or below and the decode small band moves 3.684848 to 3.705965; `git blame` dates bands.rs:84-89 to d2a9d04e 2026-07-31 and bands.rs:100 to 875c118b 2026-07-27; Python over the parsed constants recomputes the ff_rank_cmp floor gap as 0.1155 at HEAD with nop = 2 and 0.1134 at the d2a9d04e constants); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-and-holds for the current-pin head design (d2a9d04e), with the truing discipline lapsed at e7a4b7b0
- Owner-gated: no

`bin/calibrate` rewrites `bands.rs` from the `/// The toolchain that pinned` marker down and preserves everything above it verbatim, while printing the judgment constants' evidence only to stderr (calibrate.rs:218). The head therefore carries numbers the generator cannot refresh, and they are stale: the decode small band's transcription contradicts `SMALL_BANDS`; the rejection-envelope range 1.36–1.42 excludes `ff_clock_sync`'s pinned rejection slope 1.328458 (and did at the prior pin too, 1.328893); `party_without`'s success slope reads 0.72 in prose and 0.713628 in the pin; the shape-leg maximum healthy excess is +0.081 in bands.rs, +0.013 in curve.rs, and +0.006 in the design note; the corpus size is ~2.64M steps here and ~2.62M in curve.rs; the "narrowest gap is 0.113 decades" at bands.rs:181 recomputes to 0.1155 from the committed `ff_rank_cmp` constants. Principle 5 (prose states what is; no hand-maintained restatements of facts the code can change) and Principle 2 (a quantity computable two ways gets a committed comparison): the four judgment constants each justify their value by a number that exists only on stderr and in a hand copy, and nothing in the gate fails when copy and measurement diverge.

Evidence:

        84	//! [`SMALL_BANDS`] carries one constant-classified band per bootstrap-hot
        85	//! kernel, calibrated from the main corpus's own sub-floor samples pooled
        86	//! with the deterministic bootstrap stream (tick level 4.476 +0.782/−0.492
        87	//! over 10..127 bits, join 4.454 +0.395/−0.765 over 20..127, encode
        88	//! 2.729 +0.304/−0.139 over 10..127, decode 3.685 +0.683/−0.308 over
        89	//! 16..120).

    versus the generated decode small band:

       966	        slope: 0.000000,
       967	        intercept: 3.705965,
       968	        width_above: 0.666431,
       969	        width_below: 0.330484,

    the shape-leg maximum, three ways (bands.rs:99-100, curve.rs:67-68):

        99	//! 128-bit fit floor — the within-case shape diagnostic's maximum healthy
       100	//! excess over the whole corpus is +0.081 (see [`crate::curve`]) — while

        67	/// evidence-bearing (band key, case) pair was +0.013 (`ff_clock_join`,
        68	/// a deep `DenseSpine` draw). The allowance sits well above that observed

    the rejection envelope claim versus ff_clock_sync's pinned rejection arm (bands.rs:106, 475):

       106	//! ground-truth view). The rejection envelopes (1.36–1.42:

       475	        slope: 1.328458,

    and calibrate's own statement of the design:

       218	    // ── judgment-constant evidence (stderr; never part of the pin file) ──

Resolution: Have `calibrate` emit the evidence it already computes into the generated region as one constant, e.g. `pub const PIN_EVIDENCE: PinEvidence` with fields `corpus_programs`, `corpus_steps`, `nop_fuel`, `shape_max_excess` (key, case), `refit_max_divergence` (key), `floor_min_gap` (key), `replay_ceiling_excess` (key, depth), and the uncovered keys with their reasons. Make the docs of `ENFORCE_MARGIN`, `ENFORCE_MARGIN_BELOW`, `REFIT_TOLERANCE`, and `SLOPE_ALLOWANCE` cite those fields by name instead of by number, and delete the numeric narrative from the head (the small-band numbers duplicate `SMALL_BANDS` outright; the slope-by-slope commentary at bands.rs:98-125 belongs in the re-pin commit, as bands.rs:127 itself says). Add one enforcement test asserting the orderings the docs claim: `ENFORCE_MARGIN > PIN_EVIDENCE.replay_ceiling_excess`, `PIN_EVIDENCE.floor_min_gap > 0.0`, `SLOPE_ALLOWANCE > PIN_EVIDENCE.shape_max_excess`, `REFIT_TOLERANCE > PIN_EVIDENCE.refit_max_divergence`. Acceptance: after `just fuzzfit-calibrate` at HEAD, no decimal literal above the splice marker duplicates a value the generated region or calibrate's stderr carries; the ordering test exists and passes; grep finds one shape-leg maximum in the tree, generated.

### fuzzfit-strategies-6: `ops.rs` claims one op per public operation and a one-to-one guest mirror, and asserts `Rank` has no packed codec
- Where: crates/before/fuzzfit/harness/src/ops.rs:3-4 (related: crates/before/fuzzfit/harness/src/ops.rs:147-149, crates/before/fuzzfit/harness/src/lib.rs:4-5, crates/before/fuzzfit/harness/tests/enforce.rs:441-443, crates/before/fuzzfit/harness/src/strategies.rs:41-62, crates/before/src/version/rank.rs:395-461, crates/before/fuzzfit/guest/src/lib.rs:1404-1425)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (comm of `pub extern "C" fn ff_*` names in guest/src/lib.rs (107) against the `=> "ff_*"` literals in ops.rs (44): every Op kernel has an export, 63 exports have no Op; `grep` in rank.rs shows `pub fn encode` at 395, `pub fn decode` at 461, and no `FromStr for Rank`; `git log -S` dates the comment to c1fe9388 (2026-07-26) and the codec to f0f3a2ae (2026-07-29)); executed: yes: the comm and greps above settle the counts and the codec's existence
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (true when written: at c1fe9388 the guest had 51 exports, 44 measured plus 7 control; expired at 88a62a37 for the one-to-one claim and f0f3a2ae for the codec claim; neither expiry touched ops.rs)
- Owner-gated: no: whatever the scope decision (finding 7), the prose must not claim what is false

The module doc says the vocabulary has one op per public `before` operation and mirrors the guest ABI one-to-one; the guest exports 107 kernels and `Op::kernel` names 44, and `Clock`'s `Display`/`FromStr`, `Rank::encode`/`decode`, `Clock::ticks`/`Version::ticks`, the `*_all` doors, `Version::span`, `Span`, `Ranked`, and `causally` have no `Op`. The `RankDisplay` doc goes further and asserts `Rank` has no packed codec, which rank.rs contradicts and the guest's own `ff_rank_encode`/`ff_rank_decode` exports contradict; the consequence propagates: the mirror snapshots ranks as text and denominates them by rendering length where a canonical packed form exists (Principle 5: prose states what IS; lib.rs:4-5 and enforce.rs:441 inherit the overclaim, and the scope section at strategies.rs:41-62 lists three deliberate exclusions and none of these surfaces, so a reader cannot learn the omission is deliberate).

Evidence:

         3	//! A *program* is a sequence of [`Op`]s over a register file, one op per
         4	//! public `before` operation, mirroring the guest ABI one-to-one. Programs

       147	    /// `Rank` `Display` into the stage (the rank's only text direction:
       148	    /// `Rank` has no `FromStr` and no packed codec).
       149	    RankDisplay { src: Reg },

    rank.rs:395	    pub fn encode(&self) -> Vec<u8> {
    rank.rs:461	    pub fn decode<R: io::Read>(mut reader: R) -> Result<Rank, Decode> {

    lib.rs:4	//! The crate's asymptotic claims (every public operation amortized linear in
    lib.rs:5	//! its denominated size) are guarded elsewhere by chosen adversarial families

Resolution: rewrite ops.rs:3-4 to state the actual relation ("one op per operation the fuzz-fit bands price; each op calls one guest kernel by name; the guest additionally exports the kernels the fuelscape atlas measures, which no strategy reaches"); delete "and no packed codec" at 148 (or add `RankEncode`/`RankDecode` under finding 7 and snapshot ranks through `encode()`); reword lib.rs:4-5 and enforce.rs:441 to name the priced subset; extend the scope section at strategies.rs:41-62 with the omitted surfaces and the reason. Acceptance: ops.rs:3-4 and 147-148 make no claim rank.rs or the guest's export list contradicts, and the scope section names every public surface the vocabulary omits.

### fuzzfit-bands-20: The seed-location sentence is a ghost of the pre-anchor layout
- Where: crates/before/fuzzfit/harness/tests/enforce.rs:12-15 (related: crates/before/fuzzfit/harness/tests/main.rs:1-7, crates/before/fuzzfit/harness/proptest-regressions/enforce.txt, crates/before/fuzzfit/guest/src/lib.rs:276-277)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`ls harness/tests` shows enforce.rs, main.rs, sanity.rs; the only seed file is harness/proptest-regressions/enforce.txt with six entries; `git show --stat ce3664dd` adds tests/main.rs; `git blame` dates enforce.rs:13 to c1fe9388 2026-07-26; grep finds the stale `harness/tests/enforce.proptest-regressions` path at guest/src/lib.rs:277); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (ce3664dd moved the seed and added the anchor without updating either sentence)
- Owner-gated: no

Since ce3664dd introduced `tests/main.rs` as the persistence anchor, seeds resolve to `proptest-regressions/enforce.txt` at the package root, not beside the test binary; the doc still describes the sibling-file layout, and the guest's `ff_regs_reserve` doc (outside this partition, but pointing at this partition's artifact) names a path that does not exist. Principle 5 and the root AGENTS.md hard rule: no references to things that no longer exist; a maintainer following either sentence to check that a seed is committed lands nowhere.

Evidence:

        12	//! determinism makes a failure replay exactly: proptest shrinks to a
        13	//! minimal out-of-band shape and writes a seed file next to this binary —
        14	//! commit any seed that appears (repo hard rule); it is an
        15	//! out-of-band-shape finding of record.

Resolution: "writes a seed to `proptest-regressions/enforce.txt` at the package root (the `tests/main.rs` anchor)"; fix guest/src/lib.rs:276-277 to `harness/proptest-regressions/enforce.txt`. Acceptance: `grep -rn 'next to this binary\|enforce.proptest-regressions' crates/before/fuzzfit` is empty; both sentences name the path `tests/seed_liveness.rs` resolves.

### fuzzfit-bands-21: The `PROPTEST_CASES` override the suite documents is discarded by `ProptestConfig::with_cases`
- Where: crates/before/fuzzfit/harness/tests/enforce.rs:35-36 (related: tests/enforce.rs:438-439, crates/before/fuzzfit/harness/tests/sanity.rs:13-14, crates/before/fuzzfit/harness/src/strategies.rs:86-88, justfile:582)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (proptest-1.11.0 `src/test_runner/config.rs:456-461`: `pub fn with_cases(cases: u32) -> Self { Self { cases, ..Config::default() } }`; `Config::default()` clones `DEFAULT_CONFIG` (591-594), which applies `contextualize_config` (191-195) reading `PROPTEST_CASES` (20-26); the struct update then overwrites `cases` with 48; the harness Cargo.lock pins proptest 1.11.0); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: no rationale found (both the sentence and the config date from 7fb3b5ce)
- Owner-gated: no

The environment-derived default is evaluated first and then overwritten, so the sentry always runs exactly 48 cases and the module doc promises a parameter that does nothing. A re-pinner following the doc to widen a local run before a re-pin gets 48 cases and no warning. Principle 8: a false operational claim in the file's first paragraph.

Evidence:

        35	//! Case count: 48 by default (the calibration corpus is the big sweep; this
        36	//! is the sentry); override with `PROPTEST_CASES`.

       439	    #![proptest_config(ProptestConfig::with_cases(48))]

Resolution: Either delete the override sentence (the justfile already states 48), or honor it: `ProptestConfig { cases: std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(SENTRY_CASES), ..ProptestConfig::default() }` with `const SENTRY_CASES: u32 = 48` (which also anchors the other hand-written 48s in this file; see finding 26). Acceptance: the doc describes what the config does; if an override is kept, `PROPTEST_CASES=4 PROPTEST_VERBOSE=1 cargo nextest run ... fuel_stays_in_the_pinned_bands` reports 4 cases.
Construction: Run the sentry with `PROPTEST_CASES=1 PROPTEST_VERBOSE=1` in the fuzzfit workspace: 48 cases execute.

### fuzzfit-bands-24: "the demonstrations ledger" resolves to nothing in the tree; the decision to keep the reach demonstrations in git history is recorded only in the design note
- Where: crates/before/fuzzfit/harness/tests/enforce.rs:268-271 (related: tests/enforce.rs:398-436, .agent-notes/2026-07-26-before-fuzzfit-asymptotics/before-fuzzfit-asymptotics.md:348-354 and 441-463, crates/before/src/testing/validation_index.rs:100-107)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for the phrase across crates/before and .agent-notes finds it only here; the design note §8 records that each of the five genre demonstrations "is a dated record in git history at its pin commit" and lists the standing demonstrations that run every suite); executed: no
- Seen by: scaffolding (as a medium verification-gap); refutation: confirmed; history: already-known (the note records the design: demonstrations in git history, the defenses they forced standing in the tree; the phrase originally pointed at a design document since deleted and resurrected under .agent-notes)
- Owner-gated: no for the doc fix; committing real-kernel sabotage demonstrations would reopen the note's §8 design and is an open question below

The burner test's doc hands responsibility for generator reach to "the demonstrations ledger", a thing a reader cannot find: the phrase names the design note's §8, which is a design-document citation from rustdoc (a hard rule) and, after e13854de's deletion and the .agent-notes resurrection, resolves only through LLM-written notes. The owner's decision (the five known-bad reconstructions live in git history at their pin commits; what stands in the tree is the synthetic tripwires, the escalation replays, `REFIT_COVERAGE`, and the seeds) is a recorded ruling, so the verification-gap the lens raised converts to: state the decision where the doc points at it. Principle 5.

Evidence:

       268	    /// proves the wasm-execution → fuel-metering → judgment path can flag a
       269	    /// quadratic at all; whether the *generators* place real kernels where a
       270	    /// regression must flag is the reach families' and the demonstrations
       271	    /// ledger's business, not this check's.

Resolution: Replace "the demonstrations ledger's business" with the decision in the tree's own terms: "the reach families' and the escalation replays' business; the known-bad reconstructions that accepted each reach genre are recorded at their pin commits, and the defenses they forced (`ESCALATION_REPLAYS`, the outcome-keyed bands, `REFIT_COVERAGE`, the committed seeds) are what stand here". Acceptance: the phrase names tree artifacts or git history explicitly; no rustdoc in the partition points at a design document.

### fuzzfit-bands-26: Hand-maintained counts and derived probabilities across the harness prose
- Where: crates/before/fuzzfit/harness/tests/enforce.rs:402-410 (related: tests/enforce.rs:21, tests/enforce.rs:35-36, tests/enforce.rs:338-339, tests/enforce.rs:424-425, crates/before/fuzzfit/harness/src/wasm.rs:34-36, wasm.rs:55-56, crates/before/fuzzfit/harness/src/bin/calibrate.rs:62-65, crates/before/fuzzfit/harness/src/bands.rs:74-79, crates/before/fuzzfit/harness/src/curve.rs:64-65, crates/before/fuzzfit/harness/src/strategies.rs:1975-1978)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (recomputed 137 from `any_family`'s `prop_oneof!` weights at strategies.rs:1980-2015: seventeen `8 =>` arms plus one `1 =>` arm; 48 · 8 / 137 = 2.8; 20,048 = 2 · 9000 + 2048 from `ESCALATION_BUDGET` at strategies.rs:106-111; the const assertion at wasm.rs:41-47 already enforces the register bound); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: no rationale found
- Owner-gated: no

Prose restates values the code computes or constants determine: the sentry's 48 (lines 21, 35, 338-339 versus the literal at 439), the escalation draw odds ("once in 137", from seventeen families at weight 8 plus one at weight 1), the independent-regime rate ("nearly three times per default run"), "the seven single-operand rows", the register-reserve bound (wasm.rs:35-36 "20,048", beside the const assertion that enforces it), a host core count, and the corpus size 4096 as a bare `unwrap_or(4096)` named in prose at bands.rs:74, 181, 211 and curve.rs:64. Each is accurate today and rots the moment its source changes (Principle 5: state the structure, not the tally; a number that matters lives in a mechanically enforced place prose may cite by name).

Evidence:

       402	/// The sentry's random draws pick the escalation family about once in 137
       403	/// cases, so a 48-case run usually never leaves the small-operand regime —
       404	/// and an instrument whose deep reach is exercised only by rare draws has
       405	/// no standing proof its at-scale bands (the seven single-operand rows,
       406	/// the rejection arms, the deep-overlap scans) still bite. This replay is

        34	/// share count is capped by its fork budget — so no program allocates
        35	/// more than `2 · max_ops + max_forks` slots (20,048 under the larger,
        36	/// escalation budget).

Resolution: Name the constants (`pub const SENTRY_CASES: u32 = 48`, `pub const CORPUS_OF_RECORD: usize = 4096` with `calibrate` defaulting to it) and cite them by name; replace "once in 137" with the structure ("Escalation carries weight 1 against 8 for every other family"); drop the 20,048 parenthetical (the const assertion is the statement) and the host count; replace "the seven single-operand rows" with the structural description. Acceptance: changing `any_family`'s weights, `ESCALATION_BUDGET`, or the sentry case count leaves no numeral in prose to update; `grep -rn '137\|20,048\|seven single' harness/` is empty.

### fuzzfit-strategies-3: The identity-routing argument is stated in full three times and overstates the canonical-equality rung as O(1)
- Where: crates/before/fuzzfit/harness/src/drive.rs:57-66 (related: crates/before/fuzzfit/harness/src/ops.rs:289-300, crates/before/fuzzfit/harness/src/ops.rs:438-441, crates/before/fuzzfit/harness/src/bands.rs:91-96, crates/before/src/codec/bits.rs:414-420, crates/before/src/version.rs:384-395)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for `decoration-wide` finds exactly ops.rs:297, drive.rs:62, bands.rs:95; read `canonical_eq` at bits.rs:419 and before's own wording at version.rs:387-388); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed (both halves); history: no-rationale-found (all three copies and the "O(1) by mechanism" phrase landed together in 306e2de0, whose message carries the paragraph a fourth time)
- Owner-gated: no

The same paragraph (identity steps are O(1) by mechanism, fitting them smears both bands, liveness is owned by `identity_fast_paths`) appears in the driver, on `Step::identity`, and in bands.rs's module doc, and ops.rs:297 points at the driver while the driver carries a copy (writing-style rule: state a constraint once where the decision is made and cite it; three copies drift, and bands.rs already says "would smear" where the others say "makes"). The wording also overreaches: for join, meet, distance, and lag the mirrored predicate is `canonical_eq`, which is `ptr_eq || as_raw_slice() == as_raw_slice()`, so byte-equal operands in distinct buffers pay a linear memcmp, not O(1); the exclusion is still right (the memcmp is not the walk's size law and `distinct_buffers_keep_the_walked_paths_covered` owns it), but a maintainer reading "O(1) by mechanism" would reject a future check on the excluded steps' cost as unnecessary.

Evidence:

        57	        // Identity-outcome steps (operands dispatching an identity-law
        58	        // fast path: one clone-shared buffer under a comparison, equal
        59	        // versions under a metric) are measured for the differential but
        60	        // never sampled. Their cost is O(1) by mechanism, not a size
        61	        // law, and fitting them alongside the walked cloud makes both
        62	        // bands decoration-wide; their liveness has its own instrument

    ops.rs:294	    /// Identity steps are measured for the differential but never
    ops.rs:295	    /// sampled: their cost is `O(1)` by mechanism, not a size law, and
    ops.rs:296	    /// fitting them alongside the walked cloud makes both bands
    ops.rs:297	    /// decoration-wide (the driver's sampling carries the argument).

    bits.rs:419	    a.ptr_eq(b) || a.as_raw_slice() == b.as_raw_slice()

    version.rs:387	        // and canonical equality answers in `O(1)` on a shared buffer
    version.rs:388	        // (clone identity) or one byte compare, where the fused sweep

Resolution: keep the full argument on `Step::identity` (the predicate is defined there), reworded to "settled by an equality rung (clone identity or one byte compare), not by the walk whose size law the band fits"; reduce drive.rs:57-66 to one line citing `Step::identity` and bands.rs:91-96 to one sentence with a link; drop the "the driver's sampling carries the argument" pointer. Acceptance: `grep -rn 'decoration-wide\|walked cloud' harness/src` returns one site and no site says O(1) of the byte-compare rung.

### fuzzfit-strategies-15: Vocabulary: a moralized bound, unanchored coinages, and four words carrying two meanings
- Where: crates/before/fuzzfit/harness/src/strategies.rs:34-37 (related: crates/before/fuzzfit/harness/src/strategies.rs:48, crates/before/fuzzfit/harness/src/strategies.rs:87, crates/before/fuzzfit/harness/src/strategies.rs:99, crates/before/fuzzfit/harness/src/strategies.rs:133, crates/before/fuzzfit/harness/src/strategies.rs:170-174, crates/before/fuzzfit/harness/src/strategies.rs:1535, crates/before/fuzzfit/harness/src/strategies.rs:1550, crates/before/fuzzfit/harness/src/strategies.rs:1562-1563, crates/before/fuzzfit/harness/src/strategies.rs:1880, crates/before/fuzzfit/harness/src/ops.rs:297, crates/before/fuzzfit/harness/src/ops.rs:305, crates/before/fuzzfit/harness/src/drive.rs:61, crates/before/src/meter/registry.rs:588-663)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (each term read at its line; `FamilyId` variants read at registry.rs:588-663); executed: no
- Seen by: structure-prose, instrument-correctness (the "honesty bound" sentence); refutation: confirmed, with two items dropped (`organic` is crate-wide vocabulary at laws.rs:11; `divert` is the meter board's own flag name at registry.rs:598); history: no-rationale-found
- Owner-gated: no

In-partition tells: "honesty bound" (36) moralizes a mechanism and the sentence it heads is imprecise (a program's total denominated work is bounded by `max_ops` times a constant fixed by the tick, fork, and fold caps, which is "within a constant of its op budget" only with that dependence named); "decoration-wide" (ops.rs:297, drive.rs:61) promotes the doctrine's "decoration" to an adjective; "mongrel clock" (1880) is texture for "a clock assembled from two universes"; "the enforcement sentry" (87) is anchored only by enforce.rs's module doc; "envelope" (48) means "reachable region" here and "pinned counter ceiling" in tests/meter.rs. Four words carry two senses inside the harness: "rung" is before's fast-path step in ops.rs (305, 610, 623, 650, 672, 683) and a ladder step in strategies.rs (133, 1535, 1562-1563); "battery" is `B::battery` and the escalation arm's "cadence battery" (99, 1550); "mirror" is `Mirror` and the board's `MirrorNarrow`/`MirrorWide` families (174); "ladder" names five distinct sequences in the escalation arm. The `Family` doc says the names map onto the meter board's roster (172), but `DenseSpine`/`BigRoot`/`HugeLeaf`/`CliffComb`/`IdPairLockstep`/`ScatterFold`/`WideTail` differ from `Dense`/`Bigroot`/`Hugeleaf`/`Cliff`/`IdPair`/`Scatter`/`MirrorWide`, so the mapping is not greppable (every coined term must be an identifier or defined once by contrast where introduced).

Evidence:

        34	//! (packed growth per public op is amortized constant per tick/fork), which
        35	//! keeps iterated joins from compounding exponentially and doubles as the
        36	//! honesty bound for composed cases: a program's total denominated work is
        37	//! within a constant of its op budget. Most families run under [`BUDGET`];

       172	/// The names map onto the meter board's family roster; the board's control
       173	/// variants ride as parameters (`hifloor`, `plateau`, `tail_ticks = 1` for
       174	/// the narrow mirror cross).

      1880	                        // mongrel clock — meaningless as a value, but the

Resolution: 34-37: "and bounds a composed case's total denominated work to `max_ops` times a constant fixed by the tick, fork, and fold caps"; "decoration-wide" to "too wide to catch a regression"; 1880: "a clock assembled from two universes"; 87: "the enforcement suite's case count"; 48: "this instrument's scope is the region..."; keep "rung" in ops.rs (before's own term) and say "snapshot" or "step" for the ladder in strategies.rs; rename the escalation arm's "cadence battery" or fold it into a named function; cite `FamilyId` variants by identifier at 172-174 or per `Family` variant. Acceptance: each listed term names an identifier, is defined once by contrast where introduced, or is replaced by its mechanism; no word has two referents in the fuzzfit harness. (The same "honest" qualifier recurs in sanity.rs:54, enforce.rs:201, and bands.rs:52, 116-119, 170, 187-190, outside this partition; noted for those reviewers.)

### fuzzfit-strategies-17: Prose restates draw ranges and roster ratios the code owns, one of them off by the jitter term
- Where: crates/before/fuzzfit/harness/src/strategies.rs:115-120 (related: crates/before/fuzzfit/harness/src/strategies.rs:98, crates/before/fuzzfit/harness/src/strategies.rs:292, crates/before/fuzzfit/harness/src/strategies.rs:301, crates/before/fuzzfit/harness/src/strategies.rs:961-964, crates/before/fuzzfit/harness/src/strategies.rs:1975-1976, crates/before/fuzzfit/harness/src/strategies.rs:1980-2016, crates/before/fuzzfit/harness/tests/enforce.rs:402-403, crates/before/fuzzfit/harness/tests/enforce.rs:424-426, crates/before/fuzzfit/harness/src/bands.rs:274-279)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (counted 17 weight-8 arms plus one weight-1 in `any_family`, so 17 × 8 + 1 = 137; `ESCALATION_MAX_DEPTH = 1792` at 130; `SMALL_BAND_KERNELS` has four entries at bands.rs:274-279; `universes.clamp(2, 4)` at 1759; CliffComb ticks `high + b.rng.gen_range(0..=2)` at 964 with `high = 2^10 - 1` when capped); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (the 1792 literal survived 319f9c53's introduction of `ESCALATION_MAX_DEPTH`; the others are originals)
- Owner-gated: no

Hand-maintained restatements of enumerable facts: "the magnitude draws top out at 8" (117) restates the `1u32..=8` ranges at 1988 and 1992 (and misses 2001's `1..=7`) with a dated "today", and "(2¹⁰ − 1)" (119) is exceeded by `CliffComb`'s jitter (up to 2¹⁰ + 1); "(256..=1792 spine forks)" (98) restates `256u32..=ESCALATION_MAX_DEPTH`, whose own doc exists so the range lives in one place; "Universe count (2..=4)" (292) restates 2013 and the clamp at 1759; "the four small-band kernels" (301) counts `SMALL_BAND_KERNELS`; "~137" (1975-1976) is computed from the `prop_oneof!` weights and echoed at enforce.rs:402 and 425-426, so adding one roster family silently falsifies four sentences in two files (Principle 5: state the structure, not the tally).

Evidence:

       115	/// The drawn `magnitude` becomes a shift count (`1 << magnitude`), so a
       116	/// draw-range widening past 31 would otherwise overflow the shift. The
       117	/// cap never binds today — the magnitude draws top out at 8 — it makes
       118	/// the shift's definedness local to the construction instead of resting
       119	/// on the draw ranges, and bounds a capped tooth's tick count (2¹⁰ − 1)
       120	/// far below the tick budgets.

       964	                    high + b.rng.gen_range(0..=2)

      1975	/// The roster is uniform except [`Family::Escalation`], weighted at one
      1976	/// draw in ~137: its programs cost quadratically in their reach (every

Resolution: 117-119: drop the "top out at 8" clause (the cap exists precisely so the shift does not depend on the ranges) and state "a capped tooth's base count is 2¹⁰ − 1, plus the family's jitter"; 98: "the family's full depth draw (up to [`ESCALATION_MAX_DEPTH`])"; 292: "Universe count (clamped by `build`)" or a shared `UNIVERSES` range constant; 301: "the small-band kernels ([`crate::bands::SMALL_BAND_KERNELS`])"; 1975-1976: "weighted 1 against every other family's 8", with enforce.rs:402 and 425 expressed as the weight ratio or computed in the message. Acceptance: no literal in these docs duplicates a value that also appears in `any_family` or a constant, and the tick bound matches the construction.

### fuzzfit-strategies-18: Family docs and arm comments misdescribe what the constructions do
- Where: crates/before/fuzzfit/harness/src/strategies.rs:181-182 (related: crates/before/fuzzfit/harness/src/strategies.rs:896-899, crates/before/fuzzfit/harness/src/strategies.rs:200-201, crates/before/fuzzfit/harness/src/strategies.rs:207-208, crates/before/fuzzfit/harness/src/strategies.rs:982-1004, crates/before/fuzzfit/harness/src/strategies.rs:299-301, crates/before/fuzzfit/harness/src/strategies.rs:1476-1498, crates/before/fuzzfit/harness/src/strategies.rs:1834-1836, crates/before/fuzzfit/harness/src/strategies.rs:1866-1869, crates/before/src/party.rs:233-237, crates/before/src/party/ops/split.rs:8-27, crates/before/src/clock.rs:153-157)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (each construction read against its doc; `Party::fork` keeps the left half deterministically per party.rs:233-236 and split.rs:15-17, and `Clock::fork` forks the party and clones the version per clock.rs:153-157); executed: no
- Seen by: instrument-correctness (181, 200), structure-prose (207, 299), adequacy (1866); refutation: confirmed for 181, 200, 299 (severity nit for 299: "a fork rejoined" can be read loosely), 1866; reframed for 207 (the doc is accurate for lane `a` but names no lane, and the non-descending lane's fork is essential: it is what deepens that lane's id each level, so document it rather than remove it); history: 299 deliberate-but-expired (stale from birth within af2330a6, whose own message records replacing the fork-and-rejoin shape); the rest no-rationale-found
- Owner-gated: no

Five places where the prose names a different construction from the code (Principle 5: a doc comment must be accurate to the code it describes): `DenseSpine::ticks_per_level` is documented as ticks per level but each level ticks `1 + gen_range(0..=ticks_per_level)`, so the field is the jitter ceiling and a value of 0 ticks once per level; `CliffComb::magnitude`'s "High teeth tick to `2^magnitude ± 1`" is false for the top of the draw range, where 16 even teeth at about 257 ticks each request about 4100 ticks against `BUDGET.max_ticks = 3000` and `tick_n` stops silently (the truncation is stated policy; the field doc is not conditioned on it); `IdPairLockstep::divert`'s "Keep the child (true) or the parent (false)" names no lane, and the inline comment "keeps opposite sides in the two lanes" holds for both values (what differs is which lane descends, and the non-descending lane's fork each level is what keeps its id deepening in lockstep); `Family::Bootstrap`'s doc says "a fork rejoined (the success join)" while the construction joins each child into a sink of earlier children and its comment says a fork-and-rejoin schedule "would not do"; and the `Independent` arm 8 comment asserts that separately seeded universes "always overlap", but every universe's seed descends the same side on every fork, so a deeper spine's seed nests inside a shallower universe's seed and is disjoint from that universe's sink of children, and `clock_join(deeper_seed, shallower_sink)` succeeds (the harness is correct because the mirror predicts per case; arm 5's "overlap likely" at 1835-1836 is the accurate register).

Evidence:

       181	        /// Ticks per level (0..=3 adds jitter).
       182	        ticks_per_level: u32,

       898	                let jitter = b.rng.gen_range(0..=ticks_per_level);
       899	                b.tick_n(seed, 1 + jitter);

       207	        /// Keep the child (true) or the parent (false) at each level.
       208	        divert: bool,

       300	    /// The path is tick, a fork rejoined (the success join), and the
       301	    /// clock codec round-trip — so the four small-band kernels

      1481	            // fork-and-rejoin schedule would not do: rejoining restores

      1866	                        // Cross clock join: separately seeded universes
      1867	                        // always overlap, so this is `Clock::join`'s
      1868	                        // rejection arm, priced as its own outcome (the
      1869	                        // mirror predicts it per case).

Resolution: 181: rename to `jitter` or "Extra ticks per level drawn from 0..=this, on top of one"; 200: "High teeth tick toward `2^magnitude ± 1`; teeth past the tick budget stay at zero"; 207: "Which lane descends each level: the seed's (true) or the first fork's (false); both lanes fork every level so both ids deepen, and the pair walks opposite halves of the id tree", with the inline comment at 995 restated the same way; 300: "a fork whose child is joined into a sink of earlier children (the success join, growing round over round)"; 1866-1869: reword to match arm 5 (overlap likely, both arms sample, the mirror predicts each). Acceptance: each field doc and arm comment describes the construction beside it.

**Nits (4), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| fuzzfit-bands-1 | `crates/before/fuzzfit/harness/src/bands.rs:66-70` | "honest" as a moral adjective and "sentry" as an unanchored coinage | Replace each "honest" with the mechanism it names ("pre-drift divergence", "in-band work", "the tail of legitimately cheap draws" ... | `evidence/partitions/fuzzfit-bands.md` |
| fuzzfit-bands-3 | `crates/before/fuzzfit/harness/src/bands.rs:157-158` | `Band.constant`'s doc invites the converse reading; three pinned bands have slope 0 and `constant: false` | "Whether the band was constant-classified: too little denominator span or too few buckets for a slope estimate (see `fit::fit`) ... | `evidence/partitions/fuzzfit-bands.md` |
| fuzzfit-bands-13 | `crates/before/fuzzfit/harness/src/wasm.rs:49-57` | `POOL_SLOTS`'s doc names a fallback the pooling allocator does not have | Reword: "The pool has no fallback: exceeding it fails `Instance::new` with wasmtime's concurrency-limit error ... | `evidence/partitions/fuzzfit-bands.md` |
| fuzzfit-strategies-10 | `crates/before/fuzzfit/harness/src/ops.rs:314-320` | `Malformed` is documented as never a `before` bug, but the decode and parse arms map before's own round-trip failures to it | either reword the doc ("a register-file or stage violation: a generator bug, or a `before` round-trip failure surfacing through a stale stage") or ... | `evidence/partitions/fuzzfit-strategies.md` |

**Cross-references.** fuzzfit-bands-2 and gate-legs-13 both cite the bands.rs head's hand-transcribed numbers; prose-hygiene-15 the "49 band keys". fuzzfit-bands-20 pairs with fuzz-guests-pins-20. fuzzfit-strategies-3's "decoration-wide" is also fuzzfit-strategies-15's; fuzzfit-strategies-6's vocabulary claim is the scope question fuzzfit-strategies-7 (the coverage finding, another class) decides. fuzzfit-bands-26 and fuzzfit-strategies-17 both restate the 137 and the 48. prose-hygiene-6's re-pin template sites are calibrate.rs:373-374 and 384-385 here.

## The instruments: fuelscape (before-fuelscape, docs/, build.rs's island job)

19 findings (0 high, 1 medium, 8 low, 10 nit). Full records: `evidence/partitions/fuelscape-pipeline.md`, `evidence/partitions/fuelscape-render.md`, `evidence/sweeps/module-graph.md`.

### fuelscape-render-9: Format-version docs narrate the layout they replaced
- Where: crates/before-fuelscape/src/compact.rs:76-82 (related: crates/before-fuelscape/src/dump.rs:65-70; crates/before-fuelscape/src/render.rs:54-61)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (read); executed: no
- Seen by: scaffolding [8], adequacy [28], structure-prose [35]; refutation: confirmed; history: contradicts the root hard rule (both docs written by 6f63edb7 as the bump rationale; the commit body records the move fully)
- Owner-gated: no

The root hard rules say nothing in the codebase refers to code that no longer exists, and Principle 5 says dated rationale at a declaration site is the same failure. "Version N moved the measurement commit from the index" narrates the deleted `commit` field of `IndexDoc`; the present-tense invariant already lives at `RunParams` (render.rs:54-61) and on the `IndexDoc` meta fields. Rated medium rather than high: a hard-rule breach by the letter, but the remedy is two sentences and nothing downstream depends on them.

Evidence:

        76	/// The compact format version both banners carry.
        77	///
        78	/// Version 3 moved the measurement commit from the index into each
        79	/// operation document, mirroring the dump format: a dataset accretes
        80	/// across measuring runs, so the index holds only the run parameters
        81	/// every document must share.
        82	const FORMAT_VERSION: u32 = 3;

    dump.rs:
        67	/// Version 2 moved the measurement commit from the index into each
        68	/// operation document: a dataset accretes across measuring runs, so the
        69	/// index holds only the run parameters every document must share.
        70	const FORMAT_VERSION: u32 = 2;

Resolution: keep the first sentence and, if a second is wanted, state the invariant positively without "moved" or "Version N": "each operation document carries its own measurement commit; the index carries only the run parameters every document shares". Acceptance: neither constant's doc names a prior layout.

### fuelscape-pipeline-3: History and roadmap in module prose: a split rule "unchanged", a "long-term fix" not built
- Where: crates/before-fuelscape/src/plan.rs:187-188 (related: crates/before-fuelscape/src/count.rs:37-39)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both sites; `git log -1 eb6b35f42` carries "the binary rule is the k = 2 case, stream-identical", the change the sentence records); executed: no
- Seen by: scaffolding [12], adequacy [22], structure-prose [33], instrument-correctness [53]; refutation: confirmed; history: "unchanged" was written by the commit that generalized the split (eb6b35f42) and compares to the implementation that commit replaced; the persistence paragraph dates from the parallelization commit (969cf3ae1) and nothing since tracked or built it
- Owner-gated: no

"unchanged" refers to a prior revision the tree no longer shows, which the root AGENTS.md hard rule forbids ("Nothing in the codebase refers to code that no longer exists"); the count module's closing paragraph is a plan, not a statement of what the module is (Principle 5). Both sentences stand without the offending clause.

Evidence:

    (plan.rs)
       187	/// For one part this draws nothing; for two it is a single
       188	/// `gen_range(1..total)` — the binary split rule, unchanged.

    (count.rs)
        37	//! Not built here: table persistence keyed by (grammar fingerprint, span)
        38	//! is the long-term fix for routine large-span surveys — a survey would
        39	//! load its tables instead of reconvolving them.

Resolution: Drop ", unchanged" at plan.rs:188; delete count.rs:37-39 and file the persistence idea in the shadow tracker if it is still wanted (a one-line negative-space statement, "tables are rebuilt per run", may stay). Acceptance: `grep -n 'unchanged\|long-term fix\|Not built here'` over the two files returns nothing.

### fuelscape-pipeline-15: The census literals' claimed independent derivation is not in the tree
- Where: crates/before-fuelscape/src/count/tests.rs:35-49 (related: crates/before-fuelscape/src/count/tests.rs:172-205; .agent-notes/2026-07-27-before-version-entropy/before-version-entropy.md:315-319, 466-481)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn -i 'entropy census'` over crates/, tools/, and the justfile hits only count/tests.rs:40; the literal sequence appears in the tree only at count/tests.rs:48 and in the agent note at line 474-475, whose line 317 names the generating script at a path outside the repository); executed: no
- Seen by: scaffolding [4], adequacy [19], structure-prose [30], instrument-correctness [45]; refutation: reframed (the actionable part is the doc; deletion is an owner call); history: already-known: the note's section 10 specified this pin and its doc comment for a later agent, recorded the script as outside the repo, and the pin was transcribed by 3d6ab1d49; the decoder census (b858edec) later became the live second derivation; the `before` crate-doc sentence that once cited the census was removed on 2026-08-04
- Owner-gated: no

Principle 8 and Principle 5: the doc presents an out-of-tree program as a second derivation the test exercises, and cites it by description only; what the tree holds is one derivation plus a frozen literal. The literal keeps a role the doc does not state: unlike the decoder census, it does not move when the enumeration and the decoder change together, so a canonical-form change must edit these integers deliberately (tamper evidence). `version_decoder_census_matches_constrained_family` already re-derives every listed length from `Version::decode` at 0..=23 bits, a superset of 1..=20.

Evidence:

        38	/// derived here by enumerating the coding grammar (topology bits + gamma
        39	/// payloads) and filtering on the nonnegative-height rule, must equal the
        40	/// counts independently derived by the entropy census of the same grammar
        41	/// (an exact dynamic program over the validator's accept rules,
        42	/// cross-pinned against brute force over all bit strings). Two
        43	/// derivations, one number: drift in either grammar transcription moves a
        44	/// committed integer.

Resolution: Re-state the doc against what the tree holds: the literals are the committed canonical-stream counts per bit length; the enumeration must reproduce them; the decoder census below re-derives the same numbers from the shipping parser, and this pin alone survives a coordinated change to both, so a canonical-form change edits these integers deliberately. Drop the reference to the out-of-tree program (or commit it as a test if its independence is wanted live). Acceptance: the doc comment names no derivation the tree does not contain and states the tamper-evidence role; the array is unchanged.

### fuelscape-pipeline-21: Stale hand-maintained count: "767 canonical members at exactly 2 bytes" is the pre-marker-padding window; the current window holds 433
- Where: crates/before-fuelscape/src/sample/tests.rs:118 (related: crates/before-fuelscape/src/count/tests.rs:47-49, 239; crates/before-fuelscape/src/count.rs:286-290)
- Class / severity / confidence: documentation / low / high
- Provenance: verified; executed: yes: a Python sum over the committed `CENSUS` literal (count/tests.rs:48) gives 433 for bits 8..=15 and 767 for bits 9..=16; `bit_window(2, MIN_VERSION_BITS) == 8..=15` is pinned at count/tests.rs:239; `git show d800957e -- count.rs` shows `bit_window` moving from `hi = 8 * bytes; lo = 8 * (bytes - 1) + 1` to `hi = 8 * bytes - 1; lo = 8 * (bytes - 1)`
- Seen by: scaffolding [6], instrument-correctness [44] (structure-prose [36] asserted the number is "correct today", which is false); refutation: confirmed; history: deliberate-but-expired: correct when written (3d6ab1d49), expired at d800957e
- Owner-gated: no

Principle 5: no hand-maintained counts in prose. The test computes `members.len()` itself and is unaffected; the comment has already rotted once and would mislead anyone calibrating the chi-square's category count from it.

Evidence:

       118	    let size = 2; // 767 canonical members at exactly 2 bytes.

Resolution: Delete the number; if a size rationale is wanted, "a few hundred members: enough categories for the chi-square, small enough to enumerate". Acceptance: no literal member count remains in sample/tests.rs.

### fuelscape-pipeline-29: Constants restated as literals in stamped size-measure strings and overlay labels
- Where: crates/before-fuelscape/src/ops.rs:205-207 (related: crates/before-fuelscape/src/ops.rs:133, 142, 203, 425-428, 769-770, 980-981; crates/before-fuelscape/src/families.rs:433, 440-450; crates/before-fuelscape/src/compact.rs:260-270)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the pairs: `M_SLICE_CAPPED` "min(16, size)" beside `COMBINE_ARITY_CAP = 16`; `version_ticks` "10⁹" beside `TICKS_COUNT = 1_000_000_000`; `party_forks`/`clock_forks` "a declared constant, 8" beside `FORKS_SHARES = 8`; four "(k=8)" labels beside `PARTY_FOLD_OVERLAY_SHARES = 8`; compact.rs:260 compares the roster's `size_measure` against the dump's verbatim); executed: no
- Seen by: scaffolding [7], structure-prose [36]; refutation: reframed to nit on the ground that "a constant bump without a string edit fails `fuelscape-verify` loudly rather than lying silently"; history: each pair landed in one commit with no tie mechanism
- Owner-gated: no

The refutation's severity argument is backwards for the case this finding describes. compact.rs:260 compares the roster string to the dump's recorded string; if `COMBINE_ARITY_CAP` changes and `M_SLICE_CAPPED` does not, both strings still say 16, the comparison passes, and the stamped population description lies about what was sampled. Only the opposite edit (string changed, dump not re-measured) fails loudly, and that is the deliberate re-pin event. So these are hand-maintained numbers the renders depend on (Principle 5) on strings that are stamped onto renders and committed widget data. The Cargo.toml gzip figure the lenses also cited is approximately right (257.9 MB / 245.9 MiB against "~242 MB") and is not an instance.

Evidence:

       203	const COMBINE_ARITY_CAP: u32 = 16;
       204	/// The size measure of the capped-arity slice row.
       205	const M_SLICE_CAPPED: &str = "total packed bytes; arity uniform over 1..=min(16, size) \
       206	     (the combiner's compile-time arity, capped at the guest's dispatch table), split \
       207	     uniform over the compositions";

    (compact.rs)
       260	        if spec.size_measure != data.size_measure {

Resolution: Build the stamped strings from the constants (`const_format::formatcp!`, a dependency in keeping with the crate's preference for libraries over hand-rolling) or assemble them at compaction time from `OpSpec` fields; label the party-fold overlays with `format!("... (k={PARTY_FOLD_OVERLAY_SHARES})")`. Failing that, one unit test in ops/tests.rs asserting each such string contains its constant formatted. Acceptance: changing `COMBINE_ARITY_CAP`, `FORKS_SHARES`, `TICKS_COUNT`, or `PARTY_FOLD_OVERLAY_SHARES` either needs no prose edit or fails a test.

### fuelscape-render-1: Vocabulary: "guest-minted", "two-ways seam", "Honesty rule"/"honest", "backstop"
- Where: crates/before-fuelscape/src/render.rs:133-136 (related: crates/before-fuelscape/src/plan.rs:19, crates/before-fuelscape/src/plan.rs:168, crates/before-fuelscape/src/lib.rs:19, crates/before-fuelscape/src/ops.rs:105, :185, :800, :2448, :2507; crates/before-fuelscape/src/dump.rs:40-44; crates/before-fuelscape/src/dump/tests.rs:54; crates/before/build.rs:5; crates/before/docs/fuelscape.js:7-10, :553, :873-874, :1517)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep -rn -i mint over crates/before-fuelscape/src; grep two-ways over crates/ tools/ justfile; grep honest/backstop in fuelscape.js); executed: no
- Seen by: structure-prose [45], scaffolding [17]; refutation: confirmed; history: contradicts writing-style.md (:170 bans "mint"; :306-311 names "the seam" as metaphor promotion; :326-330 moralized code; :393 translates "backstop")
- Owner-gated: no

The brief's vocabulary rule and writing-style.md forbid "mint" for constructing a value outright; "two-ways seam"/"two-ways pin" compresses the owner's doctrine into a phrase no identifier anchors and is used as if the reader already shares it; "Honesty rule", "honest linear density", and "honest clicks" describe code by a moral rather than by the property that holds; "backstop" is a register transplant where no adversary exists.

Evidence:

       133	        // One-operand rows take the whole column size (the party-fold
       134	        // row's single party included: its shares are guest-minted, not
       135	        // input bytes); everything else plots a total (the stamp carries
       136	        // the row's exact measure declaration).

    dump.rs:
        40	//! recomputed from the raw samples by [`aggregate`]. The grid check is
        41	//! the two-ways seam: the grid is derivable data persisted for

    fuelscape.js:
         7	// Honesty rule: instruction counts are WASM operations metered in a
       553	    // capture dropped) must still end the drag: blur is the backstop.

Resolution: "guest-minted" to "produced in the guest" (and the other mint forms in plan.rs, ops.rs, lib.rs to construct/produce/derive); "the two-ways seam" to a sentence that defines the check ("the grid is recomputed from the samples and must equal the stored one"), and "two-ways pin" likewise where it appears; "Honesty rule" to "Presentation rule", "honest linear density" to "linear density", "most honest clicks" to "most ordinary clicks"; "blur is the backstop" to "blur ends any drag the browser never releases". Acceptance: no "mint" in before-fuelscape prose; "two-ways" appears only in a sentence that defines it; fuelscape.js contains neither "honest" nor "backstop".

### fuelscape-render-5: Unanchored design-system vocabulary in the palette constants
- Where: crates/before-fuelscape/src/render.rs:280-292 (related: crates/before-fuelscape/src/render.rs:424-425)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for "reference palette", "categorical slot", "fills-need-spacers" over crates/, tools/, justfile hits only these three sites); executed: no
- Seen by: scaffolding [9]; refutation: confirmed; history: the phrases and hex values are transcribed from the bundled `dataviz` skill's reference documents (`palette.md`, `marks-and-anatomy.md`), which are not in the repository
- Owner-gated: no

Every coined term must be anchored to an identifier or defined at its site; "the reference palette", "categorical slot 2; validated against the ramp's blue", and "the fills-need-spacers rule" name a palette, a validator, and a rule that exist nowhere in the tree, so a reader cannot check what was validated or what the rule requires.

Evidence:

       280	/// Chart surface (light): the reference palette's chart surface.
       ...
       290	/// The adversarial overlay accent (categorical slot 2; validated against
       291	/// the ramp's blue).
       292	const ACCENT: RGBColor = RGBColor(0xeb, 0x68, 0x34);
       ...
       424	                        // A slight vertical inset keeps a visible gap between
       425	                        // occupied bins (the fills-need-spacers rule).

Resolution: state the checkable property in plain terms ("orange, chosen to stay distinguishable from the ramp's blue under common color-vision deficiencies"; "a 6% inset leaves a visible gap between adjacent occupied bins so cells stay countable"), or drop the qualifiers. Acceptance: the three phrases are gone and each color constant's doc names a checkable property or none.

### fuelscape-render-29: `build.rs`'s module doc describes one of its two jobs and denies a duplication the script deliberately carries
- Where: crates/before/build.rs:1-21 (related: crates/before/build.rs:38, :54, :93-100, :103-197, :272-278, :280-316; crates/before-fuelscape/src/compact.rs:71-82, :404-410; .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md:50-52, :97-100)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); executed: no
- Seen by: scaffolding [2], [7], adequacy [31], structure-prose [43], instrument-correctness [64]; refutation: confirmed, severity medium to low (the duplication is forced by the detached workspace and today's data passes both readers); history: deliberate-and-holds (the design note §1 forbids build.rs depending on anything outside the package, so a shared crate would itself have to be published; the sentence was written with §2's binning-constant meaning)
- Owner-gated: no (the leaf-crate alternative would be)

A module doc's first sentence stands alone in a listing, and prose states what is. The doc enumerates inputs and outputs for the islands only, while lines 93-197 (`theme_svg`, `Ink`, `check_readme_figure_fresh`) read `results/space_consumption/itc_space_consumption.svg` and write `$OUT_DIR/space_consumption.svg` and `docs/itc_space_consumption_readme.svg`, none of which the doc names; and the "holds no constants the widget or compactor also hold" sentence is contradicted by `check_banner`'s literal `3` and `v3` and the banner strings at 38 and 54, which restate compact.rs's `FORMAT_VERSION`, `INDEX_FORMAT`, and `OP_FORMAT`. The two validators have also drifted: build.rs checks the commit is a string and skips the overlay check; `compact::validate` checks overlay points and nothing about the commit.

Evidence:

         1	//! Formats the committed fuelscape widget datasets into rustdoc islands.
       ...
        17	//! This script is a pure formatter: it re-bins nothing, computes no
        18	//! statistics, and holds no constants the widget or compactor also
        19	//! hold. Every failure here is a defect in the committed repository
       ...
       274	        (doc["format"].as_str(), doc["version"].as_u64()),
       275	        (Some(expected), Some(3)),
       276	        "{file}: not a {expected} v3 document"

Resolution: open the doc with both responsibilities and add a second inputs/outputs paragraph naming the three figure paths (or split the figure job into `mod figure;` with its own doc); reword the sentence to "holds no binning or statistical constant; the format banners and version are the one deliberate duplication, spelled on both sides of the package boundary"; hoist `const FORMAT_VERSION: u64 = 3;` and the two banner strings so the check and its message read one constant; and either add build.rs's overlay-positivity check or state at 280-282 which of the compactor's checks are deliberately not repeated. Acceptance: the module doc names every file build.rs reads and writes; no bare `3`/"v3" in `check_banner`; the doc names the duplicated identifiers.

### module-graph-7: before-fuelscape depends on suanpan directly on a claim that `before` exposes no touch reader; `before::meter::touch_ops` exists
- Where: crates/before-fuelscape/Cargo.toml:23-26 (related: crates/before-fuelscape/Cargo.toml:22, crates/before-fuelscape/src/bin/spanbands.rs:47-58, crates/before/src/meter.rs:3611-3630)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn suanpan crates/before-fuelscape/src` hits only spanbands.rs:50,56; blame dates the comment to 5b2ae58ce, 2026-07-31; `git log -S'pub fn touch_ops'` dates the reader to 81048ec3d, 2026-08-05); executed: yes: the grep, blame, and log commands
- Verification: confirmed; history: deliberate-but-expired: the comment was true when written and stopped being true five days later when `touch_ops`/`reset_touch_ops` landed under `limb-meter`, a feature this manifest already enables (line 22).
- Owner-gated: no

The direct `suanpan` edge from the atlas exists only to satisfy a premise the tree no longer holds:
the reader the comment says is missing sits in `before::meter` beside the two other counters the
same function already reads.

Evidence:

        23	# Read beside `before`'s counters: the accumulator digit-touch meter the
        24	# `limb-meter` feature lights (`suanpan/touch-meter`), read directly as
        25	# `suanpan::touch_meter` — `before` re-exports no reader for it.
        26	suanpan = { path = "../suanpan", features = ["touch-meter"] }
    --- spanbands.rs:48-50 ---
        48	    meter::reset_scan_bits();
        49	    meter::reset_limb_ops();
        50	    suanpan::touch_meter::reset();
    --- crates/before/src/meter.rs:3621-3624 ---
      3621	#[cfg(feature = "limb-meter")]
      3622	pub fn touch_ops() -> u64 {
      3623	    suanpan::touch_meter::touches()
      3624	}

Resolution: In spanbands.rs read `meter::reset_touch_ops()` and `meter::touch_ops()`, then drop the
`suanpan` dependency and its comment (the detached workspace's lockfile updates with it); if the
owner prefers the direct read, correct the comment to the actual reason. Acceptance: fuelscape's
manifest has no `suanpan` line, or its comment states a true reason.

**Nits (10), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| fuelscape-pipeline-10 | `crates/before-fuelscape/src/count.rs:31-35` | The count module says every count is pinned two independent ways; the version table reaches the decoder only through the enumeration | "Every count is pinned against the grammar enumeration; the party table is also pinned directly against `Party::decode`'s accept census ... | `evidence/partitions/fuelscape-pipeline.md` |
| fuelscape-pipeline-17 | `crates/before-fuelscape/src/sample.rs:178-179` | Two small doc inaccuracies: "zero pad" for the marker padding; EXHAUSTIVE_BYTES credits the wrong companion pin | "before the marker padding" at sample.rs:178 and 376; at sample/tests.rs:14-16 name count/tests.rs's decoder census (23 bits) as the companion that ca ... | `evidence/partitions/fuelscape-pipeline.md` |
| fuelscape-pipeline-25 | `crates/before-fuelscape/src/ops.rs:4-5` | "adding an operation is one OpSpec entry" names one step of the chain a new row requires | Replace the clause with a short "adding a row" list naming the kernel, the overlay arm (or its signature match), the re-measure and compact re-pin ... | `evidence/partitions/fuelscape-pipeline.md` |
| fuelscape-pipeline-26 | `crates/before-fuelscape/src/ops.rs:9-12` | "constant dispatch overhead, identical for every sample" is one register move per operand on the fold rows | "plus the guest's dispatch overhead: constant for the fixed-signature rows, one register move per operand for the folds" | `evidence/partitions/fuelscape-pipeline.md` |
| fuelscape-pipeline-27 | `crates/before-fuelscape/src/ops.rs:61-68` | The stratified-arity argument is stated three times and the totality chain four times | Keep the arity argument on the `Inputs::VersionSlice` declaration and have plan.rs cite it in one clause ... | `evidence/partitions/fuelscape-pipeline.md` |
| fuelscape-pipeline-31 | `crates/before-fuelscape/src/ops.rs:2354-2358` | Exemption reasons cite panels in unchecked prose; the Ticks entry names "min_ticks", a panel that is not a roster name | Write `version_min_ticks` at 2357. If the panel references are worth enforcing, structure the exemptions as `Exemption::NoSizeAxis(&str)` and `Exempti ... | `evidence/partitions/fuelscape-pipeline.md` |
| fuelscape-render-11 | `crates/before-fuelscape/src/compact.rs:208-212` | nit: `expect` messages that name a hope, not the proof | "hi >= k0: max and min of the same nonempty list" and "k >= k0 by construction" | `evidence/partitions/fuelscape-render.md` |
| fuelscape-render-16 | `crates/before-fuelscape/src/dump.rs:168-175` | nit: `write_atomic`'s crash claim outruns its mechanism | narrow the doc to "a dying process" (matching dump.rs:16-18), or add `File::create` + `write_all` + `sync_all` before the rename (or use `tempfile::Na ... | `evidence/partitions/fuelscape-render.md` |
| fuelscape-render-25 | `crates/before/docs/fuelscape.js:1055` | nit: literal `\u2014` escapes inside JavaScript comments, and em-dashes in `//` comments across the partition | replace `\u2014` with an em-dash or a colon at 1055, 1538, 1540; sweep the listed `//` lines to colons, semicolons, or spaced double-hyphens | `evidence/partitions/fuelscape-render.md` |
| fuelscape-render-32 | `tools/fuelscape-claims:36-38` | nit: `fuelscape-claims` describes an acceptance rule the widget does not implement | replace both passages with the rule by name: "the rule is `Fuelscape.accepts`: every measured size must give >= 1 after the smallest constant argument ... | `evidence/partitions/fuelscape-render.md` |

**Cross-references.** fuelscape-render-29 and crate-root-4 anchor the same build.rs module doc; fuelscape-render-29 carries the validator-drift and banner-literal halves. fuelscape-pipeline-21's stale 767 and fuelscape-pipeline-17's "zero pad" are both d800957e residue. module-graph-7's manifest comment about a missing touch reader is the one sweep entry here. fuelscape-render-1's "two-ways seam" recurs as rank-13's "two-ways pin".

## The instruments: gate recipes and CI (justfile, tools/, .github/workflows)

12 findings (0 high, 2 medium, 4 low, 6 nit). Full records: `evidence/sweeps/deps.md`, `evidence/sweeps/gate-legs.md`, `evidence/partitions/tools.md` (the `tools/` partition, reviewed after the main run; the README's method section records why).

### deps-6: ci.yml installs and documents a floating `nightly` (and a floating stable) that no recipe invokes; its comments describe the regime the justfile pinned away
- Where: .github/workflows/ci.yml:54-74 (related: .github/workflows/ci.yml:17-25, .github/workflows/ci.yml:128-145, .github/workflows/ci.yml:202-216, justfile:23-40, justfile:1041, rust-toolchain.toml:20-31)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read ci.yml, justfile, rust-toolchain.toml; `git log -S'nightly-2026-06-30' -- justfile` dates the pin to e7a4b7b0c on 2026-08-04; `git log -- .github/workflows/ci.yml` since then shows one hand edit, e4d92ae4 on 2026-08-17 adding cargo-mutants to the install list, and otherwise dependabot bumps; `gh run view` on the last successful main run 33560347645 shows the coverage job, including `coverage-kernel-branch`, succeeded); executed: no
- Verification: reframed: the textual claims are confirmed; the sweep's "operational edge" about llvm-tools on the wrong nightly is settled by CI history (the branch leg runs green today, so rustup installs the dated nightly on demand and cargo-llvm-cov provisions its component there), and the sweep's "moved only by dependabot" is corrected to one hand edit that did not touch toolchain steps; history: deliberate-but-expired (the comments were true until e7a4b7b0c)
- Owner-gated: no

Every nightly recipe invokes `cargo +nightly-2026-06-30` and
rust-toolchain.toml pins stable at 1.97.1 with clippy, rustfmt, and the
wasm32 target; rustup resolves both regardless of what the workflow installs.
The workflow's toolchain steps install floating `nightly` and `stable` in all
three jobs, and its comments say the recipes "invoke `cargo +nightly`", that
the instruments job "tracks nightly, so a format bump upstream can turn the
leg red on an untouched tree", and that the runner needs "a current stable
toolchain (edition 2024 needs 1.85+)". Each is contradicted by the pins: the
installed floating toolchains are dead weight and the described failure mode
(a nightly format bump reddening an untouched tree) is exactly what the pin
removed.

Evidence:

    .github/workflows/ci.yml
        18	#   - a current stable toolchain (edition 2024 needs 1.85+), with clippy + rustfmt
        ...
        56	      # The doctest and fuzz recipes invoke `cargo +nightly`, so nightly only
        57	      # needs to exist, not be the default — whereas the gate's bare
        58	      # `cargo fmt`/`clippy`/`check` must hit stable.
        ...
        64	      - name: Install nightly toolchain (merged doctests and fuzz build)
        65	        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
        66	        with:
        67	          toolchain: nightly
        ...
       130	  # format_version. This job tracks nightly, so a format bump upstream can
       131	  # turn the leg red on an untouched tree — the checker refuses loudly,
        ...
       206	      - name: Install nightly toolchain (branch coverage)
       207	        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
       208	        with:
       209	          toolchain: nightly
       210	          components: llvm-tools
    justfile
        40	nightly_toolchain := "nightly-2026-06-30"
      1041	    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} llvm-cov nextest --branch --workspace --all-features --lcov --output-path target/llvm-cov/workspace-branch.lcov

Prose speaks in the present tense: the workflow describes a floating regime
the tree retired a month ago, and three jobs each download a nightly they
never use. workflowlint pins action SHAs but nothing holds the workflow's
toolchain inputs to the justfile's pin, which is how the two drifted in
opposite directions.

Resolution: install the pinned toolchains from one source: read
`just --evaluate nightly_toolchain` in a step and pass it as `toolchain:`
(with `llvm-tools` in the coverage job), drop the floating nightly installs;
drop the stable install steps or re-denominate their comments to
"rust-toolchain.toml provisions 1.97.1 with clippy, rustfmt, and wasm32";
rewrite lines 17-25, 54-58, 128-133, 139-141, and 202-205 to the pinned
regime. Optionally extend tools/workflowlint to require every dtolnay
`toolchain:` input to equal the justfile pin or the rust-toolchain.toml
channel. Acceptance: no `toolchain: nightly` or `toolchain: stable` remains
in ci.yml; every comment naming a toolchain names the pinned one; the three
jobs stay green.

### gate-legs-5: ci.yml installs and describes floating toolchains the recipes never invoke; the dated toolchains arrive by rustup auto-install
- Where: .github/workflows/ci.yml:128-133 (related: .github/workflows/ci.yml:17-20, .github/workflows/ci.yml:54-74, .github/workflows/ci.yml:139-153, .github/workflows/ci.yml:202-216, justfile:23-40, rust-toolchain.toml:20-23)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read ci.yml in full and justfile:23-40; the `ci` job log of run 33567211421 shows `info: syncing channel updates for nightly-2026-06-30-x86_64-unknown-linux-gnu` at the first `cargo +nightly-2026-06-30` invocation and `the toolchain '1.97.1-x86_64-unknown-linux-gnu' is currently in use (overridden by ... rust-toolchain.toml)` during both dtolnay steps; the coverage job log of run 33560347645 shows cargo-llvm-cov running `rustup component add llvm-tools-preview` for both `1.97.1` and `nightly-2026-06-30`; `git show --stat e7a4b7b0` touched justfile, rust-toolchain.toml, AGENTS.md, bands.rs but not ci.yml); executed: no
- Verification: confirmed, with the mechanism now attested from CI logs rather than inferred; history: deliberate-but-expired (the floating installs predate e7a4b7b0's pinning commit, which did not update the workflow)
- Owner-gated: no

Every nightly recipe invokes `cargo +nightly-2026-06-30` and rust-toolchain.toml pins stable at 1.97.1, yet all three jobs install floating `nightly` and `stable` (with `llvm-tools` on both in the coverage job), the prose says the recipes invoke `cargo +nightly` and that the instruments job "tracks nightly, so a format bump upstream can turn the leg red", which the dated pin exists to make impossible. The jobs work because rustup auto-installs the dated nightly at first use and cargo-llvm-cov self-installs `llvm-tools-preview` on the toolchains actually used; the workflow states neither.

Evidence:

        56	      # The doctest and fuzz recipes invoke `cargo +nightly`, so nightly only
        57	      # needs to exist, not be the default — whereas the gate's bare
        66	        with:
        67	          toolchain: nightly
       130	  # format_version. This job tracks nightly, so a format bump upstream can
       131	  # turn the leg red on an untouched tree — the checker refuses loudly,
       206	      - name: Install nightly toolchain (branch coverage)
       209	          toolchain: nightly
       210	          components: llvm-tools

    justfile
        40	nightly_toolchain := "nightly-2026-06-30"

    ci job log, run 33567211421 (ANSI stripped, truncated):
        Install nightly toolchain ... info: note that the toolchain '1.97.1-x86_64-unknown-linux-gnu' is currently in use (overridden by '/home
        just ci ... cargo +nightly-2026-06-30 test --workspace --doc ...
        just ci ... info: syncing channel updates for nightly-2026-06-30-x86_64-unknown-linux-gnu

    coverage job log, run 33560347645 (truncated):
        info: running `rustup component add llvm-tools-preview --toolchain 1.97.1-x86_64-unknown-linux-gnu` to install the `llvm-tools-preview` component for the selected to
        info: running `rustup component add llvm-tools-preview --toolchain nightly-2026-06-30-x86_64-unknown-linux-gnu` to install the `llvm-tools-preview` component for th

Resolution: Install what the recipes name: `toolchain: nightly-2026-06-30` in each nightly step (with `components: llvm-tools` in the coverage job), sourced from one place so the pin cannot fork (a workflow `env` the justfile variable is checked against, or a step that reads `nightly_toolchain` from the justfile); drop the floating `stable` steps and let rust-toolchain.toml provision stable (add `llvm-tools` to its `components` if the coverage job should not rely on cargo-llvm-cov's self-install); rewrite lines 17-20, 54-58, and 128-133 for the pinned regime and its paired bump procedure. Acceptance: CI logs show no `syncing channel updates` for a toolchain the workflow did not name, and the workflow prose names the same nightly date as justfile:40.

### gate-legs-7: `all` is documented as "Everything" while omitting the gate's instrument legs and the coverage legs, and its exclusive legs have no recorded cadence
- Where: justfile:1002-1003 (related: justfile:7, justfile:14-18, crates/before/tests/bench_judge_roster.rs:54-56, tools/benchjudge-expected.json:2)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read justfile:1-19 and 975-1003, ci.yml in full, bench_judge_roster.rs:45-62; `gh run list --workflow ci` shows only the `ci` workflow on main); executed: no
- Verification: reframed: the sweep rated this medium with a claim that the crate docs' "fuzzed codecs" sentence is unbacked; the fuzz targets exist, build in the gate, and their seeds are gated, so that sentence describes verification that exists and is not a false claim. What survives is the misnomer and the absence of any record of when the `all`-only legs last ran; the cadence decision is an open question for the owner rather than a defect; history: deliberate-and-holds for the manual tier (justfile:991-997 explains why each leg is local), no-rationale-found for the word "Everything"
- Owner-gated: no

`all`'s doc line reads "Everything" although the header's own next paragraph says neither sweep repeats the gate's instrument legs, and `all` also omits both coverage legs. The fuzz smoke, the formal tier, and the bench judge run only in `all`, which no workflow invokes and nothing records; the bench roster's one required red is re-attested only by habit.

Evidence:

         7	#   no-rot sweep just ci / just all                  everything, so nothing rots
        14	# what CI cannot run (the fuzz smoke and the formal tier). Neither sweep
        15	# repeats the gate's instrument legs — the fuel bands, the board verdicts
        16	# and pins, and surface totality run in `just gate`, and GitHub CI's
      1002	# Everything: the no-rot sweep, plus the fuzz smoke, the formal tier, and the bench judge.
      1003	all: ci (fuzz fuzz_smoke_secs) lean eventdag muxprobe bench-judge bench-judge-tripwire

    crates/before/tests/bench_judge_roster.rs
        54	/// means the tripwire went dark. Rostered reds are re-attested at
        55	/// `just all` cadence: the bench-judge recipes run there with the machine
        56	/// to themselves, while the gate's parallel tier judges no wall time.

Resolution: Rename the doc line and header entry to what `all` is (the no-rot sweep plus the manual tier), or make `all` include `gate` so the word is true. For cadence, see the open question below: either a scheduled workflow for the shared-runner-safe legs (the fuzz smoke) or a committed attestation of the last `all` run. Acceptance: `just --list` describes `all` accurately, and the tree or CI states when the `all`-only legs last ran.

### tools-1: ci.yml restates memwatch's per-process cap as a number that has rotted
- Where: .github/workflows/ci.yml:26-28 (related: tools/memwatch:46)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read both sites; `git log -1 a1febcce` is "memwatch: raise the per-process cap to 32 GiB", 2026-08-04); executed: no
- Seen by: scaffolding [13], adequacy [26], instrument-correctness [54]; refutation: confirmed; history: deliberate-but-expired (the comment was written at 8 GiB in c440730b, 2026-06-19; the default was raised to 12, 16, and 32 without touching ci.yml)
- Owner-gated: no

The workflow comment hand-copies the default of `PROC_LIMIT_GB`, and the copy is stale. Principle 5: a number restated away from its declaration rots silently; the argument ("sits well above anything a normal Linux build reaches") survives without the literal.

Evidence:

    26	# memwatch's swap backstop is sysctl-based and degrades to a no-op off macOS, so
    27	# wrapping the test/doctest/bench recipes is harmless here; its 8 GiB per-process
    28	# cap sits well above anything a normal Linux build reaches.

    (tools/memwatch)
    46	PROC_LIMIT_GB="${PROC_LIMIT_GB:-32}"

Resolution: drop the figure and name the variable ("its per-process cap, `PROC_LIMIT_GB` in tools/memwatch, sits well above ..."). Acceptance: `grep -n GiB .github/workflows/ci.yml` is empty.

### tools-2: The validation index omits citecheck, covcheck, and mutantcheck
- Where: crates/before/src/testing/validation_index.rs:1-3 (related: validation_index.rs:109-119; tools/citecheck:8-34; tools/covcheck:8-22; tools/mutantcheck:9-36)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n 'citecheck\|covcheck\|mutantcheck\|tools/'` over the index matches only the bench-judge row at 109-119); executed: no
- Seen by: scaffolding [16]; refutation: confirmed (and notes the omission set is wider: surfacecheck and the wasm32 pins also have no row); history: no-rationale-found (chronology: the index landed 07-28 and was last edited 08-07; the three tools landed 08-11 to 08-13 without touching it)
- Owner-gated: no
- Cross-references: testing-oracles-28 (claims) and testing-oracles-2 (simplification) hold the validation index's totality claim and its unrendered state; this entry names three gate or CI instruments with no row.

The index promises a row for every instrument guarding the crate and the class each alone catches. Three gate or CI instruments that guard `before` (uncollected citations; unexercised kernel arms by name; exclusion patterns drifting from the live mutant inventory) have no row, so a maintainer orienting cold never learns they exist.

Evidence:

     1	//! The validation index: every instrument that guards this crate, what
     2	//! failure class each one catches that the others cannot, and where it
     3	//! lives.

Resolution: add one row each under the semantic instruments (citecheck, covcheck, mutantcheck), naming the recipe and the roster file; consider rows for surfacecheck and the wasm32 pins in the same pass. Acceptance: `grep -n 'citecheck\|covcheck\|mutantcheck' crates/before/src/testing/validation_index.rs` returns one row each.

### tools-9: benchjudge cites code that no longer exists and narrates its own migration
- Where: tools/benchjudge:128-130 (related: benchjudge:23, 29-30, 65-67, 857-858, 864; tools/benchjudge-expected.json:2; tools/memwatch:4-8)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep over the tree excluding .agent-notes, target, and .claude: `expected-failure` appears only at benchjudge:66 and the JSON's line 2, `wall-ratio` only at benchjudge:130, `moved here with the leg` only at benchjudge:30; `git log -1 4451386a` is "Retire the display canary: its judgment lives in the bench judge", 2026-07-24); executed: no
- Seen by: scaffolding [5], structure-prose [39], instrument-correctness [53], adequacy [31]; refutation: confirmed (the sibling-roster phrase had no tree referent even at its authoring commit b4942461; the compile-fail suite that may have been meant was retired in d74c7f7f); history: contradicts-hard-rule (root AGENTS.md: nothing refers to code that no longer exists; the dated-notes sweep d2a9d04e edited these paragraphs and left the phrases)
- Owner-gated: no
- Cross-references: meter-adequacy-8 and gate-legs-9 (simplification) ask to restate the same roster paragraph (benchjudge:65-69) for its queue framing; this entry adds the ghost referents in it and in the ceiling derivation.

The text-ceiling derivation names a "retired wall-ratio canary" that appears nowhere in the tree; the roster paragraph names "the gate's expected-failure test roster", which does not exist and would breach the no-known-failures rule if it did; the fit paragraph says the ceiling "moved here with the leg"; two self-test pins are labeled by the review that produced them rather than the attack they close. memwatch:4-8 opens with an incident narrative at a declaration site (doctrine rather than the hard rule: its pointer resolves to a live comment in src/tree/traverse/act.rs).

Evidence:

   128	# This ceiling separates
   129	# the honest class from the schoolbook (quadratic) class it replaces the
   130	# retired wall-ratio canary in catching. Placement is derived from the
    29	ratio over log of the denominator-bytes ratio) at the general ceiling
    30	that moved here with the leg — the same convention, not a new one. A
    66	the board-side sibling of the gate's expected-failure test roster, same
   857	    # LAUNDERING PIN (the ceiling-class review's demonstrated attack):
   864	    # LAUNDERING PIN (the review's second attack): the schoolbook tripwire

    (tools/memwatch)
     4	# Why this exists: a monomorphization bomb (see the comment in
     5	# src/tree/traverse/act.rs) once made every leaf-crate rustc invocation
     6	# consume 25+ GiB at codegen, outrunning jetsam and wedging the machine into

Resolution: benchjudge:128-130: "This ceiling separates the divide-and-conquer class from the schoolbook (quadratic) class." Delete the sibling clause at 65-67 and in the JSON notes; at 29-30 write "at the general ceiling — the same convention"; at 857 and 864 name the attack ("a roster class cannot select a ceiling"; "a rostered red is still judged at its own ceiling"); reflow the orphaned short lines at 23 and 128. memwatch:4-8: state the invariant ("a codegen runaway fails the build with the crate named instead of wedging the machine") and leave the incident to git. Acceptance: `grep -rn 'wall-ratio\|expected-failure\|moved here\|review.s .*attack\|once made' tools/` is empty.

**Nits (6), compact; the full record of each is in the file named in its last column.**

| id | anchor | claim | resolution (abridged) | record |
|---|---|---|---|---|
| gate-legs-12 | `justfile:2-3` | `just --list` renders eight recipe descriptions as sentence fragments | Insert a blank line and a one-sentence doc comment above each of the eight recipes | `evidence/sweeps/gate-legs.md` |
| gate-legs-13 | `justfile:580-583` | Hand-maintained counts and dated measurements in verification prose | Name the constants instead of their values in the justfile (`ProptestConfig::with_cases` in enforce.rs, `REFIT_PREFIX_PROGRAMS` ... | `evidence/sweeps/gate-legs.md` |
| tools-8 | `tools/benchjudge:126-127` | "honest" as an algorithm class in benchjudge and as the baseline-fixture label in every self-test; "mint" for constructing a value at four sites (benchjudge:447, 859; memwatch:70, 94) | "the divide-and-conquer class", "the baseline fixture", "a roster cannot declare a class", "forge a synthetic record"; `grep -n '\bmint' tools/*` empty and `grep -ci honest tools/benchjudge` 0 | `evidence/partitions/tools.md` |
| tools-12 | `tools/benchjudge-expected.json:2` | The roster's `notes` duplicate measured exponents held at sidecar.rs:73-78 and enumerate six of `TEXT_CEILING_CELLS`' seven text cells by hand | Trim `notes` to the roster's contract, point at `TEXT_CEILING_CELLS`, keep each measurement at one site | `evidence/partitions/tools.md` |
| tools-27 | `tools/mutantcheck:33-36` | Two prose sites restate the pinned cargo-mutants release that `tool` in tools/mutantcheck-expected.json holds, and edits left docstring lines orphaned mid-clause (mutantcheck:19-20, benchjudge:23 and :128, justfile:308-309) | Name the pin file instead of the version at both sites; reflow the orphaned lines | `evidence/partitions/tools.md` |
| tools-29 | `tools/mutantcheck:195-197` | Em-dashes in `#` comments, docstrings, and nine printed diagnostics across the tools (owner-gated: the pending em-dash ruling) | Colons or semicolons throughout tools/; the self-test needles are substring-safe except citecheck:844 and workflowlint:545 | `evidence/partitions/tools.md` |

**Cross-references.** deps-6 and gate-legs-5 are the same ci.yml toolchain prose from two sweeps; gate-legs-5 attests the mechanism from CI logs. gate-legs-11, deps-11, and module-graph-6 are the fuzz workspace (listed under fuzz and pins). gate-legs-13's hand counts overlap fuzzfit-bands-2 and prose-hygiene-15. gate-legs-12's `just --list` fragments are the one finding on the justfile's own doc lines. From the `tools/` partition: tools-9's roster paragraph (benchjudge:65-69) is the one meter-adequacy-8 and gate-legs-9 (simplification) ask to restate for its queue framing; tools-1's stale figure is the ci.yml comment on memwatch; tools-8 and tools-29 are the `tools/` instances of the crate-wide vocabulary and em-dash census (prose-hygiene-5, prose-hygiene-10, prose-hygiene-12) and defer to the same owner rulings; tools-2's three missing index rows join testing-oracles-28 (claims) on the validation index's totality; tools-12 and tools-27 restate numbers whose site of record is `benches/common/sidecar.rs` and `tools/mutantcheck-expected.json`.

## Positives

What the reports found done well, deduplicated across partitions and restated once; each item names its site so it can be used as the template for the fixes above.

- `suanpan`'s crate page (`crates/suanpan/src/lib.rs`) is the strongest public prose in the two crates: every bound in the cost table has its argument on the same page, the amortization vocabulary is the standard potential-method one, hazards are stated where a user meets them (`&mut self` on sign reads at 123-126, no `PartialEq` at 307-309, `is_literally_zero`'s one-sidedness with a worked example), and the "When not to reach for it" section (250-268) does the which-one-do-I-use job explicitly. Its coined terms are anchored to identifiers almost without exception (lazy zone/`LAZY_LIMIT`, recenter/`RECENTER_BIAS`, quick register/`quick`, zero-run ledger/`zero_runs`, collapse/`fold_and_collapse`).
- The `Rank` wire-form module doc (`rank.rs:8-131`) is a complete, checkable argument: the inverted-polarity delta header shown bijective, the fraction's in-band framing justified against a length header by the 1/2-versus-7/16 counterexample, prefix-freeness derived from the close bit, and every rejected alternative (gamma/omega/varints, `dsi-bitstream`'s codes, ordered-varint, the FoundationDB tuple form) named with the mechanism that rules it out; the decoder's comments (722-787) mirror it clause for clause.
- The type-level operation tables on `Clock` (lib.rs:27-34; clock.rs:22-50), `Version` (version.rs:50-56, with the explicit "Comparison is **partial**" paragraph), `Party` (party.rs:38-46), and `Span` (span.rs:38-48), plus the "Version vector or vector clock?" section, were enough for the fresh-eyes sweep to write a whole scratch application without opening private source; every operator meant what the table said and the quickstart compiled as written.
- The `Decode` variant docs (error.rs:69-85) draw the `Truncated`/`TrailingBits` boundary exactly at the flush-byte edge, and the `# Errors` sections on `Span::decode` (span/wire.rs:79-89), `Rank::decode` (rank.rs:435-445), and `Ranked::decode` (ranked.rs:241-247) are variant-by-variant with the input class that produces each: the template the missing sections should copy.
- The present-tense discipline for measurements is stated and followed at `ceilings.rs:56-62` and `tests/meter.rs:245-257`: readings live in the pin commits (`git log -S` the constant), never in prose, with the reason given (a quoted reading keeps asserting itself as fact while headroom absorbs drift). The dated-notes excision commit d2a9d04e applied the doctrine at scale and reported in its own message the `decided` machinery it could not dissolve.
- `overlay.rs:27-67` states the overlay-advance law's correctness argument once (nesting, the flip-level tie test, exhaustion by the all-right path), and every `expect` and `unreachable!` in the comparison kernels is a one-line proof drawn from it; the same one-line-proof standard holds across the partitions that checked it (party, clock, codec-bits, codec-base-text-tree, skyline-fill-grow, skyline-watermark, version-core, fuzzfit-strategies), and no assert, expect, or panic message in rank, fill-grow, version-core, watermark, or codec-base-text-tree carries an em-dash.
- `fold.rs`'s module doc (1-16) states the goal beside the mechanism: why the balanced counter over a left fold, with the concrete lattice shapes that make the left fold quadratic and the combiner precondition that makes regrouping value-identical. `shape.rs` leads with which-of-these-do-I-want and explains why heights travel as rises. `recurse.rs:22-28` explains a non-obvious choice (guarding the descent rather than the body) with its frame-cost argument.
- `codec/dsi.rs:1-30` is the model for documenting a dependency boundary: the production reader from the library, the writers in-house, and the reason the library's own `read_gamma` is refused (a `debug_assert`-guarded cap that would mis-decode in release). `Cargo.toml`'s feature comments say what each counter sees that no other meter can, and `before-fuelscape`'s manifest justifies every dependency by what the cheaper alternative would have cost.
- The party kernels carry their arguments where the code lives: `sum_split`'s fusion proof (sum_split.rs:13-66), `Lockstep`'s empty-stack invariant (compare.rs:103-112), `diff.rs`'s covered-block taxonomy (18-38), `PosStack`'s delta coding with its off-by-one explained at the field, and `IdIndex`'s stated trade (index.rs:1-33), whose candor is what made its findings findable.
- `version/skyline.rs:4-7` defines *skyline* in italics by contrast and introduces *plateau* beside it; the canonicality argument (65-90) re-derives the paper's normal form; `PreScan::run`'s vocabulary caution on `level` versus `depth` (prescan.rs:142-146) is a model maintainer note; `watermark.rs`'s cost claims each name a committed instrument whose name resolves.
- Every test in the skyline-coding, comparison-kernel, and span-causally partitions carries a doc comment stating its invariant, and the ones checked against their bodies were accurate; test docs across the envelope suite pair each cost pin with the value leg it rides beside.
- The fuzz-fit harness discloses its own limits candidly: the blessed-drift window section (bands.rs:42-70) argues against its own instrument where it must, the two one-sided margins (fit.rs:13-23) name the measured edges each sits between, and every judgment leg's tripwire states what it does not prove.
- Zero `TODO`/`FIXME`/`XXX`/`HACK` markers across every in-scope surface; no opaque roster tags anywhere but the three fuzz-workspace sites; "silently" paired with its mechanism at every sampled site.

## Open questions for Finch

Deduplicated across the partitions and sweeps; each item is re-grounded so it can be ruled on without scrollback, and each carries a recommendation.

1. **"door".** The word is used 208 to 279 times (by grep scope) as maintainer vocabulary for a public entry point, with no definition anywhere, and it reaches public rustdoc through `party.rs`, `laws`, and `meter`. Every partition that met it asked for one crate-level ruling rather than local edits (crate-root-21, codec-bits-5, rank-13, skyline-coding-1, oracle-laws-19, span-causally-17, testing-diff-gen-24, version-core-17, fresh-eyes-4, prose-hygiene-11). Recommendation: retire it in public rustdoc for the plain noun (the style guide's own table lists it as default-dialect for "entry point", and 22cdfbe1 already replaced every "door" in `algebra.rs` with "operator"); if kept in maintainer prose, define it once by contrast in `codec.rs`'s or `lib.rs`'s private docs and link that site.
2. **"seam", "genre", "knob", "luck-proof".** Same rule, same scope (seam 276 lines, genre 210, knob 107). Recommendation: define the two senses of *seam* once in `registry.rs` (the shape families) and use "boundary" elsewhere; keep *genre* out of public docs (span-causally-22) and define it once in `error.rs` or the codec doc if it stays private; replace *knob* with "parameter" and *luck-proof* with the property it names.
3. **"honest"/"genuine" (146 to 234 lines).** The word is also your own idiom in recorded rulings, so several partitions declined to sweep it without confirmation (board-frame-18, skyline-query-12, prose-hygiene-10). Recommendation: one crate-wide pass with an owner-confirmed word list, replacing each use with the property it stands for ("the largest production reading", "derived from irreducible work", "unmutated"), and keeping the anchored sense at `assert_honest_text`.
4. **Em-dashes in `//` comments (374 lines, 76 files).** The rule lives in your global doctrine, postdates most of the prose, and no gate tool checks it. Recommendation: one mechanical pass in a commit of its own (spaced double-hyphen or a colon on `//` lines; rustdoc keeps its dashes); add the `tools/` lint leg prose-hygiene proposes only if you want it enforced rather than swept.
5. **"mint" (82 lines) and `Reign::mint`.** The ban is unconditional in your writing rules; the identifier is private. Recommendation: rename `Reign::mint` to `Reign::new`, sweep the prose to construct/create/build or the specific operation, and rule separately on watermark.rs's latent-register sense (skyline-fill-grow-21): link it at first use or rename it with the module.
6. **`decided:` dates and `REGISTRY_RATIFIED`.** The only readers check that the string has a date's form; d2a9d04e reported the machinery to you as a design round. Recommendation: dissolve the fields and the shape checks (`git log -S` on any reason string recovers the date); if kept, say once at the field doc that the registry is an embedded decision record and dates are part of its schema (prose-hygiene-7; also surface-roster-17 and meter-registry-tier2-9 in other classes).
7. **Where the compactness bound and the sizer's prose live after re-denomination.** `testing/compactness.rs` and `meter/tier2.rs` still speak from before the flag day; the keep of the probes is a recorded ruling. Recommendation: state the "skyline at most twice the min-lifted reference" bound once in `version/skyline.rs`'s design essay, re-denominate both modules against it in the present tense, rename `Sample.current_bits` to `reference_bits`, and treat renaming `meter::tier2` (public under `meter`) as the owner-gated part (testing-diff-gen-17, meter-registry-tier2-14).
8. **Is the `Ω(M(|v|))` floor a public promise?** The three `mul_bound_*` pins guard it; no public contract states it, and the style rule keeps Ω out of headlines. Recommendation: keep it private and re-scope the pins' docs to the derivation in `query.rs`/`integral.rs` (testing-diff-gen-22).
9. **Plateau: leaf or maximal constant run?** The code yields canonical leaves, which can be adjacent and equal across a subtree boundary. Recommendation: define the item as the leaf, once, in `shape.rs`, and apply the same wording at skyline.rs:4-5, overlay.rs:99, and party.rs:473; a merging adapter is a consumer's one-liner (crate-root-38).
10. **`# Errors` uniformity.** Half the fallible public entries lack the section. Recommendation: one crate-wide pass copying `Rank::decode`'s form, and have `doclint` require the section on every `pub fn` returning `Result` so the convention cannot lapse again (fresh-eyes-2, api-audit-8, version-core-14, clock-11).
11. **Owner hand-edit commits that deleted substance pointers still cite.** a6dcfbb4, b3f09baa0, 20c0515a, and bbb9f802 removed the `forks` saturation clause, `Party::decode`'s Party-specific warning, `join_all`'s coalescing clause, the `Coverage` exactness sentence, and the no-`Eq` rationale, and introduced the inverted SAT direction and several typos; the earlier agent-written text is quoted in the entries as a restoration candidate. Recommendation: re-rule on these as a batch and restore the substance in your own words (party-7, party-8, party-14, span-causally-5, span-causally-33, span-causally-34, span-causally-38).
12. **The validation index's scope.** It claims to map every instrument and omits surfacecheck, citecheck, the two hidden-surface pins, the fuzz targets, the wasm32 pins, the verdict matrix, and the flatness criteria; and it renders under neither rustdoc pass because `testing` is `cfg(test)`. Recommendation: make it total and rendered (move it under the meter-gated tree with a light liveness pin that every gate recipe and `tests/*.rs` binary is named), since `AGENTS.md` routes maintainers there (surface-roster-16, fuzz-guests-pins-24, tests-other-1, module-graph-11, testing-oracles-29).
13. **Kernel-doc test citations.** Production docs under `skyline/**` cite tests and envelopes by bare identifier and no gate leg checks them. Recommendation: extend `tools/citecheck`'s extraction to backticked identifiers in comments under `src/version/skyline/**` that match a collected test or an envelope name, demonstrated once by a deliberate local rename (skyline-fill-grow-17).
14. **Notation defined once.** `M` (unbounded-integer multiplication) is defined at eleven sites; `|iter|` in the fold contracts is never defined. Recommendation: a crate-level complexity-notation section in `lib.rs` that defines `M`, `|x|`, and `|iter|` once, linked from the fuelscape include (rank-31; clock's open question 6).
15. **The serde representation.** Every type serializes as `serialize_bytes` of its canonical encoding, so serde_json carries a number array, and the docs do not say so. Recommendation: document the representation now (fresh-eyes-3); whether to branch on `is_human_readable()` and emit the paper notation is a separate decision that changes bytes for human-readable formats and needs text forms for `Rank`, `Ranked`, and `Span` first.
16. **`MinWeb::compacting`'s measured ratios.** The ×1.41 and ×2.0 figures are a dated measurement of an implementation not in the tree, but `.cargo/mutants.toml:74-75` cites the doc as carrying the adequacy evidence for a mutant cargo-mutants cannot filter. Recommendation: drop the ratios and re-point the roster and tests/meter.rs:7137-7139 at the `skyline_min_ticks_ascend` row, or commit the demonstration as a `#[cfg(test)]` constructor toggle so the numbers live in a test (skyline-watermark-8).
17. **The `# Panics` form for kernels.** walk.rs's "truncation and malformation panic; the rest walk silently with an unspecified result, per `causal_cmp`" is the accurate form; fill, fuse, and grow still promise a panic on any non-canonical stream. Recommendation: adopt walk.rs's form crate-wide and state it once per module with one-line pointers (skyline-fill-grow-4, skyline-sweep-place-masked-9, skyline-sweep-place-masked-13, skyline-coding-35).
18. **The board root doc's summary-plus-pointer rule.** a05918df7's verbatim-copy criterion is what let two copies drift. Recommendation: state the map rule inline at the top of `board.rs` and replace each re-narrated derivation with a one-sentence pointer to the owning submodule (board-frame-5).
19. **`AGENTS.md` as guidepost.** Besides the ghost pointers (api-audit-3), it never names the fuzzfit workspace or the re-pin discipline that `rust-toolchain.toml` routes through `just fuzzfit-calibrate`, and it repeats a `descend!` framing the test surface does not follow. Recommendation: one guidepost pass: point the model reference at "Safety rules", drop or re-point the essay clause, add one line for the fuzzfit recipes and `bands.rs`, and restate the recursion rule with its bound classes (api-audit-3, recursion-4, fuzzfit-bands' open question 3).
20. **`answer-embedded` names two claims.** The `tests/answer_embedded.rs` file uses it for a cost-coupling claim while `asymptotics.rs`, the registry, and the `answer_embedded_product` band use it for the rank's value structure. Recommendation: rename the file and its doc to the mechanism it attacks and leave "answer-embedded product" to the multiplication-bound claims (tests-other-4).

## Counts

By severity (all 399 entries):

| severity | count |
|---|---|
| high | 6 |
| medium | 29 |
| low | 197 |
| nit | 167 |
| total | 399 |

By module section (the eight prose-hygiene census entries are counted under the patterns section):

| section | high | medium | low | nit | total |
|---|---|---|---|---|---|
| Crate-wide patterns (prose-hygiene census) | 0 | 1 | 5 | 2 | 8 |
| crate root (lib.rs, error.rs, iter.rs, Cargo.toml, build.rs, AGENTS.md, README) | 0 | 2 | 13 | 13 | 28 |
| clock | 0 | 1 | 6 | 11 | 18 |
| party (party.rs, party/, idbits.rs) | 1 | 2 | 12 | 4 | 19 |
| version core (version.rs, own.rs, ticks.rs, hull_traffic.rs) | 0 | 1 | 11 | 7 | 19 |
| rank (rank.rs, rank/num.rs, ranked.rs) | 0 | 0 | 7 | 5 | 12 |
| span and causally | 0 | 1 | 11 | 10 | 22 |
| coding (skyline.rs, admit, build, decode and encode, emit, literal, validate, text, shape, walk) | 0 | 1 | 3 | 10 | 14 |
| fill and grow | 0 | 1 | 6 | 7 | 14 |
| comparison kernels (sweep, place and filter, masked, overlay, signed) | 0 | 1 | 3 | 9 | 13 |
| query (query, integral, web) | 0 | 0 | 7 | 7 | 14 |
| watermark and the traffic counters | 0 | 0 | 3 | 7 | 10 |
| bits (bits, buf, build, code, cursor, dsi, gamma, int, literal, scan, stack) | 0 | 0 | 8 | 3 | 11 |
| base, text, tree, display | 0 | 0 | 3 | 4 | 7 |
| fold, shape, recurse, serde and borsh | 0 | 1 | 5 | 1 | 7 |
| suanpan (the crate and its test suites) | 0 | 0 | 6 | 13 | 19 |
| oracle and laws | 0 | 0 | 5 | 1 | 6 |
| meter core (meter.rs and its tests) | 0 | 2 | 2 | 1 | 5 |
| the family registry and tier2 | 1 | 0 | 4 | 3 | 8 |
| the amplification board (frame; families, floors, judge; ops, render, shards, tests) | 0 | 5 | 18 | 7 | 30 |
| the surface roster (surface.rs, surface_coverage, surfacecheck, surface-scan) | 1 | 0 | 3 | 3 | 7 |
| the test harness (testing/: bridge, semantic oracle, exhaustive, algebraic laws, validation index, diff_ops, generators, compactness, asymptotics, fuelscape islands) | 0 | 2 | 11 | 6 | 19 |
| the envelopes (tests/meter.rs) | 2 | 1 | 7 | 5 | 15 |
| other suites (tests/*.rs) | 0 | 1 | 4 | 3 | 8 |
| benches and examples | 1 | 0 | 4 | 2 | 7 |
| fuzz targets, the fuzz-fit guest, and the wasm32 pins | 0 | 1 | 10 | 3 | 14 |
| fuzzfit (the harness: bands, fit, curve, wasm, strategies, ops, drive) | 0 | 2 | 8 | 4 | 14 |
| fuelscape (before-fuelscape, docs/, build.rs's island job) | 0 | 1 | 8 | 10 | 19 |
| gate recipes and CI (justfile, tools/, .github/workflows) | 0 | 2 | 4 | 6 | 12 |
| total | 6 | 29 | 197 | 167 | 399 |
