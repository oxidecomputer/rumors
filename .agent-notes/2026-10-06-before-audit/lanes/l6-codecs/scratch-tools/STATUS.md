# auditor-l6 status

- Now: writing the final report (round 1 complete).
- Confirmed defects: 1 (medium) - decoders panic on wasm32 once an
  input-proportional Vec passes 2^30 bytes; demonstrated for Rank::decode
  (fraction groups) and borsh Version decode (stream buffer).
  Record: defect-rank-decode-wasm32-growth.md; test brief + fix note beside it.
- Questions: questions.md Q1-Q4 (Q4 = post-fix contract for oversized inputs).
- Briefs: simplification-padded-prefix.md, simplification-bit-count-types.md,
  simplification-dead-splice-branch.md. Observations: observations.md.
  Coverage + instrument inventory: coverage.md.
- Blocking questions: none.
