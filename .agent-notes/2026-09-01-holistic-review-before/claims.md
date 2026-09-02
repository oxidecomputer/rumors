# The claims ledger: asymptotics, space, canonicality, derivations, paper fidelity

This document collects every finding of the `before` and `suanpan` review whose primary class is `claim`: a quantitative or semantic promise (an asymptotic bound, a space figure, a canonicality statement, a derivation, a "pinned by" attribution) that the code, the committed instruments, or the ITC paper does not back as written. It also carries the paper-fidelity sweep's account of the oracle transcription, its deviations, and the laws' relation to the paper's algebra. No finalized finding carries the primary class `paper-fidelity`: the sweep's fourteen findings were classified as `claim` (four, reproduced here in full), `documentation` (eight), `idiom` (one), and `verification-gap` (one); the ten non-claim entries are cross-referenced by id in the paper-fidelity section and live in the documents of their own classes. Ids are `<partition or sweep key>-<n>`; the full record of each lives in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md` beside this file, and the raw constructed-test results in `evidence/witness/results.md`. Severity is `high` (a public contract the code contradicts on a production path, or an instrument whose claim would mislead triage), `medium` (a public or instrument claim the code does not deliver, with a bounded cost), `low` (a false or unbacked sentence with no operational consequence today), `nit` (a notation or wording slip). Provenance: *demonstrated* means a constructed test ran and settled the claim; *executed* means a run settled it; *verified* means mechanically checked (grep, git, arithmetic, a transcribed algorithm) or re-derived; *assessed* means read. Each entry reproduces the finalizer's record; where a constructed test exists I add a `Constructed test` line summarizing `results.md`, and where I judge the record needs a correction I add a `Synthesis note` rather than editing it.

Two cross-cutting facts frame every entry below. `crates/before/src/lib.rs:350-353` makes every asymptotic claim "a hard guarantee that the operation will perform in time proportionate to that bound, for all input sizes", and `lib.rs:333-340` makes auxiliary space "at most a small constant multiple of the input size" with any violation "a bug": under that framing a false `# Complexity` sentence is a contract breach, whatever the likelihood of the input. And `crates/before/tests/meter.rs:7-9` says, in the same present tense, "Today's implementation is far from that — several operations amplify their input by large constants or worse": the two statements cannot both be true, and the constructed tests in this document side with the meter header on at least five operations (`Version::join`, `Ranked::cmp`, `OwnVersion` comparison, `Query::coverage`, `tick` on the memo families). The crate-wide patterns below state how the two sentences reconcile.

## Highest-value items

