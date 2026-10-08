# auditor-l6 resumption record (lane L6: codecs and serialization adapters)

Scratch dir: this directory (`.../scratchpad/auditor-l6/`). Worktree:
`/Users/oxide/src/rumors-audit-l6-codecs`, branch `explore/l6-codecs`, base
`58285ca5` (verified at start: HEAD == base, clean). Box sync dir:
`~/src/rumors-audit-l6-codecs`. Remote runs MUST be serialized on this worktree
(a second sync rewrites files under a running build).

## How to run things

- Remote: `/Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh /Users/oxide/src/rumors-audit-l6-codecs 'unset CARGO_TARGET_DIR; ...'`.
  The wrapper runs under errexit: capture per-step status with
  `if cmd; then rc=0; else rc=$?; fi`.
- nextest: `--build-jobs 24 --test-threads 24` (`-j` means test threads in
  nextest). Repo profile kills tests at 180 s; long campaigns use libtest:
  `cargo test -p before --all-features --locked --lib -j 24 -- l6_ --test-threads 8 --nocapture`.
- `L6_CASES=<n>` sets the spec harness's case count (default 2000).

## Explore-branch artifacts (committed)

- `7cb120ff` probes: `crates/before/src/testing/l6_probes.rs`
  (registered in `testing.rs` under `cfg(all(test, serde, borsh))`).
- `ab622a92`, `ef5bc48b` spec harness: `crates/before/src/testing/l6_spec.rs`.
  An independent spec codec for Party/Version/Rank/Clock/Span/Ranked written
  from the docs (own bit packing, gamma/zigzag, lenient parsers recording
  violations and detection positions, rank codec, pointwise join/meet). Checks
  every entry: slice decode, text FromStr, postcard, CBOR (message names an
  applicable class), borsh in-stream on bytes++junk (consumption), chunked/
  Interrupted readers, JSON span records; plus encoder agreement (production
  encode, join, meet, span, rank, ranked, clock bytes == spec bytes).
  Verdict model: accept set exact; error in applicable set; span's documented
  "structural over ordering" precedence as `required`; informational
  first-event model.

## Established (with evidence)

1. Probes (`probes2.log`): reader chunking/Interrupted/failing readers agree
   with slice decodes for all six types and borsh (512 cases); calibrated by
   making Rank::decode's first-loop `Interrupted` read as EOF -> caught
   (`calib-reader.log`), reverted, `git diff -- rank.rs` empty. Failing
   writers: every encode_to/borsh serialize writes a prefix and reports the
   error (256 cases). Leads 2 and 3: no defect.
2. Serde leniency (probes, verified): JSON positional arrays accepted for
   Clock/Span/Ranked; CBOR byte string `h'0102'` decodes as Count limbs [1,2];
   CBOR arrays accepted for byte types (ruled by decision 13); CBOR text
   strings rejected. -> `questions.md` Q1, Q2.
3. Spec harness: 300k cases per type, all pass, ZERO first-event mismatches
   (`spec-long1.log`, 423 s). Extended harness (deep spines 30..400, long rank
   fractions, chunked readers, JSON spans) passes at 2000 (`spec2.log`).
4. Calibration of the spec harness (`calib-spec2.log`): 13 env-gated
   mutations (`mutate.py`, var `L6M<k>`), all caught; reverted, production
   diff empty. M13 caught by applicable-set; span oracle since widened so the
   precedence check catches it (not re-run yet).
5. Existing-suite adequacy (`adequacy.log`, `mutate2.py`, cached var `L6M=k`,
   helper appended to lib.rs and reverted, diff empty): baseline 693/693
   pass; every one of the 13 mutations is caught by >= 1 existing test
   (weakest: M6 by 1 test, M13 by 2, M7 by 3, M8 by 4, M10 by 5). So the spec
   harness has NOT yet shown a failure class the suite misses.
6. 32-bit: inventoried every narrowing conversion in the lane's code; none can
   wrap (all bounded by buffer length). Wasm pins exist for Version::decode
   and Rank::decode only. -> observation 11.
