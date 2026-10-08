# reviewer-serde-framing: resumption notes

Task: review `fix/before-serde-record-framing` in /Users/oxide/src/rumors-slot-04, round 1.
Branch: 22e194a2 (base, on main ancestry) -> 9123a4d8 (test) -> a6653fdb (fix). Merges clean onto main 0bdeb588.
Commits signed (verified `git log --show-signature`).

## Established
- Read briefs (common, reviewer), L6 Q1/Q2 + rulings, baseline Landing check.
- Fixer's FieldName implements only visit_str. Derive's field visitor also had visit_u64 and visit_bytes.
- csv 1.3.1 (registry source, deserializer.rs ~L629): header-keyed MapAccess feeds keys via
  serde's BorrowedBytesDeserializer => visit_borrowed_bytes => visit_bytes. csv never overrides
  is_human_readable (=> true). HYPOTHESIS: csv default (headers) round trip of Clock worked at base, fails at tip.
  That is named form, not positional => blocking per coordinator's judgment call 1.
- csv has_headers(false) => deserialize_struct visits seq (positional). rmp-serde with_human_readable()
  + default tuple structs => seq. Both: human-readable formats whose round trip uses sequence. Owner question
  (ruling's premise "binary formats keep the sequence form they depend on").

## Plan
1. Probe crate (source in scratch/probe, copied to box ~/src/rumors-slot-04/target/review-probe, path dep
   ../../crates/before). Run at 9123a4d8 (detach slot) and at tip.
2. At 9123a4d8: run serde_impls tests, expect 3 json_* fail.
3. At tip: tests pass; mutants (case-fold key, len-matching visit_seq, visit_bytes) as reversible edits.
4. Restore, git diff empty, landing check in background -> scratch/audit-check.log.

## Results so far (2026-10-07)
- base-run.log (slot detached at 9123a4d8): 3 json_* FAIL "JSON decoded the positional form ... as Ok(...)"; 6 others pass.
- probe-base.log (9123a4d8 code): all 33 format x type round trips OK.
- tip-run.log (a6653fdb): serde tests 10/10 pass. Probe: csv(headers) ERR "invalid type: byte array, expected field identifier"
  for Clock/Span/Ranked = BLOCKING (named form, default csv config). csv(no headers) and rmp hr tuple ERR "invalid type: sequence"
  = owner question. bencode/quick-xml/ron/yaml/toml/json/rmp named OK.
- Slot back on branch at a6653fdb, clean.
- Next: mutant loop (mut/*.rs) -> mut-*.log, restore orig, git diff empty, then landing check -> audit-check.log.
- Mutants (mut-*.log): ma_casefold BLESSED (10/10 pass); mb_seqexact caught by 3 json_*; mc_trim BLESSED; md_bytes 10/10 pass
  AND probe csv(headers) ROUNDTRIP OK for all three (candidate repair). Worktree restored (cmp + empty git diff).
- Next: landing check in background -> audit-check.log.
- Landing check at a6653fdb: matches baseline (tests 761 = 757+4; board 5311/0 + two drift lines). Box probe dirs removed. DONE; report next.

## Round 1 (replanned branch; owner reversed Q1 to lenient). Tip 254d9483 on 94365fa5, 4 signed commits.
- Calibration of fixer (r3cal) fair. Mutants n1_default (serde(default) on ClockOwned.version), n2_ordered (hand-written
  SpanOwned, keys must be in order), n3_unknown (no deny_unknown_fields) in r2mut/; loop r2mut/run.sh -> r2mut/*.log, done.log.
- Probe extended with EDGE json cases. Public bullet "declaration order" refers to private fields (source-only).
- slot stash@{0} is fixer's; do not touch. No landing check unless needed.
- r2mut done: orig 8/8; n1/n2/n3 all BLESSED 8/8 (EDGE + bencode evidence). Restored. Report next.
- Round 2 at 0e63cfc7: B1/B2/S1/S2 resolved; tip 8/8; n1/n2/n3 each fail leniency property (r3mut/*.log). Restored. APPROVE.
