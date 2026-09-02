# Partition envelopes-a: The resource-envelope pins, first half (tests/meter.rs lines 1-5305)

## Partition summary

`crates/before/tests/meter.rs` is `before`'s resource-envelope suite: a
single integration-test binary that runs each public operation (and each
skyline kernel behind it) on the adversarial input families from
`before::meter`, reads a set of deterministic process-global counters over
the metered body, and asserts every reading against committed constants.
The counters are peak heap (an external counting allocator, `peak_alloc`),
grown stack segments (`meter::stack_segments`), big-integer limb operations
(`limb-meter`), accumulator digit touches (`suanpan::touch_meter`), scanned
stream bits (`scan-meter`), and, in one band, densified digits. Lines 1-5305
hold the file doc, the scenario-size constants, four envelope tables
(`envelope`, `rank_env`, `sweep_env`, `emit_env`, `text_env`) with their four
harness functions (`metered`, `touch_metered`, `sweep_metered`, and
`query_metered`, the last defined past the range), the heap-meter canaries,
the tick flatness pins, and three band modules (`skyline_flatness`,
`eq_early_exit`, `ledger_wide_arming`) that judge per-unit cost across a
size doubling with derived liveness floors and absolute two-scale ceilings.
Every line in the range is test code; I read all 5305 lines plus the
`QueryEnvelope`/`query_metered` definitions past the range for context, and
the library sources each finding cites.

The measurement core is sound and, in several places, exemplary. Every
flatness band carries a value leg computed outside the metered body (a
closed-form tick total, rank modularity, byte-identity against the public
operator) before any counter is judged; the derived floors state their
derivation at the constant (`SEAM_PLUNGE_TOUCH_FLOOR`,
`LADDER_MARGINAL_TOUCH_FLOOR`, the weight-comb and freeze-parade floors);
each band names a committed known-bad kernel that fails through the same
meters, and every cited kernel resolves; the flatness judge cross-multiplies
in `u128`; wall time is kept out of the suite by design; and measurements of
record are excised from prose into pin commits.

The dominant issues are two. First, the file carries a layer of prose from
before the 2026-07-25 flag day (`faf3cd0a`, "Version stores the skyline
coding"): the header still says the implementation is "far from" the linear
contract that `lib.rs` now declares a hard guarantee, and a dozen test doc
comments describe recursion frames, quadratic path sums, and a decoder that
transcodes back to a packed form, all refuted by the pinned constants beside
them and by the code they describe. Second, several instruments are weaker
than their prose says: the `segments` column has no writer in this binary
(its only increment is `#[cfg(test)]`), so eighty-four `segments <= 0`
ceilings are tautologies presented as a live meter; the scan column has no
liveness floor in any table while the `DECODE_DENSE` row comment names one;
the public `cmp_dense`/`join_dense` rows assert nothing about their result
and pass a no-op; the nine "one-touch-per-operand-byte" floors have no
derivation and fail on the file's own control families (the refutation
pass's run shows `rank_bigroot` at 7,194 touches over 13,752 stored bytes);
and `Rank::cmp`'s documented O(1) leg is priced only jointly with two
O(width) operations. Around these sit a maintenance cascade (four envelope
structs and harnesses, three copies of the slack constants, five
`assert_flat`s, nine `Run` structs) that the campaign note already dockets
for unification, and a set of prose-hygiene and idiom nits.

## Findings

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

Resolution: Rewrite lines 4-11 to state the suite's present role: the contract is `lib.rs:350-353`'s; five operations the claims document demonstrates over their documented bounds are today's exceptions to it (`Version::join`'s re-anchor cascade, skyline-coding-9; `Ranked::cmp`'s settle, rank-33; the masked comparison's `peek_flip` term, skyline-sweep-place-masked-5; `Query::coverage`'s per-hole sweep, span-causally-36; `tick`'s memo-family heap, skyline-fill-grow-2), named here or in `lib.rs`'s Asymptotic Complexity section and linked from the other; and each row pins the current measured cost at ×1.25 so a regression fails and an improvement re-pins. Replace "Today's implementation is far from that" and its recursion-and-transcode narrative with that named list, never with a sentence asserting that the contract holds. Replace the meter enumerations at 13 and 357 with non-counting phrasing ("the deterministic meters", "under every meter"). Re-state each row doc in terms of the mechanism its table row's trailing comment already names: 428-429 (the validator's bit stack, drop "today"), 440-441 (the iterative sweep, zero segments), 465-466 (drop "today"), 983-985 and 996-997 (one wide root decode, or two, linear), the header 1400-1408 and the decoder docs at 1500-1501, 1517-1519, 1533-1534, 1548-1549, 1563-1564 (validate plus one exactly-sized copy, same scan reading as the validate row by construction), and 1896-1898 (the public operator's result, not an oracle; see envelopes-a-14 for the value-leg consequence). The assert messages' "transcoded"/"the transcode round-trips" (1429, 1447, 1463, 1481, 1497, 1512, 1530, 1545, 1560, 1575) legitimately name `Packed::version`'s construction-language transcode and may stay. Acceptance: `grep -nE 'far from that|Three deterministic|both meters|today|recursion-frame|per-frame|transcode back|materializ|packed-form oracle' crates/before/tests/meter.rs` over lines 1-5305 returns nothing but the construction-language sites; each decoder doc agrees with the table comment at 293-295.

