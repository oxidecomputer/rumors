# Partition fuzzfit-strategies: The fuzz-fit harness: input strategies, operation table, and the wasm driver loop

## Partition summary

This partition is the input side of the fuzz-fit instrument: `ops.rs` defines the program vocabulary (`Op`, 44 variants, each naming one guest kernel by string), the native `Mirror` that executes a program over the same register discipline the wasm guest uses and predicts each step's return code and denominated size, and the denomination rules; `strategies.rs` defines the budgets, the family roster (`Family`, eighteen coupled archetypes plus `Independent`, `Bootstrap`, and `Escalation`), the builder `B` that emits ops under unconditional budget caps, the per-family constructions, the measurement battery, and the proptest strategies `any_family`/`any_program`; `drive.rs` replays a program in the mirror and the guest in lockstep, panics on any disagreement, and enumerates the two deterministic corpora (the calibration stream and the bootstrap stream). The four build files detach the workspace from the parent, fix the guest's codegen profile, keep wasmtime's generated sources out of the doclint sweep, justify each dependency, and embed the building toolchain's identity.

The core mechanism is sound and well defended. The differential is total, not sampled: every step's return code and every live register's final bytes are compared and any disagreement panics. The mirror's identity predicates reproduce before's own dispatch exactly where they are mirrored (`version_buffers_alias` is pointer-plus-length, matching `Bits::ptr_eq`; `va == *vb` is `codec::canonical_eq` through `Version`'s `PartialEq`). Rejection arms are predicted per case by running the real operation natively and are priced as their own band key. Budgets are enforced structurally in the builder, pinned after the fact by the sanity suite, and tied to the guest's register reserve by a compile-time assert. The scope section names what the generators never construct and which instrument owns each excluded regime. Toolchain provenance is mechanical.

Four issues carry weight. First, the mirror models one of `join_view`/`meet_view`'s three O(1) rungs (canonical equality) and omits the empty-operand rungs, while the generators produce empty versions routinely and battery arm 6 feeds them to join and meet; the pinned `ff_version_join`/`ff_version_meet` floors are 2.79 and 2.84 decades wide against 0.38 to 0.87 on every other pair kernel, which is the smearing the exclusion exists to prevent and voids the liveness floor's claim on those two keys. Second, the guest exports 107 kernels and the vocabulary reaches 44: after the 7 control exports and 7 unmeasured query constructors, 49 measured public-operation kernels have no `Op`, no band, and no roster pin that can see them, while `ops.rs` and `lib.rs` claim one op per public operation. Third, the two fixed escalation replays are the suite's deterministic reach proof, but every budget refusal in the builder is silent and nothing witnesses that the depth-cap program was emitted whole; by hand count it fits on a 12 percent op margin no test reads. Fourth, the builder's `Ty`/`slots` liveness model is written at 17 `Ty::Dead` sites and 32 `alloc` sites and read by no code path except `.len()`, while three doc sentences describe a check it does not perform.

The rest is low-severity work: a second call path in the driver, a triplicated (and slightly overstated) identity-routing argument, a tag table restating `Op::kernel`, pins unbound to the corpus that produced them, hand-maintained counts, family docs that misdescribe their constructions, dead guards settled by an executed replay, duplicated mirror arms and family epilogues, hand-copied ABI constants, and a batch of idiom nits. Lines read: 3180 across the seven partition files, plus the cited ranges of `bands.rs`, `wasm.rs`, `lib.rs`, `bin/calibrate.rs`, `tests/sanity.rs`, `tests/enforce.rs`, the guest's `lib.rs`, before's `version.rs`, `party.rs`, `clock.rs`, `party/ops/split.rs`, `codec/bits.rs`, `version/rank.rs`, `tests/meter.rs`, `meter/registry.rs`, `laws.rs`, `testing/validation_index.rs`, `before-fuelscape/src/ops.rs` and its tests, the justfile, `ci.yml`, and the fuzzfit design note. No partition file is a test file; `tests/sanity.rs` and `tests/enforce.rs` were read as context and belong to another partition.

## Findings

### fuzzfit-strategies-1: The release profile's guest-only justification governs the harness build too
- Where: crates/before/fuzzfit/Cargo.toml:17-25 (related: justfile:575-577, justfile:597)
- Class / severity / confidence: performance / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: instrument-correctness; refutation: confirmed (nit stands; `panic = "abort"` is ignored for test targets, so the cost is build time only); history: no-rationale-found (the block is the 8cfd3c92 original)
- Owner-gated: yes: the profile is part of the pin's provenance and moving it touches the justfile's artifact path and bands.rs's provenance note

The comment justifies `[profile.release]` as guest codegen provenance, but the table sits at the workspace root and also governs `cargo build -p fuzzfit-harness --tests --release`, so wasmtime, cranelift, and proptest compile with one codegen unit for no measurement benefit (Principle 3: a setting whose stated beneficiary is the guest should be scoped to the guest).

Evidence:

        17	# The guest's profile is part of the measurement's provenance: fuel counts
        18	# instructions of THIS codegen. codegen-units = 1 keeps function layout
        19	# independent of build parallelism; panic = "abort" keeps unwinding tables
        20	# (and their instruction overhead) out of the kernels. Debug assertions stay
        21	# off so fuel prices the production work alone (the amp-board's release-only
        22	# rule, applied to instruction counts).
        23	[profile.release]
        24	codegen-units = 1
        25	panic = "abort"

    justfile:577	    {{ justfile_directory() }}/tools/memwatch cargo build -p fuzzfit-harness --tests --release

Resolution: introduce a guest-only profile (`[profile.guest] inherits = "release"` carrying both settings, built with `--profile guest`) or a per-package override for the heavy tool dependencies; move `fuzzfit_guest_wasm` in the justfile and the provenance note in bands.rs with it; measure `just fuzzfit-build` wall time before and after on a quiet machine. Acceptance: the guest wasm is byte-identical under the new profile and the harness build no longer compiles wasmtime single-unit.

### fuzzfit-strategies-2: `Op::returns_i64` and the `call_i64` branch are a second call path `Guest::call` already covers
- Where: crates/before/fuzzfit/harness/src/drive.rs:47-51 (related: crates/before/fuzzfit/harness/src/ops.rs:251-254, crates/before/fuzzfit/harness/src/wasm.rs:194-252, crates/before-fuelscape/src/ops.rs:404)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read `Guest::call`'s result match at wasm.rs:206-209, `call_i64`'s `args[0]` at wasm.rs:245, and fuelscape's untyped call of the two-argument i64 kernel `ff_shape_combine` at ops.rs:404; the guest declares `ff_shape_combine(src: u32, n: u32) -> i64` at guest/src/lib.rs:797); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no-rationale-found (both paths are coeval originals; `returns_i64` was added with the one-word doc "Typed convenience")
- Owner-gated: no

`Guest::call` already returns i64 results (it matches `Val::I64`, and fuelscape drives an i64-returning two-argument kernel through it), so `returns_i64` and the branch exist only to route one kernel through `TypedFunc<u32, i64>`, which reads `args[0]` and would drop further arguments for any future multi-argument i64 op (Principle 3: two paths for one capability, the narrower one carrying a latent arity trap).

Evidence:

        47	        let measured = if op.returns_i64() {
        48	            guest.call_i64(op.kernel(), &args)
        49	        } else {
        50	            guest.call(op.kernel(), &args)
        51	        };

    wasm.rs:206	        let ret = match results[0] {
    wasm.rs:207	            Val::I32(v) => v as i64,
    wasm.rs:208	            Val::I64(v) => v,

    wasm.rs:245	            .call(&mut self.store, args[0])

Resolution: delete `Op::returns_i64` (ops.rs:251-254) and the branch; always `guest.call`. Retiring `wasm::Guest::call_i64` itself belongs to the wasm.rs partition (fuelscape calls it at six sites). Acceptance: drive.rs has one call site and the `min_ticks` differential (`expect == decimal_digest`) still passes in `just fuzzfit`.

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

### fuzzfit-strategies-4: The driver's snapshot table restates `Op::kernel` through `u8` tags and an `unreachable!`
- Where: crates/before/fuzzfit/harness/src/drive.rs:83-89 (related: crates/before/fuzzfit/harness/src/ops.rs:407-423, crates/before/fuzzfit/harness/tests/sanity.rs:55-75)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (compared the four strings against `Op::kernel` at ops.rs:167, 181, 192, 199; read the identical tag match in sanity.rs:55-75); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found (7fb3b5ce original)
- Owner-gated: no

`Mirror::live_regs` returns `(Reg, u8)` with `b'v'|b'p'|b'c'|b'r'` tags, and the driver maps each tag to a kernel string that `Op::kernel` already owns (the four snapshot kernels are exactly `Op::VersionEncode`, `Op::PartyEncode`, `Op::ClockEncode`, `Op::RankDisplay`); sanity.rs repeats the same match with the same `unreachable!` (types-first: the compiler, not a comment, should exclude the fifth tag, and kernel names belong to the vocabulary module, so a second spelling here is a rename hazard the differential catches only at runtime).

Evidence:

        83	        let kernel = match tag {
        84	            b'v' => "ff_version_encode",
        85	            b'p' => "ff_party_encode",
        86	            b'c' => "ff_clock_encode",
        87	            b'r' => "ff_rank_display",
        88	            _ => unreachable!("mirror tags are v/p/c/r"),
        89	        };

    ops.rs:407	    /// Registers currently live, tagged `'v' | 'p' | 'c' | 'r'`.
    ops.rs:408	    pub fn live_regs(&self) -> Vec<(Reg, u8)> {

Resolution: in ops.rs add `pub enum Kind { Version, Party, Clock, Rank }` with `fn snapshot_op(self, reg: Reg) -> Op` (the three `*Encode` ops and `RankDisplay`); `live_regs() -> Vec<(Reg, Kind)>`; the driver becomes `let op = kind.snapshot_op(reg); guest.call(op.kernel(), &op.args())`; sanity.rs matches on `Kind`. Acceptance: no `ff_` string literal in drive.rs; `grep -n unreachable! drive.rs tests/sanity.rs` is empty; `just fuzzfit` green.

### fuzzfit-strategies-5: The pinned bands are not bound to the corpus that produced them
- Where: crates/before/fuzzfit/harness/src/drive.rs:103-105 (related: crates/before/fuzzfit/harness/src/strategies.rs:1979-2017, crates/before/fuzzfit/harness/Cargo.toml:26-32, crates/before/fuzzfit/harness/src/bands.rs:3-8, crates/before/fuzzfit/harness/src/bands.rs:196-218, crates/before/fuzzfit/harness/tests/enforce.rs:355-396)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read); executed: no
- Seen by: scaffolding; refutation: confirmed, severity medium to low (the sentry and refit legs catch every drift that changes a verdict; what escapes is sub-tolerance drift and stale `samples`/`min_denom`/`max_denom` metadata); history: no-rationale-found, with corroboration: 4e64a4fb (2026-08-31) bumped wasmtime in the fuzzfit Cargo.lock, a re-pin event calibrate.rs:366-368 declares, touched no bands.rs, and reports a clean gate
- Owner-gated: no

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

### fuzzfit-strategies-7: 49 measured guest kernels have no `Op`, so those public operations have no fuzz-fit band and the roster pin cannot see them
- Where: crates/before/fuzzfit/harness/src/ops.rs:49-56 (related: crates/before/fuzzfit/harness/tests/sanity.rs:87-177, crates/before/fuzzfit/guest/src/lib.rs:1844-1919, crates/before-fuelscape/src/ops.rs:14-22, crates/before-fuelscape/src/ops/tests.rs:17-28, crates/before/src/testing/validation_index.rs:121-142, .agent-notes/2026-07-26-before-fuzzfit-asymptotics/before-fuzzfit-asymptotics.md:189-193)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (comm as in finding 6; of the 63 exports without an Op, 7 are control or self-test exports (`ff_nop`, `ff_regs_reserve`, `ff_reset`, `ff_selftest_quadratic`, `ff_stage_len`, `ff_stage_prepare`, `ff_stage_ptr`) and 7 are query constructors whose docs say "unmeasured preparation" (guest/src/lib.rs:1844-1919), leaving 49 measured kernels; read the roster test at sanity.rs:98-177, which binds `Op` variants to `BANDS` only); executed: yes: the comm and the grep for "unmeasured preparation" settle the count; the construction below was not run
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed, severity high to medium (fuelscape's parity test already binds public-API additions to the surface roster; the envelope suite and bench-judge cover these operations on chosen shapes; what fuzzfit lacks is its own binding and unchosen-shape coverage); history: no-rationale-found (61398dc9 and the design note record public-API additions as fuzzfit re-pin events that "fail by name"; the guest grew from 51 to 107 exports across six fuelscape commits, each stating the fuzzfit vocabulary was deliberately left unchanged, and no ruling records that fuzzfit's scope excludes those operations)
- Owner-gated: yes: whether to price these operations here or record the fuzzfit/fuelscape division as a ruling is a scope decision

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

### fuzzfit-strategies-8: The roster test's hand list is a convention against a variant added without a roster entry; a derived variant list is a check
- Where: crates/before/fuzzfit/harness/src/ops.rs:153-156 (related: crates/before/fuzzfit/harness/tests/sanity.rs:87-99, crates/before/fuzzfit/harness/tests/enforce.rs:56-65)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (the refutation pass script-checked that all 44 kernel names equal `ff_` plus the snake_case of the variant name with zero mismatches; I read the roster test and `judge`'s missing-band panic at enforce.rs:59-65); executed: no
- Seen by: structure-prose; refutation: reframed (the actionable gap is the roster's non-totality, not the explicit string table, which is legible and panics on a wrong name at first call); history: deliberate-and-holds for the hand roster's purpose (61398dc9: one representative op per variant as a committed expectation list), which a derived list satisfies identically
- Owner-gated: no

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

### fuzzfit-strategies-9: Idiom nits across `ops.rs` and `strategies.rs`
- Where: crates/before/fuzzfit/harness/src/ops.rs:273-276 (related: crates/before/fuzzfit/harness/src/ops.rs:298-300, crates/before/fuzzfit/harness/src/ops.rs:311, crates/before/fuzzfit/harness/src/ops.rs:431-448, crates/before/fuzzfit/harness/src/strategies.rs:361, crates/before/fuzzfit/harness/src/strategies.rs:711, crates/before/fuzzfit/harness/src/strategies.rs:990, crates/before/fuzzfit/harness/src/strategies.rs:1054, crates/before/fuzzfit/harness/src/strategies.rs:1094, crates/before/fuzzfit/harness/src/strategies.rs:1114, crates/before/fuzzfit/harness/src/strategies.rs:1128, crates/before/fuzzfit/harness/src/strategies.rs:1283-1286, crates/before/fuzzfit/harness/src/strategies.rs:1298, crates/before/fuzzfit/harness/src/strategies.rs:1359, crates/before/fuzzfit/harness/src/strategies.rs:1528, crates/before/fuzzfit/harness/src/strategies.rs:1547, crates/before/fuzzfit/harness/src/strategies.rs:1648-1650, crates/before/fuzzfit/harness/src/strategies.rs:1771, crates/before/fuzzfit/harness/src/strategies.rs:1772, crates/before/fuzzfit/harness/src/strategies.rs:1843-1845)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (each site read at its line); executed: no
- Seen by: structure-prose, scaffolding, refutation (the arm-selector ranges); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Legibility and named-constants doctrine, batched: `Step` exposes `pub identity` and a trivial `identity()` getter, and `done(d, e)` is `done_pair(d, e, false)`; `core::ptr::eq` in a std crate; the builder is named `B`, which forces the second register of every pair to be spelled `bb`; the battery and walk arm selectors (`gen_range(0..12u32)`, `0..10u32`, `0..20u32`, `0..13u32`) are magic ranges that must agree with each match's arm count, so an arm added without widening the range never fires; `shares.iter().skip(1).step_by(2).next().copied()` is `shares.get(1).copied()`; `std::mem::take(&mut shares)` where `shares` is dead afterwards; `take(16)`/`skip(16)` must agree and are unnamed; the pop-then-insert is `rotate_right(1)` and its comment does not say why the first tooth leads; `i % keep_every == 0` beside `cadence.is_multiple_of(2)`; `per_universe[i].clone()` per cross op and the "source of truth" retain comments work around a borrow that does not exist (`pick(&mut b.rng, &per_universe[i].versions)` borrows two distinct locals and the `Some(&a)` patterns copy the `Reg`).

Evidence:

       273	    /// Whether the step's operands dispatch an identity-law fast path
       274	    /// (see [`Step::identity`]).
       275	    pub identity: bool,

       311	    core::ptr::eq(ab.as_ptr(), bb.as_ptr()) && ab.len() == bb.len()

    strategies.rs:711	            match self.rng.gen_range(0..12u32) {

    strategies.rs:1054	                if let Some(c) = shares.iter().skip(1).step_by(2).next().copied() {

    strategies.rs:1283	            let mut rev: Vec<Reg> = shares.iter().rev().copied().collect();
    strategies.rs:1284	            if let Some(first) = rev.pop() {
    strategies.rs:1285	                rev.insert(0, first);
    strategies.rs:1286	            }

    strategies.rs:1771	                let (ui, uj) = (per_universe[i].clone(), per_universe[j].clone());

Resolution: make `Step::identity` private or drop the getter; collapse `done`/`done_pair` into one constructor; `std::ptr::eq`; rename `B` to `Builder` so pair operands read `a`/`b`; name each selector's arm count beside its match (or a `const ARMS: u32`); `shares.get(1)`; `let mut order = shares;`; `const PARTY_FOLD_WIDTH: usize = 16`; `rev.rotate_right(1)` with a comment naming the fold receiver; one modulo idiom; index `per_universe` directly and delete the retain comments. Acceptance: `just fuzzfit` (fmt, clippy, nextest) green with the deterministic corpus byte-identical (no RNG draw changes).

### fuzzfit-strategies-10: `Malformed` is documented as never a `before` bug, but the decode and parse arms map before's own round-trip failures to it
- Where: crates/before/fuzzfit/harness/src/ops.rs:314-320 (related: crates/before/fuzzfit/harness/src/ops.rs:579, crates/before/fuzzfit/harness/src/ops.rs:745, crates/before/fuzzfit/harness/src/ops.rs:761-762, crates/before/fuzzfit/harness/src/ops.rs:842, crates/before/fuzzfit/harness/src/ops.rs:856-857)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read); executed: no
- Seen by: instrument-correctness; refutation: reframed (`Mirror::step` is total over arbitrary programs, where a decode against an empty or foreign stage is a generator bug, so `Malformed` is a legitimate outcome of those arms in general; the overclaim is the doc's "never a `before` bug"); history: no-rationale-found (7fb3b5ce original)
- Owner-gated: no

For generated programs the stage always holds the immediately preceding encode or rendering, so a decode or parse failure at those five sites can only be a `before` round-trip bug, which the type's doc says it never represents; the failure is still loud (`for_each_*` panics "malformed program"), but the message attributes a library bug to the generator (the doctrine wants every failure site to name what it means).

Evidence:

       314	/// A mirrored execution error: the program is malformed (a generator bug,
       315	/// never a `before` bug).
       316	#[derive(Debug)]
       317	pub struct Malformed {
       318	    /// Which op misfired.
       319	    pub op: String,
       320	}

       579	                let clock = Clock::decode(self.stage.as_slice()).map_err(|_| malformed())?;

Resolution: either reword the doc ("a register-file or stage violation: a generator bug, or a `before` round-trip failure surfacing through a stale stage") or, at the five sites, distinguish a non-empty stage that fails to decode (panic with a differential-style message naming a round-trip bug) from an empty or wrong-typed stage (`Malformed`). Acceptance: the struct doc and the five arms agree on what a decode failure means.

### fuzzfit-strategies-11: The mirror omits join and meet's empty-operand rungs, so O(1) steps enter the fitted cloud and the `ff_version_join`/`ff_version_meet` liveness floors are about 2.8 decades wide
- Where: crates/before/fuzzfit/harness/src/ops.rs:603-627 (related: crates/before/fuzzfit/harness/src/drive.rs:57-74, crates/before/fuzzfit/harness/src/bands.rs:91-96, crates/before/fuzzfit/harness/src/bands.rs:808-855, crates/before/fuzzfit/harness/src/strategies.rs:772-785, crates/before/fuzzfit/harness/src/strategies.rs:961-980, crates/before/fuzzfit/harness/src/strategies.rs:1219-1238, crates/before/fuzzfit/harness/src/strategies.rs:1272-1273, crates/before/src/version.rs:152, crates/before/src/version.rs:864-925, crates/before/tests/meter.rs:10746-10807)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified for the mechanism (read `join_view`/`join_refs`/`meet_view` at version.rs:864-925; `Version::is_empty` is `pub` at 152; `Version`'s `PartialEq` is `codec::canonical_eq` at version.rs:1724-1727, so the mirror's `va == *vb` is exactly the first rung and nothing more; `Clock::fork` clones the parent's version at clock.rs:155 and `CliffComb`, `RevealComb`, `AscendCliff`, and `CombScatter` never tick their seed before `fork_balanced`, so un-ticked teeth hold empty versions; battery arm 6 at 772-785 extracts `version_of(ca)` from `pools.clocks` as the join/meet spare; the pinned widths read from bands.rs); attribution of the width to these steps is inferred (no run); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed (no other O(1) path exists in the join/meet ladder); history: no-rationale-found, with corroboration: the empty rungs entered version.rs at 58a37d80 (07-27) before the predicate was written at 306e2de0 (07-30); the meter pin for them landed the next day (89def207) touching no fuzzfit file; and the identity-routing re-pin did not narrow these floors (join `width_below` 2.664 at b528f2c6, 2.796 at 306e2de0, 2.790 at HEAD; meet 2.848, 2.848, 2.840) while every routed key elsewhere sits under 0.9
- Owner-gated: no

The harness's own criterion (drive.rs:57-66, bands.rs:91-96) says steps that dispatch an identity-law fast path are O(1) by mechanism and must leave the sample stream because fitting them beside the walked cloud makes both bands decoration-wide; before's `join_view`/`join_refs` and `meet_view` have three such rungs each (canonical equality; `v ∨ 0 = v`; `0 ∨ v = v`; and the meet duals `0 ∧ v = 0`, `v ∧ 0 = 0`) and `identity_fast_paths::empty_operands_answer_without_a_walk` pins the empty ones as part of the same ladder, but the mirror models only the first. The pinned bands carry the symptom: `ff_version_join` `width_below` 2.789563 and `ff_version_meet` 2.839589 against 0.399089 (`ff_version_distance`), 0.384644 (`ff_version_lag`), and 0.869608 (`ff_version_cmp`); with `ENFORCE_MARGIN_BELOW` 0.8 on top, an accidentally constant join or meet reads in band except at the largest denominators, which voids bands.rs:36-38's liveness claim ("below-band is a liveness flag") for exactly these two keys (Principle 2: a ceiling over a counter passes vacuously when the floor has no bite). The related `RevealComb` comment at 1224-1225 ("each consume revealing the shared minimum to the floor frame") describes consumes that are empty-rung no-ops for every odd tooth when `hifloor` is false, so the treatment arm exercises join less than its control does.

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

### fuzzfit-strategies-12: `Mirror::step` duplicates whole arms that differ by one method call
- Where: crates/before/fuzzfit/harness/src/ops.rs:697-736 (related: crates/before/fuzzfit/harness/src/ops.rs:458-477, crates/before/fuzzfit/harness/src/ops.rs:603-627, crates/before/fuzzfit/harness/src/ops.rs:668-688, crates/before/fuzzfit/harness/src/ops.rs:487-503, crates/before/fuzzfit/harness/src/ops.rs:790-806, crates/before/fuzzfit/harness/src/ops.rs:578, crates/before/fuzzfit/harness/src/ops.rs:744, crates/before/fuzzfit/harness/src/ops.rs:760, crates/before/fuzzfit/harness/src/ops.rs:841, crates/before/fuzzfit/harness/src/ops.rs:855)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read each arm pair); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`VersionJoinAll`/`VersionMeetAll` are twenty identical lines each except `join_all`/`meet_all`; `ClockTick`/`ClockSend` differ only in `tick()`/`send()`; `VersionJoin`/`VersionMeet` and `VersionDistance`/`VersionLag` likewise; `ClockJoin`/`PartyJoin` share the take-join-restore shape; and `(self.stage.len() as u64) * 8` appears five times (legibility: a denomination or linearity fix must be applied in two or three places, and a reviewer diffs arms by eye to confirm they still agree).

Evidence:

       697	            Op::VersionJoinAll { dst, src, n } => {
       698	                let mut denom = 0u64;
       699	                let mut operands = Vec::with_capacity(n as usize);

       717	            Op::VersionMeetAll { dst, src, n } => {
       718	                let mut denom = 0u64;
       719	                let mut operands = Vec::with_capacity(n as usize);

Resolution: extract small helpers on `Mirror` (`take_versions(src, n) -> Result<(u64, Vec<Version>), Malformed>` for the folds; `with_clock(c, f)` for tick/send/fork; a `version_pair_bits(a, b)`; `stage_bits()`), keeping one arm per `Op` so the exhaustive match still documents the ABI. Acceptance: `Mirror::step` roughly halves with no arm losing its comment; `just fuzzfit` green; the deterministic stream unchanged.

### fuzzfit-strategies-13: Two `as u64` casts on `encoded_bits()` survived the u64 denomination migration
- Where: crates/before/fuzzfit/harness/src/ops.rs:764-764 (related: crates/before/fuzzfit/harness/src/ops.rs:858, crates/before/src/version.rs:1150-1153, crates/before/src/party.rs:596-600, justfile:594-597, .github/workflows/ci.yml:117-120)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`encoded_bits` returns `u64` at version.rs:1151, party.rs:597, clock.rs:852; `git show 05d87e1b -- ops.rs` removes 20 `encoded_bits() as u64` casts and leaves these two; no commit has touched ops.rs since); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: deliberate-but-expired (real usize-to-u64 conversions until 05d87e1b, 2026-08-19)
- Owner-gated: no

The two casts convert `u64` to `u64`; the other same-shaped casts in the file (`(len as u64) * 8`) are real conversions, so these two read as if `encoded_bits` were narrower (Principle 5: code states what IS). Whether clippy's `unnecessary_cast` reads red on the `just fuzzfit` lint leg (`cargo clippy --all-targets -- -D warnings`, justfile:596) is unsettled: the lint ordinarily fires on a same-type cast of a non-literal expression, but 4e64a4fb (2026-08-31) reports `just gate clean (398s, all legs)` with these casts present and the gate's wasm stream runs that leg (justfile:467); CI does not run it (ci.yml:117-120). I was not permitted to run clippy; see the open questions.

Evidence:

       764	                let denom = text_bits + version.encoded_bits() as u64;

       858	                let denom = text_bits + party.encoded_bits() as u64;

    version.rs:1150	    #[cfg(any(test, feature = "meter"))]
    version.rs:1151	    pub fn encoded_bits(&self) -> u64 {

Resolution: drop both casts and run the fuzzfit recipe's clippy line. Acceptance: `grep -n 'encoded_bits() as u64' ops.rs` is empty and `cd crates/before/fuzzfit && cargo clippy --all-targets -- -D warnings` is clean.

### fuzzfit-strategies-14: `decimal_digest` and the ABI return codes are duplicated by hand between guest and harness
- Where: crates/before/fuzzfit/harness/src/ops.rs:910-920 (related: crates/before/fuzzfit/guest/src/lib.rs:680-692, crates/before/fuzzfit/guest/src/lib.rs:98-105, crates/before/fuzzfit/harness/src/ops.rs:322-325, crates/before/fuzzfit/harness/src/ops.rs:644-649, crates/before/fuzzfit/guest/src/lib.rs:582-587)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (read both digest bodies, byte-identical; `ERR_OP = -2` at ops.rs:325 and guest:103; the cmp codes 0/1/2/3 at ops.rs:645-648 and guest:583-586); executed: no
- Seen by: scaffolding; refutation: confirmed (the guest is a cdylib depending only on `before`, so a `#[path]`-shared module is feasible); history: no-rationale-found (68b60f06 added both bodies in one commit with "computed identically on both sides" as the only binding)
- Owner-gated: no

The digest, `ERR_OP`/`OK`, and the comparison encodings are transcribed by hand on both sides, bound only by prose ("computed identically here") and by the runtime differential on the first `min_ticks` step; the hole is defended, but by a convention plus a late runtime failure rather than one definition (prefer a dependency over hand-rolled parity).

Evidence:

       910	/// A nonnegative FNV-1a digest of a decimal rendering: the `i64`
       911	/// channel's encoding for unbounded counts, mirrored in the guest's
       912	/// `ff_version_min_ticks`.
       913	fn decimal_digest(text: &str) -> i64 {

    guest lib.rs:680	/// A nonnegative FNV-1a digest of a decimal rendering.
    guest lib.rs:685	fn decimal_digest(text: &str) -> i64 {

Resolution: factor the return codes, the digest, and ideally the kernel-name strings into one shared module both crates include (`#[path]` from a sibling `abi.rs`, or a dependency-free `fuzzfit-abi` crate); drop the mirrored-by-hand prose. Acceptance: one definition of `decimal_digest` and `ERR_OP` in the workspace; the `min_ticks` differential still passes.

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

### fuzzfit-strategies-16: The escalation replays and the bootstrap stream carry reach claims with no committed floor; the builder truncates silently when a budget binds
- Where: crates/before/fuzzfit/harness/src/strategies.rs:96-111 (related: crates/before/fuzzfit/harness/src/strategies.rs:151-159, crates/before/fuzzfit/harness/src/strategies.rs:387-434, crates/before/fuzzfit/harness/src/strategies.rs:1538-1546, crates/before/fuzzfit/harness/src/strategies.rs:1679-1716, crates/before/fuzzfit/harness/src/bin/calibrate.rs:116-131, crates/before/fuzzfit/harness/tests/enforce.rs:207-239, crates/before/fuzzfit/harness/tests/enforce.rs:398-436, crates/before/fuzzfit/harness/tests/sanity.rs:16-39)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified for the mechanism (every cap refusal in `B` returns `false`/`None` silently at 409-416, 426-434, 482-490, 556-564, 589-600; the depth loop breaks on a refused fork or tick at 1539 and 1544-1546; the tail rows at 1679-1716 skip on `!room()`; `programs_respect_the_budget` asserts ceilings only; the replay tests judge whatever program results); the op count is a hand model, not a run: at depth 1792, `keep_every` is 44, the spine costs 3584 ops, the 1751 non-cadence levels about 3503, the 41 cadence levels about 538, the tail 50, so about 7676 construction ops plus at most 192 battery ops of 9000, 1832 of 3000 ticks, 1842 of 2048 forks (the refutation pass's independent model gives 7675); executed: no
- Seen by: scaffolding, adequacy; refutation: confirmed (the escalation half; the bootstrap half low because the small bands also pool the main corpus's sub-floor samples and the at-least-once floor exists); history: no-rationale-found (f66d7c17 raised `max_ops` from 8000 to 9000 and wrote "Sized to admit" in the same diff with no test of the margin; BOOTSTRAP_MAX_ROUNDS's "tops out just under the fit floor" was established by a probe sweep and recorded as a manual procedure)
- Owner-gated: no

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
- Seen by: instrument-correctness (181, 200), structure-prose (207, 299), adequacy (1866); refutation: confirmed for 181, 200, 299 (severity nit for 299: "a fork rejoined" can be read loosely), 1866; reframed for 207 (the doc is accurate for lane `a` but names no lane, and the non-descending lane's fork is load-bearing: it is what deepens that lane's id each level, so document it rather than remove it); history: 299 deliberate-but-expired (stale from birth within af2330a6, whose own message records replacing the fork-and-rejoin shape); the rest no-rationale-found
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

### fuzzfit-strategies-19: The builder's `Ty`/`slots` liveness model is write-only
- Where: crates/before/fuzzfit/harness/src/strategies.rs:358-368 (related: crates/before/fuzzfit/harness/src/strategies.rs:337-347, crates/before/fuzzfit/harness/src/strategies.rs:382-385, crates/before/fuzzfit/harness/src/strategies.rs:471, crates/before/fuzzfit/harness/src/strategies.rs:532, crates/before/fuzzfit/harness/src/strategies.rs:547-548, crates/before/fuzzfit/harness/src/strategies.rs:569, crates/before/fuzzfit/harness/src/strategies.rs:611-614, crates/before/fuzzfit/harness/src/strategies.rs:635, crates/before/fuzzfit/harness/src/strategies.rs:645, crates/before/fuzzfit/harness/src/strategies.rs:682, crates/before/fuzzfit/harness/src/strategies.rs:848, crates/before/fuzzfit/harness/src/strategies.rs:857, crates/before/fuzzfit/harness/src/strategies.rs:865, crates/before/fuzzfit/harness/src/strategies.rs:1042, crates/before/fuzzfit/harness/src/strategies.rs:1122, crates/before/fuzzfit/harness/src/strategies.rs:1841, crates/before/fuzzfit/harness/src/strategies.rs:1936, crates/before/fuzzfit/harness/tests/sanity.rs:41-43)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (`grep -n slots` lists 24 sites: the declaration, the constructor, `push` at 383, `.len()` at 384, 594, 678, and 17 `= Ty::Dead` assignments; a grep for any other use of a `Ty` value outside `alloc(Ty::...)` (32 sites) matches nothing; every access to the field contains the token `slots`, so the list is exhaustive); executed: yes: the two greps settle that no code path reads a slot's type
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed, severity medium to low (no instrument is weakened; the mirror plus `sanity::programs_are_well_formed` own liveness); history: no-rationale-found (no version of strategies.rs in its eleven commits ever read a slot's type; 319f9c53 centralized "slot liveness" bookkeeping as if it were load-bearing)
- Owner-gated: no

`B::slots: Vec<Ty>` is written at every emitter and read only through `.len()` to allocate the next register; no expression matches on `Ty::V | P | C | R` or consults `slots[i]` before emitting an op, so the builder does not model liveness, and the struct doc, the `Ty::Dead` doc, and sanity.rs's test doc ("the builder's liveness model matches real consumption") all describe a check the mirror alone performs (Principle 3: infrastructure earns its place by naming what it catches; a write-only type record catches nothing, costs an enum plus seventeen bookkeeping lines a maintainer must keep for no reader, and its documentation invites trusting a check that does not exist). I hold this at medium against the refutation's low because the cost is spread across every emitter in a 2000-line generator and the false claim sits at the site a maintainer adding a family reads first; the dissolution is mechanical and the corpus byte-identical.

Evidence:

       358	/// The type-tracked program builder: emits ops, models liveness the way the
       359	/// mirror will, and enforces the budget unconditionally (an out-of-budget
       360	/// request is skipped, so any parameter draw stays within [`BUDGET`]).
       361	struct B {
       362	    ops: Vec<Op>,
       363	    slots: Vec<Ty>,

       344	    /// Consumed, or of uncertain liveness after a possibly-rejecting op;
       345	    /// never used again either way.
       346	    Dead,

    sanity.rs:41	    /// Every generated program is well-formed: the native mirror executes
    sanity.rs:42	    /// it end to end without a register-file violation, i.e. the builder's
    sanity.rs:43	    /// liveness model matches real consumption (linearity by construction).

Resolution: replace `slots: Vec<Ty>` with a `next_reg: Reg` counter, delete `enum Ty`, make `alloc()` take no argument, remove every `self.slots[..] = Ty::Dead` line (keeping the informative comments such as `// may underflow` where they explain why a destination is not pooled), and rewrite the struct doc to "emits ops and enforces the budget unconditionally; the mirror owns well-formedness (`programs_are_well_formed`)"; fix sanity.rs:41-43 to state what the test checks. (The alternative, making the model live with a slot-type assertion in each emitter, samples no space `programs_are_well_formed` cannot, so dissolution is the doctrinal answer.) Acceptance: `grep -n 'Ty\b\|slots' strategies.rs` returns only the counter; `generation_is_deterministic` and `programs_are_well_formed` stay green with the deterministic corpus byte-identical (register numbering unchanged); no doc names a builder liveness model.

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

### fuzzfit-strategies-21: Guards that can never fire in the gate: three in `fork_balanced`, `any_program`'s non-empty filter, and `B::push`'s release-compiled-out `debug_assert`
- Where: crates/before/fuzzfit/harness/src/strategies.rs:439-465 (related: crates/before/fuzzfit/harness/src/strategies.rs:391-394, crates/before/fuzzfit/harness/src/strategies.rs:1965-1971, crates/before/fuzzfit/harness/src/strategies.rs:1754-1760, crates/before/fuzzfit/Cargo.toml:23-25, justfile:575-577, justfile:597, crates/before/fuzzfit/harness/tests/sanity.rs:16-39)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (executed: a Python replay of `fork_balanced` as written against a stripped variant (first disjunct only; no `next.len() == pool.len()` return; no trailing break; `truncate` kept), over `n` in 0..300 and every fork-budget cutoff 0..39 plus unbounded, at `<scratchpad>/before/final:fuzzfit-strategies/fork_balanced_replay.py`: output "output diffs: 0 | disjunct decisive: 0 | eqlen taken: 0"; the structure-prose lens and the refutation pass each ran an independent replay with the same result; the filter and the assert were settled by reading: every `construct` arm opens with `b.clock_seed()`, which succeeds when `max_ops >= 1`, `Independent` clamps to at least two universes, the release profile leaves debug assertions off, and `just fuzzfit` runs `cargo nextest run --cargo-profile release`); executed: yes: the replay above
- Seen by: structure-prose, scaffolding, adequacy; refutation: confirmed and reframed (`pool.truncate(n)` at 463 is live for `n = 0`, which turns `[src]` into `[]`; no roster draw passes 0, but `build` is public, so keep it); history: no-rationale-found (all 7fb3b5ce originals; the suite has run under the release profile since the first recipe)
- Owner-gated: no

In `fork_balanced`, the disjunct `pool.len() * 2 <= n as usize` (445) is never decisive (when it holds, `next.len() <= 2·pool.len() − 1 < n`, so the first disjunct already holds), the `next.len() == pool.len()` return (455-457) is unreachable (the first element of every pass forks or returns on budget exhaustion), and the trailing `if pool.len() as u32 >= n { break; }` (459-461) restates the `while` condition; the interleaved parent/child order is semantic (the comb families alternate on index parity) and the stripped loop preserves it. `any_program`'s `prop_filter("non-empty program")` never rejects. `B::push`'s `debug_assert!` is compiled out in every gate execution and the invariant it guards is pinned in release by `programs_respect_the_budget` (legibility: conditions that look load-bearing and are not force every reader to re-derive the invariant; Principle 3: a guard must sample a space the committed tests cannot).

Evidence:

       445	                if (next.len() as u32) < n || pool.len() * 2 <= n as usize {

       455	            if next.len() == pool.len() {
       456	                return pool;
       457	            }

       459	            if pool.len() as u32 >= n {
       460	                break;
       461	            }

       392	        debug_assert!(self.room(), "callers check room() before emitting");

      1970	        .prop_filter("non-empty program", |p| !p.is_empty())

Resolution: reduce `fork_balanced` to one interleaving pass per doubling with the single `next.len() < n` guard, keeping `truncate` (or give `n = 0` an explicit early return) and stating the doubling invariant in the doc comment; delete the `prop_filter`; delete the `debug_assert` or promote it to `assert!` if the owner wants the emission site named on a breach. Acceptance: `generation_is_deterministic` and `programs_respect_the_budget` green with the deterministic corpus byte-identical (`for_each_deterministic_program` yields the same programs; the refit staleness check confirms since the stream is the same); `grep -n prop_filter strategies.rs` empty.

### fuzzfit-strategies-22: `construct` is an 840-line match with a partial domain and five repeated epilogues
- Where: crates/before/fuzzfit/harness/src/strategies.rs:885-887 (related: crates/before/fuzzfit/harness/src/strategies.rs:932-941, crates/before/fuzzfit/harness/src/strategies.rs:1070-1082, crates/before/fuzzfit/harness/src/strategies.rs:1142-1155, crates/before/fuzzfit/harness/src/strategies.rs:1168-1183, crates/before/fuzzfit/harness/src/strategies.rs:1195-1207, crates/before/fuzzfit/harness/src/strategies.rs:1226-1238, crates/before/fuzzfit/harness/src/strategies.rs:1249-1261, crates/before/fuzzfit/harness/src/strategies.rs:1504-1717, crates/before/fuzzfit/harness/src/strategies.rs:1718-1720, crates/before/fuzzfit/harness/src/strategies.rs:1754-1762)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (read the five epilogues and the two join chains; the `unreachable!` at 1719 guards a variant `build` expands at 1757 before dispatch); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`construct` spans 885-1723 and the `Escalation` arm alone is 214 lines; five spine families end with the same epilogue (push seed; `version_of(cur)` into versions, sometimes with a rank; if `cur != seed`, `split_parts(cur)` into parties and versions, sometimes with a cross tick), `RevealComb` and `PureComb` repeat the adjacent join chain verbatim, and the match carries an `unreachable!` because `Family` conflates the coupled families with `Independent`, which `build` expands and `reduced_family` never returns (modules and functions have a single clear responsibility; the repeated epilogues are where a family-specific pooling mistake would hide, and the `unreachable!` is a type admitting a value the function refuses).

Evidence:

       885	fn construct(b: &mut B, family: &Family) -> Pools {
       886	    let mut pools = Pools::default();
       887	    match *family {

      1718	        Family::Independent { .. } => {
      1719	            unreachable!("Independent is expanded by build(), not construct()")
      1720	        }

Resolution: one function per family with `construct` reduced to dispatch; a shared `spine_epilogue(b, pools, seed, cur, rank: bool, cross_tick: bool)` and `join_chain(b, shares) -> Option<Reg>`; consider a `Coupled` sub-enum returned by `reduced_family` and taken by `construct`, with `Family::Independent { .. }` and `Family::Coupled(Coupled)` at the top level so the `unreachable!` dissolves; each family function then has a rustdoc home for the construction comments now attached to match arms. Acceptance: `construct` or its replacement fits on a screen; `grep -c unreachable! strategies.rs` is 0; `generation_is_deterministic` and the enforce staleness cross-check confirm the emitted programs are unchanged.

## Positives

- The differential is total, not sampled: every step's return code and every live register's final bytes are compared, and disagreement panics rather than being filtered (drive.rs:52-56, 76-97); `min_ticks` is digested identically on both sides so an unbounded count still rides the i64 channel exactly (ops.rs:913-920, guest lib.rs:685-692).
- The mirror's identity predicates reproduce before's own dispatch exactly where they are mirrored: `version_buffers_alias` (pointer plus length) is `Bits::ptr_eq`, and `va == *vb` is `codec::canonical_eq` through `Version`'s `PartialEq` (version.rs:1724-1727), matching the rungs at version.rs:391, 432, 865, 890, 913; the comments say which rung each op dispatches on and why the two predicates differ (ops.rs:610-611, 650-651, 672-673).
- Rejection arms are predicted per case by running the real operation natively, never assumed from the regime, and the rejected operand is handed back in both executions so it stays in the end-of-program differential (ops.rs:487-503, 790-806); rejection arms are priced as their own band key (ops.rs:279-287).
- Budgets are enforced structurally by every builder method, pinned after the fact by `sanity::programs_respect_the_budget`, and tied to the guest's register reserve by a compile-time assert (wasm.rs:41-47), so a budget raise past the reserve is a compile error rather than a reallocation inside a measured window.
- The scope section (strategies.rs:41-62) is a precise map of the negative space: wide magnitude, codec rejection, and empty folds are each named with the instrument that owns them instead.
- `ESCALATION_MAX_DEPTH` is one constant shared by the draw range and the replay pin (strategies.rs:123-140), so widening the family cannot strand the replay below the true far end; `MAGNITUDE_SHIFT_CAP` makes the shift's definedness local to the construction rather than resting on the draw ranges (113-121).
- The denomination rules are stated once in the module doc (ops.rs:9-30), each departing arm carries a one-line comment naming its rule class, and the rank proxy states the direction of its error (under-counting reads as more fuel per bit, so it can mask no superlinear growth).
- The `Independent` regime keeps its pool of record consistent after consuming ops (`per_universe[j].parties.retain`, 1845, 1874, 1938), so cross-universe draws never hand the mirror a consumed register.
- The escalation construction is careful about linearity: the deep-overlap poison is placed off the snapshot cadence (1526-1533), every party-ladder arm leaves the lane as it found it (1576-1584), and the deepest-snapshot join and the seed/snapshot sync are disjoint by construction, so the at-scale success and rejection arms are each reached deliberately.
- The two deterministic streams (`for_each_deterministic_program`, `for_each_bootstrap_program`) are pure functions of case index, so calibration and the staleness cross-check are literally the same stream.
- `harness/Cargo.toml` justifies each dependency by what it serves outside itself, `fuzzfit/Cargo.toml` and `.cargo/config.toml` each state the mechanism and the failure it prevents, and `build.rs` plus `PINNED_RUSTC` make toolchain provenance mechanical rather than conventional.
- drive.rs's public functions carry accurate `# Panics` sections and its `expect` messages are one-line proofs ("live_regs only reports live slots", "family strategy cannot fail").

## Open questions for Finch

1. Is the fuzz-fit vocabulary deliberately frozen at the 44 operations it reached on 2026-07-26, with the fuelscape atlas as the sole (non-enforcing) coverage for the 49 measured kernels that have no `Op`? The design note and 61398dc9 record public-API additions as fuzzfit re-pin events that fail by name, and the six fuelscape commits that grew the guest each say the vocabulary was deliberately left unchanged, but no ruling records the division. Recommendation: bind the vocabulary to `METHOD_SURFACE` with a reviewed exemption list (finding 7), add the operations that fit the register machine today (`ticks`, `forks`, the `*_all` doors, `span`/`span_all`, `eq`, the `Rank` codec, `Ranked`, clock text I/O) and re-pin, and record the remaining exemptions per operation.
2. Does the fuzzfit clippy leg (`cargo clippy --all-targets -- -D warnings`, justfile:596) pass at HEAD? The two same-type casts at ops.rs:764 and 858 would ordinarily trip `unnecessary_cast`, and the mixed modulo idiom (`is_multiple_of` at 1648 and 1650 beside `% == 0` at 1528 and 1547) looks like a partial lint-driven edit. 4e64a4fb (2026-08-31) reports `just gate clean (398s, all legs)` with the casts present, and the gate's wasm stream runs this recipe; CI does not (ci.yml:117-120). Either clippy is silent here for a reason I could not run down by reading, or the wasm gate stream has not run clean since 05d87e1b (2026-08-19), which matters beyond the casts because it tests whether that stream is being run before commits. Recommendation: one `just fuzzfit` run settles it; if red, treat it as a process finding, not a lint nit.
3. Relay to the bands partition: 4e64a4fb bumped wasmtime to 47.0.4 in the fuzzfit Cargo.lock, which calibrate.rs:366-368 declares a re-pin event, without touching bands.rs. The staleness leg tolerates 0.7 decades, so whether fuel moved is unknown. Recommendation: pin the wasmtime version beside `PINNED_RUSTC` and assert it, the way the toolchain is asserted, so the convention becomes a check. Also for that partition: `ff_clock_join` and `ff_clock_sync` success arms carry `width_below` of about 1.5 decades (bands.rs:394, 466), far above the other construction kernels; is the version half hitting an identity rung while the denominator still counts both versions (the genre of finding 11), or something in the party join?
4. The bootstrap family also emits `ClockFork` sub-floor, which is not in `SMALL_BAND_KERNELS` and is therefore judged `BelowFloor` (skipped) in the bootstrap replay; is fork deliberately outside rumors' bootstrap hot path as the roster defines it? Recommendation: if fork is on the hot path, add it to the roster and re-pin; if not, one sentence on `SMALL_BAND_KERNELS` saying so.
5. In `IdPairLockstep`, both lanes fork every level and only one descends (strategies.rs:992-1000); the un-descended lane's fork is what deepens its id in lockstep, so I read it as intended, and finding 18 asks for it to be documented rather than removed. Recommendation: confirm, and let the field doc say so.

## Dropped

- Em-dashes in twelve `//` comments (strategies.rs): a crate-wide pattern (374 such comments in before/src per the history pass), governed by owner doctrine rather than a project hard rule; belongs to a crate-wide prose pass, not a partition finding.
- Tautological `denom_bits >= 1` assertion at sanity.rs:51 (raised by the refutation pass): out of this partition (tests/sanity.rs); relayed to the fuzzfit tests partition.
- "Honesty bound" sentence stated as false (candidate 37): reframed by the refutation and history passes to imprecision (true with the constant's dependence on the tick/fork/fold caps named); folded into finding 15.
- Comb draw ranges exceed the tick budget (candidate 41): silent truncation is the builder's stated design (strategies.rs:31-33, 358-360, and the design note); the residual field-doc inaccuracy is folded into finding 18.
- "organic" and "divert" as unanchored coinages (part of candidate 27): dropped per the refutation and history passes; `organic` is crate-wide vocabulary (laws.rs:11) and `divert` is the meter board's own flag name (registry.rs:598).
- Candidates 14, 18, 34 (write-only `Ty`): duplicates of finding 19. Candidates 16 and the cast half of 30: duplicates of finding 13. Candidates 26, 39: duplicates of finding 17. Candidates 28, 42: merged into finding 3. Candidate 33: duplicate of finding 11. Candidate 13 and the escalation half of 3: merged into finding 16. Candidates 7, 17, 31 and the guard half of 11: merged into finding 21. Candidates 0, 19, 35, 4: split into findings 6 (documentation) and 7 (coverage). Candidates 24, 25, 40, 15 and the doc residual of 41: merged into finding 18. Candidate 29: reframed into finding 8. The non-cast items of candidates 11 and 30 plus the refutation's arm-selector ranges: merged into finding 9.
- The refutation's `honest` qualifier sites in sanity.rs, enforce.rs, and bands.rs: out of this partition; noted at the end of finding 15 for those reviewers.
