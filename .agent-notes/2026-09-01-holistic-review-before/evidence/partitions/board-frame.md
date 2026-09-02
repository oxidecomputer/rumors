# Partition board-frame: The amplification board frame: module root, ceilings, cells, coverage, currencies, defects, export

## Partition summary

The board frame is the declarative skeleton of `before`'s amplification board: `board.rs` (300 lines) is the module root and its 260-line rustdoc states the criterion, the three-axis product, the liveness floors, the time leg, the ladder, denomination, declared models, the rejection surface, and the coverage tiling; `ceilings.rs` (467) holds every pinned global ceiling, floor parameter, and declared per-cell model with its derivation; `cell.rs` (323) is one prepared cell with its `Denom`/`IoSpec`/`TextSpec` denomination rule and the output-honesty assertion; `currency.rs` (141) defines the five-field `ByCurrency<T>` axis and `Liveness`; `defect.rs` (176) builds the rejection rows' maximally deferred defects; `coverage.rs` (663) is the two-table tiling over `before::surface`, with `coverage/tests.rs` (106, the only test file in the partition) enforcing it; `export.rs` (162) exposes cells to the bench suite and derives the pinned bench subset. I read all 2338 lines with line numbers, plus the sites outside the partition every surviving finding depends on (recurse.rs, Cargo.toml, the justfile recipes, judge.rs, measure.rs, operand.rs, floors.rs, ops.rs, board/tests.rs, codec/bits.rs, codec/text.rs, codec/display.rs, family.rs, clock.rs, version.rs, surface.rs, validation_index.rs). I ran no cargo, just, or build command; the one execution was a Python transcription of `judge::trend` to settle an arithmetic claim.

