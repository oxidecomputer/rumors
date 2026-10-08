# auditor-l3 status

Round 1 report returned. Run B finished: all 3 passed (long-B.log). Still running: the detached spine run (box cargo 5713, test binary 25758; log ~/src/rumors-audit-l3-events/target/l3-long/spines.log; l3_cogen_spine passed 30,000; l3_family_spine pending).
Confirmed defects:
- D1 (resource, constant factor): Version::min_ticks transient heap up to 39 B/B (plain rising right spine) and ~122-197 B/B (jump-entered) vs the board's 1024 + 20n; re-opens the closed min_ticks heap fix. defect-D1-min-ticks-heap.md, test-brief-D1.md, fix-note-D1.md.
No correctness defects. Test gaps: 4 mutations escape all committed tick tests (M6, M8, M15, M19); machinery-brief-cogen.md, machinery-brief-min-ticks-histories.md. Simplification: simplification-brief-S1.md.
Blocking questions: D1 classification (defect vs adequacy + constant-factor observation).
