# Verification gaps, instrument adequacy, and test quality

This document collects every finding of the `before` and `suanpan` review whose primary class is *verification-gap* (a property or input family no committed instrument reaches, or an instrument weaker than its documentation says) or *test-quality* (a test whose body checks less than its doc states, or a check that cannot fail). It answers two questions: what could be wrong that the committed instruments would not detect, and where are the instruments weaker than they claim. It reads the tree at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764; two later commits on `main` touch only `.agent-notes/`, so every anchor below holds at both. Entries are reproduced in the finalizers' template with their anchors, evidence, severities, and provenance unchanged; the synthesis adds four kinds of line and nothing else: a *Witness* line where the witness pass constructed and ran the finding's demonstration (`witness/results.md`), a *Cross-references* line where another entry records the same defect or the other half of it, a *Synthesis note* where the witness or a sibling report qualifies the entry, and the compact nit tables the scale rule asks for. Nothing was dropped; two entries the witness pass refuted in part are kept with the refutation stated beside them (surface-roster-6, board-ops-render-26).

Ids are `<partition or sweep key>-<n>`, the finalizers' own numbering; the full record of each, including its refutation and history passes, lives in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md` beside this file. The severity scale is the finalizers' and is reproduced unchanged; read it as: *high*, an enforced instrument passes a wrong artifact of the class it exists to catch, or a hard rule of the tree is breached; *medium*, a stated guarantee has no committed instrument, or an instrument is weaker than its documentation says, at a place a regression would land; *low*, the same at a corner, or where another instrument covers the gap indirectly; *nit*, a local inaccuracy or a dead check whose repair is one edit. Nits appear in per-section tables (id, anchor, claim, resolution) and are counted with everything else.

Provenance, per entry: *demonstrated* means a constructed test ran and showed the gap (the witness pass, or the finalizer's own execution); *executed* means a run settled a number without a constructed mutant; *verified* means mechanically checked or re-derived (grep, git, arithmetic, a transcription run in Python); *assessed* means read. The witness pass adds three outcomes of its own: *demonstrated* (the construction ran and behaved as the entry predicts), *inconclusive* (the construction needed a command the witness could not run, so the entry stands on its reading), and *refuted* (the construction ran and contradicted the entry's claim; the entry is kept and the note says which clause fell). Sixty-nine entries here carry a witness line: 52 demonstrated, 15 inconclusive, 2 refuted.

## Highest-value items

1. The board's heap exponent is fitted on allowance-inclusive readings while its constant leg subtracts the allowance, so a Theta(n^2) heap term of allowance-scale magnitude reads a four-point trend of 0.914 and passes acceptance in both windows; the top window alone reads 1.51 and the residual fit reads 2.0. Demonstrated by run through `evaluate_acceptance` (board-families-floors-judge-21).
2. The `segments` currency is judged on every board cell and pinned at zero on all 84 envelope rows, but its only writer is `#[cfg(test)] fn grow`, unreachable from every binary that reads it; a 200 000-frame recursion through `stacker::grow` in an integration binary leaves the counter at 0. Six partitions and sweeps filed it independently; the owner's call is between dissolving the column and giving it a writer (board-ops-render-15, with crate-root-32, envelopes-a-2, module-graph-1, recursion-1, inventory-2).
3. The supply-chain recipe audits five of six committed lockfiles; the omitted `wasm32-pins/Cargo.lock` resolves wasmtime 47.0.3, which RUSTSEC-2026-0268 and 0269 list with `patched >= 47.0.4`, and CI runs that recipe green. The same four-name enumeration is restated in five places (deps-1, with gate-legs-3 and fuzz-guests-pins-37).
4. Two board rows measure only their early-exit verdict: `causally_contains` admits at one witness on 28 of 29 families and `party_covers` refuses at the root on all 14, so the certifying walks the roster credits them with pricing are measured almost nowhere; the `version_eq` cell likewise compares operands that diverge at the first leaf payload (board-ops-render-9, meter-adequacy-7).
5. The admission walk's mid-stream collapsible-pair rejection has no committed witness through `Span::decode`: with the step arm's `zero_delta` forced false, `Span::decode([0xE0, 0x3E, 0xE0])` accepts a span whose `hi` carries the pair `(0, 0, 0)` while 227 committed tests stay green. Under the hard rule that `Eq` rests on byte equality, that is an equality bug the gate cannot see (skyline-coding-6).
6. The depth-100k proof `AGENTS.md` names for the no-recursion rule drives a listed subset of operations over one left-only spine; both-present id frames and right descents have no overflow-depth witness anywhere, and the public shape iterators appear in no deep test and no cost instrument (clock-22, recursion-6, meter-adequacy-1).
7. The fuzz-fit bands price 44 of the guest's kernels while the justfile calls them the cost law for every public operation; 49 measured public-operation kernels have no `Op`, no band, and no roster that can see them, and the shape leg judges within-case slope against a pooled slope its own prose calls mixture-tilted, so a d^1.45 mechanism on `ff_version_decode` passes both legs (fuzzfit-strategies-7, meter-adequacy-2, fuzzfit-bands-10).
8. The fuzz targets' own agreement tables, allowance arms, and heap cap execute only at `just all` cadence: the gate compiles them and never runs a seed, and the flat 1 GiB cap cannot see any amplification under 2^18 times a 4096-byte input. A `-runs=0` seed replay is a seconds-long gate leg (fuzz-guests-pins-38, fuzz-guests-pins-14).
9. No recipe runs the mutation campaign the exclusion roster names as its authority; the witness pass's small ad hoc campaign found two survivors, the `read_bits` chunk arm in `PackedBuilder` and the latent-annihilation ordering in `drop_below`, and two suanpan exclusions rest on equivalence premises that fail in the touch denomination (gate-legs-6, codec-bits-15, skyline-watermark-24, suanpan-40).
10. Twenty registry-dispatched generators have no size or canonicality pin, four stated closed forms are wrong, and six memo-family event shapes reach every consumer through the unvalidating `Packed::version()`: a non-canonical stream planted in `memo_chain` is measured green by two envelope rows and rejected only by `Version::decode`, which nothing on that path calls (meter-core-2).
11. Dark-meter paths: the scan column has no liveness floor in any envelope table, so deleting the validator's topology tap drops two thirds of the scan work with everything green; two flatness suites return growth 1.0 on a zero counter and the hull fold's limb leg already reads zero at both levels; suanpan's one sign-flip instrument passes with an all-zero grid (envelopes-a-6, tests-other-14, tests-other-6, suanpan-tests-25).
12. Three rosters attest that a kernel is named, not that it runs: `#[ignore]` on any known-bad kernel, twin, or band keeps every roster green; thirteen cost pins bind to no roster at all and can be deleted with the gate green; the `Party::tick` roster row cites three tests that never call it while the law that does is cited by nothing (tests-other-24, envelopes-b-22, surface-roster-4).

## Crate-wide patterns

Eight defect families recur across partitions. Each is named once here; the entries carry cross-references to their siblings.

**A judged column with no writer.** The stack-segments counter is read by the envelope suite, the board, the bench, and the example, and written only inside `#[cfg(test)] fn grow`; the prose at six sites calls the zero a measurement. One owner ruling settles all six entries (board-ops-render-15, crate-root-32, envelopes-a-2, module-graph-1, recursion-1, inventory-2).

**A ceiling with no liveness floor.** A ratio or ceiling over a counter passes when the counter goes dark: the envelope scan column (envelopes-a-6), the two bespoke flatness suites (tests-other-14, tests-other-6), suanpan's sign-flip grid (suanpan-tests-25, suanpan-39), the fuzz-fit floors' unasserted clearance over the nop reading (fuzzfit-bands-17), the exhaustive grow-minimality pin (testing-oracles-22), the limb, touch, and heap floor trips with no committed trip (board-families-floors-judge-19), and the two wasm32 premises with no self-test (fuzz-guests-pins-14, fuzz-guests-pins-26).

**A measurement that stops at the early exit.** The board's membership, covers, and `version_eq` cells measure the O(1) verdict on nearly every family (board-ops-render-9, meter-adequacy-7); the early-exit discipline is pinned only for the test-gated `sweep::eq`, never for the production `order_exit` or the masked exits (skyline-sweep-place-masked-32); the coverage walk's three exits are pinned by nothing (skyline-sweep-place-masked-20).

**Hand rosters checked only against each other.** `FamilyId::ALL`, `index()`, and `ALL_SHAPES` (meter-registry-tier2-11); the fuzz-fit kernel roster (fuzzfit-strategies-8, fuzzfit-bands-30); `Exclusion::FAMILIES` (surface-roster-3); `auto_traits.rs` (crate-root-7, api-audit-7); the lockfile enumeration (deps-1, gate-legs-3, fuzz-guests-pins-37); the all-NA cell disclosure (board-families-floors-judge-11, board-frame-3, meter-adequacy-4); the fold-operation pin roster (testing-diff-gen-23); the fuzz target list (tests-other-20); suanpan's `SOURCES` (suanpan-28). Each is a derivable list nothing derives.

**Known-bad demonstrations that live in prose or in a commit message.** The weight-comb, freeze-parade, and tooth-tail bands cite "a local probe build" (envelopes-a-22, meter-registry-tier2-7, meter-adequacy-6); the stagger and scatter fold bands compute their known-bad fold and never meter it (envelopes-b-18); the fold-operation floors are transcribed midpoints whose linear reference is printed and never asserted (testing-diff-gen-26, meter-adequacy-5).

**Rosters that attest a name, not a run.** The superlinear, inverted-twin, and band scanners read `fn` lines and cannot see `#[ignore]` (tests-other-24, surface-roster-11); the band parity scan is keyed on a naming convention thirteen two-point pins sit outside of (envelopes-b-22, envelopes-b-25, meter-registry-tier2-3); five writer-sink rows cite a doctest no resolver can see (gate-legs-8, surface-roster-6).

**Framing and constants transcribed across the detached-workspace boundary.** The fuzz targets' arity band, chunk carve, and op table are copied into the seed tests and held together by comments (fuzz-guests-pins-10, tests-other-22, tests-other-18); the fuzz-fit bands are bound to neither the corpus nor the wasmtime version that produced them (fuzzfit-strategies-5, fuzzfit-bands-6).

**Process-isolation as an unchecked premise.** Every process-global counter rests on nextest's one-process-per-test model, stated in prose and appended to failures, and checked by no test; a shared-process runner produces false passes with no note (suite-economics-1, envelopes-a-3, envelopes-b-10, meter-registry-tier2-19, testing-diff-gen-27, board-ops-render-27, suanpan-tests-12).

## The enforcement chain: gate legs and meter adequacy

**Gate legs, CI, and derived-artifact freshness (gate-legs)**

### gate-legs-3: wasm32-pins is missing from every hand-enumerated roster of detached workspaces; its lockfile is never audited and its local-only status is undocumented
- Where: justfile:328-352 (related: deny.toml:4, rust-toolchain.toml:3-4, justfile:14-18, justfile:389-390, justfile:979-986, .github/workflows/ci.yml:110-120, crates/before/wasm32-pins/Cargo.lock)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`find . -name Cargo.lock` lists six lockfiles under the tree; `grep -c '^name = "wasmtime' crates/before/wasm32-pins/Cargo.lock` = 10; read every cited range; `git log -S'wasm32-pins' -- justfile` shows eb6ba627 as the only touch and its message wires the leg into the gate's wasm stream only; `gh run view 33429688343 --log-failed` shows the 2026-08-31 instruments red was cargo-audit on `lru` and `wasmtime`); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no
- Witness (witness/results.md): demonstrated (mechanical; no cargo audit run). The four-name enumeration recurs at deny.toml:4, rust-toolchain.toml:3, and justfile:330 without wasm32-pins; a sixth lockfile exists at crates/before/wasm32-pins/Cargo.lock and the recipe at justfile:347-351 never audits it. The planted-advisory construction was not run.
- Cross-references: deps-1 (the omitted lock carries advisory-listed wasmtime 47.0.3), fuzz-guests-pins-37 (the same omission seen from the fuzz partition), fuzz-guests-pins-1 (the leg's absence from CI).

The supply-chain leg says it sweeps every lockfile in the repository but audits five; crates/before/wasm32-pins/Cargo.lock, which carries wasmtime (the crate whose advisory turned the instruments job red on 2026-08-31), is never audited. The same four-name enumeration recurs in deny.toml, rust-toolchain.toml, the gate-streams collision argument, and the CI instruments job's "what stays local" roster, none of which names wasm32-pins; the leg runs only in the gate's wasm stream and in none of `ci`, `instruments`, or `all`.

Evidence:

       329	# cargo-audit sweeps every lockfile in the repository — the root workspace
       330	# and each detached workspace (fuzz, fuzzfit, fuelscape, surfacecheck) —
       347	    cargo audit
       348	    cargo audit --file crates/before/fuzz/Cargo.lock
       349	    cargo audit --file crates/before/fuzzfit/Cargo.lock
       350	    cargo audit --file crates/before-fuelscape/Cargo.lock
       351	    cargo audit --file crates/before/surfacecheck/Cargo.lock

    deny.toml
         4	# detached workspaces (fuzz, fuzzfit, fuelscape, surfacecheck) are

    rust-toolchain.toml
         3	# directory, so the fuzz, fuzzfit, fuelscape, and surfacecheck
         4	# workspaces all take it too.

    .github/workflows/ci.yml
       117	  #   - the wasmtime fuel tier (the fuzzfit bands and the fuelscape pins):
       118	  #     deterministic, but it brings a wasm32 guest build plus wasmtime to
       119	  #     police asymptotics the board legs below already judge at the scales
       120	  #     of record — the gate keeps the second jaw.

Resolution: Add `cargo audit --file crates/before/wasm32-pins/Cargo.lock` to `supply-chain`, or drive the list from `find . -name Cargo.lock -not -path '*/target/*'` inside the recipe so the enumeration cannot rot again; fix the four prose rosters; then either add `wasm32-pins` to the `instruments` job or add it to that job's "what stays local, and why" comment with the reason (justfile:634-636 says it peaks at a few GiB across nextest workers). Acceptance: `just supply-chain` names every lockfile `find` reports, and every roster that enumerates detached workspaces names wasm32-pins or is replaced by a mechanical enumeration.
Construction: Add a crate with a current RustSec advisory to crates/before/wasm32-pins/Cargo.toml, regenerate its lockfile, and run `just supply-chain`: exit 0.

### gate-legs-4: manifestlint is a gate lint that CI never runs
- Where: justfile:1000-1000 (related: justfile:403, justfile:198-209, .github/workflows/ci.yml:107-108)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read justfile:403 and :1000; `git show 84044759 -- justfile` shows `gate-lints` gaining `manifestlint` while the `ci` line is untouched; `grep -rn manifestlint .github/` empty); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`gate-lints` includes manifestlint but the `ci` recipe, which the GitHub `ci` job runs verbatim, omits it, and neither of the other jobs runs it; a member manifest restating a version lands green in CI whenever the committer skips the gate. The omission dates from the commit that added the tool.

Evidence:

       403	gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare mutants-list readme-check
      1000	ci: fmt-check doclint testdoc workflowlint digestshare readme-check fuelscape-claims mutants-list clippy clippy-default features wasm-check docs docs-internal test-all citecheck doctest bench-build fuzz-build fuelscape-verify viz

    git show 84044759 -- justfile
    -gate-lints: fmt-check doclint testdoc workflowlint digestshare mutants-list readme-check
    +gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare mutants-list readme-check

Resolution: Add `manifestlint` to the `ci` line (build-free, seconds), and consider defining `ci` as `gate-lints` plus its build legs so the two rosters cannot diverge again. Acceptance: every recipe named in `gate-lints` appears in `ci`, ideally by construction.

### gate-legs-6: No recipe runs the mutation campaign, so the roster's kill claims are never re-attested
- Where: .cargo/mutants.toml:26-29 (related: .cargo/mutants.toml:48-51, justfile:294-326, tools/mutantcheck:21-23, tools/mutantcheck:54-55)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read .cargo/mutants.toml, tools/mutantcheck, justfile:294-326 and 1000-1003; `just --list` shows no campaign recipe; `sort -u target/mutants-raw.txt | wc -l` = 54,918; `.agent-notes/2026-08-21-cbor-wire-mutants-scopeA/README.md` records a scoped campaign of 3141 mutants at 16 jobs); executed: no
- Verification: confirmed, resolution reframed for scale; history: no-rationale-found (the roster's 2026-08-18 commits describe campaigns run by hand; nothing records a cadence or the last run)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (mechanical). The only `cargo mutants` invocations in the justfile are the two `--list` runs at justfile:324-325; no recipe in `gate`, `ci`, or `all` runs a campaign. The witness agent's own ad hoc mutant runs in this review found two survivors: the `read_bits` chunk arm at build.rs:297 (codec-bits-15) and the latent-annihilation ordering in `drop_below` (skyline-watermark-24).
- Cross-references: codec-bits-15 and skyline-watermark-24 (the two survivors), suanpan-40 (exclusions whose premises the campaign would have tested).

The roster header names `cargo mutants --workspace` as the campaign of record and lists mutants that named instruments kill, but the only cargo-mutants invocations in the tree are the two `--list` runs; the checker's own docstring leaves "a full campaign" as the authority on any mutant's fate while nothing runs one. A test weakened so that one of the "deliberately absent" mutants survives produces no signal in `gate`, `ci`, or `all`. At 54,918 distinct listed mutants, a full workspace campaign is not a per-commit leg, which is exactly why its cadence needs to be decided and recorded rather than left implicit.

Evidence:

        26	# Campaign configuration of record — the keys below make the plain
        27	# invocation (`cargo mutants --workspace`) run it: nextest, all
        28	# features, the whole workspace's suites, under cargo's default
        48	# Deliberately absent: sweep::eq_exit's `||`-guard mutant and
        49	# integral::meter_product's tap deletion. Their owning instruments kill
        50	# them — the eq_exit early-exit row and the settle-products liveness
        51	# floor in tests/meter.rs — so they need no exclusion. Likewise absent:

    justfile
       324	    cargo mutants --list --colors=never --no-config --workspace > target/mutants-raw.txt
       325	    cargo mutants --list --colors=never --workspace > target/mutants-filtered.txt

    tools/mutantcheck
        21	silently swallows every NEW mutant the function later grows — both rot
        22	invisibly because no gate leg runs cargo-mutants at all. This checker
        54	and a full campaign remains the authority on any individual mutant's
        55	fate.

Resolution: Add a campaign recipe of record (`cargo mutants --workspace` under the roster's keys) and a cheaper `--in-diff` variant for PRs; give the full campaign a cadence that fits its cost (a scheduled, sharded CI job, or a committed dated attestation of the last full run that a lint leg holds fresh), and record the expected outcome (zero MISSED under the roster) so a survivor is a red, not a report. Acceptance: a recipe exists whose exit status is the campaign verdict, and something in the tree or CI names when it last ran.
Construction: Delete the `eq_exit` early-exit row assertion in tests/meter.rs and run `just gate && just ci && just all`: all green; only `cargo mutants --workspace` (or a `--in-diff` campaign over that change) reports the survivor.

### gate-legs-8: Five writer-sink rows in the surface roster are excluded on all three legs with empty pins; their claimed doctest pin is unenforceable
- Where: crates/before/src/surface.rs:506-511 (related: crates/before/src/surface.rs:745-750, crates/before/src/surface.rs:785-790, crates/before/src/surface.rs:797-802, crates/before/src/surface.rs:987-992, crates/before/src/surface.rs:74-85, crates/before/src/surface.rs:239-245, crates/before/src/version/rank.rs:412-420, crates/before/src/testing/surface_coverage/tests.rs:212-217)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of every `pins: &[]` row in surface.rs; read the five rows and their buffer-door neighbors; grep for `encode_to(` in every tests.rs under crates/before/src shows one named test, `encode_to_matches_encode` at codec/tests.rs:812, covering Party, Version, and Clock only, cited at surface.rs:245; the five doors' byte-identity assertions live only in doctests at version.rs:1088, rank.rs:418, ranked.rs:182, ranked.rs:231, span/wire.rs:68; the coverage suite extends its resolvable set only from `pins`); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no
- Witness (witness/results.md): demonstrated (reading). The five rows cite empty pin lists on all three legs; the coverage suite extends its name set only from `pins` (surface_coverage/tests.rs:214-217) and citecheck resolves against the nextest inventory, which doctests are outside of; `encode_to_matches_encode` covers Party, Version, and Clock only (codec/tests.rs:812-832). The delete-the-doctest construction was not run.
- Cross-references: surface-roster-6 (the same five rows; note that its exposure clause for `Span::encode_to` was refuted by the witness, while the empty-pin roster hole recorded here stands).

`Version::encode_rank_to`, `Rank::encode_to`, `Ranked::encode_to`, `Ranked::encode_rank_to`, and `Span::encode_to` carry `NoWireFormatInReferences { pins: &[] }` on prod_tree, prod_fs, and tree_fs alike, so the row cites nothing checkable. The family's documentation says each writer-sink door is pinned by its doctest, but doctests are outside both the nextest inventory citecheck resolves against and the coverage suite's payload check; deleting those doctests leaves every gate leg green. The Party/Version/Clock writer doors show the intended shape: `codec_row` carries CODEC_PINS on prod_tree and `encode_to_matches_encode` is cited by name.

Evidence:

       506	    SurfaceRow {
       507	        op: "Version::encode_rank_to",
       508	        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
       509	        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
       510	        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
       511	    },
        80	    /// format goldens, and, for each writer-sink door, its doctest
        81	    /// pinning byte identity with the buffer door.
       245	            pins: &["encode_to_matches_encode"],

    crates/before/src/testing/surface_coverage/tests.rs
       214	                Exclusion::NoWireFormatInReferences { pins }
       215	                | Exclusion::DefinitionalCombinator { pins }
       216	                | Exclusion::NAryNotInReferences { pins }
       217	                | Exclusion::LinearityMechanics { pins } => names.extend(*pins),

    crates/before/src/version/rank.rs
       418	    /// rank.encode_to(&mut buf).unwrap();
       419	    /// assert_eq!(buf, rank.encode());

Resolution: Extend `encode_to_matches_encode` (or add one law over every `*_to` door) to cover the rank, ranked, and span writers, cite it in each of the five rows' `pins`, and add a coverage-suite assertion that a row excluded on all three legs cites at least one resolvable name, so the vacuous shape cannot recur. Acceptance: no roster row has three excluded legs with an empty union of pins, and the new assertion fails when one is introduced.
Construction: Delete the `# Example` block at rank.rs:412-420 and run `just gate`: green (no leg names `Rank::encode_to`).

**Adequacy of the resource instruments (meter-adequacy)**

### meter-adequacy-1: Public shape walks carry linear-cost claims with no enforcing instrument
- Where: crates/before/src/meter/board/coverage.rs:610-633 (related: crates/before/src/version.rs:734-740, crates/before/src/party.rs:476-481, crates/before/src/clock.rs:684-691, crates/before/src/shape.rs:314-317, crates/before-fuelscape/src/ops.rs:380-406, crates/before-fuelscape/src/lib.rs:16-20, crates/before/fuzzfit/guest/src/lib.rs:742-800, crates/before/src/lib.rs:344-358, crates/before/src/meter/board/coverage.rs:271-278)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep over tests/meter.rs, src/testing/asymptotics.rs, fuzzfit/harness, src/shape/tests.rs for shape()/combine/counters: no hits; read of the NA table, the four rustdocs, the fuelscape lib doc and OpSpecs, the guest exports); executed: no
- Verification: confirmed; history: no-rationale-found (the shape surface landed in 46eb64f9 on 2026-08-19; the design document its message cites is not in the tree; the fuelscape panels followed in 3c64f08a; nothing recorded excuses the walks from enforcement beyond the NA strings)
- Owner-gated: no (the enforcement home is a board row group or fuzz-fit ops; either is instrument work, not API)
- Witness (witness/results.md): demonstrated (run). With `Plateaus::next` rebuilt to re-open the stored stream from position 0 on every item (identical output, quadratic drain), all 33 tests matching shape/plateau/combine/region/cells pass, and no `.shape()` call exists in the meter suites, the board's operation table, or the benches. `just gate` was not run.
- Cross-references: clock-9 (the same walks' cost contracts from the clock side), recursion-6 (no deep-input test for the shape iterators either).

`Version::shape`, `Party::shape`, and `Clock::shape` document a linear drain
cost and `shape::combine`'s fuelscape contract states `O(N · total input
size)`, while the crate docs make every asymptotic claim a hard guarantee.
The four operations are excused from the board with prose reasons that
describe a linear walk (exactly the cost class the board prices), have no
`tests/meter.rs` row, and no fuzz-fit band; the only measurement of them is
the fuelscape atlas, which declares itself audit-only. `shape::combine`'s
own `# Complexity` section holds the chart and no stated bound.

Evidence:

       610	    (
       611	        "Version::shape",
       612	        "a linear single-pass read of the stored stream: one topology read \
       613	         and one payload decode per plateau, no arithmetic and no output \
       614	         re-coding; a wide rise materializes at the width its own code \
       615	         already spells",
       616	    ),
       ...
       628	    (
       629	        "shape::combine",
       630	        "the input walks advanced together under the overlay law: each \
       631	         stored bit read once, and the cell count is bounded by the inputs' \
       632	         total plateau count",
       633	    ),

    crates/before/src/version.rs
       738	    /// Draining the iterator is linear in the version's encoded size:
       739	    /// each plateau costs `O(1)` plus its own rise's encoded width, and
       740	    /// the walk itself performs no arithmetic.

    crates/before/src/shape.rs
       314	/// # Complexity
       315	///
       316	#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/shape_combine.html"))]
       317	///
       318	/// # Example

    crates/before-fuelscape/src/lib.rs
        16	//! **The atlas is an audit view, not enforcement.** Its committed checks
        17	//! are the sampler-correctness pins, the coverage parity pin, and a
        18	//! pipeline smoke test, nothing else: no fuel threshold, percentile gate,
        19	//! or band is ever minted from atlas data — the envelope suite and the
        20	//! fuzz-fit bands own enforcement.

    crates/before/src/lib.rs
       351	//! pathological input shapes. Any asymptotic claim is a hard guarantee that the
       352	//! operation will perform in time proportionate to that bound, for all input
       353	//! sizes, no matter how unlikely and contorted the shape of the input.

Resolution: give the four walks an enforcing home. The cheapest is a board
row group (`version_shape`, `party_shape`, `clock_shape`, `shape_combine`)
draining the iterators under the scan floor; the family bundles already
supply the operands, and the tiling test then reclassifies the four NA
entries as priced. The alternative is four fuzz-fit `Op` variants (the guest
kernels `ff_version_shape`, `ff_party_shape`, `ff_clock_shape`,
`ff_shape_combine` already exist) plus a re-pin. Separately, state
`shape::combine`'s bound in its rustdoc, since the crate docs promise a
Big-O on every operation. The sweep's proposed keyword pin on NA reason
text is not recommended: `BOARD_NOT_APPLICABLE` is `&[(&str, &str)]`, so the
tiling test judges membership only, and a string-content lint would be a
convention held in a regex. Acceptance: a committed board or fuzz-fit test
that reads red under the construction below, and the four NA entries gone.
Construction: regress `crate::shape::Plateaus::next` to re-scan the stored
stream from position 0 on every item (quadratic drain). Run `just gate`:
the board has no shape row, `tests/meter.rs` has no shape scenario, the
fuzz-fit program vocabulary has no shape op, and `fuelscape-test` asserts
sampler correctness only, so the gate stays green.

### meter-adequacy-2: The fuzz-fit bands cover 44 kernels while the justfile calls them the cost law for every public operation
- Where: justfile:564-571 (related: crates/before/fuzzfit/harness/src/bands.rs:74-79, crates/before/fuzzfit/harness/tests/sanity.rs:87-98, crates/before/src/testing/validation_index.rs:121-128, crates/before/fuzzfit/guest/src/lib.rs:1485-2014)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep and comm over guest exports vs banded kernels; Op enum variant count; fuelscape kernel count; the commit messages that added the span, query, and rank kernels); executed: no
- Verification: confirmed with corrected counts (56 measured public-operation kernels unbanded, not 63; the other 7 unbanded exports are supporting code and the self-test burner); history: no-rationale-found (the span/query/rank/n-ary/Eq-Hash kernels were added for fuelscape panels in 1f829fc7b, 54fdf8b53, and 2732a53ae; those messages decide fuelscape exemptions, never fuzz-fit band membership; the fuzz-fit design note's scope paragraph concerns operand construction, not the operation vocabulary)
- Owner-gated: yes for extending the program vocabulary (budgets, strategies, re-pin); no for the justfile wording and a surface-tiling test
- Cross-references: fuzzfit-strategies-7 (the same gap counted from the harness side).

`bands.rs` states its own reach accurately: 44 kernels, 49 band keys. The
guest exports 56 further measured kernels with no band and no `Op` variant:
the Span algebra (`ff_span_*`, `ff_own_span_*`), `Ranked`/`Rank` codec and
order, the `causally` query family (`ff_query_*`, `ff_floor_contains`,
`ff_ceiling_contains`), `ff_version_span`/`span_all`/`ticks`, the n-ary
clock and party folds, `ff_version_eq`/`hash`/`party_hash`, and the four
shape walks. The only totality pin (`bands_and_op_roster_name_the_same_kernels`)
binds bands to the `Op` enum, so nothing ties fuzz-fit's reach to
`before::surface` the way the board and fuelscape tiling tests do, and the
justfile's description overclaims.

Evidence:

    justfile
       568	# load) and judges every step against the pinned per-operation fuel bands
       569	# in harness/src/bands.rs — the committed cost law for every public
       570	# operation, so a change that moves an operation's asymptotics fails here
       571	# and re-pins deliberately (`just fuzzfit-calibrate`) instead of drifting.

    crates/before/fuzzfit/harness/src/bands.rs
        76	//! the toolchain in [`PINNED_RUSTC`], wasmtime 47 fuel. 49 band keys: 44
        77	//! kernels, of which five have sampled rejection arms (`clock_join` and

    crates/before/fuzzfit/harness/tests/sanity.rs
        94	/// sample the hole. The roster is one representative op per `Op`
        95	/// variant: a variant added to the vocabulary belongs in this list, and
        96	/// its kernel in the pinned bands.

Resolution: (a) re-state justfile:569-570 to what the bands cover (the
vocabulary in `bands.rs`, not every public operation); (b) add a
surface-tiling test to the fuzz-fit harness in the fuelscape idiom (every
`before::surface` row either reached by some `Op` kernel or carrying a
reasoned exemption), so the 56 uncovered kernels fail by name until each
gains an `Op` or an exemption; (c) owner's call on which operations join
the vocabulary. Acceptance: the tiling test committed and green with an
explicit exemption table; the justfile comment no longer says "every
public operation".

Synthesis note: The two counts differ by construction, not by disagreement: this entry counts 56 measured exports without a band; fuzzfit-strategies-7 further excludes the 7 query constructors whose docs say "unmeasured preparation" and reaches 49 operations with no `Op`. Both rest on the same `comm` over the guest exports.

### meter-adequacy-4: The bench judge's judged set is unpinned: any unrostered cell may leave judgment as SKIP with no diff
- Where: tools/benchjudge:82-85 (related: tools/benchjudge:157-160, tools/benchjudge:271-285, tools/benchjudge:537-546, tools/benchjudge:845, crates/before/src/meter/board.rs:106-113, crates/before/src/meter/board/floors.rs:99-112, crates/before/tests/bench_judge_roster.rs:72-83)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read of `roster_violations`, the SKIP row construction, the self-test assertion at 845, the roster schema pin); executed: no
- Verification: reframed: the mechanism is as the sweep states, but the free GREEN/SKIP drift is a documented design choice grounded in noise near the 10 µs floor, and an exact pin of the SKIP set would put a threshold over a noisy quantity, which the owner's doctrine forbids; severity lowered from medium because a regression drives a cell's median up into judgment, not out of it, so the exposure needs a cell to first leave judgment (speedup, family edit) and then regress while staying under 10 µs at the hi scale; history: deliberate-and-holds for the drift rule (benchjudge:82-84 states its reason), no-rationale-found for leaving the expected sub-floor set as prose
- Owner-gated: no
- Cross-references: board-families-floors-judge-11 and board-frame-3 (the same all-NA cell roster, unpinned from the board side).

The time leg is documented as the one leg that bounds work no counter sees,
and floors.rs names four benign cells as the ones it never reaches. Nothing
pins that list: a cell whose hi-scale median falls under
`MIN_JUDGED_MEDIAN_NANOS` renders SKIP, and an unrostered SKIP is never a
violation, so the set of judged cells can shrink between commits without a
reviewable diff. The "four cells" is also a hand-maintained count in prose.

Evidence:

    tools/benchjudge
        82	Any unrostered cell reading RED is a violation (exit 1). Unrostered cells
        83	may drift between GREEN and SKIP freely — both are non-red, and
        84	near-floor cells cross the judgment floor with machine noise. A roster
       ...
       845	    assert judged({"op/a": linear, "op/tiny": sub_floor}, roster=roster()) == 0

    crates/before/src/meter/board/floors.rs
        99	//! Four cells are watched by neither leg, an exposure accepted here so it is
       100	//! stated rather than silent: `version_hash`, `party_hash`, `clock_hash`, and
       101	//! `version_eq` on the benign family. Hashing folds the stored canonical bytes

Resolution: pin the expected sub-floor set without pinning a noisy
threshold: add a `may_skip` expectation class to the roster (membership
pinned in `tests/bench_judge_roster.rs`) listing the cells accurately under
the floor; a listed cell may read GREEN or SKIP (so noise at the floor never
flips a verdict), an unlisted cell reading SKIP is a violation, and RED
stays a violation everywhere. Replace the prose count at floors.rs:99 with
a reference to that list. Acceptance: the roster carries the class, the
schema pin at bench_judge_roster.rs:82 admits it, and the self-test asserts
an unlisted SKIP exits 1. Construction: lower `BENIGN_BASE_CLOCKS`
(family.rs:359) until a benign cell's hi median falls under 10 µs; `just
bench-judge` renders it SKIP and exits 0 with the roster satisfied.

### meter-adequacy-5: The log-factor pins never assert their linear reference below the floor
- Where: crates/before/src/testing/asymptotics.rs:227-239 (related: crates/before/src/testing/asymptotics.rs:12-14, crates/before/src/testing/asymptotics.rs:113-123, crates/before/src/testing/asymptotics.rs:249-256, crates/before/src/testing/asymptotics.rs:330-338)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read of `door_scan_bits`, `assert_log_factor_alive`, and all five pin docs; grep confirms every test in the module is a presence pin, none holds a linear fold under a floor); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no
- Cross-references: testing-diff-gen-26 (the same missing assertion, demonstrated there by run).

Each `*_log_factor_is_alive` pin asserts scan growth at or above its floor
across a x4 population growth on the premise that the population's own
byte growth (the linear reference) sits below the floor. `door_scan_bits`
prints the byte count for a re-pinner and returns only the scan bits, so
the premise is never checked: a generator change that makes the population's
bytes grow faster than the floor would let a scan-linear fold pass every
pin. The party fold's doc says its floor "holds the narrowest gap".

Evidence:

       117	fn door_scan_bits<R>(name: &str, n: usize, input_bytes: usize, run: impl FnOnce() -> R) -> u64 {
       118	    crate::meter::reset_scan_bits();
       119	    std::hint::black_box(run());
       120	    let bits = crate::meter::scan_bits();
       121	    eprintln!("MEASURED {name}: n={n} input_bytes={input_bytes} scan_bits={bits}");
       122	    bits
       123	}
       ...
       230	fn assert_log_factor_alive(door: &str, lo: u64, hi: u64, min_growth: f64) {
       231	    let growth = hi as f64 / lo.max(1) as f64;
       232	    assert!(
       233	        growth >= min_growth,
       ...
       335	/// thinning the upper levels), so this floor holds the narrowest gap
       336	/// of the doors here; both endpoints are exact counters, so the gap
       337	/// is stable, not noisy. The floor sits midway between the two

Resolution: return `(scan_bits, input_bytes)` from `door_scan_bits` and
assert `hi_bytes / lo_bytes < min_growth` beside `growth >= min_growth`, so
each run checks that the floor still sits above the linear reference.
Optionally hold one committed left-fold kernel under each door's floor as
the family's known-bad. Acceptance: the pins fail under the construction.
Construction: scale `FOLD_DOOR_TEETH` with `n` (instead of holding it at 64)
so encoded bytes grow more than x5.3 across 256 -> 1024; a scan-linear
`join_all` then reads growth >= 5.29 and
`version_join_all_log_factor_is_alive` stays green with no log factor
present.

### meter-adequacy-6: Three flatness bands rest on a known-bad demonstration that exists only as prose
- Where: crates/before/tests/meter.rs:4544-4555 (related: crates/before/tests/meter.rs:4480-4495, crates/before/tests/meter.rs:4652-4665, crates/before/tests/meter.rs:4737-4747, crates/before/src/meter/registry.rs:762-791, crates/before/src/meter/board/family.rs:232-257, crates/before/tests/superlinear_tripwires.rs:1-16, crates/suanpan/Cargo.toml:22-28)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep of `_reads_superlinear` across before and suanpan: ten kernels, none for the weight comb, freeze parade, or tooth-tail mechanisms; suanpan's only cargo feature is `touch-meter`, so no committed feature or parameter disables certificate consumption, the write watermark, or exact-top maintenance); executed: no
- Verification: confirmed and extended: the sweep named two bands; the comparison-sweep tooth-tail band (4737-4747) rests on the same "local probe build" prose, and the module comment at 4480-4495 states the pattern for all three; history: no-rationale-found (the module comment says "the probe readings live in the pin commits", which records numbers, not a re-runnable kernel)
- Owner-gated: yes (committing a probe path means a test-only accumulator strategy inside suanpan, a design decision for that crate)
- Cross-references: meter-registry-tier2-7 and envelopes-a-22 (the same probe-build dependency seen from the registry and envelope partitions).

The weight-comb, freeze-parade, and tooth-tail bands each justify their
adequacy by a probe build that disabled one accumulator mechanism and read
quadratic. None of the three probes is in the tree; suanpan exposes no
switch to reproduce them; and `tests/superlinear_tripwires.rs` exists
precisely because "an adequacy kernel that binds nowhere is silently
deletable". These three witnesses bind nowhere by construction, and the
registry cites them as the families' coverage answer.

Evidence:

      4548	    /// Flat per packed byte across the doubling: this family never
      4549	    /// freezes, so no segment feed deposits, and its wide cycling pays
      4550	    /// one quick-register spill per lease epoch. With certificate
      4551	    /// consumption disabled (a local probe build whose scans step
      4552	    /// digit by digit), the reading goes quadratic — `n² + O(n)`
      4553	    /// touches — and fails the band, so this band is the before-level
      4554	    /// adequacy witness for the zero-run ledger.
       ...
      4741	    /// Flat per packed byte across the doubling. With the settled top
      4742	    /// replaced by the buffer's high water (a local probe build), the
      4743	    /// reading goes quadratic — `2(g + 1)` touches per boundary, the

    crates/before/src/meter/registry.rs
       769	    /// never-written run. A settlement scan that steps the gap digit by digit
       770	    /// goes quadratic here (demonstrated by a probe build with certificate
       771	    /// consumption disabled); consuming one

    crates/suanpan/Cargo.toml
        22	[features]
       ...
        28	touch-meter = []

Resolution: either commit the three probes as kernels (a test-only
accumulator strategy in suanpan that steps digit by digit, reads from
digit 0, or tracks the buffer high water; `_reads_superlinear_on_weight_comb`
and siblings in before; rows in `TRIPWIRE_ROSTER`) and cite them from the
band docs and registry, or drop the "adequacy witness" wording at all
three bands and the registry and state that the bands pin measured
flatness only. Acceptance: no band doc cites a demonstration that is not
in the tree.

### meter-adequacy-7: The board's version_eq cell compares operands that diverge at the first leaf payload, so its time leg never times a full-length compare
- Where: crates/before/src/meter/board/ops.rs:241-255 (related: crates/before/src/meter/board/family.rs:797-803, crates/before/src/codec/bits.rs:414-420, crates/before/src/version/skyline.rs:11-19, crates/before/src/meter/board/floors.rs:109-112, crates/before/src/meter/board/floors.rs:130-136, crates/before/tests/meter.rs:7566-7598)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read of the row, the bundle post-pass, `canonical_eq`, the skyline stream layout, and the NA reason; the divergence point follows from the layout, not from a run); executed: no
- Verification: confirmed with the mechanism made precise: `PartialEq` is `codec::canonical_eq`, which is `ptr_eq` or raw-slice equality (length check, then byte compare stopping at the first difference); the stream is preorder topology bits with the first leaf's absolute height as `gamma(v1)` and every later leaf as a delta, so a seed tick (every leaf +1) changes only `gamma(v1)` and leaves topology and deltas byte-identical; the two operands therefore share the descent to the first leaf and diverge at its payload (or differ in length when the gamma code widens). On every family without its own pairing the compare reads at most one descent plus one code, never both operands whole; history: no-rationale-found (fuelscape's Eq/Hash panels were built on equal pairs "so the compare runs the whole length" per 2732a53ae; the board row was not)
- Owner-gated: no

The version_eq row's floors are all NA and its disclosure names the bench
judge's time leg as the backstop that the compare stays linear, but the
operands the row times are `(v, v.tick(seed))` wherever the family built no
pair, and that pair's canonical bytes diverge at the first leaf payload.
The cheapest passing artifact (a quadratic byte compare) reads one code
and exits; no counter and no time exponent sees it. The full-length
compare the claim is about is built elsewhere in the suite (byte-equal,
buffer-distinct operands at tests/meter.rs:7570) but never priced.

Evidence:

    crates/before/src/meter/board/family.rs
       797	        if let Some(bytes) = &data.version {
       798	            let v = decode_version(bytes);
       799	            if data.version2.is_none() {
       800	                let mut w = v.clone();
       801	                w.tick(&Party::seed());
       802	                data.version2 = Some(w.encode());
       803	            }

    crates/before/src/codec/bits.rs
       414	pub(crate) fn canonical_eq(a: &Bits, b: &Bits) -> bool {
       ...
       419	    a.ptr_eq(b) || a.as_raw_slice() == b.as_raw_slice()

    crates/before/src/version/skyline.rs
        16	//! - **Leaf payloads**, in-stream at each leaf position: the first leaf's
        17	//!   absolute height as `gamma(v1)` (this crate's gamma codes every
        18	//!   natural, zero included), every later leaf as
        19	//!   `zigzag-gamma(vi − vi−1)` over consecutive leaves in preorder. The

    crates/before/src/meter/board/floors.rs
       132	pub(super) const NA_SCAN_EQ_BYTES: &str =
       133	    "decides same-form equality on the stored canonical bytes \
       134	     wholesale (the compare may legitimately stop at the first differing byte): no stream walk \
       135	     is in the contract; unlike the hash rows' small-operand exposure, eq operands grow without \
       136	     bound, so the bench judge's time leg is the backstop that the compare stays linear";

Resolution: give version_eq a byte-equal, buffer-distinct operand pair
(decode `v`'s bytes twice, as tests/meter.rs:7569-7570 does) so the compare
runs its full length and the time leg's backstop claim is true; keep the
ticked pair as a second cell if the early-exit arm is wanted. The hash rows
are unaffected (hashing folds the whole buffer regardless of operand
relation). Acceptance: the bench cell's median scales with operand bytes
across the two scales. Construction: replace the byte compare with one that
rescans `0..i` for each `i`; on every non-pair family the loop exits after
one step, and neither the board nor the judge moves.

### meter-adequacy-9: The denominator sidecar is stamped before any bench runs and criterion's estimates carry no stamp
- Where: crates/before/benches/board.rs:159-177 (related: crates/before/benches/common/sidecar.rs:149-153, tools/benchjudge:47-55, tools/benchjudge:396-411, justfile:840-846, crates/before/fuzzfit/harness/src/wasm.rs:99-108, crates/before/fuzzfit/harness/tests/enforce.rs:319-326, crates/before/wasm32-pins/harness/src/lib.rs:41-48)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read of the harness ordering, `write_denoms`, `read_median`, the recipe, and the guest path resolution); executed: no
- Verification: confirmed; the early write has a stated reason (sidecar.rs:149-152: the parent directory must exist before criterion creates it), which is a directory-creation reason, not an ordering requirement; the `bench-judge` recipe stops at the first failing line, so within one recipe a failed bench never reaches the judge; only manual invocations are exposed; history: no-rationale-found for the ordering itself
- Owner-gated: no
- Cross-references: benches-examples-6 (the sidecar stamp's binding, from the bench partition).

`write_denoms` stamps scale, profile, sampling, and tip, then the criterion
loops run and write each cell's `estimates.json` as it completes, with no
provenance field. A run interrupted after the sidecar write (or a hand-run
criterion filter) leaves a fresh stamp over partly stale medians, and the
judge's stamp cross-checks read the sidecar only. The fuzz-fit and
wasm32-pins harnesses have the same shape one level up: the guest wasm is
located by path and its provenance is checked only through the harness's
own `FUZZFIT_RUSTC_VERSION`.

Evidence:

    crates/before/benches/board.rs
       159	    sidecar::write_denoms(scale, denoms.iter().map(|(id, n, c)| (id.as_str(), *n, *c)));
       160	    let mut next = 0;
       161	    while next < cells.len() {

    tools/benchjudge
       396	def read_median(criterion_dir, name, baseline):
       397	    """Read one cell's median point estimate (ns) from a saved baseline."""
       ...
       407	        return estimates["median"]["point_estimate"]

    crates/before/fuzzfit/harness/tests/enforce.rs
       320	fn building_toolchain_matches_the_pin() {
       321	    assert_eq!(
       322	        PINNED_RUSTC,
       323	        env!("FUZZFIT_RUSTC_VERSION"),

Resolution: write the sidecar after `wide.bench(c)` returns (the directory
argument at sidecar.rs:149-152 holds either way, since `write_denoms`
creates the parent), or add a completion stamp the judge requires; have
the judge refuse an `estimates.json` older than the sidecar's write. For
the wasm guests, export a build identifier from the guest and assert it
from the harness. Acceptance: a judge run over a baseline pair in which one
cell's estimates predate the sidecar exits 2.

### meter-adequacy-11: Envelope scenarios pass under default features with the limb, scan, and touch columns compiled out
- Where: crates/before/tests/meter.rs:27-33 (related: crates/before/tests/meter.rs:363-408, crates/before/Cargo.toml dev-dependencies `before = { workspace = true, features = ["oracle", "meter"] }`, crates/before/Cargo.toml:110-117, crates/before/src/meter/board/judge.rs:249-255, crates/before/src/meter/board/measure.rs:66-69, justfile:108-114)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (the self-dev-dependency enables `meter` but not `limb-meter`/`scan-meter`, so `just test` and `cargo nextest run -p before` build the meter binary with the counter columns cfg'd out; `metered()` gates every limb assert and the MEASURED line's columns on the feature); executed: no
- Verification: confirmed; history: deliberate-and-holds for the gate (`just test-all` and the gate pass `--all-features`; `amp_board` requires the features so its silent-green case is a cargo error); the envelope suite's quiet alternative is documented at 32-33 but has no marker a re-pinner would notice
- Owner-gated: yes (which mechanism: refuse default features in the binary, or mark the absent columns)

The board treats absent counters as loud (the example requires the
features) or as unjudged (the judge never floor-trips a `None` reading).
The envelope suite runs and passes on heap and segments alone, with the
only signal a shorter MEASURED line. Not a gate green-wash, but the MEASURED
lines are what a re-pinner reads, and a re-pin taken from a default-features
run would commit a limb column of zeros.

Evidence:

    crates/before/tests/meter.rs
        32	//!   the only column that sees a magnitude-quadratic regression. Without
        33	//!   the feature the scenarios still run and assert the other two columns.
       ...
       378	    #[cfg(not(feature = "limb-meter"))]
       379	    eprintln!(
       380	        "MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap} segments={segments}"
       381	    );

    crates/before/Cargo.toml
       110	# The board example judges limb, scan, and touch work against pinned
       111	# floors and ceilings; built without the counter features those columns
       112	# read `off` and every one of their verdicts silently renders green.

Resolution: either a `compile_error!` under
`not(all(feature = "limb-meter", feature = "scan-meter"))` at the top of
tests/meter.rs (with `just test` passing the features for `-p before`), or
an explicit `limb_ops=off scan_bits=off touches=off` marker on every
MEASURED line so absence cannot read as zero. Acceptance: a
default-features run either fails to build the meter binary or prints the
marker on every MEASURED line.

**Nits in this section** (full records in the evidence files)

| id | where | claim | resolution |
|---|---|---|---|
| meter-adequacy-10 | `crates/before/tests/meter.rs:8168-8197` | The meet-fold band's `touches >= bytes` is labelled a liveness floor with no mechanism for one touch per operand byte. | Derive from nonzero deltas times first-level merges, or relabel it a measured band (the suite's own "improvement tripwire" class). |

## Other cross-cutting sweeps

**Dependencies, lockfiles, and the build script (deps)**

### deps-1: supply-chain leg audits five of six lockfiles; the omitted wasm32-pins lock carries advisory-listed wasmtime 47.0.3
- Where: justfile:328-352 (related: justfile:390, deny.toml:4, rust-toolchain.toml:2-4, crates/before/wasm32-pins/Cargo.lock:952-953, crates/before/wasm32-pins/harness/Cargo.toml:14-17)
- Class / severity / confidence: verification-gap / high / high
- Provenance: verified (`git ls-files -- '*Cargo.lock'` lists six locks; grep of the wasm32-pins lock; the two advisory files' `patched` ranges; `git log --diff-filter=A` dates; `git show 4e64a4fb`); executed: no (cargo audit not permitted; the version-to-range comparison is mechanical)
- Verification: confirmed; history: deliberate-but-expired (the recipe's enumeration was complete when written on 2026-07-31 in 4da4779f; wasm32-pins landed 2026-08-18 in eb6ba627 and no enumeration moved)
- Owner-gated: no
- Cross-references: gate-legs-3 and fuzz-guests-pins-37 (the same omission; this entry adds the live advisory), deps-9 (the same lock's wasmtime pin diverging from its siblings).

The recipe promises an audit of every lockfile but names the detached
workspaces by hand and stops at four, so `crates/before/wasm32-pins/Cargo.lock`
is never audited. That lock resolves wasmtime 47.0.3, which RUSTSEC-2026-0268
and RUSTSEC-2026-0269 both list with `patched = [..., ">= 47.0.4"]`; the
2026-08-31 advisory bump (4e64a4fb) reached fuzzfit and fuelscape only and
its message counts "all five lockfiles". The same four-workspace roster is
restated at justfile:330, justfile:390, deny.toml:4, and
rust-toolchain.toml:3.

Evidence:

       329	# cargo-audit sweeps every lockfile in the repository — the root workspace
       330	# and each detached workspace (fuzz, fuzzfit, fuelscape, surfacecheck) —
       ...
       347	    cargo audit
       348	    cargo audit --file crates/before/fuzz/Cargo.lock
       349	    cargo audit --file crates/before/fuzzfit/Cargo.lock
       350	    cargo audit --file crates/before-fuelscape/Cargo.lock
       351	    cargo audit --file crates/before/surfacecheck/Cargo.lock
       352	    cargo deny --workspace check bans

    crates/before/wasm32-pins/Cargo.lock
       952	name = "wasmtime"
       953	version = "47.0.3"

    ~/.cargo/advisory-db/crates/wasmtime/RUSTSEC-2026-0269.md
    patched = [
        ">= 24.0.13, < 25.0.0",
        ">= 36.0.14, < 37.0.0",
        ">= 46.0.3, < 47.0.0",
        ">= 47.0.4",
    ]

    commit 4e64a4fb (2026-08-31): "just supply-chain clean across
    all five lockfiles with no warnings."

    deny.toml
         4	# detached workspaces (fuzz, fuzzfit, fuelscape, surfacecheck) are
    rust-toolchain.toml
         3	# directory, so the fuzz, fuzzfit, fuelscape, and surfacecheck

Reachability, stated plainly: the harness takes wasmtime with
`default-features = false, features = ["cranelift", "runtime"]`
(harness/Cargo.toml:14-17) and no WASI, and both advisories concern WASI
paths, so the vulnerable code is most likely not compiled. cargo-audit judges
by version, which is the rule the gate adopted at justfile:332 ("a
vulnerability anywhere fails the gate"), and by that rule the gate is green
over a lock it should refuse. The principle breached is the no-hand-maintained
enumeration rule: five prose and recipe restatements of one roster all
drifted silently when a fifth detached workspace landed.

Resolution: enumerate lockfiles mechanically in the recipe body (a loop over
`git ls-files -z -- '*Cargo.lock'` feeding `cargo audit --file`), so a new
detached workspace is audited the commit its lock lands; re-denominate the
four prose enumerations to "every committed Cargo.lock"; bump wasm32-pins to
wasmtime 47.0.4 (`cargo update -p wasmtime` inside the workspace) and run
`just wasm32-pins` to confirm every pin's outcome is unchanged. Acceptance:
`just supply-chain` at HEAD fails naming RUSTSEC-2026-0269 in
crates/before/wasm32-pins/Cargo.lock; after the bump it passes; a seventh
lockfile added anywhere in the tree is audited with no recipe edit.

### deps-2: the deny leg's stated scope (one version per crate over the whole graph) exceeds what it enforces; dev-only duplicates pass by cargo-deny's default
- Where: justfile:334-339 (related: deny.toml:1-24, Cargo.lock:1864-1876 and 1885-1896 and 2515-2525, Cargo.toml:142 and 150 and 168)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (lock inspection with a scan of dependency arrays; cargo-deny bans documentation fetched; `git show 4e64a4fb` attests the check passed with these entries present); executed: no (cargo deny not permitted)
- Verification: confirmed; history: no-rationale-found (deny.toml's header states the whole resolved graph; no comment names a dev exemption)
- Owner-gated: no (the policy choice is the owner's, but prose and mechanism must agree either way)
- Witness (witness/results.md): inconclusive. The lock structure was verified by reading (rand 0.9 and rand_chacha 0.9 enter via proptest, thiserror 1 via the wezterm crates under ratatui, all dev-only; deny.toml sets no `multiple-versions-include-dev`); cargo-deny's default was taken from its documentation and the decisive `cargo deny` runs were not executed.

The root lock carries rand 0.8.6 and 0.9.4, rand_chacha 0.3.1 and 0.9.0,
rand_core 0.6.4 and 0.9.5, and thiserror 1.0.69 and 2.0.18. The 0.9 rand
line enters only through proptest 1.11 (a dev-dependency of every member);
thiserror 1 enters only through rumors' dev-dependency ratatui (via termwiz,
filedescriptor, and the wezterm crates). cargo-deny's documented default
excludes crates reachable only over dev edges from duplicate detection, so
`just supply-chain` passes while the header and recipe comment promise one
version per crate across the full closure and a roster of every tolerated
duplicate.

Evidence:

    justfile
       334	# for triage without failing. cargo-deny holds the root workspace's
       335	# resolved graph (all members, all features, all targets) to one version
       336	# per crate; deny.toml is the roster of record, every tolerated duplicate
    deny.toml
         2	# which runs `cargo deny check bans`). Scope: the root workspace's full
         3	# feature-and-target closure — exactly the graph Cargo.lock records. The
        24	skip = []
    Cargo.lock
      1864	name = "rand"
      1865	version = "0.8.6"
      1875	name = "rand"
      1876	version = "0.9.4"
      2515	name = "thiserror"
      2516	version = "1.0.69"
      2524	name = "thiserror"
      2525	version = "2.0.18"
    Cargo.toml (rumors)
       142	rand = { workspace = true }
       150	proptest = { workspace = true }
       168	ratatui = { workspace = true }

    cargo-deny bans configuration, `multiple-versions-include-dev`:
    "If `true`, `dev-dependencies` are included when checking for multiple
    versions of crates. By default this is false, and any crates that are
    only reached via dev dependency edges are ignored when checking for
    multiple versions."

The header's own cost argument ("A crate resolved at two versions compiles
twice, doubles its audit surface") is paid by every gate test build for
these four pairs, and a reader trusting the comment believes rand and
thiserror are converged. Principle 8 (verified vs told) and the
no-unrostered-exemption rule are what this breaches.

Resolution: choose and make the text match. Either (a) set
`multiple-versions-include-dev = true` under `[bans]` and roster the
holdouts with their reasons (rand 0.8 held by the workspace pin against
proptest's 0.9; thiserror 1 through ratatui's termwiz chain), which restores
the header's promise and lets the roster shrink visibly as they converge; or
(b) state the dev-edge exemption in both comments. The rand convergence path
(workspace `rand`/`rand_chacha` to 0.9) touches rumors' normal dependency at
Cargo.toml:142 and reseeds every corpus drawn through `gen_range`, so it is
a separate, owner-ruled decision and is listed under open questions rather
than proposed here. Acceptance: with option (a), `cargo deny --workspace
check bans` at HEAD reports rand, rand_chacha, rand_core, and thiserror and
passes once the skip entries name them; with option (b), the deny.toml and
justfile comments both say dev-only duplicates are out of scope.

Construction: run `cargo deny --workspace check bans` at HEAD (passes), then
again with `multiple-versions-include-dev = true` added under `[bans]`: it
reports the four crates above as duplicates.

### deps-3: build.rs's figure-freshness check cannot fire on an incremental build; its two inputs are absent from rerun-if-changed
- Where: crates/before/build.rs:27-30 (related: crates/before/build.rs:93-100, crates/before/build.rs:183-197, .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md:159)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read build.rs in full; `git log -S` dates the rerun list to 0a8884ee on 2026-08-13 and both figure reads to 2efff149 on 2026-08-14); executed: no (a build is not permitted)
- Verification: confirmed; history: no-rationale-found (the design note specifies `cargo:rerun-if-changed` on `fuelscape/` and `docs/`; the implementation lists three files under docs/, and the figure check landed a day later without extending the list)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (build). After a warm build, editing a color literal in results/space_consumption/itc_space_consumption.svg and rebuilding yields `Fresh before`; adding `cargo:rerun-if-changed` for the two figure files makes the next build rerun the script and fail at build.rs:194 with the documented stale message. Both scratch edits were restored.
- Cross-references: crate-root-5 and fuelscape-render-30 (the same defect filed independently by two partitions; this entry carries the executed demonstration).

Once a build script prints any `cargo:rerun-if-changed`, cargo reruns it only
when a listed path (or a `rerun-if-env-changed` variable, or the script
itself) changes. The list names the fuelscape directory and three files
under docs/. `main` also reads `results/space_consumption/itc_space_consumption.svg`
and `check_readme_figure_fresh` reads `docs/itc_space_consumption_readme.svg`;
neither is listed, so editing either after a warm build neither reruns the
script nor refreshes `$OUT_DIR/space_consumption.svg`, and the staleness the
check exists to catch is caught only by a clean build.

Evidence:

        27	    println!("cargo:rerun-if-changed=fuelscape");
        28	    println!("cargo:rerun-if-changed=docs/fuelscape.css");
        29	    println!("cargo:rerun-if-changed=docs/fuelscape.js");
        30	    println!("cargo:rerun-if-changed=docs/fuelscape-header.html");
        ...
        93	    let figure = std::fs::read_to_string("results/space_consumption/itc_space_consumption.svg")
        ...
       178	/// The README references the figure by URL, so it must be a committed
       179	/// file; committed-but-derived means it can rot, and this check is what
       180	/// prevents that — the header-freshness idiom. Regenerate with
        ...
       184	    println!("cargo:rerun-if-env-changed=BEFORE_REGEN_DOC_FIGURE");
       185	    let path = "docs/itc_space_consumption_readme.svg";
        ...
       191	    let have = std::fs::read_to_string(path).unwrap_or_default();

    .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md
       159	- `cargo:rerun-if-changed` on `fuelscape/` and `docs/`.

A freshness check is a meter, and this one has a liveness hole exactly on the
edit it guards (instruments before cures: every meter needs a liveness
floor). The doc comment at 178-180 says the check "is what prevents" rot; on
the developer's incremental loop it prevents nothing until an unrelated
listed file changes.

Resolution: add `cargo:rerun-if-changed=results/space_consumption/itc_space_consumption.svg`
and `cargo:rerun-if-changed=docs/itc_space_consumption_readme.svg` beside the
existing four, or list the `docs/` directory as the design note specifies
(directory entries are scanned recursively by mtime). Acceptance: the
construction below fails at the second build.

Construction: `cargo build -p before` (warm), change one color literal in
results/space_consumption/itc_space_consumption.svg, build again: at HEAD the
script does not rerun and no error appears; with the fix, the second build
fails with "docs/itc_space_consumption_readme.svg is stale relative to
results/space_consumption: run `just doc-figure`".

### deps-9: detached-workspace lockfiles resolve shared crates at different versions than the root, with no stated convergence policy
- Where: crates/before/wasm32-pins/Cargo.lock:73-74 (related: crates/before/fuzz/Cargo.lock:39-40 and 94-95, crates/before/surfacecheck/Cargo.lock:37-38, crates/before-fuelscape/Cargo.lock:484-485, tools/manifestlint:20-22)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (grep of every committed lock for dashu-int, borsh, bytes, thiserror, wasmtime); executed: no
- Verification: confirmed; history: no-rationale-found (grep of `cargo update` across .agent-notes, justfile, and AGENTS.md finds no stated policy)
- Owner-gated: no

The `before` the fuzz targets, the 32-bit pins, the surface check, and the
fuelscape exercise links dependency versions the gate's native suites never
build: dashu-int 0.5.1 (fuzz, wasm32-pins, surfacecheck, fuelscape) against
0.5.0 (root, fuzzfit); borsh 1.8.0 (fuzz, wasm32-pins) against 1.6.1 (root);
bytes 1.12.1 (all five detached) against 1.11.1 (root); wasmtime 47.0.3
(wasm32-pins) against 47.0.4 (fuzzfit, fuelscape). The differential fuzz
target compares borsh 1.8.0's `deserialize_reader` against raw decodes while
the shipped graph carries 1.6.1; the 32-bit pins exercise the big-integer
backend's word cap against dashu-int 0.5.1 while production builds 0.5.0.
manifestlint stops at the root workspace by design.

Evidence:

    grep of `name = "dashu-int"` / `version` across committed locks:
      Cargo.lock: "0.5.0"
      crates/before-fuelscape/Cargo.lock: "0.5.1"
      crates/before/fuzz/Cargo.lock: "0.5.1"
      crates/before/fuzzfit/Cargo.lock: "0.5.0"
      crates/before/surfacecheck/Cargo.lock: "0.5.1"
      crates/before/wasm32-pins/Cargo.lock: "0.5.1"
    borsh: Cargo.lock "1.6.1"; fuzz and wasm32-pins "1.8.0"
    tools/manifestlint
        20	# library's TOML parser. Detached workspaces (the fuzz targets, with their
        21	# own Cargo.lock) are separate workspaces with their own tables, outside
        22	# this tool's scope.

Measurements bind to their run: a fuzz or pin verdict is evidence about the
shipped `before` to the extent the linked closure matches. Patch-level skew
is most likely benign here, but the property is unstated and unchecked, so a
actual divergence (a dashu-int behavior fix landing in one lock) would be
invisible.

Resolution: state the policy once, at the mechanical lockfile enumeration the
deps-1 fix introduces: either a convention ("every detached lock is updated
in the same commit as a root `cargo update`") or a cheap cross-lock diff in
that leg (a small tool over `git ls-files -- '*Cargo.lock'` reporting any
crate resolved at differing versions across locks; crates absent from the
root, such as wasmtime, need no entry). Acceptance: the policy sentence
exists in the recipe comment, and if the tool is taken, it fails at HEAD on
dashu-int, borsh, and bytes and passes after one `cargo update` sweep.

**Module graph and the production-instrument boundary (module-graph)**

### module-graph-1: The stack-segments column has a writer in exactly one binary; 84 envelope ceilings and the board's segments currency read a counter nothing can increment
- Where: crates/before/src/recurse.rs:100-109 (related: crates/before/src/recurse.rs:16-20, crates/before/src/recurse.rs:68-86, crates/before/src/recurse.rs:118-129, crates/before/src/lib.rs:421, crates/before/src/meter.rs:3543-3559, crates/before/tests/meter.rs:22-26, crates/before/tests/meter.rs:211-243, crates/before/tests/meter.rs:364-391, crates/before/src/meter/board/measure.rs:84-92, crates/before/src/meter/board/ceilings.rs:80-83, crates/before/src/meter/board/judge.rs:60-64, crates/before/src/meter/board/judge.rs:286-290, crates/before/src/meter/board/worst.rs:169, crates/before/src/meter/tests.rs:400-454, crates/before/Cargo.toml:49)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (writer/reader grep over src, tests, benches, examples; 84 envelope rows counted by kind and their second arguments extracted, all 0; blame and `git show` for the history); executed: no
- Verification: reframed: the counter is not dead everywhere. In the lib unit-test binary `cfg(test)` is set, `descend!` and `grow` exist, and `stack_segment_meter_counts_deterministically_and_resets` both drives the counter (meter/tests.rs:409-439) and holds `tick` on a deep spine to zero (meter/tests.rs:441-453); that is the one live pin. In the integration envelope suite (`tests/meter.rs`), the `amp_board` example, and the board bench, the library is compiled without `cfg(test)`, so `grow` and `descend!` do not exist and `SEGMENTS_GROWN` has no writer; history: deliberate-but-expired: 05bd2b16d chose to keep the counter compiled for the meters ("its own liveness now witnessed by a test-local guarded descent"), but `recurse.rs:74-76` (blame 5e166477c, older than the conversion) and `tests/meter.rs:22-26` (blame 0d1ea4905, older) still describe a growth arm that writes the counter in those builds.
- Owner-gated: yes: `meter::stack_segments` and `meter::reset_stack_segments` are `pub` under the `meter` feature, and `recurse.rs:16-20` records the keep decision as an owner ruling.
- Witness (witness/results.md): demonstrated (reading of the cfg attributes). `grow` (recurse.rs:100-109) and the `descend!` macro (recurse.rs:118) are `#[cfg(test)]`; the counter and its readers are `cfg(any(test, feature = "meter"))` (recurse.rs:68-86); every integration binary, example, and bench therefore reads a counter nothing in that build can increment. Neither the compile-failure construction nor the amp_board example was run.
- Cross-references: board-ops-render-15, crate-root-32, envelopes-a-2, recursion-1, inventory-2 (the segments cluster; see Crate-wide patterns).

`grow` is the only writer of `SEGMENTS_GROWN` and is `#[cfg(test)]`; the counter and its readers are
`cfg(any(test, feature = "meter"))`. Every binary that reads the counter through the `meter`
feature alone therefore reads a compile-time zero: the 84 `segments` ceilings in `tests/meter.rs`,
the board's `Currency::Segments` (ceiling `MAX_GROWN_STACK_SEGMENTS`, an exponent fit, a
floor-trip message, a worst-map column) all judge a quantity that cannot move. A production kernel
that regressed to recursion could not route through `descend!` (the macro does not exist outside
`cfg(test)`, so the crate would not compile), so the column detects nothing the depth-100k clock
test and the heap column do not.

Evidence:

       100	#[cfg(test)]
       101	#[inline]
       102	pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
       103	    if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
       104	        f()
       105	    } else {
       106	        SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);
       107	        stacker::grow(STACK_GROWTH, f)
       108	    }
       109	}
    --- recurse.rs:74-76 (predates 05bd2b16d) ---
        74	/// Compiled only for the meter surface: the counter is always written (the bump
        75	/// is inseparable from the growth arm), but nothing outside the meters ever
        76	/// reads it.
    --- recurse.rs:16-20 (the keep decision) ---
        16	//! The paper-shaped oracle is clearest written recursively, and the guard is
        17	//! what lets it meet deep inputs safely. The segment counter below stays
        18	//! compiled for the meters: it is the deterministic stand-in for
        19	//! recursion-driven stack consumption, and its zero reading over the library
        20	//! kernels is the measured fact the boards' segments column pins.
    --- tests/meter.rs:22-26 (predates 05bd2b16d) ---
        22	//! - **Grown stack segments** ([`meter::stack_segments`]): the deep
        23	//!   traversals grow the stack onto the heap in fixed-size segments that
        24	//!   bypass any allocator meter; the segment counter is the honest stand-in
        25	//!   for recursion-driven stack cost. Process-global, same isolation
        26	//!   requirement.
    --- tests/meter.rs:387-391 ---
       387	    assert!(
       388	        segments <= env.segments,
       389	        "{name}: {segments} grown stack segments exceed the pinned envelope {}: {ISOLATION_NOTE}",
       390	        env.segments,
       391	    );
    --- meter/board/ceilings.rs:80-83 ---
        80	/// Green requires at most this many grown stack segments, as an absolute count:
        81	/// the target is walks that never grow the stack, so the ceiling is flat, not
        82	/// per-byte.
        83	pub const MAX_GROWN_STACK_SEGMENTS: u64 = 1;
    --- meter/tests.rs:441-453 (the one live ratchet) ---
       441	    // The conversion ratchet: the fill walk pairs a deep spine on BOTH
       442	    // sides — exactly the descent that once grew the stack — and must now
       443	    // read zero, its depth on explicit heap stacks the heap meter prices.

Resolution: Two accurate dispositions, one of which the owner picks. (a) Retire the currency where it
cannot move: drop the `segments` field and column from `tests/meter.rs`'s four envelope kinds,
remove `Currency::Segments` and its ceiling, floor-trip string, judge arms, and worst-map arm from
the board, keep `stack_segment_meter_counts_deterministically_and_resets` as the guard's own unit
test and the tick pin, make `mod recurse` `#[cfg(test)]`, and remove `meter::stack_segments`
and `meter::reset_stack_segments`. (b) If a production `descend!` user is foreseen, gate `grow`,
`should_grow`, `descend!`, and the three constants under `any(test, feature = "meter")` so the
column has a writer in every build that reads it; the column still reads zero on every current
kernel, and the readers stay. Under either disposition, rewrite `recurse.rs:74-76` and
`tests/meter.rs:22-26` now so they describe the writer that exists in the build that reads the
counter. Acceptance: no binary carries a segments ceiling whose counter has no writer in that
binary; both doc passages name the actual writer.
Construction: Add to `tests/meter.rs` a helper that recurses through `before::recurse::descend!`
and a scenario expecting `segments > 0`. The binary fails to compile (`descend!` and `grow` are
`cfg(test)` and `pub(crate)`); that compile error is the demonstration that no reading in the
integration binary can be nonzero. Equivalently, every segments cell of `cargo run --release -p
before --example amp_board --features limb-meter,scan-meter` prints 0 on every ladder.

### module-graph-2: `clippy-default` never lints the default-feature `before` library
- Where: justfile:146-150 (related: justfile:129-143, justfile:507-518, crates/before/Cargo.toml:33-50, Cargo.toml:86, Cargo.toml:144-149, crates/suanpan/Cargo.toml:16-20, .github/workflows/ci.yml:107-108)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read the recipes and manifests; applied cargo's resolver-2 rule, which the root package's `edition = "2024"` selects; grep of ci.yml for `RUSTFLAGS` returns nothing); executed: no
- Verification: confirmed, with two refinements: the `features` recipe (`cargo check -p before --no-default-features`, justfile:511) does compile the bare lib and prints its warnings, but does not deny them, and no CI environment sets `RUSTFLAGS`; and the recipe comment at 140-143 attributes the bare-lib line to `cfg(test)`-gated modules, whereas a `--lib --tests` invocation compiles the lib target without `cfg(test)`, and what actually differs is the feature set the self-dev-dependency unifies (rumors: `test-internals`, `conformance` at Cargo.toml:145; before: `oracle`, `meter` at crates/before/Cargo.toml:49); history: no-rationale-found for omitting the before line; the comment at 129-138 names the very mechanism that defeats line 148.
- Owner-gated: no
- Witness (witness/results.md): inconclusive. The two facts the argument rests on were read (the self-dev-dependency at crates/before/Cargo.toml:49 lights oracle+meter on the lib whenever test targets build; justfile:148 is the only default-feature before line, with no bare `--lib` line); whether the bare default-feature lib is clean under `-D warnings` today needs a clippy run, which was not permitted.

Line 148 builds test targets, which activates the self-dev-dependency `before = { ..., features =
["oracle", "meter"] }` and unifies those features onto the lib in that invocation. The shipped
default-feature `before` lib is therefore held to `-D warnings` nowhere in the gate: `clippy` is
`--all-features`, `clippy-default`'s before line lights `oracle`+`meter`, and `features` checks
without denying. `suanpan` has no self-dev-dependency, so its `--lib --tests` line already lints a
default-feature lib; rumors gets the bare line; before does not.

Evidence:

       145	# Lint the default-feature library and test builds, warnings denied.
       146	clippy-default:
       147	    cargo clippy -p suanpan --lib --tests -- -D warnings
       148	    cargo clippy -p before --lib --tests -- -D warnings
       149	    cargo clippy -p rumors --lib --tests -- -D warnings
       150	    cargo clippy -p rumors --lib -- -D warnings
    --- justfile:129-131 ---
       129	# `clippy` above lints under --all-features, and the dev-dependency cycle
       130	# forces the meter/oracle features onto the lib for every test build — so a
       131	# surface that is dead under *default* features (test-only helpers left
    --- justfile:140-143 ---
       140	# The bare-lib rumors line lints the shipped configuration on its own: an
       141	# invocation that also builds test targets compiles the lib with
       142	# cfg(test)-gated modules alive, so an item dead only in the default-feature
       143	# lib — the artifact users actually build — never surfaces there.
    --- crates/before/Cargo.toml:49 ---
        49	before = { workspace = true, features = ["oracle", "meter"] }

Resolution: Add `cargo clippy -p before --lib -- -D warnings` to `clippy-default`, and re-state the
140-143 comment's mechanism as feature unification through the self-dev-dependency (the lib target
is compiled without `cfg(test)` either way). Acceptance: an ungated `pub(crate) fn` used only from a
`cfg(any(test, feature = "meter"))` site fails `just clippy-default` with `dead_code`.
Construction: Run `cargo clippy -p before --lib -- -D warnings` at HEAD. Whether or not it is clean
today, add the ungated helper described above and observe `just clippy` and the current
`just clippy-default` stay green while the bare-lib command reports `dead_code`.

### module-graph-8: The fuzz workspace's gate leg formats but never lints, unlike its four detached siblings
- Where: justfile:363-368 (related: justfile:588-597, justfile:636-644, justfile:658-663, justfile:939-945, crates/before/fuzz/src/lib.rs, crates/before/fuzz/fuzz_targets/)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read the five detached-workspace recipes; `wc -l` over fuzz/src and fuzz_targets gives 914 lines); executed: no
- Verification: confirmed; history: no-rationale-found: the recipe's own comment (363-364) justifies the fmt line by the same argument the sibling recipes use for fmt and clippy together, and gives no reason to stop at fmt.
- Owner-gated: no
- Cross-references: fuzz-guests-pins-39 (the same missing clippy leg, from the fuzz partition).

The justfile states the rule for detached workspaces (source the root clippy cannot reach rots
invisibly through green gates) and applies it in fuzzfit, wasm32-pins, fuelscape-test, and
surface-totality. The fuzz workspace, holding the shared heap-cap harness and five targets, is the
one detached tree without the clippy leg.

Evidence:

       363	# Build the libFuzzer targets (nightly). The fmt line is the detached
       364	# workspace's formatting leg: the root `cargo fmt --all` cannot reach it.
       365	[working-directory("crates/before/fuzz")]
       366	fuzz-build:
       367	    cargo fmt --check
       368	    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}
    --- justfile:588-592 ---
       588	# Run the fuzz-fit asymptotics suites against the pinned fuel bands. The
       589	# fmt/clippy lines are the detached workspace's own lint leg (the root
       590	# `cargo fmt --all`/clippy cannot reach a detached workspace, so without
       591	# them its source rots invisibly through green gates — the fuelscape and
       592	# surfacecheck recipes carry the same discipline).

Resolution: Add `cargo +{{ nightly_toolchain }} clippy --all-targets -- -D warnings` to
`fuzz-build` (the targets are ordinary `[[bin]]`s over `libfuzzer-sys`; clippy is check-only and
the recipe already has the nightly toolchain), and extend the 363-364 comment to cover both lines.
Cost: one clippy pass over 914 lines plus a check-only build of `before` with `laws,serde,borsh`
in the detached target dir. Acceptance: a clippy warning introduced in `fuzz/src/lib.rs` fails
`just gate`.

**Recursion and stack depth (recursion)**

### recursion-1: Grown-stack-segments meter is constant zero in every build that pins it
- Where: crates/before/src/recurse.rs:100-109 (related: crates/before/src/recurse.rs:16-20, crates/before/src/recurse.rs:68-69, crates/before/src/recurse.rs:118-119, crates/before/src/meter.rs:3543-3559, crates/before/tests/meter.rs:22-26, crates/before/tests/meter.rs:211-215, crates/before/tests/meter.rs:387-391, crates/before/tests/meter.rs:6131-6133, crates/before/src/meter/board.rs:47-49, crates/before/src/meter/board/ceilings.rs:80-83, crates/before/src/meter/board/measure.rs:84-92, crates/before/src/meter/board/judge.rs:286-290, crates/before/src/meter/board/floors.rs:83-85, crates/before/src/meter/tests.rs:392-453)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (cfg attributes read at every item in recurse.rs; single writer confirmed by grep; the 84 envelope rows' segments column read by regex; board drivers located by grep; stacker's dev-dependency placement read in Cargo.toml and stated in commit 1ddb5a483's message); executed: no
- Verification: confirmed; history: deliberate-but-expired (the keep decision at recurse.rs:16-20 was written in 1ddb5a483, the same commit that gated `grow` behind `cfg(test)`, and rests on a "measured fact" the pinning build cannot measure; floors.rs:83-85 records the ceiling-only floor policy but not the cfg gap; the amplification note (lines 1299-1301) still describes segment onset as detectable at scale)
- Owner-gated: yes (the dissolution removes `meter::stack_segments`/`reset_stack_segments`, public functions under the `meter` feature, and a board currency; the alternative keeps them)
- Cross-references: board-ops-render-15, crate-root-32, envelopes-a-2, module-graph-1, inventory-2 (the segments cluster).

`SEGMENTS_GROWN` is incremented only inside `recurse::grow`, which is
`#[cfg(test)]` and reachable only through the `#[cfg(test)]` `descend!` macro
that no library code calls. `tests/meter.rs`, `tests/amp_board_smoke.rs`,
`benches/board.rs`, and `examples/amp_board.rs` compile the library as a
dependency, without `cfg(test)` and without `stacker`, so `stack_segments()`
can only read 0 there: every `segments <= env.segments` assert (all 84 rows pin
0) and `MAX_GROWN_STACK_SEGMENTS` pass vacuously, and a library walk that
regressed to native recursion would still read 0 (it overflows instead of
growing a segment). The liveness witness at `meter/tests.rs:407-439` proves the
counter counts in the unit-test binary for a test-local guarded descent, which
is not the build the column is pinned in; and the "conversion ratchet" at
`meter/tests.rs:441-453` asserts zero for the fill walk in that same binary,
where a natively recursive fill walk would also read zero, so it does not
detect the regression it names either (the depth-50k `tick` crashing on
overflow is the effective detector). The doctrine breached: a ceiling over a counter
that cannot count passes vacuously, and a criterion needs a committed
demonstration that a known-bad mechanism fails it; the prose at six sites
presents the structural zero as a measurement.

Evidence:

       100	#[cfg(test)]
       101	#[inline]
       102	pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
       103	    if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
       104	        f()
       105	    } else {
       106	        SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);
       107	        stacker::grow(STACK_GROWTH, f)
       108	    }
       109	}

        68	#[cfg(any(test, feature = "meter"))]
        69	static SEGMENTS_GROWN: AtomicU64 = AtomicU64::new(0);

       118	#[cfg(test)]
       119	macro_rules! descend {

        17	//! what lets it meet deep inputs safely. The segment counter below stays
        18	//! compiled for the meters: it is the deterministic stand-in for
        19	//! recursion-driven stack consumption, and its zero reading over the library
        20	//! kernels is the measured fact the boards' segments column pins.

    tests/meter.rs:
       387	    assert!(
       388	        segments <= env.segments,
       389	        "{name}: {segments} grown stack segments exceed the pinned envelope {}: {ISOLATION_NOTE}",
       390	        env.segments,
       391	    );

    ceilings.rs:
        83	pub const MAX_GROWN_STACK_SEGMENTS: u64 = 1;

    meter/tests.rs:
       399	/// Every library walk is iterative, so the meter's liveness needs its own
       400	/// witness: a test-local descent routed through `recurse::descend!` (the same
       401	/// guard the test-only oracle bridge walks use) deep enough to outrun the
       402	/// thread stack. Without that leg, the boards' all-zero segments column could
       403	/// be a dead counter instead of a measured fact.

    floors.rs:
        83	//! - **Segments** is ceiling-only by policy: the target is walks that never
        84	//!   grow the stack, so its honest floor is zero and a zero floor asserts
        85	//!   nothing.

Resolution: Either dissolve the segments currency from the envelope suite and
the board (`Envelope.segments` and the segments columns of the other envelope
structs, `MAX_GROWN_STACK_SEGMENTS`, `Currency::Segments` and its NA policy
declarations, `meter::stack_segments`/`reset_stack_segments`), naming the
depth-100k/250k tests as the no-recursion detector where the column was cited
(recurse.rs:16-20, tests/meter.rs:22-26 and 6131-6133, board.rs:47-49), and
keep `SEGMENTS_GROWN` under `cfg(test)` only if the `meter/tests.rs` dive stays
as a test of the guard itself (re-word recurse.rs:16-20 to say it measures the
test-surface guard, not the library kernels); or keep the column and land a
committed known-bad demonstration that moves it in the meter build, which today
cannot exist without un-gating `descend!` and restoring `stacker` as a
dependency. Acceptance: either the segments currency is gone from
`tests/meter.rs` and the board with the detector named in its place, or a
committed known-bad demonstration exists in the meter build whose segments
reading exceeds the ceiling.

### recursion-6: Public shape iterators have no committed deep-input test
- Where: crates/before/src/shape.rs:146-155 (related: crates/before/src/shape.rs:333-338, crates/before/src/shape/tests.rs:24-65, crates/before/src/testing/generators.rs:293-298, crates/before/src/clock/tests.rs:653-664, crates/before/src/version/skyline/shape.rs:44-52, crates/before/src/version/skyline/overlay.rs:317-324, crates/before/fuzzfit/guest/src/lib.rs:742-785)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep for `.shape()` and `combine([` over the deep tests, `tests/`, `benches/`, `examples/`, and the board returned no hits; `shape/tests.rs` and the walk structs read); executed: no
- Verification: reframed: the sweep's companion claim that `Party::without` has no 100k-scale test is dropped (tests/meter.rs:6172-6185 and 6250-6261 drive the public door at `ID_DEPTH = 250_000`); the shape-door gap stands, with the qualification that the fuzzfit guest drains all three doors over whatever registers the fit pipeline stages (lib.rs:748, 763, 778), which is a fit, not a committed depth-100k stack-safety test; history: no-rationale-found
- Owner-gated: no
- Cross-references: meter-adequacy-1 (no cost instrument for the same walks), clock-22 (the depth-100k proof's roster).

`Version::shape` (`Plateaus`), `Party::shape` (`Regions`), `Clock::shape`
(`Overlay`), and `shape::combine` (`Cells`) are public walks over the stored
streams, yet none appears in the three depth-100k clock tests, the 250k-depth
envelope rows, or the board; their in-crate drivers are `shape/tests.rs` over
`arb_oracle_version` (recursion cap `ARB_DEPTH = 4`) and the op-trace
differentials. They are iterative by construction (`VersionWalk`/`PartyWalk`
over `LeafCursor`/`IdLeafCursor`, whose paths are `BitStack`s), so this is a
hole in the committed proof the crate docs and AGENTS.md rely on, not a
breach. Public `Party::without` is already proven at 250k.

Evidence:

       146	pub struct Plateaus<'a> {
       147	    walk: VersionWalk<'a>,
       148	    finished: bool,
       149	}

       333	pub fn combine<'a, const N: usize>(versions: [&'a Version; N]) -> Cells<'a, N> {
       334	    Cells {
       335	        walks: versions.map(|version| VersionWalk::open(version.view().live())),

    shape/tests.rs:
        32	    fn join_and_meet_are_pointwise_extrema(
        33	        a in generators::arb_oracle_version(),
        34	        b in generators::arb_oracle_version(),

    generators.rs:
       298	const ARB_DEPTH: u32 = 4;

    overlay.rs:
       317	pub(super) struct LeafCursor<'a> {
       318	    cursor: DsiCursor<'a>,
       319	    /// Root-to-leaf branch directions, root first.
       320	    path: BitStack,

    clock/tests.rs:
       661	/// `deep_tree_stack_safety` above proves the clock ops at this depth; this is
       662	/// the same proof for the surfaces it does not drive — every one an iterative
       663	/// walk whose depth lives on explicit heap or bit stacks, exercised here at a
       664	/// depth no program stack could carry.

Resolution: Extend `deep_tree_query_and_causal_stack_safety` (or add a sibling)
with the four shape doors over the deep clock, asserting item counts against
their closed forms (a depth-d left spine has d + 1 plateaus or regions):
`late.shape().count()`, `clock.party().shape().count()`,
`clock.shape().count()`, `combine([&early, &late]).count()`. Acceptance: a
committed test drives each public shape iterator over a depth-100k structure.

**Allow attributes, panic sites, visibility (inventory)**

### inventory-2: The segments column cannot fail on any library artifact in any build
- Where: crates/before/src/recurse.rs:100-109 and 118-129 (related: crates/before/src/recurse.rs:16-20 and 71-76, crates/before/Cargo.toml:33-44, crates/before/src/meter.rs:3543-3559, crates/before/tests/meter.rs:22-26 and 387-391, crates/before/src/meter/board/measure.rs:84-92, crates/before/src/meter/board/floors.rs:627-636, crates/before/src/meter/tests.rs:392-454, crates/before/src/clock/tests.rs:565-566)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read recurse.rs in full; read Cargo.toml's dependency tables; grep of `descend!`/`recurse::grow`/`stack_segments` users over src, tests, examples, benches returns only meter.rs's readers, the cfg(test) users in testing/bridge.rs, grow/tests.rs, meter/tests.rs, and the readers in tests/meter.rs and board/measure.rs; every `envelope`/`query_envelope`/`sweep_envelope`/`touch_envelope` constructor call in tests/meter.rs (84 at HEAD) passes 0 in its segments column, checked by extracting the second positional argument); executed: yes (the greps settle that no library kernel references the guard and that every pin is 0)
- Verification: reframed: the sweep found no writer under `--features meter`; the gap is wider. `descend!` and `grow` are `#[cfg(test)]` because `stacker` is a dev-dependency (1ddb5a483), so no library kernel can route through the guard in any build, and a library kernel that recursed natively would never touch the counter either. The counter therefore reads zero over library kernels by construction in every build, including the cfg(test) unit test; history: deliberate-but-expired: the pin was recorded as "the committed ratchet" (`.agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:555-558`) when library kernels still had guarded call sites; once every library walk became iterative and the guard test-only, no library artifact can move the counter, so the ratchet has nothing to catch
- Owner-gated: no (the meter surface is bench/test-only per Cargo.toml's feature comment)
- Cross-references: board-ops-render-15, crate-root-32, envelopes-a-2, module-graph-1, recursion-1 (the segments cluster).

Every envelope in `tests/meter.rs` asserts `segments <= env.segments` with the
pin at 0, the board carries a segments currency declared ceiling-only, and the
crate doc calls the zero "the measured fact the boards' segments column pins".
The only increment sits in `grow` under `#[cfg(test)]`, and `descend!`, the
only route to `grow`, is also `#[cfg(test)]`, so non-test library code cannot
reference it (it would not compile without cfg(test)). The unit test
`stack_segment_meter_counts_deterministically_and_resets` proves the counter
mechanism works over a test-local `dive`, but its library leg (`t.tick(&id)`
reads 0) is true by construction, not by measurement. The committed proof that
library walks do not recurse is `deep_tree_stack_safety` (clock/tests.rs:565,
depth 100 000), which would fail on a native recursion; the segments column
would not.

Evidence:

    recurse.rs:
        16	//! The paper-shaped oracle is clearest written recursively, and the guard is
        17	//! what lets it meet deep inputs safely. The segment counter below stays
        18	//! compiled for the meters: it is the deterministic stand-in for
        19	//! recursion-driven stack consumption, and its zero reading over the library
        20	//! kernels is the measured fact the boards' segments column pins.

        68	#[cfg(any(test, feature = "meter"))]
        69	static SEGMENTS_GROWN: AtomicU64 = AtomicU64::new(0);

       100	#[cfg(test)]
       101	#[inline]
       102	pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
       103	    if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
       104	        f()
       105	    } else {
       106	        SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);
       107	        stacker::grow(STACK_GROWTH, f)
       108	    }
       109	}

       118	#[cfg(test)]
       119	macro_rules! descend {
       ...
       128	#[cfg(test)]
       129	pub(crate) use descend;

    Cargo.toml:
        33	[dev-dependencies]
        ...
        44	stacker = { workspace = true }

    tests/meter.rs:
       387	    assert!(
       388	        segments <= env.segments,
       389	        "{name}: {segments} grown stack segments exceed the pinned envelope {}: {ISOLATION_NOTE}",
       390	        env.segments,
       391	    );

    floors.rs:
       629	const NA_SEG_CEILING_ONLY: &str = "ceiling-only by policy: the target is walks that never grow \
       630	     the stack, so the honest floor is zero and a zero floor asserts nothing";

    meter/tests.rs:
       399	/// Every library walk is iterative, so the meter's liveness needs its own
       400	/// witness: a test-local descent routed through `recurse::descend!` (the same
       401	/// guard the test-only oracle bridge walks use) deep enough to outrun the
       402	/// thread stack. Without that leg, the boards' all-zero segments column could
       403	/// be a dead counter instead of a measured fact.

Resolution: retire the segments column: drop the `segments` field and its
assert from the `Envelope`/`TouchEnvelope` harnesses in `tests/meter.rs`, the
`stack_segments` read in `board/measure.rs`, the segments currency and its
ceiling-only declaration in the board, the `meter::stack_segments` and
`reset_stack_segments` readers, and the `cfg(any(test, feature = "meter"))`
on the static and its accessors (leaving them `cfg(test)`); keep `grow`,
`descend!`, and the counter as the test-surface guard for the oracle bridge,
with `stack_segment_meter_counts_deterministically_and_resets` as that guard's
own liveness test; re-word recurse.rs:16-20 to state that the guard is
test-surface machinery and that `deep_tree_stack_safety` is the committed
no-recursion proof. The alternative, making the column able to fail, requires
promoting `stacker` to an optional dependency under `meter` and routing a
committed known-bad recursive library shape through it, which contradicts the
deliberate dev-dependency placement. Acceptance: no envelope, board cell, or
doc reports a segments reading; `deep_tree_stack_safety` remains the depth
proof; the guard's unit test still runs under `cargo nextest run -p before`.

**Public API audit (api-audit)**

### api-audit-7: auto_traits.rs claims to pin every public API type but omits Limbs, TooWide, the shape types, and the polarity markers
- Where: crates/before/src/auto_traits.rs:1-32 (related: shape.rs:88, shape.rs:108, shape.rs:118, shape.rs:128, shape.rs:146, shape.rs:197, shape.rs:255, shape.rs:345, version/ticks.rs:125, error.rs:54, causally/polarity.rs:225-240, surfacecheck/src/extract.rs:21-24, surfacecheck/src/extract.rs:267-269)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (compared the `crate::...` names in auto_traits.rs against the Structs/Enums sections of the sweep's rendered `target/doc/before/all.html`, restricted to ungated items); executed: yes: python extraction of the item lists from all.html
- Verification: confirmed, and strengthened: `surfacecheck/src/extract.rs:21-24` excludes compiler-synthesized auto-trait impls from the surface census on the premise that "`src/auto_traits.rs` asserts `Send + Sync + Unpin` on every public API type at compile time", so the census's exclusion rests on a totality the roster does not have; history: no-rationale-found
- Owner-gated: no
- Cross-references: crate-root-7 (the same roster gap; this entry adds the polarity markers' `Unpin`).

Ungated public types with no `assert_impl_all!` line: `Limbs`, `error::TooWide`,
`shape::{Plateau, Rise, Region, Cell, Plateaus, Regions, Overlay, Cells}`, and
`causally::{Down, Up, Neutral}` (the last three are covered for `Send + Sync`
by the `Polarity: sealed::Sealed + Send + Sync + 'static` supertrait bound, not
for `Unpin`). The shape iterators hold `VersionWalk`/`PartyWalk` fields, whose
auto traits are exactly what a pin exists to guard.

Evidence:

         1	//! Compile-time pins on the auto traits of every public API type.

    surfacecheck/src/extract.rs:
        21	//!   [`crate::census::TRAIT_IMPLS`]. Compiler-synthesized auto-trait
        22	//!   impls are excluded here because `before` pins those guarantees
        23	//!   directly (`src/auto_traits.rs` asserts `Send + Sync + Unpin` on
        24	//!   every public API type at compile time); blanket impls are excluded

Resolution: add the missing `assert_impl_all!` lines (`shape::Cell<1>` and `shape::Cells<'static, 1>` for the const-generic pair), or derive the roster from the surface census so it cannot drift; otherwise narrow the module doc and the two surfacecheck comments to what is pinned. Acceptance: every struct and enum in rustdoc's `all.html` for the default feature set appears in an `assert_impl_all!` line, or a committed check compares the two lists.

**Paper fidelity (paper-fidelity)**

### paper-fidelity-10: the paper's system-level freshness clause (e′ ≰ any other live x) is not pinned
- Where: crates/before/src/laws.rs:2453-2458 (related: crates/before/reference/itc2008.md:109-112, crates/before/src/oracle/tests.rs:88-100,234-253, crates/before/src/testing/semantic_oracle.rs:311-316, crates/before/src/testing/semantic_oracle/tests.rs:297-320)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (every `world_strategy` test in `oracle/tests.rs` listed by grep and the tick/receive/disjointness ones read; `semantic_oracle/tests.rs` 285-325 and `laws.rs` 2445-2470 read; grep for freshness vocabulary across `testing/`, `laws.rs`, `tests/`); executed: no
- Verification: confirmed; history: no-rationale-found (the function-space oracle states the property in prose at semantic_oracle.rs:313-315; nothing checks it)
- Owner-gated: no

§3's event condition has three clauses: strict advance (pinned: `tick_strictly_advances`, `tick_advances`), minimality in its scoped form (pinned: `grow_dominates_no_more_than_needed`), and freshness against every other live stamp. The laws' single-value and pair groups cannot express the third (it quantifies over a population), and no trace test asserts that a fresh tick is dominated by no other live clock's version or the ownership invariant it rests on. `event_dominates_local_and_advances` samples one stamp.

Evidence:

    reference/itc2008.md:
       111	that e′ is not dominated by any other entity and does not dominate more events than needed: for any
       112	other event component x in the system, e′ ≰ x and when x < e′ then x ≤ e. In version vectors the

    crates/before/src/laws.rs:
      2453	    /// `tick` strictly advances the causal order: `a < a.tick(p)`.

    crates/before/src/testing/semantic_oracle.rs:
       313	/// still tracks happens-before — it meets the §3 event condition (the result is
       314	/// fresh, `e' ≰` any other live stamp, and dominates nothing new, because the
       315	/// id owns its region exclusively) — so the causal order is identical to

Resolution: add a population law beside `disjointness_invariant` (oracle/tests.rs:238) and an impl-side trace test: for every live clock `i` in a world, after `cs[i].tick()`, `!(cs[i].version() <= cs[j].version())` for all `j ≠ i`; equivalently the ownership invariant `cs[j].version() / cs[i].party() <= cs[i].version() / cs[i].party()`. Both ride the existing `world_strategy` populations. Acceptance: a committed test fails when `tick` is replaced by an inflation that another clock could already hold (for example, a tick that raises the whole version to the join of all live versions).

Construction: proptest over `world_strategy()`: `let cs = run(&ops); for i in 0..cs.len() { let fresh = { let mut c = cs[i].clone(); c.tick(); c.version().clone() }; for (j, other) in cs.iter().enumerate() { if j != i { prop_assert!(!leq(&fresh, other.version()), "fresh tick of {i} already known to {j}"); } } }`.

**rumors dependence (rumors-dependence)**

### rumors-dependence-3: The `as_bytes == encode` laws, tests, and the roster pin they anchor are tautological: `encode` is `as_bytes().to_vec()`
- Where: crates/before/src/laws.rs:419-422 (related: crates/before/src/laws.rs:2155-2158, crates/before/src/version.rs:1027-1029, crates/before/src/party.rs:554-556, crates/before/src/party/tests.rs:982-1003, crates/before/src/version/tests.rs:625-651, crates/before/src/clock/tests.rs:296-328, crates/before/src/surface.rs:196-205, crates/before/src/testing/surface_coverage/tests.rs:24-28, crates/before/src/laws.rs:404-410, crates/before/src/laws.rs:2141-2146, crates/before/src/version.rs:1174-1180, crates/before/src/party.rs:698-704, tests/bookmark_causality.rs:1075-1094)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the definitions of `encode` on both types and the git history of the seam; the two sides of each assertion are one expression by definition, so no run is needed); executed: no
- Verification: confirmed and extended to the roster: `surface.rs:203` cites `as_bytes_matches_encode` as one of three production-side pins the codec rows' exclusions rest on, and `clock/tests.rs:300-305` describes a divergence between `as_bytes` and `encode` that the current definition makes impossible; history: deliberate-but-expired: at d0e54d955 (which added `as_bytes` and these tests) `encode` packed independently through `codec::pack_to_writer`, so the comparison discriminated; 24ff18b93 redefined `encode` as `self.as_bytes().to_vec()`, and nothing re-denominated the tests
- Owner-gated: no for re-denominating test and law bodies under their existing names; renaming or removing a law changes the names public under the `laws` feature, which the owner rules on
- Cross-references: surface-roster-6 and gate-legs-8 (other roster pins that certify less than their names say).

The laws `version_as_bytes_matches_encode` and `party_as_bytes_matches_encode`, the tests `as_bytes_matches_encode` and `as_bytes_matches_encode_after_fork` (party) and `as_bytes_matches_encode` and `as_bytes_matches_encode_after_ticks` (version), and two assertions inside `encoding_views_agree_over_impl_history` all compare `x.as_bytes()` with `x.encode()`, and `encode` on both types is `self.as_bytes().to_vec()`, so each comparison is a slice against its own copy and cannot fail. The stated invariant (stored padding is canonical after every construction and mutation path) is discharged today by the `debug_assert!` inside `as_bytes` (which the campaign configuration of record does run, per `.cargo/mutants.toml`'s header) and, independently, by the strict-decode legs: `version_codec_roundtrip`, `party_codec_roundtrip`, and the `decode(as_bytes)` assertions in `encoding_views_agree_over_impl_history`, all of which reject a non-canonical tail through `require_marker_padding` (`version.rs:1118`, `party.rs:631`). Principle 3 (circular justification) and Principle 6 (the cheapest passing artifact): an assertion whose two sides are one expression is decoration, and a roster pin citing it certifies nothing. The property is what rumors hashes into leaf paths and ships on the wire; rumors' own regression `retire_into_rebooted_absorber_absorbs_cleanly` guards the seam from outside through strict decode, not through these.

Evidence:

    crates/before/src/laws.rs:
       419	    /// The borrowed byte view is the encoding: `as_bytes == encode`.
       420	    fn version_as_bytes_matches_encode {
       421	        a.as_bytes() == &a.encode()[..]
       422	    }

    crates/before/src/version.rs:
      1027	    pub fn encode(&self) -> Vec<u8> {
      1028	        self.as_bytes().to_vec()
      1029	    }

    crates/before/src/party.rs:
       554	    pub fn encode(&self) -> Vec<u8> {
       555	        self.as_bytes().to_vec()
       556	    }

    crates/before/src/surface.rs:
       201	const CODEC_PINS: &[&str] = &[
       202	    "decode_encode_arbitrary",
       203	    "as_bytes_matches_encode",
       204	    "decode_never_panics",
       205	];

    crates/before/src/clock/tests.rs:
       301	    /// only ever compare `encode`d bytes; the `as_bytes_matches_encode` tests
       302	    /// only build via the oracle. Neither combination drives the impl's own
       303	    /// `fork`/`join`/`sync` *and* reads `as_bytes` — the exact seam where a
       304	    /// normalizing `join` once left stale bits in the stored buffer, so that
       305	    /// `as_bytes` (the borsh wire form) diverged from the canonical `encode`.

    version.rs at d0e54d955 (history, for provenance):
       131	    pub fn encode(&self) -> Vec<u8> {
       132	        let mut bytes = Vec::new();
       133	        self.encode_to(&mut bytes)
       134	            .expect("writing to a Vec is infallible");
       135	        bytes
       136	    }
       146	    pub fn encode_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
       147	        codec::pack_to_writer(&self.0, writer)

Resolution: re-denominate each body against the independent judge while keeping the names, so the roster and the duplicate-name table stay untouched: in `laws.rs`, `Version::decode(a.as_bytes()).is_ok_and(|d| d.as_bytes() == a.as_bytes())` (and the party twin; `laws.rs` has no field access); in the party and version test modules, `codec::padding_is_canonical(&v.0)` or the same decode form, so `after_fork` and `after_ticks` assert what their docs say without relying on the debug assertion. Re-state `clock/tests.rs:300-305` in the present tense (what it protects: stored padding after impl-driven `fork`/`join`/`sync`, judged by strict decode). Alternatively delete the two laws and drop the pin from `CODEC_PINS`, since the roundtrip laws already cover it. Acceptance: with the `debug_assert!` in `as_bytes` disabled and an unsealed tail introduced after `join` (the historical seam), the re-denominated tests fail; today they cannot.

Construction: temporarily replace the body of `as_bytes` with a plain `self.0.as_raw_slice()` and make `Party::join` skip `seal_padding` on its output (or hand-build a `Party` whose stored buffer carries stale bits after the marker). The current `as_bytes_matches_encode*` tests and both laws stay green on that mutant; `party_codec_roundtrip` and the re-denominated forms go red.

**Suite economics (suite-economics)**

### suite-economics-1: Process-per-test isolation is a documented premise with no runtime check
- Where: crates/before/tests/meter.rs:15-21 (related: crates/before/tests/meter.rs:351-355 and 363-371; crates/before/src/meter/tests.rs:21-25; crates/before/src/meter.rs:3549-3551; crates/before/src/codec/base/tests.rs:59-61; crates/before/tests/answer_embedded.rs:31-41; crates/before/tests/fold_skeleton.rs:20-30; crates/before/tests/coincident_span.rs:20-24; crates/suanpan/tests/amortized_sequences.rs:30-34; .cargo/mutants.toml:78-80; justfile:1034 and 1041)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of the crates for `NEXTEST`, `process-per-test`, and the `ISOLATION_NOTE` constants; `strings` on /opt/homebrew/bin/cargo-nextest shows `NEXTEST_EXECUTION_MODE` and `process-per-test`; that nextest sets the variable to `process-per-test` in its default mode is nextest's documented behavior, assessed, not fetched); executed: no
- Verification: reframed: the suite already anticipates a shared-process runner, but only as a diagnostic appended to failures (`ISOLATION_NOTE` in tests/meter.rs and src/meter/tests.rs). That covers the loud direction (a neighbor's allocations inflate a reading and a ceiling fails with the note attached) and not the silent one (a neighbor's `reset_peak_usage` inside a scenario window, or a neighbor freeing memory that sat in the scenario's baseline, lowers the reading so a ceiling passes on a peak that never covered its scenario); history: no-rationale-found
- Owner-gated: no
- Witness (witness/results.md): inconclusive. Reading confirms the premise (tests/meter.rs:17-21 states the requirement, `ISOLATION_NOTE` at :354-355 decorates failures only, `metered` at :363-367 resets process-global meters with no runtime check); readings under a shared-process runner were not observed.
- Cross-references: envelopes-a-3, envelopes-b-10, meter-registry-tier2-19, testing-diff-gen-27, board-ops-render-27, suanpan-tests-12 (the isolation cluster; see Crate-wide patterns).

Every metered reading in tests/meter.rs, the process-global counters read by answer_embedded.rs, fold_skeleton.rs, and coincident_span.rs, and suanpan's touch-meter tests are meaningful only when each test runs in its own process. The suite states this in prose at the meter modules and appends it to failures, but nothing checks it, so under `cargo test`, an IDE runner, or any future execution mode that shares a process the ceilings can pass vacuously while floors and tripwires fire spuriously. The gate honors the premise by convention (.cargo/mutants.toml:78 `test_tool = "nextest"`; justfile:1034 and 1041 `cargo llvm-cov nextest`), which is exactly why a silent breach elsewhere would go unnoticed. This breaches the doctrine that every hole becomes a committed check, never a convention held in memory, and the warning that "we are the only writer" premises can be falsified by the environment without any programmer erring.

Evidence:

        15	//! - **Peak heap bytes**: the binary-wide counting allocator
        16	//!   ([`PeakAlloc`]), read as a delta over the scenario body. One global
        17	//!   allocator exists per test binary, and the counters are process-global,
        18	//!   so per-scenario peaks are meaningful **only under nextest's
        19	//!   process-per-test isolation** — this workspace's runner. Under a runner
        20	//!   that shares one process across tests, concurrent allocation would bleed
        21	//!   between scenarios.

       351	/// Appended to every envelope failure: the first cause to rule out is a
       352	/// shared-process test runner, under which the process-global meters bleed
       353	/// other tests' work into the scenario being measured.
       354	const ISOLATION_NOTE: &str = "note: the meters are process-global and meaningful only one \
       355	     scenario per process: run under cargo nextest, not a shared-process cargo test";

       363	fn metered<R>(name: &str, input_bytes: usize, env: &Envelope, f: impl FnOnce() -> R) -> R {
       364	    meter::reset_stack_segments();
       365	    #[cfg(feature = "limb-meter")]
       366	    meter::reset_limb_ops();
       367	    HEAP.reset_peak_usage();

Resolution: add one guard and call it from every metering helper: `metered` (tests/meter.rs:363), `ticks_counters` (769-781), `identity_fast_paths::scanned` (10556), coincident_span's `scanned`, the two `counters` helpers, suanpan's `touches`, and the codec/base touch pin. The guard asserts `std::env::var("NEXTEST_EXECUTION_MODE").as_deref() == Ok("process-per-test")` and fails with a message naming the required runner; `before::meter` behind the `meter` feature is a natural single home so suanpan and the satellite binaries share it. Acceptance: the nextest gate is unchanged, and `cargo test -p before --all-features --test meter` fails every metered scenario immediately with the isolation message instead of reporting readings.
Construction: run `cargo test -p before --all-features --test meter` (libtest's default shares one process across parallel threads, one `PeakAlloc`). Without the guard, readings differ from the nextest run and some ceilings or floors flip nondeterministically between runs; with the guard, every scenario fails at its first metered call.

### suite-economics-6: Two #[ignore]d tests are run by no recipe
- Where: crates/before/src/codec/tests.rs:1959-1976 (related: crates/before/src/codec/text.rs:192-194; crates/before/src/testing/exhaustive/tests.rs:436-446; justfile:1000-1003; .config/nextest.toml:26)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn 'run-ignored\|--ignored\|include-ignored' justfile .github .config` returns nothing; the sweep's run.log shows both tests SKIP); executed: no
- Verification: confirmed and reframed toward the cheaper fix: the depth counter is an explicit `i64` whose non-overflow argument sits inline at text.rs:192-193, so the dormant witness pins less than its doc argues (it pushes past i32::MAX only, not "any physically representable input"), and it discriminates an i32 counter only in the dev profile, where the overflow panics; in release the wrapped counter goes negative at the closing paren and the parse returns `Err(Parse::Syntax)` exactly as the test expects; history: no-rationale-found
- Owner-gated: no
- Cross-references: testing-oracles-24 (the `exhaustive_deep` recipe).

Neither the justfile, the CI workflows, nor nextest.toml runs ignored tests, so `clock_text_split_survives_two_gib_of_parens` and `exhaustive_deep` are compiled and never executed by any sweep; a board nothing enforces is decoration. The deep enumeration is hour-scale and says so. The 2 GiB witness costs seconds and could ride `just all`, or the property it pins could be asserted at the counter without the allocation.

Evidence:

      1966	/// Ignored because the witness string alone costs ~2 GiB of memory; run it
      1967	/// deliberately with `--run-ignored all`.
      1968	#[test]
      1969	#[ignore = "allocates ~2 GiB to push the depth counter past i32::MAX"]
      1970	fn clock_text_split_survives_two_gib_of_parens() {

    (crates/before/src/codec/text.rs)
       192	    // i64 cannot overflow: depth moves by at most one per input byte, and an
       193	    // allocation holds at most `isize::MAX` (< 2⁶³) bytes.
       194	    let mut depth: i64 = 0;

Resolution: either add a filtered ignored run to the `all` recipe (`cargo nextest run -p before --all-features --run-ignored only -E 'test(clock_text_split_survives_two_gib_of_parens)'`), or pin the width at the site (name the counter type and add a `const` assertion that its bit width is at least 64, with the argument at text.rs:192-193 restated beside it) and retire the witness. exhaustive_deep stays as documented. Acceptance: `just all`'s nextest output shows the witness as PASS rather than SKIP, or the compile-time pin exists and the `#[ignore]` test is gone.

## Crate root and public types

**Crate root: lib docs, error, build.rs, Cargo.toml, auto traits**

### crate-root-5: build.rs declares `rerun-if-changed` for the fuelscape inputs only, so the README-figure freshness check does not rerun on the files it compares
- Where: crates/before/build.rs:27-30 (related: crates/before/build.rs:93, crates/before/build.rs:175-197, justfile:710-711)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (the four `rerun-if-changed` lines and the one `rerun-if-env-changed` at 184 are the only triggers; the figure is read at 93 and the README copy at 191); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed, severity lowered to low (`just doc-figure` flips the env trigger and so reruns the script; only a hand edit of either SVG, or a results/ regeneration without `just doc-figure`, escapes, and only in the local loop; a clean CI build still fails); history: no-rationale-found (2efff1498 added the figure job and its env trigger without path triggers)
- Owner-gated: no
- Cross-references: fuelscape-render-30 (the same defect from the fuelscape partition) and deps-3 (demonstrated by build).

Once a build script prints any `rerun-if-changed`, cargo reruns it only when a listed path or a `rerun-if-env-changed` variable changes. The two SVGs the freshness check reads are not listed, so on an incremental build an edit to either does not run the check the doc at 175-182 calls "what prevents" the derived figure from rotting.

Evidence:

    27      println!("cargo:rerun-if-changed=fuelscape");
    28      println!("cargo:rerun-if-changed=docs/fuelscape.css");
    29      println!("cargo:rerun-if-changed=docs/fuelscape.js");
    30      println!("cargo:rerun-if-changed=docs/fuelscape-header.html");
    ...
    93      let figure = std::fs::read_to_string("results/space_consumption/itc_space_consumption.svg")
    ...
   185      let path = "docs/itc_space_consumption_readme.svg";

Resolution: Add `println!("cargo:rerun-if-changed=results/space_consumption/itc_space_consumption.svg");` and `println!("cargo:rerun-if-changed=docs/itc_space_consumption_readme.svg");` beside the existing four. Acceptance: after a green `cargo build -p before`, appending a byte to `docs/itc_space_consumption_readme.svg` and rebuilding reruns build.rs and fails with the "is stale relative to results/space_consumption" message.
Construction: With a warm build dir, append a whitespace byte to `crates/before/docs/itc_space_consumption_readme.svg` and run `cargo build -p before`: the script does not rerun and the build stays green; `touch crates/before/build.rs && cargo build -p before` then fires the check.

### crate-root-7: auto_traits.rs claims every public API type but omits `Limbs`, `TooWide`, and the whole `shape` module, and surfacecheck defers totality to it
- Where: crates/before/src/auto_traits.rs:1-32 (related: crates/before/src/lib.rs:431, crates/before/src/error.rs:52-54, crates/before/src/shape.rs:88-345, crates/before/surfacecheck/src/extract.rs:21-24, crates/before/surfacecheck/src/extract.rs:267-269)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (the 25 pins read against `pub struct`/`pub enum` in shape.rs at 88, 108, 118, 128, 146, 197, 255, 345, `pub use version::{Limbs, ...}` at lib.rs:431, and `TooWide` at error.rs:54; extract.rs:21-24 and 267-269 read; the only other pins in src are the not-impl pins in clock.rs and party.rs); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: no-rationale-found for the drift (the paired design was deliberate at d60b7570 with a scope doc a431eaf1d trimmed to the one-liner; 46eb64f9 added the shape items, Limbs, and TooWide and updated surfacecheck's census but not this file). The history pass disputes the lenses' inclusion of `causally::{Down, Up, Neutral}`: `Polarity: sealed::Sealed + Send + Sync + 'static` (polarity.rs:225) plus the pinned `Query<'static, Down>`/`Query<'static, Up>` already hold them, and I agree, so they are dropped from the list.
- Owner-gated: no
- Cross-references: api-audit-7 (the same roster gap from the API sweep).

The roster is a hand-maintained enumeration of a fact the code changes without touching it, and the census tool that walks the rustdoc JSON item list excludes auto-trait rows on the strength of this file's totality, so nothing checks the roster against the public surface. The omitted types are `Send + Sync + Unpin` today (borrowed walks over cursors, `Ticks` payloads, `slice::Chunks`), so this is a gap in the pin, not a shipped defect.

Evidence:

     1  //! Compile-time pins on the auto traits of every public API type.
    ...
    26  assert_impl_all!(crate::error::Crossed: Send, Sync, Unpin);
    27  assert_impl_all!(crate::error::Decode: Send, Sync, Unpin);
    28  assert_impl_all!(crate::error::Overlap: Send, Sync, Unpin);
    29  assert_impl_all!(crate::error::Parse: Send, Sync, Unpin);

    extract.rs:
   267  /// Compiler-synthesized auto-trait impls are excluded — `before` pins
   268  /// `Send`/`Sync`/`Unpin` on every public API type at compile time in
   269  /// `src/auto_traits.rs`, which is where that guarantee is reviewed —

Resolution: Add pins for `crate::Limbs<'static>`, `crate::error::TooWide`, and each `crate::shape` type (`Plateau`, `Rise`, `Region`, `Cell<1>`, `Plateaus<'static>`, `Regions<'static>`, `Overlay<'static>`, `Cells<'static, 1>`). Then close the drift path mechanically: have surfacecheck's census compare the set of public struct and enum paths it already extracts against the names pinned in this file (a text scan suffices), so an unpinned public type fails `just surface-totality`; the exclusion rationale at extract.rs:21-26 and 267-269 then names that check. Acceptance: deleting one `assert_impl_all!` line fails a gate leg naming the type; the added pins compile.
Construction: On a branch, add a `PhantomData<*const ()>` field to `shape::Plateaus` and run `just gate`: no pin names `Plateaus` and surfacecheck skips auto-trait rows by design, so a `!Send` public type ships with every instrument green.

**Clock**

### clock-22: The depth-100k proof says "every public op" but drives a listed subset over one left-only spine family
- Where: crates/before/src/clock/tests.rs:551-566 (related: crates/before/src/clock/tests.rs:565-651, 653-664, 730-738; crates/before/src/testing/generators.rs:263-283; crates/before/AGENTS.md:26-37; crates/before/src/codec/tree.rs:11-19; crates/before/src/party/tests.rs:16-18)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: verified for the op roster and the input family (body read at 565-651: encode/decode, tick, partial_cmp, `|`, `&`, send/recv, `>=`, concurrent, fork, sync, join, Debug; `ticks(k>1)`, `forks`, `[Clock; N]`, `join_all`, `sync_all`, `recv_all`/`absorb_all`, `own_version` comparison, `shape`, `encoded_bits`, `dangerously_alias` are not called; `deep_left_spine_party` at generators.rs:274-283 is the sole 100k builder, emitting only `10` tags, and is the only deep generator tests.rs imports at line 11; party/tests.rs's other shapes run at `DEEP_SCALE = 64`); assessed for which walks a missing family would leave unproven; executed: no
- Seen by: prose, correctness; refutation: confirmed (correcting the comb construction: it must end in a unary node to be normal form); history: deliberate-but-expired (the headline was true at 9ebd1b8d; forks/join_all, ticks, sync_all, and shape widened the surface without extending the test, and a6dcfbb4 re-asserted "every public op" in the commit that added sync_all)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). generators.rs:274-283 builds only `10` tags. Two 100k-level both-present id combs built through the meter registry and the public `Party::decode` (N(100k) descending left and its mirror M(100k), 400004 bits each) survive ticks(2^40), tick, forks(3), join_all, shape().count(), encode/decode, covers/is_disjoint, own_version, Party::ticks, Debug, sync_all, recv_all, absorb_all, and a final join_all in the 2 MiB test thread, so the constructed witness finds no bug behind the gap.
- Cross-references: recursion-6 (the shape iterators are absent from the deep tests too).

AGENTS.md names this test as the proof of the no-depth-recursion hard rule, and its first sentence says deep structures "survive every public op"; the body drives a subset (its own later enumeration is accurate), and the only overflow-depth input in the crate is a left-only spine, so both-present id frames (`IdFrame::BothNeedLeft/BothNeedRight`, the `IdIndex` both-present table) and right-lean descents have no depth witness anywhere. A walk that recursed only in a both-present frame, a right descent, the counted fill of `ticks`, the balanced split, the fold's index build, or the shape overlay would pass this suite. Every test's doc must be accurate, and a proof of a universal rule should be total over the frame kinds the grammar has.

Evidence:

       551	/// Deep structures (a depth-100k id spine, and the deep event tree a tick
       552	/// builds over it) survive every public op, the codec, and the `Debug` printer
       553	/// with no stack overflow.

    (testing/generators.rs)
       277	        bits.push(true); // Left-only tag `10`: left child present ...
       278	        bits.push(false); //   ... right child absent
       280	    bits.push(false); // terminal tag `00`: the deep-left owned tip

    (AGENTS.md)
        37	  The depth-100k `clock::tests::deep_tree_stack_safety` test is the proof.

Resolution: Extend rather than narrow: add flat-loop builders beside `deep_left_spine_party` for a right spine (`01` × depth, then `00`) and a both-present comb (`11 00` × depth, ending in a unary node so no node has two terminal children), and at depth 100k over each family drive `ticks(1 << 40)`, `forks(3).collect()`, `let [a, b]: [Clock; 2] = clock.into()`, `join_all`/`sync_all` over the forks, `shape().count()`, and `own_version() == other.own_version()`; then make the doc enumerate exactly what is driven. If the owner prefers the narrow fix, reword 551-553 to list the ops exercised. Acceptance: committed depth-100k tests over left-spine, right-spine, and both-present families for the listed ops, each with a doc naming the walk it proves iterative; the headline no longer says "every public op" unless the roster is total over `surface::METHOD_SURFACE`'s `Clock::` rows.
Construction: `let mut b = BitsBuf::new(); for _ in 0..100_000 { b.push(true); b.push(true); b.push(false); b.push(false); } b.push(true); b.push(false); b.push(false); b.push(false); Party::from_bits(b)` (a 100k chain of both-present nodes whose left child is terminal, ending in a unary node); run `Clock::from_parts(comb, Version::new())` through `ticks(1u64 << 40)`, `forks(3)`, `join_all`, `sync_all`, `shape().count()`, and `Clock::decode(&encode())`; mirror with `01` tags for the right spine. Any per-frame or per-right-descent recursion overflows the 2 MiB test-thread stack at this depth.

### clock-9: `shape`'s and `Forks`'s cost contracts rest on argument-based dispositions with no enforced instrument
- Where: crates/before/src/clock.rs:688-691 (related: crates/before/src/clock/forks.rs:18-19; crates/before/src/clock.rs:162-164; crates/before/src/version.rs:738-740; crates/before/src/meter/board/coverage.rs:322-324, 611-627; crates/before/fuzzfit/harness/src/bands.rs:568-579, 1010; crates/before/fuzzfit/guest/src/lib.rs:775, 1128, 1187; justfile:675)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep of bands.rs for `ff_clock_shape`, `ff_party_shape`, `ff_version_shape`, `ff_clock_sync_all`, `ff_clock_forks` returns nothing while the guest defines `ff_clock_shape`, `ff_clock_forks`, `ff_clock_sync_all`; `ff_party_forks` is banded at bands.rs:568-579; coverage.rs:611-627 dispositions read; justfile:675 labels the fuelscape an audit view); executed: no
- Seen by: claims; refutation: reframed (`Party::forks` is banded through `ff_party_forks`; the residual gaps are the shape walks, the `sync_all` composite, the `Forks` early drop, and the share-size claim); history: no-rationale-found for the missing bands
- Owner-gated: yes (reopens the board's recorded argument-based dispositions for `Clock::shape`, `Party::shape`, `Version::shape`, and `Clock::forks` at coverage.rs:322-324, 611-627)
- Cross-references: meter-adequacy-1 (the shape walks' missing instrument, from the sweep), recursion-6.

The rustdoc states hard cost contracts ("Draining the iterator is linear in the clock's encoded size"; an early drop "rejoins in `O(|c| log k)`"; `forks` yields "balanced, logarithmic sizes") whose only enforcement is an argument in the coverage board's disposition list, while the standard for asymptotic claims here is an argument, a matching implementation, and a committed instrument that fails when the claim is false. A `shape` overlay that re-opened its cursors per fragment, or a `Split::next` that re-forked from the root per share, would pass `just gate`: the laws fix values, not work. The new evidence against the disposition is cheap remediation: the fuelscape guest already exposes `ff_clock_shape`, `ff_clock_forks`, and `ff_clock_sync_all` kernels, so banding them is a calibration step rather than new machinery, and the drain-cost claim for `forks` is already banded through `ff_party_forks`.

Evidence:

       688	    /// Draining the iterator is linear in the clock's encoded size: each
       689	    /// fragment costs `O(1)` plus its own rise's encoded width, and the
       690	    /// walk itself performs no arithmetic. [`Version::shape`]'s caveat on
       691	    /// the cost of folding rises applies here too.

    (forks.rs)
        18	/// Each `next` costs its own share's portion of the drain; an early drop
        19	/// rejoins in `O(|c| log k)`, with `|c|` the borrowed clock's size in bytes.

    (meter/board/coverage.rs)
       623	        "Clock::shape",
       624	        "the two component walks advanced together under the overlay law: \

    (justfile)
       675	# Render the full population atlas into target/fuelscape (audit view; not enforcement).

Resolution: Owner's call. If the dispositions stand, no change; if not, add fuzzfit bands for `ff_clock_shape` (and the party/version shape kernels) and `ff_clock_sync_all` at the next `just fuzzfit-calibrate`, and add an orbit-style size pin for `forks(k)` shares (every share of `Party::seed().forks(k)` reads at most `2 + 2·⌈log2 (k + 1)⌉` encoded bits) beside the iterated-fork affine chain pin. Acceptance: a `shape` walk that re-opens its version cursor per fragment fails the gate; the share-size pin trips when a share exceeds the logarithmic bound.
Construction: wrap `Overlay::next` so it re-opens the version walk from the start on every call (the yielded fragments are unchanged); run `just gate`: the laws pass and only the unenforced fuelscape would show the bend.

### clock-28: The static-orbit pin misstates its mechanism, its exchange schedule degenerates into four fixed partner pairs, and its collision branch is dead
- Where: crates/before/src/clock/tests.rs:1412-1443 (related: crates/before/src/clock/tests.rs:1226-1230, 1284-1295, 1356-1360; crates/before/src/codec/gamma.rs:28; crates/before/src/version/skyline.rs:11-24; crates/before/examples/space_consumption.rs; crates/before/reference/itc2008.md)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified and executed (Python enumeration of `s = (3r+1) % 8`, `t = (5r+2) % 8` over 4096 rounds: distinct `(s, t)` = `[(0,3),(1,2),(2,1),(3,0),(4,7),(5,6),(6,5),(7,4)]`, collision branch taken 0 times, `t − s` odd at every round; the churn orbit's identical guard fires 455 times at population 9, so only the static copy is dead); the coding derivation (four growing gamma codes at 2 bits per doubling) is assessed from skyline.rs:16-24 and gamma.rs:28 and agrees with the sibling pin's `7 + 2·⌊log2 k⌋`; executed: yes (schedule enumeration)
- Seen by: prose, claims; refutation: confirmed (re-ran the enumeration); history: no-rationale-found (the mechanism sentence was asserted, not derived, at 33d37fb0; the design note calls the scenario only "banded")
- Owner-gated: no

The doc attributes the pinned `8·i − 4` closed form to "the eight counters' gamma widths, 1 bit each per doubling", but the gamma code costs `2*floor(log2(n+1)) + 1` bits (2 bits per doubling, as the neighboring pin at 1290 says) and the skyline stores one absolute height plus zigzag deltas, not eight independent counters. The schedule's `s + t ≡ 3 (mod 8)` makes `t` a function of `s`: every peer exchanges with one fixed partner forever, six of eight leaves in every version stay zero, and the 8 bits per doubling are four growing delta codes (entering and leaving two nonzero counters in a non-adjacent pair) at 2 bits each. The `if t == s` branch cannot fire because `t − s = 2r + 1` is odd. The pin's number is right and deterministic; its stated mechanism and its framing as a transcription of the paper's static scenario (exchange over a fixed process set; the crate's own `examples/space_consumption.rs` uses random pairs) are not. A test's doc comment must state its invariant accurately, and an unreachable branch is scaffolding.

Evidence:

      1417	/// version size is monotone nondecreasing and grows exactly 8 bits per doubling
      1418	/// of the round count — the eight counters' gamma widths, 1 bit each per
      1419	/// doubling — reading exactly `8·i − 4` bits over the octave ending at round
      1437	        let s = (r * 3 + 1) % N;
      1438	        let mut t = (r * 5 + 2) % N;
      1439	        if t == s {
      1440	            t = (t + 1) % N;
      1441	        }

    (the sibling pin, correct)
      1290	/// events cost one gamma code's width (2 bits per doubling), never a ratcheting

    (codec/gamma.rs)
        28	/// Cost is `2*floor(log2(n+1)) + 1` bits; `0` costs a single bit. Canonical and

Resolution: Pick an exchange schedule whose `(sender, receiver)` orbit covers every ordered pair (e.g. `s = r % N; t = (s + 1 + (r / N) % (N − 1)) % N`), re-measure the octave array, restate the mechanism from the coding (which delta codes grow, at 2 bits per doubling each), and either delete the static orbit's collision guard or assert the schedule's pair coverage as the topology's liveness floor; leave the churn orbit's guard (1358-1360) in place, since it fires. Acceptance: the mechanism sentence is derivable by hand from skyline.rs:16-24 and gamma.rs:28 and names the number of growing codes; a committed assertion (or a fixed-period schedule with a stated proof) shows every ordered peer pair exchanges; no unreachable branch remains in the static orbit's body.

### clock-19: Fold-differential docs assert concrete hand-back outcomes the bodies check only by oracle agreement
- Where: crates/before/src/clock/tests.rs:36-42 (related: crates/before/src/clock/tests.rs:65-76, 102-139)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (both bodies call only `assert_join_all_matches_recursive_oracle`, which asserts verdict, hand-back vector, and accumulator equality against the oracle and nothing about region coverage); executed: no
- Seen by: prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Both docs state a specific result ("both hand back exactly the duplicate"; "the hand-back is the four-input group and the accumulator absorbs only a∪b"), but each body asserts only that production agrees with the recursive oracle. If the oracle's discipline changed, the stated outcome would be false while the test still passed; the doc describes an expected trace and the body checks a weaker property. A direct assertion is cheap and makes the doc's claim enforced.

Evidence:

        41	/// accumulators; with the accumulator's own region duplicated among the inputs,
        42	/// both hand back exactly the duplicate.
        59	    let (acc, children) = population(false);
        60	    assert_join_all_matches_recursive_oracle(acc, children);
        61	    let (acc, children) = population(true);
        62	    assert_join_all_matches_recursive_oracle(acc, children);

        73	/// combine, then coalesces d∪e into it, so the hand-back is the four-input
        74	/// group and the accumulator absorbs only a∪b — with each input carrying a

Resolution: Either add the direct assertions (aliased case: `back.len() == 1` and `back[0].party()` covers exactly the aliased region; coalesced case: one hand-back whose party covers alias∪c∪d∪e and `acc.party()` covers the seed's residual plus a∪b), or phrase the docs as "agree with the oracle, whose discipline hands back ...". Acceptance: each doc sentence naming an outcome corresponds to an assertion in the body or is phrased as the oracle's decision.

**Party**

### party-12: `forks`' minimal-depth balance, the reason the operation exists, has no committed instrument
- Where: crates/before/src/party/forks.rs:16-17 (related: crates/before/src/party/forks.rs:57, crates/before/src/party/forks.rs:154, crates/before/src/laws.rs:2053, crates/before/src/laws.rs:2065, crates/before/src/laws.rs:2358, crates/before/tests/forks_max.rs:22-39, crates/before/fuzzfit/harness/src/bands.rs:568-579, crates/before-fuelscape/src/ops.rs:766-777, crates/before/src/meter/board/ops.rs:1270-1518)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep for `forks(` together with `encoded_bits|depth|log2|ilog2` over tests/, laws.rs, clock/tests.rs, testing/, fuzz, fuzzfit returns nothing; the laws touching `forks` are `forks_matches_from_array`, `forks_partial_drop_folds_back`, `party_join_all_reunites_forks_at_any_width` and their clock twins, none of which reads a depth; the board's `party_*` cells (ops.rs:1270-2250) include `party_fork` but no `party_forks`; the fuelscape island fixes the share count at 8; the `ff_party_forks` band is a fuel slope band with `width_above` 0.097); executed: no
- Seen by: claims; refutation: confirmed (the 3:1 known-bad's passage through the fuel band is assessed, not run); history: no rationale found (14d36c62d's laws hold for any partition; nothing defers a depth pin)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). At HEAD every share of `forks(k)` and `[Party; N]` sits at the minimal depth for every k tried up to 4097. With forks.rs:57 changed to a 3:1 split, every committed fork/forks test, the organic law driver, and both forks_max pins stay green; only the scratch witness fails. The fuelscape island and the `ff_party_forks` band were not run.

The public promise (every share a leaf of a minimal-depth `⌈log₂ k⌉` tree; forks.rs:5-7 calls the balanced split "the cure for the `fork` footgun") is asserted by no test, band, board cell, or envelope. Every committed check compares two forms that share `Split` or checks reunion, which hold for any disjoint tiling, so an unbalanced split passes the gate. Principle 2 and the crate's own rule that every asymptotic claim is a hard guarantee.

Evidence:

        16	/// A lazy balanced partition: yields a region's `k` shares one at a time, in
        17	/// preorder, each a leaf of a minimal-depth (`⌈log₂ k⌉`) id tree.

        55	        while count > 1 {
        56	            let right = region.fork();
        57	            let left_count = count.div_ceil(2);

Resolution: Add a closed-form pin in party/tests.rs: for `Party::seed()` and `k` over a ladder (1..=64, 1023, 1024, 1025, 2^16), every yielded share and the residual read `encoded_bits() == 2 + 2·d` with `d ∈ {⌊log₂(k+1)⌋, ⌈log₂(k+1)⌉}`, and the count of shares at the deeper level equals `2·(k+1) − 2^⌈log₂(k+1)⌉` (the complete-tree leaf split), so the whole tiling is fixed. Commit the 3:1 split as a known-bad witness (a `cfg(test)` `Split` variant, or an inline transcription of `Split::next` with `count - (count / 4).max(1)`) and assert the pin convicts it. Cite the pin by name at forks.rs:16-17 and :154. Acceptance: the new test fails when `left_count` is `count - (count / 4).max(1)` and passes on `count.div_ceil(2)`.
Construction: change forks.rs:57 to `let left_count = count - (count / 4).max(1);` (a 3:1 split, depth about `2.4·log₂ k`). `forks_matches_from_array` still holds (both forms run the same `Split`), the reunion and partial-drop laws hold (the shares remain a disjoint tiling), `party_forks_max_saturates_without_panic` terminates, and at the island's `k = 8` the extra work is roughly ×1.6 fuel, under the band's `width_above + ENFORCE_MARGIN` ceiling. Nothing committed fails.

### party-33: The deep `is_disjoint` differential compares production against production, and `covers` has no deep differential at all
- Where: crates/before/src/party/tests.rs:407-423 (related: crates/before/src/party/tests.rs:829-834, crates/before/src/testing/generators.rs:298, crates/before/src/testing/exhaustive.rs:85-91, crates/before/src/surface.rs:383-391, crates/before/tests/meter.rs:6135-6146)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (the deep test holds `IdIndex::is_disjoint` to `IdReader::is_disjoint` only; the oracle legs `party_disjointness_matches_the_oracle`/`party_covers_matches_the_oracle` run over the arbitrary population at `ARB_DEPTH = 4` and organic pairs; the exhaustive deep variant is `#[ignore]` at id depth 4; `id_covers_envelope` asserts one `false`; the bridge lowers parties at `ORACLE_SCALE_MAX = 4096`); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (522705cff added the deep leg as the mechanism seam; no record considers an oracle leg at the shape scales)
- Owner-gated: no

Two production mechanisms agreeing is not agreement with the paper. The shared normal-form precondition (an `Internal` node never covers a `Full` leaf, compare.rs:84-99) is exactly the kind of assumption both mechanisms share and the oracle does not, and the bridge already reaches these scales cheaply for `diff_constructed`.

Evidence:

       416	        for (x, y) in [(&a, &b), (&a, &a), (&sa, &sb)] {
       417	            let walk = x.is_disjoint(y);
       418	            prop_assert_eq!(IdIndex::build(x.as_bits()).is_disjoint(y.view()), walk);
       419	            prop_assert_eq!(IdIndex::build(y.as_bits()).is_disjoint(x.view()), walk);

Resolution: In the deep test add `prop_assert_eq!(walk, to_oracle_party(x).is_disjoint(&to_oracle_party(y)))` and a parallel `covers` leg in both roles over the same shape triples, keeping the scale under `ORACLE_SCALE_MAX`. Acceptance: the deep test's doc names the oracle as the reference; a `covers` verdict is checked against the oracle at scale at least 64 in both operand orders.
Construction: the shape pairs already built there suffice (`(&a, &a)` gives the overlapping verdict, the skip-stress pair the disjoint one); for `covers`, add `(shape_party(shape, scale), shape_party(shape, scale / 2))` and a `node(Some(&a), None)` wrapper so the `true` arm is reached. The test today would not fail if `IdIndex` and `IdReader` shared a wrong verdict on a deep pair.

### party-34: `sum_split_scan_never_exceeds_the_composition` compares mixed currencies: the composed side counts `sum`'s builder writes, the fused side counts reads only
- Where: crates/before/src/party/tests.rs:740-769 (related: crates/before/src/party/ops/sum_split.rs:62-66, crates/before/src/party/ops/sum_split.rs:205-223, crates/before/src/party/ops/split.rs:45-53, crates/before/src/codec/build.rs:82-83, crates/before/src/codec/build.rs:127-128, crates/before/src/codec/build.rs:175-180, crates/before/src/codec/buf.rs:164-173, crates/before/src/codec/buf.rs:346-369)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (`PackedBuilder::{push_bit, reserve, splice}` call `record_bits`; `BitsBuf::push` and `extend_from_view` record nothing; `build_split`'s spine loop records nothing); executed: no
- Seen by: correctness; refutation: confirmed (nuance: the exact `spliced == 8` pin at 788-792 does convict a spliced-subtree scan; the doc clause the slack does not support is "re-reads a skipped child"); history: no rationale found (24dff4f58 pinned the ratios as observed)
- Owner-gated: no

The test asserts `fused <= composed` in scan bits, but `sum` records its output writes while `sum_split`'s half assembly (`BitsBuf::push`/`extend_from_view`) and `split`'s spine read record nothing, so the composed side carries about the union's write bits of slack (about `2k` bits on the committed "adjacent k" regime). The test doc says a fused walk that "re-reads a skipped child" moves the ratio above one; a re-read of one skipped child adds 2 bits and passes. A meter whose two sides count different things cannot fail for the regression it names.

Evidence:

       747	    /// derived around), and the lockstep spine to a targeted branch. A fused
       748	    /// walk that re-reads a skipped child or scans a spliced subtree moves the
       749	    /// ratio above one.

       768	            assert!(
       769	                0 < fused && fused <= composed,

Resolution: Either meter the fused side's writes and the spine read (the re-pin route of party-27, which also moves `ID_FORK`'s scan column and the `spliced == 8` constant) so both sides count reads plus writes, or keep the raw path and compare like with like (subtract `sum`'s write bits from `composed`, or compare against a reads-only composition) and correct the doc to name exactly which regressions the assertion convicts. Acceptance: the construction below fails the test; the doc's stated conviction matches what the assertion can refute.
Construction: on the committed "adjacent k" regime (`a = leftmost(k)`, `b = spine(k - 1, true, node(None, Some(&full())))`), inject the regression the doc names: in `branch_children` (sum_split.rs:175-179) add a second `IdReader::at(bits, start).skip()` after `probe.skip()`. `fused` rises by 2 bits and `fused <= composed` still holds at both scales; only a re-walk of at least about `2k` bits (re-scanning the spine) trips the assertion today.

### party-31: `join_all_hands_back_aliased_inputs` builds its alias by seeding two extra universes instead of `dangerously_alias`
- Where: crates/before/src/party/tests.rs:39-53 (related: crates/before/src/party/tests.rs:113, crates/before/src/party/tests.rs:129, crates/before/src/party/tests.rs:143, crates/before/src/party/tests.rs:272)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read); executed: no
- Seen by: prose, structure (bundled in [15]); refutation: confirmed; history: no rationale found (4f12b8218; `dangerously_alias` existed since 6b93f0af1 and every sibling test uses it)
- Owner-gated: no

The test needs a duplicate of one share and its expected hand-back; it obtains them by seeding two more independent universes, forking each, and feeding a share from the second into the first's `join_all`. The crate's hard rule is that independently seeded universes never interact, and `dangerously_alias` exists for exactly this need; the body relies on two seeds splitting identically, which is true but is not what the doc ("a duplicated share") says it exercises, and it models the one thing the crate tells users never to do.

Evidence:

        40	    let mut acc = Party::seed();
        41	    let shares: Vec<Party> = acc.forks(3).collect();
        42	    let mut dup_seed = Party::seed();
        43	    let mut dups = dup_seed.forks(3);
        44	    let duplicate = dups.next().expect("three shares were requested");

Resolution: `let duplicate = shares[0].dangerously_alias(); let expected_back = shares[0].dangerously_alias();` and delete the two extra seeds. Acceptance: the test constructs one universe; its assertions are unchanged and still pass.

### party-32: `parse_bare_notation` claims the `TryFrom` literals agree with the string parser but never compares them
- Where: crates/before/src/party/tests.rs:347-359 (related: crates/before/src/party/tests.rs:454-465)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read); executed: no
- Seen by: structure, prose; refutation: confirmed; history: no rationale found (unchanged since eecf92295)
- Owner-gated: no

The doc's invariant is "build parties via the same paper notation as the string parser"; the body checks only that three literals construct and `0` is rejected. Nothing ties a literal to its parsed twin, and the name says "parse" while the body exercises `TryFrom`. Every test's doc comment must state the invariant the body checks; a doc promising agreement the assertions do not test is the "not wrong, but you could not tell if it were" gap. Alongside: `crate::codec::built_view` is spelled fully qualified about fourteen times in this file where one `use` would do.

Evidence:

       347	/// `TryFrom` numeric/tuple literals build parties via the same paper notation
       348	/// as the string parser.

       355	    let _party: Party = 1.try_into().unwrap();
       356	    assert!(Party::try_from(0).is_err());
       357	    let _party: Party = (1, 0).try_into().unwrap();
       358	    let _party: Party = ((0, 1), (1, (1, 0))).try_into().unwrap();

Resolution: Assert `Party::try_from(lit).unwrap() == text.parse::<Party>().unwrap()` for each literal/text pair (`(1, 0)` and `"(1, 0)"`, `((0, 1), (1, (1, 0)))` and `"((0, 1), (1, (1, 0)))"`) and rename to `literal_doors_agree_with_the_parser`; or narrow the doc to what the body checks. Add `use crate::codec::built_view;` at the top of the file. Acceptance: the doc and the body state the same invariant; a literal door producing a different tree from the parser fails the test; `grep -c 'crate::codec::built_view' crates/before/src/party/tests.rs` is 0.

**Version core**

### version-core-15: Stored versions retain their build buffer's pre-size capacity; the tick and hull paths have no resident reading and no committed test pins retained bytes against encoded bytes
- Where: crates/before/src/version.rs:1192-1201 (related: crates/before/src/codec/bits.rs:107-125; crates/before/src/codec/buf.rs:121-125; crates/before/src/codec/build.rs:66-68, 233-240; crates/before/src/version/skyline/emit.rs:223, 228, 303; crates/before/src/version/skyline/query.rs:517, 603-607; crates/before/src/version/skyline/grow.rs:513; crates/before/src/version/skyline/fill.rs:251; crates/before/benches/presize.rs:1-14, 154-204; justfile:792-809; crates/before/tests/meter.rs:360-372; crates/before/src/version/tests.rs:2185-2201)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified for the absence (grep of `presize.rs` rows: `projection`, `projection_decoded`, `projection_outgrow`, `merge`, `display`, `parse`; no tick or hull row; `bench-alloc-ab` arms are `shipped|projection_growth|projection_shrink|display_growth`); assessed for the mechanism (read: `SkylineBuilder::with_capacity` reaches `Vec::with_capacity(cap / 8 + 1)` at build.rs:68; `finish` adopts the bytes without shrinking at 233-240; `Bits::freeze` is `Bytes::from(buf.into_bytes())`; bytes 1.11.1, the pinned version, `From<Vec<u8>>` at src/bytes.rs:960-977 boxes the slice only when `len == cap` and otherwise keeps `cap` in `Shared`; `fill.rs:251` pre-sizes a tick to exactly `event.len()`, so a tick that spills into a new byte pays `Vec` doubling); executed: no
- Seen by: claims; refutation: reframed (the "invisible to every committed meter" claim is refuted: query.rs:603-607 states the stranded-slack trade, `benches/presize.rs` is a committed allocation-strategy record with a `resident_report`, and the peak envelopes keep the result alive at measurement), severity medium to low; history: pre-sizing is owner-ratified (2026-07-27, "RATIFIED as the stated-band residual", for the projection's doubling chain); nothing in any note addresses at-rest retention on the tick or hull paths
- Owner-gated: no (measure first; a bench row or test is additive)
- Cross-references: benches-examples-12 (the presize allocation record, the only committed resident-bytes instrument).

A stored `Version`'s heap footprint is its build buffer's final capacity, not its encoding: a meet of disjoint supports retains `|a| + |b|` bits behind a one-byte value, a join of comparable operands retains the dominated operand's size, and a tick that spills a byte past `event.len()` retains roughly twice its encoding. The crate knows this and records it at bench level for projection, merge, display, and parse, but the two paths a long-lived consumer (rumors's tree memos, mostly tick outputs and bounds hulls) exercises most have no resident row, and `at_rest_size_is_one_container_per_stream` pins only the 32-byte handle. Under Principle 2 this is a measure-first item: the sign on residency is fixed (shrinking can only reduce it), the sign on time is workload-dependent (one memcpy at freeze), so a reading precedes any cure.

Evidence:

      1192	    /// Freeze a normal-form skyline bit stream as a `Version`, canonicalizing
      1193	    /// its storage. The single build-side gate every built/parsed `Version`
      1194	    /// passes through.
      ...
      1199	    pub(crate) fn from_bits(bits: codec::BitsBuf) -> Self {
      1200	        Version(codec::Bits::freeze(bits))
      1201	    }

Resolution: add `tick` (dense version, one tick that spills a byte) and `hull` (meet of disjoint supports; join of comparable operands) rows to `presize.rs`'s `resident_report`, and one committed test that reads live allocator bytes after the operation returns minus `as_bytes().len()` on those constructed shapes, pinning a ceiling with slack and a floor at the encoding. If the reading is material for the consumer's workload, shrink at the one gate (`Bits::freeze`: `into_boxed_slice` before `Bytes::from`, so `len == cap` takes bytes' no-`Shared` path) or size the emitters to the subadditivity bound minus the known collapse. Acceptance: a committed reading exists for the tick and hull paths; if the cure lands, retained bytes equal the encoded length plus the fixed `Bytes` header on every shape, and the peak-heap envelopes move only where the memcpy adds to peak.

Construction: under the meter suite's counting allocator (`HEAP.current_usage()`), build `a` as a deep left spine and `b` as a deep right spine of about N bytes each (disjoint supports), take `let m = &a & &b;` (`m.as_bytes().len() == 1`), and read live bytes after the call returns: expect about `(|a| + |b|) / 8` retained. Dual: `let mut v = dense(N); v.tick(&Party::seed());` and compare live bytes to `v.as_bytes().len()`; a doubling past the `event.len()` pre-size reads about 2x.

### version-core-32: The provenance-linearity test quotes measurements its body never produces, and its single-scale ceiling cannot see a change of order
- Where: crates/before/src/version/tests.rs:1187-1192 (related: crates/before/src/version/tests.rs:1237-1245; crates/before/src/version/rank.rs:36-38)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the body 1237-1245: one `assert!(encoded_bits <= input_bits)` per family, no print, no ratio; rank.rs:36-38 repeats "measured at 0.56 encoded bits per packed input bit" as design rationale); executed: no
- Seen by: structure ([10]), prose ([25]), correctness ([42]), claims ([53]); refutation: all confirmed; history: at the test's origin (f0f3a2aed) the body already asserted only the 1.0 ceiling and printed nothing; the ratios were a development-time measurement recorded in the commit message and transcribed into the doc, so "by this test's own instrumentation" was never true of the committed body
- Owner-gated: no

Two defects in one test. The doc reports five ratios "measured by this test's own instrumentation" that nothing computes, prints, or asserts: a hand-maintained measurement (Principle 5) that will keep asserting itself after the encoder moves, repeated in `rank.rs` as rationale. And the claim in the doc's first sentence, "encodes linearly in the version's packed bytes", is pinned by a ratio ceiling of 1.0 at one depth per family; with the worst family at 0.56 that is a constant-factor envelope, not an order pin: an encoder whose output grew as `version_bits · log(depth)` would sit under 1.0 at depth 800 and pass. The standard distinguishes an envelope with slack from an instrument that pins the order (a two-scale ratio or fitted band), and a linearity claim needs the latter.

Evidence:

      1186	/// holds each family's encoded size at or under 1.0 bit per packed input bit.
      1187	/// Measured \[by this test's own instrumentation\]: wide counter 0.56 (the
      1188	/// worst — a lone counter's version pays gamma's doubled width where the
      1189	/// encoding pays the width once), deep spine 0.38, dense staircase 0.38, deep
      1190	/// wide counter 0.27, plateau puncture 0.18; the 1.0 pin leaves headroom for
      1191	/// packing drift while sitting an order under the exponential blowup arbitrary
      1192	/// in-memory ranks can reach.

Resolution: measure each depth-parameterized family at two depths (`d` and `2d`) and assert the encoded-rank-bits to version-bits ratio does not grow beyond a stated slack; either assert the per-family ratios as a band (turning the slack into a tightened pin) or delete the five figures from this doc and from rank.rs:36-38, leaving the enforced 1.0 bound as the only number in prose; fix the stray `\[`/`\]` escapes either way. Acceptance: every numeric ratio in the doc is asserted by the body or absent; the test fails when the rank encoder is replaced by one whose output grows as `version_bits · log(depth)`.

Construction: for each family build versions at `d` and `2d` (`spine(800)`/`spine(1600)`, `staircase(800)`/`staircase(1600)`, `deep_counter(400, wide)`/`deep_counter(800, wide)`); compute `r(d) = rank().encode().len() * 8 / encoded_bits()`; assert `r(2d) <= r(d) + slack`. In a scratch build, append a depth-proportional header to the rank encoder: today's `encoded_bits <= input_bits` stays green at 800 while the two-scale check fails.

### version-core-33: `div_decomposes_along_fork`'s doc claims a disjoint-party check its body does not perform
- Where: crates/before/src/version/tests.rs:2118-2141 (related: crates/before/src/version/tests.rs:2143-2159)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (the body at 2137-2140 asserts sub-version, rejoin, empty meet of the two projections, and seed identity; `div_view_matches_materialization` at 2157-2158 owns the disjoint-party assertion); executed: no
- Seen by: prose; refutation: confirmed; history: the overclaim is original (472d646e2), not drift
- Owner-gated: no

The crate's testing rules make a test's doc comment the statement of the invariant it protects and hold its accuracy to the standard of a bug; a doc that claims an assertion the body lacks misreports coverage.

Evidence:

      2120	/// Each half's contribution is a sub-version, the two rejoin to the whole, and
      2121	/// their supports are disjoint (so their meet is empty). The whole-interval
      2122	/// seed party is the identity, and projecting onto a *disjoint* party keeps
      2123	/// nothing.

Resolution: drop the final clause (the next test owns it), or add the disjoint-party assertion here and keep the sentence. Acceptance: every clause of the doc corresponds to an assertion in the body.

**Rank**

### rank-30: num/tests.rs's testdoc claims hashing coverage the body lacks, and its module doc overstates "every wide-arm operation"
- Where: crates/before/src/version/rank/num/tests.rs:131-141 (related: num/tests.rs:1-2, crates/before/src/laws.rs:2936-2942, crates/before/src/version/tests.rs:1485-1490)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`grep -n -i hash` on num/tests.rs returns only the doc line 131; no hasher is constructed anywhere in the file; `Num::to_be_bytes` on the wide arm, `from_base`, `fold_into`, and the `Hash` impl have no test in the file; wide-arm `Hash`/`Eq` coherence is exercised elsewhere: `rank_cross_path_normalization` at laws.rs:2942 asserts `hash_of(&via_add) == hash_of(&via_sum)` and `rank_wide_arm_triple_laws_on_seeded_ranks` runs `RANK_TRIPLE` under the lowered ceiling); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (79a944ab), with the coverage picture corrected as above
- Owner-gated: no

Doctrine: every test's doc comment states its invariant and its incorrectness is a bug in the test. The doc says hashing is checked; the body asserts only `na == nb` against the oracle and clone-reflexivity. The module doc tells a maintainer this file is the coverage of record for wide-arm mechanics when four operations are exercised only through version/tests.rs and laws.rs. Because the hash property is pinned in the law group, the fix is doc-side or a small hasher clause, not a coverage emergency.

Evidence:

         1	//! Unit suites for the numerator's two-arm storage: every wide-arm
         2	//! operation differentially against the backend as oracle.

       131	    /// Structural equality and hashing are value equality across the
       132	    /// dispatch: equal values are one arm and equal, unequal values are
       133	    /// unequal whatever their arms.
       134	    #[test]
       135	    fn equality_is_value_equality(a in arb_value(), b in arb_value()) {
       136	        let _guard = ceiling::force(TEST_CEILING_BITS);
       137	        let na = num_from_oracle(&a);
       138	        let nb = num_from_oracle(&b);
       139	        prop_assert_eq!(na == nb, a == b);
       140	        prop_assert_eq!(&na, &na.clone());
       141	    }

Resolution: Add a hash-coherence clause (`a == b` implies equal `DefaultHasher` outputs for `na` and `nb`) or drop "and hashing" from the doc and cite `rank_cross_path_normalization`. Narrow the module doc to the operations pinned here (dispatch, assembly, shifts, bias steps, windows, rendering, equality) and point at the version/tests.rs wide-regime suites and the `RANK_TRIPLE` law run for `to_be_bytes`, `from_base`, `fold_into`, and `Hash`; or add oracle tests for those four. Acceptance: every `pub(crate)` fn on `Num` and every trait impl with a `Wide` arm either has a test in this file whose doc matches its body, or is named in the module doc with the covering suite cited.

Construction: Under `ceiling::force(96)`, build two `Num`s of equal value, feed each to `std::hash::DefaultHasher`, and compare; nothing in this file does so today, so a `Hash` impl that hashed the arm tag would pass every test in the file (and be caught only by the law group).

**Span and causally**

### span-causally-8: The span folds' time and space claims are priced only by proxy `version_*_all` cells that never execute `fold_endpoints`
- Where: crates/before/src/span/algebra.rs:98-102 (related: crates/before/src/span/algebra.rs:171-175, 244-248, 310-314, 339-424; crates/before/src/meter/board/coverage.rs:4-5, 165-171; crates/before/src/fold.rs:1-16)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read coverage.rs:165-171: `("Span::union_all", &["version_span_all"])`, `("Span::intersect_all", &["version_join_all", "version_meet_all"])`, and the join/meet rows; grep of tests/meter.rs for `union_all\|intersect_all\|Span::union` finds no span-fold rows, only `Version::join_all`/`meet_all` rows; grep of fuzzfit/harness/src/bands.rs finds no span or query band); executed: no
- Seen by: claims; refutation: confirmed (adds that the fuelscape span islands are audit-only, justfile:675); history: deliberate-and-holds (the board's stated policy "rows price delegations at their shared mechanism"), but its premise holds only at fold.rs's counter: `fold_endpoints` is its own body (span-causally-9), which is exactly what makes a regression there invisible
- Owner-gated: no

Four public `# Complexity` sections state "Auxiliary space is `O(|self| + |iter|)`" and their islands state `O((|self| + |iter|) log k)`. The argument (the balanced counter holds at most log k merged groups, each group's two legs bounded by the packed size of the inputs it absorbed, since a join or meet of two skylines is O(|a| + |b|) bits) is stated nowhere in the module. The board prices these rows through `Version`'s own folds, which never run `fold_endpoints`'s dedup filter, two-leg groups, or point-combine. A regression confined to `fold_endpoints` (a left fold, or a byte-copying dedup filter) would move nothing enforced. The crate docs make asymptotic claims hard guarantees, and a claim needs an argument and an instrument that fails when it is false.

Evidence:

        98	    /// # Complexity
        99	    ///
       100	    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_union_all.html"))]
       101	    ///
       102	    /// Auxiliary space is `O(|self| + |iter|)`.

    (meter/board/coverage.rs)
       165	    ("Span::union_all", &["version_span_all"]),
       166	    (
       167	        "Span::intersect_all",
       168	        &["version_join_all", "version_meet_all"],
       169	    ),
       170	    ("Span::join_all", &["version_join_all"]),
       171	    ("Span::meet_all", &["version_meet_all"]),

Resolution: state the space argument once at `fold_endpoints`'s doc. Add a board cell (heap and scan currencies) over a mixed point/wide family so both combine arms run, or a `scan-meter` two-scale row in tests/meter.rs, and map the four roster rows to it instead of the `version_*_all` proxies. If span-causally-9 unifies the folds first, the proxy becomes sound and only the argument is owed. Acceptance: rewriting `fold_endpoints` as a sequential left fold (or making the dedup filter copy bytes) fails the new cell at two scales; the current code passes.

Construction: receiver `a_1.span(&a_2)`; items alternate `Span::at(&x_i)` points and wide `a_i.span(&b_i)` hulls over interleaved single-tick versions (the shape fold.rs:9-14 names as growing without coalescing); arity k in {64, 512}. Measure peak heap and scan bits around `receiver.union_all(&items)`; a left fold scales as k · Σ|items|, the balanced fold as Σ|items| · log k.

### span-causally-28: No enforced instrument prices `Query::and` under a growing hole count; the rendered island's `n^2` claim describes a regime its one-hole operands never enter, and the survive filter re-reads the floor once per hole
- Where: crates/before/src/causally/conjunction.rs:38-68 (related: crates/before/src/causally/conjunction.rs:148-163; crates/before/src/causally/polarity.rs:81-107; crates/before-fuelscape/src/ops.rs:160-178, 2130-2157; justfile:675)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the panel spec: both operands `strictly_after(a) & before(c)`, one hole each, contract `O(|self| · |rhs|)`, claim `n^2`; read justfile:675 "audit view; not enforcement" and ops.rs:176-177 "The envelope suite and the fuzz-fit bands own worst-case enforcement"; grep of tests/meter.rs and fuzzfit bands finds no conjunction row); executed: no
- Seen by: claims; refutation: reframed (the fuelscape is not an enforcement instrument, so the gap is that nothing enforced prices `and()` under a growing hole count, and the island's contract describes a regime its operands cannot enter); history: deliberate-and-holds for the label (2efff149's rule: a claim is an O upper bound, "O already bounds above", so the `n^2` stamp over linear data is accurate under that rule); the enforcement gap stands
- Owner-gated: yes: adding an enforced row is a gate-policy addition, and the island's shape is the owner's
- Cross-references: span-causally-24 and span-causally-36 (correctness class: the multi-hole cost the instrument would price).

`and()` performs k `hole_survives` checks (each a separate `le(floor, hole_i)` sweep re-reading `floor`: O(k · |floor|), where a fused per-bound walk would be O(|floor| + Σ|holes|)) plus up to k·m `absorbs` comparisons. The only cost content on the twenty-one `&` impls is an island whose operands carry one hole each, so the measured merge does exactly one cross-side comparison regardless of size, and no enforced instrument (tests/meter.rs, board, fuzzfit) prices the k·m term at all. An instrument that pins a claim must be able to fail when the claim is false.

Evidence:

        47	        let survives =
        48	            |hole: &Hole<'a>| P::hole_survives(hole, floor.as_deref(), ceiling.as_deref());
        49	        let mut kept: Vec<Hole<'a>> = self.holes.into_iter().filter(survives).collect();
        50	        let mut added: Vec<Hole<'a>> = Vec::new();
        51	        for hole in other.holes {
        52	            if !survives(&hole) {
        53	                continue;
        54	            }
        55	            if kept.iter().any(|held| P::absorbs(held, &hole)) {
        56	                continue;
        57	            }
        58	            kept.retain(|held| !P::absorbs(&hole, held));
        59	            added.push(hole);
        60	        }
        61	        kept.append(&mut added);

    (before-fuelscape/src/ops.rs)
      2139	        size_measure: "total packed bytes of the two strict-lower bounds and the \
      2140	             two ceilings, split uniform four ways (each operand composed in \
      2141	             unmeasured preparation as strictly_after(a) & before(c) — \

Resolution: add an enforced row (a `scan-meter` or touch row in tests/meter.rs) whose operands carry k pairwise-concurrent holes with k scaling, declaring the k·m model at the cell; either add a many-hole fuelscape variant or relabel the existing island's contract to the one-hole shape it samples. Optionally fuse the survive filter through `filter::admits` with per-bound verdicts. Acceptance: an enforced row exists whose reading quadruples when k doubles at fixed bound size, and the island's contract describes what its operands measure.

Construction: `A_k = since(&a_1) & ... & since(&a_k)` and `B_k = since(&b_1) & ... & since(&b_k)` over 2k pairwise-concurrent one-tick versions; `A_k & B_k` performs k·k `absorbs` comparisons (lines 55, 58) plus 2k survive checks. Doubling k quadruples the comparison count at fixed bound size; the committed panel never varies k.

### span-causally-35: `Query::coverage`'s clone-identity rung has no agreement test across buffer identity
- Where: crates/before/src/causally/query.rs:129-135 (related: crates/before/src/span/tests.rs:812-844; crates/before/tests/coincident_span.rs:1-11; crates/before/src/laws.rs:1074-1099, 1663-1670; crates/before/src/causally/tests.rs:295-299; crates/before/src/version/skyline/place/tests.rs:475-506; crates/before/src/version/skyline/place/filter.rs:477-515)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (every coincident span reaching `Query::coverage` in committed tests shares one buffer: laws.rs:1665 builds the coincident candidate as `(meet.clone(), meet)`; laws.rs:1088 uses `Span::at(p)`; the two-party grid builds `Span::new(lo, hi)` from the same `&grid[i]`; tests/coincident_span.rs pins `Span::place`, `Span::dominance`, and the `contains` argument rung by scan parity and never calls `coverage`; `filter_coverage_matches_the_composed_sweeps` is stream-level and never reaches `refine_partial`); executed: no
- Seen by: correctness; refutation: confirmed (reading filter.rs:477-515 shows a byte-equal `(lo, hi)` can never yield `Partial`, so the rung is equivalent today and only the pin is missing); history: no rationale found (the span rungs' cross-buffer pin was added deliberately in 4874f527; db9dfa3e names no such pin for the query rung)
- Owner-gated: no

The rung answers `Full`/`Empty` from `contains(lo)`; a byte-equal coincident span in distinct buffers takes `filter::coverage` and possibly `refine_partial`. The span module pins its analogous rungs with `coincident_span_rungs_agree_across_buffer_identity` and the scan-parity tests; the query module has no twin, so the step "`refine_partial` is never reached on a coincident span" lives in the reader's head. A future change to `refine_partial` (including the fix for span-causally-36) that made a coincident distinct-buffer span read `Partial` would pass every current test.

Evidence:

       129	        if lo.view().ptr_eq(hi.view()) {
       130	            return if self.contains(lo) {
       131	                Coverage::Full
       132	            } else {
       133	                Coverage::Empty
       134	            };
       135	        }

Resolution: a proptest beside `coverage_matches_membership_on_points` (or in causally/tests.rs) over `neutral_queries`/`down_queries`/`up_queries` asserting `q.coverage(Span::new(&v, &redecoded).unwrap()) == q.coverage(Span::at(&v))` with `redecoded = Version::decode(&v.encode()[..]).unwrap()`, mirroring `span_contains_matches_place`'s redecoded-argument leg. Acceptance: the property runs in `just test-all`; deleting the `ptr_eq` rung leaves it green (equivalence); forcing `refine_partial` to return `Partial` on a coincident clamp makes it red.

Construction: for any `v` and query `q`: `let w = Version::decode(&v.encode()[..]).unwrap(); assert_eq!(q.coverage(Span::new(&v, &w).unwrap()), q.coverage(Span::at(&v)))`. Today this passes; it is the missing pin, not a failing case.

**Nits in this section** (full records in the evidence files)

| id | where | claim | resolution |
|---|---|---|---|
| rank-7 | `crates/before/src/version/rank.rs:287` | A `None` or zero result allocates nothing (rank.rs:287, 343) has no allocation-counting instrument; the pair envelope meters `cmp`, `checked_sub`, and `+` together and the board's heap floor is NA. | A counting-allocator row for `checked_sub` alone on wide operands asserting zero heap delta. |
| version-core-21 | `crates/before/src/version/own/tests.rs:45-55` | `from_impl_is_to_version`'s doc says "without re-projecting" while `From` and `to_version` each project. | Reword to what the asserts show (the `Copy` view stays usable). |
| version-core-29 | `crates/before/src/version/tests.rs:656-672` | `trace_ticks`'s doc enumerates the ops but omits `Op::Ticks`, which the body charges `k`. | Add the arm to the enumeration or restate structurally. |
| version-core-34 | `crates/before/src/version/tests.rs:2195-2200` | The at-rest size pin hard-codes `32`/`64` beside the invariant it states; on a 32-bit host it would fail on a number, not the invariant. | Derive from `size_of::<usize>()` and `size_of::<Party>() + size_of::<Version>()`, or cfg-gate. |

## The skyline coding

**The coding: module root, admit, build, encode/decode, emit, text, walk**

### skyline-coding-6: the admission walk's mid-stream collapsible-pair rejection has no committed witness through Span::decode
- Where: crates/before/src/version/skyline/admit.rs:174-180 (related: crates/before/src/version/skyline/admit.rs:168-173 and 210-219; crates/before/src/span/tests.rs:333-353; crates/before/src/borsh_impls/tests.rs:1364-1389; crates/before/src/version/skyline/tests.rs:157-185; crates/before/tests/fuzz_seeds.rs:173-221; crates/before/src/span/wire.rs:150-158)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of span/tests.rs, borsh_impls/tests.rs, and tests/fuzz_seeds.rs for the fused door's non-canonical witnesses; the byte trace below derived by hand against admit.rs:116-219 and span/wire.rs:134-158); executed: no
- Seen by: correctness; refutation: confirmed (independent trace agrees); history: no rationale (the borsh witness names itself the "close-out arm" tripwire; 9cc00ccac unified `close_ancestor` without recording the step-arm gap)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). The composite `[0xE0, 0x3E, 0xE0]` decodes at HEAD to `Err(NotCanonical)` through `Span::decode`. With the step arm's `zero_delta` forced false, `Span::decode` accepts it as `Span { lo: 0, hi: (0, (0, 0, 0), 1) }`, a span whose `hi` carries the collapsible pair, while every committed span, borsh, validator, and fuzz-seed test in the 227-test run stays green.

`CheckedCursor` is a second implementation of the validator's minimal-topology check, reachable from wire bytes through `Span::decode` and the borsh span leg. The arm that fires when a collapsible pair's parent closes inside `step()` (leaves still to come) is exercised at gate tier by nothing: the two hand-built collapsible joins (span/tests.rs:338, borsh_impls/tests.rs:1375) are the root-level pair `0b0111_1000`, which closes in `finish()`; the planted-pair proptest drives `validate_bits` only; `span_wire_decode_matches_bitwise_reference` compares two cursors under the same fused walk; the replayed fuzz seeds' `NotCanonical` cases are `span_crossed` and `span_negative_join`; the fused-vs-composed differential is the off-gate fuzz target. Hard rule: decode strictly rejects non-canonical input because `Eq`/`Hash` rest on byte equality, so a gap here is an equality bug; doctrine: a criterion needs a committed demonstration that a known-bad mechanism fails it, and a walk that checks pairs only at `finish()` passes today's gate.

Evidence:

    admit.rs
    172	        let mut is_leaf = true;
    173	        let mut zero_delta = self.last_delta_zero;
    174	        loop {
    175	            match self.path.pop() {
    176	                Some(true) => {
    177	                    // This ancestor closes: the completed subtree was its right
    178	                    // child.
    179	                    self.close_ancestor(&mut is_leaf, &mut zero_delta)?;
    180	                }

    span/tests.rs
    338	    let collapsible: Vec<u8> = vec![0b0111_1000];

    borsh_impls/tests.rs
    1370	/// forbids. It rides beside
    1371	/// [`span_wire_decode_matches_bitwise_reference`] as the deterministic
    1372	/// tripwire for the close-out arm's genre.

Resolution: add to src/span/tests.rs, beside the existing witness, a deterministic composite whose join carries a pair closing mid-stream (the bytes below; note they carry a proper padding marker, which the existing `0b0111_1000` witness does not, passing only because `NotCanonical` fires before `require_marker_padding`), and a proptest mirroring `planted_collapsible_pairs_are_rejected_at_every_leaf` through `Span::decode`: take a canonical `lo <= hi`, split one leaf of `hi` into an equal-sibling zero-delta pair at a proptest-chosen preorder position (heights unchanged, so only canonicality can reject), re-pad, assert `Err(Decode::NotCanonical)`; run the composite through the borsh span leg too. Acceptance: `Span::decode(&[0xE0, 0x3E, 0xE0]) == Err(Decode::NotCanonical)` is asserted and the proptest is committed; replacing `self.last_delta_zero` with `false` at admit.rs:173 turns both red while every existing witness stays green.
Construction: composite `[0xE0, 0x3E, 0xE0]`: `lo` = `11` (the empty version) with marker at bit 2; `hi` bits `0 0 1 1 1 1 1 011` then marker at bit 10 (root internal, left internal, leaf gamma(0), leaf zigzag(0), leaf zigzag(+1)). Trace: `hi` opens at depth 2, `D = 0`. First `advance` (`lo` depth 0 < 2): `hi.step()` flips at level 2, pushes `left_was_leaf = true`, reads the second leaf's code 0, sets `last_delta_zero = true`. Second `advance`: `hi.step()` pops `true` and `close_ancestor(is_leaf = true, zero_delta = true)` sees `left_was_leaf = true`, so `Err(NotCanonical)`. With that arm's `zero_delta` forced `false`: the walk flips at the root, reads +1, `D = -1` (Less, `equal = false`), both cursors done, `finish()` pops the single right branch with `left_was_leaf = false` and returns `Dominates`; `require_marker_padding(tail, 10)` passes; `Span::decode` accepts a span whose `hi` is the non-canonical `[0x3E, 0xE0]`, a second live spelling of the constant-0-then-1 function under `Eq`.

### skyline-coding-23: two test-local depth recursions sit outside recurse.rs's inventory and bypass `descend!`
- Where: crates/before/src/version/skyline/tests.rs:349-366 (related: crates/before/src/version/skyline/emit/tests.rs:510-519; crates/before/src/version/skyline/tests.rs:419-429; crates/before/src/recurse.rs:9-14; crates/before/AGENTS.md:32-36)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (grep `descend!` outside recurse.rs: testing/bridge.rs, grow/tests.rs, query/tests.rs, meter/tests.rs only; recurse.rs:9-14 read; tests.rs:421 feeds `Dense.packed1(1_000)` to `assert_flag_bijection`, so `walk` recurses 1000 deep today); executed: no
- Seen by: correctness; refutation: confirmed; history: contradicts-hard-rule (1ddb5a483 wrote the inventory naming two witnesses while both recursions already existed)
- Owner-gated: no
- Cross-references: testing-oracles-3 (correctness class: the bridge's own `descend!` gaps) and testing-diff-gen's open question on the oracle-tree recursions in `shape_rows`, `grow_brute_force`, and `generators`.

crates/before/AGENTS.md:32-36: a walk that must recurse routes each call through `crate::recurse::descend!`, and recurse.rs's module doc "holds the inventory". `inverted_flag_stream::walk` recurses on oracle-tree depth over the generator families (depth 1000 for `Dense.packed1(1_000)`) and `grid_version::build` over the grid's log depth; neither routes through `descend!` and neither is in the inventory, so the stated inventory is false and a deeper family fed to `assert_flag_bijection` would overflow before the rule's guard sees it. Medium because it breaches a stated hard rule; the depths reached today are bounded by the generators.

Evidence:

    tests.rs
    349	    fn walk(t: &oracle::Version, offset: &Base, prev: &mut Option<Base>, out: &mut BitsBuf) {

    359	            oracle::Version::Node(n, l, r) => {
    360	                out.push(true); // internal flag, inverted spelling
    361	                let offset = offset + n;
    362	                walk(l, &offset, prev, out);
    363	                walk(r, &offset, prev, out);

    emit/tests.rs
    511	    fn build(values: &[Base]) -> crate::oracle::Version {
    512	        match values {
    513	            [v] => crate::oracle::Version::leaf(v.clone()),
    514	            _ => {
    515	                let (l, r) = values.split_at(values.len() / 2);
    516	                crate::oracle::Version::node(0u64, build(l), build(r))

    recurse.rs
    11	//! surface, where the remaining depth recursion lives: the differential oracle
    12	//! bridge (`testing::bridge`), whose walks mirror the paper's recursive trees,
    13	//! plus the test-local recursive witnesses beside it (the grow suite's
    14	//! reference cost probe, the meter suite's segment-liveness dive).

Resolution: route both recursions through `descend!` as testing/bridge.rs does, or add both to recurse.rs's inventory with the depth bound stated at each site (grid_version already states `O(log)`; inverted_flag_stream should state it is bounded by the generator depths it is fed). Acceptance: recurse.rs's inventory and `grep -rn 'fn walk(\|fn build(' crates/before/src` agree on the set of test-local recursions, and each site either uses `descend!` or states its bound.

### skyline-coding-13: builder testdocs misstate their bodies (leaf count; an unasserted cost claim)
- Where: crates/before/src/version/skyline/build/tests.rs:69-71 (related: crates/before/src/version/skyline/build/tests.rs:74-78, 109-120, 375-383)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read: both bodies feed three leaves at depths 2, 2, 1; the third body asserts only stream equality); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale (the mismatch is from birth at 147b55f6c)
- Owner-gated: no

AGENTS.md: every test's doc comment states the behavior it protects and review holds it to the standard; an incorrect testdoc is a bug in the test. Two docs say "four equal leaves at depth 2" over bodies feeding three leaves (the tiling of `((7, 7), 7)`), and a third says the wide code "is written exactly once" while the body instruments no writes.

Evidence:

    build/tests.rs
    69	/// The absorb cascade climbs: four equal leaves at depth 2 collapse pairwise
    70	/// all the way to a single depth-0 leaf, one parent-flag truncation per level,
    71	/// with the held code never moving.

    74	    let stream = built(vec![
    75	        (2, gamma(7)),
    76	        (2, delta(Sign::Positive, 0)),
    77	        (1, delta(Sign::Positive, 0)),
    78	    ]);

    109	/// Deep uniformity around a wide code stays a single leaf: a depth-8
    110	/// left spine of equal plateaus collapses level by level while the wide
    111	/// held code is written exactly once.

    375	    // Cascade: four equal depth-2 leaves collapse pairwise to depth 0.

Resolution: lines 69-71 and 375: "three equal plateaus, two at depth 2 and one at depth 1 (the tiling of `((7, 7), 7)`), collapse pairwise to a single depth-0 leaf". Lines 109-111: drop "while the wide held code is written exactly once", or make it true by asserting on the scan meter (the write count is what `record_bits` records). Acceptance: each testdoc describes exactly the leaf sequence its body feeds and claims only what its assertions check.

**Fill and grow**

### skyline-fill-grow-30: No deep witness drives a late divergence after a long matched prefix (`Out::materialize`'s replay at scale)
- Where: crates/before/src/version/skyline/fill/tests.rs:1029-1196 (related: crates/before/src/version/skyline/fill.rs:72-74; crates/before/src/version/skyline/fill/fuse.rs:200-240; crates/before/src/version/skyline/fill/tests.rs:1626-1636)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (traced each deep case in `deep_spines_tick_and_flag_identically`: the changed ones (the full-id collapse, the wide-tail mirror, the memo chain, the reveal comb) diverge at their first emission and the rest never diverge; `deep_and_wide_ticks_match_iterated` sweeps depth only to 128; traced the corrected construction below through fill.rs's ordinary-node arm (absent-left copy of a lone zero leaf matches via `emit_step`'s `note_match`) and the left-full absent-right arm (`scan_min_from` gives 5, `signed_max(0, 5) = 5`, `emit_offset` diverges on a nonzero offset over a single-leaf range)); executed: no
- Seen by: correctness (33); refutation: reframed (the gap is real; the original construction was off by one id level, since with `d` right-only id levels the tip sits over the leaf `5`, whose fill is the identity, and the pair takes the grow branch); history: no-rationale-found (the spec prices copy-on-first-divergence and the witness roster lists its regimes, but neither names a long-matched-prefix divergence)
- Owner-gated: no

The module doc claims a divergence replays the matched prefix once, and `Out::materialize` re-decodes that prefix through a fresh builder on a `LeafWalk` bit stack. Every committed deep witness either diverges within its first emission or never diverges, and no tick envelope names a late divergence, so the regime in which thousands of plateaus match verbatim before the walk diverges is exercised only at oracle scale (depth ≤ 64 family pool, ≤ 128 sampled). A regression in the replay (a quadratic re-materialization, a held-leaf mistake at the prefix's tail after thousands of plateaus) would surface only by luck. Asymptotic claims need a committed instrument that fails when they are false.

Evidence:

    (fill.rs)
        72	//! the tags the walk's skips pay anyway — and each of the two branch epilogues
        73	//! is one more bounded pass: a divergence replays the matched prefix once, and
    (fill/tests.rs)
       989	/// The regimes: the collapse scan, the pass-through copy, the two-cursor
       990	/// descent, the memoized pre-scan, and both fused epilogues (the prefix
       991	/// materialization and the route-driven splice).

Resolution: Add a closed-form deep case to `deep_spines_tick_and_flag_identically` through `assert_deep_changed` at `d = 4096`: event `"(0, 0, ".repeat(d) + "5" + ")".repeat(d)` (a right spine of `d` nodes, every left leaf 0, tip leaf 5); id `"(0, ".repeat(d - 1) + "(1, 0)" + ")".repeat(d - 1)` (the tip `(1, 0)` sits over the deepest `(0, 0, 5)` node); expected `"(0, 0, ".repeat(d - 1) + "5" + ")".repeat(d - 1)`. The walk matches `d − 1` pass-through zero leaves, the tip's left-full raise lifts 0 to 5 through the absent-right arm, the equal pair collapses, and the divergence replays a `d − 1`-plateau prefix. Optionally register the shape as a `tick_late_divergence` envelope row (envelopes partition) pinning scan bits at about 2× the event's. Acceptance: the deep test asserts the closed form, canonicality, the re-walk flag clear, and entry agreement at `d = 4096`.

Construction: as in the resolution; the flag must trip and `tick` must equal the expected literal. If the pair takes the grow branch instead, the id has one level too many.

### skyline-fill-grow-31: The orbit test's doc states a tighter log term than its body asserts
- Where: crates/before/src/version/skyline/fill/tests.rs:1289-1331 (related: crates/before/src/version/skyline/fill/tests.rs:1352, 1369)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read 1293 and 1318; `32 - (k + 1).leading_zeros()` is the bit length of `k + 1`, which is `⌊log2(k + 1)⌋ + 1` and exceeds `⌈log2(k + 1)⌉` by one exactly when `k + 1` is a power of two, i.e. at k = 3, 7, 15, 31 inside `2..=48`); executed: no
- Seen by: prose (25); refutation: reframed (the formula half holds; the "type boundary" half is a defensible name for `Party`'s decode gate and is dropped); history: no-rationale-found (the landing commit 4e2c88bb and the spec state the `⌈log2(k + 1)⌉` form while the code computed bit length from the start)
- Owner-gated: no

Every test's doc comment states its invariant and "their incorrectness is a bug in the test". The doc's bound is 4 bits tighter than the asserted one at those k; a reader re-deriving from the doc gets a different envelope. Whether the tighter bound holds is unknown without running.

Evidence:

      1293	    /// `bits(tick^k) ≤ bits(tick^1) + 4·bits(id) + 4·⌈log2(k + 1)⌉ + 8` for
      1318	                let logk = u64::from(32 - (k + 1).leading_zeros());

Resolution: Decide the intended claim (open question below). If the tighter bound: `let logk = u64::from(u32::BITS - (k + 1).leading_zeros()) - u64::from((k + 1).is_power_of_two());` here and at 1352 and 1369, then run the three orbit tests. If the looser: write the doc as the code computes it, `4·bitlen(k + 1)` ("four bits per bit of `k + 1`"). Acceptance: the doc formula and the `logk` expression agree for `k + 1` a power of two.

Construction: change 1318 to the ceiling form and run `tick_orbit_growth_is_transient_plus_log` and `tick_deep_orbits_stay_banded` (with 1352 and 1369 changed likewise); a pass settles the doc as correct and the code as loose, a failure settles the reverse.

### skyline-fill-grow-38: Test scaffolding is triplicated and grow's pools are a hand-copied strict subset of fill's
- Where: crates/before/src/version/skyline/grow/tests.rs:211-253 (related: crates/before/src/version/skyline/grow/tests.rs:43-51, 388-393; crates/before/src/version/skyline/fill/tests.rs:46-54, 116-202, 979-984; crates/before/tests/meter.rs:2059-2064; crates/before/src/lib.rs:453-454)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`grep -rn -E 'fn (left_spike|version_of|party_of|event_pool|party_pool)\b' src tests`: `left_spike` at grow/tests.rs:388, fill/tests.rs:979, tests/meter.rs:2059 with identical bodies; `version_of` in ten test files, `party_of` in three; grow's sixteen event entries are fill's first sixteen in order and grow's eight party entries are fill's first eight plus the same `all_normal_ids(2)` loop; `testing` is `#[cfg(test)]`, so the integration crate cannot reach it); executed: no
- Seen by: structure (10), correctness (36); refutation: confirmed; history: no-rationale-found (grow's pools are the 183e3cab originals; no family-landing commit touched grow/tests.rs, and nothing describes the subset as deliberate)
- Owner-gated: no

Two hand-maintained copies of one roster drift silently: a family added to fill's pool is never route-pinned unless someone remembers the second list, and today the reference route probe and the oracle-grow byte witness never run over the memo, reveal, comb, cliff, staircase, wide-tail, or nested-id shapes (fill's `assert_tick` still pins those pairs' bytes against the oracle's `event`, so the exposure is the route pin and the per-family grow count). `FAMILY_GROW_PAIRS` protects the count of the pool it has, not the pool's parity with fill's. `left_spike` is byte-identical in three files.

Evidence:

       211	/// The adversarial event pool the deterministic grids run over.
       212	fn event_pool() -> Vec<Version> {
       388	fn left_spike(depth: usize) -> Version {
       389	    let mut text = "(0, ".repeat(depth - 1);
       390	    text.push_str("(0, 1, 0)");
       391	    text.push_str(&", 0)".repeat(depth - 1));
       392	    text.parse().expect("the spike literal is normal form")

Resolution: Move `left_spike`, `version_of`, `party_of`, and the family pools into `crate::testing` (a `pools` module beside `generators`); have grow/tests.rs draw its pools from the same source, either the full roster or a named subset with the exclusion stated at the site; re-derive `FAMILY_GROW_PAIRS` in the same commit. tests/meter.rs keeps its `left_spike` unless `testing` is exposed under the `meter` feature; note that at the site. Acceptance: one definition each of `left_spike`, `version_of`, `party_of` under `src/`; grow's pools defined in terms of a shared roster; `family_pairs_grow_identically` green with its pin re-derived and its doc still stating the count is exact.

**Comparison kernels: sweep, place, masked, overlay, signed**

### skyline-sweep-place-masked-19: The placement write-sequence identity is claimed pinned by touch readings, but the placement rows read no touch meter
- Where: crates/before/src/version/skyline/place.rs:527-531 (related: crates/before/src/version/skyline/place/filter.rs:249-256; crates/before/src/version/skyline/overlay.rs:233-236; crates/before/tests/meter.rs:9472-10084; crates/before/src/meter.rs:3561-3624; crates/before/src/codec/base.rs:274-282)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of tests/meter.rs lines 9463-10090 for `touch_meter|touches|limb_ops|scan_bits`: only `reset_scan_bits`/`scan_bits` at 9480-9482 and `reset_limb_ops`/`limb_ops` at 9707-9709; `Base`'s `Magnitude` impl at base.rs:274-282 records nothing; `limb_ops` counts `Base` operations and wide-gamma decodes, `touch_ops` is the separate suanpan counter); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (the placement rows have pinned scan and limb identities only since 0cbb9dc7/e2f4e2a5; the generic "committed touch-meter readings pin each walk's sequence" sentence generalizes from the masked walk, whose rows do pin touches)
- Owner-gated: no
- Witness (witness/results.md): demonstrated with a caveat (run). With PROBE moved last in the placement priority, all nine `placement::` rows in tests/meter.rs and both named identities stay green, as predicted. The swap changed no touch reading on either fixture tried (a 2^80-plateau clock chain and a 2^20/2^18 cliff-comb chain), so the rows' blindness is demonstrated but the premise that a reorder changes the committed write sequence remains unverified by measurement; with `QUICK_MAX = 2^96` a word-scale tie cannot spill the quick register unless a wide value already forced the digit engine.
- Cross-references: board-ops-render-10 (the board's placement rows declare the same touch column not applicable).

place.rs, filter.rs, and overlay.rs state that the accumulator write-sequence identity between each fused walk and its pair sweep is pinned by the placement rows in tests/meter.rs. Those rows read scan bits (order-independent: both orders read the same bits) and limb ops (blind to accumulator writes: `Base`'s `Magnitude` impl records nothing, and word-scale folds ride the quick register). A priority reorder that changes carry work would pass every placement row, so the instrument the docs name pins something weaker than the docs claim (Principle 8, and the adequacy rule).

Evidence:

       527	/// Priority `[PROBE, START, END]`: the probe steps first on every tie — it is
       528	/// every pair's first operand, and the binary law's equal-depth arm steps its
       529	/// first operand first — keeping each accumulator's write sequence identical
       530	/// to its pair sweep's (the placement identity rows in `tests/meter.rs` pin
       531	/// each fused walk against its composed sweeps). The start/end order among

    overlay.rs
       233	    /// which no current client does. The order is contract, not convenience: a
       234	    /// walk whose slots share an accumulator commits its digit writes in step
       235	    /// order, so the committed touch-meter readings pin each walk's sequence
       236	    /// (each impl documents which identities pin its own). The iterator is

Resolution: add `touch_ops` identity legs beside the scan and limb legs in the placement module (for the single exhaustion-confirmed bound, `touches(contains) == touches(partial_cmp)`; for `span_place_scans_each_stream_once`, a relational touch identity against the composed sweeps), on a fixture whose deltas cross a carry boundary (the CliffComb family) so step order moves the reading; then point the three docs at the touch legs by name. Acceptance: with the new legs, reordering `Cursors::priority` to `[START, END, PROBE]` (verdicts, scan, and limb rows unchanged) turns the touch legs red on the carry-boundary fixture; restoring the order turns them green.

Construction: swap `PROBE` to last in place.rs:535. Every verdict suite passes (folds are commutative sums); `span_place_scans_each_stream_once` and `query_single_bound_matches_the_pair_sweep_limbs` pass (scan bits and `Base` ops are order-independent). Only a touch identity on a fixture where the a-then-b and b-then-a write orders commit different carry work separates; on a quick-register-resident fixture both orders touch identically, so the fixture must be a carry-boundary comb.

### skyline-sweep-place-masked-20: The coverage walk's three documented early exits have no resource pin and no deterministic witness
- Where: crates/before/src/version/skyline/place/filter.rs:26-35 (related: crates/before/src/version/skyline/place/filter.rs:427-440; crates/before/src/version/skyline/place/tests.rs:330-379; crates/before/tests/meter.rs:9472-10084)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -n '\.coverage(' tests/meter.rs` returns nothing; `grep -in coverage tests/meter.rs` returns only two unrelated doc lines, 4029 and 6060; the placement module measures contains, place, dominance, and precedence); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (db9dfa3e landed four membership rows and none for coverage; no coverage row has ever existed)
- Owner-gated: no
- Witness (witness/results.md): demonstrated in a narrowed form (run). The literal deletion of lines 427-430 and 439-440 is caught, but only by the committed `debug_assert!` at filter.rs:441-450, which fires in five tests because settled holes with both pairs dead stay in their slots; no verdict changes. Deleting only the two exits that assert cannot see (the `live == 0` early `Full` return and the endpoint-cursor drops) passes every place, causally, laws, and verdict-matrix test.

The module doc promises three coverage exits: a hole refuted at both endpoints drops its stream, a probe endpoint whose every pair is settled stops being scanned, and a walk holding only settled holes returns `Full` without exhausting anything. The membership, span, and dominance walks each carry a scan-bit row stated relationally against the pair sweep; coverage has none. `filter_coverage_organic_witnesses`' `Full` case carries `After` and `Before` demands, which never drop (`After` settles only `lo`, `Before` only `hi`), so the `live == 0` return and both endpoint drops are unreached there; only the proptest can reach them, by chance. A `coverage` with the three drop paths deleted passes every committed test, because a settled pair's stale directions give `finish` the same answer.

Evidence:

        26	//! - [`coverage`]: a floor refuting `floor <= hi` (or a ceiling
        27	//!   refuting `lo <= ceiling`) proves no covered version is admitted —
        28	//!   [`Coverage::Empty`] at the refuting interval, the verdict a
        29	//!   pruning tree walk consumes. A hole whose subtraction is refuted at
        30	//!   both endpoints is settled and drops its stream, a probe endpoint
        31	//!   whose every pair is settled drops its own cursor, and a walk left
        32	//!   holding only settled holes returns [`Coverage::Full`] without
        33	//!   exhausting anything. `Partial` alone always confirms at

       427	            if !side.lo.live && !side.hi.live {
       428	                *slot = None;
       429	                live -= 1;
       430	            }
       ...
       439	        walk.lo_live = walk.lo_live && walk.sides.iter().flatten().any(|side| side.lo.live);
       440	        walk.hi_live = walk.hi_live && walk.sides.iter().flatten().any(|side| side.hi.live);

Resolution: add two scan-bit rows to the placement meter module, stated relationally like the others: (1) a hole-only query concurrent to both endpoints must read strictly under the composed four `partial_cmp`s and stop at the second deciding interval (the early `Full`); (2) a required floor plus a hole whose `lo` pair settles must show the `lo` stream's scan stopping (compare against the same query with the hole removed). Add the two deterministic verdict witnesses (all-holes-settled `Full`; endpoint drop then `Partial`) to `filter_coverage_organic_witnesses`. Acceptance: the new rows read green at HEAD and red when lines 427-430 and 439-440 are removed, while every verdict test stays green.

Construction: delete lines 427-430 and 439-440. Every verdict suite (place/tests.rs proptests, `coverage_is_exact_on_the_two_party_grid`, the laws, the verdict matrix) passes because the exits are value-preserving; no meter row fails because none measures `coverage`.

Synthesis note: The witness narrows the claim: two of the three documented exits are unobserved by any committed verdict or meter, and the third (the slot drop) is observed by a dev-build invariant assert, never by a resource pin. The entry's "passes every committed test" sentence is too strong for the literal deletion; the headline and the resolution stand.

### skyline-sweep-place-masked-32: The early-exit discipline is pinned only for the test-gated `sweep::eq`; production `causal_cmp` and both masked exits are unpinned
- Where: crates/before/src/version/skyline/sweep.rs:37-52 (related: crates/before/src/version/skyline/masked.rs:61-65; crates/before/src/version/skyline/sweep.rs:233-242; crates/before/tests/meter.rs:4983-5100, 9637-9649, 9906-9912; .cargo/mutants.toml:48-51)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of tests/meter.rs: the only early-exit pin is `mod eq_early_exit` (4999, under `limb-meter`) calling `meter::skyline::sweep::eq` at 5045; the masked rows assert `Some(Less)` with "no early exit" at 4360-4364 and 7483-7487; the concurrent references at 9639-9648 and 9906-9912 are bounds the fused walk must sit under, which only loosen if `order_exit` sweeps to exhaustion; mutants.toml:48-51 names only `eq_exit`'s guard as instrument-killed); executed: no
- Seen by: claims; refutation: confirmed (every measured `partial_cmp(...).is_none()` site checked); history: no rationale found (bff7b04c chose `eq()` and says nothing about the strict-mix exit)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). At HEAD the production `causal_cmp` and both masked exits read tail-independent absolutes (touches and scan identical at t and 2t), so the pin asked for is constructible and passes today. With `order_exit` always continuing, all three read tail-linear (8005 to 16005 touches, 36020 to 72020 scan bits) while the verdict-matrix production run and both twins, the sweep/cmp tests, and the span suites stay green. One incidental catch: `coincident_place_collapses_to_the_pair_sweep` in tests/coincident_span.rs goes red through its relative inequality (the pair sweep's 116 scanned bits exceed the fused walk's 101), not through any pin of the exit.

The sweep doc's early-exit clause ("a decided sweep reads no more of either stream") is a measured number only for `eq`, which is `#[cfg(any(test, feature = "meter"))]`. The production exit predicate `order_exit` (the strict-mix `None` behind `Version`'s `PartialOrd`) and masked.rs's two exits have no work pin, and every masked row runs to exhaustion. An `order_exit` that always returns `Continue` is value-equivalent (`Directions::relation` at exhaustion still yields `None`), so every verdict suite stays green, and every relational row that uses a concurrent `partial_cmp` as its reference only loosens. Every criterion needs a committed demonstration that the known-bad mechanism fails it.

Evidence:

        44	//! is (any `D > 0`). A refuted direction stays refuted, so breaking at the
        45	//! question's resolution never moves a verdict the completed sweep would have
        46	//! reached, and a decided sweep reads no more of either stream. That last
        47	//! clause is a measured number, not just a claim: the `eq_early_exit` row of
        48	//! the resource-envelope suite (`tests/meter.rs`) pins the sweep's touch and

       121	#[cfg(any(test, feature = "meter"))]
       122	pub fn eq(a: BitsView<'_>, b: BitsView<'_>) -> bool {

    tests/meter.rs
      4996	// stay green. This is the one committed row where the early-exit prose
      4997	// is a measured number rather than a claim.
      4998	#[cfg(feature = "limb-meter")]
      4999	mod eq_early_exit {

Resolution: add `cmp_early_exit` rows in the `eq_early_exit` idiom: a pair decided concurrent at its second elementary interval with a scale-varying tail, absolute two-scale touch and scan pins on `Version::partial_cmp`, plus the masked twins (`(v / p).partial_cmp(&w)` with the deciding intervals inside owned regions, and a masked `eq`). Update sweep.rs:46-52 and masked.rs:61-65 to name the rows. Acceptance: with `order_exit` returning `Continue` unconditionally (and masked's `run` ignoring `Break`), all verdict suites remain green and the new rows read tail-linear and fail at both scales; restored, the rows read identical at both scales.

Construction: `a` = left half at height 1, right half a CliffComb tail of t teeth; `b` = left half at height 0, right half a single plateau above every tooth. Interval 1 has D > 0 (refutes `le`), interval 2 has D < 0 (refutes `ge`), so `order_exit` breaks at the second fold before either cursor enters the tail. Pin touches and scan bits at t and 2t with fixed tooth magnitude, as `eq_early_exit` does at tests/meter.rs:5004-5089.

**Query: rank, distance, lag, min_ticks, project**

### skyline-query-9: The log-factor clause of the public `O(M(|v|) · log |v|)` contract has no committed instrument at the tier where it applies
- Where: crates/before/src/version/skyline/query/integral.rs:220-229 (related: query/integral.rs:243-254; before-fuelscape/src/ops.rs:360, 602, 616; before/src/version.rs:289-293; tests/meter.rs:5220; fuzzfit/harness/src/bands.rs:892-903; src/meter/board/family.rs:285-300)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read the fuelscape contracts, `WIDE_ARMING_SMALL = 500`, `ff_version_rank`'s `max_denom: 8767`, and the board's ~13 KiB wide-arming probe); executed: no
- Seen by: claims; refutation: reframed (fuel counts the backend's instructions, so "no deterministic counter can see it" is false; a wall-time two-scale pair cannot resolve a log factor); history: already-known (the gap is declared at integral.rs:223-225 and in the adversarial note; "not commissioned" there refers to removing the log, not witnessing it)
- Owner-gated: yes: whether a declared, argued, uninstrumented public clause is acceptable, and whether a bench-only or CI-only witness enters gate policy

The public `# Complexity` for `Version::rank`, `distance`, and `lag` is `O(M(|self|) · log |self|)`. The argument is in this doc; every committed two-scale instrument sits below the 4,000-word tier where the log factor applies (meter bands at WA(500..1000), fuzzfit to 8.7 KiB, the board probe at ~13 KiB), and the doc says so. Under the crate docs' "any violation is a bug" standard an asymptotic claim needs an argument, a matching implementation, and a committed instrument that fails when it is false; the third is absent for exactly the clause that distinguishes the contract from `O(M)`. The gap is disclosed, which is why this is low rather than higher.

Evidence:

    220  //! The shipped backend dispatches power-law tiers up to 4,000-word operand
    221  //! sides (~32 KiB parked sums per side). Past that its quasilinear tier's
    222  //! per-level costs stop telescoping, and the settle pays at most one extra
    223  //! tree-depth factor, `O(M(|v|) · log |v|)` — and the log factor is tight there
    224  //! [derived; a committed witness at this scale would need 65 KiB+ packed
    225  //! operands]:

    (ops.rs:360)          contract: "`O(M(|self|) · log |self|)` time, `O(|self|)` space",

Resolution: Either instrument or narrow. To instrument: a multi-scale fuel fit (the fuzzfit harness already counts wasm fuel for `ff_version_rank`) over the doc's tight construction at three or more scales past 65 KiB, judged against the `M(n) · log n` model with `M ≈ n log n`; or a deterministic check that records `meter_product`'s operand widths per tree level on that construction and holds the per-level product widths to the model's telescoping. To narrow: state in the public contract only what committed instruments pin and move the quasilinear-tier remark to a decision record. Acceptance: either a committed cell or test whose operands' parked sums exceed 4,000 words at every scale, named from integral.rs in place of the "[derived; ...]" bracket and rostered, or the public `# Complexity` no longer carries the unwitnessed clause.
Construction: Build the doc's own worst case: `Θ(log |v|)` armings whose parked widths grow as `4,000 · 2^i` words, each banked ahead of a trailing window of span `Θ(|v|)`, at |v| of about 65 KiB, 130 KiB, and 260 KiB packed; run `Version::rank` under the fuzzfit fuel harness; fit the exponent across the three scales against `n log^2 n`. A settle that re-ran an NTT-tier product once per tree level without telescoping reads the extra `log` here and in no committed counter.

### skyline-query-31: The adequacy kernels hand-copy the shipped driver loops, `finish`, and the settle reduction under "verbatim" claims that have drifted
- Where: crates/before/src/version/skyline/query/tests.rs:1684-1755 (related: query/tests.rs:1577-1583, 1914-1920, 2048-2119, 2263-2352, 2354-2373, 2482-2606; query.rs:201-225, 327-401 (351-357, 391-396); query/integral.rs:825-841, 1043-1130, 1142-1163)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`git show aa7c96a0 -- integral.rs` removes `if final_window_magnitude != UBig::ZERO {`, `if segment_magnitude != UBig::ZERO {`, and `let negative = (coefficient < 0) != (sign == Ordering::Less);`; `git show f77011e3 -- query.rs` replaces `.unwrap_or(1);` with `.expect("the advance law steps at least one side per boundary");`; `git show 9c7b6999 -- integral.rs` replaces `if self.promotions.is_empty() { return; }` with the debug_assert; the copies read side by side with the shipped bodies); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed (adds the `is_empty` early return and the dropped opening-sign step), severity medium -> low; history: deliberate-but-expired (the copies were verbatim when written and the lockstep was an explicit hand-maintained convention: 14259c1f re-synced them "so their 'reduction verbatim' claims stay true", and 982bd260 shared `mass_split` after a copy of the shipped rule masked an actual mutation through the full suite; the 2026-08-12 trio then changed the shipped bodies without mirroring)
- Owner-gated: no

The `adequacy` module reproduces the shipped `rank` loop four times (1686, 2050, 2357, 2590), the `pair_fold` loop twice (1706, 2070), `Integrator::finish` twice (2334, 2555), and the `settle_armings` reduction twice (2266, 2485), each documented as the shipped body "verbatim" with one move swapped. Every such claim is false today: the settle copies open with `if integ.promotions.is_empty() { return; }` where the shipped code debug-asserts non-emptiness; they and the `finish` copies guard `final_window_magnitude != UBig::ZERO` and `segment_magnitude != UBig::ZERO` where the shipped code is unconditional; the pair copies end their funded-width fold in `.unwrap_or(1)` where the shipped loop uses `.expect(...)` and omit the shipped opening-sign step (query.rs:351-357); the two standalone integrators keep the pre-aa7c96a0 conditional `jump`. Every divergence is value-neutral on reachable inputs and each kernel is value-pinned against the shipped fold on every run, so no tripwire is measuring a hybrid; the cost is false prose at ten sites (Principle 5) and a hand-maintained sync that has failed three times in one day and that the crate already knows can mask a mutation (982bd260). That the hazard has bitten is why this stays medium rather than low.

Evidence:

    1684      /// The rank fold on the span-reading integrator: the shipped
    1685      /// [`rank`](super::super::rank) loop verbatim, integrator swapped.
    1746              let funded = da
    1747                  .iter()
    1748                  .chain(db.iter())
    1749                  .map(|step| super::super::integral::int_digits(&step.magnitude))
    1750                  .max()
    1751                  .unwrap_or(1);

    (query.rs:396)              .expect("the advance law steps at least one side per boundary");

    2263      /// The shipped ledger settle with the per-digit absorb: the
    2264      /// mass-balanced product-tree reduction verbatim, every window
    2265      /// merge routed through [`merge_per_digit`].
    2282          if final_window_magnitude != UBig::ZERO {
    2283              windows.merge(&final_window_magnitude, final_window_shift);
    2284          }

    (integral.rs:1075-1076)
    1075          let mut windows = WindowMass::new();
    1076          windows.merge(&final_window_magnitude, final_window_shift);

Resolution: Test-local refactor, no production change (the precedent 982bd260 set for `mass_split`): one small trait (`open`, `interval`, `jump`, `boundary`, `live(&mut)`, `finish`) implemented by a thin wrapper over the shipped `Integrator` and by each known-bad integrator; one `fold_single` and one `fold_pair` driver in the test module; one `settle_with(integ, merge)` reducer shared by the per-digit and schoolbook kernels; one shared `finish_with(integ, close)`. The kernels then differ from the shipped code only in the component they refute, and "verbatim" becomes true by construction where it is still claimed. If the owner prefers to keep the copies independent, strike "verbatim" from all ten docs and re-sync the copies to the current shipped bodies (drop the zero guards and the `is_empty` return, adopt the unconditional `jump`, match `.expect`, add the opening-sign step). Acceptance: no doc in tests.rs says "verbatim" about a body that is not a shared function; `grep -n 'final_window_magnitude != \|segment_magnitude != \|unwrap_or(1)' tests.rs` is empty; every `_reads_superlinear` test keeps its rostered name and still reads red at its floor and value-exact against the shipped fold under `just test-all`.

### skyline-query-25: The promoting-pool rationale cites an `arb_base` ceiling the generator no longer has
- Where: crates/before/src/version/skyline/query/tests.rs:237-242 (related: query/tests.rs:644-645; testing/generators.rs:322-332)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`git log -1 28f6981e` message: "previously every arm topped out near 2^129"; the widened arm at generators.rs:330 read); executed: no
- Seen by: correctness; refutation: confirmed (promotion reachability from the arbitrary sweep was not sampled either way); history: deliberate-but-expired (e695d5cf wrote both docs against the then-true ceiling; 28f6981e widened the generator and updated neither)
- Owner-gated: no

The doc rests on "`arb_base` tops out near 2^128, under half of that". generators.rs:330 now has an arm `(2j + 1) << k` for `k in 0..512`, so `arb_base` reaches about 2^514 (17 digits), and a wide node base over two small leaves fires the freeze trigger inside `arbitrary_trees_agree`. The pool's unique contribution (promotion, and the arming trains' repeated and mixed-sign armings) holds but is not what the doc says; a test doc's incorrectness is a bug in the test (root AGENTS.md).

Evidence:

    237  /// The other pools stay under the freeze allowance almost everywhere: a
    238  /// unit-funded fold freezes only past 9 digits (288 bits) of live drift, and
    239  /// `arb_base` tops out near 2^128, under half of that — so the promotion ledger
    240  /// and its product-tree settle would run differentially unwitnessed without
    241  /// this pool: these shapes are the only ones that arm it, and the arming trains
    242  /// are the only ones that arm it more than once per sweep or with mixed signs.

    (generators.rs:330)          1 => (0u64..4, 0u32..512).prop_map(|(j, k)| codec::Base::from(2 * j + 1) << k),

Resolution: Restate the rationale against generators.rs as it is: freezes are in-support for arbitrary trees; what the pool uniquely supplies is promotion (a parked component at least 10 digits wide against a wide-spelled narrow drift, or an 18-digit parked sum) and the multi-arming trains, which the arbitrary sweep reaches rarely if at all. Delete both 2^128 figures (237-242 and 644-645). If the exclusivity sentence ("these shapes are the only ones that arm it") is to stay, back it with a promotion tap over `family_pool` and the arbitrary sweep; otherwise drop it. Acceptance: `grep -n '2^128\|128-bit' tests.rs` returns nothing and the stated premise matches the widest arm in generators.rs:322-332.
Construction: Not a runtime construction; the falsity is textual. For the freeze claim: `spine_of(&[Base::from(1u8), (Base::from(1u8) << 500) + Base::from(1u8), (Base::from(1u8) << 500) + Base::from(3u8)])` folds deltas of about 2^500 then +2 and trips the trigger (16 live digits against 1 funded + 8), a shape `arb_oracle_version` generates whenever a wide-arm node base sits over two small leaves.

### skyline-query-27: `zero_drift_freezes_keep_the_totals_exact` claims a tripped trigger and an empty freeze that nothing in the test observes
- Where: crates/before/src/version/skyline/query/tests.rs:397-406 (related: query/integral.rs:276-285, 858-875; query/web.rs:384-396; query/tests.rs:358-375; tools/covcheck-expected.json:203-209; justfile:1005-1026)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read the test, `Integrator::freeze` (the zero arm returns before the `FREEZE_HITS` bump at 875), `EpochLedger::freeze` (no tap), and the coverage pin, which lists one `integral.rs` anchor and no `web.rs` entry); executed: no
- Seen by: prose, correctness; refutation: confirmed, severity medium -> low; history: deliberate-and-holds for the instrument choice (f77011e3 designed this family as coverage-driven, with the CI coverage legs as the committed liveness instrument: an undriven integral.rs:864-865 fails the line leg by name and a never-false web.rs:388 fails the branch leg), so the finding converts to "state the instrument at the test", with two residuals
- Owner-gated: no

The doc says the width trigger trips on a zero-valued live component, that the rank integral parks nothing, and that min_ticks keeps its epoch; the body asserts value equality only. The module's own standard (integral.rs:281-283) is that a freeze-regime witness must prove its inputs park; this test carries no floor and could not use `FREEZE_HITS`, which is bumped after the zero arm returns. The committed liveness instrument is the coverage pin, which runs in CI only ("the gate never runs them") and is not named at the test. The arm is value-neutral (deleting both zero arms leaves every total exact: a zero drift parks a zero, and every settle skips a zero parked magnitude or zero factor), so the doc's "freezes nothing" and "keeps its epoch" are cost claims of the kind the mutants roster classes as performance-genre for the neighbouring trigger legs.

Evidence:

    397      /// The zero-drift family: a width trigger tripped by a live component
    398      /// whose value is exactly zero (spelled wide) freezes nothing.
    399      ///
    400      /// The rank integral parks no drift and min_ticks keeps its epoch, and
    401      /// both folds stay exact against the tree oracle through the empty
    402      /// freeze.
    403      #[test]
    404      fn zero_drift_freezes_keep_the_totals_exact(p in 9u32..=13, d in 1u64..=6) {
    405          assert_single(&spine_of(&zero_drift_heights(p, d)));
    406      }

    (integral.rs:860-866, 875)
    860          if drift == UBig::ZERO {
    864              self.live.reset();
    865              return;
    866          }
    875          FREEZE_HITS.with(|hits| hits.set(hits.get() + 1));

Resolution: Either (a) add a `#[cfg(test)]` tap on the zero-drift arm of `Integrator::freeze` and on the discard arm of `EpochLedger::freeze` and floor both here (at least one per fold per case), so the gate itself sees the regime; or (b) keep the coverage legs as the instrument and say so in the doc: "value-exact on the zero-drift schedule; that the arm is driven is pinned by the CI coverage legs (tools/covcheck-expected.json lists no entry for either arm), not by this test". Acceptance: under (a), deleting the `if drift == UBig::ZERO` arm at integral.rs:860-866 or the `if drift != UBig::ZERO` guard at web.rs:388 turns this test red; under (b), the doc and body agree.
Construction: Change the strategy to `p in 1u32..=2` (or raise `FREEZE_ALLOWANCE_DIGITS` to 16): the live spelling never exceeds the allowance, no trigger fires, and the test still passes; independently, delete both zero arms and the test still passes, because parking a zero is value-identical to not parking it.

**Watermark and the traffic counters**

### skyline-watermark-21: The test module's unreachability claim is false for min_ticks, which reaches drop_below's latent annihilation with no directed public-API witness
- Where: crates/before/src/version/skyline/watermark/tests.rs:1-18 (related: watermark.rs:452-463, 497-505, 514-519, 541-545; query/web.rs:322-334; query.rs:437-470; query/tests.rs; src/meter.rs:2897-2916; testing/exhaustive.rs:83, 104)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified by reading the structural path: `ReignWeb::leaf` (query/web.rs:322-333) calls `undercuts_here` then `undercut` with no `compare_above` in front; `decide_undercut_through_latent` returns `true` with the latent still live on its gap-dominates exit (501-502); `undercut` → `drop_below` then annihilates at 541-545. The driver opens ranges only per left branch (`web.open(cursor.depth() - flip)`, query.rs:452), so a right leaf under a range whose left child just closed with a park emits with no pending range against a live latent, unlike the fill walk, where every child (leaf included) gets its own `web.open(1)` (fill.rs:505, 535, 544, 602, 608; prescan.rs:267) and a post-park emission always arms. Absence of a min_ticks-side witness verified by grep (`latent` occurs under `query/` only in web.rs; in tests/meter.rs only for the ladder and width-circulation families; the exhaustive scope is depth 2 over bases {0,1,2}). The numeric construction and closed form were traced by the correctness lens and re-traced by the refutation pass, not executed; executed: no
- Seen by: correctness; refutation: confirmed (re-traced the construction and both answers); history: no-rationale-found (29ee22fb's recorded reasoning covers only the `compare_above` readers; its tripwire that "the committed wide-arming and query suites stay green" under a dropped annihilation shows no committed query test drives the arm, not that a stream cannot)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). The finding's tree, written in the crate's normal-form text, round-trips through the public text door and `min_ticks` reads 2^34 + 2 = 17179869186 in production; with the latent annihilation at watermark.rs:543 removed the public answer becomes 17179869185, so a public-API input executes the arm the test module doc says no packed-stream walk reaches.

Test doc comments must be accurate (an incorrect one is a bug in the test), and differential suites exercise the public API with internal-entry checks only as documented decisions whose stated rationale holds. Here the rationale is false for one of the two named gates: `drop_below`'s annihilation is reachable from `Version::min_ticks`, its value flow decides which `Reign` record counts the outer closes (so a skipped annihilation changes the answer), and no committed public-API test drives it deterministically.

Evidence:

         1	//! Direct pins on the web's latent-ladder gates that no packed-stream walk
         2	//! reaches, and on the seam contracts no packed stream drives.
         8	//! raise to the tracked minimum's emission — so `emit_offset`'s post-collapse
         9	//! restore and the undercut's latent annihilation in `drop_below` execute on
        10	//! no input either walk can be handed. The third latent-ladder arm, a

Resolution: add a directed min_ticks witness beside the query differentials (a `Version` built through the oracle's normalizing constructor, asserted against `oracle::Version::min_ticks` and the closed form), and re-state the module doc: the annihilation is reachable from `Version::min_ticks` (a right leaf emitted under a range whose left child parked) and unreachable from the two fill-side walks because every fill-side child opens its own range, so a post-park emission always arms; only `emit_offset`'s post-collapse restore is `MinWeb<()>`-only. Optionally add a decision tap on `decide_undercut_through_latent`'s three exits (the `web_traffic` idiom) so the min_ticks populations' coverage of each arm is a readable floor. Acceptance: a committed test under `Version::min_ticks` whose input parks a word-scale latent and then emits a drop that dominates it with an outer boundary still stacked, asserting the exact closed form, which fails when `residue.sub_accum(&latent)` at watermark.rs:543 is deleted; the module doc no longer claims the arm is unreachable from packed streams.
Construction: leaf heights in stream order `w = 0` (depth 1), `a = 2^34` (depth 3), `b = 2^34 + 2` (depth 4), `c = 2^34 + 1` (depth 4), `z = 1` (depth 2); the tree `R(w, O(X(a, I(b, c)), z))` is normal (no equal sibling leaves). Under `min_ticks`: `w` arms `R` at 0; `a` opens `O`, `X` and arms them 2^34 above (`Word(2^34)`, then a zero run); `b` opens `I` and arms it 2 above; `c` undercuts `I` to 2^34 + 1 (boundary 1, anchor `A = 2^34 + 1`); at `z` the step closes `I` (parks `Λ = 1`) and `X` (zero run), leaving `R`, `O` armed with diffs `[Word(2^34)]`; `z` emits with no pending range: `gap = 1 − A = −2^34`, the latent cannot dominate a two-digit gap, `gap.sign_dominates_at(0)` certifies (2^34 ≥ 3·2^32), `undercuts_here` returns true with the latent live, `undercut` → `drop_below` annihilates (residue 2^34 − 1), which stops at `Word(2^34)` and leaves `O`'s boundary at 1. Expected `min_ticks = (3·2^34 + 4) − (0 + 1 + 2^34 + 2^34 + 1) = 2^34 + 2`; with the annihilation skipped the residue 2^34 meets the outer boundary exactly, settles `R`'s record early, and the answer reads 2^34 + 1.

### skyline-watermark-24: A proptest's closing comment describes a follower and a parked boundary it never builds, and drop_below's tagged-follower arm runs in no test
- Where: crates/before/src/version/skyline/watermark/tests.rs:356-364 (related: tests.rs:108-164, 289; watermark.rs:529-547)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified by trace (the file's only `follower_set` is line 289, in the other proptest; `open(2); emit_here()` leaves `[ZeroRun(1)]`; at the trailing undercut the latent, if it survived the refusal, is collapsed by `decide_undercut_through_latent` at comparable scales (`|gap| = Λ + 1` against `Λ`) before `drop_below`; the close at 364 pops `Close::ZeroRun`, so nothing parks and the three final probes read the re-seated zero `gap` plus 100 whatever the residue's value or sign); `drop_below`'s loop with a live latent (531-539) is reached by no test in the file because the dominated-undercut proptest takes the inlined arm at 975-977 and the direct annihilation pin installs no follower; executed: no
- Seen by: prose ([11]), correctness ([25]), claims ([37]); refutation: confirmed (with the corrected expected read-out below); history: no-rationale-found (9c7b6999's blob had no follower in this test either)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). Both mutants pass all seven tests in watermark/tests.rs. Beyond that file: mutant 1 (the annihilation reordered ahead of the follower fold) survives every fill, watermark, tick, and grow test run, so `drop_below`'s value flow under a live latent is exercised by no test; mutant 2 (the polarity flip at line 537) is killed by six fill tests through the fill walk's follower client.
- Cross-references: gate-legs-6 (a campaign would have found mutant 1).

A test's English must state what it verifies: a reader would believe residue polarity into tagged followers and parked boundaries is pinned by this family when it is not, and the one arm the comment names — `drop_below` with a live latent and a live follower, whose value flow depends on the follower fold (530-540, subtracting the pre-annihilation `A − v`) running before the annihilation (541-545) — is exercised nowhere. "Not wrong, but you couldn't tell if it were" is repairable, and the repair is cheap.

Evidence:

       358	        web.fold_height(Sign::Negative, &wide(&(&height + UBig::from(1u8)))); // h = −1
       359	        web.emit_here(); // v = −1: past m = 0, a true undercut
       360	        // The undercut runs with the outer range still armed, so it propagates
       361	        // to a live follower. Close the dropped range and read the outer
       362	        // minimum back through the boundary that parked: the value survives
       363	        // only if the follower's residue moved at the right polarity.
       364	        web.close();

Resolution: rewrite 360-363 to what runs ("the refusal left the web able to fire a true undercut; its residue passes the outer pair's zero run, the close pops that run, and the outer range reads the undercut's value exactly"), optionally adding `prop_assert!(matches!(web.close(), Close::ZeroRun))`. Then add a directed pin for the missing arm in the `dominated_latent_annihilates_into_the_undercut_residue` scenario: install `follower_set(0, acc)` holding `start` after the first arming; after `emit_offset(&below(50 + E))` take and `materialize` it and expect `start + D − E` (the arms fold `+D` and `+50` into the follower via `push_boundary`, the park tags it, the undercut subtracts the pre-annihilation `E + 50`). Acceptance: the new pin fails when the annihilation block is moved above the follower loop (reads `start + D + 50 − E`) and when line 537's `sub_accum` becomes `add_accum` (reads `start + D + 50 + E + 50`); the rewritten comment names no follower and no parked boundary.
Construction: mutant 1 swaps the order of the `for slot in 0..self.followers.len()` loop and the `if let Some(latent) = self.latent.take()` block in `drop_below`; mutant 2 changes line 537 to `add_accum`. All seven tests in the file pass under both today.

### skyline-watermark-3: The pool-recycle claim is stated for every client but pinned only through min_ticks
- Where: crates/before/src/version/skyline/watermark.rs:82-87 (related: crates/before/tests/meter.rs:9381-9461; fill.rs:313-314, 527-528, 695-822, 980-987, 1091-1123; fill/prescan.rs:335-408, 555-584)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn pool_misses crates/before` finds readers only at tests/meter.rs:9418-9420, inside `#[cfg(feature = "limb-meter")] mod pool_recycle`, which drives `Version::min_ticks` over `Shape::SeamStop`; `MinWeb` is instantiated at fill.rs:303, prescan.rs:125, query/web.rs:231); executed: no
- Seen by: claims; refutation: confirmed; history: no-rationale-found (d0501cd7 placed the row on SeamStop because that family was already being built)
- Owner-gated: no
- Cross-references: skyline-watermark-24 (the fill client's follower path is also unexercised under a live latent).

Instruments before cures: the module doc claims steady-state churn allocates nothing and cites one row; that row runs the min-ticks client only. The fill walk and its pre-scan are the churn-heaviest clients, leasing and retiring buffers at their own call sites that no meter observes, and `pool_traffic.rs:8-13`'s own argument (heap and touch meters are blind to a dead recycle) applies equally to a client-side leak: an accumulator dropped instead of retired in fill.rs would read proportional misses on a tick path while every committed envelope stayed green.

Evidence:

        82	//! - Dying accumulators return to a pool and are re-armed cleared, so
        83	//!   range churn allocates nothing in steady state: misses (leases the
        84	//!   pool could not serve, counted by the [`pool_traffic`](super::pool_traffic)
        85	//!   meter) are bounded by the walk's peak simultaneous demand, never its
        86	//!   churn length — the seam-stop pool row of `tests/meter.rs` pins it,
        87	//!   the heap meter being structurally blind to a dead recycle.

Resolution: add a tick-path pool-miss row beside `pool_recycle` on a committed follower-churn family (`width_circulation_cost`'s or `memo_resolution_cost`'s shapes): reset misses, tick, assert `small >= 1`, `small == large` across the doubling, and `large <= WARMUP` with the warm-up derived from the walk's peak outstanding leases. Then cite both rows at 86, or scope the sentence to the min-ticks client. Acceptance: a `limb-meter` row whose MEASURED line reads equal misses at both scales on a tick family, and which turns red when one `self.web.retire(...)` in fill.rs becomes `drop(...)` under a local, reverted swap.
Construction: under a local reverted swap replace `self.web.retire(relation)` at fill.rs:808 with `drop(relation)`; run the existing suite: every envelope and differential stays green (peak heap falls; touches identical). Add the proposed row and observe misses proportional to `k`.

**Nits in this section** (full records in the evidence files)

| id | where | claim | resolution |
|---|---|---|---|
| skyline-query-17 | `crates/before/src/version/skyline/query/integral.rs:831-838` | The hard assert in `Integrator::jump` ("a monotone orientation's change term is a debit") has no committed demonstration that it fires; every shipped closure is monotone. | A `#[should_panic]` test driving `pair_fold` with an anti-monotone closure on `ConcurrentPair`. |
| skyline-sweep-place-masked-11 | `crates/before/src/version/skyline/overlay.rs:488-514` | `IdLeafCursor::open`'s empty-stream arm is reachable through none of its callers (every mask is a `Party`, never empty) and has no test. | One unit test opening the cursor on the empty view, or drop the guard and state that masks are never empty. |
| skyline-watermark-17 | `crates/before/src/version/skyline/watermark.rs:947-958` | `emit_here`/`emit_offset` accept an emission outside any open range and heal silently at the next arming, where their siblings debug-assert their `armed`/`pending` preconditions. | A `debug_assert!` that `pending > 0` or `armed > 0` holds, at the top of both. |
| skyline-sweep-place-masked-26 | `crates/before/src/version/skyline/place/tests.rs:1-1` | place/tests.rs has no module doc naming its oracle (the composed pair sweeps), and two tests share an indistinguishable first sentence. | Add the module doc; reword line 294. |
| skyline-sweep-place-masked-28 | `crates/before/src/version/skyline/place/tests.rs:218-224` | `dominance` is the one placement entry point without an organic-witness test; the module doc presents `precedence` as its mirror. | Add `dominance_walk_verdicts_organic_witnesses` mirroring the precedence test. |
| skyline-watermark-22 | `crates/before/src/version/skyline/watermark/tests.rs:46-53` | The `wide()` helper doc says a word-range wide spelling probes certification; `to_word` dispatches it to the word path, so it probes nothing there. | Restate: within the word range the spelling is a no-op; past it `add_wide` spills. |

## The codec

**Bits, buf, build, cursor, dsi, gamma, stack**

### codec-bits-15: PackedBuilder, the kernel every emitter writes through, has no direct model test; neither does BitsView::load_be
- Where: crates/before/src/codec/build.rs:208-228 (related: crates/before/src/codec/build.rs:144-167, 245-266, 270-282, 286-312; crates/before/src/codec/bits.rs:321-343; crates/before/src/codec/tests.rs:327-395; .cargo/mutants.toml; tools/covcheck-expected.json:2)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rln PackedBuilder` over src, tests, fuzz, wasm32-pins, examples names no test file; `grep -rn load_be` finds the definition and code.rs:51 only; `grep -i codec .cargo/mutants.toml tools/mutantcheck-expected.json` is empty; covcheck's scope is `crates/before/src/version/skyline/`); executed: no
- Seen by: correctness [26], claims [34]; refutation: confirmed; history: no-rationale-found (525e7324 pinned the builder transitively by design; 83e61b4d added the build-history model family for `BitsBuf` only)
- Owner-gated: no
- Witness (witness/results.md): demonstrated in a narrowed form (run). Two of the three builder sites are killed heavily by downstream suites (truncate: 34 tests; append_bytes: 36 tests), always with panics appearing in overlay.rs, base.rs, bridge.rs, or the borsh tests rather than at the builder. The `read_bits` committed-byte chunk arm (line 297) is different: a non-equivalent operator swap (`>>` to `<<`; byte 0xB0, pos 0, n 2 reads 2 originally and 0 under the mutant) survives all 247 committed tests in the codec, party, skyline, version, and borsh selection plus coincident_span and fuzz_seeds.
- Cross-references: gate-legs-6 (the surviving mutant).

The builder's move set carries the partition's most intricate bit arithmetic (`truncate` re-deriving `staged` from a committed byte at 216-223; `read_bits` chunking across the committed/staged boundary at 292-310; the `append_bytes` carry at 275-282; `patch_bit` in two regimes at 147-166; the 70-bit merge at 259-265), and `load_be`'s shift merge (bits.rs:337-341) feeds every `Code::from_range`. Both are pinned only through the party-ops and skyline differentials, whose failure would point at the wrong layer, and nothing committed says whether those suites kill a mutant here. The sibling `BitsBuf` has exactly the family this lacks (`build_history_spelling_is_a_function_of_content`, `build_history_spellings_are_injective`). Doctrine: property tests where the claim is a family; "not wrong, but you couldn't tell if it were" is repairable.

Evidence:

       214	        let whole = len / 8;
       215	        let rem = (len % 8) as u32;
       216	        if whole < self.bytes.len() as u64 {
       217	            self.staged = if rem > 0 {
       218	                u64::from(self.bytes[whole as usize] >> (8 - rem))
       219	            } else {
       220	                0
       221	            };
       222	            self.staged_len = rem;
       223	            self.bytes.truncate(whole as usize);
       224	        } else {
       225	            self.staged >>= self.staged_len - rem;
       226	            self.staged_len = rem;
       227	        }

Resolution: Add `codec/build/tests.rs` mirroring the `BitsBuf` family: an arbitrary interleaving of `push_bit`, `push_code` (Small at every `len` in 1..=63, and Wide), `reserve` then `patch_bit` at committed and staged positions, `splice` from a view at every source alignment into every output alignment, `truncate` to byte-aligned, mid-byte, and empty targets, and `extract_code` at ranges below and above 63 bits spanning the committed/staged boundary; assert `len()` and `finish()` equal a clean `BitsBuf` rebuild at every step, and `extract_code`'s bits equal the model's slice. Add a `load_be` differential: for random buffers and every `(start, len <= 64)` within the live length, `load_be(start, len)` equals the fold of `bit(start + i)`. Acceptance: both suites are committed and red under each hand mutation: build.rs:218 `>> (8 - rem)` to `>> rem`; 297 `(8 - within - take)` to `(8 - within)`; 278 `carry << (8 - r)` to `carry << r`; bits.rs:340's `| (u64::from(buf[8]) >> (8 - shift))` dropped.
Construction: apply the mutation at build.rs:218 and run `just test-all`; if the party-ops and skyline suites stay green the gap is demonstrated (record which test catches it otherwise); repeat at 297 and 278.

### codec-bits-30: The BitStack model test names a method that does not exist, omits set_last and trailing_ones, and reaches the spill it advertises about once in a hundred runs
- Where: crates/before/src/codec/stack/tests.rs:21-27 (related: crates/before/src/codec/stack.rs:47-56, 109-123, 141-151, 158-165; crates/before/src/version/skyline/overlay.rs:352-354; crates/before/src/version/skyline/fill.rs:1264; crates/before/src/version/skyline/fill/prescan.rs:676; crates/before/src/version/skyline/fill/fuse.rs:434; crates/before/src/meter/tier2/tests.rs:625-635)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (stack.rs:28-185 defines `new`, `len`, `push`, `push_bits`, `pop_bits`, `trailing_ones`, `trailing_ones_capped`, `pop`, `set_last`, `last`, `all_set`, no `is_empty`; the body checks `pop`, `last`, `len`, `all_set` only; `grep -rn 'set_last\|trailing_ones'` outside stack.rs finds production sites only, no test); executed: yes: `spill.py` (an exact dynamic program over the generator's distribution: lengths uniform in 1..=299, push and pop each 1/2, pop on empty a no-op) gives P(height reaches 64 within a case) = 4.0135e-05 and P(reaches 65, the spill) = 3.0997e-05, so P(any of 256 cases spills) = 0.79%; the roster claims are grep-verified, not run
- Seen by: prose [14], correctness [27], claims [34]; refutation: confirmed (independently reproduced 4.0135e-05); history: no-rationale-found (56a3dfe2 landed the doc and test together; `is_empty` never existed on `BitStack`; `set_last` and `trailing_ones` were never added to the model)
- Owner-gated: no

The testdoc promises `is_empty` (nonexistent) and coverage "across the word-spill boundary at 64 bits"; with a symmetric 50/50 walk of at most 299 steps the spill is reached in about one run in a hundred, so the single-bit spill/refill arms (stack.rs:49-53, 143-146) and `last`/`set_last` on an empty top register go unexercised by the one test that drives `BitStack` directly. `set_last` (fill.rs:1264, prescan.rs:676, fuse.rs:434, whose `top_len == 0` arm rewrites `words.last_mut()`) and `trailing_ones` (`peek_flip`, whose multi-word loop at stack.rs:114-121 fires only for a right-branch run longer than 64) are not modeled at all, and I found no oracle-checked test reaching that loop: the deep-shape join/meet differential caps at scale 48 (tier2/tests.rs:628-629), the depth-100k proof is a left spine, and the `RightSpine` generator feeds only party and fill tests. A test's doc must be accurate; a model that omits two of the type's methods leaves the branches only deep right spines reach unpinned, and `peek_flip` decides which plateaus the ownership-gated walks skip, so a wrong run count is a wrong answer.

Evidence:

        21	    /// The word-backed bit stack agrees with a plain `Vec<bool>` on any
        22	    /// interleaving of pushes and pops — `last`, `len`, `is_empty`, and
        23	    /// `all_set` included — across the word-spill boundary at 64 bits.
        24	    #[test]
        25	    fn bit_stack_matches_a_vec_of_bools(
        26	        ops in proptest::collection::vec((any::<bool>(), any::<bool>()), 1..300),
        27	    ) {

Resolution: Drop `is_empty` from the doc. Make the spill reachable by construction (bias pushes, e.g. `prop::bool::weighted(0.75)`, or prefix each case with a deterministic ramp of at least 65 pushes) and pin the reach with `prop_assert!(max_height >= 65)` per case. Add `set_last` as a third op kind (model: overwrite `model.last_mut()`) and assert `stack.trailing_ones() == model.iter().rev().take_while(|b| **b).count() as u64` at every step, with runs long enough to cross two spilled words. Acceptance: the extended test is red under each of: stack.rs:118 `if w < 64` to `if w <= 64` (caps the run at one spilled word); 117 `run += u64::from(w)` to `run = u64::from(w)`; 162-163's `words.last_mut()` arm replaced with a no-op; and the testdoc names only methods the body checks.
Construction: for the multi-word loop specifically, `let mut s = BitStack::new(); for _ in 0..70 { s.push(true); } assert_eq!(s.trailing_ones(), 70); s.push(false); for _ in 0..3 { s.push(true); } assert_eq!(s.trailing_ones(), 3);` is exercised by no committed test; the `w <= 64` mutation above passes every current suite unless some walk builds a right run deeper than 64 and checks its value.

### codec-bits-19: The claimed k = 65 gamma witness is a second k = 64 row
- Where: crates/before/src/codec/dsi/tests.rs:30-33 (related: crates/before/src/codec/dsi/tests.rs:15-17; crates/before/src/codec/dsi.rs:23-25, 268-281; crates/before/src/codec/gamma.rs:25-26)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (arithmetic against the coding at gamma.rs:25-26, `k = floor(log2(n + 1))`: `n = u64::MAX` gives `m = 2^64`, `k = 64`; `n = 2^64` gives `m = 2^64 + 1`, `k = 64`); executed: no
- Seen by: prose [17]; refutation: confirmed; history: no-rationale-found (3e5b95df23 intended "word-seam witnesses at k = 63/64/65 and ~100"; the value chosen never delivered k = 65)
- Owner-gated: no

`wide(64)` is `2^64`, whose mantissa has the same 64-zero prefix as the `u64::MAX` row; the `// k = 65` comment, the testdoc's "the next (`k = 65`)", and dsi.rs's module doc "(`k = 63, 64, 65, ~100`)" all name a row the value list lacks. The row that is missing is a boundary: `k = 65` is the smallest wide code whose mantissa needs a second, one-bit chunk in `DsiCursor::read_int`'s loop (dsi.rs:268-281), where `k = 64` takes one 64-bit chunk. A test's doc must be accurate to its body; a production module doc that transcribes test parameters inherits the error.

Evidence:

        30	        Base::from(u64::MAX - 1), // k = 63: the machine arm's ceiling
        31	        Base::from(u64::MAX),     // k = 64: the first wide-arm code
        32	        wide(64),                 // k = 65
        33	        wide(100),                // far wide

Resolution: Use `wide(65)` (`m = 2^65 + 1`, `k = 65`) and fix the comment and the testdoc at 15-17; drop the parameter list from dsi.rs:23-25 ("at and across the word seam", letting the test carry the values). Acceptance: for every row comment `k = N`, `(value + 1).bits() - 1 == N`; dsi.rs's module doc names no specific `k`.
Construction: `(UBig::ONE << 64) + 1u32` has bit length 65, so `k = 64`; `(UBig::ONE << 65) + 1u32` has bit length 66, so `k = 65`.

**Base, display, text, tree**

### codec-base-text-tree-6: `base_dispatch_read_touches_no_digits` asserts a counter `to_word` cannot reach, so any implementation passes it
- Where: crates/before/src/codec/base/tests.rs:53-85 (related: crates/before/src/codec/base.rs:266-277, crates/before/src/codec/base/tests.rs:10-12, crates/suanpan/src/accumulator.rs:20-26, crates/suanpan/src/magnitude.rs:28-34)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`grep -rn 'fn touch\b\|touch(\|touch_meter::record' crates/suanpan/src` outside `accumulator.rs` matches only a test name; `touch` is defined at accumulator.rs:21 and called 24 times in that file; `Base::to_word` is `self.to_u64()` = `u64::try_from(&self.0).ok()`, which resolves to dashu-int 0.5.0 `convert.rs:711-712` `try_to_unsigned` and enters no suanpan code); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (38c4eee8's message asserts the intent the code cannot deliver)
- Owner-gated: yes: deletes an instrument

The test's doc says it holds `Base::to_word` to O(1) "in the touch denomination", and `base.rs:270-273` repeats the claim. `suanpan::touch_meter::record` is reached only through `touch()` in `accumulator.rs`, and `to_word` never enters the accumulator, so `touches() == 0` after two `to_word` calls holds for every possible `to_word` body, including one that walks every limb. The liveness leg proves the counter counts accumulator adds, not that it sees `to_word`. Doctrine: every criterion needs a committed demonstration that a known-bad mechanism fails it; a criterion the bad mechanism also passes is decoration, and a test doc stating an invariant the test does not check is a bug in the test.

Evidence:

        57	/// The `Magnitude` rustdoc makes O(1) dispatch a contract on
        58	/// implementors; this pin holds `Base`'s implementation to it in the
        59	/// touch denomination. The touch counter is process-global, so the

        70	    touch_meter::reset();
        71	    assert_eq!(Magnitude::to_word(&word_held), Some(7));
        72	    assert_eq!(Magnitude::to_word(&spilled), None);
        73	    assert_eq!(
        74	        touch_meter::touches(),
        75	        0,
        76	        "the dispatch read is word-scale: no digit touched on either arm"
        77	    );

    (suanpan/src/accumulator.rs)
        20	#[inline(always)]
        21	fn touch(count: u64) {
        22	    #[cfg(feature = "touch-meter")]
        23	    crate::touch_meter::record(count);

    (base.rs)
       275	    fn to_word(&self) -> Option<u64> {
       276	        self.to_u64()
       277	    }

Resolution: Delete the test and the "dispatch pins … zero digit touches" clause at base.rs:270-273 and the module doc's "under `limb-meter`, touches no digits" at base/tests.rs:10-12. Keep `base_dispatch_answers_at_word_scale` as the semantic pin, and state at the `impl suanpan::Magnitude for Base` comment that `to_word`'s O(1) rests on the backend's `TryFrom<&UBig> for u64` being a representation check. Do not invent a counter for `to_word`: the limb meter is deliberately silent on `to_u64` (base.rs:37-44), and no observable in this crate distinguishes a constant-time read from a limb walk. Acceptance: no test in `base/tests.rs` claims a counter observes `to_word`; the impl comment no longer names a dispatch pin in touches.

Construction: Replace `to_word`'s body with `if suanpan::Limbs::new(&self.0).count() > 1 { return None; } self.to_u64()` (deliberately O(limbs)) and run `cargo nextest run -p before --features limb-meter base_dispatch_read_touches_no_digits`: it passes, because no touch is recorded outside accumulator digit writes.

### codec-base-text-tree-23: Three test docs describe mechanisms the code does not have: a recursive validator, a test-only entry as the decode path, and an inline spill at `u64`
- Where: crates/before/src/codec/tests.rs:884-890 (related: crates/before/src/codec/tests.rs:905-906, crates/before/src/codec/tests.rs:1626-1631, crates/before/src/codec/tests.rs:85-96, crates/before/src/codec/tree.rs:21-27, crates/before/src/codec/tree.rs:43-61, crates/before/src/codec/base.rs:20-24, crates/before/src/party.rs:630)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (tree.rs:61 `let mut stack: Vec<IdFrame> = Vec::new();` with no self-call; `grep -rn parse_id_from src` shows it `cfg(all(test, feature = "borsh"))` with one caller in borsh_impls/tests.rs; `Party::decode` calls `codec::parse_id` at party.rs:630; dashu-int 0.5.0 `src/repr.rs:69-72` holds `Small(DoubleWord)` inline, and base.rs:22-24 says so; `git log -1` on 660df29c and 8d7c112f: both 2026-06-02); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: deliberate but expired for all three (the recursion prose was written for the recursion design and orphaned by the same-day revert, which did not touch codec/tests.rs; `parse_id_from` was the grammar body at d8cd87d7 and was moved and test-gated at 78c65371/5d167a63; the `u64` spill was exact for the pre-dashu `enum Base { Small(u64), Big(BigUint) }`)
- Owner-gated: no

Every test's doc comment states its invariant and must be accurate. `reject_deep_nested_denormal_id` says the validator "runs bottom-up by recursion" and exercises "the validator's recursion", a mechanism the crate's hard rule forbids and the same test's body comment contradicts ("stack-based validator"). `parse_stacks_handle_deep_spines` attributes the id frames to `parse_id_from`, which `Party::decode` never calls. `gamma_roundtrip_just_above_u64_max` says the inline representation spills at the `u64` boundary, but the wrapped `UBig` holds two words inline; the value crosses the `to_u64` word-dispatch boundary, which is what the test exercises.

Evidence:

       884	/// The id validator runs bottom-up by recursion, so a collapsible `(v, v)` node
       885	/// buried under deep, otherwise-canonical nesting must still be caught.
       886	///
       887	/// The `NotCanonical` check fires when *any* node completes, not only at the
       888	/// root. Build a left-leaning spine `(((… (1,1) …, 0), 0), 0)` whose deepest
       889	/// node is the denormal `(1, 1)`, exercising the validator's recursion past a
       890	/// single byte.

       905	    // The encoding spans several bytes, so this drives the stack-based
       906	    // validator well past the trivial single-node case.

      1626	    // Id tree: a left spine, one frame per level in `parse_id_from`.

        85	/// The small inline `Base` representation must spill exactly at the `u64`
        86	/// boundary without changing the arbitrary-width integer codec.

    (tree.rs)
        61	    let mut stack: Vec<IdFrame> = Vec::new();

Resolution: Reword 884-890 in terms of what is: the validator completes each node's collapsible check on its explicit frame stack, so `(1, 1)` buried under deep nesting is caught at that node's close, not only at the root, and the left spine keeps many ancestors open so the frame stack, not a single tag read, carries the check. At 1626 name `parse_id_core` (the body every id decode entry drives, reached here through `parse_id`). At 85-86 name the boundary crossed: "`u64::MAX + 1` is the first value `to_u64` cannot answer; the integer code and its rendering are unchanged across that word-dispatch boundary". Acceptance: `grep -n recurs src/codec/tests.rs` returns only the deliberately recursive reference parser's passages (1636-1642, 1687); `grep -n parse_id_from src/codec/tests.rs` is empty; the gamma testdoc names the `to_u64` boundary, not inline storage.

**Nits in this section** (full records in the evidence files)

| id | where | claim | resolution |
|---|---|---|---|
| codec-base-text-tree-27 | `crates/before/src/codec/tests.rs:1656-1683` | The reference id parser's `RefCur`/`RefIdKind` duplicate `text::Cur`/`IdKind` byte for byte without saying why (the tokenizer freeze is unstated and only a space exercises whitespace). | State the freeze at the header and add a second whitespace byte, or reuse `Cur`. |

## Cross-cutting: fold, shape, recurse, serde and borsh

**recurse.rs and the segment counter**

### crate-root-32: The stack-segment counter has no writer in any binary that judges it
- Where: crates/before/src/recurse.rs:16-20 (related: crates/before/src/recurse.rs:37, crates/before/src/recurse.rs:68-86, crates/before/src/recurse.rs:100-109, crates/before/src/recurse.rs:118-129, crates/before/Cargo.toml:33-44, crates/before/src/meter.rs:3552-3558, crates/before/src/meter/board/measure.rs:84-92, crates/before/src/meter/board/ceilings.rs:80-83, crates/before/src/meter/board/judge.rs:60-64, crates/before/src/meter/board/currency.rs:138-140, crates/before/src/meter/board/floors.rs:627-636, crates/before/src/meter/tests.rs:392-432, crates/before/tests/meter.rs:22-26, crates/before/tests/meter.rs:364-391, crates/before/tests/meter.rs:263, crates/before/tests/meter.rs:441, crates/before/tests/amp_board_smoke.rs:96, justfile:903-904)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (the cfg attributes read: the static at 68 and its readers at 77 and 83 compile under `any(test, feature = "meter")`; `grow` at 100 and `descend!` at 118 under `cfg(test)`; the sole `fetch_add` is line 106 inside `grow`; `stacker` appears only under `[dev-dependencies]` at Cargo.toml:44; `grep -rn 'descend!\|recurse::grow'` outside recurse.rs hits only testing/bridge.rs, skyline/grow/tests.rs, and meter/tests.rs; the board reads the counter at measure.rs:84 and :92 and tests/meter.rs at 364 and 371; `just amp-board-acceptance` (justfile:903-904) runs the example binary with `--features limb-meter,scan-meter`; tests/amp_board_smoke.rs:96 calls `board::run` from an integration-test binary; a python parse of tests/meter.rs found 46 `envelope(` rows, every one with `0` in the segments column); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: deliberate-but-expired (05bd2b16d made the last library walk iterative, put `grow`/`descend!` under `cfg(test)`, and wrote lines 16-20 in the same diff, reasoning about the lib unit-test binary only; 1ddb5a483 moved `stacker` to dev-dependencies and recorded a keep for the guard; neither commit, the amplification note, nor meter/tests.rs:399-403 addresses that tests/meter.rs and examples/amp_board.rs link a library without `cfg(test)`)
- Owner-gated: yes (removal of an instrument, and a board policy)
- Witness (witness/results.md): demonstrated (run). In an integration-test binary built with `--all-features`, a heap stack segment grown through `stacker::grow` under a 200 000-frame recursion leaves `meter::stack_segments()` at 0; grep confirms every `descend!` site is a test surface. The delete-the-liveness-test-and-run-the-board construction was not run.
- Cross-references: board-ops-render-15, envelopes-a-2, module-graph-1, recursion-1, inventory-2 (the segments cluster).

Integration tests and examples compile the library without `cfg(test)`, so in `tests/meter.rs` and in the `amp_board` example the counter is a constant zero: every `segments <= env.segments` assertion and the board's `MAX_GROWN_STACK_SEGMENTS = 1` ceiling judge a signal that cannot move, and the liveness dive (`stack_segment_meter_counts_deterministically_and_resets`) runs in a different binary. Independently of which binary, no library kernel can reach `descend!` in any build, so the zero "over the library kernels" is a compile-time fact, not a measurement; a kernel that regressed into stack recursion would crash `clock::tests::deep_tree_stack_safety`, never bump this counter. The prose at 74-76 ("the counter is always written") is false in exactly the builds that read it; `judge.rs:60-64` carries a floor-trip message "so a future segments floor binds without a code change", the empty buffer the doctrine forbids, and no floor can ever bind on a counter nothing writes. Two smaller ghosts ride along: line 37's "`depth % STRIDE`" describes code that reads `is_multiple_of(STRIDE)` (92), and tests/meter.rs:441 ("heap stays flat, segments do not") beside `CMP_DENSE = envelope(30_720, 0, 0, 0)` at :263 is a remnant of the recursive-comparison era.

Evidence:

    16  //! The paper-shaped oracle is clearest written recursively, and the guard is
    17  //! what lets it meet deep inputs safely. The segment counter below stays
    18  //! compiled for the meters: it is the deterministic stand-in for
    19  //! recursion-driven stack consumption, and its zero reading over the library
    20  //! kernels is the measured fact the boards' segments column pins.
    ...
    68  #[cfg(any(test, feature = "meter"))]
    69  static SEGMENTS_GROWN: AtomicU64 = AtomicU64::new(0);
    ...
    74  /// Compiled only for the meter surface: the counter is always written (the bump
    75  /// is inseparable from the growth arm), but nothing outside the meters ever
    76  /// reads it.
    ...
   100  #[cfg(test)]
   101  #[inline]
   102  pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
   103      if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
   104          f()
   105      } else {
   106          SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);
   107          stacker::grow(STACK_GROWTH, f)
   108      }
   109  }
    ...
   118  #[cfg(test)]
   119  macro_rules! descend {

    Cargo.toml:
    33  [dev-dependencies]
    ...
    44  stacker = { workspace = true }

    tests/meter.rs:
   364      meter::reset_stack_segments();
    ...
   371      let segments = meter::stack_segments();
    ...
   387      assert!(
   388          segments <= env.segments,

    meter/board/judge.rs:
    60  /// The segments column's floor-trip message (unreachable while segments is
    61  /// ceiling-only by policy; the judgment loop still carries it so a future
    62  /// segments floor binds without a code change).

Resolution: The owner's call between two sound options. (a) Dissolve, my recommendation: remove the segments currency from the board (`Currency::Segments`, `seg_ceiling_only()` on every cell, `MAX_GROWN_STACK_SEGMENTS`, `SEG_FLOOR_TRIP`, the render column) and `Envelope.segments` with every `segments:` pin in tests/meter.rs, and `meter::{stack_segments, reset_stack_segments}`; confine `SEGMENTS_GROWN` and its readers to `cfg(test)` beside their one live client, the determinism dive; name `clock::tests::deep_tree_stack_safety` (depth 100k) plus the structural fact that `descend!` is `cfg(test)` and `stacker` a dev-dependency as the instruments against reintroduced depth recursion. (b) Keep and make it live: `stacker` becomes an optional dependency enabled by `meter`, `grow`/`descend!` compile under `any(test, feature = "meter")`, and a guarded 200k-deep descent in tests/meter.rs and in the board's self-check reads `stack_segments() > 0` in each enforcing binary before the kernels' zero is asserted. Either way, restate recurse.rs:16-20 and 74-76 as what IS (the guard and its counter exist for the test-only oracle bridge and its witnesses; in non-test meter builds the counter has no writer), fix line 37, and excise tests/meter.rs:441's clause. Acceptance: (a) `grep -rn 'stack_segments\|SEGMENTS_GROWN\|Currency::Segments' crates/before` finds only the `cfg(test)` determinism witness, and `just gate` is green; (b) a test in crates/before/tests/ built with `--features meter` asserts `before::meter::stack_segments() > 0` after a guarded deep descent. In both, no prose calls the segments zero a measured fact.
Construction: Read-only: in a build of the library with `--features meter,limb-meter,scan-meter` and without `cfg(test)` (what tests/meter.rs and examples/amp_board.rs link), no expression writes `SEGMENTS_GROWN`; the sole `fetch_add` is inside `#[cfg(test)] fn grow`. Runtime: add a temporary scenario to tests/meter.rs whose body recurses 10^6 frames through `stacker::grow` directly; `meter::stack_segments()` still reads 0 and every `segments: 0` envelope passes. Conversely, delete the body of `stack_segment_meter_counts_deterministically_and_resets` and run the meter suite and `just amp-board-acceptance`: every segments ceiling stays green, because nothing in those binaries could have moved the counter before the change either.

**serde and borsh test suites**

### crate-root-11: Five differential proptests share one body
- Where: crates/before/src/borsh_impls/tests.rs:434-451 (related: crates/before/src/borsh_impls/tests.rs:461-480, crates/before/src/borsh_impls/tests.rs:549-574, crates/before/src/borsh_impls/tests.rs:589-608, crates/before/src/borsh_impls/tests.rs:693-728)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (the five bodies read; each opens two `&[u8]` readers, asserts equal remaining length, and matches `(Ok, Ok)`/`(Err, Err)`/diverged; only the span test adds re-encode checks in its `Ok` arm at 713-723; both sides are already `io::Result<T>` via `decode_error`); executed: no
- Seen by: structure; refutation: confirmed; history: no-rationale-found (accreted per type)
- Owner-gated: no

The differential discipline (consumption asserted unconditionally, error genre compared by discriminant) is the valuable part and should have one spelling, so a tightening reaches all five legs at once, the same argument fold.rs:1-3 makes for the fold.

Evidence:

   441          prop_assert_eq!(
   442              subject_reader.len(),
   443              oracle_reader.len(),
   444              "byte consumption diverged",
   445          );
   446          match (subject, oracle) {
   447              (Ok(s), Ok(o)) => prop_assert_eq!(s, o),
   448              (Err(s), Err(o)) => assert_same_error(&s, &o)?,
   449              (s, o) => prop_assert!(false, "accept/reject diverged: {:?} vs {:?}", s, o),
   450          }

Resolution: A helper `fn assert_wire_matches_reference<T: PartialEq + Debug>(stream: &[u8], subject: impl FnOnce(&mut &[u8]) -> io::Result<T>, oracle: impl FnOnce(&mut &[u8]) -> Result<T, Decode>) -> Result<Option<(T, usize)>, TestCaseError>` returning the accepted value and consumed length, so the span test can add its re-encode checks; the five bodies become one call each. Acceptance: five one-line proptest bodies; the helper's doc states the three agreements it asserts.

**Nits in this section** (full records in the evidence files)

| id | where | claim | resolution |
|---|---|---|---|
| crate-root-36 | `crates/before/src/serde_impls/tests.rs:26-30` | serde testdocs: "Both deserialization paths" lists three formats, "every rejection genre" covers three, "The new impls" is dated. | Reword to what each body drives; drop "new". |

## suanpan

**The accumulator, limbs, magnitude, touch meter, claims roster**

### suanpan-2: The "on every input sequence" quantifier has no instrument over arbitrary sequences
- Where: crates/suanpan/src/lib.rs:25-29 (related: crates/suanpan/src/accumulator/tests/metered.rs:1-11; crates/suanpan/src/accumulator/tests/differential.rs:340-359; crates/suanpan/tests/amortized_sequences.rs:63-78; crates/before/tests/meter.rs:6622-6627)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: verified (grep: `touch_meter` is referenced by no file under accumulator/tests except metered.rs; differential.rs:340-359 asserts sign and value only); executed: no
- Seen by: claims; refutation: confirmed at medium, with the caveat that the known-bad mechanism the property would catch is asserted, not constructed; history: deliberate-and-holds (the derived-bounds-plus-exact-pins model is stated at metered.rs:1-11 and lib.rs:25-29, 312-323; no randomized touch bound was ever proposed or rejected)
- Owner-gated: yes (a documented instrument model)
- Witness (witness/results.md): demonstrated (reading plus a scratch instrument). Among suanpan's test modules only accumulator/tests/metered.rs references the touch meter, so the differential and ledger proptests never read it. The instrument asked for is constructible: a scratch proptest ran 512 arbitrary mixed sequences serially, reading a maximum of 44.0 touches per unit of work under a placeholder bound of 64 x work + 4096 (a work model that does not charge `shl`/`negate` their documented O(held digits)); no known-bad mutation was run against it.

The crate quantifies every amortized bound over all input sequences. The committed touch instruments are exact totals at canonical schedules (metered.rs), two-scale flatness at four fixed shapes in before's meter suite, and one 2x2 mixed-second-difference grid; the proptests that drive arbitrary mixed streams (`mixed_streams_match_the_bigint_oracle`, the run-forming stream, the ledger stream) never read the meter. A regression that kept the pinned shapes flat but broke amortization on some other interleaving would pass every committed check. The doctrine prefers a property over point pins when the claim is a family, and the potential argument (lib.rs:150-161) yields a concrete per-operation constant, so a sequence-level bound is derivable rather than measured.

Evidence:

        25	//! Every cost this page quotes holds on adversarial input sequences — the
        26	//! amortized bounds are worst-case over the whole sequence, not average-case
        27	//! claims — and every one is *derived*: the three arguments that carry them
        28	//! (the lazy zone, the collapsing sign fold, the zero-run ledger) are below, in
        29	//! full.

Resolution: a touch-meter proptest (metered.rs or tests/amortized_sequences.rs) that drives the existing `arb_op()` streams with interleaved sign reads, records `touches` beside a work denominator (word-scale calls + limbs yielded + sign reads + one spill), and asserts `touches <= K * work + D` with K derived in the test's doc comment from the potential argument (each `add_at` iteration deposits one credit; fold reads and settle steps each spend one; carry steps are bounded by the zone refill). Commit a known-bad demonstration beside it (for example `settle_top` with the `consume_run_at` arm disabled) and show the property reads red on run-forming streams. Acceptance: the property runs in the gate under `--all-features`; the known-bad demonstration fails it while the fixed-schedule pins are shown not to notice. Construction: `proptest! { fn touches_are_linear_in_work(ops in vec(arb_op(), 1..300), engine_first: bool) { touch_meter::reset(); let mut acc = fresh(engine_first); let mut work = 0u64; for op in &ops { apply(&mut acc, &mut oracle, op); work += op.limbs_or_one(); acc.sign(); work += 1; } prop_assert!(touch_meter::touches() <= K * work + D); } }` with K and D derived in the doc comment; the metered suite must run serially (touch_meter.rs:16-20).

### suanpan-40: Two suanpan mutant exclusions claim equivalence, but both mutants change exact touch counts, which the crate declares a public contract
- Where: .cargo/mutants.toml:85-95 (related: 5-24, 33-35; crates/suanpan/src/accumulator.rs:601-609, 1108-1122; crates/suanpan/src/lib.rs:283-286; tools/mutantcheck-expected.json:4-11; crates/suanpan/src/accumulator/tests/metered.rs:405-441)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (trace: with `<<=`, the drain loop over `u128` `high` in 1..=3 runs four times (`high * 2^32`, `* 2^64`, `* 2^96`, then the bits shift out to 0), adding 3 touches and 3 trailing zero digits that `sign_limbs` pops and `from_le_bytes` normalizes away; with `&&`, `shl(0)` on a digit-engine value costs a full take-and-redeposit (about 2d touches) instead of 0, and `shl(k)` on a literal-zero register costs 1 (`k <= 30`) or retires the register (`k > 30`) instead of 0. No committed pin reads a spelling with a nonzero final carry: the held-width pin reads `2^k - 1`, whose digits are all `2^32 - 1` after `add_wide`, so carry stays 0 (metered.rs:409-441); grep finds no `.shl(0)` under crates/suanpan); executed: no
- Seen by: correctness; refutation: confirmed, and added the dissolving refactor and the mutantcheck-expected.json coupling; history: deliberate-and-holds as recorded rulings (the exactness contract landed in 1255a4e2 on 2026-08-07, before the roster; the history pass reports the `shl` entry landed in d35c9019 and the `read_digits` entry was re-pointed by 8da57920), so this disputes the header's criteria as applied to a crate with no slack band, not a stale ruling
- Owner-gated: yes (recorded exclusion rulings)
- Witness (witness/results.md): demonstrated (run). Production reads 3 touches for `sign_magnitude` after the extreme park and 0 for `shl(0)` on a 2048-bit digit-engine value; with the two excluded mutants applied the same calls read 6 and 66 while every value assertion still holds and suanpan's entire committed suite (62 tests) stays green under both. The entry's estimate of about 130 touches for the `shl` mutant was high by 2x (66 measured: 64 digits plus two).
- Cross-references: gate-legs-6 (no campaign re-tests the exclusion premises).

The file's own policy admits an exclusion only for an empty discriminating class or for work moved inside a meter's deliberate headroom (5-13), and suanpan's pins have no headroom: exact counts are the contract (lib.rs:283-286). "Value-equivalent" is not "observationally equivalent" under that contract, and the `shl` rationale's claim that no band can price a zero shift without pinning the representation is false for a touch pin of 0. The header's ladder (16-19) prescribes refactoring before excluding, and the `>>=` site admits exactly that.

Evidence:

        85	    # Proven equivalent: read_digits' final carry has magnitude at
        86	    # most 3 (bound derived in the comment at the site), so each high-part
        87	    # drain emits its whole value in its first digit and the mutated shift
        88	    # direction has an empty discriminating class.
        89	    "accumulator\\.rs.*: replace >>= with <<= in Accumulator::read_digits",
        90	
        91	    # Performance genre, owner-declared benign fast path: the identity
        92	    # guard routes cost and representation only (the comment at the site
        93	    # carries the argument), and no honest meter band can price a zero
        94	    # shift without pinning the representation.
        95	    "accumulator\\.rs.*: replace \\|\\| with && in Accumulator::shl",

    accumulator.rs:
      1109	            while high > 0 {
      1110	                touch(1);
      1111	                collected.push((high & u128::from(DIGIT_MASK)) as u32);
      1112	                high >>= DIGIT_BITS;
      1113	            }

Resolution: (1) `read_digits`: the site's own derivation (1076-1079) bounds `|carry| <= 3`, so `high` fits one digit; replace both `while high > 0 { ...; high >>= DIGIT_BITS; }` loops (1109-1113, 1118-1122) with `if high > 0 { touch(1); collected.push(u32::try_from(high).expect("the final carry fits one digit: |carry| <= 3")); }`, which moves no touch, removes both `>>=` codepoints, and dissolves the exclusion per ladder step (1). (2) `shl`: delete the exclusion and add exact pins to metered.rs: `shl(0)` on a digit-engine value and `shl(40)` on a literal-zero register both cost 0 touches, with `quick.is_some()` asserted after the latter; also pin a readout with a nonzero final carry (a single digit `-(2^33 - 1)` reads out as low 1, carry -2, one drain digit: 3 touches) so the negative read-out count is covered. (3) Update tools/mutantcheck-expected.json:4-11 (listed 2 / suppressed 2 and 1 / 1) in the same commit, or the mutantcheck leg fails. If the owner keeps either exclusion, restate its rationale to name the touch leg it waives, as the header demands. Acceptance: `cargo mutants --workspace` with both entries removed reports the `&&` mutant caught and the `>>=` mutant no longer listed. Construction: apply the `<<=` mutant by hand and run a metered scenario with a nonzero final carry (`park_extreme_negative_digit(&mut a, 0)` from accumulator/tests.rs:110, then `touch_meter::reset(); a.sign_magnitude()`): production counts 3 (digit, complement pass, one drain), the mutant 6. Apply the `&&` mutant: `a.add_wide(&(UBig::ONE << 2048usize)); touch_meter::reset(); a.shl(0);`: production 0 touches, mutant about 130.

### suanpan-8: The documented trait surface (`Send + Sync`, deliberately not `PartialEq`) has no compile-time pin
- Where: crates/suanpan/src/lib.rs:299-309 (related: crates/suanpan/Cargo.toml:16-20; Cargo.toml:72, 128; crates/before/src/party.rs:74)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep: `static_assertions` is a workspace dependency used in before; suanpan's dev-dependencies are proptest and surface-scan only; no `assert_impl`/`assert_not_impl` under crates/suanpan); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (the sentence came with 5d492875; before has the precedent)
- Owner-gated: no

A field-type change (a non-`Sync` handle) or a well-meant `#[derive(PartialEq)]` would falsify the crate page with nothing failing. For each contract clause a committed check should fail if it were wrong.

Evidence:

       299	//! requires `std`; no `no_std` build is offered. [`Accumulator`] is `Clone`,
       300	//! `Default`, `Debug`, and `Send + Sync` — though `Sync` buys less than usual:
       ...
       308	//! deliberately not `PartialEq`: two spellings of one value would compare
       309	//! unequal, so compare by subtracting and reading the difference's sign.

Resolution: `static_assertions` as a dev-dependency; in accumulator/tests.rs, `assert_impl_all!(Accumulator: Send, Sync, Clone, Default, Debug); assert_not_impl_any!(Accumulator: PartialEq);`. Acceptance: adding `#[derive(PartialEq)]` at accumulator.rs:89 fails to compile the test target. Construction: add that derive today; nothing fails.

### suanpan-26: `Limbs::next_back` and the 32-bit word pairing are exercised only by a consumer crate
- Where: crates/suanpan/src/limbs.rs:68-72 (related: 19-29; crates/before/src/codec/base.rs:111)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep: no `Limbs::new`, `next_back`, or `.rev()` over a `Limbs` in crates/suanpan's tests; before's `msb_windows` at codec/base.rs:111 is the only reverse consumer; `WORDS_PER_LIMB` is 2 only where `Word` is 32 bits, and suanpan's suite has no 32-bit run); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (4cf40eb9 made `Limbs` public with no tests)
- Owner-gated: no

A public method whose only coverage lives in another crate's suite is a blind spot the roster cannot see (the roster's cost exclusion for `Limbs iteration` is about denomination, not correctness). A chunk-order or padding bug in `next_back` would appear as a rank failure in before, not as a `Limbs` failure here.

Evidence:

        68	impl DoubleEndedIterator for Limbs<'_> {
        69	    fn next_back(&mut self) -> Option<u64> {
        70	        self.chunks.next_back().map(pack_limb)
        71	    }
        72	}

Resolution: a sibling `limbs/tests.rs` proptest: for random little-endian byte strings `b`, `Limbs::new(&UBig::from_le_bytes(&b)).collect::<Vec<u64>>()` equals the minimal LE `u64` limbs of `b`, `.rev()` equals the reverse, and zero yields an empty iterator; the same test covers the pairing if a 32-bit run is ever wired. Acceptance: a mutant replacing `next_back`'s body with `self.chunks.next().map(pack_limb)` fails inside suanpan. Construction: make that swap and run `cargo nextest run -p suanpan --all-features`: green today.

### suanpan-28: `SOURCES` is a hand-kept file roster; a new module with `pub` items escapes the totality test
- Where: crates/suanpan/src/claims.rs:34-57 (related: 82-84; crates/suanpan/src/claims/tests.rs:171-195; crates/suanpan/src/lib.rs:352-362; crates/surface-scan/src/lib.rs:73-133)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read: `extract_public_fns` walks only the listed specs; the liveness probe at tests.rs:174-177 checks two known names; the family list states its review-held totality at claims.rs:82-84 while `SOURCES` states none; before's rustdoc-JSON `surfacecheck` names suanpan nowhere); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found
- Owner-gated: no

Principle 6: the roster's totality guarantee ("a new public operation fails here until its cost row is pinned", lib doc 20-23) holds only for files someone remembered to list.

Evidence:

        34	/// The public-API sources of record: every module carrying `pub` items
        35	/// (the crate root re-exports them and declares no `pub fn` of its own).
        36	pub(crate) const SOURCES: &[SourceSpec] = &[

Resolution: a binding test that parses `src/lib.rs` for its `mod name;` / `pub mod name;` declarations and asserts each (except `claims`) has a `SourceSpec`, so an unlisted module fails by name. Acceptance and construction: add `mod scratch;` with a `pub fn` to lib.rs unlisted: the claims tests are green today and must fail afterwards; listing it then fails totality until a claim row exists.

### suanpan-33: The only sequence-shaped adversarial instrument is cited by no claim
- Where: crates/suanpan/src/claims.rs:277-285 (related: 111-115; crates/suanpan/tests/amortized_sequences.rs:36-49, 63-78; crates/suanpan/src/claims/tests.rs:116-161)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read claims.rs in full: no citation of tests/amortized_sequences.rs or `sign_flip_oscillation_has_no_width_product`; the reach scan would pass: the test body calls `s1(`, whose body invokes `.add_wide_shl(`, `.sign(`, `.sub_wide_shl(`); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (4398dcd4 landed it uncited; the nearest recorded stance, b8513643's "minimal citation sets", concerns semantic pins, and the roster's policy on this point is undocumented in code)
- Owner-gated: no
- Cross-references: suanpan-tests-26 (the same uncited instrument).

Principle 6 (tamper-evidence): an instrument outside the roster can vanish without a named failure, and this is the crate's only committed check that the sign fold and shifted writes stay additive under an adversary flipping sign across a certified run.

Evidence:

       277	    Claim {
       278	        op: "Accumulator::sign",
       279	        table_cost: Some("amortized O(1)"),
       280	        evidence: Evidence::Witnessed(&[
       281	            (OWN, "no_collapse_fold_re_scans_the_prefix"),
       282	            (OWN, "sign_fold_skips_certified_runs"),
       283	            (BANDS, "accum_static_prefix_touches_flat"),
       284	        ]),
       285	    },

Resolution: `const SEQUENCES: &str = "tests/amortized_sequences.rs";` cited under `sign`, `add_wide_shl`, and `sub_wide_shl`; and state the roster's citation policy (minimal sets or every touch instrument) at claims.rs:111-115 so the next uncited instrument is a decision, not an omission. Acceptance: renaming the test fails `cited_witnesses_exist`; the reach test passes without a `REACH_EXEMPT` entry. Construction: delete the file on a scratch copy: the suanpan suite stays green today.

### suanpan-39: The sign-flip oscillation tripwire has no liveness floor
- Where: crates/suanpan/tests/amortized_sequences.rs:52-61 (related: 36-49; crates/before/tests/meter.rs:6493-6500)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read: with an all-zero grid, `mixed = 0`, `bound = 0`, and `0 <= 0` passes; `s1`'s other assertions are value-only). Hand trace of `s1`, not executed: the first write spills the register (2 touches); each steady-state round costs 2 (add: limb read + deposit) + 5 (sign: read top, zero it, read below deciding at `2^32`, zero the floor, re-deposit) + 2 (sub) + 7 (sign: read, zero, read to partial 0, zero, run skip to digit 0, read -1, zero, re-deposit) = 16, so `s1(n, d) = 16n + 2` at every grid cell; the refutation pass reached the same trace independently
- Seen by: correctness; refutation: confirmed; history: no rationale found (4398dcd4 adopted the shape with no floor; its sibling in before, tests/answer_embedded.rs, has none either)
- Owner-gated: no
- Cross-references: suanpan-tests-25 (the same anchor and defect, filed by the suanpan-tests partition with the executed run).

Doctrine: every ceiling needs a floor derived from irreducible work so it cannot pass vacuously when the counter goes dark. before's bands carry `touches >= ops` and metered.rs pins exact counts; this is the one suanpan touch instrument with neither.

Evidence:

        52	fn assert_no_product(name: &str, grid: [u64; 4]) {
        53	    let mixed = grid[3] as f64 - grid[2] as f64 - grid[1] as f64 + grid[0] as f64;
        54	    let bound = 0.10 * grid[3] as f64;
        55	    eprintln!("MEASURED {name}: grid {grid:?} mixed {mixed:.0} bound {bound:.0}");
        56	    assert!(
        57	        mixed.abs() <= bound,

Resolution: a per-cell floor from irreducible work (two one-limb writes and two sign reads per round: `s1(n, d) >= 6 * n`); better, pin the exact total `16 * n + 2` at all four cells, which subsumes the floor and the no-product bound. Treat `16n + 2` as a hypothesis until one run confirms it. Acceptance: with `touch_meter::record` stubbed to a no-op the test fails; unmodified, the pinned counts hold at all four cells. Construction: stub `record` and run `cargo nextest run -p suanpan --features touch-meter sign_flip_oscillation_has_no_width_product`: it passes today.

### suanpan-37: The testdoc says exclusions "state a mechanism"; the body checks a 20-character floor
- Where: crates/suanpan/src/claims/tests.rs:262-263 (related: 23-24, 299-303, 384-387)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-and-holds for the proxy (978c85c1 chose ">= 20 chars, checked"); the doc overstating it has no rationale
- Owner-gated: no
- Cross-references: board-frame-19 (the same bare length-floor idiom standing in for a mechanism check).

A test's doc comment states its invariant and must be accurate; a 20-character shrug satisfies this one, and suanpan-32 is the live instance: a 78-character false mechanism passed.

Evidence:

       262	/// Every witness a claim cites exists as a `#[test]`-attributed
       263	/// function in its file, and every exclusion states a mechanism.
       299	                assert!(
       300	                    reason.trim().len() >= 20,
       301	                    "{}: an exclusion reason must state a mechanism, not a shrug",
       302	                    claim.op
       303	                );

Resolution: make the doc match the check at 262-263 and 23-24 ("every exclusion carries a reason of non-trivial length; its substance is review-held"); optionally strengthen the check (require a code identifier or a denomination word such as "word-scale", "digit", "allocation") and say so. Acceptance: the docstring's invariant is one the assertion can fail on. Construction: set the `new` reason to any 20+ character string; `cited_witnesses_exist` passes.

### suanpan-38: `scaled_read_costs_the_written_span` asserts a `<= 16` ceiling where the exact count is 3
- Where: crates/suanpan/src/accumulator/tests/metered.rs:43-47 (related: 1-11, 50-54, 786-795, 810-829)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (trace: `apply_limbs` of `[7, 9]` at digit_shift 1000 deposits 7 at 1000 and 9 at 1002 (the odd-limb high halves are zero); `bottom = 1000`, `top = 1002`; `sign_magnitude_shl` starts at `min(bottom, top) = 1000` and `read_digits` walks 1000..=1002 with carry 0 and no drain: 3 touches); executed: no
- Seen by: correctness; refutation: confirmed; history: deliberate-but-expired (ef8894ac wrote the ceiling before the exactness header; 023eff6e added "Every pin asserts an exact count" over a file already holding it)
- Owner-gated: no
- Cross-references: suanpan-tests-13 (the same `<= 16` pin among the three inequality pins).

Header lines 4-5 say every pin is exact; this pin admits a watermark off by up to thirteen digits, while its sibling `scaled_read_costs_the_span_not_the_write_count` pins its 1,003 exactly.

Evidence:

         4	//! Every pin asserts an exact count, not a ceiling: exactness is the
         5	//! liveness floor (a counter that silently stops counting cannot
        43	    assert!(
        44	        scaled_read <= 16,
        45	        "the scaled read scanned {scaled_read} digits: the write watermark \
        46	         is not skipping the never-written prefix"
        47	    );

Resolution: `assert_eq!(scaled_read, 3, ...)`, confirming 3 by one run before pinning; keep the `> 1000` control; amend the header to say adequacy legs (`> 1` at 793, the control at 51) are floors by design. Acceptance: the test passes at exactly 3 and the header sentence is true of the file.

**The suanpan test suites**

### suanpan-tests-16: merge_into_wider's swap, the min in its cost row, has no touch pin in the direction that exercises it
- Where: crates/suanpan/src/accumulator/tests/metered.rs:287-296 (related: crates/suanpan/src/accumulator/tests/metered.rs:362-393, crates/suanpan/src/accumulator/tests/differential.rs:672-699, crates/suanpan/src/accumulator.rs:1169-1175, crates/suanpan/src/claims.rs:271-275, crates/suanpan/src/lib.rs:219)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep: `merge_into_wider` is called in metered.rs only at :291 and :380; at :262-291 the receiver holds 64 or 128 digits against a 2-digit operand, at :372-380 the digit counts tie, so `other.digit_count() > self.digit_count()` at accumulator.rs:1170 is never true on a metered path; `width_ordered_merges_match_the_oracle` checks values only, and the sum is the same either way); executed: no
- Seen by: blind-spots [18]; refutation: confirmed (deleting the swap keeps every committed test green; the rustdoc example takes the swap branch but checks only the value; cargo-mutants' operator mutants of the `>` are killed by the existing pins, but "no swap" is not a campaign mutant); history: no-rationale-found (the census pinned the row with the receiver always wider; the tie commit deliberately pinned only the tie clause)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). Today the narrow-receiver direction costs 4 touches (the swap is live). With the swap at accumulator.rs:1170-1172 deleted, all 39 committed suanpan tests in the metered, differential, and merge selection stay green while the narrow-receiver merge reads 128 touches.

The cost row is `amortized O(min(|self|, |other|))` and the roster cites this test as its sole witness. The row's distinctive content is the min, the min is implemented by the swap, and the swap is on no metered path. The known-bad mechanism (no swap, always read `other`) passes every committed test because the operand is never the wider one where touches are counted. Every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

       287	        let mut receiver = Accumulator::new();
       288	        receiver.add_wide(&wide_value);
       289	        let mut spare = operand;
       290	        touch_meter::reset();
       291	        spare = receiver.merge_into_wider(spare);
       292	        assert_eq!(
       293	            touch_meter::touches(),
       294	            4,
       295	            "merge_into_wider reads the narrower operand only"
       296	        );

    accumulator.rs:
      1170	        if other.digit_count() > self.digit_count() {
      1171	            core::mem::swap(self, &mut other);
      1172	        }

Resolution: add the mirrored leg inside the same `held_bits` loop: `receiver = narrow()` (2 digits), operand holding `(1 << held_bits) - 1` (64 then 128 digits); `touch_meter::reset(); let spare = receiver.merge_into_wider(operand); assert_eq!(touches, 4)`; assert the sum landed in `receiver`; state in the doc that without the swap this leg reads `2 · held_digits` (one read per operand digit plus one deposit each, no carries since every digit stays under 2^33). Acceptance: a metered leg in which `other.digit_count() > self.digit_count()` pins 4 touches at two receiver widths; a build with the swap removed fails that leg by name.
Construction: `let mut receiver = narrow(); let mut operand = Accumulator::new(); operand.add_wide(&((UBig::from(1u8) << 2_048usize) - 1u8)); touch_meter::reset(); let spare = receiver.merge_into_wider(operand); assert_eq!(touch_meter::touches(), 4);` With lines 1170-1172 deleted, `fold_accum` iterates 64 digits: 64 reads + 64 deposits = 128.

### suanpan-tests-25: assert_no_product has no liveness floor: a dark counter passes it, and an exact shift-independent total is measured and available
- Where: crates/suanpan/tests/amortized_sequences.rs:52-61 (related: crates/suanpan/tests/amortized_sequences.rs:36-49, crates/before/tests/meter.rs:6493-6500, crates/suanpan/src/accumulator/tests/metered.rs:4-6, crates/suanpan/src/touch_meter.rs:12-16)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (arithmetic by reading: with `grid = [0, 0, 0, 0]`, `mixed = 0`, `bound = 0`, and `0 <= 0` holds; the exact total is corroborated by the refutation pass's single executed run, whose log I read at `scratchpad/before/refute-suanpan-tests/sign_flip_run.log`: `MEASURED s1_sign_flip: grid [32770, 65538, 32770, 65538] mixed 0 bound 6554`, i.e. `16n + 2` at `d = 32_768` and `65_536`; my per-round trace agrees: add 2, sign 5, sub 2, sign 7, plus 2 for the first-round spill of the parked −1); executed: no (by me; the refutation pass ran the test once)
- Seen by: blind-spots [19], api-economics [28]; refutation: confirmed and executed; history: no-rationale-found (adopted in the same shape as its before twin, which has no floor either; the sibling `accum_*` bands do carry one)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run with the counter made dark). `sign_flip_oscillation_has_no_width_product` passes with grid [0, 0, 0, 0] (mixed 0 <= bound 0). In the same run 14 of the 16 metered.rs pins fail (the two that pass are `magnitude_dispatch_costs_its_width_path` and `limb_stream_matches_the_wide_entry`, so "every pin in metered.rs fails" is slightly overstated). The uniformly-undercounting variant was not run.
- Cross-references: suanpan-39 (the same finding from the suanpan partition), tests-other-6 and tests-other-14 (the before twin `assert_no_product`).

The criterion is a relative bound over a deterministic counter: if `touch_meter::record` stopped counting, or counted nothing for sign reads, every cell reads 0 and the assertion holds; a counter that undercounts uniformly also passes. Meters need liveness floors derived from irreducible work so a ceiling cannot pass vacuously when the counter goes dark. The sibling band harness in before asserts `run.touches >= run.ops` for exactly this reason (meter.rs:6493-6500); this file, the only committed instrument that flips the sign under load, is the one place that floor is missing, and the sibling metered suite's own doctrine (metered.rs:4-6) is that exactness is the liveness floor.

Evidence:

        52	fn assert_no_product(name: &str, grid: [u64; 4]) {
        53	    let mixed = grid[3] as f64 - grid[2] as f64 - grid[1] as f64 + grid[0] as f64;
        54	    let bound = 0.10 * grid[3] as f64;
        55	    eprintln!("MEASURED {name}: grid {grid:?} mixed {mixed:.0} bound {bound:.0}");
        56	    assert!(
        57	        mixed.abs() <= bound,

    before/tests/meter.rs:
      6493	            assert!(
      6494	                run.touches >= run.ops,

Resolution: pin the exact totals the way metered.rs does: assert `s1(n, d) == 16 * n + 2` for all four grid cells with the per-round derivation in the message (add 2: one limb read plus one deposit; sign 5: read, zero, deciding read, collapse zero, re-deposit; sub 2; sign 7: read, zero, read to partial 0, zero, certificate skip to digit 0, read −1, collapse zero and re-deposit; plus 2 once for the parked −1's spill). The measured number is the refutation pass's, not mine; measure once more before committing. At minimum add the universal floor `grid[0] >= 4 * n0` (four calls per round, one touch minimum each). Acceptance: with `touch()` stubbed to a no-op under the feature the test fails; with the shipped code it passes with the pinned totals, and the mixed second difference is exactly 0.
Construction: make `fn touch(count: u64)` (accumulator.rs:21-26) ignore `count` under `touch-meter` and run only `sign_flip_oscillation_has_no_width_product`: it passes with `grid [0, 0, 0, 0]` while every pin in metered.rs fails. Alternatively delete only the `touch(1)` at accumulator.rs:850 (the fold's per-digit read): the grid drops uniformly and the test still passes.

### suanpan-tests-4: zero-valued wide operands are never drawn or witnessed, and they retire the register where the magnitude entries do not
- Where: crates/suanpan/src/accumulator/tests/differential.rs:109-121 (related: crates/suanpan/src/accumulator.rs:253-256, crates/suanpan/src/accumulator.rs:279-284, crates/suanpan/src/accumulator.rs:418-424, crates/suanpan/src/limbs.rs:31-38)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (strategy arms read; `add_wide` calls `self.spill()` unconditionally; `add_magnitude` dispatches `Some(0)` to `add_u64(0)`, which is a no-op under `if delta != 0`; `add_magnitude_shl` has `Some(0) => {}`; `Limbs` doc states a zero value has no limbs); executed: no
- Seen by: blind-spots [21]; refutation: confirmed (dashu's `as_words` documented empty for zero); history: no-rationale-found
- Owner-gated: yes: whether a value-neutral wide call should spill is a production behavior decision

`any::<u64>()` draws a zero limb with probability 2^−64, the cliffy arm forces `limbs[0] = 1` when every mask bit is clear, and `LimbsShl` pads zeros only at the high end, so no test feeds `add_wide`, `sub_wide`, `add_wide_shl`, `sub_wide_shl`, `add_limbs_shl`, or `sub_limbs_shl` a zero value. In production every wide entry spills before applying an empty limb stream, so `add_wide(&UBig::ZERO)` on a register-held value retires the register with no value change, while `add_magnitude(&UBig::ZERO)` and `add_magnitude_shl(&UBig::ZERO, s)` leave it untouched. Zero is an input to every wide entry ("correct at all scales, for all inputs"); the value is preserved either way, so the differential oracle cannot see the representation effect, and nothing pins it.

Evidence:

       111	                let mut limbs: Vec<u64> =
       112	                    mask.iter().map(|&saturated| if saturated { u64::MAX } else { 0 }).collect();
       113	                if limbs.iter().all(|&limb| limb == 0) {
       114	                    limbs[0] = 1;
       115	                }

    accumulator.rs:
       253	    pub fn add_wide(&mut self, delta: &UBig) {
       254	        self.spill();
       255	        self.apply_limbs(Limbs::new(delta), false, 0);
       256	    }
    ...
       420	            Some(0) => {}

Resolution: add a witness: register-held 5; `add_wide(&UBig::ZERO)`, `sub_wide_shl(&UBig::ZERO, 96)`, `add_limbs_shl(core::iter::empty(), 0)`; `assert_value` against 5 and pin the tier the owner intends (`acc.quick.is_some()` if the wide entries should short-circuit zero like `add_magnitude_shl`, the opposite if the spill is the contract). If short-circuiting is chosen, the wide entries gain an `if delta.is_zero() { return; }` before the spill (production; owner-gated). Acceptance: one committed test drives every wide entry with a zero operand on a register-held value and asserts both the value and the tier afterward.
Construction: `let mut acc = Accumulator::new(); acc.add_small(5); acc.add_wide(&UBig::ZERO); assert!(acc.quick.is_some());` fails today, because `add_wide` spills unconditionally and the zero's limb stream is empty.

### suanpan-tests-10: the ledger checker never observes a state produced by the fold, merge, shift, reset, or negate entry points
- Where: crates/suanpan/src/accumulator/tests/ledger.rs:126-172 (related: crates/suanpan/src/accumulator/tests/ledger.rs:249-253, crates/suanpan/src/accumulator/tests/differential.rs:461-485, crates/suanpan/src/accumulator/tests/differential.rs:657-670, crates/suanpan/src/accumulator/tests/differential.rs:676-699, crates/suanpan/src/accumulator.rs:536-575, crates/suanpan/src/accumulator.rs:626-627, crates/suanpan/src/accumulator.rs:666-679)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep: `assert_ledger_invariants` is defined at ledger.rs:31 and called only at :196 and :290; neither alphabet names `add_accum`, `sub_accum`, `*_accum_shl`, `merge_into_wider`, `shl`, `negate`, or `reset`; `read_digits` consumes no certificate); executed: no
- Seen by: blind-spots [17], api-economics [30]; refutation: reframed (the multi-certificate geometry is not a distinct code path: `fold_accum` routes each nonzero digit through `add_at` with `top` settled between calls, and the exhaustive alphabet's jumps to digits 3 and 7 already create and cancel shared-endpoint certificates under the checker; what the checker never sees is the state those entry points leave); history: no-rationale-found (the alphabet's inline rationale covers the transitions it was built to probe)
- Owner-gated: no

`ledger.rs:3-9` declares that correctness depends on the certificates ("a stale certificate over a written digit would corrupt values, not just costs") and the checker is the only structural instrument for them. Both drivers' alphabets consist of digit-0 deltas, shifted one-limb writes, raw word deposits, and sign reads. `shl` on the engine rebuilds the ledger from scratch (`core::mem::take(self)` then `add_accum_shl`), `reset` clears it, and the fold entries deposit through `add_at`; the differential tests that drive those entry points end at `assert_value`, whose read-outs consume no certificate, so a stale or stranded certificate left by any of them would surface only when a later scan happened to consume it. "Not wrong, but you couldn't tell if it were" is a repairable blind spot, and these are exactly the paths a future optimization (an in-place digit shift, a lazier reset) would touch.

Evidence:

       250	        ops in proptest::collection::vec(
       251	            (0u8..5, proptest::collection::vec(any::<u64>(), 1..=2), 0u64..4_096),
       252	            1..150,
       253	        ),

    accumulator.rs:
       626	        let held = core::mem::take(self);
       627	        self.add_accum_shl(&held, shift);

Resolution: cheapest: call `assert_ledger_invariants(&acc, &[])` at the end of `merges_match_the_oracle`, `in_place_shift_matches_the_oracle`, `width_ordered_merges_match_the_oracle`, and `fold_primitives_match_the_oracle` (it is a private-state checker in the same test tree). Better: add arms to `ledger_invariants_hold_on_run_forming_streams` for `negate`, `shl(shift)`, `reset` (then continue the stream, so post-reset spills over the retained buffer are checked), and `add_accum_shl(&snapshot, shift)` where the snapshot is a clone taken earlier with its oracle tracked. Acceptance: the checker runs on a state after each of `add_accum_shl`, `sub_accum_shl`, `merge_into_wider`, `shl`, `negate`, and `reset`; the construction below fails the suite.
Construction: replace the engine branch of `shl` (accumulator.rs:626-627) with an in-place digit shift that moves the digits up by `digit_shift`, touches twice per digit (so the exact 2d pin at metered.rs:443-450 still holds), and leaves `zero_runs` un-reindexed. Every committed test passes (values are correct and no ledger driver calls `shl`); a certificate `(lo, hi)` now covers position `lo + 1`, which holds the old nonzero digit from `lo`, so the proposed `shl` arm fails the soundness clause at the first shifted state, and a subsequent zero-partial fold would skip over a nonzero digit.

### suanpan-tests-18: negative read-out cost (the complement pass) is unpinned; the held-width row is metered only on the positive spelling
- Where: crates/suanpan/src/accumulator/tests/metered.rs:413-430 (related: crates/suanpan/src/accumulator.rs:1090-1107)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (the test negates, pins `negate` at d, negates back, then meters `sign_magnitude` and `sign_limbs` on the positive value; `read_digits`' negative arm runs a second per-digit loop with `touch(1)` when the low part is nonzero; by trace of `−(2^bits − 1)`, digit 0 leaves low 1 and carry −1 and every higher digit leaves low 0 and carry −1, so the complement pass runs d touches and pushes no high digit: 2d total); executed: no
- Seen by: blind-spots [23]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The row is `O(|self|)` and 2d satisfies it, so no claim is violated; but the pin's message ("one carry pass over the span") describes only half the read-out, and the width doubling that makes the pin a linearity witness is absent for the arm with the extra pass. A cost regression confined to the complement arm (an extra pass, a per-digit rescan of `collected`) passes every committed test.

Evidence:

       420	        acc.negate();
       421	
       422	        touch_meter::reset();
       423	        let (sign, magnitude) = acc.sign_magnitude();
       424	        assert_eq!(
       425	            touch_meter::touches(),
       426	            held_digits,
       427	            "sign_magnitude at {held_digits} held digits: one carry pass \
       428	             over the span"
       429	        );

    accumulator.rs:
      1097	                for digit in collected.iter_mut() {
      1098	                    touch(1);

Resolution: after the positive legs, negate once more and meter `sign_magnitude` and `sign_limbs` on the negative spelling, pinning `2 * held_digits` at both widths with the derivation (carry pass plus complement pass; the complement of a nonzero low part never carries out, so no high digit is pushed). Acceptance: an exact pin for both read-outs on a negative d-digit value at two widths.
Construction: `acc.negate(); touch_meter::reset(); let _ = acc.sign_magnitude(); assert_eq!(touch_meter::touches(), 2 * held_digits);` (the 2d is my derivation; measure once before committing).

### suanpan-tests-7: size_probe_covers_the_value asserts a full digit of slack its doc does not claim, and the doc's own bound is one bit too tight
- Where: crates/suanpan/src/accumulator/tests/differential.rs:487-505 (related: crates/suanpan/src/accumulator.rs:39, crates/suanpan/src/accumulator.rs:942-948, crates/suanpan/src/accumulator/tests/metered.rs:377-378, crates/suanpan/src/accumulator/tests/witnesses.rs:317-321)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (derivation against `LAZY_LIMIT` and `digit_count`: with `D = digit_count`, the value is at most Σ_{i<D} (2^33 − 1)·2^(32i) < (2 + 2^−31)·2^(32D), so `bit_len ≤ 32D + 2`, and the bound is reached: digits `[2^33 − 1, 2^33 − 1]` denote 2^65 + 2^32 − 1 with `bit_len` 66 = 32·2 + 2; the register arm gives `bit_len ≤ 32D`); executed: no
- Seen by: structure-prose [0]; refutation: confirmed (severity proposed medium to low because sibling pins catch the mutant); history: no-rationale-found (the `* 32 + 33` and the doc sentence arrived together in before's codec tests and were carried verbatim)
- Owner-gated: no

The doc claims each digit spans 32 bits plus one lazy-zone bit; read per digit that is 33D, read as an aggregate it is 32D + 1, and the tight bound is 32D + 2. The body asserts `32D + 33`, a whole digit looser than any reading. With that slack, the mutant `None => self.top` in `digit_count` (one digit short) satisfies `32·top + 33 ≥ bit_len` except when `bit_len = 32·top + 34`, a corner random streams do not reach, so this test admits the known-bad mechanism its sentence exists to exclude. `merge_tie_reads_the_operand` and `sign_collapse_tightens_the_top_and_arms_domination` pin `digit_count` exactly and would catch that mutant; the suite is not blind, but this test does not test its own claim, and a testdoc's incorrectness is a bug in the test. I keep medium over the refutation's low because both the assertion and the doc are wrong, in opposite directions, and the fix is one character plus one sentence.

Evidence:

       487	    /// `digit_count` covers the held width: after any stream, the value's
       488	    /// magnitude fits inside the counted digits (each digit spans 32 bits
       489	    /// plus one lazy-zone bit of overhang).
    ...
       499	            prop_assert!(
       500	                u64::try_from(acc.digit_count()).expect("digit counts fit u64") * 32 + 33
       501	                    >= magnitude.bit_len() as u64,
       502	                "digit_count misses value width"
       503	            );

Resolution: tighten to `* 32 + 2 >= bit_len` and restate the doc with the derivation: every digit is under 2^33 in magnitude, so the value is under (2 + 2^−31)·2^(32·digit_count), at most two bits past the counted width; a register value's `digit_count` is `ceil(bits/32)`, so the same bound holds there. Import `DIGIT_BITS` for the 32 if the owner prefers named constants in the model. Acceptance: with `+ 2`, `size_probe_covers_the_value` passes on the committed tree, and the mutant `None => self.top` in `Accumulator::digit_count` fails it by name.
Construction: in accumulator.rs:946 change `None => self.top + 1,` to `None => self.top,`; run `size_probe_covers_the_value` (expected: passes with the current `+ 33`); change the test to `+ 2` and re-run (expected: fails). Restore the production line.

### suanpan-tests-13: three inequality pins and six single-shape pins in a module whose doc says every pin is exact and doubled
- Where: crates/suanpan/src/accumulator/tests/metered.rs:43-54 (related: crates/suanpan/src/accumulator/tests/metered.rs:4-8, crates/suanpan/src/accumulator/tests/metered.rs:20-23, crates/suanpan/src/accumulator/tests/metered.rs:788-795, crates/suanpan/src/accumulator/tests/metered.rs:816-823, crates/suanpan/src/lib.rs:283-286)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (module doc and the three inequalities read; exact values by trace against accumulator.rs: `[7, 9]` at shift 32_000 deposits digits 1000 and 1002, `sign_magnitude_shl` reads from `bottom = 1000` through `top = 1002` for 3 touches with carry 0; the full read walks 0..=1002 for 1003, which the sibling pin at :818-820 already asserts exactly; the `low_top` leg reads digit 4, zeroes it, reads digit 3 to partial 2^33, zeroes the floor, and `add_at(3, 2^33)` recenters in two steps to land 2 back at index 4, for 6); executed: no
- Seen by: structure-prose [2], blind-spots [20], api-economics [32]; refutation: confirmed; history: deliberate-but-expired (`<= 16` and `> 1000` predate the exact-pin census and the module doc, which neither revisited; `> 1` postdates the doc as a deliberate shape bound; the "each pin repeats across a doubling" clause overclaimed on the day it was written)
- Owner-gated: no
- Cross-references: suanpan-38 (the `<= 16` pin from the suanpan partition).

The module doc states "Every pin asserts an exact count, not a ceiling" and "each pin repeats its schedule across a doubling". `scaled_read_costs_the_written_span` asserts `<= 16` (exact: 3) and `> 1000` (exact: 1003), and `decision_bound_top_decides_on_the_first_touch` asserts `> 1` (exact: 6); the `<= 16` ceiling is the artifact the doc's own liveness argument says not to commit (a dark counter reads 0 ≤ 16, and a hidden second pass over the span reads 6 ≤ 16). Six pins run one shape with no doubling (`scaled_read_costs_the_written_span`, `top_settlement_steps_are_metered`, `sign_fold_skips_certified_runs`, `domination_reads_cost_one_touch_after_the_first`, `decision_bound_top_decides_on_the_first_touch`, `scaled_read_costs_the_span_not_the_write_count`). The crate declares touch counts an exact public contract, and the exact values are derivable. The 6-touch value also exposes a fact worth pinning: a top digit of magnitude 2 over a zero digit is a collapse fixed point (the re-deposit recenters back to the same spelling), so every later read repeats the 6-touch descent rather than costing one touch.

Evidence:

         4	//! Every pin asserts an exact count, not a ceiling: exactness is the
         5	//! liveness floor (a counter that silently stops counting cannot
         6	//! satisfy an exact total) and the flatness witness at once — each pin
         7	//! repeats its schedule across a doubling of the axis its row claims
         8	//! independence from. The adequacy tripwire
    ...
        43	    assert!(
        44	        scaled_read <= 16,
    ...
        50	    assert!(
        51	        touch_meter::touches() > 1000,
    ...
       792	    assert!(
       793	        touch_meter::touches() > 1,

Resolution: pin `scaled_read == 3`, the full read `== 1003`, and the below-bound descent `== 6` (measure the last once before committing; my 6 is by reading), each with its derivation in the message as the sibling pins do, and replace "O(1)-ish" at :22 with the number. Narrow the module doc's doubling clause to the pins that double, or add the doubling where it is cheap (a second parked digit index for the scaled reads, a second prefix width for the settlement scan). Acceptance: every assertion on `touch_meter::touches()` in metered.rs is `assert_eq!` against a derived constant, and the module doc's two sentences are true of every pin in the file.
Construction: insert a second `for &digit in &self.digits[start..=self.top] { touch(1); }` loop into `read_digits`: `scaled_read` becomes 6, still `<= 16`; the full read becomes 2006, still `> 1000`; every exact pin elsewhere fails, so the gap is specific to these assertions.

### suanpan-tests-23: the executable headroom derivation does not check what its expect messages say: checked_shl never detects bits shifted out
- Where: crates/suanpan/src/accumulator/tests/witnesses.rs:494-500 (related: crates/suanpan/src/accumulator.rs:52-65, crates/suanpan/src/accumulator/tests/witnesses.rs:525-546, rust-toolchain.toml)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (the pinned 1.97.1 core source, `library/core/src/num/int_macros.rs:1397-1398`: "Checked shift left. Computes `self << rhs`, returning `None` if `rhs` is larger than or equal to the number of bits in `self`", with the doc example `0x10.checked_shl(BITS_MINUS_ONE) == Some(0)`; the overflow consequence assessed by reading); executed: no
- Seen by: structure-prose [15]; refutation: reframed from "move to a const assert" to "the check is not a check"; history: no-rationale-found
- Owner-gated: no

`ceiling.checked_shl(QUICK_SHIFT_MAX as u32).expect("the widest shifted fold fits i128")` returns `None` only for a shift amount of 128 or more, so it passes for any `QUICK_SHIFT_MAX` under 128, including 31, where `2^96 << 31 = 2^127` overflows `i128` and wraps to `i128::MIN`. The three lines therefore claim a check they do not perform. The headroom is in fact pinned by the value-level legs that follow (a wrapped `operand_value << shift` in `fold_accum` panics in debug and mismatches the oracle in release), so the derivation is decorative and its messages are inaccurate.

Evidence:

       494	    let ceiling = i128::try_from(QUICK_MAX).expect("the ceiling fits i128");
       495	    let widest_fold = ceiling
       496	        .checked_shl(QUICK_SHIFT_MAX as u32)
       497	        .expect("the widest shifted fold fits i128");
       498	    ceiling
       499	        .checked_add(widest_fold)
       500	        .expect("the worst register sum fits i128");

Resolution: replace the three lines with a compile-time assertion beside the constants in accumulator.rs using overflow-detecting arithmetic, e.g. `const _: () = assert!((QUICK_MAX << QUICK_SHIFT_MAX) + QUICK_MAX <= i128::MAX as u128);` in `u128` (where a shift past 127 is itself a compile error), with the headroom argument restated at the constants (accumulator.rs:63-65 already states it in prose); keep the value-level extremes, which are the test's substantive content. Acceptance: the build fails if `QUICK_SHIFT_MAX` is raised past what `i128` admits; the witness body starts at the value-level extremes.
Construction: set `QUICK_SHIFT_MAX` to 31 in a scratch build: lines 494-500 pass (`checked_shl(31)` returns `Some(i128::MIN)`), and the value legs at :525-546 fail in debug (overflow panic in `fold_accum`'s shift) or by oracle mismatch in release. Restore the constant.

### suanpan-tests-26: the one committed sign-flip instrument is not cited by the claims roster's sign rows
- Where: crates/suanpan/tests/amortized_sequences.rs:66-67 (related: crates/suanpan/src/claims.rs:277-293, crates/suanpan/src/claims/tests.rs:28-55, crates/suanpan/src/claims/tests.rs:139-161, crates/suanpan/src/claims/tests.rs:279-283, crates/suanpan/tests/amortized_sequences.rs:14-16)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (grep for `amortized_sequences` and `sign_flip_oscillation` under crates/suanpan/src, crates/before/src, crates/before/tests, the justfile, and .github returns nothing; claims.rs:277-293 cites two OWN pins and the BANDS static-prefix band; `cited_witnesses_exist` resolves any manifest-relative path at claims/tests.rs:280, and `reaches` follows same-file helpers, and `s1` calls `.sign()` at :44 and :46; the sibling bands `comb_run` and `static_prefix_run` in before/tests/meter.rs:6526-6541 and :6600-6617 assert `Greater` on both halves of every cycle, so the module doc's one-polarity claim holds where I checked it); executed: no
- Seen by: structure-prose [12], api-economics [29]; refutation: confirmed; history: no-rationale-found (the roster predates the test by two days; the adopting commit did not touch it)
- Owner-gated: no
- Cross-references: suanpan-33 (the same finding from the suanpan partition).

The roster exists to bind "the legs that rot silently without a name" (claims.rs:5-7). `Accumulator::sign` and `is_negative` cite witnesses that all hold one polarity; this test is the sign row's unique evidence for the sign-flip regime, and a rename or deletion would orphan nothing today.

Evidence:

        66	#[test]
        67	fn sign_flip_oscillation_has_no_width_product() {

    claims.rs:
       277	    Claim {
       278	        op: "Accumulator::sign",
       279	        table_cost: Some("amortized O(1)"),
       280	        evidence: Evidence::Witnessed(&[
       281	            (OWN, "no_collapse_fold_re_scans_the_prefix"),
       282	            (OWN, "sign_fold_skips_certified_runs"),
       283	            (BANDS, "accum_static_prefix_touches_flat"),
       284	        ]),
       285	    },

Resolution: add `const SEQUENCES: &str = "tests/amortized_sequences.rs";` and cite `(SEQUENCES, "sign_flip_oscillation_has_no_width_product")` on the `sign` claim, and on `is_negative` with a `REACH_EXEMPT` entry mirroring the existing delegation reason. Whether to instead move the test into metered.rs is an open question below. Acceptance: `cited_witnesses_exist` and `cited_witnesses_reach_their_operations` pass with the new edge; renaming the test fails `cited_witnesses_exist` by name.

**Nits in this section** (full records in the evidence files)

| id | where | claim | resolution |
|---|---|---|---|
| suanpan-34 | `crates/suanpan/src/claims.rs:316-320` | `digit_count`'s exclusion reason names `top_settlement_steps_are_metered`, a test the roster never checks. | Cite it as a witness with a `REACH_EXEMPT` entry, or drop the name. |
| suanpan-tests-3 | `crates/suanpan/src/accumulator/tests/differential.rs:50-53` | The differential generator never draws `sub_small`, `add_u64`, `sub_u64`, `add_u64_shl`, `sub_u64_shl`; the last two run only under `touch-meter`; `merges_match_the_oracle` ignores `fresh`. | Add `Op::Word`/`Op::WordShl` arms; route negative `Small` through `sub_small`; take engine flags in the merge test. |
| suanpan-tests-12 | `crates/suanpan/src/accumulator/tests/metered.rs:1-11` | metered.rs does not state the process-per-test premise its exact pins rest on (stated at touch_meter.rs, not where a reproducer looks). | One sentence in the module doc naming nextest. |

## The instruments

**Oracle and laws**

### oracle-laws-10: The arbitrary generators' normal-form claim has no direct pin
- Where: crates/before/src/oracle/tests.rs:42-60 (related: crates/before/src/testing/generators.rs:334-366, crates/before/src/oracle/version.rs:331-339)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn 'is_normal()'` over `src/`, `tests/`, `fuzz/`: every pin is over op-trace outputs, decode results, the exhaustive enumeration, or an operation's output; none names `arb_oracle_party()`/`arb_oracle_version()` output, and `testing/generators/tests.rs` has no `is_normal` call); executed: no
- Seen by: instrument-correctness [38]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The generators' docs assert normal form by construction and the oracle's `event` relies on it (the `filled != *self` structural comparison at version.rs:333 is semantic equality only on normal forms), but no test asserts `is_normal()` on a generated tree; a regression in `node`'s normalization would appear as differential failures far from the cause rather than by name (Principle 8: a claim is verified by an instrument, not by the sentence stating it).

Evidence:

        43	    /// Every value any op produces is in normal form (parties and versions),
        44	    /// including the result of a join.
        45	    #[test]
        46	    fn normal_form(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {

    generators.rs:
       358	/// (including values near/beyond `u64::MAX`); every interior node goes through
       359	/// the oracle's normalizing `Version::node`, so the result is always in normal
       360	/// form (a zero-base child at every node, no collapsible `(n, m, m)`).

Resolution: Add one proptest beside `normal_form` (or in `testing/generators/tests.rs`) asserting `arb_oracle_party()` and `arb_oracle_version()` outputs satisfy `is_normal()`. Acceptance: removing the `debase` step from `Version::node` (version.rs:82-84) or the collapse arm from `Party::node` (party.rs:24-25) fails the new test by name.

Construction: Change `Version::node` to build `Version::Node(n, Arc::new(l), Arc::new(r))` unconditionally and run the oracle suite: `normal_form` and the paper examples fail through `join_off`, but no failure names the generator's output as the non-normal artifact.

### oracle-laws-14: Two incidental-only laws lack a constructed arm, and nothing measures antecedent liveness
- Where: crates/before/src/laws.rs:23-26 (related: crates/before/src/laws.rs:2770-2772, 601-603, 2250-2252, 3141-3143; crates/before/src/testing/algebraic_laws/tests.rs:105-115)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read each conditional law; confirmed the constructed twins the refutation named: `without_inverts_fork` 2119-2125, `fork_halves_covered_by_parent` 2044-2048, `meet_join_absorption` 474-476, `lag_is_the_rank_gap` 576-578 with `lag_zero_iff_dominated` 570-572; grep for antecedent/liveness/vacuous in the drivers returns nothing); executed: no
- Seen by: adequacy [14]; refutation: reframed (four of the six cited laws have constructed twins under other names; the party-group line numbers in the original were +200 off); history: no-rationale-found (the policy and the older implications date from the founding commit; the constructed-plus-incidental pattern was applied to newer laws in d4c7d827 and 8350c556 without revisiting these)
- Owner-gated: no

The module doc commits to constructing antecedent witnesses where possible. `disjoint_projections_share_nothing` waits for a disjoint party pair, where a fork half is constructible as `without_inverts_fork` does; the three `*_eq_implies_hash_eq` laws wait for `a == b`, where `decode(encode(a))` constructs the interesting case (equal value, distinct buffer) on every call. Under the arbitrary drivers, which draw pairs independently and exist to reach shapes the op pipeline never produces, these antecedents fire only by chance, and no floor in the suite would report a law that had gone vacuous.

Evidence:

        23	//! law holds unconditionally on the inputs its group admits (below);
        24	//! conditional laws are stated as implications, vacuously true when the
        25	//! antecedent fails, and where they can, they *construct* a witness for the
        26	//! antecedent instead of waiting for one.

      2770	    fn disjoint_projections_share_nothing {
      2771	        !p.is_disjoint(q) || ((v / p).to_version() & (v / q).to_version()).is_empty()
      2772	    }

       601	    fn version_eq_implies_hash_eq {
       602	        a != b || hash_of(a) == hash_of(b)
       603	    }

Resolution: Add a constructed arm to each (the `constructed && incidental` shape at 933-937): `disjoint_projections_share_nothing` on `(keep, give)` from `p.dangerously_alias().fork()`; the eq/hash trio on `(a, decode(encode(a)))`. Optionally one deterministic test asserting each incidental antecedent is satisfiable on a small fixed population, as a liveness pin. Acceptance: every conditional law either runs a constructed arm on every call or carries a comment naming why none is constructible; law names unchanged.

Construction: Locally count antecedent hits in the four predicates and run only the `version_pair_laws`, `party_pair_laws`, `clock_pair_laws`, and `version_party_pair_laws` drivers (organic driver disabled) at the default 256 cases; a zero count demonstrates that the arbitrary-regime driver asserts nothing for that law, and nothing committed would report it.

### oracle-laws-13: Two oracle test doc comments claim invariants their bodies never assert
- Where: crates/before/src/oracle/tests.rs:795-829 (related: none)
- Class / severity / confidence: test-quality / medium / high
- Provenance: assessed (read: the `version_min_ticks` body has no join of two versions; the `clock_own_version` body never calls `tick`; line 809 is implied by 808 for a natural count); executed: no
- Seen by: adequacy [13], structure-prose [21], instrument-correctness [37]; refutation: confirmed (severity argued down because production `min_ticks` is pinned elsewhere; I keep medium because the doc claims coverage of the oracle's own additivity, which nothing in the crate checks); history: no-rationale-found (doc and body landed mismatched in f0b83733; 56a08f90 re-typed the body only)
- Owner-gated: no

Every test's doc comment states its invariant in English and must be accurate; the crate's AGENTS.md says review holds it to that standard, and the doctrine calls an incorrect one a bug in the test. `version_min_ticks` promises additivity under a disjoint-support join and non-domination by either operand; `clock_own_version` promises that ticking raises the own version; neither body checks its promise, so a coverage audit reading these docs is misled.

Evidence:

       796	    /// `min_ticks` is the sum of every base, so it is `0` only for the zero
       797	    /// version, additive under a disjoint-support join (`(a|b)` over fork
       798	    /// halves sums their counts), and dominated by neither operand alone.

       808	        prop_assert_eq!(v.min_ticks() == crate::Ticks::ZERO, *v == Version::new()); // zero iff the zero version
       809	        prop_assert!(v.min_ticks() >= crate::Ticks::from(1u64) || *v == Version::new());
       810	        // The seed ticked once in a line costs exactly one.
       811	        let mut one = Version::new();
       812	        one.tick(&Party::seed());
       813	        prop_assert_eq!(one.min_ticks(), crate::Ticks::from(1u64));

       818	    /// `own_version` is exactly `version() / party()`: the clock's history within
       819	    /// the region it owns. It is a sub-version of the full version, and ticking
       820	    /// (which advances only the owned region) raises it.

       826	        prop_assert_eq!(c.own_version(), c.version() / c.party()); // the definition
       827	        prop_assert!(leq(&c.own_version(), &c.version())); // a sub-version

Resolution: Assert the claims or trim the docs. Additivity is constructible on every call: fork a trace member's party into `keep`/`give`, take `x = v / &keep` and `y = v / &give`, assert `(x | y).min_ticks() == x.min_ticks() + y.min_ticks()` and `(x | y).min_ticks() >= x.min_ticks().max(y.min_ticks())`. For `clock_own_version`, tick a clone and assert `own_version` strictly rises under `leq`. Delete the redundant line 809 either way. Acceptance: each sentence of both doc comments corresponds to an assertion in its body.

Construction: Read 796-798 against 808-813 and 818-820 against 826-827: no assertion involves a join of two versions or a `tick` of the clock, so the documented additivity and tick-raises clauses have no corresponding check.

### oracle-laws-16: Bundled laws report one name for seven to ten independent clauses
- Where: crates/before/src/laws.rs:383-402 (related: crates/before/src/laws.rs:696-739, 1309-1359, 1369-1404, 1419-1472, 1486-1540, 2691-2717, 3401-3447, 75; crates/before/src/surface.rs `Leg::Law` citations of the bundled names)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read the bodies; `assert_laws!` and `drive!` report only the bundle name); executed: no
- Seen by: structure-prose [26]; refutation: confirmed (and owner-gated in practice); history: no-rationale-found
- Owner-gated: yes (`Law<F>` is a `pub type` under the `laws` feature; splitting moves roster citations)

The module's stated payoff is that a harness "reports the *name* of any law that fails". The bodies already bind sub-results to descriptive names (`definitional`, `accessors`, `reborrowed`, `settled`, `commutative`, ...) and then discard them in one `&&` chain, so a shrunk fuzz input arrives with a bundle name and the failing clause must be bisected by hand, at exactly the moment the instrument is supposed to help.

Evidence:

         7	//! Each law is a `(&str, fn(...) -> bool)` pair in a slice grouped by predicate
         8	//! signature, so a harness iterates a slice, feeds every law the same inputs,
         9	//! and reports the *name* of any law that fails. The crate's law proptests

       731	        definitional
       732	            && accessors
       733	            && reborrowed
       734	            && settled
       735	            && commutative
       736	            && flip_subsumed
       737	            && empty_edge
       738	            && unary_edge

        75	pub type Law<F> = (&'static str, F);

Resolution: Either split each bundle into one law per clause (the `surface.rs` citations of `ranked_carries_own_rank` and `span_is_the_pair_hull` move with them), or change the predicate to `fn(...) -> Result<(), &'static str>` returning the failing clause's name and have the three consumers report `law/clause`. The second keeps roster and citations stable but changes a feature-gated public type. Acceptance: a deliberately negated inner conjunct (say `commutative` in `span_is_the_pair_hull`, in a scratch build) produces a driver or fuzz report naming that clause.

Construction: Negate `commutative` in `span_is_the_pair_hull` and run the `version_pair_laws` driver: the report names `span_is_the_pair_hull` with the shrunk pair and nothing identifies which of the eight named booleans went false.

### oracle-laws-21: The acceptance laws' `Err` arms assert less than their docs say, and the clock group has no best-effort law
- Where: crates/before/src/laws.rs:2345-2345 (related: crates/before/src/laws.rs:2325-2329, 2390-2400, 3278-3281, 3302-3307)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read: the accumulator starts as an alias of the receiver and only grows through `join`, so `acc.covers(p)` holds for any fold that does not corrupt `self` on refusal; `CLOCK_AND_LIST` has six laws and none plants a mid-stream clash); executed: no
- Seen by: refutation (new item 1); refutation: raised; history: not examined
- Owner-gated: no

The docs gloss "a refused one still absorbed every region it could" as "the accumulator covers its original region", but covering the original region shows only that the accumulator never shrank. The absorb-around-the-clash claim is carried by `party_join_all_is_best_effort_at_any_width` alone, for parties only; the clock group has no twin, so its collection cannot on its own distinguish best-effort from fail-fast (the oracle differentials in `clock/tests.rs` can, which is one more reason oracle-laws-2 must keep an accumulator comparison).

Evidence:

      2325	    /// An accepted fold equals the sequential pair joins (the bound pair
      2326	    /// operation, never the n-ary door, so the two sides cannot share a
      2327	    /// broken arm); a refused one still absorbed every region it could —
      2328	    /// the accumulator covers its original region — and handed at least
      2345	            Err(returned) => !pairwise_disjoint && !returned.is_empty() && acc.covers(p),

      3305	                    && acc.party().covers(c.party())
      3306	                    && le(c.version(), acc.version())

Resolution: Weaken both docs to what the clause checks (the accumulator is never corrupted on refusal), and add `clock_join_all_is_best_effort_at_any_width` as the twin of the party law (fork `width` children, tick them apart, plant an alias of the keeper mid-stream, expect exactly the alias back and the keeper's party restored with the join of every line's version). Acceptance: each `Err`-arm sentence maps to a clause; a fail-fast `Clock::join_all` fails a `CLOCK_AND_LIST` law by name.

Construction: Make `Clock::join_all` fail-fast (on the first `accept` failure, push the remaining inputs to `overlapping` and stop). Every `CLOCK_AND_LIST` law stays green by reading: the acceptance law's `Err` arm sees `!pairwise_disjoint`, a nonempty hand-back, an accumulator covering its origin and dominating its original version; the conservation law sees every unabsorbed input handed back; the reunion, `sync_all`, `recv_all`, and `absorb_all` laws feed pairwise-disjoint families. Only `clock/tests.rs`'s oracle differentials convict it.

**Meter core: generators and counters**

### meter-core-2: Twenty registry-dispatched generators have no size or canonicality pin; four exact closed forms are wrong; six event shapes never meet a strict decode
- Where: crates/before/src/meter.rs:29-32 (related: meter/tests.rs:1-2, 10-19; meter.rs:641-642, 659, 669-670, 688, 973-975, 1091-1096, 1131-1133, 1157, 1258-1263; registry.rs:311-378; version.rs:1192-1201; encode.rs:19-23, 54-58; board/family.rs:765-786, 1232-1234; tests/meter.rs:411-413; fill/tests.rs:47-49, 138-149; query/tests.rs:36; tests/verdict_matrix.rs:329-353)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep -cw of each name in meter/tests.rs returns 0, the lone `staircase` hit at 731 being prose; registry dispatch, `Version::from_bits`, `encode_bits`, `version_of` at every consumer, and the board's envelope-only arm read; closed forms re-derived by hand from each emission sequence and gamma.rs:28's width rule); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (scope reframed: only the six memo-family event shapes meet no strict decode anywhere; the board re-decodes WideTail, Staircase, RevealComb, RevealCombHifloor, PureComb, AscendCliff); history: no rationale found (the claim was true at 6014124ad, whose message records the pin catching an off-by-one in `bigroot`; the 07-25/26 generator wave landed without pins)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). The docs' exact formulas are wrong for `wide_tail` (4d+2b+2 emitted vs 4d+2b+3), `memo_chain` shared (10k+6 vs 13k+9), `memo_comb_id` (14d+8 vs 14d+12), and `memo_churn_id` (10d+4 vs 14d+6); `staircase` is exactly 6d+2 and exceeds its 5d+8 capacity hint from d = 7; `memo_fanout` is k(4b+6)+6. At HEAD all twenty uncited generators do round-trip strictly, but with `memo_chain`'s shared range leaf set to 0 the emitted stream is rejected by `Version::decode` as `NotCanonical` while the two committed `memo_chain` rows in tests/meter.rs still build and measure it green.

The module doc states that every generator's output round-trips through strict decode and that its exact bit length is a closed formula pinned by this module's tests. Neither holds for `nested_full_id`, `nested_left_full_id`, `wide_tail`, `staircase`, `memo_chain`, `memo_chain_id`, `memo_comb`, `memo_comb_id`, `memo_fanout`, `memo_oscillating`, `memo_churn`, `memo_churn_id`, `descending_raises`, `descending_raises_id`, `reveal_comb`, `reveal_comb_hifloor`, `reveal_comb_id`, `pure_comb`, `pure_comb_id`, or `ascend_cliff_id`: none is imported or called in tests.rs. Re-deriving each emission with `gamma(n)` costing `2*floor(log2(n+1)) + 1` bits: `wide_tail` emits `4d + 2b + 2` (doc `4d + 2b + 3`; the doc's own layout line sums to `4d + 2b + 2`), `memo_chain(k, false)` emits `10k + 6` (doc `13k + 9`), `memo_comb_id` emits `14d + 8` (doc `14d + 12`; its layout line sums to `14d + 8`), `memo_churn_id` emits `10d + 4` (doc `14d + 6`; its layout line sums to `10d + 4`), `staircase` is exactly `6d + 2` (doc `~5d`; the `5d + 8` capacity hint under-allocates from `d = 7`), and `memo_fanout` is `k(4b + 6) + 6` (doc `~(13k + 2kb + 9)`, half the leading term; the capacity hint `13k + 4b + 9` has no `kb` term at all). Every consumer lifts event shapes through `Packed::version()`, which is `Version::from_bits(encode_bits(..))`: `from_bits` adopts without validation ("Callers guarantee canonical skyline form") and `encode_bits` asserts only clean parsing and full consumption, never minimal topology. The six memo-family event shapes are envelope-only (family.rs:770-775 is an `unreachable!` arm), so they never pass the board's `decode_version` either; they are measured in tests/meter.rs, fill/tests.rs, query/tests.rs, and tests/verdict_matrix.rs with no check that the stream is one `Version::decode` would accept. This breaches Principle 6 (the cheapest artifact satisfying the proxy must be the intended one) and Principle 5 (the doc claim is false as written).

Evidence:

        29	//! Every generator output is strict normal form: it round-trips through
        30	//! [`Party::decode`](crate::Party::decode)/[`Version::decode`](crate::Version::decode)
        31	//! and re-encodes byte-identically, and its exact bit length is a closed
        32	//! formula in the parameters (pinned by this module's tests). Normal form is

       641	/// A right-leaning spine of zero leaves with one `2^b − 1` tail leaf: depth
       642	/// `d`, `4d + 2b + 3` bits.

       973	/// The memo-chain event `Q(k, distinct)`: a right-leaning spine of `k`
       974	/// single-leaf left-full sites, `~(14k + 9)` bits distinct (γ(j) codes), `13k +
       975	/// 9` shared.

      1091	/// The memo-comb id over [`memo_comb`]: a covering `(1, ·)` site per level
      1092	/// interleaved with the single-leaf sites' `(1, (1, 0))`, `14d + 12` bits.

      1258	/// The memo-churn id over [`memo_churn`]: a covering `(1, ·)` root, per level
      1259	/// the site's `(1, (1, 0))` under a carrier whose right arm continues, and an
      1260	/// absent id over the descending run, `14d + 6` bits.

    version.rs
      1196	    /// Callers guarantee canonical skyline form; the freeze seals the
      1199	    pub(crate) fn from_bits(bits: codec::BitsBuf) -> Self {
      1200	        Version(codec::Bits::freeze(bits))

Resolution: Add `check_version`/`check_party` pins for all twenty at two sizes, correcting the six closed forms above (spell the gamma-length sums out as `hole_region_bits` already does; `staircase` is exactly `6d + 2`) and fixing the capacity hints; then add one roster-wide pin that walks every `Shape` variant through the matching `check_*` so a new constructor cannot enter `Shape::builder` unpinned. Consider additionally routing `Packed::version()` through `Version::decode` of the transcoded bytes under `cfg(any(test, feature = "meter"))`, since construction happens before counters are reset and the only cost is one validation pass per shape. Acceptance: every arm of `Shape::builder` names a generator that appears in a meter/tests.rs pin asserting its `bits` against a closed form and round-tripping the shape; the construction below fails a committed test; the module doc's sentence at 29-32 is true of every variant.

Construction: Change `memo_chain`'s range leaf at meter.rs:1005 from `if distinct { j as u64 } else { 1 }` to emit `0` in the shared case, so every site is the equal sibling pair `(0, 0)`. `cargo nextest run -p before --lib meter::tests` stays green (nothing calls `memo_chain`), and the `memo_chain_shared_*` rows in tests/meter.rs and the fill differential at fill/tests.rs:140 build and measure the stream, which `Version::decode` would reject. Apply the same mutation to `dense` (make the bottom pair `(0, 0)`) and `dense_decodes_canonically_at_predicted_length` fails inside `check_version`, showing the pin is what catches it. For the closed forms, `assert_eq!(wide_tail(1, 1).bits, 4 + 2 + 3)` fails with 8; `assert_eq!(memo_chain(1, false).bits, 13 + 9)` fails with 16; `assert_eq!(memo_comb_id(1).bits, 14 + 12)` fails with 22; `assert_eq!(memo_churn_id(1).bits, 14 + 6)` fails with 14; `assert_eq!(staircase(7).bits, 5 * 7)` fails with 44.

### meter-core-13: The pointwise-dominance premise of two rank bands is witnessed by rank order and by the rank fold itself, not by the comparison sweep
- Where: crates/before/src/meter/tests.rs:297-307 (related: meter/tests.rs:940-950, 1490-1494; version.rs:309-311, 397-405, 428-436, 1730-1732)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (dispatch read: `Version::rank` and `Version::lag` route to `skyline::query::rank`/`lag`, both `Integrator` folds; `partial_cmp` routes to `skyline::sweep::causal_cmp`, a separate kernel; the `lag` contract at version.rs:402-403 makes `lag == ZERO` a correct pointwise witness); executed: no
- Seen by: adequacy; refutation: confirmed; history: no rationale found
- Owner-gated: no

Adequacy: a premise check that shares its kernel with the instrument it underwrites can be wrong in lockstep with it. The `promotion_rearm_mate` and `dense_suffix_mate` pins document that the wide operand dominates its mate pointwise and witness it with `a.rank().checked_sub(&b.rank()).is_some()` (rank order, strictly weaker than dominance, under a message that claims dominance) plus `a.lag(&b) == Rank::ZERO` (sufficient, but computed by the same query fold whose bands rest on the premise). The `tooth_tail` pin does it the independent way, `partial_cmp == Some(Less)` through the comparison sweep.

Evidence:

       297	    let a = promotion_rearm(3).version();
       298	    let b = promotion_rearm_mate(3).version();
       299	    assert!(
       300	        a.rank().checked_sub(&b.rank()).is_some(),
       301	        "the re-arm spine dominates its mate"
       302	    );
       303	    assert_eq!(
       304	        a.lag(&b),
       305	        crate::Rank::ZERO,
       306	        "the dominating side lags by nothing"
       307	    );

Resolution: Replace the `checked_sub` assertion in both tests with `assert_eq!(a.partial_cmp(&b), Some(Ordering::Greater), ..)`; keep `lag == ZERO` if the rank identity is wanted as a second leg, named as such. Acceptance: both tests witness dominance through `partial_cmp`; no message claims dominance over a rank-order check.

### meter-core-14: The seam-plunge control's wire-prefix check contradicts its comment and carries an underived 200-bit slack
- Where: crates/before/src/meter/tests.rs:1083-1104 (related: meter/tests.rs:1060-1067; meter.rs:2753-2767; signed.rs:129-137; gamma.rs:25-28)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (comment and code read; the control's final stored code derived by hand: leaf flag plus `gamma(zigzag(+5·2^64))`, `zigzag(+k) = 2k = 5·2^65`, `m = 5·2^65 + 1` has bit length 68, so `2·68 − 1 = 135` bits plus the flag is 136; with at most 7 padding bits and one partial divergence byte the derived bound is about 151 bits, leaving about 49 of the 200 unexplained); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no rationale found (comment and code born together in d0501cd7)
- Owner-gated: no

Comments state what the code cannot show; here the comment states what the code does not do. The test doc says the control's wire differs from `seam_plunge`'s "only in the final delta code" and the comment says the identity is "asserted structurally instead of byte-wise here, by the leaf heights"; the code counts equal leading bytes of the two wires and accepts any divergence within 200 bits of the control's end, a byte-wise check weaker than the doc's claim with a magic bound. The seam band's reading of the pair's difference as the plunge's own propagation rests on this leg (1064-1067).

Evidence:

      1083	        // The wire-prefix identity: the pair's stored streams agree byte for
      1084	        // byte up to the control's full length less its final code and
      1085	        // padding — asserted structurally instead of byte-wise here, by the
      1086	        // leaf heights: both trees walk the identical ascent.
      1094	        let shared = control_wire
      1095	            .iter()
      1096	            .zip(plunge_wire.iter())
      1097	            .take_while(|(c, p)| c == p)
      1098	            .count();
      1099	        assert!(
      1100	            shared * 8 + 200 >= control_wire.len() * 8,

Resolution: Either check the identity exactly (bit-level prefix equality up to the control stream's live length less its final code, via `BitsView`), or keep the byte-prefix check with a named, derived slack (`SEAM_CONTROL_FINAL_CODE_BITS = 1 + 2 * 68 - 1` plus a stated padding-and-divergence allowance) and rewrite the doc and comment to describe the check performed. Acceptance: the assertion's bound is a named constant with its derivation beside it; the test doc, the comment, and the code describe the same check; the test passes at the three `(k, r)` points.

**Family registry and tier-2 sizer**

### meter-registry-tier2-7: Weight-comb, freeze-parade, and tooth-tail bands cite uncommitted probe builds as their only adequacy witness
- Where: crates/before/src/meter/registry.rs:769-774 (related: registry.rs:784-786; crates/before/tests/meter.rs:4479-4498, 4544-4555, 4652-4665, 4737-4747; crates/before/tests/superlinear_tripwires.rs:27-68; crates/suanpan/src/accumulator/tests/metered.rs:8-11)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rnE 'fn [a-z_0-9]+_reads_superlinear'` over crates/before and crates/suanpan lists ten kernels, none on weight_comb, freeze_parade, or tooth_tail; the `TRIPWIRE_ROSTER` at superlinear_tripwires.rs:27-68 read; `grep -rn 'probe build'` hits registry.rs:770, 785 and tests/meter.rs:4489, 4551, 4660, 4742; suanpan's one committed known-bad is `no_collapse_fold_re_scans_the_prefix`, metered.rs:8-11); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed; history: no rationale found (round #85 recorded kill-switch probe builds; f10f5b56 later made rostered committed kernels the crate's standard; 500d4d09 kept the wording)
- Owner-gated: no
- Witness (witness/results.md): inconclusive. The construction as written cannot pass: the weight-comb band's semantic leg at tests/meter.rs:4511-4519 pins `min_ticks` to the closed form 34n - 1, and dropping the parked-unit spine changes the base sum, so the band would fail on the semantic leg rather than stay green; a degradation preserving the base sum needs a redesigned generator. The textual half (uncommitted probe builds cited at registry.rs:770-771, 784-785, and tests/meter.rs:4551; no committed known-bad kernel for the three bands) was verified by reading.
- Cross-references: envelopes-a-22, meter-adequacy-6 (the probe-build cluster).

The WeightComb and FreezeParade rows state that the known-bad mechanism was "demonstrated by a probe build", and the band docs in tests/meter.rs say the same for weight-comb, freeze-parade, and tooth-tail ("a local probe build"). No committed kernel fails on any of the three; every sibling rank band (freeze-position, rearm, dense-suffix, wide-arming, plateau-puncture, shade) has a `_reads_superlinear` kernel in the roster. Doctrine: every criterion needs a committed demonstration that a known-bad mechanism fails it; a demonstration that lives in someone's local tree is a told number with no run bound to it (Principle 8). If the zero-run certificate, the write watermark, or exact-top maintenance quietly stopped being what the family reaches, all three bands stay green and nothing committed notices.

Evidence:

       769	    /// never-written run. A settlement scan that steps the gap digit by digit
       770	    /// goes quadratic here (demonstrated by a probe build with certificate
       771	    /// consumption disabled); consuming one
       772	    /// zero-run certificate per jumped run reads flat (the `skyline_flatness`
       773	    /// weight-comb band). Designed against the linear-functional query rows.
       774	    WeightComb,
       784	    /// quadratic in the touch and limb currencies together (demonstrated by
       785	    /// a probe build whose scaled reads
       786	    /// start at digit 0); the watermark reads flat (the `skyline_flatness`

    tests/meter.rs:
      4551	    /// consumption disabled (a local probe build whose scans step
      4552	    /// digit by digit), the reading goes quadratic — `n² + O(n)`

Resolution: commit the three refuted mechanisms as kernels following the crate's own pattern (test-local variants in the query suite or suanpan's metered suite, mirroring `no_collapse_fold_re_scans_the_prefix`): a settle that steps the gap digit by digit, asserted superlinear across `WC(n) -> WC(2n)`; a scaled read starting at digit 0 (the plain `sign_magnitude` suanpan's metered.rs:20-23 already names as the full-held-width read), asserted superlinear across `FZ(k) -> FZ(2k)`; a high-water-bounded sign read on the tooth-tail pair. Add each to `TRIPWIRE_ROSTER`, then replace "demonstrated by a probe build" at registry.rs:770-771 and 784-786 and "a local probe build" at tests/meter.rs:4489, 4551, 4660, 4742 with the kernel names. The nearest existing mitigation, suanpan's exact-count row pins (`alternating_shifted_writes_cost_the_operand_not_the_gap`, `scaled_read_costs_the_written_span`, `held_width_rows_cost_the_held_digits`), holds the shipped mechanisms crate-locally but demonstrates no known-bad failing. Acceptance: the roster gains three entries, each reads red on its family at both scales, and `grep -rn 'probe build' crates/before crates/suanpan` returns nothing.
Construction: degrade `weight_comb` so no never-written gap exists (drop the parked-unit spine so the oscillation lands at digit 0); `skyline_rank_weight_comb_is_flat_per_unit` stays green (flat, under its absolute ceilings, above its `2n` nonzero-delta floor) and no committed test reports that the family has stopped exercising the zero-run ledger. A committed per-digit settle kernel would read flat on the degraded family and fail its floor, exposing the dark family; today nothing does.

Synthesis note: The witness invalidates the entry's Construction paragraph, not its claim: the semantic leg would catch the proposed degradation, so the dark-family demonstration needs a shape that keeps the base sum while removing the never-written gap. The gap the entry names (no committed kernel reads red on these three bands) stands by reading.

### meter-registry-tier2-11: `FamilyId::ALL`, the 52-arm `index()`, and `ALL_SHAPES` are hand rosters checked only against each other; a variant absent from `ALL` reaches no instrument and `index()`'s doc claims the opposite
- Where: crates/before/src/meter/registry.rs:1115-1228 (related: registry.rs:569-573, 1232-1236; crates/before/src/meter/registry/tests.rs:8-13, 84-97; crates/before/tests/amp_board_smoke.rs:363; crates/before/tests/verdict_matrix.rs:468, 1296; crates/before/src/meter/board/family.rs:765-780; crates/before/src/meter/board/ops.rs:164-182)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rn '\.index()'` over crates/before and before-fuelscape: readers only at registry/tests.rs:91, 94; every `FamilyId::ALL` reader listed by grep iterates the array (registry/tests.rs:89, 103, 118, 137, 195, 208; amp_board_smoke.rs:363; verdict_matrix.rs:468, 1296; `board()` at 1233); awk over the enum bodies counts 52 `FamilyId` and 68 `Shape` variants against `[FamilyId; 52]` and `[Shape; 68]`; strum 0.28 is in Cargo.lock (2377) only transitively); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: already known in part (b3f09baa wrote the "found by luck" concession at 572-573 and the `ALL_SHAPES` caveat at tests.rs:10-12 in the same commit that reworded `index()`'s doc to the opposite claim); no derive or macro was ever weighed
- Owner-gated: yes for the mechanism (`index()` is `pub const fn` under the `meter` feature and a derive adds a direct dependency); correcting the false sentence at 1170-1172 is unconditional
- Witness (witness/results.md): demonstrated (run). A `FamilyId::Probe` variant with an `index()` arm of 52, a `spec()` row, and the compiler-forced or-pattern arms, but no entry in `FamilyId::ALL`, compiles without a warning and passes every registry test plus the verdict matrix's coverage census. One inventory correction: tests/verdict_matrix.rs:162-412 (`matrix_operands`) also matches `FamilyId` exhaustively, so a new variant needs an arm there too; that arm is compiler-forced and makes no roster test fire.
- Cross-references: fuzzfit-strategies-8, fuzzfit-bands-30, surface-roster-3 (the hand-roster pattern).

`index()`'s doc says the roster-order tie means "a variant cannot be declared without joining the roster at a committed position", but `roster_order_is_committed` iterates `FamilyId::ALL` only, so a variant with `spec()` and `index()` arms (and the compiler-forced or-pattern arms in board/family.rs and board/ops.rs) and no `ALL` entry passes every registry test and is never seen by the board, the band parity pin, or the verdict matrix, all of which iterate `ALL`. `ALL: [FamilyId; 52]` fixes its own length; `index()` has no reader but its pin; `ALL_SHAPES` in the tests has the same hole and admits it. Principle 6 (the module's stated invariant is that every family exists inside every instrument's coverage, yet the one enumeration every instrument derives from is a hand list nothing checks for completeness) and Principle 3 (`index()` exists to be checked against `ALL` and nothing else). The doctrine prefers a dependency over hand-rolling this.

Evidence:

      1115	    pub const ALL: [FamilyId; 52] = [
      1170	    /// This family's position in [`FamilyId::ALL`] — the roster-order tie the
      1171	    /// registry tests hold against the array, so a variant cannot be declared
      1172	    /// without joining the roster at a committed position.
      1173	    pub const fn index(self) -> usize {

    registry/tests.rs:
        10	/// Completeness rides the same review discipline as [`FamilyId::ALL`]: a
        11	/// variant missing here escapes only the citation pin, never the compiler ties
        12	/// (its constructor arm in `Shape::builder` is still forced).

Resolution: (1) unconditional: fix or delete the sentence at 1170-1172, and reduce `index()` to `self as usize` (both enums are fieldless with declaration order equal to roster order, verified) so `roster_order_is_committed` pins `ALL[i] as usize == i` without a 52-arm match. (2) owner-gated: derive variant enumeration (`#[derive(strum::VariantArray)]` on `FamilyId` and `Shape`, so `FamilyId::VARIANTS` replaces `ALL`, `Shape::VARIANTS` replaces `ALL_SHAPES`, `index()` and its pin dissolve, and the "found by luck" list at 572-573 loses its first entry), or a local declaration macro producing the enum and its array from one list if a new dependency is unwanted. Acceptance: appending a variant to either enum without touching any roster either fails to compile or fails a committed test that enumerates variants totally; `grep -n 'fn index' crates/before/src/meter/registry.rs` finds nothing (or the surviving pin is a name snapshot); registry/tests.rs no longer carries the completeness caveat.
Construction: add `FamilyId::Probe` after `LatentLadder` with `index() => 52`, a `spec()` arm copying LatentLadder's row with `name: "probe"` and `Bands::Unbanded`, and the or-pattern arms in board/family.rs:765-780 and board/ops.rs:164-182 extended; leave `ALL` untouched. `roster_order_is_committed`, `family_names_are_unique`, `every_shape_is_cited_by_a_family`, `board_roster_derives_from_coverage_answers`, `envelope_only_rulings_are_dated`, and `band_tests_and_registry_citations_stay_paired` all pass, and `FamilyId::board()` never yields the variant.

### meter-registry-tier2-3: Band-name parity is keyed on a naming convention; seven two-point flatness tests escape it
- Where: crates/before/src/meter/registry.rs:66-71 (related: crates/before/tests/amp_board_smoke.rs:306-340; crates/before/tests/meter.rs:6354, 6378, 6623, 6637, 6650, 6670, 6695; registry.rs:1663-1664, 1680-1681)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read `band_test_names` at amp_board_smoke.rs:314-340; `grep -c 'assert_flat(' tests/meter.rs` = 68 against 49 convention-named fns; the seven escapees each sit under `#[test]`, read at lines 6353, 6377, 6622, 6636, 6649, 6669, 6694); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: the name-keyed scan was deliberate (bf2221554, a44502fe) but neither commit nor the registry doc mentions the two-point tests outside the convention, which predate it
- Owner-gated: no
- Cross-references: envelopes-b-22 (the thirteen unrostered pins; the seven counted here overlap that set).

The registry says the smoke suite "holds them equal, name for name" to the citation rosters, but the scanner recognizes only names containing `_is_flat_per_unit` or ending in `_band`, so a two-point `assert_flat` test with any other name needs no registry answer. Seven exist today (`id_covers_scan_cost_is_pinned_and_flat`, `id_disjoint_scan_cost_is_pinned_and_flat`, and the five `accum_*_touches_flat`). Principle 6: the parity survivor's totality is over the convention, not over the band mechanism, and this is how the cliff-fan and cancelling-chain rows came to name the wrong enforcement home (finding 10): their two-point pins never had to be cited.

Evidence:

        66	//! - **Band names.** The bands live in a separate test binary
        67	//!   (`tests/meter.rs`), and test function names are not items the
        68	//!   compiler can resolve across crates. The board smoke suite
        69	//!   (`tests/amp_board_smoke.rs`) scans that suite's band-named tests
        70	//!   and holds them equal, name for name, to the union of the specs'
        71	//!   [`Bands`] rosters and [`AXIS_BANDS`].

    amp_board_smoke.rs:
       331	                    if name.contains("_is_flat_per_unit") || name.ends_with("_band") {

Resolution: either key the scan on the mechanism (collect every `#[test]` fn whose body calls `assert_flat`/`assert_model_flat`) or rename the seven tests into the convention and cite them (the accumulator streams on their families' rows, the id scan pair in `AXIS_BANDS`); state in the module doc that the scan is total over the convention. Acceptance: adding `#[test] fn scratch_touches_flat()` with an `assert_flat` call and no registry citation fails `band_tests_and_registry_citations_stay_paired`.
Construction: add to tests/meter.rs `#[test] fn scratch_touches_flat() { let s = comb_run(4_096, 50_000); let l = comb_run(8_192, 100_000); assert_flat("scratch", &s, &l, envelope::COMB_MILLI_PER_DELTA); }` with no registry change; the parity test passes.

### meter-registry-tier2-19: Lib-side tests reset and read the process-global limb counter with nothing enforcing the one-scenario-per-process premise
- Where: crates/before/src/meter/tier2/tests.rs:247-253 (related: crates/before/src/codec/base/limb_meter.rs:16-19, 31; crates/before/tests/meter.rs:354; the other reset sites: meter.rs, party/tests.rs, codec/dsi/tests.rs, testing/asymptotics.rs, version/skyline/query/tests.rs, meter/tests.rs, meter/board/tests.rs, meter/board/measure.rs)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read limb_meter.rs:1-52: `static LIMB_OPS: AtomicU64` with `Relaxed` ordering and the premise stated at 16-19; `grep -rn NEXTEST crates tools justfile .github .config` finds no guard; `grep -rln 'reset_limb_ops\|reset_touches\|reset_scan'` lists nine lib-side files; only tests/meter.rs carries `ISOLATION_NOTE`); executed: no
- Seen by: instrument-correctness; refutation: reframed (a crate-level observation; this test is one instance); history: the premise is deliberate (limb_meter.rs) and 380d470ea established the isolation-note convention for tests/meter.rs and src/meter/tests.rs; the plain-sweep test landed hours later without it
- Owner-gated: yes (where the premise is enforced, a runtime guard on the runner or an AGENTS.md rule, is a crate-level test-policy decision)
- Cross-references: envelopes-a-3, envelopes-b-10, suite-economics-1 (the isolation cluster).

`LIMB_OPS` is process-global and its doc premises that "the metering binaries run one scenario per process"; the plain-sweep witness resets and reads it inside the `before` lib test binary. Under nextest (the configured runner) the premise holds; under `cargo test` any concurrent `Base` arithmetic bleeds into the delta and the 1.8 ratio can move either way, and for the ceiling-style pins elsewhere a dark counter can be masked by another thread's work. The premise is stated but nowhere enforced, and the lib-side sites carry no isolation note.

Evidence:

       247	        crate::meter::reset_limb_ops();
       248	        for i in 1..(2 * n) {
       249	            v = if i % 2 == 1 { &v + &one } else { v - &one };
       250	        }
       251	        let closing = Base::from(1u8) << k as u32;
       252	        v -= &closing;
       253	        let ops = crate::meter::limb_ops();

    limb_meter.rs:
        16	//! envelopes read continuously across that arm seam. Relaxed ordering
        17	//! suffices: the metering binaries run one
        18	//! scenario per process and read the counters only after the metered call
        19	//! returns.

Resolution: add a `meter::require_process_isolation()` that asserts `NEXTEST_EXECUTION_MODE == "process-per-test"` (nextest exports it) with the isolation note as its message, called from the reset entries under `cfg(test)`; or name the premise as a hard rule in `crates/before/AGENTS.md` and accept the unguarded state explicitly. Acceptance: `cargo test -p before --lib --features limb-meter` fails fast with the isolation message at the first counter reset, while `just test-all` is unaffected; or the AGENTS.md rule exists.
Construction: run `cargo test -p before --lib --features limb-meter -- cliff_comb_plain_delta_sweep skyline` with default test threads; `Base` arithmetic from the skyline tests lands between `reset_limb_ops()` and `limb_ops()`, and the measured `small`/`large` vary run to run.

### meter-registry-tier2-13: A tautological board-roster equality and a weak, duplicated date predicate with an under-stated doc
- Where: crates/before/src/meter/registry/tests.rs:176-202 (related: registry/tests.rs:204-226; crates/before/src/meter/registry.rs:1232-1236; crates/before/surfacecheck/src/check.rs:259-273)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read `board()` at registry.rs:1232-1236 and both tests); executed: no
- Seen by: adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: born tautological at cc84df7e (`board()` was already the same filter); surfacecheck's stricter date check landed the next day and was never back-ported
- Owner-gated: no

`board_roster_derives_from_coverage_answers` compares `FamilyId::board().count()` against `ALL.iter().filter(|f| matches!(f.spec().coverage, Coverage::Board { .. })).count()`, which is the literal body of `board()`; the `EnvelopeOnly` panic arm inside the loop is unreachable by that filter. The live assertions are `cells > 0` per column and `columns > 0`. In `envelope_only_rulings_are_dated` the date predicate is written twice (215, 221), admits any ten characters with two dashes, and the doc ("Every envelope-only ruling carries a dated, non-empty reason") omits the `Bands::Unbanded` clause the body also checks. Doctrine: recompute-and-compare on a pure function with the same expression is not defense in depth; a test doc must state what the body checks.

Evidence:

       193	    assert_eq!(
       194	        columns,
       195	        FamilyId::ALL
       196	            .iter()
       197	            .filter(|f| matches!(f.spec().coverage, Coverage::Board { .. }))
       198	            .count(),
       199	        "the board roster and the coverage answers disagree"
       200	    );

Resolution: rename the first test to what it holds (`board_columns_declare_nonzero_reach`), drop the equality and the unreachable arm; extract one `is_iso_date` predicate (or lift surfacecheck's month/day check) and fix the second test's doc to name both ruling kinds. If finding 9 resolves by deleting `decided`, the date test goes with it. Acceptance: no assertion in registry/tests.rs restates a registry function's body; one date predicate; each test doc matches its body.

**Board frame**

### board-frame-3: The "four cells no deterministic leg watches" disclosure is a prose-only roster; nothing pins the all-NA cell set
- Where: crates/before/src/meter/board.rs:99-100 (related: crates/before/src/meter/board/floors.rs:99-112, crates/before/src/meter/board/tests.rs, crates/before/tests/amp_board_smoke.rs)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep of `NotApplicable|neither leg|all.NA|unwatched` over board/tests.rs and tests/amp_board_smoke.rs finds only the bypass test's doc at tests.rs:403, which builds synthetic samples rather than enumerating cells); executed: no
- Seen by: scaffolding (and the count half of structure-prose's and instrument-correctness's board.rs items); refutation: confirmed; history: no-rationale-found (the prose disclosure was deliberate in 319afc262; the absence of a pin was never considered; the count "four" is from b3f09baa0, a commit titled "Partial WIP for docs pass")
- Owner-gated: no
- Cross-references: board-families-floors-judge-11 (the same roster from floors.rs), meter-adequacy-4.

board.rs:99-100 and floors.rs:99-101 state, as a count and a name list, which cells every floor column declares not-applicable; no test enumerates cells whose `Floors` are all `Liveness::NotApplicable`. A new all-NA row or a floor quietly changed to NA moves nothing but the truth of that sentence (Principle 5: a number that matters lives in a mechanically enforced place the prose cites by name; Principle 6's tamper-evidence form).

Evidence:

        99	//! constructors that commit them, along with the disclosure of the four cells
       100	//! no deterministic leg watches.

    [floors.rs:99-101]
        99	//! Four cells are watched by neither leg, an exposure accepted here so it is
       100	//! stated rather than silent: `version_hash`, `party_hash`, `clock_hash`, and
       101	//! `version_eq` on the benign family. Hashing folds the stored canonical bytes

Resolution: Add a board/tests.rs test that prepares `ops()` × `FamilyId::board()` at the smoke scale, collects the cells whose `floors.each()` are all `NotApplicable`, and asserts the sorted `(op, family)` set equals a committed list (the hash rows on each family plus `version_eq` on the benign family, or whatever the enumeration shows); have board.rs:99-100 and floors.rs:99 cite the test by name and drop the count, noting which listed cells the time leg's 10 µs floor also excludes. Acceptance: changing any one cell's floor to NA, or adding an all-NA row, fails exactly one named test; the prose carries no count.
Construction: Change `version_encode`'s heap floor in floors.rs to `na(...)`; the board's unit suite stays green while floors.rs:99 names four cells and five (or more) are all-NA.

**Board families, floors, judge**

### board-families-floors-judge-21: The heap exponent is fitted on allowance-inclusive readings, so a quadratic heap term of allowance-scale magnitude passes acceptance
- Where: crates/before/src/meter/board/judge.rs:132-147 (related: judge.rs:19-29, 100-108, 256-259; board/tests.rs:708-753; ceilings.rs:69, 73, 78; board.rs:214-222; lib.rs's space guarantees)
- Class / severity / confidence: verification-gap / high / high
- Provenance: verified (I transcribed `trend` (37-54) and the heap branch of `fit_currency` (125-147) into python and ran the construction: points (4096, 9011), (8192, 11468), (16384, 21299), (32768, 60620) all clear `HEAP_FLAT_ALLOWANCE_BYTES = 8192` and span x8; raw four-point slope 0.914 (< 1.15, green); top window alone 1.509 (red under `evaluate`); residual `(m - 8192)` slope 2.000; constants 0.20, 0.40, 0.80, 1.60 B/B against a 16.0 ceiling; the existing `over_allowance` probe reads 6.03 under a residual fit and `straddling` keeps one cleared point. No cargo command run); executed: no (a python transcription of the Rust estimator, not the Rust code)
- Seen by: adequacy; refutation: confirmed (independent re-transcription reproduced every number); history: no-rationale-found (the exclusion filter was reasoned twice, f68802c3 and 1a5e57e5, purely against false red; fitting the residual was never considered; 9e36dd28's "still reads red" was asserted, not demonstrated against a quadratic of sub-allowance magnitude)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run as a unit test appended to board/tests.rs, then removed). Readings 8192 + n^2/20480 over 4096..32768 give a four-point raw trend of 0.914 (green under `MAX_SCALING_EXPONENT` 1.15), 2.000 on the allowance-subtracted residuals, 1.509 on the top window's two points; `evaluate_acceptance` reports no red in either window (heap constants 0.40 and 1.60 B/B) while `evaluate` on the top window alone reports "heap exponent".

The heap exponent leg fits `trend` over the raw readings (points that merely clear `HEAP_FLAT_ALLOWANCE_BYTES`), while the constant leg judges `m2 - allowance`; the un-subtracted flat term deflates the log-log slope, so a peak heap of `A + n^2/20480` bytes over the acceptance ladder 4096..32768 reads a four-point trend of 0.914 and constants of 0.2-1.6 B/B, green in both windows of `evaluate_acceptance`, even though the top window's own two-point fit reads 1.51 and the residual fit reads exactly 2.0. The verdict of record is weaker than the bare debugging view on this artifact, and the docstring's promise that "a genuine super-linearity bends every point and still reads red" is falsified by construction. The crate docs make space claims hard guarantees, and the board is the one instrument pinning heap order across the op x family product; the worst artifact that passes is a Theta(n^2) heap term whose magnitude at the board's KiB-scale sizes sits within a few multiples of the 8 KiB allowance (Principles 2 and 6).

Evidence:

       132	        let cleared: Vec<(usize, u64)> = points
       133	            .iter()
       134	            .copied()
       135	            .filter(|&(_, m)| m > HEAP_FLAT_ALLOWANCE_BYTES as u64)
       136	            .collect();
       137	        return if cleared.len() >= 2 && spans(&cleared) {
       138	            Fit {
       139	                exp: Some(trend(&cleared)),
       140	                judged: true,
       141	            }

       256	        let per_unit = match c {
       257	            Currency::Heap => {
       258	                m2.saturating_sub(HEAP_FLAT_ALLOWANCE_BYTES as u64) as f64 / s2.denom_bytes as f64
       259	            }

        25	/// single generator lump at one point cannot define the estimate, while a
        26	/// genuine super-linearity bends every point and still reads red. Densifying

Resolution: Fit the heap trend over the same quantity the constant leg judges, `m.saturating_sub(HEAP_FLAT_ALLOWANCE_BYTES as u64).max(1)` on the cleared points, keeping the `cleared.len() >= 2 && spans` guards, and add a materiality guard so a residual of a few bytes above the allowance cannot manufacture an exponent (for example, each cleared residual at least a stated fraction of the allowance, derived and documented beside `HEAP_FLAT_ALLOWANCE_BYTES`). Commit the construction below as a tripwire beside `exponent_guards_skip_noise_and_keep_real_amplifiers_red`. Under the residual fit the existing `over_allowance` probe stays red and `straddling` stays unjudged, so the committed pins survive. Acceptance: a committed judge test builds four `Sample`s with `heap: Some(8192 + n*n/20480)` at n = 4096, 8192, 16384, 32768 (other columns NA or zero) and asserts `evaluate_acceptance` returns "heap exponent" in `red` for both windows; the existing judge tests stay green; `just amp-board-acceptance` stays green on the committed board (any new red is a finding to triage).
Construction: In board/tests.rs, reuse the all-NA `Sample` helper shape from `acceptance_trend_absorbs_lumps_and_keeps_amplifiers_red` (776-801) with `readings.heap = Some((HEAP_FLAT_ALLOWANCE_BYTES + n*n/20480) as u64)`, `limb: None`, and `exp_denom_bytes = denom_bytes = n`; call `evaluate_acceptance("heap_probe", "quadratic-under-allowance", (sample(4096), sample(8192)), (sample(16384), sample(32768)))` and assert both windows' `red` is empty today (the demonstration) versus containing "heap exponent" after the fix. For contrast, `evaluate("heap_probe", "top-window", sample(16384), sample(32768))` reads "heap exponent" red today: the debugging view catches what the verdict of record does not.

### board-families-floors-judge-10: Not-applicable declarations whose rendered reasons the code contradicts, leaving `clock_fork` unwatched on every version-only family
- Where: crates/before/src/meter/board/floors.rs:60-64 (related: floors.rs:143-145, 207-221, 289-296, 302-319; ops.rs:1306-1340, 1608-1626; family.rs:1058-1059; version/skyline/validate.rs:80-110; version/skyline/signed.rs:203-210; suanpan/src/accumulator.rs:219-226, 1253-1266, 1369-1373; suanpan/src/touch_meter.rs:9-12; party.rs:121-130, 233-237; party/ops/split.rs:20-27; idbits.rs:84-90, 132-140; clock.rs:153-157)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read every call path named below; no cargo run); executed: no
- Seen by: adequacy, structure-prose, instrument-correctness; refutation: confirmed (and extended: WHY_TOUCH_WIDE_STREAM carries the same false clause); history: no-rationale-found (the decode-row NA predates the quick register; NA_SCAN_SEED_PARTY was false at introduction; the two fork rows have carried different genres for one mechanism since 1a5e57e5)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run through the public API). (a) `Version::decode` of word-scale streams touches accumulator digits: 3 touches on dense(1000) and 193 on cliff_comb(20,64), contradicting `NA_TOUCH_LAZY_BATCH`. (b) `Party::seed().fork()` records 2 scan bits and the seed's packed form is one byte `[32]`, so `NA_SCAN_SEED_PARTY` is false while `scan_touch()` is met exactly. (c) `Clock::fork` on a version-only clock raises the peak heap by 2 bytes while ops.rs:1613 declares `clock_fork`'s heap NA. The `>= stored_nonzero_deltas` comparison was not reproduced (that function is `pub(super)`).

Three floor sites declare not-applicable on the strength of a mechanism claim the code contradicts, so a column (or a whole cell) goes unwatched where the row's own deterministic-liveness genre yields a positive floor. (a) The decode rows' touch: the module doc, NA_TOUCH_LAZY_BATCH, and the parenthetical inside the rendered WHY_TOUCH_WIDE_STREAM say word-scale deltas "batch in the accumulator's lazy zone and force no digit touches"; but `validate_from` folds every leaf after the first through `fold_signed_int`, which routes word-scale values to `add_u64`/`sub_u64`; those skip only `delta == 0` and otherwise call `quick_add` (`touch(1)`) or `add_at` (`touch(1)` per digit), and touch_meter.rs documents the register absorb as exactly one touch. Every nonzero word-scale delta therefore records at least one touch, and the floor the tick and query rows already commit (`touch_delta_fold(stored_nonzero_deltas(v))`) binds on `version_decode`/`clock_decode` too; on every all-narrow family they declare NA instead. (b) The seed party's scan: NA_SCAN_SEED_PARTY says "its packed form is empty", but `Party::seed()` is the 2-bit terminal tag `00` in one static byte, and `Party::fork` -> `view().split()` -> `IdReader::split` begins with `self.peek()`, which for a nonempty view records 2 scan bits; `scan_touch()` (2 bits) is met exactly. The empty stream is the anonymous id, never a `Party`. (c) `clock_fork`'s heap: NA_HEAP_FORK_SHARES says "no other allocation is semantically forced", but `Clock::fork` calls `self.party.fork()`, the same party-half materialization `party_fork` floors as WHY_HEAP_FORK_HALF; one mechanism carries two genres across two rows. Consequence of (b)+(c): `clock_fork` on every version-only family (where `FamilyData::clock` pairs the seed party, family.rs:1058-1059) is NA on all five columns and is absent from the exposure disclosure at floors.rs:99-106 (finding 11). The floors module's own rule (lines 5-7) is that a floor states the least a watching counter can read and an NA is sound only when no floor can bind; instruments before cures.

Evidence:

        60	//!   under the same premise). The validator batches word-scale deltas in the accumulator's
        61	//!   lazy zone, so the decode rows floor only what it must fold digit by
        62	//!   digit: one touch per 64 bits of every stored code wider than the
        63	//!   machine-word bound (the stream-derived
        64	//!   convention the tick rows' limb floor uses). Either floor is what a

       295	const NA_TOUCH_LAZY_BATCH: &str = "every stored code fits the machine-word bound: word-scale \
       296	     deltas batch in the accumulator's lazy zone and force no digit touches";

       144	pub(super) const NA_SCAN_SEED_PARTY: &str =
       145	    "the forked party is the seed: its packed form is empty";

       213	pub(super) const NA_HEAP_FORK_SHARES: &str = "the forked child's version hand-over is a \
       214	     refcount bump on the shared stored buffer, never a byte copy, and no other allocation \
       215	     is semantically forced";

    validate.rs:
       100	        if seen_leaf {
       101	            zero_delta = code.is_zero();
       102	            let (sign, magnitude) = unzigzag(code);
       103	            fold_signed_int(&mut height, sign, &magnitude);

    signed.rs:
       205	        (Sign::Positive, Int::Small(n)) => acc.add_u64(*n),
       206	        (Sign::Negative, Int::Small(n)) => acc.sub_u64(*n),

    suanpan accumulator.rs:
       219	    pub fn add_u64(&mut self, delta: u64) {
       220	        if delta != 0 {
       221	            let delta = i128::from(delta);
       222	            if !self.quick_add(delta) {
       223	                self.add_at(0, delta);
      1253	    fn quick_add(&mut self, delta: i128) -> bool {
      1254	        let Some(held) = self.quick else {
      1255	            return false;
      1256	        };
      1257	        touch(1);

    touch_meter.rs:
         9	//! unit every cost on the crate page is denominated in. The quick
        10	//! register meters too, though it holds no digits: a delta, sign query,
        11	//! negation, or shift the register absorbs counts exactly one touch,

    party.rs:
       122	        // The seed id is exactly the 2-bit terminal tag `00` (the whole
       123	        // interval, owned), marker-padded to the one static byte
       124	        // `0b0010_0000`: construction allocates nothing, and every seed

    split.rs:
        22	        if let IdNode::Empty = self.peek() {

    idbits.rs:
       135	            IdReader::At { bits, pos } => {
       136	                crate::codec::scan::record_bits(2); // one 2-bit tag scanned

    clock.rs:
       153	    pub fn fork(&mut self) -> Clock {
       154	        let child_party = self.party.fork();

    ops.rs:
      1612	                let floors = Floors {
      1613	                    heap: na(NA_HEAP_FORK_SHARES),
      1614	                    limb: na(NA_LIMB_NOT_FORCED),
      1615	                    segments: seg_ceiling_only(),
      1616	                    scan: if clock.party().is_seed() {
      1617	                        na(NA_SCAN_SEED_PARTY)
      1618	                    } else {
      1619	                        scan_touch()
      1620	                    },
      1621	                    touch: na(NA_TOUCH_NOT_FORCED),

Resolution: (a) Floor the decode rows' touch at the max of `touch_delta_fold(stored_nonzero_deltas(v))` and the wide-stream limb count, and reword lines 60-64, NA_TOUCH_LAZY_BATCH, and the clause inside WHY_TOUCH_WIDE_STREAM to the meter's actual accounting (a zero delta is skipped; a nonzero word-scale delta costs one register or digit touch). (b) Replace NA_SCAN_SEED_PARTY with `scan_touch()` at ops.rs:1329-1333 and 1616-1620 and delete the constant. (c) Give `clock_fork` the party half's WHY_HEAP_FORK_HALF floor as `party_fork` does (the child's packed bytes, probed at prepare), or move both rows to one genre with the reason stated. Acceptance: with the counter features, a probe that decodes `dense(1_000)`'s bytes with the touch counter reset reads `touches() >= stored_nonzero_deltas(&v)`, and a `Sample` with `touch: Some(0)` under the new floor reads TOUCH_FLOOR_TRIP through `evaluate`; `Party::seed().fork()` with the scan counter reset reads `scan_bits() >= 2`; the `clock_fork` cells on version-only families render at least one `flr[..]` entry; `just amp-board-acceptance` stays green (any new red is a finding to triage).
Construction: (a) In board/tests.rs under `limb-meter`: `let v = version_of(&dense(1_000)); suanpan::touch_meter::reset(); let _ = Version::decode(&v.encode()); assert!(suanpan::touch_meter::touches() >= stored_nonzero_deltas(&v));` passes today by the reading above, while `touch_wide_stream(&v)` returns `NotApplicable`: a validator moved onto an unmetered `i128` height would read 0 touches and stay green on every all-narrow family. (b) Under `scan-meter`: `crate::meter::reset_scan_bits(); let mut p = Party::seed(); let _ = p.fork(); assert!(crate::meter::scan_bits() >= 2);` while the cell declares "its packed form is empty". (c) Reset the peak allocator, `Clock::from_parts(Party::seed(), v).fork()`, observe peak >= the child party's packed byte: the allocation `party_fork` floors and `clock_fork` declares unforced.

### board-families-floors-judge-20: Exponent legs can go unjudged with nothing pinning which cells may
- Where: crates/before/src/meter/board/judge.rs:120-124 (related: judge.rs:149-152, 363; render.rs:74-82; ceilings.rs:218-229; shard.rs:497-520; board/tests.rs:683-707, 1059-1062; tests/amp_board_smoke.rs:229-302)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rn exp_judged` across src, tests, examples: judge.rs, render.rs:78, and the one negative assertion at tests.rs:1060; shard.rs:505-520 counts cells against `Coverage::Board` only; tools/benchjudge-expected.json pins only `red`); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found (letting an unjudged leg ride constants and floors is deliberate and stated at ceilings.rs:225-228; pinning the unjudged set was never considered)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run through `evaluate`, not the smoke capture loop). An 8x scan reading across a 2x denominator reads red ("scan exponent", trend 3.0) when the exponent denominator scales, and `red = []` with `exp_judged = false` when the second sample's `exp_denom` equals the first: the guard at judge.rs:120-124 silently unjudges the leg and the cell is GREEN.
- Cross-references: board-ops-render-18 (the disclosure half of the same skip).

An exponent leg whose denominator pair fails `MIN_EXPONENT_DENOM_GROWTH` is silently unjudged (green) and renders ` -.--`; no test, roster, or merge refusal pins the set of cells whose exponent legs are expected to be unjudged, so a family whose generator stops scaling (or a slip in a denominator's call path) converts every exponent ceiling in that column into a vacuous pass while the acceptance board stays green. A per-byte constant at a non-scaling size is not a scaling judgment. This is the exponent-ceiling analogue of a ceiling over a dead counter: the doctrine pairs every ceiling with a liveness guard and a committed known-bad artifact, and here the known-bad artifact (a cell whose denominator stopped doubling) reads green.

Evidence:

       120	    let spans = |points: &[(usize, u64)]| -> bool {
       121	        let first = points.first().map_or(0, |&(n, _)| n);
       122	        let last = points.last().map_or(0, |&(n, _)| n);
       123	        last as f64 >= first as f64 * MIN_EXPONENT_DENOM_GROWTH
       124	    };

       149	    Fit {
       150	        exp: Some(trend(&points)),
       151	        judged: spans(&points),
       152	    }

       363	        if s.exp_judged && s.exp.is_some_and(|e| e > *ceilings.get(c)) {

    render.rs:
        78	            Some(e) if s.exp_judged => format!("{e:5.2}"),
        79	            Some(_) => " -.--".to_string(),

Resolution: Commit a tamper-evident roster of the cells whose exponent legs are expected unjudged (today: the rank rows on the benign family, heap legs inside the flat allowance, capacity-model cells), in the style of the worst-rankings roster, and have `run_acceptance` (or the shard merge) refuse any compiled currency whose `Fit.judged` is false on a cell outside it; add the known-bad artifact beside `merge_refuses_a_silently_shrunk_grid_for_every_family`: a capture with one cell's second-sample `exp_denom` overwritten to equal the first's must be refused. Acceptance: the tampered-capture test fails against today's code (the board renders GREEN with ` -.--` on the tampered cell) and passes once the roster check lands; the smoke and acceptance boards match the committed roster exactly.
Construction: In tests/amp_board_smoke.rs, reuse the capture-and-edit loop at 229-302: split one cell line on tabs, set the second sample's `exp_denom` field (the second field of the second sample block, per `emit_sample`'s order at shard.rs:205-241) equal to the first sample's, keep the end count, and call `board::run` on the tampered capture. Today it succeeds and the rendered row shows ` -.--` on that cell's limb/scan/touch exponents with a GREEN verdict.

### board-families-floors-judge-11: The "four cells watched by neither leg" disclosure is a hand count with no pin on either side, already stale, and its 10 µs restates a benchjudge constant
- Where: crates/before/src/meter/board/floors.rs:99-106 (related: floors.rs:92-97, 107-112, 431-439; board.rs:99-100; ops.rs:1904, 1924, 2065, 2085, 2142, 1608-1626; tools/benchjudge:146-160; tools/benchjudge-expected.json)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the five text-rejection rows passing all-NA through `text_rejection_floors`; tools/benchjudge:146-160; no `NotApplicable` match in board tests outside render/shard; the hash and eq rows' all-NA status is as the disclosure itself states, assessed; the sub-microsecond claim for `clock_fork` on a seed party is inferred, not timed); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed (with the note that the disclosure's criterion is two-part, all-NA and sub-floor, and neither half is pinned); history: no-rationale-found (the count grew by hand: "three cells to four" in b35e3f00)
- Owner-gated: no
- Cross-references: board-frame-3 (the same roster from board.rs), meter-adequacy-4 (the bench judge's unpinned sub-floor set).

The disclosure names four cells as watched by neither the deterministic floors nor the time leg. Nothing enumerates the all-NA cells (the five text-rejection rows are all-NA too, disclosed separately at 92-97 with the time leg claimed to carry them, and finding 10 shows `clock_fork` on every version-only family is all-NA and sub-microsecond), and nothing pins the bench judge's sub-floor set (tools/benchjudge-expected.json pins only `red`). "10 µs" is `MIN_JUDGED_MEDIAN_NANOS = CRITERION_TIMER_ERROR_NANOS * RESOLUTION_DOMINANCE_FACTOR` restated as a literal. Principle 6 (every hole found becomes a committed check, never a convention held in memory) and the no-hand-maintained-counts rule: an exposure roster kept in prose grows silently while the prose keeps saying four.

Evidence:

        99	//! Four cells are watched by neither leg, an exposure accepted here so it is
       100	//! stated rather than silent: `version_hash`, `party_hash`, `clock_hash`, and
       101	//! `version_eq` on the benign family. Hashing folds the stored canonical bytes
       105	//! benign operands are small enough (a few hundred packed bytes across both
       106	//! scales) that the body never reaches the bench judge's 10 µs judgment floor.

    board.rs:
        99	//! constructors that commit them, along with the disclosure of the four cells
       100	//! no deterministic leg watches.

    tools/benchjudge:
       157	# The judgment floor: cells whose larger-scale median sits below this are
       158	# enumerated but never judged. Derived, not calibrated:
       159	# CRITERION_TIMER_ERROR_NANOS x RESOLUTION_DOMINANCE_FACTOR = 10 us.
       160	MIN_JUDGED_MEDIAN_NANOS = CRITERION_TIMER_ERROR_NANOS * RESOLUTION_DOMINANCE_FACTOR

Resolution: Add a test beside the board tests that builds every board bundle, collects every cell whose `Floors` are all `NotApplicable`, and asserts the set equals a committed roster (the four named, the five text-rejection rows, and the `clock_fork` cells finding 10 leaves all-NA until it is fixed, or the smaller set once it is); have the disclosure here and at board.rs:99-100 state the class and cite that roster and `MIN_JUDGED_MEDIAN_NANOS` by name instead of "four" and "10 µs". If the wall-time half matters, have tools/benchjudge emit its sub-floor cell set and pin it in tests/bench_judge_roster.rs. Acceptance: a committed test fails when a new op declares NA on every floored currency without joining the roster; floors.rs:99-112 and board.rs:99-100 carry no cell count and no duration literal.
Construction: Add a row to ops.rs whose `prepare` returns `Floors` with `na(..)` on heap, limb, scan, and touch, or observe that `clock_fork` on the dense family already does: nothing in the gate changes and floors.rs still reads "Four cells".

### board-families-floors-judge-19: Only the scan floor has a committed known-dead demonstration; limb, touch, and heap floor trips are asserted in prose alone
- Where: crates/before/src/meter/board/judge.rs:65-73 (related: judge.rs:77-82, 373-388; board/tests.rs:405-484; floors.rs:441-521)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn '_FLOOR_TRIP'` outside judge.rs: tests.rs:409 and :480 only, both SCAN_FLOOR_TRIP); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no-rationale-found (the touch and limb floors landed after the scan bypass tripwire with no trip demonstration)
- Owner-gated: no

`bypassing_walk_is_green_under_ceilings_alone_and_red_under_floors` demonstrates that a body routing its work around the meters reads green under ceilings and red on SCAN_FLOOR_TRIP; no committed test exercises LIMB_FLOOR_TRIP, TOUCH_FLOOR_TRIP, or HEAP_FLOOR_TRIP through `evaluate`, although the touch and limb floors carry the more intricate derivations. `below_floor` is currency-agnostic, so the mechanism generalizes; the gap is that the per-currency derivations feeding those floors are exercised only by hand-count tests, never end to end as a trip. Every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

        65	/// The limb column's floor-trip message.
        66	pub(super) const LIMB_FLOOR_TRIP: &str =
        67	    "limb floor: counter reads below floor: the meter is not watching this work";
        68	/// The scan column's floor-trip message.
        69	pub(super) const SCAN_FLOOR_TRIP: &str =
        70	    "scan floor: counter reads below floor: the meter is not watching this work";
        71	/// The touch column's floor-trip message.
        72	pub(super) const TOUCH_FLOOR_TRIP: &str =
        73	    "touch floor: counter reads below floor: the meter is not watching this work";

Resolution: Extend the probe pattern at tests.rs:430-457 with one `Sample` pair per remaining floored currency: `touch: Some(0)` under `touch_pair_fold(v, w)` on a dense pair -> `red == [TOUCH_FLOOR_TRIP]`; `limb: Some(0)` under `limb_stream(mandatory_limbs_stream(&hugeleaf(256)))` -> `[LIMB_FLOOR_TRIP]`; `heap: Some(0)` under `heap_materializes(n)` -> `[HEAP_FLOOR_TRIP]`. Acceptance: each of the four live `*_FLOOR_TRIP` constants is asserted by name in a committed test that feeds a zero reading against a floor the floors.rs constructors derived.
Construction: Reuse the tests.rs:430-457 `sample` closure with `touch: Some(0)` and `floors: walk_floors(n, touch_pair_fold(&v, &w))` where `v = version_of(&dense(1_000))` and `w` is `v` ticked at the seed; `evaluate` on two such samples must give `red == vec![TOUCH_FLOOR_TRIP]`. Repeat with `limb: Some(0)` and `floors.limb = limb_stream(mandatory_limbs_stream(&hugeleaf(256)))` (4 limbs per tests.rs:55), expecting LIMB_FLOOR_TRIP.

### board-families-floors-judge-25: The touch floor's sole basis and the flat-denominator axis have no differential pin against the public shape iterator
- Where: crates/before/src/meter/board/operand.rs:89-125 (related: operand.rs:23-45; shape.rs:80-98, 161-178; version.rs:773; board/tests.rs:52-102, 379-383, 556-620)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (`grep -n 'stored_nonzero_deltas\|value_content_bytes' board/tests.rs`: an import at 27 and two growth-ratio uses at 562, 618; the hand-count tests cover `mandatory_limbs_*` and `radix_units_*` only; shape.rs:89-95 read for the `rise: None` semantics, assessed not executed); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found (`Version::shape()` landed 2026-08-19, after every walk; nothing since has considered the pin)
- Owner-gated: no

`stored_nonzero_deltas` is the sole basis of every touch floor and `value_content_bytes` is the flat-denominator exponent axis; neither has a hand-count pin nor a check against the production reader, while `Version::shape()` yields exactly the stored rises (one `Plateau` per stored leaf, `rise: None` iff the stored delta is zero, the first rise the absolute root height), giving the identity `stored_nonzero_deltas(v) == v.shape().skip(1).filter(|p| p.rise.is_some()).count()` for free. Any quantity computable two ways gets a committed test comparing them; a flag-polarity or zigzag drift in one walk would today move every touch floor silently.

Evidence:

        89	pub(super) fn value_content_bytes(v: &Version) -> usize {
        90	    let bits = v.as_bits();
        91	    let mut pos = 0u64;
        92	    let mut pending = 1usize;

    shape.rs:
        89	    /// The height change entering this plateau; `None` continues level.
        90	    ///
        91	    /// The first plateau's rise is its absolute height (`None` if the
        92	    /// shape starts at 0): the walk begins at height 0 on the interval's
        93	    /// left edge. `None` occurs mid-stream too: two equal-height
        94	    /// plateaus separated by a subtree boundary are a real shape.

Resolution: A proptest over the crate's version generators (and a sweep over `study_family_versions`) asserting `stored_nonzero_deltas(v) == v.shape().skip(1).filter(|p| p.rise.is_some()).count()` and that `value_content_bytes(v)` equals the byte-rounded sum over `shape()` of `bits(height).max(1)` with heights accumulated from the rises; optionally implement `stored_nonzero_deltas` through `shape()` and dissolve one decoder (finding 24). Acceptance: the committed differential test passes; flipping the `!bits.bit(pos)` polarity or the odd/even zigzag arm in either walk fails it.
Construction: proptest over `crate::testing::generators`' `Version` strategy: compute both counts and assert equality; the identity follows from shape.rs:89-95.

**Board operation rows, renderer, shards, worst-case pin**

### board-ops-render-15: The `segments` column has no writer in the binary of record: a judged meter that is a compile-time zero, with prose and the ladder-top calibration resting on it
- Where: crates/before/src/meter/board/render.rs:212-221 (related: recurse.rs:17-20, 68-69, 74-76, 100-109, 118-129; Cargo.toml:44; meter.rs:3552-3558; ceilings.rs:453-467; floors.rs:627-636; ops.rs:205 and every `seg_ceiling_only()` site; worst.rs:63-66; meter/tests.rs:399-403; tests/meter.rs:22-26)
- Class / severity / confidence: verification-gap / high / high
- Provenance: verified (recurse.rs read: `SEGMENTS_GROWN` is `#[cfg(any(test, feature = "meter"))]` at 68-69 and its only incrementer `grow` is `#[cfg(test)]` at 100-109, as are `should_grow` and the `descend!` macro; `grep -rn 'SEGMENTS_GROWN\|recurse::grow\|descend!'` over src, tests, benches, examples finds `descend!` users only in testing/bridge.rs, grow/tests.rs, and meter/tests.rs; meter.rs:3552-3558 shows `stack_segments()` reads `recurse::segments_grown()`; Cargo.toml:44 lists `stacker` under `[dev-dependencies]`; `git show -s 1ddb5a483`: "stacker becomes a dev-dependency: library walks are iterative, the guard is test-surface only"); executed: no
- Seen by: instrument-correctness [46]; refutation: confirmed (with git history: the column's judged role and the ×4 ladder top were set when library kernels still recursed (ce9e73b46); the kernels went iterative (da0f6a937, 05bd2b16d) and 1ddb5a483 gated `grow` behind `cfg(test)`; the sentence calling the zero "the measured fact the boards' segments column pins" is a later docs-pass addition (b3f09baa0)); history: deliberate-but-expired (1ddb5a483's keep decision is about the test-only guard, not the board column)
- Owner-gated: yes (removal of an instrument column; a wire-format bump of the shard `PROTOCOL`)
- Witness (witness/results.md): demonstrated (reading of the cfg attributes and the render legend). The only writer of `SEGMENTS_GROWN` is the `fetch_add` at recurse.rs:106 inside `#[cfg(test)] fn grow`; the static and its readers are `cfg(any(test, feature = "meter"))`; the `amp_board` example and every integration-test binary read zero by construction while render.rs:212-214 judges the column. The temporary `descend!` board row and the example run were not attempted.
- Cross-references: crate-root-32, envelopes-a-2, module-graph-1, recursion-1, inventory-2 (the segments cluster; see Crate-wide patterns).

The board renders and judges `segments` as one of five deterministic meters (`seg[e .. ..]` on every row, `segments <= 1` in the legend, an exponent and constant leg in the judge), but the counter's only incrementer is the `#[cfg(test)]` growth arm of `recurse::grow`. The `amp_board` example (the release profile of record) and every integration-test binary compile the library without `cfg(test)`, so no code path in those binaries can write the counter: every cell reads zero by construction, the `MAX_GROWN_STACK_SEGMENTS` ceiling and the segments exponent can never fire there, and a kernel that started recursing would grow no segments through this counter (it would recurse on the native stack, which `deep_tree_stack_safety` catches). Three claims are contradicted: recurse.rs:19-20 calls the zero "the measured fact the boards' segments column pins"; recurse.rs:74-75 says "the counter is always written (the bump is inseparable from the growth arm)" of a build with no growth arm; and `LADDER_TOP_SCALE`'s doc derives ×4 entirely from segment-onset amplifiers that cannot occur in that binary. The unit test that keeps the column "honest" (meter/tests.rs:399-403) proves liveness in the lib's own test build, a different compilation. Principle 2: a ceiling over a counter that cannot count; Principle 3: machinery that outlived the constraint (recursive kernels) that justified it.

Evidence:

       212	        "green iff every meter's exponent <= {MAX_SCALING_EXPONENT}, constants within: \
       213	         heap <= {MAX_HEAP_BYTES_PER_INPUT_BYTE} B/B over {HEAP_FLAT_ALLOWANCE_BYTES} B flat, \
       214	         segments <= {MAX_GROWN_STACK_SEGMENTS}, \
       ...
       220	         the meter is not watching that work; segments is ceiling-only by policy, its honest \
       221	         floor is zero). exponent legs are fitted only where the denominator pair scales \

    recurse.rs:
        68	#[cfg(any(test, feature = "meter"))]
        69	static SEGMENTS_GROWN: AtomicU64 = AtomicU64::new(0);
       ...
       100	#[cfg(test)]
       101	#[inline]
       102	pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
       103	    if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
       104	        f()
       105	    } else {
       106	        SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);

        17	//! what lets it meet deep inputs safely. The segment counter below stays
        18	//! compiled for the meters: it is the deterministic stand-in for
        19	//! recursion-driven stack consumption, and its zero reading over the library
        20	//! kernels is the measured fact the boards' segments column pins.

    ceilings.rs:
       455	/// The base-scale sizes under-detect segment amplifiers: stacker grows a
       456	/// segment only past ~1 MiB of frames, so a recursion-frame amplifier whose
       457	/// onset sits above the base depths reads a false green there. ×4 is the
       458	/// witnessed calibration floor — the smallest sampling scale at which every
       459	/// segment-onset amplifier the suite has caught reads red — so the ladder

Resolution: Owner ruling. (a) Recommended: dissolve the segments currency from the board: drop `ByCurrency::segments`, every row's `seg_ceiling_only()` declaration, the `seg[...]` column, the legend clause, `SEG_FLOOR_TRIP`, and the judge's segments arm; bump the shard `PROTOCOL`; re-state `LADDER_TOP_SCALE`'s rationale on the onset effects that do occur at ×4 (worst.rs:65-66 names the doubling-chain steps); re-word recurse.rs:17-20 and 74-76 and meter/tests.rs:399-403 so the unit test's claim is about the counter mechanism in the test build, not the board; handle or explicitly defer tests/meter.rs's segments pins in the same change. (b) Minimum: disclose in the legend and in board.rs that the counter has no writer outside the lib's test build, so a reader does not take the column as a measurement. Acceptance for (a): `ByCurrency` has four fields; the example and smoke suite pass with no `segments` text on the board face; the acceptance and pin renders are byte-identical on the four remaining columns. For (b): the legend names the column as an inert pinned zero and cites the lib unit test as the counter's only live witness.

Construction: `grep -rn 'SEGMENTS_GROWN.fetch_add' crates/before/src` returns only recurse.rs:106, inside `#[cfg(test)] fn grow`. Add a temporary board row whose body routes through `crate::recurse::descend!`: it fails to compile in the example because the macro is `cfg(test)`. Contrast: `cargo nextest run -p before --lib stack_segment_meter` passes because the lib's own test build compiles `grow`. Any board render shows `seg[e 0.00    0]` on every row.

### board-ops-render-6: The ascend-cliff family-stated heap ceilings have no under-side band or class pin
- Where: crates/before/src/meter/board/ops.rs:388-395 (related: ops.rs:425-431, 797-804, 1584-1590; judge.rs:339-345; ceilings.rs:231-242, 360-407, 424-426; board.rs:178-183; registry.rs:1459-1467; tests/meter.rs:35-37)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (judge.rs:341-345 read: `if let Some(declared) = s2.declared_heap { ceiling = declared; }` with no floor arm, unlike the capacity model's banded floor at judge.rs:331-336; ceilings.rs:360-407 read: no under-side clause, where the mirror-wide constant at 424-426 names its liveness pin; `grep -n -i 'ascend\|certificate\|reign' src/testing/asymptotics.rs` returns nothing; registry.rs:1463 `bands: Bands::Unbanded`); executed: no
- Seen by: adequacy [18]; refutation: confirmed; history: no-rationale-found (669cf3103 landed the ascend-cliff declarations beside the mirror-wide model; the mirror-wide doc names its under-side witness, the ascend-cliff docs say nothing about one; the board module doc and the LLM-written design note both assert every declared model is held on the under side)
- Owner-gated: yes (a documented design decision about a ratified declared model)
- Witness (witness/results.md): demonstrated (run through `judge::evaluate` with the capacity-model probe's sample shape). Two samples carrying `declared_heap: Some(227.0)`, reading 10 B/B at denominators 10_000 and 20_000 (exponent leg judged, reading 1.00), produce an empty `red`; the capacity model's "improved" probe at tests.rs:1074-1086 reads `heap capacity-model floor (stale model)` for the same shape of event.

The tick trio and `version_min_ticks` on the ascend-cliff cross are judged at declared flat heap ceilings of 227 and 247 B/B in place of the global 16 B/B, but the judge substitutes the ceiling only. There is no banded floor (as the capacity model has) and no committed class-liveness pin (as the mirror-wide model has), and the family is `Bands::Unbanded` in the registry, so a certificate-memory cure that drops the reading to 10 B/B stays green under the stale 227 B/B declaration indefinitely. The board module doc commits every declared model to being "held honest on the under side"; the ceilings header says the same (231-242). This is the "mechanism for accepting known failures" the doctrine forbids, once the failure is cured.

Evidence:

       388	                    // The ascending cliff defeats certificate consumption,
       389	                    // so its tick cells carry the ratified family-stated
       390	                    // heap ceiling (the constant's derivation).
       391	                    return Some(if matches!(f.kind, FamilyId::AscendCliff) {
       392	                        cell.with_declared_heap(ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE)

    judge.rs:
       341	        if c == Currency::Heap {
       342	            if let Some(declared) = s2.declared_heap {
       343	                ceiling = declared;
       344	            }
       345	        }

    board.rs:
       180	//! declared-models section), and held honest on the under side — banded floors
       181	//! where the model predicts a quantity, committed liveness pins where it
       182	//! declares a class — so an improved kernel forces a deliberate re-declaration,

Resolution: Either band the family-stated heap ceilings as the capacity model is banded (a `DECLARED_HEAP_FLOOR` fraction of the declared constant, red as "heap family-stated floor (stale model)"), or commit a class-liveness pin for the certificate-memory mechanism (an envelope or asymptotics test asserting the ascend-cliff tick heap reads above the global 16 B/B, which reads red the day consumption is cured) and cite it from the two constants' docs as the mirror-wide constant cites `render_merge_superlinearity_is_alive`. Add the "improved" probe to tests.rs beside `declared_capacity_model_bands_the_projection_peak`. Acceptance: a synthetic `declared_heap: Some(227.0)` sample pair reading 10 B/B evaluates red on a stale-model leg, or a committed pin fails when the ascend-cliff tick heap drops under the global ceiling; ceilings.rs:360-407 name the under-side check; the release board stays green.

Construction: Through `evaluate`, build two `Sample`s with `declared_heap: Some(ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE)`, denominators 10_000 and 20_000, heap readings 100_000 and 200_000 (10 B/B, above the 8_192 B flat allowance so the exponent leg is judged and reads 1.00), all floors NA: `red` is empty. Compare the capacity model's "improved" probe at tests.rs:1074-1086, which reads red for the same shape of event.

### board-ops-render-9: `causally_contains` and `party_covers` measure only their early-exit verdict on nearly every family
- Where: crates/before/src/meter/board/ops.rs:1113-1126 (related: ops.rs:1397-1411; floors.rs:697-739; family.rs:679, 717, 751, 761, 797-803; causally/forms.rs:150-153; party.rs:405-406; coverage.rs:141-147; tests/meter.rs:6135-6144, 6354-6361; worst.rs:438, 450; board.rs:189-196)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: assessed (read: `since` is documented as everything `s` does not already contain; the post-pass sets `w = v + Party::seed()` tick wherever a shape did not build its own `version2`, and only JumpPair, DenseSuffix, ConcurrentPair, and ToothTail do; `membership_floors` refuses (full walk) only on `v.partial_cmp(w) == Some(Greater)`; `party_pair` is documented as the disjoint pair; grep of tests/meter.rs shows both `covers` scenarios assert `!a.covers(b)`); executed: no
- Seen by: instrument-correctness [47]; refutation: confirmed (by reading, not constructed; the certifying walk is measured on at most three families, and the accepting `a ⊇ b` walk on none in the instrument corpus); history: no-rationale-found (7bd84b49 established that thirteen families answer the admitting verdict and dense-suffix alone forces certification, and chose to re-derive the floors on the verdict rather than re-orient the probe; no reason for leaving the certifying path unmeasured is recorded)
- Owner-gated: yes (re-orienting a probe changes what a board row of record measures and forces a `WORST_RANKINGS` re-pin; replacing versus adding a row is the owner's call)
- Witness (witness/results.md): demonstrated (run over the board's own bundles at scale 1.0, level 0). `since(&v).contains(&w)` returns true on 28 of 29 families with a version pair and false only on dense-suffix; `a.covers(&b)` is false on all 14 families with a party pair. The refusal-walk plant in the construction was not attempted.

The membership row asks `since(&v).contains(&w)` where the bundle post-pass makes `w = v + one seed tick` for every family that did not build its own pairing, so `w` is in `v`'s strict future, the query admits at one witness, and `membership_floors` commits the root-code scan floor with touch NA. The certifying full walk is measured only where a shape's own pairing satisfies `w < v` (dense-suffix's unit-base mate, consistent with dense-suffix holding the row's limb and scan argmax in the pin). `party_covers` asks `a.covers(&b)` of the disjoint pair, false for every family and decided at the root; the accepting walk is measured nowhere, and the envelope suite's two covers scenarios also assert the disjoint case. The coverage roster credits these rows with pricing `Query::contains` and `Party::covers`. The board's own rule ("the defect maximally deferred in every shape: an early-exit-only measurement would be the cheapest artifact that passes", board.rs:191-196) is stated for the rejection rows and not applied to these two verdicts.

Evidence:

      1120	                let (v, w, n) = f.version_pair()?;
      1121	                let floors = membership_floors(&v, &w, n);
      1122	                Some(Cell::new(n, floors, move || {
      1123	                    let hit = causally::since(&v).contains(&w);

      1401	                let (a, b, n) = f.party_pair()?;
      ...
      1406	                    scan: scan_touch(),
      ...
      1409	                Some(Cell::new(n, floors, move || (a.covers(&b), a, b)))

    family.rs:
       799	            if data.version2.is_none() {
       800	                let mut w = v.clone();
       801	                w.tick(&Party::seed());
       802	                data.version2 = Some(w.encode());

    floors.rs:
       728	    if v.partial_cmp(w) == Some(Ordering::Greater) {
       729	        walk_floors(packed_bytes, touch_pair_fold(v, w))
       730	    } else {

Resolution: Orient each probe so the certifying verdict is what the cell measures, from operands the bundle already supplies: for membership, `causally::since(&w).contains(&v)` (with `w = v + tick`, `v` is covered, so the query refuses and certifies every region; keep `membership_floors` deriving from the actual verdict so a concurrent pairing still floors correctly); for covers, probe `a.covers(&decode_party(&a_bytes))` (the buffer-distinct re-decode the placement rows use at ops.rs:1128-1135) or a prepared `join(a, b).covers(&b)`, floored at `scan_examines(n)`. If both verdicts are wanted, add the certifying rows (this moves every family's `Coverage::Board { cells }` count and the bench mirror). Acceptance: on the release board both rows render a full-examination scan floor on every family, their scan constants sit near a full walk's reading, and the two pin rows move with the movement annotated; the smoke suite's per-family cell counts are unchanged (replace) or re-stated (add).

Construction: At prepare, log the verdict per family: `causally::since(&v).contains(&w)` returns `true` on every family whose `version2` came from the post-pass, and `a.covers(&b)` returns `false` on every family. Compare the two rows' scan readings against `version_cmp` on the same family (a comparable pair that must certify reads ~8 bits per packed byte; these rows read root-code scans on the post-pass-paired families). To make the gap visible, plant a refusal walk that re-scans the probe once per plateau of the bound: the board stays green on every family except dense-suffix.

### board-ops-render-10: The placement rows declare the touch column not applicable although the placement kernel's own doc states the pair-fold premise the floor needs
- Where: crates/before/src/meter/board/ops.rs:1149-1151 (related: ops.rs:1128-1140, 1169, 1187, 1205, 1228, 1257; floors.rs:328-332, 458-492, 643-651; place.rs:22-31; worst.rs:439-444; tests/meter.rs:9463-9820)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: assessed (read; grep for `touch` inside tests/meter.rs's placement module (lines 9463-9820) returns nothing, so the touch meter inside the placement kernel is unfloored in the envelope suite too; the live kernel was not run to confirm touches exceed the proposed floor); executed: no
- Seen by: adequacy [17]; refutation: confirmed (the rows already commit `scan_examines(n)` on the argument that full examination is forced; the NA reason argues from per-byte variability, which the pair floors already tolerate by taking a max); history: deliberate-and-holds for the NA's existence (declared with the rows at 363e96e0d, reason inline), but place.rs:22-31 (db9dfa3ed, 2026-08-05) postdates it and states the premise the floor rests on, which is new evidence
- Owner-gated: yes (the finding reopens a recorded NA declaration)
- Witness (witness/results.md): demonstrated (run through `evaluate`). A span_place-like cell with the committed `walk_floors(n, na(NA_TOUCH_PLACEMENT))` and touch reading 0 is `red = []`, while the same samples under `touch_pair_fold` read `[TOUCH_FLOOR_TRIP]` with a floor of 64 on the staircase(64)/dense(64) pair; on the live kernel `span.place(&probe)` on that pair reads 327 touches, so a positive floor is derivable.
- Cross-references: skyline-sweep-place-masked-19 (the placement identity rows in tests/meter.rs read no touch meter either).

Six rows commit `walk_floors(n, na(NA_TOUCH_PLACEMENT))`, leaving the touch column unfloored while the placement walk maintains one `Accumulator` running difference per bound and folds its sign per elementary interval exactly as the pair sweep does; place.rs says each accumulator "sees exactly the write sequence the corresponding pair sweep would commit", which is `touch_pair_fold`'s one universal premise (every nonzero stored delta of either operand lands in the running difference, max not sum). The rows force the confirming full sweep by construction (ops.rs:1128-1140) and commit the full-examination scan floor on that argument, so the touch floor follows from the same premise at arity three. Principle 2: a ceiling over a counter passes vacuously when the counter goes dark, and a touch-meter bypass inside `place` reads green on six rows today; the pin shows the counter live and nonzero on every one of them.

Evidence:

      1149	                Some(Cell::new(
      1150	                    n,
      1151	                    walk_floors(n, na(NA_TOUCH_PLACEMENT)),

    floors.rs:
       329	pub(super) const NA_TOUCH_PLACEMENT: &str =
       330	    "the fused placement walk's delta-fold count varies with how the \
       331	     bounds partition the probe's intervals: no per-byte fold count is \
       332	     forced";

    place.rs:
        26	//! that orientation everywhere: the probe is every pair's `a` operand. A probe
        27	//! crossing folds into both differences; a bound crossing folds into its own.
        28	//! Each accumulator therefore sees exactly the write sequence the corresponding
        29	//! pair sweep would commit — the identity the resource pins in

Resolution: Add `touch_placement_fold(probe, lo, hi)` in floors.rs returning `Floor { min: max(nz(probe), nz(lo), nz(hi)), why }` (NA only when all three store no nonzero delta), state the arity-three premise beside `touch_pair_fold`, and use it on the five placement rows; decide `query_coverage` separately (its two-probe walk has clamp legs) and either floor it the same way or state positively why not. Retire `NA_TOUCH_PLACEMENT` if nothing else uses it. Acceptance: the five rows render a nonzero touch floor on every family with stored nonzero deltas; a synthetic span_place-like sample with `readings.touch = Some(0)` evaluates red on `TOUCH_FLOOR_TRIP` (a unit test beside the bypass-walk probe); the release board stays green.

Construction: Through `evaluate`, two `Sample`s for a span_place-like cell with the committed `walk_floors(n, na(NA_TOUCH_PLACEMENT))` and `readings.touch = Some(0)`: `red` is empty. Under the proposed floor the same samples read `[TOUCH_FLOOR_TRIP]`. To confirm the premise on the live kernel, reset `suanpan::touch_meter`, run `span.place(&probe)` on a staircase pair, and check touches are at least the maximum nonzero-delta count of the three streams.

### board-ops-render-26: The delegating-parser pinned floors' separation from the bypass reading is prose, not a per-run check, and the table is a measured band labelled a liveness floor
- Where: crates/before/src/meter/board/tests.rs:128-143 (related: tests.rs:158-197; codec/base.rs:84-95; codec/base/limb_meter.rs:39-42; tests/meter.rs:35-53)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the test body 158-197: it computes `ops`, the derived floor, and the pinned floor, and never computes the radix site's contribution, so the doc's separation claim is unchecked per run; `git show -s bc222bd2`: "the delegating-parser bigroot floor separated nothing; re-pinned over the live bypass reading" with its body recording the bypass passing the guard by 46% and radix contributions of 250 of 752 (hugeleaf) and 4_127 of 24_399 (bigroot); base.rs:92-93 records once per parsed decimal value via `limb_meter::record_wide`, which records `bit_len.div_ceil(64).max(1)`); executed: no
- Seen by: scaffolding [11], adequacy [16], refutation's new item 2; refutation: confirmed (adding that the doc's "roughly half" clause disagrees with bc222bd2's own readings, 33% and 17%, and that the pinned values have since moved from 639/20_739 to 425/7_020, so the clause is a hand-maintained measurement statement of the genre 500d4d09 excised and is unverified against current readings); history: deliberate-but-expired (bc222bd2 knowingly left separation to prose, recording both readings and the 2.3% margin as its mitigation; 500d4d09 then excised measured readings from meter prose, so the mitigation no longer exists in the tree)
- Owner-gated: no
- Witness (witness/results.md): refuted on its numbers (run under limb-meter). Computing the radix site's recording exactly as base.rs:92-93 does, the radix site contributes 50% of the pipeline total on both families (250 of 501; 4127 of 8259), so the doc's "roughly half" is accurate, and the pinned floors sit above the bypass reading with margin (251 < 425; 4132 < 7020): a parser that deleted the radix recording would trip both floors today. The 752 and 24_399 totals quoted from bc222bd2 are not today's readings. The verification-gap half stands by reading: tests.rs:180-189 asserts only `ops >= floor` and `ops >= pinned_floor`, never the bypass separation; the scratch per-run check passes.

The pinned per-family floors are measured pipeline totals ×0.85 whose whole value rests on sitting above the pipeline-total-minus-radix bypass reading, but that separation is asserted only in the doc comment; nothing in the test computes the bypass reading, so the floor can degrade to decoration while reading green, which bc222bd2 records happening once. The doc also states a qualitative measurement ("the radix site alone contributes roughly half on both families") that its own re-pin commit contradicts, and the table is a measured band asserted with a liveness floor's message although tests/meter.rs:35-53 names the two genres apart (a measured band trips on an honest improvement; a derived floor never can). Principle 2: every criterion needs a committed demonstration that the known-bad mechanism (the radix site not recording) fails it, and the demonstration must live in the run, not in memory.

Evidence:

       132	/// The pipeline records from two sites — the delegated radix conversion (one
       133	/// width-proportional count per materialized value) and the gamma encoder's
       134	/// arithmetic — and the radix site alone contributes roughly half on both
       135	/// families, so a floor at ×0.85 of the pipeline total sits above what the
       ...
       138	/// encode-side arithmetic already covers them). The separation margin is
       139	/// measured, not structural — the floor sits over a bypass reading of
       140	/// pipeline-total-minus-radix — so a re-measure that moves the encoder site
       141	/// up must re-derive both readings before trusting the floor separates.
       142	#[cfg(feature = "limb-meter")]
       143	const DELEGATING_PARSE_LIMB_FLOORS: [(&str, u64); 2] = [("hugeleaf", 425), ("bigroot", 7_020)];

    base.rs:
        92	        #[cfg(feature = "limb-meter")]
        93	        limb_meter::record_wide(&value);

Resolution: Make the separation a per-run check inside `delegating_parser_stays_under_the_text_limb_ceiling`: compute the radix site's contribution outside measurement (the hypothesis is `stored_bases(&v).iter().map(|b| b.bits().div_ceil(64).max(1)).sum::<u64>()`, since `parse_decimal` records exactly that per spelled value; measure it rather than transcribe it), and assert `ops - radix < pinned_floor && pinned_floor <= ops`, so the floor is proven to sit strictly between the bypass reading and the live reading on every run. Delete the "roughly half" clause. Re-label the constant's doc to the genre it is (a measured tripwire with a per-run separation witness), or, structurally, give the radix delegation its own counter site and floor it at `mandatory_limbs_version(&v)`, which dissolves the table. Acceptance: setting a pinned floor to `radix - 1` fails the separation leg; the doc claims no separation it does not check and no reading it does not measure.

Construction: In the existing test, after `let ops = crate::meter::limb_ops();`, add the radix computation and `assert!(ops - radix < pinned_floor, ..)`. To demonstrate the gap today, set a pinned floor to `radix - 1`: every existing assertion still passes (`ops >= floor` trivially) while the floor would not trip a parser that deleted the radix recording.

Synthesis note: The witness refutes this entry's numerical evidence and its "roughly half" dispute, and the entry is kept for the half that survives: the separation is checked by no committed run, so the floors can degrade to decoration again unobserved, as bc222bd2 records once happening. Read the severity as attaching to that gap; the "measured band labelled a liveness floor" clause is a documentation point that stands.

### board-ops-render-16: Six of the seven shard-merge refusals have no committed known-bad demonstration, and the round-trip test's doc claims coverage of guards it only exercises on the accept path
- Where: crates/before/src/meter/board/shard.rs:34-42 (related: shard.rs:426-496 (the asserts at 436-439, 445, 449-453, 460, 466, 474-475, 481, 488, 494); tests/amp_board_smoke.rs:131-165, 229-302)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep for `stamp mismatch`, `outside its slice`, `duplicate cell`, `emitted past its end`, `no end line`, `unknown operation`, `unknown family`, `trailing fields`, `unknown line`, `non-UTF-8` across crates/before/tests and board/tests.rs returns nothing; the smoke suite read in full holds four tests plus the band-parity survivor, and only `merge_refuses_a_silently_shrunk_grid_for_every_family` tampers a capture); executed: no
- Seen by: scaffolding [5], adequacy [21], instrument-correctness [50]; refutation: confirmed (the duplicate refusal is necessary: `BTreeMap::insert` overwrites, so without the assert at 486-489 a duplicated line plus a restated end count merges with per-family counts intact); history: no-rationale-found (the refusals landed under the untampered round trip only; b58a80ef's tamper test was scoped to the completeness refusal it added)
- Owner-gated: no

The module doc lists the refusals the parent enforces; only the completeness refusal has a committed tampered capture. `shard_protocol_round_trips` (smoke 135-137) says it covers "the ownership and count guards", but an untampered three-shard round trip exercises only their accept paths, so a guard that never fires passes it. Deleting the `owns` assert or the duplicate assert passes every committed test. Principle 6: every criterion needs a committed demonstration that a known-bad mechanism fails it; the tampering harness (untampered capture, line surgery, `catch_unwind`, message match) already exists in the completeness test.

Evidence:

        34	//! guarding truncation. The parent refuses any mismatch — a header that is not
        35	//! byte-for-byte the one it commissioned, an unknown operation or family name,
        36	//! a cell outside the child's slice, a duplicate cell, a count that
        37	//! disagrees with the lines received, or a merged grid whose per-family

       483	            let duplicate = cells
       484	                .insert((op_position, family_position), (op, family, s1, s2))
       485	                .is_some();
       486	            assert!(
       487	                !duplicate,
       488	                "amp-board shard merge: duplicate cell {op} x {family}"
       489	            );

Resolution: Extend the smoke suite's tamper test into a table over the untampered capture: flip one hex digit of the header's scale bits (stamp mismatch); rename one cell's op or family (unknown operation/family); in a two- or three-shard deal move one cell line into another shard's capture and restate both counts (outside its slice); duplicate a cell line and bump the end count (duplicate); drop the end line (truncated); leave the count unchanged after dropping a line (count disagreement); append a field (trailing fields). Assert each is refused with the documented message fragment. Re-word `shard_protocol_round_trips`'s doc to claim the accept path only. Acceptance: one test per refusal in the module doc's list, each failing if its assert is deleted.

Construction: Take the untampered single-shard capture the completeness test already builds; duplicate `lines[1]` and set `end N+1`; today `merge_samples` panics on the duplicate assert; with that assert removed the merge succeeds and renders a board with the right per-family counts.

### board-ops-render-18: An unjudged exponent leg passes acceptance uncounted and undeclared
- Where: crates/before/src/meter/board/shard.rs:600-611 (related: render.rs:74-82, 221-225; judge.rs:120-152, 363; ceilings.rs:218-229; tests.rs:686-696)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read; the refutation checked the arithmetic of the construction by hand against `MAX_LIMB_OPS_PER_INPUT_BYTE = 128` and `MIN_EXPONENT_DENOM_GROWTH = 1.5`); executed: no
- Seen by: adequacy [19]; refutation: reframed (the skip is the documented design: an unjudged cell rides its constants and floors, which bound single-size cost, and the validation index assigns constant-tight regressions to the envelope suite; what survives is a disclosure gap: no declaration of which cells are expected to take the skip and no count on the summary line); history: no-rationale-found (the guards are an owner-ratified ruling (f68802c33) that fixes the `-.--` rendering; neither it nor 9e36dd280 addresses counting or declaring unjudged legs)
- Owner-gated: yes (the exponent guards are an owner-ratified ruling)
- Cross-references: board-families-floors-judge-20 (the same skip, with the tampered-capture construction and the roster resolution).

When a cell's denominator pair fails the `MIN_EXPONENT_DENOM_GROWTH` guard, or its heap readings sit inside the flat allowance, the exponent leg is unjudged, the renderer prints `-.--`, and the cell reads green; `Summary` and the exit code carry no count of unjudged legs, and no per-cell declaration says which cells are expected to be unjudged (the ceilings doc names one: the benign rank pair). A generator that stops scaling an operand drops that cell's exponent leg silently on the board of record. The board solves this shape for floors with the typed `Liveness::NotApplicable { reason }` the compiler demands; the exponent leg has no such declaration.

Evidence:

       602	    let red: BTreeSet<(&'static str, &'static str)> = lo_cells
       603	        .iter()
       604	        .chain(hi_cells.iter())
       605	        .filter(|cell| !cell.red.is_empty())
       606	        .map(|cell| (cell.op, cell.family))
       607	        .collect();

    render.rs:
        74	    // An exponent the guards leave unjudged renders -.-- : printing the
        75	    // fitted digits would invite reading noise as a measurement.

Resolution: At minimum, count unjudged non-heap exponent legs (and heap legs whose readings clear the allowance) on the summary line so the number is diffable. Better, give the exponent leg the floors' totality: a per-cell declaration on the rows whose operands legitimately do not scale (the benign rank pair), rendered in the legend, with an undeclared unjudged leg counted red by `run_acceptance`. Acceptance: forged captures with one cell's denominators flat across the ladder and readings growing ×100 make `run_acceptance` exit nonzero; the benign rank pair's declared reason renders in the legend; the release board stays green.

Construction: Through `evaluate`, `sample(100, limb 10)` and `sample(120, limb 10_000)` with all-NA floors: per-unit 10_000/120 = 83 < 128, the pair grows less than ×1.5, so `red` is empty and `scores.limb.exp_judged` is false (the existing `exponent_guards_skip_noise_and_keep_real_amplifiers_red` sub-scaling probe already shows the not-red half). Board-level: take an untampered in-process capture, restamp its header to the ladder scales' bit patterns, edit one cell's denominator fields to a constant and its limb readings to grow ×100 across the four points, and feed both to `run_acceptance`: `Summary.red == 0`.

### board-ops-render-22: The two verdict-of-record entry points have no committed wrong-artifact demonstration
- Where: crates/before/src/meter/board/worst.rs:593-608 (related: shard.rs:570-612; tests.rs:770-855; examples/amp_board.rs:168-187)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (grep for `check_with`, `run_acceptance`, `check_worst_map` across board/tests.rs, tests/, and examples/ finds only the two callers in examples/amp_board.rs:169 and 179; tests.rs exercises `evaluate_acceptance` at 772, 813, 835 and neither caller); executed: no
- Seen by: adequacy [20]; refutation: confirmed; history: no-rationale-found (493ef543's test inventory never drives `check_with`'s drift path; 9e36dd280 tested `evaluate_acceptance` and left `run_acceptance`'s glue to the gate run)
- Owner-gated: no

`check_with` (the pin's drift detector) and `run_acceptance`'s glue (the two-sweep zip and the distinct-red union) are exercised only by the release gate run, whose only observed outcome is "clean"; a checker that never sets `clean = false`, or a summary that counted only one window's reds, would pass the gate with no test to catch it. Principle 6: a harness bug that masks a failure is the highest-value target for a known-bad demonstration, and `check_with` is injectable through its `sweeps` closure.

Evidence:

       593	pub(super) fn check_with(
       594	    sweeps: &mut dyn FnMut(f64) -> io::Result<Vec<CellResult>>,
       595	    out: &mut dyn Write,
       596	) -> io::Result<bool> {
       ...
       608	    let mut clean = true;

Resolution: In tests.rs, (a) feed `check_with` a one-operation synthetic sweep (built from `Sample`s through `evaluate`) whose argmax disagrees with `WORST_RANKINGS` on one currency and assert `Ok(false)` plus a drift line naming op, currency, and both worsts; and a sweep missing a pinned op, asserting the stale-entry line. (b) For `run_acceptance`, restamp an untampered smoke capture to the two ladder scales, raise one cell's top-window heap reading over its ceiling, and assert `Summary.red == 1` counted once. Acceptance: both tests fail under a mutated `check_with` that never clears `clean` and under a `run_acceptance` that unions only `lo_cells`.

### board-ops-render-27: Counter-reading tests omit the one-test-per-process premise their sibling suites state
- Where: crates/before/src/meter/board/tests.rs:175-177 (related: tests.rs:261-274, 434-436, 522-527, 566-568, 619-626; meter/tests.rs:21-25; tests/meter.rs:15-26; meter.rs:3549-3551)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (`grep -n -i 'nextest\|process-global\|per process\|ISOLATION' board/tests.rs` returns nothing; meter/tests.rs:24-25 defines `ISOLATION_NOTE` for exactly this); executed: no
- Seen by: instrument-correctness [51]; refutation: confirmed (a diagnosis-quality nit under the sanctioned runner, a flake under `cargo test`); history: no-rationale-found (`ISOLATION_NOTE` predates every counter-reading board test)
- Owner-gated: no
- Cross-references: suite-economics-1 (the isolation cluster).

Six tests reset and read the process-global counters around a body. Under a shared-process `cargo test` they race with every concurrent test doing `Base` arithmetic or stream reads, and their failure messages would misdiagnose the pollution as a criterion breach ("width-scale work re-entered the parse path", "the criterion softened"). Principle 8: a failure that could be either a breach or runner pollution must say so at the failure site, as `meter/tests.rs` does.

Evidence:

       175	        crate::meter::reset_limb_ops();
       176	        let parsed: Version = s.parse().expect("a displayed version parses back");
       177	        let ops = crate::meter::limb_ops();

    meter/tests.rs:
        24	const ISOLATION_NOTE: &str = "note: the counter is process-global and meaningful only one \
        25	     test per process: run under cargo nextest, not a shared-process cargo test";

Resolution: Hoist `ISOLATION_NOTE` to a shared meter test-support location (or re-declare it here) and append it to every assertion whose operand is a counter reading in these six tests; alternatively serialize the counter-reading tests behind one process-wide lock. Acceptance: every counter-comparison assertion in tests.rs carries the note or the tests hold a shared lock.

**Surface roster, surface-coverage, surfacecheck, surface-scan**

### surface-roster-4: The `Party::tick` row cites tests that never exercise `Party::tick`; the reduction law that does is cited by nothing
- Where: crates/before/src/surface.rs:340-345 (related: crates/before/src/laws.rs:2480-2488, crates/before/src/surface.rs:346-351, crates/before/src/testing/surface_coverage.rs:53-55)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -rn '\.tick(&mut\|Party::tick'` over crates/before/{src,tests,benches,examples,fuzz}: the only non-doctest call of `Party::tick` is laws.rs:2486 inside `party_tick_matches_version_tick`; `grep -rn party_tick_matches_version_tick crates/before tools` hits only its definition at laws.rs:2482); executed: no
- Seen by: adequacy; refutation: confirmed; history: no-rationale-found (the row's citations are Version::tick anchors from the roster's first commit; the law landed the next day and was never wired in, while the sibling `Party::ticks` row cites `party_ticks_matches_version_ticks` on all three legs)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). With `Party::tick`'s body replaced by `let _ = version;`, all three roster tests stay green while the algebraic-laws driver fails naming `party_tick_matches_version_tick`; grep shows that name occurs only at its definition (laws.rs:2482), so deleting the law would leave every roster citation resolving.

All three legs are `Trans` citations to `version_tick_matches_the_oracle`, `event_dominates_local_and_advances`, and `replay_matches_across_references`, none of which calls `Party::tick`. The `Trans` contract (surface_coverage.rs:53-55: "the named test anchors the reduction") is unmet: the cited tests anchor `Version::tick`, not the `Party::tick -> Version::tick` reduction. The law that pins that reduction, `party_tick_matches_version_tick`, is cited by no roster row, so deleting it turns nothing red (Principle 6: the cheapest passing artifact; a `Party::tick` that ticks the wrong operand is caught only by a doctest and an uncited law).

Evidence:

       340	    SurfaceRow {
       341	        op: "Party::tick",
       342	        prod_tree: Leg::Trans("version_tick_matches_the_oracle"),
       343	        prod_fs: Leg::Trans("event_dominates_local_and_advances"),
       344	        tree_fs: Leg::Trans("replay_matches_across_references"),
       345	    },

    laws.rs:
      2480	    /// The two `tick` entry points agree: `version.tick(&party)` and
      2481	    /// `party.tick(&mut version)` produce the same advance.
      2482	    fn party_tick_matches_version_tick {

Resolution: cite `party_tick_matches_version_tick` on the three `Trans` legs of the `Party::tick` row, mirroring `Party::ticks`. Acceptance: `every_cited_binding_test_exists` resolves the new citation via `laws::registered_names()`; deleting the law from laws.rs turns the roster red.
Construction: replace the body of `Party::tick` (party.rs:180) with `let _ = version;`; `roster_is_total_over_the_public_fn_surface`, `every_cited_binding_test_exists`, and `exclusion_payload_citations_resolve` stay green, and only the doctests and the uncited law fail. Then delete `party_tick_matches_version_tick`: the roster suite is still green.

### surface-roster-6: Five writer-sink rows carry no resolvable name on any leg, and the only evidence behind `Span::encode_to` cannot see endpoint order
- Where: crates/before/src/surface.rs:987-992 (related: crates/before/src/surface.rs:506-511, crates/before/src/surface.rs:745-750, crates/before/src/surface.rs:785-790, crates/before/src/surface.rs:797-802, crates/before/src/surface.rs:60, crates/before/src/surface.rs:74-85, crates/before/src/span/wire.rs:62-74, crates/before/src/codec/tests.rs:805-834, crates/before/src/borsh_impls.rs:262-266)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the five rows; `grep -rn 'encode_to(\|encode_rank_to('` over every tests.rs under crates/before/src and over crates/before/tests and benches hits only codec/tests.rs:821, 826, 831, the Party/Version/Clock cases of `encode_to_matches_encode`; read span/wire.rs:44-74: `encode` and `encode_to` have independent bodies and the doctest builds `Span::new(&v, &v)`; read borsh_impls.rs:262-266 (`BorshSerialize for Span` calls `encode_to`) and every `borsh::to_vec`/`serialize` call in borsh_impls/tests.rs, none of which serializes a `Span`; serde `Serialize for Span` uses `encode()` at serde_impls.rs:100); executed: no
- Seen by: structure-prose, instrument-correctness; refutation: confirmed at medium (the doors are small delegations and a swap fails at the consumer's strict decode, but the roster's contract is breached and no floor catches an all-empty row); history: deliberate-and-holds for the disposition (94880a47: "the writer-sink door (encode_rank_to) stays in the owner-ratified doctest-pinned exclusion family"; 8350c556 gave Party/Version/Clock `encode_to` a named pin and left the others), with the Span doctest's coinciding endpoints anticipated by no record
- Owner-gated: no (the ratified disposition stands; the fix strengthens its premise and adds a floor)
- Witness (witness/results.md): refuted on the exposure clause (run under `--all-features`). Swapping the two writes in `Span::encode_to` is killed by `span_borsh_composes_and_keeps_its_genres` (borsh_impls/tests.rs:998-1056 builds `Span::new(&older, &newer)` with distinct endpoints and routes through `encode_to` at borsh_impls.rs:262-266) and by the `span_borsh_roundtrips` proptest (line 1069 compares `borsh::to_vec(&span)` to `span.encode()`). The doctest at wire.rs:62-70 does use `Span::new(&v, &v)` and the five rows do carry `pins: &[]`, so the name-resolvability half stands as a reading.
- Cross-references: gate-legs-8 (the same five rows).

The rows for `Span::encode_to`, `Rank::encode_to`, `Ranked::encode_to`, `Ranked::encode_rank_to`, and `Version::encode_rank_to` carry `pins: &[]` on all three excluded legs. The `Exclusion` doc names "for each writer-sink door, its doctest pinning byte identity with the buffer door" as the pin, but a doctest has no citable name, so these five rows are mechanically indistinguishable from unbound rows and `exclusion_payload_citations_resolve` has nothing to resolve. Three of the five doors are `write_all(&self.encode())`-style delegations (ranked.rs:185-187, 234-236; version.rs:1091-1093); two have independent bodies: `Rank::encode_to` (rank.rs:421-423, `encode_parts` directly) and `Span::encode_to` (wire.rs:71-74, two `encode_to` writes against `encode`'s `encode()` plus `as_bytes()`). The `Span` doctest uses `Span::new(&v, &v)`, so swapping the two writes leaves it green, and no test, serde path, or borsh test round-trips a `Span` through `encode_to` with distinct endpoints. Principle 6 (the cheapest passing artifact) and surface.rs:60 ("never a bare opt-out"), whose enforcement clause (67-71) has no name to act on here.

Evidence:

       987	    SurfaceRow {
       988	        op: "Span::encode_to",
       989	        prod_tree: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
       990	        prod_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
       991	        tree_fs: Leg::Excluded(Exclusion::NoWireFormatInReferences { pins: &[] }),
       992	    },

    span/wire.rs:
        66	    /// let span = Span::new(&v, &v).unwrap();
    ...
        71	    pub fn encode_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        72	        self.lo.encode_to(writer)?;
        73	        self.hi.encode_to(writer)
        74	    }

Resolution: extend `encode_to_matches_encode` (codec/tests.rs:812) to `Rank`, `Ranked` (both doors), `Span` with distinct endpoints, and `Version::encode_rank_to`, and cite it from the five rows (through `encode_to_row`, or a sibling helper for the rank doors). Then add a roster-level floor in surface_coverage/tests.rs: every row carries at least one resolvable name across its legs and payloads (a citation, a `pins` element, a `license`, a `guard`, or a `bound_at`), so a zero-binding row reads red by construction. Acceptance: the construction below turns the extended proptest red; the new floor fails on the current roster (five rows) and passes once the citations are added.
Construction: edit crates/before/src/span/wire.rs:72-73 to `self.hi.encode_to(writer)?; self.lo.encode_to(writer)`. Run `just test-all` and the doctest leg: every roster and coverage test, the doctest at wire.rs:62-70 (lo == hi), and the serde and borsh suites stay green.

Synthesis note: The witness refutes the entry's exposure claim: the gate's all-features test leg catches the endpoint swap through the borsh suite, so `Span::encode_to` is not unwatched. The entry is kept for the roster half (five rows whose exclusion payloads resolve to nothing, so an all-empty row passes by construction), which is also gate-legs-8; the Construction paragraph's "stay green" sentence is false for the borsh suites. Read the medium as attaching to the roster hole.

### surface-roster-3: `Exclusion::FAMILIES` is a hand-maintained twin of the enum, and a variant missing from it escapes the inhabitation census
- Where: crates/before/src/surface.rs:152-178 (related: crates/before/src/testing/surface_coverage/tests.rs:250-268)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read surface.rs:152-178 and tests.rs:250-268: the census iterates `FAMILIES` only; `family()` is exhaustive over the enum; the enum is `pub`, so an unconstructed variant draws no dead-code warning); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (ebe66966 introduced both without weighing a derived list)
- Owner-gated: no (a `macro_rules!` that emits the enum and the list together is dependency-free; `strum` would be a new dependency and is the owner's call)
- Cross-references: meter-registry-tier2-11, fuzzfit-strategies-8 (the hand-roster pattern).

`every_exclusion_family_is_inhabited` iterates `FAMILIES`; `family()`'s exhaustive match forces an arm for a new variant, but nothing forces a `FAMILIES` entry, so a variant added to the enum and to `family()` but not to the list is exactly the dead category the census exists to catch, and it is never checked. The doc's "keeps the two in one diff" describes reviewer attention, not a check (Principle 5: no hand-maintained enumerations of facts the code can change; prefer a dependency over hand-rolling).

Evidence:

       153	    /// Every family name, for the inhabitation census (an empty family is
       154	    /// a dead category); [`family`](Exclusion::family)'s exhaustive match
       155	    /// beside this list keeps the two in one diff when the vocabulary
       156	    /// widens.
       157	    pub const FAMILIES: &'static [&'static str] = &[

Resolution: derive the list from the enum. Dependency-free: a small `macro_rules!` that takes the variant list once and emits the `enum`, `FAMILIES`, and `family()`. With a dependency (owner's call): `strum::VariantNames` for `FAMILIES` and `strum::IntoStaticStr` for `family()`, `strum` optional under `meter`. Acceptance: adding a variant to `Exclusion` without an inhabitant fails `every_exclusion_family_is_inhabited` (or fails to compile).
Construction: add `Probe { pins: &'static [&'static str] }` to `Exclusion`, add `Exclusion::Probe { .. } => "Probe"` to `family()`, leave `FAMILIES` unchanged, use the variant in no row; `cargo nextest run -p before every_exclusion_family_is_inhabited` stays green.

### surface-roster-11: `#[ignore]`d tests satisfy the in-tree citation checks, including the `GridCap` guard that "must run"; citecheck closes the hole for before, not for suanpan
- Where: crates/before/src/testing/surface_coverage.rs:336-344 (related: crates/before/src/testing/surface_coverage.rs:292-298, crates/before/src/testing/surface_coverage/tests.rs:183-185, crates/before/src/testing/surface_coverage/tests.rs:224-231, crates/surface-scan/src/lib.rs:183, crates/surface-scan/src/tests.rs:85-88, tools/citecheck:332-344, justfile:292)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read the scan: any `#[` line between `#[test]` and `fn` is skipped with `test_pending` still set; `grep -rn '#\[ignore' crates/before/src` finds two ignored tests, codec/tests.rs:1969 and testing/exhaustive/tests.rs:445, neither cited; tools/citecheck:342-343 skips `case.get("ignored")` and runs in the gate's workspace stream and in `just ci`; the citecheck recipe is `--root crates/before` only, and suanpan uses `test_fns`, which has the same rule and whose fixture asserts the ignored `attr_between` admitted); executed: no
- Seen by: adequacy, instrument-correctness; refutation: reframed (true for the in-tree suite and `test_fns`, false for the gate); history: already-known (ffb92968's citecheck records and closes the collection seam, ignored tests explicitly; the in-tree prose and the surface-scan fixture were not updated)
- Owner-gated: no
- Cross-references: tests-other-24 (the same `#[ignore]` hole in the superlinear and inverted-twin rosters).

The module doc says "a citation is satisfiable only by an item the test runner actually executes" and the `GridCap` check's doc says "a premise guard must run, not merely resolve"; both are false for an `#[ignore]`d test, which the scan records like any other. For before the gate's citecheck leg catches it (the nextest inventory excludes ignored tests), so the defect is a doc overclaim in the in-tree suite plus a fixture in surface-scan that pins the admission as correct. For suanpan, whose witness pairs resolve through `test_fns` and which has no citecheck leg, the seam is unguarded (no ignored test exists there today). Principle 2: a guard that resolves but never runs is a dark counter.

Evidence:

       296	/// kernels, and test-support plumbing never enter the haystack, so a
       297	/// citation is satisfiable only by an item the test runner actually
       298	/// executes.
    ...
       338	                    if trimmed.starts_with("#[")
       339	                        || trimmed.starts_with("///")
       340	                        || trimmed.starts_with("//")
       341	                        || trimmed.is_empty()
       342	                    {
       343	                        continue;
       344	                    }

    surface-scan/src/tests.rs:
        85	         #[test]\n#[ignore]\nfn attr_between() {}\n\
    ...
        88	    let want: Vec<&str> = vec!["attr_between", "cfg_before", "plain"];

Resolution: in the (ideally single, per surface-roster-10) witness scanner, note `#[ignore` while armed and either drop the name or return ignored names separately; have the citation tests reject a cited ignored name; change the surface-scan fixture so `attr_between` is asserted ignored rather than admitted; reword surface_coverage.rs:296-298 and tests.rs:183-185 to what the in-tree scan attests (attribution) and name citecheck as the collection authority for before. Acceptance: the construction below reads red in `exclusion_payload_citations_resolve` (or a new `no_cited_test_is_ignored`); the surface-scan fixture change is red-then-green.
Construction: insert `#[ignore = "probe"]` between `#[test]` (semantic_oracle/tests.rs:550) and `fn grid_cap_is_never_reached()` (551). `cargo nextest run -p before` (the in-tree suite alone) stays green while the guard never runs; only `just citecheck` reads red.

### surface-roster-22: The item-grammar catch-all is silent where the type grammar is fail-loud, main.rs overclaims "every public item", and the skip's rationale is inaccurate
- Where: crates/before/surfacecheck/src/extract.rs:174-178 (related: crates/before/surfacecheck/src/main.rs:1-12, crates/before/surfacecheck/src/main.rs:13-22, crates/before/surfacecheck/src/extract.rs:228-261, crates/before/surfacecheck/src/extract.rs:352, crates/before/surfacecheck/src/extract.rs:361, crates/before/src/shape.rs:95-137, justfile:917-918)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`walk_named`'s `_ => {}` swallows `StructField`, `Variant`, `TypeAlias`, and every other `ItemEnum` variant, while `render_type` and `render_args` panic on unhandled shapes; `walk_type` never looks at fields or variants; `grep -nE '^\s+pub [a-z_]+: ' crates/before/src/shape.rs` finds six public fields at 95, 97, 120, 122, 130, 137; the three `pub type` aliases in the tree are all under module-excepted `laws`/`meter`); executed: no
- Seen by: adequacy, instrument-correctness; refutation: reframed (the gate's stated categories at main.rs:13-22 exclude fields, variants, and aliases by design; the overclaim at main.rs:1-3 and 11-12 and the rationale at extract.rs:175 are the concrete defects; censusing shape is an owner-gated extension); history: deliberate-and-holds for the scope, with the premise "aliases of already-walked types" unenforced
- Owner-gated: no for the enumerate-and-panic and the prose; widening the census to fields and variants is the owner's call

main.rs opens with "every public item of `before` is rostered, pinned, or excepted" and "holds every public item to exactly one disposition", then enumerates three categories that exclude struct fields, enum variants, and type aliases. The skip comment says fields and variants are "reached through their types' rows", but a type's rows pin its trait impls and methods, not its shape: adding a `pub` field to `shape::Plateau` or a variant to `shape::Rise` is a breaking API change the gate does not see. The `_ => {}` arm also swallows any `ItemEnum` variant a future rustdoc adds, unlike the rest of the module's fail-loud posture; the justfile sells this jaw as having "no file list to forget", and a silent catch-all reintroduces a forgettable category.

Evidence:

       174	        // Not reachable surface in this crate's grammar: enum variants
       175	        // and struct fields (reached through their types' rows), type
       176	        // aliases (aliases of already-walked types), and the rest of
       177	        // the item grammar.
       178	        _ => {}

    main.rs:
         1	//! Surface totality against rustdoc JSON: every public item of `before`
         2	//! is rostered, pinned, or excepted, checked from the compiler's own
         3	//! account of the public surface.

Resolution: enumerate the deliberately ignored variants explicitly (`Variant`, `StructField`, `TypeAlias`, `Primitive`, `ExternCrate`, ...) and panic on the remainder, matching `render_type`; correct main.rs:1-3 and 11-12 to name the three categories the gate covers; replace the rationale at 175 with the true one (shape is outside the roster's operation vocabulary). Owner's call: record public fields as `Type::field` and variants as `Type::Variant` item rows pinned in `census::ITEMS`, so a shape change reads red. Acceptance: an unhandled `ItemEnum` variant panics naming it in a unit test over a synthetic `Crate`; main.rs claims what the gate covers; or, under the widening, adding `pub extra: u8` to `shape::Plateau` turns `just surface-totality` red until pinned.
Construction: add `pub extra: u8` to `pub struct Plateau` (shape.rs:88) and initialize it where `Plateau` is built. `just surface-totality` and `roster_is_total_over_the_public_fn_surface` stay green.

### surface-roster-12: Two of the three negative probes in the citation-haystack test cannot fail, and its doc cites a scan that no longer exists
- Where: crates/before/src/testing/surface_coverage/tests.rs:130-149 (related: crates/surface-scan/src/lib.rs:137, crates/surface-scan/src/lib.rs:162, crates/before/src/testing/surface_coverage.rs:162, crates/before/src/testing/surface_coverage.rs:310-313)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`grep -rn 'fn parse_impl_self_type\|fn fn_name' crates/before/src` returns nothing; both are declared at crates/surface-scan/src/lib.rs:137 and 162; the scan walks before's `src/` only, so no scanner rule could admit them); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed at low (the positive direction and one negative probe are live); history: deliberate-but-expired (both helpers lived in before's src/ at a6746a65 and moved to the shared crate in 97d8f2b1 the next day; "the old bare-name scan" was written in 4b136e2b, the commit that replaced that scan)
- Owner-gated: no

The negative direction asserts that `parse_impl_self_type` and `fn_name` are absent from the haystack as proof that helpers cannot satisfy citations; both live in crates/surface-scan and are declared nowhere under crates/before/src, so the scan could admit every `fn` in the tree and these two assertions would still pass. Only `declared_test_names` is a live negative probe. Instruments before cures: a check whose negative half cannot fail is decoration. The doc's "the old bare-name scan" is a ghost reference (listed under surface-roster-20).

Evidence:

       133	/// Two directions. Negative: named non-test helpers — declared `fn`s the
       134	/// old bare-name scan accepted — are absent from
       135	/// [`declared_test_names`], so a roster row whose cited differential test
    ...
       143	    for helper in ["declared_test_names", "parse_impl_self_type", "fn_name"] {

Resolution: probe helpers declared under `src/` that an attribute-blind scan would admit (`declared_test_names_by_file`, `crate_root`, `extract_public_fns` from this module, or a helper `fn` adjacent to a `proptest!` block) and rewrite the doc positively. Acceptance: every name in the negative loop is found by `grep -rn 'fn <name>' crates/before/src`; temporarily removing the `if test_pending` guard at surface_coverage.rs:345 makes every negative probe fail; restored, the test is green.

**Test harness core: bridge, semantic oracle, exhaustive, validation index**

### testing-oracles-22: `check_tick`'s grow-minimality pin has no liveness floor: a fill that always changes the tree skips it on every pair while both entry points stay green
- Where: crates/before/src/testing/exhaustive/tests.rs:313-316 (related: crates/before/src/testing/exhaustive/tests.rs:317-333, crates/before/src/testing/exhaustive.rs:47-50, crates/before/src/oracle/version.rs:247-251, crates/before/src/oracle/version.rs:345-347, crates/before/src/version/skyline/grow/tests.rs:288-343, crates/before/src/version/tests.rs:560, crates/before/src/version/tests.rs:588)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read `check_tick`: nothing counts pairs reaching line 317; `fill_for_test` delegates to `fill` at 345-347; `fill` is the identity for any `Party::Node` on a `Version::Leaf` (251) and for `Party::Leaf(true)` on a leaf (250, `max_ev()` of a leaf is its value), so every leaf event with every nonempty id takes the grow branch; grow/tests.rs:304-343 pins a count but gates on the impl-side `assert_grow_depth_safe`, not on the oracle's `fill_for_test`, so it would not notice an oracle-side regression; version/tests.rs:560 and 588 `prop_assume!` on `fill_for_test` only signal if the identity case is never reached); executed: no
- Seen by: adequacy; refutation: confirmed (the always-differing `fill_for_test` construction passes `exhaustive_small` with `best_inflation` and `all_inflations` executing zero times); history: no-rationale-found for the missing floor; the pin's presence at the deep bound is owner-ruled (#53: "tick with the brute-force grow-minimality pin"), so dissolving the pin is off the table and only the floor is open
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). With `fill_for_test` always returning a changed tree, `exhaustive_small` passes in 8.2 s: every pair takes the `continue` at exhaustive/tests.rs:314-316 and `best_inflation`/`all_inflations` never run. The mutant is not invisible crate-wide: `grow_matches_brute_force` and `grow_minimal` fail (their `prop_assume!` now rejects every case) and `assert_tick` in fill/tests.rs calls `fill_for_test` directly; the pin's vacuity is what the run demonstrates.

Instruments before cures: every criterion needs a liveness floor derived from irreducible work so it cannot pass vacuously when its gate goes dark. exhaustive.rs:47-50 names this pin as the deep run's "irreplaceable value", and the gate that admits pairs to it is an oracle-internal branch probe with no count. The floor's premise is universal and needs no observation: the oracle's `fill` returns every leaf event unchanged for every nonempty id, so the grow branch is taken on at least `ids.len() * (leaf events in evs)` pairs.

Evidence:

       313	            // Grow-branch only: pin the inflation to the global brute-force optimum.
       314	            if ov.fill_for_test(op) != *ov {
       315	                continue; // fill simplified the tree; grow was not taken
       316	            }
       317	            let (best_tree, _cost) = best_inflation(op, ov).expect("non-empty id inflates");

    exhaustive.rs:
        47	//! The deep variant runs the *verdict* pair legs (`is_disjoint`, `covers` —
        48	//! borrowed operands, no allocation) and `tick` with the brute-force
        49	//! grow-minimality pin (the pin's irreplaceable value: `grow`'s DP held to the
        50	//! global optimum over all 65536 deep ids). The *structural* pair legs

    oracle/version.rs:
       249	            (Party::Leaf(false), _) => self.clone(),
       250	            (Party::Leaf(true), _) => Version::Leaf(self.max_ev()),
       251	            (Party::Node(..), Version::Leaf(n)) => Version::Leaf(n.clone()),

Resolution: Count the pairs that reach line 317 (an `AtomicUsize` under the par iter, returned from `check_tick` or asserted inside it) and assert `grow_pairs >= ids.len() * evs.iter().filter(|v| matches!(v, oracle::Version::Leaf(_))).count()`, with the premise stated at the assertion site (fill is the identity on a leaf event for every nonempty id). Acceptance: temporarily making `fill_for_test` return `self.fill(id).tick(&Party::Leaf(true))` (always different from `self`) turns `exhaustive_small` red on the new floor rather than green; restored, it is green.
Construction: In crates/before/src/oracle/version.rs change `fill_for_test` to return `self.fill(id).tick(&Party::Leaf(true))`; run `cargo nextest run -p before exhaustive_small`: every pair hits `continue`, `best_inflation` and `all_inflations` execute zero times, and the test passes. With the floor added, the same mutation fails the floor's assertion.

### testing-oracles-24: `exhaustive_deep` has no recipe and nothing runs it, while `generators.rs` defers deeper coverage to it
- Where: crates/before/src/testing/exhaustive/tests.rs:438-446 (related: crates/before/src/testing/generators.rs:293-298, crates/before/src/testing/exhaustive.rs:41-45, justfile:550, justfile:958, .github/workflows)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -n 'exhaustive\|--ignored' justfile` hits only an unrelated comment at 651; `grep -rn 'schedule\|cron\|--ignored\|exhaustive' .github/workflows/` is empty; the fuzz (justfile:550) and worst-cases (justfile:958) manual tiers have recipes; generators.rs:295-297 reads "deeper coverage is the job of the (ignored) exhaustive variant"); executed: no
- Seen by: adequacy, structure-prose; refutation: confirmed; history: already-known (ruling #53: the test "stays, and stays out of the gate — `#[ignore]`d, run detached on demand"; the ruling is silent on a recipe or cadence, so a recipe is additive; deleting the test would contradict the ruling)
- Owner-gated: yes (the ruling fixes the disposition; a recipe or cadence is a policy addition)
- Cross-references: suite-economics-6 (the two `#[ignore]`d tests no recipe runs).

AGENTS.md makes the justfile the source of truth for verification, with a recipe per artifact. The deep enumeration's invocation lives only in a doc comment, no process executes it, and a documented coverage claim in generators.rs rests on it. A `just` recipe gives the run a name in `just --list`, a place for the nextest caveat, and a target for the deferral.

Evidence:

       438	/// ```text
       439	/// cargo test -p before --release --all-features -- --ignored exhaustive_deep
       440	/// ```
       444	#[test]
       445	#[ignore = "exhaustive deep enumeration: O(corpus^2) over 65536 ids; hour-scale, run detached"]
       446	fn exhaustive_deep() {

    generators.rs:
       295	/// Kept small so the default proptest run stays CI-cheap while still covering
       296	/// every arm; deeper coverage is the job of the (ignored) exhaustive variant
       297	/// and the deep-tree stack-safety test.

Resolution: Add a manual-tier `just exhaustive-deep` recipe wrapping the documented `cargo test` line, its recipe comment carrying the budget and the cargo-test-not-nextest reason; point this doc and generators.rs:295-297 at the recipe; optionally give it a scheduled CI job so the deferred coverage is exercised on a cadence. Acceptance: `just --list` shows the recipe; the doc names it instead of an inline command; generators.rs's deferral names something that runs.
Construction: `grep -n exhaustive justfile` and `grep -rn -- '--ignored' .github/workflows/` return no relevant hit.

**Differential table, generators, asymptotics, compactness, islands**

### testing-diff-gen-14: The generator census pins classes, not arms: three `arb_base` arms have no discriminating class, and the doc claims per-arm detection
- Where: crates/before/src/testing/generators/tests.rs:78-93 (related: crates/before/src/testing/generators.rs:304-332, crates/before/src/testing/generators.rs:320-321, crates/before/src/testing/generators/tests.rs:125-137)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified by tracing each `arb_base` arm against `census_of` (tests.rs:58-63): arm 1 (`0..6`), arm 2 (`any::<u64>()`), and arm 3 (`u64::MAX - 4..=u64::MAX`) reach no class; arm 5 (`(1 << k) + 1`, `k < 96`) reaches only `wide` (for `k >= 64`), which arms 4, 6, and 7 also feed; arm 4's `u128 + u64::MAX` reaches `three_limb` only when the `u128` draw exceeds `2^128 - 2^64`, so `three_limb` and `beyond_narrow` are arm 7's alone; executed: no
- Seen by: adequacy; refutation: confirmed; history: no-rationale-found for the arm gap (28f6981e names exactly the seven classes and never the shifted-power, `any::<u64>`, or straddle arms); the fraction-of-measured floors are deliberate with an inline rationale
- Owner-gated: no for adding classes; yes for changing how floors are derived (the "a quarter to a half of measured" design is stated at tests.rs:89-93)
- Witness (witness/results.md): demonstrated (run). With arm 5's shift range cut from 0..96 to 0..64 the census still passes: the `wide` class is fed by arms 4, 6, and 7 as well and the floors are per class (tests.rs:125-137), not per arm.

A liveness census earns its place by naming what dies red. The doc promises "a dead arm or depth regression reads red here", and generators.rs:320-321 says the census "pins each of these classes alive" after describing the dense range and the `u64::MAX` straddle, yet neither has a floor, `any::<u64>()` has none, and the shifted-power arm can die under the other three wide arms' mass with every counter above its floor. The module doc (generators.rs:287-291) calls this generator "the natural home for the path-sum-overflow regression class"; the straddle arm that carries it is unwitnessed.

Evidence:

        78	/// Every named input class stays under generator mass: sampling the arbitrary
        79	/// strategies under the committed seed meets a positive floor per class, so a
        80	/// dead arm or depth regression reads red here instead of nowhere.

    generators.rs:
       319	/// reads on wide top digits) are inside every random differential's sampled
       320	/// universe, not beyond it. [`tests::generator_classes_stay_under_mass`]
       321	/// pins each of these classes alive.

Resolution: Add classes with a unique feeding arm: `near_max` (`u64::MAX - 4 <= value <= u64::MAX`), `power_plus_one` (`value - 1` a power of two with exponent in `6..96`, disjoint from the dense arm), and `machine_word` (`6 <= value <= u64::MAX - 5`), each with a floor and a comment naming the arm it witnesses; re-state the generators.rs sentence to name exactly the classes the census pins. Owner-gated alternative: derive each floor from the arm's `prop_oneof` weight (weight `w` of 13 per base, at least one base per tree, pinned at half the expectation) and state the derivation beside the number. Acceptance: narrowing arm 5 to `0u32..64` or deleting arm 3 makes the census fail; each floor's comment names the arm it derives from.

Construction: Change generators.rs:328 to `1 => (0u32..64).prop_map(...)` (values at most `2^63 + 1`, never `> 2^64`): `wide` stays far above 650 on arms 4, 6, 7 and every floor holds. Or delete line 326 (the straddle arm): no counter moves.

### testing-diff-gen-23: The fold-door pin roster claims one pin per door that documents the log factor; the island contracts state `log k` for eleven fold doors and five are pinned
- Where: crates/before/src/testing/asymptotics.rs:16-22 (related: crates/before/src/testing/asymptotics.rs:259, 285, 311, 341, 368, crates/before/fuelscape/clock_recv_all.json, crates/before/fuelscape/clock_sync_all.json, crates/before/fuelscape/span_join_all.json, crates/before/fuelscape/span_meet_all.json, crates/before/fuelscape/span_intersect_all.json, crates/before/fuelscape/span_union_all.json, crates/before/src/span/algebra.rs:339-369, crates/before/src/clock.rs:398, crates/before/src/clock.rs:565-571, crates/before/src/meter/board/ceilings.rs:347-350)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`grep -l '"contract".*log k' crates/before/fuelscape/*.json` lists fourteen files; excluding `clock_forks`, `party_forks`, and `version_ticks`, whose `log k` is a different term, eleven balanced-fold doors remain: `clock_join_all`, `clock_recv_all`, `clock_sync_all`, `party_join_all`, `span_join_all`, `span_intersect_all`, `span_union_all`, `span_meet_all`, `version_join_all`, `version_meet_all`, `version_span_all`; the `_log_factor_is_alive` tests are exactly five; span's `*_all` route through `crate::fold::balanced_reduce` (algebra.rs:369) and `Clock::sync_all` through `balanced_try_fold` (clock.rs:398); `Clock::absorb_all` delegates to `Version::join_all` (clock.rs:570)); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: deliberate-but-expired (totality was enforced by the dissolved `FoldLog` class contract, whose span excusal read "law-pinned identical to Version::span_all" at 0a5bdaeb^ complexity_claims.rs:1234-1238; 0a5bdaeb dissolved that machinery and the excusal with it, and 2efff149 then stated `log k` on eleven doors)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (mechanical). Eleven fold-door islands carry a `log k` contract in before-fuelscape/src/ops.rs and asymptotics.rs has exactly five `*_log_factor_is_alive` pins (version_join_all, version_meet_all, version_span_all, party_join_all, clock_join_all); nothing ties the two lists. The left-fold replacement behind `Span::join_all` was not run.

The module's own premise is that a door's wiring can drop the factor without touching the shared core, so an unpinned door is exactly the case the instrument exists for. A roster stated as total but hand-maintained is a hand-maintained count in disguise, and nothing mechanical ties the pin roster to the contract strings: a new `*_all` door with a `log k` contract lands with no pin and no red. `Clock::recv_all` delegates wholly to `Version::join_all`, so its gap is nominal; `Clock::sync_all` and the four `Span` doors have their own wiring.

Evidence:

        18	//! - **Fold doors** (`scan-meter`): one pin per public door whose
        19	//!   rustdoc claims the balanced reduction's `O(D log k)`, each measured
        20	//!   at its own door — the doors share the balanced core, but a door's
        21	//!   wiring (short-circuit arms, per-component walks, the hull's
        22	//!   two-direction carry) can drop the factor without touching the core.

Resolution: Add a `const PINNED_FOLD_DOORS: &[&str]` beside the pins and a totality test that reads `crates/before/fuelscape/*.json`, collects every op whose `contract` contains `log k` under a balanced fold (excluding the forks and `version_ticks` by a stated rule), and asserts each is either pinned or excused with a reason at the check site (`clock_recv_all`: delegates to `Version::join_all`). Then either pin the remaining five doors over the stagger population (versions lifted to spans for the `Span` doors; the stagger clocks for `sync_all`) or excuse each with a reason. Narrow the module doc to what is pinned until then. Acceptance: a test in this module fails when a `fuelscape/<op>.json` contract containing `log k` names a fold door absent from the roster and unexcused; it passes at HEAD only after the six doors are pinned or excused.

Construction: Replace `balanced_reduce` behind `Span::join_all` with a left fold today: its `log k` contract stands with nothing to read red. Conversely, add a new public `*_all` door whose contract says `log k` and implement it as a left fold: the suite stays green.

### testing-diff-gen-26: The fold-door floors are transcribed midpoints; the linear reference is recomputed on every run, printed, and never asserted
- Where: crates/before/src/testing/asymptotics.rs:117-123 (related: crates/before/src/testing/asymptotics.rs:227-239, crates/before/src/testing/asymptotics.rs:249-256, crates/before/src/testing/asymptotics.rs:260, 286, 312, 342, 369, crates/before/src/meter/board/ceilings.rs:56-62)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`door_scan_bits` receives `input_bytes`, prints it, and returns only the scan bits; `assert_log_factor_alive` never sees a byte ratio; `git show 4797009a -- asymptotics.rs` recovers the excised readings, bytes ×4.77/×4.77/×4.77/×4.80/×4.79 against scan ×5.82/×5.82/×5.76/×4.99/×5.32 for join/meet/span/party/clock, and every `MIN_GROWTH` (5.29, 5.29, 5.26, 4.89, 5.05) is the midpoint; the pin commit f0cd4ab2f's own "FINDING 1" records the predecessor floor (4.6) sitting below its linear reference (×5.0), exactly this failure class, cured by re-measurement rather than an in-test check); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (with the caution that today's byte ratios sit under every floor, so nothing is vacuous yet); history: the measured-midpoint design and the print-only harness are owner rulings (f0cd4ab2f "Floors are measured, never transcribed"; 4797009a excised the readings from prose), but no-rationale-found for not asserting the reference the harness has in hand
- Owner-gated: no for asserting the in-run linear reference (it adds a liveness witness inside the ruling); yes for replacing the measured floors with model-derived margins (that reverses f0cd4ab2f)
- Witness (witness/results.md): demonstrated (run). The recomputed linear references are bytes x4.771-4.800 and the scan readings x5.817/5.816/5.764/4.985/5.324, so the five committed `MIN_GROWTH` constants are exactly the transcribed midpoints; today no floor sits at or below its byte reference (the added assertion passes). The gap is that assertion's absence: `door_scan_bits` only prints `input_bytes` and `assert_log_factor_alive` never sees a byte ratio.
- Cross-references: meter-adequacy-5 (the same missing assertion, from the sweep).

Instruments before cures: every floor needs a committed demonstration that the known-bad mechanism fails it. The known-bad here is a fold without the log factor, which reads the population's byte-growth ratio; that ratio is computed in the same run and is the one number that shows the floor discriminates, and it is discarded after an `eprintln!`. A `Shape::StaggerPopulation`, `FOLD_DOOR_TEETH`, or bytes-per-block change that steepens byte growth past a constant makes that pin vacuous with nothing reading red, and the "midway" claim in each pin's doc cannot be checked from the tree, since both endpoints live only in git.

Evidence:

       117	fn door_scan_bits<R>(name: &str, n: usize, input_bytes: usize, run: impl FnOnce() -> R) -> u64 {
       118	    crate::meter::reset_scan_bits();
       119	    std::hint::black_box(run());
       120	    let bits = crate::meter::scan_bits();
       121	    eprintln!("MEASURED {name}: n={n} input_bytes={input_bytes} scan_bits={bits}");
       122	    bits
       123	}
       ...
       230	fn assert_log_factor_alive(door: &str, lo: u64, hi: u64, min_growth: f64) {
       231	    let growth = hi as f64 / lo.max(1) as f64;
       232	    assert!(
       233	        growth >= min_growth,

Resolution: Return `(scan_bits, input_bytes)` from `door_scan_bits` (and the five `*_scan_bits` helpers), and in `assert_log_factor_alive` also assert `bytes_hi / bytes_lo < min_growth` with a message naming it as the linear reference the floor must exceed; this keeps the measured floors and makes the "midway" claim checkable. Owner-gated alternative: assert the door's marginal over the in-run linear reference (`growth / linear >= 1 + margin`) with the margin derived from the model (`log2(2·1024)/log2(2·256) = 11/9 ≈ 1.22` at this quadrupling; the version doors read 5.82/4.77 = 1.22), so a `FOLD_DOOR_TEETH` change needs no hand re-pin of five numbers; the party door's 1.04 marginal then stands out as the witness that needs a sentence or a better population (finding 28). Acceptance: each fold-door pin asserts both `scan growth >= floor` and `byte growth < floor` in the same run; temporarily replacing a door with a linear pass over the population reads red.

Construction: Add `assert!(bytes_hi as f64 / bytes_lo as f64 < MIN_GROWTH)` beside each pin using the `input_bytes` already computed; if it fails for any door today, that pin is already vacuous. Alternatively change `FOLD_DOOR_TEETH` from 64 to 32: the printed `input_bytes` ratio moves and the five floors no longer sit midway between anything, with no message saying the reference moved.

### testing-diff-gen-9: The conviction witnesses reach 2 of 11 `Matches` and 1 of 9 `FsMatches` comparisons; the two non-trivial party comparisons have no known-bad
- Where: crates/before/src/testing/diff_ops/tests.rs:555-566 (related: crates/before/src/testing/diff_ops.rs:99-108, crates/before/src/testing/diff_ops.rs:189-197, crates/before/src/testing/diff_ops/tests.rs:480-527, crates/before/src/testing/diff_ops.rs:627-637)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (counted the impls at diff_ops.rs:77-159 and 179-261; the three known-bad groups at tests.rs:480-527 produce `Version`, `Option<Party>`, and `Event` results only); executed: no
- Seen by: adequacy; refutation: confirmed, severity lowered (seven unwitnessed `Matches` are `self == reference` on plain types with no drift path; the four row `FsMatches` delegate to the Event/Id scans; the live gaps are `Matches<oracle::Party> for Party` and `FsMatches<oracle::Party> for Id`); history: no-rationale-found (the design intent at 20c87ade was to catch a blinded comparison; later impls landed with no coverage decision)
- Owner-gated: no

The doc promises that a comparison "that had gone blind" is caught by the conviction witnesses, but the witnesses exercise only `Matches<oracle::Version> for Version`, `Matches<oracle::Party> for Option<Party>` (through `without`), and `FsMatches<oracle::Version> for Event`. The two comparisons with non-trivial bodies and no witness are `Matches<oracle::Party> for Party` (bridge both ways, used by `party_disjoint_join_matches_the_oracle`) and `FsMatches<oracle::Party> for Id` (the `id_order` scan, used by the same descriptor's fs leg and by `party_shape` through `party_from_rows`). `cargo-mutants` does not reach `cfg(test)` code, so nothing else would notice either returning `true`.

Evidence:

       558	/// Two directions, and the second is what makes the first mean anything. A
       559	/// comparison that had gone blind — a `Matches` implementation that always
       560	/// agrees, a bridge that erases the result — would let the wrong descriptor
       561	/// through, which the conviction witnesses catch. A comparison stuck at

Resolution: Add one known-bad `(disjoint_party, disjoint_party)` group whose production leg forgets the join (`prod: a`, `tree: { a.join(b)...; a }`) and whose fs leg lifts only `a`, with the existing two-direction pattern; state in the conviction tests' doc which comparison impls the witnesses reach. Acceptance: replacing the body of `Matches<oracle::Party> for Party` or `FsMatches<oracle::Party> for Id` with `true` makes a conviction test fail.

Construction: Edit diff_ops.rs:103-107 to `fn matches(&self, _: &oracle::Party) -> bool { true }` and run the `diff_ops` tests: every driver stays green and neither conviction test notices, because no known-bad descriptor produces a `Party` result.

### testing-diff-gen-32: The island include scan cannot see the seven macro-form include sites its own doc credits
- Where: crates/before/src/testing/fuelscape_islands.rs:44-58 (related: crates/before/src/testing/fuelscape_islands.rs:19-21, crates/before/src/testing/fuelscape_islands.rs:63-72, crates/before/src/clock.rs:1012, 1026, 1040, crates/before/src/clock.rs:1049-1050, crates/before/src/version.rs:1547, 1562, 1577, 1640, crates/before/src/version.rs:1587-1588, 1604-1605, 1651-1652, crates/before/src/clock.rs:485, crates/before/src/version.rs:445, 505, 571)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rn '"/fuelscapes/", '` lists the seven macro sites spelling `"/fuelscapes/", $island, ".html"`, so `split_once(".html")` yields `", $island, "` and the charset filter drops it; the matrix invocations pass `"version_join"` (clock.rs:1050, version.rs:1588), `"version_meet"` (1605), `"version_span"` (1652), and each has a literal twin at clock.rs:485 and version.rs:445, 505, 571; the conjunction cells' include at conjunction.rs:154 is literal and visible); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed (adding that an exemption for a matrix-only island would pass the `!included.contains(op)` check at 68-71); history: deliberate-but-expired (the literal-only scan was adequate at b5a81583; 2efff149 introduced the macro include form and the coverage claim at 19-21 without teaching the scan the new form)
- Owner-gated: no

A surface extractor must be total over the syntax it claims, and its doc must state what it checks. The pin passes today only because each matrix island also has a literal include; the doc's claim that the matrices "carry theirs through their generating macros" describes an attachment the scan cannot verify. The main assertion fails closed (a matrix-only island reads red with a misleading "no doc comment includes it"), but the exemption blind spot is real: a stale exemption for a macro-only island is accepted because the include-versus-exempt check never sees the include.

Evidence:

        47	                for site in source.split("/fuelscapes/").skip(1) {
        48	                    let Some((op, _)) = site.split_once(".html") else {
        49	                        continue;
        50	                    };
        51	                    let island_shaped = !op.is_empty()
        52	                        && op
        53	                            .bytes()
        54	                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');

    clock.rs:
      1012	        #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/", $island, ".html"))]

Resolution: Teach the scan the macro form (for each `*_matrix! {` invocation, collect its first string-literal argument as an included island), or route every island include through one `island!("name")` macro whose spelling the scanner recognizes; at minimum state beside the charset filter that macro sites are invisible and that the matrices' islands are held by their method-doc twins, and correct the sentence at 19-21. Acceptance: removing the literal include at version.rs:505 while leaving `binop_matrix!` attached keeps the test green (the scan sees the macro), or the doc says plainly that it will not.

Construction: Add a new matrix island `version_xor` to `fuelscape/index.json` and include it only via `binop_matrix!("version_xor", ...)`: the test fails with "island version_xor is emitted but no doc comment includes it" although one does.

### testing-diff-gen-12: `shape_version_wide`'s distinctness claim fails at `wide == 1`: the raised leaf collapses against its sibling
- Where: crates/before/src/testing/generators.rs:153-158 (related: crates/before/src/testing/generators.rs:166-203, crates/before/src/oracle/version.rs:80-89, crates/before/src/version/skyline/fill/tests.rs:1626-1636)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified by tracing (oracle/version.rs:85-87 collapses equal sibling leaves; `LeftSpine, scale 8, wide 1, at_tip`: `t = leaf(1)`, `k = 1` builds `node(0, leaf(1), leaf(1))` which normalizes to `Leaf(1)`, so seven nodes remain and depth is 7; `Bushy, scale 8` (nine leaves): `bushy_version_with` splits 4/5 then 2/2 and 2/3, so leaves 0/1 and 4/5 are siblings, and `wide_at` is 0 at the tip and `ceil(8/2) = 4` interior, each colliding with its sibling when `wide == 1`); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The doc says raising one counter by `wide` "keeps every leaf base distinct, so the shape and size survive normalization unchanged". `arb_base`'s dense arm draws `1` with probability about 1/13, and on that draw the raised leaf equals its sibling: every spine comes out one level shallower than `scale`, and the bushy shape loses a leaf pair. The consumer (fill/tests.rs:1612-1614) is documented as sweeping "every deep shape at swept depths 8..=128"; on that draw the depth premise is off by one. The differential verdict stays valid (1 is not wide), so this is a false docstring and a slightly weaker family, not a wrong verdict.

Evidence:

       157	/// Raising one distinct counter by `wide` keeps every leaf base distinct, so
       158	/// the shape and size survive normalization unchanged.

Resolution: Raise the chosen leaf by `wide + scale + 1` (plus its counter), which exceeds every other base for any `wide >= 0`, and pin the claim: assert `ev_depth(&to_oracle_version(&result))` equals the depth `shape_version(shape, scale)` produces before returning. Acceptance: a proptest over `arb_shape() × 1..=64 × arb_base() × bool` asserts the depth identity.

Construction: `assert_eq!(ev_depth(&to_oracle_version(&shape_version_wide(Shape::LeftSpine, 8, &Base::from(1u8), true))), 8)` fails (reads 7); the same at `wide = 2` passes.

### testing-diff-gen-15: `op_strategy` draws indices from an unnamed `0..8`, so members beyond the eighth never act; the strategy is undocumented and `Sync`'s doc omits the self-pair no-op
- Where: crates/before/src/testing/optrace.rs:37-46 (related: crates/before/src/testing/optrace.rs:17-19, crates/before/src/testing/optrace.rs:31-32, crates/before/src/testing/optrace.rs:56, crates/before/src/testing/optrace.rs:95-104, crates/before/src/testing/compactness/tests.rs:44)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (all six arms draw `0usize..8`; `i % n` with `i < 8 <= n` is `i`, so indices `>= 8` are selected only after a `Join` shifts them down; `world_strategy` draws up to 29 ops, `world_strategy_up_to(120)` up to 119); executed: no
- Seen by: adequacy, structure-prose; refutation: confirmed; history: no-rationale-found (the range is original to eecf9229; 28f6981e derived `GRID_N` from `MAX_TRACE_OPS` without touching the actor cap)
- Owner-gated: no

Every bound proved over organic populations is a bound over the universe the generator reaches. A hidden cap on which members act shapes that universe (later forks stay at fork-time state) and belongs in the doc or the constant vocabulary beside `MAX_TRACE_OPS`. `op_strategy` is the one undocumented function in an otherwise fully documented module and repeats the literal six times. `Op::Sync`'s doc ("Reconcile `i` and `j`") does not say `i == j` is a no-op in both appliers (97, 144).

Evidence:

        37	fn op_strategy() -> impl Strategy<Value = Op> {
        38	    prop_oneof![
        39	        (0usize..8).prop_map(Op::Tick),
        40	        (0usize..8, 0u8..=6).prop_map(|(i, n)| Op::Ticks(i, n)),
        41	        (0usize..8).prop_map(Op::Fork),
       ...
        17	/// One step of a seed-derived execution. Indices are reduced modulo the live
        18	/// population, so any index is valid and every member descends from one seed via

Resolution: Name the cap (`const ACTOR_INDICES: Range<usize> = 0..8`) with its rationale, or draw from `0..MAX_TRACE_OPS` so any live member can act; document `op_strategy`; append "a self-pair is a no-op" to `Sync`'s doc. Acceptance: the module doc states which members can be actors; the index range is a named constant; `Sync`'s doc matches both appliers.

Construction: A trace of nine `Fork(0)` steps yields ten members; for every later op, `i % n` with `i < 8` and `n >= 9` never selects index 8 or 9, so those clocks stay at fork-time state until a `Join` shifts them below 8.

### testing-diff-gen-18: `comb`'s doc says its closed-form sizes are pinned by this module's tests; the node count is asserted nowhere and the bit formula at one point only
- Where: crates/before/src/testing/compactness.rs:121-123 (related: crates/before/src/testing/compactness.rs:137-138, crates/before/src/testing/compactness/tests.rs:88-101)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`grep -n nodes compactness/tests.rs` hits only the header prose at line 8; re-derived the bit formula: pair subtree `1 + gamma(0) + 1 + gamma(0) + 1 + gamma(2^m - 1) = 2m + 6` bits, spine `2(pairs - 1)`, total `pairs(2m + 8) - 2 = 2,105,342` at (1024, 1024), matching tests.rs:95); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (false for the node count from f2d0011b onward)
- Owner-gated: no

Doc accuracy ("pinned by this module's tests" is false for nodes), and any quantity computable two ways gets a committed test comparing them: the closed forms and the measured `Tier2Size` are two computations of the same numbers, and the builder already computes the bit closed form as its capacity (line 138).

Evidence:

       121	/// both the size envelope and the charge bound. Exact sizes (pinned by this
       122	/// module's tests): `4 * pairs - 1` nodes,
       123	/// `pairs * (2 * m_bits + 8) - 2` bits today. Strict normal form (every

Resolution: In `alternating_combs_hold_the_envelope` (or in `comb` itself) assert `sample.tier2.nodes == 4 * pairs as u64 - 1` and `bits.len() == (pairs * pair_bits - 2) as u64` for every sampled parameter pair; drop "today" from the sentence. Acceptance: both closed forms are asserted on every `arb_comb_params` case; the doc names the test that pins them.

Construction: Change the spine loop at line 141 to `0..pairs` (one extra spine node): the node count becomes `4 * pairs`, `check_sample` still passes on every `arb_comb_params` case, and only the exact `current_bits` at tests.rs:95 catches it at one point while the doc's node formula reads wrong with no test naming it.

### testing-diff-gen-27: The asymptotics pins read process-global counters with no shared-process isolation note or guard
- Where: crates/before/src/testing/asymptotics.rs:230-239 (related: crates/before/src/testing/asymptotics.rs:34-40, crates/before/src/testing/asymptotics.rs:117-123, crates/before/src/codec/scan.rs:21-28, crates/before/src/meter.rs:3549-3551, crates/before/tests/meter.rs:351-355, justfile:109, justfile:114)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified (codec/scan.rs:28 is a `static AtomicU64` with Relaxed ordering and its module doc assumes "one scenario per process"; meter.rs:3549-3551 states the isolation requirement; tests/meter.rs:354-355 appends `ISOLATION_NOTE` to every envelope failure; the gate runs `cargo nextest run` at justfile:109 and 114, so the hazard is outside the gate); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found (the pins never carried the note in either home)
- Owner-gated: no
- Cross-references: suite-economics-1 (the isolation cluster).

Measurements bind to their run. Under a shared-process `cargo test -p before --all-features`, the five scan-meter pins and every other scan-metered test interleave on one counter, so `lo`/`hi` are arbitrary and the failure message diagnoses a documentation problem ("the documented `O(D log k)` overstates"). The meter readers' own docs state the requirement; the consumer here does not carry it forward, and its docs call the counters "Deterministic" (50, 249).

Evidence:

       230	fn assert_log_factor_alive(door: &str, lo: u64, hi: u64, min_growth: f64) {
       231	    let growth = hi as f64 / lo.max(1) as f64;
       232	    assert!(
       233	        growth >= min_growth,
       234	        "{door}'s scan work grew only x{growth:.2} across a x4 population \
       235	         growth ({lo} -> {hi} bits; the log factor reads >= x{min_growth}): \
       236	         the documented `O(D log k)` overstates for this door, so update \
       237	         its `# Complexity` section and this pin together"

Resolution: Mirror tests/meter.rs: append an isolation note to `assert_log_factor_alive`'s and the Display pin's messages and state the nextest requirement in the module doc; optionally fail closed with `assert!(std::env::var_os("NEXTEST").is_some())` at the top of each metered pin. Acceptance: a failing pin's message names the shared-process runner as the first cause to rule out, or the pins refuse to run outside nextest.

Construction: Run `cargo test -p before --all-features --lib asymptotics` (not nextest) on a multi-core machine: the five `_log_factor_is_alive` tests run concurrently, each resetting `SCAN_BITS` while others are mid-fold; the outcome varies run to run and every failure message blames the documentation.

### testing-diff-gen-29: `balanced_terms` replicates `mul_into`'s recentering with nothing tying the two, and names a kernel that never compacts the parked factor in production
- Where: crates/before/src/testing/asymptotics.rs:407-437 (related: crates/before/src/testing/asymptotics.rs:389-395, crates/before/src/version/skyline/query/web.rs:104-171, crates/before/src/version/skyline/query/web.rs:208-214, crates/before/src/version/skyline/query/web.rs:409-415, crates/before/src/version/skyline/query/integral.rs:479-491, crates/before/src/version/skyline/query/integral.rs:705-709, crates/before/src/version/skyline/query/integral.rs:728-729, crates/before/src/meter.rs:2433-2435)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (compared the replica arm by arm with web.rs:156-171: same base-2^32 digits, same `> 1 << 31` threshold, same zero-term skip, same trailing carry; `mul_into`'s doc (web.rs:104-120) says the compacted `digits` operand is "word-scale ... this module's ledgers' reference counts" and both production call sites pass a count (`Base::from(reign.count)` at 211, the ledger `suffix` at 408-412); the wide × dense settle goes through `charge_digits` (integral.rs:728-729), where the parked factor is multiplied whole per cluster (`product = factor.clone(); product *= digit` at 484-485) and the balanced recentering `(sum + (1 << 31)) >> 32` (705-709) applies to the mass's digits; meter.rs:2434-2435 names the settle "parked `−(x − 1)` against the punctured trailing mass"); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: [24]'s boundary claim refuted (the replica matches `mul_into`; the `>=` rule is `WindowMass::combine`, a different kernel), [3]/[33] confirmed (no committed tie); history: no-rationale-found (013334f20 wrote the replica as a content check; the settle path the instance takes is `charge_digits`, which never compacts `x − 1`)
- Owner-gated: no

Any quantity computable two ways gets a committed test comparing them; a replicated kernel inside a pin drifts silently from its original. Beyond drift, the pin's premise misnames its mechanism: "the settle's own balanced-digit compaction (`mul_into`'s recentering, replicated)" describes a compaction production applies only to word-scale reference counts, never to the parked plunge `x − 1`, which `charge_digits` multiplies whole. The 65-term assertion is still a sound incompressibility proxy (a dense factor is not an all-ones run that a smarter algorithm could shift), but it is a reference measure the test defines, not a property of what the settle runs, and its constants `1 << 31`, `1u64 << 32`, `chunks(4)` are unnamed.

Evidence:

       407	    /// The count of nonzero balanced signed digits the settle's own
       408	    /// compaction (`mul_into`'s recentering, replicated) spells a
       409	    /// magnitude into.
       410	    fn balanced_terms(value: &UBig) -> usize {
       ...
       422	        for digit in digits {
       423	            let t = digit + carry;
       424	            if t > 1 << 31 {

Resolution: Either factor the compaction out of `mul_into` into a `pub(crate)` balanced-digit iterator both `mul_into` and this pin consume (then the pin measures a production kernel by construction), or restate the pin: `balanced_terms` is the test's own base-2^32 balanced-digit spelling, a reference incompressibility measure, with `DIGIT_BITS` and `HALF_DIGIT` named and the "settle's own" wording dropped; in either case name which production path the instance settles through (`charge_digits` over the mass digits with the plunge as the whole factor). Acceptance: either `balanced_terms` calls a production function, or its doc no longer claims to be the settle's own compaction and its constants are named.

Construction: Change `mul_into`'s threshold at web.rs:158 from `>` to `>=`: `mul_bound_embedding_is_alive` still asserts 65 and stays green because its replica keeps the old threshold; nothing in the tree notices the two spellings differ.

**Envelopes, first half (tests/meter.rs 1-5305)**

### envelopes-a-2: The segments column has no writer in this binary; every `segments <= 0` pin is satisfied by construction
- Where: crates/before/tests/meter.rs:22-26 (related: 60-65, 364-391, 1212-1243, 1675-1709, 6900-6938; crates/before/src/recurse.rs:17-20, 68-69, 100-109, 118-129; crates/before/Cargo.toml:33-44; crates/before/src/meter/tests.rs:407-454; crates/before/AGENTS.md:26-37)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read `recurse.rs` whole: `SEGMENTS_GROWN` is `cfg(any(test, feature = "meter"))`, its only increment is inside `grow`, and `grow` and `descend!` are `#[cfg(test)]`; `stacker` is under `[dev-dependencies]`; grep shows `descend!` only in `testing/bridge.rs`, `grow/tests.rs`, `meter/tests.rs`; extracted the second argument of all 84 envelope rows in the file, all 0); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed ([17]/[46]; [10]'s proposed rewrite refuted); history: deliberate-but-expired (a defended keep of 2026-07-24 when production recursion existed; the last production recursive walk dissolved at 7b11b3ea (2026-07-26); `grow`/`descend!` became `#[cfg(test)]` at 1ddb5a48 (2026-07-31) with an inline keep decision whose "measured fact" premise now holds only in the lib's own cfg(test) unit tests)
- Owner-gated: yes: the segment meter is a recorded defended keep ("adjudicated once, not relitigated", note line 1756-1758), and dissolving a column is the removal of an instrument
- Witness (witness/results.md): demonstrated (run). In an integration-test binary of the same kind as tests/meter.rs (library linked without `cfg(test)`, `--all-features`), an actual heap-grown stack segment under a 200 000-frame recursion leaves `meter::stack_segments()` at 0, so every `segments <= env.segments` ceiling judges a signal that cannot move in that binary.
- Cross-references: board-ops-render-15, crate-root-32, module-graph-1, recursion-1, inventory-2 (the segments cluster).

An integration test binary links the library without `cfg(test)` and without dev-dependencies, so in `tests/meter.rs` the counter behind `meter::stack_segments()` exists (the `meter` feature) but nothing can increment it: the one `fetch_add` is inside `recurse::grow`, which is `#[cfg(test)]`, as is the `descend!` macro that reaches it. Every one of the 84 rows pins `segments = 0`, and the ceiling cannot fail whatever the library does (a kernel regressed to plain recursion overflows the thread stack at `DENSE_DEPTH = 125_000` and crashes the test; one routed through `descend!` does not compile in this build). The file doc presents the column as a live meter and says its counts "track per-target frame sizes". Every criterion needs a constructible failure it catches that other instruments miss; a ceiling over a counter with no writer is decoration, and the prose overstates what was verified.

Evidence:

        22	//! - **Grown stack segments** ([`meter::stack_segments`]): the deep
        23	//!   traversals grow the stack onto the heap in fixed-size segments that
        24	//!   bypass any allocator meter; the segment counter is the honest stand-in
        25	//!   for recursion-driven stack cost. Process-global, same isolation
        26	//!   requirement.

        64	//! vanish, so the dev-profile pin is the binding one), while segment counts
        65	//! track per-target frame sizes, and the slack absorbs modest variation.

    recurse.rs:
       100	#[cfg(test)]
       101	#[inline]
       102	pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
       103	    if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
       104	        f()
       105	    } else {
       106	        SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);
       118	#[cfg(test)]
       119	macro_rules! descend {

    Cargo.toml:
        33	[dev-dependencies]
        44	stacker = { workspace = true }

    recurse.rs's keep decision:
        17	//! what lets it meet deep inputs safely. The segment counter below stays
        18	//! compiled for the meters: it is the deterministic stand-in for
        19	//! recursion-driven stack consumption, and its zero reading over the library
        20	//! kernels is the measured fact the boards' segments column pins.

Resolution: Present to Finch as a premise change on the 2026-07-24 keep, not a fresh dissolution argument. Two consistent end states: (a) dissolve the `segments` field from `Envelope`, `TouchEnvelope`, `SweepEnvelope`, `QueryEnvelope`, their constructors, tables, and harness asserts, and restate lines 13-26 and 60-65: library traversals are iterative by construction (AGENTS.md's rule), the committed proof is `clock::tests::deep_tree_stack_safety`, and a recursion regression fails the deep scenarios here by overflow; or (b) if a live meter in this binary is wanted, compile `grow`/`descend!` under `any(test, feature = "meter")`, make `stacker` optional behind `meter`, and add a canary beside `heap_meter_registers_known_allocation` that dives through a guarded recursion and asserts `stack_segments() >= 1`, while stating at line 22 that no library kernel can reach the counter, so the zero pin is a rule pin rather than a cost. Under either, correct `recurse.rs:17-20`'s "measured fact" to name the binaries in which the reading is measured (the lib's cfg(test) suite). Acceptance: either no `segments` field or column remains in `tests/meter.rs` and the doc names the depth test as the proof, or a committed canary in this binary fails when the `SEGMENTS_GROWN.fetch_add` line is deleted.

Construction: in this binary no program can make `meter::stack_segments()` nonzero. Add `fn deep(n: usize) -> usize { if n == 0 { 0 } else { 1 + deep(n - 1) } }` and call `std::hint::black_box(deep(200_000))` inside `metered("probe", 0, &envelope::ID_COVERS, ...)`: the test either dies by stack overflow or passes with `segments == 0`; the segments ceiling never fires. Conversely, `grep -rn 'descend!' crates/before/src` shows no non-test library site, so no positive reading is reachable.

### envelopes-a-6: The scan column has no liveness floor in any table, and the DECODE_DENSE row comment names floors its table lacks
- Where: crates/before/tests/meter.rs:262 (related: 211-227, 280-300, 1595-1610, 1716-1721, 6746-6774; 46-49; crates/before/src/codec/scan.rs)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read `Envelope` at 211-227: no scan or touch field; `SweepEnvelope` at 1595-1610 and `QueryEnvelope` at 6746-6774: `scan_bits` ceiling, no scan floor; scan floors in range exist only inside bands at 932 and 5139; `git log -1 --format=%B e4c9b083` carries "touch and scan floors are now those rows' liveness signal", the commit that zeroed the limb columns); executed: no
- Seen by: adequacy; refutation: confirmed; history: already-known in part (2f6df434 chose two exact scan witnesses, `skip_int`/`read_int` exactness and the `id_covers`/`id_disjoint` exact pins, over per-row scan floors "since weak per-row floors clear partial undercounts"; per-column floor-or-NA is the docketed unification); the `DECODE_DENSE` annotation is a blanket sentence from e4c9b083 applied to a table with neither column
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). Removing the validator cursor's topology (unary-run) scan record drops the `SKYLINE_VALIDATE_DENSE`, `ALT_SPINE`, and `DECODE_DENSE` scan readings from 375006 to 125005 bits; every envelope stays green (`sweep_metered` asserts only `scan_bits <= env.scan_bits`; `limb 0 >= 0` holds trivially) and `eq_exit`'s scan pin moves 2054 to 2050, in band. The validate-to-`Ok(())` stub was not run.

`DECODE_DENSE` is a four-column `Envelope` (heap, segments, limb ceiling, limb floor); it has no touch or scan column, yet its comment says those floors "stay the liveness signal". The rows the table comment at 280-287 describes as rows where scan is the one column that sees the work (`SKYLINE_VALIDATE_DENSE`, `SKYLINE_VALIDATE_ALT_SPINE`: limb 0/0, segments constant, heap ceiling, scan ceiling) therefore have no live lower bound at all: a validator stubbed to `Ok(())`, or a deleted `record_bits` tap on the topology advance, leaves every assertion green. The file doc's own argument for the limb tripwire at 46-49 applies verbatim to scan. The recorded r141 decision covers the `skip_int`/`read_int` pair and the id walks, not the topology tap, and does not address a comment naming a floor that does not exist.

Evidence:

       262	    pub const DECODE_DENSE: Envelope = envelope(120_035, 0, 0, 0); // wire decode is validate + wrap on the skyline kernels; decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)

      1595	struct SweepEnvelope {
      1596	    /// Peak heap delta over the scenario body, in bytes.
      1597	    peak_heap: usize,
      1598	    /// Stack segments grown during the scenario body.
      1599	    segments: u64,
      1600	    /// Big-integer limb operations counted during the scenario body.
      1601	    #[cfg(feature = "limb-meter")]
      1602	    limb_ops: u64,
      1603	    /// Packed-stream bits scanned during the scenario body.
      1604	    #[cfg(feature = "scan-meter")]
      1605	    scan_bits: u64,
      1606	    /// Improvement tripwire under the limb column: measured ×0.75, per
      1607	    /// the file doc's tripwire convention.
      1608	    #[cfg(feature = "limb-meter")]
      1609	    limb_floor: u64,
      1610	}

Resolution: Rewrite the `DECODE_DENSE`, `CMP_DENSE`, and `JOIN_DENSE` row comments to state the liveness signal those rows actually have (today: none beyond the heap ceiling; see envelopes-a-9), or move them onto the five-column shape so they gain the scan column. Add a scan floor (the ×0.75 tripwire, or a derived one bit per live input bit where the walk provably reads its whole input, as `id_walk_scan_cost` does) to the sweep and query envelopes under `scan-meter` and pin it for every nonzero row; the docketed unification is the natural vehicle. Acceptance: stubbing `skyline::validate_bits` to `Ok(())` fails at least one `SKYLINE_VALIDATE_*` row; no row comment names a floor its table lacks.

Construction: remove the `record_bits` call on the topology (unary-run) advance in the validator's cursor: `SKYLINE_VALIDATE_DENSE`'s scan reading falls from roughly 375k bits to the leaf-code bits, every scan ceiling stays green, `limb 0 >= 0` holds, `r.is_ok()` holds, and `eq_early_exit`'s scan floor (1_540 on a reading dominated by one 2_049-bit code) moves by a few bits and stays in band. Alternatively stub `validate` to `Ok(())`: the `SKYLINE_VALIDATE_*` and `SKYLINE_DECODE_*` rows read heap near 0, scan 0, limb 0, and pass.

### envelopes-a-14: `Rank::cmp`'s documented O(1) class-first leg is priced only jointly with `checked_sub` and `+`
- Where: crates/before/tests/meter.rs:1344-1354 (related: 1197, 1324-1334; crates/before/src/version/rank.rs:875-906; crates/before/src/meter/board/ops.rs:499-506)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: assessed (read rank.rs:879 "Unequal magnitude classes settle in `O(1)`" and the class-first arm at 896-905; read the board's `rank_pair` cell, which runs the same three operations together; arithmetic from the pinned constants: limb ceiling 87_910 over a ×0.75 floor of 52_746 implies a basis near 70_328 and about 17,500 limb ops of headroom, while the mismatched operand's 500,000-bit exponent is about 7,813 limbs); executed: no
- Seen by: adequacy; refutation: confirmed (with the caveat that the slack arithmetic depends on which comparison the regression routes through: an aligned shift plus `Base::cmp` records about 23k ops and trips the ceiling, an aligned shift plus `msb_cmp` about 7.8k and hides); history: deliberate-but-expired (00cace5d pinned the row as one red baseline for three co-amplifying legs; d8f91040 cured the cmp leg and re-pinned the joint row instead of splitting it)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (arithmetic on the committed row, run under limb-meter). On the row's operands `Rank::cmp` reads 0 limb ops, `checked_sub` plus `+` read 70,328; the ceiling 87,910 leaves 17,582 limb ops of headroom, more than twice the 7,812 a limb-linear class-mismatch compare over the 500,000-bit exponent would cost, so the joint row cannot exclude a linear `cmp`.

Asymptotic claims are hard guarantees per the crate docs, and `rank.rs` documents `cmp` as settling unequal classes in O(1); the only envelope pricing `Rank::cmp` on stored ranks folds that O(1) leg into an O(width) body, so it pins the constant of the whole, not the order of the part. The row comment at 1197 reads as though the number witnessed the O(1) claim. The fuelscape `rank_cmp` widget states a linear contract over the pair and enforces nothing.

Evidence:

      1197	    pub const RANK_PAIR_MISMATCH: TouchEnvelope = touch_envelope(     234_400,        0,      87_910,      0, 52_746, 0); // class-first cmp decides order in O(1); the limb record is checked_sub's and add's mandatory output content plus the metered exponent-alignment shifts

      1348	        || {
      1349	            let ord = a.cmp(&b);
      1350	            let diff = b.checked_sub(&a);
      1351	            let sum = &a + &b;
      1352	            (ord, diff, sum)
      1353	        },

    rank.rs:
       879	/// Unequal magnitude classes settle in `O(1)`:
       902	        let class = |r: &Rank| i128::from(r.num.bits()) - i128::from(r.exp);
       903	        class(self)
       904	            .cmp(&class(other))
       905	            .then_with(|| Num::msb_cmp(&self.num, &other.num))

Resolution: Split the row: a `RANK_PAIR_CMP` envelope whose body is `a.cmp(&b)` alone, with an absolute limb ceiling at the O(1) scale (the two `bits()` reads), plus the existing sub/add row; optionally a two-scale check (`RANK_PAIR_DEPTH` and 2×) asserting the cmp-only limb reading is identical at both depths, which pins the order rather than an envelope. Acceptance: replacing the class-first arm with an aligned shift-and-compare fails the cmp-only row; the sub/add row is unchanged.

Construction: in `impl Ord for Rank`, replace lines 902-905 with `let e = self.exp.max(other.exp); (self.num.clone() << (e - self.exp)).cmp(&(other.num.clone() << (e - other.exp)))` (value-identical). Under the `msb_cmp`-style accounting the limb reading rises by roughly 7.8k to about 78k, under 87_910, touches stay 0, and `rank_pair_mismatch_envelope` passes; under `Base::cmp` accounting it trips, which is the refutation pass's caveat and is why a cmp-only row is the durable instrument.

### envelopes-a-17: The "one-touch-per-operand-byte liveness floor" is undeclared and not universal; the file's own control families fall under it
- Where: crates/before/tests/meter.rs:2933-2960 (related: 3525-3530, 3650-3656, 3740-3745, 3840-3845, 4102-4108, 4819-4824, 4926-4932, 5209-5214; 1194-1195; 4529-4540, 4637-4648; crates/suanpan/src/touch_meter.rs:3-9)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the nine assertion sites and the derived counter-examples at 4529-4540; `touch_meter.rs:8` states a wide operation costs one touch per operand limb; the refutation pass's single nextest run, whose log I read at `<scratchpad>/before/refute-envelopes-a/run1.log`, prints `rank_bigroot ... touches=7194` on `skyline_rank_bigroot input_bytes=13752` and `rank_dense ... touches=5` on `skyline_rank_dense input_bytes=46876`, and binds tightly on the families it is applied to: `skyline_rank_freeze_position_small` 87522 touches on 73328 bytes, `seam_plunge` 10804 on 8834); executed: yes (the refutation pass's run; log read by me, not re-run)
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed by running; history: no-rationale-found (the floor spread with 36afe22e "one-touch-per-byte liveness floors throughout"; the same floor genre was re-derived premise-first for `tick_run` at fe39fca3/248d5539, "wide payload funds at most bytes/8 ... the old x1 was a per-delta derivation asserted per byte, subsidized by narrow-delta families", and the nine sites here were not revisited)
- Owner-gated: no

A liveness floor asserts the mechanism's irreducible work from one universal premise, never the typical work of the families at hand. A leaf whose code is w bits wide contributes about w/4 stored bytes but folds about w/32 digits, so on wide-code operands touches fall under bytes: `v.rank()` on the file's own Bigroot and Dense controls reads 7,194 touches over 13,752 bytes and 5 over 46,876. The floor holds on the nine families only because their codes are narrow relative to their fold work, and it binds within 19-22% on FreezePosition and SeamPlunge, so a legitimate 20% touch improvement there trips a "dead meter" diagnosis. The file already shows the correct form at 4529-4533.

Evidence:

      2933	    /// One `Version::min_ticks` run over a packed family shape, with
      2934	    /// the family's closed-form tick total as the semantic leg and the
      2935	    /// one-touch-per-operand-byte liveness floor.
      2955	        assert!(
      2956	            run.touches >= run.bytes,
      2957	            "min_ticks at {bytes} operand bytes: {} digit touches under the \
      2958	             one-per-byte floor: the fold's accumulator work is not metered",

      4529	        // The liveness floor is the mechanism's irreducible work, not
      4530	        // the family's typical work: every nonzero stored delta folds
      4531	        // into the integral's accumulator at least once, and the

Resolution: Replace `touches >= bytes` at the nine sites with a floor derived per family from its nonzero stored delta count (each generator's layout doc yields the count, as the weight-comb and freeze-parade runs already do; the in-tree precedent is fe39fca3/248d5539), passing the count in from the generator; or, if one shared floor is preferred, state the class premise once (dense topology, narrow codes) and restrict the helper to families that satisfy it by construction. Acceptance: every `touches >=` floor in `skyline_flatness` and `ledger_wide_arming` names the irreducible-work premise it rests on at the assertion site; `grep -n 'touches >= run.bytes\|touches >= bytes' crates/before/tests/meter.rs` returns nothing.

### envelopes-a-3: The isolation premise is prose only; a shared-process runner can reset a counter mid-body and mask a regression
- Where: crates/before/tests/meter.rs:354-355 (related: 15-21, 363-368, 1212-1218, 1675-1681, 6900-6908; crates/suanpan/src/touch_meter.rs:16-20; justfile:109, 114)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (grep for `NEXTEST` and `RUST_TEST_THREADS` across `crates/before`, `crates/suanpan`, `tools`, `justfile`, `.config` finds nothing; the gate recipes at justfile:109 and 114 run `cargo nextest`; both AGENTS.md files prescribe nextest); executed: no
- Seen by: instrument-correctness (adequacy raised it as an open question); refutation: confirmed, downgraded to low; history: no-rationale-found (380d470e wired `ISOLATION_NOTE` into failure messages; no mechanical guard was considered)
- Owner-gated: no
- Cross-references: envelopes-b-10, suite-economics-1, meter-registry-tier2-19, testing-diff-gen-27 (the isolation cluster).

All five counters are process-global atomics reset at the start of each metered body, and the suite relies on nextest's one-process-per-test model for meaning. The only defense is a note appended to failure messages, which by construction never appears on a masked pass: under `cargo test --test meter` (parallel threads) another test's `reset_limb_ops` or `reset_peak_usage` landing inside a scenario truncates its reading, and a row past its ceiling can read under it. The gate is safe (nextest); a contributor's inner loop is not. Every hole becomes a committed check, never a convention held in memory.

Evidence:

       354	const ISOLATION_NOTE: &str = "note: the meters are process-global and meaningful only one \
       355	     scenario per process: run under cargo nextest, not a shared-process cargo test";

        17	//!   allocator exists per test binary, and the counters are process-global,
        18	//!   so per-scenario peaks are meaningful **only under nextest's
        19	//!   process-per-test isolation** — this workspace's runner. Under a runner
        20	//!   that shares one process across tests, concurrent allocation would bleed
        21	//!   between scenarios.

Resolution: Add one `fn require_isolation()` called at the top of every harness (one site once envelopes-a-4 lands) that panics with `ISOLATION_NOTE` unless `std::env::var_os("NEXTEST").is_some()` or `RUST_TEST_THREADS` is `1`. An in-flight `AtomicUsize` that panics on a second concurrent metered body is a cheaper partial guard but misses setup-phase bleed (generators record scan and limb work while building shapes), so the environment check is the one that closes the hole. Acceptance: `cargo test -p before --all-features --test meter` fails fast with the isolation message before any measurement; `cargo nextest run` and `RUST_TEST_THREADS=1 cargo test` behave as today.

Construction: temporarily double one kernel's limb recording so `decode_bigroot_envelope` exceeds its limb ceiling, then run `cargo test -p before --all-features --test meter -- --test-threads=8` several times: on runs where another scenario's `reset_limb_ops` lands inside the body the row passes; under nextest it fails every time.

### envelopes-a-22: Three accumulator-mechanism bands rest on an uncommitted "local probe build" as their known-bad demonstration; the eq early-exit band's demonstration is recorded but uncited
- Where: crates/before/tests/meter.rs:4479-4498 (related: 4544-4555, 4652-4665, 4737-4747; 5070-5078; .cargo/mutants.toml:48-51; crates/suanpan/src/accumulator/tests/metered.rs:31, 92, 406; crates/before/src/version/skyline/query/tests.rs:1495, 1806, 2169, 2197, 2641)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read the three band constants' docs and the section header; the suanpan row witnesses named at 4495-4497 exist at metered.rs:31, 92, 406; the sibling bands' `_reads_superlinear` kernels resolve in query/tests.rs; `.cargo/mutants.toml:48-51` records `sweep::eq_exit`'s `||`-guard mutant as killed by the eq_exit row; `tests/superlinear_tripwires.rs` rosters the `_reads_superlinear` genre by name); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed ([11]) and reframed ([44]: the eq early-exit demonstration is durable and recorded, only the doc citation is stale); history: no-rationale-found for the uncommitted kill switches; already-known for eq_exit (bff7b04c measured the mutation; the mutants roster records the kill)
- Owner-gated: yes: committing a probe as a gated known-bad path is a design choice
- Cross-references: meter-registry-tier2-7 and meter-adequacy-6 (the probe-build cluster), envelopes-b-18 (the same genre for the fold bands).

Every criterion needs a committed demonstration that a known-bad mechanism fails it; a probe build that lived on one machine, with its readings in commit messages, cannot be re-run when a band's ceiling is re-pinned. The suanpan row witnesses pin the three mechanisms crate-locally but do not demonstrate that these before-level bands fail when a mechanism is absent, and no mutants-roster entry names them. The eq early-exit band's doc at 5073-5077 should cite the roster entry rather than a pin-time aside.

Evidence:

      4487	    // `before` operation superlinear while the family's input stays
      4488	    // linear (demonstrated by disabling exactly one mechanism in a
      4489	    // local probe build, value-identical by the full differential
      4490	    // suite; the probe readings live in the pin commits). On the

      5073	    /// The exit-discipline mutation this row owns (`eq` sweeping a
      5074	    /// decided question to exhaustion — value-equivalent, work-only)
      5075	    /// reads tail-linear on the same pairs: orders over this ceiling
      5076	    /// and growing with the doubling \[measured under the live
      5077	    /// mutation, same harness, at pin time\].

    .cargo/mutants.toml:
        48	# Deliberately absent: sweep::eq_exit's `||`-guard mutant and
        49	# integral::meter_product's tap deletion. Their owning instruments kill
        50	# them — the eq_exit early-exit row and the settle-products liveness
        51	# floor in tests/meter.rs — so they need no exclusion. Likewise absent:

Resolution: For the three accumulator bands, either name per band the cargo-mutants mutant (file, function, replacement) that disables the mechanism so the demonstration reproduces from `cargo mutants -f crates/suanpan/src/accumulator.rs -F <pattern>` under the campaign configuration, or commit each disabled-mechanism variant as a suanpan `_reads_superlinear` witness and cite it here as the sibling bands cite theirs. For eq early-exit, replace 5076-5077 with a citation of the mutants roster's `sweep::eq_exit` entry. Acceptance: each of the four bands' docs names a committed artifact (a test or a rostered mutant) that reads superlinear on its family; `grep -n 'local probe build\|at pin time' crates/before/tests/meter.rs` returns nothing.

Construction: run `cargo mutants --workspace -f crates/suanpan/src/accumulator.rs` filtered to the zero-run certificate consumption path and confirm `skyline_rank_weight_comb_is_flat_per_unit` is among the killers; if no single mutant disables the mechanism, the committed-witness route is the one that makes the demonstration durable.

### envelopes-a-9: The public dense rows assert nothing about their result and pass a no-op body
- Where: crates/before/tests/meter.rs:443-463 (related: 262-264, 987-994, 1072-1079, 1281-1322, 1389-1395; 1760-1764, 1926; crates/before/src/meter/board/ops.rs:235-238)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (read the bodies: `cmp_dense_envelope` ends in `consumed(r)` with no verdict check, `join_dense_envelope` in `drop(joined)`; their rows are heap ceiling, tautological segments ceiling, and `limb 0 <= x <= 0`; the kernel twins at 1760-1764 and 1926 do assert verdicts and byte-identity; the board's `version_cmp` cell likewise returns the verdict unasserted); executed: no
- Seen by: adequacy; refutation: confirmed; history: no-rationale-found (`consumed(r)` dates to 0d1ea490 and survived the flag-day re-pin; the kernel rows added the next day assert verdicts)
- Owner-gated: no

A `partial_cmp` that returns `None` at once, or a `|` that returns a refcount clone of `v`, reads heap near zero and passes every assertion in `CMP_DENSE`, `JOIN_DENSE`, `CMP_BIGROOT`, `CMP_CLIFF`, and the rank rows that end in `consumed(r)`. The cheapest artifact that passes must be the intended one; a wiring change that short-circuits the public door would read here as an improvement with every column green. The differential suites carry correctness elsewhere, but these rows then assert nothing about the implementation they claim to price.

Evidence:

       446	    let r = metered("cmp_dense", p.bytes.len(), &envelope::CMP_DENSE, || {
       447	        v.partial_cmp(&Version::new())
       448	    });
       449	    consumed(r);
       450	}
       459	    let joined = metered("join_dense", p.bytes.len(), &envelope::JOIN_DENSE, || {
       460	        &v | &one
       461	    });
       462	    drop(joined);

Resolution: Add value legs from the generator's closed forms: `assert_eq!(r, Some(Ordering::Greater))` for `cmp_dense`, `cmp_bigroot`, `cmp_cliff` (each dominates the empty version); `assert_eq!(joined, expected)` for the join rows where `expected` is built outside the window through the same public operator or, for `join_dense`, the derivation that `S(d)` is 0 everywhere except one leaf at 1, so its pointwise max with the flat 1 is the flat 1 by canonical uniqueness; for the rank rows, the closed-form rank where the public API expresses it, or at least `r == v.rank()` computed outside the window. Acceptance: replacing the metered body of `cmp_dense` with `None` or of `join_dense` with `v.clone()` fails the test on its value leg.

Construction: change the closure at 446-448 to `|| None::<Ordering>` (or 459-461 to `|| v.clone()`): peak heap reads about 0 against 30_720 (or 130_277), segments 0 against 0, limb 0 within 0..=0; `consumed`/`drop` accept anything; the test passes.

### envelopes-a-12: `heap_meter_floor_on_decode_dense` measures the construction-language transcode, not a decode, on a premise that no longer holds
- Where: crates/before/tests/meter.rs:328-347 (related: 410-413, 431-438; crates/before/src/meter.rs:116-121; crates/before/src/version/skyline/encode.rs:23-24; crates/before/src/version.rs:1110-1126)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (the body is `version_of(&p)` = `p.version()` = `Version::from_bits(skyline::encode_bits(self.as_bits()))`, the transcoder; no `Version::decode` runs; the stored dense stream is 46,876 bytes (refutation run log, `skyline_rank_dense input_bytes`) against the floor's 62,501 (`p.bytes.len()`), so the floor holds through `encode_bits`'s `with_capacity(bits.len())` output buffer and offsets stack, not the stated copy); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed, low; history: deliberate-but-expired (at 380d470e `version_of` was `Version::decode(&p.bytes[..])`, so the doc was literally true; the flag day changed the helper and not the canary)
- Owner-gated: no

The doc's invariant ("the decoded version owns a copy of the packed bits") belongs to `Version::decode`, whose row (`decode_dense_envelope`, 431-438) this canary does not protect; the measured body is the generator's transcoder, and the floor holds for a reason the comment does not give. A floor whose stated premise is false holds by coincidence; the heap meter's liveness is already proven by `heap_meter_registers_known_allocation`.

Evidence:

       328	/// The dense-spine decode registers at least its packed input size.
       329	///
       330	/// The decoded version owns a copy of the packed bits, so the one big
       331	/// scenario here has a floor as well as a ceiling, and a dead heap meter
       332	/// cannot slide a big scenario under its envelope at zero.
       338	    let v = version_of(&p);
       340	    assert!(
       341	        peak >= p.bytes.len(),

Resolution: Build `let wire = version_of(&p).encode();` before the reset and meter `Version::decode(&wire[..])` with the floor `peak >= wire.len()` (the read buffer that becomes the storage, version.rs:1111-1126); rename and re-doc to match; or delete the test as redundant with the 1 MiB canary. Acceptance: the canary's body is the operation its name and doc describe, and its floor is derived from that operation's own allocation.

### envelopes-a-21: The quadruple's delta-floor comment miscounts the sparse comb: `n + 1` leaves stated, `3n/2 + 1` built
- Where: crates/before/tests/meter.rs:4436-4439 (related: crates/before/src/meter.rs:3443-3468)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read `sparse_cliff_comb`: per level, even levels emit one zero leaf and odd levels a tooth with two leaves, plus a terminal leaf, so `n` levels give `3n/2 + 1` leaves and `3n/2` delta codes; the generator's own doc at 3443-3444 says "teeth at odd levels only and a plain zero leaf at each even level"); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found (comment and generator landed together in 64ed0d1c; wrong from inception)
- Owner-gated: no

A derived floor is only as good as its derivation. The floor `3·scale` is still a valid lower bound (the pair's delta count is `7n/2`), but the comment states a leaf count the generator does not build, so a reader checking the floor against the generator finds a contradiction.

Evidence:

      4436	        let run = Run {
      4437	            // The sparse comb's n + 1 leaves put n delta codes behind its
      4438	            // first; the full comb adds 2n.
      4439	            deltas: 3 * scale as u64,

    src/meter.rs:
      3456	    for level in 0..n {
      3459	        if level % 2 == 1 {
      3462	            ev_leaf(&mut bits, 0); // tooth's left leaf: value 2^k − 1
      3463	            ev_leaf(&mut bits, 1); // tooth's right leaf: value 2^k
      3464	        } else {
      3465	            ev_leaf(&mut bits, 0); // the even level's plain zero leaf
      3468	    ev_leaf(&mut bits, 0); // terminal spine leaf

Resolution: Set `deltas: 7 * scale as u64 / 2` (the generator asserts `scale` even) and reword: the sparse comb's `n/2` plain leaves and `n` tooth leaves put `3n/2` delta codes behind its first; the full comb adds `2n`. Acceptance: the comment's arithmetic matches `sparse_cliff_comb`'s per-level emission and the constant follows from it.

Construction: count plateaus of `Shape::MaskDriftQuadruple.packed_quadruple(512, 1024).0.0.version()` through `shape::Plateaus`: 1537, not 1025.

**Envelopes, second half (tests/meter.rs 5306-10808)**

### envelopes-b-18: The stagger and scatter fold bands' known-bad mechanism is computed in the harness but never metered, and its demonstration lives in a commit message
- Where: crates/before/tests/meter.rs:7720-7729 (related: 7750, 7661, 7605-7611, 8313-8343; crates/before/tests/superlinear_tripwires.rs:27-68)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`version_fold_run` builds `sequential` at 7750 before `touch_meter::reset()` at 7753; `TRIPWIRE_ROSTER`'s only tests/meter.rs row is `sequential_meet_reduce_reads_superlinear_on_shade`; the board's `declared_fold_model_admits_the_log_factor_and_rejects_quadratic` (board/tests.rs:873-931) feeds a hand-written exponent `sample(n2, k2, 634_600)`, not a live left fold); executed: no
- Seen by: scaffolding, adequacy; refutation: confirmed; history: no rationale found (the join folds were cured in 4f12b8218 before any roster existed; be6dd6a0d committed a red kernel only for `meet_all`, a live defect at the time; 500d4d094 pointed the prose at the pin commit; no ruling exempts the join folds)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run, second iteration). `version_fold_run` builds `sequential` at 7750 before the meters reset at 7753-7755, so it is never metered. The constructed tripwire shows the mechanism is separable today: the sequential `|` fold reads x1.73 per-byte touch growth across the (64,64) to (128,64) arity doubling against the balanced fold's x1.12, clearing the x1.49 class floor the meet tripwire uses; the limb column reads 0 for both folds at this scale, so a committed demonstration should pin touches.
- Cross-references: envelopes-a-22 (the same probe-build genre).

The module comment names the refuted mechanism (the sequential left fold) and defers its readings to the pin commit; the join-fold section at 7605-7611 makes the same claim for the scatter rows. Unlike `meet_fold`, which commits and rosters `sequential_meet_reduce_reads_superlinear_on_shade`, no committed test demonstrates that a sequential `|` fold fails the stagger or scatter bands, so nothing notices if the family stops separating the two mechanisms. Every criterion needs a committed demonstration that a known-bad mechanism fails it; a demonstration in a commit message is not re-run.

Evidence:

      7720	// The known-bad mechanism these bands separate from: the sequential
      7721	// left version fold (`fold(Version::new(), |acc, v| acc | v)`) re-walks
      7722	// its never-coalescing accumulator per input, and the sequential party
      7723	// fold (one `join` per input) re-walks its accumulated region the same
      7724	// way — the growing-accumulator genre the balanced reduction exists to
      7725	// foreclose, quadratic in arity where the reduction is log-linear, so
      7726	// its readings sit several times over these bands with the gap widening
      7727	// with arity (the demonstration readings live in the pin commit); the

Resolution: add `sequential_join_reduce_reads_superlinear_on_stagger` mirroring 8313-8343 (`population.into_iter().reduce(|acc, v| acc | v)` over the version half of `Shape::StaggerPopulation.population(n, m)` on the arity axis at two scales, asserting model-normalized per-byte growth at or above a class-separating floor such as ×1.49), roster it in `TRIPWIRE_ROSTER`, and excise the parenthetical at 7727; do the same for the party left fold if its reading separates. Acceptance: a `_reads_superlinear_on_stagger` kernel in tests/meter.rs, rostered, red on the sequential fold and failing if `join_all` is swapped in; 7727 no longer points at a commit message.
Construction: copy `sequential_meet_reduce_reads_superlinear_on_shade` with `&` -> `|` and the shade population -> the stagger population at `(n, n)` and `(2n, n)`; the module comment predicts about ×2 per-byte growth per arity doubling, so a ≥ ×1.49 assertion should hold; if it does not, the module comment's adequacy claim is the finding.

### envelopes-b-21: `memo_resolution_cost::assert_flat` pins a raw ×2.5 class signature only: six of seven tests have no absolute ceiling, and the doubling the ratio divides by is unchecked
- Where: crates/before/tests/meter.rs:8413-8432 (related: 8443-8453, 8462-8485, 8497-8504, 8564-8586, 8597-8607, 8619-8629; the pattern to follow at 8506-8555; the same raw form at 8772-8778 and 9066-9073; the file header at 7-11; crates/before/src/meter/board/family.rs:770-786)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read all seven tests: only `memo_fanout_wide_cost_is_site_count_independent` asserts an absolute ceiling (8541) and tripwire (8548); family.rs:770-786 lists the memo families and DescendingRaises as envelope-only, so no board cell prices them; run1.log's large readings 88,006 / 57,018 / 89,620 / 65,628 / 56,021 / 996,964 are pinned nowhere; the memo inputs grow ×2.13-2.15 across the "doubling" (6,497 -> 13,985 B; 3,442 -> 7,374; 6,100 -> 12,989; 5,107 -> 10,996) while reveal and ascend grow exactly ×2.0); executed: no
- Seen by: adequacy, scaffolding; refutation: confirmed for the missing ceilings, downgraded the unchecked denominator to nit (today's input ratios make the raw band as tight as the per-byte convention); history: the ratio form is the red-first assert's shape carried through the cure (9de99184c's `>= x3.5` flipped to `<= x2.5` in 5f6af2462); the design note records the state, and only `memo_fanout` was given an absolute ceiling for its k-independence claim; no commit or note rules against absolute ceilings on the other six, and the file header's "every scenario here pins the current measured cost" does not hold for this module
- Owner-gated: no
- Witness (witness/results.md): inconclusive. By reading, tests/meter.rs:8425-8426 asserts the raw cross-scale ratio with no absolute ceiling and no per-byte normalization, and the input doubling is asserted nowhere in the helper; the constant-inflation kernel mutation was not run.

A ratio alone pins the class, never the constant: a change that reads every ledger link twice (still linear) keeps every ratio at ×2.0 and passes all six tests, against the header's promise that "A regression fails loudly now; each improvement tightens a committed number". Every other band module in the range pairs its ratio with an absolute measured ×1.25 ceiling, and `memo_fanout` already does so within this module. Separately, the ratio is raw (`large.touches * 2 <= small.touches * 5`) with the input doubling asserted only in prose ("the input doubles"), while sibling checks in the same module normalize per byte; the band's meaning rests on a generator property nothing checks.

Evidence:

      8413	    /// Assert the linear signature: touches grow by at most ×2.5
      8414	    /// across a size doubling.
      8415	    ///
      8416	    /// A linear resolution reads ×2.0 (the input doubles); a
      8417	    /// resolution that re-reads links once per crossing reads ~×4. A
    ...
      8425	        assert!(
      8426	            u128::from(large.touches) * 2 <= u128::from(small.touches) * 5,
      8427	            "{name}: touch growth across the doubling exceeds x2.5 \
      8428	             ({} -> {}): a ledger link is being read more than once",

Resolution: give each of the six ratio-only tests an absolute touch ceiling (measured ×1.25, rounded up) and a ×0.75 tripwire at its larger run, following `MEMO_FANOUT_TOUCH_CEILING`/`MEMO_FANOUT_TOUCH_TRIPWIRE`; change `assert_flat` (and the inline reveal/ascend growth checks) to the per-byte form `large.touches * small.input * 4 <= small.touches * large.input * 5`, or assert the input ratio it assumes. Acceptance: each test in the module asserts an absolute ceiling at its larger run; every growth band in `memo_resolution_cost` and `width_circulation_cost` divides by `input` or pins the input ratio; a constructed ×2 inflation of the resolution's per-link touches trips at least one ceiling while leaving the ratios green.
Construction: in the frame ledger's site resolution, fold each ledger link into the raise decision twice. `memo_chain_distinct`'s touches roughly double at both scales (ratio about ×2.0, under ×2.5); the `input / 8` floor is trivially satisfied; the per-byte controls are unchanged. All six pass; only `memo_fanout`'s absolute ceiling (73,402) can trip. For the denominator: alter `Shape::MemoChain` so the large run's packed input grows ×1.6 while touches grow ×2.4; the raw band passes (2.4 ≤ 2.5) although per-byte touches rose ×1.5.

### envelopes-b-22: Thirteen cost pins bind to no roster, and the registry excuses their families in prose that misdescribes three of them
- Where: crates/before/tests/meter.rs:8443-8453 (related: 8497, 8531, 8564, 8597, 8619, 8742, 8848, 9043, 9440, 6354, 6378, 7566; crates/before/tests/amp_board_smoke.rs:314-340; crates/before/tests/superlinear_tripwires.rs:87; crates/before/src/meter/registry.rs:1091-1096, 1250-1252, 1463-1466, 1723-1725, 1739-1742, 1756-1758, 1772-1774, 1789-1791; .cargo/mutants.toml:65-69)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`band_test_names` at amp_board_smoke.rs:331 matches only names containing `_is_flat_per_unit` or ending in `_band`; superlinear_tripwires.rs:87 matches only `_reads_superlinear`; the refutation pass's repo-wide grep finds none of the thirteen names outside tests/meter.rs; registry.rs:1723-1725, 1772-1774, and 1789-1791 call the MemoComb, MemoChurn, and DescendingRaises pins "absolute pins outside the band convention" while those tests (8497, 8597, 8619) assert only the ×2.5 ratio at 8426; MemoFanout's reason (1739-1742) is accurate and MemoOscillating's (1756-1758) does not say "absolute"; RevealComb and PureComb are `Priced` by first-half bands (1436, 1454) so the three width-circulation pins here are supplementary; `.cargo/mutants.toml:65-69` cites "the seam-stop pool row in tests/meter.rs" by prose only); executed: no
- Seen by: scaffolding; refutation: reframed (the claim-bearing unrostered pins are the six memo/descending pins cited by registry prose and the seam-stop pool row cited by mutants.toml; three, not four, `Unbanded` reasons misdescribe ratio bands); history: deliberate-but-expired (the `Unbanded` rulings were ratified in cc84df7e on 2026-07-29 after 5f6af2462 (07-25) had flipped the memo pins to two-point ratio bands, so their descriptions were stale at ratification; the Dense ruling "no committed two-point flatness claim" predates `join_all_equal_operands_is_clone_cheap` (4874f527b9, 07-30), a two-point Dense claim)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). With `memo_comb_resolution_reads_linear` deleted from tests/meter.rs, both parity scanners pass (`band_test_names` keeps only `_is_flat_per_unit`/`_band` names; the tripwire scan keeps only `_reads_superlinear`), and the name is cited nowhere under crates/before/src, .cargo, or tools. `just gate` itself was not run.
- Cross-references: envelopes-b-25 (the names that put these pins outside the convention), meter-registry-tier2-3 (the same scanner blind spot from the registry side).

`memo_chain_distinct_resolution_reads_linear`, `memo_comb_resolution_reads_linear`, `memo_fanout_wide_cost_is_site_count_independent`, `memo_oscillating_links_are_input_funded`, `memo_churn_undercuts_fold_one_follower`, `descending_raises_stay_linear_under_min_movement`, `reveal_comb_close_reveal_cycle_reads_width_quadratic`, `pure_comb_width_cycle_reads_width_scaled`, `ascend_cliff_undercut_cascade_reads_residue_width`, `seam_stop_pool_misses_stay_at_warmup_across_churn_doubling`, `id_covers_scan_cost_is_pinned_and_flat`, `id_disjoint_scan_cost_is_pinned_and_flat`, and `join_all_equal_operands_is_clone_cheap` are two-scale bands or liveness pins that no roster names, so any of them can be deleted with every gate leg green. `tests/superlinear_tripwires.rs` states the principle ("an unrostered kernel could be deleted with every gate leg green"); the band parity survivor's totality claim holds only for names spelled to its convention; and the registry's `Bands::Unbanded` ("Why no two-point band exists for this family") is functioning as an escape hatch for a naming choice, with three reasons calling ratio bands "absolute pins".

Evidence:

      8442	    #[test]
      8443	    fn memo_chain_distinct_resolution_reads_linear() {
    ...
      8425	        assert!(
      8426	            u128::from(large.touches) * 2 <= u128::from(small.touches) * 5,

    registry.rs:
      1723	                bands: Bands::Unbanded {
      1724	                    reason: "priced by the memo-resolution gate pins in tests/meter.rs, \
      1725	                             absolute pins outside the band convention",

    amp_board_smoke.rs:
       331	                    if name.contains("_is_flat_per_unit") || name.ends_with("_band") {

Resolution: rename the thirteen to the convention (`..._is_flat_per_unit` for the ratio bands, `..._band` for the equality and liveness pins) and replace each `Bands::Unbanded { reason: "priced by ... tests/meter.rs" }` with `Bands::Priced(&[...])` naming them; alternatively extend `band_test_names` to the `_reads_linear`/`_touches_flat`/`_is_pinned_and_flat` genres. Either way correct the three registry reasons that call ratio bands "absolute pins", and revisit the Dense ruling against the clone-cheap pin. Acceptance: `band_tests_and_registry_citations_stay_paired` fails when any one of the thirteen is deleted or renamed; `grep -c 'absolute pins outside the band convention' crates/before/src/meter/registry.rs` reads 0.
Construction: delete `fn memo_comb_resolution_reads_linear` (8496-8504) and run `just gate`: no leg fails, because neither `band_tests_and_registry_citations_stay_paired` nor `superlinear_tripwires_match_the_committed_roster` scans that name.

### envelopes-b-1: The hoisted-window densify band admits ×1.25 growth its own doc says must be zero
- Where: crates/before/tests/meter.rs:5511-5517 (related: 5453-5459, 5480-5481, 7539-7543, 9449-9453)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read; the refutation pass's run1.log reads `small=84dg/4603B large=84dg/8443B`, so the two readings are equal today); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no rationale found (93741a48 records "84 dg -> 84 dg across the tail doubling (judged absolute: the tail funds no span)" and then applied the ×1.25 convention)
- Owner-gated: no

The constant's doc and the test's doc state the densify column "must not grow across it at all", but the assertion admits a 25% band. An equality pin is available (the readings are equal) and the file already uses one for depth-independence claims in `masked_cmp_hole_depth_band` (7539-7543) and `seam_stop_pool_misses_stay_at_warmup_across_churn_doubling` (9449-9453). The assertion is weaker than the stated invariant.

Evidence:

      5453	    /// Judged absolute, never per byte: the tail doubling adds no window
      5454	    /// density and no settle width, so the densified spans — and this
      5455	    /// column with them — must not grow across it at all. An image sized
      5456	    /// by a cluster's absolute digit position grows with the tail knob —
      5457	    /// roughly ×2 across the doubling — while a span-priced column does not
      5458	    /// move at all.
    ...
      5511	        assert!(
      5512	            large_densify * 4 <= small_densify * 5,
      5513	            "rank's densify column grew more than x1.25 absolute across the \
      5514	             hoisted-window tail doubling ({small_densify} -> {large_densify}): \

Resolution: replace the ×1.25 band with `assert_eq!(small_densify, large_densify, ...)`, keeping the floor and the two absolute ceilings; if the readings are ever not equal, the doc must say why a bounded difference is admissible and pin that difference. Acceptance: the band asserts equality of the two densify readings and passes on the committed readings.
Construction: model a densification that sizes each image `span + position/2048` digits: at tail 10,240 the position term adds about 5 digits and at 20,480 about 10 over the ~84-digit span-priced base (ratio about 1.06), which passes the ×1.25 band and every ceiling while violating the stated invariant; an equality pin fails it.

### envelopes-b-10: The process-per-test premise has no mechanical guard
- Where: crates/before/tests/meter.rs:6900-6909 (related: 354-355, 5372-5374, 7481, 9479-9483; justfile:113-114; src/conformance/backend/tests.rs:31-37 for a precedent)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (grep finds no `NEXTEST_EXECUTION_MODE`, `AtomicBool`, or `Mutex` guard in the file or the tree; `.config/nextest.toml` sets only slow-timeouts; justfile `test-all` runs `cargo nextest run --workspace --all-features`, so the gate is sound); executed: no
- Seen by: instrument-correctness; refutation: reframed (an in-flight guard catches only two overlapping metered windows; unmetered concurrent work still bleeds heap and counters into a metered body, so the complete guard is an environment check at the cost of forbidding a single-threaded `cargo test`); history: the disposition is documentary (0d1ea4905: "the binary's module doc states that requirement"), and the rumors conformance backend uses a static per-test lock that makes plain `cargo test` sound
- Owner-gated: yes (a documented design decision; the guard shape is a trade-off on how the suite may be run)
- Cross-references: envelopes-a-3, suite-economics-1 (the isolation cluster).

Every harness and band `run` resets process-global relaxed atomics and reads them after the body, relying on nextest's process-per-test model documented at 16-21. Nothing checks the premise at runtime: under `cargo test --test meter --all-features` another test's `reset` landing inside a body zeroes a counter mid-flight and a ceiling passes on a partial count; `ISOLATION_NOTE` is appended only to failures, so a false pass carries no note. Every hole becomes a committed check, never a convention held in memory.

Evidence:

      6900	    meter::reset_stack_segments();
      6901	    #[cfg(feature = "limb-meter")]
      6902	    meter::reset_limb_ops();
      6903	    #[cfg(feature = "limb-meter")]
      6904	    suanpan::touch_meter::reset();
      6905	    #[cfg(feature = "scan-meter")]
      6906	    meter::reset_scan_bits();
      6907	    HEAP.reset_peak_usage();
      6908	    let baseline = HEAP.current_usage();
      6909	    let r = f();

Resolution: the owner's choice between (a) one shared entry check that `std::env::var_os("NEXTEST_EXECUTION_MODE")` reads `process-per-test`, panicking with `ISOLATION_NOTE` otherwise (complete, but forbids a single-threaded `cargo test` that is in fact isolation-safe), and (b) the status quo with the premise stated in the file doc. If (a), the check belongs in one helper called by the four harnesses and every band `run` that resets counters directly (5372-5374, 5574, 7481, 8398, 9418), or in `src/meter.rs`'s reset functions. Acceptance: running the binary under a shared-process parallel runner fails deterministically with the isolation message at the first scenario; under nextest all tests pass unchanged.
Construction: two tests in one process, A in `query_metered` and B in `hoisted_window::run`; B's `touch_meter::reset()` at 5372 executes while A's `f()` is mid-walk, so A's `touches` at 6915 reads only the post-reset tail and `touches <= env.touches` passes vacuously; if the reset lands in the final quarter of A's work, `touches >= env.touch_floor` also passes.

### envelopes-b-7: `accum_fan_touches_flat` is byte-identical to the comb test; the fan stream its doc prices is never constructed
- Where: crates/before/tests/meter.rs:6629-6641 (related: 6622-6627, 6706-6713; crates/before/src/meter.rs:342-385; crates/suanpan/src/claims.rs:126-131)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (read both bodies: they differ only in the name string; `git log -S'fn fan_run' -- crates/before/tests/meter.rs` is empty and the test entered in 2b0884c0a in this form; run1.log prints identical MEASURED lines for both, `denominator=100000 touches=200000` and `denominator=200000 touches=400000`; suanpan's claims roster cites `accum_comb_touches_flat` and never the fan); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: the shared ceiling is recorded intent (2b0884c0a: "the fan's entry/exit stream is the same arithmetic and shares the ceiling"), but no rationale exists for a second `#[test]` with an identical body
- Owner-gated: no

The doc says the test prices "The unpaid-crossing fan's entry/exit stream", but the body calls `comb_run(4_096, 50_000)` and `comb_run(8_192, 100_000)` exactly as `accum_comb_touches_flat` does. The closing clause "per delta the two streams are the same arithmetic, and the pinned ceiling says so" is asserted by nothing: the test never consults the `cliff_fan` generator (src/meter.rs:342-385, which describes an enter/leave path-sum stream crossing `2^k` twice per tooth). A test's doc must state the invariant it checks, and this test cannot fail independently of its sibling.

Evidence:

      6629	    /// The unpaid-crossing fan's entry/exit stream — the root magnitude
      6630	    /// paid once, then `±1` path-sum crossings per tooth — stays under the
      6631	    /// same pinned per-delta ceiling, flat across the doubling.
    ...
      6636	    #[test]
      6637	    fn accum_fan_touches_flat() {
      6638	        let small = comb_run(4_096, 50_000);
      6639	        let large = comb_run(8_192, 100_000);
      6640	        assert_flat("fan", &small, &large, envelope::COMB_MILLI_PER_DELTA);
      6641	    }

Resolution: either write a `fan_run(k, n)` that drives the accumulator with the fan's own stream (`add_wide(2^k - 1)` once, then per tooth the enter `+1`/sign/leave `-1`/sign crossings of the `2^k` boundary from the root magnitude, its own denominator), keeping the shared ceiling as the measured claim that the streams cost the same; or delete `accum_fan_touches_flat` and move the one-sentence remark at 6711-6713 into the comb test's doc as the reason no fan row exists. Acceptance: no two `#[test]` fns in `accum_streams` share a body; if a fan row remains, its MEASURED line reports a fan-specific stream and the test fails when its per-tooth deltas are made to widen with k.
Construction: break the fan generator's accumulator stream (make `cliff_fan` emit a sequence on which the accumulator is quadratic): `accum_fan_touches_flat` stays green because it never runs that stream.

### envelopes-b-25: Three green pins carry the names of the red readings they were born asserting
- Where: crates/before/tests/meter.rs:8741-8742 (related: 8848, 9043; the `_reads_linear` names at 8443 and 8497; crates/before/tests/superlinear_tripwires.rs:104-107)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`git log -S`: `reads_width_quadratic` entered in 03f78744c ("reads RED in a committed reading", pinned `>= x3.5`), `ascend_cliff_undercut_cascade_reads_residue_width` in cac71c8ac; the flips 99d302083 and cdb831fa8 rewrote the assertions to `<= x2.5` and removed the `RED PIN` labels but kept the fn names; run1.log reads reveal_comb 30,848 -> 61,696 on 2,758 -> 5,514 B and ascend_cliff 7,432 -> 14,848 on 1,901 -> 3,800 B, exactly linear); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (the memo pins set the precedent the fix wants: 9de99184c named them `_reads_quadratic` and 5f6af2462 renamed them `_reads_linear` at their flip)
- Owner-gated: no

The docs say "gap-funded flat" and "dying-digit-funded flat", the bodies assert `large.touches * 2 <= small.touches * 5` (linear), and the names say the cycle "reads width quadratic", "reads residue width", "reads width scaled". A test's name is the first line of its doc and the string a red run prints; these three state the negation of the invariant, and the roster convention reserves `_reads_` for committed-failing kernels (`..._reads_superlinear_on_...`, superlinear_tripwires.rs:104-107), which the two `_reads_linear` memo names also sit against. The names are also why these pins escape the band roster (envelopes-b-22).

Evidence:

      8727	    /// The reveal comb's close-reveal cycle is gap-funded flat —
      8728	    /// touches grow by at most ×2.5 across the joint (k, b) doubling
    ...
      8741	    #[test]
      8742	    fn reveal_comb_close_reveal_cycle_reads_width_quadratic() {
    ...
      8772	        assert!(
      8773	            u128::from(large.touches) * 2 <= u128::from(small.touches) * 5,

Resolution: rename to the invariant in the band convention (`reveal_comb_close_reveal_cycle_is_flat_per_unit`, `pure_comb_width_cycle_is_flat_per_unit`, `ascend_cliff_undercut_cascade_is_flat_per_unit`, and `memo_chain_distinct_resolution_is_flat_per_unit`/`memo_comb_resolution_is_flat_per_unit` if envelopes-b-21's per-byte form lands), keeping `_reads_superlinear_on_...` exclusively for red kernels, and cite them from their families' `Bands::Priced`. Acceptance: `grep -n 'fn .*_reads_' crates/before/tests/meter.rs` returns only `sequential_meet_reduce_reads_superlinear_on_shade`.

### envelopes-b-15: Three pair rows discard their result without a value leg
- Where: crates/before/tests/meter.rs:7259-7267 (related: 7284-7292, 7334-7342, 7238-7242, 7318-7324, 5984-5988, 422)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read the three bodies and their siblings; `consumed` exists at 422; `pair_run` at 5984-5988 already uses the halves-sum leg); executed: no
- Seen by: scaffolding, instrument-correctness; refutation: confirmed; history: no rationale found (d88be7d938 introduced the lag rows in this shape while giving the distance rows their value legs)
- Owner-gated: no

`version_lag_jump_pair_envelope`, `version_rank_concurrent_envelope`, and `version_lag_concurrent_envelope` return the `(r, ...)` tuple from `query_metered` and drop it, asserting nothing about `r`, while their sibling rows anchor the value (distance equals the two lags' sum at 7238-7242; distance equals rank 2 at 7318-7324). The file's convention, which the lenses list as a positive, is a semantic leg beside every cost pin so a ceiling cannot pass on a wrong answer; these three are the exceptions, and the lag rows have a cheap exact leg (`lag(a, b) + lag(b, a) == distance(a, b)`).

Evidence:

      7259	    query_metered(
      7260	        "version_lag_jump_pair",
      7261	        input_bytes,
      7262	        &query_env::LAG_JUMP_PAIR,
      7263	        move || {
      7264	            let r = a.lag(&b);
      7265	            (r, a, b)
      7266	        },
      7267	    );

Resolution: add the halves-sum leg to both lag rows and a rank closed form (or at least `consumed(r)`) to the concurrent rank row; borrowing closures (`|| a.lag(&b)`) remove the tuple-return shape copied from the distance row. Acceptance: every `query_metered` call in the range either asserts on its returned value or wraps it in `consumed`.

**Other integration suites**

### tests-other-6: The two flatness criteria outside `tests/meter.rs` have no committed known-bad demonstration and sit outside every roster
- Where: crates/before/tests/answer_embedded.rs:19-23 (related: crates/before/tests/answer_embedded.rs:109-121, crates/before/tests/fold_skeleton.rs:40-48, crates/before/tests/amp_board_smoke.rs:331, crates/before/tests/amp_board_smoke.rs:357, crates/before/tests/superlinear_tripwires.rs:6-16, crates/before/src/meter/registry.rs:46-59)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of `wide_base|wide ladder|tiny_tail|deep skeleton|mixed second` over crates/before/src and tests excluding the two files returns nothing; the band parity scan reads only `tests/meter.rs` (amp_board_smoke.rs:357) and matches only `_is_flat_per_unit`/`_band` names (line 331), which neither test name carries); executed: no
- Seen by: structure-prose (open question), adequacy, instrument-correctness; refutation: reframed (the registry's "coverage by bands alone" rule at registry.rs:54-59 governs families, and these populations are not families, so they sit outside that rule rather than in breach of it; the gap is the missing known-bad demonstration and the missing roster answer); history: the closed-form quarter-versus-tenth argument is inline and deliberate (4398dcd4); the "tripwire" vocabulary is consistent with the crate's "improvement tripwire" usage (tests/meter.rs:42-46, twenty occurrences), so the word is not misused; no ruling records that WT/WL/deep-skeleton may stay unregistered
- Owner-gated: yes: the cure is either registration as `Shape`s with band citations (a registry change) or an owner ruling stated at each site
- Witness (witness/results.md): demonstrated (mechanical grep). The only hits for the WT/WL/skeleton vocabulary in registry.rs and tests/meter.rs are the unrelated `answer_embedded_product` band and a `skeleton` word in the weave family's prose; no `FamilyId`, `Shape`, or `_reads_superlinear` kernel corresponds to the WT(n, w)/WL(n, w) grid or the deep skeleton.
- Cross-references: tests-other-14 (the same two files' missing liveness floors), suanpan-tests-25 (the suanpan twin of `assert_no_product`).

The crate's own band discipline (superlinear_tripwires.rs:6-9: each flatness band's adequacy rests on a committed kernel that still reads red through the band's own meters) and Principle 2 (every criterion needs a committed demonstration that a known-bad mechanism fails it) require a demonstration these two criteria lack: nothing in the `_reads_superlinear` roster or beside these files drives a product-law or per-leaf-re-touch fold over the WT/WL grid or the deep skeleton. The analytical separation (a product law reads a quarter; the bound is a tenth) is an argument, not a committed demonstration, and the refutation run shows the mixed difference is exactly 0 on every leg, so the 0.10 bound has never been approached. The tests' names also escape the registry's band parity scan, so no family answers for them.

Evidence:

        19	//! The tripwire is the *mixed second difference* over a 2x2 (n, w)
        20	//! grid: for an additive cost `a*n + b*w` it vanishes; for a product
        21	//! law `c*n*w` it is a quarter of the top cell. Bounding it at a tenth
        22	//! of the top cell refutes any n x w coupling while tolerating
        23	//! amortization wobble.

    (superlinear_tripwires.rs:6-9)
         6	//! Each flatness band's adequacy rests on a committed kernel that
         7	//! demonstrates the refuted mechanism (absolute-position accounting, a
         8	//! schoolbook settle, a sequential reduce, ...) still reads red through
         9	//! the band's own meters — instruments-before-cures, held forever. But

Resolution: Owner call between two shapes. (a) Register WT, WL, and the forked deep spine as `Shape`s with `Bands` answers, move the three criteria into `tests/meter.rs` under the band naming convention so the parity scan sees them, and commit one kernel per criterion under the superlinear roster: for WT the schoolbook settle already driven by `schoolbook_run` in query/tests.rs, for WL a per-rung re-densification, for the D leg a per-frame path-sum fold (the mechanism `bigroot`'s doc names). (b) Keep them bespoke and state at each assertion that its adequacy rests on the closed-form separation, with the measured margin (mixed 0 against a bound of a tenth) recorded as the reason no kernel is committed. Acceptance: (a) `band_tests_and_registry_citations_stay_paired` cites the three bands and `superlinear_tripwires_match_the_committed_roster` rosters one kernel per band, each reading red through the band's meters; or (b) the ruling is stated at the site.
Construction: grep `answer_embedded|wide_base|wide ladder|skeleton` across registry.rs and tests/meter.rs: the only hits are the unrelated `answer_embedded_product` band and memo-chain skeleton prose; no `FamilyId` or `Shape` corresponds to WT, WL, or the forked deep spine, and no kernel is run over them.

### tests-other-10: `coincident_span.rs` pins two of the four clone-identity rungs; the precedence and `contains`-receiver rungs are unpinned anywhere
- Where: crates/before/tests/coincident_span.rs:1-11 (related: crates/before/src/span.rs:255, crates/before/src/span.rs:315, crates/before/src/span.rs:380, crates/before/src/span.rs:453-460, crates/before/tests/meter.rs:9969-9979, crates/before/tests/meter.rs:10552, crates/before/src/span/tests.rs:823-835)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep `ptr_eq` in span.rs: receiver rungs at 255 (place), 315 (dominance), 380 (precedence), 459 (contains receiver), argument rung at 453; this file scans place (63-78), dominance (100-115), and the argument door (166-180) only; meter.rs:9969-9979 measures `precedence` on proper spans `Span::new(&s, &div)`; the `identity_fast_paths` module from 10552 has no `.place(`/`.dominance(`/`.precedence(`/`Span::at(` call; span/tests.rs:823-835 asserts verdict equality with no scan read; `git blame`: span.rs:380 -> b3f09baa (2026-08-06), span.rs:459 -> 22cdfbe1 (2026-08-17); this file's rung witnesses date to 47b03e89 (2026-07-30)); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no rationale found (b3f09baa is a WIP docs-pass commit with no body; 22cdfbe1's message claims scan-meter rungs prove the fast paths but its diff adds only `coincident_argument_collapses_to_the_membership_walk`; 47b03e89 states the failure class, `Bits::ptr_eq -> always false` passing the entire meter binary, which applies to the two later rungs unchanged)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). With the `Span::precedence` clone-identity rung at span.rs:380 replaced by `if false`, all 40 tests in tests/coincident_span.rs, tests/verdict_matrix.rs, `span::tests`, and the meter suite's `identity_fast_paths` and `placement::precedence_bails_at_the_refuted_end` pass. The receiver rung at span.rs:459 was not mutated.

The module doc claims to hold "the coincident-span clone-identity rungs" live, but two rungs added since the file landed have no scan-parity pin: a `ptr_eq` rung deleted at span.rs:380 or 459 leaves every verdict correct through the general arm, so the verdict matrix's `coincident precedence` leg (verdict_matrix.rs:1017) and every law stay green. Principle 6: the cheapest passing artifact (the rung removed) passes the gate today.

Evidence:

         1	//! The coincident-span clone-identity rungs, held live by scan parity.
         2	//!
         3	//! `Span::place` and `Span::dominance` on a clone-coincident span must
         4	//! read exactly the scan bits of the collapsed form they document

    (span.rs:380)
       380	        if self.lo.view().ptr_eq(self.hi.view()) {

    (span.rs:459-460)
       459	            if self.lo.view().ptr_eq(self.hi.view()) {
       460	                return codec::canonical_eq(version.view(), self.lo().view());

Resolution: Add `coincident_precedence_collapses_to_one_containment` mirroring the dominance test (`fast == collapsed` against the single containment the rung documents, `assert_ne!(walked, collapsed)` for the distinct-buffer leg since early exit varies by direction) and `coincident_receiver_and_argument_collapse_to_byte_equality` (`Span::at(&v).contains(&v.clone())` reads strictly fewer scan bits than `Span::new(&v, &redecoded).contains(&v)`; whether `codec::canonical_eq` is unmetered, so the reading is exactly 0, is not settled here, and any strict inequality suffices). Reword the module doc to enumerate the rungs it holds. Acceptance: with `if false {` at span.rs:380 only, the new precedence test reads red and everything else green; the same at span.rs:459 for the receiver test; both green at HEAD.
Construction: Mutate span.rs:380 to `if false {`. `Span::precedence` on a coincident span takes the fused walk and returns the same verdict; verdict_matrix.rs:1017 compares verdicts only; no scan-parity test names `precedence` on a coincident span; the gate is green with the rung deleted.

### tests-other-13: `doc_hidden.rs` pins a per-file count it calls "by name", misses every non-literal spelling, and is dissolvable by a rustdoc flag the pinned nightly accepts
- Where: crates/before/tests/doc_hidden.rs:21-32 (related: crates/before/tests/doc_hidden.rs:1-2, crates/before/tests/doc_hidden.rs:9-10, crates/before/src/party.rs:816-818, crates/before/surfacecheck/src/extract.rs:31-32, crates/before/surfacecheck/src/check.rs:37-48, justfile:937)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: verified (Python replica of the line-32 count: `#[cfg_attr(not(test), doc(hidden))]`, `#[doc(hidden, alias = "x")]`, `#![doc(hidden)]`, and `#[doc( hidden )]` each count 0 and a backticked prose mention counts 1; the two live occurrences are the literal spelling at party.rs:816 and 818, so the roster is accurate today; the refutation pass reports `rustdoc +nightly-2026-06-30 -Z unstable-options --help` lists `--document-hidden-items`, which I did not re-run, and whether the JSON backend honors the flag is unverified); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed; history: no rationale found (8409fe55 called a (file, count) pin "by name" from the first commit; `--document-hidden-items` appears nowhere in the tree)
- Owner-gated: yes: the recommended path retires an instrument and edits the gate's `surface-json` recipe
- Witness (witness/results.md): demonstrated (run). Moving `#[doc(hidden)]` off the sealed trait method `into_id_bits` (party.rs:818) and onto the public `Party::is_seed` (party.rs:157) keeps the count in party.rs at 2 and the roster test passes, so a public inherent method is hidden from rustdoc and from the surfacecheck leg with no committed check noticing. The `cfg_attr` spelling was not separately exercised.

The doc says every hidden item is "pinned by name" but the roster is `(file, count)`: relocating one `#[doc(hidden)]` to a different item in party.rs keeps the count at 2 and passes. The count also matches one literal spelling, so a `cfg_attr`-wrapped or multi-argument `doc(hidden)` hides an item from rustdoc JSON unseen by all three checks. The compiler's own account can enumerate hidden items (`--document-hidden-items`), so the bespoke scan survives only by its own convention (Principle 3).

Evidence:

        21	const DOC_HIDDEN_ROSTER: &[(&str, usize)] = &[("party.rs", 2)];
       ...
        32	            let count = text.matches("#[doc(hidden)]").count();

Resolution: Preferred: append `--document-hidden-items` to the rustdoc invocation at justfile:937, let surfacecheck reach `PartyLiteral` and record its exception with the sealed-trait rationale now at doc_hidden.rs:18-20, update extract.rs:31-32, and delete this file once `just surface-totality` fails on an un-excepted hidden item (the replacement demonstrating it catches what the instrument caught). Fallback: pin the declaration line after each attribute, as foreign_reexport.rs's `(file, line-content)` roster does, and match attribute syntax (`#!?\[(cfg_attr\([^\]]*?)?doc\([^)]*\bhidden\b`) instead of one literal. Acceptance: `#[cfg_attr(not(test), doc(hidden))] pub fn escape()` on any pub item reads red somewhere in the gate; relocating an existing attribute to another item reads red; HEAD passes.
Construction: Remove `#[doc(hidden)]` from `fn into_id_bits` (party.rs:818) and add it to any other public item in party.rs: the count stays 2 and `doc_hidden_occurrences_match_the_committed_roster` passes. Separately, add `#[cfg_attr(all(), doc(hidden))] pub fn escape(&self) {}` to `Party`: rustdoc omits it, the count finds no new occurrence, and the item is invisible to all three checks.

### tests-other-14: The two flatness criteria pass on a dark meter, and the hull fold's limb leg is vacuous today
- Where: crates/before/tests/fold_skeleton.rs:33-38 (related: crates/before/tests/fold_skeleton.rs:85-96, crates/before/tests/answer_embedded.rs:111-121, crates/before/tests/answer_embedded.rs:189-196, crates/before/tests/coincident_span.rs:66-69, crates/before/tests/meter.rs:35-53)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (arithmetic at the cited lines; the refutation pass's run of `cargo nextest run -p before -p suanpan --all-features -E 'binary(fold_skeleton) | binary(answer_embedded)' --no-capture`, whose log I read at <session scratchpad>/refute-tests-other/run1_fold_answer.log; I did not re-run it); executed: yes: that run shows `limb 0` at both skeleton levels and `growth limb: 1.000` passing, and every answer_embedded cell nonzero
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness (six candidate findings merged); refutation: confirmed, with the correction that the limb reading on this population is legitimately zero (no limb record site is reachable from a dense spine of 0/1 heights folded through the accumulator path), so the floor belongs on scan and touch and limb is a model to declare, not a floor to derive; history: no rationale found (4398dcd4 is silent on floors; the audit branch it adopted from has no `growth()`; the owner's ruling of 2026-07-24 in the adversarial note, "a ceiling over a dead counter proves nothing", points the other way)
- Owner-gated: no
- Cross-references: tests-other-6 (the same two files' missing known-bad demonstrations), suanpan-tests-25.

Principle 2: every meter needs a liveness floor so a ceiling cannot pass vacuously when a counter goes dark. `growth()` returns exactly 1.0 when the level-0 counter is zero, converting the dead-meter signal into a passing reading by construction; in answer_embedded the bound is `0.10 * t[3]` and `base * 1.10`, so all-zero readings give `0 <= 0` on both tests. Neither file asserts any counter positive, while `coincident_span.rs` three files over asserts `collapsed > 0, "a zero is a dead meter"` before every comparison. The run confirms the limb assertion in fold_skeleton is decoration today (`limb 0` at both levels), and shows every answer_embedded leg live, so per-leg floors are satisfiable there without carve-outs. The thresholds `0.10` (line 114, rationale in the module doc) and `1.10` (line 193, no rationale anywhere) are inline literals.

Evidence:

        33	fn growth(c0: u64, b0: usize, c1: u64, b1: usize) -> f64 {
        34	    if c0 == 0 {
        35	        return 1.0;
        36	    }
        37	    (c1 as f64 / b1 as f64) / (c0 as f64 / b0 as f64)
        38	}

    (answer_embedded.rs:113-114)
       113	    let mixed = t[3] as f64 - t[2] as f64 - t[1] as f64 + t[0] as f64;
       114	    let bound = 0.10 * t[3] as f64;

    (answer_embedded.rs:192-193)
       192	                assert!(
       193	                    *r <= base * 1.10,

    (run log, refutation pass)
    MEASURED span_all_skeleton lvl0: bytes 11295 scan 368915 limb 0 touch 46243
    MEASURED span_all_skeleton lvl1: bytes 22545 scan 736915 limb 0 touch 92243
    MEASURED span_all_skeleton growth limb: 1.000

Resolution: fold_skeleton: delete the `c0 == 0` branch; assert `scan > 0` and `touch > 0` at level 0 (better, derive them: the hull fold reads every operand at least once, so scan >= 8 * bytes minus per-boundary padding slack; touch >= one per leaf boundary), and either drop the limb leg or assert `limb == 0` at both levels with the model stated at the site (a dense spine of small heights does no `Base` arithmetic). answer_embedded: assert `t[0] > 0` per (op, label) in `assert_no_product` and `per_byte[0] > 0` in the ladder loop, with the crate's "a zero is a dead meter" message; name `MIXED_DIFFERENCE_FRACTION` and `PER_BYTE_GROWTH_BOUND` with their rationale beside them, as `GROWTH_BOUND` is. Acceptance: with `f()` moved above the three resets in `counters` (every reading zero), all three tests read red on a floor; at HEAD they stay green with the MEASURED lines unchanged; the limb leg is either gone or pinned to zero with its reason.
Construction: In `counters` (fold_skeleton.rs:20-30 or answer_embedded.rs:31-41) move `f();` above the three reset calls. Every reading is then 0: `growth(0, b0, 0, b1)` is 1.0 <= 1.35, `mixed = 0 <= 0.0`, and `0.0 <= 0.0 * 1.10`, so all three tests pass with the meters dark.

### tests-other-16: The foreign re-export pin is a substring scan that `pub use <dep>;` and import-then-alias evade, over a manifest parse that reads only a flat `[dependencies]` table
- Where: crates/before/tests/foreign_reexport.rs:78-88 (related: crates/before/tests/foreign_reexport.rs:16-21, crates/before/tests/foreign_reexport.rs:39-41, crates/before/surfacecheck/src/extract.rs:192-204, crates/before/Cargo.toml:23-31)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (Python replica of lines 70-88 with the live dependency keys: CAUGHT `pub use bytes::Bytes;`, `pub use ::bytes::Bytes;`, `pub extern crate bytes;`, `pub type Big = dashu_int::UBig;`; missed `pub use bytes;`, `pub use bytes as b;`, `use bytes::Bytes;` + `pub type Blob = Bytes;`, and each line of a wrapped brace group; false positive `pub use crate::serde_impls::X;`; extract.rs:197-204 returns on a use target absent from `index` after asserting it is in `paths`); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed, with one correction carried here: a single-item three-line `pub use {\n dep::X,\n};` is not an escape at the gate because `cargo fmt --check` normalizes it to one caught line; the wrapped form appears only past the line width, while the whole-crate and import-then-alias spellings are rustfmt-stable escapes; history: no rationale found (9daec56e chose the text pin; b1403c59 wrote extract.rs's "is not `before` surface" comment one day later; neither weighs which layer owns the check; 7f430798 closed exactly the two spellings a verification round named)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). With `pub use bytes;` at the crate root, `dependency_reexports_match_the_committed_roster` passes against its empty roster (the pathed arm requires `bytes::` or `::bytes` on the line), and the in-tree pub-fn roster test passes too. The `pub type Blob = Bytes;` spelling and the `[dependencies.<name>]` hole were assessed by reading; `just surface-totality` was not run.

The module doc says "This pin closes the channel", but the `pathed` arm wants `dep::` or `::dep` on the line, so a whole-crate re-export (`pub use bytes;`, `pub use bytes as b;`) and an import-then-alias (`use bytes::Bytes; pub type Blob = Bytes;`) publish foreign surface unseen, and `::serde` matches a local path. `dependency_names` turns `in_deps` off at any `[` header, so a `[dependencies.<name>]` table or a target-specific table drops that crate without tripping the non-empty floor. surfacecheck already sees the resolved foreign id (`paths[id].crate_id`) and returns silently, which is where a spelling-proof check belongs (Principle 3).

Evidence:

        78	                let pathed = trimmed.contains("pub use") || trimmed.contains("pub type");
        79	                let whole_crate = trimmed.contains("pub extern crate");
        80	                if !(pathed || whole_crate) {
        81	                    continue;
        82	                }
        83	                if deps.iter().any(|dep| {
        84	                    (pathed
        85	                        && (trimmed.contains(&format!("{dep}::"))
        86	                            || trimmed.contains(&format!("::{dep}"))))
        87	                        || (whole_crate && trimmed.contains(&format!(" {dep}")))
        88	                }) {

    (foreign_reexport.rs:39-41)
        39	        if line.starts_with('[') {
        40	            in_deps = line == "[dependencies]";
        41	            continue;

    (surfacecheck/src/extract.rs:197-204)
       197	    let Some(id) = use_.id else { return };
       198	    let Some(target) = krate.index.get(&id) else {
       199	        assert!(
       200	            krate.paths.contains_key(&id),
       201	            "rustdoc JSON has use target {id:?} in neither index nor paths"
       202	        );
       203	        return;
       204	    };

Resolution: In surfacecheck's `walk_use`, when `index` misses and `paths` hits, record `(prefix::name, paths[id].path, crate_id)` as a foreign re-export row, do the same for `TypeAlias` items whose target resolves to a foreign id and for `ExternCrate` items, and reconcile against a committed empty census with the existing exception discipline; then delete this file (the census test is the replacement demonstration Principle 3 requires). If the text pin is kept meanwhile: match the dependency name at a word boundary on `pub use`/`pub type` lines, carry a `pub use ... {` group across lines to its `}`, and accept `[dependencies.<name>]` headers or read the manifest with a TOML parser; reword line 16 to what the scan closes. Acceptance: `pub use bytes;` and `use bytes::Bytes; pub type Blob = Bytes;` at the crate root each read red in the gate; the manifest parse finds the same dependency set when one entry is rewritten in table form.
Construction: Add `pub use bytes;` (or `use bytes::Bytes;` and `pub type Blob = Bytes;`) to crates/before/src/lib.rs. `dependency_reexports_match_the_committed_roster` passes (no line contains `bytes::` or `::bytes`), `cargo fmt --check` is unaffected, and `just surface-totality` passes because `walk_use` returns on the foreign id.

### tests-other-18: The `fuzz_decode_ops` seeds are held byte-identical to the derivation but never re-parsed under the target's framing
- Where: crates/before/tests/fuzz_seeds.rs:7-11 (related: crates/before/tests/fuzz_seeds.rs:341-393, crates/before/tests/support/fuzz_seed_set.rs:263-299, crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:13-15, crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:40, crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:72)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read fuzz_seeds.rs in full: the per-target tests filter on `fuzz_decode` (103), `fuzz_decode_differential` (175), `fuzz_parse` (257), `fuzz_laws` (347); no test names `fuzz_decode_ops`; the target's framing at fuzz_decode_ops.rs:29-38 (flavour byte, one-byte length), 40 (`flavour & 1`), 72 (`op % 8`) is bound to fuzz_seed_set.rs:263-299 only by comments); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed; history: no rationale found (fd5c92bf chose comment cross-pointers; 23e46a7c added a per-framing test for `fuzz_laws` alone; the ops script grew from seven bytes to eight between bb6ea8b7 and HEAD with no test observing the new op)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (reading). tests/fuzz_seeds.rs holds per-seed contract tests for fuzz_decode, fuzz_differential, fuzz_parse, and the laws seeds, plus byte identity and the directory census; nothing asserts anything about the two `fuzz_decode_ops` seeds beyond byte identity, and the target source in the detached fuzz workspace is compiled into no before test binary, so editing its dispatch cannot redden these tests.
- Cross-references: tests-other-22 and fuzz-guests-pins-10 (the framing-duplication cluster).

The module doc promises every seed is held "to the contract it seeds", and the laws test (341-393) shows the intended shape (positional re-parse, in-band arity and pool indices, nothing left over). The two `fuzz_decode_ops` seeds get only byte identity: nothing asserts the flavour byte, the length prefix, that the value bytes decode as a `Clock`, that flavour-0 script bytes lie under the `% 8` table (the comment's "one full lap"), or that flavour-1's tail decodes as a `Version`. A framing change in the target regenerates nothing and reddens nothing; tests-other-26 is the kind of defect such a test would have caught.

Evidence:

         7	//! (`tests/support/fuzz_seed_set.rs`) and hold every seed to the
         8	//! contract it seeds — the decode targets' round-trips, the
         9	//! differential target's per-genre rejection witnesses, the parse
        10	//! target's display round-trips — so format drift is a red gate with a
        11	//! one-command fix (`cargo run -p before --example fuzz_seeds`).

    (fuzz_decode_ops.rs:72)
        72	        match op % 8 {

Resolution: Add `decode_ops_seeds_decode_per_framing`: for each `fuzz_decode_ops` seed, split flavour and length exactly as the target's `run` does, assert the value bytes decode as a `Clock` and re-encode identically, assert flavour-0 script bytes are each below the op-table span and together cover it (the claimed full lap), and assert flavour-1's tail decodes as a `Version` with the relation the derivation claims (after tests-other-26 fixes that relation). Share the span constant with the target per tests-other-22. Also list the ops and laws contract tests in the module doc (lines 7-10 omit both). Acceptance: changing the target's length prefix to two bytes, or dropping an index from the seed's op script, turns `fuzz_seeds` red naming the seed; HEAD (after the seed regeneration) is green.
Construction: Edit fuzz_decode_ops.rs so `drive_clock` dispatches on `op % 7`. The committed `clock_then_ops` seed's trailing byte 7 now aliases `tick`; the seed still matches its derivation byte-for-byte and the directory census is unchanged, so `committed_seeds_match_the_live_derivation` and `seed_directories_hold_exactly_the_set_of_record` stay green and no test observes that the seed no longer drives the op it was written for.

### tests-other-22: The `fuzz_laws` framing constants are transcribed across the workspace boundary and bound only by comments
- Where: crates/before/tests/fuzz_seeds.rs:307-316 (related: crates/before/tests/fuzz_seeds.rs:369-371, crates/before/tests/support/fuzz_seed_set.rs:263-268, crates/before/tests/support/fuzz_seed_set.rs:301-309, crates/before/fuzz/fuzz_targets/fuzz_laws.rs:57-65, crates/before/fuzz/fuzz_targets/fuzz_laws.rs:80-82, crates/before/fuzz/fuzz_targets/fuzz_laws.rs:197-212, crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:72)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read both sides: `const ARITY_SPAN: usize = 18` at fuzz_seeds.rs:308 and fuzz_laws.rs:65; pool sizes 4/3/3 as literals at fuzz_seeds.rs:369-371 against `vpool.len()`/`ppool.len()`/`cpool.len()` at fuzz_laws.rs:197-212; `picks` folds `% ARITY_SPAN` at line 81; `% 8` lives only at fuzz_decode_ops.rs:72 while the op list at fuzz_seed_set.rs:283 depends on it); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: no rationale found (the fuzz workspace's detachment, per fuzz/Cargo.toml:1-3, is a cargo-membership argument that does not bear on a `#[path]` file include; the gate compiles the fuzz targets separately through `fuzz-build`)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). With the target's `ARITY_SPAN` changed to 16 the seed checker stays green because it checks its own copy (fuzz_seeds.rs:308) and never reads the fuzz target; under the mutated target the committed arity bytes 17/16/15 fold to 1/0/15, so the `laws_wide_gamma` seed no longer drives the second-octave crossings (arithmetic, not an executed fuzz run).
- Cross-references: fuzz-guests-pins-10 (the same defect from the fuzz partition), tests-other-18.

`laws_script` guards that each seed byte is in-band "so no seed byte silently aliases a smaller value", but the band it checks is a local copy: if the target's `ARITY_SPAN` moves, the checker keeps its 18 and stays green while the target folds the committed `laws_wide_gamma` scripts (arities 17/16/15) to other arities. The writer and checker cannot drift from each other (shared module), but both can drift from the target they claim to seed, at exactly the constants they transcribe. Principle 6: every hole becomes a committed check, never a convention held in comments.

Evidence:

       307	fn laws_script(name: &str, data: &mut &[u8], pool: usize) -> usize {
       308	    const ARITY_SPAN: usize = 18; // the target's arity band, per its framing
       ...
       313	    assert!(
       314	        usize::from(arity) < ARITY_SPAN,
       315	        "{name}: script arity {arity} is out of the target's arity band"
       316	    );

    (fuzz_laws.rs:65, 81)
        65	const ARITY_SPAN: usize = 18;
        81	    let arity = usize::from(byte(data)) % ARITY_SPAN;

Resolution: Move the framing (`ARITY_SPAN`, the three pool sizes, `chunk`, `picks`, and the ops target's op-table span and flavour mask) into one file, e.g. `crates/before/fuzz/framing.rs`, included by `#[path]` from the fuzz targets, `fuzz_seed_set.rs`, and `fuzz_seeds.rs` (the same mechanism that already shares the derivation between the example and the test); `laws_chunk`/`laws_script` become thin callers. Acceptance: `grep -rn 'ARITY_SPAN: usize = ' crates/before` returns one line; changing it there fails `laws_seeds_decode_per_framing_and_stay_wide` (the arity-17 seed goes out of band) instead of passing.
Construction: Edit fuzz_laws.rs to `const ARITY_SPAN: usize = 16;`. `fuzz_seeds` stays green (its own 18 still admits 17), while under the target the `laws_wide_gamma` seed's version script (arity byte 17) folds to arity 1 and its party script (16) to arity 0, so the seed no longer represents the second-octave crossings the corpus docs claim, with no gate leg red.

### tests-other-24: Three rosters attest that a test is named, not that it is collected: `#[ignore]` on any kernel, twin, or band keeps every roster green
- Where: crates/before/tests/superlinear_tripwires.rs:80-87 (related: crates/before/tests/verdict_matrix.rs:1259-1287, crates/before/tests/amp_board_smoke.rs:323-327, tools/citecheck:14-19, tools/citecheck:78-81, tools/citecheck:332-344)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the three scanners: none inspects attributes; amp_board_smoke.rs:323-327 skips every `#[` line while armed, so `#[test]` then `#[ignore]` then `fn x_band()` counts; tools/citecheck fixes its citation sources at 78-81 (surface.rs, diff_ops.rs, surface_coverage.rs, laws.rs) and refuses ignored tests at 332-344; the tree's two legitimate `#[ignore]`s (codec/tests.rs:1969, exhaustive/tests.rs:445) show a blanket in-tree ban is not viable); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed; history: no rationale found (citecheck was extended over surface_coverage's TRIPWIRES the day it landed, 5c7e2d23, with this exact rationale; the superlinear roster existed then and was not added; the verdict-matrix roster landed the next day without joining)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). With `#[ignore]` on the polarity-flipped twin, the roster test still finds the name through `fn_name` (verdict_matrix.rs:1232-1257 reads `fn` lines only) and passes, while the twin never executes in a normal run; the sibling scanners' blindness was verified by reading.
- Cross-references: surface-roster-11 (the same hole in the in-tree citation checks).

"No mechanism for accepting known failures may exist, even as an empty buffer." The cheapest artifact for a kernel that stops reading red is `#[ignore]`, and that artifact passes all three rosters while the known-bad demonstration never runs. citecheck's own header names the hazard ("no source scan can attest *collection*") and its resolver already treats an ignored cited test as unresolved; it only lacks these citation sources.

Evidence:

        80	                let Some(rest) = line.trim_start().strip_prefix("fn ") else {
        81	                    continue;
        82	                };
        83	                let name: String = rest
        84	                    .chars()
        85	                    .take_while(|c| c.is_alphanumeric() || *c == '_')
        86	                    .collect();
        87	                if name.contains("_reads_superlinear") {

    (amp_board_smoke.rs:323-327)
       323	        if t.starts_with("#[") || t.is_empty() {
       324	            // cfg or other attributes between `#[test]` and the fn keep
       325	            // the arming; anything else below drops it.
       326	            continue;
       327	        }

    (tools/citecheck:342-343)
       342	            if case.get("ignored"):
       343	                continue

Resolution: Extend citecheck's citation sources to the two `TRIPWIRE_ROSTER` tables (tests/superlinear_tripwires.rs, tests/verdict_matrix.rs) and to the registry's `Bands::Priced` and `AXIS_BANDS` names (src/meter/registry.rs), with the same extraction floor and spelling-totality guard the existing sources get, and add a self-test fixture per source; keep the in-crate scans as the both-directions membership pins. If extending citecheck is not wanted, the fallback is for each scanner to capture the attribute run above a rostered fn and refuse `#[ignore` and `#[cfg` there, which re-implements the tool's job in three places (tests-other-3 would put it in one). Acceptance: `#[ignore]` on `sequential_meet_reduce_reads_superlinear_on_shade`, on `polarity_flipped_sweep_reads_inverted_through_the_matrix`, or on any registry-cited band test turns `just citecheck` red naming the test; removing it restores green.
Construction: Add `#[ignore]` immediately above `fn polarity_flipped_sweep_reads_inverted_through_the_matrix()` (verdict_matrix.rs:1370). `inverted_verdict_tripwires_match_the_committed_roster` still finds the name through `fn_name` and passes; the twin is never executed; `just gate` is green.

### tests-other-30: The polarity-flipped twin is pinned per axis, not per strict-order leg, so a leg can be neutralized with every test green
- Where: crates/before/tests/verdict_matrix.rs:1373-1379 (related: crates/before/tests/verdict_matrix.rs:701-719, crates/before/tests/verdict_matrix.rs:1139, crates/before/tests/verdict_matrix.rs:1408-1416)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read 1373-1379 against 1408-1416; traced the construction at 1139 through the production run, the equality twin's leg list, and the polarity twin's axis check; `git log -1 fcb78e44` says "Violations are now counted per (axis, leg), so both twins pin named legs"); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: the landing commit's stated intent was per-leg pinning for both twins; the diff realized it for the equality twin only
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run). With the dominance leg neutralized (`if false && ...`), the production run passes, the equality twin passes (dominance is not among its pinned legs), and the polarity twin passes because `Axis::Placement` still fires through the placement, precedence, and coincident legs.

The checker's doc says the polarity flip "dissents on the strict-order legs of every cross-surface axis", but the test asserts only `axis_count(axis) > 0`, and no roster of legs exists: deleting or neutralizing any one strict-order transcription (`dominance`, `rank monotonicity`, `floor coverage`, ...) leaves the production run, the equality twin (which names only equality legs), and the polarity twin green. Principle 6: a checker with one dead leg per axis passes this pin.

Evidence:

      1373	    for axis in [Axis::Masked, Axis::Placement, Axis::Ranked, Axis::Query] {
      1374	        assert!(
      1375	            outcome.axis_count(axis) > 0,
      1376	            "the flipped twin passed the {axis:?} axis: its strict-order legs \
      1377	             cannot catch a verdict inversion"
      1378	        );
      1379	    }

    (verdict_matrix.rs:1139)
      1139	            if dom != expected_dominance(lo_rel, hi_rel) {

Resolution: Add `const LEGS: &[(Axis, &str)]` rostering every leg name `check` can flag; assert the polarity twin fires on each strict-order leg by name (as 1408-1416 does for the equality twin); assert the union of the two twins' fired legs plus the three documented unreachable legs equals `LEGS`; have `flag` reject a leg not in the roster so a renamed leg cannot escape. Acceptance: replacing the condition at line 1139 with `if false` reads red in `polarity_flipped_sweep_reads_inverted_through_the_matrix`; HEAD is green.
Construction: Change line 1139 to `if false {`. Production: no violation and the census insert at 1134 is unconditional, so it passes. Equality twin: `dominance` is not in its leg list and the ranked axis stays quiet, so it passes. Polarity twin: `Axis::Placement` still fires through `placement`, `precedence`, `containment`, and the coincident legs, so `axis_count > 0` holds and it passes.

### tests-other-20: Nothing pins that every fuzz target has a seed directory
- Where: crates/before/tests/fuzz_seeds.rs:59-74 (related: crates/before/fuzz/fuzz_targets, crates/before/fuzz/seeds, justfile:554-559)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (`ls crates/before/fuzz/fuzz_targets crates/before/fuzz/seeds`: five targets and the same five seed directories today, so the gap is latent); executed: no
- Seen by: instrument-correctness; refutation: confirmed (the `just fuzz` recipe hand-enumerates targets and is outside the gate); history: no rationale found (fd5c92bf compared the seed root against the seed set's own targets, not the `fuzz_targets/` listing)
- Owner-gated: no

The strays test holds `fuzz/seeds/` equal to the seed set's targets, but nothing compares that set against `fuzz/fuzz_targets/*.rs`, so a new target added without seeds starts from an empty corpus with every gate leg green. The module doc's own rationale (the wide tiers random bytes essentially never reach) applies to any target.

Evidence:

        59	    // The root holds exactly one directory per target of record.
        60	    let listed_targets: BTreeSet<String> = fs::read_dir(seeds_root())
       ...
        70	    let expected_targets: BTreeSet<String> = expected.keys().cloned().collect();
        71	    assert_eq!(
        72	        listed_targets, expected_targets,
        73	        "fuzz/seeds holds directories outside the targets of record (or is missing some)"

Resolution: Assert the file stems under `fuzz/fuzz_targets/` equal `expected_targets`, so a new target must gain seeds or a documented exemption. Acceptance: adding `fuzz/fuzz_targets/fuzz_new.rs` with no seed directory reads red.
Construction: Create an empty sixth target file under `fuzz/fuzz_targets/`; `seed_directories_hold_exactly_the_set_of_record` stays green.

### tests-other-2: The worst-map smoke pin transcribes the currency axis as four string literals and `rows == 4`
- Where: crates/before/tests/amp_board_smoke.rs:185-210 (related: crates/before/tests/amp_board_smoke.rs:167-170, crates/before/src/meter/board/worst.rs:82-89, crates/before/src/meter/board/currency.rs:29-40, crates/before/src/meter/board.rs:292-300)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read worst.rs:84-89 `const MAP_CURRENCIES: [Currency; 4]`, private; board.rs:300 re-exports only `NEAR_TIE_RATIO, WORST_MAP_SCALES` from `worst`; currency.rs:29-40 has five variants); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed, severity medium -> low (the const is meter-feature surface and the drift needs a deliberate edit to worst.rs); history: no rationale found (493ef543 landed the private axis and the hard-coded test together)
- Owner-gated: yes: the fix re-exports a private board constant under the `meter` feature

The filter and the count are copies of `worst.rs`'s private `MAP_CURRENCIES` (four of the five `Currency` variants; `Segments` is deliberately excluded there). A currency added to the map is filtered out by the `matches!` while `rows == 4` stays true, and a relabeled one fails for the wrong reason. Principle 5: a number that matters lives in a mechanically-enforced place the test can cite.

Evidence:

       191	        if marker != "worst" || !matches!(currency, "heap" | "limb" | "scan" | "touch") {
       192	            continue;
       193	        }
       ...
       207	    assert!(
       208	        per_op.values().all(|&rows| rows == 4),
       209	        "every operation renders one row per mapped currency: {per_op:?}"
       210	    );

Resolution: Make `worst::MAP_CURRENCIES` `pub` and re-export it from `meter::board` beside `NEAR_TIE_RATIO`; build the accepted label set from `MAP_CURRENCIES.iter().map(Currency::label)` and assert `rows == MAP_CURRENCIES.len()`; drop the parenthetical list at line 169. Acceptance: no currency label literal and no literal `4` remain in `worst_map_covers_every_operation_row`; temporarily adding `Currency::Segments` to `MAP_CURRENCIES` fails the test and restoring passes.

### tests-other-21: The "wide" tail checks test display length, not magnitude
- Where: crates/before/tests/fuzz_seeds.rs:277-279 (related: crates/before/tests/fuzz_seeds.rs:378-380, crates/before/tests/support/fuzz_seed_set.rs:359-361)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no rationale found (the length heuristic and its leaf premise were written with the parse target in 778c89f5 and copied into the laws test in 23e46a7c)
- Owner-gated: no

`text.len() >= 21` is satisfied by any version whose display is 21 or more characters, including a nested tree of small leaves (`(1, (2, 3), (4, (5, 6)))` is 24 characters), so the assertion that the corpus keeps a magnitude past `u64::MAX` can pass with no wide value present. The check should state the invariant it names.

Evidence:

       277	                // A version leaf displays as its bare magnitude; 21+ digits
       278	                // is past u64::MAX (20 digits), i.e. the wide-gamma tier.
       279	                saw_wide |= text.len() >= 21;

Resolution: Require a bare digit run (`text.bytes().all(|b| b.is_ascii_digit())`) with the length bound, or parse the display as `Ticks`/`UBig` and assert `> u64::MAX`, at both sites. Acceptance: substituting a nested narrow version for `wide_leaf` in the derivation reads red on `saw_wide`.
Construction: In fuzz_seed_set.rs:359-361 replace the 2^128 literal with `"(1, (2, 3), (4, (5, 6)))"`; regenerate; both `saw_wide` flags still set.

**Benches and examples**

### benches-examples-12: The presize A/B record has arms compiled into production source, no recorded verdict, and an unpinned deterministic column
- Where: crates/before/benches/presize.rs:20-39 (related: crates/before/src/version/skyline/query.rs:506-517, 603-613; crates/before/src/version/skyline/text.rs:345-354; crates/before/Cargo.toml:97-108; justfile:792-809; crates/before/benches/common/mod.rs:259-281; crates/before/benches/presize.rs:136-150; crates/before/src/codec/bits.rs:120-124)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`git log -S projection_shrink` returns only b28c35ad, 2026-07-30; cd171c29 (2026-07-31) closed the sibling stacks leg from the same commit with a ratified verdict and retired its arms, bench, and dependency: "the seam existed to price the choice, and the choice is made"; grep of .agent-notes for the arm names returns nothing; the three cfg seams read at query.rs:513-516, 608-613 and text.rs:351-354; `Bits::freeze` adopts the Vec via `Bytes::from(buf.into_bytes())` (bits.rs:123) and bytes 1.11.1 keeps the Vec's capacity, so stranded slack is real; no committed test names resident bytes); executed: no
- Seen by: scaffolding [2]; refutation: confirmed (correcting the cited bytes version to 1.11.1); history: no rationale found; the sibling leg's closure is the crate's own precedent
- Owner-gated: yes (dissolving an instrument and production seams)
- Witness (witness/results.md): inconclusive (benches are forbidden). Read-only git corroborates the history: `git log -S projection_shrink` over crates/before finds only b28c35ad6, and cd171c29 is the sibling stacks leg's closing commit; whether the presize-resident lines move under the arm cfg was assessed from the quoted bench doc only.
- Cross-references: version-core-15 (the tick and hull paths' missing resident reading).

Principle 2 ("acceptance means committed measurements changing, not a report asserting improvement") and Principle 3 ("machinery outlives the constraint that justified it"): the record's one deterministic quantity, end-state resident bytes, is printed from a bench binary and pinned by nothing; the wall arms sit in production source that every reader of `own_version_to_version` and the text renderer must reason past; and a month on, no commit or note records what the record decided.

Evidence:

        20	//! The A/B sides are compile-time arms of the library, selected per site by
        21	//! `RUSTFLAGS='--cfg before_alloc_ab="<arm>"'` (the `bench-alloc-ab`
        22	//! recipe): `projection_growth` and `display_growth` start the site's
        23	//! buffer empty, `projection_shrink` adds one exact-size copy where the
        24	//! buffer becomes storage. Nothing in-process distinguishes the sides, so
        25	//! each run saves a criterion baseline named after its arm, and every line
        26	//! of the resident-bytes table below is stamped with the compiled arm.
        27	//!
        28	//! Beside the wall cells, the binary prints one `presize-resident` line per
        29	//! site and input before criterion runs: the live-heap delta of building
        30	//! and holding the operation's result, measured by this binary's counting
        31	//! allocator. That column is deterministic (allocation *requests*, in
        32	//! bytes; the platform allocator's size-class rounding is deliberately out
        33	//! of frame) and is the record's evidence on the copy-vs-stranded-slack
        34	//! trade.

Resolution: close the leg the way the stacks leg was closed: take the record per the recipe's protocol (one `shipped` run plus one run per arm), write the verdict into a note and the closing commit, then dissolve the three cfg seams, the check-cfg roster, `bench-alloc-ab`, and `alloc_arms`. Keep only what the other benches lack: the `projection_outgrow` family (unique to this file) can move to benches/version.rs's hole group; `presize/display` and `presize/parse` duplicate the board's `version_display` and `version_parse_*` rows. If end-state resident bytes matter, pin them as a deterministic test in tests/meter.rs (frozen capacity minus length per site, with a floor) rather than printing them. Acceptance: `grep -rn before_alloc_ab crates/before Cargo.toml justfile` is empty; the closing commit names the measured arm ratios; if a resident-bytes test lands, it fails when query.rs:514 is changed to `let capacity = 0;` and passes on the shipped arm.
Construction: `RUSTFLAGS='--cfg before_alloc_ab="projection_growth"' cargo bench -p before --bench presize -- --sample-size 10 --measurement-time 1`: the `presize-resident site=projection` lines move relative to the shipped build, and no committed test names the moved quantity; `just test-all` under the same RUSTFLAGS exercises transient peak-heap pins, which say nothing about end-state residency.

### benches-examples-5: `alloc_arms` is a third hand-spelled copy of the A/B arm roster, with no check against the manifest
- Where: crates/before/benches/common/mod.rs:266-281 (related: crates/before/Cargo.toml:97-108; justfile:807-809; crates/before/benches/presize.rs:144-147)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (Cargo.toml:107 `check-cfg = ['cfg(before_alloc_ab, values("projection_growth", "projection_shrink", "display_growth"))']`; justfile:808 case list `(shipped|projection_growth|projection_shrink|display_growth)`; mod.rs:268-278 three `cfg!` literals with `"shipped"` as the empty-case fallback); executed: no
- Seen by: structure-prose [38]; refutation: confirmed; history: no rationale (Cargo.toml:103-105 names the mistyped-RUSTFLAGS hole and routes it to the recipe, not the omission hole)
- Owner-gated: no

Principle 6 (the cheapest passing artifact): the stamp exists so a baseline "can never be mis-attributed to the wrong build" (mod.rs:263-265), yet an arm added to the manifest and a seam but not to `alloc_arms` stamps `arm=shipped` on a non-shipped build; `deny(unexpected_cfgs)` catches a misspelled value in `cfg!`, never an omitted one. Moot if benches-examples-12 closes the leg.

Evidence:

       266	pub fn alloc_arms() -> String {
       267	    let mut arms = Vec::new();
       268	    if cfg!(before_alloc_ab = "projection_growth") {
       269	        arms.push("projection_growth");
       270	    }
       277	    if arms.is_empty() {
       278	        arms.push("shipped");
       279	    }

Resolution: one `pub const ALLOC_ARMS: &[&str]` from which a small macro generates the `cfg!` checks, and a test that parses Cargo.toml's check-cfg line and asserts its value list equals the constant; or have `bench-alloc-ab` pass the arm through an environment variable the bench stamps, cross-checked against one `cfg!`. Acceptance: adding a value at Cargo.toml:107 without touching benches/common fails a committed test.
Construction: add `"parse_growth"` to the `values(...)` list at Cargo.toml:107 and a `#[cfg(before_alloc_ab = "parse_growth")]` seam anywhere in the library; `RUSTFLAGS='--cfg before_alloc_ab="parse_growth"' cargo bench -p before --bench presize --no-run` builds clean, and running it prints `arm=shipped` on every `presize-resident` line.

### benches-examples-6: The sidecar stamp binds sidecar to sidecar and to `--tip`, not to the criterion baseline
- Where: crates/before/benches/common/sidecar.rs:16-18 (related: tools/benchjudge:373-393, 396-411; justfile:843-845; crates/before/benches/board.rs:159)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (tools/benchjudge `cross_check_stamps` compares `profile`, `sampling`, `tip` between the two sidecars, requires `scale` to grow, and checks each stamp against `--tip`; `read_median` reads only `estimates["median"]["point_estimate"]` from criterion's estimates.json, which carries no stamp; justfile:843-845 sets the tip to `git rev-parse HEAD`); executed: no
- Seen by: adequacy [20]; refutation: confirmed; history: no rationale (the claim and the judge both date to b4942461, and the judge's error text "a leftover baseline from another run?" refers to the stamp-disagreement path)
- Owner-gated: no
- Cross-references: meter-adequacy-9 (the same stamp ordering, from the sweep).

Provenance discipline ("bind every measurement to its run; refuse mismatches"): the doc promises the judge can "refuse a sidecar/baseline pair assembled from different runs", but criterion's saved baseline carries no stamp, so a sidecar and a baseline from different runs are never compared, and `git rev-parse HEAD` is shared by a dirty tree and its clean commit. The recipes rewrite both artifacts in one flow, which bounds the exposure; the stated guarantee is still stronger than the mechanism.

Evidence:

        16	//! The stamp binds the sidecar to the bench run that wrote it, so
        17	//! `tools/benchjudge` can refuse a sidecar/baseline pair assembled from
        18	//! different runs (exit 2) instead of silently re-scoring: the resolved

    tools/benchjudge:
       375	    for field in ("profile", "sampling", "tip"):
       407	        return estimates["median"]["point_estimate"]

Resolution: State the binding precisely at sidecar.rs:16-25 (sidecar-to-sidecar and sidecar-to-invocation; the baseline is trusted to be the same run's). To close the gap instead: stamp `git describe --always --dirty` (or refuse when `git status --porcelain` is nonempty) in the recipes, and have the judge require each judged cell's estimates.json to be no older than its sidecar, which is written before any cell runs (board.rs:159). Acceptance: the doc names exactly the two cross-checks the judge performs, or the construction below exits 2 naming the stale cell.
Construction: run `just bench-judge` clean; make an uncommitted edit that makes one pinned op quadratic; re-run only the lo pass with the recipe's environment (the justfile:843 line) and invoke the judge line (justfile:845) by hand: both sidecars stamp the same tip, profile, and sampling, the hi medians are the pre-edit tree's, and the judge scores the pair.

### benches-examples-10: The ceiling-class assert compares the pinned set with itself for every board cell, and the sidecar prose describes a cell-site declaration board cells do not have
- Where: crates/before/benches/common/sidecar.rs:170-180 (related: crates/before/benches/board.rs:129-158; crates/before/benches/tripwire.rs:59; crates/before/benches/common/sidecar.rs:27-30, 82-85, 143-147; crates/before/tests/bench_judge_roster.rs:102-116; crates/before/src/meter/board/tests.rs:1094-1127; crates/before/src/meter/board/cell.rs:195-198)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (board.rs:141-145 derives `ceiling` from `TEXT_CEILING_CELLS.contains(&id.as_str())` and sidecar.rs:175-177 asserts `*ceiling == Ceiling::Text` against `TEXT_CEILING_CELLS.contains(id)`, the same predicate; only the two wide-pair literals (board.rs:152, 157) and the tripwire's `General` (tripwire.rs:59) are independent inputs, and both wide-pair IDs are in the set while the tripwire's is not; `grep -rn TEXT_CEILING_CELLS` hits sidecar.rs, board.rs, the roster test's literal copy, and the roster notes only; `bench_riders_name_declared_model_cells` (board/tests.rs:1095-1127) checks `BOARD_DECLARED_BENCH_RIDERS` against `bench_cells(0.02, BenchMode::Full)` and no analogous check exists for the text set); executed: no
- Seen by: scaffolding [7], adequacy [19], structure-prose [36], instrument-correctness [48]; refutation: confirmed, severity lowered (a stale entry can only tighten a cell's ceiling or go inert, never launder a class); history: deliberate-but-expired (514cbcea made board cells derive their class from the pin "so a class can only move by editing the pin", leaving f4c3e563's assert and its cell-site-declaration prose in place)
- Owner-gated: no

A guard must name a concrete failure it catches; for board cells this one compares a lookup with itself. sidecar.rs:27-30 ("declared here in bench code at the cell's definition site"), 82-85 ("asserts every declaration against this set ... a two-site edit"), and the `# Panics` section describe a second site board cells do not have; the actual second site is the roster test's literal pin. Separately, nothing checks that the five board entries name live cells: a renamed op leaves its entry inert and drops the renamed cell to the stricter general ceiling, visible only as a spurious red at `just all` cadence with no diff pointing at the set.

Evidence:

       175	        assert_eq!(
       176	            *ceiling == Ceiling::Text,
       177	            TEXT_CEILING_CELLS.contains(id),
       178	            "{id}: the text-ceiling set is pinned as TEXT_CEILING_CELLS; \
       179	             declare the class there and at the cell together"
       180	        );

    board.rs:
       141	            let ceiling = if sidecar::TEXT_CEILING_CELLS.contains(&id.as_str()) {
       142	                sidecar::Ceiling::Text
       143	            } else {
       144	                sidecar::Ceiling::General
       145	            };

    sidecar.rs:
        82	/// [`write_denoms`] asserts every declaration against this set, and
        83	/// `tests/bench_judge_roster.rs` pins the set itself — so widening the
        84	/// text class is a two-site edit whose diff a reviewer sees, never a
        85	/// one-character class swap at a cell.

Resolution: make membership the single declaration: `write_denoms` takes `(id, denominator_bytes)` pairs and derives the class from `TEXT_CEILING_CELLS` internally (this reproduces today's sidecar byte for byte, since both wide-pair IDs are in the set and the tripwire's is not); drop the assert and the three literal `Ceiling` arguments; reword sidecar.rs:27-30, 82-85, and 143-147 to "the class is set membership, pinned by tests/bench_judge_roster.rs". Add to tests/bench_judge_roster.rs (which already includes the sidecar module) a test that every entry other than `version_display_wide/hugeleaf` and `display_schoolbook/hugeleaf` is an `op/family` of `board::bench_cells(0.02, BenchMode::Full)`, mirroring `bench_riders_name_declared_model_cells`. Acceptance: `Ceiling` no longer appears in board.rs or tripwire.rs; the sidecar written by `just bench-judge` is byte-identical to before; renaming a text-class op in ops.rs without editing the set fails a gate test naming the stale entry.
Construction: rename `clock_parse_trailing` to `clock_parse_trail` in src/meter/board/ops.rs and its callers, leave `TEXT_CEILING_CELLS` and the roster test untouched; `just test-all` is green; `just bench-judge` judges `clock_parse_trail/hugeleaf` at 1.3, and the stale entry never fires.

### benches-examples-14: `outgrow_family` asserts a relative sweep where its doc promises an absolute crossing of the pre-size
- Where: crates/before/benches/presize.rs:92-132 (related: crates/before/src/version/skyline/query.rs:514)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (`ratio` at 121-124 is output bits over `v.encoded_bits() + p.encoded_bits()`, which is the shipped pre-size at query.rs:514 `let capacity = event_bits.len() + id_bits.len();`; the assert at 128-129 is `last >= 4.0 * first` only); executed: no
- Seen by: instrument-correctness [50]; refutation: confirmed; history: no rationale (b28c35ad says the "straddle across the pre-size's growth doublings is asserted on construction" and chose the relative form without stating why)
- Owner-gated: no

Principle 6 (what is the worst artifact that passes): the doc says the output must sweep "from near the pre-size estimate ... to at least 4x past it", so crossing doublings needs `last >= 4.0` with `first` near 1; the assertion also passes for 0.2 -> 0.8, a family that never outgrows the pre-size, the exact flattening the doc says the assertion prevents. Today's family very likely satisfies the absolute claim, so the pin is looser than its sentence rather than failing.

Evidence:

        92	/// The family's design property is asserted on construction: the
        93	/// materialized output must sweep from near the pre-size estimate
        94	/// (operands' summed lengths) to at least 4x past it, so successive cells
        95	/// cross output-buffer growth doublings. Encoded sizes are
       128	    assert!(
       129	        last >= 4.0 * first,

Resolution: assert both clauses, `first <= NEAR_PRESIZE_RATIO && last >= 4.0` with a named constant (1.5, say) and both ratios in the panic message; or reword the doc to the relative sweep if that is the intent. Acceptance: the assertion's inequality matches the doc sentence; a synthetic pair (0.3, 1.2) fails it.
Construction: multiply `OUTGROW_FRAGMENTS` by 64 so the party's bits dominate every term: ratios fall well below 1 across the sweep while `last >= 4 * first` can still hold, and no growth doubling is crossed.

### benches-examples-15: `tools/benchjudge --self-test` runs only at the head of the bench-judge recipes, not in `gate-lints`; tripwire.rs overstates its cadence
- Where: crates/before/benches/tripwire.rs:12-14 (related: justfile:379-382, 403, 842, 849-850, 854, 1003; tools/benchjudge:643-668)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -n self-test justfile`: benchjudge appears at 842 and 854 only, inside `bench-judge` and `bench-judge-tripwire`, which only `all` (1003) runs; `gate-lints` (403) runs every other tool's `--self-test` at 180-322); executed: no
- Seen by: adequacy [14]; refutation: confirmed, severity lowered (every judge invocation runs the self-test first, so no verdict is ever drawn from a softened judge); history: the gate exclusion (feeefb2c; justfile:379-382) is stated for wall-time judging, which a deterministic Python self-test is not; every tool added since joined `gate-lints` with its self-test
- Owner-gated: no

The self-test is pure arithmetic, byte-identical under load, and belongs in the build-free lint tier beside its siblings; today a commit that softens `MAX_WALL_SCALING_EXPONENT` passes `just gate` and fails only when someone runs `just all`. The doc's "cannot soften silently between live demonstrations" describes a cadence the recipes do not deliver: the pin runs at the head of each live demonstration, not between them.

Evidence:

        12	//! judge's leg catches what no deterministic meter can. The same shape is
        13	//! pinned deterministically in the judge's `--self-test`, so the criterion
        14	//! cannot soften silently between live demonstrations.

    justfile:
       403	gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare mutants-list readme-check

Resolution: add `./tools/benchjudge --self-test` to `gate-lints` in the position the other tool self-tests occupy; leave the two in-recipe invocations; reword tripwire.rs:12-14 to "pinned in the judge's `--self-test`, which the gate runs". Acceptance: `just gate-lints` invokes `tools/benchjudge --self-test`; with an uncommitted `MAX_WALL_SCALING_EXPONENT = 3.0`, `just gate` fails at the self-test before any build runs.
Construction: set `MAX_WALL_SCALING_EXPONENT = 3.0` in tools/benchjudge (uncommitted): `just gate` passes today, since nothing in gate-lints or gate-streams invokes the judge.

### benches-examples-17: The `version/partial_cmp` `equal` row times the clone-identity rung, not a traversal, and its doc says otherwise
- Where: crates/before/benches/version.rs:153-180 (related: crates/before/src/version/skyline/sweep.rs:94-105; crates/before/src/version.rs:91-94; crates/before/src/oracle/version.rs:421-433; crates/before/benches/common/mod.rs:14-18; crates/before/benches/version.rs:222-224, 248-255)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (sweep.rs:100-102 `if a.ptr_eq(&b) { return Some(Ordering::Equal); }` precedes the sweep, with the comment "Equal streams in distinct buffers still take the sweep below"; version.rs:91-94 documents the derived `Clone` as a refcount share; the oracle's `partial_cmp` (oracle/version.rs:423-432) runs two `leq` walks, and `grep -n ptr_eq crates/before/src/oracle/*.rs` finds nothing); executed: no
- Seen by: scaffolding [3], structure-prose [30], instrument-correctness [46]; refutation: confirmed, noting that the committed results README's mechanism was already wrong for its own 2026-06-02 data, which ran under e7324dc2's memcmp rung; history: deliberate-but-expired (accurate for hours on 2026-05-30; two shortcut commits, neither touched the bench; b606f2ad built distinct buffers for the hole group only)
- Owner-gated: no

A bench row is an instrument, and its doc must state what its body measures. `("equal", &base, &base, ...)` hands the same reference twice, so the impl side returns in O(1) from the ptr_eq rung while the oracle walks the tree twice: the row is not like-for-like (common/mod.rs:17-18), a regression in the equal-exhaustion sweep is invisible on it, and both "each exercising a different traversal" and "a version against its own clone" are false, since a clone shares the buffer and hits the same rung. The same file already knows the fix: `bench_hole` builds "equal projections in distinct buffers (a full co-walk, no early exit)" by re-running the construction.

Evidence:

       153	/// `partial_cmp` (the causal order) over the three outcomes the comparison
       154	/// can take, each exercising a different traversal.
       155	///
       156	/// The outcomes: `concurrent` (two independent histories), `ordered`
       157	/// (one strictly precedes the other), and `equal` (a version against its own clone).
       179	            ("equal", &base, &base, &obase, &obase),

    sweep.rs:
        98	    // `order_reflexive` law in `crate::laws`. Equal streams in distinct buffers
        99	    // still take the sweep below.
       100	    if a.ptr_eq(&b) {
       101	        return Some(Ordering::Equal);
       102	    }

Resolution: decode a twin in a distinct buffer for the equal row (`let twin = Version::decode(&base.encode()[..]).unwrap();` and `("equal", &base, &twin, &obase, &obase)`); restate the doc ("equal streams in distinct buffers: the sweep runs to exhaustion"); optionally keep `&base, &base` as an explicitly named `identical` row if the rung's cost is worth tracking. Acceptance: at every `n`, the `before/equal` median scales with `n` like `before/ordered`; the doc names distinct buffers.
Construction: `just bench-quick version partial_cmp` at HEAD: `before/equal` reads near-constant (tens of nanoseconds) across n = 8..32768 while `oracle/equal` grows with n; after the twin change, `before/equal` grows with n.

**Fuzz targets, the fuzz-fit guest, the wasm32 pins**

### fuzz-guests-pins-1: The wasm32-pins leg runs nowhere in CI and its exclusion is undeclared
- Where: .github/workflows/ci.yml:110-120 (related: justfile:985-986, justfile:1000, justfile:467, justfile:613-614, justfile:633-635)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read the `ci` recipe line, the gate's stream roster, and both comments that enumerate the local-only legs); executed: no
- Seen by: refutation pass (raised as new); refutation: new; history: not examined
- Owner-gated: yes: gate and CI policy
- Witness (witness/results.md): demonstrated (reading). Neither the workflow nor the `ci` recipe (justfile:1000) names `wasm32-pins`, while the gate's wasm stream does (justfile:467); the two comments enumerating what stays local (ci.yml:110-120, justfile:985-986) name only the wall-time judge and the wasmtime fuel tier. The push construction was not performed.
- Cross-references: gate-legs-3 (the same leg's undocumented local-only status).

The `ci` recipe (justfile:1000) omits `wasm32-pins` (and `fuzzfit`), and the two comments whose stated purpose is to name which instrument legs stay local and why (ci.yml:110-120, justfile:985-986) name only the wall-time judge and "the wasmtime fuel tier (the fuzzfit bands and the fuelscape pins)". wasm32-pins is deterministic and is not a fuel tier, and the justfile calls it "the tree's one place 32-bit code *executes*" (613-614), so the gate's wasm stream has a leg with no CI counterpart and no stated reason. Doctrine: every status board an acceptance concept exists for is wired into the required checks, or its exclusion is stated positively where the roster lives.

Evidence:

    110	  # The gate's deterministic instrument legs, re-run on the runner. Only
    111	  # counter-based legs ride here: their readings are byte-identical under
    112	  # any machine load, so a shared runner cannot flake them. What stays
    113	  # local, and why:
    114	  #   - the judged wall-time legs (bench-judge and its tripwire): criterion
    115	  #     exponent fits need the quiet-machine regime a shared runner cannot
    116	  #     promise;
    117	  #   - the wasmtime fuel tier (the fuzzfit bands and the fuelscape pins):
    118	  #     deterministic, but it brings a wasm32 guest build plus wasmtime to
    119	  #     police asymptotics the board legs below already judge at the scales
    120	  #     of record — the gate keeps the second jaw.

    (justfile:467)     start_stream wasm         10 fuzzfit fuelscape-test wasm32-pins
    (justfile:1000) ci: fmt-check doclint testdoc workflowlint digestshare readme-check fuelscape-claims mutants-list clippy clippy-default features wasm-check docs docs-internal test-all citecheck doctest bench-build fuzz-build fuelscape-verify viz

Resolution: Owner call. Either add `wasm32-pins` to the `instruments` job (the recipe prices it at minutes and a few GiB across nextest workers; it builds `before` for wasm32 plus wasmtime), or add a bullet to both comments declaring it local and why. Acceptance: every leg in the gate's stream roster (justfile:464-471) either appears in `ci` or the `instruments` job, or is named in the local-only list with its reason.
Construction: Change one expected value in pins.rs (`Outcome::Value(0)` to `Outcome::Value(1)` at line 34) and push: `just ci` and the `instruments` job stay green; only a local `just gate` reddens.

### fuzz-guests-pins-14: The flat 1 GiB heap cap cannot see sub-2^18x amplification at fuzz input sizes, and nothing committed shows it fires
- Where: crates/before/fuzz/src/lib.rs:10-17 (related: crates/before/fuzz/src/lib.rs:28-29, crates/before/fuzz/src/lib.rs:37-47, crates/before/fuzz/Cargo.toml:23-28, crates/before/src/lib.rs:333-340, crates/before/tests/meter.rs:1-11, .agent-notes/2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md:1533-1538)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (arithmetic: 2^30 / 4096 = 2^18; 4096^2 = 2^24 = 16 MiB, 64 times under the cap; committed seeds are at most 130 bytes by `wc -c`; no `#[test]` anywhere in the fuzz workspace by grep; `git show -s 48965ff3` records the trip as a hand run with a temporarily lowered ceiling; the agent note lists the proportional cap as an open closeout obligation); executed: no
- Seen by: scaffolding [0]; adequacy [24]; instrument-correctness [60]; structure-prose [51]; refutation: confirmed, with two corrections carried here (the ratio is 64 times, six binary orders, not "four orders"; the doc's "a trip means a new class" is the forward implication and true as written); history: already-known (the note records "Land the proportional ceiling, or the owner ratifies the flat cap with the harness doc re-derived to the ratified shape"; the committed trip demonstration is the unrecorded half)
- Owner-gated: yes: the note frames flat-versus-proportional as an owner ruling
- Witness (witness/results.md): inconclusive. Assessed by reading: the cap is a flat 1 GiB (lib.rs:29), its only callers are the fuzz targets, and all five `[[bin]]` targets are `test = false`, so nothing committed demonstrates the cap firing; the libFuzzer construction was not run.

At libFuzzer's default `-max_len` (4096; the committed seeds are at most 130 bytes, so the corpus does not raise the guess), the cap fires only on amplification of at least 2^18 times the input: a quadratic amplifier with unit constant peaks at 16 MiB and passes every run, although the crate docs make "auxiliary space at most a small constant multiple of the input size" a hard guarantee and this harness is the only space instrument over unchosen input shapes (`tests/meter.rs` pins chosen families). libFuzzer's own `-malloc_limit_mb` (default equal to `-rss_limit_mb`, 2048) already kills anything past about 2 GiB, so the cap's exclusive window is survivable peaks in [1 GiB, 2 GiB). No committed check demonstrates the cap fires (the bins are `test = false`; the lib has no test), so a broken `reset_peak_usage`/`peak_usage` pairing would read green forever. "not yet proportional" is dated rationale at a declaration site. Instruments before cures: every ceiling needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

    10	//! The ceiling is generous and absolute: a flat [`PEAK_HEAP_CAP_BYTES`],
    11	//! not yet proportional to input size. The known amplifiers are linear in
    12	//! the input with constants in the hundreds, and libFuzzer's default 4096-byte
    13	//! inputs keep them megabytes below the cap, so a trip means a new class of
    14	//! blowup, not a bigger constant. The peak is read after the body returns
    15	//! rather than enforced inside the allocator, so a spike that outruns the
    16	//! process before returning is stopped by libFuzzer's RSS limit instead; the
    17	//! cap's job is the far more common survivable amplification.
    29	pub const PEAK_HEAP_CAP_BYTES: usize = 1 << 30;

Resolution: Owner rules the shape (the note's two branches). If proportional: `cap = max(FLOOR_BYTES, PER_INPUT_BYTE * data.len())`, threading `data.len()` into `under_heap_cap`, with `PER_INPUT_BYTE` measured over the seeds and a smoke run first and committed with slack, so the doc's "constants in the hundreds" becomes an enforced number. If ratified flat: restate the doc positively without "not yet" and name the class the flat cap is for. Either way, add a `#[test]` to the fuzz lib (plain `cargo test --lib` in the workspace, runnable from `fuzz-build`) that `under_heap_cap` panics on a synthetic over-cap allocation and passes on a linear one. Acceptance: the lib test is green in the gate; under the proportional rule, `let _sink = vec![0u8; data.len() * data.len()];` planted in a target's `run` crashes on fuzzer-generated inputs near `-max_len` (not on the tiny seeds); "not yet" is gone.
Construction: Add `let _sink = vec![0u8; data.len() * data.len()];` to `fuzz_decode::run` and run `just fuzz`: peak 16 MiB at 4096 bytes, `peak <= 1 << 30` holds, no finding. For the liveness half: make `under_heap_cap` skip its assert; nothing committed reddens.

### fuzz-guests-pins-26: The overflow-checks premise has no liveness self-test
- Where: crates/before/wasm32-pins/Cargo.toml:26-32 (related: crates/before/wasm32-pins/guest/src/lib.rs:21-24, crates/before/wasm32-pins/harness/tests/pins.rs:84-90, crates/before/fuzzfit/guest/src/lib.rs:2016-2037, crates/before/fuzzfit/harness/tests/enforce.rs:260-310)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read the profile, the guest's module doc, and every pin's expected value; `grep -n 'selftest|set_hook|PANICKED'` over the pins guest is empty while the fuzz-fit guest ships `ff_selftest_quadratic`); executed: no
- Seen by: adequacy [23]; refutation: confirmed (not run); history: no-rationale-found
- Owner-gated: no
- Witness (witness/results.md): inconclusive. The witness agent did not read wasm32-pins/Cargo.toml or pins.rs and neither corroborates nor disputes the quoted profile lines; the liveness self-test's absence needs the guest run the construction names.

The whole "a 32-bit wrap is an observable trap, never a silently wrong value" argument rests on `overflow-checks = true` in the guest's release profile, and nothing observes that the built guest has it: `CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false` in the environment, a `[profile.release]` in a config file, or a future manifest edit silently restores wrapping, and no pin's expected value depends on a wrapped intermediate trapping (pins.rs:84-90 notes the 2^29-byte wrap is coincidentally correct), so every pin stays green. The fuzz-fit sibling carries the corresponding instrument self-test for its fuel path. Meters need liveness proof.

Evidence:

    26	# The pins run at release speed (the boundary cases walk hundreds of
    27	# megabytes), but with overflow checks kept ON: the 32-bit failure class
    28	# under audit includes silent release-mode wraps, and the checks turn
    29	# exactly those wraps into observable traps instead of wrong values the
    30	# guest would have to detect after the fact.
    31	[profile.release]
    32	overflow-checks = true

Resolution: Add `pin_selftest_overflow() -> i64` to the guest that computes `black_box(u32::MAX) + black_box(1u32)` (and a `usize` variant) and a harness test asserting `Outcome::Trapped(Trap::UnreachableCodeReached)` (with the panic-genre discriminator from finding 35, asserting the panic genre). Name it beside `version_small_roundtrips_and_rejects_typed` as the leg's second liveness pin. Acceptance: `CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false just wasm32-pins` turns the self-test red; the default build keeps it green.
Construction: Run `CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false just wasm32-pins`: every existing pin passes, because no pin's observation is a wrapped quantity and no export exercises a wrap on purpose.

### fuzz-guests-pins-29: The rank pins observe only `0 < r < 1` and `r == r.clone()`, which a decoder that drops the seam bit satisfies
- Where: crates/before/wasm32-pins/guest/src/lib.rs:189-210 (related: crates/before/wasm32-pins/guest/src/lib.rs:118-175, crates/before/wasm32-pins/guest/src/lib.rs:569-589, crates/before/wasm32-pins/guest/src/lib.rs:603-621, crates/before/wasm32-pins/guest/src/lib.rs:671-690, crates/before/wasm32-pins/guest/src/lib.rs:700-725, crates/before/wasm32-pins/harness/tests/pins.rs:155-302, crates/before/wasm32-pins/harness/tests/pins.rs:633-713, crates/before/src/version/rank.rs:244)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read `synth_rank`: it sets expansion bits 65 and `exp`, so the value is 2^-65 + 2^-exp; read every rank pin's checks; `Rank` derives `Clone, PartialEq, Eq` at rank.rs:244); executed: no
- Seen by: adequacy [22]; refutation: confirmed with a nuance carried here (the "never zeros" clause at pins.rs:180-181 is checked by `r > ZERO`; the overstated clauses are "orders exactly against reference ranks" at pins.rs:174, 205, 226, 244 and the "exact" arithmetic; deleting `r != r.clone()` would drop execution coverage of `Clone`/`Eq` on the limb arm, a trap channel, so replace rather than delete); history: no-rationale-found (the leg's original design, copied into later rank pins)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (run on the host, not wasm32). A faithful mirror of the guest's `synth_rank` layout and `pin_rank_decode`'s observation sequence decodes the true value 2^-65 + 2^-128 and the seam-dropped value 2^-65 to different ranks, and the pin's three observations return 0 for both.

`pin_rank_decode`, `pin_rank_integral_decode`, `pin_version_rank`, `pin_rank_add`, and `pin_rank_checked_sub` observe the result only by coarse order against `Rank::ZERO` or the rank of the version `1`, and by `r != r.clone()` (a copy compared to itself, which no arm can fail). A decoder that drops the bit at expansion position `exp` (the seam bit the pins target) still yields a value strictly between zero and one; an adder that loses the 2^-(2^32) term still exceeds both summands. The natural known-bad mechanism passes every rank pin except `pin_rank_roundtrip`, which runs at one exponent. Every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

    196	    if r <= Rank::ZERO {
    197	        return -2;
    198	    }
    199	    let one = match Version::try_from(1) {
    200	        Ok(v) => v.rank(),
    201	        Err(_) => return -3,
    202	    };
    203	    if r >= one {
    204	        return -4;
    205	    }
    206	    if r != r.clone() {
    207	        return -5;
    208	    }
    (pins.rs:174) /// correctly on wasm32 and orders exactly against reference ranks.

Resolution: Give each rank pin a witness that sees the seam bit inside the memory budget: decode a second synthesized stream identical except for the deep bit (drop the first stream's bytes first) and assert strict order between the two; for `pin_rank_add` and `pin_rank_checked_sub` assert the algebraic inverse (`sum.checked_sub(&small) == Some(big)`, `diff + small == minuend`) or compare `sum.encode()` against a synthesized expected stream; for `pin_version_rank` bracket with tight bounds `rank(leaf(h)) < r < rank(leaf(h + d))` rather than `rank(1)`. Replace `r != r.clone()` with a check that exercises `Clone` and `Eq` on the limb arm and can fail (compare the clone's `encode()` to the original's). Acceptance: a guest whose `synth_rank` clears the bit at `exp` reads red on every rank decode pin; a `pin_rank_add` whose result lacks the 2^-(2^32) term reads red; the committed guest stays green within the 4 GiB budget.
Construction: In `synth_rank`, change `for e in [65, exp]` to `for e in [65]` (standing in for a decoder that drops the seam bit) and run `just wasm32-pins`: `rank_decode_below_backend_capacity`, `rank_decode_at_backend_byte_capacity`, `rank_decode_at_usize_exp_boundary`, `rank_decode_at_backend_bit_capacity`, `rank_decode_past_backend_bit_capacity`, the `rank_add_*` pins, and the `rank_checked_sub_*` pins all still return `Value(0)`; only `rank_roundtrip_past_backend_bit_capacity` notices.

### fuzz-guests-pins-35: The memory-terminal pins assert a trap genre every guest abort produces
- Where: crates/before/wasm32-pins/harness/tests/pins.rs:147-153 (related: crates/before/wasm32-pins/harness/tests/pins.rs:373-379, crates/before/wasm32-pins/harness/tests/pins.rs:736-742, crates/before/wasm32-pins/harness/src/lib.rs:20-34, crates/before/wasm32-pins/guest/src/lib.rs:16-24, crates/before/wasm32-pins/Cargo.toml:26-32, crates/before/src/version/rank/num.rs:6-13)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read: wasmtime-environ 47.0.3 `trap_encoding.rs:138` defines `UnreachableCodeReached = "wasm \`unreachable\` instruction executed"`; on wasm32-unknown-unknown a panic aborts and abort lowers to `unreachable`, and allocation failure routes through `handle_alloc_error` to the same abort; the harness doc itself equates the trap with "a guest panic"; the at-capacity arithmetic for `version_rank_memory_terminal_traps` is (2^32 - 96) + 64 = 2^32 - 32 bits, exactly num.rs:10's backend cap); executed: no
- Seen by: adequacy [20]; instrument-correctness [61]; refutation: confirmed; history: no-rationale-found (c75d5022 attributes each terminal to allocation failure by probe backtraces read by hand; no discriminator was ever added)
- Owner-gated: no
- Witness (witness/results.md): inconclusive. By reading, the terminal pins assert only `Outcome::Trapped(Trap::UnreachableCodeReached)`, which the harness itself documents as "a guest panic", the genre every wasm32-unknown-unknown abort produces; the `assert!(n < 1_000_000_000)` mutation was not observed.

The three terminal pins assert only `Outcome::Trapped(Trap::UnreachableCodeReached)`, but that trap is produced by every abort: allocation failure, an explicit `panic!` or `expect`, an overflow-check trap (the failure class this workspace exists to expose, per Cargo.toml:26-32), a big-integer backend capacity panic (num.rs:13 calls both overruns "loud backend panics"), or a synthesizer failure (finding 27). The only thing tying the trap to allocation failure is a probe backtrace described in prose. `version_rank_memory_terminal_traps` sits exactly at the backend capacity coordinate, so an arm-routing off-by-one that handed the `Base` arm a value it panics on reads green there. The cheapest passing artifact: a pin for "aborts on allocation failure" passes on any panic, so the suite's most sensitive coordinates read green on exactly the class it audits.

Evidence:

    147	#[test]
    148	fn version_decode_memory_terminal_traps() {
    149	    assert_eq!(
    150	        call1("pin_version_decode", 1_073_741_817),
    151	        Outcome::Trapped(Trap::UnreachableCodeReached),
    152	    );
    153	}
    (harness/src/lib.rs:32)     /// The export trapped; `Trap::UnreachableCodeReached` is a guest panic.

Resolution: Record the abort genre before the trap: install a `std::panic::set_hook` at each export's entry that sets a `static PANICKED: AtomicBool` (std's alloc-error path calls `abort` directly and never runs the panic hook, so the flag separates the two), expose it through `pin_panicked() -> i64` or a fixed linear-memory address the harness reads from the store after the trap, and widen `Outcome::Trapped` into panic-versus-abort so the terminal pins assert the allocation-abort genre and every other pin asserts no panic flag. A cheaper second discriminator: read `Memory::size(&store)` after the trap and assert linear memory grew to within a page budget of 4 GiB. Confirm wasmtime's behavior for calling exports in the same store after a trap before choosing. Acceptance: a guest whose `pin_version_decode` panics for `n_bytes >= 1 << 30`, or spells `Bits::len` as a `usize` product, turns all three terminal pins red; the committed guest keeps them green.
Construction: Add `assert!(n < 1_000_000_000);` at the top of `pin_version_decode` and run `just wasm32-pins`: `version_decode_memory_terminal_traps` still observes `Trapped(UnreachableCodeReached)` at 1_073_741_817 and passes. Equivalently, `call1("pin_version_decode", 1 << 30)` traps in the synthesizer's `4 * n - 5` (finding 27) with the same outcome.

### fuzz-guests-pins-37: `wasm32-pins/Cargo.lock` is outside the supply-chain audit roster
- Where: justfile:345-351 (related: justfile:328-330, crates/before/wasm32-pins/Cargo.lock:952-953, crates/before/fuzzfit/Cargo.lock:1193-1194, crates/before-fuelscape/Cargo.lock:1577-1578, .github/workflows/ci.yml:169-172)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (`git ls-files '*Cargo.lock'` lists six lockfiles; the recipe audits five; the comment at 329-330 hand-enumerates "(fuzz, fuzzfit, fuelscape, surfacecheck)"; wasm32-pins locks wasmtime 47.0.3 where fuzzfit and fuelscape lock 47.0.4, so it is a distinct resolution never swept; `git show eb6ba627 --stat` shows the lockfile and a justfile edit with no audit line; ci.yml:171-172 runs `just supply-chain`); executed: no
- Seen by: scaffolding [7]; refutation: confirmed; history: deliberate-but-expired (the roster was total when 4da4779fc landed it; eb6ba627 added the sixth lockfile and edited the justfile without extending the recipe)
- Owner-gated: no
- Witness (witness/results.md): demonstrated (mechanical; no cargo audit run). Six lockfiles are committed, the recipe audits five, and wasm32-pins/Cargo.lock is the one omitted, pinning wasmtime 47.0.3 where fuzzfit and fuelscape lock 47.0.4; CI runs `just supply-chain` (ci.yml:171-172).
- Cross-references: deps-1 (the live advisory on that version), gate-legs-3.

The recipe's own comment claims "every lockfile in the repository"; a hand-maintained enumeration of workspaces rots exactly this way, and CI runs the recipe, so the gap is live. No hand-maintained rosters the code can change without touching the prose.

Evidence:

    345	# Audit advisories on every lockfile and hold the workspace to single crate versions.
    346	supply-chain:
    347	    cargo audit
    348	    cargo audit --file crates/before/fuzz/Cargo.lock
    349	    cargo audit --file crates/before/fuzzfit/Cargo.lock
    350	    cargo audit --file crates/before-fuelscape/Cargo.lock
    351	    cargo audit --file crates/before/surfacecheck/Cargo.lock

Resolution: Add the wasm32-pins lockfile, and derive the list mechanically so the next detached workspace cannot be missed: `git ls-files '*Cargo.lock' | xargs -n1 cargo audit --file` (or a `find` excluding `target/`), and drop the hand enumeration at 329-330. Consider aligning the three wasmtime pins while touching the locks. Acceptance: the recipe audits every committed lockfile without naming them; adding a lockfile to the tree changes nothing in the justfile.
Construction: Pin an advisory-bearing crate version only in `wasm32-pins/Cargo.lock` (or wait for a RustSec advisory covering wasmtime 47.0.3 alone): `just supply-chain` stays green.

### fuzz-guests-pins-38: The fuzz targets' own oracles never execute in the gate or CI
- Where: justfile:358-360 (related: justfile:548-559, justfile:1000-1003, .github/workflows/ci.yml:4-7, crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:164-179, crates/before/tests/support/fuzz_seed_set.rs:155-159)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read: `fuzz-build` compiles only; `ci` (justfile:1000) includes `fuzz-build` and not `fuzz`; only `all` (1003) runs the smoke; `tests/fuzz_seeds.rs` checks seed genres through the public API, never through the targets' `agreed` tables); executed: no
- Seen by: adequacy [25]; refutation: confirmed; history: deliberate-and-holds for the smoke's exclusion (justfile:359-360 and 04b4d1b7 price fuzzing minutes), but a seconds-long deterministic `-runs=0` seed replay was never considered
- Owner-gated: yes: gate policy
- Witness (witness/results.md): demonstrated (reading). `gate-streams` starts only `fuzz-build` (justfile:468), `ci` lists `fuzz-build` and not `fuzz` (justfile:1000), and only `all` runs the smoke (justfile:1003); the predicate-flip construction was not run.
- Cross-references: fuzz-guests-pins-14 (the heap cap, also exercised only at that cadence).

The agreement logic that lives in the targets themselves (the `agreed` tables in `span_differential` and `borsh_vs_raw`, the postcard framing arm, the law drive loop, the heap cap) executes only when someone runs the smoke. The seed corpus exists so that "a reintroduced genre-ordering defect ... crashes the very first smoke run" (fuzz_seed_set.rs:157-159), but that first run is at sweep cadence, and an inverted predicate or wrong allowance in a target is invisible to every gate and CI run. Could the instrument go dark while reading green: compiled but never executed.

Evidence:

    358	# `before` breaks a fuzz target in the same commit that lands it, and a
    359	# compile is seconds of gate time. Only the build: the libFuzzer smoke is
    360	# poor per-commit spend and runs at `just all` cadence.

Resolution: Add a gate leg beside `fuzz-build` that replays the committed seeds through each built target once: `cargo +nightly fuzz run --target <host> <target> seeds/<target> -- -runs=0` executes every corpus input and exits; deterministic, seconds, and it turns the seed corpus from a smoke-time asset into a per-commit oracle liveness check. Acceptance: inverting `(_, c, f) if c == f => true` in `span_differential` reddens the new leg on the committed `span_ordered` and `span_crossed` seeds; the committed targets pass it.
Construction: Flip `(_, c, f) if c == f => true` to `c != f` at fuzz_decode_differential.rs:168 and run `just gate`: `fuzz-build` compiles it green, `tests/fuzz_seeds.rs` is untouched, and nothing reddens until `just fuzz`.

### fuzz-guests-pins-10: The fuzz framing is a prose wire contract duplicated across the detached boundary
- Where: crates/before/fuzz/fuzz_targets/fuzz_laws.rs:26-29 (related: crates/before/fuzz/fuzz_targets/fuzz_laws.rs:46-55, crates/before/fuzz/fuzz_targets/fuzz_laws.rs:65, crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:13-15, crates/before/tests/fuzz_seeds.rs:290-299, crates/before/tests/fuzz_seeds.rs:308, crates/before/tests/support/fuzz_seed_set.rs:263-268, crates/before/tests/support/fuzz_seed_set.rs:301-309)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`ARITY_SPAN = 18` at fuzz_laws.rs:65 and again at tests/fuzz_seeds.rs:308; `chunk` at fuzz_laws.rs:46-55 and `laws_chunk` at tests/fuzz_seeds.rs:290-299 are textually identical bodies); executed: no
- Seen by: adequacy [26]; refutation: confirmed; history: no-rationale-found (fd5c92bf5 chose the cross-pointer convention without discussing a shared definition)
- Owner-gated: no
- Cross-references: tests-other-22 and tests-other-18 (the same framing duplication seen from the seed-test side).

`ARITY_SPAN`, the chunk carve, and the decode-ops flavour/length framing are re-spelled in `tests/fuzz_seeds.rs` and `tests/support/fuzz_seed_set.rs`, held together only by "a change here means regenerating the seeds with it". The seed test's in-band assertion reads its own copy, so a narrowed band in the target folds seed arities silently while the test stays green. Every hole found becomes a committed check, never a convention held in memory.

Evidence:

    26	//! feeding the variadic law groups. This framing is a wire contract with
    27	//! the committed seed corpus: `tests/support/fuzz_seed_set.rs` spells seeds
    28	//! in exactly this shape, so a change here means regenerating the seeds
    29	//! with it.
    (tests/fuzz_seeds.rs:308)     const ARITY_SPAN: usize = 18; // the target's arity band, per its framing

Resolution: Extract the framing (`ARITY_SPAN`, `chunk`, `byte`/`picks`, the decode-ops flavour/length carve) into one file `#[path]`-included by the targets, `tests/fuzz_seeds.rs`, and `tests/support/fuzz_seed_set.rs` (the seed set already shares by `#[path]` between the example and the test). Acceptance: `ARITY_SPAN` has one definition; the seed test's in-band assertion reads the constant the target folds with.
Construction: Set `ARITY_SPAN` to 16 in fuzz_laws.rs only: the seed test still passes (its copy asserts `< 18`), while the target now folds the seed's arity-17 script to 1 and never crosses the second octave it was written to cross.

### fuzz-guests-pins-25: wasm32-pins lacks the target-dir redirect its sibling workspaces carry against wasmtime-generated sources under `crates/`
- Where: crates/before/wasm32-pins/Cargo.toml:1-5 (related: crates/before/fuzzfit/.cargo/config.toml:1-7, crates/before-fuelscape/.cargo/config.toml:1-7, tools/doclint:370-373, justfile:179-181, justfile:629-631)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (the two sibling configs exist with the stated rationale; `crates/before/wasm32-pins/.cargo/config.toml` does not; the harness half builds with no `--target-dir` (justfile:631) into `crates/before/wasm32-pins/target`, present on disk; `doclint` walks `root.rglob("*.rs")` under `crates` with no exclusion). Unverified by anyone: that wasmtime's build scripts emit rustdoc'd `.rs` files (the claim rests on the sibling config's rationale); on this machine a global `build.build-dir` diverts intermediates, so the hazard is masked here, and CI never builds this workspace; executed: no
- Seen by: scaffolding [8]; refutation: confirmed; history: no-rationale-found (built "in the fuzzfit idiom" without the idiom's redirect)
- Owner-gated: no

Machinery justified once (the fuzzfit redirect) was not carried to the workspace built in its idiom; a contributor without a global build-dir override gets a red `doclint` after `just wasm32-pins`, attributed to nothing they wrote.

Evidence:

    (crates/before/fuzzfit/.cargo/config.toml:1-7)
    # Build into the repo root's target/ (its own subdirectory) instead of a
    # workspace-local target dir: the gate's doclint sweeps every .rs under
    # crates/, and wasmtime's build scripts generate rustdoc'd sources that
    # would otherwise land inside this tree and fail it. The repo root target/
    # is outside every gate sweep.
    [build]
    target-dir = "../../../target/fuzzfit"
    (crates/before/wasm32-pins/Cargo.toml:1-5)
    # Standalone 32-bit boundary-pin workspace. The empty `[workspace]` table
    # detaches it from the parent `rumors` workspace, so the ordinary gate never
    # builds it (the harness carries wasmtime, a heavy tool-side dependency that
    # must stay out of the production crates' graph; the guest builds only for
    # wasm32-unknown-unknown). Build/run through the `just wasm32-pins*` recipes.

Resolution: Add `crates/before/wasm32-pins/.cargo/config.toml` with `[build] target-dir = "../../../target/wasm32-pins"` (matching `wasm32pins_target`) and the same rationale; the harness fallback path (finding 32) then has one definition to agree with. Acceptance: on a host with no `~/.cargo/config.toml` build-dir override, `just wasm32-pins && just doclint` is green and `find crates/before/wasm32-pins -name '*.rs' -path '*/target/*'` is empty.
Construction: On a host without a build-dir override, `just wasm32-pins-build` then `./tools/doclint crates`: wasmtime's OUT_DIR sources under `crates/before/wasm32-pins/target` are linted.

### fuzz-guests-pins-39: `fuzz-build` lints the detached fuzz workspace with fmt only, unlike its siblings
- Where: justfile:365-368 (related: justfile:588-597, justfile:633-644)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read the three recipes: fuzzfit and wasm32-pins run `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` with the rationale at 636-639; fuzz-build runs fmt only). Not verified: that stable clippy compiles the fuzz workspace (no cargo command was permitted; libfuzzer-sys builds on stable via `cc`, and `cargo fmt --check` already runs there on the default toolchain, so this is likely but unrun); executed: no
- Seen by: adequacy [33]; refutation: confirmed (not run); history: no-rationale-found (1a827d68 gave fuzzfit fmt plus clippy and fuzz-build fmt alone with no stated reason; wasm32-pins later received both)
- Owner-gated: yes: gate policy
- Cross-references: module-graph-8 (the same missing leg, from the module-graph sweep).

The justfile's own argument for the sibling legs ("without them its source rots invisibly through green gates") applies verbatim to the fuzz workspace; clippy never sees the five targets or the harness lib.

Evidence:

    365	[working-directory("crates/before/fuzz")]
    366	fuzz-build:
    367	    cargo fmt --check
    368	    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}

Resolution: Add `cargo clippy --all-targets -- -D warnings` to `fuzz-build`, mirroring wasm32-pins:642-643; if stable clippy cannot build the `#![no_main]` bins, pin a clippy component on the nightly toolchain instead. Acceptance: a `clippy::needless_borrow` planted in a fuzz target reddens `just gate`.
Construction: Insert `let _ = &(&data);` in `fuzz_decode.rs::run`: `just gate` stays green; `cargo clippy --all-targets -- -D warnings` in `crates/before/fuzz` fails.

### fuzz-guests-pins-6: The differential's composite allowance admits every `(NotCanonical, TrailingBits)` pair on `Ranked` and `Span`
- Where: crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:293-300 (related: crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:130-139, crates/before/fuzz/fuzz_targets/fuzz_decode_differential.rs:237-245, crates/before/src/borsh_impls.rs:275-287, crates/before/tests/support/fuzz_seed_set.rs:195-203)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read the arm, the borsh `Span` reader's cursor-then-verdict order, and the `span_crossed_padding` seed); executed: no
- Seen by: adequacy [31]; refutation: confirmed, narrower than stated (a wrong pair verdict on an input with no trailing bytes is still caught by the Err-arm expect); history: deliberate-and-holds (the allowance and its scope are documented at 237-245; conditioning it on the stage was never considered)
- Owner-gated: no

The arm admits `(NotCanonical, TrailingBits)` whenever `composite` is true, regardless of whether borsh actually rejected at its composite check. A composite-local defect in the borsh reader that reports `NotCanonical` for a structural fault the raw door calls `TrailingBits` is admitted by the same arm, and `span_differential` decodes `hi` raw (line 134), so it never sees the borsh path. The cheapest passing artifact: an allowance conditioned on a type admits every divergence of that genre pair on that type, not only the documented one.

Evidence:

    293	            let agreed = match (borsh, raw) {
    294	                (b, r) if b == r => true,
    295	                // A prefix-scoped composite check rejects where the
    296	                // whole-slice parse still sees unconsumed input — possible
    297	                // only on the types that run one.
    298	                (Genre::NotCanonical, Genre::TrailingBits) => composite,
    299	                _ => false,
    300	            };

Resolution: Condition the allowance on evidence that the composite check was reached: parse both components with the borsh prefix readers, run the pair check, and allow `(NotCanonical, TrailingBits)` only when that composed spelling rejects at `Stage::Pair` with a nonempty remainder; otherwise require exact genre agreement. Acceptance: a borsh `Span` reader mutated to wrap `hi`'s `TrailingBits` as `NotCanonical` diverges on the committed `span_crossed_padding` seed; the committed reader agrees on every seed.
Construction: In `borsh_impls.rs`'s `Span::deserialize_reader`, map the cursor's `Decode::TrailingBits` (line 285) to `NotCanonical` before the pair check, then run `fuzz_decode_differential` on `seeds/fuzz_decode_differential/span_crossed_padding`: raw `Span::decode` says `TrailingBits`, borsh says `NotCanonical`, `composite = true` admits it.

### fuzz-guests-pins-9: `drive_clock` swallows `join`/`sync` errors its own construction proves impossible
- Where: crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:77-91 (related: crates/before/src/clock.rs:199-202, crates/before/src/clock.rs:322-328, crates/before/src/laws.rs:2029-2040, crates/before/tests/support/fuzz_seed_set.rs:279-288)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read `Clock::join`'s `# Errors` and `Clock::sync`'s `sum_split` arm: overlap is the only error path); executed: no
- Seen by: structure-prose [47]; refutation: confirmed, severity low (the single-fork case is already caught by `fork_halves_disjoint` in `fuzz_laws`; the residual is multi-op sequences); history: no-rationale-found (the hedge is verbatim from the target's first commit)
- Owner-gated: no

Every stash entry is `clock.fork()` or a child `sync` re-split from `clock`, so `join` and `sync` can fail here only if `Party::fork` or `sum_split` produced overlapping parties on a decoded tree, which is exactly the bug class this target exists to reach; the code pushes the error back or discards it under "on the off chance it errors". The cheapest passing artifact: a target whose contract is "no panic" should panic on the one error it can prove impossible, or the fuzzer minimizes nothing when a decoded party forks into overlapping halves after prior ops.

Evidence:

    78	                if let Some(child) = stash.pop() {
    79	                    // Re-join a disjoint fork; on the off chance it errors, keep the
    80	                    // returned clock so we never lose the share.
    81	                    if let Err(returned) = clock.join(child) {
    82	                        stash.push(returned);
    83	                    }
    84	                }
    ...
    88	                    let _ = clock.sync(&mut child);

Resolution: `.expect("a forked or re-split child is disjoint from its origin")` on both, with the comment rewritten as the one-line proof (every stash entry is a fork or sync re-split of `clock`, so overlap here is a `Party::fork`/`sum_split` defect). Acceptance: a `Party::fork` deliberately returning an aliased party crashes `cargo +nightly fuzz run fuzz_decode_ops seeds/fuzz_decode_ops` on the `clock_then_ops` seed (ops `[0, 1, 3, 5, 2, 4, 6, 7]` drive fork, sync, then join).
Construction: Locally make `Party::fork` return `self.dangerously_alias()` and replay the seed: today the run stays green because `join`'s `Err` is pushed back and `sync`'s `Err(Overlap)` is dropped.

### fuzz-guests-pins-11: One-byte length prefixes cap every decoded law-target operand at 255 bytes
- Where: crates/before/fuzz/fuzz_targets/fuzz_laws.rs:46-55 (related: crates/before/fuzz/fuzz_targets/fuzz_decode_ops.rs:32-38, crates/before/tests/support/fuzz_seed_set.rs:277, crates/before/tests/support/fuzz_seed_set.rs:313-318)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read both carves and the seed set's `u8::try_from(...).expect(...)`); executed: no
- Seen by: adequacy [32]; refutation: reframed (the cap binds every `fuzz_laws` operand; in `fuzz_decode_ops` it binds only the seed value, since the script's fork/join/ticks ops grow the driven clock without bound); history: no-rationale-found
- Owner-gated: no

`fuzz_laws` decodes every operand from a `u8`-length chunk, so no law is ever driven on a decoded tree over 255 bytes, and any input past about 1.5 KB spends its excess on the list scripts; the deep and wide regime where the fold shapes and the heap cap have work to do is unreachable for this target. The sizing choice is stated nowhere. In `fuzz_decode_ops` the same prefix bounds only the seed value. Does the sweep's population reach the worst case the crate docs admit exists.

Evidence:

    46	fn chunk<'d>(data: &mut &'d [u8]) -> &'d [u8] {
    47	    let Some((&len, rest)) = data.split_first() else {
    48	        *data = &[];
    49	        return &[];
    50	    };
    51	    let split = (len as usize).min(rest.len());

Resolution: Widen the prefix to `u16` (little-endian, saturated at the remainder) in both targets and regenerate the seeds, or state at the framing why 255 bytes is the intended operand ceiling for the law target. Acceptance: a regenerated seed carrying a canonical clock over 255 bytes decodes in `fuzz_laws` and drives every group; the seed set's `u8::try_from` becomes `u16`.

**Fuzz-fit strategies and driver**

### fuzzfit-strategies-7: 49 measured guest kernels have no `Op`, so those public operations have no fuzz-fit band and the roster pin cannot see them
- Where: crates/before/fuzzfit/harness/src/ops.rs:49-56 (related: crates/before/fuzzfit/harness/tests/sanity.rs:87-177, crates/before/fuzzfit/guest/src/lib.rs:1844-1919, crates/before-fuelscape/src/ops.rs:14-22, crates/before-fuelscape/src/ops/tests.rs:17-28, crates/before/src/testing/validation_index.rs:121-142, .agent-notes/2026-07-26-before-fuzzfit-asymptotics/before-fuzzfit-asymptotics.md:189-193)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (comm as in finding 6; of the 63 exports without an Op, 7 are control or self-test exports (`ff_nop`, `ff_regs_reserve`, `ff_reset`, `ff_selftest_quadratic`, `ff_stage_len`, `ff_stage_prepare`, `ff_stage_ptr`) and 7 are query constructors whose docs say "unmeasured preparation" (guest/src/lib.rs:1844-1919), leaving 49 measured kernels; read the roster test at sanity.rs:98-177, which binds `Op` variants to `BANDS` only); executed: yes: the comm and the grep for "unmeasured preparation" settle the count; the construction below was not run
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed, severity high to medium (fuelscape's parity test already binds public-API additions to the surface roster; the envelope suite and bench-judge cover these operations on chosen shapes; what fuzzfit lacks is its own binding and unchosen-shape coverage); history: no-rationale-found (61398dc9 and the design note record public-API additions as fuzzfit re-pin events that "fail by name"; the guest grew from 51 to 107 exports across six fuelscape commits, each stating the fuzzfit vocabulary was deliberately left unchanged, and no ruling records that fuzzfit's scope excludes those operations)
- Owner-gated: yes: whether to price these operations here or record the fuzzfit/fuelscape division as a ruling is a scope decision
- Cross-references: meter-adequacy-2 (the same gap, counted as 56 measured exports before excluding the 7 query constructors).

The 49 measured kernels with no `Op` (`ff_version_ticks`, `ff_clock_forks`, `ff_party_join_all`, `ff_clock_join_all`, `ff_clock_sync_all`, `ff_clock_recv_all`, `ff_clock_display`/`fromstr`, `ff_version_eq`/`hash`, `ff_party_hash`, the shape walks and `ff_shape_combine`, `ff_version_span`/`span_all`, `ff_own_version_cmp`/`pair_cmp`, `ff_rank_encode`/`decode`, the four `ff_ranked_*`, every `ff_span_*`/`ff_own_span_*`, `ff_query_contains`/`coverage`/`conjoin`, `ff_floor_contains`, `ff_ceiling_contains`) are never emitted, fitted, or judged; the only roster test pins `Op` to `BANDS` in both directions, which is blind to a kernel with no `Op`; and the validation index says the fuelscape atlas "enforces nothing", so nothing prices these operations on shapes nobody chose (Principle 6, totality: the design note's claim that a public-API addition fails by name until its band is pinned is not true of this instrument, and before-fuelscape already shows the shape of the missing jaw by binding its roster to `before::surface::METHOD_SURFACE` with a reviewed exemption list).

Evidence:

        49	/// One public operation over registers; the harness's program alphabet.

    sanity.rs:87	/// The pinned bands and the op vocabulary name the same kernels, pinned
    sanity.rs:88	/// here as an expectation list: every roster kernel has at least one
    sanity.rs:89	/// pinned band, and every pinned band prices a roster kernel.

    design note:189	every run would mask drift). **Re-pin events**: a guest toolchain bump
    design note:190	(asserted mechanically, §2), a kernel change, a `before` public-API
    design note:191	addition (a new operation means a new kernel, a new op, and a new band —
    design note:192	the harness's kernel-roster test fails by name until the band is
    design note:193	pinned), a strategy change. Re-pin =

    fuelscape ops.rs:16	//! Coverage is bound to the surface-coverage suite's committed roster
    fuelscape ops.rs:17	//! (`before::surface`): every roster row is either claimed by a panel's
    fuelscape ops.rs:18	//! `covers` list or carries a one-line reason in [`EXEMPTIONS`], and the

Resolution: bind the `Op` vocabulary to `before::surface::METHOD_SURFACE` the way fuelscape does (enable the `surface` feature on the harness's `before` dependency; add a parity test requiring every method-surface row to have either an `Op` whose kernel is pinned in `BANDS` or a one-line reviewed exemption naming why it is outside the register-machine vocabulary). Add `Op`s and strategy emissions for the operations that fit the vocabulary today (`ticks`, `forks`, the `*_all` doors, `span`/`span_all`, `eq`, the `Rank` codec, `Ranked`, clock text I/O) and re-pin with `just fuzzfit-calibrate`; exempt the rest by name. Acceptance: a committed parity test fails by name for any `METHOD_SURFACE` row with neither an `Op` nor an exemption; the 49-kernel gap shrinks to an explicit exemption list; `just fuzzfit` green after the re-pin.

Construction: insert a `std::hint::black_box`-pinned quadratic loop into `Version::ticks` (or `Clock::forks`, or `Party::join_all`), rebuild the guest, run `just fuzzfit`: every test stays green, because no strategy emits those kernels and no band exists to judge them.

### fuzzfit-strategies-11: The mirror omits join and meet's empty-operand rungs, so O(1) steps enter the fitted cloud and the `ff_version_join`/`ff_version_meet` liveness floors are about 2.8 decades wide
- Where: crates/before/fuzzfit/harness/src/ops.rs:603-627 (related: crates/before/fuzzfit/harness/src/drive.rs:57-74, crates/before/fuzzfit/harness/src/bands.rs:91-96, crates/before/fuzzfit/harness/src/bands.rs:808-855, crates/before/fuzzfit/harness/src/strategies.rs:772-785, crates/before/fuzzfit/harness/src/strategies.rs:961-980, crates/before/fuzzfit/harness/src/strategies.rs:1219-1238, crates/before/fuzzfit/harness/src/strategies.rs:1272-1273, crates/before/src/version.rs:152, crates/before/src/version.rs:864-925, crates/before/tests/meter.rs:10746-10807)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified for the mechanism (read `join_view`/`join_refs`/`meet_view` at version.rs:864-925; `Version::is_empty` is `pub` at 152; `Version`'s `PartialEq` is `codec::canonical_eq` at version.rs:1724-1727, so the mirror's `va == *vb` is exactly the first rung and nothing more; `Clock::fork` clones the parent's version at clock.rs:155 and `CliffComb`, `RevealComb`, `AscendCliff`, and `CombScatter` never tick their seed before `fork_balanced`, so un-ticked teeth hold empty versions; battery arm 6 at 772-785 extracts `version_of(ca)` from `pools.clocks` as the join/meet spare; the pinned widths read from bands.rs); attribution of the width to these steps is inferred (no run); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed (no other O(1) path exists in the join/meet ladder); history: no-rationale-found, with corroboration: the empty rungs entered version.rs at 58a37d80 (07-27) before the predicate was written at 306e2de0 (07-30); the meter pin for them landed the next day (89def207) touching no fuzzfit file; and the identity-routing re-pin did not narrow these floors (join `width_below` 2.664 at b528f2c6, 2.796 at 306e2de0, 2.790 at HEAD; meet 2.848, 2.848, 2.840) while every routed key elsewhere sits under 0.9
- Owner-gated: no
- Witness (witness/results.md): inconclusive. Verified by reading: the mirror's identity predicate for `VersionJoin`/`VersionMeet` is `va == *vb` alone (ops.rs:612, 624) while `join_refs`/`join_view` and `meet_view` also settle in O(1) on an empty operand (version.rs:864-925); the pinned `width_below` values were confirmed at bands.rs:814 and 850. That empty-operand steps account for the width was not measured.

The harness's own criterion (drive.rs:57-66, bands.rs:91-96) says steps that dispatch an identity-law fast path are O(1) by mechanism and must leave the sample stream because fitting them beside the walked cloud makes both bands decoration-wide; before's `join_view`/`join_refs` and `meet_view` have three such rungs each (canonical equality; `v ∨ 0 = v`; `0 ∨ v = v`; and the meet duals `0 ∧ v = 0`, `v ∧ 0 = 0`) and `identity_fast_paths::empty_operands_answer_without_a_walk` pins the empty ones as part of the same ladder, but the mirror models only the first. The pinned bands carry the symptom: `ff_version_join` `width_below` 2.789563 and `ff_version_meet` 2.839589 against 0.399089 (`ff_version_distance`), 0.384644 (`ff_version_lag`), and 0.869608 (`ff_version_cmp`); with `ENFORCE_MARGIN_BELOW` 0.8 on top, an accidentally constant join or meet reads in band except at the largest denominators, which voids bands.rs:36-38's liveness claim ("below-band is a liveness flag") for exactly these two keys (Principle 2: a ceiling over a counter passes vacuously when the floor excludes nothing). The related `RevealComb` comment at 1224-1225 ("each consume revealing the shared minimum to the floor frame") describes consumes that are empty-rung no-ops for every odd tooth when `hifloor` is false, so the treatment arm exercises join less than its control does.

Evidence:

       610	                // The lattice op's rung is canonical equality
       611	                // (a ∨ a = a hands the operand back).
       612	                let identity = va == *vb;
       613	                self.put(dst, NVal::V(va | vb));
       614	                done_pair(denom, OK, identity)

    version.rs:893	        if skyline::is_empty_stream(b.0.live()) {
    version.rs:894	            return a.clone(); // v ∨ 0 = v
    version.rs:895	        }
    version.rs:896	        if skyline::is_empty_stream(a.0.live()) {
    version.rs:897	            return b.clone(); // 0 ∨ v = v
    version.rs:898	        }

    bands.rs:814	        width_below: 2.789563,
    bands.rs:850	        width_below: 2.839589,

Resolution: extend the predicate for `VersionJoin` and `VersionMeet` to `va == *vb || va.is_empty() || vb.is_empty()`, reword the comments at 610-611 and 623 to name all three rungs, and note beside `Step::identity` that the empty rungs' liveness is owned by `empty_operands_answer_without_a_walk`; add a sanity.rs pin that `Mirror::step` reports identity for an aliasing pair, a byte-equal distinct-buffer pair, and an empty-operand pair, and not for a concurrent pair; then `just fuzzfit-calibrate` and commit the re-pin with the movement annotated (the deterministic stream is unchanged; only which steps are sampled moves). Acceptance: after the re-pin, `ff_version_join` and `ff_version_meet` `width_below` fall into the range the other pair kernels occupy (below 1.0 decade), their `samples` counts drop by the excluded mass, the new sanity pin is green, and the enforcement suite is green at the new pin.

Construction: over `for_each_deterministic_program`, log `(denom_bits, fuel, va.is_empty() || vb.is_empty())` for every `VersionJoin`/`VersionMeet` step (a temporary `eprintln!` in `Mirror::step`): every residual more than about 1.5 decades below the pinned line should be an empty-operand step, and `RevealComb` at `hifloor = false`, `CliffComb`, and `CombScatter` draws should contribute most of them. Then `judge_against(band_for("ff_version_join", false), d, f)` with `f` the fuel of such a step returns `InBand` for any `d` in the calibrated range, showing the floor cannot tell the adopt path from a walk.

### fuzzfit-strategies-16: The escalation replays and the bootstrap stream carry reach claims with no committed floor; the builder truncates silently when a budget binds
- Where: crates/before/fuzzfit/harness/src/strategies.rs:96-111 (related: crates/before/fuzzfit/harness/src/strategies.rs:151-159, crates/before/fuzzfit/harness/src/strategies.rs:387-434, crates/before/fuzzfit/harness/src/strategies.rs:1538-1546, crates/before/fuzzfit/harness/src/strategies.rs:1679-1716, crates/before/fuzzfit/harness/src/bin/calibrate.rs:116-131, crates/before/fuzzfit/harness/tests/enforce.rs:207-239, crates/before/fuzzfit/harness/tests/enforce.rs:398-436, crates/before/fuzzfit/harness/tests/sanity.rs:16-39)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified for the mechanism (every cap refusal in `B` returns `false`/`None` silently at 409-416, 426-434, 482-490, 556-564, 589-600; the depth loop breaks on a refused fork or tick at 1539 and 1544-1546; the tail rows at 1679-1716 skip on `!room()`; `programs_respect_the_budget` asserts ceilings only; the replay tests judge whatever program results); the op count is a hand model, not a run: at depth 1792, `keep_every` is 44, the spine costs 3584 ops, the 1751 non-cadence levels about 3503, the 41 cadence levels about 538, the tail 50, so about 7676 construction ops plus at most 192 battery ops of 9000, 1832 of 3000 ticks, 1842 of 2048 forks (the refutation pass's independent model gives 7675); executed: no
- Seen by: scaffolding, adequacy; refutation: confirmed (the escalation half; the bootstrap half low because the small bands also pool the main corpus's sub-floor samples and the at-least-once floor exists); history: no-rationale-found (f66d7c17 raised `max_ops` from 8000 to 9000 and wrote "Sized to admit" in the same diff with no test of the margin; BOOTSTRAP_MAX_ROUNDS's "tops out just under the fit floor" was established by a probe sweep and recorded as a manual procedure)
- Owner-gated: no
- Witness (witness/results.md): inconclusive. Not constructed; the fuzzfit workspace is outside what the witness agent could build or run, and it did not independently verify the line citations.

`ESCALATION_BUDGET`'s doc says the budget admits the full depth draw plus the cadence battery, and enforce.rs names the two fixed replays as the standing proof that the at-scale rows "still bite", but nothing witnesses that either replay was emitted whole: if a cap binds mid-construction, the rows that make the proof (the two finished halves' `PartyIsDisjoint`/`PartyCovers`/`PartyJoin` at top size, the seed sync, the ladder fold, the top `ClockJoin`) drop out and every leg reads green on smaller in-band steps; today the depth-1792 program fits on about a 12 percent op margin and a 10 percent fork margin that no test reads (Principle 2: a ceiling passes vacuously when the signal goes dark; the cheapest passing artifact is a truncated proof). The bootstrap half is the same genre at lower stakes: BOOTSTRAP_MAX_ROUNDS's doc claims the corpus covers the sub-floor span "end to end", calibrate.rs:121 filters `s.denom_bits < FIT_FLOOR_BITS` silently, and the only committed floor is "judged at least once" (enforce.rs:232-239), while 157-158 states a manual re-derivation as the procedure.

Evidence:

        98	/// Sized to admit the family's full depth draw (256..=1792 spine forks)
        99	/// plus the cadence battery riding on it, so every kernel row — pair,
       100	/// fold, query, and the single-operand ticks/sends/recvs/splits — sees
       101	/// denominators decades past the rest of the roster and the fitted
       102	/// *slope*, not the band's width, carries the asymptotic judgment there.

       426	    fn fork(&mut self, src: Reg) -> Option<Reg> {
       427	        if !self.room() || self.forks >= self.budget.max_forks {
       428	            return None;
       429	        }

      1539	                let Some(child) = b.fork(seed) else { break };

       157	/// small bands price. Re-derive by sweeping the printed denominator
       158	/// ranges in `bin/calibrate` when the growth-per-round changes.

Resolution: have `B` count refusals (every early return in `tick`/`fork`/`party_fork`/`party_forks`/`clock_dup`/`party_dup`/`join_all_versions`/`version_of` and every `if room()` guard that skips an emission) and expose it (`pub fn build_reporting(family, seed) -> (Vec<Op>, u32)` with `build` delegating); in sanity.rs assert zero refusals for both `ESCALATION_REPLAYS` entries and for every bootstrap program, so a cap that binds on a corpus of record fails by name; optionally pin each replay's key roster (each band key the doc names present with a maximum denominator above a stated floor, one tick per level being the irreducible growth). For the bootstrap stream, assert every sample is sub-floor and the small-band kernels' maximum denominator lies within a stated distance below `FIT_FLOOR_BITS`, replacing the manual procedure at 157-158. Acceptance: temporarily lowering `ESCALATION_BUDGET.max_ops` to 7000, or setting `BOOTSTRAP_MAX_ROUNDS` to 3 or 40, reads red by name in `just fuzzfit`; at the committed values it is green, and the doc at 96-111 points at the witness.

Construction: set `ESCALATION_BUDGET.max_ops` to 6000 and run the fuzzfit suite: the depth loop breaks when room runs out, the depth-1792 replay lacks its top-size `PartyIsDisjoint`/`PartyCovers`/`PartyJoin`, seed sync, ladder fold, and top `ClockJoin`, and `the_escalation_depth_cap_stays_in_the_pinned_bands` still passes.

### fuzzfit-strategies-5: The pinned bands are not bound to the corpus that produced them
- Where: crates/before/fuzzfit/harness/src/drive.rs:103-105 (related: crates/before/fuzzfit/harness/src/strategies.rs:1979-2017, crates/before/fuzzfit/harness/Cargo.toml:26-32, crates/before/fuzzfit/harness/src/bands.rs:3-8, crates/before/fuzzfit/harness/src/bands.rs:196-218, crates/before/fuzzfit/harness/tests/enforce.rs:355-396)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read); executed: no
- Seen by: scaffolding; refutation: confirmed, severity medium to low (the sentry and refit legs catch every drift that changes a verdict; what escapes is sub-tolerance drift and stale `samples`/`min_denom`/`max_denom` metadata); history: no-rationale-found, with corroboration: 4e64a4fb (2026-08-31) bumped wasmtime in the fuzzfit Cargo.lock, a re-pin event calibrate.rs:366-368 declares, touched no bands.rs, and reports a clean gate
- Owner-gated: no
- Cross-references: fuzzfit-bands-6 (the wasmtime half of the same provenance gap).

The corpus of record is a pure function of `any_family`'s draw ranges and weights, proptest's deterministic runner, `ChaCha8Rng`, and `build`; bands.rs names "a strategy change" as a re-pin event, but nothing checks it, and `REFIT_TOLERANCE` is 0.7 decades, so a generator edit or a proptest/rand bump can leave committed widths and sample counts describing a corpus that no longer exists with every deterministic test green (Principle 6, provenance form: bind every measurement to its run; the toolchain is asserted mechanically, the generators and their RNG dependencies are a convention held in memory).

Evidence:

       103	/// The family stream comes from proptest's deterministic runner and each
       104	/// program's seed is its case index, so any two consumers observe
       105	/// byte-identical samples for the same `programs` count. The calibration

    bands.rs:5	//! constants and never refits. To re-pin — after a deliberate guest
    bands.rs:6	//! toolchain bump, a kernel change, or a strategy change — run
    bands.rs:7	//! `just fuzzfit-calibrate`, review the diff like a snapshot, and commit
    bands.rs:8	//! with a dated movement annotation.

Resolution: have `calibrate` write a corpus digest beside `PINNED_RUSTC` (the harness already has an FNV; hash the ops of the first `REFIT_PREFIX_PROGRAMS` deterministic programs plus the bootstrap stream into `pub const CORPUS_DIGEST: u64`), and add an enforce.rs test beside `building_toolchain_matches_the_pin` that recomputes it from `for_each_deterministic_program`/`for_each_bootstrap_program` and names `just fuzzfit-calibrate` on mismatch; state in harness/Cargo.toml that proptest and rand_chacha are pin provenance. Acceptance: changing any draw range, weight, or family body, or bumping proptest/rand_chacha in Cargo.lock, fails `just fuzzfit` by name before any fuel is judged; a re-pin restores green and the diff shows the digest move.

Construction: change `Benign`'s weight in `any_family` from 8 to 9 and run `just fuzzfit`: every deterministic test stays green although `BANDS`' `samples` counts and `REFIT_COVERAGE` were computed from a corpus that no longer exists.

### fuzzfit-strategies-8: The roster test's hand list is a convention against a variant added without a roster entry; a derived variant list is a check
- Where: crates/before/fuzzfit/harness/src/ops.rs:153-156 (related: crates/before/fuzzfit/harness/tests/sanity.rs:87-99, crates/before/fuzzfit/harness/tests/enforce.rs:56-65)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (the refutation pass script-checked that all 44 kernel names equal `ff_` plus the snake_case of the variant name with zero mismatches; I read the roster test and `judge`'s missing-band panic at enforce.rs:59-65); executed: no
- Seen by: structure-prose; refutation: reframed (the actionable gap is the roster's non-totality, not the explicit string table, which is legible and panics on a wrong name at first call); history: deliberate-and-holds for the hand roster's purpose (61398dc9: one representative op per variant as a committed expectation list), which a derived list satisfies identically
- Owner-gated: no
- Cross-references: fuzzfit-bands-30 (the same hand roster from the bands partition), meter-registry-tier2-11 and surface-roster-3 (the hand-roster pattern).

The roster test's doc says "a variant added to the vocabulary belongs in this list": a variant added with a kernel string but no roster entry passes the roster test until some generator emits it, and only then does `judge` panic on the missing band (Principle 6: the doc claims the hole fails "before any generator has to happen to sample the hole", which the hand list cannot deliver for an unlisted variant). Deriving the variant list (strum's `EnumDiscriminants` plus `VariantArray`, or `EnumCount` asserted against the roster length) closes it; deriving `Op::kernel` itself is optional taste.

Evidence:

       153	    /// The guest kernel this op calls; also the calibration's band key.
       154	    pub fn kernel(&self) -> &'static str {
       155	        match self {
       156	            Op::ClockSeed { .. } => "ff_clock_seed",

    sanity.rs:94	/// sample the hole. The roster is one representative op per `Op`
    sanity.rs:95	/// variant: a variant added to the vocabulary belongs in this list, and
    sanity.rs:96	/// its kernel in the pinned bands.

Resolution: derive a variant count or discriminant array on `Op` and have `bands_and_op_roster_name_the_same_kernels` iterate it (or assert the hand roster's length against the derived count so an omission fails by name). Acceptance: adding an `Op` variant with a kernel string but no roster entry fails the sanity suite before any generator emits it.

Construction: add `Op::ClockTicks { src: Reg }` with `=> "ff_clock_ticks"` in `kernel` and `args`, no roster entry, no band, no strategy emission; run `just fuzzfit`: the roster test stays green.

### fuzzfit-strategies-20: The register-appetite premise behind `REGS_RESERVE` is stated in prose but not pinned over generated programs
- Where: crates/before/fuzzfit/harness/src/strategies.rs:382-385 (related: crates/before/fuzzfit/harness/src/wasm.rs:28-47, crates/before/fuzzfit/harness/tests/sanity.rs:16-39, crates/before/fuzzfit/guest/src/lib.rs:269-284)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read every `alloc` site: `split_parts` allocates two, `party_forks` allocates `n`, every other emitter one, so the premise holds today; `programs_respect_the_budget` asserts ops, ticks, forks, and fold width only); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found (4e861272 fixed a hand miscount of this very premise, "REGS_RESERVE's comment undercounted the worst per-program appetite", and added the const assert on the constants, not on the builder's behavior)
- Owner-gated: no

The const assert in wasm.rs pins `REGS_RESERVE >= 2 · max_ops + max_forks`, but the other half of the argument, that the builder never allocates more than that, is prose; a builder change adding a third destination to an op or a scratch slot to a fold keeps every committed test green while making the premise false, and the symptom would again be a false above-band flag on a program near `max_ops` (the guest doc records that incident), not a named failure (Principle 6: a premise a mechanism rests on gets a committed check; today's slack, 32768 reserved against 20048 derived, makes the risk low and the pin cheap).

Evidence:

       382	    fn alloc(&mut self, ty: Ty) -> Reg {
       383	        self.slots.push(ty);
       384	        (self.slots.len() - 1) as Reg
       385	    }

    wasm.rs:31	/// so the file never reallocates during a measured call. The bound:
    wasm.rs:32	/// every op allocates at most two registers (`into_parts`), except
    wasm.rs:33	/// `Party::forks(n)` allocates `n` in one op — and a program's total

    sanity.rs:23	        prop_assert!(program.len() <= budget.max_ops, "{} ops", program.len());

Resolution: in `programs_respect_the_budget`, compute the largest register index the program writes (`dst` fields; `dst + n - 1` for `PartyForks`) and assert `max_index + 1 <= 2 * budget.max_ops + budget.max_forks` (the premise the const assert encodes), or expose `REGS_RESERVE` and assert directly against it. Acceptance: the sanity suite fails by name when an `Op` variant or builder path allocates past the documented bound; the const assert and this test together cover both halves of the argument.

Construction: add a third `alloc(Ty::R)` scratch slot to battery arm 3 (two ops, three slots): no committed test fails, yet the "at most two registers" premise is now false; nothing names it until a program near the budget reallocates the file inside a measured call.

**Fuzz-fit bands, fitter, enforcement**

### fuzzfit-bands-5: `REFIT_TOLERANCE` is one global 0.7 decades where the per-key evidence is deterministic and exact
- Where: crates/before/fuzzfit/harness/src/bands.rs:207-218 (related: crates/before/fuzzfit/harness/tests/enforce.rs:367-395, bands.rs:42-70, bands.rs:989-1038, crates/before/fuzzfit/harness/src/bin/calibrate.rs:238-272, crates/before/fuzzfit/harness/src/drive.rs:101-108)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: assessed (read); executed: no
- Seen by: scaffolding; refutation: confirmed; history: already-known (the design note's 2026-07-26 ruling accepts the sub-3x uniform meter-degradation residual "without new machinery", rejecting a dedicated meter-calibration instrument; the per-key proposal here is not that instrument and is not addressed by the ruling)
- Owner-gated: yes: it reopens a recorded ruling and changes re-pin cadence
- Witness (witness/results.md): inconclusive. Grep confirms one global `REFIT_TOLERANCE: f64 = 0.7` at bands.rs:218; whether a uniform 2x fuel undercount on one key passes every leg was not tested.

The staleness check compares an exactly reproducible per-key quantity (the prefix refit's line against the pin; drive.rs:101-108 documents the stream as byte-identical across consumers) against one tolerance sized by the worst key's pin-time sampling gap. It measures `|d|` against 0.7 rather than `|d - d0|` against a slack, so on most keys it is deaf to roughly 4.5x drift and on every key to a 2x uniform undercount (the "k = 2 hides" the design note records). Per-key pinning is the same comparison with a generated constant per key, and `REFIT_COVERAGE` is already the per-key list those values would ride on; `calibrate` already computes `d` per key at calibrate.rs:244-255. Principle 2: a threshold over a stable quantity belongs in a measured gap.

Evidence:

       210	/// Measured at pin time (`bin/calibrate` re-derives the
       211	/// evidence on every re-pin; the 4096-program corpus): the prefix's own
       212	/// sampling difference from the full corpus peaks at 0.497
       213	/// (`ff_party_join`'s rejection arm, the thinnest-sampled band key
       214	/// genre). The tolerance sits above that, so the
       215	/// check is deaf to sub-half-decade drift on the worst key but fails
       216	/// loud on anything past ~5x in fuel constants — a staleness detector,
       217	/// not the criterion of record.
       218	pub const REFIT_TOLERANCE: f64 = 0.7;

Resolution: Generate `REFIT_COVERAGE` as `(kernel, rejected, divergence_at_min, divergence_at_max)` carrying the signed pin-time endpoint deltas; in the enforcement test compute the fresh deltas and assert `|fresh - pinned| <= REFIT_SLACK` at each endpoint, with `REFIT_SLACK` set deliberately as the cadence parameter (0.2 matches `ENFORCE_MARGIN` and catches k = 2). Keep `REFIT_TOLERANCE` only if the coarse absolute bound is also wanted. Re-denominate the blessed-drift paragraph from the slack. Acceptance: a test that halves `Measured.fuel` for one kernel inside `Guest::call` reads red on the deterministic prefix by name; on unchanged code every pinned delta reproduces to floating-point noise.
Construction: In `wasm.rs` `Guest::call`, temporarily return `fuel: (FUEL_TANK - remaining) / 2` when `name == "ff_version_tick"` (log10 2 = 0.301). Point leg: every sample drops 0.301 below its line, inside `width_below + 0.8`, no `Below`. Shape leg: slopes unchanged. Staleness leg: `|d| <= 0.301 + d0 < 0.7` on every covered key. The suite stays green. With per-key pinned deltas and slack 0.2, `ff_version_tick`'s fresh delta moves by 0.301 > 0.2 and the test reads red.

### fuzzfit-bands-10: The shape leg's allowance is stacked on the mixture-tilted pooled slope, so the within-case threshold reaches exponent 1.4–1.55 on the worst keys
- Where: crates/before/fuzzfit/harness/src/curve.rs:61-71 (related: curve.rs:113, crates/before/fuzzfit/harness/src/bands.rs:98-106, bands.rs:595, bands.rs:607, bands.rs:751, crates/before/fuzzfit/harness/tests/enforce.rs:139-153, crates/before/fuzzfit/harness/src/bin/calibrate.rs:218-237)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (Python over the parsed constants: for the twelve non-exempt keys with pooled slope > 1.1 the escaping exponent `min(slope + 0.3, slope + (width_above + 0.2) / span_decades)` is 1.549 `ff_party_join`, 1.543 `ff_party_is_disjoint`, 1.465 `ff_version_decode`, 1.417 `ff_version_cmp`/`concurrent`, 1.628–1.709 on the rejection arms; a `d^1.4` mechanism anchored at `ff_version_decode`'s 128-bit line reaches log fuel 6.754 at 8768 bits under the ceiling 6.872 with within-case excess 0.225 < 0.3); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed; history: already-known in part (the design note's residual-risk section states the per-row bound and rules per-family bands "architectural" and out of scope; the note's listed worst rows are ~1.28 and understate the current pin, and changing only the shape leg's reference slope is not the architectural change ruled out)
- Owner-gated: yes: a criterion change; reopens the note's residual-risk ruling with new arithmetic
- Witness (witness/results.md): demonstrated (arithmetic over constants read from bands.rs:749-758 and 605-614, `ENFORCE_MARGIN` 0.2, `SLOPE_ALLOWANCE` 0.3, and `local_slope_excess`'s `- band.slope`). A hypothetical d^1.45 mechanism on the `ff_version_decode` line stays under both the ceiling leg (6.845 < 6.872 at max_denom) and the shape leg (excess 0.275 < 0.3); `ff_party_join`'s shape threshold is 1.5515. Not run through the harness.

`local_slope_excess` subtracts `band.slope`, the pooled log-log fit, from a within-case local slope. The module doc of `bands.rs` states that pooled slopes above 1.1 are "family-mixture composition" that "every lane's own medians" do not follow, and `curve.rs`'s whole argument for judging within one case is that a family-pure population cannot be mixture-tilted; the leg then compares it to the mixture-tilted number anyway. The allowance's stated premise ("a third of the +1.0 a quadratic mechanism adds over a linear pin") is false for those twelve keys. Principle 6 (the cheapest passing artifact) and the crate's asymptotic contract: a sub-quadratic superlinear mechanism (`O(n sqrt n)`, a bad block size) is a regression class this instrument exists to catch, and today `d^1.5` on `ff_party_join` passes both legs.

Evidence:

        68	/// a deep `DenseSpine` draw). The allowance sits well above that observed
        69	/// ceiling and a third of the +1.0 a quadratic mechanism adds over a
        70	/// linear pin, so the gap it lives in is wide on both sides.
        71	pub const SLOPE_ALLOWANCE: f64 = 0.3;

       113	    Some((top_y - bot_y) / (top_x - bot_x) - band.slope)

       101	//! the pooled envelope slopes above 1.1 remain the documented
       102	//! family-mixture composition (families' per-bit cost levels differ
       103	//! severalfold and the cheap families' mass sits in the small buckets,
       104	//! tilting a pooled envelope no single family follows; every lane's own
       105	//! medians are flat or falling, and `bin/diag` per family is the

       751	        slope: 1.175460,

Resolution: Judge the within-case slope against the claimed law rather than the pooled fit: for non-fold keys `excess = local - band.slope.min(LINEAR_CLAIM)` with `LINEAR_CLAIM = 1.0`, or a per-key declared exponent where a documented superlinear within-case mechanism exists (the note names `ff_rank_display`'s digits-times-limbs conversion as a candidate). Re-derive `SLOPE_ALLOWANCE` from `bin/calibrate`'s shape-leg evidence under the new reference (the code path at calibrate.rs:221-229 exists; only the reference changes), and rewrite curve.rs:64-70 to name the reference actually used. State the per-row escaping-exponent bound in the blessed-drift section so the instrument's reach is documented where its criterion lives. Acceptance: a committed `curve/tests.rs` case builds `fuel = L(128) · (d / 128)^1.4` over 128..8768 against `band_for("ff_version_decode", false)` and asserts `local_slope_excess > SLOPE_ALLOWANCE` (today it reads +0.225 and passes); calibrate's recomputed maximum healthy excess stays under the allowance on the 4096-program corpus; the suite stays green at the current pin.
Construction: `ff_version_decode`'s line at 128 bits is `1.706616 + 1.175460 · log10(128) = 4.184`. A mechanism costing `10^4.184 · (d / 128)^1.4` reaches log fuel 6.754 at `max_denom` 8768, below the ceiling `1.706616 + 1.175460 · log10(8768) + 0.330785 + 0.2 = 6.872`, so every point reads `InBand`; its within-case excess is `1.4 - 1.175460 = 0.225 < 0.3`, so the shape leg passes. At exponent 1.45 both still pass (6.845 < 6.872; 0.275 < 0.3). For `ff_party_join` the shape threshold is `1.251540 + 0.3 = 1.552`.

### fuzzfit-bands-17: The floors' liveness claim is calibrate stderr, evaluated at one endpoint over the main bands only; no committed check asserts floor > nop
- Where: crates/before/fuzzfit/harness/src/bin/calibrate.rs:273-297 (related: crates/before/fuzzfit/harness/src/bands.rs:175-194, bands.rs:688-699, bands.rs:700-711, crates/before/fuzzfit/harness/tests/enforce.rs:242-258, crates/before/fuzzfit/harness/tests/sanity.rs:185-217, crates/before/fuzzfit/harness/src/fit.rs:242-250)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read all three test files: the only `ff_nop` use is `fuel_metering_is_live`, and no test compares any pinned floor to the nop reading; Python over the pinned constants with nop = 2 gives floor gaps at both endpoints of every band and small band: narrowest +0.1155 `ff_rank_cmp` at 128 bits, then +0.7015 `ff_clock_sync` [err], +0.8343 `ff_version_meet`; `ff_rank_checked_sub` [err] has slope −0.018256 with floors 1.460 at 128 bits and 1.432 at 4216 bits, the lower one at `max_denom`); executed: no
- Seen by: adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-and-holds for the stderr mechanism (b528f2c6; it worked once by human reading when 875c118b moved the margin 1.0 to 0.8), with the committed-test ask exceeding the recorded decision, not contradicting it; the "non-negative slope" caveat was written beside a negative-slope arm; the small-band omission has no rationale
- Owner-gated: no
- Witness (witness/results.md): inconclusive. Verified by reading: the only floor-versus-nop evaluation is calibrate's stderr loop over `fits` at `min_denom` (calibrate.rs:273-297), which walks neither `small_fits` nor `SMALL_BANDS`; `ff_rank_checked_sub` [err] has a negative slope (bands.rs:691). The `width_below + 0.12` construction was not run.

The only place the instrument evaluates whether every pinned floor sits above the dead-meter reading is this loop, which prints the narrowest gap for a human at re-pin time. The `ENFORCE_MARGIN_BELOW` doc claims "every pinned floor still clears that reading with this slack subtracted" over every band key, and records that at 1.0 the `ff_rank_cmp` floor already dipped under nop, so the margin is tuned to the edge of validity; the last re-pin demonstrably did not re-read the evidence (finding 2). The loop also takes the floor at `min_denom` only, under a premise ("for a non-negative slope") the pin falsifies, and walks `fits` only, never `small_fits`, though `judge_small` applies the same margin to the `SMALL_BANDS` floors. Principle 6 (every hole becomes a committed check, never a convention held in memory) and Principle 2 (a ceiling passes vacuously when the floor excludes nothing). At this pin the `ff_rank_cmp` floor is about 2.6 fuel at 128 bits against a nop of 2, so it distinguishes only a return-immediately kernel from work; a stubbed pair kernel that still borrows its registers costs more than nop.

Evidence:

       273	    // The liveness margin's evidence: the narrowest gap, over every band
       274	    // key, between the effective floor (line − width_below −
       275	    // ENFORCE_MARGIN_BELOW, at min_denom — the floor's lowest judged
       276	    // point for a non-negative slope) and the nop-level reading a dead
       277	    // meter produces. The liveness claim rests on every gap staying
       278	    // positive.
       279	    let nop = Guest::new().call("ff_nop", &[]).fuel;
       280	    let nop_log = (nop.max(1) as f64).log10();
       281	    let mut floor_min: Option<(f64, Key)> = None;
       282	    for (&key, f) in &fits {
       283	        let floor = f.intercept + f.slope * (f.min_denom as f64).log10()
       284	            - f.width_below
       285	            - ENFORCE_MARGIN_BELOW;

       691	        slope: -0.018256,

Resolution: Add an enforcement test that calls `ff_nop` on a live `Guest` and, for every band in `BANDS` and `SMALL_BANDS`, asserts `intercept + slope · log10(d) - width_below - ENFORCE_MARGIN_BELOW > log10(nop)` at both `d = min_denom` and `d = max_denom` (the endpoint argument `line_divergence` already uses). Make calibrate's loop evaluate both endpoints over `fits` and `small_fits` and fail rather than print when any gap is non-positive; drop the caveat once the code no longer needs it. Consider measuring a dispatch-only control kernel (borrow two registers, return) once, so the `ff_rank_cmp` floor's liveness claim is stated against the dead reading a stubbed pair kernel actually produces (see open question 6). This composes with finding 2's `PIN_EVIDENCE.floor_min_gap`. Acceptance: a committed synthetic case (the pinned `ff_rank_cmp` band with `width_below` widened by 0.12) reads `InBand` for `fuel = nop` at 128 bits and the new check rejects such a band by name; calibrate exits non-zero on a non-positive gap; the printed minimum covers 53 floors at both endpoints.
Construction: Copy the pinned `ff_rank_cmp` `Band` (bands.rs:700-711) into a test, add 0.12 to `width_below`, and call `judge_against(&band, 128, 2)`: the result is `InBand`. Nothing in `tests/enforce.rs` or `tests/sanity.rs` would fail if calibrate emitted such a band; the judgment tripwire at sanity.rs:186-217 and the burner test at enforce.rs:273-310 use synthetic bands and never touch the pinned floors.

### fuzzfit-bands-19: Fuel determinism, the instrument's stated foundation, has no committed test; the only check is a probe binary no recipe runs
- Where: crates/before/fuzzfit/harness/src/bin/probe.rs:1-8 (related: probe.rs:62-64, crates/before/fuzzfit/harness/src/wasm.rs:4-11, wasm.rs:117-135, crates/before/fuzzfit/harness/src/lib.rs:40-42, crates/before/fuzzfit/harness/src/drive.rs:19-31, justfile:573-607)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read every test file in the harness: no test runs a program twice or compares two `Measured.fuel` values from separate guests; `grep -n probe justfile` matches only `muxprobe` and "probe cursors"; `git log -- wasm.rs` shows the pooled allocator (78e24230) and the warm-slot cap (9b051e19) landing 2026-08-06, after the last re-pin e7a4b7b0 2026-08-04; `Sample` derives only `Debug, Clone, Copy`); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: deliberate-but-expired (the probe was the bring-up feasibility check, its purpose discharged when the harness was built; the design note's "re-run at each pin" convention is unenforced and no re-pin commit records a probe run)
- Owner-gated: no for the test; deleting `probe.rs` afterwards is the owner's call (removal of an instrument)
- Witness (witness/results.md): inconclusive as a runtime demonstration; the grep construction was performed and agrees: the only comparison of two independent fuel readings is bin/probe.rs:64, the two test files contain no fuel-equality assertion, and the fuzzfit recipes run `cargo nextest run`, never the probe binary.

The harness states everywhere that fuel is a pure function of (guest bytes, call sequence, payloads) and that fresh pooled-slot guests start from identical state, and builds replay, shrinking, the committed seeds, and the staleness cross-check on that premise. The only check that two executions of one program agree is `probe.rs:64`, run by hand. Two of the probe's three checks are performed by every enforcement run (instantiation by every `Guest::new`; the host-to-guest-to-host differential by `drive::run_program`), leaving fuel determinism as its one live function, which the pooled allocator's slot-reset semantics (environmental state a harness cannot reason about from code) now carry without any committed exercise. Principles 2 and 6: a premise everything rests on needs a committed instrument; nondeterminism here would surface only as unexplained seed-replay flakiness inside the margins, never as a named failure.

Evidence:

         4	//! Verifies, before anything is built on top: (1) the guest builds and
         5	//! instantiates; (2) the same input yields byte-identical fuel across two
         6	//! fresh instances in-process (and across process invocations — compare two
         7	//! runs' stdout); (3) a random input round-trips host → guest → host with
         8	//! the guest's result byte-equal to the native mirror's.

        64	    assert_eq!(a, b, "fuel is not deterministic across fresh instances");

Resolution: Add `fuel_is_deterministic_across_fresh_guests` to `tests/enforce.rs`: run one fixed program (e.g. `build(&Family::Escalation { depth }, seed)` from `ESCALATION_REPLAYS[0]`, or the first bootstrap program) through `run_program` twice, each with its own fresh `Guest`, and `assert_eq!` the two `Vec<Sample>` including fuel (derive `PartialEq, Eq` on `Sample`). Then retire `probe.rs`, whose remaining unique function the test owns, or reword its doc so it no longer claims to be the determinism proof. Acceptance: the committed test fails if two fresh-guest replays differ in any sample's fuel; demonstrated red under a deliberate break (skip `ff_regs_reserve` on the second instance so a reallocation lands inside a measured call) before the probe is removed; `just fuzzfit` green after.
Construction: Grep the tests for a second `run_program` on the same program or any comparison of two `Measured.fuel` values from separate guests: none exists. A failure the test would catch: a `static mut` counter read inside a guest kernel, or a pooled slot that is not reset to the initial image; the suite today stays green while replays diverge.

### fuzzfit-bands-4: The deterministic prefix leg pays the out-of-corpus ceiling margin it does not need
- Where: crates/before/fuzzfit/harness/src/bands.rs:161-173 (related: bands.rs:42-70, crates/before/fuzzfit/harness/tests/enforce.rs:53-56, tests/enforce.rs:355-366, tests/enforce.rs:221-240, crates/before/fuzzfit/harness/src/bin/calibrate.rs:80-131)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read); executed: no
- Seen by: adequacy; refutation: reframed (the bootstrap leg's main-band judgments are out-of-corpus and legitimately need the margin; the prefix leg and the small-band portion of the bootstrap leg do not); history: already-known (d17a8018 documents the window as an accepted property; a separate deterministic-leg margin was not considered)
- Owner-gated: yes: it narrows the blessed-drift window and therefore raises re-pin frequency, a cadence trade the owner rules on

`ENFORCE_MARGIN` is justified as absorbing variance between calibration and enforcement contexts, yet `judge` applies it identically to the deterministic prefix, whose programs are the fitted corpus itself (residual at most `width_above` by construction, modulo six-decimal rounding of the printed constants). The documented x1.6 window on most keys is therefore set by a margin whose rationale does not apply on that leg. Principle 6: name the worst passing artifact and close the path.

Evidence:

       161	/// Slack beyond each band's fitted ceiling, in `log₁₀` units.
       162	///
       163	/// Absorbs allocator-history variance between calibration and
       164	/// enforcement contexts (≈ 1.6× in fuel). Measured at pin time
       165	/// (`bin/calibrate` replays the enforcement suite's fixed escalation
       166	/// programs — deterministic enforcement-context executions outside the
       167	/// calibration corpus — and re-derives their worst ceiling excess on
       168	/// every re-pin): the observed maximum is +0.024 decades

Resolution: Thread the margins through `judge` (or add `judge_against_with(band, d, fuel, margin_above, margin_below)`), pass `ENFORCE_MARGIN` for the sentry, the escalation replays, and the bootstrap leg's main-band judgments, and a documented `DETERMINISTIC_MARGIN` (sized for the printed constants' rounding, or a deliberate x1.1) for the prefix leg and the small-band judgments; state the two windows separately in the blessed-drift section. Acceptance: the blessed-drift doc states two windows; a committed synthetic case shows a uniform x1.5 drift on one key reads `Above` on the prefix while still inside the sentry's margin; the gate stays green at the current pin.
Construction: Multiply every `ff_clock_tick` fuel by 1.5 (a uniform +0.176 decade drift). Prefix leg: the argmax sample's residual becomes `width_above + 0.176 < width_above + 0.2`, `InBand`; refit divergence 0.176 < 0.7; the shape leg is deaf to intercept shifts. The suite stays green, exactly as bands.rs:61 documents. With a 0.05 deterministic margin the same drift reads `Above` on that sample every run.

### fuzzfit-bands-6: The wasmtime-bump-is-a-re-pin sentence is a convention with no mechanism, contradicted at both patch bumps
- Where: crates/before/fuzzfit/harness/src/bands.rs:312-321 (related: crates/before/fuzzfit/harness/tests/enforce.rs:319-326, crates/before/fuzzfit/harness/build.rs:1-15, crates/before/fuzzfit/Cargo.lock:1193-1194, crates/before/fuzzfit/harness/src/bin/calibrate.rs:360-369)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`git show --stat 4e64a4fb -- crates/before/fuzzfit` lists only Cargo.lock; `git show --stat 8490af3f` lists only the two detached Cargo.lock files; `git log -- bands.rs` ends at e7a4b7b0 2026-08-04; Cargo.lock at HEAD names wasmtime 47.0.4; enforce.rs asserts only `PINNED_RUSTC`); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: deliberate-and-holds for the rustc-constant/lockfile asymmetry (e7a4b7b0's rationale); the "likewise a re-pin event" sentence (c1fe9388) has no mechanism and was skipped at 8490af3f and 4e64a4fb, both with green gates
- Owner-gated: no
- Cross-references: fuzzfit-strategies-5 (the corpus half of the same provenance gap).

`PINNED_RUSTC` is enforced by `building_toolchain_matches_the_pin`; the doc's parallel claim about wasmtime is enforced by nothing: no constant records the pinning wasmtime and no test compares it. Two lockfile bumps (47.0.2 to 47.0.3, 47.0.3 to 47.0.4) landed without a re-pin, and the staleness leg would notice a fuel-schedule change only past `REFIT_TOLERANCE` (about 5x). Principle 6: every hole found becomes a committed check, never a convention held in memory; or the sentence states the softer guarantee that actually holds.

Evidence:

       315	/// Generated by `just fuzzfit-calibrate` alongside [`BANDS`]: guest
       316	/// codegen (and so every fuel constant) is a function of this compiler,
       317	/// and the suite asserts the building toolchain matches, so a toolchain
       318	/// bump reads red until the bands are re-pinned. wasmtime (the fuel
       319	/// schedule's other half) is pinned exactly by the workspace
       320	/// `Cargo.lock`; bumping it there is likewise a re-pin event.
       321	pub const PINNED_RUSTC: &str = "rustc 1.97.1 (8bab26f4f 2026-07-14)";

Resolution: Either enforce it (build.rs parses the workspace Cargo.lock's `wasmtime` version into `FUZZFIT_WASMTIME_VERSION`; calibrate emits `PINNED_WASMTIME`; a sibling of `building_toolchain_matches_the_pin` asserts equality) or soften the sentence to what holds ("a wasmtime bump that moves the fuel schedule reads red through the staleness leg past `REFIT_TOLERANCE`; a bump that leaves the schedule alone stays green and is not a re-pin event"). Acceptance: a lockfile wasmtime bump without re-pin turns `just fuzzfit` red by name, or the sentence names the mechanism (`REFIT_TOLERANCE`) that actually bounds it.
Construction: The two historical bumps are the demonstration: `git show --stat 8490af3f` and `git show --stat 4e64a4fb` touch only lockfiles, `bands.rs` is unchanged since e7a4b7b0, and 4e64a4fb's message records a clean gate; `grep -n wasmtime tests/enforce.rs` returns nothing.

### fuzzfit-bands-9: `SHAPE_EXEMPT`'s justification (the point leg catches a degenerate fold) is argued inline, never pinned
- Where: crates/before/fuzzfit/harness/src/curve.rs:38-49 (related: crates/before/fuzzfit/harness/src/bands.rs:820-831, bands.rs:856-867, crates/before/fuzzfit/harness/src/strategies.rs:89-94, strategies.rs:1096-1108)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read); executed: no
- Seen by: adequacy; refutation: confirmed; history: deliberate-and-holds (86350975 introduced the exemption with a measured left-fold demonstration: 10.6x, +1.03 decades against a +0.35 ceiling at the ladder top, after a first attempt at `max_fold` 64 read green; the shrunk shape is a committed seed)
- Owner-gated: no

The fold kernels are carved out of the shape leg on the claim that the point leg catches a left fold's `n / log n` excess "far past the pinned ceiling inside the reachable width range". Nothing committed binds that claim to the pinned `join_all` and `meet_all` ceilings (0.355 + 0.2 and 0.293 + 0.2 decades) or to `BUDGET.max_fold` (1024), and nothing asserts the deterministic prefix contains folds wide enough for detection. The rationale is stated at the site, which is right; the premise it rests on is not mechanically held, so a re-pin widening the fold ceilings or a budget cut would silently turn the exemption into an accepted failure. Principle 3: a carve-out names what still catches the failure, mechanically.

Evidence:

        45	/// regression. The point leg owns these rows instead: the generators'
        46	/// width ladder puts a degenerate (left-fold) reduction's excess, which
        47	/// grows as n / log n, far past the pinned ceiling inside the reachable
        48	/// width range.
        49	pub const SHAPE_EXEMPT: &[&str] = &["ff_version_join_all", "ff_version_meet_all"];

Resolution: Either (a) an arithmetic tripwire in `curve/tests.rs` binding the argument to the constants: assert `log10(max_fold / (2 · log2 max_fold)) > width_above + ENFORCE_MARGIN` for both fold bands, and assert in the prefix leg that at least one `join_all`/`meet_all` step above the detection width is judged; or (b) a guest control kernel that left-folds the same registers, judged against the pinned `join_all` band and required to read `Above` at the budget width. Acceptance: a committed test fails if the fold ceilings widen or `max_fold` shrinks past the point where a left fold reads `InBand`, or if the prefix stops exercising wide folds.
Construction: With a balanced-versus-left model of `n / (2 · log2 n)`: at n = 32 the excess is about 3.2x = 0.51 decades < 0.555 (`join_all` ceiling plus margin), in band; at n = 64, 5.3x = 0.73 decades, `Above`. Whether the 256-program prefix contains a `join_all` at n >= 64 is not determinable from code (`ScatterFold` draws `clocks` in 8..=1024 and ladders at doubling widths below it).

### fuzzfit-bands-8: `fit` and `fit_constant` have no direct tests; the fitter's headline claims are untested families
- Where: crates/before/fuzzfit/harness/src/fit/tests.rs:26-35 (related: crates/before/fuzzfit/harness/src/fit.rs:25-35, fit.rs:102-179, fit.rs:194-210)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read all three test files; only `line_divergence` is exercised); executed: no
- Seen by: structure-prose; refutation: confirmed (a raw-OLS fitter might trip the staleness leg on some wide key, but not by name); history: no rationale found (c1fe9388 scoped fit/tests.rs to the comparator and never widened it)
- Owner-gated: no

`fit/tests.rs` exercises only `line_divergence`; its `linear_fit()` helper fits a clean `fuel = 100 · d` corpus and never asserts the fitter recovered slope 1, intercept 2, zero widths. The module doc's claims (bucket medians keep bounded spikes out of the slope; the floor drop rule; constant classification under a decade or three buckets; `fit_constant`'s `# Panics`) are each a family with no committed demonstration. Property tests over point tests where the claim is a family; every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

        26	/// A clean linear corpus to fit: fuel = 100 · d over 128..131072 bits.
        27	fn linear_fit() -> Fit {
        28	    let samples: Vec<(u64, u64)> = (7..=17)
        29	        .flat_map(|k| {
        30	            let d = 1u64 << k;
        31	            [(d, d * 100), (d + d / 2, (d + d / 2) * 100)]
        32	        })
        33	        .collect();
        34	    fit(&samples).expect("enough samples to fit")
        35	}

Resolution: Add proptests to `fit/tests.rs`: (a) a noise-free `fuel = 10^b · d^a` corpus over at least two decades recovers (a, b) within 1e-9 with both widths near 0; (b) the same corpus plus k spikes of x10 on random samples leaves the slope within 1e-6 of a while `width_above` is about 1 (raw OLS moves the slope); (c) samples spanning under a decade, or fewer than three buckets, classify constant with slope 0 and intercept the mean; (d) sub-floor samples are dropped when at least two floored remain and kept otherwise; (e) `fit_constant` returns the mean level and panics on a floored sample (`#[should_panic]`). Acceptance: the five properties committed and green; replacing the bucket medians at fit.rs:138-147 with raw OLS fails (b) by name.
Construction: Replace fit.rs:138-147 with a plain OLS over `logs` and run the harness unit tests: nothing in `fit/tests.rs` or `curve/tests.rs` fails.

### fuzzfit-bands-12: The noisy-linear tripwire's jitter is bucket-correlated, so its doc overstates what it proves
- Where: crates/before/fuzzfit/harness/src/curve/tests.rs:63-64 (related: curve/tests.rs:27-39, curve/tests.rs:58-70)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (Python transcription of `sampled` and `local_slope_excess` reproducing the test's inputs exactly: per-bucket fraction of samples at 2x is 0.0, 0.5, 0.5, 1.0, 1.0, 0.5 for buckets 4..9 and the excess is +0.0817; the quadratic arm of the same transcription reads 1.000, matching the sibling test); executed: yes: the Python transcription (scratchpad `arith.py`), whose output settles both numbers
- Seen by: adequacy, instrument-correctness; refutation: confirmed (the instrument-correctness lens's "~+0.12" was imprecise; +0.082 is right); history: no rationale found
- Owner-gated: no

The comment claims deterministic 1x..2x jitter "uncorrelated with size", but `sampled()` strides by `d / 16`, which is even at d = 128 (every `dd` even, so every sample in the bottom bucket is at 1x) and odd or mixed in later buckets, so two upper buckets are entirely at 2x. The measured excess is +0.082 (27% of the allowance) from a size-correlated step, not spread. Every test's doc comment must state its invariant accurately; "the leg flags trends, not spread" is the property and the inputs prove a weaker one while consuming allowance headroom.

Evidence:

        63	    // Deterministic 1x..2x jitter, uncorrelated with size.
        64	    let excess = local_slope_excess(&band, &sampled(|d| d * 100 * (1 + d % 2)))

Resolution: Jitter by within-bucket index (expose `i` from `sampled`, or hash `d` so parity is balanced within every bucket), then tighten the assertion to a bound that demonstrates median insulation (e.g. `excess.abs() < 0.05`); or reword the comment to say the jitter is a size-correlated step and the assertion is against the allowance. Acceptance: every bucket holds half its samples at 2x, and the asserted bound is well under `SLOPE_ALLOWANCE`.

### fuzzfit-bands-30: The kernel roster in `sanity.rs` is a hand-maintained one-per-variant list nothing ties to `Op`
- Where: crates/before/fuzzfit/harness/tests/sanity.rs:94-99 (related: tests/sanity.rs:99-160, crates/before/fuzzfit/harness/src/ops.rs:56-150, ops.rs:152-201)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (counted 44 `Op` variants at ops.rs:56-150 and 44 roster entries at sanity.rs:99-160; `Op::kernel` at ops.rs:154-201 is an exhaustive match the compiler polices; the roster is not); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-and-holds (61398dc9 chose a committed expectation list so a kernel added or orphaned fails by name in a reviewable diff; that rationale is stated inline and stands; the totality gap stands and is unaddressed by it)
- Owner-gated: no
- Cross-references: fuzzfit-strategies-8 (the same roster seen from the strategies partition).

The parity test derives the kernel set from a 44-entry hand list whose completeness over `Op` rests on the sentence asking the author to extend it. A variant added with a kernel and no band, and not emitted by any strategy, passes this test and is priced and flagged by nothing until a generator happens to emit it. Principle 6 (the cheapest passing artifact, adding a variant and skipping the roster, is not the intended one) and Principle 5 (no hand-maintained enumerations of facts the code can change without touching the prose). The expectation-list design can stay; what closes the gap is making its totality a compile error.

Evidence:

        94	/// sample the hole. The roster is one representative op per `Op`
        95	/// variant: a variant added to the vocabulary belongs in this list, and
        96	/// its kernel in the pinned bands.
        97	#[test]
        98	fn bands_and_op_roster_name_the_same_kernels() {
        99	    let roster: Vec<Op> = vec![

Resolution: Make totality a compile error without a new dependency: an exhaustive `match op { Op::ClockSeed { .. } => (), ... }` over the roster's variants in the test (or a `fn representative(op: &Op) -> Op` the test walks), so a new variant fails to compile until rostered; or colocate the roster beside `Op::kernel` in `ops.rs` as `pub const REPRESENTATIVES` so both edits land in one diff. Acceptance: adding an `Op` variant without a roster entry fails to compile or fails a test by name; the convention sentence is gone.
Construction: Add `Op::VersionNoop { src: Reg }` with `kernel() => "ff_version_noop"`, omit it from the roster and from every strategy: `bands_and_op_roster_name_the_same_kernels` stays green and no test names the unpriced kernel.

**Fuelscape pipeline**

### fuelscape-pipeline-9: split_budget is pinned for reachability where the module doc claims exact uniformity, and the suite's own argument says reachability cannot pin uniformity
- Where: crates/before-fuelscape/src/plan/tests.rs:263-276 (related: crates/before-fuelscape/src/plan.rs:180-206, 11-22; crates/before-fuelscape/src/plan/tests.rs:167-178, 127-143; crates/before-fuelscape/src/ops.rs:49-59)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read; the implementation, distinct cuts via `BTreeSet` rejection, is a uniform k-subset draw and is correct, so this is a missing pin, not a bug); executed: no
- Seen by: adequacy [16], instrument-correctness [46]; refutation: confirmed; history: the reachability test came with the k-way split (eb6b35f42); the audit that added the arity chi-square (04cbd1ab, adopted 8409fe55) discussed the arity draw only
- Owner-gated: no

plan.rs:180-185 claims the cut-point rejection "keeps the composition draw exactly uniform" and every multi-operand row's stamped size measure rests on it; the only pins are positivity/sum (127-143) and that the ten compositions of (6, 3) appear. The sibling test at 167-178 states the principle: a biased-but-total draw reaches every count and passes a reachability check. Principle 6: the worst artifact that passes here is a biased split, which skews the x-axis of every binary, ternary, and slice panel while the caption still says "split uniform".

Evidence:

       263	#[test]
       264	fn split_budget_reaches_every_composition() {
       265	    let mut rng = cell_rng(0xc0de, "split-reach", 6, 3);
       266	    let mut seen = std::collections::BTreeSet::new();
       267	    for _ in 0..2_000 {
       268	        seen.insert(split_budget(6, 3, &mut rng));
       269	    }
       270	    // C(5, 2) = 10 compositions of 6 into 3 positive parts.
       271	    assert_eq!(
       272	        seen.len(),
       273	        10,

Resolution: Replace the reachability test with a one-sided chi-square over the ten compositions of (6, 3) at 2000 draws (expected 200 each; threshold `chi_square_threshold(10)` = 9 + 6 * sqrt(18), about 34.5, the sampler pins' idiom), which subsumes reachability (an unreached composition alone contributes 200). Acceptance: green on the current `split_budget`, red on either known-bad split below, at the committed seed.
Construction: Known-bad A (stick-breaking): first part `gen_range(1..=total - parts + 1)`, recurse on the remainder; for (6, 3), P((4,1,1)) = 1/4 and P((1,1,4)) = 1/16 against the uniform 1/10; every composition is reachable, sum and positivity hold, and the chi-square over 2000 draws reads in the hundreds. Known-bad B (collision nudge): draw `parts - 1` cuts in `1..total` with replacement, sort, and move each duplicate to the next unused position; for (6, 3) the compositions with adjacent cuts carry 3/25 each and the rest 2/25, all reachable, statistic about 84.

### fuelscape-pipeline-5: The determinism test says its op list walks every input space; VersionSliceCapped has no replay pin
- Where: crates/before-fuelscape/src/plan/tests.rs:14-37 (related: crates/before-fuelscape/src/ops.rs:79, 393-406; crates/before-fuelscape/src/plan.rs:293-311)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (mapped the seven names to their `Inputs` variants in ops.rs: Packed (version_rank, party_covers, clock_sync), PackedDistinct (party_without), VersionSlice (version_join_all), ClockSlice (clock_join_all), PartyShares (party_join_all); `VersionSliceCapped` is used only by `shape_combine` at ops.rs:395 and is absent); executed: no
- Seen by: adequacy [17], structure-prose [28], instrument-correctness [48]; refutation: confirmed; history: deliberate-but-expired: the claim was true when written and the commit that added the seventh variant (46eb64f9) touched no test file
- Owner-gated: no

Every test's doc comment must state its invariant accurately; this one over-claims by one input space, and the list is a hand-maintained enumeration the `Inputs` enum can outgrow without touching the test. The capped row is also the only one whose measure dispatches through a per-arity table in the guest, which the replay would exercise for free.

Evidence:

        14	/// re-derives a cell's RNG from execution order fails it. The op list
        15	/// walks every input space: unary and binary packed draws (both
        16	/// samplers, the split rule, the version rejection path), the slice
        17	/// arity-and-composition draw, the distinct-pair rejection, the
        18	/// three-way split with in-guest fork preparation, and both
        19	/// variable-arity fold draws (the party-plus-version-slice clock fold
        20	/// and the guest-split party shares).
        ...
        29	    for name in [
        30	        "version_rank",
        31	        "party_covers",
        32	        "version_join_all",
        33	        "party_without",
        34	        "clock_sync",
        35	        "clock_join_all",
        36	        "party_join_all",
        37	    ] {

Resolution: Add `"shape_combine"` to the list and name the capped draw in the doc; better, derive the list from `ROSTER` by picking the first row per `Inputs` variant through a `match` with no wildcard arm, so a new variant is a compile error and the doc can say "one row per `Inputs` variant". Acceptance: every `Inputs` variant has a row in the replay; adding a variant without one fails to compile (derived) or the doc no longer says "every".

### fuelscape-pipeline-7: The chi-square statistic is written three times; plan/tests.rs hardcodes the threshold sample/tests.rs derives
- Where: crates/before-fuelscape/src/plan/tests.rs:188-197 (related: crates/before-fuelscape/src/sample/tests.rs:96-106, 140-152, 184-196)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read the three loops; `chi_square_threshold(9)` = 8 + 6 * sqrt(16) = 32.0, the literal at line 197); executed: no
- Seen by: structure-prose [29]; refutation: confirmed; history: the arity pin's commit names the sampler pins' 6-sigma idiom it copies and still re-derives the bound by hand
- Owner-gated: no

One formula, one home: the 6-sigma one-sided bound is a shared criterion, and a hand-copied 32.0 silently diverges if `chi_square_threshold` is ever tightened (Principle 5, hand-maintained numbers). The two sampler pins duplicate about forty lines of the same tally-and-assert.

Evidence:

       188	    let expected = DRAWS as f64 / TOTAL as f64;
       189	    let chi2: f64 = observed
       190	        .iter()
       191	        .map(|&o| {
       192	            let d = o as f64 - expected;
       193	            d * d / expected
       194	        })
       195	        .sum();
       196	    // 8 degrees of freedom: mean 8, variance 16, so 8 + 6·4 = 32.
       197	    let threshold = 32.0;

Resolution: A `#[cfg(test)]` helper module (`src/testing.rs`) with `chi_square(observed: &[u64], expected: f64) -> f64`, `chi_square_threshold(categories) -> f64`, and `assert_uniform(observed, label)`; the two sampler pins, the arity pin, and the split pin proposed in fuelscape-pipeline-9 call it. Acceptance: the literal `32.0` is gone from plan/tests.rs and one chi-square implementation exists in the crate.

### fuelscape-pipeline-8: fold_rows_expose_the_arity_axis_in_fuel asserts a sign with no margin, on a tuple sort that biases toward passing
- Where: crates/before-fuelscape/src/plan/tests.rs:236-252 (related: crates/before-fuelscape/src/plan/tests.rs:204-216; crates/before-fuelscape/src/plan.rs:388-408)
- Class / severity / confidence: test-quality / low / medium
- Provenance: assessed (read; the test needs the guest wasm, which no permitted command builds here, so the pass probability under the null is reasoned, not measured); executed: no
- Seen by: adequacy [18], instrument-correctness [47]; refutation: confirmed (could not run the mutant either); history: deliberate-and-holds: the criterion and its purpose were stated in 98c63b26; the commit asserts without a demonstration that a fixed-arity kernel cannot produce the reading, so the null-hypothesis analysis is new
- Owner-gated: no

`column.sort_unstable()` sorts `(arity, fuel)` tuples, so within the median arity group the lower-fuel samples land in the "low" half and the higher-fuel ones in the "high" half; under a null of fuel independent of arity the two means still differ in the passing direction by that group's spread, and `high > low` accepts any positive difference. A known-bad measure (fuel independent of the recorded arity) therefore passes at a seed-determined fraction of seeds well above one half, and at the committed seed it passes or fails forever. Doctrine: a criterion a bad implementation can pass by luck is decoration.

Evidence:

       236	        column.sort_unstable();
       ...
       243	        let half = column.len() / 2;
       244	        let mean = |cells: &[(usize, u64)]| {
       245	            cells.iter().map(|&(_, fuel)| fuel).sum::<u64>() as f64 / cells.len() as f64
       246	        };
       247	        let (low, high) = (mean(&column[..half]), mean(&column[half..]));
       248	        assert!(
       249	            high > low,

Resolution: Sort by arity alone with a stable sort (`sort_by_key(|&(arity, _)| arity)`) and require a relationship with margin that costs no fuel threshold: a Spearman rank correlation between arity and fuel across the column at or above a stated bound, or the mean of the top-arity third exceeding the bottom-arity third by a factor the fold's `log k` model predicts. Commit the known-bad demonstration beside it. Acceptance: passing a constant arity to `op.measure` at plan.rs:394 while leaving `CellSample.arity` as drawn fails the test deterministically at the committed seed for both fold rows.
Construction: At plan.rs:394 replace `(op.measure)(&mut guest, &inputs, arity)` with `(op.measure)(&mut guest, &inputs, 4)` for the `PartyShares` row (for `clock_join_all`, whose measure ignores the argument, fix the drawn clock count instead). Fuel is then independent of the recorded arity; sorting by `(arity, fuel)` and splitting at the median yields two means whose order depends only on which party bytes landed in which half at seed 0x5eed.

**Fuelscape render and datasets**

### fuelscape-render-7: The cross-platform grid pin's stated sensitivity mechanism is not how it is sensitive, and no known-bad demonstration exists
- Where: crates/before-fuelscape/src/render/tests.rs:131-138 (related: crates/before-fuelscape/src/render/tests.rs:141-173, :182-194; crates/before-fuelscape/src/render.rs:223-224, :240-241, :257-258, :317-319; crates/before-fuelscape/src/dump.rs:237-243; justfile:660-663, :700-703)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (replicated the test's SplitMix64 stream in python: 5,500 samples, `fuel_lo` is 1 with 153 samples reading 1, `fuel_hi` is 140523145363644, not a power of two, range 48.10 octaves, bin height 0.859); executed: yes (the python replication settles the fixture's extremes; the missing demonstration is by reading)
- Seen by: instrument-correctness [61]; refutation: reframed (severity low to medium: the hash's platform sensitivity rides on one `log2` evaluation); history: no rationale found; commit 84a97a39 records the observed mechanism ("the ulp never moved a drawn pixel; only the committed f64 grid bits carried the platform") and names the illumos gate run as the second-architecture check, neither of which the docstring states
- Owner-gated: no

Every criterion needs a committed demonstration that a known-bad mechanism fails it, and a test's doc must be accurate. The docstring says a platform `log2` divergence "anywhere in the log2 range flips at least one bin count and the hash". With `fuel_lo = 1`, `y_lo = -0.4` is exact on every libm; `x_lo`/`x_hi` are `log2` of powers of two (exact); medians are integer arithmetic; a bin count flips only when a sample's `log2` sits within an ulp of a bin edge (about 5,500 chances at roughly 2^-52 per octave, negligible). The only platform-sensitive value in the hashed serialization is `y_hi = lg(140523145363644) + 0.7`, so the pin is one coin flip per host, and no committed test substitutes `f64::log2` for `lg` to show the hash moves. The pin runs only in the local gate (`fuelscape-test`; CI runs `just ci`, which omits it), so cross-architecture agreement is exercised by the illumos gate host, as the commit says and the test does not. The stronger real-data witness is `fuelscape-verify` on ubuntu CI, which recomputes 104 grids measured on illumos and refuses any drift.

Evidence:

       131	/// A dump commits its `HeatGrid`, and the loader re-derives that grid
       132	/// bit-for-bit on whatever machine opens the dump — so the bin geometry
       133	/// must not lean on the platform math library, whose `log2` differs by
       134	/// an ulp across libms exactly at bin boundaries. The sample set below
       135	/// spreads fuel values across ~48 octaves so a platform divergence
       136	/// anywhere in the log2 range flips at least one bin count and the
       137	/// hash. The committed constant was produced by this test's own first
       138	/// run; its value carries no meaning beyond cross-host agreement.

Resolution: restate the mechanism (the serialized f64 domain edges, chiefly `y_hi`, carry the platform; bin flips do not); make the fixture carry more sensitive values (a non-power-of-two smallest fuel and smallest size, so `y_lo`, `x_lo`, `x_hi` are all live `log2` evaluations) and state that the cross-host check of record is the illumos gate run plus `fuelscape-verify` on CI; add the known-bad demonstration where feasible: a test-only shadow of `aggregate` using `f64::log2`, asserting its hash differs from the committed one on at least one documented host, or an explicit statement that no single host can witness the swap. Acceptance: the docstring's mechanism matches the arithmetic; the fixture has no exact-power-of-two extremes; either a committed test shows the `f64::log2` variant hashes differently somewhere, or the docstring says plainly that sensitivity is established by the two-architecture gate run and `fuelscape-verify`, not by this host alone.
Construction: change `lg` to `f64::log2(v.max(1) as f64)` and run `aggregate_bins_identically_on_every_platform` on the development host; if it passes there, the docstring's sensitivity claim is refuted on that host and only the illumos run and `fuelscape-verify` stand between a libm regression and the committed dump.

### fuelscape-render-19: Measurement provenance is stamped unchecked and accepted as any string
- Where: crates/before-fuelscape/src/bin/fuelscape.rs:155-159 (related: crates/before-fuelscape/src/bin/fuelscape.rs:20-24; justfile:678; crates/before/build.rs:64-67; crates/before-fuelscape/src/compact.rs:379-412; crates/before/docs/fuelscape.js:756-758; crates/before-fuelscape/src/render.rs:44-45)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read the stamp, the recipe, and both readers; a read-only scan shows today's committed documents carry two 40-hex commits, both ancestors of HEAD, so the gap is in the checks, not the data); executed: no
- Seen by: adequacy [19]; refutation: confirmed; history: the `untracked` fallback is deliberate for ad-hoc runs (bin/fuelscape.rs:20-24); no record considers a dirty-tree guard or a shape check on the committed path
- Owner-gated: no
- Witness (witness/results.md): inconclusive. Reading confirms the three quoted sites (the `"untracked"` fallback at fuelscape.rs:156, `is_string()` as the only shape check at build.rs:64-67, `git rev-parse HEAD` with no dirty-tree check at justfile:678); no run was made.

Measurements bind to their run. The commit stamp is the only thing binding a measuring run, every committed dataset document, and every rustdoc island footer to the code that was measured, and the accretion design rests on it being meaningful per operation. The recipe stamps `git rev-parse HEAD` with no dirty-tree check while the guest wasm is built from the working tree; the binary falls back to the literal `untracked`; and the only check anywhere is `build.rs`'s `is_string()`, which `untracked` satisfies, so a dataset whose documents all say `"commit": "untracked"`, or name a commit whose tree is not what was measured, passes compaction, `fuelscape-verify`, `build.rs`, and renders in the docs.

Evidence:

       155	    let meta = RenderMeta {
       156	        commit: std::env::var("FUELSCAPE_TIP").unwrap_or_else(|_| "untracked".into()),
       157	        base_seed: plan.base_seed,
       158	        samples_per_column: plan.samples_per_column,
       159	    };

    justfile:
       678	    FUELSCAPE_TIP=$(git rev-parse HEAD) FUZZFIT_GUEST_WASM=... cargo run --release --bin fuelscape -- --out ... {{ args }}

    build.rs:
        64	        assert!(
        65	            doc["meta"]["commit"].is_string(),
        66	            "{file}: the measurement commit is missing"
        67	        );

Resolution: (1) in the `fuelscape` recipe, refuse to run a `--dump` survey when `git diff --quiet HEAD -- crates/before crates/suanpan` fails (or stamp `git describe --always --dirty --abbrev=40` so a dirty tree is visible); (2) make `--dump` refuse to run without `FUELSCAPE_TIP` instead of writing `untracked` (ad-hoc renders may keep the fallback); (3) require the commit to be exactly 40 lowercase hex characters in `compact::validate` and in `build.rs`'s loop, so neither `untracked` nor `<sha>-dirty` can reach the committed dataset. Acceptance: a `--dump` run without `FUELSCAPE_TIP` exits nonzero naming the variable; a tamper line in compact/tests.rs setting `doc["meta"]["commit"] = "untracked"` is rejected naming the check; the same edit to a committed `crates/before/fuelscape/<op>.json` fails `cargo build -p before` naming the file.
Construction: `FUZZFIT_GUEST_WASM=... cargo run --bin fuelscape -- --dump --max-bytes 2 --samples 1 --out <tmp> version_tick` without `FUELSCAPE_TIP`, then `--compact-from <tmp> --out <tmp2>`: the written `version_tick.json` carries `"commit":"untracked"` and passes build.rs's checks verbatim. Or edit one committed document's `meta.commit` to `"untracked"` and run `cargo build -p before`: it succeeds.

### fuelscape-render-30: `build.rs` reads two figure files it does not declare in `rerun-if-changed`, so the README-figure freshness check is dormant under incremental builds
- Where: crates/before/build.rs:27-30 (related: crates/before/build.rs:93-94, :175-197; justfile:710-711)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read; cargo's rule that once any `rerun-if-changed` is emitted only listed paths and env vars retrigger the script is documented behavior); executed: no
- Seen by: scaffolding [6], adequacy [20], structure-prose [39], instrument-correctness [54]; refutation: confirmed, severity medium to low (every clean build, CI included, still runs the check; the blind spot is the warm local gate); history: no rationale found (the four lines are from 0a8884ee with one job; 2efff149 added the figure job without extending them; the design note §4 had planned directory-level `docs/`)
- Owner-gated: no
- Cross-references: crate-root-5 (the same defect from the crate-root partition) and deps-3 (demonstrated by build).

A freshness check whose trigger set excludes the files it guards passes vacuously in exactly the loop where the rot is introduced. The script lists `fuelscape`, the three widget files, and `BEFORE_REGEN_DOC_FIGURE`, but also reads the measurement figure (93) and the committed README copy (185), so editing either leaves `$OUT_DIR/space_consumption.svg` stale and `check_readme_figure_fresh` unrun until some listed path changes, while its doc (175-182) says the check "is what prevents" the rot.

Evidence:

        27	    println!("cargo:rerun-if-changed=fuelscape");
        28	    println!("cargo:rerun-if-changed=docs/fuelscape.css");
        29	    println!("cargo:rerun-if-changed=docs/fuelscape.js");
        30	    println!("cargo:rerun-if-changed=docs/fuelscape-header.html");
       ...
        93	    let figure = std::fs::read_to_string("results/space_consumption/itc_space_consumption.svg")

Resolution: add `println!("cargo:rerun-if-changed=results/space_consumption/itc_space_consumption.svg");` and `println!("cargo:rerun-if-changed=docs/itc_space_consumption_readme.svg");` beside the existing four (the second may sit next to the env-changed line in `check_readme_figure_fresh` for locality). Acceptance: after a warm `cargo build -p before`, append a byte to `docs/itc_space_consumption_readme.svg` and build again: the script reruns (visible with `-vv`) and fails with "is stale relative to results/space_consumption".
Construction: warm build of `before`; edit a color in `results/space_consumption/itc_space_consumption.svg` (in a scratch copy of the repo); `cargo build -p before -vv` shows `Fresh before` with no build-script run and no staleness panic; `cargo clean -p before` then fails with the stale-figure message.

### fuelscape-render-14: The tamper test's doc undercounts its cases, and several enumerated rejections in both readers have no known-bad demonstration
- Where: crates/before-fuelscape/src/compact/tests.rs:137-141 (related: crates/before-fuelscape/src/compact/tests.rs:168-183; crates/before-fuelscape/src/compact.rs:45-53, :338-343, :362-370, :387-399; crates/before-fuelscape/src/dump/tests.rs:163-200; crates/before-fuelscape/src/dump.rs:33-44, :197-236)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (counted seven `tamper(...)` calls at 168-183 against the doc's five; `grep -rn unknown crates/before-fuelscape/src/*/tests.rs` is empty); executed: no
- Seen by: scaffolding [10], adequacy [26], structure-prose [37], instrument-correctness [65]; refutation: confirmed; history: the doc's five items are the design note's five (§2:110-111); the body already had seven at introduction (c6d8106a), so the undercount is drift from birth
- Owner-gated: no

Every test's doc must be accurate and every criterion needs a committed demonstration that a known-bad artifact fails it. The doc lists five rejections; the body tampers seven. The module doc promises rejections with no tamper case anywhere: unknown fields (`deny_unknown_fields`, untested in both modules), `op_name` disagreeing with the index, `sizes.len() != cols.len()`, an empty size axis, an empty histogram, and an empty `ops` index. `dump/tests.rs` demonstrates only the grid check, leaving banner, run-parameter drift, name disagreement, empty samples, and unknown fields undemonstrated; any of those branches can be deleted or inverted with the gate green.

Evidence:

       137	/// Tamper with one committed document field and the loader refuses it,
       138	/// naming the file and check: the histogram-tightness, size-order,
       139	/// banner, meta-uniformity, and empty-claim rejections are each alive.
       140	#[test]
       141	fn read_rejects_each_tampered_document() {

Resolution: rewrite the doc as the family ("each structural rejection the module doc enumerates that a single-field edit can reach is alive") and let the `tamper` calls be the list; add the missing cases (`doc["op"]["extra"] = 1` expecting "unknown field"; `doc["op"]["op_name"] = "other"` expecting "index claims"; `doc["op"]["cols"] = []` expecting "one histogram per size column"; `doc["op"]["sizes"] = []` expecting "empty"; `doc["op"]["cols"][0]["c"] = []` expecting "empty"; through the index file, `ops: []`); lift the closure into dump/tests.rs for the dump reader's banner, run-parameter, name, empty-samples, and unknown-field checks. Acceptance: every `malformed(...)`/`reject(...)` site in `dump::read` and `compact::validate`/`read` is reached by a tamper case whose assertion names it; commenting out any one rejection branch turns a test red; neither test doc enumerates by hand.
Construction: comment out dump.rs:225-233 (the op-name check) and run the fuelscape suite: nothing fails today; the proposed `doc["op"]["op_name"] = "other"` case would.

**Nits in this section** (full records in the evidence files)

| id | where | claim | resolution |
|---|---|---|---|
| benches-examples-11 | `crates/before/benches/party.rs:34-75` | fork/join/sync/send bench routines drop the consumed operand inside criterion's timed span while tick/receive return it; scopes differ across rows (symmetric across impl and oracle). | Return surviving operands from every `iter_batched` routine, or document the timed scope once in common/mod.rs. |
| board-frame-19 | `crates/before/src/meter/board/coverage/tests.rs:9-16` | The tiling test derives the board's operation axis by building every family at a bare `0.02`, twice, instead of from `ops()`, and guards NA reasons with `reason.len() >= 20`. | Derive the axis from `ops()`; name or drop the length guard. |
| board-ops-render-25 | `crates/before/src/meter/board/tests.rs:73-73` | An assertion message at board/tests.rs:73 carries a run of ten spaces mid-sentence. | Collapse to one space. |
| envelopes-b-2 | `crates/before/tests/meter.rs:5581-5584` | `parse_wide_arming` and `answer_embedded_product` restate the generators' structural strides (33, 66) as literals. | Expose the strides on the meter surface and cite them. |
| fuelscape-pipeline-6 | `crates/before-fuelscape/src/plan/tests.rs:153-178` | `arity_draw_reaches_every_count` is subsumed by the chi-square pin, and the known-bad draw it cites (statistic 723.2) exists only in prose. | Delete the reachability test; build the biased draw in the uniformity test and assert its statistic exceeds the threshold. |
| fuzz-guests-pins-30 | `crates/before/wasm32-pins/guest/src/lib.rs:552-552` | `synth_rank_ladder`'s layout check is a `debug_assert_eq!` compiled out of the only profile built (release with overflow checks only). | `assert_eq!` with a message, or a negative return code. |
| fuzzfit-bands-29 | `crates/before/fuzzfit/harness/tests/sanity.rs:51-51` | `prop_assert!(step.denom_bits >= 1)` in `programs_are_well_formed` cannot fail: `Step` is built only with `denom_bits.max(1)`. | Delete, or assert that no live operand encodes to zero bits. |
| meter-core-12 | `crates/before/src/meter/tests.rs:27-29` | `check_version`'s doc claims "canonicality of both codings" while no validator for the construction language exists; the hoisted-window comment describes a rank agreement the body reduces to `>=`. | Restate both docs to what the bodies check. |
| meter-registry-tier2-18 | `crates/before/src/meter/tier2/tests.rs:213-220` | The tier-2 ratio-floor loop divides two closed-form literals already asserted; no measured quantity enters. | Delete the loop and the three hand-computed ratios. |
| meter-registry-tier2-20 | `crates/before/src/meter/tier2/tests.rs:261-268` | The plain-sweep witness's `>= 1.8` floor is underived and sits 0.04 above the deterministic 1.841 reading; `closing` is built inside the metered region. | State or compute the expected ratio; hoist `closing` above the reset. |
| oracle-laws-12 | `crates/before/src/oracle/tests.rs:597-600` | A `prop_assume!(!cands.is_empty())` that never rejects (the party is drawn nonempty), beside siblings that spell the premise as `.expect(..)`. | Drop the assume; use the siblings' `expect`. |
| oracle-laws-18 | `crates/before/src/laws.rs:861-877` | `merge_is_least_upper_bound` and its meet dual check only a join-built bound; the clause that means "least" (an arbitrary `c` above both implies above the join) is absent. | Conjoin the incidental implication clause, here and in oracle/tests.rs. |
| oracle-laws-20 | `crates/before/src/laws.rs:1850-1851` | `span_all_is_the_family_hull`'s containment clause admits `Concurrent` placements its own argument rules out. | Use the existing `within` helper. |
| oracle-laws-23 | `crates/before/src/laws.rs:3033-3033` | `clock_ticks_matches_version_ticks` pins a hard-coded count of 3 where the version-level twin draws `a.min_ticks()`. | Use the operand's count. |
| surface-roster-13 | `crates/before/src/testing/surface_coverage/tests.rs:181-185` | `exclusion_payload_citations_resolve`'s doc omits registered descriptor names its body admits; `render_names_the_findings` says every category and exercises four of ten. | Fix the first doc; populate all ten `Findings` fields or narrow the second doc. |
| surface-roster-14 | `crates/before/src/testing/surface_coverage/tests.rs:343-353` | `tripwires_are_labeled` asserts the shape of labels nothing consumes (`cited_test_names` and citecheck read only the test name). | Delete the test. |
| surface-roster-30 | `crates/surface-scan/src/tests.rs:14-25` | surface-scan fixtures hand-roll pid-keyed temp dirs under `temp_dir()` and never remove them. | `tempfile` as a dev-dependency; return the `TempDir`. |
| testing-oracles-8 | `crates/before/src/testing/bridge.rs:174-188` | `to_oracle_party`/`to_oracle_version` discard the end position, so a stored stream with trailing live bits lowers to its prefix silently (byte-equality legs elsewhere mask this). | Bind the position and assert it equals `bits.len()`. |
| testing-oracles-14 | `crates/before/src/testing/semantic_oracle/tests.rs:233-237` | `order_is_a_partial_order`'s transitivity arm is conditional on `le(a,b) && le(b,c)` for three independent draws; nothing constructs the chain. | Construct `b = join(a, x)`, `c = join(b, y)` and assert unconditionally. |

## Workspace tools (tools/)

The `tools/` directory (the gate's build-free checkers and their three committed expectation rosters) was reviewed as a partition after the main run; the README's method section records why, and its entries follow the same template. The thirteen below are the verification gaps in those checkers: two instruments still carry a vocabulary for accepting known failures (tools-6, tools-14), four floors are one coarse premise that cannot see partial darkness (tools-7, tools-16, tools-19, tools-28), the `ci` roster omits a gate lint (tools-4), workflowlint's recognizer passes the most common installer spelling (tools-33), and five checkers have a self-test or a floor that cannot fail (tools-11, tools-15, tools-23, tools-26, tools-30). None carries a Witness line: the partition postdates the witness pass, and the demonstrations its finalizer executed are recorded in each entry's provenance.

**The workspace verification tools and their expected-value rosters (tools)**

### tools-4: `ci` omits manifestlint, so the lint tier has two hand-maintained rosters and GitHub CI never runs it
- Where: justfile:1000 (related: justfile:403; .github/workflows/ci.yml:14-15, 107-108)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read lines 403 and 1000 side by side; ci.yml:107-108 runs only `just ci`; `git log -1 84044759` shows manifestlint landing into gate-lints on 2026-08-31); executed: no
- Seen by: scaffolding [1], adequacy [25]; refutation: confirmed (one correction: ci.yml:14-15's "exactly one definition" sentence is about the workflow re-listing justfile steps, which holds; the drift is between two lines inside the justfile); history: already-known (recorded as verification-infra-7 in the rumors review, no-rationale-found; the same omission happened once for digestshare and was patched by de9e0bdf adding the leg to `ci`, which supports the structural fix below)
- Owner-gated: no
- Cross-references: gate-legs-4 (this document) records the same omission from the gate-legs sweep; this entry adds the structural fix (`ci` depending on `gate-lints`) and the second precedent (de9e0bdf patched the same omission for digestshare).

`gate-lints` lists eight build-free legs; `ci` restates seven of them and omits `manifestlint`, so a member manifest that restates a version or names a `path`/`git` source passes GitHub CI. Two hand-maintained lists of the same legs drift by exactly this mechanism, and it has now happened twice.

Evidence:

   403	gate-lints: fmt-check doclint testdoc workflowlint manifestlint digestshare mutants-list readme-check

  1000	ci: fmt-check doclint testdoc workflowlint digestshare readme-check fuelscape-claims mutants-list clippy clippy-default features wasm-check docs docs-internal test-all citecheck doctest bench-build fuzz-build fuelscape-verify viz

Resolution: make `ci` depend on `gate-lints` instead of re-listing its legs (`ci: gate-lints fuelscape-claims clippy clippy-default ...`), so the lint tier has one definition; ci.yml:83-86 already installs cargo-mutants and cargo-rdme, so nothing new is required of the runner. Acceptance: `just --show ci` names `gate-lints` as a dependency; a member manifest carrying `path = "../suanpan"` beside `workspace = true` fails `just ci`.
Construction: add `path = "../suanpan"` beside `workspace = true` on any member dependency; `just gate-lints` fails at manifestlint and `just ci` passes.

### tools-6: The roster's `red` class is documented as a buffer for owned reds awaiting cures, and the mechanism accepts any cell
- Where: tools/benchjudge:65-69 (related: benchjudge:74-80, 476-480, 528-536; tools/benchjudge-expected.json:2; crates/before/tests/bench_judge_roster.rs:7, 45-63; crates/before/benches/common/sidecar.rs:36-60)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read `load_roster` 476-480 and `roster_violations` 528-536: `red` is any list of strings, and any rostered cell that reads RED satisfies it; `git log -1` for 162290ab "bench judge: the hugeleaf display pair stays rostered red - the realization run's verdict" (07-27), a066a8e9 "declared models instead of standing reds" (07-28), and 920bfabb2 "board: excise the expected-reds triage buffer; any red of record fails outright" (08-07); `git log -S'owned reds' -- tools/benchjudge` returns only b4942461); executed: no
- Seen by: scaffolding [6], adequacy [21]; refutation: confirmed; history: deliberate-but-expired (the framing was written for a transitional roster during the kernel flip and was accurate then; the red set collapsed to the tripwire two days later; the owner ratified red = untriaged and moved the last library reds to declared models, rewriting the JSON notes but not the docstring; the 08-07 ruling's excision inventory lists no tools/ file)
- Owner-gated: yes (a roster's vocabulary is gate policy; the recorded ruling is board-scoped in letter, general in principle)
- Cross-references: meter-adequacy-8 and gate-legs-9 (simplification) record the docstring's queue framing from the sweeps; this entry adds the mechanism (`red` accepts any string, and a library cell can be rostered by two one-line edits) and the construction.

Three sites describe the class two ways: the docstring says roster mode keeps `just all` meaningful "while owned reds await their cures", while the JSON notes and the roster test say the set is exactly the permanent tripwire. The mechanism matches the docstring: a library cell added to `red` plus a one-line edit to `roster_red_membership_is_pinned` converts a standing regression into a pass, and history shows the path was used (162290ab). Doctrine under Principle 2: no mechanism for accepting known failures may exist, even as an empty buffer; a known-bad artifact required red is legitimate adequacy machinery, and the class name does not distinguish the two.

Evidence:

    65	Roster mode (`--roster FILE`): the committed expected-verdict roster —
    66	the board-side sibling of the gate's expected-failure test roster, same
    67	membership-by-name philosophy — makes the judge enforce a *fixed* verdict
    68	map instead of all-green, so `just all` stays meaningful while owned reds
    69	await their cures. The file pins the configuration it was recorded under

    (crates/before/tests/bench_judge_roster.rs)
    57	/// Removing an owned red silences a standing judgment and adding one
    58	/// launders a new regression as expected, so both directions must show
    59	/// up as a diff of this pin.

    (git show -s 920bfabb2)
    Owner ruling (2026-08-07, superseding the empty-buffer allowance): no
    expected-reds/accepted-reds mechanism may exist at all.

Resolution: bind the class to bench code the way `Ceiling` already is: the sidecar declares a tripwire flag at the cell's definition (only `display_schoolbook/hugeleaf` and tripwire.rs's cell qualify), `load_cells` cross-checks the two sidecars agree, and `roster_violations` requires roster `red` to equal the sidecar's tripwire set exactly, so a library cell can never be rostered and an owned regression must become a cure or a declared model. Rename the key (`tripwire`) so an awaiting-cure entry needs a schema change that `roster_schema_carries_expectations_only` and `load_roster` both refuse. Reword benchjudge:65-80, the JSON notes, and bench_judge_roster.rs:7 and 57-59 (drop "owned red"). Acceptance: self-test cases: a roster naming a non-tripwire cell is an input error (exit 2); a tripwire cell missing from the roster is an input error; the existing laundering pins still hold; the three descriptions agree.
Construction: add `"version_rank/harmonic"` to `red` in tools/benchjudge-expected.json and to the expected array at bench_judge_roster.rs:62. If that cell ever reads RED, `just bench-judge` exits 0 and the gate's test suite is green: a regression accepted by two one-line edits, with no cure and no declared model.

### tools-7: Unrostered cells drift GREEN and SKIP freely, so a board cell whose bench body goes dark passes the judge
- Where: tools/benchjudge:82-84 (related: benchjudge:537-547; crates/before/benches/board.rs:166-171, 242-245; crates/before/tests/bench_judge_roster.rs:50-52)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: assessed (read `roster_violations` 513-547: it flags a rostered red reading GREEN or SKIP and an unrostered RED, and nothing else; board.rs:166-171 is one shared closure for every board cell, while the tripwire cell at 242-245 is a separate closure); executed: no
- Seen by: adequacy [20]; refutation: confirmed (the construction traces to SKIP on every board cell, RED on the tripwire, no violation, exit 0); history: deliberate-and-holds for near-floor cells only (the rationale at 82-84 and in b4942461 is machine noise across the judgment floor; nothing addresses a cell far above the floor going dark)
- Owner-gated: yes (the drift is declared as design in the docstring)
- Cross-references: meter-adequacy-4 (this document) records the unpinned judged set from the adequacy sweep and proposes a roster `may_skip` class; this entry proposes the dual, a sidecar-declared `judged` flag, so the roster keeps carrying expectations only. board-families-floors-judge-11 and board-frame-3 (this document) hold the all-NA cell roster from the board side.

The only liveness signal in roster mode is the single rostered red, produced by `schoolbook_decimal` in bench code. Every library cell may read GREEN or SKIP with no expectation attached, so a cell whose timed body degenerates to trivial work is never flagged; the tripwire proves the judge's arithmetic and the criterion pipeline are alive, not that any library bench measures library work. bench_judge_roster.rs:50-52 claims the schoolbook red proves "the judge's time leg" alive, which overstates what that red covers. Doctrine: every ceiling needs a liveness floor so it cannot pass vacuously when the measured quantity goes dark.

Evidence:

    82	Any unrostered cell reading RED is a violation (exit 1). Unrostered cells
    83	may drift between GREEN and SKIP freely — both are non-red, and
    84	near-floor cells cross the judgment floor with machine noise. A roster

    (crates/before/benches/board.rs)
   166	            group.bench_function(cell.family, |b| {
   167	                b.iter_batched(
   168	                    || cell.body(),
   169	                    |body| black_box(body()),
   170	                    BatchSize::LargeInput,
   171	                );

Resolution: add a judged-set expectation: cells whose hi median sat well above the floor at pinning (a 10x margin, so noise cannot cross it) must read GREEN or RED, never SKIP, and such a cell reading SKIP is a violation. Declare it where the ceiling class is declared (the sidecar, a per-cell `judged` flag asserted against a pinned set), so the roster still carries expectations only and `roster_schema_carries_expectations_only` stays as is. Acceptance: a self-test case in which a judged-declared cell with a sub-floor pair returns exit 1 in roster mode; the construction below returns 1 instead of 0.
Construction: at board.rs:169 replace `|body| black_box(body())` with `|body| black_box(&body)` and run `just bench-judge`: every board cell reads SKIP (hi median below 10 us) or a flat GREEN, `display_schoolbook/hugeleaf` still reads RED, `roster_violations` finds nothing, exit 0.

### tools-14: covcheck's `remediation` disposition is a mechanism for accepting known failures, empty today
- Where: tools/covcheck:11-14 (related: covcheck:77, 264, 272, 282, 289; justfile:1006-1009, 1024-1026; tools/covcheck-expected.json)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified; executed: yes (`run()` over an expectation entry `{"anchor": "covered();", "disposition": "remediation", "why": "later"}` anchored on an uncovered reachable line returned no problems; the committed file tallies line 16 panic-arm / 8 unreachable, branch 10 panic-arm, 0 remediation)
- Seen by: scaffolding [3], adequacy [22], structure-prose [33]; refutation: confirmed; history: deliberate-but-expired (designed as the landing ratchet's named-gap category holding 31 entries at 8a9583c5 on 08-11, drained to zero by f77011e3 on 08-12; the owner ruled four days before covcheck landed, for the sibling board instrument in 920bfabb2, that no accepted-reds mechanism may exist even empty; the letter of that ruling is board-scoped, its principle general)
- Owner-gated: yes (the justfile designs the category)
- Cross-references: gate-legs-9 (simplification), the same disposition from the gate-legs sweep; tools-18 (simplification), the empty roster key its last cured entry left behind.

A `remediation` entry passes the coverage pin while stating that the arm is reachable and unexercised: an accepted known failure by construction, and git shows CI read green while seven such entries stood (aa7c96a0). Doctrine under Principle 2: every contradiction resolves to a fix or a model the owner declares; a reachable uncovered kernel line is a test gap, and until the test lands the leg is red.

Evidence:

    11	and the checker rejects any other: "panic-arm" (the untaken code is a
    12	panic only programmer error can trigger), "unreachable" (genuinely
    13	unexercisable, with a reconstructible argument naming the pinned
    14	invariant it rests on), and "remediation" (reachable but unexercised: a
    77	    dispositions = {"panic-arm", "unreachable", "remediation"}

    (justfile)
  1008	# panic-arm or an unreachable arm, its argument stated at the entry) or a
  1009	# named remediation item, and any NEW hole fails by name. MECHANISM:

Resolution: delete `remediation` from `dispositions` and the docstring; move the self-test fixtures at 264, 272, 282, 289 to `unreachable` or `panic-arm`; restate justfile:1006-1009 as "curated (panic-arm or unreachable, its argument at the entry) or a failure until a directed test lands" and 1024-1026 accordingly. If the owner keeps the category, record the ruling positively at line 77 as a deliberate exception. Acceptance: an entry carrying `"disposition": "remediation"` fails with the "never invent a category" message; `covcheck --self-test` passes with two dispositions; `just coverage-kernel` still passes on the committed roster.

Synthesis note: the excerpt's line numbers and the Where anchor were corrected at merge time: the quoted docstring lines sit at tools/covcheck:11-14 at `9e5784fb` (the partition report numbered them 12-15); the text is verbatim. gate-legs-9 (simplification) records the same disposition from the gate-legs sweep; this entry adds the executed demonstration that a `remediation` entry on an uncovered reachable line passes `run()`.

### tools-16: covcheck has no per-file presence floor: a scope file absent from the lcov is invisible unless it already carries an entry
- Where: tools/covcheck:150-154 (related: covcheck:53-56, 123-126, 170-173; tools/covcheck-expected.json:202; justfile:1006)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified; executed: yes (a second file under scope with no SF record and no expectation entry produced no problem from `run()`; enumerating crates/before/src/version/skyline/ gives 39 .rs files, 27 non-test and 12 tests.rs siblings; 12 of the 27 non-test files have no entry in either map: decode, emit, encode, fill/memo, literal, pool_traffic, query, query/web, shape, validate, walk, web_traffic)
- Seen by: adequacy [23], instrument-correctness [51]; refutation: confirmed (correcting [51]'s file counts); history: no-rationale-found (8a9583c5 enumerates the two tamper directions and the whole-report floor; a file dropping out of the lcov was not considered; the only presence pin that exists is the accidental empty key at covcheck-expected.json:202)
- Owner-gated: no

`parse_lcov` collects whatever SF records contain the scope prefix, and both checks iterate that set plus the expectation's files; nothing enumerates the scope directory itself. A kernel file with no entry can drop out of the report (cfg'd out, excluded by an ignore regex, moved) and covcheck reads green; the floor at 150-154 fires only when the whole scope is dark. Doctrine: a liveness floor asserts irreducible work per boundary, and the boundary here is the file. The justfile's GOAL (1006) is per arm.

Evidence:

    53	        if raw.startswith("SF:"):
    54	            path = raw[3:]
    55	            at = path.find(scope)
    56	            current = files.setdefault(path[at:], {"da": {}, "brda": {}}) if at >= 0 else None
   150	    if not any(hits > 0 for data in files.values() for hits in data["da"].values()):
   151	        problems.append(
   152	            "liveness: no covered kernel line in the lcov report; "
   153	            "the coverage run did not exercise the kernel at all"
   154	        )

Resolution: enumerate `Path(root, scope).rglob("*.rs")` at check time and fail by name for every file with no SF record, in both modes; state the presence rule in the docstring; decide deliberately at the scope declaration whether tests.rs siblings are in or out; add a self-test case with a second scope file the lcov omits; then delete the `place/filter.rs: []` entry (tools-18), whose only role this replaces. Acceptance: a coverage run with `--ignore-filename-regex 'skyline/walk\.rs'` makes `just coverage-kernel` fail naming walk.rs; the committed roster passes on a fresh lcov; the self-test case is committed.
Construction: gate `mod walk;` in crates/before/src/version/skyline.rs behind an unlit cfg, or pass cargo-llvm-cov `--ignore-filename-regex 'skyline/walk\.rs'`: the lcov carries no SF record for walk.rs, `files` and `resolved` have no key for it, the floor sees covered lines elsewhere, and covcheck exits 0.

### tools-19: digestshare passes a vacant, missing, or partially drifted corpus, keeps a dead V1 branch, and tracebacks on a headerless file
- Where: tools/digestshare:89-96 (related: digestshare:20-23, 43-49, 71-78, 81; justfile:211-220)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified; executed: yes (`python3 tools/digestshare <empty dir>` and `python3 tools/digestshare <nonexistent dir>` both printed nothing and exited 0; a `.snap` without insta's two-`---` header raised `IndexError` at line 47; `grep -l '│' tests/snapshots/*.snap | wc -l` is 0 of 22)
- Seen by: scaffolding [4], adequacy [27], [28], [57], structure-prose [37]; refutation: confirmed (all five; it also verified that every committed capture has at least four header lines, so a per-file `total > 0` floor is a live universal premise); history: no-rationale-found for the vacuity path (the missing-root discipline of 651eb92a predates digestshare's landing 5f140251 and was not adopted); already-known for the V1 half (verification-infra-11 in the rumors review; correct at landing, expired at the V1 retirement 368da2a5, whose re-denomination did not reach tools/)
- Owner-gated: no

The floor fires only when `v2_files` is nonzero, so a snapshot directory that is missing, empty, or renamed yields no output and exit 0; every other lint-tier tool treats a vacant scan as a usage error and says why at the site (doclint:387-394, testdoc:119-126, workflowlint:521-531). The V1 skip (20-23, 48-49, 76-78) guards a render form the tree no longer produces and is the only way a file becomes "not V2", so it doubles as a silent-skip path for any future render containing `│`. The floor is also corpus-wide: one capture whose headers stop matching contributes zero to the totals while its neighbours keep them nonzero, although the recipe's stated purpose (justfile:212-216) is exactly that vocabulary contract. The header split at 47 is a traceback on any file without two `---`.

Evidence:

    47	    body = text.split("---", 2)[2]
    48	    if "│" in body:
    49	        return None
    71	    dirs = [Path(arg) for arg in sys.argv[1:]] or [root / "tests" / "snapshots"]
    89	    if v2_files and (not grand_total or not grand_digest):
    90	        print(
    91	            "digestshare: LIVENESS FAILURE: the corpus has V2 captures but "
    92	            f"{grand_total} wire B / {grand_digest} digest B were parsed; "
    93	            "the renderer's vocabulary has moved out from under this tool.",
    94	            file=sys.stderr,
    95	        )
    96	        return 1

Resolution: refuse a nonexistent directory (exit 2, named); make the floor unconditional (zero files, zero wire bytes, or zero digests exits 1 naming the directory); delete the V1 handling and its prose so every `.snap` is a capture; require `total > 0` per file, failing by name (digests per file stay corpus-wide, since a capture with no listing legitimately has none); turn a file without two `---` into a named problem. Acceptance: `./tools/digestshare <empty dir>` and `<missing dir>` exit nonzero; rewriting one committed capture's `control item N (B bytes)` lines to another spelling fails naming that file; a headerless file is reported, not a traceback; `grep -c V1 tools/digestshare` is 0; the committed corpus still prints its table and TOTAL line.

### tools-28: mutantcheck never accounts for mutants missing from the filtered listing, and reads only `exclude_re`
- Where: tools/mutantcheck:156-161 (related: mutantcheck:38-41, 47-55, 377-378; .cargo/mutants.toml:78-82; tools/mutantcheck-expected.json)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified; executed: yes (`run()` over raw {a, b, d, e}, filtered {e}, two patterns claiming a and b with pins listed = suppressed = 1, returned no problems although d vanished from the filtered listing with no pattern claiming it; every committed entry has listed == suppressed, 14 of 14, so a widened exclusion cannot move any pinned number; the config today carries only additional_cargo_args, exclude_re, test_tool, test_workspace)
- Seen by: instrument-correctness [50]; refutation: confirmed; history: no-rationale-found (the docstring's model of record names exactly one accepted residual, the count-preserving swap; 3ed79544, which added the executable campaign keys to the config, changed only the docstring)
- Owner-gated: no
- Cross-references: gate-legs-6 (this document) asks when the mutation campaign runs; this entry is about what the `mutants-list` leg can see when it does.

The checker asserts filtered is a subset of raw but never that every mutant in raw minus filtered is claimed by a pinned pattern, and it reads only `exclude_re`. A file- or crate-level restriction added via `exclude_globs`, `examine_globs`, or `examine_re` shrinks the campaign of record while every pinned listed/suppressed count stays exactly where it was (listed is counted over the `--no-config` raw listing, and suppressed already equals listed for every entry), so the gate's mutants-list leg passes. Principle 6, against the docstring's promise at 38-41 that the judgment is tamper-evident in both directions.

Evidence:

   156	    extra = filtered - raw
   157	    if extra:
   158	        problems.append(
   159	            f"{len(extra)} filtered-listing mutant(s) absent from the raw "
   160	            "listing; the two captures are not from the same tree"
   161	        )
   377	    config = tomllib.loads(Path(args.config).read_text(encoding="utf-8"))
   378	    patterns = config.get("exclude_re", [])

Resolution: after parsing both captures, compute the unclaimed set (mutants in raw minus filtered matched by no pinned pattern) and fail by name for each; refuse any filtering key in the config beyond `exclude_re` (`exclude_globs`, `examine_globs`, `examine_re`) as an unpinned campaign restriction; add a self-test case: raw {a, b, c, d}, filtered {c}, d matched by no pattern, must red with "suppressed by no pinned pattern". Acceptance: with `exclude_globs = ["**/watermark.rs"]` appended to .cargo/mutants.toml and the captures regenerated, `just mutants-list` fails naming the watermark mutants or the key; the current tree stays green; the new case is committed.

### tools-33: workflowlint's interpreter recognizer stops at the first non-prefix token, so `| sudo -E bash -` and `| env -i sh` pass as fetch-without-execute
- Where: tools/workflowlint:151-160 (related: workflowlint:34-37, 40-43, 45-46, 405-473; justfile:188-196)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified; executed: yes (`fetch_feeds_interpreter` returned False for `curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -`, `curl https://x/i.sh | sudo -u runner sh`, and `curl https://x/i.sh | env -i sh`, and True for `| sudo bash` and `| bash -s -- --yes`; `run()` over a workflow fixture carrying the nodesource line returned no problems)
- Seen by: scaffolding [0]; refutation: confirmed; history: no-rationale-found (facc7550's fixtures at 426 and 444 are bare-prefix only; the docstring's own over-matching policy at 40-43 argues for the fix)
- Owner-gated: no

`invokes_interpreter` returns on the first token that is neither a listed prefix word nor a `VAR=value` assignment, so any flag on `sudo` or `env` makes `base` the flag and the segment reads as a non-interpreter. The docstring enumerates `| env sh` and `| sudo bash` as caught, and the flag-bearing spellings are the ones installers use; they sit inside the tool's stated one-pipeline boundary (45-46), not the documented two-step exception. Principle 6: the cheapest passing artifact must be the intended one.

Evidence:

   151	    for token in segment.strip().split():
   152	        word = token.strip("\"'")
   153	        if word in ("sudo", "env", "exec", "command", "nice", "time"):
   154	            continue
   155	        if "=" in word and not word.startswith(("<", "$")):
   156	            continue  # an env-style VAR=value prefix
   157	        if word in ("$SHELL", "${SHELL}"):
   158	            return True
   159	        base = word.rsplit("/", 1)[-1]
   160	        return base in INTERPRETERS or bool(PYTHON.fullmatch(base))

Resolution: adopt the tool's own over-matching policy: after a prefix word, skip flag tokens (those starting with `-`) and their arguments, or treat a post-fetch segment as fetch-execute when an interpreter token appears anywhere in it after stripping prefix words, flags, and assignments. Add `curl ... | sudo -E bash -`, `curl ... | sudo -u runner sh`, and `curl ... | env -i sh` to the self-test's red fixtures. Acceptance: `./tools/workflowlint --self-test` fails on the current recognizer with the three new fixtures and passes after the fix; the fetch-to-a-file green fixture stays green.

### tools-11: benchjudge's self-test is fifty bare asserts that `python3 -O` strips
- Where: tools/benchjudge:596-598 (related: benchjudge:586-923; tools/citecheck:805)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified; executed: yes (`PYTHONOPTIMIZE=1 python3 tools/benchjudge --self-test` printed nothing and exited 0; `grep -cE '^\s*assert '`: benchjudge 50, citecheck 1, every other tool 0)
- Seen by: adequacy [29]; refutation: confirmed; history: no-rationale-found (facc7550 closed this hole for workflowlint alone: "self-test failures raise explicitly, surviving PYTHONOPTIMIZE"; benchjudge's asserts were never revisited)
- Owner-gated: no
- Cross-references: benches-examples-15 (this document) records that the self-test runs only at the head of the bench-judge recipes; this entry adds that where it does run, `PYTHONOPTIMIZE=1` makes it check nothing.

Under `PYTHONOPTIMIZE=1` every `assert` in `self_test` is removed and `--self-test` returns 0 having checked nothing, while the bench-judge recipes run `--self-test` first precisely so the judge cannot go dark unnoticed. Every other tool raises `AssertionError` explicitly (citecheck has one bare assert at 805). Principle 1: the likelihood of the environment carries no weight.

Evidence:

   596	    assert verdicts([("op/linear", 1e6, 4e6, 1000, 4000, "general")]) == [
   597	        ("GREEN", "op/linear")
   598	    ]

Resolution: open `self_test` with `if sys.flags.optimize: raise SystemExit("benchjudge --self-test needs asserts; run without -O")`, or convert the asserts to explicit `if not ...: raise AssertionError(...)` as the sibling tools do; same one-liner for citecheck:805. Acceptance: `PYTHONOPTIMIZE=1 python3 tools/benchjudge --self-test` exits nonzero.

### tools-15: covcheck validates only the disposition: a `why`-less or extra-keyed entry passes and a missing `anchor` tracebacks
- Where: tools/covcheck:87-95 (related: covcheck:8-10; tools/mutantcheck:204-219)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified; executed: yes (an entry `{"anchor": "None => unreachable!(", "disposition": "unreachable", "bogus": 1}` with no `why` passed `run()` with no problems; an entry lacking `anchor` raised `KeyError: 'anchor'` at line 95; every committed entry carries a nonempty `why` and no keys outside {anchor, disposition, why, offset})
- Seen by: instrument-correctness [55]; refutation: confirmed; history: no-rationale-found (the landing self-test pins the invented-category path only; the round 11363f89 that made sibling checkers total on malformed input did not touch covcheck)
- Owner-gated: no

The docstring promises "each with a curated disposition and the argument for it"; the checker never requires the argument. The cheapest artifact that passes the pin is an `unreachable` entry with no `why`, exactly the category the docstring says must carry "a reconstructible argument". A missing `anchor` discards every other finding as a traceback, against the discipline mutantcheck's `compile_pattern` applies.

Evidence:

    87	        for entry in file_entries:
    88	            if entry.get("disposition") not in dispositions:
    89	                problems.append(
    90	                    f"{relpath}: entry {entry.get('anchor')!r} carries disposition "
    91	                    f"{entry.get('disposition')!r}; only "
    92	                    f"{sorted(dispositions)} exist — re-curate, never invent a category"
    93	                )
    94	                continue
    95	            anchor = entry["anchor"]

Resolution: validate each entry's key set as exactly {anchor, disposition, why} plus optional {offset}, with anchor and why nonempty strings and offset an int, reporting violations as problems; add self-test cases for a missing why, an extra key, and a missing anchor. Acceptance: `{"anchor": "...", "disposition": "unreachable"}` fails by name; the committed file still passes.

### tools-23: fuelscape-claims has no liveness floor and no self-test
- Where: tools/fuelscape-claims:30-49 (related: justfile:175-176, 700-703, 723-726)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified; executed: yes (a scratch copy of the tool and bundle with `index.json` set to `{"ops": []}` printed "fuelscape-claims: 0 claims accepted by the widget's own rule" and exited 0; the committed index carries 104 ops)
- Seen by: scaffolding [10], structure-prose [36], instrument-correctness [56]; refutation: confirmed (noting `fuelscape-verify` byte-diffs the committed directory against regeneration in ci, which bounds an emptied index landing silently but demonstrates nothing about `accepts` refusing a bad claim); history: no-rationale-found (61f05692 recorded one live catch and no floor; every other tool gained a self-test at or after landing under the recipe convention at justfile:175-176)
- Owner-gated: no

The loop over `index.ops` is the only failure source; a zero-length list passes as a clean sweep, and nothing committed demonstrates that a known-bad claim (the header's own example, `log n` over a dataset whose smallest column is n=1) is refused. Principle 2: a meter over a counter passes vacuously when the counter goes dark; Principle 6: every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

    33	for (const op of index.ops) {
    48	if (failures) process.exit(1);
    49	console.log(`fuelscape-claims: ${index.ops.length} claims accepted by the widget's own rule`);

Resolution: fail when `index.ops.length === 0` or when any `doc.op.claim`/`sizes` is missing, naming the file; add `--self-test` asserting `Fuelscape.accepts` refuses a syntax error and the `log n` at n=1 case and accepts a linear claim; run the self-test first in the recipe like the other tools. Acceptance: `{"ops": []}` exits 1 naming the floor; `tools/fuelscape-claims --self-test` fails when `accepts` is stubbed to `() => ({})`; the committed index still passes with its count printed.

### tools-26: memwatch's kill path has no committed demonstration
- Where: tools/memwatch:100-115 (related: memwatch:82-87, 131-158; justfile:101-105)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (grep for any invocation of memwatch other than as a recipe wrapper finds none; the script has no `--self-test`); executed: no
- Seen by: adequacy [30]; refutation: confirmed (a self-test is feasible: `PROC_LIMIT_GB=0` makes every process a candidate and `is_build_proc` is name-matched, so a fake `*rustc*`-named child that allocates demonstrates the path); history: no-rationale-found (the kill and freeze paths were demonstrated by hand and the demonstrations live only in commit messages 38aaad3a and ccbcede8)
- Owner-gated: no

The watchdog wraps the codegen-running recipes, and its targeting (the awk parent-chain walk, the per-pid re-check, the build-proc filter) is the most intricate code in tools/, yet nothing committed ever drives a candidate over the limit. A rot in the `ps` column parse or the chain walk reads as a green gate until the next runaway. Principle 6: every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

   100	own_procs() {
   101	    ps -axo pid=,ppid=,rss= | awk -v root="$ROOT" '
   109	                    if (a == root) { print p, rss[p]; break }

Resolution: add a `--self-test` that runs memwatch with a tiny `PROC_LIMIT_GB` around a child whose command matches the filter (a copied `python3` named `rustc-selftest`, or a script) that allocates past the limit, asserting the `KILL pid` note and a nonzero exit; and a second case where the child is named outside the filter and survives; run it at the head of one recipe; degrade explicitly (skip with a message) where `ps -o rss` differs. Acceptance: both cases pass on macOS; the construction below fails the self-test.
Construction: swap the fields at line 109 to `print rss[p], p`: every recipe still passes, and pid numbers are compared against `lim_kib`, so the next runaway is never killed.

### tools-30: readme's crate roster is hand-maintained with no totality check, and `image_refs` is never exercised by the self-test
- Where: tools/readme:78-108 (related: readme:144-154, 184-203, 238; tools/manifestlint:83-90; Cargo.toml:2-8)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`grep -rl 'cargo-rdme start' --include='*.md'` returns exactly README.md, crates/before/README.md, crates/suanpan/README.md, crates/rumors-tracing/README.md, matching the four CRATES entries; the workspace lists five members plus the root package; crates/before-viz/README.md exists without markers; every `self_test` case at 238 passes `{}` for image_refs); executed: no
- Seen by: scaffolding [11], structure-prose [35]; refutation: confirmed; history: no-rationale-found (the roster began as two crates in 2a3a752c and grew by hand; the stripper self-test landed with `{}` in every case)
- Owner-gated: no

Nothing checks that the set of marker-bearing READMEs equals CRATES, so a new crate or a README that gains markers is silently never derived or checked; manifestlint in the same directory already shows the idiom (ask `cargo metadata` for the members). The comment-wrapped image-reference unwrap at 146-151 is untested although the docstring (17) says the self-test pins the forms rewritten.

Evidence:

    78	CRATES = [
    79	    Crate(name="rumors", readme="README.md"),
    80	    Crate(
    81	        name="rumors-tracing",
    82	        readme="crates/rumors-tracing/README.md",
    83	        rdme_args=["-w", "rumors-tracing"],
    84	    ),
   238	        actual = strip_intra_doc_links(source, {})

Resolution: derive the crate set from `cargo metadata --no-deps` packages whose README contains the markers, keeping only `image_refs` as per-crate data; or, minimally, assert in `check()` that the marker-bearing README set equals CRATES and fail by name. Add a self-test case for the comment-wrapped image reference. Acceptance: adding the markers to crates/before-viz/README.md with no roster change makes `just readme-check` derive it or fail naming it; `tools/readme self-test` has an `image_refs` case.

## Positives

The instruments below do what the doctrine asks and are worth naming so the gaps above are read in proportion. Each is verified by a finalizer's reading of the cited artifact unless marked otherwise.

- Every judgment leg of the board pairs a known-bad artifact that reads red with a correct one that reads green, through `evaluate` itself: the meter-bypassing walk against the floors, the chunked schoolbook converter against the `n_io` exponent, the lump ladder against the quadratic ladder through `evaluate_acceptance`, the fold model's fat constant and quadratic, the capacity band red on both the regressed and the improved side (board/tests.rs).
- Floors are derived from operands at prepare, never from readings, and fork on the verdict a cell will produce; `touch_pair_fold` (floors.rs:458-492) is a model minimum-work derivation: per boundary not per element, a max rather than a sum with the reason stated, zero deltas excluded with the mechanism named. `ByCurrency` makes a new currency a compile error at every declaration and judgment site.
- The shard merge's completeness refusal is a total check on the union with a committed known-bad capture swept over the family axis at every scale (`merge_refuses_a_silently_shrunk_grid_for_every_family`); floats cross the wire as IEEE-754 bit patterns and the parent refuses any header not byte-identical to the one it commissioned; `required-features` on `amp_board` makes an unfeatured board a cargo error rather than a matrix that looks judged.
- The envelope suite puts a value leg beside almost every cost pin (closed-form tick totals, rank modularity, byte identity against the public operator), derives its liveness floors with the premise at the constant (`SEAM_PLUNGE_TOUCH_FLOOR`, `LADDER_MARGINAL_TOUCH_FLOOR`, the weight-comb and freeze-parade floors), cross-multiplies ratios in `u128`, and states its costs relationally where it can: the `placement`, `span`, `span_codec`, and `identity_fast_paths` modules pin exact identities against compositions on the same operands (`fused + cmp_ss / 2 == cmp_sv + cmp_se`), each with a nonzero liveness read and a walking control, so nothing there can rot.
- `id_walk_scan_cost` pins the covers and disjoint walks with two-sided exact equality (500,004 and 1,000,004 = 2·(2d + 2) at both depths); `masked_cmp_hole_depth_band` asserts `lo == hi`; `dominated_undercut_cost` reads the decision counter beside the touch band; `pool_recycle` derives its ceiling of 2 from peak simultaneous demand and `.cargo/mutants.toml` relies on it to kill the retire/lease mutants.
- `tests/superlinear_tripwires.rs` binds every committed known-bad kernel by name in both directions, and the eight adequacy kernels in query/tests.rs are value-exact and rostered; `meet_fold` commits and rosters its own known-bad and places its floor midway between the linear and quadratic signatures.
- The verdict matrix executes the doctrine's order: verdict-class liveness floors first, a committed demonstration that the floors read red on degenerate pools with the census shape pinned, two committed mutant twins through the same checker with the legs neither can reach rostered by name, and only then a green production run; its pool derives from an exhaustive match over `FamilyId` and its budget from the roster count.
- The semantic-oracle conviction pair (`rank_differential_convicts_the_cell_dropping_riemann_sum`, `worked_value_anchor_convicts_the_mirrored_embedding`) commits each defective reference behind an inverted assertion, proves the pointwise differential blind to the mirrored embedding, and is rostered in `surface_coverage::TRIPWIRES` and resolved against nextest's live inventory by `tools/citecheck`; `corpus_counts_are_exact` pins the id corpus by the closed form `2^(2^d)` and event distinctness by an independent path-sum walk.
- `join_all_differential_convicts_the_dropped_group_oracle` (party/tests.rs) commits a known-bad reference and shows the differential rejects it at exactly the widths that reach the arm; the fold hand-back discipline has a deterministic witness, a proptest, and that convicted oracle.
- `deep_tree_stack_safety` reasons about its own vacuity (equal operands would short-circuit on `canonical_eq`, so the join/meet sweep is driven on two distinct deep versions), and the deep proof is broader than AGENTS.md:37 says: three depth-100k clock tests, the id text parser at 100k, the id diff ladder at 100k, and envelope rows at `ID_DEPTH = 250_000` including the public `without`.
- The `diff_ops!` macro makes registration and execution the same act, its signature tuples are a compile-time tie, the tiling pin is two-directional, and the known-bad descriptors are convicted with two-direction witnesses; `laws!` and `for_each_law_group!` give every consumer compile-time refusal of a novel signature (verified at algebraic_laws/tests.rs:304 and 447, fuzz_laws.rs:225, surface_coverage/tests.rs:110).
- The fuelscape sampler's adequacy pins form a triangle of independent oracles (table versus enumeration to 24 bits, grammar versus the shipping decoders as set equality to 2 bytes, decoder census to 23 bits, fixed-seed chi-square, proptest round trips to 48 bytes); `parallel_build_matches_sequential_reference` const-asserts its own liveness; the dump reader recomputes every grid and refuses disagreement with a committed tamper demonstration; replay from a dump is pinned byte-identical to a direct render across the whole roster.
- The fuzz seed corpus has one derivation shared by `#[path]` between the writer example and the checker test, every hand-authored non-canonical byte carries its bit-level derivation, and `fuzz_decode` asserts byte identity on accept, refusing an accept-and-normalize decoder outright; `fuzz_laws` expands its drive loop from `for_each_law_group!`.
- The fuzz-fit harness prices ceiling and floor widths separately with each margin naming the two measured edges it sits between, ties `REGS_RESERVE` to both budgets by a const assert, ships a synthetic known-bad case per judgment leg that states what it does not prove, turns a sampled verdict into a total one with the `(1 - q)^48` argument written where the leg lives, and discloses its blessed-drift window candidly; the differential against the native mirror is total (every step's return code and every live register's bytes).
- The wasm32 guest synthesizes every input in-guest with the strict canonical decoder as the synthesizer's oracle, gives each coordinate adjacency witnesses on both sides, runs each pin in a fresh instance, keeps `overflow-checks = true` in the release profile, and carries an always-green liveness pin.
- suanpan's metered pins assert exact totals at two scales so the liveness floor and the flatness witness are one assertion; `no_collapse_fold_re_scans_the_prefix` is a committed known-bad shown red at two widths; the claims roster is total in both directions over the extracted surface and binds witnesses by reach; `assert_value` drives every value check through all three read-outs against an exact `IBig` oracle.
- The bench judge's `--self-test` pins its whole exit contract including two ceiling-laundering attacks, `MIN_JUDGED_MEDIAN_NANOS` is derived rather than calibrated, the roster schema pin refuses any expectation vocabulary beyond `red`, and bench IDs are the board's own cell names, so a red board cell names the bench that times it.
- `gate-streams` carries a liveness floor on its own verdict (a stream killed without writing a marker fails the gate); every `tools/` checker leads with a `--self-test`, refuses a missing root as a usage error, and pins its tool version; `covcheck` is tamper-evident in both directions and fails closed on a vanished anchor; the `features` recipe checks every cfg-gated surface alone.
- The two witness passes did what the doctrine asks of a review: 54 of the 69 constructions in these classes ran (52 demonstrated, 2 refuted; 15 needed a command the witness could not run), and where a construction fell (surface-roster-6's exposure clause, board-ops-render-26's numbers, meter-registry-tier2-7's degradation) the refutation is recorded beside the entry rather than folded into it.

## Open questions for Finch

Deduplicated across the reports; each carries the finalizers' recommendation, restated where two reports recommended the same thing.

1. **The segments currency.** Dissolve it from the board and the envelope suite (naming `deep_tree_stack_safety` and the `cfg(test)` gate on `descend!` as the no-recursion instruments), or make `stacker` an optional dependency under `meter` and give the column a writer in every build that reads it? Six reports recommend dissolution; the 2026-07-24 keep's premise changed on 2026-07-31 when `grow` went `#[cfg(test)]`. Either way recurse.rs:16-20 and 74-76 and tests/meter.rs:22-26 need restating now (board-ops-render-15, crate-root-32, envelopes-a-2, module-graph-1, recursion-1, inventory-2).
2. **The heap exponent's residual fit.** Under a fit over `m - HEAP_FLAT_ALLOWANCE_BYTES`, what materiality guard: a fixed fraction of the allowance (the finalizer suggests a quarter), or a per-denominator-byte minimum tied to `MAX_HEAP_BYTES_PER_INPUT_BYTE`? The choice sets how small a super-linear heap term the board resolves at KiB inputs (board-families-floors-judge-21).
3. **The membership and covers rows.** Replace the early-exit probes with certifying ones (`since(&w).contains(&v)`; a buffer-distinct `a.covers(&decode(a))`) and re-pin the two `WORST_RANKINGS` rows with the movement annotated, or add certifying rows beside them and move every family's declared cell count? Recommendation: replace (board-ops-render-9). Relatedly, should `version_eq` take a byte-equal, buffer-distinct pair so the time leg's backstop claim is true (meter-adequacy-7)?
4. **The fuzz-fit vocabulary.** Is the 44-kernel scope a standing decision (then the justfile wording is the whole fix), or the state the fuelscape panels outran? Recommendation: bind the `Op` vocabulary to `METHOD_SURFACE` with a reviewed exemption list, add the operations that fit the register machine today (`ticks`, `forks`, the `*_all` operations, `span`/`span_all`, `eq`, the `Rank` codec, `Ranked`, clock text I/O), and re-pin (fuzzfit-strategies-7, meter-adequacy-2).
5. **The shape-leg reference.** If `local_slope_excess` judges against `min(band.slope, 1.0)`, does any lane legitimately trend above 1.0 within a case (the design note names `ff_rank_display`)? One `bin/calibrate` run under the new reference answers it; declare per-key exponents for those rows rather than loosening the global allowance (fuzzfit-bands-10).
6. **The fuzz heap cap and the fuzz oracles' cadence.** Proportional cap (`max(FLOOR, PER_INPUT_BYTE * len)`, measured over the seeds first) or ratified flat cap with the doc restated; and is a `-runs=0` seed replay per target an acceptable gate leg? Recommendation: proportional, and add the replay leg; both branches owe a committed test that the cap fires (fuzz-guests-pins-14, fuzz-guests-pins-38).
7. **The mutation campaign's cadence.** A scheduled sharded CI job, a committed dated attestation a lint leg holds fresh, or an explicit statement at each recipe that the campaign is manual? At 54,918 listed mutants it cannot be per-commit; the witness pass's ad hoc campaign found two survivors, so the question is live (gate-legs-6, codec-bits-15, skyline-watermark-24).
8. **The wasm32-pins leg in CI.** Add it to the `instruments` job or declare it local in both comments with the reason (runner memory, justfile:634-636)? Either answer also fixes the four prose rosters that omit it, and the lockfile audit should be driven from `git ls-files '*Cargo.lock'` so a seventh workspace cannot be missed (deps-1, gate-legs-3, fuzz-guests-pins-1, fuzz-guests-pins-37).
9. **The memory-terminal pins.** Are the three wasm32 terminal pins a declared model or a pending cure? Their header and their docs disagree. Recommendation: declare the model, restate the pins positively, and add the panic-genre discriminator so the instrument rather than the prose establishes "allocation failure" (fuzz-guests-pins-35, with fuzz-guests-pins-33 in the documentation class).
10. **Process isolation.** One shared `require_process_isolation()` asserting `NEXTEST_EXECUTION_MODE == process-per-test` in `before::meter` (complete, but forbids a single-threaded `cargo test` that is in fact safe), or the status quo with the premise stated where each pin lives? Recommendation: the environment check, since every gate and coverage leg already runs nextest (suite-economics-1, envelopes-a-3, envelopes-b-10, meter-registry-tier2-19).
11. **Probe-build adequacy witnesses.** Commit the three refuted mechanisms as kernels (a digit-by-digit settle, a scaled read from digit 0, a high-water-bounded sign read), which means a test-only accumulator strategy inside suanpan, or drop the "adequacy witness" wording at the weight-comb, freeze-parade, and tooth-tail bands and the registry? (envelopes-a-22, meter-registry-tier2-7, meter-adequacy-6).
12. **Fold-operation floors.** Assert the in-run linear reference beside each `MIN_GROWTH` floor (additive, inside the f0cd4ab2f ruling), or replace the measured midpoints with a model-derived margin over the in-run reference (reverses that ruling)? Recommendation: assert the reference now; and should a totality test derive the pinned-operation roster from the `fuelscape/*.json` contracts, then pin or excuse `sync_all` and the four `Span` fold operations? (testing-diff-gen-26, meter-adequacy-5, testing-diff-gen-23).
13. **Bespoke flatness suites.** Register WT, WL, and the forked deep spine as `Shape`s with band citations and one committed known-bad kernel each, or record at each site that the closed-form separation stands in for a known-bad; and is `limb == 0` the intended model for the hull fold on a dense spine? Recommendation: register; pin `limb == 0` with its reason and put the floors on scan and touch (tests-other-6, tests-other-14).
14. **Unrostered pins and the band convention.** Rename the thirteen two-point pins into the `_is_flat_per_unit`/`_band` convention and cite them from `Bands::Priced`, or key the parity scan on the mechanism (`assert_flat` calls)? Either way, should `tools/citecheck` become the collection authority for the superlinear, inverted-twin, and band rosters so `#[ignore]` reads red? Recommendation: rename and extend citecheck (envelopes-b-22, envelopes-b-25, meter-registry-tier2-3, tests-other-24, surface-roster-11).
15. **The `Party::forks` balance and the shape walks.** The public minimal-depth promise has no instrument (a 3:1 split passes the gate); and should the four shape walks join the board as a row group or gain fuzz-fit ops? Recommendation: the closed-form depth pin with the 3:1 known-bad committed; board rows first for the walks (party-12, meter-adequacy-1, clock-9, recursion-6).
16. **The join/meet subadditivity bound.** Where does the packed-size bound the `O((|self| + |iter|) log k)` fold contracts need live: the rustdoc of a test-module constant (today) or the public contract of `join`/`meet`? The paper-fidelity and rumors-dependence sweeps both ask (version-core-5 in the claim class; span-causally-8 here for the span folds priced by proxy).
17. **The `O(M(|v|) log |v|)` clause.** Instrument it (a multi-scale fuel fit past 65 KiB, or a `meter_product` operand-width check on the doc's tight construction) or narrow the public contract to what committed counters pin? Recommendation: the fuel fit, since the harness already prices `ff_version_rank` in the currency that counts the backend's work (skyline-query-9).
18. **The masked-cmp envelope under llvm-cov.** CI run 33567211421 on `main` failed the `coverage` job at `just coverage-kernel (line, stable)` with `masked_cmp_hole_envelope` reading peak heap 1156 B against a pinned 480 B, while the same test reads 384 B under plain nextest and the previous `main` run passed all three jobs. What allocates under instrumentation, and should the envelope suite leave the coverage recipes? Not investigated by any report; the log is at `scratchpad/before/final-sweep-deps/ci-failed.log` (gate-legs open question 1, deps open question 4).
19. **Detached-workspace lockfiles.** State a convergence policy for the five detached locks (dashu-int 0.5.1 against the root's 0.5.0, borsh 1.8.0 against 1.6.1, bytes 1.12.1 against 1.11.1, wasmtime 47.0.3 against 47.0.4), or add a cross-lock diff to the supply-chain leg? (deps-9). And should `cargo deny` include dev-only duplicates (rand 0.8/0.9, thiserror 1/2) with a roster, or state the exemption? (deps-2).
20. **`exhaustive_deep` and the 2 GiB paren witness.** Has `exhaustive_deep` completed since the #53 leg split, and does it get a `just exhaustive-deep` recipe with a cadence? Should the 2 GiB witness ride `just all` or become a compile-time width pin at text.rs:194? (testing-oracles-24, suite-economics-6).
21. **The `decided` fields.** Are the dated `decided:` literals and `REGISTRY_RATIFIED` an intended embedded decision-record schema, or dated rationale to dissolve? d2a9d04e already put this to you; two sweeps and the registry partition recommend dissolving (meter-registry-tier2-13 here; surface-roster-17 and prose-hygiene-7 in other classes).
22. **`fuelscape-verify` and the cross-platform grid pin.** Promote `fuelscape-verify` from `ci`-tier to the gate if its minute fits the wasm stream, and state at `aggregate_bins_identically_on_every_platform` that the two-architecture gate run plus `fuelscape-verify` are the cross-host check of record, with a fixture whose extremes are not powers of two (fuelscape-render-7).
23. **Verbatim adequacy copies.** Share the query adequacy kernels' drivers test-locally (the 982bd260 precedent for `mass_split`), or keep the copies independent and strike "verbatim" from the ten docs? Recommendation: the shared driver (skyline-query-31).
24. **Small rulings the entries ask for.** Whether a value-neutral wide call should spill the register (suanpan-tests-4); whether `Query::coverage`'s clone-identity rung needs a cross-buffer agreement test now or only when `refine_partial` next changes (span-causally-35); whether the two `#[ignore]`-blind fixtures in surface-scan should assert the ignored name excluded (surface-roster-11); and whether the bench judge's expected sub-floor set should be pinned as a `may_skip` class rather than left as a prose count (meter-adequacy-4).

## Counts

| section | high | medium | low | nit | total | verification-gap | test-quality |
|---|---|---|---|---|---|---|---|
| The enforcement chain: gate legs and meter adequacy | 0 | 6 | 6 | 1 | 13 | 12 | 1 |
| Other cross-cutting sweeps | 1 | 7 | 7 | 0 | 15 | 15 | 0 |
| Crate root and public types | 0 | 3 | 15 | 4 | 22 | 13 | 9 |
| The skyline coding | 0 | 8 | 8 | 6 | 22 | 12 | 10 |
| The codec | 0 | 3 | 2 | 1 | 6 | 1 | 5 |
| Cross-cutting: fold, shape, recurse, serde and borsh | 0 | 1 | 1 | 1 | 3 | 1 | 2 |
| suanpan | 0 | 5 | 13 | 3 | 21 | 12 | 9 |
| The instruments | 2 | 53 | 65 | 19 | 139 | 86 | 53 |
| Workspace tools (tools/) | 0 | 8 | 5 | 0 | 13 | 13 | 0 |
| **all** | 3 | 94 | 122 | 35 | 254 | 165 | 89 |

Witness outcomes recorded above: demonstrated 52, inconclusive 15, refuted 2.
