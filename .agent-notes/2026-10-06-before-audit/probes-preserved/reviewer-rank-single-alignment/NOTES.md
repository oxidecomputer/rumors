# reviewer-rank-single-alignment: resumption notes

Task: round-1 review of `simplify/rank-single-alignment` in /Users/oxide/src/rumors-slot-20,
tip 66ebbcb0 (signed G), base 19154cd1 (#28 tip). Cap: 4 box runs (<=3 calibrate), landing
check only if recommending a code repair. nice -n 10. touch edited files before each run.
Slot must end clean (git diff empty), no commits.

## Established (local, no box)
- HEAD = 66ebbcb0, clean. WIP 0e3753b4 differs from final only in `aligned` rustdoc.
- num-bigint 0.4.8: `Shl<u64> for &BigUint` exists (Cow::Borrowed). biguint_shl: digits =
  (shift/BITS).to_usize().expect("capacity overflow"); BITS=32 on wasm32 -> expect at gap>=2^37.
  biguint_shl2 digits>0: Vec::with_capacity(digits+len+1) -> std panics past isize::MAX bytes,
  i.e. gap ~>= 2^34 on wasm32 (LOWER than builder's 2^35 bound). digits==0: Borrowed clones
  anyway (into_owned), so borrowed shift saves the clone only when gap >= 32 (wasm32) / 64.
- Decoder: fraction groups in one Vec<u8> -> decoded exp <= 8*isize::MAX < 2^34 on wasm32;
  doubling growth caps it near 2^33 in practice. Representability argument in `aligned` doc
  holds regardless: shifted operand <= max(result, other input) + 1 digit.
- Builder fuel dumps corroborated from its artifacts (fuelcmp.py): add +45..75 all 5143;
  sub +50..72 on 1217/9069 accepted; inline(always) add +30..60, sub moved +30..52 (builder
  wrote 0..52: the 0 is unmoved samples).
- Board: builder parent vs tip outputs identical after stripping cargo lines; tip rebuilt before.
- Bands drift: builder-one-sweep base refit at d5e80103 (on main, ancestor of 19154cd1, only
  #28's two commits between in crates/) equals this parent's refit except 6th-decimal moves in
  ff_version_distance/lag/rank. => the 145-line drift vs committed bands.rs predates #28.
- No ghost refs (git grep alignment_fits|accumulate|guarded route|...).

## Plan
Run 1: scaffold rank.rs with option_env!("REVIEW_RANK_VARIANT") variants
  p(parent) t(tip) b(inline always) c(inline at sites, clone) d(helper, &shift)
  e(inline at sites, &shift) f(always + &shift); mutants A(builder swap) N(self gap narrowed)
  S(swapped return). fuel dumps p..f; board d,e; native full before for A,S; native rank
  tests + temp edge test for d,e; pins t,d/e,N with temp cases 5..10 (deep exp 2^32+8,
  smalls exp 9/8/7 -> gaps 2^32-1, 2^32, 2^32+1).
Run 2: calibrate chosen variant. Run 3: landing check on clean repair if recommending code.

## Background jobs
- Run 1 (box run 1/4) launched ~21:42 box time: worktree has scaffold (patch saved as
  scaffold-rank.patch for rank.rs; guest checks.rs + pins.rs temp cases; untracked
  review-run1.sh and fuzzfit/harness/src/bin/review_dump_rank_fuel.rs). Local log run1.log;
  box outputs ~/src/rumors-slot-20/target/review/ (statuses file lists step status).
  DO NOT edit worktree until it finishes. Afterwards: scp target/review back to scratch.

## Prose findings so far (verified)
- commit msg: 2^35 bound does not clear Vec::with_capacity isize::MAX limit (~2^34 on wasm32);
  conclusion survives via decoder Vec<u8> bound (<2^34, ~2^33 by doubling) and representability.
- inline(always) checked_sub range is 30..52 on moved samples (builder said 0-52).
- refit claims (slope .321453->.315056, <=0.0145 move over 128..3952 bits) verified.

## Run 1 results (verified; outputs in out1/)
- Scaffold neutral: fuel-p/t/b byte-identical to builder parent/tip/always dumps.
- Fuel vs parent (add | sub moved): t +45..75 | +50..72; b +30..60 | +30..52; c -2..29 | -2..21;
  d +20..51 | +33..48; e -27..+5 (5137 fall, 6 rise +1..5 all at 96 denom bits) | -19..-3 (all 1217 fall);
  f +5..36 | +13..28.  => recommend e (inline at both sites, borrowed shift).
- Board acc+wc for d and e byte-identical to tip (no heap row moves).
- Pins t,d,e pass incl temp cases 5-10 (gaps 2^32-1, 2^32, 2^32+1 both sides). N fails pin case 4
  by Trapped(UnreachableCodeReached) (BigUint underflow), and temp case 9 likewise.
- Native rank filter 59/59 pass p,t,d,e (incl edge probe). A full before: 26 fail (25 committed +
  probe); S: 29 fail (28 committed + probe).
- Scaffold reversed: git diff empty at 66ebbcb0 (verified). Saved scaffold-full.patch etc.

## Run 2 (box run 2/4): landing check on clean repair-e.patch (applied in slot, uncommitted).
Log run2.log. Then Run 3: re-add dump bin (copy in scratch), calibrate + dump; compare dump with
out1/fuel-e.txt. Then reverse repair, prove git diff empty.

## Runs 2-3 done (verified)
- Run 2 landing check on repair-e: matches baseline pre-board-pin form (tests 758+142, docs 193+4,
  surface 14 / 211=134+77, board 5311 green + exactly 2 count_display drift lines, wasm 9/25/43).
- Run 3 calibrate+dump on repair-e: dump byte-identical to scaffold e; refit add line falls
  0.0003..0.0009 log10 over 128..3952 bits vs parent refit; committed bands untouched.
- Slot restored: git status/diff empty at 66ebbcb0. Box runs used: 3 of 4; calibrate runs: 1.
- NEXT: write report.
