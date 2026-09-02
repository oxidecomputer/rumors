# Partition codec-base-text-tree: The arbitrary-precision Base (with its limb meter), display, text notation, the id-tree parser, and the codec test suite

## Partition summary

This partition is the codec's arithmetic and notation kernel. `Base` (`crates/before/src/codec/base.rs`) is a thin newtype over `dashu_int::UBig`: every arithmetic, comparison, equality, and hashing operator records its operands' 64-bit limb widths through the shims in `base/limb_metered.rs` (which compile to nothing without the `limb-meter` feature) and then delegates to the backend whole; `base/limb_meter.rs` holds the process-global counters and the argument for what this currency sees that the heap and scan meters cannot. `MsbWindows` and `msb_cmp_windows` stream an MSB-aligned comparison for the rank's class-tie ordering. `tree.rs` parses the packed 2-bit presence-tag id grammar on an explicit heap frame stack and wraps it as `validate_id`; `text.rs` parses the paper's `0 | 1 | (i1, i2)` id notation on its own frame stack, reads decimal bases by delegating the radix conversion to the backend, and splits a stamp `(i, e)` into its id bits and event text; `display.rs` renders an id with one to two control bits per open node on a `BitsBuf`. `codec/tests.rs` (1976 lines) and `base/tests.rs` (179 lines) are the test files; the remaining six files total 1047 lines of production code, 3202 lines read in all.

The production code is close to its simplest shipped form and honors the crate's hard rules everywhere I looked: no walk recurses on depth, every `expect` message is a one-line proof, the id normal form has exactly one wire-expressible violation and `parse_id_core` rejects it at every node close, and the 32-bit totality of `bit` and `Shr<u64>` is argued at the code. The test suite is the partition's strength: the id text parser is pinned by an exhaustive small-scope differential against a deliberately recursive reference plus whitespace-injected round trips, single-edit mutations, point pins, and a 100k-deep round trip; the build-history family is a lockstep model-based test of the buffer's representation invariant at every intermediate state; the padding and flush-cut families give every decode door a per-door witness for both rejection classes.

The dominant issues are prose and instruments that outlived the change that justified them. Three commits explain most of them: the 4-state pruned id encoding (2026-06-18) forced inline collapsible-pair checks into the text and literal doors and left the shared `validate_id` pass and its "single source of truth" sentence in place; the move of `Base` onto dashu (2026-07-24) removed the constraints behind the `Sub` debug assertion, the `SubAssign` clone, and the "spills at `u64`" test doc; and the recursion experiment and its same-day revert (2026-06-02) left a test doc describing a recursive validator. Two findings rise to medium: `base_dispatch_read_touches_no_digits` asserts a counter that `to_word` cannot reach, so any implementation passes it; and the `Party` literal door's public `O(n)` claim is quadratic on a nested spine because each level copies and re-validates the subtree below it. Two public-contract corners are owner questions: the clock door trims Unicode whitespace where every other door is ASCII-only, and the id and version text doors disagree on whether trailing junk outranks `NotCanonical`. The rest is low-grade simplification (a redundant validator pass, a two-pass stamp split with a depth counter that generated its own 2 GiB witness, a `BitsBuf` used as a stack where the crate owns `BitStack`) and nits.

## Findings

### codec-base-text-tree-1: Small inaccuracies in `Base`'s prose: "every operation records", an ambiguous shift clause, an undocumented `bit`, a ragged wrap
- Where: crates/before/src/codec/base.rs:16-26 (related: crates/before/src/codec/base.rs:46-50, crates/before/src/codec/base.rs:466-478, crates/before/src/codec/base/limb_meter.rs:16-19)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: prose, claims; refutation: confirmed; history: no rationale found (the quantifier was written when `Display` was already unmetered)
- Owner-gated: no

