# Partition suanpan-tests: suanpan test suites: differential, ledger, metered, witnesses, amortized sequences

Reviewed at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean).

## Partition summary

The partition is the whole test surface of `suanpan`'s `Accumulator`: a shared harness (`crates/suanpan/src/accumulator/tests.rs`, 116 lines) and four sibling suites under `tests/` (`differential.rs` 1051, `ledger.rs` 297, `metered.rs` 890, `witnesses.rs` 652), plus one integration binary (`crates/suanpan/tests/amortized_sequences.rs`, 78). Every file in the partition is a test file; 3084 lines read in full with line numbers, together with the production code each traced count rests on (`accumulator.rs`, `touch_meter.rs`, `limbs.rs`, `magnitude.rs`, `claims.rs`, `claims/tests.rs`), the two proptest seed files, the sibling `before` band harness, and the refutation pass's run log.

The structure is sound. The harness gives every suite one mode-forcing constructor (`fresh`), one total value check (`assert_value`, which drives all three read-outs against an exact `IBig` oracle on every call), and one zone-edge construction (`park_extreme_negative_digit`). `differential.rs` runs randomized streams with a biased generator and pins the deterministic adversarial shapes the crate docs name; `ledger.rs` holds the zero-run ledger's full letter invariant on a ~1.95M-state exhaustive prefix tree and a deep-shift proptest; `metered.rs` pins exact digit-touch totals for the claims roster's rows, most of them repeated across an axis doubling, and commits one executable known-bad mechanism; `witnesses.rs` pins the decision constants at their tight edges, each with the surviving mutation it was built to kill recorded; `amortized_sequences.rs` is the one instrument that flips the sign under load. I re-derived the exact pins the lenses traced (alternating pair 5 and 3, settlement 16, certified-run skip 6, u64 comb 6n+1, domination 5 then 1, no-collapse model 2k/32+3) against `accumulator.rs` and they agree; the refutation pass executed the sign-flip test once and its output (`grid [32770, 65538, 32770, 65538]`) confirms a shift-independent `16n + 2` total.

Three findings are worth scheduling. `size_probe_covers_the_value` asserts a whole digit more slack than its doc claims, so the probe alone admits a `digit_count` that under-reports by one digit (other tests catch that mutant; this one does not test its own sentence). `merge_into_wider`'s swap, the `min` in its cost row, is never on any metered path: delete it and every committed test stays green. `assert_no_product` in `amortized_sequences.rs` bounds a relative quantity with no liveness floor, so a counter that records nothing passes it vacuously, while an exact pin is available and now measured. Below those: the ledger checker never observes states left by the fold, merge, shift, reset, or negate entry points; the shared helper's stated reason for its two-deposit shape is false against `LAZY_LIMIT`; three inequality pins sit in a module whose doc says every pin is exact; and a cluster of prose and idiom nits (hand-maintained tallies and a wall-time figure in a testdoc, two fragments left by a dated-note excision, a `u8` alphabet with a hand-counted cardinality, copied replay loops and run-forming arms, workspace-wide vocabulary and dash conventions) that a maintainer would batch.

## Findings