### envelopes-a-2: The segments column has no writer in this binary; every `segments <= 0` pin is satisfied by construction
- Where: crates/before/tests/meter.rs:22-26 (related: 60-65, 364-391, 1212-1243, 1675-1709, 6900-6938; crates/before/src/recurse.rs:17-20, 68-69, 100-109, 118-129; crates/before/Cargo.toml:33-44; crates/before/src/meter/tests.rs:407-454; crates/before/AGENTS.md:26-37)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read `recurse.rs` whole: `SEGMENTS_GROWN` is `cfg(any(test, feature = "meter"))`, its only increment is inside `grow`, and `grow` and `descend!` are `#[cfg(test)]`; `stacker` is under `[dev-dependencies]`; grep shows `descend!` only in `testing/bridge.rs`, `grow/tests.rs`, `meter/tests.rs`; extracted the second argument of all 84 envelope rows in the file, all 0); executed: no
- Seen by: scaffolding, adequacy, instrument-correctness; refutation: confirmed ([17]/[46]; [10]'s proposed rewrite refuted); history: deliberate-but-expired (a defended keep of 2026-07-24 when production recursion existed; the last production recursive walk dissolved at 7b11b3ea (2026-07-26); `grow`/`descend!` became `#[cfg(test)]` at 1ddb5a48 (2026-07-31) with an inline keep decision whose "measured fact" premise now holds only in the lib's own cfg(test) unit tests)
- Owner-gated: yes: the segment meter is a recorded defended keep ("adjudicated once, not relitigated", note line 1756-1758), and dissolving a column is the removal of an instrument

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

### envelopes-a-3: The isolation premise is prose only; a shared-process runner can reset a counter mid-body and mask a regression
- Where: crates/before/tests/meter.rs:354-355 (related: 15-21, 363-368, 1212-1218, 1675-1681, 6900-6908; crates/suanpan/src/touch_meter.rs:16-20; justfile:109, 114)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (grep for `NEXTEST` and `RUST_TEST_THREADS` across `crates/before`, `crates/suanpan`, `tools`, `justfile`, `.config` finds nothing; the gate recipes at justfile:109 and 114 run `cargo nextest`; both AGENTS.md files prescribe nextest); executed: no
- Seen by: instrument-correctness (adequacy raised it as an open question); refutation: confirmed, downgraded to low; history: no-rationale-found (380d470e wired `ISOLATION_NOTE` into failure messages; no mechanical guard was considered)
- Owner-gated: no

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

