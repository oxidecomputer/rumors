# Sweep module-graph: Module dependency graph, layering, and the production/instrument boundary

## Method and coverage

This pass disputes the module-graph sweep's twelve seeded findings against the tree at
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean, verified with `git rev-parse HEAD`
and `git status --porcelain`). Everything was read with line numbers (`cat -n`, `awk` ranges);
every excerpt below is verbatim from the cited lines.

Mechanical checks performed:

- Writers and readers of the stack-segments counter: `grep -rn` for `descend!`, `recurse::grow`,
  `segments_grown`, `stack_segments`, `SEGMENTS_GROWN`, and `Currency::Segments` over
  `crates/before/{src,tests,benches,examples}`. The only `descend!` sites outside `recurse.rs` are
  `src/testing/bridge.rs:56,57,150,151`, `src/version/skyline/grow/tests.rs:125,172,177`, and
  `src/meter/tests.rs:417`, all compiled only under `cfg(test)`.
- Envelope rows in `crates/before/tests/meter.rs`: `pub const ...: Envelope` 15, `SweepEnvelope` 31,
  `QueryEnvelope` 33, `TouchEnvelope` 5 (84 rows); the second argument of every row's constructor
  call is `0`.
- History: `git log -S'descend!'` over the non-test source, `git show` of 05bd2b16d (the fill-walk
  conversion), 22cdfbe1 (the `implementation` module's retirement), e4d4817f1 and 76579a3c (the two
  traffic counters), 5c990b9069 (the law-group macro), 81048ec3d (`touch_ops`); `git blame` on
  `recurse.rs:71-86`, `tests/meter.rs:22-26`, `before-fuelscape/Cargo.toml:23-26`, `laws.rs:107-108`.
- The workspace resolver: the root package declares `edition = "2024"` (Cargo.toml:86), so
  dev-dependency features unify onto a package only when its test targets build; finding 2's
  mechanism depends on this.
- The sweep's cycle list: each closing edge read at its cited line (the `use` statements quoted in
  finding 10).

No cargo, just, or test command was run. None of the surviving findings turns on a runtime outcome:
each is a `cfg` attribute, a recipe line, a manifest comment, or a prose/code contradiction, and
another review workflow may be building in this workspace.

What this pass could not see: whether `cargo clippy -p before --lib -- -D warnings` is clean today
(not permitted here); the sweep's full edge list (`modgraph.py` output) was taken as reported and
only the cycle-closing edges were re-read.

## Findings

### module-graph-1: The stack-segments column has a writer in exactly one binary; 84 envelope ceilings and the board's segments currency read a counter nothing can increment
- Where: crates/before/src/recurse.rs:100-109 (related: crates/before/src/recurse.rs:16-20, crates/before/src/recurse.rs:68-86, crates/before/src/recurse.rs:118-129, crates/before/src/lib.rs:421, crates/before/src/meter.rs:3543-3559, crates/before/tests/meter.rs:22-26, crates/before/tests/meter.rs:211-243, crates/before/tests/meter.rs:364-391, crates/before/src/meter/board/measure.rs:84-92, crates/before/src/meter/board/ceilings.rs:80-83, crates/before/src/meter/board/judge.rs:60-64, crates/before/src/meter/board/judge.rs:286-290, crates/before/src/meter/board/worst.rs:169, crates/before/src/meter/tests.rs:400-454, crates/before/Cargo.toml:49)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (writer/reader grep over src, tests, benches, examples; 84 envelope rows counted by kind and their second arguments extracted, all 0; blame and `git show` for the history); executed: no
- Verification: reframed: the counter is not dead everywhere. In the lib unit-test binary `cfg(test)` is set, `descend!` and `grow` exist, and `stack_segment_meter_counts_deterministically_and_resets` both drives the counter (meter/tests.rs:409-439) and holds `tick` on a deep spine to zero (meter/tests.rs:441-453); that is the one live ratchet. In the integration envelope suite (`tests/meter.rs`), the `amp_board` example, and the board bench, the library is compiled without `cfg(test)`, so `grow` and `descend!` do not exist and `SEGMENTS_GROWN` has no writer; history: deliberate-but-expired: 05bd2b16d chose to keep the counter compiled for the meters ("its own liveness now witnessed by a test-local guarded descent"), but `recurse.rs:74-76` (blame 5e166477c, older than the conversion) and `tests/meter.rs:22-26` (blame 0d1ea4905, older) still describe a growth arm that writes the counter in those builds.
- Owner-gated: yes: `meter::stack_segments` and `meter::reset_stack_segments` are `pub` under the `meter` feature, and `recurse.rs:16-20` records the keep decision as an owner ruling.

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