1. `Version::join` on a flat wide leaf against a left spine of two-leaf pairs does Θ(depth × width) work on Θ(depth + width) input: the builder's cascade amortization prices each copy against its deletion but never prices the re-flush of a re-anchored wide code one level up. Demonstrated through the public operator: scan per input bit 43.8, 86.5, 171.8, 342.5 as depth doubles. The published contract is `O(|self| + |other|)` (skyline-coding-9).
2. `Ranked::cmp` publishes `O(|self| + |other|)` while `rank_cmp` runs the same mass-balanced settle as `rank`, `distance`, and `lag`, whose contract is `O(M(|self|) · log |self|)`. Demonstrated for the claim's core: at every scale `Ranked::cmp` does the rank fold's work, 0.77× `rank`'s limb operations and about 1.08× its wall time in the same run, while the `partial_cmp` control does a fraction of either. The superlinear order stands by reading (the settle is the M-bound product tree at integral.rs:1142-1143): the deterministic limb meter read 0.989, linear, because it prices a multiplication by its operand limbs and is structurally blind to the term, and the construction's wall exponent of 1.14 to 1.28 was taken in a dev build under undisclosed load, so it is wall time, not a measurement of the order (rank-33).
3. The masked comparison (`OwnVersion`'s production path) re-scans a parked cursor's trailing right-branch run on every step of the other operand: `block_skip` evaluates `peek_flip`, which reads one word per 64 bits of the run, without moving. Demonstrated: spilled words read grow ×4.000 on a ×2.000 input, exactly n·r/64; no committed meter counts a stack-word read (skyline-sweep-place-masked-5, codec-bits-29).
4. `Query::coverage`'s `Partial` refinement sweeps the clamped endpoint once per surviving hole, Θ(k · |hi|) against a published `O(|self| + |span|)`; the fused membership and coverage traversals pay a further factor of the hole count per elementary interval that the public "linear time" wording and `filter.rs`'s own cost derivation omit. Demonstrated: 56 added holes add 578,644 scanned bits (56.3 × |hi|) to `coverage`; touches halve when k halves on a fixed probe; every committed instrument builds at most one hole (span-causally-36, span-causally-24, skyline-sweep-place-masked-21).
5. Three prose sites say a new operator impl "fails that gate until ... a family row here ... is added"; `surfacecheck` never reads `FAMILY_SURFACE`, and the census already holds `Default for Version`, the `Cow<Version>` conversions, and `error::Overlap`/`TooWide` behind no family row (surface-roster-7).
6. The crate's front page promises "asymptotically linear" performance and its own hard-guarantee paragraph makes that a contract; the roster documents `O(M(|self|) · log |self|)` for the rank family, `log k` for the folds, "superlinear, subquadratic" for rendering, and `O(|self| · |rhs|)` for conjunction, and the meter suite's header says the implementation is "far from" linear. The constructed rows in this document settle it for five operations: the header's "far from" is true of `Version::join`, `Ranked::cmp`, the masked comparison, `Query::coverage`, and `tick` on the memo families today, so the hard-guarantee sentence is the stale side until each is fixed or named in `lib.rs` as an exception (crate-root-25, paper-fidelity-3, crate-root-40; the crate-wide pattern below states the resolution).
7. The front page's "approximately 100× more space-efficient than a naïve transcription" and the "100 parties and 1,000,000 events" figures name no committed measurement; the committed run (`results/space_consumption`) covers populations 4 to 128 and reports parity with the paper's Appendix A coding. A constructed measurement gives about 62× against heap-boxed trees and 0.99× against Appendix A, so the sentence is true only under a referent it does not name (crate-root-24, paper-fidelity-2, crate-root-29, paper-fidelity-1).
8. `tick` on the distinct-minima memo families holds one `SuspendedLevel` (two `Accumulator`s) per open site-nesting level and one `Accumulator` per nonzero link; the crate promises transient space at a small constant multiple of the input and the board enforces 16 B/B. Demonstrated: 104.7 and 98.6 B/B on the memo comb, 53.6 and 50.5 B/B on the distinct chain, flat across a doubling; no committed heap meter reads these families (skyline-fill-grow-2).
9. `Rank`'s `Sum` claims a summand raising the maximum exponent pays "the exponent the summand itself carries"; the rescale costs the held width. Demonstrated: a wide rank followed by 1/2, 1/4, ... in ascending order costs 2,019,782 touches against 20,897 for the same summands descending (×96.7), and both committed pins fix the benign order while naming it the adversary (rank-20).
10. The `O((|self| + |iter|) log k)` fold contracts rest on two premises: the counter's per-input participation bound, which `fold.rs` mis-states as operand-size balance, and the join/meet output-size (subadditivity) lemma, which is derived only in test prose and promised by no public contract, though rumors's mirror window budgets on it (crate-root-17, paper-fidelity-5, version-core-5).
11. `Ranked::encode_rank` is documented at four sites as a fused emission cheaper than `rank().encode()`; it has been `rank().encode()` in three lines since the commit that introduced it, and the `raw_parts`/`encode_parts` boundary is justified by a mechanism that does not exist. Demonstrated: identical limb counts and bytes on seven versions (rank-32). Beside it, the bold "never is larger than the version it measures" fails in bits at small scale (rank-4).
12. The board's `MAX_SCALING_EXPONENT = 1.15` is documented as excluding "a real log factor at these input sizes"; a transcribed `judge::trend` fits an n·log n kernel at 1.07 to 1.10 on every committed ladder, and the envelope suite's ×1.25 one-doubling flatness band admits exponents up to about 1.32 while its docs say "linear". The instruments' own claims about what they exclude are the ones a maintainer relies on for triage (board-frame-8, envelopes-a-16).

## Crate-wide patterns

- **Summary claims outrun their per-operation contracts.** The crate front page ("asymptotically linear"), the `causally` module summary ("each pass and walk is linear"), the fuzz-fit sentry ("every public operation"), the validation index ("every instrument"), the atlas ("the adversarial frontier"), and `lib.rs`'s Testing paragraph ("every operation is verified differentially against" both references) each state a universal that the roster, the island contracts, or the code beneath them qualifies. The careful statement usually already exists one level down (crate-root-25, span-causally-25, fuzzfit-bands-27, testing-oracles-28, fuelscape-pipeline-1, crate-root-31).
- **Per-level re-work hidden inside "linear".** Six independent findings share one mechanism: a traversal re-copies, re-reads, or re-folds something proportional to the current level or the live set once per level or per interval, and the amortization argument prices only the first pass. The tuple-literal composers for `Party`, `Version`, and `Clock` copy and re-validate every subtree per nesting level (party-11, codec-base-text-tree-13, version-core-16, skyline-coding-20, clock-14); the builder re-flushes a re-anchored wide code per cascade level (skyline-coding-9); the render merge re-adds a wide `span` per spine level (skyline-coding-29); `peek_flip` re-scans a parked run per step (skyline-sweep-place-masked-5, codec-bits-29); the filter walks read every live pair per elementary interval (skyline-sweep-place-masked-21, span-causally-24); the coverage refinement re-sweeps the clamped endpoint per hole (span-causally-36); `sum_ranks` rescales the held width per exponent raise (rank-20).
- **The committed instrument measures the benign case.** Where a claim is false, the instrument that should have caught it drives the shape on which it is true: `join` envelopes exercise the absorb face, never a re-anchor (skyline-coding-9); the `rank_sum` pins fix high-first order (rank-20); every query instrument has at most one hole (span-causally-24, -36); the masked-hole band's spine carries zero deltas (skyline-sweep-place-masked-4); the `ranked_cmp` island's overlay omits exactly the settle-firing families the `version_rank` island includes (rank-33); the `hoisted_window` band runs at a width where the promotion it exists to price cannot fire (meter-core-8).
- **Meter blind spots decide what can be claimed.** The limb meter prices a multiplication by its operand limbs, so the `M(n)` term reads linear by denomination (rank-33 constructed test); the scan meter counts cursor advances and decodes, not accumulator reads or stack-word reads, so the k factor and the `peek_flip` term are invisible to it (span-causally-24, codec-bits-29); the stack-segments column has no writer in any binary that judges it (crate-root-32, board-frame-1, envelopes-a-2, meter-core-11, recursion-1, in their own classes). A claim whose only instrument cannot see its failure mode is, for the doctrine's purposes, uninstrumented.
- **Flag-day residue in cost prose.** The 2026-07-25 change that made the skyline the stored coding (faf3cd0a) left present-tense sentences describing the earlier state: the meter suite header (crate-root-40), the tier-2 and compactness prose (meter-registry-tier2-14, testing-diff-gen-17, other classes), the "two levels per fork" grid rationale (testing-oracles, other classes). The claims here that expired the same way are marked `deliberate-but-expired` in their history lines.
- **The hard-guarantee sentence and the meter header, reconciled.** `lib.rs:350-353` makes every asymptotic claim a hard guarantee for all input sizes, and `tests/meter.rs:7-9` says the implementation is "far from" that; both are in the present tense. The five demonstrated rows in this document, `Version::join`'s re-anchor cascade (skyline-coding-9), `Ranked::cmp`'s settle (rank-33), the masked comparison's `peek_flip` term (skyline-sweep-place-masked-5), `Query::coverage`'s per-hole sweep (span-causally-36), and `tick`'s memo-family heap (skyline-fill-grow-2), are today's exceptions to the guarantee, so for those operations the header's sentence is true and the guarantee is the stale side; what is stale in the header is its account of *why* (recursion frames, quadratic path sums, a transcoding decoder), which the pinned rows beside it refute (envelopes-a-1 and prose-hygiene-1, documentation). The board cannot settle which sentence is stale, because its committed families are the shapes on which the claims hold (the *committed instrument measures the benign case* pattern above), so a green `just amp-board-acceptance` is consistent with all five breaches. The owner has since ruled (owner ruling 1, 2026-09-02, `triage/rulings.md`) that the guarantee is the contract as intended and holds absolutely: the five rows are defects to fix, never exceptions to declare or models to adopt; the meter header may name them as under repair but must not assert that the contract holds today or that the excess is accepted; and for each, the instrument (an envelope row on the breaching shape, and a board family where the operation has one) lands before the fix so the breach reads as a failure first. The entries below (crate-root-40; envelopes-a-1 in the documentation document; README owner-decision items 12, 21, and 22) carry that disposition.
- **Numbers without artifacts.** "100×", "3 bytes", "100 bytes", "2,000 bytes", "~40 KB", "×1.17", "2 EiB", "order of magnitude", "by measurement": each is a figure in prose that no committed run, CSV, or test reproduces (crate-root-24, -29, fuelscape-render-33, envelopes-b-4, rank-10, codec-bits-2, party-21). The paper-fidelity sweep traced the space figures to two closed-form estimates a same-day commit deleted while keeping their outputs (paper-fidelity-1). `MinWeb::compacting`'s ×1.41 heap and ×2.0 touch ratios are the same pattern at a declaration site: measured under a manual swap that no committed constructor reproduces, and cited by `.cargo/mutants.toml:74-75` as the adequacy evidence for a mutant exclusion (skyline-watermark-8, documentation, owner-gated).

## The claims table

One row per claim examined across the partition reports and the paper-fidelity sweep. "Argument" asks whether a derivation exists where the claim lives (or is cited from there); "Impl" whether the implementation delivers the claim as stated; "Instrument" names the committed check and what it actually pins. Rows marked backed are the positives a maintainer can rely on; the rest have entries below.

| Claim | Where stated | Argument | Impl | Instrument (what it pins) | Verdict |
|---|---|---|---|---|---|
| ~100× more space-efficient than a naïve transcription | lib.rs:3-4 | no | unmeasured | none; `results/space_consumption` compares against Appendix A (parity) | unbacked; constructed run: 62× vs boxed trees, 0.99× vs Appendix A (crate-root-24, paper-fidelity-2) |
| "asymptotically linear ... performance" over all inputs | lib.rs:5-6 | no | no: rank `O(M·log)`, folds `log k`, render superlinear, conjoin `n²` | asymptotics pins hold the log factors alive (`*_log_factor_is_alive`) | contradicted by the crate's own contracts (crate-root-25, paper-fidelity-3) |
| Party ~3 B, Version ~100 B at 100 parties / 10⁶ events; 50 B and 2,000 B under churn | lib.rs:312-319 | deleted formulas `⌈ln N/2⌉`, `⌈N/2 + N·log₂(E/N)/24⌉` | unmeasured | `space.csv` runs 4..128 entities, whole stamps only | unbacked (crate-root-29, paper-fidelity-1) |
| every asymptotic claim is a hard guarantee for all sizes | lib.rs:350-353 | n/a (meta-claim) | no (rows below) | board exponent ≤ 1.15, heap 16 B/B on board families | contradicted by tests/meter.rs:7-9 and by five demonstrated rows (crate-root-40) |
| auxiliary space ≤ small constant × input | lib.rs:333-340 | per operation | partly | board heap ceiling on board families; envelope heap rows on committed families | contradicted on memo families, 50-105 B/B (skyline-fill-grow-2); scope open (paper-fidelity q2) |
| every operation verified differentially against both references | lib.rs:407-410 | n/a | no: 193 `Leg::Excluded` rows | surface roster | overstates (crate-root-31) |
| skyline canonical form; byte equality is semantic equality | skyline.rs:65-90 | yes, re-derived | yes | strict reject corpus, mutation re-derivation through the oracle bridge, `fuzz_decode` byte identity, exhaustive bijection pins | backed, with two qualifications: the admission walk's mid-stream collapsible-pair rejection has no committed witness through `Span::decode` (skyline-coding-6, verification, demonstrated: with `zero_delta` forced false a non-canonical `hi` is accepted while 227 tests stay green), and `fuzz_decode` runs only at `just all` cadence, outside the gate (fuzz-guests-pins-38) |
| no library traversal recurses on input depth | AGENTS.md; recurse.rs:9-14 | yes | yes (call-graph scan) | `deep_tree_*` at depth 100k; envelope rows at 250k | backed (recursion sweep) |
| fold: "similarly sized partners ... bounded factor" | fold.rs:5-8 | wrong (count balance, not size) | no | fold ceiling rests on per-level partition | false clause (crate-root-17, paper-fidelity-5) |
| join/meet output ≤ inputs' encoded sizes (subadditivity) | test prose only (tier2/tests.rs:328-346) | yes, in tests | yes | `*_encoding_is_subadditive*`, tier2 pins; 128 pairs re-checked | pinned but promised nowhere public; folds' space bound and rumors depend on it (version-core-5) |
| n-ary fold operations `O((|self| + |iter|) log k)` | ops.rs:630 etc. | counter + missing size premise | yes as far as pinned | 5 of 11 fold operations pinned; `party_join_all` margin 1.9% | partly (testing-diff-gen-28; roster totality is testing-diff-gen-23, other class) |
| shape walks: "nothing allocates" / "nothing is materialized up front" | party.rs:480-481; shape.rs:22-25 | no | no: `BitStack` spills at 65 levels | none | contradicted; 64 B at depth 200 (party-9, crate-root-37) |
| `Version::new` clone identity across calls | version.rs:124-128 | static-not-const | yes (unpinned) | none | unpinned (version-core-1) |
| `shape` folding cost dismissed by "realistically reachable" versions | version.rs:742-748 | likelihood | n/a | the file's own doctest builds an 87-bit rise | argument invalid under the crate's own rule (version-core-9) |
| `Version`/`Clock` tuple literal `O(m)`, `O(n)` | version.rs:1486-1488; clock.rs:960-962 | no | `O(m·d)`: per-level rescan | none | constant hides depth (version-core-16, skyline-coding-20, clock-14) |
| `Ticks` `Sum` `O(N)` | ticks.rs:41-42 | no | `Θ(N + k)` | none | omits per-summand term (version-core-23) |
| `Party::covers`/`is_disjoint` "no allocation" | fuelscape ops.rs:821, 835 | no | no: `Lockstep::pending` is a `BitsBuf` | envelopes pin heap 8 B (×1.25 = 10) | contradicted by the crate's own pin (party-1) |
| `Party` tuple literal `O(n)` | party.rs:897-899 | no | `Θ(n·d)`: copy + `validate_id` per level | none | quadratic in depth; scan ×3.83 at bytes ×1.89 (party-11, codec-base-text-tree-13) |
| `IdIndex` table "strictly smaller than the operand" | index.rs:43-44 | no | no: up to 8× larger | none | false for every B ≥ 1 (party-22) |
| `join_all` fold contract at every size | index.rs:53-57 vs ops.rs:806 | fallback documented at field | `Θ(k·|self|)` past 2³² bits | `build_unindexed` differentials | size clause missing from public contract (party-23) |
| `IdIndex::is_disjoint` `O(Σ inputs + B log n)` | index.rs:15-17 | yes, but not the code's | no: searches fire on left-only nodes | scan floors calibrated on inflated readings | contradicted; 3,595 discarded probes at B = 0 (party-25) |
| index searches "tie or win wall time ... by measurement" | index.rs:22-24 | n/a | n/a | no bench-judge cell for party ops | unbound measurement (party-21) |
| `forks` yields exactly `k` | clock.rs:166; clock/forks.rs:9 | no | no: `u64::MAX − 1` at `k = u64::MAX` | `tests/forks_max.rs` pins the opposite | contradicted at one input (clock-3, tests-other-17) |
| rank "never is larger than the version it measures" | rank.rs:198-202 | constant-factor only | no | provenance pin at five fixed scales | false in bits at small scale, tied in bytes (rank-4) |
| `NotCanonical` needs 2 EiB of input | rank.rs:442-444 | conflates trigger and bound | no: 9 bytes reach it | genre test at version/tests.rs:1167-1170 | false (rank-10) |
| `sum_ranks` rescale "paid by the exponent the summand carries" | rank.rs:1011-1018 | no | no: `shl` is `O(held digits)` | `RANK_SUM_MIXED`, board `rank_sum` (high-first only) | false for ascending order; ×96.7 (rank-20) |
| wasm32 pins hold arm placement | num.rs:28-34 | n/a | pins assert values | past-capacity pins bound the ceiling from above only | over-describes (rank-23) |
| `encode_rank` fused, "more efficient", "without materializing" | ranked.rs:46-49, 194-196; version.rs:1051, 1073 | no | no: `rank().encode()` by another name | none | fiction; identical limb counts (rank-32) |
| `Ranked::cmp` `O(|self| + |other|)` | ops.rs:1350; ranked.rs:372-378 | lost at b5a81583 | no: runs the settle, `O(M(n)·log n)` | uniform fuzz-fit band on a roster without the settle families; board cell with adequacy floors | contradicted; wall exponent 1.14-1.28 (rank-33) |
| `version_join`/`meet`/`span` `O(|self| + |other|)`; cascade "amortized O(1) per output bit" | ops.rs:571; build.rs:29-34 | omits the re-flush | no: `Θ(d·W)` | `SKYLINE_JOIN_ABSORB`, board twins (absorb face only) | contradicted; per-bit scan doubles per doubling (skyline-coding-9) |
| transcoder "priced by the packed stream it reads" | skyline.rs:119-121 | no | `Θ(d·b)` on Bigroot | none (test/meter only) | overstates (skyline-coding-2) |
| capacity overrun "costing one reallocation" | emit.rs:295-297 | no | doubling buffer | heap envelopes | unargued (skyline-coding-16) |
| render "superlinear, subquadratic" / "n log n" | ops.rs:331-332 | no | quadratic term `s·⌈s/64⌉` | `render_merge_superlinearity_is_alive` (class exists), exponent ceiling 1.96 | class misstated; ratios 3.27, 3.56 (skyline-coding-29) |
| `fill` heap: "never an accumulator per open site-nesting level" | fill.rs:121-134 | scoped to `PreFrames` | `SuspendedLevel` holds two per level | no heap reading on memo families | instrument gap; 50-105 B/B (skyline-fill-grow-2) |
| integral shifts cannot panic because "the storage caps below 2^32" | integral.rs:464-471; web.rs:136-141 | false premise | conclusion holds on the bound the codec states | none | premise contradicted by bits.rs:117-119 (skyline-query-13) |
| dashu tier thresholds 24 / 96 / 4,000 | query/tests.rs:944-948; integral.rs:220 | literals | true today | value tests, not boundary tests | unanchored to the dependency pin (skyline-query-28) |
| masked cost `O(|v| + |p| + |w|)` "every path bit pushed and popped at most once" | masked.rs:52-56 | omits non-popping peeks | no: `Θ(L·r/64)` | `masked_cmp_*` rows, hole band (amortized case) | contradicted; ×4 on ×2 (skyline-sweep-place-masked-5, codec-bits-29) |
| masked-hole band: "accumulator work is a function of the mask depth alone" | masked.rs:313-316 | zero-delta premise unstated | flat only because skipped deltas are zero | `masked_cmp_hole_depth_band` | misattributed mechanism (skyline-sweep-place-masked-4) |
| `O(2|v| + |s| + |e|)` vs `O(|v| + |s| + |e|)` | place.rs:91-92 | n/a | n/a | `span_place_scans_each_stream_once` (correct form) | vacuous notation (skyline-sweep-place-masked-14) |
| filter walks `O(|v| + Σ|bound|)` | filter.rs:37-48 | acknowledges `O(#bounds)` per interval, drops it | no: `Θ(k·(|v| + Σ|bᵢ|))` | rows with ≤ 2 bounds | contradicted; ×3.96 on k doubling (skyline-sweep-place-masked-21) |
| polarity restriction guarantees "linear time" | causally.rs:85-86; query.rs:28-30; polarity.rs:211-212 | bits-decoded only | k sign reads and folds per interval | every instrument has ≤ 1 hole | true in bits, false in time for k ≥ 2 (span-causally-24) |
| `causally`: "each pass and walk is linear" | causally.rs:105-106 | no | conjoin island says `O(|self| · |rhs|)` | conjoin island | self-contradictory (span-causally-25) |
| `Query::coverage` `O(|self| + |span|)`, "at most two traversals" | query.rs:114-117; islands | exactness argued, cost not | `Θ(k·|hi| + Σ|holeᵢ|)` | one-hole panels only | contradicted; 56.3 × |hi| (span-causally-36) |
| `BitStack`: "every operation is O(1)" | stack.rs:13-17 | no | `all_set`, `trailing_ones` scan | none | false (codec-bits-27) |
| `with_capacity`: past-address-space hint "allocates nothing up front" | buf.rs:68-78 | no | `unwrap_or(0)` never fires on 64-bit | none | overstates (codec-bits-10) |
| memcmp rung "roughly an order of magnitude cheaper per bit" | bits.rs:22-29 | no | n/a | identity fast-path rows pin zero walk work, no ratio | unmeasured number (codec-bits-2) |
| suanpan: "a nonzero partial decides within one step" | suanpan lib.rs:143-146; accumulator.rs:838-842 | false in general | true over a zero digit (what the conclusion needs) | the `witnesses.rs` corners | false intermediate step (suanpan-4) |
| suanpan: amortized O(1) word deltas, O(limbs) wide, "on every input sequence" | suanpan lib.rs:1-3, 25-29 | sign fold and ledger derived; lazy zone asserted | yes as pinned | exact touch pins at canonical schedules | partly backed; potential supplied in paper-fidelity-9; random-stream pin absent (suanpan-2, other class) |
| suanpan touch counts exact, a public contract | suanpan lib.rs:283-286 | yes | yes | `metered.rs` exact two-scale pins; claims roster | backed |
| rank wire form: byte order equals `Ord`; prefix-free; header bijective | rank.rs:8-131 | yes, checkable | yes | `ranked_encoding_orders_like_ord`, 0-2 byte exhaustive sweep, goldens | backed |
| `Rank::cmp` O(1) class test then MSB windows, zero allocation | rank.rs:882-906 | yes | yes | alignment-oracle sweeps, `RANK_TRIPLE` laws | backed |
| `BACKEND_CAPACITY_BITS` = dashu `Buffer::MAX_CAPACITY` | num.rs | yes | yes | wasm32 past-capacity pins | backed |
| placement fusions cost composed minus one probe decode | tests/meter.rs:9471-10071 | relational identity | yes | `span_place_scans_each_stream_once` and siblings, with liveness reads | backed |
| `eq` early exit is tail-independent | tests/meter.rs:4983-5150 | yes | yes | two-scale absolute pin; mutant recorded | backed |
| masked-hole comparison is depth-independent | tests/meter.rs:7515-7544 | yes (with the zero-delta premise) | yes | `masked_cmp_hole_depth_band` `lo == hi` | backed as an order pin (premise: skyline-sweep-place-masked-4) |
| fork orbit closed form `7 + 2·⌊log₂ k⌋` | clock/tests.rs:1218-1474 | yes (3 topology bits + gamma(0) + gamma(2k)) | yes | orbit pins with liveness floors | backed |
| deep spine marginal cost is three bits per level | version/tests.rs:2375-2402 | yes | yes | closed-form two-scale pin | backed |
| id covers/disjoint scan `2·(2d + 2)` exactly | tests/meter.rs:6297-6300 | yes | yes | two-sided exact equality at two depths | backed |
| pool recycle ceiling 2 from peak simultaneous demand | tests/meter.rs:9399-9411 | yes | yes | `pool_recycle` equality across churn doubling; kills retire/lease mutants | backed |
| compactness comb closed forms; factor-2 envelope tight (> 1.994) | compactness.rs:131-166 | yes | yes | tightness test (ratio > 1.994) | backed (prose tense is testing-diff-gen-17, other class) |
| fuelscape samplers are exact-uniform | sample.rs:289-292, 473-489 | yes (weights partition the recurrences) | yes | table vs enumeration to 24 bits, decoder set equality, chi-square | backed |
| the oracle transcribes §5 (`leq`, `join_off`, `fill`, `grow` tie-break) | oracle/version.rs:100-154, 246-263, 275-285 | yes (line-by-line against itc2008.md) | yes | differential suites; worked examples with section citations | backed (deviations: paper-fidelity-8, oracle-laws-2) |
| `MAX_SCALING_EXPONENT = 1.15` excludes a log factor at these sizes | ceilings.rs:64-69 | no | n·log n fits 1.07-1.10 | quadratic tripwire only | false exclusion (board-frame-8) |
| board ceilings are "class-scale" | validation_index.rs:103-105 | expired at c0b5d701 | touch, κ, fold-scan, family constants are worst ×1.25 | n/a | stale map (board-frame-26) |
| dead touch meter trips on every committed pair family | floors.rs:474-478 | conditional half correct | hugeleaf pair declares NA | none | universal false (board-families-floors-judge-14) |
| flatness band: cost "is linear" | tests/meter.rs band docs | ×1.25 per doubling ⇒ exponent ≤ 1.32 | n/a | kernels at ×1.5-×2 caught; `O(n^1.2)` not | overstates (envelopes-a-16) |
| settle level ratio "at most ×1.17" | tests/meter.rs:5830-5836 | formula gives 1.5 / 1.33 at committed n | n/a | band holds by non-dominance | wrong number (envelopes-b-4) |
| touch floor `input / 8` from stated premises | tests/meter.rs:8384-8410 | premises compose to `input / 64` | holds by margin | floor asserted | derivation does not reach the constant (envelopes-b-20) |
| `wide_arming` "one ledger arming" for `w ≥ 10` | meter.rs:1885-1912 | rationale describes `w ≥ 18` | no promotion for 10 ≤ w ≤ 17 | `hoisted_window` band at w = 12 (zero promotions) | contradicted (meter-core-8) |
| registry reason strings name enforcement homes | registry.rs:1108-1109, 1629-1664 | n/a | named sites hold no such pin | n/a | misdirecting (meter-registry-tier2-10) |
| surface-totality gate fails until a `FAMILY_SURFACE` row is added | surface.rs:1010-1015 and two siblings | n/a | `surfacecheck` never reads `FAMILY_SURFACE` | census pins impls only | false; orphans already exist (surface-roster-7) |
| `auto_traits.rs` covers every public type | extract.rs:267-270 | n/a | thirteen types missing | compile-time pin (partial) | premise expired; `PhantomData<*const ()>` compiles clean (surface-roster-23) |
| asymptotics floors: "a crossing is a class change, never noise" | asymptotics.rs:12-14 | determinism ≠ attribution | n/a | `party_join_all` floor 1.9% from both endpoints | overstates (testing-diff-gen-28) |
| validation index maps every instrument | validation_index.rs:1-3 | n/a | ≥ 7 committed instruments unrowed | none holds the page total | overstates (testing-oracles-28) |
| verdict-matrix pool "at the smallest committed-valid knobs" | verdict_matrix.rs:151-161 | n/a | registry records no floor | none | unpinned (tests-other-28) |
| fuzz-fit bands price "every public operation" | enforce.rs:441-443; lib.rs:4-5; justfile:569-570 | n/a | 44 kernels banded of ~100 measured exports | no tiling against `METHOD_SURFACE` | overstates (fuzzfit-bands-27) |
| guest: every nonzero return is a harness bug | fuzzfit guest lib.rs:21-24 | expired at f66d7c17 | `ERR_OP` is a priced outcome | `rejected: true` bands | contradicted (fuzz-guests-pins-16) |
| "PINNED AS FOUND ... a cure must move" | wasm32-pins tests/pins.rs:4-10 | n/a | three pins describe the terminal as intended | the pins themselves | genre unsettled (fuzz-guests-pins-33) |
| the atlas overlay shows "the adversarial frontier" | before-fuelscape lib.rs:13-14 | n/a | 18 of 68 shapes by signature; board-pinned worst families absent | none | overstates (fuelscape-pipeline-1) |
| `COMBINE_ARITY_CAP` kept equal to the guest's by the smoke test | before-fuelscape ops.rs:198-203 | n/a | smoke runs at 8 bytes; no test compares | none | claims a check that does not exist (fuelscape-pipeline-28) |
| injected header is "~40 KB" | justfile:251-253 | n/a | 77,988 bytes | none | drifted (fuelscape-render-33) |
| island bounds "in total input bytes" | build.rs:227-235 | n/a | datasets carry per-op `size_measure` | none | wrong denominator for rank and text rows (crate-root-6) |
| counting allocator's overhead "identical across arms" | benches/presize.rs:37-39 | no | `peak_alloc::realloc` copies; growth arm pays | none | biased A/B (benches-examples-13) |
| results/benchmarks record | results/benchmarks/README.md | dated 2026-06-02 | benches renamed/removed; mechanism wrong | none | stale record (benches-examples-25) |

## Findings by module

Entries reproduce the finalizers' records verbatim; ids and anchors are those of `evidence/`. A `Constructed test` line summarizes the record in `evidence/witness/results.md`; a `Synthesis note` records a correction or a cross-reference without editing the record.

## Crate root and public types

### Crate root (`lib.rs` and its derived README)

### crate-root-24: "Approximately 100× more space-efficient than a naïve transcription" has no committed measurement
- Where: crates/before/src/lib.rs:3-4 (related: crates/before/results/space_consumption/README.md:27-41, crates/before/examples/space_consumption.rs, crates/before/src/oracle.rs)
- Class / severity / confidence: claim / medium / medium
- Provenance: verified (grep for `100×|100x|naïve transcription|naive transcription` over crates/before/{src,examples,results,benches,tests}: only lib.rs matches; results/space_consumption/README.md:29-34 compares against the paper's Appendix A encoding and reports parity to modest wins); executed: no
- Seen by: prose, claims; refutation: confirmed; history: no-rationale-found (a431eaf1d, the owner's edit, replaced 67970b75c's provenance-tagged "between 2–20× faster (measured: the workspace bench suite ...)" with this figure and no artifact)
- Owner-gated: yes (the owner's own headline; a measurement may exist off-tree)

A number you were handed is a hypothesis: this is the first number a reader sees, and nothing in the tree produces it. The referent is also undefined: if "naïve transcription" means the `oracle` module's boxed trees, no program measures their footprint; if it means the paper's own Appendix A encoding, the committed table says parity, not two orders of magnitude.

Evidence:

     3  //! using a compact representation which is approximately 100× more
     4  //! space-efficient than a naïve transcription of the original paper, while

    results/space_consumption/README.md:
    31  | Data, 100k iters   |        128 |                ~3262 B  | "< 2900 B" (chart ~3000–4000) |

Resolution: Either commit the measurement (extend `examples/space_consumption.rs` to record the oracle values' in-memory or boxed-node encoded size beside `encode().len()` at each checkpoint, and quote the observed ratio with its scenario) or drop the multiplier and say what the README supports. Acceptance: the figure is reproduced by a committed example or CSV the docs name, or is absent; `just readme` regenerated.
Construction: Compute both sizes for one population: `Clock::encode().len()` against the oracle tree's size under any naive spelling (boxed nodes with u64 counts, or the paper's Appendix A bits). No committed program does this, so the ratio is unchecked; if it is not about 100 at the paper's parameters the sentence is false as written.

Constructed test: demonstrated (results.md lines 2142-2234). On the paper's data scenario at 128 entities for 10,000 iterations, the mean stamp is 1,791.7 B; a boxed-tree transcription (32/16/24/8-byte nodes) is 111,631 B (62.31×); the paper's Appendix A coding, reconstructed from the agent's recollection rather than a committed artifact, is 1,776 B (0.99×). The sentence is true only if "naïve transcription" means heap-boxed trees, and false if it means the paper's own encoding.

Synthesis note: The constructed measurement gives the entry's construction a number: about 62× against heap-boxed trees and 0.99× against Appendix A, the same order as 100× under one reading and a contradiction under the other. The finding's resolution (name the denominator, commit the measurement) is unchanged; the Appendix A tally in the run is recollected, not committed, and cannot itself be cited.

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

Synthesis note: Same claim as crate-root-24 from the sweep's side; kept as filed because its evidence (the Appendix A parity table and the absent denominator) is the sweep's own. The constructed measurement under crate-root-24 applies.

### crate-root-25: The headline promises "asymptotically linear" performance; the crate's own contracts are superlinear for rank, text I/O, the render merge, and the folds
- Where: crates/before/src/lib.rs:5-6 (related: crates/before/src/lib.rs:350-358, crates/before/src/version.rs:293, crates/before/src/version.rs:1058, crates/before/src/version/ticks.rs:41-48, crates/before/fuelscape/version_rank.json, crates/before/src/testing/asymptotics.rs:1-12)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (python3 read of fuelscape/version_rank.json: `op.claim` = "n (log n)^2", `op.contract` = "`O(M(|self|) · log |self|)` time, `O(|self|)` space"; version.rs:293 and :1058 "`M` is the complexity of unbounded-integer multiplication (about `O(n log n)` in this implementation)"; ticks.rs:42-43 "text I/O is superlinear but subquadratic"; asymptotics.rs:5-7 names the log factor, the render merge's superlinear growth, and the settle's multiplication-bound case as live pins); executed: no
- Seen by: prose, claims; refutation: confirmed; history: no-rationale-found (the headline is a431eaf1d and the hard-guarantee paragraph bbb9f8023, both the owner's hand; the superlinear contracts were already in the tree)
- Owner-gated: yes (the owner's own headline; the intended meaning, probably "linear for the core operations", is not stated)

Lines 351-353 make every asymptotic claim a hard guarantee, so the headline is itself a guarantee, and it is one the crate's asymptotics suite holds alive counterexamples to. The careful statement already exists in the per-operation `# Complexity` sections; the opening sentence overstates it.

Evidence:

     5  //! maintaining asymptotically linear and practically quick performance even
     6  //! over the most adversarially pessimal inputs.
    ...
   351  //! pathological input shapes. Any asymptotic claim is a hard guarantee that the
   352  //! operation will perform in time proportionate to that bound, for all input
   353  //! sizes, no matter how unlikely and contorted the shape of the input.

    version.rs:
   293      /// Typical inputs run far below the worst case; `M` is the complexity of unbounded-integer multiplication (about `O(n log n)` in this implementation).

Resolution: Reword to the bound the crate guarantees, for example "while every operation carries a documented, guaranteed time bound: linear in the encoded input for the core operations (tick, fork, join, comparison, the codecs), near-linear for the n-ary folds, and multiplication-bound only where the answer itself is a wide integer (rank and its relatives)"; then `just readme`. Acceptance: the opening paragraph names no bound any `# Complexity` section exceeds; the words "asymptotically linear" do not stand as a crate-wide promise while `fuelscape/version_rank.json` carries contract `O(M(|self|) · log |self|)`.
Construction: Textual: the rendered docs for `Version::rank` state `O(n (log n)^2)` in total input bytes on the same page set whose front page states "asymptotically linear"; the instruments that would fail a literal linear claim are already committed and green (`render_merge_superlinearity_is_alive`, `version_join_all_log_factor_is_alive`).

Constructed test: demonstrated textually (results.md lines 3852-3882). The headline, the hard-guarantee paragraph, and the committed `version_rank` contract `O(M(|self|) · log |self|)` coexist on one doc set; `party_join_all_log_factor_is_alive` ran green in the same session, holding a log factor above linear.

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

Synthesis note: Same claim as crate-root-25; the sweep verified it across all 105 `contract:` rows and names the projection-materialization and conjunction contracts crate-root-25 does not.

### crate-root-29: The space-efficiency figures name parameters no committed measurement uses
- Where: crates/before/src/lib.rs:312-319 (related: crates/before/results/space_consumption/README.md:8-34, crates/before/results/space_consumption/space.csv, crates/before/examples/space_consumption.rs:9-22, crates/before/build.rs:93-99)
- Class / severity / confidence: claim / medium / high
- Provenance: verified; executed: yes (python3 csv parse of results/space_consumption/space.csv: 444 rows; columns `scenario, entities, iteration, mean_bits, std_bits, mean_bytes, std_bytes, runs`; scenarios {data, process}; entities {4, 8, 16, 32, 64, 128}; max iteration 100000 for data and 25000 for process; no row at 100 entities or 10^6 events, and no party/version split)
- Seen by: claims, prose; refutation: confirmed; history: no-rationale-found (67970b75c introduced the figures tagged "(measured: the space-consumption experiment that draws the figure below)", which even then ran the paper's parameters; a431eaf1d removed the tag and added "steady-state")
- Owner-gated: yes (the owner's paragraph; a run at these parameters may exist off-tree)

The prose quotes sizes at 100 parties and 1,000,000 events (Party about 3 B, Version about 100 B) and under churn (about 50 B and 2,000 B, "linearly in N" and "roughly N²"), directly above a figure derived from an artifact that runs populations 4..128 at 100k/25k iterations and reports whole-stamp sizes (about 146 B static and 3262 B dynamic at 128). The N² reading is plausible (2,000 × (128/100)² is about 3,277 against the measured 3,262) but is an inference the reader cannot check, and the 10^6-event denominator has no committed source.

Evidence:

   312  //! At 100 parties and 1,000,000 events, the expected size of a [`Party`] is
   313  //! about 3 bytes and the expected size of a [`Version`] is about 100 bytes.
    ...
   316  //! bounds. Under sustained random membership churn, those same 100 parties will
   317  //! each stabilize at around 50 bytes (growing linearly in the steady-state
   318  //! number of parties `N`) and their corresponding versions at around 2,000
   319  //! bytes (roughly `N²` in the steady-state number of parties `N`).

    results/space_consumption/README.md:
     8  - `space.csv` — raw measurements (100 runs, paper parameters). Columns:

Resolution: Re-denominate the paragraph to the committed artifact's parameters and quantities (stamp bytes at 128 entities after 100k/25k iterations, static versus dynamic), or commit the run that produces the 100-party / 10^6-event figures (the example takes the population and iteration budget) and cite it; state the N and N² fits as slopes over the committed columns with their band. Acceptance: every number in lib.rs:312-319 is readable from results/space_consumption/space.csv (or a newly committed CSV) at the parameters the prose names, and the growth laws are stated as fits over committed columns.

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

Synthesis note: Same paragraph as crate-root-29; the sweep adds the history (the figures are outputs of deleted closed-form estimates) and the arithmetic showing the deleted formula would not close the gap either.

### crate-root-40: The meter suite's header says the implementation is "far from" the contract the crate docs state as a hard guarantee
- Where: crates/before/tests/meter.rs:7-9 (related: crates/before/src/lib.rs:350-358, crates/before/src/meter/board/ceilings.rs:64-73)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (both texts read; ceilings.rs:64-69 judges every cell at exponent at most 1.15 and :71-73 at 16 heap bytes per input byte, the crate docs' position; which side is true is not settled here, since the board was not run); executed: no
- Seen by: claims (and flagged by correctness for the envelopes reviewer); refutation: confirmed; history: deliberate-but-expired (the header is from 4348636693, 2026-07-22, while the amplification campaign's V1-V6 were open; the campaign cured them (C2, P4.2, P5) and the dated-notes excision swept dates but left this sentence; the owner's bbb9f8023 then wrote the hard-guarantee paragraph)
- Owner-gated: no

Two present-tense statements of the crate's central promise cannot both be true. This file sits outside the listed partition files but is the counterpart of lib.rs:350-358, which is in it; per the history, the meter header is the stale side.

Evidence:

     7  //! on value magnitude, tree depth, or encoded size. Today's implementation
     8  //! is far from that — several operations amplify their input by large
     9  //! constants or worse — so every scenario here pins the *current* measured

    lib.rs:
   351  //! pathological input shapes. Any asymptotic claim is a hard guarantee that the
   352  //! operation will perform in time proportionate to that bound, for all input
   353  //! sizes, no matter how unlikely and contorted the shape of the input.

Resolution (as ruled, owner ruling 1, 2026-09-02, `triage/rulings.md`): the hard-guarantee sentence is the contract and stays as written; the five operations this document demonstrates over their documented bounds (`Version::join`, skyline-coding-9; `Ranked::cmp`, rank-33; the masked comparison, skyline-sweep-place-masked-5; `Query::coverage`, span-causally-36; `tick` on the memo families, skyline-fill-grow-2) are defects to fix. Restate the header as regression pins on measured costs under the documented bounds, naming the operations still over their bound as under repair (with their finding ids) and dropping the recursion-and-transcode narrative; never as exceptions, and never asserting that the contract holds today. Acceptance: `lib.rs` names no exception; the header's list of operations under repair is exactly the set of this document's demonstrated rows not yet fixed, and it empties as the fixes land; the meter suite's module doc attributes every gap it names to a listed operation.
Construction: Textual; `just amp-board-acceptance` (the exponent and heap legs) is the committed instrument that settles which side is true: a red cell sides with the meter header.

Constructed test: inconclusive (results.md lines 2235-2272). Both present-tense texts were confirmed verbatim; the settling instrument the Construction names, `just amp-board-acceptance`, was forbidden to the reviewers and, as the crate-wide patterns state, could not have settled the question, because its committed families are the shapes on which the claims hold, so a green board is consistent with the five breaches.

Synthesis note: the body's "per the history, the meter header is the stale side" is the history pass's reading of the amplification campaign that cured V1-V6; the constructed rows in this document overturn it for five operations (skyline-coding-9, rank-33, skyline-sweep-place-masked-5, span-causally-36, skyline-fill-grow-2), which are over their documented bounds today. The Resolution above is rewritten accordingly, from "decide which is true" to naming the exceptions; the Construction line is the finalizer's and is kept as written, with this note recording why the instrument it names cannot settle the question.

### crate-root-31: "Every operation is verified differentially against" both references overstates the roster
- Where: crates/before/src/lib.rs:407-410 (related: crates/before/src/surface.rs:72-115, crates/before/src/surface.rs Leg::Excluded rows)
- Class / severity / confidence: claim / low / high
- Provenance: verified (`grep -c 'Leg::Excluded' crates/before/src/surface.rs` = 193; the `Exclusion` enum at 72-115 names the families: no wire format in the references, definitional combinators, n-ary not in the references, linearity/borrowing mechanics); executed: no
- Seen by: claims; refutation: confirmed; history: deliberate-but-expired (the paragraph is from dee3cd61aa, before the surface roster existed; the roster later made it checkably false)
- Owner-gated: no

The roster is the crate's own record of which doors carry which legs; the crate docs should not claim more legs than it records. The accurate statement is stronger: differential where a reference exists, law- and battery-pinned where none can.

Evidence:

   407  //! Every operation is verified differentially against the paper's naive
   408  //! recursive implementation as well as a nondeterministic function-space
   409  //! semantics, alongside exhaustive small-scope enumeration of clock shapes,
   410  //! algebraic-law property suites, and fuzzed codecs.

Resolution: "Every operation with a counterpart in the paper is verified differentially against its naive recursive implementation and a nondeterministic function-space semantics; operations with no reference (the codecs, text, the n-ary folds, borrowing mechanics) are pinned on production by algebraic laws, round-trip and strict-rejection batteries, and format goldens; the `surface` roster records which leg each door carries." Acceptance: the Testing paragraph's quantifier matches the `Leg` variants the roster assigns; `just readme`.
Construction: Textual: list the surface.rs rows whose `prod_tree`/`prod_fs` legs are `Leg::Excluded(_)`; each is an operation the sentence claims is differentially verified against both references and is not.

### crate-root-6: Island summaries denominate every bound "in total input bytes" while the datasets carry a per-operation `size_measure`
- Where: crates/before/build.rs:227-235 (related: crates/before/build.rs:212-222, crates/before/docs/fuelscape.js:758, crates/before/docs/fuelscape.js:819, crates/before/fuelscape/rank_add.json, crates/before/fuelscape/party_fromstr.json)
- Class / severity / confidence: claim / low / high
- Provenance: verified (python3 read of `op.size_measure` in rank_add.json: "total packed bytes of the two versions whose ranks are added, split uniform (ranks derived by Version::rank in preparation)"; party_fromstr.json: "packed bytes of the sampled value, rendered to text by Display (the value measure pushed through rendering — not uniform ..."; fuelscape.js:758 shows `size_measure` only in the provenance tooltip and :819 hardcodes the x-axis caption); executed: no
- Seen by: claims; refutation: confirmed; history: already-known (the fuelscape note records the widget's uniform bytes denomination as a decision and, as open item 4, that `size_measure` should drive the x-axis caption, "currently hardcoded"; that item is unimplemented at fuelscape.js:819, and the summary and noscript strings here are two more sites of the same convention)
- Owner-gated: no

The visible summary and the no-JavaScript fallback both say "in total input bytes", but for the Rank rows the x-axis is the packed bytes of the versions the ranks were derived from (the inputs are ranks), and for the FromStr rows it is the packed bytes of the value the text renders (the input is text). The doctrine asks that every amplification claim state its denominator; the datasets do, and the two readers without the widget see a different one.

Evidence:

   227      format!(
   228          "<details class=\"toggle fs-details\"><summary>{variant}\
   229           <span class=\"fs-claim\"><code>O({claim_html})</code> \
   230           in total input bytes</span>; {contract}</summary>\
   231           <div class=\"fuelscape\"><script type=\"application/json\">{data}</script></div>\
   232           <noscript><p>The interactive chart requires JavaScript; the bound \
   233           is O({claim_html}) in total input bytes.</p></noscript>\
   234           </details>\n"
   235      )

Resolution: Give the widget data a short denominator phrase beside `size_measure` (or derive one from its first clause) and interpolate it into the summary and noscript strings; this is the same change the note's open item 4 wants for the x-axis caption, done once for all three readers. Acceptance: the rendered summary for `rank_add` names the versions' packed bytes and for `party_fromstr` the value's packed bytes; the constant phrase no longer appears in build.rs.
Construction: Open the rendered docs for `Rank::add` with JavaScript disabled: the noscript text reads "in total input bytes" while the dataset's x-axis is the packed bytes of two versions the ranks were derived from.

### Clock

### clock-3: `forks` publicly promises exactly `k` children; the count saturates at `u64::MAX`, and "keeps the last share" names the wrong position
- Where: crates/before/src/clock.rs:159-169 (related: crates/before/src/clock/forks.rs:9-12; crates/before/src/party.rs:239-249; crates/before/src/party/forks.rs:80-84, 105-118; crates/before/tests/forks_max.rs:1-11, 49-50)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read party/forks.rs:105-118: `k.saturating_add(1)` and the residual drawn as the first preorder leaf; read tests/forks_max.rs asserting `len() == u64::MAX - 1` for `Clock::forks(u64::MAX)`; `git log -S saturat` on the four doc sites lists cdad4606 adding the public sentences and a431eaf1, a6dcfbb4, b3f09baa removing them); executed: no
- Seen by: prose, correctness, claims; refutation: confirmed (severity medium to low proposed); history: no-rationale-found (the public sentences existed at cdad4606 and were removed by three doc-editing commits with no body)
- Owner-gated: no (restoring the sentence); changing the behavior instead would be an API change and owner-gated

The public docs on `Clock::forks` ("yields `n` children") and `Forks` ("Yields exactly `k` disjoint clocks") are false at one input: `party::Forks::new` reserves `k.saturating_add(1)` shares and draws one for the residual, so `forks(u64::MAX)` yields `u64::MAX - 1`. The integration test that pins this calls it "the documented behavior", but the only surviving statement is a private `//` comment in `party::Forks::new`. Correct at all scales, for all inputs applies to the contract text: a public promise contradicted at any input is a claim finding, whatever the input's likelihood. Secondarily, "`self` keeps the last share" is positionally wrong: the residual is the first preorder leaf of the `k + 1` split, which matters to anyone pairing `forks` output with the `[Clock; N]` preorder.

Evidence:

       166	    /// The iterator yields `n` children and `self` keeps the last share, so it
       167	    /// stays a valid clock even once the iterator is fully drained. Forks not

    (forks.rs)
         9	/// Yields exactly `k` disjoint clocks, each pairing one structurally balanced

    (party/forks.rs, private)
       107	        // share even once every yielded share has been consumed. The first
       108	        // preorder leaf becomes that residual — reaching it costs O(log k)
       111	        // The count saturates: at `k == u64::MAX` the residual's headroom is
       112	        // spent and the iterator yields one share fewer than asked.
       114	        let mut split = Split::new(whole, k.saturating_add(1));

    (tests/forks_max.rs)
         1	//! `forks(u64::MAX)`: the documented behavior at the split count's
         2	//! saturation boundary, pinned in both profiles.

Resolution: Restore one sentence to the public docs of `Clock::forks`, `Forks`, `Party::forks`, and `party::Forks`: the iterator yields `k` children except at `k == u64::MAX`, where the residual consumes the count's headroom and `u64::MAX - 1` are yielded. Reword "keeps the last share" to "keeps the first (residual) share of the balanced split". Point `tests/forks_max.rs`'s module doc at the public sentence. Acceptance: the four public doc blocks state the boundary and the residual's position; `Forks` no longer says "exactly `k`" unqualified; the pin's "documented behavior" resolves to a public sentence.
Construction: `let mut c = Clock::seed(); assert_eq!(c.forks(u64::MAX).len() as u64, u64::MAX);` fails on a 64-bit host with `u64::MAX - 1`, exactly what tests/forks_max.rs:49-50 asserts, against the `Forks` doc's "exactly `k`".

Constructed test: demonstrated (results.md lines 3782-3823). `forks(u64::MAX)` yields `u64::MAX − 1`, against `Forks`'s "exactly `k`" and `Clock::forks`'s "`n` children"; the residual's position was verified by reading only.

Synthesis note: Same boundary as tests-other-17 (the test file's side); both stand because the fix touches four public doc blocks and one test doc.

### clock-14: The literal door's `O(n)` inherits a per-level rescan that is quadratic in nesting depth
- Where: crates/before/src/clock.rs:960-962 (related: crates/before/src/version.rs:1486-1488, 1502-1509; crates/before/src/version/skyline/literal.rs:32-34, 82-95)
- Class / severity / confidence: claim / low / medium
- Provenance: assessed (read `node`: `scan(left)` and `scan(right)` materialize a `Vec<Base>` of every leaf height at every level; read the recursive `TryFrom<(u64, T, S)>` that calls it per node); executed: no
- Seen by: claims; refutation: confirmed (noting nesting depth is bounded by the compiler's recursion limit, which bounds the factor per program but not by the stated contract); history: no-rationale-found
- Owner-gated: no

`TryFrom<(I, E)> for Clock` claims `O(n)` in the built clock's size, composing `Version`'s `O(m)`. The version literal builds each child as a `Version` and then `skyline::literal::node` scans both child streams into `Vec<Base>` before re-emitting, so a left-deep literal of nesting depth `d` rescans `i` leaves at level `i`: `Θ(d²)` work for a `Θ(d)`-leaf output. The stated bound is loose by the nesting depth; an asymptotic claim is a hard guarantee for all inputs. The fix lives in the version partition; the clock door inherits whichever is chosen.

Evidence:

       960	/// # Complexity
       961	///
       962	/// `O(n)`, `n` the built clock's size in bytes.

    (version/skyline/literal.rs)
        32	pub(crate) fn node(base: u64, left: BitsView<'_>, right: BitsView<'_>) -> Result<BitsBuf, Parse> {
        33	    let (left_topology, left_heights) = scan(left);
        34	    let (right_topology, right_heights) = scan(right);
        82	fn scan(bits: BitsView<'_>) -> (BitsBuf, Vec<Base>) {

Resolution: Either restate both bounds as `O(m · d)` with `d` the literal's nesting depth, or have `node` fold children in one pass carrying only the running last height (no `Vec<Base>` per level) so `O(m)` becomes true. Acceptance: the doc names the depth factor, or a scan-meter test over a depth-doubling left-deep literal reads flat per built byte.
Construction: `Version::try_from((0u64, (0u64, (0u64, … (0u64, 0u64, 1u64) …, 2u64), 3u64), 4u64))` nested `d` deep; `node` at level `i` scans a left stream of `i` leaves, total `Σ_{i≤d} i = Θ(d²)` for a `Θ(d)`-leaf stream.

### Party

### party-1: `covers`/`is_disjoint` contracts say "no allocation"; the walk allocates and the crate's own envelopes pin the heap
- Where: crates/before-fuelscape/src/ops.rs:821-821 (related: crates/before-fuelscape/src/ops.rs:835, crates/before/src/party.rs:386, crates/before/src/party.rs:410, crates/before/src/party/ops/compare.rs:113-115, crates/before/src/party/ops/compare.rs:162-165, crates/before/tests/meter.rs:274-275, crates/before/src/codec/stack.rs:47-56)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read the contract strings, the `Lockstep::pending` field and its `push` sites, the `ID_COVERS`/`ID_DISJOINT` envelope constants, and `envelope()`'s first parameter being `peak_heap`); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (da0f6a937 pinned heap 8 on both rows; 7e63c1b96 and c6d8106a1 later wrote "no allocation" without reconciling)
- Owner-gated: no

The public `# Complexity` islands rendered into `Party::is_disjoint` and `Party::covers` promise no allocation, but `lockstep_holds` queues two bits per both-present ancestor pair on a `BitsBuf` (a heap-backed byte vector), and the committed envelopes for both operations pin a nonzero peak heap. A public space claim is a hard guarantee per the crate docs ("any violation is a bug"); this one is contradicted by the crate's own committed measurement. Moving the stack to `BitStack` (party-19) does not restore the claim: the register spills a word every 64 bits (stack.rs:49-53), so the accurate statement is a bound, not an absence.

Evidence:

       821	        contract: "`O(|self| + |other|)`, no allocation",
       835	        contract: "`O(|self| + |other|)`, no allocation",

       113	struct Lockstep {
       114	    /// Two presence bits per queued right child pair, innermost on top.
       115	    pending: BitsBuf,

       274	    pub const ID_COVERS: Envelope       = envelope(        10,        0,             0, 0); // iterative id walks
       275	    pub const ID_DISJOINT: Envelope     = envelope(        10,        0,             0, 0); // iterative id walks

Resolution: Restate both contracts as "`O(|self| + |other|)`; transient state is two bits per queued ancestor pair" (the same wording fits the board's `party_disjoint`/`party_covers` heap floors, which already treat heap as structurally near-zero rather than absent). Acceptance: the rendered `# Complexity` text on `Party::covers` and `Party::is_disjoint` no longer contradicts the `ID_COVERS`/`ID_DISJOINT` heap pins; no envelope or band moves.
Construction: the `IdSpine` divert pair already used by `id_covers_envelope`/`id_disjoint_envelope` (tests/meter.rs:6135-6161): one both-present node queues one right pair, allocating the `BitsBuf`'s byte vector; the pinned ceiling of 10 bytes is the measured 8 × 1.25. Any pair with a both-present node reaches the `push` at compare.rs:163-164.

Constructed test: demonstrated (results.md lines 3-63). On the diverted `IdSpine(1000)` pair, `covers` and `is_disjoint` each raise peak heap by 8 B under `PeakAlloc`; the unary control pair allocates 0 B, isolating the allocation to the queued right pair. Matches the `ID_COVERS`/`ID_DISJOINT` basis (10 = 8 × 1.25).

### party-9: `Party::shape` says "nothing allocates"; the walk's path stacks spill a word every 64 levels, and nothing prices the drain
- Where: crates/before/src/party.rs:480-481 (related: crates/before/src/shape.rs:204-209, crates/before/src/version/skyline/shape.rs:74-87, crates/before/src/version/skyline/overlay.rs:471-485, crates/before/src/codec/stack.rs:47-56, crates/before/src/meter/board/coverage.rs:617-621)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read the chain `Regions::of_party` -> `PartyWalk::open` -> `overlay::IdLeafCursor` with `path: BitStack`/`right_present: BitStack`, and `BitStack::push`'s `self.words.push(self.top)` at 64 bits; `grep -rn 'party_shape\|Party::shape' crates/before/tests/meter.rs` is empty and coverage.rs:617-621 lists the operation unpriced with a rationale); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (46eb64f97's design note intends allocation-free draining; the word spill was not considered)
- Owner-gated: no

A public space claim is a hard guarantee per lib.rs:350-358. The `O(|self|)` drain is right and argued (each tag read once, each path bit pushed and popped once), but "nothing allocates" is false for any party deeper than 64 levels (a 64-deep fork chain is a plain construction), and no envelope, board cell, or band prices the drain. `Version::shape`'s neighbouring sentence (skyline/shape.rs:22-25 region) already states the accurate form for its own walk.

Evidence:

       480	    /// Draining the iterator is linear in the party's encoded size: each
       481	    /// region costs `O(1)`, and nothing allocates.

        48	    pub(crate) fn push(&mut self, bit: bool) {
        49	        if self.top_len == 64 {
        50	            self.words.push(self.top);

Resolution: Restate: "each region costs amortized `O(1)`; transient state is two bits per open ancestor (one machine word per 64 levels), nothing per region." If the owner wants the drain priced, a `tests/meter.rs` sweep row over the `IdSpine` party (scan = every tag once, heap = the two stacks' words) is a closed-form pin. Acceptance: the doc states the bit-per-level transient; if a row is added, its scan reading equals the party's packed bits and its heap reading equals `2·⌈depth/64⌉·8` bytes plus allocator slack.
Construction: `shape_party(Shape::LeftSpine, 200)` (testing/generators.rs) drained under the `PeakAlloc` meter used by `tests/meter.rs`: two `BitStack` spills of one word each (`path` and `right_present`), so peak heap is nonzero.

Constructed test: demonstrated (results.md lines 1578-1622). Draining `Party::shape()` on the registry's 200-level id spine allocates 64 B of peak heap (two `BitStack` spills past 64 levels).

### party-11: The tuple-literal door is `O(n·depth)`, not `O(n)`: every nesting level re-copies and re-validates its subtree
- Where: crates/before/src/party.rs:897-899 (related: crates/before/src/party.rs:838-844, crates/before/src/codec/literal.rs:52-66, crates/before/src/codec/text.rs:75-84, crates/before/src/party.rs:794-800)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read `id_node`: it copies both children into a fresh buffer and calls `validate_id` over the assembled subtree; the tuple impl recurses per level; the text door validates once at the end); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (per-level validation since eecf92295; the `O(n)` line came later in 3bba6cbbc)
- Owner-gated: yes: the one-pass fix changes the signature of the `#[doc(hidden)]` sealed `PartyLiteral::into_id_bits`; the doc-fix alternative is not gated

The `# Complexity` section claims `O(n)` in the built party's bytes, but a `d`-deep literal costs `Σ O(subtree)` copies and full re-parses, `O(n·d)`. The per-level `validate_id` is also a recompute-and-compare on a construction whose two local checks (`(0, 0)` and `(1, 1)`) are the whole normal-form rule, and children are normal by induction, so it catches nothing the local checks miss (the doctrine on runtime asserts over deterministic pure functions). The depth is fixed by the tuple type, so no runtime input controls it; the claim is still false as stated.

Evidence:

       897	/// # Complexity
       898	///
       899	/// `O(n)`, `n` the built party's size in bytes.

        59	    let mut b = BitsBuf::with_capacity(2 + l.len() + r.len());
        60	    b.push(!l.is_empty()); // bit 0 = left present
        61	    b.push(!r.is_empty()); // bit 1 = right present
        62	    b.extend_from_buf(l);
        63	    b.extend_from_buf(r);
        64	    validate_id(super::buf::built_view(&b))?;

Resolution: Delete the `validate_id` call in `id_node` (keep the two local collapse checks; validate once in `finish_id` if a belt is wanted) and either restate the bound as `O(n·d)`, `d` the literal's nesting depth, or thread one `IdBuilder` through `PartyLiteral::into_id_bits` (reserve/patch/close per level, one pass, truly `O(n)`). Acceptance: a left-nested literal of depth `d` performs one validation pass in total (scan-meter reading about `2·nodes` bits, not a sum over levels); the `# Complexity` section states the bound the code implements; `parse_bare_notation` and the text round-trip laws stay green.
Construction: under `--features scan-meter`, build `Party::try_from(((((1u8, 0u8), 0u8), 0u8), 0u8))` and its depth-8 and depth-16 extensions, reading `scan_bits()` around each: level `i` copies and re-parses about `2i` bits, so the reading grows about quadratically in `d` for a `2d`-bit result where an `O(n)` door reads linearly.

Constructed test: demonstrated (results.md lines 3314-3385). Left-nested tuple literals at depths 4, 8, 16, 32 read scan bits 28, 88, 304, 1,120 for sizes 10, 18, 34, 66 bits: ×3.45 and ×3.68 per depth doubling against size ×1.89 and ×1.94.

Synthesis note: Same mechanism as codec-base-text-tree-13 (which owns the `validate_id` multiplier) and the version-side siblings version-core-16, skyline-coding-20, and clock-14; one reshaping of the composers fixes all five.

### party-21: Index doc claims a wall-time measurement no committed artifact holds
- Where: crates/before/src/party/ops/index.rs:22-24 (related: tools/benchjudge-expected.json)
- Class / severity / confidence: claim / nit / high
- Provenance: verified (`grep -c -i 'party\|join_all' tools/benchjudge-expected.json` is 0); executed: no
- Seen by: claims; refutation: confirmed; history: deliberate and holds as a measurement (29d3c8f27's message records the wall readings) but nothing in the tree binds it
- Owner-gated: no

"the searches tie or win wall time on every committed fold population, by measurement" names a measurement with no run to bind to: no bench-judge expectation covers a party operation. Principle 8 (a measurement binds to its run) and Principle 5 (a past measurement without an artifact rots silently). The asymptotic argument two sentences earlier is the justification the claim rests on, and it stands alone.

Evidence:

        22	//! (the overlap rows and the flatness pin in `meter::board`'s tests price that
        23	//! regression), while the searches tie or win wall time on every committed fold
        24	//! population, by measurement. Every probe records one table word in the scan

Resolution: Delete the clause, or add `party_join_all` to the bench-judge roster (`tools/benchjudge-expected.json`) and cite that cell by name. Acceptance: every measurement claim in the module doc names a committed instrument.
Construction: none needed beyond the grep: the claim names no artifact, and `tools/benchjudge-expected.json` has no party entry.

### party-22: `IdIndex` doc says its `u32` table is "strictly smaller than the operand itself"; it is up to eight times larger
- Where: crates/before/src/party/ops/index.rs:43-44 (related: crates/before/src/party/ops/index.rs:92, crates/before/src/party/ops/index.rs:189, crates/before/src/party.rs:324, crates/before/src/idbits.rs:5-10)
- Class / severity / confidence: claim / medium / high
- Provenance: assessed (arithmetic from the coding at idbits.rs:5-10: a both-present node's tag is 2 bits and each of its two present children is at least a 2-bit terminal, so `B` both-present nodes need at least `4B + 2` stream bits while the table is `32B` bits); executed: no
- Seen by: prose, correctness, claims; refutation: confirmed; history: no rationale found (a99e8c8f7's sentence, rewrapped by b3f09baa0, no derivation)
- Owner-gated: no

The sentence is a space statement in a transient-state sentence, so "smaller" means bits, and it is false for every `B ≥ 1`. The crate's promise that transient space is "a small constant multiple of the input size" (lib.rs:333-340) still holds, and `join_all`'s `O(|self| + |iter|)` auxiliary space (party.rs:324) still covers it, so this is a false sentence, not a space bug; a maintainer sizing the fold's peak from it is misled by nearly an order of magnitude. The 24-byte pending entries (party-24) belong to the same sentence if the per-node figure is meant to be the space envelope.

Evidence:

        41	/// Build once with [`build`](IdIndex::build), then run
        42	/// [`is_disjoint`](IdIndex::is_disjoint) against any number of independent
        43	/// operands. Transient state is at most one `u32` per both-present node of the
        44	/// indexed operand — strictly smaller than the operand itself.

Resolution: State the bound the code has: "one machine word (32 bits) per both-present node of the indexed operand: up to eight times the operand's bit length (a both-present node and its forced subtree cost at least four bits), `O(|self|)` and freed with the fold", and add the pending stack's per-level cost if the sentence is meant as the space envelope. Acceptance: the smallest witness (`(1, (1, 0))`, 8 bits of operand, one 32-bit entry) satisfies the stated bound; no prose in the partition claims the index is smaller than its operand.
Construction: `Party::try_from((1u8, (1u8, 0u8)))` encodes as `11 00 10 00` (8 bits) with one both-present node; `IdIndex::build` allocates `vec![0u32; 1]` (32 bits). The left comb `L_0 = (1, 0)`, `L_{k+1} = (L_k, 1)` is `4d + 4` bits with `d` both-present nodes, so the table is `32d` bits and the ratio approaches 8.

Constructed test: demonstrated (results.md lines 1623-1704). `(1, (1, 0))` is 8 encoded bits against a 32-bit table; on the left comb `L_d` the ratio reads 4.000, 6.400, 7.529, 7.877, 7.969 at `d` = 1, 4, 16, 64, 256.

### party-23: Past 2^32 packed bits the fold index falls back to the quadratic discipline; the public bound carries no size clause
- Where: crates/before/src/party/ops/index.rs:53-57 (related: crates/before/src/party/ops/index.rs:73-75, crates/before/src/party/ops/index.rs:156-159, crates/before/src/party/ops/index.rs:174-178, crates/before/src/party/tests.rs:373-470, crates/before-fuelscape/src/ops.rs:806, crates/before/src/lib.rs:350-353)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read `build`'s early return, `is_disjoint`'s fallback arm, the island contract string, and lib.rs:351-353 "for all input sizes, no matter how unlikely"); executed: no
- Seen by: claims; refutation: confirmed; history: the fallback is deliberate and documented at the field from birth (a99e8c8f7) and pinned via `build_unindexed` (522705cff); no record weighs it against the public contract
- Owner-gated: yes: a documented design decision, and the alternative is a public-contract clause

When the accumulator exceeds `u32::MAX` bits, `build` returns no table and `is_disjoint` cursor-walks the fixed side per input, making `join_all` `Θ(k·|self|)` against the public island contract `O((|self| + |iter|) log k + (|self| + |iter|) log |self|)` and the crate's all-sizes guarantee. A 512 MiB party is materializable, so this is not the tolerated 2^64-iteration corner. The `build_unindexed` test-only constructor and the two fallback differentials exist only to hold the arm the contract does not admit.

Evidence:

        53	    /// `None` when the stream's positions do not fit `u32` (≥ 512 MiB of packed
        54	    /// id); [`is_disjoint`](IdIndex::is_disjoint) then falls back to the
        55	    /// per-input cursor walk, which answers the same predicate at the fold's
        56	    /// unindexed cost.
        57	    rights: Option<Vec<u32>>,

       806	        contract: "`O((|self| + |iter|) log k + (|self| + |iter|) log |self|)` time, `k` the operand count",

Resolution: Owner's call between keeping the order at every size (`enum Rights { Narrow(Vec<u32>), Wide(Vec<u64>) }` chosen by `bits.len()`, or `Vec<u64>` unconditionally if the board's `party_join_all` heap reading tolerates it; measure at the parent first), dissolving the fallback arm, `build_unindexed`, and the fallback differentials or converting the latter to a wide-table differential, or carrying the size clause in the island contract and `join_all`'s rustdoc. Acceptance: either `is_disjoint` has no unindexed arm and the deep and arbitrary index differentials pass against a forced wide table, or the public contract names the threshold.
Construction: any accumulator over 2^32 bits with `k` one-byte overlapping probes reads `Θ(k·|self|)` scan bits under the current code where the contract predicts `Θ(k log |self|)`; unaffordable to materialize in a test (as the field doc says), which is why the finding is a contract clause rather than a failing test.

Constructed test: inconclusive (results.md lines 3386-3413). An accumulator over 2³² packed bits is not materializable within the test cap; the fallback paths (`rights: None` at `index.rs:73-75`, the cursor walk at 174-178) were confirmed by reading.

### party-25: `IdIndex::is_disjoint`'s stated `O(Σ inputs + B log n)` bound does not describe the code: a table search runs in the left-only arm and its result is discarded
- Where: crates/before/src/party/ops/index.rs:220-247 (related: crates/before/src/party/ops/index.rs:15-17, crates/before/src/party/ops/index.rs:279-291, crates/before/src/meter/board/ops.rs:1370-1377, crates/before/src/party/tests.rs:1130-1162)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read the arm order: the `if al && ar` search runs before `match (bl, br)`, and in the `(true, false)` arm `a_right`/`right_entry` are never read; the module doc defines `B` as "the inputs' both-present node count"; the construction below was traced by hand against the match arms); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (the arm structure is a99e8c8f7's from birth; 29d3c8f27 priced searches per input both-present node without noting the discarded case)
- Owner-gated: no

The module doc prices one `O(log n)` search "per node both sides own", with `B` the inputs' both-present node count. The implementation runs `metered_partition_point` whenever the indexed node is both-present and the input node is `Internal`, regardless of the input's presence bits, so searches fire on pairs the doc's `B` does not count and the code pays for a result it throws away. The argument and the implementation disagree in both directions, and the discarded searches inflate the scan-currency readings the board's fold cells and the parity-halves floor are calibrated against.

Evidence:

        15	//! costs `O(input)` node visits plus one `O(log n)` table search per node both
        16	//! sides own — so the fold's up-front tests total `O(Σ inputs + B log n)`, `B`
        17	//! the inputs' both-present node count. The search term is not bounded by the

       220	                            let (a_right, right_entry) = if al && ar {

       244	                                (true, false) => {
       245	                                    (node, entry) = (a_left, left_entry);
       246	                                    continue;
       247	                                }

Resolution: Compute the right-child position lazily: move the `if al && ar { … metered_partition_point … }` block into the `(true, true)` and `(false, true)` arms (or a closure invoked only there), leaving `(true, false)` at `O(1)`; then restate the bound with `B` defined as the visited pairs whose indexed node is both-present and whose input node has a right child. Acceptance: under `scan-meter` the construction below records zero 32-bit probes; `indexed_disjointness_matches_the_cursor_walk[_deep]`, `unindexed_fallback_matches_the_walk_on_constructed_pairs`, and the `join_all` oracle differentials stay green; `indexed_disjointness_search_bits_stay_metered` still passes (on the parity halves every input node is both-present); any board or envelope reading that moves is measured at the parent and attributed.
Construction: indexed operand `a` = the left comb `L_0 = (1, 0)`, `L_{k+1} = (L_k, 1)`, `a = L_d` (`d` both-present nodes whose right children are terminals). Input `b` = `d` left-only nodes over one right-only node over a terminal, which owns a cell inside the one region `a` leaves unowned, so the pair is disjoint. At depths `0..d-1` the pair is (both-present indexed node, left-only input node): the current code runs one `metered_partition_point` per level and discards it; `b` has zero both-present nodes, so the doc's bound predicts zero searches. Wrap `IdIndex::build(a).is_disjoint(IdReader::root(b))` in `scan::reset()`/`scan_bits()` under `scan-meter` and subtract the cursor walk's reading on the same pair: the difference is `d` searches' worth of 32-bit probes today and must be zero once the search is lazy.

Constructed test: demonstrated (results.md lines 4980-5048). With an input `b` of zero both-present nodes, the indexed walk records 13 / 264 / 3,595 32-bit probes at `d` = 8 / 64 / 512 beyond its tag reads (38× the cursor walk's 3,080 bits at `d` = 512), where the module doc's bound predicts none.

### Version core

### version-core-1: `Version::new`'s cross-call clone-identity claim has no pin
- Where: crates/before/src/version.rs:124-128 (related: crates/before/src/party.rs:122-129; crates/before/tests/meter.rs:10746-10797; crates/before/src/codec/tests.rs:183-188)
- Class / severity / confidence: claim / nit / high
- Provenance: verified (grep for `ptr_eq`/`from_static` over version/tests.rs, codec/tests.rs, clock/tests.rs, party/tests.rs, tests/meter.rs: no test compares two `Version::new()` or two `Party::seed()` results by pointer); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale for the missing pin (the static choice itself is deliberate, 1bb610d19)
- Owner-gated: no

The comment stakes a constant-factor claim (two independent `new()` calls answer `ptr_eq`) on `static` versus `const` promotion semantics, and nothing committed observes it. The standard for asymptotic and constant claims asks for a committed instrument; `codec/tests.rs:183-188` pins that two independently frozen *empty* `Bits` alias, a different mechanism (the zero-byte dangling pointer, not this one-byte static), and the identity fast-path meter rows drive `Version::new()` against a value, never two empties against each other.

Evidence:

       124	        // `0b1110_0000`: construction allocates nothing, and every empty
       125	        // version shares the one static buffer (clone identity holds
       126	        // even across separate `new()` calls). A `static`, not a
       127	        // `const`: a const's promoted allocation has no guaranteed
       128	        // unique address, and the cross-call sharing claim rests on one.

Resolution: one test-only assertion, `assert!(Version::new().view().ptr_eq(Version::new().view()))`, with the `Party::seed()` twin; or drop the parenthetical and let the `static` speak for itself. Acceptance: a committed test reads red when `EMPTY_STREAM` becomes a `const`, or the comment no longer claims cross-call sharing.

Construction: change `static EMPTY_STREAM` to `const EMPTY_STREAM`; every committed test stays green whether or not the promoted allocation happens to be shared.

### version-core-5: The join/meet subadditivity lemma is derived only in test prose, though the folds' auxiliary-space bounds and a downstream budget rest on it
- Where: crates/before/src/version.rs:438-445 (related: crates/before/src/version.rs:475, 498-505, 539, 610, 1361, 1374, 1387, 1400; crates/before/src/version/tests.rs:110-119, 137-146, 481-528; crates/before/src/meter/tier2/tests.rs:328-346; crates/before/src/fold.rs:1-16; src/tree/mirror/streaming/window.rs:326-329; crates/before/fuelscape/version_join.json)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (`grep -rn -i subadditiv` over crates/before/src and src excluding test files: the only production hits are a code comment at grow.rs:511 and the rumors consumer at window.rs:328; the derivation at tier2/tests.rs:328-346 and the lemma statement at tests.rs:110-119 read in full; the join island contract reads `O(|self| + |other|)` time only); executed: no
- Seen by: correctness ([41], from the fold's space bound), claims ([48], from the join/meet contract); refutation: both confirmed; history: already-known (the amplification note records the lemma of record, `size(c) ≤ size(a) + size(b) − 2`, its tier2 pins, and an owner ruling of 2026-07-23 that the bound stays unless falsified; no ruling addresses where a production-side statement lives)
- Owner-gated: yes (adds a sentence to the public contract of API-stable doors)

Two consequences of one gap. First, the n-ary folds promise `O(|self| + |iter|)` auxiliary space, and the balanced counter (fold.rs:41-80) holds up to `log k` merged groups over disjoint input subsets, so the bound holds only if each merged group's encoding is at most the sum of its inputs' encodings, which is exactly the subadditivity lemma; that argument appears nowhere a reader of the fold can find it, only in test doc comments ("Callers that track version-size maxima rely on this"). Second, rumors's mirror window budgets on the byte-level corollary ("before's pinned join- and meet-subadditivity lemmas"), yet the public rustdoc of `join`/`meet` and the operator matrices state time complexity only, so under the crate's own framing (documented claims are hard guarantees) the property a consumer relies on is promised by no door. The standard asks that each hard-guarantee claim carry (a) an argument, (b) a matching implementation, and (c) a committed instrument; (a) is missing for the space bound, and the lemma itself has (b) and (c) without a public (a).

Evidence:

       438	    /// The join (least upper bound) of this [`Version`] and `other`: their
       439	    /// combined causal history.
       440	    ///
       441	    /// Identical to the operator form `self | other`.
       442	    ///
       443	    /// # Complexity
       444	    ///
       445	    #[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/version_join.html"))]

       475	    /// Auxiliary space is `O(|self| + |iter|)`.

Resolution: state the lemma once in production prose, positively, with the derivation's one-line shape (output boundaries lie in the union of input boundaries; pointwise max/min is 1-Lipschitz in each operand; zigzag-gamma length depends only on magnitude; the shared root and unmatched first-leaf code give the `−2`): in the `# Complexity` sections of `Version::join` and `Version::meet` (the operator matrices' `$opdoc` can cite them), and cited from `balanced_fold`'s doc or the `crate::fold` module doc as the reason a merged group cannot outgrow its inputs. Point the four `*_encoding_is_subadditive*` tests and the tier2 pins at the stated lemma by name. Acceptance: `grep -rn -i subadditiv crates/before/src/version.rs` matches the public rustdoc of `join` and `meet`; the aux-space lines read as consequences of a stated lemma; the tier2 pins are unchanged; `just readme` regenerates cleanly.

Construction: no committed artifact fails today, which is the gap. Before writing the lemma down, search for a counterexample: extend `join_encoding_is_subadditive_arbitrary` and its meet dual with a deeper generator than `arb_oracle_version`'s depth and with the meter families' deep spines and comb parties, asserting `encode(a|b).len() <= encode(a).len() + encode(b).len()`. If none appears, the fix is prose; if one appears, the aux-space claim needs a corrected bound.

Constructed test: inconclusive (results.md lines 95-147). A missing sentence cannot be shown by a run; as a sanity check, join and meet were subadditive in encoded bytes on all 128 ordered pairs of eight adversarial shapes (no counterexample).

### version-core-9: `shape`'s closing paragraph argues from "realistically reachable" versions, which the same file's doctest refutes
- Where: crates/before/src/version.rs:742-748 (related: crates/before/src/version.rs:205-208; crates/before/src/version/tests.rs:441)
- Class / severity / confidence: claim / low / high
- Provenance: verified (version.rs:205-208 parses `"100000000000000000000000000"` into a `Ticks` and ticks a version by it in the public doctest; tests.rs:441 parses a leaf above `u64::MAX`); executed: no
- Seen by: prose, claims; refutation: confirmed; history: deliberate-and-holds as owner-authored prose (a5f06cad, 2026-08-19, replaced "Typical shapes sit far from that bound." with the current text)
- Owner-gated: yes (owner-authored substance; the likelihood framing is his call)

The paragraph prices the caller's own `Ticks` arithmetic (fine, in one sentence), then dismisses the quadratic worst case on the ground that reachable versions have machine-word `Ticks`; the crate's own `ticks` example builds a version whose one rise is 87 bits wide from the public API, and the crate's contract disclaims likelihood arguments ("the likelihood of an input carries no weight"). The grammatical subject of "are therefore effectively constant-time" is `[Version]s`, not the additions.

Evidence:

       742	    /// Arithmetic *you* do with the [`Ticks`] is priced separately. [`Ticks`]
       743	    /// addition costs the operands' widths, so folding the rises into a
       744	    /// running absolute height can cost each step the running value's full width,
       745	    /// inherently quadratic over the drain in the worst case. Typically, this is
       746	    /// not an issue, however, because realistically reachable [`Version`]s have
       747	    /// [`Ticks`] which are bounded by a machine word, and are therefore effectively
       748	    /// constant-time.

Resolution: state the price as a function of the input and stop: "Arithmetic you do with the yielded [`Ticks`] is not included: summing rises into a running height costs the running value's width per step, `O(1)` while every height fits a machine word and up to quadratic in the encoded size when heights are wide; the walk itself stays linear regardless." Acceptance: the paragraph names no likelihood, its subject agrees with its predicate, and the island's `O(|self|) to drain` contract is unchanged.

Construction: `let mut v = Version::new(); v.ticks(&Party::seed(), "100000000000000000000000000".parse::<Ticks>().unwrap());` yields a version whose one rise is 87 bits wide, refuting "bounded by a machine word" from the public API alone.

### version-core-16: The tuple-literal `TryFrom` states `O(m)` but each nesting level re-scans its children
- Where: crates/before/src/version.rs:1486-1488 (related: crates/before/src/version/skyline/literal.rs:32-34; crates/before/src/version/tests.rs:394-400)
- Class / severity / confidence: claim / nit / high
- Provenance: assessed (read literal.rs:33-34: `scan(left)` and `scan(right)` collect every leaf height at each level, and the children are re-emitted; the test doc at tests.rs:396-397 describes the re-derivation); executed: no
- Seen by: claims; refutation: confirmed; history: the `O(m)` line entered in 3bba6cbbc with no pricing of the depth factor
- Owner-gated: no (a doc correction toward the code)

A literal nested `d` levels deep scans the innermost stream `d` times, so the cost is `O(m · d)`; `d` is a compile-time property of the tuple type, so no caller input reaches it, but a public `# Complexity` section is a contract and this one hides the factor the test doc describes.

Evidence:

      1486	    /// # Complexity
      1487	    ///
      1488	    /// `O(m)`, with `m` the built version's size in bytes.

Resolution: state `O(m · d)` with `d` the literal's nesting depth (each level re-derives its children's absolute heights from their streams), or restructure `literal::node` to compose without re-scanning. Acceptance: the doc names the depth factor, or `literal::node` no longer re-scans and `descending_literals_build_the_oracle_tree` stays green.

Construction: a 20-deep right-nested literal of `u64` leaves scans the innermost leaf's stream 20 times; under the scan meter the reading is about the sum of subtree sizes rather than `m`.

### version-core-23: `Ticks`' `Sum` bound omits the per-summand term and states no amortization argument
- Where: crates/before/src/version/ticks.rs:41-42 (related: crates/before/src/version/ticks.rs:37-39, 275-293)
- Class / severity / confidence: claim / low / high
- Provenance: assessed (ticks.rs:37-39 defines `N` as the summands' total bit width, zero for any number of `Ticks::ZERO`; `Sum` at 276-282 is a fold of `+=` over the iterator, `Θ(k)` iterations; the per-add cost and the carry amortization are the claims lens's reading of dashu-int's `add_large`, not re-read here); executed: no
- Seen by: correctness ([44]), claims ([59]); refutation: both confirmed; history: the clause entered with the Tier-3 complexity lines (669cf3103) from a uniform template; no argument recorded
- Owner-gated: no (a doc correction toward the code)

Every asymptotic claim is a hard guarantee for all inputs; a bound that reads `O(0)` on a nonempty iterator is wrong for the zero-width family, however cheap the miss (the missing term is exactly the per-item constant). And the `O(N)` half does need an argument: a single `+=` can carry through the whole accumulator, so only the binary-counter potential (each carry clears a bit an earlier summand set) brings the total to `O(N + k)`; the other bounds on the line read straight off the implementation and need none.

Evidence:

        41	/// Construction is `O(1)`; comparison and hashing `O(‖n‖)`; addition `O(‖a‖ +
        42	/// ‖b‖)`, `Sum` `O(N)`; text I/O is superlinear but subquadratic in the count's

Resolution: write `Sum` as `O(N + k)` for `k` summands, "amortized: each carry clears bits an earlier summand set", or define `N` as `Σ(1 + ‖nᵢ‖)`. Acceptance: the stated `Sum` bound is nonzero for every nonempty iterator and names its amortization.

Construction: `(0..k).map(|_| Ticks::ZERO).sum::<Ticks>()` has `N = 0` and performs `Θ(k)` iterations.

### Rank

### rank-4: The bold public size claim is false at small scale
- Where: crates/before/src/version/rank.rs:198-202 (related: crates/before/src/version/tests.rs:962-1014, version/tests.rs:1179-1246, crates/before/src/version/skyline.rs:11-24, crates/before/src/codec/tests.rs:78-80, crates/before/src/version.rs:1129-1131)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (hand arithmetic from the committed coding and goldens: skyline.rs:11-24 gives one preorder flag bit per node, the first leaf as `gamma(height)`, later leaves as `gamma(zigzag(delta))`; codec/tests.rs:78-80 pins `gamma(0)` at 1 bit and `gamma(2)` at 3 bits; version.rs:1131 gives `encode().len() == (encoded_bits() + 1).div_ceil(8)`; version/tests.rs:974 pins rank 1/2 at `[0x60, 0x00]`); executed: no
- Seen by: correctness; refutation: confirmed (re-derived independently); history: no rationale found (the sentence is a new `# Complexity` section from 8d8a06e2 with no instrument behind the universal)
- Owner-gated: no

The version `"(0, 1, 0)"` is root flag `0`, leaf `1` + `gamma(0)` = `1`, leaf `1` + `gamma(zigzag(+1) = 2)` = `011`: 7 live bits, one byte with its marker; its rank is 1/2, whose canonical bytes are the committed two-byte golden. `Version::try_from(1)` is `1` + `gamma(1)` = 4 bits while its rank golden `[0x80]` is 8 bits, so in the provenance pin's own denomination the inequality `encoded_bits <= input_bits` fails at that scale, and the pin's five fixed-scale families at ratio at or under 1.0 are an envelope, not a proof of "never". Statement faithfulness: never stronger than proven; the crate docs hold every space claim as a hard guarantee, and this one is in public rustdoc a KV-store user would size keys by. The paragraph that follows argues only a constant-factor bound.

Evidence:

       198	/// **A rank's representation never is larger than the version it measures, and
       199	/// often is exponentially smaller.** In-memory size and encoded size are both,
       200	/// within small constant factors, at most the length of the rank's binary
       201	/// expansion, which we'll write `‖r‖`. Notably, `‖r‖` is at most linear in the
       202	/// size of the originating version.

    (version/tests.rs)
       973	        // 1/2 = "0" ++ "1 10000000 0".
       974	        (rank_of("(0, 1, 0)"), vec![0x60, 0x00]),
      ...
       978	        // 1 = "1000" ++ "0": the first integral header step.
       979	        (int(1), vec![0x80]),

Resolution: Restate the bold sentence as the bound the paragraph argues (a small constant multiple of the version's packed size, and often exponentially smaller), and state the denominator the pin measures (encoded rank bits against packed version bits at the tested scales). If a byte-size bound is wanted as a contract, derive it (per level at least two topology/payload bits against 9/8 fraction bits; a b-bit counter costs 2b+1 gamma bits against b + 2 log b) with the small-scale exception stated, and pin it at two scales. Acceptance: the public doc no longer asserts an unqualified "never"; a committed test asserts the small-scale witness (`"(0, 1, 0)".parse::<Version>().unwrap().encode().len() == 1` alongside the existing 1/2 golden), and the provenance pin's doc names its denominator and scales.

Construction: `let v: Version = "(0, 1, 0)".parse().unwrap(); assert_eq!(v.encode().len(), 1); assert_eq!(v.rank().encode(), vec![0x60, 0x00]);` (rank bytes 2 > version bytes 1). Bit-denominated, the pin's own: `let one = Version::try_from(1).unwrap(); assert!(one.rank().encode().len() * 8 > one.encoded_bits() as usize);` (8 > 4). Both rank-side byte strings are already committed goldens at version/tests.rs:974 and 979.

Constructed test: demonstrated, with a correction (results.md lines 1705-1776). In bits the claim fails at both scales: `"(0, 1, 0)"` is 9 live bits and its rank golden `[0x60, 0x00]` 16 bits; `Version::try_from(1)` is 4 live bits and its rank `[0x80]` 8 bits. In bytes the two are tied (2 = 2 and 1 = 1): the entry's "7 live bits, one byte" premise is wrong (the version encodes to 2 bytes), so its byte-level construction is refuted while the bit-denominated one stands.

Synthesis note: The constructed test refutes the entry's byte-level clause ("7 live bits, one byte"; the version is 9 live bits and two bytes, so rank and version bytes are equal, not 2 > 1) and confirms the bit-denominated clause, which is the denomination of the provenance pin the entry names. The verdict stands with the byte example struck; the resolution's proposed small-scale witness should assert `encode().len() == 2`, not 1.

### rank-10: Rank::decode's # Errors says NotCanonical needs 2 EiB of input; a nine-byte header reaches it
- Where: crates/before/src/version/rank.rs:442-444 (related: rank.rs:727-731, rank.rs:124-131, crates/before/src/version/tests.rs:1164-1171)
- Class / severity / confidence: claim / low / high
- Provenance: verified (the check at 727-731 fires after a unary run of 64 ones; the committed genre witness at version/tests.rs:1167-1170 feeds `[0xFF; 8] ++ [0x00]` and asserts `NotCanonical`); executed: no
- Seen by: correctness, claims; refutation: confirmed; history: no rationale found (8d8a06e2 transcribed the code comment's size argument as the trigger; the nine-byte witness predates it, 6ae76895)
- Owner-gated: no

The `# Errors` section is the contract a caller decides handling by, and it calls this arm effectively unreachable; the crate's own test reaches it with nine bytes. What needs 2 EiB is a canonical stream of that width, not the rejection; the sentence conflates the error's trigger with the bound's provenance. The module doc at 128-131 already has the accurate framing.

Evidence:

       442	    /// - [`Decode::NotCanonical`] when otherwise valid content exceeds the type's
       443	    ///   representation bound (an integral mantissa of `2⁶⁴` or more bits, effectively
       444	    ///   unreachable, since it can only be hit by reading inputs of 2 EiB or more);

       727	    if rho >= 64 {
       728	        // The format bound: an integral width of 2⁶⁴ or more bits exceeds both
       729	        // the numerator this crate can hold and any input under 2 EiB (the
       730	        // mantissa alone would need 2⁶⁴ − 1 bits).
       731	        return Err(Decode::NotCanonical);

Resolution: Reword: "[`Decode::NotCanonical`] when the integral header claims a mantissa of 2^64 or more bits (a unary run of 64 or more ones: eight leading 0xFF bytes); no canonical encoding carries such a header, since its mantissa alone would exceed 2 EiB, so the genre only ever names a forged stream." Acceptance: the clause names the trigger and separates it from the size argument; the genre test passes unchanged.

Construction: `assert!(matches!(Rank::decode(&[0xFF; 8][..].iter().copied().chain([0x00]).collect::<Vec<u8>>()[..]), Err(Decode::NotCanonical)))`; this is the committed case at version/tests.rs:1167-1170.

### rank-20: sum_ranks' amortization claim is false for ascending exponent order, and both committed pins fix the benign order while naming it the adversary
- Where: crates/before/src/version/rank.rs:1011-1018 (related: rank.rs:994-1006, rank.rs:1032-1049, crates/suanpan/src/accumulator.rs:592-595, accumulator.rs:626-627, crates/before/tests/meter.rs:1364-1396, crates/before/src/meter/board/ops.rs:509-559)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (suanpan's `shl` doc at accumulator.rs:594-595 states `O(|self|)` digit touches per call and its body at 626-627 is `core::mem::take(self)` then `add_accum_shl(&held, shift)`; rank.rs:1036-1039 calls it on every new exponent maximum; tests/meter.rs:1379-1388 and board/ops.rs:522-530 build the summands high-first); executed: no
- Seen by: claims; refutation: confirmed (no public cost sentence is falsified; the type doc's `O(‖a‖ + ‖b‖)` at rank.rs:211 covers `Add`, not `Sum`); history: no rationale found (ee894835 states the amortization and pins high-first as the adversary of the pre-cure per-element fold; no derivation and no consideration of ascending order anywhere)
- Owner-gated: no

The doc says a summand raising the maximum exponent rescales the accumulator "O(held digits) — paid by the exponent the summand itself carries". The rescale's cost is the held width, not the summand's exponent: a W-bit integer rank followed by 1/2, 1/4, ..., 1/2^n in ascending order pays n rescales of about W/32 digit touches each (plus n buffer rebuilds) against W + n(n+1)/2 content bits, so for W much larger than n^2 the touches per content bit grow with n. Both committed pins sum high-first and document that order as the worst case, which was true of the per-element-normalizing fold ee894835 replaced and is not true of the fold in the tree; a regression on the ascending order would pass every committed check. The `impl Sum` blocks carry no `# Complexity` at all. Instruments before cures: the failing row lands first. A linear algorithm exists (geometric headroom, below), so the bound the doc claims is achievable.

Evidence:

      1011	/// The accumulator holds the running numerator at the largest exponent seen so
      1012	/// far: a summand at a smaller exponent is digit-routed in at the exponent gap
      1013	/// (O(its own limbs), independent of the gap), and a summand raising the
      1014	/// maximum rescales the accumulator once, O(held digits) — paid by the exponent
      1015	/// the summand itself carries. Nothing renormalizes per element, so a
      1016	/// high-exponent summand costs its own width once instead of once per later
      1017	/// element, and the result is the identical [`Rank`] the pairwise fold produces
      1018	/// (one exact value, one shared normalization at the end).

      1036	        if rank.exp > exp {
      1037	            acc.shl(rank.exp - exp);
      1038	            exp = rank.exp;
      1039	        }

    (suanpan accumulator.rs)
       594	    /// `O(|self|)` digit touches, independent of the shift; the digit
       595	    /// buffer covers the shifted positions.
      ...
       626	        let held = core::mem::take(self);
       627	        self.add_accum_shl(&held, shift);

    (tests/meter.rs)
      1372	/// High-first ordering was the adversarial arm of the fold's
      1373	/// order-dependence (`Sum` accepts arbitrary order, so the worst order is
      1374	/// the honest pin); under the raw accumulator it is the order that makes
      1375	/// every later add a shifted word, which is why the pin stays the
      1376	/// scenario of record.

    (board/ops.rs)
       516	                // both sides of the value content scale together. High-first
       517	                // is the committed adversarial order: `Sum` accepts arbitrary
       518	                // order, and under a fold that re-normalizes per element it
       519	                // is the order that makes every later add a full-width
       520	                // operation. The denominator is the summands' total value

Resolution: Instrument first: a two-scale touch row (W fixed at, say, a 2^20-bit counter rank; n in {64, 128} spine ranks 1/2^k summed in ascending k) asserting touches per content bit flat across the two scales; it reads red today (touches double with n while content barely moves). Then cure with geometric headroom: when a summand exceeds the held exponent by `gap`, shift by `max(gap, held_bits)` and carry the surplus as exponent headroom, so the held width at least doubles per rescale and total rescale touches telescope to O(final held width); `from_num`'s `trailing_zeros`/`shr` already strip the headroom at the end (every summand entered at a shift at least the headroom). Rewrite 1011-1018 to the true bound, add `# Complexity` to both `impl Sum` blocks, and correct the two pins' adversarial-order prose. Acceptance: the new row is committed red then green with touches at most c · Σ‖r_i‖ at both scales; `RANK_SUM_MIXED` unchanged or re-pinned with attribution; `rank_sum_equals_the_pairwise_fold` and `rank_cross_path_normalization` still hold.

Construction: `let wide = Rank of a single-leaf version whose counter is 2^(2^20) - 1` (num.bits() about 2^20, exp 0); `let steps: Vec<Rank> = (1..=n).map(|k| spine(k).rank()).collect()` using version/tests.rs's `spine` helper (rank 1/2^k); reset the touch meter; `once(wide).chain(steps).sum::<Rank>()`. Each step raises `exp` by one and triggers `acc.shl(1)` over about 2^20/32 = 32768 held digits: n = 64 gives about 2.1M touches, n = 128 about 4.2M, while content bits move from about 1.05M to 1.06M. The reverse order (steps descending, then `wide`) stays near n + W/32.

Constructed test: demonstrated (results.md lines 3414-3494). A ~265k-bit integer rank followed by 1/2, ..., 1/2ⁿ in ascending order costs 1,023,760 touches at `n` = 64 and 2,019,782 at `n` = 128 (about `n` times the ~8,306 held digits); the same summands descending cost 20,897 (×96.7).

### rank-23: The module doc attributes arm placement to pins that assert values only
- Where: crates/before/src/version/rank/num.rs:28-34 (related: crates/before/wasm32-pins/harness/tests/pins.rs:165-171, crates/before/wasm32-pins/guest/src/lib.rs:603-643)
- Class / severity / confidence: claim / nit / medium
- Provenance: verified (`grep -rn 'is_wide\|numerator_is_wide' crates/before/wasm32-pins/` returns nothing; pins.rs:167-170 asserts `Outcome::Value(0)`; the guest entries decode, compare, clone, and re-encode); executed: no
- Seen by: history pass (cross-cutting observation 8); refutation: not examined; history: no rationale found
- Owner-gated: no

The parenthetical states which arm each pin decodes on; no pin observes the arm. What the pins do hold is the decisive direction the constant's own doc names (num.rs:77-82): a past-capacity decode that misrouted to the `Base` arm would panic inside the backend, so the past-capacity pins' success bounds the ceiling from above. The below/at-capacity pins would pass on either arm. The sentence should claim the side the pins hold.

Evidence:

        28	//! routing: both arms denote the same integers exactly, so a misplaced
        29	//! ceiling could misroute cost, never value. The production ceiling is the
        30	//! backend capacity itself, held to the real backend by the wasm32
        31	//! boundary pins (the below/at-capacity decode pins fill the backend's
        32	//! last word on the [`Base`] arm; the past-capacity pins decode on the
        33	//! [`Wide`] arm); tests may lower it (the test-only `ceiling` module) so
        34	//! every public door drives both arms and the seam between them at

    (pins.rs)
       167	    assert_eq!(
       168	        call1("pin_rank_decode", (1u64 << 32) - 32),
       169	        Outcome::Value(0),
       170	    );

Resolution: "held to the real backend from above by the wasm32 boundary pins: a decode one fraction group past the capacity succeeds, which it could not if the ceiling routed that width to the backend (the pins assert values, not arms; the lower side is not load-bearing)". Acceptance: the sentence claims only what a pin observes.

### rank-32: Ranked::encode_rank is documented as a fused, more efficient emission; it is Rank::encode by another name, and has been since the commit that introduced it
- Where: crates/before/src/version/ranked.rs:211-215 (related: ranked.rs:46-49, ranked.rs:189-196, crates/before/src/version/rank.rs:395-397, rank.rs:486-498, rank.rs:612-618, crates/before/src/version.rs:1049-1052, version.rs:1071-1075, crates/before-fuelscape/src/ops.rs:1312-1314, crates/before/src/version/tests.rs:1639-1641, crates/before/src/version/skyline/query.rs:201-225)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (both bodies read; `git show c8ea49f4:crates/before/src/version/ranked.rs` lines 171-175 show the same three-line body as `Ranked::encode` at the commit titled "fused encode", and 8519dd47 renamed it unchanged; `skyline::query::rank` returns a `Rank` via `Rank::from_raw` at query.rs:224, so there is no fold-side `(num, exp)` to hand off; `grep -rn 'encode_parts\|raw_parts' crates/before/src` shows `encode_parts`' only caller outside rank.rs is ranked.rs:214 and `raw_parts`' remaining callers are version/tests.rs:804, 1372 and meter/board/ops.rs:2277); executed: no
- Seen by: structure, prose, correctness, claims (four independent reports); refutation: confirmed; history: no rationale found (the private "hand-off" prose was written in c8ea49f4 and never matched the code; the public efficiency sentences were added later in 8d8a06e2 and c8fd70e6)
- Owner-gated: no

The body materializes the `Rank` (`self.version.rank()`), destructures it, and calls the same `encode_parts` that `Rank::encode` calls on the same normalized parts: identical work, byte for byte and cost for cost. Yet three public sentences claim efficiency or non-materialization, the `pub(crate)` visibility of `encode_parts` is justified by "the ranked view's fused emission", `raw_parts` is described as "The fused encode's hand-off", the fuelscape `size_measure` says "one fused rank fold and emission", and a test doc says "the fused emission from the fold's raw parts". Public rustdoc states a cost contract the code does not deliver (the crate makes cost claims guarantees), and the two crate-private entries justify their existence by a mechanism that does not exist: the circular-justification tell.

Evidence:

       211	    pub fn encode_rank(&self) -> Vec<u8> {
       212	        let rank = self.version.rank();
       213	        let (num, exp) = rank.raw_parts();
       214	        encode_parts(num, exp)
       215	    }

        48	/// choosing; [`encode_rank`](Self::encode_rank) emits exactly the corresponding
        49	/// [`Rank`]'s encoded bytes without materializing the intermediate [`Rank`].

       194	    /// - `v.ranked().encode_rank()`
       195	    /// - `v.rank().encode()` (this one is less efficient)

    (rank.rs)
       395	    pub fn encode(&self) -> Vec<u8> {
       396	        encode_parts(&self.num, self.exp)
       397	    }

       488	    /// The fused encode's hand-off from a rank fold's output to the canonical
       489	    /// emission ([`encode_parts`]), and the raw normalized form the reference

       615	/// `pub(crate)` alongside [`Rank::encode`] so the ranked view's fused emission
       616	/// can emit straight from its rank fold's `(numerator, exponent)` output, with
       617	/// no walk beyond the fold's own.

    (version.rs)
      1051	    /// Equivalent to `self.rank().encode()`, but more efficient. Exactly
      ...
      1073	    /// Equivalent to `self.rank().encode_to(writer)`, but more efficient.

    (before-fuelscape/src/ops.rs)
      1312	        size_measure: "packed bytes of the viewed version (the composite key's \
      1313	             rank component alone: one fused rank fold and emission; view \

Resolution: Make `Ranked::encode_rank` `self.version.rank().encode()` and `encode_rank_to` likewise; drop the `encode_parts` import from ranked.rs and make `encode_parts` private to rank.rs (delete its `pub(crate)` rationale); re-justify `raw_parts` by its remaining callers (the oracles' raw form and the board's limb denomination), or replace it with a `#[cfg(any(test, feature = "meter"))]` accessor beside `content_bits`. Rewrite ranked.rs:46-49 and 192-196 to "equivalent to `v.rank().encode()`" with no efficiency ranking; version.rs:1051 and 1073 drop "but more efficient" (keep "more succinct"); rank.rs:486-491 becomes "The stored parts, the raw normalized form the reference computations and the meter denominators read"; the fuelscape `size_measure` reads "one rank fold and its emission" (regenerate the JSON); version/tests.rs:1640-1641 drops the parenthetical. If an actual fusion is wanted, note that constructing the `Rank` after normalization is free, so there is no cheaper fold-side path to build; the true statement is that `encode_rank` exists as a spelling convenience on a view. Acceptance: `grep -rn 'fused emission\|fused encode\|more efficient\|less efficient\|without materializing the intermediate' crates/before/src crates/before-fuelscape/src` returns nothing about `encode_rank`; `encode_parts` has no `pub(crate)`; `ranked_carries_own_rank` (laws.rs:388-389, `ranked.encode_rank() == a.rank().encode()`) still passes; the board cells `rank_encode` and `ranked_encode_rank` read identical limb and touch counts for the same version, as they must already.

Construction: Read the two bodies side by side (ranked.rs:211-215 and rank.rs:395-397): both reduce to `encode_parts(num, exp)` on the output of `skyline::query::rank`. For a mechanical witness under `--features limb-meter`: reset, call `v.rank().encode()`, read `limb_ops()`; reset, call `Ranked::from(&v).encode_rank()`, read again; the counts are equal for every `v`. Alternatively wrap `Rank::from_raw` in a thread-local call counter under test and assert each spelling increments it exactly once.

Constructed test: demonstrated (results.md lines 5049-5118). On seven versions (empty, leaf 5, a text tree, bigroot, cliff comb, dense, hugeleaf) `v.rank().encode()`, `Ranked::from(&v).encode_rank()`, and `v.encode_rank()` record identical limb counts (5 to 28) and identical bytes.

### rank-33: Ranked::cmp promises O(|self| + |other|) but runs the multiplication-bound rank settle
- Where: crates/before/src/version/ranked.rs:372-378 (related: ranked.rs:19-22, ranked.rs:328-347, crates/before-fuelscape/src/ops.rs:1339-1357, crates/before/fuelscape/ranked_cmp.json, crates/before/src/version/skyline/query.rs:271-290, query.rs:390-401, crates/before/src/version/skyline/query/integral.rs:1142-1163, integral.rs:205-229, crates/before/src/meter/board/ops.rs:667-688)
- Class / severity / confidence: claim / high / high
- Provenance: verified (query.rs:286-290 `rank_cmp` is `pair_fold(a, b, |_| 1).0`; `pair_fold` ends at query.rs:399 with `integral.finish(overlay_depth)`; `Integrator::finish` at integral.rs:1142-1143 runs `self.settle()`, the mass-balanced product tree the module doc prices at `O(M(|v|))` and, past the backend's power-law tiers, `O(M(|v|) · log |v|)` (integral.rs:213-229); the total is computed and discarded. The committed contract for `ranked_cmp` (ops.rs:1350, fuelscape/ranked_cmp.json) is `O(|self| + |other|)` with claim `n`, while `version_distance.json`, the same kernel with a different orientation, is `O(M(|self|) · log |self|)`. The `ranked_cmp` island's overlay roster is jump_pair, tooth_tail, concurrent_pair, dense × self, hugeleaf × self; `version_rank`'s includes wide_arming and plateau_puncture, the two families whose settle products fire, and neither appears on the comparison island); executed: no
- Seen by: claims; refutation: confirmed, with the roster corroboration added; history: no rationale found; a regression, not a decision: at 3bba6cbb the type doc stated "time `O(M(n) · log n)` worst case" and that "On answer-embedding pairs the shipped co-sweep provably pays the backend's multiplication cost"; c6d8106a wrote the linear roster string, b5a81583 deleted the type-doc paragraph because the roster became the single source, and 2efff149 re-denominated the string
- Owner-gated: no

The crate docs make every asymptotic claim a hard guarantee ("any violation is a bug"), and this public `# Complexity` states a class the implementation does not meet: the sign-only fold still settles the exact signed difference, whose value embeds an input-funded product, so its cost is the pair measures' bound with the promotion ledger and settle tree allocated, not a linear sweep. The only instruments pricing this entry are a uniform fuzz-fit band fitted on a roster that omits exactly the superlinear families the same kernel's other island exhibits, and a board cell that takes the distance/lag touch floors (an adequacy floor, not an order pin). The type doc's "cheaper than two rank folds and a compare" (19-22) is a constant-factor statement that remains true; the class claim is the defect.

Evidence:

       372	/// The total order: rank first, canonical bytes on rank ties.
       373	///
       374	/// # Complexity
       375	///
       376	/// One fused signed rank co-sweep over the two viewed versions:
       377	///
       378	#[doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/ranked_cmp.html"))]

    (query.rs)
       286	pub fn rank_cmp(a: BitsView<'_>, b: BitsView<'_>) -> Ordering {
       287	    // `∫ D`, signed: σ is constantly `+1`, the total is
       288	    // `rank(a) − rank(b)`, and only its sign is kept.
       289	    pair_fold(a, b, |_| 1).0
       290	}
      ...
       399	    let (sign, total) = integral.finish(overlay_depth);

    (integral.rs)
      1142	    pub(super) fn finish(mut self, closing_shift: u64) -> (Ordering, UBig) {
      1143	        self.settle();

    (before-fuelscape/src/ops.rs)
      1350	        contract: "`O(|self| + |other|)`",
      1351	        claim: "n",

    (3bba6cbb, ranked.rs)
        81	/// With `n = |a| + |b|`, the compared views' versions' total size in
        82	/// bytes: `O(n)` space; time `O(M(n) · log n)` worst case, `O(n log n)`
        83	/// with width-bounded parked drifts.

Resolution: Restore the contract at ops.rs:1350 to the pair measures' form (`O(M(|self| + |other|)) · log(|self| + |other|))` time, `O(|self| + |other|)` space, mirroring `version_distance`'s committed contract) and regenerate fuelscape/ranked_cmp.json through the compactor so the island and fuelscape-verify agree; add the plateau_puncture and wide_arming pair families to the `ranked_cmp` OpSpec so the fit itself confronts the superlinear shapes; rewrite ranked.rs:19-22 to what is true (one co-sweep instead of two folds and a compare, a constant, with the integrator's ledger and settle tree as its transient). If a cheaper order kernel is wanted, note that a sign-only fold cannot be linear on exact ties (the answer-embedded product must be resolved), so a domination-certificate early exit can only improve the non-tie case; the public worst case stays M-bound either way. Acceptance: the island text reads the M-bound contract and fuelscape-verify passes on the regenerated JSON; a committed two-scale meter row `ranked_cmp × PlateauPuncture` (`Shape::PlateauPuncture.packed2(w, d)` against `Version::new()`, at (w, d) and (2w, 2d)) whose limb and touch readings track `a.rank()`'s rather than a flat per-byte line.

Construction: `let a = Shape::PlateauPuncture.packed2(64, 48).version(); let b = Version::new();` reset the limb and touch meters; `Ranked::from(&a).cmp(&Ranked::from(&b));` read the counters and compare with `a.rank()`'s under the same meters: the co-sweep's settle performs the same product tree (`rank_cmp` discards `total` after computing it), so both readings share the multiplication term; doubling (w, d) grows that term at the backend's power-law exponent, not linearly in packed bytes.

Constructed test: demonstrated for the claim's core; the order stands by reading (results.md lines 148-257). What the run settled: at every scale `Ranked::cmp` does the rank fold's work, 0.77× `rank`'s limb operations (281/364, 561/726, 1113/1440 at the three deterministic scales) and about 1.08× its wall time in the same run, while the `partial_cmp` control does a fraction of either; the structural reading (`rank_cmp` is `pair_fold(a, b, |_| 1).0`, `pair_fold` ends in `integral.finish`, `finish` calls `self.settle()`) was confirmed at the cited lines. What the run did not settle: the order. The deterministic limb-counter leg read 0.989, linear, because `meter.rs:3565-3566` prices each `Base` operation by its operands' limb counts, so one multiplication counts linearly by denomination and the limb meter is structurally blind to the `M(n)` term. The wall rows (min of three in a dev build; exponents 1.14, 1.20, 1.28, 1.25 across four doublings against a `partial_cmp` control at 1.00) carry no load disclosure, on a machine the review elsewhere records at load averages between 3.8 and 19.9, so they are wall time under undisclosed load and not a measurement of the exponent; the M-bound stands by reading and by the settle's own module doc (integral.rs:213-229). The witness's own conclusion is the same: "Demonstrated for the claim's core (Ranked::cmp pays the rank fold's settle while its island says linear), with a caveat on the order."

Synthesis note: The run adds an instrument observation the entry's resolution should carry: the limb meter prices a multiplication by its operand limbs, so no limb-metered row can pin the M(n) class; the two-scale row the resolution asks for must read wall time (the bench judge) or fuel, not limb ops.

### Span and causally

### span-causally-24: The fused query walks do k sign reads and k probe folds per elementary interval for k live holes; the public contracts and the polarity rationale say "linear time"
- Where: crates/before/src/causally.rs:85-86 (related: crates/before/src/causally/query.rs:26-30, 91-107; crates/before/src/causally/polarity.rs:209-212; crates/before/src/version/skyline/place/filter.rs:37-48, 183-207, 279-297; crates/before-fuelscape/src/ops.rs:1742-1743; crates/before/src/meter/board/ops.rs:1213-1238)
- Class / severity / confidence: claim / medium / high
- Provenance: assessed (read `filter::admits`'s per-interval loop and `MemberCursors::step`; read filter.rs's Cost paragraph; read codec/scan.rs's header for what the scan meter counts); executed: no
- Seen by: claims; refutation: reframed (the k factor is present in time: every live side's pair is read per interval and every probe step folds into every live pair; but the bit-read cost is linear because each stream is decoded once, and `codec::scan` counts cursor advances and decodes, not accumulator reads or folds, so a scan-bits row would pass vacuously; the factor is visible only in the touch currency or wasm fuel); history: no rationale found (filter.rs's Cost paragraph names the per-interval k work and then states a bound without k; no note or declared-model cell records a hole-count model)
- Owner-gated: yes: the contract wording of public API docs, and possibly an algorithm decision

`Query::contains` says "One traversal of `version` and the stored bounds" (true), and the module, `Query`, and `Polarity` docs motivate the polarity restriction as guaranteeing "linear time" (causally.rs:85-86, query.rs:28-30, polarity.rs:211-212). With k holes that never drop (an `Up` hole `!after(h_i)` with `h_i <= v`, or a `Down` hole `since(h_i)` with `v <= h_i`), `filter::admits` reads every live pair's sign on every elementary interval (filter.rs:185-207) and folds every probe step into every live pair (filter.rs:283-285): Θ(k · #intervals) operations with #intervals up to leaves(v) + Σ leaves(bound). That is k times the input in operation count, and quadratic in k when holes dominate; the same holds for `filter::coverage`. filter.rs's own Cost paragraph acknowledges the k folds per probe delta and then states `O(|v| + Σ|bound|)`. The bits-decoded bound is true; the word "time" is not, for k >= 2. Every committed instrument has at most one hole (fuelscape ops.rs:1742-1743; the board's `query_contains` is `after(&lo) & before(&hi)`; the meter's contains rows are `since(&v) & before(&e)`), so nothing sees the k axis. The crate treats asymptotic claims as hard guarantees.

Evidence:

        85	//! Instead, we restrict queries to only those whose verdicts can assuredly be
        86	//! resolved in linear time: those with a uniform *polarity*. We say a [`Query`]

    (causally/query.rs)
        28	/// NP-complete (non-polynomial). The [`Polarity`] restriction enforced by the
        29	/// types of [`Query`] ensures that only linear-time decidable queries are
        30	/// expressible.

    (version/skyline/place/filter.rs, outside the partition)
        39	//! Derived, by the placement walk's argument stream by stream: every topology
        40	//! bit of every stream read at most once, every leaf payload decoded once and
        41	//! folded into at most one accumulator per pair it participates in — the
        42	//! probe's deltas into each live bound's pair, a bound's deltas into its own —
        46	//! bookkeeping absorbed by the same per-interval read loop. `O(|v| + Σ|bound|)`

Resolution: owner decision, then wording. (a) Restate the contracts with the hole-count factor: "linear in the bits decoded (each stream once); per-interval work proportional to the number of live holes, `O(k · (|probe| + |self|))` in operations", and replace "linear time" at causally.rs:85-86, query.rs:28-30, polarity.rs:211-212 with the actual payoff of the polarity restriction (a polynomial decision procedure with an exact verdict; the SAT reduction is span-causally-33). (b) Keep the linear promise, which needs an indexed per-hole design. In both cases add a k-scaling instrument in the touch (accumulator) currency or fuel, never scan bits. Also correct filter.rs:46-48 (outside this partition). Acceptance: a touch-currency row with probe `v` fixed and `Q_k = until(&h_1) & ... & until(&h_k)` at k in {8, 64}; under (a) it pins the ratio ~8 as the declared model; under (b) it asserts the difference is bounded by the added holes' bits and fails today. The public docs no longer say "linear time" without the hole-count clause.

Construction: fork k = 64 parties from one seed and tick each once, giving h_1..h_k pairwise concurrent. Let `v` be the join of all h_i followed by ~10^4 further received sends from fresh forks (n ~ 10^4 plateaus). Build `q = until(&h_1) & ... & until(&h_k)` (Up polarity, an antichain of k holes; every side's demand is `NotAfter` with `h_i <= v`, so filter.rs:201-204 never drops a side before exhaustion). `q.contains(&v)` is false and the walk runs ~n + k log k elementary intervals reading k pairs each: ~64 · 10^4 sign reads and probe folds against ~10^4 + 64 log 64 packed bits of input. Halving k halves the touch (accumulator digit) reading at fixed `v`, which `O(|self| + |version|)` forbids; a scan-bits reading is the same at both k.

Constructed test: demonstrated (results.md lines 1777-1864). On one fixed 56,986-bit probe, `until(h_1) & ... & until(h_k)` reads 1,288,319 / 644,160 / 322,080 accumulator touches at `k` = 64 / 32 / 16 while scan bits stay near 58k: halving `k` halves the touch reading, and the scan meter cannot see it.

### span-causally-25: The module-level `# Complexity` says every pass is linear; the conjoin island it links declares `O(|self| · |rhs|)`
- Where: crates/before/src/causally.rs:101-106 (related: crates/before/src/causally/conjunction.rs:38-68, 148-163; crates/before-fuelscape/src/ops.rs:2146-2147)
- Class / severity / confidence: claim / low / high
- Provenance: verified (read the committed island contract at before-fuelscape/src/ops.rs:2146: "linear, plus one comparison per opposite-side hole pair: `O(|self| · |rhs|)` at worst", claim "n^2"; read `and()`'s k survive checks and up to k·m absorption comparisons); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (db9dfa3e's Complexity section priced conjunction as "one lattice walk per floor/ceiling merge and one causal comparison per hole pair" and coverage's clamp walks separately; a6dcfbb4 and 3bba6cbb trimmed it to the blanket sentence)
- Owner-gated: no

Two statements of the same crate disagree about the same operation: the module summary tells the reader every pass and walk is linear, while the conjunction island rendered on every `&` impl declares a hole-pair product. The walk clauses of this paragraph are settled by span-causally-24 and span-causally-36; this finding is the conjunction clause and the paragraph's structure.

Evidence:

       101	//! # Complexity
       102	//!
       103	//! Atoms and named constructors are `O(1)`.
       104	//!
       105	//! Each pass and walk is linear in its operands' sizes in bytes and
       106	//! stops as soon as its verdict is decided.

    (before-fuelscape/src/ops.rs)
      2146	        contract: "linear, plus one comparison per opposite-side hole pair: `O(|self| · |rhs|)` at worst",
      2147	        claim: "n^2",

Resolution: rewrite the paragraph as three clauses once span-causally-24 and -36 are settled: atoms and named constructors `O(1)`; membership and coverage as fused walks over the probe(s) and every bound (with the hole-count factor the owner chooses); conjunction linear in the bounds plus one comparison per cross-side hole pair. Acceptance: the module summary, the conjoin island contract, and the `contains`/`coverage` islands agree on the hole-count dependence.

Construction: none needed beyond reading; the two doc strings contradict each other on their face.

### span-causally-36: `Query::coverage`'s `Partial` refinement sweeps the clamped endpoint once per hole: Θ(k·|hi|) against a published `O(|self| + |span|)`, with no multi-hole instrument
- Where: crates/before/src/causally/query.rs:152-171 (related: crates/before/src/causally/query.rs:114-117; crates/before/src/causally.rs:105-106; crates/before/src/causally/polarity.rs:69-79, 127-137; crates/before/src/causally.rs:161-168; crates/before/src/version/skyline/sweep.rs:233-242; crates/before/src/version/skyline/place/filter.rs:477-515; crates/before-fuelscape/src/ops.rs:1740-1746, 1985-1993; crates/before/src/meter/board/ops.rs:1239-1266; justfile:675)
- Class / severity / confidence: claim / high / high
- Provenance: assessed for the cost trace (read `refine_partial`, `hole_covers` -> `hole_subtracts` -> `le`/`lt` -> `partial_cmp` -> `causal_cmp` with `order_exit`, and `filter::finish`); verified for the instrument census (fuelscape ops.rs:1742-1743 "with at most one hole" and every coverage panel one hole; board `query_coverage` is `delta(&v, &w)`; grep of tests/meter.rs for `.coverage(` finds no row; fuzzfit has no query band); executed: no
- Seen by: prose, correctness, claims; refutation: confirmed (traced the construction below; notes the fuelscape is audit-only, so the failing witness must be a tests/meter.rs row or board cell, and that a fused replacement is linear in bits decoded but keeps the per-interval hole factor of span-causally-24); the prose lens's construction was refuted (a version where every party ticked once normalizes to a single leaf) and is replaced by the claims lens's; history: no rationale found (the per-hole loop's cost was never priced: db9dfa3e's module doc priced coverage's extra work as "two further lattice walks", the clamps, never a per-hole comparison; a6dcfbb4 and 3bba6cbb trimmed that to a blanket linear sentence; `refine_partial`'s doc argues exactness, not cost)
- Owner-gated: no

The public `# Complexity` on `coverage` promises "At most two traversals of the span's endpoints and the stored bounds", the seven islands state `O(|self| + |span|)`, and the module doc says every pass is linear in operand bytes. `refine_partial` then evaluates `self.holes.iter().any(|hole| P::hole_covers(hole, ...))`, and each `hole_covers` is `hole_subtracts(hole, clamped_hi)` (`Down`) or `(hole, clamped_lo)` (`Up`): a full `partial_cmp` sweep of the clamped endpoint against that hole's bound, exiting early only on concurrency (`order_exit`). When the clamped endpoint strictly dominates every hole (the shape a `Partial` verdict with many satisfied holes produces), each sweep runs both streams to exhaustion, so the refinement re-decodes the clamped endpoint k times: Θ(k · |hi| + Σ|hole_i|) on the `Down` `Partial` path, dually for `Up`. Holes survive absorption whenever pairwise concurrent, and k is unbounded (an anti-entropy query conjoining `since(v_i)` for many concurrent peers is exactly this shape). The exactness argument at 143-151 is correct; only the cost is wrong. The crate docs make every asymptotic claim a hard guarantee, and a claim needs an argument, a matching implementation, and a committed instrument that fails when it is false; here the implementation does not match and no instrument can see it.

Evidence:

       114	    /// # Complexity
       115	    ///
       116	    /// At most two traversals of the span's endpoints and the stored
       117	    /// bounds (`|self|`, their total size); one chart per bounds shape:

       161	        if !le(&clamped_lo, &clamped_hi)
       162	            || self
       163	                .holes
       164	                .iter()
       165	                .any(|hole| P::hole_covers(hole, &clamped_lo, &clamped_hi))
       166	        {
       167	            Coverage::Empty
       168	        } else {
       169	            Coverage::Partial
       170	        }

    (polarity.rs)
        77	        fn hole_covers(hole: &Hole<'_>, _clamped_lo: &Version, clamped_hi: &Version) -> bool {
        78	            Self::hole_subtracts(hole, clamped_hi)
        79	        }

    (before-fuelscape/src/ops.rs)
      1742	    // One contains and one coverage panel per stored-bound shape a public
      1743	    // constructor can produce with at most one hole. The query is composed
      1992	        contract: "`O(|self| + |span|)`",

Resolution: instruments before cures. First land a deterministic `scan-meter` row in tests/meter.rs's `placement` module shaped as below, measured at (k, |hi|), (2k, |hi|), (k, 2|hi|), asserting the marginal cost of doubling |hi| is independent of k (within the crate's slack); commit it red. Then replace the per-hole loop with one fused membership walk over the polarity's deciding clamp end: for `Down`, `Empty <=> !filter::admits(clamped_hi.view().live(), self.holes.iter().map(|h| (h.at.view().live(), P::hole_demand(h.strict))))` (with the crossed-clamp test kept as `le(clamped_lo, clamped_hi)`, or reduced to `!le(floor, ceiling)` since `floor <= hi` and `lo <= ceiling` are established by the fused walk returning `Partial`); dually `clamped_lo` for `Up`; a new sealed method naming which clamp end decides replaces `hole_covers` (its only caller). `admits` returns false exactly when some hole's subtraction holds on the probe (filter.rs:221-230), which is the `any(hole_covers)` predicate. Then restate the `# Complexity` text to the traversals the code performs (the fused walk's own hole factor is span-causally-24). Add a many-hole fuelscape variant for the rendered island. Acceptance: the meter row goes green; the doc says what the code does; `coverage_is_exact_on_the_two_party_grid`, `coverage_clamp_refinement_is_exact`, and the new pin of span-causally-35 stay green.

Construction: fork k = 64 parties from `Clock::seed()` and tick each once: h_1..h_k are pairwise concurrent. `q = since(&h_1) & ... & since(&h_k)` (`Down`; `and()` keeps all k as an antichain since no pair compares; `rendered_holes` in causally/tests.rs reads k). `hi` = the join of all h_i followed by many more received sends on further forked parties (n >> k log k plateaus), so `hi > h_i` strictly for every i; `span = Span::new(&Version::new(), &hi)`. `filter::coverage` returns `Partial` (filter.rs:487: `hi`'s relation to each hole is `Greater`, so no hole empties; filter.rs:503: `lo`'s relation is `Less`, so `admits_all` is false). `refine_partial` then computes `le(hi, h_i)` for each i; `causal_cmp(hi, h_i)` refutes `le` early and never refutes `ge`, so `order_exit` never breaks and each sweep reads all of `hi` and all of `h_i`. Wrap `q.coverage(span.reborrow())` in `meter::reset_scan_bits()`/`meter::scan_bits()` at k = 8 and k = 64 with the same `hi`: the difference is ~56 · |hi| bits, where the contract predicts a difference bounded by the added holes' own bits. `q.contains(&hi)` on the same query is the linear control (one fused walk).

Constructed test: demonstrated (results.md lines 3495-3596). With 64 pairwise-concurrent once-ticked forks as holes and a 10,285-bit `hi`, going from 8 to 64 holes adds 1,342 hole bits but 578,644 scanned bits on `coverage` (56.3 × |hi|, one full sweep per added hole); the linear control `contains(&hi)` adds 200 bits.

## The skyline coding

### Coding

### skyline-coding-2: transcoder cost sentence overstates on Bigroot
- Where: crates/before/src/version/skyline.rs:119-121 (related: crates/before/src/version/skyline/encode.rs:14-15 and 42-45, crates/before/src/meter.rs:174-179)
- Class / severity / confidence: claim / low / high
- Provenance: assessed (read); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale (205a361da wrote the sentence while re-denominating transcoder prose)
- Owner-gated: no

`encode_bits` pushes `value.clone()` for every internal node onto a `Vec<Base>` of inherited path sums; on `Bigroot(b, d)` each of the `d` spine nodes clones a `b`-bit `Base`, so time and peak stack are Θ(d·b) against Θ(2b + 4d) packed input bits, the genre the Bigroot family exists to expose. encode.rs's own doc states the accurate bound; the module root claims a linearity the code does not have (statement faithfulness). Test- and meter-only code, so a fixture cost, not a shipped one.

Evidence:

    skyline.rs
    119	//! flight. The construction-language transcoder (`encode_bits`, test- and
    120	//! meter-only) is the one walk that materializes path sums, priced by the
    121	//! packed stream it reads.

    encode.rs
    42	        let value = &offset + &base;
    43	        if internal {
    44	            offsets.push(value.clone());
    45	            offsets.push(value);

    meter.rs
    177	/// 1 · 0^b` (`2b + 1` bits). Puts a `b`-bit magnitude on every root-to-node
    178	/// path sum while keeping paths long — the shape that makes owned per-frame
    179	/// path sums quadratic in the input.

Resolution: reword skyline.rs:119-121 to encode.rs:14-15's bound ("transient state is one `Base` per open subtree, bounded by the packed input's depth and magnitudes"), or make the transcoder push the node's base rather than the running sum so the stack holds Θ(input) bits and the sentence becomes true. Acceptance: the two docs state the same bound; if the delta-stack rewrite lands, the length-agreement and round-trip tests in skyline/tests.rs stay green.
Construction: under `limb-meter`, transcode `Shape::Bigroot.packed2(b, d)` for (b, d) = (2048, 2048) and (4096, 4096) and read `meter::limb_ops()` per input bit; the per-bit cost roughly doubles where a stream-priced walk would stay flat.

### skyline-coding-9: the re-anchor cascade re-copies a wide left-sibling code once per level: join is Θ(depth × width), not linear
- Where: crates/before/src/version/skyline/build.rs:29-34 (related: crates/before/src/version/skyline/build.rs:152-155 and 318-340; crates/before/src/codec/build.rs:93-107 and 110-122; crates/before/src/version/skyline/emit.rs:46-57; crates/before/src/version.rs:889-900; crates/before-fuelscape/src/ops.rs:562-578; crates/before/src/lib.rs:350-358; crates/before/tests/meter.rs:1881 and 1929-1949; crates/before/src/version/skyline/emit/tests.rs:242-261; crates/before/src/version/skyline/build/tests.rs:138-150)
- Class / severity / confidence: claim / high / high
- Provenance: assessed (read; three independent hand traces of `SkylineBuilder::leaf`/`cascade` on the construction below agree: the claims lens, the refutation pass, and mine); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale (build.rs, the 2026-07-23 design note, and the exposition all price the copy against the deletion and none prices the re-flush)
- Owner-gated: no

The module doc prices each cascade copy against "that deletion", but after a re-anchor the extracted code becomes the held leaf, the next leaf's flush writes it again (build.rs:154-155), and the next cascade extracts it again one level up (build.rs:333-335). On `join` of a flat wide leaf against a left spine whose right child at every level is a two-leaf pair, the emitted leaf sequence is `(d, gamma(W))` then zero deltas at depths `d+1, d+1, d, d, ..., 2, 2`; each level flushes W bits (`push_code` splices a `Code::Wide`), extracts W bits (`extract_code` copies bit by bit past `SMALL_CODE_BITS`), and truncates W+2, so the builder does Θ(d·W) work on Θ(d + W) input bits and a (1 + W)-bit output. Contract breached: crates/before-fuelscape/src/ops.rs:571 publishes `O(|self| + |other|)` for `version_join`, and lib.rs:351-353 makes every asymptotic claim a hard guarantee for all input shapes. The public path reaches the kernel: `join_refs` (version.rs:889-900) short-circuits only equality and the empty operand. Uninstrumented: `SKYLINE_JOIN_ABSORB` (tests/meter.rs:1881, 1936-1949) joins the flat leaf against `Dense`, a left spine, so it exercises the absorb face where the held code never moves; the board pairs each shape with its once-ticked twin; `reanchor_cascade_climbs_chained_levels` is a pure right spine where the code moves once; no row or cell mentions re-anchor.

Evidence:

    build.rs
    29	//! Cascading is the loop over re-anchor: a merged leaf may in turn be a
    30	//! zero-delta right sibling one level up. Each cascade step deletes at least
    31	//! three stream bits and copies only a code already priced by that deletion, so
    32	//! emission stays amortized O(1) per output bit; the wide code a deep uniform
    33	//! region telescopes onto is *held*, never re-copied (the absorb repair moves
    34	//! no code bits at all, whatever the held width).

    153	        let flushed_len = held.len();
    154	        self.out.push_bit(true);
    155	        self.out.push_code(&held);

    333	            let code_len = self.lens.pop();
    334	            let code = self.out.extract_code(self.out.len() - code_len);
    335	            self.out.truncate(self.out.len() - code_len - 2);

    codec/build.rs
    102	        let mut out = BitsBuf::with_capacity(n);
    103	        for i in start..start + n {
    104	            out.push(self.bit_at(i));
    105	        }
    106	        Code::Wide(out)

    before-fuelscape/src/ops.rs
    571	        contract: "`O(|self| + |other|)`",

Resolution: the class is decided (owner ruling 1, 2026-09-02, `triage/rulings.md`): cure it, and land the instrument first. One structural cure: emit topology flags and payload codes into two streams and interleave once in `finish()`; every repair then becomes an O(1) truncation of the topology stream plus, for absorb, one 1-bit truncation of the payload stream, and the held-leaf discipline, `lens`, and `extract_code` dissolve (`continue_verbatim` would de-interleave its source range by walking it, still linear). A smaller cure to try first (the reviewer's suggestion after the ruling, unverified): after `cascade` truncates the pair, the sibling's code is already the stream's tail, so hold it in place rather than extracting it, and make the next flush a no-op while the held code is still the tail; `extract_code` is then needed only when bits were appended after the held code, and each cascade step is O(1) plus the flag truncation. Whether this composes with `lens` and `continue_verbatim` decides between it and the two-stream builder. Restating the contract as Θ(output + Σ re-anchored widths) is off the table under the ruling. The instrument lands first, before either cure: a two-scale flatness pin on the construction below (scan bits, and peak heap), plus a committed known-bad demonstrator if the builder is rewritten. Acceptance: a committed test under `scan-meter` builds `b(d)` = the text `(0, T_{d-1}, (0, 0, 1))` iterated from `T_0 = 0` and `a = Shape::Hugeleaf.packed1(10·d)`, measures `meter::scan_bits()` around `&a | &b` at `d` and `2d`, and asserts the per-input-bit scan cost stays within ×1.25 (it reads about ×2 per doubling today by the trace); the same pair added to `assert_emits` still matches the oracle byte for byte; the island contracts and the build.rs/emit.rs cost prose agree with the code.
Construction: `fn spine_of_pairs(d: usize) -> Version { let mut t = String::from("0"); for _ in 0..d { t = format!("(0, {t}, (0, 0, 1))"); } t.parse().unwrap() }` (canonical: every node has a zero-base child, no equal sibling leaves; preorder heights 0, then 0,1 repeated). `fn join_scan(d: usize) -> (u64, u64) { let a = Shape::Hugeleaf.packed1(10 * d).version(); let b = spine_of_pairs(d); let bits = (a.encode().len() + b.encode().len()) as u64 * 8; meter::reset_scan_bits(); std::hint::black_box(&a | &b); (meter::scan_bits(), bits) }`. Expected under the doc's claim: scan/bits flat from `d` to `2d`. Expected from the trace: about `2·d·(20d)` scan bits against about `30d` input bits, so the per-bit ratio doubles per doubling.

Constructed test: demonstrated (results.md lines 258-334). Through the public `Version::join` in both orders, scan bits per input bit read 43.77, 86.45, 171.79, 342.45 at `d` = 32, 64, 128, 256; `scan/bits²` is flat (about 0.045), so total scan is `Θ(d · W)` on `Θ(d + W)` input, and the result equals the flat leaf, so the whole cost is the cascade.

Disposition (owner ruling 1, 2026-09-02, `triage/rulings.md`): the asymptotic claims are absolute, so this is a defect to cure, not a class to accept; the instrument (the two-scale scan pin above, as an envelope row and a board family) lands first so the breach reads as a failure, then the fix; the candidate cure is to hold the re-anchored code in place at the stream's tail instead of extracting and re-splicing it, with the two-stream builder as the fallback (unverified). The ruling also asks for an audit of what else the join, meet, span, rank, masked-comparison, and coverage instruments miss by driving only the benign face.

### skyline-coding-16: "costing one reallocation" is unargued; a doubling buffer can reallocate more than once
- Where: crates/before/src/version/skyline/emit.rs:295-297 (related: crates/before/src/codec/build.rs:60-72)
- Class / severity / confidence: claim / nit / medium
- Provenance: assessed (read: `PackedBuilder::with_capacity` is `Vec::with_capacity(capacity / 8 + 1)` with std's doubling growth); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale (c8ae28c70 rewrote the sentence to state its epistemic status; "one" was phrasing, not a derived bound)
- Owner-gated: no

Statement faithfulness in cost prose: an overrun past twice the estimate costs two reallocations; the heap envelopes pin the measured peak, so nothing is at risk beyond the sentence.

Evidence:

    emit.rs
    295	    // is bounded by the boundary's input codes only up to a constant, so a
    296	    // pathological switch-heavy pair could outgrow it — costing one
    297	    // reallocation, never correctness.

Resolution: "a bounded number of reallocations, never correctness", or derive the output bound (each elementary interval's code is at most the wider input code at that boundary plus a constant) and size the capacity to it. Acceptance: the comment states only what is argued.

### skyline-coding-20: the literal composer rescans both children per level; the `O(m)` constant is depth-proportional and unstated
- Where: crates/before/src/version/skyline/literal.rs:32-34 (related: crates/before/src/version/skyline/literal.rs:82-119; crates/before/src/version.rs:1486-1488 and 1497-1511)
- Class / severity / confidence: claim / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: correctness, claims; refutation: reframed (the nesting depth is a property of the tuple type, so per instantiation `O(m)` holds; the constant grows with the literal's static nesting); history: no rationale (the claim was written as prose and transcribed into the retired checker as `Bound::Linear`; the fuelscape roster files `TryFrom literals` under `version_display`'s superlinear contract)
- Owner-gated: no (the doc-precision fix); a one-pass composer that reshapes the `TryFrom` bounds would be

`node()` scans both already-built child streams in full, materializing a `Vec<Base>` of every leaf height, at every composition level; the public `O(m)` holds because a tuple literal's depth is fixed by its type, but the constant is that depth, the `# Complexity` section does not say so, and the per-node `Vec<Base>` is the materialization the render and parse kernels deliberately avoid.

Evidence:

    literal.rs
    32	pub(crate) fn node(base: u64, left: BitsView<'_>, right: BitsView<'_>) -> Result<BitsBuf, Parse> {
    33	    let (left_topology, left_heights) = scan(left);
    34	    let (right_topology, right_heights) = scan(right);

    version.rs
    1486	/// # Complexity
    1487	///
    1488	/// `O(m)`, with `m` the built version's size in bytes.

Resolution: state the constant in the `# Complexity` section ("each nesting level of the literal rescans its children, so the constant is the literal's static depth"), or build the literal in one pass by lowering the tuple to an iterative walk that feeds `SkylineBuilder` directly (the parse kernel already does this for text), which also retires `scan()`'s `Vec<Base>`. Acceptance: the `TryFrom<(u64, T, S)>` complexity text matches a stated derivation; if the one-pass builder lands, the literal doctests and the text/tests.rs corpus stay green.

### skyline-coding-29: the render merge re-adds a wide `span` once per level; the class is published as "subquadratic"/"n log n" without an argument
- Where: crates/before/src/version/skyline/text.rs:443-482 (related: crates/before/src/version/skyline/text.rs:10-20, 211-225, 337-340; crates/before/src/version/skyline/signed.rs:282-291; crates/before/src/meter.rs:641-667; crates/before-fuelscape/src/ops.rs:324-337; crates/before/src/meter/board/ceilings.rs:409-430; crates/before/src/testing/asymptotics.rs:42-70)
- Class / severity / confidence: claim / medium / high
- Provenance: assessed (read: `signed_sum`'s same-sign arm is `&x + y`, allocating a `Base` of the wider operand's width; `wide_tail` is a right spine of zero leaves over one `2^b − 1` tail; the fuelscape `version_display` overlay families exclude WideTail; `grep -rn 'grows faster'` finds the sentence asymptotics.rs:45-46 quotes only in asymptotics.rs itself); executed: no
- Seen by: claims; refutation: confirmed (plus a second superlinear term: `entries.sort_unstable()` at text.rs:340 is Θ(k log k) in printed nonzero bases, documented as a mechanism at text.rs:219 and absent from every class statement); history: already-known (the superlinearity is a declared, owner-ratified model with `render_merge_superlinearity_is_alive` as its liveness floor); the class words "subquadratic" (2ee971421) and "n log n" (the fuelscape roster) are what is new
- Owner-gated: yes: reopens a ratified model

`merge` computes `span = entry_step + right.span` with `signed_sum`, whose same-sign arm allocates and adds at the wider width; on `WideTail(s, s)` every one of the `s` spine levels carries `span = +W` with `drop` and `lift` zero, so each merge does one `s`-bit add: Θ(s · ⌈s/64⌉) limb ops on `6s` input bits and Θ(s) output bytes, a quadratic leading term with constant 1/64. Every asymptotic claim needs an argument, a matching implementation, and an instrument that fails when false. (a) "superlinear, subquadratic time" (ops.rs:331) has no argument anywhere; ceilings.rs:416-418 points back at the `# Complexity` sections that carry the same string. (b) `a·n² + b·n` is not `o(n²)`. (c) The instruments pin weaker facts: the liveness floor proves the class exists, and `MIRROR_WIDE_RENDER_LIMB_EXPONENT_CEILING = 1.96` caps a fitted exponent at ladder scale, where a quadratic term beside a linear companion fits under 1.96, so the sentence "a genuinely quadratic conversion (~2.0) still reads red" is a snapshot, not a model. text.rs's module doc argues only the space pricing and never states the time class of its own kernel, and asymptotics.rs:45-46 cites a rustdoc sentence that no longer exists.

Evidence:

    text.rs
    457	    // The node's span: its last leaf is the right child's last.
    458	    let span = signed_sum(
    459	        entry_step.0,
    460	        entry_step.1.clone(),
    461	        right.span.0,
    462	        &right.span.1,
    463	    );

    signed.rs
    282	pub(super) fn signed_sum(x_sign: Sign, x: Base, y_sign: Sign, y: &Base) -> (Sign, Base) {
    283	    if x_sign == y_sign {
    284	        return (x_sign, &x + y);
    285	    }

    before-fuelscape/src/ops.rs
    331	        contract: "superlinear, subquadratic time; `O(|self|)` space",
    332	        claim: "n log n",

    ceilings.rs
    423	/// 0.15; the fitted exponents live in the pin commit), so a genuinely
    424	/// quadratic conversion (~2.0) still reads red. The model's under-side is not banded here: the class's

Resolution: owner ruling on a ratified model; two consistent options. Cure: carry `span` (and `drop`) up the spine instead of re-summing them at each level (fold the small `entry_step` into the moved child summary in place; an `Accumulator` per flowing summary keeps the fold amortized O(1) across carry cliffs, and only a printed base pays a magnitude read), measure at the parent on `SKYLINE_RENDER_*` and the mirror-wide cells, then retire the declared model and the liveness pin together as ceilings.rs:425-429 prescribes. Accept: replace "superlinear, subquadratic"/"n log n" with the derived class Θ(|self| + depth × max interior summary width + k log k) in ops.rs and the island, state both mechanisms in text.rs's render doc, delete the "genuinely quadratic still reads red" sentence, re-point asymptotics.rs:45-46 at a sentence that exists, and add the closed-form witness below so the instrument pins the order rather than a scale-bound fit. Acceptance: either the mirror-wide cells and `render_merge_superlinearity_is_alive` both read linear after the cure (the pin flips red and is retired in the same commit), or the `version_display`/`version_fromstr` contracts, the text.rs render doc, and the ceilings.rs prose all state the same derived class and a committed check `render_limb_ops(s) >= s * s.div_ceil(64)` holds at three doublings.
Construction: under `limb-meter`, for `s in [1024, 2048, 4096]` assert `render_limb_ops(s) as usize >= s * s.div_ceil(64)` (each of the `s` spine levels' span sum costs at least `⌈s/64⌉` limb ops); as an order witness, the doubling ratios `render_limb_ops(2s) / render_limb_ops(s)` increase with `s` and exceed 3.5 by `s = 4096`, where an `n log n` mechanism would hold near `2·(1 + 1/log2 s) ≈ 2.2`.

Constructed test: demonstrated (results.md lines 1865-1933). `Display` on `WideTail(s, s)` reads 25,716 / 84,196 / 299,460 limb ops at `s` = 1024 / 2048 / 4096, each above `s · ⌈s/64⌉`, with doubling ratios 3.274 and 3.557 (an n log n mechanism would sit near 2.2).

### Fill and grow

### skyline-fill-grow-2: The distinct-minima memo forests' transient heap has no instrument, and the heap paragraph does not name the per-level accumulators
- Where: crates/before/src/version/skyline/fill.rs:121-134 (related: crates/before/src/version/skyline/fill/prescan.rs:105-117, crates/before/src/version/skyline/fill/prescan.rs:346-359, crates/before/src/version/skyline/fill/memo.rs:77-80, crates/before/src/meter/board/ops.rs:162-174, crates/before/tests/meter.rs:8376-8411, crates/before/tests/meter.rs:6746-6748, crates/before/src/meter/board/ceilings.rs:71-73, crates/before/src/lib.rs:333-340, crates/suanpan/src/accumulator.rs:103-154)
- Class / severity / confidence: claim / medium / medium
- Provenance: assessed (read; traced `PreScan::record` by hand over `memo_comb`'s layout (meter.rs:1048-1089): each single-leaf site `A_i` records at level `i` while `head_level = i − 1`, so it takes the suspend arm, and nothing resolves until `X_{d+1}` closes as `A_d`'s sibling, so the suspend stack reaches depth `d`; read `Accumulator`'s fields; read `memo_resolution_cost`'s `Run { input, touches }` and ops.rs's envelope-only roster; read `QueryEnvelope`'s `peak_heap` column and the `TICK_MIRROR_WIDE` row's shared-minimum comment); executed: no
- Seen by: claims (39), structure (30); refutation: reframed (the doc sentences reconcile by scoping "its frames" to `PreFrames`; the instrument gap stands; the per-byte constant is a layout derivation, not a measurement); history: no-rationale-found for the distinct-minima half (the shared-minimum per-site cost was cured in 5f6af246 and is board-judged as mirror-narrow; no commit, spec line, or ceiling treats the per-nonzero-link or per-suspended-level `Accumulator` struct cost)
- Owner-gated: yes (adding families to the board, or declaring a family model, is gate policy)

lib.rs:333-340 promises that transient space is "at most a small constant multiple of the input size" and that anything more "is a bug", and the board enforces `MAX_HEAP_BYTES_PER_INPUT_BYTE = 16.0` on its families. `PreScan::suspend` holds one `SuspendedLevel`, two `Accumulator` structs plus a slot and a level, per site-nesting level whose first child has recorded and whose forest parent has not closed; `Memo::links` holds one `Accumulator` per nonzero link. An `Accumulator` is roughly 96 bytes of inline state before any digit (`Option<i128>`, `Vec<i64>`, two `usize`, a `BTreeMap`). On `MemoComb` the suspend stack reaches depth `d`; on `MemoChain(distinct)` every link is nonzero; both families cost roughly 4 packed input bytes per site (the generators' own layouts: `14k + 9` and `10k + 8` bits for the chain, `~18d` and `14d + 12` for the comb). By layout that is on the order of 50-100 heap bytes per input byte. No committed instrument reads heap on these families: they are envelope-only (never on the board), and `memo_resolution_cost` reads the touch counter alone; the one tick row with a heap pin over a ledger shape (`TICK_MIRROR_WIDE`) is the shared-minimum shape where every link is zero and the suspend stack stays at depth one. The heap paragraph's "never an accumulator per open site-nesting level" is true of `PreFrames` but does not name the suspend stack that does hold them, and "stays flat on nested-site chains" holds for the chain (suspend depth ≤ 1), not for the comb. Instruments before cures: a space claim with no meter; asymptotic and space claims are hard guarantees per the crate docs.

Evidence:

       121	//! Heap: O(paired depth) transient frame *bits* plus O(n + m) total live
       122	//! digits; the memo holds one queue entry per covered site — an accumulator
       123	//! only where the link is nonzero, so sites sharing one minimum store nothing —
       124	//! plus one suspended entry per open site-nesting level.
       130	//! depth can grow stacker segments or overflow. The pre-scan parks no wide
       131	//! quantity per open site: a left-full site's raise decision belongs to the
       132	//! walk alone (the `prescan` module doc carries the argument), so its frames
       133	//! hold bits and unit deltas — never an accumulator per open site-nesting
       134	//! level — and the transient stays flat on nested-site chains.
    (prescan.rs)
       107	pub(super) struct SuspendedLevel {
       108	    /// The outer head's final value, `m_first(inner) − m_ref(outer)` —
       109	    /// immutable once pushed, both minima final.
       110	    head: Accumulator,
       111	    /// The outer level's sibling-chain keeper.
       112	    keeper: Accumulator,
    (tests/meter.rs)
      8376	    struct Run {
      8377	        input: u64,
      8378	        touches: u64,
      8379	    }
    (ops.rs)
       162	        // Envelope-only families never reach the board's product, so
       163	        // they have no designed diagonal.
       169	        | FamilyId::MemoChain
       170	        | FamilyId::MemoComb
    (lib.rs)
       335	//! shaped inputs, the auxiliary space required to compute any operation is at
       336	//! most a small constant multiple of the input size. In many cases, no scratch

Resolution: (1) Instrument first: add a `peak_heap` reading to `memo_resolution_cost`'s `tick_run` (the `PeakAlloc` harness the tick rows use) at two scales for `MemoChain(distinct)` and `MemoComb`, judged per input byte; or promote those two families to the board's tick group so `MAX_HEAP_BYTES_PER_INPUT_BYTE` judges them with their own `(event, id)` pair. (2) If the reading is red, either declare a family model at the constant the owner ratifies (as `ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE` does) or cure, measuring first: store narrow links and suspended heads and keepers at machine width (the `Boundary::Word | Wide(Accumulator)` trade `MinWeb` already makes), boxing only the wide ones. (3) Rewrite fill.rs:130-134 to state what holds: `PreFrames` holds bits and slot deltas; the suspend stack (`SuspendedLevel`) holds one head and one keeper accumulator per level with a recorded-but-unresolved first child, moved rather than copied, so their digits count toward the O(n + m) live total; and `Memo::links` holds one accumulator per nonzero link. Acceptance: a committed two-scale heap reading exists for both families under the board ceiling or a declared model whose derivation names the per-level struct cost; the heap paragraph names `SuspendedLevel` and no longer says "never an accumulator per open site-nesting level".

Construction: In tests/meter.rs under the `PeakAlloc` harness, tick `Shape::MemoComb.packed1(d)` × `Shape::MemoCombId.packed1(d)` and `Shape::MemoChain.packed_flagged(k, true)` × `Shape::MemoChainId.packed1(k)` at d, k ∈ {1000, 2000}; divide `peak_usage()` over the tick body by `v.encode().len() / 8 + id.bytes.len()`. Layout predicts about 216 bytes per `SuspendedLevel` × d on the comb at the moment `A_d` closes, plus about 96 bytes per nonzero link, against roughly 4 input bytes per site: a reading well above 16.0 that stays flat across the doubling. A word-compacted representation drops it by an order of magnitude; a representation boxing a second accumulator per entry doubles it.

Constructed test: demonstrated (results.md lines 3597-3686). Under `PeakAlloc`, ticking the memo comb peaks at 104.7 and 98.6 transient heap bytes per input byte (`d` = 1000, 2000) and the distinct-minima memo chain at 53.6 and 50.5 B/B, over the 8,192 B allowance, flat across the doubling and 3× to 6.5× over the board's 16 B/B ceiling. Whether the excess is the suspend stack or `Memo::links` was not separated.

### Comparison kernels

### skyline-sweep-place-masked-4: The masked-hole band's flatness rests on zero deltas in the skipped run; the docs attribute it to "accumulator work"
- Where: crates/before/src/version/skyline/masked.rs:313-316 (related: crates/before/src/version/skyline/overlay.rs:377-382; crates/before/src/meter.rs:146-156, 3506-3515; crates/before/tests/meter.rs:7515-7544; crates/suanpan/src/accumulator.rs:219-226, 718-722)
- Class / severity / confidence: claim / nit / high
- Provenance: assessed (read `skip_deeper`, `ev_spine`, and suanpan's `add_u64`/`sign`); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (a6385d5f's generator doc states the depth-independence; the zero-delta premise is stated nowhere)
- Owner-gated: no

The block skip removes per-interval sign reads, not folds: `skip_deeper` (overlay.rs:377-382) folds every skipped delta into `net`, and suanpan's `add_u64` returns before touching only when the delta is zero. The band reads flat across a spine doubling because `ev_spine`'s skipped right siblings are `ev_leaf(bits, 0)`; on a spine with nonzero deltas the touch reading grows with depth with the skip engaged. A maintainer reading "accumulator work is a function of the mask depth alone" takes the skip for an asymptotic mechanism; it is a constant-factor one, and the premise is stated nowhere near the claim.

Evidence:

       313	    /// The `masked_cmp_hole` envelope and its depth band (`tests/meter.rs`)
       314	    /// pin the skip engaging: on the masked-hole triple the comparison's
       315	    /// accumulator work is a function of the mask depth alone, flat across
       316	    /// a spine-depth doubling.

    src/meter.rs
       153	    for _ in 0..d - 1 {
       154	        ev_leaf(bits, 0); // each ancestor's right sibling
       155	    }

Resolution: state the pinned mechanism as "no per-interval sign read inside an unowned run" at masked.rs:313-316 and at the generator (src/meter.rs:3506-3515) and band (tests/meter.rs:7515-7517), and name the zero-delta premise at the generator. Optionally add a nonzero-delta spine variant whose reading is expected linear, as the fold floor. Acceptance: the prose names sign reads and the zero-delta premise; the band is unchanged.

Construction: replace the spine's right-sibling leaves with alternating 0/1 heights (still canonical); under the current code the touch reading grows with `d` at both band points with the skip engaged, so the flat ceiling fails though the skip works: the band pins sign reads, not folds.

### skyline-sweep-place-masked-5: `block_skip` re-peeks a stationary cursor's trailing run every round: Θ(L·r/64) word reads on Θ(r + L) input
- Where: crates/before/src/version/skyline/masked.rs:317-345 (related: crates/before/src/version/skyline/overlay.rs:347-354; crates/before/src/codec/stack.rs:104-123; crates/before/src/version/skyline/masked.rs:48-59; crates/before/src/version/own.rs:113-121; crates/before/src/lib.rs:350-358; crates/before/tests/meter.rs:7434-7544)
- Class / severity / confidence: claim / high / high
- Provenance: assessed (hand trace against the code and `BitStack::trailing_ones`, not run); executed: no
- Seen by: claims; refutation: confirmed (independent trace); history: no rationale found (stack.rs:106-107 prices each peek per call and admits the no-pop case; no record analyzes a stationary re-peeked cursor)
- Owner-gated: no

The masked cost claim (masked.rs:54-56: scan, decode, stack, and fold work all linear in the operand streams' bits) is a hard guarantee under lib.rs:350-358. On every round, for a masked side whose current region is unowned, `block_skip` evaluates `self.a.peek_flip()` (and the `b` twin) before comparing it to the other slots' depth. `peek_flip` is `len - trailing_ones()`, and `BitStack::trailing_ones` walks one word per 64 bits of the trailing right-branch run. When that side is not the deepest slot it does not step, so the same run is re-read on every round while the other side consumes its subtree. The extra work is a read of the path stack, so it is invisible to every committed deterministic meter (touch, heap, segments, limb, scan), and it is on the production path: `OwnVersion`'s comparisons route through `masked::causal_cmp` (own.rs:113-121).

Evidence:

       319	            let a_bound = self.others_deepest(Self::A);
       320	            if self.a_mask.as_ref().is_some_and(|mask| !mask.owned())
       321	                && self.a.peek_flip() > a_bound
       322	            {

    overlay.rs
       352	    pub(super) fn peek_flip(&self) -> u64 {
       353	        self.path.len() - self.path.trailing_ones()
       354	    }

    codec/stack.rs
       106	    /// One word read per 64 bits of the run: the cost is the run the caller is
       107	    /// about to pop (or has decided not to), never the whole stack. `u64`,
       ...
       115	        for &word in self.words.iter().rev() {
       116	            let w = word.trailing_ones();
       117	            run += u64::from(w);
       118	            if w < 64 {
       119	                break;
       120	            }
       121	        }

Resolution: guard each peek with the depth test the skip already implies: `self.a.depth() > a_bound && self.a.peek_flip() > a_bound` (and the `b` twin). Since `peek_flip() <= depth()`, the guard is a necessary condition and changes no step; when it holds, `a` is the unique deepest slot, so if the skip does not engage `advance_set` steps `a` this same round and pops exactly the run the peek read, and every peek is then amortized against an immediate pop. (Caching the flip level in `LeafCursor` at open and step time has the same effect.) State the amortization premise the caller must keep in `peek_flip`'s doc (overlay.rs:347-354). Acceptance: a committed two-scale band on the family below (fuel per packed byte under the deterministic wasm fuel meter, or the bench judge's wall exponent) reads flat across a doubling of `(r, L)`: red at HEAD, green after the guard; the existing `masked_cmp_*` rows and `masked_cmp_hole_depth_band` unmoved.

Construction: `a` is a skyline whose preorder reaches a leaf at path `[left, right × r]` (the last leaf of the left half): root internal, left child internal, each spine node with a left leaf sibling and a right internal child, heights alternating 0/1 so no sibling pair collapses (about 5r bits). `a_mask` is the party owning only the right half, so [0, 1/2) is one unowned region at depth 1. `b` is unmasked, with the same left-spine skeleton to depth r + 1 and then a dense subtree of L leaves inside `a`'s last left-half leaf [1/2 - 2^-(r+1), 1/2) (about 5r + 5L bits). Once the sweep enters `a`'s leaf, `a.path = [0, 1 × r]` (depth r + 1, trailing run r, `peek_flip() == 1`). For each of `b`'s L boundaries there, `block_skip` computes `a_bound = depth(b) > depth(a)`, the mask is unowned so `peek_flip()` runs and reads ceil(r/64) words, the compare fails, `b`'s own check is skipped (`b_mask` is `None`), and `advance_set` steps `b`. Total about L·r/64 word reads on about 10r + 5L bits: with r = L = n/15, Θ(n²/14400). With the guard the same family does Θ(n) work.

Constructed test: demonstrated (results.md lines 5187-5279). With a scratch counter on `trailing_ones`'s spilled-word loop, the finding's family reads 262,144 and 1,048,576 spilled words at `r = n` = 4096, 8192 (exactly `n·r/64`), ×4.000 on an input that grows ×2.000; the lockstep spine phase contributes nothing, so the whole reading is the parked-cursor re-scan, on the production `OwnVersion` comparison path.

### skyline-sweep-place-masked-14: A constant written inside big-O in the placement cost comparison
- Where: crates/before/src/version/skyline/place.rs:91-92 (related: crates/before/tests/meter.rs:9762-9766)
- Class / severity / confidence: claim / nit / high
- Provenance: assessed (read); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found
- Owner-gated: no

`O(2|v| + |s| + |e|)` equals `O(|v| + |s| + |e|)`, so the comparison is vacuous as written; the intended statement is a scan-bit count, which the meter row `span_place_scans_each_stream_once` states correctly as `fused + cmp_vv / 2 == cmp_vs + cmp_ve`.

Evidence:

        91	//! linear in the streams' bits — `O(|v| + |s| + |e|)` against the
        92	//! two-walk composition's `O(2|v| + |s| + |e|)` — and the per-interval sign

Resolution: write the comparison in bits scanned ("|v| + |s| + |e| bits against the composition's 2|v| + |s| + |e|") and keep one O() for the order. Acceptance: no O() expression in the partition carries a numeric constant.

Construction: not applicable; a notation defect settled by reading.

### skyline-sweep-place-masked-21: The filter walks' stated `O(|v| + Σ|bound|)` omits the per-interval factor k
- Where: crates/before/src/version/skyline/place/filter.rs:37-48 (related: crates/before/src/version/skyline/place/filter.rs:183-207, 359-367, 580-613; crates/before/src/version/skyline/overlay.rs:268-286; crates/before/src/causally/query.rs:91-96, 114-117; crates/before/src/causally/conjunction.rs:47-61; crates/before/src/causally/polarity.rs:98-107, 156-163; crates/before/tests/meter.rs:9472-10084)
- Class / severity / confidence: claim / medium / high
- Provenance: assessed for the derivation (read loops and `advance_set`; suanpan's `sign()` touches once on the quick register); verified for the instrument census (grep: the placement rows use at most two bounds, tests/meter.rs has no coverage call, and `Query::holes` is an uncapped `Vec` kept by `Query::and` for every pairwise-unabsorbed hole); executed: no
- Seen by: correctness, claims; refutation: confirmed; history: no rationale found (a736ef14 #17 added the sentence acknowledging O(#bounds) per-interval bookkeeping and left the total unchanged; no record treats the bound count as an input axis)
- Owner-gated: no

The read loop performs one `sign()` read per live pair per elementary interval of the (k+1)-stream overlay, the probe's step folds into every live pair, and `advance_set` makes two O(k) passes per boundary. The overlay has Θ(|v| + Σ|b_i|) elementary intervals, so the total is Θ(k · (|v| + Σ|b_i|)), not O(|v| + Σ|b_i|); the composed pair sweeps read Σ_i (|v| + |b_i|) = k|v| + Σ|b_i|, so on many large holes against a small probe the fusion is asymptotically worse on the bounds' term than the composition it is stated "against". The bound count is caller-controlled: `Query::and` keeps every pairwise-unabsorbed hole and pairwise-concurrent hole versions never absorb. The crate docs make every asymptotic claim a hard guarantee for all input sizes; the public `Query::contains` doc (query.rs:95-96, "One traversal of `version` and the stored bounds") inherits the denomination.

Evidence:

        43	//! and the per-interval sign reads ride the accumulator's amortized-O(1)
        44	//! collapse. The coverage walk additionally recomputes its endpoint-liveness
        45	//! flags and sweeps the settled flags once per interval — O(#bounds)
        46	//! bookkeeping absorbed by the same per-interval read loop. `O(|v| + Σ|bound|)`
        47	//! for membership, `O(|lo| + |hi| + Σ|bound|)` for coverage, against the
        48	//! composed sweeps' one probe decode per bound.

       184	        // One read per live bound per elementary interval, in demand order.
       185	        for slot in &mut walk.sides {
       186	            let Some(side) = slot else { continue };
       187	            side.pair.read();

Resolution: either (a) restate the bound accurately at filter.rs:37-48 and the public docs it feeds (scan bits O(|v| + Σ|b_i|); folds O(k|v| + Σ|b_i|); sign reads and advance bookkeeping O(k · (|v| + Σ|b_i|))), with the accurate comparison (the fusion saves k-1 probe decodes and pays a k factor on the bounds' boundaries), and pin the exponent in k with a two-scale touch row; or (b) restructure: read only pairs whose cursor stepped this round (a bound step changes one pair; a probe step changes all k, which the composition also pays; `Directions::fold` is idempotent on an unchanged sign), and pick the deepest slot with a heap keyed by depth, reaching Θ(k|v| + Σ|b_i| · log k); the single-bound identity rows are unmoved because every interval of a two-stream overlay is dirty. Acceptance: a committed tests/meter.rs row builds k and 2k `Demand::NotBefore` holes with pairwise-disjoint leaf-boundary sets against the probe `Version::new()` (below every hole, so nothing exits early), measures `touch_ops` over `filter::admits`, and asserts the ratio sits under the stated exponent, with a liveness floor of k touches (one read per bound on the first interval).

Construction: k holes, each an oracle tree with a B-leaf shape under the i-th depth-ceil(log2 k) dyadic prefix and zero leaves elsewhere (Σ|h_i| about kB; the joint overlay has about kB elementary intervals); probe `Version::new()`; demands all `NotBefore`. Run `filter::admits` at k and 2k with B fixed and read `touch_ops`. The fused walk reads k live pairs per interval, about k · kB touches, quadrupling on the doubling; the composed baseline Σ_i `sweep::causal_cmp(v, h_i)` reads about kB, doubling. A fused ratio near 4 refutes `O(|v| + Σ|bound|)`.

Constructed test: demonstrated (results.md lines 3687-3781). With `k` concurrent 64-leaf holes and the empty probe, fused `contains` touches grow ×3.96 when `k` doubles at fixed hole size; the composed per-hole sweeps grow ×2.14 alongside the bounds' bits (×2.15); scan bits stay linear.

Synthesis note: Same `k` factor as span-causally-24, seen at the kernel's own cost derivation rather than the public docs; the constructed tests differ (empty probe with dyadic holes here; a wide probe with `until` holes there) and agree.

### Query

### skyline-query-13: Shift panic-freedom argued from a storage cap the codec denies
- Where: crates/before/src/version/skyline/query/integral.rs:464-471 (related: query/web.rs:136-141; codec/bits.rs:117-119; codec/buf.rs:25-31; overlay.rs:114-116)
- Class / severity / confidence: claim / low / high
- Provenance: verified (`grep -rn "caps below 2^32" crates/before/src` finds only these two comments); assessed (read the three codec statements); executed: no
- Seen by: correctness; refutation: confirmed; history: deliberate-but-expired (7ea3df58 wrote both comments while the borrowed view's 32-bit length encoding still capped walks; 5d167a63 and 83e61b4d removed the cap the same day and 05d87e1b states the new fact, without re-denominating these two comments)
- Owner-gated: no

Both comments prove the accumulator shifts cannot reach the documented `usize` panic on the premise "the storage caps below 2^32", and the codec states the opposite: `Bits::freeze` "imposes no bound of its own", `BitsBuf` says "allocatable memory is the only bound anywhere on the build path", and `PlateauCursor::depth` says live lengths outgrow a 32-bit `usize` from 512 MiB. The conclusion survives on the achievable bound (on a 32-bit target allocatable memory keeps the stream under 2^35 bits, so digit positions stay under 2^30 against the accumulator's 2^32-digit `usize` bound, a margin of two binary orders, not "multiple"; on 64-bit `usize` is 64 bits wide and the panic is unreachable outright). A panic-freedom argument is a one-line proof (Principle 1), and this one's premise is false.

Evidence:

    464      // Every accumulator shift in this module — the `32 * index` digit
    465      // routings here, the interval weights and segment scales below — is
    466      // bounded by the walked stream's own content: digit indexes by a
    467      // value's width over 32, weights by the tree's depth, both under the
    468      // stored stream's bit length, which the storage caps below 2^32. The
    469      // shifted entry points' documented panic (a digit position past
    470      // `usize`, from shift 2^37 on a 32-bit target) therefore sits multiple
    471      // binary orders of magnitude beyond anything this fold can feed it.

    (bits.rs:117-119)
    117      /// Exact at every size on every target: lengths and positions are `u64`
    118      /// on both sides of this seam, so an emission is storable whenever its
    119      /// buffer is allocatable — the door imposes no bound of its own.

Resolution: Restate the premise at both sites in the codec's terms: the shift is bounded by the stream's bit length, itself bounded by allocatable memory, under 2^35 bits on a 32-bit target, so a digit position stays under 2^30; on 64-bit targets the documented panic is unreachable. Drop "multiple". Acceptance: the grep returns nothing and both comments name the bound bits.rs states.
Construction: Textual: the premise is contradicted by bits.rs:117-119 and buf.rs:25-31 as written. For the runtime side, on a 64-bit target a right spine of 2^31 unit leaves (about 1.6 GiB of stream) passes `Version::from_bits` and every shift in `rank` and `min_ticks` still fits, which is the correct argument the comment should carry.

### skyline-query-28: Backend multiplication-tier thresholds restated as literals with no anchor to the dependency pin
- Where: crates/before/src/version/skyline/query/tests.rs:944-948 (related: query/tests.rs:1142-1143; query/integral.rs:220-221; suanpan/src/lib.rs:296-297)
- Class / severity / confidence: claim / nit / medium
- Provenance: assessed (read dashu-int 0.5.0's `mul/mod.rs:16, 23` (`THRESHOLD_SIMPLE_DEFAULT = 24`, `THRESHOLD_KARATSUBA_DEFAULT = 96`) and `mul/ntt/mod.rs:29` (`THRESHOLD_NTT = 4_000`) in the cargo registry; Cargo.lock holds dashu-int 0.5.0; suanpan's doc pins "dashu-int 0.5" and calls a bump a breaking change); executed: no
- Seen by: claims, correctness (open question); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The three dispatch thresholds appear as literals here, at tests.rs:1142-1143, and in the integral doc's "4,000-word operand sides". They are correct today, but nothing ties them to the pin: after a dashu bump that moves a threshold, both tier-boundary tests stay green (they are value tests) while no longer straddling any dispatch boundary, and integral.rs:220 becomes false with no red anywhere. Principle 5's hand-maintained-count rule applied to a dependency's internals.

Evidence:

    945      // dashu 0.5 dispatches on the smaller side in 64-bit words: simple ≤ 24,
    946      // Karatsuba ≤ 96, Toom-3 ≤ 4,000, NTT above. One width at each threshold
    947      // and one past it, in base-2^32 digits.
    948      for words in [24usize, 25, 96, 97, 4_000, 4_001] {

Resolution: Name them once as a test-module constant (for example `DASHU_05_MUL_TIER_WORDS: [usize; 3] = [24, 96, 4_000]`) with a comment tying them to suanpan's dashu 0.5 pin and the bump procedure; have `dense_factor_tier_legs` and the tier-boundary loop derive their widths from it, and have the integral doc cite the constant by name. Acceptance: one definition site for the thresholds.
Construction: Bump dashu-int past a release that moves its Toom-3 threshold: `clustered_charge_agrees_at_backend_tier_boundaries` and `dense_factors_agree_through_the_public_fold_at_tier_boundaries` still pass while straddling no dispatch boundary.

## The codec

### Bits

### codec-bits-2: The ladder essay's "order of magnitude" ratio is a number no instrument measures
- Where: crates/before/src/codec/bits.rs:22-29 (related: crates/before/src/codec/bits.rs:426-428; crates/before/tests/meter.rs:10552)
- Class / severity / confidence: claim / nit / medium
- Provenance: verified for the site (`grep -rn 'order of magnitude' crates/before/src` finds bits.rs:25 and the `canonical_hash` doc at 426-428 only); assessed for the instrument (the `identity_fast_paths` module pins zero walk work, not a ratio); executed: no
- Seen by: claims [40]; refutation: confirmed; history: already-known (3d976922 de-numbered `canonical_eq`/`canonical_hash` and deliberately kept `canonical_hash`'s phrase with a mechanism clause; the essay's phrase predates that commit and was not swept)
- Owner-gated: no (the `canonical_hash` phrase is the ruled site and is left alone here)

The rule deciding which sites take the `memcmp` rung rests on "roughly an order of magnitude cheaper per bit than a decoding walk"; the two named instruments pin zero walk work on adopted rungs and measure no ratio. A number in prose is a hypothesis unless it names its measurement, and 3d976922's own reasoning ("a plausible code change could falsify either without any committed check firing") applies here.

Evidence:

        22	//! - **The `memcmp` rung pays for itself exactly where the walk it
        23	//!   replaces is expensive relative to a byte scan.** A miss costs an
        24	//!   early-exiting byte compare over the operands' shared prefix —
        25	//!   byte-parallel, roughly an order of magnitude cheaper per bit than
        26	//!   a decoding walk — and a hit deletes the walk whole. The

Resolution: Restate as mechanism ("byte-parallel, with no decode per bit") or cite a bench cell comparing a `canonical_eq` miss to `causal_cmp` on byte-equal operands. Acceptance: the sentence makes no quantitative claim or names a committed measurement.

### codec-bits-10: with_capacity docs promise a past-address-space hint "allocates nothing up front", which holds only where usize::try_from fails
- Where: crates/before/src/codec/buf.rs:68-78 (related: crates/before/src/codec/build.rs:61-72)
- Class / severity / confidence: claim / nit / high
- Provenance: assessed (read both bodies; `usize::try_from::<u64>` is total on 64-bit targets, and `Vec::with_capacity` panics on capacity overflow or aborts on allocation failure); executed: no
- Seen by: claims [41]; refutation: confirmed (every production hint is a sum of live lengths, so no input reaches the corner); history: no-rationale-found (83e61b4d wrote both sentences for the 32-bit boundary it cured)
- Owner-gated: no

On a 64-bit target the `unwrap_or(0)` never fires, and an unallocatable request panics or aborts rather than allocating nothing; even on 32-bit a byte count in `(isize::MAX, usize::MAX]` panics with a capacity overflow. The sentence overstates the mechanism.

Evidence:

        68	    /// An empty buffer with room for `bits` bits before reallocation.
        69	    ///
        70	    /// The capacity is a hint: a request past the target's address space
        71	    /// allocates nothing up front, and the buffer still grows to whatever
        72	    /// the pushes actually demand.
        73	    pub(crate) fn with_capacity(bits: u64) -> Self {
        74	        BitsBuf {
        75	            bytes: Vec::with_capacity(usize::try_from(bits.div_ceil(8)).unwrap_or(0)),

Resolution: State what the code does ("a request that does not fit `usize` allocates nothing up front; callers pass hints bounded by their operands' live lengths"), or clamp the hint if the no-op semantics are wanted. Acceptance: both sentences match the code's behavior on both target widths.

### codec-bits-27: BitStack's type doc claims every operation is O(1); all_set and trailing_ones are scans
- Where: crates/before/src/codec/stack.rs:13-17 (related: crates/before/src/codec/stack.rs:104-123, 176-184; crates/before/src/version/skyline/build.rs:301-304)
- Class / severity / confidence: claim / nit / high
- Provenance: assessed (read both scans; `all_set`'s only production use is the debug assert at skyline/build.rs:302); executed: no
- Seen by: correctness [29], claims [33]; refutation: confirmed; history: no-rationale-found (b3f09baa wrote the sentence after both scans existed)
- Owner-gated: no

`all_set` iterates every spilled word, O(len / 64); `trailing_ones` reads one word per 64 bits of the run and its own doc prices it so. Complexity statements in this crate are hard guarantees, and a false O(1) on a primitive is what lets a caller treat a run-proportional read as free (codec-bits-29).

Evidence:

        13	/// A pop-able stack of bits over machine words.
        14	///
        15	/// The newest bit lives at the low end of the top register; a filled register
        16	/// spills whole into the word vector and refills on the pop that crosses back.
        17	/// Every operation is O(1) with no bit-addressing arithmetic.

Resolution: "`push`, `pop`, `last`, and `set_last` are O(1); the run scans (`trailing_ones`, `all_set`) are priced where they are declared." Acceptance: the type doc names no operation as O(1) that is not.

### codec-bits-29: peek_flip re-scans the path's trailing run on every call; parked cursors in the masked and projection walks pay O(run / 64) per step of the other operand, outside every meter
- Where: crates/before/src/codec/stack.rs:104-108 (related: crates/before/src/version/skyline/overlay.rs:347-354, 377-382; crates/before/src/version/skyline/masked.rs:48-59, 317-357; crates/before/src/version/skyline/query.rs:533-549; crates/before/tests/meter.rs:13-33; crates/before/src/meter/registry.rs)
- Class / severity / confidence: claim / medium / medium
- Provenance: assessed (read the call path: `masked::advance` (349-357) calls `block_skip` on every advance; `block_skip` (317-345) evaluates `self.a.peek_flip()` whenever the a-mask is unowned, before the `> a_bound` compare can fail; `peek_flip` (overlay.rs:352-354) is `path.len() - path.trailing_ones()`; `trailing_ones` (stack.rs:109-123) reads one spilled word per 64 trailing ones; the projection loop (query.rs:536-549) peeks once per unowned outer iteration; `grep record_bits crates/before/src/codec/stack.rs` is empty); executed: no
- Seen by: claims [31]; refutation: confirmed (the construction was not run; the term is established from the code); history: no-rationale-found (63b2f1b5 introduced `peek_flip` as "read without moving" with no amortization argument; masked.rs's cost derivation counts pushes and pops only)
- Owner-gated: no

`trailing_ones` prices itself as the run the caller is about to pop, but `peek_flip` reads it without moving, and two walks evaluate it once per step while the cursor is parked. A cursor parked at a leaf whose path ends in `r` right branches (`r > 64`) costs about `r / 64` word reads per peek; if the other operand takes `n` steps inside that leaf's interval, the walk does `Theta(n * r / 64)` word reads on inputs of `Theta(r + n)` bits. masked.rs:52-55 derives `O(|v| + |p| + |w|)` from "every path bit pushed and popped at most once", which does not cover these non-popping reads; none of the board's deterministic meters (scan bits, peak heap, stack segments, limb ops) counts a stack-word read, and the `FamilyId` roster has no parked-deep-run times wide-neighbor family (the `MaskedHole` shape is the amortized case: peek, then pop). Crate docs (lib.rs:351-353): any asymptotic claim is a hard guarantee for all input shapes.

Evidence:

       104	    /// The exact run of set bits at the top of the stack.
       105	    ///
       106	    /// One word read per 64 bits of the run: the cost is the run the caller is
       107	    /// about to pop (or has decided not to), never the whole stack. `u64`,
       108	    /// as [`len`](Self::len): the run is bounded by the stack's own height.
    (masked.rs)
       319	            let a_bound = self.others_deepest(Self::A);
       320	            if self.a_mask.as_ref().is_some_and(|mask| !mask.owned())
       321	                && self.a.peek_flip() > a_bound
    (overlay.rs)
       352	    pub(super) fn peek_flip(&self) -> u64 {
       353	        self.path.len() - self.path.trailing_ones()
       354	    }

Resolution: Cache the flip level in `LeafCursor`: recompute `len - trailing_ones()` once after each `descend`/`step` (that one scan is bounded by the run the next `step` pops, so it amortizes to O(1) per plateau) and have `peek_flip` return the cached `u64`. Then (a) add the peek to masked.rs's cost argument and to `trailing_ones`'s doc ("callers that peek repeatedly must cache"); (b) register the dual family (a parked right run of depth `r` in one operand, `n` plateaus inside its interval in the other, the a-mask unowned there) for `masked_cmp` and `project`; (c) give the board a currency that sees it, either `scan::record_bits_u64(64 * words)` inside `trailing_ones` or the bench judge's two-scale ratio on the new cells. Acceptance: a committed test builds the dual shape at `(r, n)` and `(2r, 2n)` and asserts the walk's stack-word reads stay within a flat per-input-bit band across the doubling (about 4x today, about 2x with the cached flip level); the masked and projection cost arguments name the peek and its amortization.
Construction: version `v`: root = node(left = L1, right = leaf h = 0); L_k = node(left = leaf h = 1, right = L_{k+1}) for k = 1..r, with L_{r+1} = leaf h = 0 (every sibling pair is leaf/internal or of distinct heights, so canonical). v's preorder leaves have paths 00, 010, ..., 0 1^(r-1) 0, then the parked leaf 0 1^r (r trailing ones), then 1. Party `p` = "(0, 1)" (unowned on [0, 1/2)). Version `w`: the same spine to depth r + 1 with L_{r+1} replaced by a subtree of n leaves with alternating heights 0/1, all inside v's parked leaf's interval. Run `(&v / &p).partial_cmp(&w)`: after r lockstep advances `a` is parked and `b` advances n - 1 times; each advance runs `block_skip`, whose first conjunct is true, so `self.a.peek_flip()` scans about r / 64 words and then compares false against `a_bound >= r + 1`. Count words read in `trailing_ones` (a temporary counter) at r = n = 8192 and r = n = 16384 against input bits; expect the ratio to approach 4 rather than 2. For `project`, replace `w` by a party alternating owned/unowned across n leaves inside the same interval and call `(&v / &p2).to_version()`.

Constructed test: demonstrated (results.md lines 5425-5461), sharing the construction above; the projection-loop variant (`(&v / &p2).to_version()`) was not constructed.

Synthesis note: Same mechanism as skyline-sweep-place-masked-5 from the codec's side (the primitive's pricing is correct per call; the caller's derivation omits the non-popping reads); the two resolutions (guard the peek; cache the flip level) are alternatives, not both required.

### Base, text, and tree

### codec-base-text-tree-13: The `Party` literal door's public rustdoc claims `O(n)`; per-level copying and `validate_id` make a nested literal quadratic
- Where: crates/before/src/codec/literal.rs:52-66 (related: crates/before/src/party.rs:838-843, crates/before/src/party.rs:897-899, crates/before/src/clock.rs:960-962, crates/before/src/codec/tree.rs:104-124)
- Class / severity / confidence: claim / medium / high
- Provenance: assessed (read `id_node`, `PartyLiteral for (T, S)`, and the two `# Complexity` sections; the quadratic reading is by inspection of the per-level copy and validate, not measured); executed: no
- Seen by: claims; refutation: confirmed (and extended to the `Clock` tuple door); history: no rationale found (the `O(n)` sentence is Phase 7's and `id_node` has validated per level since the same commit; 3bba6cbb re-denominated it to bytes without re-deriving it)
- Owner-gated: yes: the `# Complexity` sections are public rustdoc, and before's crate docs make complexity claims hard guarantees

This is a boundary finding: the multiplier (`validate_id`) is in this partition, the caller and the claim are in `literal.rs` and `party.rs`. `PartyLiteral for (T, S)` builds bottom-up, calling `id_node` at every nesting level; each `id_node` copies both children into a fresh buffer and runs `validate_id`, a full `parse_id` walk, over the assembled subtree. For a left-spine literal of depth d, level k holds Θ(k) bits, so the copy sum and the validate sum are each Σ_{k≤d} Θ(k) = Θ(d²) while n = Θ(d): quadratic in the claimed denomination. Depth is bounded by the compiler's recursion limit on the tuple type, not by infeasible work, so the doctrine's 2^64-corner exception does not apply. The per-level `validate_id` also re-parses children that were each validated when built, so its only new information is the `(0, 0)`/`(1, 1)` check the two lines above it already made. `Clock::try_from((party_literal, version))` inherits the same shape through its party half.

Evidence:

        59	    let mut b = BitsBuf::with_capacity(2 + l.len() + r.len());
        60	    b.push(!l.is_empty()); // bit 0 = left present
        61	    b.push(!r.is_empty()); // bit 1 = right present
        62	    b.extend_from_buf(l);
        63	    b.extend_from_buf(r);
        64	    validate_id(super::buf::built_view(&b))?;
        65	    Ok(b)

    (party.rs)
       838	impl<T: PartyLiteral, S: PartyLiteral> PartyLiteral for (T, S) {
       839	    fn into_id_bits(self) -> Result<codec::BitsBuf, Parse> {
       840	        let l = self.0.into_id_bits()?;
       841	        let r = self.1.into_id_bits()?;
       842	        codec::id_node(&l, &r) // assembles + validates normal form
       843	    }
       844	}

       897	/// # Complexity
       898	///
       899	/// `O(n)`, `n` the built party's size in bytes.

    (clock.rs)
       960	/// # Complexity
       961	///
       962	/// `O(n)`, `n` the built clock's size in bytes.

Resolution: Owner's choice between (a) restating both rustdocs as `O(n · d)` with d the literal's nesting depth and dropping the per-level `validate_id` (children are normal by construction; the two collapsibility checks are the whole normal-form rule at a node), leaving only the copying; or (b) reshaping the sealed, `#[doc(hidden)]` `PartyLiteral` to emit top-down into one shared `BitsBuf` (reserve the tag, emit children, patch the tag, exactly `text.rs`'s `parse_id_tree` discipline), validating once at the root, and keeping `O(n)`. Recommend (b): the trait is sealed, so it changes no reachable surface, and the claim stays true. Acceptance: under `scan-meter`, `before::meter::scan_bits()` across `Party::try_from(spine(d))` at d = 32 and d = 64 reads a ratio near 2, or the rustdoc says `O(n · d)` and the ratio near 4 is the documented behavior; either way `id_node` no longer re-parses a subtree its own two checks already classified.

Construction: Build a left-spine literal by macro (`((…((1u8, 0u8), 0u8)…), 0u8)`) at depths 32 and 64 (inside the default recursion limit). Under `--features scan-meter`: `before::meter::reset_scan_bits(); Party::try_from(spine).unwrap(); let bits = before::meter::scan_bits();`. Every `validate_id` runs `parse_id` over a `DsiCursor`, whose `read_bit` records each bit (dsi.rs:180 `super::scan::record_bits(1);`), so the reading grows as Σ_{k≤d} (2k + 2): the 64/32 ratio reads near 4, not near 2.

Constructed test: demonstrated (results.md lines 580-633). Doubling a left-spine `Party` literal's depth from 32 to 64 grows the packed party ×1.89 (9 to 17 bytes) and the scanned bits ×3.83 (1,120 to 4,288).

## Cross-cutting: fold and shape

### crate-root-17: fold.rs states operand-size balance the counter does not provide
- Where: crates/before/src/fold.rs:5-8 (related: crates/before/src/fold.rs:55, crates/before/src/meter/board/ceilings.rs:247-250)
- Class / severity / confidence: claim / low / high
- Provenance: verified (the counter merges on `*w == weight`, equal input counts, at line 55; nothing bounds packed-size ratios; ceilings.rs:247-250 states the argument the `O(D log k)` bound actually rests on: "every input passes through `O(log k)` joins, and each join level re-scans the operands it merges"); executed: no
- Seen by: claims; refutation: confirmed; history: no-rationale-found (lines 5-8 are from the b3f09baa0 squash, not the fold's author commit)
- Owner-gated: no

Every bound needs an argument the code implements. The sentence offers one (bounded operand-size ratio) the code does not implement, while the correct one (each counter level's groups partition the inputs, so per-level work is `O(D)`) is what the board's fold ceiling and the `*_log_factor_is_alive` pins rest on. A maintainer reading fold.rs alone would defend the wrong invariant.

Evidence:

     5  //! An incoming operand merges upward while the top stack entry holds as many
     6  //! inputs as it does, so every input passes through `O(log k)` combines against
     7  //! similarly sized partners and no combine's operand is more than a bounded
     8  //! factor larger than its partner. A sequential left fold instead combines

Resolution: "so each input passes through `O(log k)` combines, each pairing two groups holding equally many inputs; because the groups at any counter level partition the inputs, one level's combines cost `O(D)` in total packed size and the whole fold `O(D log k)`." Acceptance: the cost argument mentions input-count balance and per-level partition only; no claim about operand packed-size ratio remains.
Construction: Two inputs, a one-leaf version and `Shape::Dense.packed1(125_000)`: `balanced_reduce` performs exactly one combine whose operands differ in packed size by about five orders of magnitude, contradicting the "bounded factor" clause while the `O(D log k)` bound holds trivially at `k = 2`.

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

Synthesis note: Same clause as crate-root-17; the sweep's construction (`[H, t, t, ...]`) and its naming of the missing output-size premise (version-core-5) are what the two entries add to each other.

### crate-root-37: Shape walks allocate an O(depth)-bit path stack past 64 levels; `Party::shape` says nothing allocates
- Where: crates/before/src/shape.rs:22-25 (related: crates/before/src/party.rs:480-481, crates/before/src/version.rs:738-740, crates/before/src/version/skyline/overlay.rs:317-324, crates/before/src/version/skyline/overlay.rs:471-478, crates/before/src/codec/stack.rs:48-56)
- Class / severity / confidence: claim / low / high
- Provenance: verified (overlay.rs:317-320 `LeafCursor` holds `path: BitStack` and 471-477 `IdLeafCursor` two `BitStack`s; codec/stack.rs:48-53 `push` spills `top` into `words: Vec<u64>` when `top_len == 64`, the first heap allocation of a `Vec::new()`, on the 65th live bit; `grep shape crates/before/tests/meter.rs` finds no shape-drain scenario; the depth-65 construction is assessed, not run); executed: no
- Seen by: claims; refutation: confirmed; history: no-rationale-found (party.rs:480-481 and shape.rs:22-25 are from 46eb64f9; the path stacks and the spill predate the shape module; no commit prices the walk's auxiliary stack)
- Owner-gated: no

Space claims are hard guarantees per lib.rs:333-340, and the doctrine judges by the clause, not the likelihood of a 65-deep party. The walk's auxiliary space is a bounded `O(depth)` bits (within "a small constant multiple of the input"), so the contract holds; the false clause is "nothing allocates", and no heap-metered row pins any shape drain.

Evidence:

    22  //! Every walk borrows its value and streams in place: nothing is
    23  //! materialized up front, draining is linear in the value's encoded
    24  //! size, and an item allocates only when its rise magnitude exceeds two
    25  //! machine words.

    party.rs:
   480      /// Draining the iterator is linear in the party's encoded size: each
   481      /// region costs `O(1)`, and nothing allocates.

    codec/stack.rs:
    48      pub(crate) fn push(&mut self, bit: bool) {
    49          if self.top_len == 64 {
    50              self.words.push(self.top);

Resolution: State the walk's auxiliary space in the module doc (one path stack per input, `O(depth)` bits, heap-resident past 64 levels, freed at drop) and amend party.rs:481 to "allocates nothing per item" (likewise version.rs:738-740 and the clock door if they carry the clause). Add one heap-metered scenario per shape door to tests/meter.rs on the deep spine families so the auxiliary-space claim has a pin. Acceptance: under `PeakAlloc`, draining `deep_left_spine_party(64).shape()` reads zero heap delta and `deep_left_spine_party(65).shape()` a nonzero one; the prose states the `O(depth)`-bit path stack; a committed envelope row bands the deep-spine drains.
Construction: In a `PeakAlloc`-instrumented test: `let p = deep_left_spine_party(65); reset peak; p.shape().count(); assert_eq!(peak_delta, 0)` fails: `IdLeafCursor::open` pushes 65 path bits and `BitStack::push` allocates its first `words` entry at the 65th push.

## suanpan

### suanpan-4: The ledger argument's "a nonzero partial decides within one step" is false in general
- Where: crates/suanpan/src/lib.rs:143-146 (related: crates/suanpan/src/accumulator.rs:838-842 (same sentence), 138-142 (the precise statement))
- Class / severity / confidence: claim / low / high
- Provenance: verified (arithmetic: partial 1 over digit `-(2^32 - 1)` yields `2^32 - 2^32 + 1 = 1`; the digit is in the zone and constructible: `sub_magnitude_shl(&UBig::from((1u64 << 32) - 1), 32 * i)` routes through `add_shifted_word` to `add_at`, which stores `|total| < 2^33` as-is); executed: no
- Seen by: claims; refutation: confirmed (as nit); history: no rationale found (entered with f7596470; 5f16e2e5 restated the precise mechanism at the field doc and left these two sites)
- Owner-gated: no

A running partial `s` with `|s|` in {1, 2} does not decide at the next digit whenever that digit is near `-s * 2^32`, and the descent can continue through arbitrarily many such digits. What is true, and what the conclusion needs, is that a nonzero partial decides within one step over a zero digit (`|s * 2^32| >= 2^32 >= 3`), which is why a fold cannot enter a certified run carrying value. The field doc at accumulator.rs:138-142 states exactly that. A derivation the crate presents as complete should not contain a false intermediate step (statement faithfulness).

Evidence:

       143	//! reaches a certified run consumes the certificate and skips to `lo` whole,
       144	//! one touch instead of one per digit; the sign fold does the same when its
       145	//! running partial is zero (a nonzero partial decides within one step, so a
       146	//! fold never walks into a certified run while carrying value). A write whose

    accumulator.rs:
       840	    /// skips certified zero runs whole — a nonzero partial decides
       841	    /// within one step, so the fold never walks into a certified run
       842	    /// while carrying value. The rewrite is value-preserving: the

Resolution: at both sites, "a nonzero partial decides within one step over a zero digit, so a fold never walks into a certified run while carrying value". Acceptance: both sentences mention the zero digit. Construction (a witness worth adding to witnesses.rs if the small-partial descent is not already pinned): build digits `[1 at index k, -(2^32 - 1) at each of k-1..1]` via `sub_magnitude_shl` for `i in 1..k` then `add_magnitude_shl(&UBig::ONE, 32 * k)`; `sign()` descends k digits with partial exactly 1 at every step, deciding only at digit 0; every step is over a nonzero digit, refuting the clause, and no certified run is entered, so the conclusion stands.

## The instruments

### Meter core, registry, tier2

### meter-core-8: The `wide_arming` and `hoisted_window` guard admits widths at which the promotion their docs describe cannot fire, and the committed hoisted-window band runs at one
- Where: crates/before/src/meter.rs:1904-1912 (related: meter.rs:1885-1889, 1931-1933, 1952-1963, 1683-1689, 2480-2489; query.rs:201-225; integral.rs:846-849, 858-887; suanpan accumulator.rs:942-947; meter/tests.rs:960-961, 993-994; tests/meter.rs:5306-5332, 5186-5220)
- Class / severity / confidence: claim / medium / medium
- Provenance: assessed (hand trace of `query::rank`'s fold through `Integrator::boundary` and `freeze` for `wide_arming(w, d)`: the first freeze parks `2^(32w)`, `w + 1` digits; the second freeze's drift is `2^288 + 1`, ten digits; integral.rs:880 promotes only when `w + 1 > 10 + 8`, so `w >= 18`); executed: no
- Seen by: refutation pass (raised as new); refutation: raised with the same trace; history: not examined
- Owner-gated: no for the guard and docs; the `hoisted_window` band's width is the envelope partition's pin

The generator doc says the block "climbs `2^288` (whose unit's freeze finds the parked component over-wide and promotes it — the one ledger arming)", and the `# Panics` rationale says `w >= 10` is "the parked component must clear the settling drift's ten digits by more than the freeze allowance". That rationale describes `w + 1 > 18`, that is `w >= 18`; the number 10 is instead the width at which the arming's own unit code trips the freeze (`w + 1 > 9`). By the trace, for `10 <= w <= 17` the sweep freezes twice and never promotes, so the mechanism the family exists to price is not realized at parameters the guard admits. `hoisted_window` inherits the guard and the claim (1931-1933, 1954-1955), and the committed `hoisted_window` band runs at `HOISTED_WINDOW_WIDTH = 12` (tests/meter.rs:5332), where by this trace no promotion fires; the `ledger_wide_arming` band runs at `w = 500` and `1000`, inside the promoting range. The sibling constants agree with the rule: `PROMOTION_REARM_ARM_BITS` is twenty digits (1685-1688, "more than the ... allowance above the settling drop's ten") and `arming_train` guards `w >= 19` with the same rationale (2480-2489). No promotion counter exists to settle this in a run, and the meter/tests.rs pins at `(10, 1)` and `(12, 5)` check length and `min_ticks` only, both regime-independent. A claim in a generator's doc is a statement of record for every band denominated on it; this one is contradicted by the promotion rule as read.

Evidence:

      1885	/// One promotion whose parked mass is as wide as the input, owing its debt
      1886	/// across a trailing mass as dense as the input. Exactly `134d + 64w + 600`
      1887	/// bits. The one block climbs `2^(32w)` (parked at its unit), climbs `2^288`
      1888	/// (whose unit's freeze finds the parked component over-wide and promotes it —
      1889	/// the one ledger arming), and the sweep then consumes the `Θ(d)`-dense

      1906	/// Panics if `w < 10` (the parked component must clear the settling drift's ten
      1907	/// digits by more than the freeze allowance) or `d == 0`.
      1908	fn wide_arming(w: usize, d: usize) -> Packed {
      1909	    assert!(
      1910	        w >= 10,
      1911	        "the wide arming must out-span the settling drift plus the allowance"
      1912	    );

    integral.rs
       880	        if self.parked.digit_count() > base_digits(&drift) + FREEZE_ALLOWANCE_DIGITS {
       881	            self.promote();

    tests/meter.rs
      5332	    const HOISTED_WINDOW_WIDTH: usize = 12;

Resolution: Add a `cfg(test)` promotion tap beside `FREEZE_HITS` (integral.rs:284) and a meter/tests.rs pin that `wide_arming(w, d).version().rank()` promotes exactly once for `w >= 18` and zero times at `w = 17`; then either raise both guards to the promotion threshold, derived from `FREEZE_ALLOWANCE_DIGITS` once meter-core-7 makes it reachable, and hand the envelope partition a re-parameterized `hoisted_window` band at a promoting width, or re-state both docs and the band prose to the freeze-only mechanism the current widths realize. Acceptance: the promotion pin exists and passes; the guard's number and its rationale describe the same threshold; the `hoisted_window` band's width sits on the documented side of it.

Construction: With the tap in place, run `wide_arming(12, 5).version().rank()` under `cargo nextest run -p before --lib` and read the tap: zero promotions, two freezes. Run `wide_arming(18, 5)`: one promotion. Without the tap, the same fact is visible by instrumenting `Integrator::promote` with an `eprintln!` for one local run.

Constructed test: demonstrated (results.md lines 769-839). With a `cfg(test)` counter on `Integrator::promote`, `rank` on `wide_arming(w, d)` records zero promotions for `w` = 10, 12, 17 and exactly one (one ledger arming) from `w` = 18 on; `HoistedWindow(12, 40, 1024)` records zero. `FREEZE_HITS` reads three freezes per run, not the two the entry's trace states.

Synthesis note: The tap read three freezes per run where the entry's trace says two; the promotion conclusion (none below `w` = 18, one from 18 on, zero at the committed `hoisted_window` width) is exactly as the entry states.

### meter-registry-tier2-10: Reason strings name enforcement homes that do not hold the named pin or documentation
- Where: crates/before/src/meter/registry.rs:1108-1109 (related: registry.rs:1382, 1415, 1426, 1628-1636, 1642-1647, 1656-1666, 1673-1683; crates/before/tests/meter.rs:468-476, 483-485, 503-505, 529-531, 2688-2721, 2829-2852, 4504-4523, 6569-6585, 6637-6641, 6670-6673; crates/before/src/meter/board/family.rs:564-603)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read every `fn tick_*` operand at tests/meter.rs:468-560 and the ticks bands at 728-745, 888-900; read `rank_wide_tooth_run` 2688-2721 and `rank_jump_run` 2829-2852 and `grep -n -i internal tests/meter.rs` (hits only at 4604 and 7704, unrelated); read the accum tests 6560-6700 and `cancelling_run`; grep of `CliffFan|cliff_fan` over the skyline and tier2 suites shows corpus membership only, no counter read); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed on all three legs; history: all strings unedited since cc84df7e; the wide-tooth internal entry had a technical reason at the band's landing (f39751d67, one day before the flag day, when `Version::rank` was still the tree fold) that faf3cd0a removed
- Owner-gated: no (correcting the strings toward the code is sanctioned; whether wide-tooth and jump-comb become board columns is the owner's)

The registry is "the single source of truth ... from which every instrument derives" (lines 1-2), so a reason string that misdirects an audit is a claim contradicted (Principle 8). Three kinds:

(1) `TICK_CROSS_UNBANDED` is shared by NestedFull (Dense × NestedFullId), NestedWide, MirrorWide, MirrorNarrow (WideTail(1, d) × NestedLeftFullId), and Staircase (Staircase × IdSpine(d, false), per board/family.rs:564-603). tests/meter.rs holds tick pins for Bigroot × NestedFullId (`tick_nested_wide_envelope`, 483-485) and WideTail(s, s) × NestedLeftFullId (`tick_mirror_wide_envelope`, 503-505); `tick_dense_envelope` ticks the dense spine with `Party::seed()` (468-476), and the only staircase tick pin (`tick_ownership_hole_envelope`, 529-531) uses `IdSpine.packed_flagged(HOLE_ID_DEPTH, true)`, the diverted spine, a different cross. NestedFull, MirrorNarrow, and Staircase are priced by their board columns, not by a tick gate pin on their cross.

(2) CliffFan's and CancellingChain's reasons say their pins are "absolute envelopes in the in-crate skyline and tier2 suites, not two-point bands in tests/meter.rs". Those suites hold both shapes only as corpus members (`assert_agreement` at skyline/tests.rs:514-533 checks length, validity, and round trip with no counter; the subadditivity grid at tier2/tests.rs:531). The actual two-point flatness pins are `accum_fan_touches_flat` and `accum_cancelling_touches_flat` in tests/meter.rs (6637, 6670), the file the reason says they are not in, and they drive a raw `suanpan::Accumulator` (`cancelling_run`, 6569-6585) rather than a `before` operation; the fan test calls `comb_run` and never builds `Shape::CliffFan` (finding 12).

(3) WideToothComb and JumpComb claim "a deliberate, documented internal-entry decision at the band's citation site". `rank_wide_tooth_run` and `rank_jump_run` call `meter::skyline::query::rank(meter::skyline::view(&enc))` and document the liveness floor and the answer pin, never the entry choice; `rank_weight_comb_run` (4522) measures the same kernel through public `v.rank()`. Doctrine: suites exercise the public API "except as deliberate, documented decisions at the check site"; the registry asserts documentation a reader cannot find.

Evidence:

      1108	/// The tick crosses' shared no-band reason.
      1109	const TICK_CROSS_UNBANDED: &str = "tick cross: the tick gate pins in tests/meter.rs price its walk";
      1629	                    reason: "kernel-seam probe measured through the internal skyline \
      1630	                             entries, which the board's public-operation rows cannot host \
      1631	                             — a deliberate, documented internal-entry decision at the \
      1632	                             band's citation site",
      1663	                    reason: "its pins are absolute envelopes in the in-crate skyline and \
      1664	                             tier2 suites, not two-point bands in tests/meter.rs",

    tests/meter.rs:
      2704	        let r = meter::skyline::query::rank(meter::skyline::view(&enc));
      6637	    fn accum_fan_touches_flat() {
      6638	        let small = comb_run(4_096, 50_000);

Resolution: give each family its own accurate reason. Nested-full, mirror-narrow, staircase: "priced by its board column; no tick gate pin exists on this cross" (or add the pins). Cliff-fan, cancelling-chain: name `accum_fan_touches_flat`/`accum_cancelling_touches_flat` and say they price the accumulator's stream, not a `before` operation (or resolve per finding 12). Wide-tooth, jump-comb: either route both runs through `Version::rank` like the weight-comb run and re-rule the coverage answer, or write the internal-entry decision and its reason at the two run fns and quote it. Add a registry-side check that every `reason` naming a test fn resolves (the parity scanner already reads tests/meter.rs). Acceptance: for every `reason` string naming a test, file, or module, a grep of the named site finds the named artifact.
Construction: `grep -nE 'Shape::Dense\.packed1.*NestedFullId|WideTail\.packed2\(1,|IdSpine\.packed_flagged\([A-Z_]+, false\)' crates/before/tests/meter.rs` inside `fn tick_*` bodies finds none; `grep -n 'CliffFan\|cliff_fan' crates/before/src/version/skyline crates/before/src/meter/tier2` finds corpora only; `awk 'NR>=2688&&NR<=2697' crates/before/tests/meter.rs` finds no internal-entry rationale.

Constructed test: demonstrated by reading (results.md lines 4055-4089). All three legs confirmed at the cited lines; one caveat: `meter/tier2/tests.rs:531` does use `cliff_fan(48, 32).version()` and its context was not read, so the "tier2 suites" half of the cliff-fan reason may hold there.

### The board

### board-frame-8: `MAX_SCALING_EXPONENT`'s doc claims 1.15 excludes a log factor at these sizes; an n·log n kernel fits 1.07-1.10 on every committed ladder
- Where: crates/before/src/meter/board/ceilings.rs:64-69 (related: crates/before/src/meter/board/ceilings.rs:342-345, crates/before/src/meter/board/judge.rs:37-54, crates/before/src/meter/board/tests.rs:756-855)
- Class / severity / confidence: claim / low / high
- Provenance: verified (transcribed `judge::trend` (judge.rs:37-54) to Python and ran it); executed: yes (`trend.py` in my scratch directory: four-point ladders `[b, 2b, 4b, 8b]` with work `8n·log2(8n)` read 1.100 (b = 1 KiB), 1.096 (1.5 KiB), 1.088 (4 KiB), 1.085 (6 KiB), 1.074 (32 KiB), 1.067 (128 KiB), all under 1.15; `n·log² n` reads 1.13-1.20; a quadratic reads 2.000; two-point windows read 1.07-1.10. The board itself was not run.)
- Seen by: scaffolding, adequacy; refutation: confirmed, severity medium to low (at these sizes a whole-work log factor multiplies per-byte constants by roughly 13-17, which the touch, scan, and limb constant legs catch; the misattribution is what is false); history: no-rationale-found (verbatim from 7d81a248a; never re-derived; the same file later quantified a log marginal at ~1.14-1.17 without revisiting this sentence)
- Owner-gated: no

The doc states an exclusion the leg does not deliver: what 1.15 excludes is polynomial super-linearity (a quadratic reads ~2 on every committed ladder), while a log factor in the operand size sits inside the slack at the ladder's sizes. The same file's fold model (ceilings.rs:343-344) quotes a log marginal at ~1.14-1.17, straddling the ceiling. A maintainer trusting the sentence would believe an undocumented O(n log n) regression in an unmetered currency reads red on the board; it does not (Principle 8: a ceiling's doc states what it excludes accurately; the crate promises asymptotic claims as hard guarantees).

Evidence:

        64	/// Green requires every meter's scaling exponent at or below this.
        65	///
        66	/// The contract is amortized-linear; 1.15 leaves room for measurement noise
        67	/// (allocator rounding, `Vec` doubling) without admitting a real log factor at
        68	/// these input sizes.
        69	pub const MAX_SCALING_EXPONENT: f64 = 1.15;

    [ceilings.rs:342-344]
       342	/// Work `c·D·log2(2k)` fitted across the cell's two probes (`D₁, k₁) → (D₂,
       343	/// k₂`) reads exponent `1 + log2(log2(2k₂)/log2(2k₁)) / log2(D₂/D₁)` — the log
       344	/// factor's marginal, ~1.14–1.17 at the committed populations — so the ceiling

Resolution: Re-word: "1.15 excludes polynomial super-linearity (a quadratic reads ~2 on every committed ladder) while leaving 0.15 for allocator rounding and `Vec` doubling; a log factor in the operand size fits inside that slack at the ladder's sizes (an n·log n kernel reads about 1.07-1.10), so documented log factors are held by the asymptotics suite's liveness pins and the fold rows' declared model, and an undocumented one by the constant legs, not by this bound." Add a probe beside the quadratic tripwire in board/tests.rs asserting `trend` over `(n, n·log2(8n))` at the smallest committed base reads under the ceiling, so the stated limit is pinned in both directions. Acceptance: no sentence in ceilings.rs asserts the global exponent ceiling excludes a log factor; a committed test asserts the n·log n ladder reads under `MAX_SCALING_EXPONENT` beside the existing assertion that a quadratic reads over it.

### board-frame-26: The validation index calls the board's ceilings "class-scale", but the touch, κ, fold-scan, and family-stated constants are pinned at worst-reader ×1.25
- Where: crates/before/src/testing/validation_index.rs:103-105 (related: crates/before/src/meter/board/ceilings.rs:119-127, 174-177, 258-261, 382-384, 402-404, 439-442)
- Class / severity / confidence: claim / low / medium
- Provenance: verified (`git log -S'would forgive' -- validation_index.rs` gives 669cf3103 (2026-07-28); c0b5d701 (2026-08-10) is "re-pin the touch ceiling to the worst-reading x1.25 convention" with "ceil(17.18 x 1.25) = 22" in its message; ceilings.rs:119-120, :176, :258-260, :382-383, :402-403, :439-440 each state the ×1.25 convention; heap 16, limb 128, scan 96, and the 1.15 exponent remain class-scale); executed: no
- Seen by: scaffolding; refutation: reframed (the sub-claim that tests/meter.rs already pins the worst touch readers per scenario is wrong: its `TouchEnvelope` rows are all `RANK_*` scenarios; the cascade objection is an owner-gated design judgment, not a defect); history: deliberate-but-expired for the index sentence (κ was already ×1.25 when it was written; the touch re-pin expired it for that leg); the ×1.25 convention itself is owner-ratified in c0b5d701 and holds
- Owner-gated: yes (which of two consistent states the owner wants)

The index is the crate's own map of what each instrument alone catches; it says the envelope suite alone catches constant-factor regressions because "the board's ceilings are class-scale and would forgive a doubled constant". Since c0b5d701 the touch ceiling, and by the same convention κ, the fold-scan constant, both `ASCEND_CLIFF_*` ceilings, and the mirror-wide render constant, are the worst honest reading ×1.25, so those board legs are envelope-class and two instruments now claim the same failure class; only the heap, limb, and scan globals and the exponent legs remain class-scale (Principle 8: a stale map misdirects triage). Separately, a global ceiling at worst ×1.25 is tight only at the argmax cell (a cell reading 2 touches per byte can regress 10× under 22 and stay green) while every honest family that later reads past the worst witness forces a re-pin; that trade is stated as intended at ceilings.rs:123-126 and is an owner judgment.

Evidence:

       103	//! it alone catches: **constant-factor regressions and cure
       104	//! backslides** — the board's ceilings are class-scale and would forgive
       105	//! a doubled constant; the envelope pins move only through a reviewed

    [ceilings.rs:119-121]
       119	/// model. The ceiling is the worst honest reading ×1.25, rounded up
       120	/// (owner-ratified: the family-stated ceilings' margin convention; the
       121	/// reading lives in the pin commit), so a kernel that re-reads digit state

Resolution: Owner's call. (a) Keep the ×1.25 convention and re-word the index: the touch, κ, fold-scan, and family-stated constant legs are envelope-class (×1.25 over the worst honest reader, re-pinned when the worst reader moves); the heap, limb, and scan globals and the exponent legs are class-scale. (b) Restore a class-scale touch ceiling and add `TouchEnvelope` rows to tests/meter.rs for the worst readers the pin commit names, so the board judges class and the envelopes judge constants as the index says. History favors (a): the convention is owner-ratified. Acceptance: the index sentence and ceilings.rs agree on which legs are class-scale; if (b), the re-pin and the new envelope rows land in one change and the board of record reads green.

### board-families-floors-judge-14: `touch_pair_fold`'s doc overclaims that a dead touch meter trips on every committed pair family
- Where: crates/before/src/meter/board/floors.rs:474-478 (related: floors.rs:479-492, 668-679; family.rs:510-516, 797-803; meter.rs:204-210; operand.rs:39-42)
- Class / severity / confidence: claim / low / high
- Provenance: assessed (read: hugeleaf is one leaf flag plus one gamma code; the post-pass ticks it at the seed, leaving one leaf; `stored_nonzero_deltas` counts only payloads after the first, so both operands read 0 and the constructor returns `na(NA_TOUCH_NO_DELTAS)`, which `comparison_floors` carries on a comparable pair); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no-rationale-found (false or undefined from be6dd6a0 on; "pair family" has a narrower module-local sense at family.rs:413-415 under which it might hold, but nothing pins either reading)
- Owner-gated: no

The sentence is the liveness argument for the touch column: because the floor is positive wherever either operand stores a nonzero delta, "a dead touch meter still trips it on every committed pair family". The hugeleaf column is a committed board family whose two operands store no deltas at all, so the constructor returns NA there and a dead touch meter is not tripped on its comparison, join, or meet cells. The conditional half is correct; the universal is a hand-maintained roster claim no test pins.

Evidence:

       474	/// sign-read traffic as mandatory and ban the efficiency. The floor stays
       475	/// strictly positive wherever either operand stores a nonzero delta, so a dead
       476	/// touch meter still trips it on every committed pair family. Equal operands
       477	/// are answered by canonical byte identity before any sweep runs (`a ∨ a = a`,

Resolution: Drop the universal clause and state the conditional ("positive wherever either operand stores a nonzero delta; a delta-free pair such as the single-leaf hugeleaf column declares NA"), or, if every pair family is meant to carry a live touch floor, pin it with a test over `FamilyId::board()`'s version pairs and give hugeleaf a counterpart that stores a delta. Acceptance: the doc asserts no property of every committed family, or a committed test iterating the board's version pairs asserts `touch_pair_fold` is `Liveness::Floor` on each and passes.
Construction: Build the hugeleaf bundle (`FamilyData::build(FamilyId::Hugeleaf, 1.0, 0)`), decode `version` and `version2`, and assert `matches!(touch_pair_fold(&v, &w), Liveness::NotApplicable { .. })`: it holds today, contradicting the sentence.

### The surface roster

### surface-roster-7: Three prose sites say the surface-totality gate fails until a `FAMILY_SURFACE` row is added; surfacecheck never reads `FAMILY_SURFACE`, and impls already sit behind no row
- Where: crates/before/src/surface.rs:1010-1015 (related: crates/before/src/testing/surface_coverage.rs:25-30, crates/before/surfacecheck/src/main.rs:17-20, crates/before/surfacecheck/src/main.rs:100-103, crates/before/surfacecheck/src/check.rs:159-165, crates/before/surfacecheck/src/census.rs:4-10, crates/before/src/testing/surface_coverage/tests.rs:334-341, crates/before/surfacecheck/src/census.rs:251, crates/before/surfacecheck/src/census.rs:256, crates/before/surfacecheck/src/census.rs:262, crates/before/surfacecheck/src/census.rs:401-429)
- Class / severity / confidence: claim / high / high
- Provenance: verified (read every surfacecheck source: the only roster read is main.rs:100 `before::surface::METHOD_SURFACE`; `FAMILY_SURFACE` appears in surfacecheck only in prose at main.rs:20 and census.rs:9 and in the guidance string at check.rs:162; a case-sensitive grep of surface.rs for `Default`, `Cow`, `Overlap`, `TooWide` returns nothing while census.rs pins `Default for Version` (262), the two `Cow<Version>` conversions (251, 256), and full `error::Overlap`/`error::TooWide` rows (401-429); the "error verdict types" row at 1241 names only Decode / Parse / Crossed); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (d60b7570 replaced the accurate "Totality here is by review of this file" with the overclaim; its message describes only the pin mechanism; 9aa8c9ac states "Trait-impl methods stay FAMILY_SURFACE's review-governed domain"; tests.rs:334-336 still says totality "is by review of the file")
- Owner-gated: no for the prose correction; binding the two layers is a design change for the owner

The roster doc, the suite's module doc, and surfacecheck's module doc claim a new operator impl "fails that gate until its pin — and, for a new family, a family row here — is added". The gate reconciles trait impls against `census::TRAIT_IMPLS` only; nothing maps a census pin to a family row in either direction. A maintainer who adds an impl, sees the census go red, and pins the row is green everywhere with no family disposition recorded, and the census already holds `Default for Version` (a second spelling of `Version::new`), the `From<&Version>`/`From<Version> for Cow<Version>` conversions, and `error::Overlap`/`error::TooWide` behind no row. Principle 8 (verified vs told): a reader trusting the roster believes every operator family has a recorded leg disposition when only the impl's existence is pinned; Principle 2: a board nothing enforces is decoration. The severity follows the brief's rubric for a claim contradicted by the code; the minimum fix is three sentences.

Evidence:

      1010	/// Rows here carry the leg dispositions by family; the concrete
      1011	/// impl inventory behind them is held mechanically total by the
      1012	/// surface-totality gate (`crates/before/surfacecheck`), which pins every
      1013	/// reachable trait impl by name against nightly rustdoc JSON. A new
      1014	/// operator impl is a deliberate API event: it fails that gate until its
      1015	/// pin — and, for a new family, a family row here — is added.

    surface_coverage/tests.rs:
       334	/// The family roster's rows are unique by op description (totality over
       335	/// the operator/trait surface is by review of the file; this pins the
       336	/// table's internal hygiene).

    surfacecheck/src/main.rs:
       100	    let rostered: BTreeSet<&str> = before::surface::METHOD_SURFACE
       101	        .iter()
       102	        .map(|row| row.op)
       103	        .collect();

Resolution: minimum, now: restate surface.rs:1013-1015, surface_coverage.rs:28-30, and main.rs:17-20 so the pin is mechanical and the family row is review-maintained, matching tests.rs:334-336; add family rows (or extend existing ones) for `Default` on `Version`/`Rank`/`Ticks`, the `Cow<Version>` conversions, and `error::Overlap`/`TooWide`. Better, as a design round: give each `FAMILY_SURFACE` row a machine-checkable membership (an `impls: &'static [&'static str]` of census-row prefixes or exact rows) and reconcile in surfacecheck both ways, with the three non-impl rows ("unbounded depth", "meter instrumentation plumbing", "error verdict types") excepted by name; `TRAIT_IMPLS` then becomes derived data. Acceptance: no prose attributes the family-row obligation to the gate, and the orphans above carry a disposition; or, under the binding, a synthetic census pin with no covering family row reads red in check/tests.rs and the construction below reads red in `just surface-totality`.
Construction: add `impl core::ops::Neg for Version { type Output = Version; fn neg(self) -> Version { self } }` in src/version.rs and the line `"Version: impl core::ops::arith::Neg for Version",` to `TRAIT_IMPLS`, with no `FAMILY_SURFACE` row. `just surface-totality` and `just test-all` are both green.

Constructed test: demonstrated by reading (results.md lines 4238-4273). `surfacecheck` reconciles function-like items against `METHOD_SURFACE` (main.rs:100-107) and trait impls against `census::TRAIT_IMPLS`; `FAMILY_SURFACE` is named only in its module doc and never read; the census pins `Default for Version`, both `Cow<Version>` conversions, and the `error::Overlap` impls behind no family row. The `Neg` construction was not run.

### surface-roster-23: Auto-trait impls are excluded from the census on the premise that `auto_traits.rs` covers every public type; it misses thirteen
- Where: crates/before/surfacecheck/src/extract.rs:267-270 (related: crates/before/surfacecheck/src/extract.rs:21-24, crates/before/surfacecheck/src/extract.rs:279-281, crates/before/src/auto_traits.rs:5-32, crates/before/surfacecheck/src/census.rs:60, crates/before/surfacecheck/src/census.rs:332-335, crates/before/surfacecheck/src/census.rs:349-352, crates/before/surfacecheck/src/census.rs:384-387, crates/before/surfacecheck/src/census.rs:420-429, crates/before/surfacecheck/src/census.rs:435-463)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read auto_traits.rs in full, 26 instantiations; the census owners it does not cover are `Limbs`, `error::TooWide`, `shape::{Cell, Cells, Overlay, Plateau, Plateaus, Region, Regions, Rise}`, and `causally::{Down, Up, Neutral}`, thirteen types); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: deliberate-but-expired (the premise was true at d60b7570, which listed every then-public type; db9dfa3e added the polarity markers and pinned `Query<Down>`/`Query<Up>` rather than the markers; 46eb64f9 introduced the shape types, `Limbs`, and `TooWide` without touching auto_traits.rs)
- Owner-gated: no

`record_impl` drops `is_synthetic` impls because `auto_traits.rs` "asserts `Send + Sync + Unpin` on every public API type at compile time". The list is a hand-maintained enumeration of public types (the exact rot this partition's rustdoc walk exists to prevent), nothing holds it total, and it has already drifted by thirteen types. For those, no check pins `Send`/`Sync`/`Unpin` anywhere: a shape iterator that gains an `Rc`- or `Cell`-bearing field loses `Send` with no red. Principle 3: an exclusion earns its place by naming what covers the excluded space, and the named cover is partial; the polarity markers are uninhabited enums and trivially auto-trait, but the shape types, `Limbs`, and `TooWide` are not.

Evidence:

       267	/// Compiler-synthesized auto-trait impls are excluded — `before` pins
       268	/// `Send`/`Sync`/`Unpin` on every public API type at compile time in
       269	/// `src/auto_traits.rs`, which is where that guarantee is reviewed —

Resolution: either (a) stop excluding synthetic impls and pin the `Send`/`Sync`/`Unpin` rows in the census for every reachable type, dissolving the hand list (the JSON already carries them, so the pin becomes mechanical and total); or (b) keep auto_traits.rs, extend it with the thirteen missing types, and have surfacecheck hold it total by asserting every reachable struct and enum has a synthetic impl for each of the three traits. Acceptance: a public type losing `Send` makes `just surface-totality` (a) or `cargo check` (b) red.
Construction: add `_marker: core::marker::PhantomData<*const ()>` to `shape::Plateaus` and initialize it. The crate compiles, `Plateaus` is no longer `Send` or `Sync`, and `just gate` is green.

Constructed test: demonstrated (results.md lines 5928-5987). With a `PhantomData<*const ()>` field added to `shape::Plateaus`, the crate and every test target compile under `--all-features` with no diagnostic and the shape tests pass; a grep confirms none of `Limbs`, `TooWide`, the eight `shape::` types, or `causally::Neutral` appears in `auto_traits.rs` (`Down`/`Up` appear only as `Query` type parameters).

### The test harness

### testing-diff-gen-28: The `Party::join_all` pin's discriminating margin is about 2%, and the module doc's "a crossing is a class change, never noise" overstates
- Where: crates/before/src/testing/asymptotics.rs:339-349 (related: crates/before/src/testing/asymptotics.rs:12-14, crates/before/src/testing/asymptotics.rs:330-338, crates/before/src/party/ops/index.rs)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (readings of record from `git show 4797009a`: bytes ×4.80 (40,960 → 196,608 B), door ×4.99 (7,260,488 → 36,191,048 bits, i.e. 4.985); the floor 4.89 sits 1.9% from each endpoint and the log factor's marginal on this population is 3.8%; f0cd4ab2f: "party_join_all is the narrowest gap of the five (3.9%, exact counters both sides)"); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: deliberate-and-holds for the gap (measured, recorded, and accepted at the pin commit, with the inline stability argument at 330-338), but that rationale answers noise, not the two-term ambiguity, so the module-doc claim stands unaddressed
- Owner-gated: yes (the floor and its population were ruled at f0cd4ab2f)

A ratio of `a·k·log k + b·k` across a quadrupling moves with `b/a`; exact counters remove noise, not that ambiguity. A constant-factor change in the linear index term (`metered_partition_point` in `party/ops/index.rs` recording one more scan word per probe) crosses 4.89 with the log factor intact and the message blames the documented `O(D log k)`; conversely a linear fold whose per-input scan grows 2% faster than bytes reads 4.90 and passes with the log factor gone. The doc at 330-338 concedes the narrow gap and argues determinism makes it stable, which is true and not the point.

Evidence:

        12	//! moves in the same commit. Floors sit midway between the linear
        13	//! reference and the measured reading; both endpoints are exact
        14	//! counters, so a crossing is a class change, never noise.
       ...
       341	fn party_join_all_log_factor_is_alive() {
       342	    const MIN_GROWTH: f64 = 4.89;

Resolution: Witness the id fold's log factor on a population whose unions do not densify (isolated owned leaves at maximal depth, so each balanced union's encoding is near the sum of its parts), where the door should read near ×6.0 against ×4.8 as the version doors do; independently, restate lines 12-14 to what the pins can attribute: a deterministic tightness pin at two scales whose crossing means re-derive, not class change. Acceptance: for every fold-door pin the measured reading exceeds the in-run byte ratio by a stated margin the population is shown to express, and a committed known-bad (a left fold behind the same door on the same population) reads below the floor.

Construction: With the door at 4.985 and bytes at 4.80, `b/a ≈ 43` (from `(a·10240 + b·1024)/(a·2048 + b·256) · (4.80/4) = 4.985`); raising the per-input index cost by about 5% moves the ratio a few hundredths and crosses 4.89 while the balanced fold is untouched.

Constructed test: demonstrated (results.md lines 4310-4350). The committed pin ran: door growth 36,191,048 / 7,260,488 = ×4.985, byte growth ×4.80, floor 4.89 sitting 1.9% from each endpoint. The two-term decomposition and the ~5% crossing claim are the finder's arithmetic model, not mutation-verified.

### testing-oracles-28: The validation index claims to map every instrument but has no row for at least eight committed instruments, misdescribes the exhaustive corpus as "reachable states", and gives the replay keystone no row
- Where: crates/before/src/testing/validation_index.rs:1-3 (related: crates/before/src/testing/validation_index.rs:11-12, crates/before/src/testing/validation_index.rs:64-66, crates/before/src/testing/validation_index.rs:164-170, crates/before/AGENTS.md, justfile:321, justfile:366, justfile:641, justfile:853, justfile:941, crates/before/tests, crates/before/fuzz, crates/before/wasm32-pins, crates/before/surfacecheck, tools/covcheck, tools/mutantcheck, .cargo/mutants.toml)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (`grep -n -i 'wasm\|mutant\|covcheck\|surfacecheck\|verdict\|superlinear\|tamper\|fuzz\|tripwire'` over the index matches only the fuzz-fit bands, "shared with the fuzz targets", "fuzz seeds", and generic uses of "coverage"/"tripwires"; every omitted instrument exists on disk and has a justfile recipe (`mutants-list` 321, `fuzz-build` 366, `wasm32-pins` 641, `bench-judge-tripwire` 853, `surface-totality` 941) or a `tests/*.rs` binary; `tools/citecheck` scopes itself to surface.rs, diff_ops.rs, and surface_coverage.rs; the module is never rendered (testing-oracles-2), so no link check runs; the exhaustive enumerator produces canonical trees whether or not any op sequence reaches them, per exhaustive.rs:106-116 and tests.rs:252-253); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed; history: no-rationale-found ("every instrument" is the recorded design intent and stood at birth in 669cf310; the omitted instruments were never rowed; no ruling narrows the page)
- Owner-gated: no

The page is the crate's designated orientation map (before's AGENTS.md routes a maintainer to the testing module docs, and the review brief names it the validation map). It opens with a totality claim and has no row for the fuzz targets, the wasm32 pins, the rustdoc-JSON surface totality check, the coverage pins, the mutants roster, the verdict matrix, the superlinear tripwires, the bench-judge tripwire, or the `tests/` tamper pins, so a maintainer asking "is there a fuzz target for non-canonical decode?" concludes there is none. Its own bar for a new instrument ("a failure class no row below already catches") cannot be applied against rows that are missing. Unlike the crate's other rosters (surface held to the `pub fn` scan, `for_each_law_group!` pinned, `TRIPWIRES` names checked live), nothing holds this page to the instruments that exist. Two rows misdescribe: "every reachable state" is a reachability claim the exhaustive enumerator does not make (it enumerates canonical normal-form trees, including shapes the tests doc says "the op pipeline never builds"), and the function-space replay, the one thing the semantic oracle alone catches (a bug both tree recursions share; dependence on the impl's particular fork/inflation policy), has no row of its own.

Evidence:

         1	//! The validation index: every instrument that guards this crate, what
         2	//! failure class each one catches that the others cannot, and where it
         3	//! lives.
        11	//! trips one instrument, this page says which neighbors to check; when a
        12	//! new instrument is proposed, the bar is a failure class no row below
        64	//! **Exhaustive small-scope enumeration** ([`super::exhaustive`]). Total
        65	//! enumeration of every reachable state and operation pairing inside
        66	//! small bounds, checked against the oracle. What it alone catches:

Resolution: Either add one row per omitted instrument at the same altitude (what it alone catches, where it lives, which recipe runs it), plus a row for the fs replay, or narrow the opening sentence to the classes the page covers and point at `just --list` and the justfile's recipe comments as the recipe-level inventory. Rewrite the exhaustive row as "every canonical normal-form id tree to `ID_SMALL_DEPTH` and event tree to `EV_SMALL_DEPTH`, every ordered pair, for the operations its check set names; kernel suites sweep further operations over the same corpus". Owner's call whether to pin the page mechanically (a test that every recipe under the gate/ci composition and every `tests/*.rs` binary is named in the index). Acceptance: every verification recipe the justfile's gate and ci compositions run for before, and every `crates/before/tests/*.rs` binary, is named in the index or the header states the page's scope and where the rest is indexed; the exhaustive row says "canonical normal-form trees"; `grep -n -i 'wasm32\|covcheck\|mutants\|surfacecheck\|verdict_matrix\|superlinear' validation_index.rs` finds each.
Construction: The grep in the acceptance clause returns nothing at this commit; `ls crates/before/{fuzz,wasm32-pins,surfacecheck} tools/{covcheck,mutantcheck} .cargo/mutants.toml crates/before/tests/{verdict_matrix,superlinear_tripwires}.rs` all exist. For the exhaustive row: `all_normal_events(2)` contains trees with base patterns no tick sequence over a seed-derived party produces at that depth, and they are checked, so reachability is not the enumerated property.

Constructed test: demonstrated by reading, with two corrections (results.md lines 4274-4309). The page does carry a bench-judge row (lines 109-119, naming `tools/benchjudge`, the expected roster, and `tests/bench_judge_roster.rs`) and a fuzz-fit bands row (121-125), and mentions fuzz seeds (169) and "the tripwires" generically (106-107); no row names the wasm32 pins, surfacecheck, covcheck, the mutants roster, the verdict matrix, the `tests/` tamper pins, or `superlinear_tripwires.rs`.

Synthesis note: "At least eight committed instruments" overstates by two: the bench judge and the fuzz-fit bands have rows (the constructed read confirms the remaining seven omissions). The verdict, the exhaustive-row misdescription, and the missing replay row stand.

### The envelopes

### envelopes-a-16: The ×1.25 one-doubling flatness band admits growth exponents up to about 1.32 while the band docs say "linear" and the board judges at 1.15
- Where: crates/before/tests/meter.rs:2392-2408 (related: 2342-2347; band docs at 3005, 3044, 3550, 3669, 3766, 3862, 3910, 3958, 4015, 4125, 4243, 4560, 4670, 4752; 4843-4859; crates/before/src/meter/board/ceilings.rs:64-69)
- Class / severity / confidence: claim / low / medium
- Provenance: assessed (arithmetic on the assertion: for cost `c·n^a` the per-unit ratio across one doubling is `2^(a−1)`, so `5/4` admits `a <= 1 + log2(1.25) ≈ 1.32`; read `MAX_SCALING_EXPONENT = 1.15` and its inline rationale); executed: no
- Seen by: adequacy; refutation: confirmed (noting every band also carries absolute two-scale ceilings at measured ×1.25, and that the dense-suffix band at 4855-4859 deliberately relies on the slack for its declared log model)
- Owner-gated: yes: the slack is a gate-policy constant

Statement faithfulness: a test doc must be neither weaker nor stronger than what the assertion proves; "is linear" overstates a bound of exponent at most about 1.32, and two instruments hold the same claim to different bars without saying why. The counters here are exact, so the slack is not absorbing noise; the committed adequacy kernels read ×1.5 to ×2 and are caught, an O(n^1.2) regression is not, and the absolute ceilings catch it only at the measured scales.

Evidence:

      2402	        assert!(
      2403	            u128::from(m2) * u128::from(n1) * u128::from(SLACK_DEN)
      2404	                <= u128::from(m1) * u128::from(n2) * u128::from(SLACK_NUM),

    ceilings.rs:
        66	/// The contract is amortized-linear; 1.15 leaves room for measurement noise
        67	/// (allocator rounding, `Vec` doubling) without admitting a real log factor at
        68	/// these input sizes.
        69	pub const MAX_SCALING_EXPONENT: f64 = 1.15;

Resolution: Either tighten the flatness slack toward the board's bar (`10/9` matches exponent 1.15 across one doubling; the dense-suffix band would need its own declared slack for its log model) or restate the band docs as "per-unit growth at most ×1.25 across the doubling (exponent at most 1.32), which the committed kernel's ×1.5+ reading exceeds" and record at `SLACK_NUM` why the envelopes' bar differs from the board's. Acceptance: each band doc states the exponent bound its assertion enforces, and that bound is either the board's or justified at the constant.

Construction: add a synthetic term growing as `bytes^0.3` touches to any metered fold; from 512 to 1024 the per-unit reading grows about 23% and passes every `assert_flat` while the doc claims linearity.

### envelopes-b-4: The settle-flatness module comment's level ratio (×1.17) is wrong at the committed probe counts, and `assert_flat_step`'s doc names one band while its limb row uses another
- Where: crates/before/tests/meter.rs:5830-5836 (related: 5866-5868, 5883-5905, 6023-6031, 6083-6087, 7759, 8184, 7833-7852)
- Class / severity / confidence: claim / low / high
- Provenance: verified (arithmetic by hand; `git blame` puts 5830-5835 in 016b91c4a, whose tree already ran `train_run(4/8/16, ...)` at its lines 4497-4501; src/meter.rs:2465-2466 fixes one arming per block, so the arming count is `n`; the run1.log limb readings 6806/7652 -> 16873/14500 give a per-byte ratio of ×1.31 at 4->8, which is why the limb row needs ×1.5); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed; history: no rationale found for the number (wrong at birth with the same probe sizes); deliberate-but-expired for the `assert_flat_step` doc (e4c9b083e widened the limb band to ×1.5 and updated the in-body comment at 5883-5888 but not the fn doc at 5866-5868)
- Owner-gated: no

The comment's own formula `log2(2n)/log2(n)` gives 1.5 at n = 4 and 1.33 at n = 8; under the `(2n).log2()` level model the fold bands use (7759, 8184) the per-doubling ratios are 1.33 (4->8) and 1.25 (8->16). The value 1.17 (7/6) corresponds to n = 64, which no probe runs. The band holds because, as the sentence's second clause says, the settle does not dominate, not because ×1.25 covers the model's admissible growth; a re-pinner reasoning from the comment would call a ×1.3 touch reading at 4->8 a regression the model admits. Separately, `assert_flat_step`'s doc says "per currency, flatness (×1.25 per byte)" while its limb row is banded at 2/3 (×1.5). A band's slack must be justified by a correct argument.

Evidence:

      5830	// The tree rewrites a window's digits once per level and the mass
      5831	// balance keeps levels logarithmic in the arming count, so the
      5832	// settle's metered traffic per byte can grow only by the level ratio
      5833	// across an arming-count doubling — ×log₂(2n)/log₂(n), at most ×1.17
      5834	// from the probes' smallest count — and only if the settle dominated
      5835	// the fold's linear work, which it does not: the ×1.25 flatness
      5836	// convention covers the model's whole admissible growth here. The
    ...
      5866	    /// Assert one probe's reading against its absolute pinned ceilings
      5867	    /// and, per currency, flatness (×1.25 per byte) across the
      5868	    /// doubling, and report the readings.
    ...
      5903	                2u128,
      5904	                3u128,

Resolution: either normalize the settle probes by the level count as `fold_stagger::assert_model_flat` does, so ×1.25 judges the model's constant, or correct the comment to the ratios at the committed counts (×1.5 at 4->8 and ×1.33 at 8->16 under the comment's formula; ×1.33 and ×1.25 under the level model) and rest the touch band explicitly on the measured non-dominance of the settle. Make `assert_flat_step`'s doc name both bands (touches ×1.25, limb ops ×1.5) and why they differ. Acceptance: the comment's number equals its formula at the smallest committed n; the fn doc matches its two numerator/denominator pairs.
Construction: compute log2(8)/log2(4) = 1.5 and log2(16)/log2(8) = 1.333 against the `train_run(4, ...)`, `(8, ...)`, `(16, ...)` calls at 6083-6087; neither is 1.17 and neither is at or under 1.25.

### envelopes-b-20: Touch liveness floors whose stated premises do not reach the asserted constant
- Where: crates/before/tests/meter.rs:8384-8410 (related: 8690-8717, 5380-5385, 5720-5725, 5857-5862, 5989-5994, 8192-8198, 8661-8668)
- Class / severity / confidence: claim / low / medium
- Provenance: verified for the `input / 8` composition (arithmetic: one touch per 64-bit limb is one touch per 8 payload bytes, and a payload of at least an eighth of the input yields `touches >= input / 64`, not `input / 8`; fe39fca35 states the same two premises for `input/8` and records margins only against readings, "plateau 1,438 vs 206 (x7.0)"); assessed for the per-byte rank floors (no derivation appears in their docs at 5349-5352, 5685-5689, 5847-5848; the adequacy lens's estimate that PP(500, 500)'s irreducible touches sit around a third of its byte count is a re-derivation from src/meter.rs:2380-2415 and is unverified); executed: no
- Seen by: adequacy; refutation: confirmed; history: the `input/8` floor is a deliberate premise-first re-derivation (fe39fca35) whose purpose holds, but the arithmetic critique survives; the `touches >= bytes` floors have no derivation anywhere in history
- Owner-gated: no

The memo and width-circulation `tick_run` docs derive a one-touch-per-eight-bytes floor from two premises whose composition gives one per sixty-four; the floor holds today because readings sit far above it (run1.log: memo_oscillating 996,964 touches against 260,503/8 = 32,562). The module's own header at 8661-8668 distinguishes derived floors from measured tripwires, and a floor whose derivation does not reach its number is an observed-value floor in the liveness genre's clothing. The rank and pair probes' `touches >= bytes` floors carry no derivation at all; if the adequacy lens's estimate is right, an improvement that touches each digit of the plateau once would trip a floor labelled "liveness". A floor asserts the minimum possible work from one universal premise; a legitimate input below it is a finding about the premise.

Evidence:

      8384	    /// Enforces a one-touch-per-eight-input-bytes liveness floor before
      8385	    /// returning, derived from the walk's irreducible work: every
      8386	    /// consumed code's magnitude folds into the height accumulator at
      8387	    /// least once — one digit touch per 64-bit limb of the operand,
      8388	    /// zero limbs included — and in every family here the folded
      8389	    /// payload (the circulated memo minima and the per-leaf delta
      8390	    /// codes) is at least an eighth of the packed input. A reading
    ...
      8404	        assert!(
      8405	            run.touches >= run.input / 8,

Resolution: for the `/8` floors, restate the premise so it yields the constant (for example: every consumed code costs at least one touch, a code spans at most eight input bytes per touch it costs, and the payload is the whole input) or lower the constant to what the stated premises support (`input / 64`); for the per-byte rank floors, derive them per family from the code structure (leaf count plus the wide codes' digit counts) or relabel them as measured-basis tripwires. Acceptance: each floor's doc reproduces its constant from its premises; a hand computation of PP(500, 500)'s irreducible touches is at or above its asserted floor.
Construction: not a runtime failure today. Demonstration for the composition: `(input / 8) / 8 = input / 64`. For the rank floor, a settle that delegates the whole wide × dense product to the backend and touches each of x's ~500 digits once plus ~500 leaf folds reads roughly 1,000-2,000 touches on a ~4.6 KB PP(500, 500) operand and trips `touches >= bytes` at 5720 while being strictly cheaper.

### Other suites

### tests-other-17: `forks_max.rs` pins a "documented behavior" no public doc states; `Clock::forks`'s doc is false at that boundary; both docs name `n` for a parameter called `k`; "both profiles" describes a dissolved mechanism
- Where: crates/before/tests/forks_max.rs:1-11 (related: crates/before/src/clock.rs:159-166, crates/before/src/clock.rs:192, crates/before/src/party.rs:239, crates/before/src/party.rs:274, crates/before/src/party/forks.rs:111-114, justfile:113-114)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read clock.rs:166 "The iterator yields `n` children" against the signature `forks(&mut self, k: u64)` at 192 and party.rs:239 "Splits `n` balanced shares" against `k` at 274; `grep saturat` over party.rs, clock.rs, party/forks.rs finds only the private comment at forks.rs:111-112; the justfile runs before's integration tests only through `test-all` (dev profile, line 114) and `--cargo-profile release` appears only for fuzzfit and wasm32-pins; `git show 14f8b019:crates/before/tests/forks_max.rs` shows `#[cfg(debug_assertions)]`/`#[cfg(not(debug_assertions))]` tests; cdad4606 added the saturating-corner sentence to both public docs and a6dcfbb4 (clock.rs) and b3f09baa (party.rs) removed them; 2efff149 renamed the count to `k` "so n means bytes alone"); executed: no
- Seen by: scaffolding, adequacy; refutation: confirmed; history: deliberate-but-expired ("both profiles" was literal under cfg branches that cdad4606 dissolved; the public saturation sentences were removed in Finch's own WIP docs-pass commits, so their return is an owner decision)
- Owner-gated: yes: the public rustdoc content was the owner's own edit; the test-doc clauses and the `n`/`k` mismatch are agent-correctable toward the code

Statement faithfulness: the test doc calls the saturation "the documented behavior", but the only statement of it is a private comment; `Clock::forks`'s rustdoc says the iterator yields `n` children, which is false at `n == u64::MAX` (it yields `u64::MAX - 1`, exactly the reading this test pins); both public docs call the parameter `n` while the signatures name it `k`; and "pinned in both profiles" names a mechanism (cfg-split tests) that no longer exists, while the gate runs these tests in the dev profile only and `saturating_add` leaves no profile-dependent behavior to pin.

Evidence:

         1	//! `forks(u64::MAX)`: the documented behavior at the split count's
         2	//! saturation boundary, pinned in both profiles.

    (clock.rs:166, 192)
       166	    /// The iterator yields `n` children and `self` keeps the last share, so it
       192	    pub fn forks(&mut self, k: u64) -> Forks<'_> {

    (party/forks.rs:111-112)
       111	        // The count saturates: at `k == u64::MAX` the residual's headroom is
       112	        // spent and the iterator yields one share fewer than asked.

Resolution: Owner call on the public docs: either restore a one-sentence boundary note in both `forks` rustdocs ("total over every `k`; at `u64::MAX` one share fewer than asked is yielded") or leave the boundary undocumented and have the test doc say it pins a property the public contract leaves implicit. Either way: rename `n` to `k` in both rustdocs (or the reverse in the signatures), and replace "pinned in both profiles" with the property actually held ("total: no panic in any profile"). Acceptance: `Clock::forks`'s rustdoc no longer claims `n` children unconditionally, or the test doc no longer says "documented"; doc and signature agree on the parameter name; no test doc claims a release-profile run the gate does not perform.
Construction: Read clock.rs:166, then run the test's own steps: `Clock::seed().forks(u64::MAX).len() == u64::MAX - 1` (forks_max.rs:49-50). The doc and the pinned reading disagree at the one input the test exists for.

Constructed test: demonstrated (results.md lines 2840-2878). `Clock::seed().forks(u64::MAX).len()` reads 18446744073709551614, one short of the documented `n`.

### tests-other-28: "At the smallest committed-valid knobs" is a claim nothing pins
- Where: crates/before/tests/verdict_matrix.rs:151-161 (related: crates/before/tests/verdict_matrix.rs:48, crates/before/src/meter/registry.rs:393-510, crates/before/src/meter/registry.rs:859)
- Class / severity / confidence: claim / low / medium
- Provenance: verified (grep `smallest|min_knob|knob floor` over registry.rs finds only "the settle's smallest nonempty configuration" at 859; the registry records constructor knob preconditions in prose, no floor accessor); executed: no
- Seen by: scaffolding; refutation: confirmed; history: deliberate-and-holds for the exhaustive match (39d64f4d and the module doc state the compile-time tie); the minimality claim is prose only
- Owner-gated: yes: a knob-floor accessor on `Shape` is a registry API addition

The exhaustive match is a deliberate and defensible compile-time tie. The residual is a hand-chosen knob per arm (`packed1(8)`, `packed2(7, 3)`, `packed3(10, 2, 384)`, ...) that the doc calls "the smallest committed-valid knobs" while the registry records no floor and nothing checks minimality beyond the constructors' own preconditions.

Evidence:

       157	/// order — capped at two versions and two masks, the pool-budget rule the
       158	/// module doc derives — at the smallest committed-valid knobs, so every
       159	/// operand stays tens to hundreds of packed bytes while keeping its
       160	/// family's adversarial structure.

Resolution: Either add a `smallest()` knob-floor accessor per `Shape` in the registry and derive the pool from it (owner call), or soften the claim to what is checkable ("at small knobs, each operand tens to hundreds of packed bytes") and, where a knob is at a constructor's precondition floor, say so in the arm's comment. Acceptance: the module doc and `matrix_operands`'s doc claim only what a test or the registry pins.

### Benches and examples

### benches-examples-13: The wall cells run under `peak_alloc`, whose `realloc` never grows in place, so the counting allocator's overhead is not "identical across arms"
- Where: crates/before/benches/presize.rs:37-51 (related: crates/before/benches/presize.rs:206-272; crates/before/src/version/skyline/query.rs:513-517; crates/before/src/version/skyline/text.rs:351-354; justfile:807-809)
- Class / severity / confidence: claim / low / high
- Provenance: verified (peak_alloc 0.3.0 src/lib.rs:168-188 from the cargo registry: `realloc` is `System.alloc(new_layout)`, `copy_nonoverlapping`, `System.dealloc(ptr, layout)`, never an in-place extension; presize.rs:50-51 installs it as the global allocator for the whole binary, wall cells included; the growth arms start empty and double, the shipped arms pre-size); executed: no
- Seen by: instrument-correctness [47]; refutation: confirmed, severity lowered (doubling's copy volume is a geometric series bounded by about twice the final buffer, and the record drives no decision yet); history: no rationale (the "identical across arms" claim was written at b28c35ad without a recorded check of the allocator)
- Owner-gated: no

The module doc's claim contradicts the dependency's mechanism: every doubling under `projection_growth` and `display_growth` pays a full copy that the pre-sized arm never pays, so the harness biases the A/B toward the shipped pre-size. The resident column (allocation requests) is unaffected; the wall column is. Moot if benches-examples-12 closes the leg.

Evidence:

        37	//! Wall times are compared *within* a machine and build only; the counting
        38	//! allocator adds a small uniform overhead to every allocation, identical
        39	//! across arms, so arm-to-arm deltas stay honest.
        50	#[global_allocator]
        51	static HEAP: PeakAlloc = PeakAlloc;

    peak_alloc-0.3.0/src/lib.rs:
       176	        let new_ptr = System.alloc(new_layout);
       182	            std::ptr::copy_nonoverlapping(ptr, new_ptr, std::cmp::min(size, new_size));
       184	            System.dealloc(ptr, layout);

Resolution: keep the wall cells on the system allocator: move `resident_report` into its own binary (it is deterministic and needs no criterion), or gate `#[global_allocator]` behind a cfg the wall runs leave off; or install a counting allocator whose `realloc` forwards to `System.realloc` and adjusts the counter by the delta. Restate lines 37-39 to what the harness then does. Acceptance: the binary producing the `presize/*` criterion cells has no copy-realloc counting allocator; `presize-resident` lines still print from a binary that counts.
Construction: run `just bench-alloc-ab presize display_growth presize/display` and `just bench-alloc-ab presize shipped presize/display` as committed and record the ratio; remove lines 50-51 and repeat: the ratio moves toward 1 because the per-doubling copy disappears from the growth arm only.

Synthesis note: Moot if benches-examples-12 (the presize A/B leg's retirement, class vestigial) lands, as the entry says.

### benches-examples-25: results/benchmarks is a 2026-06-02 record of benches that no longer exist, with a wrong mechanism for its largest quoted win and a broken table row
- Where: crates/before/results/benchmarks/README.md:72-73 (related: crates/before/results/benchmarks/README.md:42-46; crates/before/results/benchmarks/speedup.md:10-16; crates/before/scripts/plot_benchmarks.py:117-120, 136; crates/before/benches/party.rs:120-127; .agent-notes/2026-08-04-perf-probe/probe-report.md:151-160)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (`git log -- crates/before/results/benchmarks`: figures from 786fd8e4 (2026-06-02), later commits (58a37d80, 35a09c5b) touched prose only; 7139904b the same day removed Party's `PartialOrd` and the party partial_cmp bench group; plot_benchmarks.py:117-120 still names `party/partial_cmp` `before/ancestor`/`before/equal`, absent from benches/party.rs's group (fork, join, is_disjoint, codec); at 786fd8e4 `causal_cmp` (version/compare.rs:172) returned Equal via `trivially_eq`, a memcmp, so README:43-46's "single-pass compare" never described the equal row; speedup.md:13 splits on the unescaped `|` in "merge  ( | , least-upper-bound)", which comes from plot_benchmarks.py:136; nothing outside the directory and the script references it); executed: no
- Seen by: scaffolding [9]; refutation: confirmed and reframed (the mechanism was wrong at origin, not because of the later ptr_eq rung) plus the new speedup.md:13 row; history: no rationale (stale within six hours of commit; two prose edits since without re-measuring)
- Owner-gated: yes (excise vs regenerate)

Principle 5: dated measurement reports are not exempt; when the code they cite is gone, re-denominate or excise. The record predates the skyline coding, the marker padding, and the perf campaign that moved version join from 2.3x slower to 0.78x (probe-report.md:155); its plot script names dead bench IDs; its explanation of the ~31x equal-case win is the wrong mechanism; a reader finding this directory takes away a performance picture two codings old.

Evidence:

        72	All curves in the committed figures were collected in a single run on
        73	2026-06-02.
        42	- **Big wins** come from operations where the packed form prunes whole subtrees
        43	  cheaply or avoids redundant traversal: `clock/fork` (~15×), `party/fork`
        44	  (~6×), and the `partial_cmp` *equal* case (`version` ~31×, `party` ~12×),
        45	  where the single-pass compare beats a two-pass containment formulation that
        46	  would walk the whole tree twice.

    speedup.md:
        13	| Version | merge  ( | , least-upper-bound) | 32768 | 2.12 ms | 2.10 ms | 1.0× |

    plot_benchmarks.py:
       117	            cmp_panel("party/partial_cmp", "partial_cmp: ancestor",
       118	                      "before/ancestor", "oracle/ancestor"),

Resolution: excise crates/before/results/benchmarks and scripts/plot_benchmarks.py, or regenerate under the live suite (drop the party partial_cmp panels, add `version/hole`, escape the pipe in the merge title, re-run `cargo bench -p before` on a quiet machine, date the README by commit rather than calendar). Drop the mechanism sentence at README:43-46 either way (see benches-examples-17). Acceptance: every group/function the plot script names exists in benches/*.rs; the speedup table matches the live bench roster; the README's mechanism claims match sweep.rs; no table row splits on a pipe.
Construction: `cargo bench -p before --bench party -- --sample-size 10 --measurement-time 1` then `python3 crates/before/scripts/plot_benchmarks.py`: the party partial_cmp panels render empty (`series()` returns None for absent groups) and the version join speedup inverts relative to the committed table.

Constructed test: demonstrated by reading and read-only git (results.md lines 2927-2963; benches are forbidden). The 2026-06-02 dates, the removed party `partial_cmp` group, the dead plot-script ids, and the unescaped pipe were all confirmed; the ~31× attribution and the join speedup inversion were not measured.

### Fuzz and pins

### fuzz-guests-pins-16: The guest's contract says every nonzero return is a harness bug; the harness prices `ERR_OP` as an outcome
- Where: crates/before/fuzzfit/guest/src/lib.rs:21-24 (related: crates/before/fuzzfit/guest/src/lib.rs:533-536, crates/before/fuzzfit/guest/src/lib.rs:1354-1358, crates/before/fuzzfit/harness/src/ops.rs:278-287, crates/before/fuzzfit/harness/src/bands.rs:10-16)
- Class / severity / confidence: claim / low / high
- Provenance: verified (ops.rs:285-287 `pub fn rejected(&self) -> bool { self.expect == ERR_OP }`; bands.rs carries `rejected: true` bands for `ff_clock_join`, `ff_clock_sync`, `ff_party_join`, `ff_party_without`, `ff_rank_checked_sub`); executed: no
- Seen by: adequacy [29]; refutation: confirmed; history: deliberate-but-expired (true at the guest's birth 8cfd3c929; f66d7c172 the same day made `ERR_OP` a predicted, separately priced outcome without touching the guest doc)
- Owner-gated: no

The module doc is the contract a maintainer reads before touching a kernel, and it says the rejection arms are unmeasured error paths that abort the case; the harness predicts `ERR_OP` per step and bands it as its own mechanism (kernel by outcome). The per-kernel comments at 533-536 and 1354-1358 repeat the overstatement.

Evidence:

    21	//! - Every export returns `0` for success and a negative code for a misuse
    22	//!   (missing register, wrong type, operation error). The harness treats any
    23	//!   nonzero return as a harness bug and aborts the case: its generators
    24	//!   construct programs that are valid by construction.

Resolution: State the three codes' roles: `ERR_REG` and `ERR_CODEC` are harness bugs (abort the case); `ERR_OP` is a predicted outcome on the kernels with a rejection arm and is priced as a separate band; `ERR_OP` elsewhere is a harness bug. Trim the per-kernel repeats to point at the module doc. Acceptance: the module doc names `ERR_OP` as a priced outcome for exactly the kernels `bands.rs` carries `rejected: true` bands for.
Construction: Textual: compare guest lines 21-24 with ops.rs:285-287 and the five `rejected: true` bands.

### fuzz-guests-pins-33: Three "PINNED AS FOUND" trap pins are labeled as red baselines awaiting a cure while their docs describe the terminal as intended
- Where: crates/before/wasm32-pins/harness/tests/pins.rs:4-10 (related: crates/before/wasm32-pins/harness/tests/pins.rs:130-153, crates/before/wasm32-pins/harness/tests/pins.rs:359-379, crates/before/wasm32-pins/harness/tests/pins.rs:715-742, crates/before/wasm32-pins/harness/src/lib.rs:23-26, crates/before/tests/meter.rs:1-11)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read the header against the three pins; `git show -s c75d5022` uses the same framing, "the terminal is pinned as found ... A leaner working set — not a wider denomination — is what would move these terminals outward"; `grep -rl 'wasm32-pins' .agent-notes` is empty, so no ruling exists); executed: no
- Seen by: scaffolding [10]; adequacy [21]; structure-prose [38]; instrument-correctness [63]; refutation: confirmed, and raised harness/src/lib.rs:23-26 as a fourth site carrying the same "until its cure lands" contract; history: no-rationale-found (the label is deliberate per c75d5022, which in the same breath declines a cure; the model-versus-defect ruling does not exist)
- Owner-gated: yes: a design decision the owner has not ruled on

The header contracts that a pinned trap "is never an accepted behavior: it is a committed bad baseline its cure must move". `version_decode_memory_terminal_traps`, `ranked_decode_memory_terminal_traps`, and `version_rank_memory_terminal_traps` pin allocation failure inside the 4 GiB address space at about 1 GiB of input, and each doc argues the trap is "the doors' one terminal", that the backend capacity "is unreachable through the doors on this target", and names only a hypothetical "leaner working set" with no cure tracked anywhere. No seam is pinned; what is pinned is a memory constant factor (the docs enumerate the working set qualitatively; no measured multiple exists), a quantity the envelope suite in `tests/meter.rs` owns. Doctrine: no mechanism for accepting known failures may exist; every contradiction resolves to a fix or a model the owner declares, stated positively at the declaration site. These three are in neither genre cleanly, and a fourth `Trapped(...)` assertion added tomorrow for an actual defect would be indistinguishable in kind.

Evidence:

    4	//! Red-first discipline: a boundary found misbehaving is pinned AS FOUND —
    5	//! the assertion names the trap or wrong value, and the doc comment names
    6	//! the wrong behavior it stands for — and the commit that engineers the
    7	//! seam around flips the same test to the correct-value assertion. A pinned
    8	//! trap is therefore never an accepted behavior: it is a committed bad
    9	//! baseline its cure must move. Each pin's own history of red and green
    10	//! lives in this file's git log.
    130	/// PINNED AS FOUND: a valid ~1 GiB (1073741817-byte) version encoding
    131	/// aborts on allocation failure — the doors' one terminal here.
    ...
    145	/// `rank_decode_past_backend_bit_capacity`.) A leaner working set — not
    146	/// a wider denomination — is what would move this terminal outward.
    (harness/src/lib.rs:24-26)
    /// surfaces as the `unreachable` trap under `panic = abort`, and a boundary
    /// found panicking is pinned as exactly that trap until its cure lands, so
    /// the pins assert on this axis directly.

Resolution: Owner rules the genre. If the address-space bound is the model (the evidence reads that way): drop "PINNED AS FOUND" from the three, rename them to state the behavior positively (for example `version_decode_past_address_space_aborts_loudly`), state the working-set multiple they pin, narrow the header and harness/src/lib.rs:23-26 to distinguish red-first seam pins (none today) from declared terminals, and pair the restatement with the origin discriminator from finding 35 so the instrument, not the prose, establishes "allocation failure". If a leaner working set is a planned cure: say so at each pin and track it as open work with its target multiple, and consider whether a heap envelope in `tests/meter.rs` on decode's working set is the right home for the constant factor, leaving the wasm32 leg to seams. Acceptance: every trap pin in the file is either a seam with a tracked cure or a positively stated declared model, and the header's contract is true of every pin below it.
Construction: Textual, not runnable: compare lines 4-10 with 130-146, 359-372, and 715-735 in the same file, and harness/src/lib.rs:23-26.

Constructed test: demonstrated by reading (results.md lines 4650-4683). The header's contract and the three pins' "the doors' one terminal" / "a leaner working set" wording were confirmed at the cited ranges, as was `harness/src/lib.rs:23-26`; whether any agent note tracks a cure was not searched.

### Fuzzfit

### fuzzfit-bands-27: "Every public operation" overstates the 44-kernel vocabulary the bands price
- Where: crates/before/fuzzfit/harness/tests/enforce.rs:441-443 (related: crates/before/fuzzfit/harness/src/lib.rs:4-5, justfile:569-570, crates/before/fuzzfit/harness/tests/sanity.rs:87-96, crates/before/src/testing/validation_index.rs:139-142 and 158-162, crates/before/src/surface.rs:322)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (extracted the guest's `pub extern "C" fn ff_*` exports (107) and `BANDS`' distinct kernels (44) and diffed the sets: 63 exports outside the bands, of which 7 are control or self-test (`ff_nop`, `ff_reset`, `ff_regs_reserve`, `ff_stage_*`, `ff_selftest_quadratic`) and the rest measured kernels for public operations: `ff_span_*` (16), `ff_query_*` and `ff_floor_contains`/`ff_ceiling_contains` (12), `ff_own_span_*`/`ff_own_version_*` (6), `ff_ranked_*`/`ff_rank_encode`/`ff_rank_decode` (6), `ff_clock_display`/`fromstr`/`forks`/`join_all`/`sync_all`/`recv_all`, `ff_party_join_all`/`hash`/`shape`, `ff_version_eq`/`hash`/`shape`/`span`/`span_all`/`ticks`, `ff_shape_combine`, `ff_clock_shape`; `before::surface::METHOD_SURFACE` exists and no fuzzfit test reads it); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no rationale found (the wording dates from the harness's first crate doc; the design note §5 claims a public-API addition fails the roster test by name, which is unfounded since the roster test ties `Op` variants to `BANDS` and never reads the surface; later guest kernels landed for fuelscape panels with "no re-pin")
- Owner-gated: yes: the doc restatement can land now; a tiling test against `METHOD_SURFACE` extends the gate, and extending the vocabulary to the exported-but-unbanded kernels is strategies-partition work the owner scopes

The sentry's doc comment, the crate doc, and the justfile describe the bands as the committed cost law for every public operation, but the vocabulary prices 44 kernels while the shared guest exports about fifty-six further measured kernels for public operations that no band judges and no tiling test accounts for. The board and the fuelscape both carry a tiling against `crate::surface` ("priced or excused, never neither"); the fuzz-fit vocabulary has none, so a public operation outside the 44 is neither priced nor excused while the prose claims otherwise. Every test's doc comment must state its invariant accurately, and an asymptotic claim needs an instrument that fails when it is false.

Evidence:

       441	    /// Every public operation stays inside its pinned fuel band on
       442	    /// shapes nobody chose, and no band key's within-case cost trend
       443	    /// out-climbs its pinned slope.

         4	//! The crate's asymptotic claims (every public operation amortized linear in
         5	//! its denominated size) are guarded elsewhere by chosen adversarial families

       569	# in harness/src/bands.rs — the committed cost law for every public
       570	# operation, so a change that moves an operation's asymptotics fails here

Resolution: Now: restate the three claims as "every operation in the program vocabulary ([`ops::Op`])". Then add a tiling test (sibling to `bands_and_op_roster_name_the_same_kernels`) that walks `before::surface::METHOD_SURFACE` and requires each row to be either a vocabulary kernel or an entry in a committed exclusion list stating its mechanism, so a new public operation cannot land unpriced and unexcused. Owner-gated: extend the vocabulary to the exported-but-unbanded kernels (the span algebra and causally kernels already exist in the guest because the fuelscape prices them; the generator work is the missing piece). Acceptance: the tiling test fails when a surface row is removed from both the vocabulary and the exclusion list; each exclusion names a mechanism; no doc says "every public operation" unless the vocabulary covers the surface.
Construction: Pick an existing public operation with a guest kernel and no band, such as `Span::join` (`ff_span_join`), and run `just fuzzfit`: every leg passes because nothing samples the kernel and nothing lists it as excused, while enforce.rs:441 asserts the claim over every public operation.

Constructed test: demonstrated mechanically (results.md lines 1406-1439). 107 guest `ff_*` exports against 44 distinct band kernels; 63 exports outside the bands (not classified into supporting-code versus public-operation kernels in this run), `ff_span_join` among them. Not run through `just fuzzfit`.

### Fuelscape

### fuelscape-pipeline-1: The overlay is a curated, signature-keyed subset presented as the committed adversarial frontier
- Where: crates/before-fuelscape/src/lib.rs:13-14 (related: crates/before-fuelscape/src/lib.rs:36-39; crates/before-fuelscape/src/families.rs:87-113, 167-180; crates/before/src/meter/board/worst.rs:401, 414, 417, 435, 456; crates/before/src/meter/registry.rs:97; crates/before/src/testing/validation_index.rs:135-136; crates/before-fuelscape/src/compact.rs:146-148; crates/before-fuelscape/src/render.rs:501-509)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (grep: 18 distinct `Shape::` variants used in families.rs against 68 variants of `pub enum Shape` at registry.rs:97; `grep -c` for FreezePosition, PromotionRearm, AscendCliff, DominatedUndercut, LoneFreeze in families.rs returns 0 while all five exist in the registry; read worst.rs:401-470); executed: no
- Seen by: scaffolding [0], adequacy [14]; refutation: reframed (the labels cannot all be `FamilyId::name()` because crosses and fixed-knob ramps have no `FamilyId`; the board's `designed()` applicability is per `OpGroup`, not per op; `WORST_RANKINGS` is `pub(super)`; severity argued down to low because the atlas enforces nothing); history: signature keying is the recorded design with its rationale inline at families.rs:87-97; the lib.rs wording is unchanged since the first overlay commit and predates the registry (68 shapes) and the board's per-op pin
- Owner-gated: yes: binding the overlay to the board's per-operation pin needs new public surface in `before::meter` (`WORST_RANKINGS` is `pub(super)` and its op names differ from atlas rows)

The crate doc promises that the marked points are the committed adversarial families and that one canvas shows the adversarial frontier; the mechanism is a hand list of eighteen registry shapes chosen per operand signature, with no parity pin against the registry or the board, and for many panels the board's tamper-evident worst-case pin names families the overlay never draws (version_tick: ascend-cliff, dominated-undercut, mirror-narrow absent; version_rank: freeze-pos absent; version_display: mirror-narrow, mirror-wide, dominated-undercut absent). This breaches Principle 8 at the instrument level (the doc reports a frontier the artifact does not derive from the thing that pins the frontier) and the roster idiom the same crate states at ops.rs:22, "membership is enforced, never remembered". I keep medium rather than the refutation's low because the claim is repeated in `before`'s validation index and the atlas exists to be read by a human auditing exactly this; a frontier that is not the frontier misleads the one reader the instrument has.

Evidence:

        13	//! committed adversarial families overlaid as marked points on the same
        14	//! axes. One canvas shows the bulk cloud and the adversarial frontier.

    (families.rs)
        98	pub fn overlay_inputs(op: &OpSpec, max_bytes: usize) -> Vec<FamilyInput> {
        99	    let (operands, distinct) = match op.inputs {
       100	        Inputs::Packed(operands) => (operands, false),
       ...
       167	        [Operand::Version, Operand::Party] => {
       168	            out.extend(ramp("dense × scattered_id", max_bytes, |t| {
       ...
       174	            out.extend(ramp("hugeleaf × id_spine", max_bytes, |t| {

    (crates/before/src/meter/board/worst.rs)
       414	    ("default", "version_tick", ["ascend-cliff", "dominated-undercut", "hugeleaf", "mirror-narrow"]),

Resolution: Either (a) derive the overlay from the board: expose a per-operation family roster from `before::meter` (the `WORST_RANKINGS` rows, or a `FamilyId`-per-op table), add an atlas-op to board-op map tested total, ramp exactly those families per row (keeping the fold-cure families for the slice rows), and pin it with a test that every board-pinned worst family for an atlas-covered operation has a point on that operation's panel at a span large enough to admit it; or (b) keep the curated list and re-state lib.rs:13-14 and :36-39, families.rs:5-9, and validation_index.rs:135-136 to say the overlay marks a fixed signature-keyed subset of the committed shapes, with a committed test that names each board-pinned worst family missing from its atlas panel so the gap is visible in the gate. Acceptance: under (a), adding a `Shape` the board prices for a version operation fails `just fuelscape-test` until an overlay or exemption names it; under (b), the atlas docs no longer use "the committed adversarial families" or "the adversarial frontier" for the overlay and the gap list is a committed test.
Construction: Take the board row at worst.rs:414. Call `overlay_inputs` on the `version_tick` row (`Inputs::Packed(&[Operand::Version, Operand::Party])`, ops.rs:407-420) at `max_bytes` 256 and list the `FamilyInput.family` labels: only `dense × scattered_id` and `hugeleaf × id_spine` appear, so three of the four board-pinned worst families for this operation have no point on its panel while lib.rs:13-14 tells the reader the frontier is drawn.

Constructed test: demonstrated by reading the exhaustive `match operands` (results.md lines 3149-3193). For `version_tick`'s signature the overlay is exactly the two ramps `dense × scattered_id` and `hugeleaf × id_spine`, while `WORST_RANKINGS` names ascend-cliff, dominated-undercut, hugeleaf, and mirror-narrow for that operation. `overlay_inputs` was not executed (detached crate).

### fuelscape-pipeline-28: COMBINE_ARITY_CAP claims a smoke-test pin to the guest's cap that does not exist
- Where: crates/before-fuelscape/src/ops.rs:198-203 (related: crates/before-fuelscape/src/ops.rs:205-207, 393-406; crates/before-fuelscape/src/plan.rs:293-311, 395-400; crates/before-fuelscape/src/render/tests.rs:15-22; crates/before/fuzzfit/guest/src/lib.rs:788-810, 831)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (read: the smoke plan is `max_bytes: 8` (render/tests.rs:21), the capped draw is `draw_arity(size.min(cap as usize), rng)` (plan.rs:296), so no smoke arity exceeds 8; the guest's own `COMBINE_ARITY_CAP: u32 = 16` (guest lib.rs:791) refuses `n > COMBINE_ARITY_CAP` with `-1` (808-810) and its `dispatch!` list runs to 16 (831); `grep -rn 'shape_combine\|COMBINE_ARITY\|ff_shape_combine' crates/before-fuelscape/src` outside ops.rs hits only the overlay arm at families.rs:513; no test compares the constants); executed: no
- Seen by: scaffolding [2], adequacy [15], structure-prose [24], instrument-correctness [43]; refutation: confirmed, severity argued to low (dev-tool doc; the guest-below-host direction fails loudly in a survey; the host-below-guest direction leaves the stamped population self-consistent); history: no-rationale-found: the constant, the guest's constant, and the sentence landed in one commit (46eb64f9) that added no fuelscape test; the smoke plan's `max_bytes: 8` predates it
- Owner-gated: no

The doc names a check that does not exist: two hand-maintained constants in two crates must agree, the sentence tells a maintainer they are pinned, and nothing in the gate compares them. If the guest's cap drops below the host's, `ff_shape_combine` returns `-1` and the survey aborts at plan.rs:395-400 only when a column of 17 or more bytes draws an arity past the guest cap, hours after the gate read green; if the host's rises above the guest's, the same. Principle 8 (a claimed pin is "told", not verified) and Principle 6 (the cheapest passing artifact is a green gate over an inconsistent pair). I keep medium rather than the refutation's low because the sentence is a trap set for exactly the edit it describes: a maintainer bumping either cap will trust it and stop looking.

Evidence:

       198	/// The `shape_combine` row's arity cap.
       199	///
       200	/// The public combiner's arity is a compile-time constant, so the guest
       201	/// dispatches one instantiation per arity up to this bound (the guest's
       202	/// own cap, kept equal by the pipeline smoke test's combine case).
       203	const COMBINE_ARITY_CAP: u32 = 16;

    (render/tests.rs)
        18	    let plan = Plan {
        19	        base_seed: 0x5eed,
        20	        samples_per_column: 3,
        21	        max_bytes: 8,
        22	    };

Resolution: Add a boundary test beside the parity tests: build one `Guest`, load `COMBINE_ARITY_CAP` one-byte canonical versions, call `ff_shape_combine(0, COMBINE_ARITY_CAP)` and assert `ret >= 0`, load one more and call with `COMBINE_ARITY_CAP + 1`, asserting `ret == -1`. That pins host cap equal to guest cap at the boundary independently of the smoke's random draws (alternatively export the cap from the guest and derive the host constant from it). Rewrite lines 200-202 to name that test. Acceptance: changing either constant alone (and the guest's `dispatch!` list) fails `just fuelscape-test` by name.
Construction: Lower the guest constant at fuzzfit/guest/src/lib.rs:791 to 8 and trim `dispatch!` to `0..=8`; rebuild the guest. `just fuelscape-test` passes (every smoke arity is at most 8). `just fuelscape --max-bytes 32 shape_combine` then panics with "shape_combine: guest kernel reported -1 at size 32 sample N" on the first sample whose arity draw exceeds 8. Dually, raise the host constant to 32 with the guest at 16: same gate pass, same survey panic.

Constructed test: inconclusive at runtime (results.md lines 6376-6416; the guest rebuild and `just fuelscape` were forbidden). Mechanical checks confirm two separate constants both equal to 16, the smoke plan's `max_bytes: 8`, and no test or assertion referencing either constant.

### fuelscape-render-33: nit: the justfile's head-weight figure for the injected header has drifted
- Where: justfile:251-253 (related: crates/before/docs/fuelscape-header.html; .agent-notes/2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md:260)
- Class / severity / confidence: claim / nit / high
- Provenance: verified (`wc -c crates/before/docs/fuelscape-header.html` = 77988); executed: yes
- Seen by: scaffolding [18]; refutation: confirmed; history: no rationale (written at 61f05692 as "~40 KB", the design note the same day as "~30 KB"; the widget grew through four later commits with neither updated)
- Owner-gated: no

No hand-maintained counts: the comment says non-before pages carry "~40 KB of inert head weight"; the committed header is 77,988 bytes. (The same comment's "activates only on .fuelscape elements" is finding 27.)

Evidence:

       251	# every page's head. RUSTDOCFLAGS is workspace-wide — cargo has no
       252	# per-crate rustdocflags — so non-before pages carry ~40 KB of inert
       253	# head weight; the script activates only on .fuelscape elements.

Resolution: drop the figure ("carry the header's inert weight") or tie it to the file ("the size of docs/fuelscape-header.html"). Acceptance: the comment carries no byte figure, or the figure is derived.

## Paper fidelity

This section is the paper-fidelity sweep's account, cross-referencing the non-claim entries by id. The sweep read `crates/before/reference/itc2008.md` (§3 at lines 100-143, §5.3 at 430-550) against the oracle (`crates/before/src/oracle.rs`, `oracle/version.rs`, `oracle/tests.rs`), the implementation sites the oracle is compared to, the test surfaces that bind them, and the crate-level prose that speaks for the paper. Every claim below was verified by reading both texts side by side unless it says otherwise; nothing in this section was executed.

### The oracle as transcription

The oracle transcribes §5 faithfully on every definition read line by line. `leq` is the paper's lift form threaded through offsets (`oracle/version.rs:100-112`); `join_off` and `meet_off` are the absolute-offset form (114-154); `fill` has the paper's six cases in the paper's order, with `(1, ir)` before `(il, 1)` (246-263 against `itc2008.md:513-518`); `grow`'s tie-break is the paper's (`cl < cr` goes left, else right; 275-285 against 541-543), using the lexicographic `(expansions, depth)` pair that the paper's own closing paragraph sanctions in place of the `N`-weighted integer. The worked examples are transcribed with section citations (`event_normalization` §5.2, `split_equations` §5.3.2, `sum_and_join` §5.3.3, `event_fills_to_single_integer` §5.3.4, `worked_example` §5.1), so the oracle can be audited against the source directly. The brute-force reference (`testing/grow_brute_force.rs`) enumerates the whole inflation space with exact unchecked arithmetic and shares no `deepen` with the DP; the oracle documents its one production coupling (`RouteCost::deepen`, `oracle/version.rs:14-22`) with the reason (paper-fidelity open question 4 asks whether that coupling is acceptable under `oracle.rs`'s independence framing).

The function-space oracle (`testing/semantic_oracle.rs`) is a third reference of a different kind: the paper's §4 construction realized as closures over dyadic points, drawing random §4-valid `fork` and `event` policies per call and naming its one concession (bisecting an indivisible piece). It shares no tree recursion with the implementation or the recursive oracle, which is what lets it catch a bug both recursions share.

### Deviations from the paper and how each is pinned

- **`grow` defines two arms the paper does not** (`oracle/version.rs:268-289`): a `1`-over-node arm (the paper's `(il, ir)` rule read on the unnormalized `(1, 1)`, so the optimality proptests can quantify over arbitrary pairs) and an empty-id arm (the infeasible sentinel). Both are unreachable through `event` (`fill(1, e) = max(e)` collapses first; the `i ≠ 0` precondition keeps `grow` off empty ids), documented at the implementation (`grow.rs:47-48`) and the brute force (`grow_brute_force.rs:57`), and undocumented at the oracle, whose module doc makes transcription fidelity its purpose. Pinned by the differential; owed a paragraph (paper-fidelity-8, documentation, low; oracle-laws open question 7).
- **`Party::join_all` and `Clock::join_all` on the oracle are not paper definitions** but verbatim copies of production's `fold::balanced_try_fold`, and the differentials that consume them pin the hand-back vector element-wise while the public contract declares the absorbed-versus-handed-back set unspecified. The laws already know the coalesced hand-back and convict the dropped-group variant (`laws.rs:2402-2416`, `party/tests.rs:95-105`); only the public `# Errors` prose lags (oracle-laws-2 and paper-fidelity-7, other classes).
- **§3's event condition has three clauses**; two are pinned (`tick_strictly_advances`, `tick_advances`; `grow_dominates_no_more_than_needed` for the scoped minimality reading) and the third, freshness against every other live stamp (`e′ ≰ x` for every other live `x`, `itc2008.md:111-112`), is stated in the function-space oracle's prose (`semantic_oracle.rs:313-315`) and pinned by nothing: the laws' single-value and pair groups cannot quantify over a population (paper-fidelity-10, verification-gap, low). The sweep supplies the population law to add beside `disjointness_invariant`.
- **`grow_dominates_no_more_than_needed`** (`oracle/tests.rs:609-643`) explains why the literal §3 clause `x < e′ ⇒ x ≤ e` is false over the full pointwise lattice and pins the correct scoped reading: the right way to transcribe an informally stated paper property.
- **The paper's `peek` and anonymous stamp** are modeled (a bare `Version` is `(0, e)`; `Clock::version` is `peek`; no anonymous `Party` exists, which makes `event`'s `i ≠ 0` precondition structural) but never named in public rustdoc; a reader arriving from the paper cannot find where they went (paper-fidelity-11, documentation, low).
- **The crate's normal form is the paper's**: `node`/`is_normal` implement the §5.2 normal form, and the skyline canonicality argument (`skyline.rs:65-90`) re-derives that minimal topology is exactly that normal form, that gamma and zigzag are bijections with no negative-zero spelling, and hence that byte equality is semantic equality. The one canonicality rule the id coding leaves to enforcement (a `00` terminal rewritten to `11 00 00`) is exactly what `defect.rs` plants and `parse_id_core` rejects.
- **The space comparison against the paper reports what was measured, where it exists**: `results/space_consumption/README.md:27-41` compares this crate's encoding to the paper's Appendix A figures, including the regime where the crate lands inside the paper's band rather than below it. The front page's "100×" and the "100 parties and 1,000,000 events" figures are the two places the crate's prose outruns that record (paper-fidelity-1, -2, entries above).

### Laws versus the paper's algebra

The laws header (`laws.rs:15-20`) says the algebraic laws "transcribe the ITC algebra (…, §2–§4): versions form a distributive lattice under `|`/`&` whose partial order is causality, ids form a partial commutative monoid under disjoint join with `fork` as its splitting inverse, events inflate strictly and only within the owned region, and `rank` is a strictly monotone valuation". The paper's §2 is Related Work; its algebra is §3-§5; and it requires only a join semilattice (`itc2008.md:119`: "the order must form a join semilattice"), defining no meet, no distributivity, and no rank. `meet_distributes_over_join` (`laws.rs:882`) and `rank_is_a_valuation` (`laws.rs:536`) are the crate's own extensions; so are projection, span, and `causally`. The same off-by-one citation appears at `semantic_oracle.rs:11` ("closure combinator (§2-3)") beside a first line that correctly says §4 (paper-fidelity-6, documentation, low). What the paper does state, the laws pin: ids under disjoint sum with fork as split (`sum_and_join`, `split_equations`), event as strict inflation within the owned region (`tick_strictly_advances`, `tick_only_inflates_the_region`), and the join semilattice whose order is causality (the lattice group's join laws). The sweep's recommendation is to split the header into the paper's algebra and the crate's extensions and to cite §3-§5.

Two orientation pointers for paper-readers are ghosts: `crates/before/AGENTS.md:5-6` names a public `implementation` module (deleted at 22cdfbe1) and a "Law of Disjointness" no source file uses (the crate docs call the rules Causal Singularity and Identity Linearity, `lib.rs:224, 231`), and `version/skyline/build/tests.rs:394` cites the same retired essay (paper-fidelity-4, documentation, low). The other two paper-fidelity entries are a trait-name slip (`PartialEq` for `PartialOrd`, `lib.rs:277-279`; paper-fidelity-12) and the README's CSV column list omitting the bit columns the file carries (paper-fidelity-14), both nits.

## Derivations

The crate's rule is that a cost claim carries its argument where it lives, that the implementation matches, and that a committed instrument fails when the claim is false. This section lists the derivations the review examined and what each was found to establish.

**Derivations found wanting.**

- *The fold's `O((|self| + |iter|) log k)`* (`fold.rs:5-8`; `ops.rs:630`). The counter pairs groups of equal input *count*, so each input passes through at most `⌈log₂ k⌉ + 1` combines; the module doc instead asserts operand-*size* balance, which the counter does not provide (one 125,000-leaf dense version combines with a one-leaf version at `k = 2`). The stated bound then needs a second premise the tree states nowhere in production prose: a combine's output packed size stays within a constant of its operands' total (join, meet, and span subadditivity), derived only in `meter/tier2/tests.rs:328-346` with the tight constant `size(c) ≤ size(a) + size(b) − 2` and pinned by the `*_encoding_is_subadditive*` tests. The paper-fidelity sweep asked where that premise lives (open question 1); version-core-5 asks for it in `join`/`meet`'s public `# Complexity`. Both premises are true as far as the instruments reach; the prose states the wrong one and omits the other (crate-root-17, paper-fidelity-5, version-core-5).
- *The builder's cascade amortization* (`build.rs:29-34`). "Each cascade step deletes at least three stream bits and copies only a code already priced by that deletion" prices the copy but not the flush: after a re-anchor the extracted code becomes the held leaf, the next leaf's flush writes it again (154-155), and the next cascade extracts it again one level up (333-335). Three independent hand traces and one constructed run agree the total is `Θ(d · W)` on `Θ(d + W)` input (skyline-coding-9).
- *The masked cost derivation* (`masked.rs:52-56`). "Every path bit pushed and popped at most once" covers the pops and not `peek_flip`'s non-popping reads, which `block_skip` performs once per round on a parked unowned side; `trailing_ones` prices itself per call as "the run the caller is about to pop (or has decided not to)", correct per call and paid `Θ(n)` times without a pop (skyline-sweep-place-masked-5, codec-bits-29).
- *The filter walks' cost* (`filter.rs:37-48`). The derivation acknowledges "O(#bounds) bookkeeping absorbed by the same per-interval read loop" and then states `O(|v| + Σ|bound|)` with no `k`; the read loop performs one `sign()` per live pair per elementary interval and `advance_set` two `O(k)` passes per boundary, so the total is `Θ(k · (|v| + Σ|bᵢ|))`, asymptotically worse than the composition on the bounds' term for many large holes against a small probe (skyline-sweep-place-masked-21, span-causally-24).
- *`Query::coverage`'s cost* (`query.rs:114-117, 152-171`). The exactness argument for `refine_partial` (143-151) is correct and tight; the cost of `self.holes.iter().any(|hole| P::hole_covers(...))`, one full `partial_cmp` sweep of the clamped endpoint per hole when that endpoint strictly dominates every hole, is nowhere derived (span-causally-36).
- *`sum_ranks`' amortization* (`rank.rs:1011-1018`). "A summand raising the maximum rescales the accumulator once, O(held digits) — paid by the exponent the summand itself carries" charges the rescale to the wrong quantity: `Accumulator::shl` is `O(|self|)` digit touches (suanpan `accumulator.rs:594-595, 626-627`), so `n` unit raises after a `W`-bit rank cost about `n · W/32` touches against `W + n(n+1)/2` content bits. A linear algorithm exists (geometric headroom: shift by `max(gap, held_bits)` and carry the surplus as exponent headroom, stripped at the end by the existing `trailing_zeros`/`shr`), so the claimed bound is achievable (rank-20).
- *The rank size bound* (`rank.rs:198-202`). The bold sentence asserts an unqualified "never"; the paragraph that follows argues only a small-constant-factor bound in the rank's binary expansion. In the provenance pin's own bit denomination the version `1` (4 live bits) has rank golden `[0x80]` (8 bits) and `"(0, 1, 0)"` (9 live bits) has rank golden `[0x60, 0x00]` (16 bits); the constructed run also refuted the entry's byte-level clause (see the entry's synthesis note). A byte-size bound, if wanted as a contract, needs the per-level derivation the resolution sketches (rank-4).
- *`Rank::decode`'s `NotCanonical` provenance* (`rank.rs:442-444`). What needs 2 EiB is a *canonical* stream with a 2⁶⁴-bit mantissa, not the rejection, which a nine-byte forged header reaches (and the committed genre test does reach). The sentence conflates the trigger with the bound's provenance (rank-10).
- *`MAX_SCALING_EXPONENT`'s exclusion* (`ceilings.rs:64-69`). `judge::trend` transcribed to Python and run on four-point ladders `[b, 2b, 4b, 8b]` with work `8n · log₂(8n)` reads 1.100 at `b = 1 KiB` down to 1.067 at 128 KiB, all under 1.15; `n · log² n` reads 1.13-1.20; a quadratic reads 2.000. What 1.15 excludes is polynomial super-linearity; a log factor in the operand size sits inside the slack, and the same file's fold model (342-344) quotes a log marginal at ~1.14-1.17 straddling the ceiling (board-frame-8).
- *The envelope flatness band's exponent* (`tests/meter.rs:2392-2408`). For cost `c · nᵃ`, the per-unit ratio across one doubling is `2^(a−1)`, so `5/4` admits `a ≤ 1 + log₂(1.25) ≈ 1.32`; `10/9` would match the board's 1.15. The band docs say "linear" (envelopes-a-16).
- *The settle level ratio* (`tests/meter.rs:5830-5836`). The comment's own formula `log₂(2n)/log₂(n)` gives 1.5 at `n = 4` and 1.33 at `n = 8`, the committed probe counts; the quoted ×1.17 (7/6) corresponds to `n = 64`, which no probe runs. The band holds because the settle does not dominate, as the sentence's second clause says (envelopes-b-4).
- *The `input / 8` touch floors* (`tests/meter.rs:8384-8410`). The two stated premises (one touch per 64-bit limb, payload at least an eighth of the input) compose to `touches ≥ input / 64`, not `input / 8`; the floor holds by a ×30 margin. The rank probes' `touches ≥ bytes` floors carry no derivation at all, and a settle that delegates the whole product to the backend would trip one while being strictly cheaper (envelopes-b-20).
- *The `wide_arming` guard's rationale* (`meter.rs:1904-1912`). "The parked component must clear the settling drift's ten digits by more than the freeze allowance" describes `w + 1 > 10 + 8`, so `w ≥ 18`; the guard admits `w ≥ 10`, at which the arming's own unit code trips the freeze but no promotion fires. Demonstrated with a counter tap: zero promotions for `w ∈ {10, 12, 17}`, one from `w = 18`; the committed `hoisted_window` band runs at 12 (meter-core-8).
- *The integral's shift panic-freedom* (`integral.rs:464-471`; `web.rs:136-141`). Argued from "the storage caps below 2^32", which the codec denies at `bits.rs:117-119` and `buf.rs:25-31`. The conclusion survives on the achievable bound (allocatable memory keeps the stream under 2³⁵ bits on a 32-bit target, so digit positions stay under 2³⁰ against a 2³²-digit `usize`, a margin of two binary orders, not "multiple"; on 64-bit the panic is unreachable outright) (skyline-query-13).
- *`IdIndex`'s space and search bounds* (`index.rs:15-17, 43-44`). A both-present node and its forced subtree cost at least four stream bits while the table costs 32 per such node, so the table is up to eight times the operand, not "strictly smaller" (party-22); the `B` in `O(Σ inputs + B log n)` counts the inputs' both-present nodes while the code searches whenever the indexed node is both-present and the input node is `Internal`, and discards the result in the `(true, false)` arm (party-25).
- *`Ticks`' `Sum` bound* (`ticks.rs:41-42`). `O(N)` with `N` the summands' total width reads `O(0)` on `k` copies of `Ticks::ZERO`; the true bound `O(N + k)` also needs the binary-counter potential (each carry clears a bit an earlier summand set), which the line does not state (version-core-23).
- *The asymptotics floors' attribution* (`asymptotics.rs:12-14, 339-349`). A ratio of `a · k · log k + b · k` across a quadrupling moves with `b/a`; exact counters remove noise, not that ambiguity. For `Party::join_all` the operation reads ×4.985 against bytes ×4.80 with the floor at 4.89 (1.9% from either endpoint; `b/a ≈ 43`), so a ~5% change in the linear index term crosses the floor with the log factor intact (testing-diff-gen-28, readings re-measured by the constructed run).
- *The space figures* (`lib.rs:312-319`). The `git log -S` trail shows the numbers are evaluations of two closed-form estimates (`⌈ln(100)/2⌉ = 3`; `⌈50 + 100 · log₂(10⁴)/24⌉ = 106`) that d45597843 deleted the same day it kept their outputs. Evaluated at the one point the committed run can check (`N = 128`, `E/N ≈ 195`), the deleted formula gives about 105 B for the version alone against a measured whole-stamp 146 B, so restoring it would not close the gap either (paper-fidelity-1, crate-root-29).
- *The lazy-zone amortization* (suanpan `lib.rs:66-82`). The page promises every argument "in full"; the sign-fold section (103-113) and the ledger section (150-161) deliver, while the lazy-zone section states two facts and asserts the geometric thinning. The sweep supplies the missing potential: `Φ = Σ_{i≥1} |dᵢ| / 2³²`; a machine-word deposit raises `Φ` by at most about 1, every further carry out of a digit `i ≥ 1` requires `|dᵢ + c| ≥ 2³³` and leaves `|r| < 2³¹`, lowering `Φ` by at least about 0.5 at digit 1 or 1.5 higher, so every carry touch is prepaid and a wide operand's per-limb contributions each add less than 1 (paper-fidelity-9, documentation, low). A neighboring step in the ledger argument is false as written: "a nonzero partial decides within one step" fails for a partial of magnitude 1 or 2 over a digit near `−s · 2³²`; what the conclusion needs, and what `accumulator.rs:138-142` states, is decision within one step over a *zero* digit (suanpan-4).

**Derivations found sound** (listed so a maintainer knows which arguments to trust as written).

- The skyline canonicality argument (`skyline.rs:65-90`): minimal topology, natural heights, exactness; gamma and zigzag bijective with no negative-zero spelling; byte equality is semantic equality. Re-derived by the sweep.
- The rank wire form (`rank.rs:8-131`): the inverted-polarity delta header shown bijective, the fraction's in-band framing justified against a length header by the 1/2-versus-7/16 counterexample, prefix-freeness from the close bit, every rejected alternative named with the mechanism that rules it out; the decoder's comments (722-787) mirror it clause for clause.
- `Rank::cmp`'s class test on `bits(num) − exp` with zero settled first, then MSB-aligned windows with the odd-numerator tail rule stated at the site (`rank.rs:882-906`).
- suanpan's sign-fold bound (`2.01 · 2^(32·i)`, stop at `|s| ≥ 3`; `lib.rs:103-113`), the zero-run ledger's credit argument (150-161), `read_digits`' final-carry closure over `[−3, 2]` (`accumulator.rs:1074-1079`), and `add_at`'s recentering (`|carry| ≥ 2` whenever `|total| ≥ 2³³`).
- The subadditivity constant `JOIN_MEET_SUBADDITIVITY_SAVINGS_BITS` derived term by term (topology, first leaf, `gamma(2m) = gamma(2m − 1)`) with the equality case committed as a test (`tier2/tests.rs:328-346, 496-516`).
- The closed forms the two-scale pins rest on: the fork orbit `7 + 2 · ⌊log₂ k⌋` (3 topology bits + gamma(0) + gamma(2k)); three bits per deep-spine level; the compactness comb's `pairs · (2 · m_bits + 8) − 2`; the id-walk scan `2 · (2d + 2)`; `plateau_puncture`'s stored size `128w + 198d + 2` and rank `(2xy + 1)/2^(66d)`; the harmonic telescoping `1 − rank(H(d)) = 1/2^d`.
- The derived liveness floors in the envelope suite with their arithmetic checked at the site: `PURE_COMB_TOUCH_FLOOR` 2·999 + 32, `ASCEND_CLIFF_TOUCH_FLOOR` 4·1,999 + 64, `PLATEAU_TOUCH_FLOOR` 1,999 + 64, `DOMINATED_UNDERCUT_TOUCH_FLOOR` 1,024·(48 + 2), `HOISTED_WINDOW_DENSIFY_FLOOR` 2·40, `SEAM_STOP_POOL_WARMUP` = 2 from peak simultaneous demand; and the board's `touch_pair_fold` (max not sum, zero deltas excluded with the mechanism stated).
- `TEXT_BYTES_PER_RADIX_UNIT` (`ceilings.rs:203-216`): at most 6 syntax bytes plus digits per value, one radix unit minimum, hence under 7; enforced by `assert_honest_text` on both the board and bench paths and tripwired.
- `BACKEND_CAPACITY_BITS` as `usize::MAX / word-bits` words matching dashu-int 0.5.0's `Buffer::MAX_CAPACITY`, held from above by the wasm32 past-capacity pins.
- The fuelscape samplers' exact uniformity: the `bare`/`internal` and `term_left`/`deep_left` weights partition exactly the pairs the counting recurrences count, with the `Θ(1/√leaves)` rejection rate for the nonnegativity rule derived at the site.
- The board's mod-32 remainder-alignment derivations on the base constants (`family.rs:127-139` and siblings), which explain why an amortized-O(1) constant would otherwise read as growth across the ladder.

## Positives

Claims found fully backed, with the instrument that holds each, deduplicated across the partition reports and the sweeps. Each was verified by reading the argument and the instrument by at least one finalizer; where a constructed test also ran, it says so.

- **Canonicality is argued, implemented, and pinned from every side.** `skyline.rs:65-90` derives the three conditions and the bijections; `validate.rs` is exactly its module doc's contract (two bits per open ancestor, one accumulator, no panic reachable from bytes); the reject corpus covers every genre with its exact variant and sweeps truncation at every cut; accepted mutants are re-derived through the oracle bridge into a fresh composite, the one comparison a lax validator cannot pass; `fuzz_decode` asserts byte identity on accept, refusing an accept-and-normalize decoder outright; the fuelscape's grammar-versus-decoder set-equality census to two bytes holds the coding's population from a third side. Two qualifications ride with this (the claims table row carries them too): one decode door's canonicality arm, the admission walk's mid-stream collapsible-pair rejection through `Span::decode`, has no committed witness (skyline-coding-6, demonstrated), and `fuzz_decode` executes only at `just all` cadence, outside the gate (fuzz-guests-pins-38).
- **The no-depth-recursion rule holds and is proven beyond what AGENTS.md advertises.** The recursion sweep's call-graph scan finds no library function recursing on tree depth; three depth-100k clock tests, the id text parser's 100k round trip, the diff ladder at 100,000, and envelope rows at `ID_DEPTH = 250,000` (including the public `without`) are the proofs; every explicit stack states its per-level cost at its declaration. The breadth is on the operation and depth axes; on the shape axis the proof is narrower than `AGENTS.md`'s "every public op": the depth-100k tests drive a listed subset over one left-only spine, so both-present id frames and right descents have no overflow-depth witness, and the shape iterators appear in no deep test (clock-22, recursion-6, meter-adequacy-1).
- **The oracle is a faithful transcription of §5**, with the two deviations named and pinned (the `grow` arms, the fold copies), and the function-space oracle is a third reference sharing no recursion with either. `grow_dominates_no_more_than_needed` transcribes an informally stated paper property the right way: by stating why the literal reading is false and pinning the scoped one.
- **suanpan's cost table is derived and exactly pinned.** The sign-fold bound and the ledger credit argument hold as written with their potentials; the exact-count-at-two-scales pin form makes liveness floor and flatness one assertion; the known-bad collapse-less fold is committed and shown red at two widths; the three decision constants are pinned tight by constructed corners that mutation testing motivated. The only gaps are the lazy-zone potential (supplied in paper-fidelity-9) and one false intermediate sentence (suanpan-4).
- **The rank wire form's ordering and prefix-freeness argument is complete and checkable**, mirrored clause for clause in the decoder, held by `ranked_encoding_orders_like_ord` and the exhaustive 0-2 byte sweep that asserts both rejection genres fire so strictness cannot pass vacuously; `Rank::cmp` is O(1) then windowed as documented.
- **Relational cost identities that cannot rot.** The `placement`, `span`, `span_codec`, and `identity_fast_paths` sections of `tests/meter.rs` state each fusion's cost as an exact identity against its composition on the same operands (`fused + cmp_ss / 2 == cmp_sv + cmp_se`; `fused == decode_lo + cmp`), each with a nonzero liveness read and a walking control; `admit.rs`'s fused-parse claim is pinned the same way. There is no constant to re-pin.
- **Closed-form two-scale pins with derivations at the site**: the fork orbit's `7 + 2·⌊log₂ k⌋`, three bits per deep-spine level, the id-walk scan's `2·(2d + 2)` at two depths (two-sided exact equality, with the reason a ceiling plus slack floor cannot see a uniform undercount), the compactness comb's closed forms with the factor-2 envelope shown tight (> 1.994), `masked_cmp_hole_depth_band`'s `lo == hi`, and `join_all_equal_operands_is_clone_cheap`'s byte-identical peak heap across a 4× growth.
- **Derived liveness floors done the way the doctrine asks**: `touch_pair_fold` (per boundary not per element, max not sum, zero deltas excluded with the mechanism stated); the envelope suite's `SEAM_PLUNGE`/`SEAM_STOP`/`LADDER_MARGINAL` floors from irreducible register folds; the weight-comb and freeze-parade floors with "the mechanism's irreducible work, not the family's typical work" stated; `pool_recycle`'s ceiling of 2 from peak simultaneous demand, which `.cargo/mutants.toml:65-70` relies on to kill the retire/lease mutants; the fuzz-fit register reserve tied to both budgets by a `const` assertion.
- **Known-bad demonstrations that make criteria non-vacuous**: the plain-accumulator sweep (`cliff_comb_plain_delta_sweep_is_quadratic_in_tier2_wire_bits`, value-exact, two-scale, cited from production rustdoc), `parse_schoolbook` (asserted red at ≥ ×1.5 per byte while the shipped kernel stays ≤ ×1.25), eight value-exact superlinear kernels rostered by name in `tests/superlinear_tripwires.rs` in both directions, the bench judge's schoolbook renderer and unmetered quadratic pinned in `--self-test`, and the board's meter-bypassing walk, chunked schoolbook, and fat-constant fold model each convicted through `evaluate` itself.
- **Space claims hold where the crate's own record speaks.** The results README compares against the paper's Appendix A figures as measured, including the regime where the crate lands inside the paper's band; at small scale 13 parties encode in 1-2 bytes each and a 23-event fleet root's version in 9 bytes (the fresh-eyes sweep's run); the constructed 128-entity run reads 1,791.7 B per stamp against the paper's 1,776 B.
- **Denomination is stated and mechanically checked where the board and the atlas meet**: `Denom`/`IoSpec`/`TextSpec` with output readers over the actual result, `assert_honest_text` on every text stream entering a denominator, and `size_measure` on every atlas dataset (the one leak is the widget's uniform caption, crate-root-6).
- **The fuelscape samplers are exact-uniform by construction and checkable from the code**, with a triangle of independent oracles (table versus enumeration, grammar versus decoders as set equality, decoder census, fixed-seed chi-square).
- **The watermark's cost claims resolve to named instruments.** Every cost claim in `watermark.rs`'s module doc names a committed instrument and each name resolves: `skyline_min_ticks_latent_ladder_is_flat_per_unit` (tests/meter.rs:3449) for the O(1) latent decision, the `skyline_min_ticks_seam_*` bands for `propagate`'s wide hops, `pool_recycle` (9382) for the pool claim, `dominated_undercut_cost` (9209) for the arm-liveness floor; the suanpan witnesses are cited by stable test name with the assumed clause restated inline (watermark.rs:488-494, 751-771). The latent-ladder band (tests/meter.rs:3434-3490) is an order pin rather than an envelope: a per-decision marginal compared across a doubling of the parked latent's width in both directions, with a floor derived from three irreducible register folds (skyline-watermark partition; verified by reading). The one number in that module no instrument reproduces is `compacting()`'s dated ratio pair, listed above under *Numbers without artifacts* (skyline-watermark-8).

## Open questions for Finch

Deduplicated across the reports; each carries the reports' recommendation.

1. **The front-page bound** (crate-root-25, paper-fidelity-3, crate-root-40). Ruled (owner ruling 1, 2026-09-02, `triage/rulings.md`): the asymptotic claims hold absolutely; the five demonstrated rows (join re-anchor, `Ranked::cmp`, masked `peek_flip`, coverage per-hole sweeps, memo heap) are defects, not exceptions, and no owner-declared model is on the table. Still open: whether to tighten the headline to the per-operation contracts ("linear for the core operations (tick, fork, join, meet, comparison, the codecs), near-linear for the n-ary folds, multiplication-bound where the answer is a wide integer") once the five are fixed. Recommendation: tighten it then; `just amp-board-acceptance` cannot detect the breaches, because its committed families are the shapes on which the claims hold, and the constructed rows already have.
2. **The 100× figure and the space paragraph** (crate-root-24, paper-fidelity-2, crate-root-29, paper-fidelity-1). Is there an off-tree measurement behind either? Recommendation: extend `examples/space_consumption.rs` to emit the oracle tree's boxed footprint beside `encode().len()` and a party/version split, commit the run, and cite it with the denominator named ("against heap-boxed paper trees"); otherwise drop the multiplier and re-denominate the paragraph to the committed run's parameters and fitted exponents (0.91 static, 1.85 dynamic between 64 and 128 entities).
3. **Join's re-anchor cascade** (skyline-coding-9). Ruled (ruling 1): cure; `O(|self| + |other|)` for join is the crate's headline guarantee and restating it is off the table. Land the two-scale scan pin on `join(Hugeleaf, spine_of_pairs(d))` as an envelope row and a board family first, red before the fix. Candidate cure, unverified: hold the re-anchored code in place at the stream's tail instead of extracting and re-splicing it, with the two-stream builder (topology and payload interleaved once in `finish()`, dissolving the held leaf, `lens`, and `extract_code`) as the fallback. Then audit the join, meet, span, rank, masked-comparison, and coverage instruments for the benign-face blind spot and report what else they miss.
4. **`Ranked::cmp`'s contract** (rank-33): restore `O(M(|self| + |other|)) · log(...)` (mirroring `version_distance`) and regenerate the island, adding `plateau_puncture` and `wide_arming` pairs to the `ranked_cmp` `OpSpec`? Recommendation: yes, now; a sign-only kernel cannot be linear on exact ties, so a domination-certificate early exit is future work that leaves the worst case M-bound.
5. **The masked `peek_flip` term** (skyline-sweep-place-masked-5, codec-bits-29): guard each peek with `depth() > bound`, or cache the flip level in `LeafCursor`? And should the board gain a currency that sees stack-word reads (charge 64 scan bits per word inside `trailing_ones`)? Recommendation: the guard (a necessary condition that changes no step and makes every peek amortize against an immediate pop), plus the scan charge so the dual family reads on existing floors and ceilings; the projection loop (`query.rs:533-549`) needs the same check.
6. **Multi-hole query contracts** (span-causally-24, -36, skyline-sweep-place-masked-21, span-causally-25): keep `O(|self| + |span|)` and "linear time" as the promise, or restate with the hole-count factor? Recommendation: fuse `refine_partial` regardless (a fixed-sign deletion of `k − 1` endpoint decodes), then restate "linear time" as "linear in bits decoded, with per-interval work proportional to live holes" and pin the `k` axis in the touch currency, never scan bits; rewrite the `causally` module summary as three clauses.
7. **The fold contract's second premise** (version-core-5, crate-root-17, paper-fidelity-5). Where should the join/meet subadditivity lemma live in present tense: the public `# Complexity` of `join` and `meet` (which rumors's mirror window already relies on), cited from `fold.rs` as the reason a merged group cannot outgrow its inputs? Recommendation: yes, with the derivation's one-line shape; before writing it down, extend the subadditivity proptests to deeper generators and the meter families (the constructed run found no counterexample on 128 adversarial pairs).
8. **Auxiliary-space scope and the memo families** (skyline-fill-grow-2; paper-fidelity open question 2). Is `lib.rs:333-340` scoped to the committed families, and should `MemoChain(distinct)` and `MemoComb` join the board's tick group under 16 B/B or get a declared model at the constant you ratify? Recommendation: add a `peak_heap` reading to `memo_resolution_cost` first (red at 50-105 B/B today), then decide between a declared model and the word-compaction cure (`Boundary::Word | Wide`, the trade `MinWeb` already makes).
9. **`sum_ranks`** (rank-20): land the ascending-order touch row red, then the geometric-headroom cure, and add `# Complexity` to both `impl Sum` blocks? Recommendation: yes; the two committed pins' "adversarial order" prose is corrected in the same change.
10. **The fold index past 2³² bits** (party-23): widen the table (`Narrow`/`Wide` or `Vec<u64>`, measured at the parent) and dissolve `build_unindexed` and the fallback differentials, or carry the size clause in the island contract? Recommendation: widen; the "for all input sizes" sentence is the one you chose to make the contract.
11. **The tuple-literal constructors** (party-11, codec-base-text-tree-13, version-core-16, skyline-coding-20, clock-14): restate as `O(n · d)`, or reshape the sealed `PartyLiteral` to emit top-down into one buffer and the version composer to fold children without `Vec<Base>`? Recommendation: the reshape for `Party` (sealed and hidden, so no reachable surface moves) with the per-level `validate_id` deleted either way; the version composer's fix is the same idea.
12. **`forks` at `u64::MAX`** (clock-3, tests-other-17): restore the one-sentence boundary note (cdad4606's wording) to the four public docs, or leave it undocumented and soften the test doc's "documented behavior"? Your doc passes removed the sentence without a message. Recommendation: restore it, rename `n` to `k`, and reword "keeps the last share" to the residual's true position.
13. **The envelope flatness slack** (envelopes-a-16, envelopes-b-4): share the board's bar (`10/9` per doubling for 1.15) or keep ×1.25 and state the exponent bound each band enforces, normalizing every model-bearing band by its model so ×1.25 means one thing? Recommendation: keep ×1.25 where a declared log model needs it and state the bound; tighten the rest if the pinned readings allow.
14. **What `1.15` excludes** (board-frame-8): re-word to "polynomial super-linearity", and add a committed probe that an n·log n ladder reads under the ceiling beside the quadratic tripwire? Recommendation: yes; the exponent leg's documented reach is what a maintainer triages by.
15. **The wasm32 terminal pins** (fuzz-guests-pins-33): declared model or pending cure? Recommendation: declare the address-space bound as the model, restate the three pins positively, narrow the header to the boundary pins it describes, and add the origin discriminator so the instrument establishes "allocation failure".
16. **The atlas overlay** (fuelscape-pipeline-1): bind it to the board's per-operation worst-case pin (exposing a roster from `before::meter`), or keep the curated list and restate `lib.rs:13-14` with a committed gap-listing test? Recommendation: restate now; decide the binding as a design item.
17. **The fuzz-fit vocabulary** (fuzzfit-bands-27): is the 44-kernel scope a standing decision? Recommendation: restate the three "every public operation" sites to "every operation in the program vocabulary" now, add a tiling test against `METHOD_SURFACE` with a reasoned exclusion list, and extend the vocabulary as strategies-partition work.
18. **The validation index's scope** (testing-oracles-28, board-frame-26): a total map with a light liveness pin (every gate/ci recipe and every `tests/*.rs` binary named), or a scoped page pointing at `just --list`? Recommendation: total and rendered; re-word the "class-scale" sentence to name which legs are envelope-class under the ×1.25 convention.
19. **Denominations and notation** (clock open question 6; rank-31; skyline-query-28): define `|iter|` beside `|x|` in `lib.rs` (total encoded size of the items, or their count?); define `M` once in a crate-level complexity section; name the dashu tier thresholds once with the bump note. Recommendation: all three, small and mechanical.
20. **suanpan's exact-touch contract** (suanpan open question 1): does "a change to any operation's count is a breaking change" freeze constant-factor improvements? Recommendation: restate as "deterministic and pinned; a count change is a versioned change named in its commit", which keeps the pins as enforcement without forbidding improvement. This decides whether rank-20's cure and the fused per-limb pass are admissible.
21. **The instruments' blind spots as a standing item** (rank-33 and codec-bits-29 constructed tests): the limb meter is linear in the `M(n)` term by denomination and no deterministic counter sees stack-word reads; the bench judge's two-scale wall exponent is the only committed currency that saw either. Recommendation: record both blind spots in the validation index's rows for the limb and scan currencies, and give the bench judge a `ranked_cmp × PlateauPuncture` cell.

## Counts

By severity (all 75 entries of class `claim`, including the paper-fidelity sweep's four):

| Severity | Count |
|---|---|
| high | 5 |
| medium | 35 |
| low | 22 |
| nit | 13 |
| total | 75 |

By module (partition or sweep key; high/medium/low/nit):

| Module | Entries | high | medium | low | nit |
|---|---|---|---|---|---|
| crate-root | 8 | 0 | 4 | 4 | 0 |
| paper-fidelity (sweep) | 4 | 0 | 3 | 1 | 0 |
| clock | 2 | 0 | 1 | 1 | 0 |
| party | 7 | 0 | 6 | 0 | 1 |
| version-core | 5 | 0 | 1 | 2 | 2 |
| rank | 6 | 1 | 3 | 1 | 1 |
| span-causally | 3 | 1 | 1 | 1 | 0 |
| skyline-coding | 5 | 1 | 1 | 1 | 2 |
| skyline-fill-grow | 1 | 0 | 1 | 0 | 0 |
| skyline-sweep-place-masked | 4 | 1 | 1 | 0 | 2 |
| skyline-query | 2 | 0 | 0 | 1 | 1 |
| codec-bits | 4 | 0 | 1 | 0 | 3 |
| codec-base-text-tree | 1 | 0 | 1 | 0 | 0 |
| suanpan | 1 | 0 | 0 | 1 | 0 |
| meter-core | 1 | 0 | 1 | 0 | 0 |
| meter-registry-tier2 | 1 | 0 | 1 | 0 | 0 |
| board-frame | 2 | 0 | 0 | 2 | 0 |
| board-families-floors-judge | 1 | 0 | 0 | 1 | 0 |
| surface-roster | 2 | 1 | 1 | 0 | 0 |
| testing-diff-gen | 1 | 0 | 1 | 0 | 0 |
| testing-oracles | 1 | 0 | 1 | 0 | 0 |
| envelopes-a | 1 | 0 | 0 | 1 | 0 |
| envelopes-b | 2 | 0 | 0 | 2 | 0 |
| tests-other | 2 | 0 | 1 | 1 | 0 |
| benches-examples | 2 | 0 | 1 | 1 | 0 |
| fuzz-guests-pins | 2 | 0 | 1 | 1 | 0 |
| fuzzfit-bands | 1 | 0 | 1 | 0 | 0 |
| fuelscape-pipeline | 2 | 0 | 2 | 0 | 0 |
| fuelscape-render | 1 | 0 | 0 | 0 | 1 |
| total | 75 | 5 | 35 | 22 | 13 |

Constructed tests: 36 of the 75 entries have a record in `evidence/witness/results.md`; 32 read `demonstrated` and 4 `inconclusive` (version-core-5, a missing sentence no run can show; crate-root-40, whose named settling instrument, the board, was forbidden to the reviewers and could not have settled it (crate-wide patterns); party-23, unmaterializable within the test cap; fuelscape-pipeline-28, a guest rebuild outside the permitted commands). No constructed test refuted an entry's verdict; two corrected an entry's evidence (rank-4's byte-level clause; testing-oracles-28's count of unrowed instruments) and one its trace detail (meter-core-8's freeze count), each recorded in the entry's synthesis note.
