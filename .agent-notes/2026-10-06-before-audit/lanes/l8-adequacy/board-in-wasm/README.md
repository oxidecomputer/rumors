# Running the amplification board in wasm under fuel: scope

The scope agent's findings on whether the board can run in a Wasmtime guest
with fuel as a fourth currency, to catch time regressions that heap, scan,
and touch cannot see. Its spike is on `explore/board-in-wasm`.

- [`report.md`](report.md): the agent's hand-back: verdict, six answers,
  proposed design, risks, effort, defect, candidates, questions. It cites
  these files at scratchpad paths; the copies here are the archive.
- [`scope.md`](scope.md): the full scope, with commands and evidence.
- [`brief-machinery-fuel-board.md`](brief-machinery-fuel-board.md): the
  fuel-board design proposal, for the owner's ruling.
- [`brief-defect-benign.md`](brief-defect-benign.md): the `benign` family's
  pointer-width dependence, with a test brief and fix note.
- [`candidates-rank-fuel.md`](candidates-rank-fuel.md): `rank_sum` and
  `version_rank × freeze-parade`, superlinear in fuel, not root-caused.
- [`NOTES.md`](NOTES.md): the agent's working notes.

The coordinator verified the six spike commits on the branch, the clean
tree, and the `benign` defect's site and arithmetic (`XorShift::next`
returns `u64`, truncated by `as usize` on 32-bit targets). The measurements
are the agent's, not re-run.