Resolution: Two honest dispositions, one of which the owner picks. (a) Retire the currency where it
cannot move: drop the `segments` field and column from `tests/meter.rs`'s four envelope kinds,
remove `Currency::Segments` and its ceiling, floor-trip string, judge arms, and worst-map arm from
the board, keep `stack_segment_meter_counts_deterministically_and_resets` as the guard's own unit
test and the tick ratchet, make `mod recurse` `#[cfg(test)]`, and remove `meter::stack_segments`
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

### module-graph-3: The `meter` feature compiles two hot-path atomic counters into bench builds, against the manifest's stated policy
- Where: crates/before/Cargo.toml:60-68 (related: crates/before/Cargo.toml:49, crates/before/Cargo.toml:79-84, crates/before/src/version/hull_traffic.rs:16-21, crates/before/src/version/hull_traffic.rs:45-58, crates/before/src/version/hull_traffic.rs:105-118, crates/before/src/version/skyline/web_traffic.rs:19-24, crates/before/src/version/skyline/web_traffic.rs:46-57, crates/before/src/version/skyline/web_traffic.rs:100-113, crates/before/src/version.rs:968-999, crates/before/src/version/skyline/watermark.rs:962-984, crates/before/src/version/skyline/pool_traffic.rs:27-60, crates/before/src/meter.rs:75-81, crates/before/src/meter.rs:3646-3675, crates/before/src/meter/tests.rs:578-616, crates/before/src/version/skyline/fill/tests.rs:459-468, crates/before/src/version/skyline/fill/tests.rs:597-600, crates/before/tests/meter.rs:9208, crates/before/tests/meter.rs:9244-9250, crates/before/benches/board.rs:81-84, Cargo.toml:117-122, src/tree/tests.rs:1414-1416)
- Class / severity / confidence: modularity / low / high
- Provenance: assessed (read every gate and call site of both counters, the self-dev-dependency, the bench imports, and the introducing commits e4d4817f1 and 76579a3c); executed: no
- Verification: confirmed, with one refinement on the idiom claim: `hull_traffic.rs:16-17` and `web_traffic.rs:19-20` call their gate "the `codec::scan` counter's idiom", but `codec::scan` is gated under `scan-meter`, a counter feature kept out of bench builds; the shim shape matches, the gate does not; history: no-rationale-found: e4d4817f1 introduced the rung counters "under the meter feature" without addressing the bench-build policy, and 76579a3c followed "the hull_traffic idiom".
- Owner-gated: yes: `SpanTraffic`, `EmitTraffic`, `span_traffic`, `emit_traffic`, and their resets are `pub` under `meter`.

The `meter` feature is documented as exposure (generators, counter readers, the surface roster,
`encoded_bits`), and the `limb-meter` comment states the policy: a counter that adds a relaxed
atomic bump to a production path lives behind an additive counter feature and stays out of the
bench builds. `hull_traffic::record` (one bump per pair-hull call in `Version::span_refs`) and
`web_traffic::record` (one bump per priced-offset domination read in `MinWeb::emit_offset`) are
gated on `feature = "meter"`, which the self-dev-dependency lights for every bench and test build.
The board and amplify benches therefore time a span ladder and an emit path carrying one more
atomic per call than the shipped library.

