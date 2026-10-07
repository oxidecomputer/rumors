# Defect D1: `Version::min_ticks` transient heap exceeds the board's heap ceiling by up to 10x

Auditor: auditor-l3. Severity: resource (constant factor), no wrong answers. Linear in input,
but 2x (plain) to 10x (jump-entered) over the enforced per-sample ceiling, on a constructible
family the amplification board does not sample. It re-opens the closed triage fix "Apply the
global transient-heap ceiling to `min_ticks` by storing its open minimum boundaries and leaf
contributions compactly, with exact fallbacks" (`.agent-notes/2026-09-16-before-triage-plan/checklist.md:159`).

## Contract clause breached

- Crate docs, `crates/before/src/lib.rs:335-340` at 58285ca5 (the paragraph after
  "Space Efficiency"): "Even on pathologically shaped inputs, the auxiliary space required to
  compute any operation is at most a small constant multiple of the input size. ... Any operation
  which requires more than a small amount of temporary memory (relative to its input size) is a
  bug: please report it!"
- The board's operationalization of "small constant multiple", which the closed fix targeted:
  `crates/before/src/testing/meter/board/ceilings.rs:15-30`, per-sample heap ceiling
  `HEAP_INTERCEPT_BYTES + MAX_HEAP_BYTES_PER_INPUT_BYTE * D` = `1024 + 20 * n` for the
  `version_min_ticks` cell, which uses the default input-byte model
  (`testing/meter/board/ops.rs:1166-1179`).

## Reproduction (verified)