### suanpan-tests-1: differential.txt carries three seeds whose shrink records name an arm its property cannot draw
- Where: crates/suanpan/proptest-regressions/accumulator/tests/differential.txt:7-9 (related: crates/suanpan/proptest-regressions/accumulator/tests/ledger.txt:7-9, crates/suanpan/src/accumulator/tests/differential.rs:615-621, crates/suanpan/src/accumulator/tests/ledger.rs:249-254)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (both seed files read; `git show --stat 023eff6e` shows `tests.txt => tests/differential.txt` as a pure rename and `ledger.txt | 9 +`; the split commit's message read); executed: no
- Seen by: structure-prose [13], api-economics [31 seed half]; refutation: confirmed; history: deliberate-and-holds for the duplication, with the recorded premise overstated for the arm-4 seed
- Owner-gated: yes: seeds are never stripped without the owner's word

The three `cc` lines are byte-identical across the two files, and the first records a shrunk case with arm `4`, which `differential.rs`'s run-forming strategy `0u8..4` cannot draw; the split commit's rationale ("the three committed cases' (arm, limbs, shift) shape is drawn by the run-forming strategies in both files") does not hold for that seed. Proptest replays every seed in a file as an RNG seed for every property in it, so the lines are harmless; the shrink comments mislead a reader reproducing a failure by hand.

Evidence:

         7	cc 6bbecf10b34d1939a04794d4c9bbcc84b110f6e275db38dfdae1fae6dd92eaa8 # shrinks to ops = [(0, [4611686018158952448], 4003), (1, [25769803775], 3998), (4, [0], 0)]

    differential.rs:
       617	            (0u8..4, proptest::collection::vec(any::<u64>(), 1..=2), 0u64..4_096),

    ledger.rs:
       251	            (0u8..5, proptest::collection::vec(any::<u64>(), 1..=2), 0u64..4_096),

Resolution: owner's call. Either leave the lines and add a `#` comment line in `differential.txt` stating that the three seeds were carried at the suite split and their shrink comments describe the ledger property, or prune them from `differential.txt` in a commit that names the split as their provenance (`ledger.txt` keeps them). Acceptance: every shrink comment in a seed file is producible by a property in the file it sits beside, or the file says why not.

### suanpan-tests-2: park_extreme_negative_digit's stated reason for two deposits is false: one deposit of −(2^33 − 1) lands in the zone
- Where: crates/suanpan/src/accumulator/tests.rs:100-116 (related: crates/suanpan/src/accumulator.rs:34-39, crates/suanpan/src/accumulator.rs:1218-1238, crates/suanpan/src/accumulator.rs:1369-1380, crates/suanpan/src/magnitude.rs:46-49)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (trace by reading: `to_word` is `Some` for 2^33 − 1; `add_shifted_word` reaches `add_at`; `add_at` keeps `|total| < LAZY_LIMIT = 2^33`); executed: no
- Seen by: structure-prose [1]; refutation: confirmed; history: no-rationale-found (the sentence was false at inception; the later spill is the load-bearing step)
- Owner-gated: no

The doc says a single deposit of the full value would recenter. Against the code, `sub_magnitude_shl(&UBig::from((1u64 << 33) - 1), 32 * index)` takes the word path (`u64::try_from` succeeds), reaches `add_at` with `value = −(2^33 − 1)`, and `total.abs() = 2^33 − 1 < LAZY_LIMIT` selects the in-zone arm: no recenter. Comments state what the code cannot show and must be true; a maintainer reading this shared helper learns a wrong fact about where the zone closes.

Evidence:

       104	/// Two deposits of `−2^32` and `−(2^32 − 1)` land in one digit because
       105	/// each intermediate total stays inside the zone; a single deposit of
       106	/// the full value would recenter. This is the construction behind the
    ...
       113	    acc.spill();
       114	    acc.sub_magnitude_shl(&UBig::from(1u64 << 32), 32 * index);
       115	    acc.sub_magnitude_shl(&UBig::from((1u64 << 32) - 1), 32 * index);

    accumulator.rs:
        39	const LAZY_LIMIT: i128 = 1 << (DIGIT_BITS + 1);
      1375	            if total.abs() < LAZY_LIMIT {

Resolution: collapse to one call after the spill (`acc.sub_magnitude_shl(&UBig::from((1u64 << 33) - 1), 32 * index);`) and restate the doc: the zone is open at 2^33, so the extreme digit lands in one deposit; the spill first keeps the register from holding the value exactly instead of as a digit. Optionally add `debug_assert_eq!(acc.digits[index as usize], -((1i64 << 33) - 1))` so the helper pins its own postcondition. Acceptance: `witnesses.rs` and `differential.rs` pass unchanged with the one-call helper and the postcondition assert.
Construction: make the one-call change and run the accumulator tests; a green run demonstrates the single deposit does not recenter, refuting the sentence.

### suanpan-tests-3: the differential generator never calls the public word entries directly, and the word-shift entries are invoked only under the touch-meter feature
- Where: crates/suanpan/src/accumulator/tests/differential.rs:50-53 (related: crates/suanpan/src/accumulator/tests/differential.rs:467-472, crates/suanpan/src/accumulator/tests/metered.rs:122-126, crates/suanpan/src/accumulator/tests.rs:35-44)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (grep: `add_u64_shl`/`sub_u64_shl` appear outside `accumulator.rs` only at metered.rs:122-126 and in the README table; `merges_match_the_oracle` uses `Accumulator::new()` at :467 and :472); executed: no
- Seen by: blind-spots [26]; refutation: reframed (the bodies of `sub_small`/`add_u64`/`sub_u64` are reached through the exhaustive ledger alphabet and the magnitude dispatch proptests; only the direct public entries `add_u64_shl`/`sub_u64_shl` are unexercised outside metered.rs); history: no-rationale-found
- Owner-gated: no

`apply` maps `Op::Small` to `add_small` alone, so `sub_small`, `add_u64`, `sub_u64`, `add_u64_shl`, and `sub_u64_shl` are never drawn by a random stream; the last two are exercised by no test that builds without `--features touch-meter` (metered.rs is `#[cfg(feature = "touch-meter")]`, tests.rs:25-26). The gate runs `--all-features`, so nothing is unexercised in the gate; the inner-loop `just test` never invokes them. Separately, `merges_match_the_oracle` builds both operands with `Accumulator::new()` while `fresh` exists so "neither path's coverage goes vacuous", and every sibling merge test takes engine flags.

Evidence:

        50	        Op::Small(delta) => {
        51	            acc.add_small(*delta);
        52	            *oracle += *delta;
        53	        }
    ...
       467	        let mut receiver = Accumulator::new();
    ...
       472	        let mut operand = Accumulator::new();

Resolution: add an `Op::Word { negative, value: u64 }` arm (`add_u64`/`sub_u64`), route negative `Op::Small` through `sub_small`, add an `Op::WordShl` arm (`add_u64_shl`/`sub_u64_shl` at `0..200`), and give `merges_match_the_oracle` the two engine flags `fold_primitives_match_the_oracle` takes. Acceptance: every public write entry is reachable from `arb_op`, and the word-shift entries have a value-checking caller outside metered.rs.

### suanpan-tests-4: zero-valued wide operands are never drawn or witnessed, and they retire the register where the magnitude entries do not
- Where: crates/suanpan/src/accumulator/tests/differential.rs:109-121 (related: crates/suanpan/src/accumulator.rs:253-256, crates/suanpan/src/accumulator.rs:279-284, crates/suanpan/src/accumulator.rs:418-424, crates/suanpan/src/limbs.rs:31-38)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (strategy arms read; `add_wide` calls `self.spill()` unconditionally; `add_magnitude` dispatches `Some(0)` to `add_u64(0)`, which is a no-op under `if delta != 0`; `add_magnitude_shl` has `Some(0) => {}`; `Limbs` doc states a zero value has no limbs); executed: no
- Seen by: blind-spots [21]; refutation: confirmed (dashu's `as_words` documented empty for zero); history: no-rationale-found
- Owner-gated: yes: whether a value-neutral wide call should spill is a production behavior decision

`any::<u64>()` draws a zero limb with probability 2^−64, the cliffy arm forces `limbs[0] = 1` when every mask bit is clear, and `LimbsShl` pads zeros only at the high end, so no test feeds `add_wide`, `sub_wide`, `add_wide_shl`, `sub_wide_shl`, `add_limbs_shl`, or `sub_limbs_shl` a zero value. In production every wide entry spills before applying an empty limb stream, so `add_wide(&UBig::ZERO)` on a register-held value retires the register with no value change, while `add_magnitude(&UBig::ZERO)` and `add_magnitude_shl(&UBig::ZERO, s)` leave it untouched. Zero is an input to every wide entry ("correct at all scales, for all inputs"); the value is preserved either way, so the differential oracle cannot see the representation effect, and nothing pins it.

Evidence:

       111	                let mut limbs: Vec<u64> =
       112	                    mask.iter().map(|&saturated| if saturated { u64::MAX } else { 0 }).collect();
       113	                if limbs.iter().all(|&limb| limb == 0) {
       114	                    limbs[0] = 1;
       115	                }

    accumulator.rs:
       253	    pub fn add_wide(&mut self, delta: &UBig) {
       254	        self.spill();
       255	        self.apply_limbs(Limbs::new(delta), false, 0);
       256	    }
    ...
       420	            Some(0) => {}

Resolution: add a witness: register-held 5; `add_wide(&UBig::ZERO)`, `sub_wide_shl(&UBig::ZERO, 96)`, `add_limbs_shl(core::iter::empty(), 0)`; `assert_value` against 5 and pin the tier the owner intends (`acc.quick.is_some()` if the wide entries should short-circuit zero like `add_magnitude_shl`, the opposite if the spill is the contract). If short-circuiting is chosen, the wide entries gain an `if delta.is_zero() { return; }` before the spill (production; owner-gated). Acceptance: one committed test drives every wide entry with a zero operand on a register-held value and asserts both the value and the tier afterward.
Construction: `let mut acc = Accumulator::new(); acc.add_small(5); acc.add_wide(&UBig::ZERO); assert!(acc.quick.is_some());` fails today, because `add_wide` spills unconditionally and the zero's limb stream is empty.

### suanpan-tests-5: constructor and conversion idioms are spelled two ways for the same role
- Where: crates/suanpan/src/accumulator/tests/differential.rs:415-415 (related: crates/suanpan/src/accumulator/tests.rs:48, crates/suanpan/src/accumulator/tests/differential.rs:227, crates/suanpan/src/accumulator/tests/differential.rs:626, crates/suanpan/src/accumulator/tests/witnesses.rs:315)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep counts over the six files: `IBig::from(0)` 31 against `IBig::ZERO` 3; `UBig::from(1u8)` 39 against `UBig::ONE` 14; `usize::try_from(` 14 against `shift as usize` 6); executed: no
- Seen by: structure-prose [14]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The same role is spelled two ways, sometimes inside one function: the checked shift conversion beside the bare cast, the literal-zero constructor beside the associated constant. A reader pauses on each variant to ask whether the difference means something; for the shift conversion it does not, and the checked form is the one that carries the crate's own 32-bit discipline.

Evidence:

       415	            oracle += IBig::from(value << *shift as usize);
    ...
       626	            let scaled = IBig::from(value.clone()) << usize::try_from(*shift).unwrap();

Resolution: pick `IBig::ZERO`, `UBig::ONE`, and `usize::try_from(..).expect("shift fits usize")` (or a tiny `fn shift_usize(u64) -> usize` in tests.rs) and apply uniformly. Acceptance: one spelling per role across the six files.

### suanpan-tests-6: the stream-replay loop is copied at eleven sites and the run-forming arm bodies are copied across two files
- Where: crates/suanpan/src/accumulator/tests/differential.rs:467-476 (related: crates/suanpan/src/accumulator/tests/differential.rs:223-232, crates/suanpan/src/accumulator/tests/differential.rs:614-652, crates/suanpan/src/accumulator/tests/ledger.rs:241-296)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep `for op in` lists the loop at differential.rs:228, 469, 474, 496, 524, 529, 566, 591, 664, 684, 689, 804; the arm bodies at differential.rs:627-645 and ledger.rs:263-280 compared by reading); executed: no
- Seen by: structure-prose [5], api-economics [31 test half]; refutation: confirmed (with the note that the two run-forming properties differ in sign-read schedule: every step in differential.rs, only on arm 4 in ledger.rs); history: no-rationale-found (the ledger copy landed six hours after the differential one as a "complement" with no argument for two)
- Owner-gated: no

The block "fresh accumulator, zero oracle, `for op in ops { apply }`" is the `Held::Stream` arm of `build_held` and is repeated at the plain-replay sites (differential.rs:467-476, 522-531, 564-568, 662-666, 682-691, 802-806). The four run-forming match arms (`add_wide_shl`, `sub_wide_shl`, `sub_magnitude_shl`, `add_small` with oracle updates) are byte-identical between `run_forming_shift_streams_match_the_bigint_oracle` and `ledger_invariants_hold_on_run_forming_streams` apart from the ledger's added sign-read arm. The second duplication is the riskier one: a change to one file's arms silently desynchronizes the ledger sweep from the oracle sweep that shares its documented shape.

Evidence:

       467	        let mut receiver = Accumulator::new();
       468	        let mut receiver_oracle = IBig::from(0);
       469	        for op in &receiver_ops {
       470	            apply(&mut receiver, &mut receiver_oracle, op);
       471	        }
       472	        let mut operand = Accumulator::new();
       473	        let mut operand_oracle = IBig::from(0);
       474	        for op in &operand_ops {
       475	            apply(&mut operand, &mut operand_oracle, op);
       476	        }

Resolution: add `fn replay(ops: &[Op], engine: bool) -> (Accumulator, IBig)` to tests.rs (the body of `build_held`'s `Stream` arm) and use it at every plain-replay site. Lift the run-forming arm into tests.rs as `enum RunFormingOp` with `arb_run_forming_op()` and `apply_run_forming(acc, oracle, &op)`; either keep two properties drawing from that one definition, or fold the differential one into the ledger proptest with a `sign_every_step: bool` parameter so both sign-read schedules survive under the checker. Acceptance: one definition of the replay loop and one of the run-forming arms; both suites green; the per-step-sign schedule still runs.

### suanpan-tests-7: size_probe_covers_the_value asserts a full digit of slack its doc does not claim, and the doc's own bound is one bit too tight
- Where: crates/suanpan/src/accumulator/tests/differential.rs:487-505 (related: crates/suanpan/src/accumulator.rs:39, crates/suanpan/src/accumulator.rs:942-948, crates/suanpan/src/accumulator/tests/metered.rs:377-378, crates/suanpan/src/accumulator/tests/witnesses.rs:317-321)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (derivation against `LAZY_LIMIT` and `digit_count`: with `D = digit_count`, the value is at most Σ_{i<D} (2^33 − 1)·2^(32i) < (2 + 2^−31)·2^(32D), so `bit_len ≤ 32D + 2`, and the bound is reached: digits `[2^33 − 1, 2^33 − 1]` denote 2^65 + 2^32 − 1 with `bit_len` 66 = 32·2 + 2; the register arm gives `bit_len ≤ 32D`); executed: no
- Seen by: structure-prose [0]; refutation: confirmed (severity proposed medium to low because sibling pins catch the mutant); history: no-rationale-found (the `* 32 + 33` and the doc sentence arrived together in before's codec tests and were carried verbatim)
- Owner-gated: no

The doc claims each digit spans 32 bits plus one lazy-zone bit; read per digit that is 33D, read as an aggregate it is 32D + 1, and the tight bound is 32D + 2. The body asserts `32D + 33`, a whole digit looser than any reading. With that slack, the mutant `None => self.top` in `digit_count` (one digit short) satisfies `32·top + 33 ≥ bit_len` except when `bit_len = 32·top + 34`, a corner random streams do not reach, so this test admits the known-bad mechanism its sentence exists to exclude. `merge_tie_reads_the_operand` and `sign_collapse_tightens_the_top_and_arms_domination` pin `digit_count` exactly and would catch that mutant; the suite is not blind, but this test does not test its own claim, and a testdoc's incorrectness is a bug in the test. I keep medium over the refutation's low because both the assertion and the doc are wrong, in opposite directions, and the fix is one character plus one sentence.

Evidence:

       487	    /// `digit_count` covers the held width: after any stream, the value's
       488	    /// magnitude fits inside the counted digits (each digit spans 32 bits
       489	    /// plus one lazy-zone bit of overhang).
    ...
       499	            prop_assert!(
       500	                u64::try_from(acc.digit_count()).expect("digit counts fit u64") * 32 + 33
       501	                    >= magnitude.bit_len() as u64,
       502	                "digit_count misses value width"
       503	            );

Resolution: tighten to `* 32 + 2 >= bit_len` and restate the doc with the derivation: every digit is under 2^33 in magnitude, so the value is under (2 + 2^−31)·2^(32·digit_count), at most two bits past the counted width; a register value's `digit_count` is `ceil(bits/32)`, so the same bound holds there. Import `DIGIT_BITS` for the 32 if the owner prefers named constants in the model. Acceptance: with `+ 2`, `size_probe_covers_the_value` passes on the committed tree, and the mutant `None => self.top` in `Accumulator::digit_count` fails it by name.
Construction: in accumulator.rs:946 change `None => self.top + 1,` to `None => self.top,`; run `size_probe_covers_the_value` (expected: passes with the current `+ 33`); change the test to `+ 2` and re-run (expected: fails). Restore the production line.

### suanpan-tests-8: merge_into_wider hands back an ordinary Accumulator whose value is unspecified; three test sites carry the reset obligation by convention
- Where: crates/suanpan/src/accumulator/tests/differential.rs:695-698 (related: crates/suanpan/src/accumulator.rs:1146-1149, crates/suanpan/src/accumulator/tests/metered.rs:297, crates/suanpan/src/accumulator/tests/metered.rs:390-391, crates/before/src/version/skyline/watermark.rs:395)
- Class / severity / confidence: api-surprise / nit / medium
- Provenance: assessed (read); executed: no
- Seen by: api-economics [35]; refutation: confirmed (before resets every drained buffer through `retire`, which also takes live accumulators); history: no-rationale-found (the unspecified-value classification was deliberate; a typed obligation was never weighed)
- Owner-gated: yes: a public API reshaping of a stable crate

The rustdoc declares the returned buffer "a valid accumulator holding an unspecified value", and three test sites plus before's `retire` carry the reset by convention. A caller who forgets it reads the narrower operand's digits as a fresh total with no compile-time or runtime signal. Types-first: a `Drained` newtype whose only exit is `reset(self) -> Accumulator` makes the missing reset unrepresentable and deletes the doc sentence and the three comments.

Evidence:

       695	        // The pool contract: a drained buffer re-arms to a clean zero.
       696	        let mut reused = drained;
       697	        reused.reset();
       698	        assert_value(&reused, &IBig::from(0));

Resolution: owner's call: return a newtype that can only become an `Accumulator` by resetting (before's `retire` would need a second entry or a `From`, since it also takes live accumulators); the roster gains one trivially excluded row. Acceptance: a forgotten reset fails to compile; the drained-buffer prose in `merge_into_wider`'s rustdoc is gone.

### suanpan-tests-9: the ledger alphabet is a u8 matched against literals, with a hand-maintained cardinality and a catch-all arm
- Where: crates/suanpan/src/accumulator/tests/ledger.rs:110-111 (related: crates/suanpan/src/accumulator/tests/ledger.rs:126-172, crates/suanpan/src/accumulator/tests/ledger.rs:184-203, crates/suanpan/src/accumulator/tests/ledger.rs:212)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read: `const LEDGER_OPS: u8 = 11`, arms `0..=9` plus `_`, `for op in 0..LEDGER_OPS` at :191); executed: no
- Seen by: structure-prose [4], api-economics [33 LEDGER_OPS half]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`LEDGER_OPS` counts the arms of `ledger_op`'s `match` by hand. An arm added without a bump is never drawn; a bump without an arm falls into the `_` sign-read arm, doubling that op and inflating the sweep by about (12/11)^6 for no coverage; the catch-all hides both. Types-first: an `enum LedgerOp` with an exhaustive match makes the alphabet self-describing, the compiler catches a missing arm, the cardinality comes from an `ALL` array, and the shrunk `schedule` prints op names instead of numerals.

Evidence:

       110	/// Ops in the exhaustive ledger driver's alphabet.
       111	const LEDGER_OPS: u8 = 11;
    ...
       168	        _ => {
       169	            assert_eq!(acc.sign(), oracle_sign(oracle), "sign read");
       170	        }
    ...
       191	    for op in 0..LEDGER_OPS {

Resolution: introduce `#[derive(Clone, Copy, Debug)] enum LedgerOp { AddOne, SubOne, AddMax, SubMax, AddAt96, SubAt96, AddAt224, SubAt224, SubWord32At192, AddWord32At192, SignRead }`, a `const ALL: [LedgerOp; 11]` (or derive the list), an exhaustive `match`, and `schedule: Vec<LedgerOp>`. Acceptance: `ledger_op` has no `_` arm; `LEDGER_OPS` is gone; the sweep explores the same state count (check once in a scratch run that the number of `assert_ledger_invariants` calls is unchanged).

### suanpan-tests-10: the ledger checker never observes a state produced by the fold, merge, shift, reset, or negate entry points
- Where: crates/suanpan/src/accumulator/tests/ledger.rs:126-172 (related: crates/suanpan/src/accumulator/tests/ledger.rs:249-253, crates/suanpan/src/accumulator/tests/differential.rs:461-485, crates/suanpan/src/accumulator/tests/differential.rs:657-670, crates/suanpan/src/accumulator/tests/differential.rs:676-699, crates/suanpan/src/accumulator.rs:536-575, crates/suanpan/src/accumulator.rs:626-627, crates/suanpan/src/accumulator.rs:666-679)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep: `assert_ledger_invariants` is defined at ledger.rs:31 and called only at :196 and :290; neither alphabet names `add_accum`, `sub_accum`, `*_accum_shl`, `merge_into_wider`, `shl`, `negate`, or `reset`; `read_digits` consumes no certificate); executed: no
- Seen by: blind-spots [17], api-economics [30]; refutation: reframed (the multi-certificate geometry is not a distinct code path: `fold_accum` routes each nonzero digit through `add_at` with `top` settled between calls, and the exhaustive alphabet's jumps to digits 3 and 7 already create and cancel shared-endpoint certificates under the checker; what the checker never sees is the state those entry points leave); history: no-rationale-found (the alphabet's inline rationale covers the transitions it was built to probe)
- Owner-gated: no

`ledger.rs:3-9` declares the certificates load-bearing for correctness ("a stale certificate over a written digit would corrupt values, not just costs") and the checker is the only structural instrument for them. Both drivers' alphabets consist of digit-0 deltas, shifted one-limb writes, raw word deposits, and sign reads. `shl` on the engine rebuilds the ledger from scratch (`core::mem::take(self)` then `add_accum_shl`), `reset` clears it, and the fold entries deposit through `add_at`; the differential tests that drive those entry points end at `assert_value`, whose read-outs consume no certificate, so a stale or stranded certificate left by any of them would surface only when a later scan happened to consume it. "Not wrong, but you couldn't tell if it were" is a repairable blind spot, and these are exactly the paths a future optimization (an in-place digit shift, a lazier reset) would touch.

Evidence:

       250	        ops in proptest::collection::vec(
       251	            (0u8..5, proptest::collection::vec(any::<u64>(), 1..=2), 0u64..4_096),
       252	            1..150,
       253	        ),

    accumulator.rs:
       626	        let held = core::mem::take(self);
       627	        self.add_accum_shl(&held, shift);

Resolution: cheapest: call `assert_ledger_invariants(&acc, &[])` at the end of `merges_match_the_oracle`, `in_place_shift_matches_the_oracle`, `width_ordered_merges_match_the_oracle`, and `fold_primitives_match_the_oracle` (it is a private-state checker in the same test tree). Better: add arms to `ledger_invariants_hold_on_run_forming_streams` for `negate`, `shl(shift)`, `reset` (then continue the stream, so post-reset spills over the retained buffer are checked), and `add_accum_shl(&snapshot, shift)` where the snapshot is a clone taken earlier with its oracle tracked. Acceptance: the checker runs on a state after each of `add_accum_shl`, `sub_accum_shl`, `merge_into_wider`, `shl`, `negate`, and `reset`; the construction below fails the suite.
Construction: replace the engine branch of `shl` (accumulator.rs:626-627) with an in-place digit shift that moves the digits up by `digit_shift`, touches twice per digit (so the exact 2d pin at metered.rs:443-450 still holds), and leaves `zero_runs` un-reindexed. Every committed test passes (values are correct and no ledger driver calls `shl`); a certificate `(lo, hi)` now covers position `lo + 1`, which holds the old nonzero digit from `lo`, so the proposed `shl` arm fails the soundness clause at the first shifted state, and a subsequent zero-partial fold would skip over a nonzero digit.

### suanpan-tests-11: the exhaustive ledger testdoc carries hand-maintained tallies, a wall-time figure, a dated anecdote, and ragged wrapping
- Where: crates/suanpan/src/accumulator/tests/ledger.rs:212-216 (related: crates/suanpan/src/accumulator/tests/ledger.rs:174-179)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (the tally Σ_{k=1..6} 11^k = 1,948,716 checked by hand; git blame attributes the text to 023eff6e, which moved it from the introducing commit 5f16e2e5c whose message carries the same numbers); executed: no
- Seen by: structure-prose [3], blind-spots [25]; refutation: confirmed (and noted the raggedness at :214-216); history: no-rationale-found (the dated-notes excision d2a9d04e matched only calendar dates, so these lines survived by pattern, not by ruling)
- Owner-gated: no

The doc restates `LEDGER_OPS` and `LEDGER_DEPTH` as "11-op" and "≤ 6", tallies the state count, quotes a dev-profile wall time, and records that a depth-7 sweep "also passed once, at pin time". All four rot silently when either constant moves; the timing is machine-dependent narration; "at pin time" is history whose evidence is a commit. Prose speaks in the present tense, with no hand-maintained counts. The structure is already stated at `LEDGER_DEPTH`'s own doc (:176-178). Lines 214-216 also wrap with "recentering" alone on a line, the same excision residue as suanpan-tests-13.

Evidence:

       212	/// Exhaustive over all 11-op schedules of length ≤ 6 (1,948,716
       213	/// states, each checked once; ~4 s dev — the length-≤ 7 sweep's
       214	/// 21.4M states also passed once, at pin time): word-scale deltas,
       215	/// recentering
       216	/// `u64::MAX` deltas, one-limb jumps to digits 3 and 7 in both

Resolution: rewrite as "Exhaustive over every schedule of at most `LEDGER_DEPTH` ops drawn from the alphabet, each state checked once: word-scale deltas, recentering `u64::MAX` deltas, ..." and drop the state count, the seconds, and the depth-7 sentence (git history holds them; a depth bump is a deliberate commit that can cite its own run). Acceptance: no numeral in the doc duplicates a constant, no sentence reports a past run, and the paragraph reflows without a one-word line.

### suanpan-tests-12: metered.rs does not state the process-per-test premise its exact pins rest on
- Where: crates/suanpan/src/accumulator/tests/metered.rs:1-11 (related: crates/suanpan/src/touch_meter.rs:16-20, justfile:108-114)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (touch_meter.rs:24 is one process-global relaxed `AtomicU64`; :16-20 states the serial requirement; justfile:109 and :114 run `cargo nextest run --workspace [--all-features]`; metered.rs read in full, no serialization primitive); executed: no
- Seen by: blind-spots [24]; refutation: confirmed, severity low to nit (the premise is documented at the counter and the runner of record is nextest); history: deliberate-and-holds for the counter-side doc; the pointer from the pins' file was never ruled on
- Owner-gated: no

The pins are exact only because nextest runs each test in its own process. Under `cargo test -p suanpan --features touch-meter`, the differential and ledger tests in the same binary drive `touch()` concurrently with a pin's reset/read window and the exact assertions fail nondeterministically. The requirement lives at `touch_meter.rs`, not where a developer reproducing a pin will look, and the exactness contract (lib.rs:283-286: "never measurement noise") is true only under a runner this file never names.

Evidence:

         1	//! The touch-metered cost pins: exact digit-touch totals for the
         2	//! claims roster's cost rows, at their canonical shapes.

    touch_meter.rs:
        16	//! of this crate, never measurement noise. Because the counter is
        17	//! process-global with relaxed ordering, readings are meaningful only
        18	//! when metered scenarios run serially — [`reset`] between them, read
        19	//! after the metered call returns; a default-parallel test runner
        20	//! interleaves scenarios into one count.

Resolution: one sentence in the module doc: the counter is process-global, so these pins hold under nextest's process-per-test execution (the gate's `test-all`) and are not meaningful under a threaded `cargo test` of the same binary. Acceptance: the premise is stated where the pins are.

