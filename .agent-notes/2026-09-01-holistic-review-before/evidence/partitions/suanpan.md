# Partition suanpan: suanpan: the cliff-free signed accumulator, limbs, magnitude, touch meter, and the claims roster

Reviewed at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean). Nothing was built, run, or modified; every "verified" below means read with line numbers, grepped, hand-traced, or checked with read-only git.

## Partition summary

suanpan is a small crate with one load-bearing type. `Accumulator` (accumulator.rs, 1515 lines) holds a running signed integer as redundant balanced base-2^32 digits kept in the lazy zone `|d| < 2^33`, in front of which an exact `i128` quick register absorbs word-scale work until the first wide operand, wide shift, or outgrown sum spills it, once per reset epoch. Three arguments carry every cost the crate page quotes: recentering keeps carries amortized O(1) per word-scale write; the sign fold collapses the cancelling prefix it scans so a digit is read at most once per write that made it nonzero; and a zero-run ledger of `(lo, hi)` certificates lets top settlement skip never-written gaps in one touch. `Limbs` (limbs.rs) denominates wide operands in 64-bit limbs whatever the backend word width; `Magnitude` (magnitude.rs) is the width-dispatch seam; `touch_meter` (touch_meter.rs) is the feature-gated counter every cost row is denominated in; `claims.rs` and `claims/tests.rs` are a roster that binds the crate-page table, the public surface, and the committed witnesses to each other.

The kernel is reviewably correct on 64-bit targets. I traced `add_at`'s recentering (carry `(t + 2^31) >> 32`, remainder in `[-2^31, 2^31)`, `|carry| >= 2` whenever `|total| >= 2^33` so the chain always ends in the in-zone arm), `read_digits`' final-carry closure over `[-3, 2]`, the `SIGN_DECIDED = 3` margin against the `2.01` geometric tail, the register arm's checked chain in `sign_dominates_at`, and the ledger's three maintainers through collapse; every clause of the `zero_runs` field doc holds as written. The instruments are stronger than before's envelope convention: the metered pins assert exact totals across a doubling of the axis each row claims independence from, so the liveness floor and the flatness witness are one assertion, and the known-bad collapse-less fold is committed and shown red. The crate page is the best public prose in the two crates: every bound sits beside its derivation, hazards are stated where a user meets them, and coined terms are anchored to identifiers.

Four findings carry weight. One correctness defect: the digit-position arithmetic in `apply_limbs`, `fold_accum`, and `deposit_value` is unchecked `usize` addition after a checked `shift / 32`, and because zero contributions are skipped before any buffer growth, a shift near `usize::MAX` digits on a 32-bit release build deposits the operand at digit 0 with no panic and no allocation failure, against the `# Panics` contract (suanpan-24; 32-bit only, one helper fixes it). Two verification disputes: the `add_at` exit `debug_assert!` scans the whole buffer above `top` on every digit write in every debug build, the exact shape the owner's own assertion audit retired three days before it was added (suanpan-21); and the two suanpan entries in `.cargo/mutants.toml` are excluded as value-equivalent or work-only, but both mutants change exact touch counts, which the crate declares a public contract, and one of them dissolves by the header's own refactor step (suanpan-40). One instrument gap: the headline "on every input sequence" quantifier is pinned only at canonical schedules, never on the random streams the differential suite already drives (suanpan-2). The rest are small repeated forms in `accumulator.rs` that one private home each would collapse, roster data that describes the pre-register code, and prose hygiene under Principle 5.

Lines read: 2,939 across the eight partition files (Cargo.toml 28, lib.rs 365, accumulator.rs 1515, limbs.rs 72, magnitude.rs 54, touch_meter.rs 48, claims.rs 463, claims/tests.rs 394), plus the cited ranges of accumulator/tests/{metered,ledger,witnesses,differential}.rs, tests/amortized_sequences.rs, .cargo/mutants.toml, tools/mutantcheck-expected.json, the root Cargo.toml, and the before-side callers. Test files in the partition: claims/tests.rs; `claims.rs` itself is `#[cfg(test)]` roster data. magnitude.rs and touch_meter.rs are clean under every lens.

## Findings

### suanpan-1: Edition 2021 under a 2024 workspace root
- Where: crates/suanpan/Cargo.toml:4-4 (related: Cargo.toml:86, crates/before/Cargo.toml:4, crates/surface-scan/Cargo.toml:4)
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified (grep of `edition` across workspace manifests); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (root moved to 2024 in 6b70ee9a; before has carried 2021 since eecf9229; suanpan inherited it at extraction)
- Owner-gated: yes (a workspace-wide toolchain decision, not a suanpan choice)

suanpan, before, before-viz, before-fuelscape, and surface-scan declare edition 2021 while the root package and rumors-tracing declare 2024. Mixed editions cost a reader the question of which lint and capture rules apply per crate, for no stated benefit.

Evidence:

         4	edition = "2021"

Resolution: workspace-level: migrate the 2021 crates together (`cargo fix --edition`), or add `edition.workspace = true` under `[workspace.package]`, or state at the root why they differ. Acceptance: every member declares one edition, or the root states the reason.

### suanpan-2: The "on every input sequence" quantifier has no instrument over arbitrary sequences
- Where: crates/suanpan/src/lib.rs:25-29 (related: crates/suanpan/src/accumulator/tests/metered.rs:1-11; crates/suanpan/src/accumulator/tests/differential.rs:340-359; crates/suanpan/tests/amortized_sequences.rs:63-78; crates/before/tests/meter.rs:6622-6627)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: verified (grep: `touch_meter` is referenced by no file under accumulator/tests except metered.rs; differential.rs:340-359 asserts sign and value only); executed: no
- Seen by: claims; refutation: confirmed at medium, with the caveat that the known-bad mechanism the property would catch is asserted, not constructed; history: deliberate-and-holds (the derived-bounds-plus-exact-pins model is stated at metered.rs:1-11 and lib.rs:25-29, 312-323; no randomized touch bound was ever proposed or rejected)
- Owner-gated: yes (a documented instrument model)

The crate quantifies every amortized bound over all input sequences. The committed touch instruments are exact totals at canonical schedules (metered.rs), two-scale flatness at four fixed shapes in before's meter suite, and one 2x2 mixed-second-difference grid; the proptests that drive arbitrary mixed streams (`mixed_streams_match_the_bigint_oracle`, the run-forming stream, the ledger stream) never read the meter. A regression that kept the pinned shapes flat but broke amortization on some other interleaving would pass every committed check. The doctrine prefers a property over point pins when the claim is a family, and the potential argument (lib.rs:150-161) yields a concrete per-operation constant, so a sequence-level bound is derivable rather than measured.

Evidence:

        25	//! Every cost this page quotes holds on adversarial input sequences — the
        26	//! amortized bounds are worst-case over the whole sequence, not average-case
        27	//! claims — and every one is *derived*: the three arguments that carry them
        28	//! (the lazy zone, the collapsing sign fold, the zero-run ledger) are below, in
        29	//! full.

Resolution: a touch-meter proptest (metered.rs or tests/amortized_sequences.rs) that drives the existing `arb_op()` streams with interleaved sign reads, records `touches` beside a work denominator (word-scale calls + limbs yielded + sign reads + one spill), and asserts `touches <= K * work + D` with K derived in the test's doc comment from the potential argument (each `add_at` iteration deposits one credit; fold reads and settle steps each spend one; carry steps are bounded by the zone refill). Commit a known-bad demonstration beside it (for example `settle_top` with the `consume_run_at` arm disabled) and show the property reads red on run-forming streams. Acceptance: the property runs in the gate under `--all-features`; the known-bad demonstration fails it while the fixed-schedule pins are shown not to notice. Construction: `proptest! { fn touches_are_linear_in_work(ops in vec(arb_op(), 1..300), engine_first: bool) { touch_meter::reset(); let mut acc = fresh(engine_first); let mut work = 0u64; for op in &ops { apply(&mut acc, &mut oracle, op); work += op.limbs_or_one(); acc.sign(); work += 1; } prop_assert!(touch_meter::touches() <= K * work + D); } }` with K and D derived in the doc comment; the metered suite must run serially (touch_meter.rs:16-20).

### suanpan-3: Opaque `\[derived\]` tags in the crate page
- Where: crates/suanpan/src/lib.rs:72-72 (related: crates/suanpan/src/lib.rs:150, 27)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep: two sites, no definition anywhere in the crate); executed: no
- Seen by: prose; refutation: confirmed; history: no rationale found (entered inline with 1f13983b; never read by any machinery, roster or drift checker)
- Owner-gated: no

Two bracketed markers sit inside sentences of the crate page with no legend; the sign-fold section (101-126) carries the same kind of derivation untagged, and line 27 already says every cost is derived. They render as literal `[derived]` in rustdoc and the cargo-rdme README. Vocabulary rule: an opaque roster-style tag in public prose competes with the contract.

Evidence:

        72	//! facts make this cheap \[derived\]: a freshly recentered digit must absorb at
       150	//! The amortization is a potential argument over the ledger \[derived\]: at

Resolution: delete both; if the intent is to mark which paragraphs the table's "derived above" (line 200) points at, say it in words. Acceptance: `grep -c 'derived\\]' crates/suanpan/src/lib.rs` is 0; `just readme` re-derived.

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

