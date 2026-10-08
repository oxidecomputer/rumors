# builder-docs-branch resumption notes

Task: docs branch per coordinator-briefs/docs-branch.md (11 items). Worktree
/Users/oxide/src/rumors-slot-38, branch docs/audit-corrections, base 176d3d43
(verified = main, clean, at start).

## Established
- Read: common.md, builder.md, docs-branch.md, README, baseline, AGENTS (both).
- branches.txt (this dir): files per audit/fix/simplify branch vs merge base.
- Landed already (git cherry all '-' or none): audit/board-count-display-pin,
  fix/pr38-concision, fix/suanpan-limb-index-wrap, fix/before-writer-retention.
  fix/mutants-dispositions, fix/pr38-mechanical: pre-audit, rumors-only files.
- Deferral criterion chosen: actual merge-tree conflict (brief says "check
  with git merge-tree"; item 7 says "defer if ... conflicts"). Report clean
  file overlaps too.
- L4 O1 and L7 O3 edit the same sum_iter comment in rank.rs -> one commit.
- L1 O4 likely moot: fix/before-fork-size-hint implements D1 (exact hints).

## Next
- Read code at each item, predict conflicts from branch hunks, build commits.

## Commits so far (all signed G), on docs/audit-corrections
- 16131cea9 item1 obs1: contract `O(n log k)`, `n` total input bytes, on
  version_join_all + span_union_all + span_intersect_all (json + sites +
  before-fuelscape roster). Variant: Span Sum/Product had same defect.
- 7797d69c0 item1 obs3: < vs <= in Version docs.
- a189d0fb2 item1 obs4: "in the worst case" on join_all/meet_all/span_all/
  Clock::recv_all/absorb_all.
- 3f8428b83 item2: Rank normalization prose.
- ba52a81db item2 O1 + item5 (L7 O3): sum_iter comment rewritten. Finding:
  fold exp can exceed summand exps (widening shift by held width); bound
  E + H + max(W,H) < 3*2^35 on 32-bit. Records missed this.
- 3687bce36 item2 O8: Count usize doc (option A).
- Deferral check not yet run (do merge-tree at the end, per commit).
- a7c7e1697 item3 L5 defect1 (law doc); 9f6d2e9c3 defect2 (toward);
  150200673 defect3 (Polarity "if any", gloss fixed); a3169c687 tie order;
  37d139168 grid witness argument (causally/tests.rs; conflict-prone with
  simplify/span-refine-partial, check).
- 266299d05 item4 TrailingBits; 1a4e60c61 borsh obs3 (Span lo/hi);
  d3f8a08b5 borsh obs2 SERIALIZE half only (Deserialize half collides with
  fix/before-wasm32-buffer-growth which documents them); 565b57893 borsh
  obs1 error mapping in lib.rs + README (finding: Count truncation is
  InvalidData via borsh's integer readers); d02585aba CBOR bridging.
- 7ef8ebd6c EXTRA: lib.rs PartialEq -> PartialOrd (+README).
- Item 6: O2 78ba80ad4, O3 79799350e, O6 FusedIterator d28a15f3a (test
  drained_forks_stay_exhausted in party/forks/tests.rs), from_parts
  274a8effe. O4 MOOT (fix/before-fork-size-hint rewrites size_hint docs).
- Item 7 DEFERRED (simplify/suanpan-read-high-part rewrites the same lines;
  its new text says "high part is at most 3", which item 7 corrects).
- Item 8 fab43af8b; item 9 942bff4f4; item 10 LEFT FOR OWNER (no reading-free
  justification; deep test detection is alignment-dependent); item 11 48bb1e424.
- Borsh obs1 (error mapping) DEFERRED: conflicts with
  fix/before-wasm32-buffer-growth on the borsh bullet; dropped via rebase;
  patch saved as deferred-borsh-errors.patch. Tip rescan: no conflicts.
- Cold build done (build1.log exit 0). NEXT: run new test, calibrate M1
  (remove ClockForks impl -> compile error) and M2 (PartyForks::next resets
  plan.remaining = Remaining::Exact(1) before returning None), then landing
  check in background.
