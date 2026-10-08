# fixer-serde-framing resumption notes

Task: fix commit on fix/before-serde-record-framing in /Users/oxide/src/rumors-slot-04,
on top of test commit 9123a4d8 (parent 22e194a2). Verified HEAD == 9123a4d8, clean.
Brief: .agent-notes/2026-10-06-before-audit/coordinator-briefs/test-serde-record-framing.md;
ruling Q1 at end of lanes/l6-codecs/round-1/questions.md.

## Established (by reading)
- Only ClockOwned/SpanOwned/RankedOwned derive Deserialize in serde_impls.rs; used only
  in is_human_readable() branches. Party/Version/Rank/Count HR = String. Binary unaffected.
- serde_derive 1.0.228 field visitor also accepts visit_u64 (field index) and visit_bytes,
  so a map keyed by field position ({0: .., 1: ..}) is a second positional spelling.
  Plan: reject it too (string keys only); flag as judgment call.
- Board serde rows are binary-only (OwnedBytes is_human_readable false): no meter moves.
- No other crate (fuzz differential, rumors) deserializes these composites HR.

## Design
- Generic `TextRecord<const N>` visitor: visit_map only, string field names only,
  duplicate/missing/unknown errors, expecting "struct <Name>". Returns [String; N];
  callers parse with FromStr (same as Party/Version HR Deserialize).
- Extra test in fix commit: MapDeserializer keyed by field index rejected, by name accepted.

## Status
- (update)
- Fix written (uncommitted) in slot-04: generic RecordVisitor/FieldName/deserialize_record,
  module doc + Span impl doc restated; new proptest readable_serde_reads_only_complete_named_records
  (MapDeserializer: named ok; position keys, omitted, repeated, unknown rejected).
  Fixed copy saved at $S/serde_impls.fixed.rs.
- base-run.log: at test commit, json_* 3 fail + position-key test fail (Ok(Clock..)). No new seed on box.
- fixed-run.log: 10/10 serde pass. clippy.log: clean (after dropping unused Deserialize import).
- Mutant runs (background task b3zswv29v): m0=base, m1=no dup check, m2=visit_u64, m4=skip unknown;
  logs mut-*.log; loop restores fixed file (verify cmp!).
- Coordinator: verification of record is now `~/bin/audit-reserved ~/bin/audit-check`, not just gate.
- Next: check mutant logs, verify restore, commit fix (signed), run audit-check in background.
- Mutants: m0 (base) fails new test "keyed by field position decoded as Ok" + 3 json tests;
  m1 "repeating field \"party\" decoded as Ok"; m2 "keyed by field position"; m4 "unknown field".
  Each mutant fails ONLY the new property. Fixed file restored (cmp ok).