### suanpan-5: `reserve_digits` is absent from the crate page
- Where: crates/suanpan/src/lib.rs:225-233 (related: crates/suanpan/src/accumulator.rs:79-80, 681-702; crates/suanpan/src/claims.rs:322-326)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -c reserve_digits crates/suanpan/src/lib.rs` is 0); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-and-holds for the table omission (the table is denominated in digit touches, lib.rs:200, and 8da57920 landed the method with `table_cost: None`); no rationale for the memory paragraph's silence
- Owner-gated: no

The type doc sends readers to the crate page as the overview, and the memory paragraph describes exactly the buffer growth `reserve_digits` shapes ("A shifted entry point grows the digit buffer to cover the shifted position") without naming the hint. Documentation altitude: the crate page is where a user learns an operation exists.

Evidence:

       225	//! Digit touches are shift-independent; memory is not. A shifted entry point
       226	//! grows the digit buffer to cover the shifted position, so memory is O(shift /
       227	//! 32) plus the operand's own digits (the zero-run ledger adds at most one
       228	//! entry per write that lands above the held top, bounded by half the held
       229	//! digit positions). The *written span* is every digit from the lowest position

Resolution: one sentence in this paragraph: "A caller that knows the scale its writes will reach can pre-size the buffer once with [`reserve_digits`](Accumulator::reserve_digits)." The table stays as is (no digit-touch axis). Acceptance: `grep reserve_digits lib.rs` finds it; `cost_table_rows_bind_to_the_roster` untouched; README re-derived.

### suanpan-6: The register-metering sentence says every absorbed delta or shift counts one touch; zero deltas and `shl(0)` count none
- Where: crates/suanpan/src/lib.rs:277-279 (related: crates/suanpan/src/touch_meter.rs:9-12; crates/suanpan/src/accumulator.rs:183-190, 418-424, 536-541, 601-611)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read: `add_small` returns at 184 before `quick_add`'s touch at 1257; `add_magnitude_shl` matches `Some(0) => {}` at 420; `shl` returns at 607 before the touch at 611; `fold_accum` touches once at 538 even for a zero register operand); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (the `!= 0` guards predate the register; the "exactly one touch" sentences came with 1c1cb67d and 1255a4e2)
- Owner-gated: no

The exactness contract (283-286) makes this sentence a statement of record, and it is not exact: `add_small(0)`, `add_magnitude_shl(&0, s)`, `shl(0)`, and `shl` of a literal zero record no touch, while folding a register-held zero operand records one.

Evidence:

       277	//! quick register holds no digits, yet its work is metered too: a delta, sign
       278	//! query, negation, or shift the register absorbs counts exactly one touch, a
       279	//! register read-out ([`sign_magnitude`](Accumulator::sign_magnitude) and its

Resolution: amend here and at touch_meter.rs:9-12: "a nonzero delta, a sign query, a negation, or a nonzero shift of a nonzero value the register absorbs counts exactly one touch; a zero delta or a zero shift counts none, and an accumulator operand's register read counts one even at zero"; or meter the zero cases uniformly and pin them. Acceptance: a metered pin of `add_small(0)`, `shl(0)`, and `add_accum(&zero_register)` matching the sentence.

### suanpan-7: Hand-maintained restatements of enumerable facts, two already stale
- Where: crates/suanpan/src/lib.rs:296-307 (related: crates/suanpan/src/accumulator.rs:72-73, 1027; Cargo.toml:50)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (git blame: accumulator.rs:73 dates to 456c5e9f, when there were two arguments; lib.rs:27 moved to "three" in bf95fdfe; 8da57920 added `sign_limbs` and extended the table row (222) and seam paragraph (239, 247) but not the `&self` list; root Cargo.toml:50 pins dashu-int 0.5); executed: no
- Seen by: prose (17), claims (44, 52); refutation: confirmed all three legs; history: deliberate-but-expired for "both" and the `&self` list; deliberate-and-holds for the version sentence's purpose (7ab518ce), with the number as residue
- Owner-gated: no

Three prose restatements of facts the code owns, Principle 5 (state the structure, not the tally). (1) accumulator.rs:73 says the crate page carries "both cost arguments"; lib.rs:27 names three. (2) The Interop list of `&self` reads omits `sign_limbs` (`pub fn sign_limbs(&self)`, accumulator.rs:1027), the readout the same page motivates for the 32-bit case, so a user behind a shared reference is told it is unavailable. (3) The dashu-int version is restated by hand.

Evidence:

       296	//! [`UBig`] is `dashu_int::UBig` (compiled against `dashu-int` 0.5; bumping
       ...
       301	//! every amortized-O(1) sign query takes `&mut self`, so the value reads
       302	//! available behind a shared reference are
       303	//! [`is_literally_zero`](Accumulator::is_literally_zero),
       304	//! [`digit_count`](Accumulator::digit_count), the O(held digits)
       305	//! [`sign_magnitude`](Accumulator::sign_magnitude) (and its scaled twin
       306	//! [`sign_magnitude_shl`](Accumulator::sign_magnitude_shl)), and a
       307	//! [`clone`](Clone::clone) — wrap in a lock for shared sign reads. It is

    accumulator.rs:
        72	/// held value to a normalized magnitude. The crate docs carry the
        73	/// representation and both cost arguments. Sign queries take `&mut self`

Resolution: (1) "the representation and the cost arguments"; (2) add `sign_limbs` beside `sign_magnitude`, or restate structurally ("every `&self` method: the two O(1) probes, the three O(held digits) readouts, and clone"); (3) drop the number ("the workspace's pinned dashu-int; bumping it is a breaking change") or pin it mechanically. `just readme` afterwards. Acceptance: no numeral or hand list in these sentences that the code can change without touching the prose.

### suanpan-8: The documented trait surface (`Send + Sync`, deliberately not `PartialEq`) has no compile-time pin
- Where: crates/suanpan/src/lib.rs:299-309 (related: crates/suanpan/Cargo.toml:16-20; Cargo.toml:72, 128; crates/before/src/party.rs:74)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep: `static_assertions` is a workspace dependency used in before; suanpan's dev-dependencies are proptest and surface-scan only; no `assert_impl`/`assert_not_impl` under crates/suanpan); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (the sentence came with 5d492875; before has the precedent)
- Owner-gated: no

A field-type change (a non-`Sync` handle) or a well-meant `#[derive(PartialEq)]` would falsify the crate page with nothing failing. For each contract clause a committed check should fail if it were wrong.

Evidence:

       299	//! requires `std`; no `no_std` build is offered. [`Accumulator`] is `Clone`,
       300	//! `Default`, `Debug`, and `Send + Sync` — though `Sync` buys less than usual:
       ...
       308	//! deliberately not `PartialEq`: two spellings of one value would compare
       309	//! unequal, so compare by subtracting and reading the difference's sign.

Resolution: `static_assertions` as a dev-dependency; in accumulator/tests.rs, `assert_impl_all!(Accumulator: Send, Sync, Clone, Default, Debug); assert_not_impl_any!(Accumulator: PartialEq);`. Acceptance: adding `#[derive(PartialEq)]` at accumulator.rs:89 fails to compile the test target. Construction: add that derive today; nothing fails.

### suanpan-9: The register-or-digit-0 dispatch is spelled out at five sites
- Where: crates/suanpan/src/accumulator.rs:183-190 (related: 201-208, 219-226, 237-244, 547-550; the enter-then-deposit pairs at 617-618, 621-622, 1262-1263, 1273-1274)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep `self.add_at(0, `: 187, 205, 223, 241, 549, each behind the same `if !self.quick_add(x)` guard); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (1c1cb67d wrote the bodies inline; d07fc20e added `#[inline]` for cross-crate inlining, which a helper must carry too)
- Owner-gated: no

`add_small`, `sub_small`, `add_u64`, `sub_u64`, and `fold_accum`'s register arm each spell "register when held, else digit 0"; the four public twins differ only in sign and width and the reader diffs seven lines per entry to confirm it. A named home states the dispatch once and moves no `touch` call.

Evidence:

       183	    pub fn add_small(&mut self, delta: i64) {
       184	        if delta != 0 {
       185	            let delta = i128::from(delta);
       186	            if !self.quick_add(delta) {
       187	                self.add_at(0, delta);
       188	            }
       189	        }
       190	    }

Resolution: `#[inline] fn add_word_scale(&mut self, delta: i128) { if !self.quick_add(delta) { self.add_at(0, delta); } }` called from the four entries and `fold_accum`; optionally `fn spill_value(&mut self, value: i128, shift: u64)` for the four enter-then-deposit pairs (`spill` generalized by a shift). Acceptance: one `self.add_at(0,` site; the metered pins unchanged.

### suanpan-10: A zero wide operand spills the register and can flip a domination certificate
- Where: crates/suanpan/src/accumulator.rs:253-256 (related: 264-267, 324-327, 345-348, 374-377, 396-399, 557, 777-800; crates/suanpan/src/accumulator/tests/witnesses.rs:274-294; crates/suanpan/src/lib.rs:50-53)
- Class / severity / confidence: api-surprise / low / high
- Provenance: assessed (read and trace: register-held `2^80` certifies at floor 1 since `2^80 >= 3 * 2^64` (812-817); after `add_wide(&UBig::ZERO)` the spill deposits `2^16` at digit 2 and the fold decides at index 2 < floor + 2 = 3); executed: no
- Seen by: correctness; refutation: confirmed; history: deliberate-and-holds (spill by operand class, not value, is stated at lib.rs:50-53 and the `quick` field doc 95-98)
- Owner-gated: yes (a stated design rule; a lazy spill also changes touch counts, which lib.rs:283-286 makes a breaking change)

`add_wide`, `add_wide_shl`, `add_limbs_shl` (and the sub twins), and `add_accum` of a literally-zero digit-engine operand call `spill()` before discovering there is nothing to deposit, retiring the register for the epoch; `add_magnitude(&ZERO)` (which routes to `add_u64(0)`) does not. Within the documented contract (`decided` is representation-dependent), but a value-neutral call changing a verdict and costing deposit touches is a surprise the cheaper ordering avoids.

Evidence:

       253	    pub fn add_wide(&mut self, delta: &UBig) {
       254	        self.spill();
       255	        self.apply_limbs(Limbs::new(delta), false, 0);
       256	    }

Resolution: owner decision: spill lazily (move `self.spill()` into `apply_limbs` ahead of the first `add_at`, and skip it in `fold_accum`'s digit path when `other.is_literally_zero()`) and re-derive the affected pins as a named touch-count change; or state at the wide entry points that a zero wide operand retires the register. Acceptance: a witness that `add_wide(&UBig::ZERO)` on a register value leaves `quick.is_some()` and records 0 touches, or the doc states the behavior. Construction (passes today): `let mut a = Accumulator::new(); a.add_u64(1 << 20); a.shl(30); a.shl(30); assert_eq!(a.sign_dominates_at(1), (Ordering::Greater, true)); a.add_wide(&UBig::ZERO); assert_eq!(a.sign_dominates_at(1), (Ordering::Greater, false));`.

### suanpan-11: `negative: bool` threaded through three private paths yields seven `if negative { -x } else { x }` sites
- Where: crates/suanpan/src/accumulator.rs:542-546 (related: 567-571, 1225, 1237, 1321-1325, 1480, 1486)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep `if negative`: 542, 567, 1225, 1237, 1321, 1480, 1486); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found
- Owner-gated: no

`fold_accum`, `add_shifted_word`, `apply_limbs`, and `deposit_value` negate each contribution by a ternary at the point of use; the bool parameter's polarity is a lookup at every call site. One sign multiplier per function removes six of the seven branches and moves no `touch`.

Evidence:

       542	            let operand_value = if negative {
       543	                -operand_value
       544	            } else {
       545	                operand_value
       546	            };

Resolution: `let sign: i128 = if negative { -1 } else { 1 };` once per function and multiply contributions, or a two-variant private enum whose method returns the multiplier. Acceptance: one sign application per function; touch counts unchanged.

### suanpan-12: The shift split and its `expect` are duplicated at four sites; one helper would carry the 32-bit argument and the checked landing position
- Where: crates/suanpan/src/accumulator.rs:558-560 (related: 1232-1234, 1309-1311, 1468-1470, 317-323)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep `digit positions fit a usize`: 560, 1234, 1311, 1470, each behind the same `(shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS))` split); executed: no
- Seen by: structure, prose (as an open question); refutation: confirmed; history: no rationale found (introduced once in 092b149f and replicated per path; 9f68c475 counted "the four shift-position expects" without consolidating)
- Owner-gated: no

Eight public `# Panics` sections describe one hazard; the proof for the `expect` (only a target narrower than 64 bits can fail it, from `shift = 2^37`) lives in `add_wide_shl`'s rustdoc (319-323) and at none of the four `expect`s. Every `expect` message is a one-line proof; one helper puts the proof beside the panic, and it is where suanpan-24's checked landing position belongs.

