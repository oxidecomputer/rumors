# 3. The transient-heap defect

## Terms and the instrument

*B/B* means bytes of peak transient heap per byte of encoded input. The
*amplification board* (the board) runs each public operation on each of a
set of generated input *families*, at a ladder of sizes, and records three
*currencies* per sample: *heap* (peak transient heap, read through the
`peak_alloc` allocator as peak minus baseline), *scan* (encoded bits read),
and *touch* (`suanpan` accumulator digits touched). A cell fails when any
sample's heap exceeds `1024 + 20 · n` bytes or a fitted growth exponent
exceeds 1.15
([`ceilings.rs`](../../crates/before/src/testing/meter/board/ceilings.rs#L7-L30)).
`peak_alloc` implements `realloc` as allocate, copy, free, so a growing
`Vec` briefly counts its old and new buffers together (reported:
`fixer-min-ticks-heap/NOTES.md`, "verified in source").

## The inputs that cause it

All of them are right spines whose nested subtree minima strictly rise:
each internal node's left leaf is its subtree's minimum, and each deeper
node's minimum is above its parent's. Every level therefore arms a range
above the current innermost minimum, pushes a positive boundary, and
suspends the outer contribution on it (section 2). Nothing ever undercuts,
so every suspended contribution stays live until the final `close_all`.
The input pays a few bits per level, because every change after the first
is small.

- **The plain rising spine**, leaves `0, 1, 2, …`. About 5 bits per level
  (reported: defect record).
- **The jump-entered rising spine** `JR(b, d)`: a right spine of `d`
  internal nodes whose leaves in order are `0, 2^b + 1, 2^b + 2, …,
  2^b + d`. Its encoded size is `6d + 2b` bits, so the jump is paid once
  and each level costs 6 bits (reported: the demonstration commit
  `08573e15`, whose committed property checks the closed-form size). The
  board samples it at `b = 40` as the family `jump-rising-spine`. That
  family exists only on the demonstration commit, which is not on `main`
  (code: no `jump-rising-spine` in `main`'s `crates/before/src`).
- **The real-scale input** the round-1 reviewer built:
  `stepped_spine(31, 2^23 + 16, 2^25)`, a rising spine whose first
  `2^23 + 16` levels each step up by `2^31` and whose next `2^25` levels
  step up by one. It encodes to 91,226,247 bytes (measured:
  `reviewer-min-ticks/run1-release.log`). It was built to probe the
  re-anchoring design of section 4, not `main`.

## The mechanism: what grows with what

Two mechanisms add up (code; first isolated in the defect record).

**Mechanism 1: one payload word per positive boundary, in a doubling
`Vec`.** Every suspended contribution is an 8-byte `StoredContribution` in
`Boundaries::payloads: Vec<P>`
([`boundaries.rs`](../../crates/before/src/version/range_minima/boundaries.rs#L37-L38),
pushed at
[line 85](../../crates/before/src/version/range_minima/boundaries.rs#L85)).
At 5 input bits per level that alone is 12.8 B/B. `Vec` doubling leaves
up to half the capacity unused, and the reallocation transient holds old
and new buffers at once.

**Mechanism 2: every contribution spills when its offset leaves `i32`, and
the freeze rule does not prevent it.** On `JR(40, d)` the first leaf is 0,
so prefix 0 is 0 and every later leaf's offset is `2^40 + k`, past
`i32::MAX`. The freeze test compares the live change's stored width, two
base-`2^32` digits, with the next change's width plus the allowance,
`1 + 8 = 9` digits, so it never fires
([`min_ticks.rs`](../../crates/before/src/version/measure/min_ticks.rs#L68-L72)).
Every stored contribution spills. The freeze rule exists to keep `L`
within 256 bits of recent changes, which is a time bound; the inline word
needs `L` within 32 bits, which is a space bound. Any jump between `2^31`
and about `2^288` falls between the two.

Per suspended level of `JR`, the live heap is (code, by layout):

| Item | Bytes | Count grows with |
|---|---|---|
| payload word in `Boundaries::payloads` | 8 | suspended minima |
| spill slot in `ContributionStore::slots` | 48 | suspended minima whose offset, prefix, or count leaves its field |
| the slot's `BigInt` digit buffer | 8 per 64 bits of offset | the same |
| the boundary itself (value 1, packed) | a few bits | positive boundaries |

That is about 64 bytes per level against 6 input bits (0.75 bytes), about
85 B/B live. `Vec` growth of the slot and payload vectors, counted as
`peak_alloc` counts reallocation, takes the peak higher. In short, the
transient heap is (simultaneously suspended minima) × (per-minimum cost),
and the per-minimum cost is 8 bytes when the packed word fits and about 64
when it spills. The input pays a few bits per suspended minimum in both
cases, so the ratio is a constant per case: about 13 to 40 B/B when the
word fits, and from about 85 B/B upward when it spills, rising with the
offset's width because the digit buffer holds the whole offset. The growth
is linear; the constant is what breaches the ceiling.

## The measured scale

**The board.** On the demonstration commit, whose `min_ticks` code is
identical to `main`'s (code: `git diff 5065caeb 08573e15` touches no
`min_ticks`, `range_minima`, `bits`, or `suanpan` file), the board marks
exactly one cell failing, `version_min_ticks × jump-rising-spine`, on heap
(measured: `fixer-min-ticks-heap/parent-board.log`, and the commit
message of `08573e15`):

| Sample | Input bytes | Heap |
|---|---|---|
| default scale | 5,011 → 10,011 | 124.9 B/B |
| top scale | 20,011 → 40,011 | 125.0 B/B |
| small input | 61 → 111 | 177.2 B/B |

Touch reads 12.8 per byte and scan 8.0 per byte with exponent 1.00, so
time is linear (same log).

**The auditor's probes** (measured: defect record, release profile, the
board's allocator accounting) show how the constant depends on the jump:

| Shape | Size `n` | Heap |
|---|---|---|
| plain rising spine | 1,025 | 19.7 B/B |
| plain rising spine | 1,026, 65,538, and 262,146 | 39.4 B/B |
| jump `2^16` (offsets fit `i32`) | 65,536 | 19.7 B/B |
| jump `2^31` | 1,024 to 65,536 | 120.5 to 122.1 B/B |
| jump `2^31` | 98,304 | 162.8 B/B |
| jump `2^200` | 98,304 | 196.8 B/B |

Here `n` is the probe's size parameter; the plain spine peaks just past
each power of two. Its readings fit mechanism 1 exactly: the peak of
12,656 bytes at `n = 1025` is `8 · (512 + 1024) + 368`, a `Vec` of 512
payload words being copied into one of 1,024, and the peak of 25,264 bytes
at `n = 1026` is `8 · (1024 + 2048) + 688`, the next doubling. (The defect
record's prose places these two readings at `n = 1024` and `n = 1536`; its
verbatim probe lines, quoted in section 7, place them at 1,025 and 1,026.)

**The real-scale input.** The 91 MB input above needed 6,135,222,240 bytes
of peak heap, 67.25 B/B, against a ceiling of 1,824,525,964 (measured:
`reviewer-min-ticks/run1-release.log`, release). That reading was taken on
the re-anchoring design `7222d49b`, after it had crossed its 23-bit prefix
field (section 4), **not on `main`**. No log shows `main` on this input. By
reasoning from the code, `main` spills every level of it from the second
level onward, through the offset field rather than the prefix field: the
first step is already `2^31`, and the live change is never frozen. At
about 64 bytes per level over its roughly 41.9 million levels, the live
footprint alone would be about 2.7 GB before any `Vec` growth slack. That
is an estimate, not a measurement.

## Why the board on `main` does not show it

`main`'s board has no family whose nested minima rise one level at a time
after a jump into the band between `2^31` and `2^288` (code: the family
was added by `08573e15`, which is not on `main`). The `ascend-cliff`
family has a rising shape and passes; the auditor infers that its first
leaf is itself wide, so it becomes prefix 0 and the later offsets stay
small (reported: defect record, "inferred, not measured").
`jump-rising-spine` would fail on `main` today, which is why it can only
land together with a fix: the owner's rules allow no mechanism for
accepting a known failure.

## What it is and is not

It is a constant-factor breach of the board's heap ceiling, up to about
10 times over, on a constructible family the board does not sample. The
values are always correct, and time stays linear. It re-opens an earlier
closed fix, whose goal was this ceiling (reported: defect record, citing
`.agent-notes/2026-09-16-before-triage-plan/checklist.md:159`). The owner's
summary of the status quo is that `main` "may use more transient heap than
hoped on certain corner-case inputs."
