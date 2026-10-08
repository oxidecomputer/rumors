# 4. The attempted designs

Five designs were built and measured, on six commits. All of them
compute correct values; each stopped for a cost reason. In the table, "JR"
is the board's `jump-rising-spine` cell at the default scale (124.9 B/B on
`main`'s code), and "91 MB" is the real-scale input of section 3.

| Design | Commit | JR heap | 91 MB input | Time | Stopped because |
|---|---|---|---|---|---|
| A. Re-anchoring, packed word, chunked stack | `7222d49b` | 14.3 B/B | 6.1 GB (67.25 B/B) | linear on the board | the 23-bit prefix field overflows past `2^23` prefixes, out of the board's reach |
| B. Boundary-derived heights | `966a263e` | 14.3 B/B | not run | quadratic in touches on `reveal-comb` (exponent 1.97) | quadratic |
| C1. Variable-width records with re-anchoring | `9088ba94` | 2.8 B/B | 1.05 GB (11.49 B/B) | linear by derivation; touch exponents up 0.01 to 0.03 | the rule that no exponent may rise |
| C2. Variable-width records without re-anchoring | `61f55dcb`, `de4ac861` | 5.1 B/B | 305 MB (3.34 B/B) | touch and scan identical to `main` | heap exponent rises (rule as above) |
| D. The final records design | `1a31f8d1` | 5.1 B/B | not run; C2 read 305 MB | quadratic in `num-bigint` limbs on a constructed comb | quadratic, and invisible to the board |

(Measured: board logs `r1-board.log`, `rd-board.log`, `rv2-board.log`,
`rr1-board.log`, and `rf1-board.log` in `fixer-min-ticks-heap/`; the
real-scale readings from `reviewer-min-ticks/run1-release.log`, `rv2.log`,
`rr1.log`, and `rc1.log`. Section 7 quotes the lines.)

All five sit on top of the demonstration commit `08573e15` (earlier signed
as `ba094bbe`), which adds the `jump-rising-spine` family and changes no
`min_ticks` code. "Parent" below means that commit, and so `main`'s
implementation.

---

## A. Re-anchoring with the packed word (`7222d49b`)

**Idea.** Repair each mechanism of section 3 directly.

- For mechanism 2, *re-anchor*: when a leaf that is about to be stored as
  a minimum has an offset outside `i32`, make its whole offset a new frozen
  component, so the leaf sits at offset zero on the new prefix and the
  live change restarts
  (`git show 7222d49b:crates/before/src/version/measure/min_ticks/minima/heights.rs`,
  `HeightPrefixes::narrow`, lines 106–141). Every stored offset then fits
  the packed word.
- For mechanism 1, store payloads and wide boundary values in a
  `ChunkedStack`, a stack of chunks that double up to 4 KiB and never move,
  so growth never copies
  (`range_minima/chunked_stack.rs` on the same commit).

**Why it seemed right.** It kept `main`'s architecture and changed only the
two places the defect record isolated. Each re-anchoring follows at least
`2^31` of height change since the previous freeze, which costs at least 63
input bits, so components grow by at most one per 63 input bits (reported:
commit message, with the cost argument from the fix note).