### suanpan-tests-13: three inequality pins and six single-shape pins in a module whose doc says every pin is exact and doubled
- Where: crates/suanpan/src/accumulator/tests/metered.rs:43-54 (related: crates/suanpan/src/accumulator/tests/metered.rs:4-8, crates/suanpan/src/accumulator/tests/metered.rs:20-23, crates/suanpan/src/accumulator/tests/metered.rs:788-795, crates/suanpan/src/accumulator/tests/metered.rs:816-823, crates/suanpan/src/lib.rs:283-286)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (module doc and the three inequalities read; exact values by trace against accumulator.rs: `[7, 9]` at shift 32_000 deposits digits 1000 and 1002, `sign_magnitude_shl` reads from `bottom = 1000` through `top = 1002` for 3 touches with carry 0; the full read walks 0..=1002 for 1003, which the sibling pin at :818-820 already asserts exactly; the `low_top` leg reads digit 4, zeroes it, reads digit 3 to partial 2^33, zeroes the floor, and `add_at(3, 2^33)` recenters in two steps to land 2 back at index 4, for 6); executed: no
- Seen by: structure-prose [2], blind-spots [20], api-economics [32]; refutation: confirmed; history: deliberate-but-expired (`<= 16` and `> 1000` predate the exact-pin census and the module doc, which neither revisited; `> 1` postdates the doc as a deliberate shape bound; the "each pin repeats across a doubling" clause overclaimed on the day it was written)
- Owner-gated: no

