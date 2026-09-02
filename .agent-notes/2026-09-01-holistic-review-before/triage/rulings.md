<!-- CAVEAT LECTOR: the ruling texts below are transcribed by Claude from Finch's decisions in review sessions; the decisions are Finch's, the wording is Claude's unless quoted. Read with the ground rules in ../../README.md. -->

# Rulings

One numbered entry per decision, in the order made. A ruling names the finding ids or the owner-decision items it disposes, states the decision positively, and records what follows for the work. Entries in the review documents that a ruling disposes carry a Disposition line citing the ruling by number.

## Ruling 1 (2026-09-02): the asymptotic claims are absolute

Disposes: skyline-coding-9; the "name the five as exceptions" alternative in crate-root-40, envelopes-a-1, README owner-decision items 12, 21, and 22, and claims open questions 1 and 3. Applies by the same sentence to the other demonstrated cost rows, rank-33, skyline-sweep-place-masked-5, and span-causally-36; the transient-space row skyline-fill-grow-2 is the same kind of promise (`lib.rs`'s "at most a small constant multiple of the input size") and the review reads the ruling as covering it, pending the owner's confirmation.

Finch's words: "The asymptotic claims *must* hold absolutely. I'm surprised doubly: that they don't, and that we failed to catch it using the instruments that exist. We should ensure the instruments can catch this case (and find out what else they aren't catching, if anything) and repair it."

Decision. `lib.rs:350-353` is the contract as intended: every asymptotic claim is a hard guarantee for all input sizes and shapes, including shapes reachable only through `decode`, text, or literal construction. Every demonstrated over-bound row is a defect to fix; none is an exception to declare in `lib.rs` or a model to adopt. The `tests/meter.rs` header may state which operations still measure over their bound and that each is under repair, and must not assert that the contract holds today or present the excess as accepted.

What follows. Instruments first: for each demonstrated row, land the breaching shape as an envelope row (and as a board family where the operation has a board row) so the breach reads as a failure before its cure; then fix. For skyline-coding-9 the candidate cure is to hold the re-anchored code in place at the stream's tail instead of extracting and re-splicing it (Claude's suggestion, unverified), with the two-stream builder as the fallback. Beyond the five rows: audit the join, meet, span, rank, masked-comparison, and coverage instruments for the same blind spot, an instrument that drives only the benign face of its operation, and report what else they miss (the claims document's crate-wide pattern "The committed instrument measures the benign case" is the starting list).
