# reviewer-wasm32-growth NOTES
Branch fix/before-wasm32-buffer-growth, worktree /Users/oxide/src/rumors-slot-14, tip d0a6205f (signed), base d6d22b4c (code-equal main a49cbd40). Round 1.
## Established
- HEAD verified = d0a6205f, clean.
## Open
## Background jobs
## Next
- read diff, growth.rs, run landing check in background
## Established (cont.)
- Landing check at d0a6205f: landing.log + landing-legs/. Matches baseline: lints/docs/surface/tests/wasm ok; board 5311/0 + exactly 2 count_display drift lines. tests 762 (757+5) 1 skipped; snapshots 142; doctests 193/3; surface 14, 211=134+77; wasm32-pins 12/12, fuzzfit 25, fuelscape 43. "2 leaky" are unrelated, nondeterministic.
- std 1.97.1 verified: small_probe_read infallible extend (io/mod.rs:425-440); Chain::read_to_end calls second.read_to_end on full buffer (mod.rs:2735-2743); BufReader::read_to_end same pattern (bufreader.rs:411-418). => growth::read_to_end still panics via Chain on wasm32 (to demonstrate).
- Commit msg d0a6205f omits suanpan outstanding site -> blocking per coordinator.
- growth.rs doc "at most eight times the final capacity" wrong when last grant clamped: bound 9x.
- `.max(required)` in fallback steps never exercised by any test (mutant survives) - test gap.
- rank: image built eagerly even for integral-only ranks (board cliff 3.3->3.7). num-bigint from_bytes_be copies (biguint.rs:641): dense rank ~1 GiB aborts at exhaustion (residual, fixer noted).
## Established (final)
- Chain probe (probe.py, probe-chain.diff, probe-chain.log): at tip, Version::decode(2^30-byte slice .chain(lazy 1-byte rest)) -> Trapped(UnreachableCodeReached); 1 MiB head control -> Passed. Probe removed; worktree clean (status empty).
- serde_bytes 0.11.19 ByteBuf::visit_seq bytes.push (bytebuf.rs:211-223): missed site, source-verified only (pins lockfile lacks serde_bytes).
- Mutants A (drop .max(required)) and B (below-limit fallback &[]) both survive growth+bit_stack tests (mutantA.log, mutantB.log). Restored; clean.
- Board readings identical to fixer's tip run; moved rows only rank*/ranked* (board.diff).
- Fixer's clamp-only dispute corroborated by S1/S6 Exhausted -> Passed.
## Verdict: CHANGES REQUIRED. Report delivered via SubagentHandback.
## Narrowed task (coordinator): rank restructure in isolation
- UNEXPECTED: slot-14 branch reset to 932d2350 (reflog HEAD@{0} reset), d0a6205f only in reflog. Not by me. Made no edits/runs after discovering; variant.py (V1/V2/MUT) ready but unapplied.
- Board parent vs d0a6205f rank_decode: 53 up (47 fitted, old ~2.7, ratio up to 1.33=4/3), 78 down (min ratio 0.78), 19 same. Hypothesis: image doubling slack carried into from_bytes_be materialization (image+to_vec copy+digits). Not isolated experimentally.
- Restructure alone (plain push): to_bytes_be(0)=[0] -> image 1+g bytes -> g=2^30 control traps on wasm32 (inferred from source).
## Fresh round (rebuilt branch faba9766 / 15e1ae7c / 73cf4e70)
- Slice probe (probe2.py, probe-slice.diff, probe-slice.log): Version::decode(&slice) of 2^30+1-byte wide leaf -> Passed (exact bytes). Removed; clean.
- boardcheck.txt: 452 heap down, 0 up, 0 scan/touch, 0 exponent rises; ceiling 4.6->3.7 (m>=2000) reproduces 6.0 and 5.0.
- span/wire.rs: precedence sentence orphaned under # Panics.
- Verdict: CHANGES REQUIRED (doc overclaim; trap-pin docs/asserts vs unspecified contract; span doc placement).