Explore branch `explore/l3-events` at `1a8601b3`, probes in
`crates/before/src/version/tick/l3_heap.rs` (a `peak_alloc::PeakAlloc` global allocator in the
lib test binary, read exactly as the board reads it: reset peak, baseline current, run, peak minus
baseline; release profile, the board's profile of record).

```
on-illumos.sh /Users/oxide/src/rumors-audit-l3-events 'unset CARGO_TARGET_DIR; nice -n 10 cargo nextest run --locked -p before --all-features --release --build-jobs 12 --test-threads 1 --no-capture -E "test(/l3_heap_min_ticks_rising_spine_fine/)"'
```
exit 0 (diagnostic test; it prints and passes). Verbatim lines (log `run-heap-release-2.log`):

```
FINE min_ticks rising-right-spine n=1025 stored_bytes=641 encoded_bytes=641 peak=12656 ratio=19.74 board_ceiling=13844 green
FINE min_ticks rising-right-spine n=1026 stored_bytes=642 encoded_bytes=642 peak=25264 ratio=39.35 board_ceiling=13864 RED
FINE min_ticks rising-right-spine n=65538 stored_bytes=40962 encoded_bytes=40962 peak=1613872 ratio=39.40 board_ceiling=820264 RED
FINE min_ticks rising-right-spine n=262146 stored_bytes=163842 encoded_bytes=163842 peak=6455344 ratio=39.40 board_ceiling=3277864 RED
```

```
on-illumos.sh /Users/oxide/src/rumors-audit-l3-events 'unset CARGO_TARGET_DIR; nice -n 10 cargo nextest run --locked -p before --all-features --release --build-jobs 12 --test-threads 1 --no-capture -E "test(/l3_heap_min_ticks_jump/)"'
```
exit 0. Verbatim (log `run-heap-release-3.log`):

```
JUMP min_ticks jump=2^16 n=65536 encoded_bytes=40965 peak=806960 ratio=19.70 board_ceiling=820324 green
JUMP min_ticks jump=2^31 n=1024 encoded_bytes=649 peak=78192 ratio=120.48 board_ceiling=14004 RED
JUMP min_ticks jump=2^31 n=4096 encoded_bytes=2569 peak=312624 ratio=121.69 board_ceiling=52404 RED
JUMP min_ticks jump=2^31 n=16384 encoded_bytes=10249 peak=1250352 ratio=122.00 board_ceiling=206004 RED
JUMP min_ticks jump=2^31 n=65536 encoded_bytes=40969 peak=5001264 ratio=122.07 board_ceiling=820404 RED
JUMP min_ticks jump=2^31 n=98304 encoded_bytes=61449 peak=10002480 ratio=162.78 board_ceiling=1230004 RED
JUMP min_ticks jump=2^200 n=98304 encoded_bytes=61491 peak=12100312 ratio=196.78 board_ceiling=1230844 RED
```

## The failure family

Shape: a right spine of internal nodes whose preorder leaves are `a_0, a_1, …, a_n` with
`a_{k+1} > a_k` (each nested subtree arms its minimum strictly above its parent's), coded at
about 5 bits per level (node flag, leaf flag, `gamma` of a `+1` delta).

- Appears: plain rising spine `(0, (1, (2, …)))` at every size from 2^8 up, peaking at
  39.4 B per input byte when the level count just passes a power of two (`n = 2^k + 2`), 26.3 at
  `1.5 * 2^k`, 19.7 just below `2^k + 2`.
- Appears, larger: the same spine entered by one wide code, `(0, (J + 1, (J + 2, …)))` with
  `2^31 <= J < 2^288`: 122 B/B steady and up to 163 (J = 2^31, 2^40) or 197 (J = 2^200) B/B.
- Disappears: entry jump `J = 2^16` (offsets stay inside `i32`): the plain sawtooth only.
- Disappears: any shape whose nested minima are equal or descending (zero runs and undercuts store
  no payload; inferred from `RangeMinima::finish_arming`, which pushes a payload only on a positive
  difference).
- The board's `AscendCliff` family has the rising shape and is green (baseline.md). Inferred, not
  measured: its first leaf is itself wide (`2^b + 1`) and becomes the opening height prefix, so its
  later offsets are small and stay inline; and its sampled level counts fall on the low side of the
  sawtooth.

## Root cause (isolated; two mechanisms)

1. **One 8-byte payload per positive boundary, grown by doubling.** `min_ticks` instantiates
   `RangeMinima<StoredContribution>` (`version/measure/min_ticks/minima.rs:115`), the only
   non-zero-sized payload in the crate. Each positive boundary suspends the outer range's current
   contribution in `Boundaries::payloads: Vec<P>` (`version/range_minima/boundaries.rs:38`, pushed at `:85`). At 5 input
   bits per boundary that is 12.8 B/B at full capacity; `Vec` doubling leaves up to 2x slack, and
   a reallocation under the board's allocator accounting (`peak_alloc` implements `realloc` as
   allocate, copy, free) holds old and new buffers at once. The readings fit this exactly:
   `12,656 = 8 * (512 + 1024) + 368` at n = 1024; `25,264 = 8 * (1024 + 2048) + 688` at n = 1536.
2. **Every contribution spills when its offset leaves `i32`, and the freeze rule cannot prevent
   it.** `StoredContribution::inline` (`version/measure/min_ticks/minima/contributions.rs:83-91`)
   keeps a contribution in one word only while the leaf's offset from its frozen height prefix fits
   `i32`; otherwise `ContributionStore::spill` stores an `Option<Contribution>` (a `BigInt` offset,
   a prefix index, a close count) in a `Vec` plus the `BigInt`'s digit allocation. The live height
   change is frozen into a new prefix only when it is more than `HEIGHT_FREEZE_ALLOWANCE_DIGITS`
   = 8 digits (256 bits) wider than the next delta (`version/measure/min_ticks.rs`, the freeze test
   in `min_ticks_for`, `version/measure/min_ticks.rs:66-72`; `version/measure.rs:21-26`). So any live change between `2^31` and about
   `2^288` makes every subsequent leaf contribution spill, and one entry code pays for all of them.
   Same shape, same boundary count, same input size within 4 bytes: J = 2^16 reads 19.7 B/B and
   J = 2^31 reads 122 B/B.

## Dependence on the allocator accounting

The plain spine's peak above 26 B/B comes from the reallocation transient, which `peak_alloc`
counts as old plus new buffers; an allocator that grows in place would read lower there, though
`Vec` capacity slack alone still reaches about 25.6 B/B. The jump-entered family stays far over the
ceiling without any transient: by arithmetic (inferred, not separately measured), its live footprint
at the end of the walk is about 64 bytes per level (a 48-byte spill slot, the offset's 8-byte digit
buffer, the 8-byte payload) against about 5 input bits per level, roughly 100 B/B, five times the
ceiling; the measured 122 B/B peaks add the last slot-vector growth. The board reads heap through
`peak_alloc`, so the board's verdict is the one quoted.

## Public API and formats

A fix changes only `min_ticks`'s transient representation: no public API, wire, or storage
format change.

## Classification question for the coordinator

The common brief ranks asymptotic findings first and calls constant factors observations; this is
linear growth with a constant 2x to 10x above the board's committed ceiling, on a family the board
does not sample, and it re-opens a closed fix whose stated goal was that ceiling. I report it as a
defect against the enforced resource contract. If the owner instead treats 20 B/B as a regression
limit only for sampled families, this becomes an adequacy finding (the board misses a family) plus
a constant-factor observation.

## Owner's guidance (provisional)

The owner found the coordinator's recommendation reasonable at first blush:
repair mechanism 2 by also freezing when the next stored contribution's
offset would leave the inline `i32` range, and repair mechanism 1 with the
fix note's repair 1, a segmented LIFO for the payload stack so growth never
reallocates. The fix note's repair 2 (recomputing payloads instead of
storing them) becomes an observation, not part of this fix. The guidance is
provisional: the fixer measures the result against the board's heap ceiling
and reports, and anything the measurements contradict returns to the owner.

