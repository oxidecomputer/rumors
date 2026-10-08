**CAVEAT LECTOR: Claude wrote this document** for Finch, from the audit's
records, the archived branches, and the fixers' and reviewers' notes. It
is unaudited; verify any claim against the tree or the cited artifact
before relying on it.

# `Version::min_ticks` and its transient heap

`before`'s `Version::min_ticks` computes, in one linear-time pass, the
fewest ticks that could have produced a version. On `main` it can hold far
more temporary memory than the crate promises on some constructible inputs:
about 125 bytes of heap per input byte on the board's jump-entered rising
spine, against a ceiling of 20. Between 2026-10-06 and 2026-10-08 the audit
built and measured five repairs. One recreated the problem past a
threshold the board cannot reach, two reduced heap at the cost of time
that is quadratic on some input, and two stopped on small rises in fitted
growth exponents. The owner stopped the work (question 87). `main` keeps
its implementation, and this document records what was learned, so the
problem can be picked up later without repeating the rounds.

## Reading order

If you have not seen the audit, read the sections in order. If you know
the code and want the failures, start at section 4.

1. [What `min_ticks` computes](01-what-min-ticks-computes.md): the
   definition, the identity the code evaluates, and a worked example.
2. [How `main` computes it](02-how-main-computes-it.md): the one-pass fold,
   frozen height prefixes, range minima and their boundaries, and the packed
   contribution word.
3. [The transient-heap defect](03-the-defect.md): the inputs that cause it,
   what grows with what, and the measured scale.
4. [The attempted designs](04-attempted-designs.md): re-anchoring,
   boundary-derived heights, variable-width records with and without
   re-anchoring, and the final records design, each with its idea, its
   result, and what stopped it.
5. [Lessons](05-lessons.md): fixed-width fields and the second cost path they create,
   neighbour differences that turn moves into reads, and the board's
   blindness to big-integer time.
6. [The open problem](06-open-problem.md): what a correct fix must achieve,
   in which measures, and which instruments have to exist before anyone
   tries again.
7. [Evidence](07-evidence.md): the commits, branches, and logs behind every
   number, with the decisive log lines quoted, because the logs live in a
   session scratch directory that will not last.

## Summary

`min_ticks` evaluates `Σ leaf heights − Σ internal-node subtree minima` in
leaf order (section 1). For each minimum it tracks, `main` keeps a
*contribution*: the leaf's height as an offset from a frozen prefix, the
prefix's index, and a close count. The contribution is packed into one
64-bit word when the offset fits 32 bits, the index 23 bits, and the count
8 bits, and moved into a 48-byte slot with an arbitrary-precision offset
otherwise (section 2). On a right spine whose nested minima rise one level
at a time, every level suspends one contribution until the end. When the
spine is entered by one jump between `2^31` and about `2^288`, every
offset exceeds 32 bits, but the height-freezing rule never fires, because
it tolerates 256 bits of extra width. Every level therefore costs about 64
bytes of heap against about 6 bits of input (section 3).

The repairs failed in three ways (section 4):

- A repair that keeps a fixed-width field moves the failure to the far
  side of that field's capacity, creating a second *regime*: a range of
  inputs on which the code takes a different, costlier path. Re-anchoring
  kept every offset within 32 bits by creating a new prefix, and so moved
  the failure to the 23-bit prefix field. Past `2^23` prefixes, about
  66 MB of input, the failure returned at 6.1 GB of heap for a 91 MB
  input. The board samples inputs of tens of kilobytes, so it reported
  every cell passing.
- A repair that stores each suspended minimum as a difference from its
  neighbour, or derives it from a stored boundary, must read that
  neighbour or boundary on every push and pop, where `main` only moves an
  opaque word. A value the input pays for once can then be read once per
  cycle. The boundary-derived design read a circulating boundary on every
  pop, with a touch exponent of 1.97. The final records design read a wide
  outer neighbour on every push and pop, about `2k²` limbs for a `k`-tooth
  comb against `k` on `main`. The board's time measures do not count
  `num-bigint` work, so they read identical to `main`.
- Two variable-width record designs stopped under a rule that no fitted
  growth exponent may rise, on rises of 0.01 to 0.11. The audit later
  found that such rises can come from a smaller fixed cost rather than
  growth, and the board cannot yet tell the two apart.

The variable-width record designs brought the jump-entered spine from
124.9 to as low as 2.8 bytes per input byte, and the 91 MB input to
between 305 MB and 1.05 GB, so heap well under the ceiling is achievable.
What is missing is a design that gets there with per-event work
independent of the width of any long-lived neighbour, and an instrument
that would show whether a design does (section 6). The variable-width
design that kept re-anchoring (C1 in section 4) may be one; it was never
measured that way.

## Provenance labels

Factual claims in sections 2 to 7 carry one of three labels, inline or
as a blanket statement at the start of a passage.

- **(measured: *source*)**: a number printed by a run whose log or commit
  message I read. I ran nothing; the source names who ran it and where the
  output lives.
- **(code: *location*)**: a property I derived by reading the code at the
  cited commit.
- **(reported: *source*)**: an agent's statement that I did not
  corroborate against a log or the code.

Inferences and estimates say so in the sentence.

Code on `main` is cited at `e7e107a2`. Links into `crates/` are relative to
this file and resolve against whatever commit you view it at; the
`min_ticks` and `range_minima` code they point at is identical at
`5065caeb`, the base of every archived branch (code: `git diff --stat
5065caeb e7e107a2` over those paths is empty). Code on the archived
branches is cited as `git show <sha>:<path>`, with function names and line
ranges.