### envelopes-a-4: Four envelope structs, four const constructors, four harness bodies, and five table preambles implement one measurement procedure
- Where: crates/before/tests/meter.rs:363-408 (related: 211-243, 1120-1172, 1206-1275, 1595-1635, 1669-1732, 6746-6805, 6894-6976; preambles 245-257, 1174-1189, 1637-1647, 1865-1875, 2139-2147; 265-267; 2635-2670; 5203-5208)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`grep -n '^fn [a-z_]*metered'` returns exactly 363, 1206, 1669, 6894; the improvement-tripwire message appears at 402, 1260, 1269, 1726, 5123, 5141, 6961, 6970, 8551; read all four struct/constructor/harness triples and the five preambles); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: already-known (the campaign note's post-campaign docket, line 1751-1755: "collapse the envelope harness shapes in `tests/meter.rs` into one five-column shape with per-column floor-or-NA")
- Owner-gated: no

`Envelope`/`envelope`/`metered`, `TouchEnvelope`/`touch_envelope`/`touch_metered`, `SweepEnvelope`/`sweep_envelope`/`sweep_metered`, and `QueryEnvelope`/`query_envelope`/`query_metered` are the same reset, run, read, print, assert sequence differing only in which columns exist, with two print-composition styles (two cfg'd `eprintln!`s versus `format!` fragments) and the tripwire message copied nine times. The cascade is visible: a row can pin only the columns its struct happens to carry, so `skyline_render_records_zero_touches` exists as a separate test because `SweepEnvelope` has no touch column, the tick rows moved to `query_env` with a pointer comment at 265-267 because the four-column table "never watched" touches, the door rows lack the scan column their kernel twins carry (envelopes-a-8), `ledger_wide_arming::run` reads densified digits ad hoc, and the preamble convention is restated five times with drift (only 1186-1189 carries the re-denomination sanction). Infrastructure that generates its own maintenance cascade is suspect; this one is already docketed.

Evidence:

       363	fn metered<R>(name: &str, input_bytes: usize, env: &Envelope, f: impl FnOnce() -> R) -> R {
      1206	fn touch_metered<R>(
      1669	fn sweep_metered<R>(
      6894	fn query_metered<R>(

       265	    // The tick rows live in `query_env`: the tick walk's cost currency
       266	    // is accumulator digit touches (with scanned bits beside it), which
       267	    // this four-column table never watched.

Resolution: This is the docketed unification; the note records it as a deferred disposition, so treat this entry as a reminder with the range's evidence attached. One `Envelope` with `peak_heap` and, per optional column (limb, scan, touch, densify), an `Option<Pin { ceiling, floor }>` where `None` means not watched; one `const fn`; one `metered` that walks a static column table and composes the MEASURED line from fragments as `sweep_metered` already does; the pin convention stated once in the file doc and cited by each table in a sentence. Fold `skyline_render_records_zero_touches` into the `SKYLINE_RENDER_*` rows as a touch column pinned to 0, move the tick scenarios (465-713, 2043-2122) beside the table they use and delete 265-267. Acceptance: one harness function and one envelope type serve every table; every existing row's numbers are unchanged; the tripwire message appears once; the pin convention appears once.

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

### envelopes-a-6: The scan column has no liveness floor in any table, and the DECODE_DENSE row comment names floors its table lacks
- Where: crates/before/tests/meter.rs:262 (related: 211-227, 280-300, 1595-1610, 1716-1721, 6746-6774; 46-49; crates/before/src/codec/scan.rs)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read `Envelope` at 211-227: no scan or touch field; `SweepEnvelope` at 1595-1610 and `QueryEnvelope` at 6746-6774: `scan_bits` ceiling, no scan floor; scan floors in range exist only inside bands at 932 and 5139; `git log -1 --format=%B e4c9b083` carries "touch and scan floors are now those rows' liveness signal", the commit that zeroed the limb columns); executed: no
- Seen by: adequacy; refutation: confirmed; history: already-known in part (2f6df434 chose two exact scan witnesses, `skip_int`/`read_int` exactness and the `id_covers`/`id_disjoint` exact pins, over per-row scan floors "since weak per-row floors clear partial undercounts"; per-column floor-or-NA is the docketed unification); the `DECODE_DENSE` annotation is a blanket sentence from e4c9b083 applied to a table with neither column
- Owner-gated: no

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

### envelopes-a-7: Presentation hygiene: half-aligned rustfmt::skip tables, em-dashes in comments and messages, a whitespace run, a terse floor message
- Where: crates/before/tests/meter.rs:261-263 (related: 271-276, 291, 299, 1193-1198, 1651-1661, 1879-1886, 2151-2159; 759; 402-404, 934, 1260, 1269, 1726, 3242, 3249, 3394, 5123, 5141; 4215-4218)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the five tables; awk counts 39 non-doc `//` comment lines and 11 code or string lines carrying an em-dash in the range; line 759 carries a 16-space run); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: no-rationale-found (padding was uniform at inception and after the flag day; e4c9b083's re-pin dropped it on the rows it rewrote and `rustfmt::skip` re-aligns nothing)
- Owner-gated: no

Under `#[rustfmt::skip]` some rows are padded to the header columns (271-276, 291, 299, 1197-1198, 1661, 1884, 2152-2155, 2158) and the rest are compact, so the header aligns with no row and trailing comments run past 300 characters (262, 2156); five-column `SweepEnvelope` rows sit in `mod envelope` under a four-column header. Em-dashes appear in 39 non-doc comment lines and in the assert messages that reach the terminal on failure; line 759's message carries a run of literal spaces mid-sentence; the floor assert at 4215-4218 prints no counts while every sibling floor does.

Evidence:

       261	    //                                              peak heap,  segments, limb ops, limb floor
       262	    pub const DECODE_DENSE: Envelope = envelope(120_035, 0, 0, 0); // wire decode is validate + wrap on the skyline kernels; decoded payloads ride the word-valued form, so narrow-value work leaves the limb denomination (touch and scan floors stay the liveness signal)
       271	    pub const DECODE_HUGELEAF: Envelope = envelope(   122_504,        0,         2_443, 1_465); // the validating wire decode holds the running height; one wide gamma code's linear limb work

       759	                "{name}/{col}: ticks({}) -> ticks({}) moved {delta}                  (from {at_lo} to {at_hi}), outside the gamma-width band {band}",

      4215	        assert!(
      4216	            run.touches >= run.deltas,
      4217	            "rank height state left the accumulator"
      4218	        );

Resolution: Align every row to its header or drop the skip attribute; move each row's mechanism sentence to a `///` line above the row; put the validator/decoder rows in `sweep_env` (or dissolve them per envelopes-a-10). Rewrite the comment and message em-dashes with colons or semicolons; collapse the whitespace at 759; give 4215-4218 the `{touches} under the {deltas}-delta floor` form its siblings use. Acceptance: each skipped table is uniformly aligned under a header naming its columns; no em-dash on a `//` line or inside a string literal in the range; 759 has single spaces.

### envelopes-a-8: Five public-door rows and five kernel rows pin identical numbers on identical shapes
- Where: crates/before/tests/meter.rs:263-264 (related: 269-270, 278-279, 1649-1662, 1876-1887, 1745-1849, 1909-2041; crates/before/src/version.rs:889-899, 1729-1733)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (compared the constants: `CMP_DENSE`/`SKYLINE_CMP_DENSE` 30_720; `CMP_CLIFF`/`SKYLINE_CMP_CLIFF` 1_330, 88, 52; `JOIN_DENSE`/`SKYLINE_JOIN_DENSE` 130_277; `JOIN_BIGROOT`/`SKYLINE_JOIN_BIGROOT` 85_060, 1_565, 939; `JOIN_CLIFF`/`SKYLINE_JOIN_CLIFF` 5_362, 308, 184; `partial_cmp` routes to `sweep::causal_cmp` (version.rs:1731) and `|` to `emit::join` (version.rs:899)); executed: no
- Seen by: scaffolding; refutation: reframed (the kernel twins are not "the scan column only": they hold the value legs the door rows lack); history: deliberate-but-expired (the kernel rows were added on 2026-07-23 to measure the skyline candidate beside the packed production coding; the flag day routed every door to the kernels; both rosters survived under the campaign's never-remove-tests constraint, which has lapsed)
- Owner-gated: yes: which roster survives is a design choice, and removal of rows follows the note's dissolution ratchet

Two rosters pin the same surface, born of a constraint (two codings) that no longer exists. Today the door rows can catch door regressions (a clone rung allocating, an empty-operand identity misrouting) but assert nothing about the result; the kernel rows carry the scan column and the verdict or byte-identity legs. Consolidation must move both the scan column and the value leg to whichever side survives.

Evidence:

       263	    pub const CMP_DENSE: Envelope = envelope(30_720, 0, 0, 0); // the iterative sweep over the Bytes-backed at-rest form (OpenedPair states the pair walk's opening move once); word-valued payloads keep the limb column at zero
       264	    pub const JOIN_DENSE: Envelope = envelope(130_277, 0, 0, 0); // the emit kernel's peak alone: the value-operator cell's lhs clone is a refcount bump, not a byte copy of the operand; word-valued payloads keep the limb column at zero

      1652	    pub const SKYLINE_CMP_DENSE: SweepEnvelope = sweep_envelope(30_720, 0, 0, 468_760, 0); // path-bit stacks and one accumulator; word-valued payloads keep the limb column at zero
      1880	    pub const SKYLINE_JOIN_DENSE: SweepEnvelope = sweep_envelope(130_277, 0, 0, 625_018, 0); // the peak is the emitted stream itself; word-valued payloads keep the limb column at zero

Resolution: Decide which side is the roster of record. If the door: give the door rows the scan column (falls out of envelopes-a-4) and the value legs (envelopes-a-9), port the kernel-only shapes (`SKYLINE_CMP_DENSE_SELF`, `SKYLINE_CMP_WIDE_TOOTH`, `SKYLINE_JOIN_ABSORB`, `SKYLINE_JOIN_WIDE_TOOTH`, `SKYLINE_MEET_*`) to `partial_cmp`, `|`, `&`, and retire the five twins with their tests. If the kernel: state at each retained door row the door cost it exists to catch. Acceptance: no two rows pin the same (shape, operation) through a door and its kernel; every retained kernel row's comment names the door cost it deliberately excludes.

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

### envelopes-a-11: SKYLINE_DECODE_* rows pin a meter-only wrapper whose limb and scan columns duplicate the validate rows exactly
- Where: crates/before/tests/meter.rs:293-300 (related: 288-292, 1500-1576; crates/before/src/version/skyline.rs:167-171, 211-216, 262-274; crates/before/src/version/skyline/decode.rs:14-20; crates/before/src/version.rs:1110-1127)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`mod decode` is `#[cfg(any(test, feature = "meter"))]`; `decode_bits` callers are skyline.rs:273 and skyline/tests.rs only; `Version::decode` runs `validate_prefix` and adopts the read buffer; the five decoder rows' limb and scan columns equal the five validator rows' (0/468_758, 88/17_923, 29_509/1_000_480, 2_443/312_503, 0/468_758); `decode_bits` frees the validator's transient before allocating the copy, so the row's heap is the larger of the two, and the Dense and AltSpine decode heap pins equal their validate pins exactly at 61_440); executed: no
- Seen by: scaffolding, adequacy; refutation: confirmed, severity low; history: deliberate-but-expired (warranted when `skyline::decode` transcoded the candidate coding into the production packed Version; orphaned at the flag day; retained under the campaign's never-remove-tests constraint, which has lapsed; f6c0a6a4 re-pinned them to the copy rather than dissolving)
- Owner-gated: yes: removal of an instrument, governed by the note's dissolution ratchet

The world without these five rows loses only a heap pin on a five-line meter-only function allocating more than one copy; the production decode door is already pinned by `DECODE_DENSE`/`BIGROOT`/`HUGELEAF`/`CLIFF`, and every other column is pinned by the validate rows byte for byte. Suites exercise the public API, with internal entries as documented exceptions; here the exception pins nothing outside itself.

Evidence:

       296	    pub const SKYLINE_DECODE_DENSE: SweepEnvelope = sweep_envelope(61_440, 0, 0, 468_758, 0); // decode is validate + wrap: the wrap allocates the copy once, exactly sized
       288	    pub const SKYLINE_VALIDATE_DENSE: SweepEnvelope = sweep_envelope(61_440, 0, 0, 468_758, 0); // the open-ancestor bit stack; word-valued payloads keep the limb column at zero

    skyline.rs:
       269	/// Test- and meter-only: the production decode ([`Version::decode`]) validates
       270	/// the prefix and adopts the buffer without this wrapper.

Resolution: Dissolve the five `SKYLINE_DECODE_*` rows and their tests (1500-1576), keeping the round-trip equality as a plain unit test if `skyline::decode` stays for the tests' vocabulary; if WideTooth and AltSpine matter at the decode door, add `DECODE_WIDE_TOOTH` and `DECODE_ALT_SPINE` through `Version::decode`. Acceptance: no envelope row measures `meter::skyline::decode`; the public `DECODE_*` rows cover every shape the owner wants pinned at decode.

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

### envelopes-a-13: Mixed idioms for one purpose: `consumed` beside `black_box`, a one-line `version_of` alias, qualified `Ordering`, bare-bool selectors
- Where: crates/before/tests/meter.rs:410-424 (related: 318, 449, 993, 1078, 1287, 1302, 1321, 1361, 1395, 3519, 3734, 3834, 4523, 4631, 4813, 5205; 68, 1356, 2453, 4362, 4433, 4724; 1899-1907, 4201; 3215-3216 against 3237-3238; crates/before/src/meter.rs:116-121)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep: `consumed(` at 8 sites and `std::hint::black_box` at 8 sites in range; `std::cmp::Ordering::` spelled out at 1356 in the scope that imports `Ordering` at 68 and at four band sites; `git log -S'fn version_of'` dates the helper to 0d1ea4905, before `Packed::version` landed at faf3cd0a); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Two idioms keep results alive; `consumed` adds a `Debug` bound and a `String` allocation, and its doc overclaims (formatting a returned `Option<Ordering>` proves a value was returned, not that a walk ran to completion). `version_of` is `p.version()` and its name hides the transcode the callee's own doc states, which is how envelopes-a-12's premise went stale. `skyline_oracle(p, join: bool)` and `rank_jump_pair_operand_run(.., band: bool)` read as `skyline_oracle(&p, true)` at the call site. `as i64` casts in a MEASURED print sit beside `i64::try_from(..).expect(..)` in the assert two lines later.

Evidence:

       410	/// Lift a generated shape into a [`Version`], outside any measurement.
       411	fn version_of(p: &meter::Packed) -> Version {
       412	    p.version()
       413	}
       420	/// Assert a scenario result is consumed, so the operation cannot be
       421	/// dead-code-eliminated and the walk provably ran to completion.
       422	fn consumed<T: Debug>(v: T) -> String {
       423	    format!("{v:?}")
       424	}

      1356	    assert_eq!(ord, std::cmp::Ordering::Less, "1/2^d is under 3");

Resolution: Use `std::hint::black_box` everywhere and delete `consumed`; replace `version_of(&p)` with `p.version()` and delete the alias (keep `party_of`, which decodes); import `Ordering` in the band modules; replace the bools with an `enum Op { Join, Meet }` or two functions; use `i64::try_from` in both places or print `abs_diff` with a sign. Acceptance: `grep -c 'fn consumed\|fn version_of'` is 0; no `std::cmp::Ordering::` where `Ordering` is imported; no bool selector parameters.

### envelopes-a-14: `Rank::cmp`'s documented O(1) class-first leg is priced only jointly with `checked_sub` and `+`
- Where: crates/before/tests/meter.rs:1344-1354 (related: 1197, 1324-1334; crates/before/src/version/rank.rs:875-906; crates/before/src/meter/board/ops.rs:499-506)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: assessed (read rank.rs:879 "Unequal magnitude classes settle in `O(1)`" and the class-first arm at 896-905; read the board's `rank_pair` cell, which runs the same three operations together; arithmetic from the pinned constants: limb ceiling 87_910 over a ×0.75 floor of 52_746 implies a basis near 70_328 and about 17,500 limb ops of headroom, while the mismatched operand's 500,000-bit exponent is about 7,813 limbs); executed: no
- Seen by: adequacy; refutation: confirmed (with the caveat that the slack arithmetic depends on which comparison the regression routes through: an aligned shift plus `Base::cmp` records about 23k ops and trips the ceiling, an aligned shift plus `msb_cmp` about 7.8k and hides); history: deliberate-but-expired (00cace5d pinned the row as one red baseline for three co-amplifying legs; d8f91040 cured the cmp leg and re-pinned the joint row instead of splitting it)
- Owner-gated: no

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

### envelopes-a-15: Flatness helpers (`Run`, `assert_flat`, `assert_ceilings`, the 5/4 slack) are re-implemented per module and bypassed inside their own module
- Where: crates/before/tests/meter.rs:2344-2347 (related: 2351-2356, 2927-2931, 2966-2985; 2775-2809, 2882-2910, 4273-4311; 2731-2735, 2862-2866, 4238-4241; 5016-5021, 5186, 5295-5296; 6331, 6457-6460, 6485, 8080-8083, 8087, 8420; inline 4/5 ratios at 5439, 5512, 5635, 5811, 6333, 8476, 8578, 8960, 9153)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (grep over the whole file: `const SLACK_NUM` at 2344, 6457, 8080; `fn assert_flat` at 2393, 6331, 6485, 8087, 8420 with differing signatures plus `assert_flat_step` at 5869; `struct Run` at 2351, 5016, 6285, 6468, 7822, 8202, 8376, 8681, 9222 plus `QueryRun` at 2927; `fn assert_ceilings` at 2966 and 7857; the inline `* 4 <= ... * 5` ratios listed; read the three hand-rolled ceiling loops in range); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: no-rationale-found (the three `SLACK_NUM` copies arrived with three separate rounds; the inline loops at 2775-2809 and 2882-2910 predate `assert_ceilings`)
- Owner-gated: no

Within `skyline_flatness`, `Run` and `QueryRun` differ by one field, `assert_ceilings` accepts only `QueryRun`, so the `Run`-based bands re-spell its loop three times over `(u64, u64)` tuple constants that exist only because the `[(u64, u64); 2]` shape is not shared; `eq_early_exit::Run` and `ledger_wide_arming::run`'s bare 4-tuple add two more reading shapes, and the 5/4 ratio is inlined at ten sites across the file. Today every copy agrees on 5/4, which is luck the file relies on; a reader must re-verify each inline ratio's direction.

Evidence:

      2344	    const SLACK_NUM: u64 = 5;
      2347	    const SLACK_DEN: u64 = 4;

      2966	    fn assert_ceilings(name: &str, small: &QueryRun, large: &QueryRun, ceilings: [(u64, u64); 2]) {

      2775	        for (run, (touch_ceiling, limb_ceiling), scale) in [
      2776	            (
      2777	                &over_small,
      2778	                (
      2779	                    FREEZE_BAND_OVER_TOUCH_CEILINGS.0,
      2780	                    FREEZE_BAND_OVER_LIMB_CEILINGS.0,
      2781	                ),

      5186	    fn run(w: usize) -> (u64, u64, u64, u64) {
      5295	                u128::from(large) * u128::from(small_bytes) * 4
      5296	                    <= u128::from(small) * u128::from(large_bytes) * 5,

Resolution: Hoist one `Run` (with `deltas: Option<u64>`), one `assert_flat`, one `assert_ceilings`, and the slack constants into a file-level `#[cfg(feature = "limb-meter")] mod support` used by every band module; convert the tuple ceilings (`FREEZE_BAND_OVER_*`, `RANK_JUMP_*`, `DISTANCE_JUMP_PAIR_*`) to `[(u64, u64); 2]` and route the three inline loops through `assert_ceilings`; return a `Run` from `ledger_wide_arming::run`. Acceptance: one `const SLACK_NUM`, one `fn assert_flat`, one `struct Run` in the file; no inline `* 4 <= ... * 5` ratio remains.

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

### envelopes-a-19: Closed forms, the tick-family fixture, and the `UBig`-to-`Ticks` round trip are spelled twice, and generator widths and file-level scales appear as literals
- Where: crates/before/tests/meter.rs:3503-3506 (related: 3977-3980; 3719-3721 and 4034-4036; 729-745 and 888-904; 770-781 and 796-815; 8 `parse::<before::Ticks>` sites plus 880-885; literals: 4352 and 4418 against 188, 4398-4399 and 4463-4464 against 191, 2418-2419, 2486-2487, 2553-2554, 2625-2626, 5102-5103, 923 and 932 (`64`), 2079-2080; widths 288/289/290/608/33/34/16 at 3503-3506, 3719-3721, 3818, 4174, 4205, 4510, 4609-4618, 4798-4800, 5191; crates/before/src/meter.rs:1630, 1689, 1696, 1701, 1799, 2153, 2967)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both copies of each form; grep confirms `MASK_DRIFT_MAGNITUDE_BITS = 512` at 188 and `MASK_DRIFT_TEETH = 1_024` at 191 beside `packed_triple(512, scale)` and `masked_cmp_run(1_024)`; the generator constants `FREEZE_POSITION_DROP_BITS = 288`, `PROMOTION_REARM_ARM_BITS = 608`, `PROMOTION_REARM_SETTLE_BITS = 288`, `PROMOTION_REARM_LEVELS_PER_BLOCK = 32`, `DENSE_SUFFIX_DIGIT_STRIDE = 33`, `LONE_FREEZE_PLATEAU_BITS = 288`, `JUMP_PAIR_DIGIT_STRIDE = 33` exist privately in src/meter.rs); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed ([13]/[52]) and reframed ([37]: the closed-form widths are legitimately spelled independently of the generator as an oracle, but deserve one named definition in the band module); history: no-rationale-found (64ed0d1c introduced `MASK_DRIFT_MAGNITUDE_BITS` and the literal 512 in the same commit; 36afe22e copied the closed forms; the seam bands landed later with named `*_ticks` helpers)
- Owner-gated: no

A closed form is the value leg every band rests on; two copies can drift apart when a generator changes and only one is updated (the seam and ladder bands already show the better shape: `seam_plunge_ticks`, `latent_ladder_ticks`). The 512/1_024/2_048 literals shadow constants the file already names. The widths 288 and 608 are the oracle's independent statement of the generator's premises, which is the point, but a reader cannot tell 608 from 288 without opening src/meter.rs; one named definition per width at the top of `skyline_flatness`, with a comment naming the generator constant it mirrors, keeps the independence and the legibility.

Evidence:

      3503	        let band = 289 + (usize::BITS - k.leading_zeros()) as usize;
      3504	        let expected = (UBig::from(2 * k as u64) << band)
      3505	            + UBig::from((k * (k - 1)) as u64) * ((UBig::ONE << 288usize) + UBig::ONE)
      3506	            + UBig::from(k as u64);

      3977	            let band = 289 + (usize::BITS - k.leading_zeros()) as usize;
      3978	            (UBig::from(2 * k as u64) << band)
      3979	                + UBig::from((k * (k - 1)) as u64) * ((UBig::ONE << 288usize) + UBig::ONE)
      3980	                + UBig::from(k as u64)

      4352	        let (comb, mask, plateau) = Shape::MaskDriftTriple.packed_triple(512, scale);
       188	const MASK_DRIFT_MAGNITUDE_BITS: usize = 512;

Resolution: Add `freeze_position_ticks(k)`, `promotion_rearm_ticks(p)`, a `tick_families()` fixture, and a `fn ticks(u: &UBig) -> before::Ticks` beside the existing closed-form helpers; reference `super::MASK_DRIFT_MAGNITUDE_BITS`, `super::MASK_DRIFT_TEETH`, `super::CLIFF_SCALE` in the band modules; name the small-run scales per band; name the word slack at 923/932; define the closed-form widths once in `skyline_flatness`; use `div_ceil(8)` for both terms at 2080. Acceptance: each closed form and the tick fixture have one definition; the decimal round trip appears once; no bare 512/1_024/2_048 where a file-level constant exists; no bare 288/608/33 in a closed form.

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

### envelopes-a-22: Three accumulator-mechanism bands rest on an uncommitted "local probe build" as their known-bad demonstration; the eq early-exit band's demonstration is recorded but uncited
- Where: crates/before/tests/meter.rs:4479-4498 (related: 4544-4555, 4652-4665, 4737-4747; 5070-5078; .cargo/mutants.toml:48-51; crates/suanpan/src/accumulator/tests/metered.rs:31, 92, 406; crates/before/src/version/skyline/query/tests.rs:1495, 1806, 2169, 2197, 2641)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read the three band constants' docs and the section header; the suanpan row witnesses named at 4495-4497 exist at metered.rs:31, 92, 406; the sibling bands' `_reads_superlinear` kernels resolve in query/tests.rs; `.cargo/mutants.toml:48-51` records `sweep::eq_exit`'s `||`-guard mutant as killed by the eq_exit row; `tests/superlinear_tripwires.rs` rosters the `_reads_superlinear` genre by name); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed ([11]) and reframed ([44]: the eq early-exit demonstration is durable and recorded, only the doc citation is stale); history: no-rationale-found for the uncommitted kill switches; already-known for eq_exit (bff7b04c measured the mutation; the mutants roster records the kill)
- Owner-gated: yes: committing a probe as a gated known-bad path is a design choice

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

## Positives

- Derived liveness floors done the way the doctrine asks, with the premise stated at the constant: `SEAM_PLUNGE_TOUCH_FLOOR` and `SEAM_STOP_TOUCH_FLOOR` from the dying folds' three-digit spans (3145-3154, 3301-3309), `LADDER_MARGINAL_TOUCH_FLOOR` from three register folds per decision leaf (3417-3427), the weight-comb and freeze-parade floors from nonzero-delta counts with the explicit sentence "the mechanism's irreducible work, not the family's typical work" (4529-4533, 4637-4641), and one touch per delta for the comb validate/cmp/join/parse runs (2361-2367, 2438-2441). (Verified by reading.)
- Value legs before cost legs: every flatness run asserts the family's closed-form tick total, rank modularity (`distance == lag + lag`, 4191-4195), pointwise-domination identities (3643-3649), verdicts (2451-2455), or byte-identity against the public operator (2529) before any counter is judged, and `ticks_counters_wide` holds `min_ticks` moving by exactly `n` (809-813). (Verified by reading.)
- Known-bad kernels are cited by name and exist: `absolute_position_accounting_reads_superlinear_on_freeze_position`, `span_promotion_accounting_reads_superlinear_on_rearm_spine`, `suffix_walk_settle_reads_superlinear_on_dense_suffix(_pair)`, and `schoolbook_settle_reads_superlinear_on_wide_arming` all resolve in `query/tests.rs`, and `tests/superlinear_tripwires.rs` binds the genre by name in both directions. (Verified by grep.)
- The wide-count ticks band (817-939) is an exemplary asymptotic pin: a growth bound derived from the mechanism (two count-carrying codes, width-linear arithmetic), the piecewise-linear regime knee analysed and every family placed relative to it, a derived scan floor of `2·(bits(n) − 64)`, and an exact `min_ticks` value leg. (Verified by reading.)
- `eq_early_exit` (4983-5150) turns a prose contract into a tail-independent two-scale number with the deciding interval's magnitude fixed so only the tail the exit must not read grows, and the mutants roster records the mutant it kills. (Verified by reading.)
- Two-sided bands where the standard growth band is one-sided: the clearance band (3283-3298) and the latent-ladder marginal (3467-3477) check both directions; the seam plunge/stop pairs difference out shared work against a wire-near-identical control (3160-3175, 3315-3327). (Verified by reading.)
- `assert_flat` (2393-2408) compares ratios by `u128` cross-multiplication with no floating point and prints milli-per-unit so re-pins are legible. (Verified by reading.)
- Measurements of record are excised from prose and live in pin commits (`git log -S` the constant); `ISOLATION_NOTE` rides every envelope failure; the heap meter has a real canary (305-326); the heap column uses an external counting allocator rather than a hand-rolled one. The instrument-correctness lens reports (unverified by me) that `peak_alloc 0.3.0`'s `reset_peak_usage` stores CURRENT into PEAK and counts `alloc_zeroed` and `realloc`, so the delta arithmetic at 367-370 matches the allocator's semantics.

## Open questions for Finch

1. Segments column (envelopes-a-2): the 2026-07-24 keep's premise changed on 2026-07-31 when `grow`/`descend!` became `#[cfg(test)]`. Dissolve the column from the envelope suite (and re-state the stack-cost story as the depth test), or make a writer reachable under `meter` and add a canary here? Recommendation: dissolve; the iterative-walk rule is proven by `clock::tests::deep_tree_stack_safety`, and the deep scenarios here crash on any recursion regression regardless. The same no-writer property should hold for `examples/amp_board.rs` (an example binary links the non-test library); the board reviewer should confirm.
2. Door versus kernel roster (envelopes-a-8, envelopes-a-9): which side is the roster of record? Recommendation: the door, with the kernel-only shapes ported and the value legs added; keep a kernel row only where the door demonstrably adds cost the row must exclude. The refutation pass observed a further twin pair past this range (`rank_bigroot`/`skyline_rank_bigroot` at heap 57_604, limb 2_191, touches 7_194; `rank_dense`/`skyline_rank_dense` at 24_576/3/5), for the envelopes-b finalizer.
3. Flatness slack (envelopes-a-16): share the board's exponent bar (`10/9` per doubling for 1.15) or keep ×1.25 and say so? Recommendation: keep ×1.25 where a band's declared model needs it (the dense-suffix log model) and state the exponent bound in each band doc; tighten the rest to `10/9` if the pinned readings allow (the run log shows the per-unit ratios within 1% on most bands).
4. Isolation guard (envelopes-a-3): is a `NEXTEST`/`RUST_TEST_THREADS` check wanted, or is `ISOLATION_NOTE` on failure text sufficient given the gate runs nextest? Recommendation: add the guard; it is one function, and the failure it prevents is a silent pass.
5. Public decode rows read roughly twice the kernel copy (`DECODE_DENSE` 120_035 against `SKYLINE_DECODE_DENSE` 61_440; `DECODE_CLIFF` 4_052 against a stream near 1.5 KB) even though `Version::decode` reads into one `Vec` and adopts it. What allocates the second copy's worth of peak at the door (`read_to_end` growth, or the `Bytes` conversion)? A question for the version-core reviewer; if it is growth, the crate doc's "the result reuses the read buffer" describes only half the peak.
6. Should the meter surface expose the generator width constants under the `meter` feature so the closed forms can cite them (envelopes-a-19)? Recommendation: no; keep the oracle's independence and name the widths once in the band module with a comment naming the generator constant each mirrors.
7. If a directory split of `tests/meter.rs` is wanted, `amp_board_smoke::band_tests_and_registry_citations_stay_paired` (reads `tests/meter.rs` by path) and suanpan's `claims.rs` `BANDS` path must move in the same commit. Recommendation: defer until the harness unification lands; the pointer comment at 265-267 dissolves with it.

## Dropped

- [9] dashu-int manifest pin: refuted. The committed `Cargo.lock` is the correct layer for a library's measurement determinism; an exact `=0.5.0` in the workspace manifest would propagate to every downstream consumer. The stale "thin margin" prose leg survives inside envelopes-a-20.
- [10] segments doc rewrite as a tamper pin: the proposed present role ("a nonzero reading means a depth-recursive walk entered the library") is itself false in this binary, since `descend!` does not compile outside `cfg(test)`; merged into envelopes-a-2.
- [23], [32], [48]: duplicates of envelopes-a-1 (header and row docs).
- [21], [31]: duplicates of envelopes-a-1 (transcode prose).
- [29]: `skyline_oracle` ghost name; merged into envelopes-a-1, with the value-leg consequence in envelopes-a-9.
- [22]: duplicate of envelopes-a-11.
- [25], [50]: duplicates of envelopes-a-17.
- [34], [54]: duplicates of envelopes-a-4.
- [35]: duplicate of envelopes-a-15.
- [27], [33], [49]: duplicates of envelopes-a-12.
- [28]: duplicate of envelopes-a-10.
- [17], [46]: duplicates of envelopes-a-2.
- [44]: eq early-exit leg reframed and merged into envelopes-a-22; the accumulator-band leg is envelopes-a-22.
- [42]: merged into envelopes-a-13; [16] (`version_of` alias) merged into envelopes-a-13.
- [37], [52]: merged into envelopes-a-19; [36] duplicate of envelopes-a-19.
- [43]: duplicate of envelopes-a-7; [30], [41] merged into envelopes-a-7.
- [38], [39]: merged into envelopes-a-20; [53] converted (deliberate calibration kept at 500d4d09; the rationale lives only in history) and merged into envelopes-a-20.
- [45]: the tick-row placement is folded into envelopes-a-4 (the docketed unification dissolves the pointer comment); the scanner constraint is open question 7.
- New from the refutation pass, out of partition: the board's segments column (open question 1) and the `rank_bigroot` twin pair (open question 2).
