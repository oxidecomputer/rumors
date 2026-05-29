# Interval Tree Clocks (`itc`) — Implementation Plan for Claude Code

This document is an execution plan for Claude Code. It specifies a production-grade,

safe-Rust implementation of Interval Tree Clocks (Almeida, Baquero & Fonte, 2008)

using a packed `BitVec` storage form, a transient fixed-width working form for

mutation, and an ergonomic linear-typed public API. Tests come first and are

comprehensive from the start, checked against an embedded reference oracle.

**The design below is frozen.** Do not redesign the API or representations. If a

genuine blocker is found, stop and surface it rather than improvising a different

design.

---

## 0. How to execute this plan

Operating protocol (apply throughout):

1. **Plan, then code.** Re-read the relevant plan section before each phase.

2. **Test-driven development, strictly.** For every phase, write the listed tests

   **first** and run them to confirm they **fail/`todo!()`-panic**, *before* writing

   any implementation. Do not write implementation code ahead of its tests. Tests

   drive the design; they are not after-the-fact validation.

3. **Small, verifiable increments.** Implement the minimum to make the current

   phase's tests pass. Do not add features or optimizations a phase's tests don't

   require. Do not refactor unrelated code.

4. **Verification gate before every commit:**

   ```

   cargo fmt --all

   cargo clippy --all-targets --all-features -- -D warnings

   cargo test --all-features

   ```

   All three must be clean. Then commit with a descriptive message (one commit per

   phase, or per logical sub-step).

5. **Review the risky parts.** After Phase 5 and Phase 7, do a dedicated review

   pass (a fresh subagent acting as a skeptical reviewer) over `grow`, the

   decode validator, and every traversal's iterativeness.

Definition of Done is in §9. Do not consider the project complete until every box

there is checked.

---

## 1. Project setup (Phase 0 prerequisites)

- Crate: library named `itc`, edition 2021, recent stable Rust.

- Crate root attributes:

  ```rust

  #![forbid(unsafe_code)]

  #![warn(missing_docs)]

  ```

- `Cargo.toml`:

  ```toml

  [dependencies]

  bitvec = "1"

  serde = { version = "1", optional = true, default-features = false, features = ["derive", "alloc"] }

  [dev-dependencies]

  proptest = "1"

  [features]

  default = []

  serde = ["dep:serde"]

  ```

- Initialize git. Add `CLAUDE.md` (Appendix C) and a `.gitignore` (`/target`).

- All code is safe Rust. `unsafe` is forbidden by the crate attribute; never add it.

---

## 2. Locked architecture and invariants

### 2.1 Two representations

- **Storage / wire / at-rest form** — a canonical, normal-form, **variable-width

  packed `BitVec<u8, Msb0>`**, one inline preorder stream per tree. This is what a

  `Party`, `Version`, and `Clock` hold at rest, what crosses the wire, and what

  `Eq`/`Hash`/comparison read. Because it is canonical, **bit-equality ⇔ semantic

  equality**.

- **Working form (events only, transient)** — a **fixed-width topology+payload

  split**: a topology `BitVec` (one bit per node, preorder, `1`=internal/two

  children, `0`=leaf) plus a `base: Vec<u64>` (one entry per node, preorder, indexed

  1:1 with the topology bits). Used only for the duration of a mutating event

  operation. Fixed-width `u64` makes the parent-base back-reference an O(1)

  overwrite, so the minimizing operations are single forward passes (see §2.3).

- **Ids have no working form.** Ids carry no integers, so id operations

  (`split`, `sum`, ordering, `is_disjoint`) run directly on the packed `BitVec`.

  An id's in-memory form and wire form coincide.

### 2.2 Transcode boundary

- Mutating event ops unpack packed→working **lazily** (on the first event-arithmetic

  step), compute in the working form, and repack working→packed **once on drop** of

  the batch (only if the working form was materialized).

- Observing ops (`leq`/causal order, `Eq`, `Hash`, `min`, `max`, the comparison

  matrix) read the packed form (or the working form) directly — **no transcode**.

- Composite ops (`receive`, `sync`) thread the working form through their stages and

  transcode once at the batch boundary, never per inner step.

- Consequence: only `tick` (= `fill` then `grow`) and the version side of `join`/

  `sync`/`merge` pay a transcode. `fork`/`peek`/`seed`/comparison do not.

### 2.3 Everything iterative

Every traversal — encode, decode+validate, unpack, repack, `split`, `sum`, `leq`,

`fill`, `grow`, `ev_join`, normalization — **must be iterative** (explicit stack or a

preorder cursor with a pending-children counter). No recursion on tree depth. A

valid-but-pathological clock (e.g. a long fork chain → deep id spine) must not

overflow the stack. Upstream size capping is assumed (no operation is superlinear in

its input), so internal depth/size guards are not required, but stack safety is.

### 2.4 Linearity

A party is a non-duplicable resource. `Party` and `Clock` are **not `Clone`**;

`Version` is freely `Clone`. The `BitOr` operators consume their `Clock` operand by

value for this reason (a borrowing form would duplicate a party). `join` consumes the

other `Clock` (its party is absorbed) and hands it back on overlap.

### 2.5 Normal form is an invariant

Every value produced by any public operation is in canonical normal form. In debug

builds, assert this (`debug_assert!`) at the end of every mutating op. `decode`

**strictly rejects** any input that is not in normal form (§5.4).

---

## 3. Locked public API

The authoritative interface is **Appendix B**. Implement exactly that surface:

modules `party` / `version` / `clock`; types `party::Party`, `version::Version`,

`version::Batch`, `clock::Clock`, `clock::Batch`; errors `OverlapError`,

`DecodeError`; the trait impls and operators as written. Re-export `Party`,

`Version`, `Clock` at the crate root.

Key points the skeleton encodes — do not deviate:

- `Party`: `seed()` (the whole space; only nonzero constructor), `fork`, `join`

  (`Result<(), Party>` — hands the party back on overlap), `is_disjoint`,

  `encode`/`decode`. `PartialOrd` is **descent / reverse-inclusion** (seed is the

  minimum; an ancestor is *less than* its forked descendants; cousins are `None`).

  `PartialEq`/`Eq`/`Hash` are structural. Not `Clone`.

- `Version`: `new`/`Default` (empty history; identity for `|`), `tick(&Party)`,

  `batch()`, `encode`/`decode`. `PartialOrd` is the **causal order** (`None` ⇔

  concurrent); `PartialEq`/`Eq`/`Hash` structural; `Clone`.

- `clock::Clock`: `seed`, `from_parts`, `into_parts`, `party() -> &Party`,

  `version() -> Version` (peek; does not advance), `tick`, `fork`, `join`

  (`Result<(), Clock>`), `sync(&mut Clock)`, `has_seen`/`happens_before`/

  `concurrent_with`, `send`/`receive`, `batch()`, `encode`/`decode`. **No

  comparison traits** on `Clock` (compare party and version separately). Not `Clone`.

- Batches carry the full operation set; **the event complexity lives in

  `version::Batch::tick`**, and `clock::Batch` is built on top of it. Batches have

  **no `peek`**; instead the full comparison matrix is implemented for

  `Version`×`version::Batch`, and `clock::Batch` exposes `version() -> &version::Batch`

  (for comparison) and `party() -> &Party`. Batches commit (repack) on drop.

- `sync`: `clock::Batch::sync(&mut self, other: &mut clock::Batch<'_>)` (borrowing —

  keeps both live batches); `Clock::sync(&mut self, other: &mut Clock)` wraps it as

  `self.batch().sync(&mut other.batch())`. No `Into`/`BorrowMut`.

- Operators (value level): `Clock|Version→Clock`, `Version|Clock→Clock`,

  `Version|Version→Version`, `Clock|=Version`, `Version|=Version`. Batch level:

  `clock::Batch |= &Version`, `version::Batch |= &Version` (borrow rhs; `merge` is

  the chainable companion). No `Clock|Clock` (that is the fallible `join`).