7. Resource side: board covers every slice decoder (incl. rejection rows) and
   serde/borsh deserializers on all families; `pure-comb` is the carry-ripple
   worst case for the validator's accumulator. Not duplicated.

8. Under-rejection round (`adequacy2.log`, `mutate3.py`, cached `L6M=14..17`):
   M14 admission close never rejects; M15 admission finish skips; M16
   standalone validator never rejects collapsible; M17 party never rejects
   owned pair. All caught by the existing suite AND the spec harness (M14/M15
   by 2 existing tests each). Conclusion: across 17 calibrated defects the spec
   harness catches nothing the suite misses -> NO machinery brief for it.
9. Widened span oracle now catches M13 via the precedence check
   (`calib-m13b.log`: "documented precedence requires {TrailingBits}").
10. Hostile resource families (`tests/l6_resource.rs`, commit `34e6cf2e`,
   `resource1.log`): Ranked wide-integral prefix 5.00 B/B, borsh 6.00,
   long-fraction 3.67, truncated wide rank 1.50, tiny prefix + wide version
   5.00 (touches x4 per x4), wide-version reference 2.50; constant across
   16 KiB..1 MiB. No resource finding.

11. Extended spec campaign (deep spines, chunked readers, JSON spans): 300k
   per type, all pass, no first-event mismatch (`spec-long2.log`, 1940 s).
12. DEFECT (confirmed on wasm32, `wasm1.log`, explore commit `679cc2e5`):
   `Rank::decode` panics on a canonical 2^-(8g) stream at g = 2^30 + 1 groups
   (`Trapped(UnreachableCodeReached)`, 124 s); g = 2^30 passes (128 s);
   `VecGrowthBoundary(2^30)` shows try_reserve(1) on a full 2^30-byte Vec fails
   as capacity overflow. Root cause: `groups: Vec<u8>` push, amortized doubling
   past isize::MAX on 32-bit -> capacity_overflow() panic. Shared mechanism
   (inferred): BitSink mantissa, borsh StreamBitsReader.bytes, Ranked borsh
   rank_bytes, BitStack words, BitsWriter growth; read_to_end decoders return
   Io(OutOfMemory) instead (error, target-dependent).
13. Writer copy-path harness (`l6_writer.rs`, `2036c94c`): 20k cases pass;
   calibration W1/W3 caught (also by 60/65 committed tests); W2 survived all ->
   equivalent: the early-return branch in `SplitOutput::splice_continuation` is
   unreachable (last leaf's flag always directly follows the penultimate
   payload). -> simplification brief (dead branch).
14. usize sweep: only `BitsWriter::reserve(width: usize)`, `TAG_BITS: usize`,
   `scan::record_bits(usize)` are non-memory usizes; values <= 64 ->
   `simplification-bit-count-types.md`.

15. Reach (`reach-hist.log`): my version inputs depth>32 in 11.8%, negative
   violation 22.3%, collapsible 15.5%, incomplete 32.2%; committed
   arb_oracle_version max depth 4. Writer reach probes (`reach5.log`): split
   splice 16 committed tests, depth-1 copy 57 (tick/raise.rs:360), wide-last 26.
16. Pins workspace: guest now enables before's `borsh` feature; lock adds
   borsh 1.6.1 + cfg_aliases 0.2.1 (same as main workspace); original lock
   saved as `wasm32-pins-Cargo.lock.orig`.

17. Second growth-panic site DEMONSTRATED (`wasm2.log`, commits `3854b4c1`,
   `431011b3`): borsh Version of 2^30+5 bytes traps; 2^30 bytes passes;
   Version::decode on the same lazy stream returns Io(OutOfMemory). Folded into
   the one defect record (same root cause).

## Deliverables written

- `questions.md` (Q1-Q3), `simplification-padded-prefix.md`,
  `observations.md` (11 items). `STATUS.md`.
- No machinery brief (see item 8). Pending: coverage record, final report.

## Open hypotheses / next steps

1. Final report via SubagentHandback (defect, Q1-Q4, three simplification
   briefs, observations, inventory).

## Background jobs

- None running.
  Every remote command now starts `unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24;`.
