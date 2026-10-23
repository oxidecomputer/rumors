# fix-decode-exact-parts: resumption record

Worktree /Users/oxide/src/rumors-slot-71, branch fix/before-decode-exact-parts, base fb18dade9 (verified HEAD == base, clean).

## Established
- Bits is built from a Bytes only by: statics (party.rs SEED_STREAM, version.rs empty), from_canonical_vec, and the
  decode paths (party/io.rs, version/io.rs, span/wire.rs). Tests in bits/tests.rs also call from_canonical(Bytes).
- alloc_meter counts a shrinking realloc as new + old block in peak; so any two-exact-part split peaks ~2n.
- Board decode heap ceilings: shared 15 B/B + 1024 intercept; serde rows DESERIALIZE 5 B/B, ranked serde 9 B/B.
- `just test` has only touch-meter,scan-meter: serde/borsh entries must be cfg-gated per entry.
- Span coincident decode shares lo's buffer (lo.clone()); Span::shared_endpoint uses ptr_eq. Test budget for coincident
  parts = max(encoded, held by same route on Span::at(Version::decode(lo))).

## Plan / structure
- Bits: no constructor taking Bytes outside storage.rs: from_canonical_vec(Vec) + from_static.
- Party/Version::decode_bytes(Vec<u8>) seal; decode_head(Vec) -> (value, rest) via split_off replaces decode_prefix.
- Clock/Span/Ranked decode_bytes(Vec<u8>).

## Background jobs
- board parent capture: local log $S/board-parent.log; box file ~/src/rumors-slot-71/target/board-parent.cap
  (do NOT sync this worktree until it ends)

## Next
- write test commit; run on box (after board finishes); quote failure.

## Progress (update 1)
- Test commit 7b6585b39 (signed G). Failing output at base: $S/test-run-base.log (minimal: clocks=1, steps=[]),
  unshrunk span failures: $S/test-run-base-noshrink.log.
- Parent board: 5729 green / 0 red, pin clean; capture on box target/board-parent.cap.
- Fix written (uncommitted): Bits::from_canonical private + from_static; decode_bytes(Vec); decode_head(Vec)->(T, rest)
  via split_off; Clock/Span/Ranked split; serde fn(Vec); decode_hex via decode_to_slice into exact vec.
- tests/o1_probe.rs copied in UNCOMMITTED -- remove before committing fix.
- main moved to 86d101a71 with per-cell heap ceilings (worst+10%, thousandths per unit after 1024 intercept);
  evaluate after-fix decode readings against `git show 86d101a71:crates/before/src/testing/meter/board/cell_ceilings/heap.rs`.
- Running: full before suite -> $S/fix-tests-1.log
- Full before suite with fix: 751 passed, 1 skipped ($S/fix-tests-1.log). Probe after fix: $S/probe-after.log (all
  decode routes exact; dangerously_alias/forks(0)/sync_all(empty) hold a constant 24 B clone header: observation).
- Probe removed. Clippy (all-features / none / meter features) clean ($S/clippy-fix.log).
- Added SPLIT_INPUT NA reason for composite serde rows in board ops.rs (text only).
- Running: after-fix board capture -> $S/board-after.log, box target/board-after.cap. Then:
  scp both caps, python3 $S/compare_heap.py parent after main-heap-table.
- Fix commit message draft: $S/fix-commit-msg.txt
## STOPPED (final)
- After-fix board: 5729 green / 0 red at base ceilings, but pin drift: version_parse x heap (both scales), party_parse x heap
  (improvements), party_join_all x heap (weave -> meet-shade: rise = regression-caused flip).
- 464 heap cells would cross main's per-cell ceilings (86d101a71) from within. Comparison: $S/heap-compare.txt + inline script.
- Header shift verified: version_span_all meet-shade rise = 24 B x operand count exactly.
- Fix saved as $S/fix-decode-exact-parts.patch (applies cleanly on 7b6585b39); worktree restored to test commit, clean.
- Handed back with proposal.
