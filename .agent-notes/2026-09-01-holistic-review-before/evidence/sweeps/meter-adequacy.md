# Sweep meter-adequacy: Adequacy of the resource instruments: envelopes, board, bench judge, fuzz-fit bands, wasm32 pins, coverage pins

## Method and coverage

Verification pass over the twelve findings of the meter-adequacy sweep at
commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean,
confirmed with `git rev-parse HEAD` and `git status --porcelain`).

Mechanically:

- Opened every cited site with line-numbered `awk` reads and quoted from
  those reads; every excerpt below is verbatim from the file at this tip.
- Recounted the fuzz-fit coverage: `grep -o 'pub extern "C" fn ff_*'` over
  `crates/before/fuzzfit/guest/src/lib.rs` (107 exports) against
  `grep -o 'kernel: "ff_*"'` over `harness/src/bands.rs` (44 distinct
  kernels; 53 band lines including `SMALL_BANDS`), `comm -23` for the
  difference (63 exports, of which 7 are register-machine plumbing or the
  instrument self-test: `ff_nop`, `ff_reset`, `ff_regs_reserve`,
  `ff_stage_len`, `ff_stage_prepare`, `ff_stage_ptr`,
  `ff_selftest_quadratic`; 56 are measured public-operation kernels).
  Counted the `Op` enum in `harness/src/ops.rs`: 44 variants. Counted
  distinct `ff_*` kernels named in `crates/before-fuelscape/src/ops.rs`: 97.
- Grepped `tests/meter.rs`, `src/testing/asymptotics.rs`, the fuzz-fit
  harness, and `src/shape/tests.rs` for any reference to `shape()`,
  `shape::combine`, or the counters; none.
- Reproduced `judge.rs::trend`'s least-squares fit in Python on synthetic
  `n log2 n` ladders at the board's sizes (see finding 3).
- Read the git history the findings rest on: `46eb64f9` (shape surface),
  `1f829fc7b`, `54fdf8b53`, `2732a53ae` (guest span/query/rank kernels),
  `b49424614`, `c95230c8`, `ef8894ac`, `a066a8e9` (bench roster),
  `d2a9d04e` (dated-notes excision), `cc84df7e1` (registry `decided`).
- Read `.agent-notes/2026-07-26-before-fuzzfit-asymptotics` (scope
  paragraph), `.agent-notes/2026-08-13-before-fuelscape-rustdoc` (shape
  mentions), `crates/before/AGENTS.md`, and the `.cargo/mutants.toml`
  header for recorded rationales.
- Ran no cargo, just, bench, or test command (0 of the 2 permitted test
  invocations used: no finding turned on a runtime fact an existing test
  could settle).

Not seen: the bench judge's current SKIP set and the board's version_eq
readings (both need a run); `tests/meter.rs` was read at the cited sites
and their surrounding module docs, not end to end; the design document
`design/version-skyline-iterator.md` named in `46eb64f9`'s message is not
in the tree (only `design/rumors-frame-fuzz.md` exists), so no shape-walk
instrumentation rationale beyond the NA reason strings was found.

## Findings