## Owner's ruling on the measured costs

The fixer built both repairs from the provisional guidance and measured them.
Together they turn `version_min_ticks × jump-rising-spine` green (14.3 B/B at
the default scale), with values unchanged. The extra freeze costs other
families: the `seam_plunge` touch reading rises about 10% past its pinned
ceiling (29,752 against 26,945), three families' heap goes from constant to
linear in input (`dominated-undercut`, `jump-pair`, `cancelling-chain`, all
still green), and `propagate-seam` becomes the heap worst case. Each freeze
adds a permanent frozen component, at most one per 63 input bits. The owner
accepted these costs rather than add machinery to free dead components: land
both repairs, raise the `seam_plunge` ceiling with its reason, re-pin the
heap worst case, and annotate the exponent changes.

**Superseded, pending a re-ruling.** The ruling above rested on an incomplete
statement of the costs. The `seam_plunge` rise is +52.5% against the parent
(19,503 to 29,752 touches), not only "about 10%" past the ceiling; and the
same test's `DESCENDING_PARK_SURPLUS_BAND` inverts (control minus plunge goes
from +2,021 to −1,060), which needs a negative floor. A cheaper variant also
exists. The owner is re-asked with these facts (QUESTIONS #47 at the time of
writing).

## Owner's re-ruling

With the costs stated fully, the owner chose the deferred variant
(`both-deferred.patch`) together with the chunked payload stack:

- A leaf's offset enters the result only after its minima are observed, so a
  re-anchored leaf adds nothing and no wide add-then-subtract occurs.
- The freeze-position liveness floor gets a premise derived from work the
  fold cannot avoid, demonstrated live by a dead-meter variant.
- The descending-park surplus band is re-centered, with a committed
  demonstration that the per-hop residue regression it guards still trips it.
- The input that re-anchors at every level becomes a new board family, so the
  cost of permanent components is checked from now on, not described in prose.
  The regime past the inline prefix field's capacity is to be made reachable
  by a committed check if that can be done cleanly; otherwise it returns to
  the owner.
- No performance values appear in tree prose; docs and comments state
  mechanisms and bounds.