The frame is, on the whole, an instrument built the way the doctrine asks. `ByCurrency` is a genuine compile-time totality mechanism (no `Default`, no `..`, an exhaustive destructure in `each`), so a half-wired currency is a compile error rather than a convention. The ceilings file states and follows the present-tense discipline for measurements (readings live in pin commits, `git log -S` the constant; c0b5d701 carries the touch ceiling's basis). The output-honesty ceiling is derived, implemented as an assertion on every text stream entering a denominator on both the board and bench paths, and tripwired. The defect builders justify every placement, and the packed-side constructions are correct against the codings. The tiling test checks both directions plus disjointness. The bench export's criterion IDs are the board's own cell names, with the denominator read back from the actual result.

The dominant issues are of two kinds. First, machinery that outlived its constraint: the segments currency reads zero by construction in every board binary (its only writer is `#[cfg(test)]`), yet the front page presents it as a live meter and `LADDER_TOP_SCALE` derives its ×4 from segment-onset amplifiers the board cannot observe; the rider list is a hand-maintained cell roster in a module whose doc says none exists, with a one-direction pin, and it is the time leg the committed cadence actually judges. Second, prose that has drifted from code because it is stated in several places: the root doc's "all four counters" survived the fifth currency, the fold-model roster is stated three ways (two, three, four rows), two sites prescribe a "dated owner rationale" the tree excised, and the `truncated_bytes` rationale argues from a decoder verdict the decoder stopped producing. Two findings were false at birth rather than drifted: `party_noncanonical_text`'s "at the text's end" (the id notation spells absent children as `0`, so the last `1` is never the last token on any board operand) and `MAX_SCALING_EXPONENT`'s claim to exclude a log factor (an n·log n kernel fits 1.07-1.10 on every committed ladder). Everything else is low or nit: a duplicated denominator rule, five copies of one preorder skeleton, an underived heap ceiling, a wrong-leg one-line proof, vocabulary and punctuation the crate uses everywhere.

## Findings

### board-frame-1: The segments currency reads zero by construction in every board binary; the front page, its ceiling, and the ladder-top derivation present it as a live meter
- Where: crates/before/src/meter/board.rs:47-49 (related: crates/before/src/meter/board/currency.rs:32-33, crates/before/src/meter/board/currency.rs:138-140, crates/before/src/meter/board/ceilings.rs:80-83, crates/before/src/meter/board/ceilings.rs:453-459, crates/before/src/meter/board/floors.rs:83-85, crates/before/src/meter/board/judge.rs:60-64, crates/before/src/recurse.rs:17-20, crates/before/src/recurse.rs:74-75, crates/before/src/recurse.rs:100-107, crates/before/Cargo.toml:33,44,115-117, justfile:889,893,904, crates/before/src/meter/tests.rs:399-403; outside the partition and for the envelopes reviewer: crates/before/tests/meter.rs:22-26,440-441)
- Class / severity / confidence: scaffolding / medium / high
- Provenance: verified (grep of `SEGMENTS_GROWN.fetch_add` over crates/before finds the single site recurse.rs:106 inside `grow`, which carries `#[cfg(test)]` at recurse.rs:100; `descend!` at recurse.rs:118-129 is `#[cfg(test)]`; Cargo.toml:33 `[dev-dependencies]` holds `stacker` at :44 and :115-117 declares the `amp_board` example with `required-features = ["limb-meter", "scan-meter"]`; justfile:889 and :904 run it as `cargo run --release ... --example amp_board`; judge.rs:366 compares `v > ceiling` strictly; `grep -c seg_ceiling_only()` gives 13 in floors.rs and 38 in ops.rs; `grep 'segments: Liveness::Floor'` finds no segments floor; `git log -1` on 05bd2b16d (2026-07-26, gated `grow` and `descend!` under cfg(test)), 1ddb5a483 (2026-07-31, stacker to dev-dependency), 9e36dd280 (2026-08-07, `LADDER_TOP_SCALE` with the segment-onset rationale)); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed (with justfile:893 added as a further site); history: deliberate-but-expired (05bd2b16d kept the column with "its own liveness now witnessed by a test-local guarded descent"; that witness runs only under cfg(test), where `grow` exists)
- Owner-gated: yes (removal of an instrument column and re-derivation of a ratified constant)

No code compiled into the board of record (the release `amp_board` example, or any integration or bench binary) can increment `SEGMENTS_GROWN`: its only writer is `recurse::grow`, which is `#[cfg(test)]`, and `stacker` is a dev-dependency. The column's ceiling therefore passes vacuously (Principle 2: a ceiling over a counter that cannot count), the column names no constructible failure the board can catch (Principle 3: circular justification, machinery outliving the constraint that justified it), and three sites state as present-tense fact what the binary cannot measure: board.rs:47-49 calls it "the honest stand-in for recursion-driven stack cost", recurse.rs:19-20 calls the zero "the measured fact the boards' segments column pins", and recurse.rs:74-75 says "the counter is always written". `LADDER_TOP_SCALE`'s ×4 (ceilings.rs:455-459) and the justfile's "segment-onset witness" (justfile:893) derive from segment-onset amplifiers that cannot fire there; the "~1 MiB of frames" onset figure is not derivable from recurse.rs's constants either (RED_ZONE at :51 is 256 KiB of remaining stack; STACK_GROWTH at :55 is the 1 MiB segment size). `MAX_GROWN_STACK_SEGMENTS = 1` admits one growth (judge.rs:366 is strict `>`) against a stated target of zero, with no rationale for the 1. The liveness witness in meter/tests.rs:399-403 proves the counter is alive under cfg(test); it says nothing about the board's binary.

Evidence:

        47	//! - **grown stack segments** ([`stack_segments`](crate::meter::stack_segments)),
        48	//!   the honest stand-in for recursion-driven stack cost, which bypasses any
        49	//!   heap meter;

    [recurse.rs:17-20]
        17	//! what lets it meet deep inputs safely. The segment counter below stays
        18	//! compiled for the meters: it is the deterministic stand-in for
        19	//! recursion-driven stack consumption, and its zero reading over the library
        20	//! kernels is the measured fact the boards' segments column pins.

    [recurse.rs:100-107]
       100	#[cfg(test)]
       101	#[inline]
       102	pub(crate) fn grow<R>(f: impl FnOnce() -> R) -> R {
       103	    if stacker::remaining_stack().is_some_and(|remaining| remaining >= RED_ZONE) {
       104	        f()
       105	    } else {
       106	        SEGMENTS_GROWN.fetch_add(1, Ordering::Relaxed);
       107	        stacker::grow(STACK_GROWTH, f)

    [ceilings.rs:80-83]
        80	/// Green requires at most this many grown stack segments, as an absolute count:
        81	/// the target is walks that never grow the stack, so the ceiling is flat, not
        82	/// per-byte.
        83	pub const MAX_GROWN_STACK_SEGMENTS: u64 = 1;

    [ceilings.rs:455-459]
       455	/// The base-scale sizes under-detect segment amplifiers: stacker grows a
       456	/// segment only past ~1 MiB of frames, so a recursion-frame amplifier whose
       457	/// onset sits above the base depths reads a false green there. ×4 is the
       458	/// witnessed calibration floor — the smallest sampling scale at which every
       459	/// segment-onset amplifier the suite has caught reads red — so the ladder

    [Cargo.toml:115-117; justfile:889]
       115	[[example]]
       116	name = "amp_board"
       117	required-features = ["limb-meter", "scan-meter"]
       889	    cargo run --release -p before --example amp_board --features limb-meter,scan-meter -- {{ args }}

Resolution: Owner's call between two consistent states. (a) Dissolve: remove `Currency::Segments` and the `segments` field from `ByCurrency` (the totality mechanism ripples the removal through every `Floors` literal, the judgment, and the render), delete `MAX_GROWN_STACK_SEGMENTS`, `seg_ceiling_only`, and `SEG_FLOOR_TRIP`, re-derive `LADDER_TOP_SCALE`'s doc from what the ladder still buys (the four-point trend and the heap flat-allowance regime, or state it as owner-ratified with the calibration story in the pin commit), re-word recurse.rs:17-20 and :74-75 to the true statement (the counter is written only by the test-surface guard; the depth guarantee is AGENTS.md's iterative rule plus `deep_tree_stack_safety`), and re-word justfile:893. Keep `SEGMENTS_GROWN` and `stack_segment_meter_counts_deterministically_and_resets` only if the oracle bridge's guard liveness needs the witness. (b) Keep as a structural pin: set the ceiling to 0, document at currency.rs:32 and ceilings.rs:80 that the reading is identically zero on every non-test binary because the library's only growth arm is test-only, drop the segment-onset sentence from `LADDER_TOP_SCALE`, and explain what the column would ever catch (nothing outside cfg(test), so this is the weaker state). History favors (a): 1ddb5a483's reason for the dev-dependency move is exactly that no library code recurses. Acceptance: (a) `grep -rn segments crates/before/src/meter/board` returns no currency field, `just gate` is clean, and no prose in board.rs, ceilings.rs, floors.rs, recurse.rs, or the justfile names segment onset or a measured segments column; (b) `MAX_GROWN_STACK_SEGMENTS` is 0 and the constant's doc names the cfg boundary.
Construction: Mechanically, `grep -rn 'SEGMENTS_GROWN.fetch_add' crates/before/src` yields one site under `#[cfg(test)]`, so `cargo build --release --example amp_board --features limb-meter,scan-meter` contains no writer of the atomic. Behaviourally, add an unguarded deep plain recursion (no `descend!`, which is unavailable to non-test code) to any board row's body on a deep family and run `just amp-board`: the segments column stays 0 and the cell stays green on that column, or the child overflows its stack; no input exists on which the column reads nonzero.

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

### board-frame-3: The "four cells no deterministic leg watches" disclosure is a prose-only roster; nothing pins the all-NA cell set
- Where: crates/before/src/meter/board.rs:99-100 (related: crates/before/src/meter/board/floors.rs:99-112, crates/before/src/meter/board/tests.rs, crates/before/tests/amp_board_smoke.rs)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep of `NotApplicable|neither leg|all.NA|unwatched` over board/tests.rs and tests/amp_board_smoke.rs finds only the bypass test's doc at tests.rs:403, which builds synthetic samples rather than enumerating cells); executed: no
- Seen by: scaffolding (and the count half of structure-prose's and instrument-correctness's board.rs items); refutation: confirmed; history: no-rationale-found (the prose disclosure was deliberate in 319afc262; the absence of a pin was never considered; the count "four" is from b3f09baa0, a commit titled "Partial WIP for docs pass")
- Owner-gated: no

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

### board-frame-4: Two sites prescribe a "dated owner rationale" at the declaring constants; no constant carries a date, and the doctrine forbids one there
- Where: crates/before/src/meter/board.rs:170-172 (related: crates/before/src/meter/board/ceilings.rs:233-235, crates/before/src/meter/board/ceilings.rs:56-62, crates/before/src/meter/board.rs:240-241)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n dated` over board.rs and board/*.rs hits :171, :240, ceilings.rs:234; `grep -E '20[0-9]{2}-[01][0-9]'` over the same files returns nothing; `git log -1 d2a9d04e` is "dated-notes excision: history lives in git, prose states what is" (2026-07-31)); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: deliberate-but-expired (669cf3103 introduced both the prescription and real dated rationales; d2a9d04e excised the dates and left the two sentences)
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

The bench-mirror paragraph at board.rs:253-256 is verbatim export.rs:3-6; the tiling story appears in board.rs, coverage.rs, and coverage/tests.rs; the declared-models mechanism is derived in board.rs, the ceilings.rs module doc, and the ceilings.rs comment block; denomination is derived in board.rs and cell.rs. The design (a summary per submodule pointing at the owner) is recorded and sound, but the criterion that copies survive verbatim is what produced two independent drifts ("all four counters"; "the two n-ary fold rows" versus three versus four). This is an undocumented deliberate choice whose cost has materialized (Principle 5; documentation altitude: each mechanism derived in exactly one place).

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

### board-frame-6: Fifteen re-exported ceiling constants and the `Currency` type have no reference outside the module; ten more are public only so the module doc can link them
- Where: crates/before/src/meter/board.rs:279-292 (related: crates/before/src/meter/board/ceilings.rs:69-229, crates/before/src/meter/board/currency.rs:27-141, justfile:258, justfile:260-264)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git grep -n -w <NAME> -- '*.rs' '*.py' '*.json' justfile ':!crates/before/src/meter/board' ':!crates/before/src/meter/board.rs' ':!.agent-notes'` per name: `DEFAULT_SCALE` and `LADDER_TOP_SCALE` have code consumers (examples/amp_board.rs:18,194; benches/common/sidecar.rs:9,109,113); the eight `MAX_*`/`HEAP_FLAT_ALLOWANCE_BYTES` constants and `ByCurrency`, `Floors`, `Liveness` appear only as intra-doc links in board.rs itself; the remaining fifteen constants and `Currency` appear nowhere; the `Floors`/`Liveness` hits outside are English words in test docs; justfile:258 runs `cargo doc` under `RUSTDOCFLAGS="-D warnings"` and :260-264 names the `private_intra_doc_links` lint); executed: no
- Seen by: structure-prose; refutation: reframed (seventeen corrected to fifteen); history: no-rationale-found (public since 7d81a248a; 89cf4c8d5 re-exported them as a pure-movement invariant, not a decision)
- Owner-gated: yes (the `meter`-feature board surface is public; narrowing is an API change)

Of the 25 constants in the `pub use ceilings` block, two have code consumers outside the module, eight are public only because the module doc links them under `-D warnings`, and fifteen (`ASCEND_CLIFF_*`, `CAPACITY_MODEL_*`, `FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL`, `INDEX_PROBE_SCAN_BITS`, `MACHINE_WORD_MAGNITUDE_BITS`, `MIN_EXPONENT_DENOM_GROWTH`, `MIRROR_WIDE_*`, `SCAN_FLOOR_BITS_PER_INPUT_BYTE`, `SCAN_TOUCH_FLOOR_BITS`, `TEXT_BYTES_PER_RADIX_UNIT`, `TEXT_PIPELINE_LIMB_OPS_PER_VALUE`, `TICKS_BOARD_COUNT`) are referenced nowhere outside the module. `Currency` likewise. A `pub` item earns its visibility by naming a consumer (Principle 3); here the doc justifies the API rather than the reverse, and every `pub const` is surface a future reader must assume someone depends on.

Evidence:

       279	pub use ceilings::{
       280	    ASCEND_CLIFF_MIN_TICKS_HEAP_BYTES_PER_INPUT_BYTE, ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE,
       281	    CAPACITY_MODEL_CEILING, CAPACITY_MODEL_FLOOR, DEFAULT_SCALE,
       282	    FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL, HEAP_FLAT_ALLOWANCE_BYTES, INDEX_PROBE_SCAN_BITS,
       283	    LADDER_TOP_SCALE, MACHINE_WORD_MAGNITUDE_BITS, MAX_GROWN_STACK_SEGMENTS,
       292	pub use currency::{ByCurrency, Currency, Floors, Liveness};

Resolution: Narrow the fifteen unused constants to `pub(super)` (sibling submodules already import them via `super::ceilings::`) and drop them from the `pub use`. For the doc-linked items, either decide that the judgment constants and currency types are part of the meter API (state that at the `pub use`) or refer to them in prose by backticked path without a link and narrow them too. `ShardSpawner`, `bench_cells`, `BenchCell`, `BenchMode`, `HeapMeter`, `Summary`, and the `run*` functions stay public: they are the example's and bench's entry points. Acceptance: every name in the `pub use ceilings` block has at least one non-doc use outside the module or is absent from the block; `just docs` stays clean.

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

### board-frame-12: Two measured readings are quoted in ceilings.rs prose against the file's own rule
- Where: crates/before/src/meter/board/ceilings.rs:223-224 (related: crates/before/src/meter/board/ceilings.rs:343-344, crates/before/src/meter/board/ceilings.rs:56-62)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read both sites against the header at :56-62; `git show 500d4d094 -- ceilings.rs` touches neither line); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no-rationale-found for these two (the limb calibration's "tens per packed byte ... over a hundred" at :88-90 and the ~8 bits per byte at :133 are order-of-magnitude calibrations 500d4d094 deliberately kept, so they are excluded here)
- Owner-gated: no

The header rules that readings live in pin commits, never in prose, because a quoted reading keeps asserting itself as present-tense fact while headroom absorbs drift; the benign rank pair's "6 -> 7 bytes" and the fold marginal's "~1.14–1.17 at the committed populations" are measurements the sweep that wrote the rule did not triage.

Evidence:

       223	/// this says the operand does not scale with the knob at all (the benign rank
       224	/// pair moves 6 -> 7 bytes) and the division manufactures exponents out of

    [ceilings.rs:56-59]
        56	// Several ceilings below argue their calibration from the worst honest reader
        57	// at the release profile of record. The readings themselves live in the pin
        58	// commits (`git log -S` the constant), never in this prose: a quoted reading

Resolution: Keep the mechanism (the pair's denominator barely moves; the log factor's marginal) and excise the numbers or move them to the pin commits. Acceptance: no measured reading appears in ceilings.rs prose outside the deliberately kept order-of-magnitude calibrations.

### board-frame-13: `both_present_nodes` is an operand-content walk living in the constants module
- Where: crates/before/src/meter/board/ceilings.rs:286-298 (related: crates/before/src/meter/board/ceilings.rs:52, crates/before/src/meter/board/operand.rs:1-3, crates/before/src/meter/board/operand.rs:259-285, crates/before/src/meter/board/ops.rs:1373-1377)
- Class / severity / confidence: modularity / nit / high
- Provenance: verified (ceilings.rs:52 `use crate::Party;` serves only this function; consumers are ops.rs:1373 and :1377; operand.rs:1-3 claims the responsibility); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: no-rationale-found (placed beside `INDEX_PROBE_SCAN_BITS` in 29d3c8f27; the module split preserved adjacency as pure movement)
- Owner-gated: no

ceilings.rs's header is "The judgment constants"; operand.rs's is "Deterministic operand-content walks: the quantities the liveness floors and denominators are stated in". `both_present_nodes(p: &Party) -> u64` is the latter and is the file's only walk over a domain value (`capacity_chain_peak` and `fold_exponent_ceiling` are closed-form model formulas tied to their constants). Modules have one clear responsibility.

Evidence:

       286	/// A packed id operand's both-present node count: the size of the `IdIndex`
       287	/// table a fold builds over it, and the per-input factor of the declared search
       288	/// allowance. One 2-bit presence tag per node.
       289	pub(super) fn both_present_nodes(p: &Party) -> u64 {

Resolution: Move it to operand.rs beside `radix_units_party` (the same 2-bit-tag walk), keep the derivation prose on `INDEX_PROBE_SCAN_BITS` linking the function, and drop `use crate::Party` from ceilings.rs. Acceptance: ceilings.rs imports no domain types; ops.rs imports the function from `super::operand`.

### board-frame-14: "The tripwire pair below" points at tests that live in another file
- Where: crates/before/src/meter/board/cell.rs:45-48 (related: crates/before/src/meter/board/tests.rs:556, crates/before/src/meter/board/tests.rs:614)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (cell.rs has no `#[test]`; the pair is `flat_denominator_packed_fit_manufactures_an_exponent` at board/tests.rs:556 and `quadratic_in_teeth_work_reads_red_against_the_content_denominator` at :614); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (the sentence crossed a file boundary from birth in 7999b8ec2; a05918df7's "no positional above/below crosses a file boundary" audit missed it)
- Owner-gated: no

Nothing below in cell.rs is a test; the spatial reference sends the reader to the wrong file (Principle 5: no ghost references).

Evidence:

        45	//!   `n_io`, whose output side already scales. The tripwire pair below
        46	//!   pins both directions: the packed fit reads a manufactured exponent
        47	//!   on measured flat per-tooth work, and a genuinely quadratic-in-teeth
        48	//!   probe still reads red against the content denominator.

Resolution: Name the two tests in backticks. Acceptance: cell.rs contains no "below"/"above" that refers outside the file; the two test names appear verbatim so a rename breaks a grep.

### board-frame-15: The version output reader undercounts a flush stream by its marker byte
- Where: crates/before/src/meter/board/cell.rs:173-174 (related: crates/before/src/meter/board/operand.rs:292-297, crates/before/src/version.rs:1151-1152, crates/before/src/version.rs:1174-1180, crates/before/src/codec/bits.rs:160-166, crates/before/src/codec/buf.rs:383-386)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (bits.rs:160-166 `len()` returns the live bit count recovered from the marker; version.rs:1151-1152 `encoded_bits` returns it; `as_bytes` at :1174-1180 returns the raw slice including the marker; buf.rs:383-386 appends the marker after the live bits, so 8k live bits store as k+1 bytes while `encoded_bits().div_ceil(8)` gives k); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: deliberate-but-expired (exact under the unmarked coding of 6814d77b9; d800957e8 changed the size law to `encode().len() == (encoded_bits() + 1).div_ceil(8)` and re-derived the board's other adapters but not operand.rs)
- Owner-gated: no

`IoSpec::output_bytes` is documented as reading the actual output's byte size, and board.rs:158-159 insists the output side is read back from the result, never assumed; the version reader behind it derives the size from a bit count with a rounding that disagrees with the stored bytes one time in eight, so `n_io` is one byte short on flush outputs while the input side uses the wire `bytes.len()`. Negligible for any verdict, but the reader's contract and its arithmetic disagree, and `as_bytes().len()` is exact and O(1) (Principle 1: correct for all inputs).

Evidence:

       173	    /// Read the actual output's byte size from the boxed result.
       174	    pub(super) output_bytes: fn(&dyn Any) -> usize,

    [operand.rs:293-296]
       293	pub(super) fn version_output_bytes(v: &Version) -> usize {
       294	    // The measured value's stored buffer is allocated on this host, so its
       295	    // byte count fits `usize`.
       296	    usize::try_from(v.encoded_bits().div_ceil(8)).expect("an allocated buffer's byte count")

    [bits.rs:165]
       165	                self.bytes.len() as u64 * 8 - 1 - u64::from(last.trailing_zeros())

Resolution: `v.as_bytes().len()` in `version_output_bytes`. Acceptance: for a version whose `encode()` ends in `0x80`, `version_output_bytes(&v) == v.encode().len()`; a unit test beside the reader pins it.
Construction: Any version with 8k live bits (`encoded_bits() == 8k`): `div_ceil(8) == k` while `encode().len() == k + 1`; `Version::new().encoded_bits()` is 2, so tick a seeded version until `encoded_bits() % 8 == 0` and compare.

### board-frame-16: `Cell`'s three constructors repeat the same struct literal, and its two mutually exclusive heap models are flat fields whose precedence lives in the judge
- Where: crates/before/src/meter/board/cell.rs:204-308 (related: crates/before/src/meter/board/cell.rs:126-144, crates/before/src/meter/board/judge.rs:311-345, crates/before/src/meter/board/ops.rs:392,428,801,903,1587,1711)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (the literal appears at cell.rs:209-219, :269-282, :294-307 differing only in `denom`; `with_capacity_model` is set at ops.rs:903 and :1711, `with_declared_heap` at :392, :428, :801, :1587, never on the same row; judge.rs:315-338 handles the capacity band first and `continue`s past the declared-heap override at :341-345); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (`io`/`text` and each model field accreted one commit at a time, each edit repeating the literal three times; the no-`Default` totality argument is `ByCurrency`'s in currency.rs, not `Cell`'s)
- Owner-gated: no

Every new declared-model field is three edits that must agree, and the exclusivity of `capacity_model: bool` and `declared_heap: Option<f64>` is expressed only by the judge's ordered `if` chain. Types-first and legibility: one private constructor removes the cascade, and an enum makes the exclusivity a fact the compiler holds.

Evidence:

       209	        Cell {
       210	            input_bytes,
       211	            denom: Denom::Input,
       212	            floors,
       213	            fold_arity: None,
       214	            fold_search_bits: 0,
       215	            capacity_model: false,
       216	            declared_heap: None,
       217	            declared_limb: None,
       218	            body: Box::new(move || Box::new(body())),
       219	        }

Resolution: One private `fn with_denom<R: Any>(input_bytes, denom: Denom, floors, body) -> Cell` holding the literal, with `new`/`io`/`text` as one-line wrappers; consider `heap_model: HeapModel { Global, CapacityChain, Declared(f64) }` (and `Sample` following) so judge.rs matches on it. Acceptance: one `Cell { .. }` literal in cell.rs; the board's unit and smoke tests unchanged.

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

### board-frame-18: Vocabulary tells across the frame's prose: "mint", an unanchored "honest" in four senses, register transplants, "genre", "today", a past-tense justification, a duplicated sentence
- Where: crates/before/src/meter/board/coverage.rs:435 (related: crates/before/src/meter/board/coverage.rs:657-658, crates/before/src/meter/board/coverage.rs:17-18 and 40-41, crates/before/src/meter/board.rs:48, 66-67, 145, 238, crates/before/src/meter/board/ceilings.rs:7, 242, 326, 376, 419, 437, crates/before/src/meter/board/cell.rs:42, 57, 92, 143, crates/before/src/meter/board/currency.rs:138-140)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -w mint|mints|minted` over the partition gives coverage.rs:435 and :658 (family.rs and ops.rs carry seven more outside it); `grep -c -i honest` gives board.rs 6, ceilings.rs 25, cell.rs 16, currency.rs 2, export.rs 3, total 52; `grep -w genre` over the board module gives 18; `grep -w earns|buys|ratchet` gives board.rs:238, cell.rs:143, ceilings.rs:7, 242, 376; `grep -w today` gives currency.rs:139 (floors.rs carries six more); crate-wide `honest` appears 146 times in 35 files); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: no-rationale-found (the writing-style rules postdate this prose; the idiom is crate-wide, so the right unit of fix is a crate-wide prose pass under the Documentation Change Policy)
- Owner-gated: yes (a crate-wide prose pass; a partition-local fix would make this module the odd one out)

"mint" for constructing a value appears twice in reasons that render on the board legend. "honest" carries four meanings (the largest reading from a conforming kernel; the bytes actually read and written; an NA with a derivation; the anti-padding assertion), only the last anchored to `assert_honest_text`. "earns a column", "buys", "honesty ratchet" are register transplants; "genre" is used 18 times for a class of rows with no identifier to anchor it. currency.rs:139 dates a policy with "today". board.rs:66-67 justifies the touch column by a past finding. coverage.rs states that `Debug` delegates to `Display` at both :17-18 and :40-41.

Evidence:

       435	        "O(1) hole mint over the atom's bound: no comparison, no walk",

    [board.rs:66-67]
        66	//!   this is the one deterministic column that sees the genre (the tick
        67	//!   walk's width-circulation finding lived entirely in this currency).

    [currency.rs:138-140]
       138	/// doc's totality argument). Segments' declaration is the ceiling-only policy
       139	/// NA on every cell today: the target is walks that never grow the stack, so
       140	/// its honest floor is zero, and a zero floor asserts nothing.

Resolution: Schedule a crate-wide prose pass. In it: "construct"/"build" for mint; keep "output honesty" anchored to `assert_honest_text` and replace the other senses with their mechanism ("the largest production reading" or "the calibrating reader"; "the bytes the operation reads and writes"; "not-applicable by derivation"); "is a column when" for "earns", "trades for" for "buys", "the under-side floor that forces re-declaration" for "ratchet"; anchor "genre" to a row-kind identifier or write "class"; state board.rs:66-67 positively without the parenthetical; drop "today" (the policy is structural: no floor constructor produces a segments floor); delete one of the two `Debug` sentences. Acceptance: no "mint" under board/; "honest" appears only where the output-honesty assertion is named; no "today" at a declaration site; one `Debug` sentence in coverage.rs.

### board-frame-19: The tiling test derives the board axis by building every family at a bare 0.02, twice, and guards NA reasons with a bare `20`
- Where: crates/before/src/meter/board/coverage/tests.rs:9-16 (related: crates/before/src/meter/board/coverage/tests.rs:41, 68-71, 97, crates/before/src/meter/board/ops.rs:193, crates/before/src/meter/board/tests.rs:1099,1113-1114, crates/before/tests/amp_board_smoke.rs:28-30, crates/before/src/meter/board/export.rs:138-140)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (ops.rs:193 `pub(super) fn ops()` is visible to `board::coverage::tests`, a descendant of `board`, and board/tests.rs:1114 already calls it; `board_ops()` runs at coverage/tests.rs:41 and :97 with `ops` still in scope; `bench_cells` builds every family per call (export.rs:138-140); amp_board_smoke.rs:30 names the same value `SMOKE_SCALE`); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (`bench_cells(0.02, Full)` was the only door when the test lived outside the board module in 535d69b76; 0a5bdaebd moved it inside, where `ops()` is reachable)
- Owner-gated: no

`board_ops()` takes the operation axis from the cells applicable at scale 0.02 rather than from `ops()`, so a row inapplicable on every family at that scale would vanish from the orphan leg, and it rebuilds every family twice. The `reason.len() >= 20` guard admits any twenty characters while its message claims to detect a missing mechanism (named constants over magic numbers; Principle 6: name the worst artifact that passes).

Evidence:

        11	fn board_ops() -> BTreeSet<String> {
        12	    bench_cells(0.02, BenchMode::Full)
        13	        .into_iter()
        14	        .map(|cell| cell.op.to_owned())
        15	        .collect()
        16	}

    [coverage/tests.rs:68-71]
        68	        assert!(
        69	            reason.len() >= 20,
        70	            "{op}: the not-applicable reason is too thin to be a mechanism: {reason:?}"
        71	        );

Resolution: `fn board_ops() -> BTreeSet<&'static str> { ops().into_iter().map(|op| op.name).collect() }`, called once; where a tiny build scale is genuinely needed (the riders test, if it survives board-frame-25), share a named constant with the smoke test's `SMOKE_SCALE`; either drop the length check or name it (`MIN_REASON_CHARS`) and word the message as what it checks. Acceptance: the tiling test builds no `FamilyData` and names no scale; no bare `0.02` under `meter/board/`; no bare numeric literal in the assertions.

### board-frame-20: Em-dashes in two `//` comments and one assert message
- Where: crates/before/src/meter/board/coverage/tests.rs:78-79 (related: crates/before/src/meter/board/ceilings.rs:61, crates/before/src/meter/board/ceilings.rs:235)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -n -E '^\s*//[^/!].*—'` over the partition gives ceilings.rs:61 and :235; `grep -n '—' coverage/tests.rs` gives :78; the em-dashes inside `BOARD_PRICED` strings are `surface.rs` row names the tiling test requires byte-equal; crate-wide, 374 of 4250 `//` comment lines carry an em-dash); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (the preference is Part II doctrine, not an AGENTS.md rule, and the crate does not follow it anywhere)
- Owner-gated: yes (crate-wide convention; a local fix makes this module inconsistent)

The doctrine prefers colons, semicolons, or spaced double hyphens in `//` comments and log or assert strings (terminal compatibility of panic output). Three partition sites, and the whole crate, use true em-dashes; the unit of change is a crate-wide decision.

Evidence:

        78	            "{op}: priced by board rows AND excused in BOARD_NOT_APPLICABLE — \
        79	             the tiling sides must stay disjoint; remove one"

Resolution: Decide crate-wide; if adopted, `; ` in the assert message and `: ` or ` -- ` in the two comments. Acceptance: the two greps above return nothing (crate-wide, `grep -rn -E '^\s*//[^/!].*—' crates/before/src` returns nothing).

### board-frame-21: `truncated_bytes` argues its two-byte cut from a decoder verdict the decoder no longer produces, and the one-byte cut is both correct and more deferred
- Where: crates/before/src/meter/board/defect.rs:30-51 (related: crates/before/src/codec/bits.rs:467-470, crates/before/src/codec/bits.rs:489-491, crates/before/src/error.rs:72-74, crates/before/src/version.rs:1116-1118, crates/before/src/meter/board/ops.rs:1849-1854, crates/before/src/codec/bits.rs:449)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (bits.rs:489-491 `match remainder { 0 => Err(Decode::Truncated), ...}` and :467-470 document the empty remainder as `Truncated`; error.rs:72-74 says the same; version.rs:1116-1118 validates the whole buffer then calls `require_marker_padding`, so a one-byte cut on a flush stream parses the complete tree and fails at the padding judge with `Truncated`; ops.rs:1851-1853 asserts `Decode::Truncated`; `git show 61d00223a --stat` touches bits.rs and error.rs only); executed: no
- Seen by: instrument-correctness (scaffolding's marker-literal item folds in); refutation: confirmed; history: deliberate-but-expired (d800957e8 wrote the rationale when the empty remainder was `TrailingBits`; 61d00223a reclassified it as `Truncated` at every decode door and did not touch defect.rs)
- Owner-gated: no

The doc justifies cutting two bytes on a flush stream because dropping only the marker byte "would leave a complete tree missing its padding — a `TrailingBits` defect, not a cut". The padding judge returns `Decode::Truncated` for an empty remainder, so the one-byte cut is already the defect the row asserts and is more deferred (the whole tree parses before the judge fires), while the two-byte cut removes eight live bits so the parse fails early. The rejection rows exist to price the defect maximally deferred (board.rs:194-196); a rationale that contradicts the decoder's documented taxonomy is a ghost. The one-byte cut also removes the bare `0b1000_0000` marker literal (spelled `[0x80]` at bits.rs:449, with no named constant).

Evidence:

        35	/// parsing to the cut. The final byte is pure padding exactly when it is
        36	/// the whole-byte marker `1000_0000` (the live bits end flush against
        37	/// the byte boundary); dropping only that byte would leave a *complete*
        38	/// tree missing its padding — a `TrailingBits` defect, not a cut — so
        39	/// the cut then takes the last live byte with it.
        40	pub(super) fn truncated_bytes(bytes: &[u8]) -> Vec<u8> {
        41	    let cut = if bytes.last() == Some(&0b1000_0000) {
        42	        2
        43	    } else {
        44	        1
        45	    };

    [bits.rs:489-491]
       489	    let remainder = total - pos;
       490	    match remainder {
       491	        0 => Err(Decode::Truncated),

    [bits.rs:467-469]
       467	/// - An empty remainder is [`Decode::Truncated`]: the input ends where the
       468	///   padding should begin — a flush stream cut before its whole marker byte —
       469	///   so required data is missing, exactly what a byte-starved reader reports

Resolution: Always cut one byte and re-state the doc: on a non-flush stream the cut removes the last live bits and the marker, and the tree walk runs out of input; on a flush stream it removes the marker byte alone, the whole tree parses, and the padding judge reports `Truncated` at the end, the most deferred placement byte granularity allows. Relax the guard to `bytes.len() > 1`. Acceptance: a unit test beside the builders: for a version whose `encode()` ends in `0x80`, `truncated_bytes(&bytes).len() == bytes.len() - 1` and `Version::decode(&truncated_bytes(&bytes)[..])` is `Err(Decode::Truncated)`; the truncation rows' `matches!(err, Decode::Truncated)` assertions stay green; scan readings on flush-stream families rise, never fall.
Construction: Take any version whose live bits are a multiple of 8 (its `encode()` ends in `0x80`); drop only the final byte; `Version::decode` walks the complete tree, reaches `pos == total`, and `require_marker_padding` returns `Decode::Truncated` (remainder 0), not `TrailingBits`.

### board-frame-22: Five hand-rolled preorder decoders of the version stream under `meter/board/`, two re-implementing the zigzag rule; three are dissolvable onto the public shape walk
- Where: crates/before/src/meter/board/defect.rs:67-86 (related: crates/before/src/meter/board/operand.rs:23-45, 57-78, 89-125, 156-191, crates/before/src/version/skyline/signed.rs:179-185, crates/before/src/shape.rs:88-98, crates/before/src/version.rs:773)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (the `while pending > 0 { ... // skyline flag: 0 internal, 1 leaf ... expect("a stored stream is canonical") }` skeleton appears at defect.rs:72-84 and operand.rs:29-43, :62-76, :95-103, :163-172; operand.rs:108-118 and :177-188 re-implement `unzigzag_base` (signed.rs:179-185, `pub(super)` there); `Version::shape` (version.rs:773) yields `Plateau { rise: Option<Rise>, depth }` with `None` for a zero delta and the first rise absolute (shape.rs:40-42, 88-98); `git log -1 46eb64f9` is 2026-08-19, after the decoders were written); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed (reframed from three copies to five); history: deliberate-but-expired (the decoders predate the public shape walk; 46eb64f9 integrated the board only at the coverage tables and never weighed the instrument's own decoders)
- Owner-gated: no

The per-leaf action each function is about sits under twelve lines of identical scaffolding, and two copies track production's zigzag convention by hand. `stored_nonzero_deltas`, `value_content_bytes`, and `stored_bases`' height pass can ride `Version::shape()` with no wire decoding (`rise.is_some()` is exactly a nonzero delta; a running sum of rises is the absolute height); the two that need stream positions or code widths (`last_leaf_flag_pos`, `mandatory_limbs_stream`) can share one private leaf iterator. Trade-off to state: floors derived through `Version::shape()` rest on a rostered public operation with differential coverage instead of a private decoder, which is the more principled footing for a floor stated in terms of what the operation must do.

Evidence:

        72	    while pending > 0 {
        73	        pending -= 1;
        74	        let flag = pos;
        75	        let internal = !bits.bit(pos); // skyline flag: 0 internal, 1 leaf
        76	        pos += 1;
        77	        if internal {
        78	            pending += 2;
        79	            continue;
        80	        }
        81	        let (_, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");

    [operand.rs:108-113, the zigzag re-implementation]
       108	                let odd = code.bit(0);
       109	                let magnitude = if odd {
       110	                    (code + 1u32) >> 1u32
       111	                } else {
       112	                    code >> 1u32
       113	                };

Resolution: In operand.rs express `stored_nonzero_deltas` as `v.shape().skip(1).filter(|p| p.rise.is_some()).count()` and the two absolute-height reconstructions as a running sum over `rise`; add one private `stored_leaves(v) -> impl Iterator<Item = (flag_pos, code, next_pos)>` for the code-width floor and defect.rs's last-leaf position; delete both zigzag re-implementations. The operand.rs functions are outside this partition's file list; the pattern is reported once here. Acceptance: exactly one `// skyline flag: 0 internal, 1 leaf` loop remains under `meter/board/`; `radix_units_match_hand_counts` and `mandatory_limbs_match_hand_counts` pass unchanged; no `>> 1u32` zigzag arithmetic remains in operand.rs.

### board-frame-23: `party_noncanonical_text` never places its defect at the text's end on any board operand
- Where: crates/before/src/meter/board/defect.rs:168-176 (related: crates/before/src/meter/board/ops.rs:2078-2097, crates/before/src/meter/board/family.rs:1109-1126, crates/before/src/meter/board/family.rs:1036-1039, crates/before/src/codec/display.rs:41-52, crates/before/src/codec/text.rs:141, 165-169, crates/before/src/meter.rs:561-574, crates/before/src/meter/board/floors.rs:426-439, crates/before/src/meter/board/defect.rs:10-12)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (display.rs:41-42 renders a terminal as `1` and :52 renders an absent child as `0`; text.rs:141 treats `0` as absence and :165-169 rejects `(0, 0)` and `(1, 1)` identically at the node's `)`; family.rs:1114-1115 pushes `left, !left` so the mount adapter's `a` is `(shape, 0)`; `party_pair` (:1036-1038) returns `a` first and the row feeds it (ops.rs:2082-2083); `id_spine` (meter.rs:564-572) is a left spine rendering `((((1, 0), 0), 0), 0)`; text-rejection floors are all NA (floors.rs:431-439)); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed, severity medium to low (the row's floors are NA, the id parser does no metered work, and a linear prefix leaves the time exponent unchanged, so the shortfall is a constant fraction of one row's readings; the false doc claim is the concrete defect); history: no-rationale-found (false at birth: 3c43a8154 wrote "at the text's end" against an operand 3eadcb107 had already made `(shape, 0)`)
- Owner-gated: no

The placer re-spells the last `1` token, but the id notation spells absent children as `0`, so on every board operand at least `, 0)` follows the last `1`, and on the left-leaning spine (the id side of `id-pair` and `staircase`) the pair lands about `d+2` bytes into a roughly `5d` byte text and the parser rejects with most of the text unparsed. This contradicts the doc's "at the text's end" and the module's criterion that every defect is maximally deferred (defect.rs:10-12; Principle 6: an early-exit measurement is the cheapest artifact that passes). The packed-side sibling is correct because absent children occupy no bits; the text side needs its own construction.

Evidence:

       168	/// `text` with its last `1` token re-spelled `(1, 1)`: the collapsible pair,
       169	/// judged non-normal at the node's close, at the text's end
       170	/// ([`Parse::NotCanonical`](crate::error::Parse)).
       171	pub(super) fn party_noncanonical_text(text: &str) -> String {
       172	    let at = text
       173	        .rfind('1')
       174	        .expect("a party's text spells at least one owned leaf");
       175	    format!("{}(1, 1){}", &text[..at], &text[at + 1..])
       176	}

    [display.rs:52; text.rs:166-169]
        52	            f.write_str("0")?; // an absent child renders `0`
       166	                        (IdKind::Empty, IdKind::Empty) => return Err(Parse::NotCanonical), // (0, 0)
       167	                        (IdKind::Terminal, IdKind::Terminal) => {
       168	                            return Err(Parse::NotCanonical); // (1, 1)
       169	                        }

    [family.rs:1114-1115; ops.rs:2082-2083]
      1114	        bits.push(left);
      1115	        bits.push(!left);
      2082	                let (a, _, _) = f.party_pair()?;
      2083	                let fed = party_noncanonical_text(&a.to_string());

Resolution: Target the last leaf token whichever it is, `rfind(|c: char| c == '0' || c == '1')`, and re-spell `t` as `(t, t)`; the parser rejects `(0, 0)` and `(1, 1)` identically at the `)`, so the row's `Parse::NotCanonical` assertion holds and only closing parens follow the defect. Re-word the doc ("its last leaf token `t` re-spelled `(t, t)`, the non-normal pair judged at the node's close, the text's last token"). Acceptance: a unit test beside the builder: for the mounted `id-pair` operand, the produced text's `(t, t)` closes at the last non-paren byte and `parse::<Party>()` returns `Parse::NotCanonical`; the row's heap readings on the left-mounted families do not fall.
Construction: `Party` text `(((((1, 0), 0), 0), 0), 0)` (the `id_spine(4, false)` shape mounted left, 26 bytes) becomes `((((((1, 1), 0), 0), 0), 0), 0)`; `parse_id_tree` returns `NotCanonical` after consuming the 12 bytes `((((((1, 1)`, leaving 19 unparsed. A test asserting `d.len() - d.find("(1, 1)").unwrap() <= 8` fails on the current placer.

### board-frame-24: `BenchCell::denominator_bytes` re-implements `measure`'s denominator rule by hand and runs the body when the denominator does not need it
- Where: crates/before/src/meter/board/export.rs:69-85 (related: crates/before/src/meter/board/measure.rs:97-122, crates/before/src/meter/board/export.rs:47-51, 57-62, crates/before/src/meter/board/cell.rs:161-199)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (export.rs:73-84 and measure.rs:97-122 both `match cell.denom`, both use `content.unwrap_or(cell.input_bytes)` for `Input`, both read `(spec.output_bytes)(result)` and call `assert_honest_text` under `output_is_text`; export.rs:72 runs the body before the match and the `Input` arm at :74 never reads `result`; `body()` (:47-51) and `denominator_bytes` (:70-71) repeat the same `.expect(...)`); executed: no
- Seen by: scaffolding, structure-prose (two items); refutation: confirmed, severity medium to low (the exhaustive `match Denom` forces both sites to handle a new variant; the residual drift is identical handling); history: no-rationale-found (the copy is from 54b68b4fd; 7999b8ec2 already paid the cascade once, updating both sites)
- Owner-gated: no

export.rs:57-62 promises the bench denominator is "exactly as the board's measurement does", but the sameness is held by two parallel code paths, and a change to either arm desynchronizes the judge's time exponents from the board's counter exponents (Principle 6: the denominator is the one thing binding the two instruments). The body also runs unconditionally although the doc gives output read-back as its reason; the `Input` arm ignores it.

Evidence:

        72	        let result = (cell.body)();
        73	        match cell.denom {
        74	            Denom::Input => self.data.content_bytes.unwrap_or(cell.input_bytes),
        75	            Denom::Io(spec) => {
        76	                let output_bytes = (spec.output_bytes)(result.as_ref());
        77	                if let Some(text) = spec.text {
        78	                    if text.output_is_text {
        79	                        assert_honest_text(self.op, output_bytes, text.radix_units);
        80	                    }
        81	                }
        82	                cell.input_bytes + output_bytes
        83	            }
        84	        }

Resolution: One method on `Cell` (or `Denom`), e.g. `fn exponent_denominator(&self, op: &str, content: Option<usize>, result: impl FnOnce() -> Box<dyn Any>) -> usize`, that performs the content-or-input choice, the lazy output read-back, and the honesty assertion; `measure` derives `exp_denom_bytes` from it and `export` returns it; lift the repeated `.expect` into one private `fn cell(&self) -> Cell`. Acceptance: exactly one `match` over `Denom` computes a denominator in the board module; for a `Denom::Input` cell, `denominator_bytes` does not invoke the body (a counting body in a unit test); the bench sidecar's denominator file is byte-identical before and after at the record scales.

### board-frame-25: `BOARD_DECLARED_BENCH_RIDERS` is a hand-maintained cell list in a module whose doc says none exists, derivable from each cell's own declarations, pinned in one direction only, and it is the time leg the committed cadence judges
- Where: crates/before/src/meter/board/export.rs:104-121 (related: crates/before/src/meter/board/export.rs:9-11, 92-93, 144-151, crates/before/src/meter/board/tests.rs:1089-1127, crates/before/src/meter/board/ops.rs:107-187, crates/before/src/meter/board.rs:184-187, 293, crates/before/src/meter/registry.rs:581, tools/benchjudge-expected.json:2, justfile:840, 843-844, 1003)
- Class / severity / confidence: scaffolding / medium / high
- Provenance: verified (tests.rs:1103-1125 asserts only rider implies declared (`declared_heap.is_some() || declared_limb.is_some()`); export.rs:147-148 unions the list with `designed` and :151 already runs `prepare`, so the `Cell` is in hand; `designed` maps `AscendCliff` and `MirrorWide` to `Tick` only (ops.rs:125-135) while `version_min_ticks` is `Measure` and the display rows are `Version`/`Clock`, and the other `with_declared_heap` rows (ops.rs:392, 428, 1587) are `Tick` rows on `AscendCliff`, so the derived set equals today's three riders; justfile:840 `bench-judge sampling="quick" cells="pinned"` sets `BOARD_BENCH_MODE=full` only when `cells == "full"` (:843-844) and `just all` (:1003) invokes it bare, so the pinned subset is what the cadence judges); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (severity stands at medium because pinned is the cadence of record; the export.rs:9-11 "full ... for final verdicts" sentence describes a mode no recipe runs); history: deliberate-but-expired (born as `BOARD_RED_BENCH_RIDERS` in 3eadcb107, a triage set with no cell property to derive from; 669cf3103 migrated every red to a `Cell` property and kept the list form only to hold the bench roster byte-identical, adding the one-direction pin; the constraint that required a list expired in that commit)
- Owner-gated: yes (removes a `pub` const of the meter-feature surface cited from registry.rs and the benchjudge roster's notes; whether the fold and capacity models count as declared models for rider purposes is an owner ruling)

export.rs states twice that the pinned subset is "a rule over the product, never a hand-maintained cell list" (:10-11, :92-93), yet the constant is exactly that, with a prose roster of its "current membership" and an instruction that a cure must edit it. The stated policy (:107-108) is that any declared-model cell keeps a wall-clock witness; the rule `designed(kind, group) || cell.declared_heap.is_some() || cell.declared_limb.is_some()` implements it from the `Cell` `bench_cells` already prepares. The test pins only rider implies declared, so a new declared model on a non-designed pairing lands without a time-leg witness while the test stays green (Principle 6), and the "declared model" the test knows (heap, limb) is narrower than the four ratified models board.rs:184-187 lists. Because `just all` runs the pinned subset, a missing rider has no wall-clock witness in the cadence that actually runs; the sentence at :9-11 saying full mode "times the whole product for final verdicts" describes a mode no committed recipe invokes.

Evidence:

       107	/// A cell judged green under an owner-declared counter model keeps a wall-clock
       108	/// witness even where no designed pairing times its shape.
       109	///
       110	/// Membership is by `(operation, family)` cell name, expectations live in the
       111	/// judge's roster as ever; a cell whose declared model dissolves (a cure
       112	/// landing) leaves this list in the same change. The current membership: the
       113	/// `version_min_ticks` reign-state cell and the display pair's render-merge
       114	/// cells. The tick trio's ascend-cliff cells need no rider — the tick group is
       115	/// those crosses' designed diagonal — and the tooth-tail parse cell rides its
       116	/// designed pairing likewise.
       117	pub const BOARD_DECLARED_BENCH_RIDERS: &[(&str, &str)] = &[
       118	    ("version_min_ticks", "ascend-cliff"),
       119	    ("version_display", "mirror-wide"),
       120	    ("clock_display", "mirror-wide"),
       121	];

    [export.rs:9-11]
         9	//! declared-model riders) while `BOARD_BENCH_MODE=full` times the whole product
        10	//! for final verdicts — the subset is a rule over the product, never a
        11	//! hand-maintained cell list.

    [board/tests.rs:1120-1121; justfile:840]
      1120	        assert!(
      1121	            cell.declared_heap.is_some() || cell.declared_limb.is_some(),
       840	bench-judge sampling="quick" cells="pinned":

Resolution: In `bench_cells`, keep the prepared `Cell` and include a cell under `BenchMode::Pinned` when `designed(family.kind, op.group) || cell.declared_heap.is_some() || cell.declared_limb.is_some()`; delete the constant, its re-export at board.rs:293, the registry.rs:581 link, and the one-way test (the property becomes structural); re-word the benchjudge roster's notes. Rule explicitly on the fold and capacity models (today every such cell sits on a designed pairing, so the derived subset is unchanged either way) and state the rule at `BenchMode::Pinned`. Re-word export.rs:9-11 to what the cadence judges (the pinned subset; full mode is an owner-invoked verdict) or add a full-mode leg to `just all`. If the constant must survive as public vocabulary, pin the converse instead: compute declared minus designed from the cell table and assert set equality with the roster. Acceptance: `bench_cells(scale, BenchMode::Pinned)` at the record scales yields today's cell set (diff the sidecar's denominator file); adding `.with_declared_heap(x)` to a row on a non-designed family makes that cell appear in the pinned subset with no other edit; no `BOARD_DECLARED_BENCH_RIDERS` symbol remains, or a test asserts the converse.
Construction: Add `.with_declared_heap(ASCEND_CLIFF_MIN_TICKS_HEAP_BYTES_PER_INPUT_BYTE)` to `version_min_ticks` on a second family whose `designed` arm excludes `Measure` (e.g. `Staircase`, a Tick-designed family); `bench_riders_name_declared_model_cells` passes unchanged, `bench_cells(_, BenchMode::Pinned)` omits the new cell, and `just bench-judge` never times it, contradicting export.rs:107-108.

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

## Positives

- `currency.rs` is the strongest design in the frame: `ByCurrency<T>` has no `Default` and no `..` path, `each` destructures exhaustively into a `[_; 5]` the compiler checks, and `Floors = ByCurrency<Liveness>` forces every cell to answer floor-or-NA per currency. Adding a currency really is a compile error at every declaration and judgment site, and the module doc (:13-25) names the exact failure class this makes inexpressible rather than asserting safety in the abstract. (Verified by reading.)
- `ceilings.rs:56-62` states and follows the present-tense discipline for measurements: readings live in the pin commits, `git log -S` the constant, with the reason given (a quoted reading keeps asserting itself as fact while headroom absorbs drift). c0b5d701's message carries the touch ceiling's basis (16.0 and 17.18 touches per byte; ceil(17.18 × 1.25) = 22; 2,096 green / 0 red). (Verified via `git log -1 --format=%b`.)
- The output-honesty ceiling is derived rather than calibrated (`TEXT_BYTES_PER_RADIX_UNIT`, ceilings.rs:203-216: at most 6 syntax bytes plus digits per value, one radix unit minimum, hence under 7), enforced by `assert_honest_text` (cell.rs:317-323) on every text stream entering a denominator on both the board (measure.rs:114-116) and bench (export.rs:78-80) paths, and tripwired (`rendered_text_is_honest_and_padding_trips`, board/tests.rs:108). Derivation, implementation, and tripwire all present. (Verified by reading; the tripwire's name and location, not its body.)
- `defect.rs` places every rejection defect maximally deferred with a per-shape derivation of why that position is the last discoverable one; the packed-side constructions are correct against the codings (a `00` terminal rewritten to `11 00 00` is exactly the one id canonicity rule left to enforce; an equal-sibling zero delta is exactly the skyline minimality violation); every walk is iterative; the `expect` messages are one-line proofs ("a stored stream is canonical"); `clock_trailing_text` correctly anticipates `parse_clock_str`'s O(1) outer-paren check (text.rs:188) by riding the defect inside the version component. (Verified by reading.)
- `coverage/tests.rs` checks the tiling in both directions (every surface row priced or excused, never both; every cited row live; every board row cited) plus duplicates, and every assertion message names the fix. The tables are plain `&[(&str, ...)]` data. (Verified by reading.)
- `export.rs`: the bench suite's criterion IDs are exactly the board's `(op, family)` names, `denominator_bytes` reads the output side back from the actual result, and `bench_cells` derives its rows from `ops()` and `FamilyId::board()`, so board coverage is bench coverage with no second enumeration of cells (the rider list, board-frame-25, is the one exception). (Verified by reading.)
- `measure.rs:84-123`: the metered region is exactly the body; every counter resets immediately before `(cell.body)()` and is read immediately after in a fixed order; denominators and the honesty assertion are settled after the reads and before `drop(result)`; peak heap is `peak - baseline` so pre-built operands do not count. (Verified by reading.)
- Cargo.toml:115-117 `required-features = ["limb-meter", "scan-meter"]` on the example, so the board of record cannot be built with three columns dark while still printing verdict colors. (Verified.)
- The `Denom`/`IoSpec`/`TextSpec` shape (cell.rs:161-199) with `fn(&dyn Any) -> usize` output readers keeps the ops rows declarative and makes a predicted output size unable to substitute for a measured one. (Verified by reading.)

## Open questions for Finch

1. Segments (board-frame-1): dissolve the currency, or keep it as a documented structural-zero pin at ceiling 0? Recommendation: dissolve. 1ddb5a483's reason for the dev-dependency move is that no library code recurses; a column that can only ever read zero in the binary of record names no failure it catches, and `LADDER_TOP_SCALE` should be re-derived from the four-point trend or stated as owner-ratified with its calibration story in the pin commit. The envelopes reviewer should confirm whether `tests/meter.rs`'s `segments: 0` pins (an integration binary, also without cfg(test)) are vacuous by the same construction; its module doc (:22-26) and :440-441 present segments as live.
2. Riders (board-frame-25): does the "every declared-model cell keeps a wall-clock witness" rule apply to all four ratified models (fold, capacity, family-stated heap, mirror-wide limb) or to the heap/limb pair the current test checks? Recommendation: derive the pinned subset from `declared_heap`/`declared_limb` and state at `BenchMode::Pinned` that the fold and capacity models are excluded because their cells sit on designed pairings by construction (pin that with a test if it must hold). Also: should `just all` run `bench-judge quick full`, or should export.rs:9-11 say the pinned subset is the verdict of record?
3. Validation index (board-frame-26): re-word the index to the envelope-class legs, or return the touch ceiling to class-scale and add envelope rows for the worst readers? Recommendation: re-word the index; the ×1.25 convention is owner-ratified and the cascade is stated as intended.
4. Public surface (board-frame-6): is the meter-feature board surface (the `pub use` constants, `ByCurrency`, `Currency`, `Floors`, `Liveness`) part of before's declared-stable API, or instrument surface that may be narrowed? Recommendation: narrow the fifteen unreferenced constants to `pub(super)` now; decide the doc-linked ten explicitly and record the decision at the `pub use`.
5. Root doc (board-frame-5): state a05918df7's summary-plus-pointer rule inline and replace verbatim copies with links? Recommendation: yes; the two verified drifts are the cost of the verbatim-copy criterion.
6. Vocabulary and em-dashes (board-frame-18, -20): schedule a crate-wide prose pass under the Documentation Change Policy? Recommendation: yes, as one pass; a partition-local fix would make this module inconsistent with roughly 140 other sites.
7. Process note from the history pass: b3f09baa0, titled "Partial WIP for docs pass, additional API tweaks", is on main and carries two of the stale counts found here ("four cells"; the reflowed "two n-ary fold rows"). Worth a look at whether WIP-titled commits should reach main.
8. Two items belong to the judgment partition but surfaced from this one's constants: judge.rs judges constants at `s2` alone while board.rs:222-224 says constants "stay judged per size across the ladder" and :71 says "at the larger scale" (one wording should give); and ceilings.rs:342 describes the fold exponent ceiling as fitted "across the cell's two probes" while judge.rs fits a four-point trend and computes the ceiling from the ladder's endpoints. Recommendation: settle both in the judgment review and align the wording here.
9. The riders test and the bench export build every family at 0.02 and run every prepare; is that scale a documented floor of applicability for every generator, or could a generator legitimately return `None` there and silently narrow the axis these tests reason over? Recommendation: name the constant once and state the premise where it is defined.

## Dropped

- Duplicates merged: candidates 14 and 40 into board-frame-1 (segments); 17, 30, and 43 into board-frame-25 (riders); 16 into board-frame-8 (log factor); 25 and 38 into board-frame-24 (denominator); 26 into board-frame-22 (preorder skeleton); 22 and 45 into board-frame-2 (board.rs drift) with their "four cells" halves into board-frame-3; 28 into board-frame-4 (dated rationale); 27 into board-frame-13 (`both_present_nodes`); 39 and 49 into board-frame-16 (`Cell` literals); 34 and 47 into board-frame-19 (tiling test); 21, 31, 32, 37 (the "today" and past-tense sites), and 48 into board-frame-18 (vocabulary); 44 into board-frame-23 (party text); 41 into board-frame-17 (`clock_hash`).
- Candidate 12 (bare `0b1000_0000` marker literal): folded into board-frame-21, whose one-byte cut removes the literal from defect.rs; the codec-side `[0x80]` is outside this partition.
- Candidate 13 (the board tiling as a second string-keyed roster): deliberate and documented; surface.rs:3-5 and :12-17 state inline that the roster is public under `meter` so instrument crates bind coverage tables by row name, and the tiling test is that mechanism's promised totality. A `SurfaceRow` field would couple the public roster to one consumer.
- Candidate 37's "has caught" (ceilings.rs:458-459): folded into board-frame-1 (the segment-onset rationale is the expired premise).
- Candidate 46's limb-calibration sub-item ("tens per packed byte ... over a hundred", ceilings.rs:88-90) and the ~8 bits per byte at :133: deliberately kept by 500d4d094 as order-of-magnitude calibrations with stated bands; board-frame-12 keeps only the two untriaged readings.
- Candidate 2's sub-claim that tests/meter.rs already pins the worst touch readers per scenario: refuted (its `TouchEnvelope` rows are all `RANK_*` scenarios); board-frame-26 carries the corrected form.
