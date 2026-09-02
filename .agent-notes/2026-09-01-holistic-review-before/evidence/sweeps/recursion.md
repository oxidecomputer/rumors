# Sweep recursion: Recursion and stack-depth audit against the no-depth-recursion rule

Verification-and-finalization pass over the recursion sweep, at commit
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean, verified with
`git rev-parse HEAD` and `git status --porcelain`).

## Method and coverage

The sweep's mechanical basis is a name-resolved call-graph scan of
`crates/before/src` and `crates/suanpan/src` (`callgraph2.py` -> `sccs2.txt`,
524 lines; `classify.py` -> `classified.txt`, 406 candidate self-call lines),
with every candidate then read at its self-call sites and summarized in
`INVENTORY.txt` under `scratchpad/before/sweeps/recursion/`. This pass did not
rerun the scan; it disputed each of the ten reported findings by opening the
cited lines, grepping use sites, and reading the git history and the
before-prefixed agent notes for recorded rationales. Everything below marked
"verified" was checked mechanically in this pass:

- `descend!` use sites: `grep -rn 'descend!'` over both crates finds exactly
  four call sites in `testing/bridge.rs` (all with the literal depth 0), three
  in `version/skyline/grow/tests.rs` (depth threaded), one in
  `meter/tests.rs` (depth threaded), plus prose mentions in `recurse.rs` and
  `query/tests.rs`.
- Writers of the segments counter: the only `fetch_add` on `SEGMENTS_GROWN`
  is `recurse.rs:106`, inside `grow`, which carries `#[cfg(test)]`; the
  macro that reaches `grow` is `#[cfg(test)]` too (`recurse.rs:118`). The
  readers are `meter::stack_segments` and the metered helpers in
  `tests/meter.rs` (lines 364-371, 1212-1221, 1675-1684, 6900-6911),
  `meter/board/measure.rs:84-92`, and `meter/tests.rs`. The board runs from
  `tests/amp_board_smoke.rs`, `benches/board.rs`, and `examples/amp_board.rs`.
  `stacker` is a dev-dependency (`Cargo.toml:44`, under the
  `[dev-dependencies]` header at line 33).
- The envelope table's segments column: a multi-line regex over every
  `envelope(` row in `tests/meter.rs` finds 84 rows, all pinning `0`.
- Shape-door drivers: `grep -rn '\.shape()\|combine(\['` over
  `clock/tests.rs`, `version/tests.rs`, `party/tests.rs`, `tests/`, `benches/`,
  `examples/`, and `src/meter` returns nothing; the only in-crate drivers are
  `shape/tests.rs` and `testing/diff_ops.rs` (proptest scales), and the
  fuzzfit guest drains all three doors (`fuzzfit/guest/src/lib.rs:748, 763,
  778`).
- Public `without` at depth: `tests/meter.rs:6176` and `:6255` call
  `Party::seed().without(..)` on a spine of `ID_DEPTH = 250_000`
  (`tests/meter.rs:94`).
