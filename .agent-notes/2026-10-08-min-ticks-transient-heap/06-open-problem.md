# 6. The open problem

## What a fix must achieve

For every valid `Version` of `n` encoded bytes, a replacement for `main`'s
suspended-minimum storage must meet all of the following at once.

1. **Values.** `min_ticks` is unchanged: the tree oracle, the differential
   suites, and the law suites agree.
2. **Heap.** Peak transient heap is at most `1024 + 20 · n` bytes, counted
   as `peak_alloc` counts it, with reallocation holding the old and new
   buffers together. The denominator is input bytes, as on the board's
   `version_min_ticks` row.
3. **One regime.** No field has a fixed capacity whose crossing changes the
   per-element cost, unless the board samples an input past that capacity.
   An exact fallback counts as a second regime.
4. **Time.** Total work is `O(n)` in every currency: `suanpan` digit
   touches, encoded bits scanned, and `num-bigint` limb work. Stated per
   event, no push, pop, resume, or settle does work proportional to the
   width of a value that survives the event; wide reads are charged to
   input the event consumes or to a value it destroys.
5. **No unaccounted movement.** Compared exactly against `main`, family by
   family, every difference is classified as a constant shift or as
   growth, and every growing difference goes to the owner as a finding.

`main` meets 1 and 4 by argument (section 2), not by a `num-bigint`
measurement. It fails 2, and its packed word has three fixed-width fields
(section 5.1). Design D met 1 and 3, met 2 at board sizes (its
real-scale reading was taken on C2's commit), and failed 4. Design A met 1
and 4, met 2 only at board sizes, and failed 3, which is how it lost 2 at
scale. Design C1 met 1, 2, and 3, and 4 by its derivation; it was stopped
by a rule over rendered exponents, which item 5 would replace.

## The design space as the audit mapped it

A suspended minimum has to be able to recover its height when it resumes
or settles. The audit tried four ways to identify that height, and each
spends a different resource.

| The height is identified… | Space per suspended minimum | Time per push or pop | Tried in |
|---|---|---|---|
| as an offset from a frozen prefix | 8 bytes while the offset fits 32 bits; otherwise a 48-byte slot plus the offset, which may be as wide as the freeze allowance (256 bits) plus the change that reached the leaf | O(1): the record is moved | `main` |
| from the boundaries popped to reach it | none | the popped boundary's width, which a cycle can repeat | B |
| as a difference from the neighbouring record | the change across the stretch between the two leaves | the neighbour's width, which a cycle can repeat | C2, D |
| as above, with offsets kept within `i32` by re-anchoring | at most 33 bits of difference | O(1) | C1 (and A, with a packed word) |

Re-anchoring gets a narrow reference point by creating frozen components,
which are permanent: their heap persists after the minima that created
them settle, and each is multiplied once at settlement. That is the source
of design A's heap exponent changes on `dominated-undercut`, `jump-pair`,
and `cancelling-chain`, which the owner accepted (reported: commit message
of `7222d49b`), and C1 keeps re-anchoring.

The first row shows where the defect comes from (reasoned from the code).
The freeze allowance is sized for time: it keeps the fold from freezing
over and over on small fluctuations in width. A stored offset can be as
wide as that allowance, while the input may pay only a few bits for the
level that stores it, so any design that stores one offset per suspended
level at the offset's own width can hold up to 288 bits (nine base-`2^32`
digits) per 6 input bits, 48 B/B before any overhead. `main` avoids that cost for offsets
within 32 bits and pays a 48-byte slot beyond them, which is most of
section 3's 125 B/B. Narrowing the allowance would shrink stored offsets
and create more frozen components; nobody has measured that trade.

## Instruments that must exist first

Each item names the failure it detects. None of them is built.

1. **A time meter over `num-bigint` work**, with the reviewer's
   cross-prefix and same-prefix combs as committed families. It detects
   design D's quadratic limb work, which every current meter misses. Before
   it is trusted, design D must be shown to fail it and `main` to pass it
   (section 5.3; the instrument survey's candidate 1.1, which ranks it first
   and makes it wait on question 87).
2. **The `jump-rising-spine` family on the board.** It detects the defect
   itself. It exists on the demonstration commit `08573e15` and fails on
   `main`, so it lands together with a fix, not before.
3. **A sampled input past every fixed-width threshold** in the chosen
   design, or no such thresholds. It detects the failure of design A. The
   enumeration of thresholds across `before` and `suanpan` that the audit's
   state file proposes has not been done.
4. **Exact capture and comparison of board runs** (question 95). It
   separates a smaller fixed cost from growth, which stopped C2 and may
   have stopped C1 for the wrong reason (section 5.4).
5. **An on-demand real-scale probe**: the stepped spine of section 3 at
   about 91 MB, run on demand and outside `just gate`. It detects a
   regime split that item 3 failed to enumerate. It is a developer check, not a substitute for item
   3, because the board cannot afford it.

## Questions for the owner when this reopens

These are recorded, not asked now.

1. **Is design C1 worth re-measuring once instruments 1 and 4 exist?** It
   is the only design built that has one regime and O(1) work per push and
   pop by construction, and it met the heap ceiling on the 91 MB input
   (1.05 GB against 1.82 GB). It was stopped on touch exponent rises of
   0.01 to 0.03, one cause of which the fixer attributes to growth on
   sublinear families, and on `bigroot`'s heap exponent rising from 0.98
   to 1.00. It was never adversarially reviewed or run against the comb
   probes. Its one threshold, the re-anchoring trigger, is crossed at
   every level of a family the board samples. My recommendation is to
   re-measure it first, because it is the cheapest experiment that could
   close the problem; it was built under the owner's earlier ruling and is
   not the per-push repair the owner suspects.
2. **What counts as regression on a family whose cost was sublinear?**
   Re-anchoring adds a linear term to `dominated-undercut` (touch exponent
   0.85 to 0.88). The result stays linear in the input, within the crate's
   contract, but it is an asymptotic change on that family. The stop rule
   treated it as a regression; a ruling either way would settle that part
   of C1's stop.
3. **Should the freeze allowance be revisited?** The allowance bounds how
   wide a stored offset can be relative to the input that paid for its
   level. A smaller allowance trades heap per stored offset for more
   frozen components; no one has measured it.