Evidence:

        60	# Exposes the adversarial input generators and deterministic resource meters
        61	# (`src/meter.rs`) as `before::meter`, so the metering test binaries can build
        62	# worst-case inputs and read the counters they envelope, and the public
    --- Cargo.toml:80-83 (the policy, stated at limb-meter) ---
        80	# sees them. Additive and off by default: it adds one relaxed atomic bump per
        81	# `Base` operation, so it stays out of the bench builds (deliberately not in the
        82	# self-dev-dependency above) and is lit by `--all-features` runs and the
        83	# metering envelopes that assert it.
    --- hull_traffic.rs:112-118 ---
       112	#[inline(always)]
       113	pub(crate) fn record(rung: Rung) {
       114	    #[cfg(feature = "meter")]
       115	    counter::record(rung);
       116	    #[cfg(not(feature = "meter"))]
       117	    let _ = rung;
       118	}
    --- version.rs:970-971 ---
       970	        if codec::canonical_eq(&a.0, &b.0) {
       971	            hull_traffic::record(Rung::Equal);
    --- watermark.rs:966 ---
       966	                    web_traffic::record(web_traffic::Decision::DominatedAbove);

Resolution: Either move both counters' `mod counter`, their `record` bodies, and the `meter::*_traffic`
readers under a counter feature (`scan-meter`, whose contract already reads "one relaxed atomic
bump per primitive, out of the bench builds", or a new `traffic-meter = ["meter"]`), mirroring how
`pool_traffic` rides `limb-meter`; or amend the `meter` feature comment to name the two bumps it
adds to production paths. Costs to name for the first option: the unit-test readers at
`meter/tests.rs:578-616` and `fill/tests.rs:459-468, 597-600` run under `cfg(test)` with no
feature gate, so the counter gate becomes `any(test, feature = "scan-meter")` or those tests gain a
cfg; the integration reader at `tests/meter.rs:9244-9250` already sits under `limb-meter` (line
9208); the rumors consumer reads `span_traffic` under its own `meter = ["before/limb-meter",
"before/scan-meter"]` (Cargo.toml:122; src/tree/tests.rs:1414), so it keeps compiling under either
counter feature (the dependence sweep should confirm). Acceptance: the `meter` feature comment and
the set of production-path bumps it compiles agree.

### module-graph-4: Limb-meter taps are spelled three ways and their module path five ways
- Where: crates/before/src/codec/base.rs:10-14 (related: crates/before/src/codec/base.rs:92-93, crates/before/src/codec/base.rs:142-143, crates/before/src/codec/base.rs:167-168, crates/before/src/codec/gamma.rs:259-262, crates/before/src/codec/dsi.rs:282-285, crates/before/src/version/rank/num.rs:58-69, crates/before/src/version/skyline/query/integral.rs:366-376, crates/before/src/version/skyline/query/integral.rs:387-393, crates/before/src/version/skyline/query/integral.rs:414-420, crates/before/src/codec/base/limb_metered.rs:1-54, crates/before/src/codec/base/limb_meter.rs:29-69, crates/before/src/codec/scan.rs:24-59, crates/before/src/codec.rs:39-40, crates/before/src/version/skyline/pool_traffic.rs:20-21, crates/before/Cargo.toml:115-117)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (grep of every `limb_meter::` and `limb_metered::` reference outside tests; each site read); executed: no
- Verification: confirmed, and one cost added: the same counter is reached as `limb_meter::` (base.rs), `super::base::limb_meter::` (gamma.rs:262), `super::limb_meter::` (dsi.rs:285, through the gated re-export at codec.rs:39-40), `crate::codec::limb_meter::` (integral.rs:370-372, 390, 417), and `crate::codec::base::limb_meter::` (rank/num.rs:66); history: no-rationale-found for the split beyond `limb_metered.rs` needing `Base` from `super` while the counter does not.
- Owner-gated: no

The crate's counter idiom is one ungated `#[inline(always)]` shim over a cfg-gated body, called
unconditionally (`codec::scan`, `hull_traffic`, `web_traffic`, `pool_traffic`, suanpan's `touch`).
The limb meter is tapped through `limb_metered.rs` shims, through `#[cfg(feature = "limb-meter")]`
attributes placed on statements inside kernel bodies (five sites), and through four private
site-local shims that re-spell the idiom. A cfg attribute inside a kernel body makes the featured
and unfeatured token streams differ at that site, so the "compiles to nothing" argument is re-made
per site instead of once at the shim, and nothing mechanical checks it. The `base.rs:10` comment
"Test-only metering" is also inaccurate: the release `amp_board` example requires the feature.

Evidence:

        10	// Test-only metering for big-arithmetic operations:
        11	#[cfg(feature = "limb-meter")]
        12	pub(crate) mod limb_meter;
        13	pub(crate) mod limb_metered;
        14	use limb_metered::*;
    --- base.rs:167-168 ---
       167	                #[cfg(feature = "limb-meter")]
       168	                limb_meter::record(2);
    --- gamma.rs:261-262 ---
       261	    #[cfg(feature = "limb-meter")]
       262	    super::base::limb_meter::record_wide(&m);
    --- rank/num.rs:63-69 ---
        63	#[inline(always)]
        64	fn meter_wide(limbs: u64) {
        65	    #[cfg(feature = "limb-meter")]
        66	    crate::codec::base::limb_meter::record(limbs);
        67	    #[cfg(not(feature = "limb-meter"))]
        68	    let _ = limbs;
        69	}
    --- pool_traffic.rs:20-21 ---
        20	//! The recording compiles to nothing without the `limb-meter` feature —
        21	//! its siblings' idiom (`codec::limb_meter`, `suanpan::touch_meter`) —

Resolution: Fold `limb_metered.rs` into `limb_meter.rs` in the `codec::scan` shape: an inner
`#[cfg(feature = "limb-meter")] mod counter` holding the statics, readers, and resets; ungated shims
`record(u64)`, `record_wide(&UBig)`, `record_densified(u64)`, and the existing `meter_limbs*`; make
`pub(crate) mod limb_meter` unconditional and the codec.rs:39-40 re-export ungated; replace the five
inline `#[cfg]` taps with plain calls; reduce the four site-local shims to direct calls; spell the
path one way (`crate::codec::limb_meter`); re-word base.rs:10 to name the feature. Acceptance: no
`#[cfg(feature = "limb-meter")]` attribute appears outside `limb_meter.rs` and `meter.rs`; behaviour
and every envelope reading unchanged.

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

### module-graph-8: The fuzz workspace's gate leg formats but never lints, unlike its four detached siblings
- Where: justfile:363-368 (related: justfile:588-597, justfile:636-644, justfile:658-663, justfile:939-945, crates/before/fuzz/src/lib.rs, crates/before/fuzz/fuzz_targets/)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read the five detached-workspace recipes; `wc -l` over fuzz/src and fuzz_targets gives 914 lines); executed: no
- Verification: confirmed; history: no-rationale-found: the recipe's own comment (363-364) justifies the fmt line by the same argument the sibling recipes use for fmt and clippy together, and gives no reason to stop at fmt.
- Owner-gated: no

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

### module-graph-9: meter.rs holds a private generator table and the public counter read surface in one file
- Where: crates/before/src/meter.rs:18-19 (related: crates/before/src/meter.rs:67-122, crates/before/src/meter.rs:3543-3723, crates/before/src/meter/registry.rs:12-15, crates/before/src/meter/board/ops.rs:192-194)
- Class / severity / confidence: modularity / low / medium
- Provenance: assessed (item skeleton of meter.rs by grep: 3726 lines, 103 `fn`s, the sixteen public counter readers all at 3552-3721; 69 `super::` references in registry.rs; ops.rs's `ops()` at 193 with `#[allow(clippy::too_many_lines)]` above it); executed: no
- Verification: confirmed as a source-navigation cost only: the rendered `before::meter` rustdoc shows the public items alone, so the doc reader is unaffected; the file reader scrolls 3.4k lines of private constructors to reach the read API every metering binary imports; history: no-rationale-found.
- Owner-gated: no

The module has two responsibilities: about eighty private shape constructors reachable only through
`registry::Shape`'s exhaustive match, and the deterministic-counter read surface plus `Packed`. The
registry's "private to the meter module" invariant survives unchanged if the constructors move to a
child module with `pub(super)` visibility.

Evidence:

        18	//! The generators themselves are private: every instrument mints its shapes
        19	//! through the family registry ([`registry`]), whose roster is the single
    --- registry.rs:12-15 ---
        12	//! - **Construction.** The raw shape constructors are private to the
        13	//!   [`meter`](crate::meter) module, and [`Shape`] is the one public door:
        14	//!   its constructor table (`Shape::builder`, the exhaustive match) is
        15	//!   the constructors' only caller outside the module's own unit tests.
    --- board/ops.rs:192-193 ---
       192	#[allow(clippy::too_many_lines)]
       193	pub(super) fn ops() -> Vec<Op> {

Resolution: Move the constructors, `Packed::from_bits`, and the `ev_*`/`pow2*` helpers into
`meter/shapes.rs` as `pub(super)` items; leave meter.rs with its module doc, submodule declarations,
`Packed`, the counter readers, and the re-exports; rewrite registry.rs's `super::x` to
`super::shapes::x` (mechanical). Nit in the same spirit: `ops()` spans lines 193-2275 under a
`too_many_lines` allow; concatenated per-`OpGroup` functions would keep the table declarative while
making a row findable. Acceptance: `before::meter`'s public items and the `compile_fail,E0603`
doctest at registry.rs:23-27 are unchanged; file motion only.

### module-graph-10: The production module cycles are facade/engine pairs; one (codec::display -> idbits) is the substrate reaching up
- Where: crates/before/src/codec/display.rs:1-3 (related: crates/before/src/idbits.rs:30, crates/before/src/codec.rs:9-10, crates/before/src/codec.rs:59, crates/before/src/party.rs:752, crates/before/src/shape.rs:74, crates/before/src/version/skyline/shape.rs:25, crates/before/src/span.rs:10, crates/before/src/version/skyline/place.rs:117, crates/before/src/causally/query.rs:18, crates/before/src/causally/polarity.rs:34, crates/before/src/version/skyline/place/filter.rs:54, crates/before/src/codec/bits.rs:67, crates/before/src/codec/buf.rs:36, crates/before/src/version/skyline/fill.rs:159-161, crates/before/src/version/skyline/fill/fuse.rs:53, crates/before/src/version/skyline/fill/prescan.rs:60)
- Class / severity / confidence: modularity / nit / high
- Provenance: verified (each closing edge read at its cited line; `grep -rn idbits crates/before/src/codec/` shows display.rs:1 as codec's only code-level use of idbits, scan.rs:9 being a doc mention); executed: no
- Verification: confirmed; history: no-rationale-found (591038730 made the renderer iterative through `IdReader`, which is where the edge appears).
- Owner-gated: no

Six strongly connected pairs, each closed by a plain data type flowing upward: `shape` <->
`skyline::shape` (`Rise`), `span` <-> `skyline::place` (the verdict enums), `causally` <->
`skyline::place::filter` (`Coverage`), `codec::bits` <-> `codec::buf` (`BitsView`/`BitsBuf`),
`fill` <-> `fill::{fuse, prescan}` (`DeltaReg`), and `codec::display` <-> `idbits`. The first five
are the ordinary facade pattern and not defects. Only the last inverts a layer: `write_id`, the id
renderer, lives in the substrate module `codec` yet consumes `idbits::IdReader`, which is built on
`codec::BitsView`; its one caller is `party.rs:752`.

Evidence:

         1	use crate::idbits::{IdNode, IdReader};
         2	
         3	use super::{BitsBuf, BitsView};
    --- idbits.rs:30 ---
        30	use crate::codec::BitsView;
    --- codec.rs:9-10 ---
         9	//! and the *id* tree's parse and strict validation (`tree`, with `literal`
        10	//! and `display`). The event coding — the skyline — and its validation are
    --- the five facade closings ---
    shape.rs:74	use crate::version::skyline::shape::{advance_refinement, PartyWalk, Refine, VersionWalk};
    version/skyline/shape.rs:25	use crate::shape::Rise;
    span.rs:10	use crate::version::skyline::place;
    version/skyline/place.rs:117	use crate::span::{Dominance, Endpoint, Placement, Precedence};
    causally/query.rs:18	use crate::version::skyline::place::filter::{self, Demand};
    version/skyline/place/filter.rs:54	use crate::causally::Coverage;
    codec/bits.rs:67	use super::buf::{seal_padding, BitsBuf};
    codec/buf.rs:36	use super::bits::BitsView;
    version/skyline/fill/fuse.rs:53	use super::DeltaReg;
    version/skyline/fill/prescan.rs:60	use super::{DeltaReg, REL_FOLLOWER};

Resolution: Leave the five facade pairs. Consider moving `write_id` (72 lines) beside `idbits` or
under `party`, re-exporting from codec only if the `codec::write_id` path is wanted, and updating
codec.rs:9-10's inventory. Acceptance: no `use crate::idbits` under `codec/`. Taste-level; recorded
because the brief asks for every cycle with its closing edge.

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

### module-graph-12: Redundant cfg on the exported law-roster macro inside an already-gated module
- Where: crates/before/src/laws.rs:107-109 (related: crates/before/src/lib.rs:444-445)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read the module declaration and the macro attributes; `git show 5c990b9069`, which added both lines, gives no reason for the attribute); executed: no
- Verification: confirmed; history: no-rationale-found.
- Owner-gated: no

The macro's `#[cfg(any(test, feature = "laws"))]` repeats the enclosing module's gate exactly. When
the module's cfg is false the file is not loaded, so the macro cannot exist regardless of its own
attribute; `#[macro_export]` hoists the macro's path to the crate root but does not lift it out of
the module's cfg.

Evidence:

       107	#[cfg(any(test, feature = "laws"))]
       108	#[macro_export]
       109	macro_rules! for_each_law_group {
    --- lib.rs:444-445 ---
       444	#[cfg(any(test, feature = "laws"))]
       445	pub mod laws;

Resolution: Drop the attribute, or keep it with a comment if the intent is to survive a future
ungating of the module. Acceptance: one gate for the module and its macro.

## Positives

- `version::skyline` is declared twice (version.rs:22-29): `pub` under `any(test, feature =
  "meter")`, `pub(crate)` otherwise, with the reason stated inline. The shipped build leaks no
  representation while the envelope suite can path-name kernels. Verified by reading.
- The registry door's privacy claim is mechanically checked: the `compile_fail,E0603` doctest at
  registry.rs:23-27 fails to compile `before::meter::cliff_comb(4, 4)` while the sibling doctest
  builds the same comb through `Shape::CliffComb`. Verified by reading.
- The compile-to-nothing shim idiom is uniform at `codec::scan`, `hull_traffic`, `web_traffic`,
  `pool_traffic`, and suanpan's `touch`: an ungated `#[inline(always)]` function over a cfg-gated
  `mod counter`, called unconditionally from the kernels. Verified by reading each.
- No inline `#[cfg(test)] mod tests { }` block exists anywhere under `crates/before/src` or
  `crates/suanpan/src` (grep for `mod tests {` returns nothing); every suite is a sibling file.
- suanpan's module graph is a DAG: `accumulator`, `limbs`, `magnitude`, `touch_meter` behind its
  feature, `claims` under `cfg(test)` (lib.rs:352-365). Verified by reading.
- The web_traffic counter's introduction (76579a3c) records the known-bad demonstration the doctrine
  asks for: under a guard-disable mutation the arm-liveness floor reads red at both homes while
  every value differential stays green. Assessed from the commit message and web_traffic.rs:11-17.
- The envelope table's comment (tests/meter.rs:245-257) keeps measurement history out of the tree:
  "the measurements of record — and every re-pin's movement and attribution — live in the pin
  commits (`git log -S` the constant), never in this prose."
- `tests/support/fuzz_seed_set.rs` is included by `#[path]` from both the checker test
  (tests/fuzz_seeds.rs:19-20) and the writer example (examples/fuzz_seeds.rs:13-14), so the seed
  corpus has one derivation. Verified by reading.
- The `features` recipe (justfile:508-522) checks every cfg-gated surface alone, so nothing rots
  behind `--all-features`; the feature implications (`laws -> meter`, `limb-meter -> meter,
  suanpan/touch-meter`, `scan-meter -> meter`) match where the read surfaces live.
- Four of the five detached workspaces (fuzzfit, wasm32-pins, fuelscape, surfacecheck) carry their
  own fmt+clippy leg with the discipline stated in the recipe comment. Verified by reading.

## Open questions for Finch

1. Stack-segments column (module-graph-1): is a production traversal ever expected to route through
   `crate::recurse::descend!`? If not, the column outside the lib unit-test binary measures a
   compile-time fact and retiring it (option a) is the honest state; if so, the guard needs to
   exist in the `meter` builds before the column can mean anything (option b). Either way the two
   doc passages need re-stating now. Recommendation: (a).
2. `meter` feature scope (module-graph-3): is `meter` meant to stay exposure-only, in which case
   `hull_traffic` and `web_traffic` move under a counter feature (and the ungated unit-test readers
   in `meter/tests.rs` and `fill/tests.rs` gain a cfg or the gate becomes `any(test, feature)`),
   or do you accept the two relaxed-atomic bumps in bench builds and want the `meter` comment to
   name them? Recommendation: move them under `scan-meter`, since rumors' own `meter` feature
   already lights it.
3. Design essay (module-graph-5): 22cdfbe1 says the `implementation` module was retired. Does its
   content survive anywhere you want AGENTS.md to point at, or does the clause go?
4. Validation index (module-graph-11): do you want it rendered (moved under the meter-gated tree)
   or declared source-only? The `--cfg test` rustdoc route is blocked by dev-dependency imports in
   the cfg(test) tree.

## Dropped

None: every seed survived against the code. Reframings are recorded in each finding's Verification
line (the segments counter has one live writer in the lib unit-test binary; the `features` recipe
prints but does not deny default-feature warnings; the fuelscape comment was true when written; the
`--cfg test` rustdoc alternative is blocked by dev-dependencies; codec's only idbits use is
display.rs).