### meter-adequacy-1: Public shape walks carry linear-cost claims with no enforcing instrument
- Where: crates/before/src/meter/board/coverage.rs:610-633 (related: crates/before/src/version.rs:734-740, crates/before/src/party.rs:476-481, crates/before/src/clock.rs:684-691, crates/before/src/shape.rs:314-317, crates/before-fuelscape/src/ops.rs:380-406, crates/before-fuelscape/src/lib.rs:16-20, crates/before/fuzzfit/guest/src/lib.rs:742-800, crates/before/src/lib.rs:344-358, crates/before/src/meter/board/coverage.rs:271-278)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep over tests/meter.rs, src/testing/asymptotics.rs, fuzzfit/harness, src/shape/tests.rs for shape()/combine/counters: no hits; read of the NA table, the four rustdocs, the fuelscape lib doc and OpSpecs, the guest exports); executed: no
- Verification: confirmed; history: no-rationale-found (the shape surface landed in 46eb64f9 on 2026-08-19; the design document its message cites is not in the tree; the fuelscape panels followed in 3c64f08a; nothing recorded excuses the walks from enforcement beyond the NA strings)
- Owner-gated: no (the enforcement home is a board row group or fuzz-fit ops; either is instrument work, not API)

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
- Verification: confirmed with corrected counts (56 measured public-operation kernels unbanded, not 63; the other 7 unbanded exports are plumbing and the self-test burner); history: no-rationale-found (the span/query/rank/n-ary/Eq-Hash kernels were added for fuelscape panels in 1f829fc7b, 54fdf8b53, and 2732a53ae; those messages decide fuelscape exemptions, never fuzz-fit band membership; the fuzz-fit design note's scope paragraph concerns operand construction, not the operation vocabulary)
- Owner-gated: yes for extending the program vocabulary (budgets, strategies, re-pin); no for the justfile wording and a surface-tiling test

`bands.rs` states its own reach honestly: 44 kernels, 49 band keys. The
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

### meter-adequacy-4: The bench judge's judged set is unpinned: any unrostered cell may leave judgment as SKIP with no diff
- Where: tools/benchjudge:82-85 (related: tools/benchjudge:157-160, tools/benchjudge:271-285, tools/benchjudge:537-546, tools/benchjudge:845, crates/before/src/meter/board.rs:106-113, crates/before/src/meter/board/floors.rs:99-112, crates/before/tests/bench_judge_roster.rs:72-83)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read of `roster_violations`, the SKIP row construction, the self-test assertion at 845, the roster schema pin); executed: no
- Verification: reframed: the mechanism is as the sweep states, but the free GREEN/SKIP drift is a documented design choice grounded in noise near the 10 µs floor, and an exact pin of the SKIP set would put a threshold over a noisy quantity, which the owner's doctrine forbids; severity lowered from medium because a regression drives a cell's median up into judgment, not out of it, so the exposure needs a cell to first leave judgment (speedup, family edit) and then regress while staying under 10 µs at the hi scale; history: deliberate-and-holds for the drift rule (benchjudge:82-84 states its reason), no-rationale-found for leaving the expected sub-floor set as prose
- Owner-gated: no

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
pinned in `tests/bench_judge_roster.rs`) listing the cells honestly under
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
- Provenance: verified (grep of `_reads_superlinear` across before and suanpan: ten kernels, none for the weight comb, freeze parade, or tooth-tail mechanisms; suanpan's only cargo feature is `touch-meter`, so no committed knob disables certificate consumption, the write watermark, or exact-top maintenance); executed: no
- Verification: confirmed and extended: the sweep named two bands; the comparison-sweep tooth-tail band (4737-4747) rests on the same "local probe build" prose, and the module comment at 4480-4495 states the pattern for all three; history: no-rationale-found (the module comment says "the probe readings live in the pin commits", which records numbers, not a re-runnable kernel)
- Owner-gated: yes (committing a probe path means a test-only accumulator strategy inside suanpan, a design decision for that crate)

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

### meter-adequacy-8: The bench roster's `red` class is documented as a buffer for owned reds awaiting cures, a shape the roster outgrew
- Where: tools/benchjudge:65-69 (related: tools/benchjudge-expected.json:2, crates/before/tests/bench_judge_roster.rs:45-63, crates/before/tests/bench_judge_roster.rs:47, crates/before/src/meter/board/export.rs:117, crates/before/src/meter/board.rs:174-178)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (read of the docstring, roster notes, and membership pin; `git log -p --follow` on the roster shows the red set once held fifteen bigroot sweeps, the hugeleaf display pair, and `version_distance/jump-pair`, all removed by c95230c8 and ef8894ac; the docstring's phrase dates from b49424614, when those reds were real); executed: no
- Verification: confirmed; history: deliberate-but-expired (the class was built to hold standing reds while cures landed; every cure has landed and the roster's own notes and a066a8e9 now describe the survivors as declared models, yet the judge's docstring still frames the class as a queue). Also a ghost name: bench_judge_roster.rs:47 says "the board-red riders" while the constant is `BOARD_DECLARED_BENCH_RIDERS` (renamed per a066a8e9, "the rider const's new name")
- Owner-gated: no

The board side says red means untriaged and nothing else. The judge's
roster mode is the one place in the apparatus where a RED verdict is
recorded as expected, and its docstring says the mechanism exists "so `just
all` stays meaningful while owned reds await their cures". Today the class
holds exactly the schoolbook tripwire, a designed known-bad kernel, and the
membership pin is what keeps the class from becoming a queue again.

Evidence:

    tools/benchjudge
        65	Roster mode (`--roster FILE`): the committed expected-verdict roster —
        66	the board-side sibling of the gate's expected-failure test roster, same
        67	membership-by-name philosophy — makes the judge enforce a *fixed* verdict
        68	map instead of all-green, so `just all` stays meaningful while owned reds
        69	await their cures. The file pins the configuration it was recorded under

    crates/before/tests/bench_judge_roster.rs
        47	/// Every other cell — the designed diagonal, the board-red riders, and

Resolution: rename the class to what it holds (`tripwire`), or route the
schoolbook cell through the existing `--expect-red` path in its own
invocation so the roster needs no red class; re-state benchjudge:65-69 and
the roster notes so no text describes reds awaiting cures; fix "board-red
riders" at bench_judge_roster.rs:47 to the constant's name. The self-test's
laundering pins (benchjudge:857-884) hold under either name. Acceptance:
no prose in the judge or roster describes a queue of expected failures.

### meter-adequacy-9: The denominator sidecar is stamped before any bench runs and criterion's estimates carry no stamp
- Where: crates/before/benches/board.rs:159-177 (related: crates/before/benches/common/sidecar.rs:149-153, tools/benchjudge:47-55, tools/benchjudge:396-411, justfile:840-846, crates/before/fuzzfit/harness/src/wasm.rs:99-108, crates/before/fuzzfit/harness/tests/enforce.rs:319-326, crates/before/wasm32-pins/harness/src/lib.rs:41-48)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (read of the harness ordering, `write_denoms`, `read_median`, the recipe, and the guest path resolution); executed: no
- Verification: confirmed; the early write has a stated reason (sidecar.rs:149-152: the parent directory must exist before criterion creates it), which is a directory-creation reason, not an ordering requirement; the `bench-judge` recipe stops at the first failing line, so within one recipe a failed bench never reaches the judge; only manual invocations are exposed; history: no-rationale-found for the ordering itself
- Owner-gated: no

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

### meter-adequacy-10: The meet-fold band's liveness floor states one touch per operand byte with no derivation
- Where: crates/before/tests/meter.rs:8168-8197 (related: crates/before/tests/meter.rs:36-49, crates/before/tests/meter.rs:8143-8161, crates/before/tests/meter.rs:8687-8700)
- Class / severity / confidence: test-quality / nit / medium
- Provenance: verified (read of the band, its module comment, the suite's genre rule, and the tick floor's derivation); executed: no
- Verification: confirmed; the module comment (8143-8161) explains why the sweep walks both streams whole but not why every operand byte forces an accumulator touch; the tick floor at 8690-8700 shows the genre's expected form (one touch per 64-bit limb, hence one per eight input bytes); history: no-rationale-found
- Owner-gated: no

The suite distinguishes derived liveness floors (irreducible work) from
measured x0.75 tripwires and says the distinction matters because their
trips mean opposite things. The meet-fold floor is labelled a liveness
floor but states no mechanism for one touch per byte, so a reader cannot
tell whether it is irreducible or observed.

Evidence:

      8192	        assert!(
      8193	            run.touches >= run.bytes,
      8194	            "meet fold at {bytes} operand bytes: {} digit touches under \
      8195	             the one-per-byte floor: the fold's accumulator work is not metered",
      8196	            run.touches,
      8197	        );

Resolution: derive the floor from the carrier's nonzero deltas times the
first-level merges (the stagger bands' idiom) or relabel it a measured
tripwire with the reading of record in the pin commit. Acceptance: the doc
at 8172-8174 names the mechanism or the genre.

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

### meter-adequacy-12: Registry and surfacecheck rulings carry `decided` dates whose only consumers are format tests
- Where: crates/before/src/meter/registry.rs:1073-1097 (related: crates/before/src/meter/registry.rs:1102-1103, crates/before/src/meter/registry/tests.rs:204-226, crates/before/surfacecheck/src/check.rs:33-34, crates/before/surfacecheck/src/check.rs:259-272, crates/before/src/meter/board.rs:170-172, crates/before/src/meter/board/ceilings.rs:233-235)
- Class / severity / confidence: scaffolding / nit / high
- Provenance: verified (grep for `decided` consumers: the registry format test and surfacecheck's `dated` closure only; grep for dates in board.rs and ceilings.rs: none remain); executed: no
- Verification: confirmed; history: already-known (d2a9d04e's message reports "the enforced dated-exception machinery (surfacecheck Exception.decided + its YYYY-MM-DD validation and tests, registry Coverage/Bands decided fields + REGISTRY_RATIFIED enforced by envelope_only_rulings_are_dated) requires field and test changes to dissolve — a design round, not a prose sweep"). The same excision left board.rs:171 and ceilings.rs:234 promising "a dated owner rationale" at declaring constants that carry no dates now
- Owner-gated: yes (the field was a deliberate convention; dissolving it is the design round d2a9d04e deferred)

Evidence:

      1075	    EnvelopeOnly {
      1076	        /// Why this family earns no column (the board-roster criterion answer).
      1077	        reason: &'static str,
      1078	        /// The date of the ruling of record.
      1079	        decided: &'static str,
      1080	    },

    crates/before/src/meter/board.rs
       170	//! Some cells are judged against a **declared model** — a ratified cost law
       171	//! derived at the cell, with a dated owner rationale committed at the declaring
       172	//! constant — in place of one global ceiling, because the global form is

Resolution: the design round d2a9d04e deferred: drop `decided` from
`Coverage::EnvelopeOnly`, `Bands::Unbanded`, and surfacecheck's `Exception`,
delete `REGISTRY_RATIFIED` and the two format tests, and let `git log -S`
on the reason strings carry the dates; re-state board.rs:171 and
ceilings.rs:234 as "an owner rationale" (the dates are already gone).
Acceptance: no `decided` field remains and no prose promises a date.

## Positives

Verified directly in this pass (the sweep's other positives about
`currency.rs`, the coverage tiling test bodies, and `board/tests.rs`
tripwires were not re-read here and are carried as the sweep's, not mine):

- The bench judge's self-test pins its whole exit contract, including the
  two ceiling-laundering attacks (benchjudge:857-884): a roster cannot
  select a ceiling class, and moving the schoolbook cell between roster
  classes cannot lower its bar. The roster schema pin
  (bench_judge_roster.rs:72-83) refuses any expectation vocabulary beyond
  `red`, so an exemption class cannot appear by edit.
- `MIN_JUDGED_MEDIAN_NANOS` is derived, not calibrated (benchjudge:157-160:
  timer error times a stated dominance factor), which is the right shape
  for a floor over a noisy quantity.
- `tests/superlinear_tripwires.rs` binds every committed known-bad kernel
  by name in both directions, with its module doc stating exactly the
  failure it closes (a kernel that binds nowhere is silently deletable).
- The asymptotics module states its floor discipline up front
  (asymptotics.rs:12-14: floors midway between the linear reference and
  the reading, both exact counters) and prints the linear reference beside
  every run; finding 5 is one missing assert away from the stated design.
- `floors.rs:99-112` discloses the four cells no leg watches by name and
  bounds the exposure by mechanism rather than leaving it silent.
- Fuelscape's Eq/Hash panels were deliberately built on equal pairs so the
  compare runs its whole length (2732a53ae); the board's version_eq row
  (finding 7) is the one place that convention did not reach.
- The dated-notes excision (d2a9d04e) reported the `decided` machinery as
  out of scope in its own message instead of half-dissolving it; finding
  12 is that report, still open.
- `.cargo/mutants.toml`'s header states the campaign configuration of
  record and the disposition ladder, and names the instruments that kill
  mutants deliberately absent from the roster.

## Open questions for Finch

1. Shape walks (finding 1): board row group or fuzz-fit ops? The board
   gives the scan floor and the ladder for free; fuzz-fit gives coverage of
   shapes nobody chose. Recommendation: board rows first (the operands
   exist), fuzz-fit ops when the vocabulary next grows.
2. Fuzz-fit vocabulary (finding 2): is the 44-kernel scope a standing
   decision (in which case the justfile wording is the whole fix) or the
   state the atlas panels outran? The commit history records the panels
   growing without a band decision either way.
3. Probe kernels (finding 6): is a test-only "naive scan" strategy in
   suanpan acceptable, or should the three bands drop the "adequacy
   witness" wording?
4. The `decided` design round (finding 12): d2a9d04e deferred it; is it
   wanted now?
5. Carried from the sweep, not re-examined here: family adequacy for the
   three largest declared constants (ascend-cliff tick and min_ticks heap,
   the weave search allowance); whether the wall-time leg should leave a
   committed "last judged at tip" record; whether the hugeleaf parse trio
   should carry its own text ceiling; whether the `causally & conjunction`
   hole axis is bounded by construction.

## Dropped

None dropped. Two reframed: the bench-judge SKIP finding (now finding 4)
lowered from medium to low and its resolution reshaped so it does not pin
a noisy threshold; the version_eq finding (now finding 7) raised from
medium to high confidence once the stream layout showed the divergence
point exactly. One extended: the probe-build finding (now finding 6)
covers three bands, not two. One corrected: the fuzz-fit count is 56
measured public-operation kernels unbanded, not 63 (7 exports are plumbing
and the self-test burner).