- `BitStack::push_bits` callers: only `PopStack::push` at `stack.rs:221-229`
  (the other `push_bits` hits are `BitsBuf`'s method in `codec/buf.rs`).
- `mint` tally: 57 occurrences across 26 files under `crates/before/src` and
  `crates/suanpan/src` (`.rs` only), plus README, `tests/`, and the fuzzfit
  guest.
- History: `git log` on `recurse.rs`; the message of `1ddb5a483` ("stacker
  becomes a dev-dependency: library walks are iterative, the guard is
  test-surface only"); `git blame` on the three mis-worded comments; the
  segments mentions in
  `.agent-notes/2026-07-22-before-adversarial-resource-amplification/`.
  `.cargo/mutants.toml` mentions neither `recurse` nor segments.

No cargo, just, or test command ran in this pass (the two permitted test
invocations were not needed: no committed test can distinguish the disputed
claims, and the one correctness finding needs a multi-hundred-GiB input).
What this pass could not see: whether the fuzzfit pipeline stages registers
deep enough for its shape drains to count as a depth proof, and whether the
pre-scan bounds a scan span by anything other than the ledger's u32 index
(read `memo.rs` in full; `prescan.rs` only at the cited sites).

## Findings

### recursion-1: Grown-stack-segments meter is constant zero in every build that pins it
- Where: crates/before/src/recurse.rs:100-109 (related: crates/before/src/recurse.rs:16-20, crates/before/src/recurse.rs:68-69, crates/before/src/recurse.rs:118-119, crates/before/src/meter.rs:3543-3559, crates/before/tests/meter.rs:22-26, crates/before/tests/meter.rs:211-215, crates/before/tests/meter.rs:387-391, crates/before/tests/meter.rs:6131-6133, crates/before/src/meter/board.rs:47-49, crates/before/src/meter/board/ceilings.rs:80-83, crates/before/src/meter/board/measure.rs:84-92, crates/before/src/meter/board/judge.rs:286-290, crates/before/src/meter/board/floors.rs:83-85, crates/before/src/meter/tests.rs:392-453)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (cfg attributes read at every item in recurse.rs; single writer confirmed by grep; the 84 envelope rows' segments column read by regex; board drivers located by grep; stacker's dev-dependency placement read in Cargo.toml and stated in commit 1ddb5a483's message); executed: no
- Verification: confirmed; history: deliberate-but-expired (the keep decision at recurse.rs:16-20 was written in 1ddb5a483, the same commit that gated `grow` behind `cfg(test)`, and rests on a "measured fact" the pinning build cannot measure; floors.rs:83-85 records the ceiling-only floor policy but not the cfg gap; the amplification note (lines 1299-1301) still describes segment onset as detectable at scale)
- Owner-gated: yes (the dissolution removes `meter::stack_segments`/`reset_stack_segments`, public functions under the `meter` feature, and a board currency; the alternative keeps them)

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
overflow is the real detector). The doctrine breached: a ceiling over a counter
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

### recursion-5: Memo ledger's u32 link index panics on a feasible input where IdIndex degrades gracefully
- Where: crates/before/src/version/skyline/fill/memo.rs:132-139 (related: crates/before/src/version/skyline/fill/memo.rs:69-97, crates/before/src/version/skyline/fill/prescan.rs:686-694, crates/before/src/party/ops/index.rs:53-57, crates/before/src/party/ops/index.rs:72-75, crates/before/src/party/ops/index.rs:173-178, crates/suanpan/src/accumulator.rs:90-155)
- Class / severity / confidence: correctness / low / medium
- Provenance: assessed (memo.rs read in full; prescan.rs, index.rs, and the Accumulator struct read at the cited sites; no upstream cap on a scan span's site count found, but prescan.rs was not read in full); executed: no
- Verification: reframed: the panic's reachability is set by the link store, not the packed id: 2^32 links need roughly 2^32 times `size_of::<Accumulator>()` bytes (the struct holds an `Option<i128>`, a `Vec<i64>`, two `usize`, and a `BTreeMap`) in addition to the ~2 GiB id and a version with one leaf per site, so on hosts with less heap the `Vec` growth aborts first (the crate's uniform exhaustion mode) and the `expect` fires only on hosts that can hold the store; history: no-rationale-found (memo.rs:134-135 calls the expect "the u32 capacity contract alone"; prescan.rs:691-692 cites a "link-storage contract" that is not derived anywhere)
- Owner-gated: yes (the u32 cell is a committed memory pin, memo.rs:96-97; widening it moves the affected heap envelopes)

A `tick` over a party whose one pre-scan span holds more than `u32::MAX - 1`
nonzero-link left-full sites reaches `set_link`'s `expect`. The input is
large but constructible, and the doctrine's tolerated corner is one costing
infeasible work, not a large allocation. The crate's own `IdIndex` shows the
sanctioned shape for a u32-sized table: past the width it falls back to the
unindexed walk rather than panicking.

Evidence:

       133	    pub(super) fn set_link(&mut self, slot: usize, link: Accumulator) {
       134	        // Push first: the store's length is then provably a valid, nonzero
       135	        // 1-based index (the `expect` is the u32 capacity contract alone).
       136	        self.links.push(link);
       137	        let index = u32::try_from(self.links.len()).expect("site count fits u32");
       138	        self.queue[slot] = NonZeroU32::new(index);
       139	    }

    prescan.rs:
       691	        // Ledger slots are queue indices, capped far below `u32::MAX` by the
       692	        // ledger's own link-storage contract.
       693	        self.slots.pop(&mut self.values) as usize

    index.rs:
        73	        if bits.len() > u64::from(u32::MAX) {
        74	            return IdIndex { bits, rights: None };
        75	        }

Resolution: Either widen the queue cell to `Option<NonZeroUsize>` (re-pin the
affected heap envelopes and the const assert at memo.rs:97), or make the
pre-scan launch a fresh scan when the link store would exceed u32 (a scan-span
cap mirroring `IdIndex`'s unindexed arm), or rule the corner tolerable and
document it at `set_link` with its true reachability (a multi-GiB operand pair
and a link store of 2^32 accumulators), replacing the undefined "link-storage
contract" at prescan.rs:691-692 with that ruling. Acceptance: either no u32
capacity panic remains on the tick path for any decodable input, or the corner
is documented at `set_link` with its reachability and an owner ruling, and
prescan.rs cites that ruling.

Construction: Build a `Party` whose packed id chains 2^32 left-full sites
under one covering site (each site is a both-present tag `11` followed by the
full terminal `00`, then the right child: 4 bits per site, about 2 GiB of id
bits), plus a `Version` with one leaf per site so every site has a nonzero
link; call `Clock::tick`. On a host that can hold 2^32 `Accumulator`s, expect
the panic "site count fits u32" from memo.rs:137; on a smaller host, expect an
allocation abort from `links.push` first. Not run: the input is multi-GiB and
the link store is hundreds of GiB.

### recursion-6: Public shape iterators have no committed deep-input test
- Where: crates/before/src/shape.rs:146-155 (related: crates/before/src/shape.rs:333-338, crates/before/src/shape/tests.rs:24-65, crates/before/src/testing/generators.rs:293-298, crates/before/src/clock/tests.rs:653-664, crates/before/src/version/skyline/shape.rs:44-52, crates/before/src/version/skyline/overlay.rs:317-324, crates/before/fuzzfit/guest/src/lib.rs:742-785)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep for `.shape()` and `combine([` over the deep tests, `tests/`, `benches/`, `examples/`, and the board returned no hits; `shape/tests.rs` and the walk structs read); executed: no
- Verification: reframed: the sweep's companion claim that `Party::without` has no 100k-scale test is dropped (tests/meter.rs:6172-6185 and 6250-6261 drive the public door at `ID_DEPTH = 250_000`); the shape-door gap stands, with the qualification that the fuzzfit guest drains all three doors over whatever registers the fit pipeline stages (lib.rs:748, 763, 778), which is a fit, not a committed depth-100k stack-safety test; history: no-rationale-found
- Owner-gated: no

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

### recursion-7: Doc wording inverts iterative and recursive at three sites
- Where: crates/before/src/party/ops/split.rs:19-19 (related: crates/before/src/party/ops/split.rs:43-44, crates/before/src/party/ops/sum.rs:17, crates/before/src/clock/tests.rs:649, crates/before/src/clock.rs:916-922, crates/before/src/party.rs:757-761, crates/before/src/codec/display.rs:20-23, crates/before/src/version/skyline/text.rs:219-221, crates/before/src/codec/scan.rs:3-5)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (all cited lines read; the Debug impls traced to `codec::write_id` and `skyline::text::render`); executed: no
- Verification: confirmed; history: no-rationale-found (split.rs:19 dates to 32a655438f, clock/tests.rs:649 to 660df29c1e, both before the id walks became iterative)
- Owner-gated: no

split.rs calls the iterative cursor implementation "The recursive form of
`oracle::Party::split`" where its sibling sum.rs says "The cursor form";
clock/tests.rs calls the iterative Debug printer "recursive" (Clock's Debug
prints Party and Version, whose Debug route to the iterative `write_id` and
`text::render`); codec/scan.rs says a walk "recurses through an iterative
loop". Under a hard rule that forbids library recursion, "recursive" beside a
library walk is the word a rule auditor greps for.

Evidence:

    split.rs:
        19	    /// The recursive form of `oracle::Party::split` (the paper's `split`).
        43	/// nonempty, so no collapse can arise). Iterative: the spine walk is a loop, so
        44	/// deep ids cannot overflow.

    sum.rs:
        17	    /// The cursor form of `oracle::Party::sum` (the paper's `sum`/`norm`),

    clock/tests.rs:
       649	    // The recursive Debug pretty-printer must not overflow either.

    codec/scan.rs:
         3	//! Traversal work over the packed bit streams is invisible to every other meter
         4	//! when it allocates nothing (no heap delta), recurses through an iterative
         5	//! loop (no grown segments), and touches no `Base` arithmetic (no limb

Resolution: split.rs:19 -> "The cursor form of `oracle::Party::split`";
clock/tests.rs:649 -> "The Debug printer (an iterative walk) must not overflow
either"; codec/scan.rs:4 -> "walks an iterative loop (no grown segments)".
Acceptance: no library or test comment describes an iterative walk as
recursive.

### recursion-8: BitStack::push_bits carries a dead len == 64 arm; pop_bits's one-level recursion is undocumented
- Where: crates/before/src/codec/stack.rs:58-102 (related: crates/before/src/codec/stack.rs:212-231)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (stack.rs read in full; the only `BitStack::push_bits` callers are `PopStack::push` at 221-229, every `len` at most 63); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`push_bits` documents and debug-asserts `len <= 63`, yet the assert carries a
`len == 64 ||` disjunct that is dead under `len <= 63`, and the body has an
unreachable `if len == 64 { value }` arm. `pop_bits` recurses into itself once
(after the word refill `top_len` is 64 and `rest <= 63`) without saying so, so
a reader auditing the recursion rule has to derive the constant bound.

Evidence:

        58	    /// Push `len <= 63` bits at once, oldest at the value's high end — popping
        61	    fn push_bits(&mut self, value: u64, len: u32) {
        62	        debug_assert!(len <= 63 && (len == 64 || value >> len == 0));
        63	        let total = self.top_len + len;
        64	        if total <= 64 {
        65	            self.top = if len == 64 {
        66	                value
        67	            } else {
        68	                (self.top << len) | value
        69	            };

        95	        let low_len = self.top_len;
        96	        let low = self.top;
        97	        let rest = len - low_len;
        98	        self.top = self.words.pop().expect("bit stack underflow");
        99	        self.top_len = 64;
       100	        let high = self.pop_bits(rest);
       101	        (high << low_len) | low

Resolution: Drop the `len == 64 ||` disjunct and the `if len == 64 { value }`
arm (assert `len <= 63 && value >> len == 0`); at stack.rs:100 either inline
the second pop (it cannot spill again: `rest <= 63 <= 64`) or add a one-line
note that the self-call happens at most once because the refill leaves
`top_len = 64 >= rest`. Acceptance: `push_bits` has no dead 64-bit arm and
`pop_bits` states or removes its one-level self-call.

### recursion-9: "mint" for constructing values and coining terms is a crate-wide idiom
- Where: crates/before/src/meter.rs:18-19 (related: crates/before/src/version/skyline/fill/memo.rs:4, crates/before/src/party/ops/sum_split.rs:166, crates/before/src/version/skyline/query/web.rs:190, crates/before/src/lib.rs:49, crates/before/src/version/skyline/watermark.rs:303, crates/before/src/laws.rs:2956, crates/before/src/meter/registry.rs:20)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rn '\bmint'` over `crates/before/src` and `crates/suanpan/src`, `.rs` only: 57 occurrences in 26 files; further hits in `crates/before/README.md`, `tests/`, and the fuzzfit guest); executed: no
- Verification: reframed: the sweep named three sites; the word is a crate-wide idiom, and one module anchors it to an identifier (`Reign::mint` at web.rs:190, with the module doc's "mint and its one death" vocabulary), so a sweep of the term has to distinguish the anchored uses from the unanchored ones; history: no-rationale-found
- Owner-gated: no

The vocabulary rule for these crates forbids "mint" for constructing a value.
Three sites are quoted below as the sample the sweep saw; the disposition of
the whole idiom belongs to the vocabulary sweep, and this entry exists so the
count is not understated in the record.

Evidence:

    meter.rs:
        18	//! The generators themselves are private: every instrument mints its shapes
        19	//! through the family registry ([`registry`])

    memo.rs:
         4	//! A left-full site (minted in [`fill`](super)'s module doc: an id node whose

    sum_split.rs:
       166	#[allow(clippy::type_complexity)] // two optional bit ranges: an inline pair over a minted name

    web.rs:
       190	    fn mint(sign: Sign, offset: &Base, epoch: u32) -> Reign {

Resolution: Sweep the 57 sites with the vocabulary pass: replace the
construction sense ("builds", "creates", "constructs") and the coining sense
("defined in", "named in"), keeping any use that is anchored to an identifier
the module defines (`Reign::mint`) only if the owner wants that identifier
kept. Acceptance: no unanchored "mint" in the two crates' prose.

### recursion-10: Oracle prose says "boxed" trees and clones; the trees are Arc and clones are refcount bumps
- Where: crates/before/src/oracle.rs:16-18 (related: crates/before/src/oracle/party.rs:7-14, crates/before/src/oracle/version.rs:32-39, crates/before/src/testing/exhaustive.rs:51-53)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (oracle.rs, oracle/party.rs, oracle/version.rs read at the cited lines; exhaustive.rs:44-60 read); executed: no
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

oracle.rs describes "the derived `Drop` of the boxed trees" and "the boxed
representation", and exhaustive.rs prices "the oracle's boxed clones", while
both oracle types hold children behind `Arc`, whose derived `Clone` is a
refcount bump by the types' own docs. `Drop` of an `Arc` spine does still
recurse, so oracle.rs's envelope claim stands; the exhaustive.rs pricing
sentence attributes cost to deep copies that do not happen (the cost there is
the result trees' node allocations).

Evidence:

    oracle.rs:
        16	//! Small-scope, bounded-depth inputs only. Every traversal here — the ops, the
        17	//! derived `Drop` of the boxed trees, `Clone` — recurses on native stack
        18	//! frames, and the boxed representation crawls a long spine pointer by pointer;

    oracle/party.rs:
         7	/// Children sit behind [`Arc`] so the derived [`Clone`] is a refcount bump: the
        14	    Node(Arc<Party>, Arc<Party>),

    exhaustive.rs:
        51	//! (`join`, `without`) run at the small bound only: each builds a result tree
        52	//! per pair — plus the oracle's boxed clones and the lowering for the
        53	//! structural compare — which prices at roughly seven eighths of the pair

Resolution: oracle.rs:17-18 -> "the derived `Drop` of the `Arc`-linked trees"
and "the linked representation"; exhaustive.rs:52 -> "the oracle's result
trees" (or attribute the measured cost to the `Version::node` allocations of
each pair's result). Acceptance: oracle prose names the `Arc` representation
and does not attribute deep-copy cost to `Clone`.

## Positives

- The hard rule holds: the sweep's call-graph scan over both crates finds no
  library function recursing on input tree depth, and this pass's reading of
  every candidate agrees. The three library self-calls that exist are
  constant (`BitStack::pop_bits`, one level, stack.rs:87-102), type-level
  (`TryFrom<(u64, T, S)>` for `Version`, `PartyLiteral` for tuples), or
  log2-of-size in the meter-feature generators.
- Every explicit stack in library code states its per-level cost at its
  declaration: `LeafCursor` and `IdLeafCursor` ("Root-to-leaf branch
  directions" as `BitStack`s, overlay.rs:317-324, 471-481), `IdLeafCursor` in
  diff.rs:245-255, the `sum` frames ("two or three bits on a bit stack",
  sum.rs:17-20), `write_id` ("one to two bits per open node on a bit stack",
  display.rs:20-23), the text emitter ("one phase bit per open node, no
  recursion", text.rs:219-221), and `PopStack` ("Each entry costs `2·w` bits",
  stack.rs:187-195). Exhaustion is uniformly `Vec` growth to allocation
  failure.
- The deep-input proof is broader than AGENTS.md:37 advertises: three
  depth-100k clock tests (`deep_tree_stack_safety`,
  `deep_tree_query_and_causal_stack_safety`,
  `deep_tree_text_and_min_ticks_stack_safety`, clock/tests.rs:565-761)
  covering the clock ops, the query/causal/span surface, and the text mirrors;
  `codec::tests::deep_id_text_roundtrip` for the id text parser (cited at
  clock/tests.rs:734); the id diff ladder at `SCALES = [256, 4096, 100_000]`
  (party/tests.rs:830); and the `tests/meter.rs` envelope rows at `ID_DEPTH =
  250_000`, including `covers` and the public `without` (tests/meter.rs:6135,
  6250).
- The settle's mass-balanced reduction keeps its control stack explicit and
  pins its depth bound `2 * total.ilog2() + 2` against a recursive reference
  over a proptest of the split rule (query/tests.rs:1342-1379): an instrument
  that pins the order, not an envelope.
- Every in-crate oracle-facing harness carries a stated cap: `ARB_DEPTH = 4`
  (generators.rs:298), `ORACLE_SCALE_MAX = 4096` with the ladder's top
  deliberately beyond it (party/tests.rs:832-834), the leaf-count cap on the
  split-depth proptest (query/tests.rs:1345), and the text-parser reference's
  small-scope note (codec/tests.rs:1640-1642).
- Where `descend!` is used as documented, the design is right: it guards the
  descent rather than the body (recurse.rs:22-28), the `STRIDE`/`RED_ZONE`
  relation is derived from a frame-size measurement (recurse.rs:41-51),
  `meter/tests.rs:407-439` proves at depth 200k that the guard grows the
  stack, and `grow/tests.rs:125-180` threads a real depth through it.
- `party/forks.rs:53-54` states the reason its `Split` is iterative ("The
  recursion of `Split` made iterative, so a huge `count` cannot overflow the
  call stack").

## Open questions for Finch

1. Segments currency (recursion-1): dissolve it from `tests/meter.rs` and the
   board outright, naming the depth-100k/250k tests as the no-recursion
   detector, or keep it and re-document it as a pin on the test-surface guard
   only? Either is consistent; today's prose claims a measurement the meter
   build cannot make, and the dissolution removes public `meter`-feature
   functions. Recommendation: dissolve; the depth tests already fail on
   overflow, which is the only signal a native-recursion regression emits.
2. Bridge guard policy (recursion-2): given every bridge input is bounded by
   the oracle's own recursive `Drop`, should the ev-side `descend!` calls stay
   (guarding heavier `Base`-arithmetic frames than the oracle's `Drop` frames)
   or go? If they stay, the id walks should be guarded the same way with real
   depths threaded. Recommendation: go, with the bound stated at bridge.rs.
3. Memo link index (recursion-5): keep the u32 cell as a memory pin with the
   corner documented and ruled at `set_link`, or make `tick` total over every
   decodable input? Recommendation: document and rule; the corner's cost is a
   link store of 2^32 accumulators, and the ruling replaces an undefined
   "link-storage contract" in prescan.rs with a real one.
4. Recursion inventory (recursion-4): a mechanically checked roster
   (surface-scan flagging self-recursive functions outside `descend!`), or the
   rule stated once with its bound classes? Recommendation: the rule; the
   roster's only stable content is the three `descend!` users, which a grep
   finds.

## Dropped

- The sub-claim of the sweep's finding [5] that `Party::without` appears in no
  100k-scale test: `tests/meter.rs:6172-6185` and `6250-6261` drive the public
  door at `ID_DEPTH = 250_000`; the finding survives as recursion-6 for the
  shape iterators alone.
- The sweep's count of three "mint" sites in finding [8]: the grep finds 57
  occurrences in 26 source files, one of them anchored to `Reign::mint`; the
  finding survives as recursion-9 with the tally and is deferred to the
  vocabulary sweep for disposition.
- The sweep's `owner_gated = false` on finding [0]: dissolving the segments
  currency removes `meter::stack_segments`/`reset_stack_segments`, public under
  the `meter` feature, so recursion-1 is marked owner-gated.