Evidence:

       558	        let (digit_shift, bit_shift) =
       559	            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
       560	        let digit_shift = usize::try_from(digit_shift).expect("digit positions fit a usize");

Resolution: `fn split_shift(shift: u64) -> (usize, u32)` whose doc carries the 32-bit-only argument and the single `expect`, called at the four sites; `bit_shift` as `u32` is the idiomatic shift-amount type. Acceptance: one `digit positions fit a usize` string remains; the `# Panics` sections need no contract change.

### suanpan-13: Em-dashes in `//` comments, a TOML comment, and panic/error strings
- Where: crates/suanpan/src/accumulator.rs:603-603 (related: 879, 1078, 1357; crates/suanpan/Cargo.toml:25-26; crates/suanpan/src/claims/tests.rs:291, 364, 371; crates/suanpan/tests/amortized_sequences.rs:58; crates/suanpan/src/accumulator/tests/differential.rs:243; crates/suanpan/src/accumulator/tests/witnesses.rs:134, 184, 641)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -nE '^\s*//[^/!].*—'` over crates/suanpan/src and tests; string scan of claims/tests.rs and amortized_sequences.rs; `grep '—' Cargo.toml`); executed: no
- Seen by: prose; refutation: confirmed and added the test-file sites; history: no rationale; the applicable rule is the owner's doctrine (colons or semicolons over em-dashes in comments and messages), not a root or before AGENTS.md hard rule, and no tools/ lint checks it
- Owner-gated: no

Rendered rustdoc keeps its dashes; `//` comments, the `#` comment, and message strings are the terminal-facing register.

Evidence:

       603	        // nothing, and returning here keeps both free — no rebuild of a

    claims/tests.rs:
       291	                            "{}: {file} no longer holds the #[test] fn `{witness}` — \

Resolution: a colon or semicolon at each site (e.g. 603 "keeps both free: no rebuild of a"; tests.rs:291 "no longer holds the #[test] fn `{witness}`; re-derive the claim"). Acceptance: the two greps above return nothing under crates/suanpan.

### suanpan-14: `shl` on a digit-engine value rebuilds into a fresh buffer grown one position at a time
- Where: crates/suanpan/src/accumulator.rs:626-627 (related: 557, 587-590, 681-702, 1283-1292, 1370-1372)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read: `mem::take` leaves `digits: Vec::new()`; the fold's spill reserves exactly one (1290); each new position `resize`s by one (1371), so the buffer reaches its width through Vec doubling with the transient overshoot `reserve_digits`' own doc names (684-694)); executed: no
- Seen by: claims; refutation: confirmed (fixed sign, small); history: no rationale found (the rebuild dates from ee894835; 22cb962d made "may release the buffer" the contract)
- Owner-gated: no for the pre-reserve (moves no touch); an in-place variant for `shift % 32 == 0` would change counts and is owner-gated

Evidence:

       626	        let held = core::mem::take(self);
       627	        self.add_accum_shl(&held, shift);

