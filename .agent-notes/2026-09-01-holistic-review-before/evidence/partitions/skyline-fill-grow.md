# Partition skyline-fill-grow: The fused tick: fill (fuse, memo, prescan) and grow

## Partition summary

This partition is the `tick` kernel of `before`'s skyline coding: `fill.rs` runs one iterative walk that pairs the packed id (`IdReader`) against the event stream, decides in-pass whether `fill(id, e)` moves the tree (the changed flag, realized as the `Out` output mode in `fuse.rs`), and folds grow's `(expansions, depth)` route DP over the same nodes (`RouteProbe`); `grow.rs` replays the recorded `Route` in one splice when the flag stays clear, compounding `+k` events at the chosen leaf. Left-full shortcut sites need a minimum from a range the walk has not reached, so `prescan.rs` runs one memoized pre-scan per uncovered site over the pre-scan's own watermark web, recording each interior site's minimum as a ledger link in `memo.rs`. Both walks hold suspended ancestors as bits (`Frames`, `PreFrames`) with word deltas on a `PopStack`, so no input depth touches the call stack. `fill/tests.rs` and `grow/tests.rs` pin the whole thing against the recursive oracle (`event`, `fill`, `grow`), a brute-force minimal inflation, a reference recursive route probe, the exhaustive small scope, organic histories, `ticks(n)` against iterated ticks and a monoid-action law at `2^100`, and closed-form witnesses at depth 4096.

I read all seven partition files in full with line numbers (5754 lines: fill.rs 1312, fuse.rs 463, memo.rs 146, prescan.rs 695, fill/tests.rs 1964, grow.rs 692, grow/tests.rs 482; the two tests.rs files are the test surfaces) plus the callees and instruments the findings cite (idbits.rs, codec/stack.rs, codec/buf.rs, codec/base.rs, codec/build.rs, skyline/build.rs, signed.rs, watermark.rs, sweep.rs, walk.rs, party.rs, oracle/version.rs, meter.rs's memo generators, meter/board/ops.rs, meter/board/ceilings.rs, tests/meter.rs's envelope struct and `memo_resolution_cost` module, tools/citecheck, tools/covcheck-expected.json, .cargo/mutants.toml, lib.rs, version.rs, suanpan's `Accumulator`, and the pinned toolchain's `debug_assert!` source), and ran read-only git (`blame`, `log -S`) on the one prose-contradicts-code finding. I ran no cargo or just command; every finding below is assessed by reading or verified by grep, git, or hand trace, and none is settled by execution.

The kernel is in very good shape. Every `expect`/`unreachable!` message is a one-line proof and none carries an em-dash; the compile-time bindings (`OUT_FOLLOWER`/`REL_FOLLOWER` against `FOLLOWER_SLOTS`, `Cost::CEILING < Cost::INFEASIBLE`, the `Option<NonZeroU32>` niche) put layout claims under the compiler; the changed flag as an output mode and the `Step`/`Relation` enums put invariants in types rather than asserts; and the grow suite pins its grow-branch pair counts exactly so a rerouting regression cannot pass vacuously. No correctness defect surfaced. The dominant issues are structural duplication (the frame bit-stack type and the paired-walk skeleton spelled in both walks, `IdReader`'s cursor re-spelled in grow.rs, `fold_block` inlined in `consume_payload`), encapsulation (fill.rs drives `Memo` and `PreScan` through `pub(super)` fields, so the ledger's documented lifetime is enforced from another file), and prose that outlived its code (a `usize` width the walk no longer has, measured exponents no instrument holds, a `# Panics` form the crate superseded elsewhere, a "recursion argument" in an iterative walk). The one substantive claim finding is a space instrument gap: the distinct-minima memo forests hold on the order of a hundred heap bytes per input byte in suspended `Accumulator` structs and nonzero links, against a crate-level "small constant multiple" promise, and no committed meter reads heap on those families.

## Findings

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

Disposition (owner ruling 2, 2026-09-02, `triage/rulings.md`): the transient-space promise is absolute, like the time bounds, and this cost is to be prevented, not modeled. The cause is representation: every link and both per-level parkings in `SuspendedLevel` are whole `Accumulator`s by value (about 96 bytes of inline state each before any digit), against about four packed input bytes per site. Plan of record, measure-first: land the witness's two-scale heap reading as an envelope row first, red, with `suspend.len()` and `links.len()` recorded at the peak; then store links and the suspended head and keeper word-or-wide with the wide arm boxed, through one shared type modeled on `MinWeb::Boundary` (`watermark.rs:154-159`); reserve the suspend stack from the id tree's nesting depth; consider writing the deferred head into its ledger slot at suspend time, since its value is final then; and restate the heap paragraph at `fill.rs:130-134`. The layout predicts the comb in the low twenties of bytes per input byte after the representation change and a floor of a few bytes per input byte; the envelope row confirms or refutes that, measured at the parent.

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