**How it got there.** The fixer first built both repairs as the owner's
provisional guidance described, with the re-anchoring applied eagerly.
Together they fixed JR (14.3 B/B), but the extra freezes cost other
families: three families' heap went from constant to growing with input
(`dominated-undercut` exponent 0.00 to 0.70, `jump-pair` 0.02 to 1.00,
`cancelling-chain` 0.06 to 0.82), and the `seam_plunge` touch meter rose
52.5% against the parent (19,503 to 29,752 touches) (measured:
`fixer-min-ticks-heap/both-compare.txt` and `meter-diff.txt`, summarised in
the fixer's notes). A frozen component is permanent, while the spill slot
it replaces was recycled when its minimum settled; that is the cause the
fixer gives (reported). The owner accepted the heap growth and chose a
deferred variant, in which a leaf's offset enters the answer only after
its minima are observed, so a re-anchored leaf adds nothing and no wide
value is added and later subtracted (defect record, "Owner's re-ruling").
The commit also added a board family, `wide-step-spine`, in which every
level re-anchors.

**Result.** JR fell from 124.9 to 14.3 B/B at the default scale, 14.1 at
the top scale, and 23.9 on the small sample; the board read 5,493 cells
passing and none failing (measured: `r1-board.log`; the commit message).

**What broke it.** Re-anchoring moves the overflow from the offset field to
the prefix field. Each re-anchoring adds a prefix, and the packed word
holds a prefix index in 23 bits. Past `2^23` prefixes, every new
contribution spills again into a 48-byte slot, which is mechanism 2 again.
The round-1 reviewer measured it on production code with the stepped spine
of section 3:

| Input | Bytes | Peak heap | Verdict against `1024 + 20n` |
|---|---|---|---|
| `2^23 + 16` levels stepping by `2^31` | 70,254,727 | 1,077,678,048 (15.34 B/B) | passes |
| the same, then `2^25` unit levels | 91,226,247 | 6,135,222,240 (67.25 B/B) | fails |

(Measured: `reviewer-min-ticks/run1-release.log`, release.) The reviewer
also forced the inline prefix bound down to 1 to reach the spill regime at
small sizes, and JR then read 125.1 to 125.7 B/B, the original defect's
figure (measured: same log, as summarised in the reviewer's notes). The
board cannot reach this regime: `2^23` re-anchorings need at least
`2^23 × 63` input bits, about 66 MB, and the board's largest samples are
tens of kilobytes. The fixer had predicted the overflow in its notes
before the review, as an unmeasured inference. The branch's unit tests ran
the fold with a narrower inline bound, so the spill path's values were
tested, but the board, which measures cost, runs production code.

**Ruling.** The owner rejected landing a fix that holds in one regime and
fails in another, because the split gives false confidence, and asked for
a single-regime redesign (defect record, "Owner's ruling on the regime
past the inline prefix field").

---

## B. Boundary-derived heights (`966a263e`)

**Idea.** Store no height for a suspended minimum at all. `RangeMinima`
already stores the difference between adjacent minima, so when a suspended
minimum resumes, its height is the current minimum minus the boundary just
popped. A suspended minimum's payload becomes only its close count, a
`u64`. Only the current minimum carries an explicit height. This is the
fix note's "repair 2", promoted by the owner's ruling.

The fixer derived the design before writing code
(`fixer-min-ticks-heap/redesign.md`, Part I). A *chain* holds the base
leaf's `(prefix, offset)`, the sum `S` of boundaries popped since the
chain began, and close counts; the innermost minimum is always
`F_p + o − S`. Settling a chain uses Abel summation, so the wide sum `S`
is multiplied once per chain rather than once per minimum
(`git show 966a263e:crates/before/src/version/measure/min_ticks/minima.rs`,
`Chain::descend` and `MinimumClient for Chain`, lines 245–305). The
invariant the design rests on, that the boundary stack holds exactly
`m_{i+1} − m_i` for each adjacent pair at every resume point, was checked
case by case and asserted in a `range_minima` property.

**Why it seemed right.** It has no fixed-width field, so it has one
regime, and it stores nothing that is already stored. The cost argument
charged each popped boundary's read to the input that created it.

**Result.** Values matched the tree oracle (755 of 755 non-meter tests).
JR read 14.3 B/B, the same as design A, on which it was built (measured:
`rd-board.log`). But the board failed
`version_min_ticks × reveal-comb` on touch:

| Sample | Touch per byte | Exponent |
|---|---|---|
| parent | 6.9 | 1.00 |
| spike, 751 → 1,501 bytes | 219.4 | 1.97 |
| spike, 3,001 → 6,001 bytes | 840.1 | 1.97 |

The committed meter `min_ticks reveal_comb` read 330,335 touches on its
small run against the parent's 11,366 (measured: `rd-board.log`,
`rd1.log`; redesign notes section 5).

**What broke it.** The cost argument assumed that `RangeMinima` bounds the
total width of the boundaries it ever stores by the input. It does not: it
bounds the width it holds *at once*. Because it moves boundary buffers
without reading them (section 2), a boundary can be created and popped
again and again while the input pays its width once. `reveal-comb` is a
committed family built for exactly this: `k` sites each arm a width-`b`
boundary above a shared floor and pop it when the site closes, with no
input change funding each cycle
([`reveal_comb`](../../crates/before/src/testing/meter.rs#L1347-L1365)).
The chain reads every popped boundary twice (`S += b` and `Y += C·b`), so
it does `k · b` work on an input of about `k + b`. On `main` a suspended
minimum keeps its own narrow record, which is moved, not read, at each
pop. The fixer's conclusion: any scheme that derives a resumed minimum's
height from the popped boundary reads that boundary at every pop, so the
defect is in the premise, not a detail.

**Ruling.** `min_ticks` must not ship with a quadratic case; keeping the
current implementation is much better than that (defect record, "Owner's
ruling on the boundary-derived spike").

---

## C. Variable-width records

The owner's next ruling: keep re-anchoring, and replace the packed word
and its spill store with variable-width records, so that no field has a
capacity the board cannot reach; stop if this works only by tolerating a
small family of unlikely inputs, because asymptotics must never regress to
fix a constant multiple (defect record, "Owner's ruling on the
boundary-derived spike").

**The record.** Each suspended minimum is a *record* `(prefix, offset,
closes)`. The innermost record is held explicitly. Outer records go on a
`PackedU64Stack`, and each stores its prefix and offset as differences
from the record directly outside it, every field at its own width. The
stack keeps the top record's absolute height, so a push encodes against it
and a pop restores the next one. `RangeMinima` loses its payload type: it
only tells the client when to suspend, retire, and resume, in stack order
(`redesign.md`, Parts II and III).

**The space argument.** An outer minimum's leaf precedes the inner
minimum's leaf, because the outer range's minimum lies outside the
still-open inner range. Prefix indices only grow, so prefixes never
decrease up the stack, and the stretches of input between consecutive
records' leaves are disjoint. A same-prefix offset difference is the sum
of the stored changes in its stretch, so the stretch's codes pay for its
width. A cross-prefix difference is bounded by the freeze rule and charged
to the freeze in the stretch (`redesign.md`, sections 7, 8, and 13). On JR
every difference after the first is 1, though every offset is 41 bits.

### C1. With re-anchoring (`9088ba94`)

Re-anchoring keeps every offset within `i32`, so every offset difference
fits 33 bits and each push and pop is O(1) word operations (`redesign.md`,
section 8; `git show 9088ba94:crates/before/src/version/measure/min_ticks/minima/suspended.rs`).

**Result.** Values correct (764 of 764 tests); every committed meter reading
identical to design A; the board 5,493 passing; JR 2.8 B/B. On the
real-scale input, 14.49 B/B (1.02 GB) on the 70 MB prefix part and 11.49
B/B (1.05 GB) on the 91 MB input, both under the ceiling (measured:
`rv2-board.log`, `rv2.log`). Its one threshold, the `i32` re-anchoring
trigger, is crossed at every level of the `wide-step-spine` family that
design A added to the board, which read 12.4 B/B there (measured:
`rv2-board.log`). I infer that most of the remaining gigabyte is the
permanent frozen components that re-anchoring creates, one per wide level;
no measurement attributes it.

**What stopped it.** The fixer applied the owner's ruling as a stop rule:
no growth exponent may rise against the parent. Several did (measured:
board logs, as listed in the commit message):

- touch, `dominated-undercut` 0.85 to 0.88 and `hoisted-window` 1.00 to
  1.01, from re-anchoring itself: each site's component is settled once at
  its width, which adds a linear term to a family that grew as about
  `n^0.85` (reported: the fixer's attribution);
- touch, `comb-scatter` 0.90 to 0.91 and `wide-tooth-comb` 0.88 to 0.89,
  from replacing the live accumulator instead of resetting it (reported);
- heap, `bigroot` 0.98 to 1.00, from the records' doubling word vectors
  (reported).

The step to C2 was the coordinator's, not an owner ruling: drop
re-anchoring, whose own costs caused most of these rises (`redesign.md`,
Part III heading).

### C2. Without re-anchoring (`61f55dcb`, then `de4ac861`)

Without re-anchoring, offsets have arbitrary width, so `Δo` is stored at
its own width as sign and magnitude limbs. The freeze rule still bounds
every offset to within 288 bits of the change that reached its leaf
(`redesign.md`, sections 11 to 14).

**Result.** Values correct (762 of 762); every meter reading equal to the
parent's; no touch or scan reading moved anywhere on the board. Of 88 heap
readings that moved, most fell: JR 124.9 to 5.1 B/B, `ascend-cliff` 11.4
to 3.5, `propagate-seam` 14.2 to 9.0. The real-scale input read 305 MB:
4.34 B/B on the 70 MB part and 3.34 B/B on the 91 MB input (measured:
`rr1-board.log`, `rr1.log`).

**What stopped it.** Five heap exponents rose at both scales:
`bigroot` 0.98 to 1.00, `cancelling-chain` 0.06 to 0.07, `memo-fanout`
0.93 to 0.95, `reveal-hifloor` 0.19 to 0.28, and `tooth-tail` 0.80 to 0.91
(measured: `rr1-board.log`, as listed in the commit message). A heap probe
found that `bigroot` pushes no records at all, while `tooth-tail` holds up
to 64 records with offsets 2,049 to 16,385 bits wide and `reveal-hifloor`
up to 1,024 records with offsets up to 4,000 bits (measured:
`fixer-min-ticks-heap/hp.log`). Encoding each record against its inner
neighbour instead (`de4ac861`) moved only `cancelling-chain`.

The coordinator later read these rises as changes in fixed allocation, not
growth: with heap about `a + b·n`, shrinking `a` raises the two-point
log-log slope toward 1 with no change in asymptotics (reported:
`instrument-survey.md` section 1.7 and `STATE.md`). No exact comparison of
the two runs exists to settle it; the board renders exponents to 0.01 and
densities to 0.1.

---

## D. The final records design (`1a31f8d1`)

**Idea.** C2, with each record encoded against its outer neighbour again,
a push and a pop that compute in place, and a pop that decodes the stored
difference into one digit buffer
(`git show 1a31f8d1:crates/before/src/version/measure/min_ticks/minima/suspended.rs`,
`push` at lines 64–83 and `pop` at lines 85–103).

**Result.** Values correct (762 tests); three reversible mutations of
`push` (prefix difference against the origin, offset difference against
the origin, a lost close count) failed 27, 30, and 24 committed tests
(reported: commit message; the swap log `mu2.log` exists). The board read
5,402 passing; only heap readings moved, and JR read 5.1, 5.1, and 5.6
B/B (measured: `rf1-board.log`). Decoding each popped difference into one
buffer, rather than through three, lowered `tooth-tail`'s heap exponent
from 0.91 to 0.88 and `reveal-hifloor`'s from 0.28 to 0.25, still above
`main`'s 0.80 and 0.19 (reported: commit message). The real-scale probe was
not run on this commit: its log `rf1.log` has no probe line. The defect
record's "6.1 GB → 305 MB" for this branch carries over C2's measurement,
which used the same records with different push and pop arithmetic.

**What broke it.** The final reviewer found that the design is quadratic in
big-integer work on constructible inputs. A push computes
`top.offset − outer.offset`, and a pop computes `top.offset − difference`,
each at the width of the wider operand. When a narrow record is pushed and
popped many times directly above an outer record whose own offset is wide,
every cycle pays that width. On `main` the same cycle moves an 8-byte word
in `RangeMinima`'s payload column and reads nothing.

The reviewer's probe (`reviewer-min-ticks-final/reviewer_probe.rs`) builds
a left comb of `k` teeth under an outer minimum whose offset is about
`2^(64k)`, in two variants: *cross-prefix*, where the outer minimum keeps a
wide offset on prefix 0 and the comb's records sit on the next prefix near
offset 0; and *same-prefix*, where they share a prefix and the outer
offset is `1 − 2^(64k)`. A temporary counter added the 64-bit limbs of both
operands at the records' push and pop on the branch, and at the
contribution store on the parent (measured: `run1-branch.log`,
`run2-parent.log`, debug profile):

| k | Input bytes (cross-prefix) | Limbs, branch | Limbs, parent | Heap, branch | Heap, parent |
|---|---|---|---|---|---|
| 250 | 4,314 | 125,751 | 251 | 4.83 B/B | 4.82 B/B |
| 500 | 8,626 | 501,501 | 501 | 4.75 B/B | 4.73 B/B |
| 1,000 | 17,251 | 2,003,001 | 1,001 | 4.71 B/B | 4.69 B/B |
| 2,000 | 34,501 | 8,006,001 | 2,001 | 4.69 B/B | 4.67 B/B |

The branch reads about `2k²` limbs where the parent reads `k`. The
same-prefix variant reads the same pattern (8,007,997 against 2,000 at
`k = 2,000`). Answers agree bit for bit, and heap is flat in both. The two
counters sit at different call sites, so the comparison is between the
work each design does where it stores a suspended minimum, not one
instrument applied twice.

**Why the space argument did not cover time.** The argument of C charges
each record to the stretch of input between its leaf and its outer
neighbour's leaf, and those stretches are disjoint *along the stack at one
moment*. That bounds the bits stored at any instant. Over time, a
different narrow record occupies the slot directly above the same wide
outer record at every cycle, and each one is charged to the same stretch,
whose wide change the input paid for once. Space is a statement about one
moment; time sums over moments. (Reasoned from the code and the probe.)

**Why the board did not see it.** The board's time currencies are touch,
which counts `suanpan` accumulator digit work only
([`accumulator.rs`](../../crates/suanpan/src/accumulator.rs#L42-L49)), and
scan, which counts encoded bits read. The records' arithmetic is
`num-bigint` arithmetic, which neither counts, so every touch and scan
reading equalled the parent's while the limb work grew quadratically
(code; the instrument survey's section 1.1 says the same, verified there by
grep).

**Other findings of the final review** (reported:
`reviewer-min-ticks-final/NOTES.md`), recorded because a future attempt
will meet them:

- The `ChunkedStack` came from design A and is not needed by this one;
  the fixer's own probe showed mixed moves off the board.
- `wide-arming`'s heap density rise, 2.9 to 4.0 B/B, matches the retained
  capacity of the `BitStack` columns holding two wide differences, values
  plus unary widths, rounded up to a power of two.
- `PackedU64Stack` stores widths in unary, so a full 64-bit limb costs 128
  bits, and a pop of width 62 or more takes a bit-at-a-time loop.
- A comment in `suspended.rs` at lines 75–76 is false when the outer
  offset is zero, because `num-bigint` then clones.
- A test comment refers to "one contribution", a name the design removed.

**Ruling (question 87).** Stop. `main`'s `min_ticks` stays. The owner
suspects that the proposed per-push repair, "a short packed difference or
a whole record", reintroduces a second regime that regresses, the pattern
rejected after design A (defect record, "Owner's ruling: stop").

I read the suspicion this way (an inference; the proposal itself is not in
the records I had): choosing per push between a difference and a whole
record needs a rule. Records stored whole put one wide offset per level
back on a JR-like spine, which is section 3's mechanism in a packed form;
with a jump near `2^200` that is about 200 bits per level against 6 input
bits, over 30 B/B. Records stored as differences keep the quadratic case.
Whatever threshold separates the two choices separates two cost regimes,
and the board samples only one side of any threshold it cannot reach.