- `From<&mut Clock> for clock::Batch` and `From<&mut Version> for version::Batch`

  exist as convenience constructors delegating to `batch()`.

- `Debug` pretty-prints the decoded tree. `serde` (feature-gated) serializes via

  `encode`/`decode` bytes.

---

## 4. Build order — the oracle is the foundation

The first deliverable is a *trusted* reference oracle, not the optimized

implementation. Everything else is validated by differential testing against it, so

the oracle's own correctness must be established first. Phase 0 builds the oracle

(Appendix A) and makes its property suite (Appendix D) pass; no implementation work

begins until that suite is green. The oracle mirrors the target's semantic API

(Appendix B) so the differential harness can drive both with identical calls.

---

## 5. Precise encodings

All bit I/O uses `bitvec` with `BitVec<u8, Msb0>` / `BitSlice<u8, Msb0>`. Integers

are written most-significant-bit first.

### 5.1 Integer code for event bases — Elias gamma of `n + 1`

We deliberately deviate from the paper's doubling code (wire-compatibility with other

implementations is not required) and use a code tuned to the real distribution of

event-tree bases. Those bases are *deltas* — the sink pushes the common minimum up to

the parent — so they are small, with mode 1 and a fast-decaying tail; and normal form

forces at least one child base per internal node to be 0, so **more than half of all

bases are exactly 0**. The code must also be **canonical** (exactly one encoding per

value) and **self-delimiting** (trees are bit-concatenated with no length prefix).

Encode a non-negative base `n` as the Elias gamma code of `m = n + 1` (so `m >= 1`):

```

encode_int(out, n):                  // Elias gamma of (n + 1)

  m = n + 1

  k = floor(log2(m))                 // = bit_length(m) - 1

  push k zero bits

  push m as (k + 1) bits, MSB first  // the leading bit is always 1

decode_int(bits, pos) -> (n, new_pos):

  k = 0

  while bit(pos + k) == 0: k += 1    // count leading zeros (running past end => Truncated)

  m = read (k + 1) bits at (pos + k), MSB first   // the leading 1 plus k more bits

  return (m - 1, pos + 2*k + 1)

```

Cost is `2*floor(log2(n + 1)) + 1` bits:

| base `n` | bits | (paper's doubling code, for contrast) |

|---|---|---|

| 0 | **1** | 3 |

| 1 | 3 | 3 |

| 2 | 3 | 5 |

| 3-6 | 5 | 5-7 |

| 7-14 | 7 | 7-9 |

The dominant win is the forced zero: 3 -> 1 bit on the majority of bases, and 1 bit is

information-theoretically right (when one sibling is zero, a random base is zero with

probability ~0.5, so -log2(p) ~ 1 bit). The log-scaling tail keeps a rare large delta

bounded (delta 1000 is ~21 bits, not unary's 1001). Gamma is prefix-free, so it is

canonical (no minimal-width check is needed at decode) and self-delimiting.

**Optional variant** (adopt only if profiling shows delta = 1 dominates): encode a base

as `0` for zero, else `1` followed by `gamma(n)` (with `n >= 1`). This costs 2 bits for

delta = 1 (vs gamma's 3) at the cost of 4 bits for delta = 2. Keep `gamma(n + 1)` by

default — it is a single clean code and the two are within ~1 bit either way.

**Out of scope for v1, documented for later:** a deterministic range coder with a model

that assigns probability 0 to the impossible `(+,+)` sibling pattern and a geometric

prior to the deltas would beat any prefix code by ~10-15% and remain canonical if fully

deterministic. Because the working form is `u64` and we transcode only at the

encode/decode boundary, such a code would never touch the compute path — but the trees

are tiny, so the byte saving does not justify a harder-to-fuzz decoder now. Topology

stays at 1 bit per node (already within O(log L) of optimal for a strictly-binary

shape); do not arithmetic-code it.

### 5.2 Id (party) packed encoding — preorder, uniform flag

```

enc_id(Leaf(v))  = 0 , v                 (2 bits)

enc_id(Node l r) = 1 , enc_id(l) , enc_id(r)

```

(We deliberately do not use Appendix A's half-empty-id tags; the uniform encoding is

simpler and the size difference is immaterial. Note this in a code comment.)

### 5.3 Event (version) packed encoding — preorder, uniform flag

```

enc_ev(Leaf(n))    = 0 , encode_int(n)

enc_ev(Node n l r) = 1 , encode_int(n) , enc_ev(l) , enc_ev(r)

```

### 5.4 Clock wire form and decode validation

- A `Clock` serializes as `enc_id(party)` immediately followed (bit-concatenated, no

  padding between) by `enc_ev(version)`, then the **whole stream is zero-padded to a

  byte boundary**.

- Each preorder tree is **self-delimiting** (the parser knows exactly where a tree

  ends), so no length prefix is needed: parse the party tree to completion, then the

  version tree to completion, then require that only zero padding bits remain.

- `decode` returns `Result<_, DecodeError>` and is **iterative**. It must reject:

  - `Truncated` — the bit stream ends mid-tree (or mid-integer).

  - `TrailingBits` — non-padding bits remain after a complete tree (or non-zero

    padding).

  - `NotCanonical` — the structure is well-formed but not in normal form.

- **Normal-form predicate** (validate bottom-up; mirror the oracle's `nf_ok` for

  events and the id collapse rule):

  - Id node `(l, r)`: reject if `l` and `r` are both leaves with equal value

    (would collapse to a single leaf); both children must also be normal.

  - Event node `(n, l, r)`: require `min(l) == 0 || min(r) == 0` (for a normalized

    child, `min == base == its root integer`); reject if `l` and `r` are both leaves

    with equal base (collapsible `(n,m,m)`); both children must also be normal.

- Strict rejection means: a `Version`/`Party`/`Clock` only ever exists in canonical

  normal form, so byte-equality is sound for `Eq`/`Hash`.

### 5.5 Working form (events only)

- `WorkingVersion { topo: BitVec<u8, Msb0>, base: Vec<u64> }`.

- Preorder; `topo[i] == true` ⇔ node `i` is internal (two children); `base[i]` is its

  integer. `topo.len() == base.len() == node count`.

- Navigation is by preorder cursor (left child is the next node; the right child is

  reached after the left subtree is consumed — threaded by the algorithm, never by

  re-scanning). The fixed-width `base` array makes a node's integer an O(1) indexed

  read/overwrite.

- `unpack(packed) -> WorkingVersion` and `repack(&WorkingVersion) -> BitVec`

  (canonical) are both single iterative passes.

---

## 6. Implementation phases (TDD)

Each phase: **write the tests first and confirm they fail**, then implement, then run

the verification gate, then commit. Test groups (A–I) are defined in §7.

### Phase 0 — The oracle and its property suite (FOUNDATION; gate for all later work)

- Create the crate, `Cargo.toml`, lints, `CLAUDE.md`, `.gitignore`, git init.

- Implement the reference oracle (Appendix A) at `src/oracle.rs` (gated

  `#[cfg(test)]`, or `tests/oracle/mod.rs`). It mirrors the target's *semantic* API,

  so the differential harness will later drive both with identical calls.

- **Write the oracle property suite (Appendix D) and make every property O1–O15

  pass.** This is the gate — the oracle is only trustworthy as ground truth once the

  suite is green. Include the paper §5.1 worked example (O15) as a deterministic unit

  test. (TDD applies: write the O-properties first; the oracle is simple, so most

  should pass immediately, but a few — normal-form preservation, the LUB laws,

  `leq ⇔ join`, sync re-split — are real checks that catch transcription slips.)

- Then stub the entire public API (Appendix B) with `todo!()` so the crate compiles,

  and build the differential-harness scaffolding (§8): the op-trace generator and the

  structural lowering (oracle `Clock::trees()` vs the impl's `#[cfg(test)]`

  `to_oracle_trees()`). The impl side is not exercised yet.

- **Gate:** the full oracle suite (Appendix D) is green; `cargo build` clean; impl

  tests present but `#[ignore]`d. **Commit. Do not begin Phase 1 until O1–O15 pass.**

### Phase 1 — Codecs + round-trip / canonical / reject  *(tests group A 1–3)*

- Tests first: round-trip `decode∘encode == id` for `Party`, `Version`, `Clock`;

  canonical injectivity (`a == b ⇔ encode(a) == encode(b)`) over impl values from

  random op traces; `decode` rejects malformed (truncated, trailing/garbage bits)

  and non-canonical (denormal) inputs.

- Implement: `encode_int`/`decode_int`; `enc_id`/`dec_id`; `enc_ev`/`dec_ev`;

  iterative `decode` with normal-form validation; `encode`/`decode` on all three

  types; the `DecodeError` variants.

- **Gate + commit.**

### Phase 2 — Working-form transcode  *(tests group A 4)*

- Tests first: `repack ∘ unpack == identity` (and yields canonical bytes) over

  random versions sourced from the oracle.

- Implement: `WorkingVersion`, iterative `unpack`, iterative `repack`.

- **Gate + commit.**

### Phase 3 — Observing ops + comparison matrix  *(tests groups C 7–10, F 28–29)*

- Tests first: causal-order laws (reflexive, antisymmetric, transitive,

  `Eq`/`Ord` consistency, concurrency ⇒ `None`); representation parity across the

  `Version`×`version::Batch` matrix; all agree with the oracle's `leq`.

- Implement: iterative offset-threaded `leq` reading the packed form (and the

  working form, for batch comparisons); `Version` `PartialOrd` (+ derived

  `PartialEq`/`Eq`/`Hash`); the full comparison matrix for `version::Batch`;

  `Clock::{has_seen, happens_before, concurrent_with}`.

- **Gate + commit.**

### Phase 4 — Party ops + party order  *(tests group D 17–20)*

- Tests first: party partial-order laws under descent; `fork`→`join` round-trip

  recovers the original party; forks are disjoint and are descendants (parent <

  child); `join` is the meet for disjoint parties and returns `Err` (handing the

  party back) on overlap; `is_disjoint` correctness.

- Implement: iterative id `split` and `sum` on the packed form; `Party::{seed, fork,

  join, is_disjoint}`; `Party` `PartialOrd` (descent) + structural `Eq`/`Hash`.

- **Gate + commit.**

### Phase 5 — Event mutation (the core)  *(tests groups C 11–16, plus differential subset)*

- Tests first: `tick` strict monotonicity (`a < a.tick(p)`); join/merge is an upper

  bound and a least upper bound; join semilattice laws (commutative, associative,

  idempotent); `Version::new()` identity; absorbing (`a ≤ b ⇒ a|b == b`). All

  checked against the oracle's `event`/`ev_join`.

- Implement (all iterative, on the working form, single-pass with O(1) base

  backpatch and structural-collapse handling):

  - `fill` (note the left-child-depends-on-right-sibling minimum — process the

    relevant subtree first; the id=`1` side collapses to a leaf),

  - `grow` (a read-only cost probe with lexicographic cost `(expansions, depth)` to

    choose the min-cost child, then a single emitting pass that rewrites only the

    chosen path and copies the rest),

  - `version::Batch::tick` = `fill`, and if `fill` changed nothing, `grow`,

  - `merge` = `ev_join` on the working form,

  - `Version::tick` wrapper; `Version` `|`/`|=`; `version::Batch |= &Version`.

- **Gate + commit.** (This is the phase to schedule a review pass.)

### Phase 6 — Clock, clock::Batch, protocol + master differential harness

  *(tests groups B 5–6 & 21, E 22–27, G 30–32)*

- Tests first:

  - The **master differential harness** (§8): random seed-derived op traces applied

    in lockstep to the impl and the oracle; after every step assert canonical-byte

    agreement *and* the disjointness invariant (all live parties pairwise disjoint).

    Target ≥ 10,000 traces of mixed length in CI.

  - Protocol semantics: `fork` preserves version; `version()` does not advance;

    own-message/dominated `receive` equals a bare `tick`; `sync` makes both versions

    equal (to the join of the two) and re-splits the merged party (disjoint, and

    their `join` equals the pre-sync merged party); heterogeneous join semantics

    (`Clock|Version`, `Version|Clock`, `Version|Version`) match the oracle.

  - Batch equivalence (a batch ≡ the same ops applied as value-level calls);

    lazy/no-op batches don't mutate bytes; commit-on-drop reflects exactly the ops.

- Implement: `version::Batch` and `clock::Batch` fully (`tick`, `merge`, `fork`,

  `join`, `sync`, `version()`, `party()`, `|=`); `clock::Batch` lazy unpack + commit

  on drop, built on `version::Batch`; `Clock` methods as single-op batch wrappers

  (`tick`, `fork`, `join`, `sync`, `send`, `receive`); `from_parts`/`into_parts`/

  `party`/`version`; the `From<&mut _>` constructors; `Clock` operators.

- **Gate + commit.**

### Phase 7 — Hardening, robustness, serde, docs  *(tests groups A 5, H 33–34, I 35)*

- Tests first: deep-tree stack safety (build a depth-100k id spine and a deep event

  tree via long op chains; run every op + `encode`/`decode`; assert no overflow and

  agreement with the oracle); decode fuzz (random byte strings → never panic, always

  `Ok`/`Err`); normal-form invariant proptest (every op output satisfies the ported

  `nf` predicate); serde round-trip (under `--features serde`); the paper §5.1

  worked example as a deterministic unit test; doctests on the public examples.

- Implement: audit every traversal for iterativeness (grep for recursion; convert any

  stragglers); `Debug` tree pretty-printer; `serde` impls (feature-gated); doc

  comments + runnable doctests; `#![warn(missing_docs)]` satisfied.

- **Gate + commit. Tag `v0.1.0`.**

---

## 7. Comprehensive property catalog

Use `proptest`. Generate values **via operations from a seed** (never by fabricating

trees directly) so inputs are always valid, normal-form, and — for populations —

pairwise party-disjoint. See §8 for generators.

The *oracle's* own suite (Appendix D) bootstraps trust in the oracle and runs first

(Phase 0). The catalog below is for the **implementation**, checked in Phases 1–7.

**A. Codec & representation**

1. Round-trip: `decode(encode(x)) == x` for `Party`, `Version`, `Clock`.

2. Canonical: `a == b` ⇔ `encode(a) == encode(b)`; encode is injective on normal

   forms.

3. Decode rejects: malformed (truncate a valid encoding; append/garble bits) and

   non-canonical (emit a deliberately denormal tree: a collapsible `(n,m,m)` event

   node, an uncollapsed `(0,0)`/`(1,1)` id node, a child whose min ≠ 0) ⇒ `Err`.

4. Working round-trip: `repack(unpack(v)) == v` (and canonical).

5. Normal-form invariant: every value produced by every op satisfies the ported `nf`

   predicate.

6. (See B.)

**B. Differential vs oracle (master harness)**

6. Lockstep traces: random sequences over a seed-derived population, applied to impl

   and oracle, agree on canonical bytes after every step (the highest-leverage test).

21. Disjointness invariant: all live clocks' parties are pairwise disjoint after

    every step (so `join`/`sync` never error in correct usage).

**C. Causal order & lattice (Version)**

7. Reflexive `a ≤ a`. 8. Antisymmetric `a ≤ b ∧ b ≤ a ⇒ a == b`.

9. Transitive `a ≤ b ∧ b ≤ c ⇒ a ≤ c`. 10. `a == b ⇔ cmp == Some(Equal)`;

incomparable ⇒ `None`. 11. Upper bound: `a ≤ a|b ∧ b ≤ a|b`.

12. Least upper bound: for `c` with `a ≤ c ∧ b ≤ c`, `a|b ≤ c`.

13. Semilattice: `a|b == b|a`; `(a|b)|c == a|(b|c)`; `a|a == a`.

14. Identity: `Version::new() | a == a`. 15. Monotone tick: `a < a.tick(p)`.

16. Absorbing: `a ≤ b ⇒ a|b == b` (⇒ merging a dominated version is a no-op).

**D. Party**

17. Partial-order laws under descent. 18. `fork` then `join` recovers the original

party (version unchanged). 19. Forks are disjoint; parent `<` each child.

20. `join` is the meet for disjoint parties; `Err` (party returned) on overlap;

`is_disjoint` matches.

**E. Protocol**

22. `fork` preserves version on both halves. 23. `version()`/peek does not advance;

returned `Version` equals the clock's. 24. `receive(msg)` with `msg ≤ self` equals a

bare `tick` (own-message receive is benign). 25. After `a.sync(&mut b)`:

`a.version() == b.version()` (= join of the two), parties disjoint, and their `join`

equals the pre-sync merged party. 26/27. Heterogeneous joins (`Clock|Version`,

`Version|Clock`, `Version|Version`) and the anonymous-as-party-0 identity match the

oracle.

**F. Comparison-representation parity**

28. For random `a, b`: `cmp(a,b)`, `cmp(a.batch(), b)`, `cmp(a, b.batch())`,

`cmp(a.batch(), b.batch())` all agree and match the oracle.

29. Mid-batch comparison reflects uncommitted ticks; after drop the committed value

equals what the comparison observed.

**G. Batch equivalence & laziness**

30. A batch of ops `== ` the same ops applied as value-level calls.

31. A batch with no arithmetic op (e.g. fork-only, or created-and-dropped) leaves the

underlying bytes unchanged. 32. Commit-on-drop reflects exactly the applied ops.

**H. Robustness**

33. Deep structures (≥100k depth) — all ops + codec — no stack overflow, correct vs

oracle. 34. `decode` of arbitrary bytes never panics.

**I. Worked example**

35. The paper's Section 5.1 example reproduced step-by-step matches expected values.

---

## 8. Differential testing strategy

- **Oracle** (Appendix A): the paper's trees as plain recursive enums — each op a method

  on `Party`/`Version`/`Clock` — mirroring the target's semantic API. It is trusted ground truth **because it passes the Appendix D

  suite** (Phase 0). It is recursive and for bounded-depth use only — the deep-tree

  stack-safety test (group H 33) runs against the impl alone, never the oracle.

- **Identical API ⇒ one harness, no mapping table.** The oracle exposes the same

  method names and signatures as the impl (construction, `tick`, `fork`, `join`,

  `sync`, `receive`, `|`/`|=`, ordering), so the op-trace driver issues the *same*

  calls to both populations. The oracle omits only the byte codec (impl-only; tested

  by round-trip/reject) and batches (a batch only ever equals its value-ops; tested

  impl-side, group G).

- **Agreement is structural.** Lower both clocks to plain trees and compare with `==`:

  the oracle via `Clock::trees() -> (&Party, &Version)` (its types *are* the structural

  form), the impl via a `#[cfg(test)] to_oracle_trees() -> (oracle::Party, oracle::Version)`

  that rebuilds that shape from its *internal* representation (packed/working) — **not**

  via the public codec. This decouples algorithm correctness (structural diff vs the

  oracle) from codec correctness (group A, tested separately). Both forms are

  normalized, so structural `==` ⇔ semantic equality.

- **Op-trace generator:** maintain parallel populations `Vec<(oracle::Clock,

  itc::Clock)>` from one seed, applying the identical op to both each step: `fork k`

  (split clock *k* into two), `tick k`, `join j k` (`j ≠ k`; remove *k*), `sync j k`,

  `receive k` (deliver another member's `version()`), `merge anon k`

  (`clock |= version_of(m)`). Every member descends from one seed via fork/join/sync,

  so parties stay pairwise disjoint and join/sync never error — assert it.

- After each step: structural agreement on touched clocks, plus per-step invariants

  (normal form via `Party::is_normal`/`Version::is_normal`; disjointness). Periodically exercise the

  comparison matrix and codec round-trips on sampled values. Target ≥10,000 traces.

---

## 9. Definition of Done

- [ ] **Oracle property suite (Appendix D, O1–O15) green — the Phase 0 gate, completed before any impl work.**

- [ ] Public API matches Appendix B exactly (names, signatures, trait impls,

      operators, module paths).

- [ ] All property groups A–I implemented and green, including ≥10,000 differential

      traces and the deep-tree (≥100k) stack-safety test.

- [ ] `decode` rejects every malformed and every non-canonical input in the reject

      suite; `decode` of random bytes never panics.

- [ ] No recursion on tree depth anywhere (audited); all traversals iterative.

- [ ] `#![forbid(unsafe_code)]` holds; no `unsafe`.

- [ ] `cargo fmt --check`, `cargo clippy --all-targets --all-features -D warnings`,

      and `cargo test --all-features` all clean.

- [ ] `serde` round-trips under `--features serde`; doctests pass; `missing_docs`

      satisfied.

- [ ] Tagged `v0.1.0`.

---

## 10. Risks & gotchas

- **Var-width back-patching is the trap we designed around.** Never mutate integers

  in the packed form in place. All arithmetic happens in the fixed-width working

  form; repack to packed only at the boundary.

- **`fill` has a sideways dependency** (a filled child's leaf value depends on a

  sibling's post-fill minimum). Process the needed subtree before emitting the

  dependent leaf; don't assume strict left-to-right emission.

- **`grow` must preserve the paper's cost-driven choice.** Use a read-only cost probe

  (lexicographic `(expansions, depth)`), then emit along the chosen path. Do not

  normalize mid-grow in a way that changes the cost decision; normalize the final

  result.

- **Canonical equality depends on strict decode.** If `decode` ever admits a denormal

  value, `Eq`/`Hash`/byte-comparison become unsound. Keep validation strict.

- **`bitvec` proxy references**: indexing yields proxy/`bool` semantics; centralize

  bit reads in small helpers to avoid scattering quirks. Confirm `BitVec<u8, Msb0>`

  throughout for byte-canonical output.

- **Linearity is enforced by `!Clone` + by-value operators.** Don't add `Clone` to

  `Party`/`Clock` or borrowing `BitOr` overloads for `Clock` — both would let a party

  be duplicated.

- **Iterativeness is a correctness requirement, not an optimization** (untrusted deep

  inputs). The deep-tree test guards it; keep it in CI.

---

## Appendix A — Reference oracle (`src/oracle.rs`, test-only)

Expresses the paper's algorithms directly on the public types — `Party` and `Version`

*are* the recursive trees and every operation is a method, so there is no second

representation to keep in sync. Simple, suboptimal, recursive; its only job is obvious

correctness. It mirrors the target's *semantic* API and omits the two purely

representational concerns that carry no semantics: the byte codec and batches (see the

note after the code). Build it and pass Appendix D before any implementation work.

```rust

//! Reference oracle — the paper's trees as plain recursive enums.

//!

//! `Party` and `Version` *are* the trees; every operation is a method, so there is no

//! second representation to keep in sync. Deliberately simple, suboptimal, and

//! recursive: its only job is to be obviously correct, so it can serve as differential

//! ground truth. It mirrors the target's **semantic** surface (construction,

//! operations, ordering, operators) and omits the two purely *representational*

//! concerns that carry no semantics: the byte codec (`encode`/`decode`) and the batch

//! optimization (a batch only ever equals its value-level ops). Bounded-depth use only

//! — the deep-tree stack-safety test runs against the impl, never the oracle.

//!

//! All three types derive `Clone`: a reference oracle needs cheap snapshots of "before"

//! states for the property checks, and linearity (`!Clone` on `Party`/`Clock`) is a

//! *type-level* guarantee checked against `itc` by compile-fail tests — not a runtime

//! semantic the differential harness exercises.

//!

//! Build this and make Appendix D's property suite pass BEFORE any implementation work.

use std::cmp::Ordering;

use std::ops::{BitOr, BitOrAssign};

#[derive(Debug)]

pub struct OverlapError;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]

pub enum Party { Leaf(bool), Node(Box<Party>, Box<Party>) }

#[derive(Clone, PartialEq, Eq, Hash, Debug)]

pub enum Version { Leaf(u64), Node(u64, Box<Version>, Box<Version>) }

#[derive(Clone, Debug)]

pub struct Clock { party: Party, version: Version }

impl Party {

    pub fn seed() -> Self { Party::Leaf(true) }

    fn node(l: Party, r: Party) -> Party {

        match (&l, &r) {

            (Party::Leaf(false), Party::Leaf(false)) => Party::Leaf(false),

            (Party::Leaf(true),  Party::Leaf(true))  => Party::Leaf(true),

            _ => Party::Node(Box::new(l), Box::new(r)),

        }

    }

    fn is_empty(&self) -> bool {

        match self { Party::Leaf(b) => !*b, Party::Node(l, r) => l.is_empty() && r.is_empty() }

    }

    fn is_full(&self) -> bool {

        match self { Party::Leaf(b) => *b, Party::Node(l, r) => l.is_full() && r.is_full() }

    }

    fn split(&self) -> (Party, Party) {

        match self {

            Party::Leaf(false) => (Party::Leaf(false), Party::Leaf(false)),

            Party::Leaf(true)  => (Party::node(Party::Leaf(true), Party::Leaf(false)),

                                   Party::node(Party::Leaf(false), Party::Leaf(true))),

            Party::Node(l, r) => {

                if l.is_empty() {

                    let (a, b) = r.split();

                    (Party::node(Party::Leaf(false), a), Party::node(Party::Leaf(false), b))

                } else if r.is_empty() {

                    let (a, b) = l.split();

                    (Party::node(a, Party::Leaf(false)), Party::node(b, Party::Leaf(false)))

                } else {

                    (Party::node((**l).clone(), Party::Leaf(false)),

                     Party::node(Party::Leaf(false), (**r).clone()))

                }

            }

        }

    }

    fn sum(self, other: Party) -> Party {

        match (self, other) {

            (Party::Leaf(false), b) => b,

            (a, Party::Leaf(false)) => a,

            (Party::Node(l1, r1), Party::Node(l2, r2)) =>

                Party::node((*l1).sum(*l2), (*r1).sum(*r2)),

            _ => Party::Leaf(true), // overlap: unreachable (callers check disjointness)

        }

    }

    pub fn fork(&mut self) -> Party {

        let (a, b) = self.split();

        *self = a;

        b

    }

    pub fn join(&mut self, other: Party) -> Result<(), Party> {

        if !self.is_disjoint(&other) { return Err(other); }

        let mine = std::mem::replace(self, Party::Leaf(false));

        *self = mine.sum(other);

        Ok(())

    }

    pub fn is_disjoint(&self, other: &Party) -> bool {

        match (self, other) {

            (Party::Leaf(false), _) | (_, Party::Leaf(false)) => true,

            (Party::Leaf(true), x) | (x, Party::Leaf(true)) => x.is_empty(),

            (Party::Node(a1, a2), Party::Node(b1, b2)) => a1.is_disjoint(b1) && a2.is_disjoint(b2),

        }

    }

    fn contains(&self, other: &Party) -> bool {

        match (self, other) {

            (_, Party::Leaf(false)) => true,

            (Party::Leaf(true), _)  => true,

            (Party::Leaf(false), x) => x.is_empty(),

            (x, Party::Leaf(true))  => x.is_full(),

            (Party::Node(a1, a2), Party::Node(b1, b2)) => a1.contains(b1) && a2.contains(b2),

        }

    }

    pub fn is_normal(&self) -> bool {

        match self {

            Party::Leaf(_) => true,

            Party::Node(l, r) => {

                let collapsible = matches!((&**l, &**r), (Party::Leaf(a), Party::Leaf(b)) if a == b);

                !collapsible && l.is_normal() && r.is_normal()

            }

        }

    }

}

impl PartialOrd for Party {

    /// Descent: an ancestor (larger region) is *less than* its forked descendants.

    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {

        match (self.contains(other), other.contains(self)) {

            (true, true)   => Some(Ordering::Equal),

            (true, false)  => Some(Ordering::Less),

            (false, true)  => Some(Ordering::Greater),

            (false, false) => None,

        }

    }

}

type Cost = (u32, u32); // (#expansions, depth), lexicographic

impl Version {

    pub fn new() -> Self { Version::Leaf(0) }

    fn base(&self) -> u64 { match self { Version::Leaf(n) | Version::Node(n, ..) => *n } }

    fn max_ev(&self) -> u64 {

        match self { Version::Leaf(n) => *n, Version::Node(n, l, r) => *n + l.max_ev().max(r.max_ev()) }

    }

    fn debase(self, m: u64) -> Version {

        match self {

            Version::Leaf(n) => Version::Leaf(n - m),

            Version::Node(n, l, r) => Version::Node(n - m, l, r),

        }

    }

    /// `norm((n,l,r))`, assuming `l`,`r` already normal.

    fn node(n: u64, l: Version, r: Version) -> Version {

        let m = l.base().min(r.base());

        let l = l.debase(m);

        let r = r.debase(m);

        match (&l, &r) {

            (Version::Leaf(a), Version::Leaf(b)) if a == b => Version::Leaf(n + m + *a),

            _ => Version::Node(n + m, Box::new(l), Box::new(r)),

        }

    }

    fn normalized(&self) -> Version {

        match self {

            Version::Leaf(n) => Version::Leaf(*n),

            Version::Node(n, l, r) => Version::node(*n, l.normalized(), r.normalized()),

        }

    }

    /// `self+so <= other+oo` pointwise (offset-threaded).

    fn leq(&self, so: u64, other: &Version, oo: u64) -> bool {

        let sn = so + self.base();

        let on = oo + other.base();

        if sn > on { return false; }

        match self {

            Version::Leaf(_) => true,

            Version::Node(_, sl, sr) => match other {

                Version::Leaf(_) => sl.leq(sn, other, oo) && sr.leq(sn, other, oo),

                Version::Node(_, ol, or) => sl.leq(sn, ol, on) && sr.leq(sn, or, on),

            },

        }

    }

    /// Join (LUB) of two event trees, offset-threaded.

    fn join_off(&self, so: u64, other: &Version, oo: u64) -> Version {

        if let (Version::Leaf(sn), Version::Leaf(on)) = (self, other) {

            return Version::Leaf((so + *sn).max(oo + *on));

        }

        let sb = so + self.base();

        let ob = oo + other.base();

        let z = Version::Leaf(0);

        let (sl, sr) = match self  { Version::Node(_, l, r) => (l.as_ref(), r.as_ref()), _ => (&z, &z) };

        let (ol, or) = match other { Version::Node(_, l, r) => (l.as_ref(), r.as_ref()), _ => (&z, &z) };

        let l = sl.join_off(sb, ol, ob);

        let r = sr.join_off(sb, or, ob);

        Version::node(0, l, r)

    }

    /// `fill(id, self)`.

    fn fill(&self, id: &Party) -> Version {

        match (id, self) {

            (Party::Leaf(false), _) => self.clone(),

            (Party::Leaf(true), _)  => Version::Leaf(self.max_ev()),

            (Party::Node(..), Version::Leaf(n)) => Version::Leaf(*n),

            (Party::Node(il, ir), Version::Node(n, el, er)) => {

                if il.is_full() {

                    let er2 = er.fill(ir);

                    let x = el.max_ev().max(er2.base());

                    Version::node(*n, Version::Leaf(x), er2)

                } else if ir.is_full() {

                    let el2 = el.fill(il);

                    let x = er.max_ev().max(el2.base());

                    Version::node(*n, el2, Version::Leaf(x))

                } else {

                    Version::node(*n, el.fill(il), er.fill(ir))

                }

            }

        }

    }

    /// `grow(id, self)` → (tree, cost).

    fn grow(&self, id: &Party) -> (Version, Cost) {

        match (id, self) {

            (Party::Leaf(true), Version::Leaf(n)) => (Version::Leaf(*n + 1), (0, 0)),

            (Party::Leaf(true), Version::Node(n, el, er)) => {

                let (el2, cl) = el.grow(&Party::Leaf(true));

                let (er2, cr) = er.grow(&Party::Leaf(true));

                if cl < cr { (Version::Node(*n, Box::new(el2), er.clone()), (cl.0, cl.1 + 1)) }

                else       { (Version::Node(*n, el.clone(), Box::new(er2)), (cr.0, cr.1 + 1)) }

            }

            (Party::Leaf(false), _) => (self.clone(), (u32::MAX, u32::MAX)),

            (Party::Node(..), Version::Leaf(n)) => {

                let expanded = Version::Node(*n, Box::new(Version::Leaf(0)), Box::new(Version::Leaf(0)));

                let (e2, c) = expanded.grow(id);

                (e2, (c.0 + 1, c.1))

            }

            (Party::Node(il, ir), Version::Node(n, el, er)) => {

                if il.is_empty() {

                    let (er2, cr) = er.grow(ir);

                    (Version::Node(*n, el.clone(), Box::new(er2)), (cr.0, cr.1 + 1))

                } else if ir.is_empty() {

                    let (el2, cl) = el.grow(il);

                    (Version::Node(*n, Box::new(el2), er.clone()), (cl.0, cl.1 + 1))

                } else {

                    let (el2, cl) = el.grow(il);

                    let (er2, cr) = er.grow(ir);

                    if cl < cr { (Version::Node(*n, Box::new(el2), er.clone()), (cl.0, cl.1 + 1)) }

                    else       { (Version::Node(*n, el.clone(), Box::new(er2)), (cr.0, cr.1 + 1)) }

                }

            }

        }

    }

    fn event(&self, id: &Party) -> Version {

        let filled = self.fill(id);

        if filled != *self { filled } else { let (grown, _) = self.grow(id); grown.normalized() }

    }

    pub fn tick(&mut self, party: &Party) { *self = self.event(party); }

    pub fn is_normal(&self) -> bool {

        match self {

            Version::Leaf(_) => true,

            Version::Node(_, l, r) => {

                let one_zero = l.base() == 0 || r.base() == 0;

                let collapsible = matches!((&**l, &**r), (Version::Leaf(a), Version::Leaf(b)) if a == b);

                one_zero && !collapsible && l.is_normal() && r.is_normal()

            }

        }

    }

}

impl Default for Version { fn default() -> Self { Self::new() } }

impl PartialOrd for Version {

    /// Causal order; `None` means concurrent.

    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {

        match (self.leq(0, other, 0), other.leq(0, self, 0)) {

            (true, true)   => Some(Ordering::Equal),

            (true, false)  => Some(Ordering::Less),

            (false, true)  => Some(Ordering::Greater),

            (false, false) => None,

        }

    }

}

impl Clock {

    pub fn seed() -> Self { Self::from_parts(Party::seed(), Version::new()) }

    pub fn from_parts(party: Party, version: Version) -> Self { Clock { party, version } }

    pub fn into_parts(self) -> (Party, Version) { (self.party, self.version) }

    pub fn party(&self) -> &Party { &self.party }

    pub fn version(&self) -> Version { self.version.clone() }

    pub fn tick(&mut self) { self.version.tick(&self.party); }

    pub fn fork(&mut self) -> Clock {

        let child = self.party.fork();

        Clock { party: child, version: self.version.clone() }

    }

    pub fn join(&mut self, other: Clock) -> Result<(), Clock> {

        let (op, ov) = other.into_parts();

        match self.party.join(op) {

            Ok(()) => { self.version |= ov; Ok(()) }

            Err(op) => Err(Clock::from_parts(op, ov)),

        }

    }

    pub fn sync(&mut self, other: &mut Clock) -> Result<(), OverlapError> {

        if !self.party.is_disjoint(&other.party) { return Err(OverlapError); }

        let theirs = std::mem::replace(&mut other.party, Party::Leaf(false));

        self.party.join(theirs).expect("disjoint, just checked");

        other.party = self.party.fork();

        let merged = self.version.clone() | other.version.clone();

        self.version = merged.clone();

        other.version = merged;

        Ok(())

    }

    pub fn has_seen(&self, msg: &Version) -> bool { msg.leq(0, &self.version, 0) }

    pub fn happens_before(&self, other: &Clock) -> bool { self.version < other.version }

    pub fn concurrent_with(&self, other: &Clock) -> bool {

        self.version.partial_cmp(&other.version).is_none()

    }

    pub fn send(&mut self) -> Version { self.tick(); self.version() }

    pub fn receive(&mut self, msg: Version) { self.version |= msg; self.tick(); }

    pub fn trees(&self) -> (&Party, &Version) { (&self.party, &self.version) }

}

impl BitOr<Version> for Version {

    type Output = Version;

    fn bitor(self, rhs: Version) -> Version { self.join_off(0, &rhs, 0) }

}

impl BitOrAssign<Version> for Version {

    fn bitor_assign(&mut self, rhs: Version) { *self = self.join_off(0, &rhs, 0); }

}

impl BitOr<Version> for Clock {

    type Output = Clock;

    fn bitor(mut self, rhs: Version) -> Clock { self.version |= rhs; self }

}

impl BitOr<Clock> for Version {

    type Output = Clock;

    fn bitor(self, mut rhs: Clock) -> Clock { rhs.version |= self; rhs }

}

impl BitOrAssign<Version> for Clock {

    fn bitor_assign(&mut self, rhs: Version) { self.version |= rhs; }

}

```

Because the oracle mirrors the semantic API, the differential harness needs **no**

per-op mapping — it issues the same calls to both populations. The only adapter is the

structural lowering for agreement (§8): the oracle's `Party`/`Version` already *are* the

canonical structural form, so `Clock::trees()` simply borrows them; the impl provides a

`#[cfg(test)] to_oracle_trees() -> (oracle::Party, oracle::Version)` that rebuilds that

shape from its packed/working representation, and the two are compared with `==`. Two

correspondences worth stating explicitly:

| target concept | oracle realization |

|---|---|

| anonymous `Version` in a join | a clock whose party is `0` — version join (`|`) only |

| `Party` descent order | id-region containment (reverse inclusion) |

## Appendix B — Locked public API skeleton

Implement exactly this surface. (`sync` uses the borrowing `&mut clock::Batch`

form with a `Clock` convenience wrapper; no `Into`/`BorrowMut`.)

```rust

//! Interval Tree Clocks.

//!

//! `party::Party` is a nonzero share of the id space (ordered by descent: an

//! ancestor is *less than* its forked descendants). `version::Version` is an

//! event tree / message, also serving as the paper's anonymous stamp.

//! `clock::Clock` is a `Party` paired with a `Version` — purely a convenience;

//! `into_parts`/`from_parts` move between them, and the whole `Clock` API can be

//! reconstructed by hand from the `Party` and `Version` APIs.

//!

//! Linearity: `Party`/`Clock` are not `Clone`; `Version` clones freely.

//!

//! All mutation goes through a batch (`version::Batch`, `clock::Batch`) that

//! unpacks the version to a fast fixed-width working form lazily, applies a run

//! of operations, and repacks once on drop. Value-level methods are single-op

//! batches. Comparison reads the current state in place — no repack — so batches

//! are compared directly rather than peeked. All traversals are iterative.

pub use party::Party;

pub use version::Version;

pub use clock::Clock;

/// Two parties were not disjoint. (`join` instead hands the clock back.)

#[derive(Debug)]

pub struct OverlapError;

#[derive(Debug)]

pub enum DecodeError { Truncated, TrailingBits, NotCanonical }

// ───────────────────────────── party ─────────────────────────────

pub mod party {

    use core::cmp::Ordering;

    use crate::DecodeError;

    /// A nonzero share of the id space. Not `Clone`. Ordered by descent /

    /// reverse-inclusion: `seed` is the minimum, leaves are maximal, cousins are

    /// `None`. For disjoint parties, `join` computes the meet under this order.

    #[derive(PartialEq, Eq, Hash)]

    pub struct Party(/* BitVec<u8, Msb0> */);

    impl Party {

        /// The whole id space (the paper's `1`). The only nonzero constructor.

        pub fn seed() -> Self { todo!() }

        /// Split in two; `self` keeps one half, the other is returned.

        pub fn fork(&mut self) -> Party { todo!() }

        /// Merge a disjoint share into `self`; on overlap, `other` is returned.

        pub fn join(&mut self, other: Party) -> Result<(), Party> { todo!() }

        pub fn is_disjoint(&self, other: &Party) -> bool { todo!() }

        pub fn encode(&self) -> Vec<u8> { todo!() }

        pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> { todo!() }

    }

    impl PartialOrd for Party { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { todo!() } }

    impl core::fmt::Debug for Party { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { todo!() } }

}

// ──────────────────────────── version ────────────────────────────

pub mod version {

    use core::cmp::Ordering;

    use core::marker::PhantomData;

    use core::ops::{BitOr, BitOrAssign};

    use crate::{Party, DecodeError};

    /// An event tree / message; an anonymous clock. `Eq`/`Hash` are structural

    /// over the canonical encoding; `PartialOrd` is the causal order (`None` ⇔

    /// concurrent), consistent with `Eq` because normal form is canonical.

    #[derive(Clone, PartialEq, Eq, Hash)]

    pub struct Version(/* BitVec<u8, Msb0> */);

    impl Version {

        /// The empty history (identity for `|`).

        pub fn new() -> Self { todo!() }

        /// Advance `party`'s component by one event. Single-op batch.

        pub fn tick(&mut self, party: &Party) { self.batch().tick(party); }

        /// Begin a working-form session.

        pub fn batch(&mut self) -> Batch<'_> { todo!() }

        pub fn encode(&self) -> Vec<u8> { todo!() }

        pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> { todo!() }

    }

    impl Default for Version { fn default() -> Self { Self::new() } }

    impl PartialOrd for Version { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { todo!() } }

    impl core::fmt::Debug for Version { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { todo!() } }

    /// A working-form session over a `Version`. The event-tree complexity

    /// (fill/grow) lives in [`tick`](Self::tick). Repacks on drop.

    pub struct Batch<'v> { _p: PhantomData<&'v mut Version> /* work: WorkingVersion (lazy) */ }

    impl<'v> Batch<'v> {

        /// Advance `party`'s component. Chainable. **Core event operation.**

        pub fn tick(&mut self, party: &Party) -> &mut Self { todo!() }

        /// Merge another history in place. Chainable.

        pub fn merge(&mut self, other: &Version) -> &mut Self { todo!() }

    }

    impl Drop for Batch<'_> { fn drop(&mut self) { /* repack into *version if materialized */ } }

    impl<'a> From<&'a mut Version> for Batch<'a> { fn from(v: &'a mut Version) -> Self { v.batch() } }

    impl BitOr<Version> for Version { type Output = Version; fn bitor(self, r: Version) -> Version { todo!() } }

    impl BitOrAssign<Version> for Version { fn bitor_assign(&mut self, r: Version) { todo!() } }

    impl BitOrAssign<&Version> for Batch<'_> { fn bitor_assign(&mut self, r: &Version) { self.merge(r); } }

    // causal comparison across {Version, Batch}², reading current state in place.

    impl PartialEq<Batch<'_>> for Version { fn eq(&self, o: &Batch<'_>) -> bool { todo!() } }

    impl PartialOrd<Batch<'_>> for Version { fn partial_cmp(&self, o: &Batch<'_>) -> Option<Ordering> { todo!() } }

    impl PartialEq<Version> for Batch<'_> { fn eq(&self, o: &Version) -> bool { todo!() } }

    impl PartialOrd<Version> for Batch<'_> { fn partial_cmp(&self, o: &Version) -> Option<Ordering> { todo!() } }

    impl<'a, 'b> PartialEq<Batch<'b>> for Batch<'a> { fn eq(&self, o: &Batch<'b>) -> bool { todo!() } }

    impl<'a, 'b> PartialOrd<Batch<'b>> for Batch<'a> { fn partial_cmp(&self, o: &Batch<'b>) -> Option<Ordering> { todo!() } }

}

// ───────────────────────────── clock ─────────────────────────────

pub mod clock {

    use core::marker::PhantomData;

    use core::ops::{BitOr, BitOrAssign};

    use crate::{version, Party, Version, OverlapError, DecodeError};

    /// A `Party` paired with a `Version`. Not `Clone`. Implements no comparison

    /// traits — compare the party and version separately with any lexicography.

    pub struct Clock { party: Party, version: Version }

    impl Clock {

        pub fn seed() -> Self { Self::from_parts(Party::seed(), Version::new()) }

        pub fn from_parts(party: Party, version: Version) -> Self { Clock { party, version } }

        pub fn into_parts(self) -> (Party, Version) { (self.party, self.version) }

        pub fn party(&self) -> &Party { &self.party }

        /// Snapshot the history as a transmittable `Version`. Does not advance.

        pub fn version(&self) -> Version { self.version.clone() }

        pub fn tick(&mut self) { self.batch().tick(); }

        pub fn fork(&mut self) -> Clock { self.batch().fork() }

        pub fn join(&mut self, other: Clock) -> Result<(), Clock> { self.batch().join(other) }

        pub fn sync(&mut self, other: &mut Clock) -> Result<(), OverlapError> {

            self.batch().sync(&mut other.batch())

        }

        pub fn has_seen(&self, msg: &Version) -> bool { todo!() }

        pub fn happens_before(&self, other: &Clock) -> bool { todo!() }

        pub fn concurrent_with(&self, other: &Clock) -> bool { todo!() }

        pub fn send(&mut self) -> Version { self.tick(); self.version() }

        pub fn receive(&mut self, msg: Version) { self.batch().merge(&msg).tick(); }

        pub fn batch(&mut self) -> Batch<'_> { todo!() }

        pub fn encode(&self) -> Vec<u8> { todo!() }

        pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> { todo!() }

    }

    impl core::fmt::Debug for Clock { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { todo!() } }

    /// A session over a `Clock`, built on `version::Batch`. Repacks on drop.

    pub struct Batch<'c> {

        _p: PhantomData<&'c mut Clock>,

        // party: &'c mut Party,

        // version: version::Batch<'c>,

    }

    impl<'c> Batch<'c> {

        pub fn tick(&mut self) -> &mut Self { /* self.version.tick(&*self.party); */ self }

        pub fn merge(&mut self, msg: &Version) -> &mut Self { todo!() }

        pub fn fork(&mut self) -> Clock { todo!() }                 // splits party; child gets the current version

        pub fn join(&mut self, other: Clock) -> Result<(), Clock> { todo!() }

        pub fn sync(&mut self, other: &mut Batch<'_>) -> Result<(), OverlapError> { todo!() }

        /// The in-progress version, for comparison (no repack).

        pub fn version(&self) -> &version::Batch<'c> { todo!() }

        /// The current party (may have changed via fork/join/sync).

        pub fn party(&self) -> &Party { todo!() }

    }

    impl Drop for Batch<'_> { fn drop(&mut self) { /* repack version into *clock */ } }

    impl<'a> From<&'a mut Clock> for Batch<'a> { fn from(c: &'a mut Clock) -> Self { c.batch() } }

    // join operators. The `Clock` operand is consumed (a borrowing form would

    // duplicate its party). No `Clock | Clock` — that is the fallible `Clock::join`.

    impl BitOr<Version> for Clock { type Output = Clock; fn bitor(self, r: Version) -> Clock { todo!() } }

    impl BitOr<Clock> for Version { type Output = Clock; fn bitor(self, r: Clock) -> Clock { todo!() } }

    impl BitOrAssign<Version> for Clock { fn bitor_assign(&mut self, r: Version) { todo!() } }

    impl BitOrAssign<&Version> for Batch<'_> { fn bitor_assign(&mut self, r: &Version) { self.merge(r); } }

}

// #[cfg(feature = "serde")] Serialize/Deserialize for Party, Version, Clock via encode/decode.

```

---

## Appendix C — `CLAUDE.md` (drop into the repo root)

```markdown

# itc — Interval Tree Clocks

Safe-Rust ITC: packed `BitVec` storage, transient fixed-width working form for

mutation, linear-typed API. The full design and execution plan is in

`IMPLEMENTATION_PLAN.md` — it is frozen; follow it, don't redesign.

## Commands

- Build:  `cargo build`

- Test:   `cargo test --all-features`

- Lint:   `cargo clippy --all-targets --all-features -- -D warnings`

- Format: `cargo fmt --all`

- Verification gate (run all three, must be clean, before every commit):

  fmt → clippy → test.

## Workflow (always)

- Build order: implement the oracle and make its property suite (Appendix D) pass

  before writing ANY implementation code.

- TDD: write the phase's tests FIRST and confirm they fail before implementing.

  Do not write implementation ahead of its tests.

- Make minimal, scoped changes; do not refactor unrelated code.

- One commit per phase (or logical sub-step), descriptive message.

- When unsure between two approaches, stop and explain both rather than guessing.

## Hard rules (always)

- No `unsafe` (crate has `#![forbid(unsafe_code)]`).

- Every tree traversal is ITERATIVE (explicit stack / preorder cursor). No

  recursion on tree depth — deep inputs must not overflow.

- Never mutate integers in the packed form in place; arithmetic happens only in the

  fixed-width working form, repacked at the batch boundary.

- `decode` strictly rejects non-canonical (non-normal-form) input; canonical

  byte-equality is relied on for `Eq`/`Hash`.

- `Party`/`Clock` are not `Clone`; `Version` is. Don't add `Clone` to the first two

  or borrowing `BitOr` overloads for `Clock` (would duplicate a party).

- The public API matches `IMPLEMENTATION_PLAN.md` Appendix B exactly.

## Layout

- `src/lib.rs`        crate root, re-exports, errors, `#![forbid(unsafe_code)]`

- `src/party.rs`      `party::Party` (+ packed id ops)

- `src/version.rs`    `version::{Version, Batch}` (+ event codec, working form, ops)

- `src/clock.rs`      `clock::{Clock, Batch}`

- `src/codec.rs`      bit I/O, Elias-gamma integer code, encode/decode + validation

- `src/oracle.rs`     `#[cfg(test)]` reference oracle — mirrors the target API; ground truth

- `tests/`            property tests (proptest) + the differential harness

## Testing

- Property tests via `proptest`; generate values via ops from a seed (always valid,

  normal-form, party-disjoint). The oracle (mirrors the API) must pass its property

  suite (Appendix D) first. The differential harness then checks impl vs oracle by

  **structural** agreement — lower both to oracle trees — after every op; the byte

  codec is tested separately. Keep deep-tree (≥100k) and decode-fuzz tests in CI.

```

---

## Appendix D — Oracle property suite (Phase 0 gate)

Build the oracle (Appendix A) and make every property below pass **before** starting

any implementation work. These establish that the oracle is a faithful (if suboptimal)

realization of the paper, so it can be trusted as differential ground truth. Use

`proptest`; generate values via operations from a seed (always valid, normal-form, and

— for populations — pairwise party-disjoint). all three oracle types derive `Clone` (a reference oracle needs cheap snapshots; the

production types instead enforce linearity via `!Clone` + compile-fail tests), so capture

any "before" state with `.clone()` and compare structurally.

**O1. Genesis & identity.** `Clock::seed()` decomposes to `(Party::seed(),

Version::new())`. `Version::new()` is the identity for `|` (both `new() | v == v` and

`v | new() == v`).

**O2. Normal form is preserved by every operation.** For every value produced by any

oracle op (tick / fork / join / sync / receive / `|`), `is_normal()` holds. (Proves

the normalizing constructors actually normalize on every path — the property the whole

design leans on.)

**O3. Causal order is a partial order.** On random `Version`s: reflexive (`v <= v`);

antisymmetric (`a <= b && b <= a` ⇒ `a == b`); transitive; `a == b` ⇔

`partial_cmp == Some(Equal)`; concurrency ⇔ `None`.

**O4. `tick` strictly advances.** For a `Version v` and `Party p`:

`let mut w = v.clone(); w.tick(&p);` gives `v < w` (`v <= w` and `!(w <= v)`) and

`w != v`.

**O5. Join is a bounded join-semilattice (the LUB).** Commutative (`a|b == b|a`),

associative, idempotent (`a|a == a`), identity (O1); upper bound (`a <= a|b`,

`b <= a|b`); and least: for random `c` with `a <= c` and `b <= c`, `a|b <= c` (build

such `c` as `(a|b) | extra`).

**O6. The order is induced by the join.** `a <= b` ⇔ `a|b == b`. (Strong internal

consistency tying `leq` to `ev_join`; a bug in either surfaces here.)

**O7. Party fork is invertible by join.** `let b = a.fork(); a.join(b)` (Ok) recovers

the original party (`a` equals the pre-fork snapshot — clone `a` first); `a.is_disjoint(&b)` held

before the join; a clock's version is unchanged by `fork`.

**O8. Party order is descent.** Each fork child is `>` its parent; reflexive /

antisymmetric / transitive; `is_disjoint` correct (siblings disjoint; an

ancestor/descendant pair is not disjoint); for disjoint parties `join` is their meet

(greatest lower bound under descent); `join`/`sync` return `Err` on overlapping parties

(and `join` hands the party back unchanged).

**O9. Disjointness invariant.** Over any seed-derived trace of fork/join/sync, all live

parties are pairwise disjoint (so join/sync never error) and their overall `sum` equals

`Party::seed()`'s region.

**O10. Fork preserves history; peek doesn't advance.** Both fork halves carry the

parent's version; `clock.version()` leaves the clock unchanged and returns its current

version.

**O11. Dominated receive == tick; re-delivery is idempotent.** If

`msg <= a.version()` then `a.receive(msg)` yields the same version as a bare

`a.tick()` (merging a dominated version is a no-op). At the version level,

`v | m | m == v | m`.

**O12. `sync` reconciles and re-splits.** After `a.sync(&mut b)`: `a.version() ==

b.version()`, both equal `a_pre.version() | b_pre.version()`; the parties are disjoint;

and `a.party` joined with `b.party` recovers `a_pre.party + b_pre.party` (the re-split

loses no ownership).

**O13. Heterogeneous joins match.** `Clock|Version`, `Version|Clock`, `Clock|=Version`

all change only the version, to the `ev_join` of the two; `Version|Version` is

`ev_join`. A bare `Version` behaves exactly as a clock with party `0` would in these

joins.

**O14. Causal-order theorem (the property ITC exists for).** Over a generated

execution — forks, ticks, and deliveries where each delivered `Version` was a prior

`send`/`version()` of some clock — the partial order on the resulting versions agrees

with the happens-before order of the underlying events: a tick on a clock that has seen

`v` yields a version `> v` and strictly above that clock's prior version; two forked

clocks that tick without ever exchanging messages are concurrent (incomparable); no

version ever decreases; and `join` never loses prior knowledge (`a <= a|b`).

**O15. Worked example (paper §5.1).** Reproduce the paper's Section 5.1 trace

step-by-step (seed → fork → ticks → join, etc.) as a deterministic unit test and assert

the resulting trees equal the paper's stated values.

**Gate: all of O1–O15 green. Only then proceed to Phase 1.**