The type doc says every operation records limb widths, but `Display`, `bits`, `bit`, `to_u64`, and the `From` impls record nothing (the O(1) reads by design; `Display`'s conversion class is judged by the bench judge, per `tests/meter.rs:55-60`). The shift comment's "at or past usize bits" reads as amounts at or above `usize::BITS` where it means amounts `usize` cannot hold. `bit` is the one `pub(crate)` method without a doc comment, and the limb-meter module doc breaks a sentence mid-clause.

Evidence:

        20	/// `u64` overflow class, in any build profile. A thin metered wrapper around
        21	/// [`UBig`]: every operation records its operands' 64-bit limb widths into the
        22	/// limb meter, then delegates the arithmetic whole. Values up to two machine

       467	// exponent is u64). The two directions part on totality. A left shift's
       468	// checked conversion fails only for amounts at or past usize bits: never

        46	    pub(crate) fn bit(&self, i: u64) -> bool {

    (limb_meter.rs)
        16	//! envelopes read continuously across that arm seam. Relaxed ordering
        17	//! suffices: the metering binaries run one
        18	//! scenario per process and read the counters only after the metered call
        19	//! returns.

Resolution: Narrow the quantifier to "every arithmetic, comparison, equality, and hashing operation" and say the O(1) reads and `Display` do not record; rephrase the shift clause as "fails only for amounts `usize` cannot hold"; give `bit` a one-line doc; reflow limb_meter.rs:16-19. Acceptance: the type doc's quantifier matches limb_meter.rs:6-8; every `pub(crate)` fn in base.rs carries a doc comment; the paragraph reads as one wrapped block.

### codec-base-text-tree-2: The limb denomination `bits.div_ceil(64).max(1)` is spelled independently at several sites, and two `cfg` blocks exist only because `record_wide` takes a raw `UBig`
- Where: crates/before/src/codec/base.rs:64-69 (related: crates/before/src/codec/base/limb_meter.rs:38-42, crates/before/src/codec/base.rs:92-94, crates/before/src/codec/base.rs:142-144, crates/before/src/version/rank/num.rs:240, crates/before/src/version/rank/num.rs:339, crates/before/src/version/rank/num.rs:413, crates/before/src/version/rank/num.rs:489)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (`grep -rn 'div_ceil(64)' src`: base.rs:68, limb_meter.rs:41, num.rs:240 and :489 with `.max(1)`, num.rs:339 and :413 without; `record_wide` callers at dsi.rs:285, gamma.rs:262, integral.rs:370-372); executed: no
- Seen by: structure, claims; refutation: confirmed; history: no rationale found (both original spellings were born in the same commit)
- Owner-gated: no

The limb-meter module doc argues that the unit is "the value's width in 64-bit limbs, not any particular storage" and that the rank's wide arm records into the same unit, yet the formula has no single definition: `Base::limbs`, `record_wide`, and four `meter_wide` call sites in `num.rs` each spell it, two of them dropping the `.max(1)`. `parse_decimal` and `from_be_bytes` are the only operator bodies carrying a `#[cfg]`, and only because `record_wide` takes the raw `UBig` they wrap into a `Base` on the next line. Principle: one definition for a quantity the whole column rests on; the `limb_metered` shims exist to keep `cfg` noise out of the operator bodies.

Evidence:

        64	    /// The number of 64-bit limbs this magnitude occupies, at least one:
        65	    /// even a zero costs a word of arithmetic.
        66	    #[cfg(feature = "limb-meter")]
        67	    fn limbs(&self) -> u64 {
        68	        self.bits().div_ceil(64).max(1)
        69	    }

    (limb_meter.rs)
        38	/// Record the limb width of a raw `UBig` working value.
        39	pub(crate) fn record_wide(n: &dashu_int::UBig) {
        40	    use dashu_int::ops::BitTest;
        41	    record((n.bit_len() as u64).div_ceil(64).max(1));
        42	}

        92	        #[cfg(feature = "limb-meter")]
        93	        limb_meter::record_wide(&value);
        94	        Base(value)

Resolution: Add one `pub(crate) fn limbs_of_bits(bits: u64) -> u64` in `limb_meter` (with `use dashu_int::{ops::BitTest, UBig};` at module top), define `record_wide`, `Base::limbs`, and the `num.rs` `meter_wide` arguments through it; in `parse_decimal` and `from_be_bytes` write `let base = Base(value); meter_limbs_solo(&base); base`. Acceptance: `grep -rn 'div_ceil(64)' src/codec src/version/rank` shows one definition; `grep -n 'cfg(feature = "limb-meter")' src/codec/base.rs` shows only the module declaration and `limbs`; limb envelopes unchanged.

### codec-base-text-tree-3: `parse_decimal`'s rustdoc pins a probe measurement ("parse exponent 1.49") that no committed instrument holds
- Where: crates/before/src/codec/base.rs:71-83 (related: tools/benchjudge-expected.json (notes), crates/before/tests/meter.rs:55-60)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '1\.49\|dependency-selection' src tests tools`: base.rs:75, an unrelated ×1.49 floor at tests/meter.rs:8309, and the agent note `.agent-notes/2026-07-22-before-adversarial-resource-amplification/…md:423`; `tools/benchjudge-expected.json` notes record the parse trio at "measured e 1.28/1.33/1.30" under the 1.7 text ceiling); executed: no
- Seen by: prose, claims; refutation: confirmed; history: deliberate but expired (54b68b4f made the bench judge the class judge the same day f3da6377 placed the number)
- Owner-gated: no

The bracketed measurement and its source ("the dependency-selection probe") are dated rationale at a declaration site whose only provenance is an agent note; the committed instrument for the conversion class, which the same paragraph names, records different exponents for the same quantity. Principle 5 (no dated rationale or design-doc citations at code sites) and Principle 8 (a transcribed number is a hypothesis, not a measurement).

Evidence:

        73	    /// The radix conversion is delegated whole to the backend, whose
        74	    /// divide-and-conquer parser is subquadratic in the digit count
        75	    /// \[measured — the dependency-selection probe: parse exponent 1.49
        76	    /// over doubling digit counts\]. The conversion therefore runs inside
        77	    /// the dependency, below the limb shim, so this records one
        78	    /// width-proportional limb count for the materialized value — the
        79	    /// same convention as the wide-gamma decode — and the bench judge's
        80	    /// time leg is what judges the conversion's complexity class.

Resolution: Replace the bracketed clause with the instrument by name: "whose divide-and-conquer parser is subquadratic in the digit count; the bench judge's text-ceiling parse cells (`version_parse_trailing/hugeleaf`, `version_parse_noncanon/hugeleaf`, `clock_parse_trailing/hugeleaf`) hold the class." Keep the rest. Acceptance: `grep -rn '1\.49\|dependency-selection' crates/before/src` is empty; the paragraph names the bench judge as the sole authority for the class.

### codec-base-text-tree-4: `Base::msb_cmp` is a one-caller wrapper whose body the sibling match arms already spell inline
- Where: crates/before/src/codec/base.rs:97-106 (related: crates/before/src/version/rank/num.rs:277-292)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (`grep -rn 'msb_cmp\b' src`: the only production caller is num.rs:287; the other three arms call `msb_cmp_windows` directly); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (79a944ab extracted the kernel and left the wrapper)
- Owner-gated: no

The wrapper makes the four arms of `Num::msb_cmp` non-uniform, and its doc credits it with carrying arguments that live in `msb_cmp_windows`; the caller's doc justifies it as "the historical path", dated rationale at a declaration site (Principle 5).

Evidence:

       101	    /// The stored-magnitude instance of [`msb_cmp_windows`], which carries
       102	    /// the streaming argument, the tail rule's normalization premise, and
       103	    /// the per-window metering.
       104	    pub(crate) fn msb_cmp(a: &Base, b: &Base) -> Ordering {
       105	        msb_cmp_windows(a.msb_windows(), b.msb_windows())
       106	    }

    (num.rs)
       287	            (Num::Base(x), Num::Base(y)) => Base::msb_cmp(x, y),
       288	            (Num::Base(x), Num::Wide(y)) => msb_cmp_windows(x.msb_windows(), y.msb_windows()),
       289	            (Num::Wide(x), Num::Base(y)) => msb_cmp_windows(x.msb_windows(), y.msb_windows()),
       290	            (Num::Wide(x), Num::Wide(y)) => msb_cmp_windows(x.msb_windows(), y.msb_windows()),

Resolution: Delete `Base::msb_cmp`; write the same expression at num.rs:287 as the other arms and reword num.rs:281-284 to say all four pairings stream through one kernel. Acceptance: `grep -rn 'msb_cmp\b' src` shows only `Num::msb_cmp` and `msb_cmp_windows`; rank ordering tests green.

### codec-base-text-tree-5: `msb_cmp_windows` documents a stronger premise than `Rank` holds; the argument that makes the tail rule sound is unwritten
- Where: crates/before/src/codec/base.rs:148-159 (related: crates/before/src/version/rank.rs:520-534, crates/before/src/version/rank.rs:813-816, crates/before/src/version/rank.rs:883-906, crates/before/src/version/rank.rs:244-248, crates/before/src/version/rank/num/tests.rs:103-120)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read rank.rs `from_num` and the `debug_assert!` at 813-816; read the `Ord` impl at 883-906; read the differential at num/tests.rs:110-120); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (the premise was overstated from d8f91040 onward; `from_raw` already shifted by `tz.min(exp)`)
- Owner-gated: no

The tail rule ("the longer string is the larger value") is justified by "the caller's normalization invariant that the strings end in a set bit (an odd numerator)". `Rank`'s actual invariant is `exp == 0 || num.bit(0)`: an integral rank such as 2 is stored as numerator 2, exponent 0, an even numerator. The kernel is still correct for `Rank` because it runs only on a class tie (`bits(num) - exp` equal), where a longer numerator forces a larger exponent, hence `exp > 0`, hence oddness of the longer string, which is all the tail rule needs; that derivation appears nowhere, and rank.rs:889-893 repeats the overstatement. The differential ORs both operands with 1, so it never exercises an even operand. Statement faithfulness: a proof resting on a premise the caller does not hold is one a future reader will either distrust or extend wrongly.

Evidence:

       154	/// cost is O(shared-prefix limbs) with zero allocation. When every shared
       155	/// window agrees, the longer bit string is the larger value: this rides on
       156	/// the caller's normalization invariant that the strings end in a set bit
       157	/// (an odd numerator), so the longer string's extension is nonzero. The

    (rank.rs)
       813	    debug_assert!(
       814	        exp == 0 || num.bit(0),
       815	        "a nonempty fraction ends in its last set bit, so the numerator is odd"
       816	    );

       889	        // shift; its longer-string-wins tail rule is sound because
       890	        // normalization keeps numerators odd (the longer string ends in a set
       891	        // bit). The order is exact at any magnitude — a false tie here would

    (num/tests.rs)
       112	        // Odd operands: the tail rule's normalization premise (the
       113	        // stored numerator invariant).
       114	        let (a, b) = (a | UBig::ONE, b | UBig::ONE);

Resolution: Restate the premise at base.rs:155-157 as "the longer string ends in a set bit", and at rank.rs:889-891 write the one-line derivation: on a class tie, more numerator bits means a larger exponent, so `exp > 0` and the numerator is odd by normalization. Widen the differential to OR only the wider operand with 1 so an even shorter operand is exercised. Acceptance: base.rs and rank.rs carry the class-tie argument; the differential covers an even shorter operand and stays green.

Construction: Rank 2 (num 2, exp 0) versus Rank 5/2 (num 5, exp 1) tie at class 2; the first windows `[1<<63]` versus `[5<<61]` decide `Less` inside the window, correct. The tail rule is reached only across different widths, where the longer operand has `exp > 0`; no counterexample exists for `Rank`. The finding is that the written premise does not say this.

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

### codec-base-text-tree-7: `Sub`'s `debug_assert!` compares through the metered `Ord` and duplicates the backend's own underflow panic; `SubAssign` clones the whole magnitude
- Where: crates/before/src/codec/base.rs:416-430 (related: crates/before/src/codec/base.rs:284-295, crates/before/tests/meter.rs:60-65, crates/before/src/meter.rs:2123, crates/before/src/meter.rs:2511, crates/before/src/oracle/version.rs:398, crates/before/src/testing/shape_rows.rs:34)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read base.rs:284-295: `PartialOrd::partial_cmp` calls `Ord::cmp`, which calls `meter_limbs2`; read dashu-int 0.5.0 `src/add_ops.rs:185-235`, every negative-result arm calls `panic_negative_ubig`, defined at `src/error.rs:19-21`; read `src/helper_macros.rs:323-338`, `impl_binop_assign_by_taking!` emits `SubAssign<&UBig> for UBig` via `mem::take`, invoked at `add_ops.rs:9`; `grep -rn '\-= &' src` on `Base` values: meter.rs:2123, :2511, oracle/version.rs:398, testing/shape_rows.rs:34, plus test files); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: deliberate but expired (the assert named an underflow the pre-dashu `u64` arm would not name; 2bfc1398 made the backend panic on every negative result in every profile)
- Owner-gated: no

`debug_assert!(self >= *rhs, …)` resolves to `PartialOrd::ge` → `Ord::cmp` → `meter_limbs2`, so under `limb-meter` in a dev build every `Base - &Base` records twice its operands' limbs and once in release; `tests/meter.rs:63-64` carries that profile dependence as a caveat, and the refutation pass reports no other `debug_assert!` comparing two `Base` values in the crate, so this assert is the caveat's one source. dashu's `UBig` subtraction already panics on a negative result in every profile (`panic_negative_ubig`), so the assert names nothing the backend misses. `sub_assign` clones `self` to reuse `Sub` although dashu provides an in-place `SubAssign<&UBig>` by `mem::take`. Doctrine: asserts get the meters' adequacy scrutiny, and a guard beside a meter must not feed it; a deterministic counter should be a function of the operation sequence alone.

Evidence:

       416	impl Sub<&Base> for Base {
       417	    type Output = Base;
       418	
       419	    fn sub(self, rhs: &Base) -> Base {
       420	        meter_limbs2(&self, rhs);
       421	        debug_assert!(self >= *rhs, "Base subtraction underflow");
       422	        Base(self.0 - &rhs.0)
       423	    }
       424	}
       425	
       426	impl SubAssign<&Base> for Base {
       427	    fn sub_assign(&mut self, rhs: &Base) {
       428	        *self = self.clone() - rhs;
       429	    }
       430	}

    (tests/meter.rs)
        63	//! (limb counts shrink under release, where `debug_assert!` comparisons
        64	//! vanish, so the dev-profile pin is the binding one), while segment counts

Resolution: Delete the `debug_assert!` and add a `# Panics` line to the impl stating that an underflowing difference panics (the backend's own check, every profile; programmer error per the crate's panic policy). Rewrite `sub_assign` as `meter_limbs2(self, rhs); self.0 -= &rhs.0;`. Measure every limb-denominated envelope whose cell subtracts at the parent commit, then tighten the committed ceilings in the same change with the movement attributed to the assert's removal; strike the profile caveat at tests/meter.rs:63-64 if no other source remains. Acceptance: under `--features limb-meter`, `limb_ops()` after one `Base - &Base` on two k-limb operands reads 2k in dev and release alike; envelopes re-pinned with the parent measurement recorded; the meter.rs header either drops the caveat or names its remaining source.

### codec-base-text-tree-8: `Shl<i32>` and `BitOr<Base>` on `Base` exist for one test idiom; `Shl<i32>` wraps a negative amount in release
- Where: crates/before/src/codec/base.rs:448-455 (related: crates/before/src/codec/base.rs:503-510, crates/before/src/codec/tests.rs:51-61, crates/before/src/codec/tests.rs:521-546, crates/before/src/meter.rs:73, crates/before/src/version/skyline/query.rs:422)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn 'Shl<i32>\|<< 64)\|| Base::from' src tests benches examples fuzz fuzzfit wasm32-pins surfacecheck`: the only `Base` shifts by an unsuffixed literal and the only `|` on two `Base`s are codec/tests.rs:54 and :528, every other hit is on `u128`; `grep -rn '\bi32\b'` finds no other `i32` value in `before` outside a `powi` doc example, the ignored test's prose, and fuzzfit's wasm ABI; `Base::from(0u32)` and `acc += 1u32` have instrument-side callers at meter/board/defect.rs:102 and meter/board/tests.rs:266, 620-627, so `From<u32>` and `AddAssign<u32>` are not vestigial); executed: no
- Seen by: structure, correctness, claims; refutation: reframed (the `From<u32>`/`AddAssign<u32>` half is refuted by the board callers; `Shl<i32>` is not dead, it is what lets the two unsuffixed literals type-check); history: no rationale found (the idiom was written against num-bigint's operator matrix; the impls arrived with the owned enum and were carried onto dashu unreconsidered)
- Owner-gated: yes: under the `meter` feature `Base` values are reachable through `before::meter::skyline::query::min_ticks` (meter.rs:73 `pub use crate::version::skyline;`, query.rs:422 `pub fn min_ticks(bits: BitsView<'_>) -> Base`), so an operator impl on `Base` is observable from outside the crate even though the type has no public path

`n << 64` and `value << 64` in the two gamma tests fall back to `i32` for the literal, so `Shl<i32>` exists to let them compile; its `debug_assert!(rhs >= 0)` is its whole protection, and in release a negative amount casts to a `u32` near 2^32 and becomes an unbounded widening shift in the backend. `BitOr<Base>` is used only on the same two lines, where `|` means "append a limb" on a type whose contract is event counts. Types-first: a sign the type can carry should not be a runtime assertion; circular justification: an impl whose only reason to exist is a test's missing suffix.

Evidence:

       448	impl Shl<i32> for Base {
       449	    type Output = Base;
       450	
       451	    fn shl(self, rhs: i32) -> Base {
       452	        debug_assert!(rhs >= 0, "Base left shift must be non-negative");
       453	        self << rhs as u32
       454	    }
       455	}

       503	impl BitOr<Base> for Base {
       504	    type Output = Base;
       505	
       506	    fn bitor(self, rhs: Base) -> Base {
       507	        meter_limbs2(&self, &rhs);
       508	        Base(self.0 | rhs.0)
       509	    }
       510	}

    (codec/tests.rs)
        54	            n = (n << 64) | Base::from(limb);
       528	            value = (value << 64) | Base::from(limb);

Resolution: Delete both impls. At codec/tests.rs:54 and :528 write `(n << 64u32) + Base::from(limb)` (the low limb is disjoint from the shifted value, so `+` and `|` agree), or build the wide value with `UBig::from_le_bytes` as `base/tests.rs::from_limbs` does. Acceptance: `cargo check --all-features --all-targets -p before` and the detached workspaces compile without the impls; `grep -n 'Shl<i32>\|BitOr' src/codec/base.rs` is empty; `gamma_roundtrip_wide` and `gamma_word_encode_matches_bit_encode` stay green.

Construction: `Base::from(1u8) << -1i32` in a release-profile test: no assertion fires; the backend attempts a shift by 4_294_967_295 bits (a ~512 MiB allocation or a capacity panic), neither of which is a documented contract.

### codec-base-text-tree-9: The `limb_meter` module doc enumerates record sites by name and the list is stale; the materialization convention has no home
- Where: crates/before/src/codec/base/limb_meter.rs:6-16 (related: crates/before/src/codec/base/limb_meter.rs:38-42, crates/before/src/codec/dsi.rs:282-285, crates/before/src/version/skyline/query/integral.rs:352-376, crates/before/src/version/skyline/query/integral.rs:388-392, crates/before/src/codec/base.rs:132-139)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'record_wide\|limb_meter::record(' src` outside `codec/base`: dsi.rs:285, gamma.rs:262, num.rs:66, integral.rs:370-372 and :390); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate but expired (the roster was complete at 43486366; 3e5b95df and 14259c1f added recorders without touching this doc)
- Owner-gated: no

The doc names `codec::gamma` and `version::rank::num` as the recorders outside `Base`, but `codec::dsi` and `skyline::query::integral` also record. Meanwhile the convention every materializing site cites ("one width-proportional limb count for the materialized value") is stated by analogy at base.rs:78-79, base.rs:136-139, and integral.rs:355-356, but not at `record_wide`, whose doc is one line. Principle 5: no hand-maintained caller enumerations; the rule the sites follow belongs once, where the recorder lives.

Evidence:

         6	//! The proxy counted here is the operands' 64-bit limb counts per `Base`
         7	//! operation — arithmetic, comparison, equality, and hashing all record before
         8	//! they run, and the wide-gamma decode in `codec::gamma` records one
         9	//! value-width count per decoded value — so amortized-linear algorithms count

        12	//! not any particular storage: the rank numerator's wide arm
        13	//! (`version::rank::num`, magnitudes past the backend's capacity on
        14	//! 32-bit targets) records its operations' operand and materialization

        38	/// Record the limb width of a raw `UBig` working value.
        39	pub(crate) fn record_wide(n: &dashu_int::UBig) {

    (dsi.rs)
       284	        #[cfg(feature = "limb-meter")]
       285	        super::limb_meter::record_wide(&m);

Resolution: At `record_wide`, state the convention once: one value-width count wherever a wide value is materialized from bits, bytes, or text, because the backend touches every limb of the result and a meter that missed it would let a decoder build arbitrarily wide values while reading zero. In the module doc, state the two rules (operand widths per `Base` operation; one value-width per materialized wide value) without naming modules. Have base.rs:78-79 and 136-139 cite `record_wide` rather than "the wide-gamma decode". Acceptance: the module doc names no recording module; `record_wide`'s doc states the convention; base.rs's two materialization docs point at `record_wide`.

### codec-base-text-tree-10: `meter_limbs1` and `meter_limbs_solo` are both one-`Base` recorders whose difference lives only in their doc comments
- Where: crates/before/src/codec/base/limb_metered.rs:15-25 (related: crates/before/src/codec/base/limb_metered.rs:27-37, crates/before/src/codec/base.rs:366-400)
- Class / severity / confidence: idiom / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: structure; refutation: reframed (the digit is consistent, counting `Base` operands; the missing distinction is scalar-vs-solo); history: deliberate but expired (`meter_limbs_solo` arrived a day after `meter_limbs1`, when `1` stopped being the only one-operand recorder)
- Owner-gated: no

Each operator body is one meter call and one delegation, so the meter call's name is the whole statement of what is charged; `meter_limbs1` records `limbs + 1` (a `Base` and a machine scalar) while `meter_limbs_solo` records `limbs`, and a reader must open the doc to know which is which.

Evidence:

        15	/// Record a `Base`-with-machine-scalar operation's limb-scale work (the
        16	/// scalar counts as one limb).
        17	///
        18	/// Compiles to nothing without the `limb-meter` feature.
        19	#[inline(always)]
        20	pub(crate) fn meter_limbs1(a: &Base) {
        21	    #[cfg(feature = "limb-meter")]
        22	    limb_meter::record(a.limbs() + 1);

        27	/// Record a single-operand `Base` operation's limb-scale work (hashing
        28	/// walks every limb of its one operand).

Resolution: Rename `meter_limbs1` to `meter_limbs_scalar` (or `meter_limbs_with_word`); optionally `meter_limbs2` to `meter_limbs_pair` for symmetry. Acceptance: `grep -rn 'meter_limbs1' src` is empty; the names in base.rs read as the charge they make.

### codec-base-text-tree-11: `write_id`'s `sep` parameter has one caller and one value; the separator is already a named constant elsewhere
- Where: crates/before/src/codec/display.rs:16-28 (related: crates/before/src/party.rs:750-754, crates/before/src/version/skyline/text.rs:81-82)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (`grep -rn write_id src`: the only caller is party.rs:752, passing `", "`; skyline/text.rs:82 `const SEP: &str = ", ";`); executed: no
- Seen by: prose, correctness; refutation: confirmed; history: deliberate but expired (Phase 7 designed a second value, `" "` for `Debug`, and in the same commit made `Debug` delegate to `Display`, so the second value never had a caller)
- Owner-gated: no

A parameter whose only documented value is its one caller's constant exists to serve itself, and the paper's separator is spelled twice in the crate, once as dead generality.

Evidence:

        16	/// Write an id tree in the paper's grammar with `sep` between a node's two
        17	/// children (`", "`).

        24	pub(crate) fn write_id(
        25	    bits: BitsView<'_>,
        26	    f: &mut core::fmt::Formatter<'_>,
        27	    sep: &str,
        28	) -> core::fmt::Result {

    (party.rs)
       752	        codec::write_id(self.0.live(), f, ", ")

    (skyline/text.rs)
        81	/// The separator the paper notation prints between a node's parts.
        82	const SEP: &str = ", ";

Resolution: Drop the parameter; hoist one `SEP` constant to `codec::text` (both renderers and both parsers share the grammar) and use it from `write_id` and the skyline renderer. Acceptance: one definition of the separator in the crate; `Display` output byte-identical (snapshot and round-trip suites green).

### codec-base-text-tree-12: `write_id` keeps its phase stack on a `BitsBuf` while the crate owns `BitStack` for exactly that role, one of ten such stacks
- Where: crates/before/src/codec/display.rs:30-32 (related: crates/before/src/codec/stack.rs:1-2, crates/before/src/codec/buf.rs:175-181, crates/before/src/party/ops/compare.rs:115, crates/before/src/party/ops/sum.rs:136, crates/before/src/party/ops/diff.rs:253-257, crates/before/src/version/skyline/admit.rs:78-81, crates/before/src/version/skyline/grow.rs:516, crates/before/src/version/skyline/text.rs:239-240, crates/before/src/version/skyline/text.rs:355, crates/before/src/version/skyline/text.rs:512)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (`grep -rn '\.pop()' src` filtered to `BitsBuf`-typed receivers, cross-checked against each struct's field declarations: display.rs `pending` (57, 61); compare.rs `pending: BitsBuf` (180, 185); sum.rs `bits: BitsBuf` (178-190); diff.rs `path`, `pending_right: BitsBuf` (416, 429); admit.rs `path`, `left_was_leaf: BitsBuf` (151, 175, 185, 214); grow.rs `pending` (681); skyline/text.rs `phase` and `pending` (280, 379, 581). `BitStack::new()` users: party/ops/build.rs:197-198, skyline/fill.rs:1206-1208 and the sites 56a3dfe2 converted); executed: no
- Seen by: structure, correctness; refutation: confirmed (and extended the roster by compare.rs); history: no rationale found (56a3dfe2 claimed "every path, phase, and frame" but its file list has neither display.rs nor skyline/text.rs; `BitsBuf::pop` was added at 83e61b4d as a like-for-like replacement of bitvec's pop)
- Owner-gated: no

`codec::stack::BitStack` documents itself as "the word-backed bit stack the deep walks keep their paths and phases on", with O(1) register push and pop; `write_id` and nine sibling stacks across `compare.rs`, `sum.rs`, `diff.rs`, `admit.rs`, `grow.rs`, and `skyline/text.rs` instead pop a `BitsBuf`, the append-truncate build buffer whose `pop` is an asserted `get`, a `truncate`, and a tail mask that re-establishes a storage invariant no LIFO needs. Two disciplines for one role (Principle 3), and `BitsBuf::pop` exists only for these sites. The roster here is wider than any lens or the refutation pass recorded, which also means the conversion commit's completeness claim is further from true than the history pass found.

Evidence:

        30	    // Per open node: a phase bit on top ([`LEFT_PHASE`]/[`RIGHT_PHASE`]); under
        31	    // a left phase, the right child's presence bit.
        32	    let mut pending = BitsBuf::new();

    (stack.rs)
         1	//! Pop-able stacks held as bits: the word-backed bit stack the deep walks keep
         2	//! their paths and phases on, and the nonnegative-integer stack built over it.

    (buf.rs)
       175	    /// Pop the newest bit.
       176	    pub(crate) fn pop(&mut self) -> Option<bool> {
       177	        let pos = self.live.checked_sub(1)?;
       178	        let bit = self.get(pos);
       179	        self.truncate(pos);
       180	        Some(bit)
       181	    }

Resolution: Make `pending` a `BitStack` in `write_id` (same `push`/`pop` calls), and migrate the nine sibling stacks in the same change (`BitStack::len()` returns `u64`, matching the depth read at skyline/text.rs:572); then delete `BitsBuf::pop` once the compiler confirms no caller remains. While there, spell the closing arm at display.rs:68 as `Some(RIGHT_PHASE)` instead of `Some(_)`, and consider hoisting `LEFT_PHASE`/`RIGHT_PHASE` beside `BitStack` as the shared phase vocabulary, since skyline/text.rs:92 redefines `LEFT_PHASE`. Acceptance: `grep -n 'fn pop' src/codec/buf.rs` is empty; `deep_id_text_roundtrip`, `parse_stacks_handle_deep_spines`, and the display and text envelopes in `tests/meter.rs` stay green, with any heap envelope that moves re-measured at the parent before re-pinning.

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

### codec-base-text-tree-14: `parse_base`'s doc restates the conversion story `parse_decimal` owns and defines the grammar by an unnamed comparison
- Where: crates/before/src/codec/text.rs:46-54 (related: crates/before/src/codec/tests.rs:1048-1056, crates/before/src/codec/base.rs:71-83, crates/before/src/meter/board/tests.rs:221)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (`grep -rn 'digit-at-a-time' src`: text.rs:53, tests.rs:1053, and an unrelated query comment; `grep -n 'schoolbook_limb_ops\|digit-by-digit' src/meter/board/tests.rs` shows the digit-by-digit strategy alive as the board's schoolbook probe); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate and holds in part (the digit-by-digit strategy is not deleted code; it survives as the board's `schoolbook_limb_ops(text, 1)` probe, so the comparison has a live referent, but neither site names it)
- Owner-gated: no

The ghost-reference half of the lens finding does not survive history: "digit-at-a-time accumulation" refers to a live probe, not removed code. What remains is that `parse_base` restates the delegated-conversion paragraph `parse_decimal` already carries, and that the comparison names neither the probe nor the pin, so a reader cannot tell the referent is live. Every sentence competes with the grammar contract the reader came for (run boundary, whitespace, empty).

Evidence:

        50	/// The cursor slices the whole digit run and hands it to
        51	/// [`Base::parse_decimal`], which delegates the radix conversion to the
        52	/// backend's subquadratic divide-and-conquer parser; leading zeros are
        53	/// value-preserving (`"007"` is 7), exactly as digit-at-a-time accumulation
        54	/// would read them.

Resolution: At `parse_base`, keep the grammar only (maximal ASCII digit run after a leading whitespace skip, ended by the first non-digit, empty is `Parse::Syntax`, leading zeros value-preserving) and point to `Base::parse_decimal` for the conversion. If the accumulation comparison stays here or at tests.rs:1052-1053, name the board's schoolbook probe as its referent. Acceptance: the conversion delegation is described at exactly one site; any accumulation comparison names a live identifier.

### codec-base-text-tree-15: `parse_base` re-validates UTF-8 over a digit run it has already classified byte by byte
- Where: crates/before/src/codec/text.rs:64-66 (related: crates/before/src/codec/text.rs:10-13)
- Class / severity / confidence: performance / nit / high
- Provenance: assessed (read); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (`Cur` has held `&[u8]` since f3da6377 introduced the slice-and-delegate form)
- Owner-gated: no

The loop at 58-60 has already proved every byte in `start..pos` is an ASCII digit; `core::str::from_utf8` walks them again. Keeping the `&str` in `Cur` and slicing it (both ends are on ASCII digits, hence char boundaries) removes the pass and the `expect`. Fixed-sign deletion, denominated in digit-run bytes.

Evidence:

        64	    let digits = core::str::from_utf8(&cur.bytes[start..cur.pos])
        65	        .expect("an ASCII digit run is valid UTF-8");
        66	    Ok(Base::parse_decimal(digits))

Resolution: Store `s: &'a str` in `Cur`, scan via `s.as_bytes()`, and slice the run as `&cur.s[start..cur.pos]`. Acceptance: no `from_utf8` in `parse_base`; the codec and skyline text suites unchanged.

### codec-base-text-tree-16: `parse_id_str` re-validates bits its own parser built canonically; the test reference runs the same pass, and `validate_id`'s "single source of truth" is untrue
- Where: crates/before/src/codec/text.rs:75-84 (related: crates/before/src/codec/text.rs:161-176, crates/before/src/codec/tree.rs:104-106, crates/before/src/codec/literal.rs:52-66, crates/before/src/codec/tests.rs:1729-1743, crates/before/src/codec/tests.rs:1747-1757)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn validate_id src`: definition tree.rs:114, calls at text.rs:82, literal.rs:64, tests.rs:1741; the cannot-fail argument is assessed by reading text.rs:129-180 and literal.rs:47-66); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: deliberate but expired (Phase 7's "the string and literal paths share the one normal-form validator" was written when the text parser emitted raw bits with no canonicality check; 32a655438 made `(0, 0)` unspellable, forced the inline arms in, and kept the pass and the sentence)
- Owner-gated: no

`parse_id_tree` emits exactly the presence-tag preorder of the tree it parses (every `(` reserves a tag patched at `)` from its children's kinds, every `1` emits `00`, a `0` emits nothing) and rejects both collapsible forms before patching, so its `Ok` output is always one complete canonical tree; `parse_id_core` rejects only a `(1, 1)` node or a malformed stream, neither of which the parser can emit (a `Node` child's tag is never `00`, so the wire's terminal test coincides with `IdKind::Terminal`). Line 82 therefore cannot return `Err`; it is a recompute-and-compare guard on a deterministic pure function, an extra O(n) pass on every `Party::from_str` and `Clock::from_str`, and because the reference parser at tests.rs:1741 makes the identical call, the exhaustive and proptest differentials compare `Ok(bits)` to `Ok(bits)` and are blind to the call's presence or absence. The inline `(0, 0)` arm cannot be replaced by the validator (the form has no bit spelling; dropping it would parse `((0, 0), 1)` as `(0, 1)`), so the validator is the redundant half. The same shape recurs at literal.rs:64, whose children come only from `id_leaf`/`id_node`. Meanwhile tree.rs:105 calls `parse_id` "the single source of truth for id normal form" while the collapsible rule is implemented at three sites. Doctrine: a guard must name a constructible failure the committed tests cannot catch; recompute-and-compare on a pure function is not defense in depth once differential coverage exists.

Evidence:

        75	pub(crate) fn parse_id_str(s: &str) -> Result<BitsBuf, Parse> {
        76	    let mut cur = Cur::new(s);
        77	    let mut bits = BitsBuf::new();
        78	    parse_id_tree(&mut cur, &mut bits)?;
        79	    if cur.peek().is_some() {
        80	        return Err(Parse::Syntax); // trailing junk
        81	    }
        82	    validate_id(super::built_view(&bits))?;
        83	    Ok(bits)
        84	}

       165	                    match (left, kind) {
       166	                        (IdKind::Empty, IdKind::Empty) => return Err(Parse::NotCanonical), // (0, 0)
       167	                        (IdKind::Terminal, IdKind::Terminal) => {
       168	                            return Err(Parse::NotCanonical); // (1, 1)
       169	                        }

    (tree.rs)
       104	/// Confirm a freshly built id bit stream is exactly one canonical-normal-form
       105	/// tree. Wraps [`parse_id`] (the single source of truth for id normal form),
       106	/// mapping its outcome onto [`Parse`].

    (tests.rs)
      1741	    super::validate_id(crate::codec::built_view(&bits))?;

Resolution: Delete text.rs:82 and literal.rs:64 (behavior-preserving: both return `Ok` on every reachable input). Move the guarantee they stood in for ("text-door acceptance is contained in wire-decode acceptance") into the harness: in `assert_id_parse_matches_reference` (tests.rs:1747-1757), on `Ok(bits)` also assert `validate_id(built_view(&bits)).is_ok()`, and drop the reference's own `validate_id` call so the reference is a pure grammar transcription; add the same assertion to a literal-door test. Reword tree.rs:105 to "the wire decoders' normal-form check". If the owner prefers the runtime pass as a totality defense against a future emission bug, state that rationale at each call site instead of the "single source of truth" sentence. Acceptance: every accepted `parse_id_str` and `id_node` output is asserted canonical by a test, not by a production re-parse; `id_text_parser_matches_reference_exhaustively`, the three id-parser proptests, `id_text_parser_error_precedence_pins`, `deep_id_text_roundtrip`, and the literal tests stay green; tree.rs:105 claims no single source of truth.

Construction: Temporarily replace `validate_id(...)?` at text.rs:82 with `debug_assert!(validate_id(...).is_ok())` and run the id text parser pins and `deep_id_text_roundtrip`; nothing fires. Conversely delete line 82 outright and run the codec suite: every test passes, because the reference validates its own output. `grep -n NotCanonical src/codec/{tree,text,literal}.rs` shows the collapsible rule at three sites.

### codec-base-text-tree-17: Prose texture in the parsers: two private `IdFrame` enums, the stack discipline restated five times, fragment heads, the "X, never Y" figure, and "exactly" as intensifier
- Where: crates/before/src/codec/text.rs:98-120 (related: crates/before/src/codec/text.rs:69-74, crates/before/src/codec/text.rs:182-184, crates/before/src/codec/tree.rs:5-19, crates/before/src/codec/tree.rs:21-27, crates/before/src/codec/display.rs:16-23, crates/before/src/codec/base/tests.rs:25-32)
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified (`grep -c ', never ' ` over the production files and tests.rs: base.rs 1, display.rs 1, text.rs 2, tree.rs 3, tests.rs 7; `grep -rn 'enum IdFrame' src/codec`: tree.rs:11 and text.rs:105); executed: no
- Seen by: structure, prose; refutation: confirmed; history: no rationale found (56b27eed introduced `text::IdFrame` beside `tree::IdFrame` and the first restatement; later docs passes added the rest)
- Owner-gated: no

`text::IdFrame` (tag position and left kind) and `tree::IdFrame` (both/unary presence frames) share a name for differently shaped types, so a grep or backtrace lands on two. The explicit-frame-stack discipline is stated at text.rs:72-74, 102-104, and 128 and at tree.rs:7-8 and 26-27; the crate's hard rule asks for it once, where it lives (the frame type). "Iterative." stands alone at text.rs:184 over a flat byte loop with nothing that could recurse. Fragment-headed sentences ("Iterative: …", "Adequacy: …"), fourteen "X, never Y" clauses, "exactly" as intensifier (text.rs:54, 104, 127; tree.rs:34), first person at tree.rs:5, and the parenthetical question at tree.rs:14-15 are default-dialect texture rather than mechanism.

Evidence:

       102	/// One frame per unfinished ancestor, on an explicit heap `Vec` — as deep as
       103	/// the nesting, never the call stack, exactly the packed parsers' discipline
       104	/// ([`super::tree`]).
       105	enum IdFrame {

        72	/// Iterative, like the packed-tree parsers in [`super::tree`]: depth lives on
        73	/// an explicit frame stack, never the call stack, so nesting depth cannot
        74	/// overflow.

       184	/// returns the event side for the caller's version parser. Iterative.

    (tree.rs)
         5	/// While building a node bottom-up, what we still need from the stream.

        14	    /// A both-present node whose left child is parsed (a terminal? — needed for
        15	    /// the `(1, 1)` check); the next subtree is its right child.

Resolution: Rename the text parser's frame `TextFrame` (or `IdTextFrame`). State the stack discipline once per file at the frame type and have `parse_id_str`, `parse_id_tree`, and `parse_id` say only what differs; drop "Iterative." at text.rs:184. Recast fragment heads as sentences, reserve "exactly" for measures, replace "what we still need" with "what the node still needs", and the tree.rs:14-15 parenthetical with "whether the left child was a terminal, for the `(1, 1)` check". Acceptance: `grep -rn 'enum IdFrame' src/codec` returns one hit; the explicit-stack claim appears once per file; text.rs:184's "Iterative." is gone.

### codec-base-text-tree-18: The id and version text doors disagree on whether trailing junk outranks `NotCanonical`
- Where: crates/before/src/codec/text.rs:161-169 (related: crates/before/src/codec/text.rs:79-81, crates/before/src/version/skyline/text.rs:488-494, crates/before/src/version/skyline/text.rs:617-622, crates/before/src/codec/tests.rs:1709-1711, crates/before/src/error.rs:106-121)
- Class / severity / confidence: api-surprise / nit / high
- Provenance: assessed (read both parsers' close and trailing-check order and the `Parse` docs); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (before 32a655438 both doors deferred `NotCanonical` behind the trailing-junk check; that commit moved the id door's check to node close without mentioning precedence, and e83ef600 and 6aa6fe54 then pinned each door separately)
- Owner-gated: yes: `FromStr` error variants on public types

The id parser returns `Parse::NotCanonical` the moment a collapsible node closes, before the trailing-input check at 79-81; the skyline parser defers `NotCanonical` until after the whole syntax pass, so trailing junk outranks it there. One notation, one error type, two precedence rules, and `Parse`'s docs state neither. The reference parser mirrors the id door's rule, so the differential cannot see the cross-door divergence.

Evidence:

       161	                Some(IdFrame::NeedRight { tag, left }) => {
       162	                    if cur.bump() != Some(b')') {
       163	                        return Err(Parse::Syntax);
       164	                    }
       165	                    match (left, kind) {
       166	                        (IdKind::Empty, IdKind::Empty) => return Err(Parse::NotCanonical), // (0, 0)
       167	                        (IdKind::Terminal, IdKind::Terminal) => {
       168	                            return Err(Parse::NotCanonical); // (1, 1)
       169	                        }

    (skyline/text.rs)
       491	/// sibling leaves) is checked at each close and reported after the whole syntax
       492	/// pass, so syntax errors — including trailing junk — outrank
       493	/// [`Parse::NotCanonical`].

       617	    if cursor.peek().is_some() {
       618	        return Err(Parse::Syntax); // trailing junk
       619	    }
       620	    if !canonical {
       621	        return Err(Parse::NotCanonical);
       622	    }

Resolution: Owner ruling on which rule wins, then align: either defer `NotCanonical` in `parse_id_tree` (a `canonical` flag reported after the trailing check, as the skyline parser does, mirrored in `ref_parse_id_node`), or document the per-node rule on `Parse` and in the skyline parser. Add one cross-door pin with the same trailing-junk-plus-collapsible text through both doors. Acceptance: `"(1, 1) x".parse::<Party>()` and `"(0, 5, 5) x".parse::<Version>()` return the same variant, and `Parse`'s rustdoc states the precedence.

Construction: `"(1, 1) x".parse::<Party>()` returns `Err(Parse::NotCanonical)` (line 168 fires at `)` before line 79 sees ` x`); `"(0, 5, 5) x".parse::<Version>()` returns `Err(Parse::Syntax)` (skyline/text.rs:617 fires before :620).

### codec-base-text-tree-19: `parse_clock_str` pre-scans for the top-level comma with an `i64` depth counter, work the id parser on a cursor already does
- Where: crates/before/src/codec/text.rs:185-215 (related: crates/before/src/codec/tests.rs:1926-1957, crates/before/src/codec/tests.rs:1959-1976, crates/before/src/clock.rs:942-952, crates/before/src/version/skyline/text.rs:494-502, crates/before/src/version/skyline/text.rs:617-618, crates/before/src/error.rs:106-121)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read the scanner, the ignored witness, `Clock::from_str`, and the skyline parser's cursor-based body and trailing check); executed: no
- Seen by: structure, claims; refutation: confirmed (with the owner gate added); history: no rationale found (the two-pass split is Phase 7's original; d089bca0 widened the counter and added the never-panics family and the 2 GiB witness without weighing an alternative to the pre-scan)
- Owner-gated: yes: the `Parse` variant moves for one input class (`"((1, 1) 0)"`: `Syntax` today, `NotCanonical` under a cursor-based split)

The stamp split walks `inner` once counting paren depth to find the first depth-0 comma, then hands the prefix to `parse_id_str`, which parses the same bytes again. Parsing the id directly from a `Cur` over `inner` and requiring the next token to be `,` yields the same accept set and value: a well-formed id has balanced parens and every comma at depth ≥ 1, so the first depth-0 comma is the one after the id; a malformed id rejects either way. The counter generated its own maintenance cascade: an `i32` to `i64` widening, the overflow proof at 192-193, a never-panics proptest, and a ~2 GiB `#[ignore]`d test that exists only to defend the counter's width. Principle 3: infrastructure that generates its own maintenance cascade is suspect; fixed-sign deletion, denominated in id-text bytes scanned.

Evidence:

       192	    // i64 cannot overflow: depth moves by at most one per input byte, and an
       193	    // allocation holds at most `isize::MAX` (< 2⁶³) bytes.
       194	    let mut depth: i64 = 0;
       195	    let mut split = None;
       196	    for (k, &c) in inner.as_bytes().iter().enumerate() {
       197	        match c {
       198	            b'(' => depth += 1,
       199	            b')' => {
       200	                depth -= 1;
       201	                if depth < 0 {
       202	                    return Err(Parse::Syntax);
       203	                }
       204	            }
       205	            b',' if depth == 0 => {
       206	                split = Some(k);
       207	                break;
       208	            }
       209	            _ => {}
       210	        }
       211	    }
       212	    let k = split.ok_or(Parse::Syntax)?;
       213	    let id_bits = parse_id_str(&inner[..k])?;
       214	    Ok((id_bits, &inner[k + 1..]))

Resolution: Minimal shape: keep the outer-paren strip; then `let mut cur = Cur::new(inner); let mut bits = BitsBuf::new(); parse_id_tree(&mut cur, &mut bits)?; if cur.bump() != Some(b',') { return Err(Parse::Syntax); } Ok((bits, cur.rest()))` with a `Cur::rest(&self) -> &'a str` accessor (the position after an ASCII `,` is a char boundary). Deeper shape: add a cursor-taking entry to `skyline::text` (its body already runs on `Cur` and its trailing check is one `peek`) and make the stamp one cursor pass returning both bit streams. Either way delete the depth loop, its comment, and `clock_text_split_survives_two_gib_of_parens`; keep `clock_text_deep_nesting_never_panics`, which exercises the whole `Clock::from_str`. Add a point pin for the precedence corner so the change is visible. Acceptance: `grep -n 'depth: i64' src/codec/text.rs` is empty; the 2 GiB ignored test is gone; the clock text pins and the never-panics proptest stay green; the clock `FromStr` fuel band in fuzzfit is unchanged or lower, measured at the parent.

### codec-base-text-tree-20: The clock text door trims Unicode whitespace; every other door and the cursor's own contract are ASCII-only
- Where: crates/before/src/codec/text.rs:185-191 (related: crates/before/src/codec/text.rs:5-9, crates/before/src/codec/text.rs:23-27, crates/before/src/party.rs:784-789, crates/before/src/version.rs:1454-1459, crates/before/src/codec/tests.rs:1766, crates/before/src/codec/tests.rs:1803)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read `parse_clock_str`, `Cur::skip_ws`, and the `Party`/`Version` `FromStr` impls, which go straight to `Cur`-based parsers with no trim; std documents `str::trim` as `char::is_whitespace`, Unicode White_Space, which includes U+000B and U+00A0, and `u8::is_ascii_whitespace` as excluding U+000B); executed: no
- Seen by: correctness, claims; refutation: confirmed (with the owner gate added); history: no rationale found (`s.trim()` is Phase 7's original line, introduced beside `Cur`'s ASCII predicate with no recorded whitespace-class decision)
- Owner-gated: yes: narrows what a public `FromStr` accepts

`parse_clock_str` strips the stamp's outer whitespace with `str::trim`, while `Cur::skip_ws` (and hence the `Party` and `Version` doors, and the id and event halves inside the same stamp) skip `u8::is_ascii_whitespace` only. The same byte sequence is accepted at the stamp's edges and rejected everywhere else, contradicting the module's own contract that the grammar is pure ASCII and that the skyline kernel "must make byte-identical grammar decisions to this module's". The exhaustive alphabet (`b"()01, "`) and the whitespace injector (`b" \t\n\r"`) are ASCII-only, so no committed instrument sees the divergence. Correct at all scales, for all inputs: three doors spell one notation.

Evidence:

       185	pub(crate) fn parse_clock_str(s: &str) -> Result<(BitsBuf, &str), Parse> {
       186	    let t = s.trim();
       187	    let bytes = t.as_bytes();
       188	    if bytes.first() != Some(&b'(') || bytes.last() != Some(&b')') {
       189	        return Err(Parse::Syntax);
       190	    }
       191	    let inner = &t[1..t.len() - 1];

         5	/// A whitespace-skipping byte cursor over the input string. The grammar is pure
         6	/// ASCII (`(`, `)`, `,`, digits, `0`/`1`), so byte-level scanning is exact.

        24	        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {

Resolution: Trim with the cursor's own predicate, `s.trim_matches(|c: char| c.is_ascii_whitespace())`, or let the cursor-based split of finding 19 do the skipping through `Cur` so the stamp door has no whitespace rule of its own. Add a point pin beside `id_text_parser_error_precedence_pins` asserting the three `FromStr` doors agree on a leading U+000B and U+00A0 (all `Err(Parse::Syntax)`). Acceptance: `"\u{0B}(1, 0)".parse::<Clock>()` and `"\u{A0}(1, 0)".parse::<Clock>()` return `Err(Parse::Syntax)`, matching `"\u{0B}1".parse::<Party>()`; the pin and `clock_text_deep_nesting_never_panics` stay green.

Construction: `"\u{0B}(1, 0)".parse::<Clock>()` is `Ok` today (vertical tab is Unicode White_Space, stripped by `trim`), while `"(1,\u{0B}0)".parse::<Clock>()` is `Err(Parse::Syntax)` (the event text `\u{0B}0` reaches `Cur`, which does not skip U+000B) and `"\u{0B}1".parse::<Party>()` is `Err(Parse::Syntax)`. The same with U+00A0 in place of U+000B.

### codec-base-text-tree-21: `parse_id`'s `pos` parameter is always 0, and three named layers wrap one grammar body
- Where: crates/before/src/codec/tree.rs:35-60 (related: crates/before/src/codec.rs:71-80, crates/before/src/borsh_impls.rs:131-135, crates/before/src/borsh_impls/tests.rs:372-384, crates/before/src/party.rs:630, crates/before/src/clock.rs:801)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (`grep -rn 'parse_id(' src`: party.rs:630, clock.rs:801, tree.rs:118, all passing `0`; `grep -rn parse_id_from src`: definition tree.rs:44 under `cfg(all(test, feature = "borsh"))`, the re-export codec.rs:80, one caller borsh_impls/tests.rs:378; production borsh at borsh_impls.rs:133 calls `parse_id_core` and reads its own position via `finish()`); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate but expired (78c65371 split the layers because the byte doors read the end position at `u64` width while other cursors read `usize`; 05d87e1b unified every position at `u64` and kept the layering)
- Owner-gated: no

Every caller of `parse_id` passes `pos = 0`; `parse_id` and the test-gated `parse_id_from` are both `parse_id_core` plus `cursor.position()`; and `codec.rs:71-80` carries three re-export blocks with two `cfg`s and explanatory prose to expose them. The constraint that motivated the `()`-returning body (differing position widths) is gone.

Evidence:

        35	pub(crate) fn parse_id(bits: BitsView<'_>, pos: u64) -> Result<u64, Decode> {
        36	    let mut cursor = super::DsiCursor::new_at(bits, pos);
        37	    parse_id_core(&mut cursor)?;
        38	    Ok(cursor.position())
        39	}

        43	#[cfg(all(test, feature = "borsh"))]
        44	pub(crate) fn parse_id_from<C: BitCursor>(cursor: &mut C) -> Result<u64, Decode>

    (codec.rs)
        75	#[cfg(feature = "borsh")]
        76	pub(crate) use tree::parse_id_core;
        77	// The generic-position parse serves the wire-side (borsh) test suite; the
        78	// grammar body above is what production readers drive.
        79	#[cfg(all(test, feature = "borsh"))]
        80	pub(crate) use tree::parse_id_from;

Resolution: One always-compiled `parse_id_from<C: BitCursor>(cursor: &mut C) -> Result<u64, Decode>` (the grammar body, returning the end position); `parse_id(bits: BitsView<'_>) -> Result<u64, Decode>` as `parse_id_from(&mut DsiCursor::new_at(bits, 0))`; borsh calls `parse_id_from(&mut cursor)?;` and drops the position. `codec.rs` re-exports shrink to `tree::{parse_id, validate_id}` plus one `#[cfg(feature = "borsh")] pub(crate) use tree::parse_id_from;`. Acceptance: `grep -rn parse_id_core src` is empty; `grep -rn 'parse_id(' src` shows single-argument calls only; the borsh tests and `parse_stacks_handle_deep_spines` green.

### codec-base-text-tree-22: Long qualified paths beside existing imports, and two `DefaultHasher` helpers, in the codec test suite
- Where: crates/before/src/codec/tests.rs:14-17 (related: crates/before/src/codec/tests.rs:307-314, crates/before/src/codec/tests.rs:729-734, crates/before/src/codec/tests.rs:590, crates/before/src/codec/tests.rs:1741)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -c 'crate::codec::' src/codec/tests.rs` = 22 in a file that imports `BitsView` at line 15; `super::Bits::freeze` occurs 12 times; `crate::error::Parse` is both fully qualified and function-locally imported; `hash_of` and `default_hash` both build a `DefaultHasher`); executed: no
- Seen by: structure, correctness; refutation: confirmed; history: no rationale found (accretion across commits; `built_view` and `extend_from_view` have always been `pub(crate)` re-exports at codec.rs:45)
- Owner-gated: no

Imports over long qualified paths except where the qualification informs; here the same item is spelled both ways in one file. `hash_of(b)` equals `default_hash(&b.as_raw_slice())`, so two helpers name one operation.

Evidence:

        14	use super::{
        15	    bits_buf, decode_int, decode_int_from, encode_int, Base, BitCursor, BitsBuf, BitsView,
        16	    DsiCursor, SliceCursor,
        17	};

       590	        let bits = crate::codec::BitsView::whole(&bytes);

       307	fn hash_of(bits: &super::Bits) -> u64 {
       308	    use core::hash::{Hash, Hasher};
       309	    let mut hasher = std::hash::DefaultHasher::new();

       729	fn default_hash<T: std::hash::Hash>(v: &T) -> u64 {

Resolution: Extend the `use super::{…}` list with `built_view, extend_from_view, Bits, canonical_eq, padding_is_canonical` and add `use crate::error::Parse;` at file top; define `hash_of` as `default_hash(&bits.as_raw_slice())` or use `default_hash` at its three call sites. Acceptance: `grep -c 'crate::codec::' src/codec/tests.rs` reports 0; one `DefaultHasher::new()` in the file.

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

### codec-base-text-tree-24: Em-dashes in `//` code comments across the partition
- Where: crates/before/src/codec/tests.rs:214-222 (related: crates/before/src/codec/base.rs:266-273, crates/before/src/codec/base.rs:473, crates/before/src/codec/base.rs:497, crates/before/src/codec/tree.rs:66-67)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -n '//.*—' <file> | grep -v '///\|//!'` over the eight partition files returns 23 lines: base.rs 267, 269, 272, 473, 497; tree.rs 66; tests.rs 184, 214, 215, 221, 222, 624, 1017, 1077, 1196, 1199, 1230, 1231, 1233, 1376, 1377, 1476, 1637; the same grep for em-dashes inside quoted assert or expect messages is empty); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (the repo has applied the colon rule to messages; 374 em-dash `//` comments exist in before/src and were still being added on 2026-08-18)
- Owner-gated: no (but see the open question on scope)

Owner doctrine: colons or semicolons over em-dashes in log messages and comments alike; rendered rustdoc is exempt, `//` comments are not. No assert, expect, or debug_assert message in the partition contains one, which is the half of the rule the repo has enforced.

Evidence:

       214	// The build buffer promises that its byte image — and therefore the sealed
       215	// spelling the freeze door emits — is a function of the bit *content* alone,

    (tree.rs)
        66	        // `summary` is whether the just-completed subtree is a terminal — the
        67	        // only fact a parent needs, to reject `(1, 1)`.

Resolution: Recast each as a colon, semicolon, parenthetical, or two sentences; the grep above is the mechanical sweep. Acceptance: that grep returns nothing for the eight partition files.

### codec-base-text-tree-25: Register words: "honest", "genre", "keystone", "real", "major finding", a caps `WITNESS` label, and a "Test-only" mislabel
- Where: crates/before/src/codec/tests.rs:1225-1238 (related: crates/before/src/codec/tests.rs:235, crates/before/src/codec/tests.rs:1128, crates/before/src/codec/tests.rs:1306, crates/before/src/codec/tests.rs:1341, crates/before/src/codec/tests.rs:1359, crates/before/src/codec/tests.rs:1476, crates/before/src/codec/tests.rs:1503, crates/before/src/codec/tests.rs:1598, crates/before/src/codec/base.rs:157-159, crates/before/src/codec/base.rs:10-13, crates/before/Cargo.toml:80-84, crates/before/Cargo.toml:115-117)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n -i 'honest\|genre\|keystone\|major finding\|WITNESS\|\breal\b'` over the partition: honest ×1 (base.rs:159), genre ×6, keystone ×1 (1234), "major finding" ×1 (1238), the `WITNESS —` testdoc head (1359), "real" ×1 (1598); Cargo.toml:80-82 says `limb-meter` "stays out of the bench builds" and :115-117 requires it for the `amp_board` example); executed: no
- Seen by: prose, claims; refutation: confirmed (with the correction that the feature is out of bench builds, not in them); history: no rationale found ("honest" entered at d8f91040 against the owner's stated rule; the "Test-only" label was written after the example already required the feature)
- Owner-gated: no

Moralized or transplanted vocabulary where a plain word carries the meaning: "honest" for a counter, "genre" for an error class, "keystone" for an invariant, "real" for organic trees, review vocabulary ("is a major finding") in a test comment, and an all-caps label opening a testdoc. base.rs:10 labels `limb_meter` "Test-only" with a colon-fronted fragment, though the `amp_board` example requires the feature and the always-compiled `limb_metered` module sits under the same label.

Evidence:

      1234	// value lowers to a normal-form oracle tree (the keystone byte-canonicity
      1235	// invariant, the thing byte-equality `Eq`/`Hash` rests on) *and* re-encodes to

      1238	// or one whose re-encode disagrees with its own input, is a major finding.

      1359	/// WITNESS — the padding boundary cases the two mutation proptests above

    (base.rs)
       158	/// limb meter records one limb per streamed window pair, keeping the
       159	/// metered cost honest about the scan.

        10	// Test-only metering for big-arithmetic operations:
        11	#[cfg(feature = "limb-meter")]
        12	pub(crate) mod limb_meter;
        13	pub(crate) mod limb_metered;

Resolution: "so the recorded cost counts exactly the window pairs the scan compared"; "class" or "kind" for "genre"; "the byte-canonicity invariant" for "keystone …"; "organic" or "production" for "real"; drop "is a major finding" (state the contract and stop); "The padding boundary cases …" without the caps label; base.rs:10 → "// Instrument-only limb metering (the `limb-meter` feature) and its always-compiled shims." Acceptance: a grep for `honest|genre|keystone|major finding|WITNESS` over the partition is empty; base.rs:10 no longer says "Test-only".

### codec-base-text-tree-26: Hand-maintained counts: "the 256 uniform-random vectors" and "the five marker-padded wire types"
- Where: crates/before/src/codec/tests.rs:1227-1228 (related: crates/before/src/codec/tests.rs:1476-1478, crates/before/src/clock/tests.rs:763-773)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (clock/tests.rs:763 `proptest! {` … :773 `fn decode_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..512))` with no `ProptestConfig` in the file, so 256 is proptest's default case count; tests.rs:1477 writes "five" beside the five-name list); executed: no
- Seen by: structure, prose, correctness; refutation: confirmed; history: no rationale found (the "256" has transcribed a configuration default since 5a2679c9)
- Owner-gated: no

Principle 5: no hand-maintained counts. The case count moves with proptest configuration and the type list moves with the wire; neither edit touches these comments.

Evidence:

      1227	// The 256 uniform-random vectors in `decode_never_panics` are a thin panic net:
      1228	// truly random bytes almost never form a *nearly*-valid stream, so they barely

      1476	// missing entirely — the truncation genre, through every decode door. The
      1477	// family spans the five marker-padded wire types (`Party`, `Version`,
      1478	// `Clock`, `Ranked`, `Span`); `Rank` has no marker padding (its stream is

Resolution: "The uniform-random vectors in `clock::tests::decode_never_panics` are a thin panic net"; "The family spans the marker-padded wire types (`Party`, `Version`, `Clock`, `Ranked`, `Span`)". Acceptance: neither number appears in the two comments.

### codec-base-text-tree-27: The reference parser duplicates the production tokenizer and kind enum byte for byte, and the header does not say why
- Where: crates/before/src/codec/tests.rs:1656-1683 (related: crates/before/src/codec/tests.rs:1636-1642, crates/before/src/codec/tests.rs:1644-1654, crates/before/src/codec/text.rs:10-44, crates/before/src/codec/text.rs:86-96, crates/before/src/codec/tests.rs:1766)
- Class / severity / confidence: test-quality / nit / medium
- Provenance: assessed (read both cursors and both enums side by side; the exhaustive alphabet at 1766 contains one whitespace byte); executed: no
- Seen by: structure; refutation: confirmed; history: deliberate and holds (6aa6fe54 framed the reference as a full independent transcription while `Cur` was already `pub(crate)`, so the copy was a choice; the tokenizer-freeze intent is unstated)
- Owner-gated: no

`RefCur` is textually identical to `text::Cur` and `RefIdKind` to `text::IdKind`. A byte-identical copy cannot disagree with its original today; what it can do is freeze the tokenizer against a future edit to `Cur`, and that intent is not written at the header (which attributes the differential's independence to the recursive transcription alone) and is only partly exercised, since the exhaustive alphabet's sole whitespace is a space. An undocumented deliberate choice: state the rationale at the site.

Evidence:

      1656	/// The reference cursor: byte-level, skipping ASCII whitespace before every
      1657	/// token, exactly the grammar's tokenization.
      1658	struct RefCur<'a> {
      1659	    bytes: &'a [u8],
      1660	    pos: usize,
      1661	}

    (text.rs)
        10	pub(crate) struct Cur<'a> {
        11	    bytes: &'a [u8],
        12	    pos: usize,
        13	}

Resolution: Either state in the header (1636-1642) that the copies freeze the tokenizer and extend `ALPHABET` at 1766 with `\t` or `\n` so the freeze is exercised, or reuse `Cur` and make `IdKind` `pub(super)`, stating that the differential pins the parse structure and the round trip, not the tokenizer. Acceptance: the header names the tokenizer freeze and the alphabet holds a second whitespace byte, or `grep -n 'RefCur\|RefIdKind' src/codec/tests.rs` is empty.

### codec-base-text-tree-28: Dead `continue` at the end of the exhaustive odometer's outer loop
- Where: crates/before/src/codec/tests.rs:1794-1797 (related: crates/before/src/codec/tests.rs:1764-1798)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (read 1769-1797: the `if` is the last statement of the `for len` body, and the `len == 0` case already terminates the inner loop at 1790-1792, since `all` over an empty `idx` is true); executed: no
- Seen by: structure, prose, correctness, claims; refutation: confirmed; history: no rationale found (present verbatim as the loop's last statement in the commit that introduced the test)
- Owner-gated: no

A `continue` as the last statement of a loop body is a no-op; a reader stops to work out what it guards, and the answer is nothing.

Evidence:

      1790	            if idx.iter().all(|&j| j == 0) {
      1791	                break;
      1792	            }
      1793	        }
      1794	        if len == 0 {
      1795	            continue;
      1796	        }
      1797	    }

Resolution: Delete lines 1794-1796. Acceptance: `id_text_parser_matches_reference_exhaustively` green; clippy's `needless_continue` quiet on the file.

## Positives

- `MsbWindows<I>` and `msb_cmp_windows` (base.rs:148-241): a streamed MSB-aligned comparison with one register of carry, no shifted copy, and cost exactly O(shared-prefix limbs) because the loop stops at the first differing window. The generic over the reversed limb source pays: the stored magnitude (via `suanpan::Limbs`) and the rank's wide limb vector stream through one implementation. I walked the limb-aligned, partial-top-limb, single-limb, and zero cases against the `next` arms and the doc holds, including "a zero value has no windows".
- `limb_metered.rs` keeps every operator body in base.rs to the shape "meter, then delegate" with no `cfg` at the call sites (the two exceptions are finding 2), and `limb_meter.rs:1-27` states precisely what this currency sees that the heap and scan meters cannot and why densified fill is a separate column.
- The Shl/Shr totality paragraph (base.rs:466-478) is a model of the one-line-proof rule: it names the exact values the `expect` can fire on, shows the backend's own capacity assert bounds the same set, and explains why the right shift clamps rather than fails; 32-bit totality is handled by clamping, not truncation (`bit` at 46-50, `Shr<u64>` at 491-500).
- `tree.rs:29-34` states the id grammar exactly where the parser lives (2-bit presence tags, `0` as structural absence, no empty production) and derives the Truncated-on-empty rule from it; the `summary` comment at 66-67 says what the code cannot show and nothing else; `IdFrame`'s doc (5-10) prices the frame stack against the tag reads in one sentence.
- `display.rs::write_id` documents the one place `IdNode::Empty` can fire (39-40, which checks out against idbits.rs:38-39 and 82-88), keeps its control state to one to two bits per open node with both phases named at their constants, and its single `expect` (62) is a one-line proof true by the push order at 45-46.
- `text.rs:192-193` is a one-line overflow proof at the declaration (finding 19 argues the counter should go, not that the proof is wrong); `parse_base` slices the whole digit run and delegates radix conversion to the backend, with the metering convention stated at `Base::parse_decimal` and cross-referenced from the gamma decoder.
- Every `expect` in the partition is a one-line proof (base.rs:91, text.rs:65, display.rs:62), and no assert, expect, or debug_assert message in the partition contains an em-dash (mechanically checked).
- The codec test suite: the build-history family (tests.rs:212-384) is a lockstep model-based property test of the buffer's representation invariant at every intermediate state; the mutation family (1225-1469) states the accept-canonically disjunction once and applies it to single-bit flips, byte truncations, and every padding position across all three doors; the padding and flush-cut families (914-1594) give every marker-padded door a per-door witness for both rejection classes, including the interior seams, and explain why round-trip checks alone would be blind to intra-byte padding defects; `base_eq_hash_agree_with_derived_semantics` (714-779) pins the manual metered `PartialEq`/`Hash` against a `#[derive]`d mirror, a clean way to state "metering never changes an answer".
- The id text parser pin (tests.rs:1634-1922) is complete along every axis that matters: exhaustive to length 7 over the grammar alphabet, a recursive reference whose error precedence mirrors production, rendered ids with pseudo-random ASCII whitespace injection round-tripping to `party.as_bits()`, single-character insert/delete/replace mutations, point pins for precedence and the bare `0`, and a 100k-deep render-and-parse round trip.
- `base/tests.rs` drives both dispatch arms against an exact `IBig` oracle with the sign checked after every step, and the pattern of pairing a zero-assertion with a liveness leg (79-84) is the right one, even though finding 6 shows the particular counter cannot see the operation under test.

## Open questions for Finch

1. Does `validate_id` stay in `parse_id_str` and `id_node` as a deliberate totality defense (every door's bits pass the wire validator before storage), or go? Recommendation: delete both calls and move the canonicity assertion into the test helpers (finding 16); if kept, the rationale belongs at the call sites and the "single source of truth" sentence goes either way.
2. Which `Parse` precedence is the notation's: the id door's per-node rule (`NotCanonical` at the node's `)` even with trailing junk) or the version door's whole-pass rule (any syntax defect, trailing junk included, outranks `NotCanonical`)? Recommendation: the whole-pass rule, documented on `Parse`; it is the simpler statement ("only well-formed strings get canonicality judgments") and the reference parser needs only a `canonical` flag to mirror it (finding 18).
3. Is the comment half of the em-dash rule live for this repo? The repo has applied it to messages; 374 em-dash `//` comments exist in before/src (23 in this partition) and were still being added on 2026-08-18. Recommendation: rule once, then sweep crate-wide with the grep in finding 24 rather than partition by partition.
4. Under the `meter` feature, `before::meter::skyline::query::min_ticks` returns `Base` while `Base` has no public path. Does the meter surface count as stable API for the purpose of trait impls on `Base` (finding 8's gate), and does the surface-totality check see `Base` as an unnameable type in a public signature? For the surface-roster reviewer as much as for the owner. Recommendation: treat the meter surface as an instrument surface, not the stable API, and record that ruling where the surface check reads it.
5. For the literal-door claim (finding 13): restate the public rustdoc as `O(n · d)`, or reshape the sealed `PartyLiteral` to emit top-down into one buffer and keep `O(n)`? Recommendation: the reshape; the trait is sealed and hidden, so it changes no reachable surface.
6. After removing `Sub`'s `debug_assert!` (finding 7), which limb envelopes in `tests/meter.rs` move, and by how much? This needs a measurement run at the parent commit, which this review did not perform; the re-pin should attribute the movement to the assert's removal.
7. Should the `BitStack` migration (finding 12) cover all ten `BitsBuf`-as-stack sites in one change so `BitsBuf::pop` can be deleted in the same commit? Recommendation: yes; a partial migration leaves the two disciplines coexisting, which is the finding.

## Dropped

- Five `From` impls for `Base` could be one generic impl (candidate 14): below the bar; a blanket `impl<T> From<T> for Base where UBig: From<T>` would also admit `From<bool>` and `From<usize>` (dashu-int 0.5.0 `convert.rs:717-719` implements both for `UBig`), widening the surface on a type whose contract is event counts, and the owner may prefer the explicit width list.
- `parse_base` defines the grammar by contrast with the deleted digit-at-a-time loop (candidate 19, ghost-reference half): refuted by history; the digit-by-digit strategy survives as the board's `schoolbook_limb_ops(text, 1)` probe (meter/board/tests.rs:221), so the comparison has a live referent. The duplicated-paragraph half survives as finding 14.
- `AddAssign<u32>` and `From<u32>` have no production caller (candidate 34, second half): refuted by grep; `Base::from(0u32)` at meter/board/defect.rs:102 and meter/board/tests.rs:266, 620, 627 and `acc += 1u32` at meter/board/tests.rs:623 are instrument-side callers.
- `Shl<i32> for Base` has no caller (candidate 46): reframed into finding 8; the impl is what lets the two unsuffixed `<< 64` literals type-check, so it is not dead, it is load-bearing for a test spelling.
- Candidates 15, 16, 35, 48, 51 (recursion prose; `parse_id_from`): duplicates of finding 23. Candidate 17 (`u64` spill): merged into finding 23.
- Candidates 20, 32, 43 (`validate_id`): duplicates of finding 16. Candidates 22, 33, 50 (`Sub` assert): duplicates of finding 7. Candidate 49 (`SubAssign` clone): merged into finding 7; its `from_utf8` half is finding 15.
- Candidate 38 (`BitsBuf` phase stack): duplicate of finding 12. Candidate 44 (single-pass stamp split): duplicate of finding 19. Candidate 45 (Unicode trim): duplicate of finding 20. Candidate 39 (`sep` and `SEP`): duplicate of finding 11. Candidate 40 (limb formula): duplicate of finding 2. Candidate 47 (1.49): duplicate of finding 3. Candidates 26, 37, 52 (dead `continue`, hasher, count): duplicates of findings 28, 22, 26. Candidate 53 ("honest", ragged wrap): merged into findings 25 and 1.
- Candidate 25's "Iterative." and repetition items: merged into finding 17 with candidate 13.
- The `Rank` Ord site's "historical path" wording (num.rs:281) and the `tests.rs:1154` paper citation (§3 versus §5.3.4): the first belongs to the rank partition and dissolves with finding 4; the second is loose but defensible and below the bar.