Resolution: before the fold, `self.reserve_digits(held.top + 1 + digit_shift + 2)` (the `+2` covers a carry out of the top digit; `digit_shift` via suanpan-12's helper). Acceptance: one allocation for `shl` on a 64-digit value at shift 32_000 (stats_alloc is a workspace dependency); `held_width_rows_cost_the_held_digits` still reads 2d.

### suanpan-15: `sign_dominates_at`'s public doc carries the margin proof at user altitude
- Where: crates/suanpan/src/accumulator.rs:786-794 (related: 45-50, 808-811; crates/suanpan/src/lib.rs:170-180)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: prose; refutation: confirmed as nit (a placement call with no false statement); history: deliberate-and-holds (22cb962d rewrote the paragraph to correct a refuted claim about `decided`; the inequality chain is that correction's proof)
- Owner-gated: no

The user-facing contract is four facts (representation-dependence; the register certifies at `|v| >= 3 * 2^(32(floor+1))`; the fold certifies at decision index `>= floor + 2`; a spill can flip `decided` to false), all of which the paragraph states and should keep. The inequality chain proving the margins is maintainer material and duplicates the crate page's domination bound.

Evidence:

       786	    /// running partial reaches `|s| ≥ 3` at digit index `floor + 2` or
       787	    /// higher: at decision index `i` the unscanned digits below
       788	    /// contribute under `2.01 · 2^(32·i)` (the crate docs' domination
       789	    /// bound), so `|value| ≥ 0.99 · 2^(32·i)`; an operand with top digit
       790	    /// index at most `floor` holds under `2.01 · 2^(32·(floor + 1))`
       791	    /// (the same geometric bound one level up), and
       792	    /// `0.99 · 2^(32·(floor + 2)) > 2.01 · 2^(32·(floor + 1))` by a
       793	    /// factor over `2^30` — so folding any such operand in could flip
       794	    /// neither the sign nor which magnitude is larger. The register's

Resolution: move 786-794's inequality chain to the `SIGN_DECIDED` doc (45-50) or a `//` comment above 826; the public text says the fold's decision index clears the operand bound with the same margin the register test uses (see the crate page). Acceptance: the public doc states which values certify in each tier and the spill hazard, with the inequality living once beside the constant it justifies.

### suanpan-16: Literal `32` and `128` beside the named `DIGIT_BITS`
- Where: crates/suanpan/src/accumulator.rs:812-817 (related: 945, 1008, 1040, 29)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep over non-doc lines: 814, 945, 1008, 1040; `DIGIT_BITS` defined at 29 and used everywhere else); executed: no
- Seen by: structure (5), correctness (37); refutation: confirmed both; history: no rationale found (`DIGIT_BITS` is `u32`; these sites need `usize`/`u64`, a one-token conversion)
- Owner-gated: no

Named constants over magic numbers. The 812-817 site also carries an unstated totality premise: `checked_shl` reports only a shift `>= 128`, not lost value bits, so `3u128.checked_shl(bits)` is exact only because `bits` is always a multiple of the digit width and can never be 127; naming the constant is where that premise belongs.

Evidence:

       812	            let decided = floor
       813	                .checked_add(1)
       814	                .and_then(|digits| digits.checked_mul(32))
       815	                .and_then(|bits| u32::try_from(bits).ok())
       816	                .and_then(|bits| 3u128.checked_shl(bits))
       817	                .is_some_and(|bound| value.unsigned_abs() >= bound);
       945	            Some(value) => (128 - value.unsigned_abs().leading_zeros() as usize).div_ceil(32),

Resolution: `DIGIT_BITS` (with the conversions the sites already do) and `u128::BITS` at 814, 945, 1008, 1040; one clause at 812-817: "bits is a multiple of DIGIT_BITS, so the only shift that would drop a value bit (127) is unreachable and `checked_shl` returns `None` exactly at `bits >= 128`". Acceptance: no bare `32` or `128` denoting the digit width outside the constants' definitions.

### suanpan-17: A collapse whose re-deposit recenters back into the digit it just zeroed is a fixed point: every sign read costs 6 touches
- Where: crates/suanpan/src/accumulator.rs:869-877 (related: 1342-1396; crates/suanpan/src/lib.rs:115-123; crates/suanpan/src/accumulator/tests/metered.rs:786-795)
- Class / severity / confidence: performance / low / high
- Provenance: verified (hand trace of `fold_and_collapse` and `add_at` on `add_wide(&(UBig::from(2u8) << 128usize))`: digit 4 = 2, run (0, 4). Each `sign()`: read digit 4 (=2), zero it, read digit 3 (=0) so partial `2^33` decides, zero the floor, `add_at(3, 2^33)` recenters with carry 2 and remainder 0 and rewrites digit 4 = 2: six touches, and the state after the first read (run cropped to (0, 3)) is a fixed point); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found
- Owner-gated: yes (touch counts are a public exact contract, lib.rs:283-286; any fix re-derives pins and is a named contract change)

When the top digit is +/-1 or +/-2 over a digit in `[-2^31, 2^31)`, the fold cannot decide at the top, descends one digit, decides, and re-deposits a partial that recenters into exactly the spelling it started from. Amortized O(1) holds (constant 6) and lib.rs:119-121 stays literally true (the re-deposited digit is the next fold's first step), but four of the six touches are churn that leave the representation unchanged, on a shape before's many production sign reads can sit on. metered.rs:786-795 observes the shape only as `> 1`.

Evidence:

       869	        if index < start_top {
       870	            // Collapse: the descent zeroed everything above; zero the
       871	            // floor digit too and re-deposit the exact partial there.
       872	            self.digits[index] = 0;
       873	            touch(1);
       874	            self.top = index;
       875	            if partial != 0 {
       876	                self.add_at(index, partial);
       877	            } else {

Resolution: owner decision. Either skip the collapse when it cannot make progress (`index == start_top - 1` and `partial.abs() >= LAZY_LIMIT`: the re-deposit would carry straight back), or write `(carry, remainder)` directly at `(start_top, index)` without the recenter pass; either re-derives the affected pins and names the touch-count change. Acceptance: a metered pin: after `acc.add_wide(&(UBig::from(2u8) << 128usize)); acc.sign();`, 1,000 further `sign()` reads cost 1,000 touches (6,000 today by trace).

### suanpan-18: Destructure-and-retuple in `sign_magnitude`; a redundant clamp between `sign_magnitude_shl` and `read_digits`
- Where: crates/suanpan/src/accumulator.rs:966-967 (related: 1006, 1080-1081, 1057-1061)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read: every caller of `read_digits` passes 0 (966, 1037) or the already-clamped start (1006-1007), so the clamp at 1080 is dead for all callers; it is what keeps `with_capacity(self.top - start + 2)` at 1081 from underflowing on a hypothetical unclamped caller. The caller-side clamp at 1006 is load-bearing: `bottom` is `usize::MAX` in digit mode after a spill of zero with no deposit yet, e.g. after `Accumulator::new().add_wide(&UBig::ZERO)`); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (ef8894ac introduced the retuple verbatim with `read_magnitude` already returning a pair)
- Owner-gated: no

Evidence:

       966	        let (sign, magnitude) = self.read_magnitude(0);
       967	        (sign, magnitude)
      1006	        let start = self.bottom.min(self.top);
      1080	        let start = start.min(self.top);

Resolution: `sign_magnitude`: `self.read_magnitude(0)`. Keep exactly one clamp: drop 1080 and add `start <= top` to `read_digits`' precondition beside its soundness condition (1057-1061), or drop 1006 and return the clamped start from `read_digits`. Acceptance: no retuple; one `min(self.top)` on the read path.

### suanpan-19: Vocabulary tells: moralized code, a significance adverb, a mechanism-less "silently", three spellings of one term, colon-fronted labels and antitheses
- Where: crates/suanpan/src/accumulator.rs:992-992 (related: crates/suanpan/src/claims.rs:5-6; crates/suanpan/src/claims/tests.rs:321; accumulator.rs:86, 652, 677 (pool-reuse spellings); accumulator.rs:79 and claims.rs:89, 398 ("derived" meaning `#[derive]`); colon labels at accumulator.rs:602, 821, 855, 870, 1074, 1091, 1356, 1462; antitheses at lib.rs:260, 287, accumulator.rs:927, 955, 981, 1022, Cargo.toml:25-26, claims/tests.rs:301, 386; the test files' "honest" at metered.rs:337, 804 and witnesses.rs:327, 394, 411)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rnE 'honest|genuinely|rot silently'` over crates/suanpan/src; grep of the three pool-reuse spellings; label and antithesis sites read directly); executed: no
- Seen by: prose (20, 22); refutation: confirmed these legs, refuted the "arm" leg (the arming family is before's watermark-web term of art, 245 uses in crates/before/src) and weakened the "certificate" leg (each referent is defined under its own crate-page heading, 128 and 170); history: no rationale (the pool-reuse spellings come from three uncoordinated commits)
- Owner-gated: no

The vocabulary rule flags moralized code ("honest"), significance adverbs ("genuinely"), "silently" without the mechanism, and the "X, not Y" and "Label: explanation" tics when they recur enough to read as texture. "derived" also carries two meanings across the crate: argued-from-first-principles on the crate page (27, 72, 150, 200) and `#[derive]`d in accumulator.rs:79 and the roster label.

Evidence:

       992	    /// `(magnitude, shift)` pair is therefore one honest spelling of the

    claims.rs:
         5	//! each stating its own bound inline; the roster binds the legs that
         6	//! rot silently without a name, and the binding tests

    accumulator.rs:
        86	/// next spill — the pooled-reuse contract — while
       652	    /// The pool-reuse entry point: a caller that opens and closes many
       677	        // next spill, so pooled reuse keeps its capacity.

Resolution: drop "honest" at 992 ("one spelling of the value, not a normal form" already says it) and "genuinely" at tests.rs:321; at claims.rs:5-6 name what rots (the table cell, the witness name, the surface row); one spelling of "pool reuse"; write `#[derive]`d where the derive macro is meant; thin the colon labels and antitheses where the second half restates the code (927 and 981 carry a contrast the reader needs and stay). Acceptance: `grep -rnE 'honest|genuinely'` over crates/suanpan/src returns nothing; one pool-reuse spelling.

### suanpan-20: `add_u64_shl` / `sub_u64_shl` have never had a caller outside suanpan's own tests
- Where: crates/suanpan/src/accumulator.rs:1193-1196 (related: 1212-1215; crates/suanpan/src/claims.rs:178-193; crates/suanpan/src/lib.rs:213; crates/before/src/version/skyline/signed.rs:203-210)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep `\.add_u64_shl(` / `\.sub_u64_shl(` across crates/, src/, tests/: hits only at crates/suanpan/src/accumulator/tests/metered.rs:122, 125, 126; `git log --all -S'.add_u64_shl(' -- crates/before src tests` is empty; e4c9b083's message says the entries are what "the Small arm folds through", but `fold_signed_int` at signed.rs:205-206 folds `Int::Small` through `add_u64`/`sub_u64`, and every shifted fold in before goes through `add_magnitude_shl`/`sub_magnitude_shl`); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found (the introducing commit's stated consumer never materialized; nothing since records a decision to keep them)
- Owner-gated: yes (stable public API)

Principle 3: the only things naming these two entries are the roster rows, the table row, two `# Panics` sections, and the metered witness leg that exists because the entries exist. `sub_accum_shl` likewise has no production caller (differential.rs:478, metered.rs:330, witnesses.rs:539 only) but is the symmetric twin of `add_accum_shl` (integral.rs:800, 1160), a defensible reason to keep it.

Evidence:

      1193	    #[inline]
      1194	    pub fn add_u64_shl(&mut self, word: u64, shift: u64) {
      1195	        self.add_shifted_word(word, false, shift);
      1196	    }

    signed.rs:
       205	        (Sign::Positive, Int::Small(n)) => acc.add_u64(*n),
       206	        (Sign::Negative, Int::Small(n)) => acc.sub_u64(*n),

Resolution: owner decision: retire both (their claims rows, the table row at lib.rs:213, and the second loop of `alternating_shifted_writes_cost_the_operand_not_the_gap`), or route before's shifted word-scale folds through them so the entry has the caller its introducing commit described. Acceptance: either no `add_u64_shl`/`sub_u64_shl` remain and `claims_are_total_over_the_public_surface` passes, or a production caller in before invokes them.

### suanpan-21: `add_at`'s exit `debug_assert!` scans the whole buffer above `top` on every digit write; the ledger suite already holds the clause
- Where: crates/suanpan/src/accumulator.rs:1391-1395 (related: 1365-1368; crates/suanpan/src/accumulator/tests/ledger.rs:51-60; Cargo.toml:180-187; .cargo/mutants.toml:33-35)
- Class / severity / confidence: scaffolding / medium / high
- Provenance: verified (read: `assert_ledger_invariants` asserts the identical two clauses at ledger.rs:51-60 after every step of every schedule and on the randomized streams; root Cargo.toml:180-187 keeps debug-assertions on for suanpan in the dev profile; `digits.len()` shrinks only through `shl`'s `mem::take` (626) and `reset` keeps the buffer (676-678)); executed: no
- Seen by: structure (0), claims (47); refutation: confirmed both; history: deliberate for its purpose but in direct tension with a recorded ruling: 9f68c475 (2026-08-07) retired `read_magnitude`'s O(skipped-prefix) debug_assert "per the recompute-and-compare ruling" and declared "The surviving debug_asserts are O(1) or once-per-epoch discipline documentation"; b93a3628 (2026-08-10) then added this O(buffer) exit scan knowingly, its message conceding "The exhaustive ledger sweep (which checks the same clause after every public step) passes unchanged". The `enter_digit_engine` scan (1293-1296) is once-per-epoch and stays within 9f68c475's ruling; that leg of the lens findings is dropped.
- Owner-gated: yes (reopens a recorded ruling; the new evidence is the earlier ruling it contradicts and the cost below)

Doctrine (Assertions and Guards): a runtime assert on a deterministic function is justified as an O(1) probe, not an O(n) re-encode, once committed coverage holds the invariant. The second conjunct walks `digits[top + 1..]` on every `add_at` in every debug build: a pooled accumulator that was once wide pays O(wide) per word-scale write for the rest of the process, in suanpan's and before's whole test suites, against an amortized-O(1) contract. Neither assert body meters, so the exactness contract is unaffected either way.

Evidence:

      1391	        debug_assert!(
      1392	            (self.top == 0 || self.digits[self.top] != 0)
      1393	                && self.digits[self.top + 1..].iter().all(|&digit| digit == 0),
      1394	            "top must rest on the highest nonzero digit at add_at exit"
      1395	        );

    ledger.rs:
        51	    assert!(
        52	        acc.digits[acc.top + 1..].iter().all(|&digit| digit == 0),
        53	        "nonzero digit above top {} after {schedule:?}",
        54	        acc.top
        55	    );
        56	    assert!(
        57	        acc.top == 0 || acc.digits[acc.top] != 0,
        58	        "top {} rests on a zero digit after {schedule:?}",
        59	        acc.top
        60	    );

Resolution: keep the O(1) conjunct (`self.top == 0 || self.digits[self.top] != 0`), delete the `digits[top + 1..]` scan, and re-state the comment at 1365-1368 to name `ledger_invariants_hold_exhaustively` (and the run-forming stream property) as the check that holds the exit invariant. Acceptance: the retiring-an-instrument discipline: run the suanpan mutants campaign under the configuration of record before and after and confirm no mutant moves from caught to survived (the dev profile's assertions are "part of the observer", mutants.toml:33-35, and the ledger suite's assertions remain in it); the ledger suite unchanged. Construction (debug build; corrected from the lens: `reset()` returns to the register at 678, so `add_small` after a reset never reaches `add_at` and a spill must precede the loop): `let mut acc = Accumulator::new(); acc.add_wide_shl(&UBig::ONE, 32 * 100_000); acc.reset(); acc.add_wide(&UBig::ONE); for _ in 0..100_000 { acc.add_small(1); }`: after the spill every `add_small` reaches `add_at` and the exit scan walks about 100,000 slack digits, roughly 10^10 element reads in debug; linear in release.

### suanpan-22: The zero-run ledger reads as a type but lives as a field plus three methods on `Accumulator`
- Where: crates/suanpan/src/accumulator.rs:1404-1440 (related: 124-154, 1353, 674, 882, 1299; crates/suanpan/src/accumulator/tests/ledger.rs:34, 68)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (grep: `zero_runs` is mutated at 674, 882, 1299 (`clear`), 1353, 1421, 1424 (`insert`), 1419, 1435 (`remove`); `crop_runs` and `consume_run_at` read no other field); executed: no
- Seen by: structure; refutation: confirmed (an adoptable-now design proposal); history: no rationale found (f7596470 designed it as a field; no note mentions a type)
- Owner-gated: no

Types-first and single responsibility: the certificate semantics, disjointness, and containment invariants (a 30-line field doc) and three methods that touch only the map sit on the accumulator with `&mut` access to digits they never touch, so a reader verifies by inspection what a newtype would make structural. Touch counts are unaffected: the ledger is bookkeeping outside the digit denomination (lib.rs:163-165).

Evidence:

      1432	    fn consume_run_at(&mut self, above: usize) -> Option<usize> {
      1433	        let (&lo, &hi) = self.zero_runs.range(..above).next_back()?;
      1434	        if hi >= above {
      1435	            self.zero_runs.remove(&lo);
      1436	            Some(lo)
      1437	        } else {
      1438	            None
      1439	        }
      1440	    }

Resolution: a private `struct ZeroRuns(BTreeMap<usize, usize>)` (same file or a sibling `accumulator/ledger.rs`) with `record(lo, hi)`, `crop(from, to)`, `consume_below(above)`, `clear()`, and `iter()` for the ledger suite; move the certificate semantics onto it, and let the `Accumulator` field doc state only how the accumulator maintains it. Acceptance: no `self.zero_runs.insert/remove/range` outside the newtype; `ledger_invariants_hold_exhaustively` asserts the same clauses.

### suanpan-23: Headroom comments state loose bit counts with an underived `33`
- Where: crates/suanpan/src/accumulator.rs:1473-1474 (related: 1235-1236, 1223)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (arithmetic: `limb & DIGIT_MASK` and `limb >> DIGIT_BITS` are 32-bit values, so a contribution after a sub-digit shift of at most 31 is 63 bits, stated as 33 + 31 = 64; a 64-bit word shifted by at most 31 is 95 bits, stated as 96; 1223's "94 bits" is exact); executed: no
- Seen by: prose; refutation: reframed (true over-bounds, cosmetic; the 33 has no derivation at the site); history: no rationale found (092b149f wrote "33 + 31" beside the `DIGIT_MASK` line from the start)
- Owner-gated: no

A headroom proof is useful only when its arithmetic is exact; the 33 looks like the lazy-zone bound leaking into a place where the operand is an unsigned 32-bit half-limb.

Evidence:

      1473	            // At most 33 + 31 bits per contribution after the sub-digit
      1474	            // shift: well inside the `i128` `add_at` carries from.
      1235	        // At most 96 bits after the sub-digit shift: well inside `i128`.

Resolution: "At most 32 + 31 bits" and "At most 64 + 31 bits". Acceptance: each bound equals the exact maximum width of the value it describes.

### suanpan-24: Digit-position arithmetic wraps on 32-bit targets before any allocation can fail: a silent wrong value in release
- Where: crates/suanpan/src/accumulator.rs:1477-1487 (related: 1470; 561-573 (`offset + digit_shift`); 1314-1330 (`position += 1`); 1371 (`resize(pos + 1, 0)`); 1423 (`to + 1`); 317-323; Cargo.toml:174-192; crates/suanpan/src/limbs.rs:14-19; crates/suanpan/src/lib.rs:240-243)
- Class / severity / confidence: correctness / high / high
- Provenance: verified (read and trace; grep: the root Cargo.toml declares `[profile.dev]`, two `dev.package` overrides, and `[profile.bench]` only, so release builds carry Rust's default `overflow-checks = false`; crates/before/wasm32-pins contains no suanpan reference, so no first-party 32-bit run of suanpan exists); executed: no
- Seen by: correctness (26), claims (39); refutation: confirmed both and added the boundary sibling at 1371; history: no rationale found (the unchecked sums date from 092b149f; the `# Panics` text (7ab518ce) and the assertion audit (9f68c475) considered only `shift / 32`)
- Owner-gated: no
- Witness (witness/results.md, `## 32-bit pass`): demonstrated (run in the `wasm32-pins` executor in two guest builds; the main pass was inconclusive on the 64-bit host). In the unchecked guest (release, rustc's default `overflow-checks = false`, the profile a consumer ships) construction (1) returns `Value(5)` with `sign == Greater`, `limbs.len() == 1`, `digit_count() == 1`, and construction (2) returns `Value(1)`: the predicted wrong values, with no trap. In the checked guest (the pins workspace's `[profile.release] overflow-checks = true`) both trap before any allocation, so the position arithmetic at accumulator.rs:1479 and :566 is what wraps. Construction (3) traps in both builds; its panic message and site are attributed by reading, since the guest has no stderr. The tree's own executor could never have observed this as a wrong value, because its guest profile turns wraps into traps by design; no committed pin reaches suanpan's shift entry points.

Only `shift / 32` is checked. The landing positions `2 * limb_index + digit_shift`, `2 * limb_index + 1 + digit_shift`, `offset + digit_shift` (566), and `position += 1` (1329) are plain `usize` additions after the guard, and zero contributions are skipped before any `resize` (1477, 1483, 563, 1317), so when a shift's low neighbours carry nothing the wrap happens before any allocation could fail loudly. On a 32-bit target in release the operand lands at digit 0 or 1 and the value reads back wrong with no panic. Principle 1 (never incorrect behavior; input likelihood carries no weight) and the `# Panics` clause, which promises a panic or an allocation failure. The crate names 32-bit targets as supported (limbs.rs:14-19, lib.rs:240-243). Both lens authors rated this medium (32-bit and release only); under the rubric (a bug that contradicts a stated contract) it is high, and the fix is one helper.

Evidence:

      1470	        let digit_shift = usize::try_from(digit_shift).expect("digit positions fit a usize");
      ...
      1477	            if low != 0 {
      1478	                self.add_at(
      1479	                    2 * limb_index + digit_shift,
      1480	                    if negative { -low } else { low },
      1481	                );
      1482	            }
      1483	            if high != 0 {
      1484	                self.add_at(
      1485	                    2 * limb_index + 1 + digit_shift,
      1486	                    if negative { -high } else { high },
      1487	                );

       319	    /// Panics if the shifted digit position `shift / 32` overflows
       320	    /// `usize` — possible only on targets narrower than 64 bits (from
       321	    /// `shift = 2^37` on a 32-bit one). On 64-bit targets every `u64`
       322	    /// shift fits, and an enormous one fails at allocation instead, like
       323	    /// any collection asked to grow to `shift / 32` entries.

Witness output (32-bit pass; two guest builds for `wasm32-unknown-unknown`, the second with `CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false`, each run through the wasmtime harness):

    ```text
    # run 2 (unchecked guest; run2-unchecked.log):
        Starting 3 tests across 4 binaries (59 tests skipped)
            PASS [   0.131s] (1/3) wasm32-pins-harness::zz_witness suanpan_add_limbs_shl_wrap_unchecked_guest
            PASS [   0.133s] (2/3) wasm32-pins-harness::zz_witness suanpan_shl_wrap_unchecked_guest
            PASS [   0.135s] (3/3) wasm32-pins-harness::zz_witness suanpan_add_limbs_shl_boundary_unchecked_guest
         Summary [   0.135s] 3 tests run: 3 passed, 59 skipped
    # run 1 (checked guest; run1-checked.log), the suanpan lines:
            PASS [   0.139s] ( 1/11) wasm32-pins-harness::zz_witness suanpan_add_limbs_shl_boundary_checked_guest
            PASS [   0.140s] ( 3/11) wasm32-pins-harness::zz_witness suanpan_add_limbs_shl_wrap_checked_guest
            PASS [   0.144s] ( 8/11) wasm32-pins-harness::zz_witness suanpan_shl_wrap_checked_guest
    # profile verification from the -v build logs (grep of the suanpan / wasm32_pins_guest rustc lines):
    # checked:   crate-name suanpan	-C overflow-checks=on	--target wasm32-unknown-unknown
    # unchecked: crate-name suanpan	-C opt-level=3	--target wasm32-unknown-unknown   (no overflow-checks flag anywhere in the log: `grep -c overflow-checks` = 0)
    ```

    (`suanpan_add_limbs_shl_wrap_unchecked_guest` asserts `Outcome::Value(5)`, returned only when `sign == Greater`, `limbs.len() == 1`, and `digit_count() == 1`; `suanpan_shl_wrap_unchecked_guest` asserts `Outcome::Value(1)`; the `_checked_guest` twins and both boundary tests assert `Outcome::Trapped(Trap::UnreachableCodeReached)`.)

Resolution: compute every landing position through one checked helper wired to the documented panic (the natural home is suanpan-12's `split_shift`): `digit_shift.checked_add(offset).expect("digit positions fit a usize")`, with `2 * limb_index` via `checked_mul` and `position.checked_add(1)` in `deposit_value`; or compute positions in `u64` and `usize::try_from` once per `add_at`. Restate the `# Panics` text in terms of the landing position (`shift / 32` plus the operand digit's offset). Acceptance: a red-first pin on a 32-bit target (extending crates/before/wasm32-pins' guest, which names suanpan nowhere today, or a new guest) observes the documented panic message for the constructions below; the 64-bit differential and ledger suites unchanged. Construction (32-bit target, release profile): (1) `let mut a = Accumulator::new(); a.add_limbs_shl([0u64, 0, 5], 32 * (u64::from(u32::MAX) - 3));`: `digit_shift = 2^32 - 4` passes `try_from`; limbs 0 and 1 are skipped (`low == high == 0`); limb 2's `low = 5` lands at `2 * 2 + (2^32 - 4) = 2^32`, which wraps to 0, so `a.sign_limbs()` returns `(Greater, vec![5])` and `a.digit_count()` is 1. (2) `a.add_u64(1); a.shl(64); a.shl(32 * (u64::from(u32::MAX) - 1));`: the held digits `[0, 0, 1]` skip offsets 0 and 1 in `fold_accum` and offset 2 wraps, so the value reads as 1. (3) The boundary sibling: at `shift = 32 * (2^32 - 1)`, `resize(pos + 1, 0)` at 1371 computes `usize::MAX + 1`, wraps to `resize(0)`, and `self.digits[pos]` at 1374 panics with an index-out-of-bounds: still the panic class, but not the documented message. In debug builds every case panics with "attempt to add with overflow".

### suanpan-25: `magnitude_from_digits` collects bytes through an unsized `flat_map`
- Where: crates/suanpan/src/accumulator.rs:1505-1511 (related: 1499-1504)
- Class / severity / confidence: performance / nit / high
- Provenance: assessed (read: `FlatMap`'s `size_hint` lower bound counts only active inner iterators, so `collect` starts small and grows by doubling); executed: no
- Seen by: claims; refutation: confirmed; history: deliberate-and-holds for the byte route (1499-1504 states the one-code-path rationale); no rationale for the missing presize
- Owner-gated: no

Evidence:

      1505	fn magnitude_from_digits(digits: Vec<u32>) -> UBig {
      1506	    let bytes: Vec<u8> = digits
      1507	        .iter()
      1508	        .flat_map(|digit| digit.to_le_bytes())
      1509	        .collect();
      1510	    drop(digits);
      1511	    UBig::from_le_bytes(&bytes)

Resolution: `Vec::with_capacity(4 * digits.len())` plus `extend`; no touch or behavior change. (`UBig::from_words` would remove the byte pass but trades the stated one-code-path property; leave it unless the readout shows in before's bench-judge.) Acceptance: one allocation for the byte image.

### suanpan-26: `Limbs::next_back` and the 32-bit word pairing are exercised only by a consumer crate
- Where: crates/suanpan/src/limbs.rs:68-72 (related: 19-29; crates/before/src/codec/base.rs:111)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep: no `Limbs::new`, `next_back`, or `.rev()` over a `Limbs` in crates/suanpan's tests; before's `msb_windows` at codec/base.rs:111 is the only reverse consumer; `WORDS_PER_LIMB` is 2 only where `Word` is 32 bits, and suanpan's suite has no 32-bit run); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found (4cf40eb9 made `Limbs` public with no tests)
- Owner-gated: no

A public method whose only coverage lives in another crate's suite is a blind spot the roster cannot see (the roster's cost exclusion for `Limbs iteration` is about denomination, not correctness). A chunk-order or padding bug in `next_back` would surface as a rank failure in before, not as a `Limbs` failure here.

Evidence:

        68	impl DoubleEndedIterator for Limbs<'_> {
        69	    fn next_back(&mut self) -> Option<u64> {
        70	        self.chunks.next_back().map(pack_limb)
        71	    }
        72	}

Resolution: a sibling `limbs/tests.rs` proptest: for random little-endian byte strings `b`, `Limbs::new(&UBig::from_le_bytes(&b)).collect::<Vec<u64>>()` equals the minimal LE `u64` limbs of `b`, `.rev()` equals the reverse, and zero yields an empty iterator; the same test covers the pairing if a 32-bit run is ever wired. Acceptance: a mutant replacing `next_back`'s body with `self.chunks.next().map(pack_limb)` fails inside suanpan. Construction: make that swap and run `cargo nextest run -p suanpan --all-features`: green today.

### suanpan-27: Incident history in roster prose ("twice found wrong in review")
- Where: crates/suanpan/src/claims.rs:12-14 (related: crates/suanpan/src/claims/tests.rs:202-204)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn 'found wrong\|before this binding'` hits exactly these two sites; `git log -S'twice found wrong'` resolves to the roster's landing commit 9e7b7ce3); executed: no
- Seen by: structure (11), prose (16), correctness (35), claims (51); refutation: confirmed; history: no rationale found (survived the dated-notes sweep d2a9d04e only for lacking a date); a Principle 5 doctrine breach, not a root AGENTS.md letter breach (no deleted code is named)
- Owner-gated: no

State the rule and the failure it prevents; provenance lives in git.

Evidence:

        12	//!   row must be named by some claim — the table was twice found wrong
        13	//!   in review before this binding existed, so it is held to the roster
        14	//!   like any other committed data.

    claims/tests.rs:
       202	/// The table is the crate's most-read cost surface and was twice found
       203	/// wrong in review; this binding makes editing it without the roster
       204	/// (or vice versa) a named failure.

Resolution: at both sites, "the table is committed data held to the roster; an edit to either without the other is a named failure". Acceptance: the grep returns nothing.

### suanpan-28: `SOURCES` is a hand-kept file roster; a new module with `pub` items escapes the totality test
- Where: crates/suanpan/src/claims.rs:34-57 (related: 82-84; crates/suanpan/src/claims/tests.rs:171-195; crates/suanpan/src/lib.rs:352-362; crates/surface-scan/src/lib.rs:73-133)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read: `extract_public_fns` walks only the listed specs; the liveness probe at tests.rs:174-177 checks two known names; the family list states its review-held totality at claims.rs:82-84 while `SOURCES` states none; before's rustdoc-JSON `surfacecheck` names suanpan nowhere); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found
- Owner-gated: no

Principle 6: the roster's totality guarantee ("a new public operation fails here until its cost row is pinned", lib doc 20-23) holds only for files someone remembered to list.

Evidence:

        34	/// The public-API sources of record: every module carrying `pub` items
        35	/// (the crate root re-exports them and declares no `pub fn` of its own).
        36	pub(crate) const SOURCES: &[SourceSpec] = &[

Resolution: a binding test that parses `src/lib.rs` for its `mod name;` / `pub mod name;` declarations and asserts each (except `claims`) has a `SourceSpec`, so an unlisted module fails by name. Acceptance and construction: add `mod scratch;` with a `pub fn` to lib.rs unlisted: the claims tests are green today and must fail afterwards; listing it then fails totality until a claim row exists.

### suanpan-29: `pub(crate)` on `cfg(test)` roster items read only by their child module
- Where: crates/suanpan/src/claims.rs:36-36 (related: 61, 71, 74, 77, 79, 85, 116, 428; crates/suanpan/src/lib.rs:364-365)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep `pub(crate)` in claims.rs: the nine sites; no file outside src/claims names `SOURCES`, `FAMILY_SURFACE`, `CLAIMS`, `cost_table`, `Evidence`, or `Claim`; `mod claims` is `#[cfg(test)]`); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found
- Owner-gated: no

The modifier suggests a crate-wide consumer that does not exist; the child `claims::tests` sees the parent's private items and fields by module privacy.

Evidence:

        36	pub(crate) const SOURCES: &[SourceSpec] = &[

Resolution: drop the modifier on all nine items. Acceptance: the claims tests compile unchanged.

### suanpan-30: The roster reaches into before's test file as the sole evidence for `add_small`/`sub_small`
- Where: crates/suanpan/src/claims.rs:96-99 (related: 123-132; crates/suanpan/src/claims/tests.rs:279-283; crates/suanpan/src/accumulator/tests/metered.rs:498-520; crates/before/tests/meter.rs:6622-6627)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (read: `BANDS` resolves through `crate_root().join(file)` at tests.rs:280; `add_small`/`sub_small` cite only `(BANDS, "accum_comb_touches_flat")`; every other `Witnessed` row has an `OWN` co-witness; `u64_comb_touches_are_flat_and_exact` is the exact u64 comb one signedness away); executed: no
- Seen by: structure (8), claims (46); refutation: confirmed both; history: already-known (deliberate at landing in 9e7b7ce3 and "flagged for the owner" in 318cd654, with no recorded ruling)
- Owner-gated: yes (a flagged owner decision; also whether standalone `cargo test -p suanpan` outside this workspace layout is a goal, since tests.rs:280-282 panics on a missing file)

Dependency direction: a substrate's verification reaching upward into its consumer's test-file layout couples suanpan's gate to before's envelope names, carries a special case in prose, and leaves the crate's headline rows (the crate page's opening example is built on them) without an in-crate exact pin. An exact in-crate pin is also stronger than before's x1.25 band.

Evidence:

        96	/// The digit-touch stream bands committed in the sibling meter suite —
        97	/// the one workspace-relative path in the roster; the binding tests
        98	/// resolve it from the manifest directory.
        99	const BANDS: &str = "../before/tests/meter.rs";
       123	    Claim {
       124	        op: "Accumulator::add_small",
       125	        table_cost: Some("amortized O(1)"),
       126	        evidence: Evidence::Witnessed(&[(BANDS, "accum_comb_touches_flat")]),
       127	    },

Resolution: an `i64` twin of `u64_comb_touches_are_flat_and_exact` driving `add_small(1)`/`sub_small(1)` on the `2^k - 1` cliff at k = 4096 and 8192 with the derived exact total, cited under both rows; keep or drop the `BANDS` citations as the owner rules (their `OWN` co-witnesses stand). Acceptance: `add_small` and `sub_small` each cite an `OWN` witness that reaches them; if `BANDS` is dropped, no path outside the manifest directory remains in claims.rs.

### suanpan-31: The `constant()` shorthand covers four rows while six rows of the same shape are longhand
- Where: crates/suanpan/src/claims.rs:101-109 (related: 118, 322, 415, 419; 362-413)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read); executed: no
- Seen by: structure; refutation: confirmed; history: no rationale found
- Owner-gated: no

The helper builds `{ table_cost: None, evidence: Excluded(reason) }` and is used for four rows; six rows at 362-413 have exactly that shape written out. Its doc says "word-scale operation", which the derived-surface and re-export rows are not, so either the name or the usage is inconsistent.

Evidence:

       101	/// Shorthand for a word-scale operation with no table row, excluded
       102	/// from instrumentation with its mechanism.
       103	const fn constant(op: &'static str, reason: &'static str) -> Claim {

Resolution: rename to `excluded(op, reason)` and use it for all ten no-table-row exclusions, or delete it. Acceptance: every `table_cost: None` + `Excluded` row uses one spelling.

### suanpan-32: Roster data states mechanisms and labels the code does not have
- Where: crates/suanpan/src/claims.rs:118-121 (related: 313-320, 89, 397-405; crates/suanpan/src/accumulator.rs:89, 163-171, 942-948, 1286-1291, 1493-1497; crates/suanpan/src/claims/tests.rs:299-303)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (git: `git show 1c1cb67d -- crates/suanpan/src/accumulator.rs` replaces `digits: vec![0]` with `quick: Some(0)` and `digits: Vec::new()`; `git log -S'allocates the one-digit buffer'` resolves to 9e7b7ce3 (2026-07-29), before 1c1cb67d (2026-08-04); accumulator.rs:89 is `#[derive(Debug, Clone)]` and 1493-1497 a hand-written `impl Default`); executed: no
- Seen by: structure (10), prose (15), claims (43), correctness (38); refutation: confirmed, reframing 15 (the `is_literally_zero` reason is accurate in the meter's read-modify-write denomination); history: deliberate-but-expired (both reasons landed against the pre-register bodies; the `Default` label was inaccurate from birth)
- Owner-gated: no

In this roster an exclusion reason is the claim's only evidence (claims.rs:65-66), so a false mechanism is a claim with false evidence, and a row label is committed data the tests byte-compare. `new()` allocates nothing (the one-digit allocation lives in `enter_digit_engine`, 1286-1291); `digit_count`'s register arm is a `leading_zeros`/`div_ceil`, not "one field read plus an increment"; `Default` is hand-written, not derived. Principle 5. The binding test holds reasons to a 20-character floor (tests.rs:299-303), which is why these passed (suanpan-37).

Evidence:

       118	    constant(
       119	        "Accumulator::new",
       120	        "allocates the one-digit buffer: word-scale, no input axis to measure against",
       121	    ),
       316	        evidence: Evidence::Excluded(
       317	            "one field read plus an increment; the exact-top maintenance it rests on is \
        89	    "Accumulator Clone / Debug / Default (derived surface)",

    accumulator.rs:
       163	    pub fn new() -> Accumulator {
       164	        Accumulator {
       165	            quick: Some(0),
       166	            digits: Vec::new(),
      1493	impl Default for Accumulator {

Resolution: rewrite the `new` reason ("builds a register-held zero: no allocation, no digit, no input axis to measure against"), the `digit_count` reason to cover both tiers, and the label to "(trait surface)" at 89 and 398 with the exclusion text at 400-404 adjusted. Acceptance: each sentence matches the current body it describes; `cited_witnesses_exist` and `claims_are_total_over_the_public_surface` still pass.

### suanpan-33: The only sequence-shaped adversarial instrument is cited by no claim
- Where: crates/suanpan/src/claims.rs:277-285 (related: 111-115; crates/suanpan/tests/amortized_sequences.rs:36-49, 63-78; crates/suanpan/src/claims/tests.rs:116-161)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read claims.rs in full: no citation of tests/amortized_sequences.rs or `sign_flip_oscillation_has_no_width_product`; the reach scan would pass: the test body calls `s1(`, whose body invokes `.add_wide_shl(`, `.sign(`, `.sub_wide_shl(`); executed: no
- Seen by: claims; refutation: confirmed; history: no rationale found (4398dcd4 landed it uncited; the nearest recorded stance, b8513643's "minimal citation sets", concerns semantic pins, and the roster's charter on this point is undocumented in code)
- Owner-gated: no

Principle 6 (tamper-evidence): an instrument outside the roster can vanish without a named failure, and this is the crate's only committed check that the sign fold and shifted writes stay additive under an adversary flipping sign across a certified run.

Evidence:

       277	    Claim {
       278	        op: "Accumulator::sign",
       279	        table_cost: Some("amortized O(1)"),
       280	        evidence: Evidence::Witnessed(&[
       281	            (OWN, "no_collapse_fold_re_scans_the_prefix"),
       282	            (OWN, "sign_fold_skips_certified_runs"),
       283	            (BANDS, "accum_static_prefix_touches_flat"),
       284	        ]),
       285	    },

Resolution: `const SEQUENCES: &str = "tests/amortized_sequences.rs";` cited under `sign`, `add_wide_shl`, and `sub_wide_shl`; and state the roster's citation charter (minimal sets or every touch instrument) at claims.rs:111-115 so the next uncited instrument is a decision, not an omission. Acceptance: renaming the test fails `cited_witnesses_exist`; the reach test passes without a `REACH_EXEMPT` entry. Construction: delete the file on a scratch copy: the suanpan suite stays green today.

### suanpan-34: `digit_count`'s exclusion reason names a test the roster never checks
- Where: crates/suanpan/src/claims.rs:316-320 (related: crates/suanpan/src/claims/tests.rs:269-306; crates/suanpan/src/accumulator/tests/metered.rs:195-214)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (read: `cited_witnesses_exist` checks names only inside `Evidence::Witnessed`; the test exists today at metered.rs:196); executed: no
- Seen by: correctness; refutation: confirmed; history: no rationale found
- Owner-gated: no

A name in prose is a leg without a binding, exactly the rot the roster exists to prevent (claims.rs:5-6).

Evidence:

       316	        evidence: Evidence::Excluded(
       317	            "one field read plus an increment; the exact-top maintenance it rests on is \
       318	             priced on the writes (top_settlement_steps_are_metered pins the settle \
       319	             scan's metering)",
       320	        ),

Resolution: make the claim `Witnessed(&[(OWN, "top_settlement_steps_are_metered")])` with a `REACH_EXEMPT` entry stating the delegation, or drop the name from the prose. Acceptance: renaming the test fails a claims test by name.

### suanpan-35: Hand-rolled source scanning where `syn` is the mature tool
- Where: crates/suanpan/src/claims/tests.rs:66-72 (related: 73-108, 357-361; crates/surface-scan/src/lib.rs:19-25)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read: `fn_bodies` counts every brace character (88-98) and ends a body when the count returns to zero (99-102), so a `}` inside a literal ends a body early with no signal; the convention is stated in prose (66-72) and checked nowhere; on an exempted edge the truncation lands in the silent `(false, Some(_))` arm at 361. The refutation pass reports syn 1.0.109 and 2.0.117 already in Cargo.lock); executed: no
- Seen by: structure; refutation: confirmed (the masking clause is narrow; the steelman is real); history: deliberate-and-holds (the line-scan design and its loud-panic defense are stated at surface-scan lib.rs:19-25 and tests.rs:66-72; no syn discussion anywhere)
- Owner-gated: yes (a design proposal against a stated design)

Doctrine: prefer a dependency over hand-rolling; infrastructure that rests on a convention held in prose rather than a committed check is suspect. The scanners are short and panic loudly on an unclosed body, which is why this is a proposal, not a defect.

Evidence:

        66	/// A line scan over rustfmt-normalized shape, like the shared scanners:
        67	/// a definition is a line whose trimmed form starts with `fn ` (or
        68	/// `pub fn `), and its body ends where the braces opened since the def
        69	/// line balance. Brace counting is textual — the witness files carry no
        70	/// unbalanced brace in any literal or comment (format placeholders pair)
        71	/// — and an unclosed body panics rather than silently truncating the
        72	/// reach analysis.

Resolution: owner decision: a `syn`-based visitor (`full` + `visit`) in surface-scan yielding `#[test]` fn names, `pub fn` items by impl/module context, and method-call receivers by name, replacing the brace-balance convention; or keep the scanners and add a committed check that no witness file contains an unbalanced brace inside a literal or comment. Acceptance: a witness file containing `"}"` in an assert message changes no test's verdict.

### suanpan-36: The table locator hardcodes `Accumulator::`, an unreachable `else` arm, and `position` without the uniqueness the doc claims
- Where: crates/suanpan/src/claims/tests.rs:216-217 (related: 342-344; crates/suanpan/src/claims.rs:425-427, 437-440)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read: `rsplit("::").next()` always yields `Some`; every current table row is an `Accumulator` method; `cost_table`'s doc says "locates the unique" header while the code uses `position`); executed: no
- Seen by: claims (50), prose (open question); refutation: confirmed; history: no rationale found (verbatim from 9e7b7ce3)
- Owner-gated: no

Latent, not live: a future table row for a `Limbs` or `Magnitude` operation could never bind, and dead arms are noise in a file whose job is to be obviously right.

Evidence:

       216	        let short = claim.op.rsplit("::").next().expect("ops are pathed");
       217	        let locator = format!("](Accumulator::{short})");
       342	        let Some(method) = claim.op.rsplit("::").next() else {
       343	            continue;
       344	        };

    claims.rs:
       437	    let header = doc_lines
       438	        .iter()
       439	        .position(|line| *line == "| Operation | Cost |")

Resolution: `format!("]({})", claim.op)` as the locator; a plain `rsplit` + `expect` at 342; assert exactly one header line (or drop "unique" from the doc). Acceptance: identical verdicts for the current roster.

### suanpan-37: The testdoc says exclusions "state a mechanism"; the body checks a 20-character floor
- Where: crates/suanpan/src/claims/tests.rs:262-263 (related: 23-24, 299-303, 384-387)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (read); executed: no
- Seen by: prose; refutation: confirmed; history: deliberate-and-holds for the proxy (978c85c1 chose ">= 20 chars, checked"); the doc overstating it has no rationale
- Owner-gated: no

A test's doc comment states its invariant and must be accurate; a 20-character shrug satisfies this one, and suanpan-32 is the live instance: a 78-character false mechanism passed.

Evidence:

       262	/// Every witness a claim cites exists as a `#[test]`-attributed
       263	/// function in its file, and every exclusion states a mechanism.
       299	                assert!(
       300	                    reason.trim().len() >= 20,
       301	                    "{}: an exclusion reason must state a mechanism, not a shrug",
       302	                    claim.op
       303	                );

Resolution: make the doc match the check at 262-263 and 23-24 ("every exclusion carries a reason of non-trivial length; its substance is review-held"); optionally strengthen the check (require a code identifier or a denomination word such as "word-scale", "digit", "allocation") and say so. Acceptance: the docstring's invariant is one the assertion can fail on. Construction: set the `new` reason to any 20+ character string; `cited_witnesses_exist` passes.

### suanpan-38: `scaled_read_costs_the_written_span` asserts a `<= 16` ceiling where the exact count is 3
- Where: crates/suanpan/src/accumulator/tests/metered.rs:43-47 (related: 1-11, 50-54, 786-795, 810-829)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (trace: `apply_limbs` of `[7, 9]` at digit_shift 1000 deposits 7 at 1000 and 9 at 1002 (the odd-limb high halves are zero); `bottom = 1000`, `top = 1002`; `sign_magnitude_shl` starts at `min(bottom, top) = 1000` and `read_digits` walks 1000..=1002 with carry 0 and no drain: 3 touches); executed: no
- Seen by: correctness; refutation: confirmed; history: deliberate-but-expired (ef8894ac wrote the ceiling before the exactness header; 023eff6e added "Every pin asserts an exact count" over a file already holding it)
- Owner-gated: no

Header lines 4-5 say every pin is exact; this pin admits a watermark off by up to thirteen digits, while its sibling `scaled_read_costs_the_span_not_the_write_count` pins its 1,003 exactly.

Evidence:

         4	//! Every pin asserts an exact count, not a ceiling: exactness is the
         5	//! liveness floor (a counter that silently stops counting cannot
        43	    assert!(
        44	        scaled_read <= 16,
        45	        "the scaled read scanned {scaled_read} digits: the write watermark \
        46	         is not skipping the never-written prefix"
        47	    );

Resolution: `assert_eq!(scaled_read, 3, ...)`, confirming 3 by one run before pinning; keep the `> 1000` control; amend the header to say adequacy legs (`> 1` at 793, the control at 51) are floors by design. Acceptance: the test passes at exactly 3 and the header sentence is true of the file.

### suanpan-39: The sign-flip oscillation tripwire has no liveness floor
- Where: crates/suanpan/tests/amortized_sequences.rs:52-61 (related: 36-49; crates/before/tests/meter.rs:6493-6500)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (read: with an all-zero grid, `mixed = 0`, `bound = 0`, and `0 <= 0` passes; `s1`'s other assertions are value-only). Hand trace of `s1`, not executed: the first write spills the register (2 touches); each steady-state round costs 2 (add: limb read + deposit) + 5 (sign: read top, zero it, read below deciding at `2^32`, zero the floor, re-deposit) + 2 (sub) + 7 (sign: read, zero, read to partial 0, zero, run skip to digit 0, read -1, zero, re-deposit) = 16, so `s1(n, d) = 16n + 2` at every grid cell; the refutation pass reached the same trace independently
- Seen by: correctness; refutation: confirmed; history: no rationale found (4398dcd4 adopted the shape with no floor; its sibling in before, tests/answer_embedded.rs, has none either)
- Owner-gated: no

Doctrine: every ceiling needs a floor derived from irreducible work so it cannot pass vacuously when the counter goes dark. before's bands carry `touches >= ops` and metered.rs pins exact counts; this is the one suanpan touch instrument with neither.

Evidence:

        52	fn assert_no_product(name: &str, grid: [u64; 4]) {
        53	    let mixed = grid[3] as f64 - grid[2] as f64 - grid[1] as f64 + grid[0] as f64;
        54	    let bound = 0.10 * grid[3] as f64;
        55	    eprintln!("MEASURED {name}: grid {grid:?} mixed {mixed:.0} bound {bound:.0}");
        56	    assert!(
        57	        mixed.abs() <= bound,

Resolution: a per-cell floor from irreducible work (two one-limb writes and two sign reads per round: `s1(n, d) >= 6 * n`); better, pin the exact total `16 * n + 2` at all four cells, which subsumes the floor and the no-product bound. Treat `16n + 2` as a hypothesis until one run confirms it. Acceptance: with `touch_meter::record` stubbed to a no-op the test fails; unmodified, the pinned counts hold at all four cells. Construction: stub `record` and run `cargo nextest run -p suanpan --features touch-meter sign_flip_oscillation_has_no_width_product`: it passes today.

### suanpan-40: Two suanpan mutant exclusions claim equivalence, but both mutants change exact touch counts, which the crate declares a public contract
- Where: .cargo/mutants.toml:85-95 (related: 5-24, 33-35; crates/suanpan/src/accumulator.rs:601-609, 1108-1122; crates/suanpan/src/lib.rs:283-286; tools/mutantcheck-expected.json:4-11; crates/suanpan/src/accumulator/tests/metered.rs:405-441)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (trace: with `<<=`, the drain loop over `u128` `high` in 1..=3 runs four times (`high * 2^32`, `* 2^64`, `* 2^96`, then the bits shift out to 0), adding 3 touches and 3 trailing zero digits that `sign_limbs` pops and `from_le_bytes` normalizes away; with `&&`, `shl(0)` on a digit-engine value costs a full take-and-redeposit (about 2d touches) instead of 0, and `shl(k)` on a literal-zero register costs 1 (`k <= 30`) or retires the register (`k > 30`) instead of 0. No committed pin reads a spelling with a nonzero final carry: the held-width pin reads `2^k - 1`, whose digits are all `2^32 - 1` after `add_wide`, so carry stays 0 (metered.rs:409-441); grep finds no `.shl(0)` under crates/suanpan); executed: no
- Seen by: correctness; refutation: confirmed, and added the dissolving refactor and the mutantcheck-expected.json coupling; history: deliberate-and-holds as recorded rulings (the exactness contract landed in 1255a4e2 on 2026-08-07, before the roster; the history pass reports the `shl` entry landed in d35c9019 and the `read_digits` entry was re-pointed by 8da57920), so this disputes the header's criteria as applied to a crate with no slack band, not a stale ruling
- Owner-gated: yes (recorded exclusion rulings)

The file's own policy admits an exclusion only for an empty discriminating class or for work moved inside a meter's deliberate headroom (5-13), and suanpan's pins have no headroom: exact counts are the contract (lib.rs:283-286). "Value-equivalent" is not "observationally equivalent" under that contract, and the `shl` rationale's claim that no band can price a zero shift without pinning the representation is false for a touch pin of 0. The header's ladder (16-19) prescribes refactoring before excluding, and the `>>=` site admits exactly that.

Evidence:

        85	    # Proven equivalent: read_digits' final carry has magnitude at
        86	    # most 3 (bound derived in the comment at the site), so each high-part
        87	    # drain emits its whole value in its first digit and the mutated shift
        88	    # direction has an empty discriminating class.
        89	    "accumulator\\.rs.*: replace >>= with <<= in Accumulator::read_digits",
        90	
        91	    # Performance genre, owner-declared benign fast path: the identity
        92	    # guard routes cost and representation only (the comment at the site
        93	    # carries the argument), and no honest meter band can price a zero
        94	    # shift without pinning the representation.
        95	    "accumulator\\.rs.*: replace \\|\\| with && in Accumulator::shl",

    accumulator.rs:
      1109	            while high > 0 {
      1110	                touch(1);
      1111	                collected.push((high & u128::from(DIGIT_MASK)) as u32);
      1112	                high >>= DIGIT_BITS;
      1113	            }

Resolution: (1) `read_digits`: the site's own derivation (1076-1079) bounds `|carry| <= 3`, so `high` fits one digit; replace both `while high > 0 { ...; high >>= DIGIT_BITS; }` loops (1109-1113, 1118-1122) with `if high > 0 { touch(1); collected.push(u32::try_from(high).expect("the final carry fits one digit: |carry| <= 3")); }`, which moves no touch, removes both `>>=` codepoints, and dissolves the exclusion per ladder step (1). (2) `shl`: delete the exclusion and add exact pins to metered.rs: `shl(0)` on a digit-engine value and `shl(40)` on a literal-zero register both cost 0 touches, with `quick.is_some()` asserted after the latter; also pin a readout with a nonzero final carry (a single digit `-(2^33 - 1)` reads out as low 1, carry -2, one drain digit: 3 touches) so the negative read-out count is covered. (3) Update tools/mutantcheck-expected.json:4-11 (listed 2 / suppressed 2 and 1 / 1) in the same commit, or the mutantcheck leg fails. If the owner keeps either exclusion, restate its rationale to name the touch leg it waives, as the header demands. Acceptance: `cargo mutants --workspace` with both entries removed reports the `&&` mutant caught and the `>>=` mutant no longer listed. Construction: apply the `<<=` mutant by hand and run a metered scenario with a nonzero final carry (`park_extreme_negative_digit(&mut a, 0)` from accumulator/tests.rs:110, then `touch_meter::reset(); a.sign_magnitude()`): production counts 3 (digit, complement pass, one drain), the mutant 6. Apply the `&&` mutant: `a.add_wide(&(UBig::ONE << 2048usize)); touch_meter::reset(); a.shl(0);`: production 0 touches, mutant about 130.

## Positives

- The crate page (lib.rs) is the strongest public prose in the two crates: every bound in the cost table has its argument on the same page, the amortization vocabulary is the standard potential-method one, hazards are stated where a user meets them (sign queries take `&mut self` and why, 123-126; no `PartialEq` and why, 307-309; `Sync` buys less than usual with the exact list, 299-307; `is_literally_zero` is one-sided with a worked example, accumulator.rs:888-911; `sign_magnitude_shl`'s pair is not a normal form, 970-993; touch readings need serial scenarios, touch_meter.rs:16-20), and the "When not to reach for it" section (250-268) does the which-one-do-I-use job explicitly, binary-counter argument included.
- Coined terms are anchored to identifiers almost without exception: lazy zone/`LAZY_LIMIT`, recenter/`RECENTER_BIAS`, quick register/`quick`, zero-run ledger/`zero_runs`, collapse/`fold_and_collapse`, spill/`spill`, digit engine/`enter_digit_engine`, written span/`bottom`, limb/`Limbs` (the 64-bit definition stated once at lib.rs:85-86 and enforced by `WORDS_PER_LIMB`).
- The `zero_runs` field doc (accumulator.rs:124-154) is a correct and complete statement of the ledger's invariants; I traced jump-insert, `crop_runs`' descending early stop, `consume_run_at`, and collapse through a certified run by hand, and every clause (soundness, disjointness, containment under the settled top, including the lower-remnant collapse case) holds as written. `ledger_invariants_hold_exhaustively` checks the letter of each clause after every step of every schedule at depth 6 over an alphabet chosen to reach every collapse-over-certificate interaction.
- The exact-count-at-two-scales pin form (metered.rs:1-11) makes the liveness floor and the flatness witness one assertion (e.g. `6 * pairs + 1` at k = 4096 and 8192), stronger than before's x1.25 envelope convention, and the known-bad mechanism (`no_collapse_fold_re_scans_the_prefix`) is committed and shown red at two widths. `scaled_read_costs_the_span_not_the_write_count` refutes the tempting misreading of the O(w) row with a 1,003-touch pin.
- The three constants are pinned tight by constructed witnesses that mutation testing motivated (`SIGN_DECIDED` 3 -> 2 and a decision index of floor + 1 each survive the differential suite and fail only in witnesses.rs); the 2.01 geometric bound and the 3 > 2.01 margin check out, and `sign_dominates_at`'s doc is precise that `decided` is a property of the representation, pinned both ways.
- `read_digits`' carry comment (1074-1079) is a real one-paragraph proof (I checked that with `|digit| < 2^33` the recurrence keeps carry in `[-3, 2]` from 0); `add_at`'s invariant-window comment (1356-1368) names exactly where the exact-top invariant does not hold and why the loop restores it; `sign_dominates_at`'s register arm is total by construction (`checked_add`/`checked_mul`/`try_from`/`checked_shl`) and the digit arm's `saturating_add(2)` states the wrap it prevents in one sentence.
- The metering seam (`touch`, accumulator.rs:16-26) compiles to nothing without the feature so the kernel carries no cfg noise; no `debug_assert!` body meters, so the exactness contract is independent of the debug-assertions setting; `Limbs` derives `WORDS_PER_LIMB` from `u64::BITS / Word::BITS` so the denomination is target-independent by construction and iteration borrows the stored words.
- The claims roster is total in both directions over the extracted surface, binds witnesses by reach rather than existence (closing the hollowed-out-witness path), and holds `REACH_EXEMPT` load-bearing so a stale exemption fails by name (claims/tests.rs:369-373). Module decomposition is clean: four single-responsibility modules, an acyclic dependency graph, and a crate root that re-exports without declaring anything of its own.

## Open questions for Finch

1. Is the exact-touch contract (lib.rs:283-286, "a change to any operation's count is a breaking change") meant to freeze constant-factor improvements to the kernels, or to guarantee determinism with the specific numbers versioned? suanpan-17 (the 6-touch collapse fixed point), suanpan-10 (lazy spill), and the fused per-limb pass hypothesis below are all blocked by the first reading. Recommendation: restate it as "deterministic and pinned; a count change is a versioned change named in its commit", which keeps the pins as the contract's enforcement without forbidding improvement.
2. Is suanpan meant to be publishable or testable standalone? The `../before/tests/meter.rs` path (suanpan-30) and the workspace-path `surface-scan` dev-dependency make `cargo test -p suanpan` fail outside this checkout. Recommendation: add the in-crate `add_small`/`sub_small` pin either way; decide on `BANDS` by whether the answer is yes.
3. Should suanpan's own suite ever run on a 32-bit target? Today crates/before/wasm32-pins names suanpan nowhere, so suanpan-24's red-first pin and suanpan-26's `WORDS_PER_LIMB = 2` coverage have no first-party home. Recommendation: extend the wasm32 guest with a small suanpan leg (the two constructions in suanpan-24 plus the `Limbs` round trip); the fix for suanpan-24 should land regardless.
4. What is the roster's citation charter: minimal citation sets per row (b8513643's stance, stated only in a commit message and about semantic pins), or every touch instrument cited? suanpan-33 turns on it. Recommendation: state it at claims.rs:111-115 and cite tests/amortized_sequences.rs either way, since it is a cost instrument, not a semantic pin.
5. A measure-first design hypothesis, not a finding: `apply_limbs` (1467-1489) makes two `add_at` calls per limb, each running the jump check, carry loop, `crop_runs`, and `settle_top`; a fused low-to-high pass over the operand span (one carry variable, one crop and settle at the end) would remove per-call overhead and touch a carried-into position once instead of twice. Release overhead per call is small (a bounds check, an empty-map early return, one loop-condition read), suanpan has no bench, and the change moves the public touch counts. Recommendation: only if before's bench-judge shows the wide path on a profile; measure at the parent first.
6. The touch-accounting contract is stated in full twice (lib.rs:272-292 and touch_meter.rs:3-20) plus a summary in Cargo.toml:23-27; because `touch_meter` is cfg'd out without the feature the crate page cannot simply link to it. Recommendation: keep both, add one line to each saying the two move together, and fold the suanpan-6 amendment into both at once.

## Dropped

- [49] Fused per-limb `add_at` pass: reframed by the refutation pass to a measure-first hypothesis with unmeasured gain and a contract cost; moved to open question 5.
- [15]'s `is_literally_zero` leg: "two field reads: no digit is touched" is accurate in the meter's read-modify-write denomination (a plain read of `digits[0]` is not a touch); the `new` and `digit_count` legs survive in suanpan-32.
- [20]'s "arm" leg: the arming family is before's established watermark-web vocabulary (245 uses in crates/before/src), not a register transplant; the other legs survive in suanpan-19.
- [22]'s "certificate" leg: each referent is defined under its own crate-page heading (128 and 170) and the bare uses in accumulator.rs sit under docs whose subject fixes the referent; the "derived" and pool-reuse legs survive in suanpan-19.
- [17]'s "Three maintainers" and "four `*_accum` entry points" legs: accurate as written (the list names the three certificate mechanisms; four public fns are so named); only the "both cost arguments" leg drifted (suanpan-7).
- [0]/[47]'s `enter_digit_engine` scan leg (1293-1296): once-per-epoch, explicitly within 9f68c475's ruling; the `add_at` exit scan survives as suanpan-21.
- [23]'s table-row leg: the table is denominated in digit touches (lib.rs:200) and `reserve_digits` has no digit-touch axis; the memory-paragraph leg survives as suanpan-5.
- [47] duplicate of suanpan-21; [39] duplicate of suanpan-24; [15], [43], [38] merged into suanpan-32; [16], [35], [51] duplicates of suanpan-27; [37] merged into suanpan-16; [46] merged into suanpan-30; [44], [52] merged into suanpan-7; [22] merged into suanpan-19.
- [0]'s construction as written (`reset()` then `add_small`): `reset()` returns the accumulator to the register (accumulator.rs:678), so the loop never reaches `add_at`; corrected in suanpan-21 with a spill before the loop.
