# Sweep paper-fidelity: Fidelity to the ITC 2008 paper, the oracle as ground truth, and the derivations

## Method and coverage

This is the verification-and-finalization pass over the paper-fidelity sweep's fourteen seed findings, at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 with a clean tree. For each seed I opened every cited site with line numbers, re-read the excerpt against the file, and checked the claim's mechanism: the paper transcription (`crates/before/reference/itc2008.md` §3 lines 100-143, §5.3 lines 430-550, the section headings), the oracle (`crates/before/src/oracle.rs` 1-45; `oracle/version.rs` 1-30, 100-300; `oracle/tests.rs` 85-110, 230-255, 275-306, 545-565, 600-650), the impl sites (`version/skyline/grow.rs` 20-50 and 130-145; `version/skyline.rs` 1-12 and 60-130; `fold.rs` in full; `party.rs` 300-385 and 755-770; `clock.rs` 225-245, 426-501, 620-650; `version.rs` 20-32, 41-62, 285-300, 444-491), the test surfaces (`testing/grow_brute_force.rs` 45-80; `testing/semantic_oracle.rs` 1-40 and 300-347; `testing/semantic_oracle/tests.rs` 285-325; `testing/optrace.rs` by grep; `laws.rs` 1-60, 2395-2470; `party/tests.rs` 95-125; `testing/validation_index.rs` 130-150; `tests/superlinear_tripwires.rs` 1-40), `crates/suanpan/src/lib.rs` 1-260, `crates/before/examples/space_consumption.rs` 1-80, `crates/before/results/space_consumption/README.md` in full and the CSV header plus the endpoint rows cited below, `crates/before/AGENTS.md` in full, `crates/before/README.md` 1-8 and 312-322, `crates/before/build.rs` by grep, the `contract:` strings of `crates/before-fuelscape/src/ops.rs` (105 rows by grep, with lines 622-635 and 2140-2150 read), and the `.cargo/mutants.toml` header.

Mechanical checks: `grep` for `mod implementation` and `Law of Disjointness` across `crates/before` (the first hits only AGENTS.md and one test comment, the second nothing); `grep -c '\bmint'` over `crates/before/src` (56 sites, per-file counts recorded); `grep` for `peek` and `anonymous` in the public source files; `grep` for `100×|100x|naïve|naive` across `results/`, `examples/`, `benches/`, `tests/`; `git log -S` for the strings `1,000,000 events`, `ln(N)`, `100× more`, `asymptotically linear`, and `pub mod implementation` in `crates/before/src/lib.rs`, with the resulting commits (4488d657e, d45597843, a431eaf1, 67970b75, 22cdfbe1, f204638d) read; `Cargo.lock` for `dashu-int` and the resolved 0.5.0 source's NTT `cfg`. I ran no cargo, just, or test command: none of the surviving findings turns on a runtime fact that a hand trace or the committed tests' own doc comments did not already settle, and another review workflow may be building in this workspace.