The module doc states "Every pin asserts an exact count, not a ceiling" and "each pin repeats its schedule across a doubling". `scaled_read_costs_the_written_span` asserts `<= 16` (exact: 3) and `> 1000` (exact: 1003), and `decision_bound_top_decides_on_the_first_touch` asserts `> 1` (exact: 6); the `<= 16` ceiling is the artifact the doc's own liveness argument says not to commit (a dark counter reads 0 ≤ 16, and a hidden second pass over the span reads 6 ≤ 16). Six pins run one shape with no doubling (`scaled_read_costs_the_written_span`, `top_settlement_steps_are_metered`, `sign_fold_skips_certified_runs`, `domination_reads_cost_one_touch_after_the_first`, `decision_bound_top_decides_on_the_first_touch`, `scaled_read_costs_the_span_not_the_write_count`). The crate declares touch counts an exact public contract, and the exact values are derivable. The 6-touch value also exposes a fact worth pinning: a top digit of magnitude 2 over a zero digit is a collapse fixed point (the re-deposit recenters back to the same spelling), so every later read repeats the 6-touch descent rather than costing one touch.

Evidence:

         4	//! Every pin asserts an exact count, not a ceiling: exactness is the
         5	//! liveness floor (a counter that silently stops counting cannot
         6	//! satisfy an exact total) and the flatness witness at once — each pin
         7	//! repeats its schedule across a doubling of the axis its row claims
         8	//! independence from. The adequacy tripwire
    ...
        43	    assert!(
        44	        scaled_read <= 16,
    ...
        50	    assert!(
        51	        touch_meter::touches() > 1000,
    ...
       792	    assert!(
       793	        touch_meter::touches() > 1,

Resolution: pin `scaled_read == 3`, the full read `== 1003`, and the below-bound descent `== 6` (measure the last once before committing; my 6 is by reading), each with its derivation in the message as the sibling pins do, and replace "O(1)-ish" at :22 with the number. Narrow the module doc's doubling clause to the pins that double, or add the doubling where it is cheap (a second parked digit index for the scaled reads, a second prefix width for the settlement scan). Acceptance: every assertion on `touch_meter::touches()` in metered.rs is `assert_eq!` against a derived constant, and the module doc's two sentences are true of every pin in the file.
Construction: insert a second `for &digit in &self.digits[start..=self.top] { touch(1); }` loop into `read_digits`: `scaled_read` becomes 6, still `<= 16`; the full read becomes 2006, still `> 1000`; every exact pin elsewhere fails, so the gap is specific to these assertions.

### suanpan-tests-14: two doc fragments left by the dated-note excision: "Green pin: an" and a verbless "Measured (...)"
- Where: crates/suanpan/src/accumulator/tests/metered.rs:69-71 (related: crates/suanpan/src/accumulator/tests/metered.rs:495-497, crates/suanpan/src/accumulator/tests/ledger.rs:214-216)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`git show d2a9d04e -- crates/suanpan/src/accumulator/tests.rs` shows `-/// Green pin (cure of adversarial review 2026-07-28, task #37 F3): an` becoming `+/// Green pin: an` and `Measured 2026-07-29` becoming `Measured`; 023eff6e moved the lines unchanged); executed: no
- Seen by: structure-prose [7]; refutation: confirmed; history: no-rationale-found, with the provenance corrected from the split to the excision
- Owner-gated: no

The first doc line is a label plus a fragment: "Green pin:" names nothing in this file (the module doc speaks of one tripwire reading red, not of green pins), and the line wrap shows where a clause was cut. The second ends a sentence with "alike. Measured" followed by a parenthetical with no verb, hiding a real fact (word-pairing in `Limbs` makes counts target-independent) behind a dangling fragment. A doc comment's first sentence stands alone in a listing.

Evidence:

        69	/// Green pin: an
        70	/// alternating shifted pair costs its operand, not the zero run under
        71	/// it — exact totals, identical across a shift doubling.
    ...
       495	/// at `k = 4096` and `k = 8192` alike. Measured
       496	/// (deterministic counter; word-pairing keeps counts
       497	/// target-independent).

Resolution: "An alternating shifted pair costs its operand, not the zero run under it: exact totals, identical across a shift doubling." and "The counts are target-independent: `Limbs` pairs 32-bit backend words into 64-bit limbs, so a wasm32 build reads the same totals." Reflow ledger.rs:214-216 while there. Acceptance: both docs read as complete sentences; no one-word wrapped line remains.

### suanpan-tests-15: alternating_shifted_writes repeats one loop body four times, and the word-magnitude path is the one never doubled
- Where: crates/suanpan/src/accumulator/tests/metered.rs:92-183 (related: crates/suanpan/src/accumulator/tests/metered.rs:674-717, crates/suanpan/src/claims.rs:205-232)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (three `for shift in [32_000u64, 64_000]` blocks at :94-164 and a single-shift block at :165-182; claims.rs:205-232 cites this test for `add_magnitude_shl`/`sub_magnitude_shl` "at any shift"); executed: no
- Seen by: structure-prose [6]; refutation: confirmed; history: deliberate-but-expired (the block was born as a red pin whose number encoded the shift, so one shift sufficed; the cure kept the shape)
- Owner-gated: no

The test repeats "build, reset, 1_000 × (sub, add), `assert_eq!(touches, N)`, assert value" for the wide, word-shift, and occupied-digit-0 scenarios across two shifts, then runs the `*_magnitude_shl` scenario at one shift only. The roster cites this test as the shift-axis evidence for the magnitude-shift rows, and the doubling is what makes "whatever the shift" a measurement rather than a point; the word-magnitude path is the one row without it. The same file already table-drives `magnitude_dispatch_costs_its_width_path` with `&dyn Fn` closures.

Evidence:

        94	    for shift in [32_000u64, 64_000] {
        95	        let mut acc = Accumulator::new();
        96	        acc.add_wide_shl(&one, shift);
        97	        touch_meter::reset();
        98	        for _ in 0..1_000 {
        99	            acc.sub_wide_shl(&one, shift);
       100	            acc.add_wide_shl(&one, shift);
       101	        }
    ...
       166	    let five = UBig::from(5u8);
       167	    let mut acc = Accumulator::new();
       168	    acc.add_magnitude_shl(&five, 32_000);
       169	    touch_meter::reset();
       170	    for _ in 0..1_000 {
       171	        acc.sub_magnitude_shl(&five, 32_000);
       172	        acc.add_magnitude_shl(&five, 32_000);
       173	    }

Resolution: table-drive the four scenarios over `[32_000, 64_000]` with an inline tuple of setup/sub/add closures, expected per-pair cost, and label (as the dispatch test does, with `#[allow(clippy::type_complexity)]`); expected totals stay 5_000 / 3_000 / 5_000 / 3_000. Acceptance: one loop body; the magnitude word path is asserted at both shifts; green.

### suanpan-tests-16: merge_into_wider's swap, the min in its cost row, has no touch pin in the direction that exercises it
- Where: crates/suanpan/src/accumulator/tests/metered.rs:287-296 (related: crates/suanpan/src/accumulator/tests/metered.rs:362-393, crates/suanpan/src/accumulator/tests/differential.rs:672-699, crates/suanpan/src/accumulator.rs:1169-1175, crates/suanpan/src/claims.rs:271-275, crates/suanpan/src/lib.rs:219)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep: `merge_into_wider` is called in metered.rs only at :291 and :380; at :262-291 the receiver holds 64 or 128 digits against a 2-digit operand, at :372-380 the digit counts tie, so `other.digit_count() > self.digit_count()` at accumulator.rs:1170 is never true on a metered path; `width_ordered_merges_match_the_oracle` checks values only, and the sum is the same either way); executed: no
- Seen by: blind-spots [18]; refutation: confirmed (deleting the swap keeps every committed test green; the rustdoc example takes the swap branch but checks only the value; cargo-mutants' operator mutants of the `>` are killed by the existing pins, but "no swap" is not a campaign mutant); history: no-rationale-found (the census pinned the row with the receiver always wider; the tie commit deliberately pinned only the tie clause)
- Owner-gated: no

The cost row is `amortized O(min(|self|, |other|))` and the roster cites this test as its sole witness. The row's distinctive content is the min, the min is implemented by the swap, and the swap is on no metered path. The known-bad mechanism (no swap, always read `other`) passes every committed test because the operand is never the wider one where touches are counted. Every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

       287	        let mut receiver = Accumulator::new();
       288	        receiver.add_wide(&wide_value);
       289	        let mut spare = operand;
       290	        touch_meter::reset();
       291	        spare = receiver.merge_into_wider(spare);
       292	        assert_eq!(
       293	            touch_meter::touches(),
       294	            4,
       295	            "merge_into_wider reads the narrower operand only"
       296	        );

    accumulator.rs:
      1170	        if other.digit_count() > self.digit_count() {
      1171	            core::mem::swap(self, &mut other);
      1172	        }

Resolution: add the mirrored leg inside the same `held_bits` loop: `receiver = narrow()` (2 digits), operand holding `(1 << held_bits) - 1` (64 then 128 digits); `touch_meter::reset(); let spare = receiver.merge_into_wider(operand); assert_eq!(touches, 4)`; assert the sum landed in `receiver`; state in the doc that without the swap this leg reads `2 · held_digits` (one read per operand digit plus one deposit each, no carries since every digit stays under 2^33). Acceptance: a metered leg in which `other.digit_count() > self.digit_count()` pins 4 touches at two receiver widths; a build with the swap removed fails that leg by name.
Construction: `let mut receiver = narrow(); let mut operand = Accumulator::new(); operand.add_wide(&((UBig::from(1u8) << 2_048usize) - 1u8)); touch_meter::reset(); let spare = receiver.merge_into_wider(operand); assert_eq!(touch_meter::touches(), 4);` With lines 1170-1172 deleted, `fold_accum` iterates 64 digits: 64 reads + 64 deposits = 128.

### suanpan-tests-17: moralized and register-transplant vocabulary: "honest" (5), "real fold", "minted", and "tripwire" for criteria with no committed known-bad
- Where: crates/suanpan/src/accumulator/tests/metered.rs:337-337 (related: crates/suanpan/src/accumulator/tests/metered.rs:678-679, crates/suanpan/src/accumulator/tests/metered.rs:804, crates/suanpan/src/accumulator/tests/metered.rs:835, crates/suanpan/src/accumulator/tests/witnesses.rs:173, crates/suanpan/src/accumulator/tests/witnesses.rs:327, crates/suanpan/src/accumulator/tests/witnesses.rs:394, crates/suanpan/src/accumulator/tests/witnesses.rs:411, crates/suanpan/src/accumulator/tests/witnesses.rs:415, crates/suanpan/tests/amortized_sequences.rs:22, crates/suanpan/tests/amortized_sequences.rs:51)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep over the six files; each hit read in context; the uses at tests.rs:11, metered.rs:8, and metered.rs:831 attach "tripwire" to the committed known-bad mechanism and are fine); executed: no
- Seen by: structure-prose [8]; refutation: confirmed; history: no-rationale-found, noting the words are workspace-wide idiom (production `accumulator.rs:992` uses "honest spelling"; `before/src` has 159 "honest" and 75 "tripwire" hits)
- Owner-gated: no

Each site has a plain mechanism word available: "honest" moralizes code that is merely correct; "real fold" contrasts with nothing; "minted" is the one word the owner's vocabulary rules name; "tripwire" is established jargon only where a committed known-bad artifact fails the check (metered.rs's no-collapse model earns it; the mixed-second-difference bound and the deterministic point witnesses do not). Because the pattern is workspace-wide, a partition-local edit would create inconsistency; this list is the suanpan input to a workspace prose pass.

Evidence:

       337	        // difference is negative: the honest value is operand-first.
    ...
       678	    // The inline tuple type is the documentation; a minted alias would
       679	    // only add a name to track.
    ...
       835	/// The known-bad mechanism is committed here as a real fold over the

    amortized_sequences.rs:
        22	//! The tripwire is the mixed second difference over a 2×2 (n, d) grid:

Resolution: metered.rs:337 "the difference is operand − receiver"; :678-679 drop the comment (the `#[allow]` says why) or "a named alias would only add a name to track"; :804 "the denominator"; :835 "as a fold over the digits"; witnesses.rs:327 "no comparand at the cancelled value's true scale"; :394 "one spelling of the unchanged value"; :411 "the mode asserts pin which tier the witness runs in"; :173 and :415 "deterministic pin"; amortized_sequences.rs:22 and :51 "the criterion" / "the no-product bound". Acceptance: `grep -n -i 'honest\|minted\|real fold'` over the partition returns nothing; "tripwire" survives only where a committed known-bad mechanism fails the check.

### suanpan-tests-18: negative read-out cost (the complement pass) is unpinned; the held-width row is metered only on the positive spelling
- Where: crates/suanpan/src/accumulator/tests/metered.rs:413-430 (related: crates/suanpan/src/accumulator.rs:1090-1107)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (the test negates, pins `negate` at d, negates back, then meters `sign_magnitude` and `sign_limbs` on the positive value; `read_digits`' negative arm runs a second per-digit loop with `touch(1)` when the low part is nonzero; by trace of `−(2^bits − 1)`, digit 0 leaves low 1 and carry −1 and every higher digit leaves low 0 and carry −1, so the complement pass runs d touches and pushes no high digit: 2d total); executed: no
- Seen by: blind-spots [23]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The row is `O(|self|)` and 2d satisfies it, so no claim is violated; but the pin's message ("one carry pass over the span") describes only half the read-out, and the width doubling that makes the pin a linearity witness is absent for the arm with the extra pass. A cost regression confined to the complement arm (an extra pass, a per-digit rescan of `collected`) passes every committed test.

Evidence:

       420	        acc.negate();
       421	
       422	        touch_meter::reset();
       423	        let (sign, magnitude) = acc.sign_magnitude();
       424	        assert_eq!(
       425	            touch_meter::touches(),
       426	            held_digits,
       427	            "sign_magnitude at {held_digits} held digits: one carry pass \
       428	             over the span"
       429	        );

    accumulator.rs:
      1097	                for digit in collected.iter_mut() {
      1098	                    touch(1);

Resolution: after the positive legs, negate once more and meter `sign_magnitude` and `sign_limbs` on the negative spelling, pinning `2 * held_digits` at both widths with the derivation (carry pass plus complement pass; the complement of a nonzero low part never carries out, so no high digit is pushed). Acceptance: an exact pin for both read-outs on a negative d-digit value at two widths.
Construction: `acc.negate(); touch_meter::reset(); let _ = acc.sign_magnitude(); assert_eq!(touch_meter::touches(), 2 * held_digits);` (the 2d is my derivation; measure once before committing).

### suanpan-tests-19: em-dashes in two assert messages and four code comments
- Where: crates/suanpan/src/accumulator/tests/metered.rs:599-600 (related: crates/suanpan/tests/amortized_sequences.rs:58, crates/suanpan/src/accumulator/tests/differential.rs:243, crates/suanpan/src/accumulator/tests/witnesses.rs:134, crates/suanpan/src/accumulator/tests/witnesses.rs:184, crates/suanpan/src/accumulator/tests/witnesses.rs:641)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for the em-dash over the six files, filtered to lines that are not `///` or `//!`: exactly the six sites); executed: no
- Seen by: structure-prose [9]; refutation: confirmed; history: no-rationale-found, noting the pattern is workspace-wide (production `accumulator.rs` uses em-dashes in `//` comments; `before/tests/meter.rs` has 398)
- Owner-gated: no

Owner doctrine prefers colons or semicolons over em-dashes in log messages and comments (terminal compatibility and sentence flow); assert messages are log output. As with suanpan-tests-17, the convention is workspace-wide and belongs to a prose pass; these are the partition's sites.

Evidence:

       599	        "a padded [5, 0, 0] stream: 3 yielded-limb reads + 1 deposit — \
       600	         zero limbs are value-neutral but pay their touch"

Resolution: replace with a colon or semicolon at the six sites. Acceptance: the em-dash appears in the partition only on `///` and `//!` lines.

### suanpan-tests-20: domination_reads' doc names sign_dominates_at for a loop that calls sign_dominates_word, and overclaims one touch per later read
- Where: crates/suanpan/src/accumulator/tests/metered.rs:719-726 (related: crates/suanpan/src/accumulator/tests/metered.rs:742-750, crates/suanpan/src/claims.rs:294-303, crates/suanpan/src/claims/tests.rs:139-161, crates/suanpan/src/accumulator.rs:844-886)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (the loop at :743-744 calls `sign_dominates_word()`; claims.rs:294-303 cites this one test for both operations, and `reaches` requires a cited witness's body to invoke the operation, so the entry point is load-bearing; the fixed-point trace is in suanpan-tests-13); executed: no
- Seen by: structure-prose [10]; refutation: confirmed; history: deliberate-and-holds for the loop's entry point (the roster's reach test needs it), converting to "state it inline"; the general one-touch sentence stands as an overclaim
- Owner-gated: no

The doc reads as if `sign_dominates_at(3)` is repeated a thousand times; the loop calls `sign_dominates_word` deliberately, because the roster cites this test for both operations and the reach test needs both invocations, but the doc does not say so. The opening sentence's "every later read is a single touch" holds for a decided top; a top digit of magnitude 2 over a zero digit is a collapse fixed point that repeats a 6-touch descent on every read (still O(1), so no claim breaks, but the sentence is stronger than the mechanism supports).

Evidence:

       719	/// Domination certificates are amortized O(1): the first read may
       720	/// collapse, every later read is a single touch, and the certificate
       721	/// stays decided.
    ...
       744	        assert_eq!(acc.sign_dominates_word(), (Ordering::Greater, true));

Resolution: state the shape: after the first collapse a decided top answers every later read in one touch; the thousand reads go through `sign_dominates_word`, the second operation this pin evidences for the roster. Acceptance: the doc names both entry points and scopes the one-touch claim to decided tops.

### suanpan-tests-21: the known-bad fold model hardcodes the decision threshold 3 where SIGN_DECIDED is importable
- Where: crates/suanpan/src/accumulator/tests/metered.rs:855-855 (related: crates/suanpan/src/accumulator.rs:50, crates/suanpan/src/accumulator/tests/ledger.rs:19, crates/suanpan/src/accumulator/tests/witnesses.rs:16)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`const SIGN_DECIDED: i128 = 3;` at accumulator.rs:50; the sibling suites import `LAZY_LIMIT`, `QUICK_MAX`, `QUICK_SHIFT_MAX` the same way); executed: no
- Seen by: api-economics [33 unique part]; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The model is documented as "the production decision rule, minus the collapse"; a literal `3` can diverge from the rule it claims to model if `SIGN_DECIDED` moves. Named constants over magic numbers.

Evidence:

       855	            if partial.abs() >= 3 || index == 0 {

    accumulator.rs:
        50	const SIGN_DECIDED: i128 = 3;

Resolution: `use crate::accumulator::SIGN_DECIDED;` and `partial.abs() >= SIGN_DECIDED`. Acceptance: changing `SIGN_DECIDED` changes the model.

### suanpan-tests-22: witness docs narrate the mutation history in the past tense, three times
- Where: crates/suanpan/src/accumulator/tests/witnesses.rs:6-9 (related: crates/suanpan/src/accumulator/tests/witnesses.rs:28-33, crates/suanpan/src/accumulator/tests/witnesses.rs:75-79)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read: "passed everything else", "survived the whole differential suite", "passed the whole differential suite", "passed the entire committed test suite"); executed: no
- Seen by: structure-prose [16]; refutation: confirmed; history: no-rationale-found (the dated-note excision removed the dates but its charter was calendar dates, so the tense was outside its scope)
- Owner-gated: no

The adequacy fact is present-tense and re-checkable: the mutation passes every other committed test and this witness kills it. That is exactly the provenance a future reader needs to judge whether a corner can be retired, so the fix is tense and deduplication, not deletion.

Evidence:

         6	//! The extreme-cancellation witnesses exist because their mutations
         7	//! passed everything else: `SIGN_DECIDED: 3 → 2` and a decision index
         8	//! of `floor + 1` each survived the whole differential suite, so the
         9	//! tight corners are pinned here by construction.

Resolution: module doc: "each witness pins a corner whose mutation passes every other committed test". Test docs (:28-33, :75-79): "The mutation `SIGN_DECIDED: 3 → 2` reads this value as positive and passes every other committed test; this pin is what kills it." Acceptance: no past-tense narration of a run; each adequacy statement appears once, at the witness it describes.

### suanpan-tests-23: the executable headroom derivation does not check what its expect messages say: checked_shl never detects bits shifted out
- Where: crates/suanpan/src/accumulator/tests/witnesses.rs:494-500 (related: crates/suanpan/src/accumulator.rs:52-65, crates/suanpan/src/accumulator/tests/witnesses.rs:525-546, rust-toolchain.toml)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (the pinned 1.97.1 core source, `library/core/src/num/int_macros.rs:1397-1398`: "Checked shift left. Computes `self << rhs`, returning `None` if `rhs` is larger than or equal to the number of bits in `self`", with the doc example `0x10.checked_shl(BITS_MINUS_ONE) == Some(0)`; the overflow consequence assessed by reading); executed: no
- Seen by: structure-prose [15]; refutation: reframed from "move to a const assert" to "the check is not a check"; history: no-rationale-found
- Owner-gated: no

`ceiling.checked_shl(QUICK_SHIFT_MAX as u32).expect("the widest shifted fold fits i128")` returns `None` only for a shift amount of 128 or more, so it passes for any `QUICK_SHIFT_MAX` under 128, including 31, where `2^96 << 31 = 2^127` overflows `i128` and wraps to `i128::MIN`. The three lines therefore claim a check they do not perform. The headroom is in fact pinned by the value-level legs that follow (a wrapped `operand_value << shift` in `fold_accum` panics in debug and mismatches the oracle in release), so the derivation is decorative and its messages are inaccurate.

Evidence:

       494	    let ceiling = i128::try_from(QUICK_MAX).expect("the ceiling fits i128");
       495	    let widest_fold = ceiling
       496	        .checked_shl(QUICK_SHIFT_MAX as u32)
       497	        .expect("the widest shifted fold fits i128");
       498	    ceiling
       499	        .checked_add(widest_fold)
       500	        .expect("the worst register sum fits i128");

Resolution: replace the three lines with a compile-time assertion beside the constants in accumulator.rs using overflow-detecting arithmetic, e.g. `const _: () = assert!((QUICK_MAX << QUICK_SHIFT_MAX) + QUICK_MAX <= i128::MAX as u128);` in `u128` (where a shift past 127 is itself a compile error), with the headroom argument restated at the constants (accumulator.rs:63-65 already states it in prose); keep the value-level extremes, which are the test's real content. Acceptance: the build fails if `QUICK_SHIFT_MAX` is raised past what `i128` admits; the witness body starts at the value-level extremes.
Construction: set `QUICK_SHIFT_MAX` to 31 in a scratch build: lines 494-500 pass (`checked_shl(31)` returns `Some(i128::MIN)`), and the value legs at :525-546 fail in debug (overflow panic in `fold_accum`'s shift) or by oracle mismatch in release. Restore the constant.

### suanpan-tests-24: amortized_sequences.rs housekeeping: an orphaned "S1" label, qualified Ordering, f64 over exact counters, an unnamed 0.10, and a cross-crate copy of the helper
- Where: crates/suanpan/tests/amortized_sequences.rs:8-8 (related: crates/suanpan/tests/amortized_sequences.rs:37, crates/suanpan/tests/amortized_sequences.rs:44-46, crates/suanpan/tests/amortized_sequences.rs:51-61, crates/before/tests/answer_embedded.rs:109-121)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`S1(` occurs only at :8 across crates/suanpan, crates/before/tests, .agent-notes, and the justfile; `std::cmp::Ordering::Greater`/`Less` at :44 and :46 with no import; `f64` arithmetic and the literal `0.10` at :53-54; answer_embedded.rs:111-121 is the same function modulo label parameters and the message noun); executed: no
- Seen by: structure-prose [11], blind-spots [27]; refutation: confirmed; history: no-rationale-found (adopted wholesale from an audit branch that is not an ancestor of main; the audit itself named only S1, so no roster ever existed)
- Owner-gated: no

Four small idiom issues in one 78-line file: the helper `s1` and the doc's `S1(n, d)` number a family of one; `Ordering` is spelled out where the sibling files import it; the mixed second difference over exact `u64` counters is computed in `f64` against an unnamed `0.10` where `i128` arithmetic and a named constant are exact; and `assert_no_product` duplicates before's `answer_embedded.rs` helper, two copies of one criterion that drift independently. If suanpan-tests-25 pins the schedule exactly, the `f64` and the bound dissolve here.

Evidence:

         8	//! **Wide sign-flip oscillation** `S1(n, d)`: hold `-1` at digit 0,
    ...
        44	            assert_eq!(acc.sign(), std::cmp::Ordering::Greater);
    ...
        53	    let mixed = grid[3] as f64 - grid[2] as f64 - grid[1] as f64 + grid[0] as f64;
        54	    let bound = 0.10 * grid[3] as f64;

Resolution: `use core::cmp::Ordering;`; rename `s1` to `sign_flip_oscillation` and drop the `S1(n, d)` label; if the grid check survives suanpan-tests-25, compute `mixed` in `i128` and name the bound with its rationale beside it. The shared-helper question is in the open questions. Acceptance: no qualified `std::cmp::Ordering`; no `S1`; no `f64` unless the bound survives with a named constant.

### suanpan-tests-25: assert_no_product has no liveness floor: a dark counter passes it, and an exact shift-independent total is measured and available
- Where: crates/suanpan/tests/amortized_sequences.rs:52-61 (related: crates/suanpan/tests/amortized_sequences.rs:36-49, crates/before/tests/meter.rs:6493-6500, crates/suanpan/src/accumulator/tests/metered.rs:4-6, crates/suanpan/src/touch_meter.rs:12-16)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (arithmetic by reading: with `grid = [0, 0, 0, 0]`, `mixed = 0`, `bound = 0`, and `0 <= 0` holds; the exact total is corroborated by the refutation pass's single executed run, whose log I read at `scratchpad/before/refute-suanpan-tests/sign_flip_run.log`: `MEASURED s1_sign_flip: grid [32770, 65538, 32770, 65538] mixed 0 bound 6554`, i.e. `16n + 2` at `d = 32_768` and `65_536`; my per-round trace agrees: add 2, sign 5, sub 2, sign 7, plus 2 for the first-round spill of the parked −1); executed: no (by me; the refutation pass ran the test once)
- Seen by: blind-spots [19], api-economics [28]; refutation: confirmed and executed; history: no-rationale-found (adopted in the same shape as its before twin, which has no floor either; the sibling `accum_*` bands do carry one)
- Owner-gated: no

The criterion is a relative bound over a deterministic counter: if `touch_meter::record` stopped counting, or counted nothing for sign reads, every cell reads 0 and the assertion holds; a counter that undercounts uniformly also passes. Meters need liveness floors derived from irreducible work so a ceiling cannot pass vacuously when the counter goes dark. The sibling band harness in before asserts `run.touches >= run.ops` for exactly this reason (meter.rs:6493-6500); this file, the only committed instrument that flips the sign under load, is the one place that floor is missing, and the sibling metered suite's own doctrine (metered.rs:4-6) is that exactness is the liveness floor.

Evidence:

        52	fn assert_no_product(name: &str, grid: [u64; 4]) {
        53	    let mixed = grid[3] as f64 - grid[2] as f64 - grid[1] as f64 + grid[0] as f64;
        54	    let bound = 0.10 * grid[3] as f64;
        55	    eprintln!("MEASURED {name}: grid {grid:?} mixed {mixed:.0} bound {bound:.0}");
        56	    assert!(
        57	        mixed.abs() <= bound,

    before/tests/meter.rs:
      6493	            assert!(
      6494	                run.touches >= run.ops,

Resolution: pin the exact totals the way metered.rs does: assert `s1(n, d) == 16 * n + 2` for all four grid cells with the per-round derivation in the message (add 2: one limb read plus one deposit; sign 5: read, zero, deciding read, collapse zero, re-deposit; sub 2; sign 7: read, zero, read to partial 0, zero, certificate skip to digit 0, read −1, collapse zero and re-deposit; plus 2 once for the parked −1's spill). The measured number is the refutation pass's, not mine; measure once more before committing. At minimum add the universal floor `grid[0] >= 4 * n0` (four calls per round, one touch minimum each). Acceptance: with `touch()` stubbed to a no-op under the feature the test fails; with the shipped code it passes with the pinned totals, and the mixed second difference is exactly 0.
Construction: make `fn touch(count: u64)` (accumulator.rs:21-26) ignore `count` under `touch-meter` and run only `sign_flip_oscillation_has_no_width_product`: it passes with `grid [0, 0, 0, 0]` while every pin in metered.rs fails. Alternatively delete only the `touch(1)` at accumulator.rs:850 (the fold's per-digit read): the grid drops uniformly and the test still passes.

### suanpan-tests-26: the one committed sign-flip instrument is not cited by the claims roster's sign rows
- Where: crates/suanpan/tests/amortized_sequences.rs:66-67 (related: crates/suanpan/src/claims.rs:277-293, crates/suanpan/src/claims/tests.rs:28-55, crates/suanpan/src/claims/tests.rs:139-161, crates/suanpan/src/claims/tests.rs:279-283, crates/suanpan/tests/amortized_sequences.rs:14-16)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (grep for `amortized_sequences` and `sign_flip_oscillation` under crates/suanpan/src, crates/before/src, crates/before/tests, the justfile, and .github returns nothing; claims.rs:277-293 cites two OWN pins and the BANDS static-prefix band; `cited_witnesses_exist` resolves any manifest-relative path at claims/tests.rs:280, and `reaches` follows same-file helpers, and `s1` calls `.sign()` at :44 and :46; the sibling bands `comb_run` and `static_prefix_run` in before/tests/meter.rs:6526-6541 and :6600-6617 assert `Greater` on both halves of every cycle, so the module doc's one-polarity claim holds where I checked it); executed: no
- Seen by: structure-prose [12], api-economics [29]; refutation: confirmed; history: no-rationale-found (the roster predates the test by two days; the adopting commit did not touch it)
- Owner-gated: no

The roster exists to bind "the legs that rot silently without a name" (claims.rs:5-7). `Accumulator::sign` and `is_negative` cite witnesses that all hold one polarity; this test is the sign row's unique evidence for the sign-flip regime, and a rename or deletion would orphan nothing today.

Evidence:

        66	#[test]
        67	fn sign_flip_oscillation_has_no_width_product() {

    claims.rs:
       277	    Claim {
       278	        op: "Accumulator::sign",
       279	        table_cost: Some("amortized O(1)"),
       280	        evidence: Evidence::Witnessed(&[
       281	            (OWN, "no_collapse_fold_re_scans_the_prefix"),
       282	            (OWN, "sign_fold_skips_certified_runs"),
       283	            (BANDS, "accum_static_prefix_touches_flat"),
       284	        ]),
       285	    },

Resolution: add `const SEQUENCES: &str = "tests/amortized_sequences.rs";` and cite `(SEQUENCES, "sign_flip_oscillation_has_no_width_product")` on the `sign` claim, and on `is_negative` with a `REACH_EXEMPT` entry mirroring the existing delegation reason. Whether to instead move the test into metered.rs is an open question below. Acceptance: `cited_witnesses_exist` and `cited_witnesses_reach_their_operations` pass with the new edge; renaming the test fails `cited_witnesses_exist` by name.

## Positives

- `assert_value` (tests.rs:58-92) drives every value check through all three read-outs (`sign_magnitude`, `sign_limbs` with the minimality clause, `sign_magnitude_shl` reconstructed at scale) against an exact `IBig` oracle that shares no code with the fold or the carry pass; one small helper makes the read surface total at every value check in every suite.
- `floor_domination_is_sound` (differential.rs:701-789) pins the register arm's contract as exact (`decided ⇔ |v| ≥ 3·2^(32·(floor+1))`) rather than merely sound, carries a point mass at the extremal lazy-zone spelling (`all_extreme`), and its strategy docs (:177-184) say where the mass is concentrated and why.
- `ledger.rs` states its alphabet as structure: `ledger_op`'s doc (:115-125) justifies each op by the ledger transition it reaches within a short schedule, and `assert_ledger_invariants` (:21-92) holds the full letter of the invariant with a named failure message per clause, on a ~1.95M-state exhaustive prefix tree.
- `no_collapse_fold_re_scans_the_prefix` (metered.rs:831-890) commits the known-bad mechanism as an executable model and shows the criterion reads red on it at two widths; `merge_tie_reads_the_operand` (:348-393) makes the tie routing observable by giving the operands different nonzero populations; `alternating_shifted_writes_cost_the_operand_not_the_gap`'s doc (:69-90) derives every pinned number and names what both known-bad mechanisms would read. This is adequacy discipline done right.
- Every exact pin the lenses re-derived from `accumulator.rs` matches the committed number (u64 comb 6n+1 including the one-time extra carry, alternating pair 5 and 3, settlement 16, certified-run fold 6, first domination read 5 then 1, no-collapse separation 2k/32+3 then 1); I checked the same traces and agree.
- `witnesses.rs` records the surviving mutation each constructed corner was built to kill (`SIGN_DECIDED: 3 → 2`, decision index `floor + 1`, register constant `3 → 2`), which is exactly the provenance a future reader needs to judge whether a corner can be retired; `sign_threshold_survives_extreme_cancellation` (:40-47) cross-checks the fold against the independent low-to-high read-out before reading the sign, so a wrong threshold is convicted by a path that does not share it.
- `amortized_sequences.rs`'s module doc makes checkable claims about other files (every sibling `accum_*` band holds one polarity; the comb band drives the dual attack), and they hold where I read them (`comb_run`, `static_prefix_run` in before/tests/meter.rs).
- `claims/tests.rs`'s reach test (:139-161) makes the witness names in metered.rs load-bearing: a hollowed-out witness fails by name, and the exemption list is held load-bearing in both directions.

## Open questions for Finch

1. Where should the sign-flip test live? It uses only the public API and the `touch-meter` feature the metered module is gated on, so it could move into metered.rs (dropping the `touches` closure helper and gaining the exact-pin idiom), or stay as the crate's one integration binary (the adopting commit placed the three audit-adopted witnesses symmetrically across before/tests and suanpan/tests). Recommendation: cite it by manifest-relative path from claims.rs now (suanpan-tests-26 is a one-line change either way), and decide placement when suanpan-tests-25 rewrites its assertion; if suanpan keeps no other integration test, folding it into metered.rs removes a link step and the seed-anchor question for this crate never arises.
2. Should a zero-valued wide operand spill the register? Today `add_wide(&UBig::ZERO)` retires the register and `add_magnitude(&UBig::ZERO)` does not (suanpan-tests-4). Recommendation: short-circuit zero before the spill in the wide entries, matching `add_magnitude_shl`'s `Some(0) => {}`; a value-neutral call should not change the representation or the cost class of every later word-scale call. Either answer wants the witness.
3. The collapse fixed point: a top digit of magnitude 2 over a zero digit re-deposits 2^33 one index down, which recenters straight back to the same spelling, so every later sign read costs a constant 6 touches and lowers `bottom` for nothing. Amortized O(1) holds, but lib.rs:117-121 ("the next sign read re-reads none of them") describes the k−1 landing, not this carry-back. Recommendation: pin the 6 exactly (suanpan-tests-13) and hand the prose to the suanpan-docs partition; whether to special-case the re-deposit (deposit at the original index when the partial is a multiple of 2^32) is a production question I would not open without a caller that reads such values in a hot loop.
4. `assert_no_product` is duplicated between suanpan/tests/amortized_sequences.rs and before/tests/answer_embedded.rs. Recommendation: if suanpan-tests-25 replaces the grid check with an exact pin, the suanpan copy dissolves and no shared helper is needed; otherwise a comment at each copy naming its twin is cheaper than a shared dev-dependency.
5. Outside this partition, for the envelopes reviewer: before/tests/meter.rs:6637-6641 `accum_fan_touches_flat` calls `comb_run` at both scales exactly as `accum_comb_touches_flat` does at :6623-6627, while its doc describes a fan (Dyck-walk) stream the body never constructs. Duplicate test or missing `fan_run`? Recommendation: route to that partition's final; I verified the two bodies are identical by reading.
6. The vocabulary and dash findings (suanpan-tests-17, -19) are instances of workspace-wide conventions (159 "honest", 75 "tripwire" in before/src; 398 em-dashes in meter.rs). Recommendation: a workspace prose pass owns them; partition-local edits alone would create inconsistency. The site lists here are the suanpan input to that pass.

## Dropped

- No public way to observe the tier (api-economics [34]): refuted. Sibling unit tests asserting private representation state is what the `mod tests;` layout exists for; the crate page publishes the register's bounds and the `sign_dominates_at` rustdoc names the tier only as a hazard a caller acts on by not treating `decided` as a value function; before's comment at watermark.rs:761-763 says the forfeited register certificate "only reroutes the hop", so the consumer does not need the probe.
- Multi-level descent through near-cancelling digits reached only two levels deep (blind-spots [22]): below the bar after the reframe. The cancelling prefix `+2^k` then `−(2^k − 1)` sustains a partial of exactly 1 across 16, 11, 64, and 128 levels in `cancelling_prefix_chain_matches_the_oracle`, `sign_collapse_tightens_the_top_and_arms_domination`, and `no_collapse_fold_re_scans_the_prefix` (the last metered exactly); only a sustained partial of magnitude 2 is limited to the two-level witness, and the fold branches only on `partial == 0` and `|partial| >= 3`, so that case exercises no code the magnitude-1 case does not.
- Metered module doc promises exact pins (blind-spots [20]) and three assertions are bounds (api-economics [32]): duplicates of suanpan-tests-13.
- Hand-maintained state count in the ledger doc (blind-spots [25]): duplicate of suanpan-tests-11.
- Fully qualified Ordering (blind-spots [27]): duplicate of suanpan-tests-24.
- assert_no_product has no liveness floor (api-economics [28]): duplicate of suanpan-tests-25; its executed run is credited there.
- Only sign-flipping instrument unrostered and alone in its binary (api-economics [29]): duplicate of suanpan-tests-26; placement moved to the open questions.
- Ledger invariants never observed after rebuild/clear ops (api-economics [30]): duplicate of suanpan-tests-10.
- Run-forming proptest duplicated with copied seeds (api-economics [31]): the test half is in suanpan-tests-6, the seed half in suanpan-tests-1.
- Test models restate production constants (api-economics [33]): split into suanpan-tests-21 (SIGN_DECIDED), suanpan-tests-9 (LEDGER_OPS), suanpan-tests-11 (tally), and suanpan-tests-7 (the `+ 33`, where the constant is wrong, not merely unnamed).
