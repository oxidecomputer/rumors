# The CBOR-wire adversarial review packet

[`REVIEW.md`](./REVIEW.md) is the adversarial review packet of record for
the CBOR-legible wire stack (the branch this note lives on). It served as
the working spec for the entire fix-and-feature campaign that followed:
every finding carries an executable **Resolution** block (file, mechanism,
acceptance criteria), each written to stand without the review's own
context, and the packet was amended in place as owner rulings landed.

What it holds, in its own order: the functional chase list; findings by
severity (bugs, contract–prose mismatches, risks, behavioral notes), each
marked **verified** (ran or constructed) versus **assessed** (read only);
dispositions for all fourteen charter seeds; round-4 assumption checks;
public-API-delta judgments; simplification candidates; the clean-bill
sections (where no issues were found, and what was considered and
dismissed); residual risks and test gaps; and the out-of-range payload
depth observation, whose owner disposition grew into the full ten-step
implementation spec for the payload-depth-limit feature (the knob, the
symmetric greeting exchange with equality abort, and the peer-constructed
codec that carries the limit beside the serializer and deserializer
function pointers).

Retired as executed: every in-range resolution landed through the
worktree-isolated fix lanes and their review rounds, and the depth-limit
spec shipped as the feature that followed. Reading notes for today's
tree:

- Line anchors and commit SHAs cite the review-time branch states
  (`a5c4ca1a`, re-anchored at `d05a6d03` after the PR #37 rebase). Both
  predate the signed rebase onto main, so those exact SHAs survive only
  in the `backup/cbor-wire-pre-37` and `backup/cbor-wire-pre-resign`
  branches; the same changes live on this branch under different SHAs.
- The packet's vocabulary predates two later sweeps: the error-taxonomy
  rename (the send-side `EncodeError` naming and the scope-qualified
  adapter pair) and the crate-wide constructor-naming pass. Where the
  packet and the code disagree on a name, the code is current.
- The rebase-audit and dispatch-sequencing notes describe campaign
  mechanics (re-sign ordering, seed re-checks) that completed; they are
  history here, not open instructions.