What this pass could not see: whether the deleted design essay (`src/implementation.rs`, 270 lines removed in 22cdfbe1) was relocated anywhere other than `skyline.rs`/`lib.rs` (the commit changes those by 6 and 7 lines, so not there); the derivation, if one exists, of the packed-size bound the `O((|self| + |iter|) log k)` fold contracts need (not in `fold.rs`, `version.rs`'s join docs, or `skyline.rs` 1-130; `emit.rs` and `tests/meter.rs` were read only by grep); and the process-regime event count per iteration in `examples/space_consumption.rs` (lines 80 onward unread), which the extrapolation in finding 1 hedges.

## Findings

### paper-fidelity-1: Space Efficiency paragraph quotes figures no committed measurement produced
- Where: crates/before/src/lib.rs:312-319 (related: crates/before/README.md:316-322, crates/before/results/space_consumption/space.csv:40,79,264-270,305, crates/before/results/space_consumption/README.md:29-34)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (grep of the tree for the quoted numbers; CSV rows read and ratios computed from them; `git log -S` on the paragraph); executed: no
- Verification: confirmed, reframed by history: the figures are the retained outputs of closed-form estimates that a later commit deleted; history: deliberate-but-expired (4488d657e introduced `⌈ln(N)/2⌉` and `⌈N/2 + N·log₂(E/N)/24⌉` together with the figures they evaluate to; d45597843, the same day, removed the formulas and kept the figures, with no recorded reason)
- Owner-gated: no

The crate's front page states per-`Party` and per-`Version` byte sizes at "100 parties and 1,000,000 events" and churn figures ("around 50 bytes", "around 2,000 bytes"). Nothing committed produces them: the only space measurement in the tree (`results/space_consumption/space.csv`, from `examples/space_consumption.rs`) runs populations up to 128 for 25,000 (process) and 100,000 (data) iterations and records whole-stamp sizes (`Clock::encoded_bits`), never a party/version split. The `git log -S` trail shows the numbers are evaluations of two analytic formulas (`⌈ln(100)/2⌉ = 3`, `⌈50 + 100·log₂(10⁴)/24⌉ = 106`) whose derivation the tree no longer carries.

Evidence:

       312	//! At 100 parties and 1,000,000 events, the expected size of a [`Party`] is
       313	//! about 3 bytes and the expected size of a [`Version`] is about 100 bytes.
       314	//! These figures assume static membership; continually [`fork`](Clock::fork)ing
       315	//! and [`join`](Clock::join)ing causes these to grow, but with reasonable
       316	//! bounds. Under sustained random membership churn, those same 100 parties will
       317	//! each stabilize at around 50 bytes (growing linearly in the steady-state
       318	//! number of parties `N`) and their corresponding versions at around 2,000
       319	//! bytes (roughly `N²` in the steady-state number of parties `N`).

    d45597843 (crates/before/src/lib.rs), removed lines:
    -//! For a system with `N` parties and `E` total events, this crate's
    -//! implementation represents an individual [`Party`] in approximately `⌈ln(N) /
    -//! 2⌉` bytes and a [`Version`] in approximately `⌈N / 2 + N · log₂(E / N) /
    -//! 24⌉` bytes. To give a sense of scale, at 100 parties and 1,000,000 events

    space.csv endpoint rows:
       270	process,128,25000,1167.2947,25.5304,146.3521,3.1871,100
       305	process,64,25000,617.4920,17.2790,77.6238,2.1590,100
        40	data,128,100000,26091.3812,2397.0920,3261.8591,299.6376,100
        79	data,64,100000,7242.7070,999.3833,905.7737,124.9237,100

What the committed data supports: static (process) stamps at 128 parties grow about 7.5 B per doubling of the iteration count along rows 264-270 (131.88 B at 5,623 to 146.35 B at 25,000), so extrapolating 5.3 doublings to 1,000,000 gives a stamp near 185 B, not a 100 B version; the population exponents from the 64 to 128 endpoints are about 0.91 static (77.62 to 146.35) and about 1.85 dynamic (905.77 to 3261.86), which is consistent with "linearly" and "roughly N²" but is not what the paragraph cites. The deleted formula, evaluated at the one point the committed run can check (N = 128, E/N about 195 if each process-regime iteration records one event), gives about 105 B for the version alone against a measured stamp of 146 B, so restoring the formula as written would not close the gap either.

Resolution: re-denominate the paragraph against the committed run (populations 4-128, the paper's two regimes, stamp sizes at the committed iteration counts, the two fitted exponents), or commit the run that produces the quoted figures (extend `examples/space_consumption.rs` with a party/version split and a 100-party, 1,000,000-event checkpoint, regenerate `space.csv`) and let the prose cite those rows. `build.rs` already binds the figure to `results/` (lines 93 and 194); the prose numbers have no such binding. Acceptance: every number in the paragraph is traceable to a row of a committed CSV or to a formula stated beside it with its validity band; `README.md` regenerates to match.

### paper-fidelity-2: "100× more space-efficient than a naïve transcription" has no committed measurement and no stated denominator
- Where: crates/before/src/lib.rs:1-6 (related: crates/before/README.md:5-8, crates/before/results/space_consumption/README.md:27-34, crates/before/results/benchmarks/README.md:5)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (grep for `100×|100x|naïve|naive` across `results/`, `examples/`, `benches/`, `tests/`, `src/lib.rs`; `results/space_consumption/README.md` read in full); executed: no
- Verification: confirmed; history: no-rationale-found (a431eaf1, 2026-08-04, "Doc editing", introduces the sentence)
- Owner-gated: no

The headline ratio is measured nowhere in the tree. The one committed comparison against the paper (`results/space_consumption/README.md`) shows rough parity with the paper's own Appendix A bit encoding; a reader who takes "transcription of the original paper" to mean the paper's encoding therefore finds the tree contradicting the sentence, and the plausible intended denominator (the paper's recursive trees held in memory, that is the oracle's boxed representation) is measured by nothing: `results/benchmarks/` measures time only.

Evidence:

         3	//! using a compact representation which is approximately 100× more
         4	//! space-efficient than a naïve transcription of the original paper, while

    results/space_consumption/README.md:
        31	| Data, 100k iters   |        128 |                ~3262 B  | "< 2900 B" (chart ~3000–4000) |
        33	| Process, 25k iters |        128 |                 ~146 B  | "slightly above 170 B"      |

Resolution: name the denominator in the sentence and commit the measurement behind the ratio (for example, have `examples/space_consumption.rs` also report the oracle tree's heap footprint per stamp, or add a compactness test asserting the ratio band on the committed families), or drop the ratio and cite the committed Appendix A comparison. Acceptance: the sentence's ratio and its denominator are both produced by a committed artifact the sentence can name.

### paper-fidelity-3: "asymptotically linear performance" contradicts the roster's own documented bounds
- Where: crates/before/src/lib.rs:5-6 (related: crates/before/src/lib.rs:350-353, crates/before-fuelscape/src/ops.rs:331,360,450,630,1251,2146, crates/before/src/version.rs:293)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (every `contract:` string in `crates/before-fuelscape/src/ops.rs` listed by grep, 105 rows; the cited rows read in context); executed: no
- Verification: confirmed; history: no-rationale-found (same commit a431eaf1 as finding 2)
- Owner-gated: no

The front page promises asymptotically linear performance over all inputs, and lines 351-353 make every asymptotic claim a hard guarantee. The roster documents superlinear bounds for the rank family, the n-ary folds, text rendering, projection materialization, and query conjunction.

Evidence:

         5	//! maintaining asymptotically linear and practically quick performance even
         6	//! over the most adversarially pessimal inputs.
       351	//! pathological input shapes. Any asymptotic claim is a hard guarantee that the
       352	//! operation will perform in time proportionate to that bound, for all input
       353	//! sizes, no matter how unlikely and contorted the shape of the input.

    crates/before-fuelscape/src/ops.rs:
       331	        contract: "superlinear, subquadratic time; `O(|self|)` space",
       360	        contract: "`O(M(|self|) · log |self|)` time, `O(|self|)` space",
       450	        contract: "the view is `O(1)`; materializing: `O(|self| + |result|)`, `|result| = O(|self|^2)`",
       630	        contract: "`O((|self| + |iter|) log k)` time, `k` the operand count",
      2146	        contract: "linear, plus one comparison per opposite-side hole pair: `O(|self| · |rhs|)` at worst",

Resolution: qualify the headline to what the roster supports: linear on the core operations (tick, join, meet, compare, fork, codec), a `log k` factor on the n-ary folds, `M(n) · log n` on the rank family, quadratic output on projection materialization, with each operation's `# Complexity` section as the contract of record. Acceptance: no sentence on the front page states a bound that any `contract:` row in the roster exceeds.

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

### paper-fidelity-5: fold.rs's partner-size clause is false in the crate's byte-size denomination
- Where: crates/before/src/fold.rs:5-8 (related: crates/before/src/lib.rs:346-348, crates/before/src/fold.rs:41-81, crates/before-fuelscape/src/ops.rs:630)
- Class / severity / confidence: claim / low / high
- Provenance: verified (the counter in `balanced_try_fold` traced by hand on the construction below); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The module doc asserts that no combine's operand exceeds its partner's size by more than a bounded factor. The counter pairs groups of equal input *count*; input *size* (encoded bytes, lib.rs:346-348) is unconstrained. With inputs `[H, t, t, ...]`, `|H|` one megabyte and `|t|` one byte, the first combine pairs `H` (weight 0) with `t` (weight 0), and every level above pairs the `H`-bearing group with a group of tiny inputs. The clause is not needed for the bound: what the counter gives is that each input participates in at most `⌈log₂ k⌉ + 1` combines, each linear in its operands; the `O((|self| + |iter|) log k)` contract then also needs the combined output's packed size to stay within a constant of its operands' total, which is stated nowhere I read (open question 1).

Evidence:

         5	//! An incoming operand merges upward while the top stack entry holds as many
         6	//! inputs as it does, so every input passes through `O(log k)` combines against
         7	//! similarly sized partners and no combine's operand is more than a bounded
         8	//! factor larger than its partner. A sequential left fold instead combines

Resolution: replace the partner-ratio clause with the per-input participation bound, and state (or cite) the output-size bound the roster contracts rest on. Acceptance: the paragraph's every clause is true under the crate's byte-size denomination, and the `log k` contract's two premises are both stated.

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

### paper-fidelity-8: the oracle's grow defines two arms the paper does not, undocumented at the oracle
- Where: crates/before/src/oracle/version.rs:268-289 (related: crates/before/reference/itc2008.md:536-543, crates/before/src/version/skyline/grow.rs:42-48, crates/before/src/testing/grow_brute_force.rs:55-58, crates/before/src/oracle/tests.rs:556-559, crates/before/src/oracle.rs:22-25)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (the oracle's arms read against the paper's five `grow` equations; grow.rs and grow_brute_force.rs read at the cited lines); executed: no
- Verification: confirmed; history: already-known at the impl (grow.rs:47-48 asserts the arm unreachable) and the brute force (grow_brute_force.rs:57), not at the oracle
- Owner-gated: no

The paper's `grow` (lines 536-543) has no `grow(1, (n, el, er))` and no `grow(0, e)`: `fill(1, e) = max(e)` collapses any fully owned event node before `event` reaches `grow`, and the `i ≠ 0` precondition plus the `(0, ir)`/`(il, 0)` arms keep `grow` off empty ids. The oracle adds both arms with a one-line doc comment, while `oracle.rs:22` makes transcription fidelity the module's purpose, and `grow_cost_is_globally_minimal` drives `grow` over arbitrary `(id, e)` pairs, so the arms are load-bearing for the brute-force pins.

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

### paper-fidelity-12: "PartialEq describes a causal ordering" should read PartialOrd
- Where: crates/before/src/lib.rs:277-279 (related: crates/before/src/version.rs:58-60, crates/before/README.md)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (both sites read); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The crate docs attribute the causal ordering to `Version`'s `PartialEq`; the ordering is `PartialOrd`, and `PartialEq` is canonical byte equality, as version.rs:58 says correctly.

Evidence:

       277	//! [`Version`]'s [`PartialEq`] describes a causal ordering: `a <= b` tests
       278	//! containment of history, and two versions with no containing order are

Resolution: replace `PartialEq` with `PartialOrd`; regenerate the README. Acceptance: the sentence names the trait that defines `<=`.

### paper-fidelity-13: "mint" used for constructing values across the crate
- Where: crates/before/src/lib.rs:49 (related: crates/before/src/party.rs:15,872, crates/before/src/version.rs:1624, crates/before/src/laws.rs:2956, crates/before/src/version/skyline.rs:7, crates/before/src/testing/validation_index.rs:142; 56 sites in all, the densest in `version/skyline/query/web.rs` (8), `version/skyline/watermark.rs` (7), `meter/board/family.rs` (6))
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rn '\bmint' crates/before/src --include='*.rs' | wc -l` = 56; the cited sites read); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The vocabulary rule ("never write mint for constructing a value") is breached at 56 sites, including the Quickstart doctest a first-time reader sees.

Evidence:

        49	//! // New participants fork off a live clock, never mint themselves.

    crates/before/src/version/skyline.rs:
         6	//! spanning a dyadic interval of width `2^-depth` (the `overlay` machinery
         7	//! module's cursor vocabulary mints the term). Topology plus absolute leaf

Resolution: one pass replacing the verb with the plain one at each site (create, build, coin for vocabulary, set for a threshold). Acceptance: the grep count is zero.

### paper-fidelity-14: the results README's CSV column list omits the bit columns the file carries
- Where: crates/before/results/space_consumption/README.md:8-9 (related: crates/before/results/space_consumption/space.csv:1, crates/before/examples/space_consumption.rs:49-51)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (CSV header and both docs read); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The README lists six columns; the committed header has eight, and the example's `# Output` section names all eight and says why the bit column is the one to fit.

Evidence:

         8	- `space.csv` — raw measurements (100 runs, paper parameters). Columns:
         9	  `scenario,entities,iteration,mean_bytes,std_bytes,runs`.

    space.csv:
         1	scenario,entities,iteration,mean_bits,std_bits,mean_bytes,std_bytes,runs

Resolution: update the column list or point at the example's `# Output` section. Acceptance: the README's list equals the CSV header.

## Positives

- The oracle transcribes §5 faithfully on every definition I read line by line: `leq` is the paper's lift form threaded through offsets (oracle/version.rs:100-112), `join_off` and `meet_off` are the absolute-offset form (114-154), `fill` has the paper's six cases in the paper's order with `(1, ir)` before `(il, 1)` (246-263 against itc2008.md:513-518), and `grow`'s tie-break is the paper's (`cl < cr` goes left, else right; 275-285 against 541-543), using the lexicographic `(expansions, depth)` pair the paper's own closing paragraph sanctions in place of the `N`-weighted integer.
- `grow_dominates_no_more_than_needed` (oracle/tests.rs:609-643) explains exactly why the literal §3 clause `x < e′ ⇒ x ≤ e` is false over the full pointwise lattice and pins the correct scoped reading; that is the right way to transcribe an informally stated paper property.
- The brute-force witness (`testing/grow_brute_force.rs`) is genuinely independent of the DP (full enumeration, no saturation), and the oracle documents its one production coupling (`RouteCost::deepen`, oracle/version.rs:14-22) with the reason.
- The skyline canonicality argument (version/skyline.rs:65-90) re-derives: minimal topology is exactly the paper's normal form, gamma and zigzag are bijections with no negative-zero spelling, so byte equality is semantic equality.
- suanpan's sign-fold bound (`2.01 · 2^(32·i)`, stop at `|s| ≥ 3`; lib.rs:103-113) and the zero-run ledger's credit argument (150-161) hold as written and are stated with their potentials.
- The function-space oracle (testing/semantic_oracle.rs) is a real third reference: it shares no tree recursion with impl or oracle, draws random §4-valid inflations and partitions per call, and names its one concession (bisecting an indivisible piece).
- The laws and the party tests already know that `join_all` hands back coalesced groups (laws.rs:2402-2416; party/tests.rs:95-105) and convict the dropped-group variant; the fold policy is adequacy-pinned, only the public prose lags (finding 7).
- The results README (space_consumption/README.md:27-41) compares this crate's encoding to the paper's Appendix A figures honestly, including the regime where the crate lands inside the paper's band rather than below it.

## Open questions for Finch

1. Where is the packed-size bound the `O((|self| + |iter|) log k)` fold contracts need argued: that a join's (or meet's, or span's) output stays within a constant of its operands' total encoded size? I did not find it in `fold.rs`, `version.rs`'s `join`/`join_all` docs, or `skyline.rs` lines 1-130 (`emit.rs` and `tests/meter.rs` were read only by grep). Without it the counter argument alone gives per-input participation, not the stated bound. This also determines finding 5's resolution.
2. lib.rs:333-340 claims auxiliary space "at most a small constant multiple of the input size" for any operation; `tests/meter.rs` pins heap peaks per committed family (for example line 7589). Is that claim scoped to the committed families, or is there an instrument covering shapes nobody chose?
3. version.rs:293 says unbounded-integer multiplication is "about O(n log n) in this implementation". `Cargo.lock` resolves `dashu-int 0.5.0`, whose NTT dispatch is disabled only under `target_pointer_width = "16"` (mul/mod.rs:27-32), so the claim holds on wasm32 today; the same file's comment says "unavailable on 16/32-bit word targets", so a bump deserves a one-line re-check of the `cfg`.
4. The oracle's `Cost` steps depth through the production `RouteCost::deepen` (oracle/version.rs:8, 20-22), documented at grow.rs:137-142 as the one function every DP implementation shares. Is that coupling acceptable to you given oracle.rs's independence framing, or should the oracle carry its own saturating step and let the differential compare the two?

## Dropped

None. Every seed finding survived on the cited text; finding 1 was reframed by the `git log -S` history (the figures are outputs of a deleted closed-form estimate), finding 4 gained a second site (`version/skyline/build/tests.rs:394`), and findings 10 and 11 are carried at medium confidence because their cost is a reader's, not a caller's.