- COMMITTED fix a6653fdb (signed, G). Branch = 9123a4d8 test + a6653fdb fix.
- audit-check running in background (task bfsq2pfzp), log $S/audit-check.log ("audit-check exit=N" appended at end).
- Next: read audit-check log vs baseline (all ok except board's two count_display drift lines); report.
- audit-check DONE on a6653fdb: surface/docs/lints/wasm/tests ok; board FAILED with 5311 green/0 red and
  exactly the two baseline count_display x heap drift lines. Matches baseline. Report handed back.

## Round 2 (review findings, from coordinator)
B1: add FieldName::visit_bytes (csv header keys arrive as bytes); test case keyed by name.as_bytes() via BytesDeserializer;
    show fails without visit_bytes. Restate prose (serde_impls.rs ~88, ~126-127, module doc, commit body).
S1: case-variant key case (e.g. "PARTY"); keep unknown case (m4 must still fail).
S2: equivalence property: record field text s decodes iff Party/Version own HR deserializer accepts s, equal values; mc_trim must fail.
P2: helper/property docs overclaim "exactly when"; enumerate rejections.
5: leave lib.rs bullet and Deserialize impl docs unchanged; visit_seq unchanged (owner question).
Amend fix commit (signed); test commit 9123a4d8 first. One landing check in background -> $S/audit-check-r2.log.
Report: SHAs, signatures, each mutant verbatim, csv probe at tip (reviewer probe/ crate), check log path.
Reviewer artifacts: .../scratchpad/reviewer-serde-framing/ (mut/*.rs mutants vs a6653fdb, probe/ crate).
- R2 edits in worktree (uncommitted): visit_bytes + prose; tests: read_map/read_text helpers, helper w/ byte keys + upper-case key,
  property renamed readable_serde_reads_named_records_and_rejects_their_variants, new FieldText + readable_records_read_field_text_as_the_field_types_do.
  Saved copies: $S/r2-fixed.rs, $S/r2-tests.rs (tests copy predates final formatting tweaks; worktree is truth).
- Running: fixed tests+clippy+probe -> $S/r2-fixed-run.log (task bmoqbnayt). Then r2mut/run.sh (mutants) -> r2mut/*.log.
- commit-msg.txt rewritten for amend.

## REPLAN (owner reversed Q1: "Option 2. We should be lenient.") 
Dropped: test commit 9123a4d8, fix a6653fdb (reflog). Round-2 uncommitted work saved as
$S/round2-uncommitted.patch and in `git stash` of slot-04 ("fixer-serde-framing round 2 ...").
Round-2 partial mutant results (r2mut/*.log): base, no_bytes, casefold, trim all exit 100 (loop stopped at skip_unknown).
New plan from main: (1) Span impl doc correction; (2) leniency property: named (str + bytes keys), seq, position-keyed map
(if derive accepts) all read same value; calibrate with sequence-rejecting RecordVisitor swap; (3) FromStr-equivalence
property on main's derive code (finding if differs); (4) lib.rs bullet clause + just readme, flag sentence.
Verify: fmt --check, clippy, serde tests, landing check bg -> $S/audit-check-r3.log.
- R3 commits: 2902eff8 Span doc (signed G); aede4c2d leniency property (signed G; passed on main derive, r3-c2.log).
- Commit-3 equivalence property appended in worktree (uncommitted). Calibration loop r3cal/run.sh -> r3cal/{committed,strict_all,strict_seq,no_positions,derive_trim}.log, loop.log.
  committed.rs = serde_impls.rs at commit 1/2 (restore target).
- Next: commit 3, then commit 4 docs (module doc + *Owned docs + lib.rs bullet + just readme on box? check tools/readme), landing check.
- R3 calibration (r3cal/*.log): committed 8/8 + clippy 0; strict_all fails leniency at "byte keys"; strict_seq at "a sequence";
  no_positions at "position keys"; derive_trim fails equivalence 'clock party " 20"' (left Some, right None). No step-3 finding.
- Commits: 2902eff8 Span doc, aede4c2d leniency test, a8fde310 equivalence test, 4f20235b docs+README. All signed G.
- Landing check at 4f20235b -> $S/audit-check-r3.log (background).
- lints failed (doclint summary 249>220 in commit 3); amended -> a8241142, cherry-picked docs -> 254d9483. Rerun landing check -> audit-check-r3b.log. Old SHAs a8fde310/4f20235b superseded.
- audit-check-r3b at 254d9483: all ok except board (baseline drift only). DONE; report.
- R4: rebuilding from aede4c2d (old tip 254d9483, a8241142). Plan: amend c2 (reversed form + rejections), cherry-pick+amend c3 (seq form, drop 'five'), cherry-pick+amend c4 (module doc any-order, lib.rs B2 wording, readme).
- R4 commits: 2902eff8, 76959581 (c2), d5502e0d (c3), 0e63cfc7 (c4 tip), all signed G. Calibration r4cal/run.sh -> r4cal/*.log (committed n1_default n2_ordered n3_unknown seq_trim). Then landing check -> audit-check-r4.log.
- audit-check-r4 at 0e63cfc7: all ok except board baseline drift. DONE.