### skyline-fill-grow-5: `# Panics` on tick and ticks documents an empty-id state the `Party` type excludes
- Where: crates/before/src/version/skyline/fill.rs:205-208 (related: crates/before/src/version/skyline/fill.rs:244-246, crates/before/src/version/skyline/grow.rs:489-492, crates/before/src/party.rs:791-800, crates/before/src/party.rs:643-654, tools/covcheck-expected.json:25-29)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read `finish_id` at party.rs:794-800 and `Party::anonymous` at 643-654, `pub(crate)` and documented transient; covcheck-expected.json:28 records the same type fact for the `IdNode::Empty` arm); executed: no
- Seen by: structure (17), claims (43); refutation: confirmed (`anonymous()`'s one caller is a placeholder in forks.rs, overwritten two lines later); history: no-rationale-found (the clause landed in fd5c92bf when `tick` already took `&Party` and decode already rejected the anonymous id)
- Owner-gated: no

Both functions take `&crate::Party`, and every `Party` passes `finish_id`, which rejects the anonymous identity. The clause documents what the type prevents and alarms the reader with an unreachable "unspecified" outcome. Never document what the types prevent; `grow::emit`'s `debug_assert!(!id_bits.is_empty(), ..)` sits at the raw-bits seam and is the right place for the fact.

Evidence:

       205	/// [`validate`](fn@super::validate) first on untrusted bytes. The id must own
       206	/// at least one region: an empty id leaves `fill` the identity, and the grow
       207	/// fallback requires an owning id (debug builds assert it; the result on an
       208	/// empty id is unspecified in release builds).
    (party.rs)
       794	fn finish_id(bits: codec::BitsBuf) -> Result<Party, Parse> {
       795	    if codec::id_is_empty(codec::built_view(&bits)) {
       796	        Err(Parse::Anonymous)

Resolution: Delete the empty-id sentences from `tick`, `ticks`, and `grow::emit`'s docs; keep `emit`'s debug assert. Acceptance: the three doc sites name only the canonical-stream precondition.

### skyline-fill-grow-6: Long qualified paths where the module is already imported
- Where: crates/before/src/version/skyline/fill.rs:264-271 (related: crates/before/src/version/skyline/fill.rs:152-167, 185-188, 209, 247, 282, 293, 686, 690, 806, 981)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the import block and every cited site); executed: no
- Seen by: structure (15); refutation: confirmed; history: no-rationale-found (a local habit; every test file imports `Party`)
- Owner-gated: no

fill.rs imports `Cost` from `super::grow` and `MinWeb` from `super::watermark`, yet spells `super::grow::emit` (264, 271), `super::grow::Route` (282), `super::watermark::FOLLOWER_SLOTS` (186-187), `crate::Party` in three signatures (209, 247, 293), `core::mem::replace` three times (690, 806, 981), and `self::memo::position_check` (686). Imports over long qualified paths, except where the qualification informs; `grow::emit` and `mem::replace` are the informing one-segment forms.

Evidence:

       162	use super::grow::Cost;
       264	                    super::grow::emit(codec::built_view(&bits), id.as_bits(), &route, &remaining)
       271	        FillOutcome::Unchanged(route) => super::grow::emit(event, id.as_bits(), &route, n),
       690	        match core::mem::replace(&mut self.relation, Relation::None) {

Resolution: `use super::grow::{self, Cost, Route}; use super::watermark::{MinWeb, FOLLOWER_SLOTS}; use crate::Party; use core::mem;` and `use self::memo::{position_check, Memo}` (the former under `cfg(debug_assertions)`). Acceptance: no `super::grow::`, `super::watermark::`, `crate::Party`, or `core::mem::` at a use site in fill.rs.

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

### skyline-fill-grow-8: Output-delta anchoring is a bool plus an idle accumulator beside an enum-shaped sibling, and the anchor switch is spelled twice
- Where: crates/before/src/version/skyline/fill.rs:357-364 (related: crates/before/src/version/skyline/fill.rs:858-866, 924-931, 971-984, 394-414, 912, 1024)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read); executed: no
- Seen by: structure (6); refutation: confirmed (`emit_offset` folds the offset between the bridge and materialize, so a helper returns the accumulator pre-materialize); history: no-rationale-found (`Relation` gained its tag-in-struct/storage-in-web enum and stated invariant in 055f2e48 and a736ef14; `w_anchored`/`gap` were not revisited)
- Owner-gated: yes for the enum (it moves readings); the helper is adoptable now

`w_anchored: bool` tags whether `min − prev_out` rides `OUT_FOLLOWER` or `gap` holds `h − prev_out`, with `gap` documented as idle while the tag is set. `Relation` (394-414) models the same tag-in-struct/storage-in-web shape as an enum with its invariant stated. The watermark-to-height switch (`follower_take(OUT_FOLLOWER)`, `bridge_add_gap`, `w_anchored = false`) is spelled in `emit_step` and again in `emit_offset`. Two spellings of one state shape in one struct cost the reader a second model; an idle-while-tagged accumulator is an invariant the type could carry.

Evidence:

       357	    /// `h − prev_out` while the output delta is height-anchored: every consumed
       358	    /// step folds in, and every emitted leaf re-derives it. Idle (zero) while
       359	    /// `w_anchored`.
       360	    gap: Accumulator,
       361	    /// Whether the output delta is watermark-anchored: the last emission took
       362	    /// the tracked minimum, and `min − prev_out` rides the web's
       363	    /// [`OUT_FOLLOWER`] instead of `gap`.
       364	    w_anchored: bool,
       862	            let mut out_delta = self.web.follower_take(OUT_FOLLOWER);
       863	            self.web.bridge_add_gap(&mut out_delta);
       864	            self.w_anchored = false;
       927	                let mut out_delta = self.web.follower_take(OUT_FOLLOWER);
       928	                self.web.bridge_add_gap(&mut out_delta);
       929	                fold_signed_int(&mut out_delta, offset.sign, &offset.magnitude);
       930	                self.w_anchored = false;

Resolution: Now: `fn out_delta_from_min(&mut self) -> Accumulator` wrapping the three-line switch; `emit_step` materializes it, `emit_offset` folds the offset first. Proposal for the owner: `enum OutAnchor { Height(Accumulator), Min }` mirroring `Relation`, which removes the idle-gap sentence and the `debug_assert!(!self.w_anchored, ..)` at 912 and 1024; `emit_at_min`'s `mem::replace(&mut self.gap, fresh)` at 981 becomes a variant swap. Acceptance: one switch helper; if the enum is adopted, no `w_anchored` field and no idle-gap sentence.

### skyline-fill-grow-9: Hand-maintained `depth` counters mirror O(1) stack lengths, justified by a recount that does not exist
- Where: crates/before/src/version/skyline/fill.rs:433-440 (related: crates/before/src/version/skyline/fill.rs:536, 545, 559, 569, 603; crates/before/src/version/skyline/grow.rs:516-519, 616-625; crates/before/src/codec/stack.rs:37-45; crates/before/src/codec/buf.rs:104-107)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read `BitStack::len`, `words.len() as u64 * 64 + u64::from(self.top_len)`, and `BitsBuf::len`, `self.live`); executed: no
- Seen by: structure (5); refutation: confirmed (fill.rs's ascend arms read `depth + 1` after the decrement at 569 but before the pops, so the replacement there is `frames.len()`); history: no-rationale-found (the "never recounts" sentence was written in a736ef14 when `len` was already the O(1) multiply-add it is today)
- Owner-gated: no

The comment's reason names a cost the code has never had: `frames.len()` is `site.len()`, a multiply-add. The `debug_assert_eq!` exists only to check the mirror, and the mirror exists only for the phantom cost: circular justification. `grow::emit` carries the same shape against `pending.len()`. A derived counter maintained by hand is one more invariant to keep across the `+= 1`/`-= 1` sites and the width paragraph it needs.

Evidence:

       433	        // Derived state: always equal to `frames.len()` (the assert below),
       434	        // carried as a word so the hot loop never recounts a bit stack.
       438	        let mut depth = 0u64;
       440	            debug_assert_eq!(depth, frames.len(), "one frame per open branch level");
    (codec/stack.rs)
        43	    pub(crate) fn len(&self) -> u64 {
        44	        self.words.len() as u64 * 64 + u64::from(self.top_len)
        45	    }
    (grow.rs)
       621	    debug_assert_eq!(
       622	        path_depth,
       623	        pending.len(),
       624	        "one pending record per path level"
       625	    );

Resolution: grow.rs is the clean case: drop `depth`, use `pending.len() + 1` at 564 and `let path_depth = pending.len();` at 616, delete the assert. fill.rs: bind `let depth = frames.len();` at the head of the descend loop and `let depth = frames.len() - 1;` after the `web.close()` in each ascend iteration (the arms' `depth + 1` is then the pre-pop `frames.len()`); delete the counter, its updates, and the asserts at 440 and 559. Acceptance: no `depth` counter or sync assert in `FillWalk::walk` or `grow::emit`; tests green; no envelope movement.

### skyline-fill-grow-10: Em-dashes in `//` comments (62 sites in the partition)
- Where: crates/before/src/version/skyline/fill.rs:456-459 (related: fill.rs 23 sites, prescan.rs 12, fill/tests.rs 19, grow.rs 5, fuse.rs 3; 312 further sites across the crate outside this partition)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -c -E '^\s*//[^/!].*—'` per file; the same grep over `src/` excluding the partition sums to 312; no assert/expect/unreachable message carries one); executed: no
- Seen by: prose (26); refutation: reframed (the partition follows the crate's uniform comment style; the double-hyphen rule is a crate-wide ruling); history: contradicts the owner's global writing-style rule, first tracked 2026-08-19, after every cited site
- Owner-gated: yes (a crate-wide style ruling)

The owner's register rule puts spaced double-hyphens in code comments and reserves em-dashes for rendered prose (terminal rendering is the stated reason). The partition has 62 such `//` lines and the rest of the crate 312; this is a crate-wide ruling, not a defect of these files. Recorded so the owner can decide once.

Evidence:

       456	                    // fill(1, e) = max(e): a fully-owned region collapses. On a
       457	                    // route-live walk the region is a single leaf — a node
       458	                    // would collapse to fewer plateaus and trip the flag at the
       459	                    // emission below — so the cost is the free increment

Resolution: If the owner rules for the double-hyphen in this crate, one mechanical pass over `//` (not `///`/`//!`) lines replacing ` — ` with ` -- ` or a colon; leave rustdoc em-dashes alone. Acceptance: the grep returns nothing crate-wide.

### skyline-fill-grow-11: Every shortcut arm reads its full child's 2-bit tag twice (peek, then skip)
- Where: crates/before/src/version/skyline/fill.rs:486-495 (related: crates/before/src/version/skyline/fill.rs:584-591, crates/before/src/version/skyline/fill/prescan.rs:193-199, crates/before/src/version/skyline/fill/prescan.rs:255-259, crates/before/src/idbits.rs:132-156)
- Class / severity / confidence: performance / nit / high
- Provenance: verified (read `IdReader::peek`, which records 2 bits, and `IdReader::skip`, whose `skip_subtree` header probe reads and records the Full node's own tag once more); executed: no
- Seen by: claims (45); refutation: confirmed; history: no-rationale-found (`IdReader` has no advance-past-terminal method)
- Owner-gated: no

`id.peek()` records and reads the tag; the following `id.skip()` on the known-Full node runs `skip_subtree`, whose header probe reads and records the same tag again to learn it has no children. Four scan-meter bits per shortcut site where two suffice, at four sites. Fixed-sign deletion on the hot path; acting re-pins every tick row's scan column, which is why this is a nit and not a recommendation to act now.

Evidence:

       486	                if left && matches!(id.peek(), IdNode::Full) {
       495	                    id.skip();
    (idbits.rs)
       136	                crate::codec::scan::record_bits(2); // one 2-bit tag scanned
       151	                crate::codec::scan::record_bits(2);

Resolution: A crate-private `IdReader::skip_terminal()` (advance by 2, no read) for the case where the caller has just peeked `Full`, used at the four sites; re-pin the affected scan columns with attribution in the same commit. Acceptance: tick-row scan readings drop by exactly 2 bits per shortcut site, stated in the re-pin.

### skyline-fill-grow-12: fill.rs drives `Memo` and `PreScan` through `pub(super)` fields; the ledger's lifetime is prose in memo.rs and mechanism in fill.rs
- Where: crates/before/src/version/skyline/fill.rs:516-532 (related: crates/before/src/version/skyline/fill.rs:317-334, 676-689; crates/before/src/version/skyline/fill/memo.rs:45-57, 69-94; crates/before/src/version/skyline/fill/prescan.rs:60, 65-103, 287-299)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (read the field visibilities: memo.rs:76 `queue`, 82 `cursor`, 85 `covered_until`, 90/93 the checksums; prescan.rs:70 `web`, 102 `suspend`; the launch sequence and `consume_site`'s hand-advanced cursor; prescan.rs:60 already imports `REL_FOLLOWER`); executed: no
- Seen by: structure (2); refutation: confirmed; history: no-rationale-found (055f2e48's split replaced a 10-field struct literal with `PreScan::new` and left the launch sequence and the consume half in fill.rs; nothing records a reason for the field exposure)
- Owner-gated: no

The fresh-scan launch (519-531) sequences `begin_scan`, `PreScan::new`, `reserve`, `web.open(1)`, `run`, `record(slot, 0)`, `follower_take`/`retire`/`close` on the scan's web, an assert on `scan.suspend`, and `memo.covered_until = end` from outside prescan.rs; `consume_site` folds the checksum, takes the link, and advances `memo.cursor` by hand; `fused_fill`'s epilogue asserts on `memo.cursor`, `memo.queue.len()`, and both checksums. memo.rs titles the rule ("# Lifetime: one create, one consume") whose consume half fill.rs implements. Modules have a single clear responsibility; the invariant the walk's cost argument rests on is stated where it is not implemented.

Evidence:

       519	                        self.memo.begin_scan();
       520	                        let scan_start = self.pos();
       521	                        let mut scan = PreScan::new(self.event, scan_start, &mut self.memo);
       522	                        let slot = scan.reserve(scan_start);
       523	                        scan.web.open(1);
       524	                        let mut reader = IdReader::at(id.bits(), id.pos());
       525	                        let end = scan.run(&mut reader);
       526	                        scan.record(slot, 0);
       527	                        let relation = scan.web.follower_take(REL_FOLLOWER);
       528	                        scan.web.retire(relation);
       529	                        scan.web.close();
       530	                        debug_assert!(scan.suspend.is_empty(), "every suspended level resolves");
       531	                        self.memo.covered_until = end;
       688	        let link = self.memo.take_link(self.memo.cursor);
       689	        self.memo.cursor += 1;
    (memo.rs)
        45	//! # Lifetime: one create, one consume

Resolution: Give `Memo` the consume half: `consume(&mut self, pos: u64) -> Option<Accumulator>` (debug-assert `cursor < queue.len()`, fold `pos` into `consumed_check`, take the link, advance), `reserve(&mut self, pos: u64) -> usize` (moved from `PreScan::reserve`), `is_covered(&self, pos) -> bool` / `cover_until(&mut self, end)`, and `drained(&self) -> bool` for the epilogue asserts. Give `PreScan` one entry, e.g. `cover(event, start, id_bits, id_pos, &mut memo) -> u64`, that owns new/reserve/open/run/record/retire/close and the `suspend.is_empty()` assert. Then every `pub(super)` field on `Memo` and `PreScan` becomes private and `record`/`reserve` become private too. Behavior-preserving. Acceptance: no `pub(super)` field on `Memo` or `PreScan`; fill.rs's left-full arm calls one `PreScan` method and `consume_site` calls one `Memo` method; `just gate` clean.

### skyline-fill-grow-13: `consume_payload` inlines `fold_block`'s body
- Where: crates/before/src/version/skyline/fill.rs:640-674
- Class / severity / confidence: simplification / low / high
- Provenance: verified (side-by-side read of 648-655 and 666-673); executed: no
- Seen by: structure (4); refutation: confirmed (`consume_payload` already builds `Signed { sign, magnitude }` at 656); history: no-rationale-found (`fold_block` landed in 77d7da0b as the block scans' re-entry fold; nothing records leaving the per-leaf copy inline)
- Owner-gated: no

Lines 648-655 are statement-for-statement the body of `fold_block` (666-673) with `sign, &magnitude` in place of `net.sign, &net.magnitude`. The doc at 662-664 already states the two are the same fold; making it structural means a new height-carried register is added in one place.

Evidence:

       648	        fold_signed_int(&mut self.height, sign, &magnitude);
       649	        self.web.fold_height(sign, &magnitude);
       650	        if !self.w_anchored {
       651	            fold_signed_int(&mut self.gap, sign, &magnitude);
       652	        }
       653	        if let Relation::Height(relation) = &mut self.relation {
       654	            fold_signed_int(relation, sign, &magnitude);
       655	        }
       662	    /// Exactly what [`consume_payload`](Self::consume_payload) would have
       663	    /// folded leaf by leaf: nothing reads the registers between a block's

Resolution: `consume_payload`: decode, then `let step = Signed { sign, magnitude }; self.fold_block(&step); step`; restate `fold_block`'s doc as the primitive (`consume_payload` decodes and folds through it). Acceptance: one fold sequence in fill.rs.

### skyline-fill-grow-14: Register texture that names no mechanism
- Where: crates/before/src/version/skyline/fill.rs:895-897 (related: crates/before/src/version/skyline/fill.rs:1018; crates/before/src/version/skyline/fill/memo.rs:88; crates/before/src/version/skyline/fill/fuse.rs:36; crates/before/src/version/skyline/grow.rs:36, 127-129; crates/before/src/version/skyline/fill/tests.rs:670, 702, 1267, 1385)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n -E 'honest|genuinely|simply|route tax|freight|bill the'` over the partition returns exactly these sites); executed: no
- Seen by: prose (27); refutation: confirmed; history: contradicts the owner's global writing-style rule ("describe code by the property that holds"); the vocabulary is crate-wide (ceilings.rs's "worst honest reader") and was a Wave 7 deliverable, so any sweep is crate-wide
- Owner-gated: no

The crate's cost vocabulary ("priced by", "funded", "dies") is anchored. Beyond it: "genuinely discriminates" (897), "their freight" (1018), "bill the heap meter" (memo.rs:88), "the route tax" (fuse.rs:36), "simply carries `k`" (grow.rs:36), "Reaching the ceiling honestly" (grow.rs:127), and "honest"/"honestly" in four test docs. Metaphors only where they rewrite as mechanism; "honest" moralizes a computation whose property is "without saturation" or "by a real chain"; significance adverbs add nothing.

Evidence:

       895	            // A value-reproducing emission on a verbatim walk always
       896	            // matches (the doc's argument), unlike `emit_step`'s
       897	            // unguarded call, where the bool genuinely discriminates.
    (grow.rs)
       127	    /// toward an absent child. Reaching the ceiling honestly needs
       128	    /// `u64::MAX - 1` id levels — beyond any encodable id — so saturation

Resolution: 897 drop "genuinely"; 1018 "their size"; memo.rs:88 "charge the heap meter for"; fuse.rs:36 "the route fold's cost"; grow.rs:36 drop "simply"; grow.rs:127 "Reaching the ceiling by a real chain needs"; tests.rs:670/702 "is decidable"/"undecided"; 1267 "is exact arithmetic"; 1385 "reaching the production ceiling without saturation". Acceptance: the grep returns nothing over the partition.

### skyline-fill-grow-15: `let _ = matched;` after `debug_assert!(matched, ..)` is dead in both build profiles
- Where: crates/before/src/version/skyline/fill.rs:898-900 (related: crates/before/src/version/skyline/fill.rs:1038-1040)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (read the pinned toolchain's `debug_assert!` at `$(rustc --print sysroot)/lib/rustlib/src/rust/library/core/src/macros/mod.rs:287-293`, rustc 1.97.1); executed: no
- Seen by: structure (14); refutation: confirmed; history: no-rationale-found (introduced verbatim in 41b11560 and copied)
- Owner-gated: no

`debug_assert!` expands to `if cfg!(debug_assertions) { assert!(..) }`, so `matched` is a syntactic use in release builds and no unused-variable warning fires; the two `let _ = matched;` lines suppress nothing and invite the reader to look for a reason.

Evidence:

       898	            let matched = self.out.note_match(self.pos());
       899	            debug_assert!(matched, "a verbatim walk records a value-reproducing raise");
       900	            let _ = matched;
    (core/src/macros/mod.rs, toolchain 1.97.1)
       287	macro_rules! debug_assert {
       288	    ($($arg:tt)*) => {
       289	        if $crate::cfg!(debug_assertions) {
       290	            $crate::assert!($($arg)*);

Resolution: Delete fill.rs:900 and 1040. Acceptance: `just clippy` clean in both profiles.

### skyline-fill-grow-16: The collapse-then-readout idiom is spelled inline four times; the first-leaf variant rebuilds `Signed` by hand
- Where: crates/before/src/version/skyline/fill.rs:913-920 (related: crates/before/src/version/skyline/fill.rs:870-872, 935-937, 1119-1120; crates/before/src/version/skyline/fill/prescan.rs:580-581; crates/before/src/version/skyline/watermark.rs:1140-1150; crates/before/src/version/skyline/signed.rs:105-113; crates/suanpan/src/accumulator.rs:704-721, 950-968)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep for a `.sign();` immediately preceding `sign_magnitude()` under skyline/ returns fill.rs:870, 913, 935 and watermark.rs:1146; read `Signed::from_sign_magnitude`, which maps every non-`Less` ordering to `Positive`; read suanpan's `sign` (amortized O(1), collapses a cancelling prefix) and `sign_magnitude` (O(held digits), no collapse)); executed: no
- Seen by: structure (7); refutation: confirmed; history: no-rationale-found (signed.rs was created with deliberately minimal helpers; the hand-built `Signed { sign: Sign::Positive, .. }` is residue of the `Sign` enum conversion in 4ac8fd70)
- Owner-gated: no

`acc.sign(); let (s, m) = acc.sign_magnitude(); Signed::from_sign_magnitude(s, m)` appears at 870-872, 935-937, and in `MinWeb::materialize` (watermark.rs:1146-1149); at 913-920 the same readout is followed by a hand-built `Signed` where `from_sign_magnitude` already yields `Positive` for a non-`Less` sign. signed.rs presents itself as the one home of the vocabulary every walk exchanges heights in; the reason for the collapse (watermark.rs:1144-1145) is written once there and nowhere at the fill.rs sites. On the structure lens's open question: the collapse is load-bearing for the touch-meter claim, not for correctness. `sign()` compacts a cancelling prefix at amortized O(1); `sign_magnitude()` reads O(held digits) without compacting, so without the collapse a readout can touch digits the value's width does not price, breaking "materialized once, post-collapse, at the width the code itself prices" (fill.rs:56). The block-net readouts at fill.rs:1119 and prescan.rs:580 skip the collapse; their reads are funded by the block scan that just read the same digits, so no cost claim breaks, but the helper should say so.

Evidence:

       913	            self.height.sign();
       914	            let (sign, magnitude) = self.height.sign_magnitude();
       915	            debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
       916	            let value = Signed {
       917	                sign: Sign::Positive,
       918	                magnitude: Int::from_ubig(magnitude),
       919	            }
       920	            .sum(&offset);
    (signed.rs)
       108	    pub(super) fn from_sign_magnitude(sign: Ordering, magnitude: UBig) -> Self {
       109	        Signed {
       110	            sign: Sign::from_is_negative(sign == Ordering::Less),
       111	            magnitude: Int::from_ubig(magnitude),
    (watermark.rs)
      1144	        // Collapse for an honest width before the read-out: `sign()` is
      1145	        // called for its compaction side effect, the value unread.
      1146	        let _sign = dying.sign();
      1147	        let (sign, magnitude) = dying.sign_magnitude();

Resolution: Add `Signed::read(acc: &mut Accumulator) -> Signed` beside `from_sign_magnitude` (collapse via `sign()`, then `sign_magnitude`, then `from_sign_magnitude`), carrying the width note: the collapse bounds the O(held digits) readout to the value's width plus slack, so the read is priced by the code that emits it; a block net may skip it because its scan already paid for the held digits. Use it at fill.rs:870-872, 935-937, watermark.rs:1146-1149, and make 913-920 `Signed::read(&mut self.height).sum(&offset)` (the non-negativity assert at 921 covers the sum). Acceptance: no inline `sign(); sign_magnitude(); from_sign_magnitude` triple outside signed.rs; no hand-built `Signed { sign: Sign::Positive, .. }` from a readout.

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

### skyline-fill-grow-18: `continue_verbatim`'s seven positional `u64`s are hand-marshalled from two different summary structs
- Where: crates/before/src/version/skyline/fill/fuse.rs:163-198 (related: crates/before/src/version/skyline/build.rs:233-243, crates/before/src/version/skyline/fill.rs:1060-1068, crates/before/src/version/skyline/grow.rs:392-400, crates/before/src/version/skyline/grow.rs:318-331)
- Class / severity / confidence: idiom / low / medium
- Provenance: verified (read both `#[allow(clippy::too_many_arguments)]` sites and both call sites); executed: no
- Seen by: structure (9); refutation: confirmed; history: no-rationale-found (the seventh argument arrived in e7d2548f; the allow comment rationalizes the count, not the positional shape)
- Owner-gated: no (the signature lives in build.rs, another partition; both call sites are here)

`Out::continue_verbatim` forwards seven positional arguments under a `too_many_arguments` allow, filled from a `RegionSkip` at fill.rs:1060-1068 and from a `Subtree` at grow.rs:392-400, each hand-marshalling `(start, end, root_depth, first_rel_depth, last_rel_depth, last_code_len)`. Types-first: six same-typed positional integers are a swap waiting to happen, and the allow acknowledges the shape; `RegionSkip` and `Subtree` are already the same product (a block summary of one subtree for the verbatim splice).

Evidence:

       173	    #[allow(clippy::too_many_arguments)] // (src, start, end) is one logical range argument
       174	    pub(super) fn continue_verbatim(
       175	        &mut self,
       176	        src: BitsView<'_>,
       177	        start: u64,
       178	        end: u64,
       179	        root_depth: u64,
       180	        first_rel_depth: u64,
       181	        last_rel_depth: u64,
       182	        last_code_len: u64,
    (grow.rs)
       392	        out.continue_verbatim(
       393	            event.bits,
       394	            subtree.first_code.end,
       395	            subtree.end,
       396	            depth,
       397	            subtree.first_rel_depth,
       398	            subtree.last_rel_depth,
       399	            subtree.last_code.end - subtree.last_code.start,
       400	        );

Resolution: Introduce a splice-range struct in build.rs (`start`, `end`, `first_rel_depth`, `last_rel_depth`, `last_code_len`, or a `Range<u64>` plus a leaf-coordinate pair); have `RegionSkip` and `Subtree` each produce one; `continue_verbatim(src, root_depth, splice)` drops both allows. Acceptance: no `too_many_arguments` allow on either `continue_verbatim`; both call sites pass one struct.

### skyline-fill-grow-19: `if left { id.skip() } if right { id.skip() }` re-spells `IdReader::skip_present_children`
- Where: crates/before/src/version/skyline/fill/fuse.rs:320-325 (related: crates/before/src/version/skyline/fill/prescan.rs:185-190, crates/before/src/idbits.rs:158-170)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rn skip_present_children src`: defined idbits.rs:161, used party/ops/compare.rs:78 and party/ops/index.rs:197, not under skyline/fill); executed: no
- Seen by: structure (16); refutation: confirmed (both sites hold destructured bools, so using the helper means keeping the `IdNode`); history: no-rationale-found (65bbda3e created the helper for party/ops and did not convert these pre-existing sites)
- Owner-gated: no

`RouteProbe::expand`'s dead-probe arm and `PreScan::run`'s leaf arm skip present children with two conditionals; `IdReader::skip_present_children(node)` is the one home of the idiom. Using it means keeping the decoded `IdNode` in hand rather than destructured bools, which also reads as the paper's match.

Evidence:

       320	            if left {
       321	                id.skip();
       322	            }
       323	            if right {
       324	                id.skip();
       325	            }
    (idbits.rs)
       161	    pub(crate) fn skip_present_children(&mut self, node: IdNode) {

Resolution: Keep the `IdNode` from `id.read()` at both sites and call `id.skip_present_children(node)`. Acceptance: no paired conditional skips in fuse.rs or prescan.rs.

### skyline-fill-grow-20: `expand_subtree`'s `# Panics` attributes an unrepresentable state to a normal-form violation
- Where: crates/before/src/version/skyline/fill/fuse.rs:380-385 (related: crates/before/src/version/skyline/fill/fuse.rs:421-425, crates/before/src/idbits.rs:12-13, crates/before/src/idbits.rs:98-109)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read `IdReader::tag`: `!left && !right` decodes as `Full`, so an `Internal` always has a present child; idbits.rs:12-13 states `(0, 0)` is unrepresentable in the pruned coding); executed: no
- Seen by: correctness (35); refutation: confirmed (a truncated id panics first in `BitsView::bit`'s range assert); history: deliberate-and-holds for the assert (9c7b6999 replaced a fabricated-cost fallback under the owner's direction that impossible states panic); the doc's stated trigger is the residue
- Owner-gated: no

The rise loop's `assert_ne!` and its `# Panics` say the infeasible root fold is reached by "an internal node with no present child", a normal-form violation `decode` rejects. But the coding cannot spell that node: the tag `00` decodes as `Full`, so every `Internal` has a present child and every present child's subtree bottoms out in a `Full` terminal. The state is unrepresentable in the coding, not merely non-normal; the assert is right to stay (an impossible state panics rather than falling through) but its proof names the wrong mechanism.

Evidence:

       382	    /// Panics if the id is not in normal form. An internal node with no present
       383	    /// child leaves the root fold infeasible, which `decode` rejects and no
       384	    /// operation produces, so reaching it is programmer error: the fold's own
       385	    /// answer would be a fabricated cost the caller cannot tell from a real one.
       421	                        assert_ne!(
       422	                            distance,
       423	                            Cost::INFEASIBLE,
       424	                            "an internal node in normal form has a present child"
    (idbits.rs)
       104	        if !left && !right {
       105	            IdNode::Full
       106	        } else {
       107	            IdNode::Internal { left, right }

Resolution: Restate the `# Panics` and the assert message: an `Internal` tag has a present child by the coding (`00` is the terminal), and every present child's subtree holds a `Full` terminal, so the root distance is always finite; reaching the arm is programmer error in the fold, never an input. Keep the assert; keep the one genuine panic (non-canonical id bits through `bits.bit`). Acceptance: neither the rustdoc nor the message claims a normal-form trigger.

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

### skyline-fill-grow-22: "forest parent" and "site forest" are used as terms without a definition
- Where: crates/before/src/version/skyline/fill/memo.rs:21-25 (related: crates/before/src/version/skyline/fill/memo.rs:42; crates/before/src/version/skyline/fill/prescan.rs:79-82, 94-96, 310, 374; crates/before/src/meter.rs:1415; crates/before/tests/meter.rs:8924)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn -E 'forest parent|site forest' src tests`: nine occurrences, none defining); executed: no
- Seen by: prose (28); refutation: confirmed; history: no-rationale-found (entered with 5f6af246 undefined; the Wave 7 vocabulary passes anchored other terms and did not touch this one)
- Owner-gated: no

The ledger's reference discipline rests on "forest parent" and "the site forest's nesting", and nowhere is the forest named by contrast: that the left-full sites a scan records nest (a site can sit inside another's sibling range), forming a forest whose parent relation is enclosure. Every coined term is anchored to an identifier or defined once by contrast at its definition site.

Evidence:

        21	//! - A site with an earlier sibling under the same forest parent stores
        22	//!   `m_s − m_prev`, the difference against that sibling's minimum.
        23	//! - A forest parent's first child stores `m_s − m_parent`, written at the
        24	//!   *parent's* own close — when the parent's minimum is final — into the
        25	//!   child's earlier queue slot.

Resolution: One sentence before the bullets at memo.rs:17: the sites a scan records nest (a site can sit inside another's sibling range); that nesting is the site forest, and a site's forest parent is the innermost site enclosing it. Acceptance: the first occurrence of "forest parent" in memo.rs follows its definition.

### skyline-fill-grow-23: `Memo::set_link`'s `u32` link cap is a production panic reachable by input scale, undisclosed at the public tick
- Where: crates/before/src/version/skyline/fill/memo.rs:133-139 (related: crates/before/src/version/skyline/fill/prescan.rs:43-45; crates/before/src/version/skyline/fill.rs:202-208, 241-246; crates/before/src/version/skyline/fill/memo.rs:96-97; crates/before/src/version.rs:161-182; crates/before/src/meter.rs:993-1009)
- Class / severity / confidence: correctness / low / medium
- Provenance: assessed (read `set_link`; read `memo_chain(k, distinct)` and `memo_chain_id(k)`, canonical for every `k`, yielding `k` distinct nonzero links under one covering site; read `Version::tick`'s rustdoc, which has no `# Panics` section); executed: no
- Seen by: correctness (32), claims (42); refutation: confirmed (not constructible here: hundreds of GB of link store); history: deliberate-and-holds for the mechanism (055f2e48 pinned the compactness trade and the const assert; 3b883dd3 states the policy that a width-capped counter fails loudly at its cap; 05d87e1b scoped itself to `usize`-derived caps); the public-door disclosure is the unaddressed residue
- Owner-gated: yes (a documented design decision: compactness vs. cap)

The frame ledger indexes its nonzero links behind a `NonZeroU32` and panics via `expect` when one fresh pre-scan records more than `2^32 − 1` nonzero links. The trigger is a canonical stream and a canonical party (about 13 GB of packed input on the `MemoChain(distinct)` layout), not programmer error, and the `expect` message is an assumption about input size rather than a one-line proof. The cap is disclosed only in prescan.rs's private module doc; `fill::tick`'s `# Panics` names only non-canonical bytes and the empty id, and the public `Version::tick` has no `# Panics` at all. The doctrine's tolerable-unreachability exemption covers `2^64`-iteration corners, not `2^32`-site inputs; the memory-pricing argument (roughly 400 GB of link store before the cap) is real but is the kind of argument the crate's own `u64` denomination work declined to rest on.

Evidence:

       133	    pub(super) fn set_link(&mut self, slot: usize, link: Accumulator) {
       134	        // Push first: the store's length is then provably a valid, nonzero
       135	        // 1-based index (the `expect` is the u32 capacity contract alone).
       136	        self.links.push(link);
       137	        let index = u32::try_from(self.links.len()).expect("site count fits u32");
       138	        self.queue[slot] = NonZeroU32::new(index);
    (prescan.rs)
        43	//! containers. The ledger's capacity contract is different in kind:
        44	//! stored-link indices fail loudly at their `u32` cap
        45	//! ([`Memo::set_link`]). The fill walk's near-synonymous `depth` (the

Resolution: Owner's call between (a) widening the index to `Option<NonZeroUsize>` (8 bytes per queue cell on 64-bit, still niche-packed; update the const assert at memo.rs:97 and the "one index-sized cell per site" claim; re-pin the memo rows' heap columns with attribution) so the only cap is allocatable memory, and (b) keeping the cap and stating it as a capacity contract in `fill::tick`/`ticks`'s `# Panics`, with `Version::tick` and `Party::tick` pointing at it. Either way, the `expect` message should name the capped quantity: "nonzero link count fits u32". Acceptance: no `u32::try_from(...).expect` on the ledger path, or the public tick doors' docs name the cap.

Construction: Build `Shape::MemoChain.packed_flagged(k, true)` × `Shape::MemoChainId.packed1(k)` at `k = 2^32` (each interior site has the distinct minimum `j`, so every sibling link is nonzero and every site lands in one fresh scan under the root covering site), decode both through the public doors, and call `Version::tick`; the `2^32`-th `set_link` panics with "site count fits u32". Needs roughly 13 GB of packed input and hundreds of GB of heap; stated to fix reachability, not as a test to commit.

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

### skyline-fill-grow-25: The paired id-times-event walk skeleton is spelled twice (`FillWalk::walk` and `PreScan::run`)
- Where: crates/before/src/version/skyline/fill/prescan.rs:155-285 (related: crates/before/src/version/skyline/fill.rs:431-623; crates/before/src/version/skyline/fill/prescan.rs:136-154)
- Class / severity / confidence: modularity / low / medium
- Provenance: assessed (read both walks arm for arm: the `id.read()` prologue with Empty/Full/Internal, the leaf arm with two conditional skips, the left-full peek and site push, the ordinary-node push with the absent-left inline copy, and the Site/AwaitLeft/AwaitRight ascend with the right-full peek and absent-right copy recur in both; only the leaf actions and the depth-vs-level counters differ); executed: no
- Seen by: structure (11); refutation: confirmed; history: no-rationale-found (the twin relationship is deliberate and stated; no commit, note, or comment records evaluating a shared driver)
- Owner-gated: yes (a design proposal that moves readings)

The pre-scan's correctness argument is "same reads, virtual emissions" (prescan.rs:137); today that sameness is prose and a careful diff, not structure, so a new arm or a reordered peek must be mirrored by hand. The honest steelman: the actions differ substantially (consume plus emit plus route fold plus memo consume versus virtual emission plus record), the two counters are different notions, and a visitor trait would need on the order of eight hooks. skyline-fill-grow-27 (the shared frame type) is the adoptable-now half.

Evidence:

       136	    /// The pre-scan image of the walk's arms over the subtree at the cursor:
       137	    /// same reads, virtual emissions; returns the range end.
       138	    ///
       139	    /// The iterative twin of the fill walk: the descend phase resolves the

Resolution: After the shared frame type lands, construct a `PairedWalk` driver parameterized by a visitor (hooks: empty region, full region, leaf under id node, left-full site open/close, ordinary node left/right/absent), measure the envelopes, and keep two spelled-out walks if the hook count makes the driver less legible than the twin files. Acceptance: either one driver with two visitors and the "same reads" claim structural, or a recorded decision to keep the twins with the frame types shared.

### skyline-fill-grow-26: `record`'s `while head_level > level` runs at most once
- Where: crates/before/src/version/skyline/fill/prescan.rs:324-328 (related: crates/before/src/version/skyline/fill/prescan.rs:329-360, 372-412; crates/before/src/version/skyline/fill.rs:526)
- Class / severity / confidence: simplification / nit / medium
- Provenance: verified by hand trace (every arm of `record` leaves `head_level == level`: the sibling arm keeps it, the suspend arm sets it at 359, and the resolve loop runs until one of those applies; records are post-order over the site forest, so the record before a level-`L` site's own is its last direct child at `L + 1` or a site at or below `L`; hence `head_level <= level + 1` at every entry, the root record at fill.rs:526 included, and `resolve_inner` runs zero or one times; checked on the `memo_comb` layout, where each `X_i` resolves exactly one level); executed: no
- Seen by: correctness (38); refutation: confirmed by its own trace; history: no-rationale-found (landed with the ledger in 5f6af246 without a stated cascade depth)
- Owner-gated: no

The `while` reads as a multi-level cascade the site forest's structure rules out, and recovering that fact costs every reader a full trace. Finished kernel code should be obviously, reviewably correct; a loop whose body runs at most once hides a structural invariant behind generality. The `while` is total and correct as written, so this is a legibility choice, not a defect.

Evidence:

       324	        // A deeper level is complete iff the head still serves it: its forest
       325	        // parent is THIS site, whose minimum is final now.
       326	        while self.head_level > level {
       327	            self.resolve_inner();
       328	        }

Resolution: Either `debug_assert!(self.head_level <= level + 1, "the enclosed level records before its parent"); if self.head_level > level { self.resolve_inner(); }` with the one-level argument in the comment, or keep the `while` and add the sentence "at most one level is deeper: a level-`L + 2` site closes inside a level-`L + 1` site, which records before this one". Acceptance: the comment states the bound; the memo families and fill differentials stay green.

### skyline-fill-grow-27: `Frames`/`PreFrames` and `Frame`/`PreFrame` duplicate the suspended-ancestor control-bit discipline
- Where: crates/before/src/version/skyline/fill/prescan.rs:602-695 (related: crates/before/src/version/skyline/fill/prescan.rs:590-600; crates/before/src/version/skyline/fill.rs:1129-1140, 1179-1297)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (side-by-side read: `top()` at prescan.rs:634-647 and fill.rs:1220-1233 are byte-identical apart from the enum name, as are `aux_top`, the three-bit push in `push_node`/`push_site`, `flip_to_await_right`'s debug assert plus `set_last`, and the triple pop; `PreFrame` (592-600) and `Frame` (1130-1140) are the same three variants); executed: no
- Seen by: structure (0); refutation: confirmed (severity lowered to low: the shared portion is roughly 45 lines, the payload halves legitimately differ, and nothing forces the two stacks to agree); history: no-rationale-found (both stacks were born in 05bd2b16; 055f2e48 extracted the one shared idiom it noticed, `DeltaReg`, and left the control-bit triple spelled twice; prescan.rs:602-604 itself asserts the sameness)
- Owner-gated: no

The three control `BitStack`s and their `top`/`aux_top`/`push`/`flip`/`pop` discipline are one type spelled in two files that document themselves as twins; only the value-stack payload differs (route keys plus deferred costs versus ledger slot deltas). A change to the frame encoding must be made twice, with the invariants ("`phase` unread on site frames", one aux bit per frame) documented twice. I keep medium against the refutation's low: the extraction is clean, behavior-preserving (the same bits are pushed), and it is the partition's largest duplication; "nothing forces the two to agree" is the maintenance hazard, not a mitigation.

Evidence:

       602	/// The pre-scan's suspended ancestors, held as bits: the fill walk's stack
       603	/// shape with the site payload (its ledger slot) as a pop-able word delta in
       604	/// place of route keys and costs.
       634	    fn top(&self) -> Option<PreFrame> {
       635	        let site = self.site.last()?;
       636	        Some(if site {
       637	            PreFrame::Site
       638	        } else if self
       639	            .phase
       640	            .last()
       641	            .expect("site and phase stack one bit per frame")
    (fill.rs)
      1220	    fn top(&self) -> Option<Frame> {
      1221	        let site = self.site.last()?;
      1222	        Some(if site {
      1223	            Frame::Site
      1224	        } else if self
      1225	            .phase
      1226	            .last()
      1227	            .expect("site and phase stack one bit per frame")

Resolution: Extract the control bits into one type in fill.rs, e.g. `FrameBits { site, phase, aux: BitStack }` with `len`, `top() -> Option<Frame>`, `aux_top`, `push(site: bool, aux: bool)`, `flip_to_await_right`, `pop`, and one `Frame` enum. `Frames` composes it with `values: PopStack` plus `keys: DeltaReg` and keeps the cost encode/decode; `PreFrames` composes it with `values` plus `slots`. Acceptance: one `Frame` enum and one `top()` in the fill module; `PreFrame` and the duplicated bodies gone; `just gate` clean with no envelope movement.

### skyline-fill-grow-28: `pop_site`'s comment gives the wrong reason the slot cast is lossless
- Where: crates/before/src/version/skyline/fill/prescan.rs:691-693 (related: crates/before/src/version/skyline/fill/prescan.rs:287-291, 662-667; crates/before/src/version/skyline/fill/memo.rs:33-35, 132-139)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (memo.rs:33-35 and prescan.rs:289-291: every reserved site pushes a queue cell, zero-link sites included; memo.rs:136-137 caps only `links.len()`; the slot is pushed as `slot as u64` at 663); executed: no
- Seen by: prose (29); refutation: confirmed; history: no-rationale-found (written in 5d167a63 during the `BitsView` migration alongside the cast)
- Owner-gated: no

The `as usize` on a popped slot is justified by "the ledger's own link-storage contract". `set_link` caps the nonzero-link count; slots are `queue` indices, and zero-link sites reserve a slot without a link, so the link cap does not bound slots (they are bounded only by memory). The cast is lossless for a different reason: a slot is a `Vec` index pushed as `u64` and popped back. A comment stating a false reason is worse than none. The `expect` at memo.rs:137 has the sibling slip: "site count" where the capped quantity is the nonzero-link count.

Evidence:

       691	        // Ledger slots are queue indices, capped far below `u32::MAX` by the
       692	        // ledger's own link-storage contract.
       693	        self.slots.pop(&mut self.values) as usize
       663	        self.slots.push(&mut self.values, slot as u64);
    (memo.rs)
       137	        let index = u32::try_from(self.links.len()).expect("site count fits u32");

Resolution: "A slot is a `queue` index pushed as `u64` at `push_site`, so the round trip to `usize` is exact." At memo.rs:137: `expect("nonzero link count fits u32")`. Acceptance: the comment names the `Vec`-index round trip; the message names the capped quantity.

### skyline-fill-grow-29: Two line-wrap typos split compound modifiers
- Where: crates/before/src/version/skyline/fill/tests.rs:1005-1005 (related: crates/before/src/version/skyline/grow.rs:73)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n -- '- sibling'` over the partition returns exactly these two lines); executed: no
- Seen by: prose (31); refutation: confirmed; history: rewrap artifacts (d3a029d4)
- Owner-gated: no

A stray space after a hyphen at a wrap point splits "nested-full-sibling" and "pending-sibling".

Evidence:

      1005	/// canonical uniqueness. The nested-full- sibling id over its matched spine
    (grow.rs)
        73	//! the pending- sibling path bits, and the builder's per-level stacks — no node

Resolution: "nested-full-sibling id"; "pending-sibling path bits". Acceptance: the grep returns nothing.

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

### skyline-fill-grow-32: `Cost::deepen`'s test-seam parameter is spelled three ways across its callers
- Where: crates/before/src/version/skyline/grow.rs:131-149 (related: crates/before/src/version/skyline/fill/fuse.rs:306, 342-343, 457; crates/before/src/version/skyline/grow/tests.rs:198-204; crates/before/src/oracle/version.rs:14-22; crates/before/src/version/skyline/fill/tests.rs:1378-1386)
- Class / severity / confidence: scaffolding / nit / high
- Provenance: verified (`grep -rn 'deepen(' src`: fuse.rs:306, 342, 343 and grow/tests.rs:198-204 pass `Cost::CEILING`; fuse.rs:457 forwards `expand_subtree`'s parameter, whose only non-`CEILING` caller is `assert_chain_saturation` at fill/tests.rs:1424; oracle/version.rs:20-22 wraps the two-argument form in a one-argument `deepen`); executed: no
- Seen by: structure (8); refutation: reframed (the parameter is a documented deliberate test seam at both ends, inside the doctrine's sanctioned exception; the residue is the oracle wrapper hiding it at one site); history: deliberate-and-holds (fc06c5e8 named the scaled-sentinel family as the point; the wrapper landed in the same commit)
- Owner-gated: no

The `ceiling` parameter is documented at the declaration (140-142) and at the check site (fill/tests.rs:1383-1386), so it is a sanctioned deliberate seam, not a circular-justification breach. Its callers spell it three ways: `Cost::CEILING` inline, forwarded from `expand_subtree`, and hidden behind the oracle's one-argument `deepen`. One seam, one spelling.

Evidence:

       140	    /// one function; production callers pass
       141	    /// [`CEILING`](Self::CEILING) — the parameter exists so the saturation
       142	    /// tests can scale the bound into constructible range.
    (oracle/version.rs)
        20	fn deepen(component: u64) -> u64 {
        21	    RouteCost::deepen(component, RouteCost::CEILING)
        22	}

Resolution: Delete the oracle's wrapper in favor of the two-argument call at its seven use sites, so every production-adjacent caller shows the seam. Acceptance: `grep -rn 'deepen(' src` shows `Cost::CEILING` or the forwarded `ceiling` at every call.

### skyline-fill-grow-33: `EvScan::skip` is a `#[cfg(test)]` method in the production file with one test consumer
- Where: crates/before/src/version/skyline/grow.rs:270-287 (related: crates/before/src/version/skyline/grow/tests.rs:145-149; crates/before/src/version/skyline/grow.rs:212-216; crates/before/src/version/skyline/fill/tests.rs:1440-1447)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (`grep -rn EvScan` shows no use outside grow.rs and grow/tests.rs; `grep -rn '\.dirs()'` shows grow/tests.rs:90-91 and fill/tests.rs:1443); executed: no
- Seen by: structure (12); refutation: reframed (`Route::dirs` cannot be replaced by field access: fill/tests.rs:1443 calls it and `fill::tests` is not a descendant of `grow`, so the `#[cfg(test)]` getter is the narrowest exposure and stays; `EvScan::skip` is movable, since `grow::tests` is a child module and an inherent `impl EvScan<'_>` there reaches the private `cursor`); history: deliberate-but-expired (`skip` was production code at 183e3cab; its production consumer left with the probe in 78206426 and it was cfg-gated in place)
- Owner-gated: no

Tests live in sibling files; a `#[cfg(test)]` method in the production file is the small version of an instrument hook left in a production path, and this one also hand-rolls the pending-children counter `idbits::skip_subtree` owns.

Evidence:

       277	    #[cfg(test)]
       278	    fn skip(&mut self) {
       279	        // One unary read per descent: each internal node in the run opens two
       280	        // children, and the terminating leaf closes one.
       281	        let mut pending = 1u64;
       282	        while pending > 0 {

Resolution: Move `skip` into an `impl EvScan<'_>` block in grow/tests.rs (or express it via `skip_subtree` if the batched unary read is not needed there). Leave `Route::dirs` as is. Acceptance: no `#[cfg(test)]` method on `EvScan` in grow.rs; tests green.

### skyline-fill-grow-34: grow.rs re-spells `IdReader`'s cursor as `id_tag`/`id_skip` over a bare position
- Where: crates/before/src/version/skyline/grow.rs:290-306 (related: crates/before/src/version/skyline/grow.rs:534-607; crates/before/src/version/skyline/grow/tests.rs:116-184; crates/before/src/idbits.rs:12-13, 92-96, 114-124, 145-156; crates/before/src/version/skyline/fill.rs:471)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`id_tag` is `IdReader::read`'s `record_bits(2)` plus two bit reads returning bools; `id_skip`'s closure is `IdReader::skip`'s closure verbatim; `grep -rn 'id_tag|id_skip|EvScan'` finds uses only under grow; `IdReader::at` exists at idbits.rs:94; fill.rs:471 derives the same route key from `id.pos() - 2`); executed: no
- Seen by: structure (1); refutation: confirmed (severity low: a dozen lines whose meter accounting the envelope rows already pin equal on both halves); history: deliberate-but-expired (183e3cab needed random access to re-read child presence at a suspended frame's stored key; 78206426 fused the probe into the fill walk, leaving `emit` and the test probe purely linear; `IdReader::at` predates grow.rs by seven weeks)
- Owner-gated: no

`emit` threads a bare `id_pos: u64` with manual `+= 2` where the fill walk threads an `IdReader` and reads `id.pos()`; the id tag reading and its scan-meter accounting are spelled in idbits.rs and again here. The reason for the raw spelling expired with the fused probe. `id_tag`'s doc also attributes to canonicity ("a canonical id has no `(0, 0)` node") what the coding makes unrepresentable (idbits.rs:12-13), the same slip as skyline-fill-grow-20; deleting the function removes it.

Evidence:

       292	/// Neither present is the full `1` terminal; a canonical id has no `(0, 0)`
       293	/// node. `O(1)` random access into the packed id.
       294	fn id_tag(bits: BitsView<'_>, pos: u64) -> (bool, bool) {
       295	    codec::scan::record_bits(2);
       296	    (bits.bit(pos), bits.bit(pos + 1))
       297	}
       300	fn id_skip(bits: BitsView<'_>, pos: u64) -> u64 {
       301	    crate::idbits::skip_subtree(pos, |at| {
       302	        codec::scan::record_bits(2);
       303	        let children = u64::from(bits.bit(at)) + u64::from(bits.bit(at + 1));
       304	        (children, at + 2)
    (idbits.rs)
       148	            *pos = skip_subtree(*pos, |at| {
       151	                crate::codec::scan::record_bits(2);
       152	                let children = u64::from(bits.bit(at)) + u64::from(bits.bit(at + 1));
       153	                (children, at + 2)

Resolution: Have `emit` and grow/tests.rs's `rec` take an `IdReader`: `let key = id.pos(); match id.read() { IdNode::Full => .., IdNode::Internal { left, right } => .., IdNode::Empty => unreachable!(..) }` and `id.skip()` in place of `id_pos = id_skip(id_bits, id_pos)`; delete `id_tag` and `id_skip`. The expansion-chain loop's `current = (key, left_present, right_present)` tuple threading (577-602) collapses to reading the tag at the loop head. Acceptance: `id_tag`/`id_skip` gone; the route differential and both grids green with `FAMILY_GROW_PAIRS`/`EXHAUSTIVE_GROW_PAIRS` unchanged; scan-meter tick envelopes unchanged (both spellings record 2 bits per tag).

### skyline-fill-grow-35: `recode` spells its zero-crossing test two ways; both grow.rs mutants exclusions are the symptom
- Where: crates/before/src/version/skyline/grow.rs:436-476 (related: .cargo/mutants.toml:15-24, 131-142; crates/before/src/codec/base.rs:31-35, 284-289, 416-423)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read the two arms; read .cargo/mutants.toml:133-142's two `recode` exclusions and its header's disposition ladder; `Base::cmp`, `eq`, and `sub` are limb-metered via `meter_limbs2` while `Base::bits()` is not); executed: no
- Seen by: structure (3); refutation: confirmed (both equivalences hold; unifying applies the width prefilter to the down-delta arm too, which lowers limb readings on that path and needs attribution when re-pinned); history: split (the width prefilter is deliberate and its rationale holds inline, 4cc9b995: the k = 1 path stays byte-identical on every meter column; the two-spellings shape and the exclusions have no recorded step-(1) refactor attempt, which the roster's header requires before any entry)
- Owner-gated: no

The two crossing arms both compute `zigzag_signed(sign.negate(), events − magnitude)` and the non-crossing arm computes `magnitude − events` with a zero-to-positive normalization; the crossing test is a plain metered `<` in one arm and a width-prefiltered `<` in the other. .cargo/mutants.toml excludes two `recode` mutants as proven equivalent, both at the boundaries this redundancy creates. The roster's standing policy is to refactor the mutated codepoint out of structural existence first; a three-way `Ordering` match has no `<`/`>` operator to mutate, so both exclusions dissolve. The width prefilter must survive in one place because `Base::cmp` is limb-metered and `Base::bits()` is not.

Evidence:

       445	        (false, Sign::Positive) if magnitude < *events => {
       458	        (true, Sign::Negative) | (false, Sign::Positive) => {
       459	            let crosses = sign.is_negative()
       460	                && (magnitude.bits() < events.bits()
       461	                    || (magnitude.bits() == events.bits()
       462	                        && events.bits() > 1
       463	                        && magnitude < *events));
    (.cargo/mutants.toml)
       137	    "grow\\.rs.*: replace > with >= in recode",
       142	    "grow\\.rs:463:38: replace < with <= in recode",

Resolution: Two top-level arms, same-sign (`magnitude + events`) and opposing-sign; the latter `match width_first_cmp(&magnitude, events) { Less => zigzag_signed(sign.negate(), events.clone() - &magnitude), Equal => zigzag_signed(Sign::Positive, Base::ZERO), Greater => zigzag_signed(sign, magnitude - events) }`, where `width_first_cmp` compares `bits()` first and falls through to `cmp` only on a width tie. Keep the `magnitude.bits() == 0` shortcut inside the `Less` arm if the measured limb touches need it. Delete both `grow\.rs` entries from .cargo/mutants.toml. Acceptance: one spelling of the crossing test; both exclusions gone and `just mutants-list` clean; tick rows re-measured at the parent and at the change, any movement recorded as an attribution.

### skyline-fill-grow-36: The splice builder's capacity hint omits the count's width
- Where: crates/before/src/version/skyline/grow.rs:511-513 (related: crates/before/src/version/skyline/grow.rs:644-668; crates/before/src/codec/build.rs:60-72; crates/before/tests/meter.rs:834)
- Class / severity / confidence: performance / nit / medium
- Provenance: assessed (read `PackedBuilder::with_capacity`, which sizes the `Vec` to `capacity / 8 + 1`; read the two count-carrying codes at 645/654 and 662; `TICKS_WIDE_COUNT_BITS = 8_192`); executed: no
- Seen by: claims (44); refutation: confirmed (not measured); history: deliberate-but-expired (the hint and comment were written for the +1 splice in 183e3cab; 4cc9b995's +k generalization changed exactly the two codes to carry `±k` and did not revisit them)
- Owner-gated: no

The hint is input plus `64`, argued as "a few bits per id level". The output also carries two count-coded deltas (`+k` at the grown leaf and `−k` at its successor, or the chain's `±k` fresh leaves), each about `2·bits(k) + 1` bits, so at the wide-count flatness pin's 8,192-bit counts the output exceeds the hint by about `4·bits(k)` and the byte `Vec` reallocates once, doubling at peak. Fixed-sign correction; the comment's bound also omits a term the code reaches.

Evidence:

       511	    // Subadditivity of the coding bounds the output by the input plus the
       512	    // expansion chain's fresh codes, each a few bits per id level.
       513	    let mut out = SkylineBuilder::with_capacity(event_bits.len() + id_bits.len() + 64);

Resolution: Widen the hint by `4 * events.bits() + 4` and extend the comment: input, plus a few bits per id level of the chain, plus the two count-carrying codes. Acceptance: on the `ticks_wide_count_flatness` cases the builder's byte `Vec` does not reallocate after the initial reservation (an unchanged peak-heap reading with the doubling gone, or a `capacity()` probe before and after in a debug check).

### skyline-fill-grow-37: `assert_grow` drops the ticked stream, so two tests re-tick and re-inline the oracle comparison
- Where: crates/before/src/version/skyline/grow/tests.rs:59-72 (related: crates/before/src/version/skyline/grow/tests.rs:304-343, 445-460)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read the three sites); executed: no
- Seen by: structure (13); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`assert_grow` returns `bool` and drops the stream `assert_grow_depth_safe` returned, so `exhaustive_small_scope_grows_identically` re-inlines the oracle comparison against `assert_grow_depth_safe`'s output and `arbitrary_pairs_grow_identically` calls `assert_grow` and then runs `tick(..)` a third time per pair to compare with the brute force. Duplicated assertion bodies in test code drift independently.

Evidence:

        59	fn assert_grow(v: &Version, p: &Party) -> bool {
       451	        if assert_grow(&v, &p) {
       452	            let (best, _) = best_inflation(&op, &ov).expect("an owning id always inflates");
       453	            let minimal = from_oracle_version(&best.normalized_for_test());
       454	            prop_assert_eq!(
       455	                tick(crate::codec::built_view(&encode(&v)), &p),

Resolution: `assert_grow(v, p) -> Option<BitsBuf>` (`None` on the fill branch); the exhaustive test and the proptest use the returned stream for the brute-force comparison; the count pins use `.is_some()`. Acceptance: one oracle-comparison body in grow/tests.rs; no second `tick(..)` in `arbitrary_pairs_grow_identically`.

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

## Positives

- The changed flag realized as an output mode (`Out::Unstarted`/`Verbatim`/`Built`, fuse.rs:78-114): the unchanged branch does zero output work, the first divergence materializes the prefix once, and the absolute-vs-delta first-leaf decision is carried by the state itself rather than a side flag; the `large_enum_variant` allow at 85-88 states its cost argument at the site.
- `Cost` derives `Ord` with the field order spelling the lexicographic rule and the doc saying so (grow.rs:99-114); `CEILING < INFEASIBLE` is pinned by a const assert with the no-aliasing argument and the reason a runtime test cannot probe it (grow.rs:166-171).
- Layout claims under the compiler, not prose: `OUT_FOLLOWER`/`REL_FOLLOWER` bound to `watermark::FOLLOWER_SLOTS` with the cross-file roster named (fill.rs:182-188); the `Option<NonZeroU32>` niche behind the per-site cost claim (memo.rs:96-97).
- Types over asserts: `Step`'s absent variant is argued from the types ("the variant does not exist, rather than being asserted away", grow.rs:405-410); `Relation`'s tag/slot invariant is stated with the `expect` a violation trips (fill.rs:394-401); `Repair` names the boundary semantics instead of an `Option<&Base>`.
- grow/tests.rs pins its grow-branch pair counts exactly (`FAMILY_GROW_PAIRS = 182`, `EXHAUSTIVE_GROW_PAIRS = 114_621`, 255-290) with a failure message that tells the reader what to confirm before re-deriving: the liveness-floor discipline applied to a differential, and the sanctioned form of a hand-maintained count.
- Witness tests bind themselves to the arm they exist to drive through the emit-traffic decision counter (`dominated_undercut >= 1`, fill/tests.rs:467-471, 599-603) and say why `>=` rather than `=`.
- `materialize_is_a_noop_once_built` documents its internal entry as a deliberate decision at the check site in exactly the doctrine's form (fill/tests.rs:401-403).
- Both walks are iterative on bit stacks with `DeltaReg` word deltas on a `PopStack`, the transient cost stated where it lives (fill.rs:1142-1148, 1179-1184); the depth-4096 closed-form witnesses cover the memo, reveal, cliff, staircase, nested-full, mirror, and spike regimes with each derivation stated in the test doc (fill/tests.rs:986-1027).
- Every `expect`/`unreachable!`/`debug_assert!` message in the partition is a one-line proof and none carries an em-dash (verified by grep); the release `assert_ne!` in `expand_subtree` states in its `# Panics` why a fabricated cost would be worse than a panic.
- The vocabulary caution in `PreScan::run` (prescan.rs:142-146) naming the `level` vs `depth` collision between the twin walks is a model maintainer note.
- `grow::emit`'s loop-not-helper comment (grow.rs:528-533) steelmans its own shape by naming the six pieces of state a helper boundary would thread.
- The debug-only ledger check is O(1) state (the FNV-style `position_check` pairing recorded and consumed positions, memo.rs:99-103) rather than a position buffer that would bill the heap meter for debug scaffolding.
- Width tests kept off the limb meter with the reason at the site (fill.rs:248-254; grow.rs:503-508), so dev and release readings agree.

## Open questions for Finch

1. Heap on the distinct-minima memo families (skyline-fill-grow-2). Should `MemoChain(distinct)` and `MemoComb` join the board's tick group under the 16 B/B ceiling, or get a heap column in `memo_resolution_cost` with a declared model? Recommendation: measure first through `memo_resolution_cost` (cheapest instrument), then decide between a declared model and the word-compaction cure by the reading; I expect the reading to be red against 16 B/B and the cure to be the `Boundary::Word | Wide` trade `MinWeb` already makes.
2. The `u32` link index (skyline-fill-grow-23). Widen to `Option<NonZeroUsize>` (8 bytes per queue cell, no cap) or keep the 4-byte cell and state the cap in `fill::tick`'s and `Version::tick`'s `# Panics`? Recommendation: widen; it touches no public surface, the memo rows' heap columns re-pin with attribution, and the `expect` on the walk path disappears.
3. Kernel-doc citations (skyline-fill-grow-17). Extend `citecheck` to backticked test names in `src/version/skyline/**` comments, or reduce kernel docs to module-level pointers? Recommendation: extend citecheck; the citations are useful navigation and the tool already exists.
4. "mint" (skyline-fill-grow-21). Is watermark.rs's latent-register sense a term of art to keep (then link it at first use in fill.rs and fill/tests.rs) or to rename crate-wide? Recommendation: rename in watermark.rs ("a boundary moves into the latent register" already says it) and fix the three plain-sense uses here regardless.
5. The orbit bound (skyline-fill-grow-31). Is the intended lemma `4·⌈log2(k + 1)⌉` (the landing commit's and the spec's statement) or `4·bitlen(k + 1)` (what the code asserts)? Recommendation: try the tighter form once (the construction in the finding); if it passes, keep the doc and tighten the code.
6. The `# Panics` form (skyline-fill-grow-4). walk.rs's truncation/malformation-vs-silent form cites `causal_cmp` as the one statement; if you agree it is the crate's form, the sweep is crate-wide (the `EvScan` methods and any other kernel still carrying "Panics if ... not canonical"). I have not surveyed the other kernels.
7. `PreScan::max_range`'s arm-by-arm proof at the use site (dropped below). The 0f13590b ruling put the distilled call-chain proof at the use site; the doctrine's rule against hand-maintained caller enumerations says such proofs rot when an arm changes. I verified the chain still matches `run()`'s arms at HEAD. Does the ruling stand, or should the chain move to `prescan_raise_shapes`'s module doc, which already names itself the invariant's negative space? Recommendation: move it; the debug assert keeps the invariant and the test module keeps the proof.
8. Em-dashes in `//` comments (skyline-fill-grow-10). The rule is yours and postdates the prose; 374 sites crate-wide. One mechanical pass or leave the crate's comment style as is? Recommendation: one pass, since the rule's reason (terminal rendering) applies uniformly.
9. From the claims lens, not filed as a finding: the memo and width-circulation envelope modules discriminate linear from the refuted quadratic at ×2.5 per doubling (exponent ≤ 1.32), looser than the board's 1.15, and those families never see the board's exponent. Deliberate calibration to the named known-bad mechanism, or would you want them under the board's exponent too? Recommendation: leave as is unless the families join the board (question 1), at which point the board's exponent judges them anyway.
10. From the claims lens, not filed: `ticks`'s changed branch runs a full second fused walk over the filled output solely to record the grow route. The nested/mirror-wide rows show the second walk's touch cost is small on those shapes; a route-only pass is a workload-dependent trade. Is there a shape where the second walk's accumulator work is not small? Recommendation: no action unless such a shape is constructed.

## Dropped

- max_range's doc justifies its debug assert by enumerating call sites (candidate 23): deliberate and recorded (0f13590b, "the use site carries the distilled proof"), the chain verified accurate against `run()`'s arms at HEAD; reopening the ruling needs new evidence, so it is raised as open question 7 instead.
- "parks no wide quantity per open site" reads against SuspendedLevel's two accumulators (candidate 30): merged into skyline-fill-grow-2 as its documentation half.
- prescan.rs says depth stays usize (candidates 34, 40): duplicates of skyline-fill-grow-24.
- Module doc pins measured exponents / measured exponents as unbound snapshots (candidates 37, 41): duplicates of skyline-fill-grow-1 (41's extra site at fill/tests.rs:1302-1304 folded in).
- The memo's u32 link index (candidate 42): duplicate of skyline-fill-grow-23.
- fill::tick's Panics documents an empty-id case (candidate 43): duplicate of skyline-fill-grow-5.
- grow/tests.rs's pools are a strict subset of fill's (candidate 36): duplicate of skyline-fill-grow-38.
- Route::dirs as a test-only member to delete (half of candidate 12): refuted; fill/tests.rs:1443 needs the accessor and `fill::tests` cannot see `grow`'s private field, so the `#[cfg(test)]` getter is the narrowest exposure.
- "type boundary" misnames the decode check (half of candidate 25): refuted; `Party`'s only constructors route through `finish_id`/decode, so "the type boundary" is a reasonable name for that gate.
- The ceiling parameter as circular justification (candidate 8's framing): refuted; the seam is documented at the declaration and the check site, the doctrine's sanctioned form. The oracle wrapper's inconsistency survives as skyline-fill-grow-32.
- The fill module doc restates the memo discipline six times (candidate 22's framing): reframed; the memo appears once per cost axis, which the Cost structure requires. The `# Testing` duplication survives as skyline-fill-grow-3.
- The heap paragraph contradicts SuspendedLevel (candidate 39's doc framing): reframed; the sentences reconcile by scoping "its frames" to `PreFrames`. The instrument gap survives as skyline-fill-grow-2.
- The recursion-argument item of the texture sweep (part of candidate 27): not dropped but split out as skyline-fill-grow-7, since it is a factual ghost rather than register.
