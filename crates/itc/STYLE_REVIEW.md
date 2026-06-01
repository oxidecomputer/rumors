# `itc` — Style, Readability & Presentation Review

A review along a *different axis* from the existing
[REVIEW_FINDINGS.md](file:///Users/oxide/src/rumors/crates/itc/REVIEW_FINDINGS.md),
which covers correctness, coverage, and paper conformance. This document is about
**idiomatic Rust style, structural presentation (macro → micro), and how clearly the
tricky algorithms read** — no correctness claims, no test-coverage claims. Where the two
overlap I defer to the correctness doc.

Scope: every `src/**.rs` file read in full; tests/benches surveyed for structure.

---

## 0. Headline assessment

This is, frankly, unusually well-presented systems code. The module decomposition is
clean and single-responsibility; every non-obvious decision carries a rationale; the
iterative-traversal discipline is honoured everywhere and *explained*; the "thread
register" abstraction is named and documented as a first-class concept. The oracle is a
genuinely beautiful recursive spec sitting next to the optimized impl.

So the recommendations below are not about rescuing the code — they're about taking
already-good code from "a careful reader can follow it" to "the structure teaches the
algorithm." The improvements cluster into four themes:

1. **The two-tree threaded walks repeat a structural skeleton** that is described in prose
   but not embodied in shared types — most visibly the symmetric `a`/`b` bookkeeping,
   which is spelled out longhand four times per machine.
2. **The recursive oracle is the natural form of every tricky algorithm**, but the
   iterative machines don't *link* to their oracle twin. Closing that loop is the
   single highest-leverage readability win, and it costs only doc comments.
3. **Named structs would replace positional tuples and inline bit-tests** at dozens of
   call sites (`header` returns, id empty/full tests).
4. **Pervasive trait-impl boilerplate** (the `{Version,Batch}²` comparison matrix, the
   `BitOr` families, the three hand-rolled error types) is mechanical and could shrink.

And one **documentation-coherence defect**: `IMPLEMENTATION_PLAN.md` is deleted from the
working tree but is still cited ~20 times across module docs (`plan §5.5`, `plan Appendix
A`, `Phase 5`, `See IMPLEMENTATION_PLAN.md`). Those references now dangle, and should be
removed, because public documentation shouldn't refer to internal ephemeral planning docs.

---

## 1. Macro level — architecture & cross-cutting structure

### M1. Dangling references to the deleted plan *(do first; cheap, and it's misleading today)*

`git status` shows `D IMPLEMENTATION_PLAN.md` — it's gone from disk. Yet:

- [src/lib.rs:20](file:///Users/oxide/src/rumors/crates/itc/src/lib.rs) — "See `IMPLEMENTATION_PLAN.md` for the full, frozen design."
- [src/codec.rs:2](file:///Users/oxide/src/rumors/crates/itc/src/codec.rs) — "See `IMPLEMENTATION_PLAN.md` §5."
- ~18 more `plan §N` / `plan Appendix A` / `Phase N` citations in
  [party/ops.rs:1](file:///Users/oxide/src/rumors/crates/itc/src/party/ops.rs),
  [version/working.rs:1](file:///Users/oxide/src/rumors/crates/itc/src/version/working.rs),
  [version/event.rs:1](file:///Users/oxide/src/rumors/crates/itc/src/version/event.rs),
  [version/compare.rs:8](file:///Users/oxide/src/rumors/crates/itc/src/version/compare.rs),
  [version/event/grow.rs](file:///Users/oxide/src/rumors/crates/itc/src/version/event/grow.rs),
  and the test module docs.

A reader following these hits a dead end, and `CLAUDE.md` still says the plan is the
frozen source of truth. Replace `plan §X` citations with either the
self-contained explanation (most are already self-contained — the citation is
vestigial) or a citation to the paper (`reference/itc2008.txt`, which *is* on disk and
is already cited by line number elsewhere — good).

Either way the `Phase N` / `Appendix D group A` scaffolding is internal-process
vocabulary that has outlived its purpose now that the code exists; it reads as
archaeology. Prefer naming *what the thing is* over *which phase built it*.

### M2. Module decomposition — keep as-is

`codec` (bit I/O) / `idbits` (id cursor) / `party::ops` / `version::{compare, working,
event, event::grow}` / `clock` / `oracle` is a clean dependency DAG with crisp
responsibilities. No grab-bag modules. This is exactly the architecture-first shape the
project aims for; I would not touch it.

### M3. Two iterative-traversal idiom *families*, used consistently within each — give the reader a map

There are four distinct shapes of "iterative tree walk" in the crate:

| Idiom | Where | Shape |
|---|---|---|
| Job-stack + `ret` thread register (`Eval`/`Right`/`Close`/`Combine`) | compare, ev_join, fill, grow probe/emit, sum, is_disjoint | threaded two-tree DFS |
| `NeedLeft`/`NeedRight` frame stack | codec parse_id/parse_ev/write_id/write_ev, split's pass 1 | single-tree build/print |
| Pending-counter (`pending: i64`) | idbits::skip, compare::skip, Builder::copy | subtree-span scan |
| `children_left` ancestor stack | ev_max | postorder accumulate |

Each family is internally consistent — that's good. But a reader meeting `ev_max`'s
`Ancestor { cumulative, children_left }` right after `ev_join`'s `Eval`/`Right`/`Close`
has to re-orient, because they're the same "iterative postorder" goal in two unrelated
spellings. Two cheap fixes, in order of value:

- **Document the taxonomy once** (e.g. in `version/event.rs`'s already-excellent module
  doc, or a short `ARCHITECTURE`/`internals.md`): "we use N traversal idioms; here's when
  each applies." This converts implicit pattern-knowledge into something a new reader can
  acquire in one place.
- Consider whether `ev_max` can be expressed in the dominant job-stack idiom (it computes
  `base + max(children)`, a textbook fold) so there's one fewer shape to learn. Only if
  it doesn't get longer — `ev_max`'s current form is tight, so this is a judgement call.

### M4. Do **not** attempt a grand unified "threaded DFS" trait *(anticipating the instinct)*

The natural systems-thinker move here is: "all these machines share a skeleton — extract a
`TreeFold`/`ThreadedWalk` trait and instantiate it six times." I think that's a trap, and
worth saying explicitly so it's a *considered* non-recommendation rather than an
oversight.

The machines diverge in ways that resist a single abstraction without leaking:
`is_disjoint` early-exits on `false`; `compare`/`causal_cmp` early-exit on `None` and do
*lazy-skip*; `causal_cmp`/`ev_join` thread `u64` offsets and *broadcast* a leaf side to
both of the other's children; `sum` folds child *outputs* (needs a results `Vec`, not a
bare register); `fill` is id-driven and asymmetric (three different close arms); `grow` is
a *two-pass* probe/emit pair coupled by a coordinate contract. A trait general enough to
host all of that would need so many hooks (early-exit, skip, broadcast, output-fold,
asymmetric-close) that each impl would be as long as today's code *plus* the trait
plumbing — and the efficiency margins (no re-scan, `O(n+m)`) are exactly what a generic
driver tends to erode. The user asked for clarity "without compromising efficiency or
correctness"; full unification compromises both.

**The right granularity is partial** (themes M5, S1, S3 below): extract the *mechanical*
shared pieces, leave each machine's control flow legible and its own.

### M5. Close the loop to the recursive oracle — the highest-leverage readability win

The oracle (`src/oracle.rs`) is the *natural form* of every tricky algorithm: `leq`
(8 lines), `join_off`, `fill`, `grow` are clear, recursive, and obviously-correct. The
iterative machines are CPS-transformed versions of exactly these. Today the connection
lives only in scattered prose ("the recursive `leq` of the paper made iterative" —
[compare.rs:8](file:///Users/oxide/src/rumors/crates/itc/src/version/compare.rs)).

Make it a structural, navigable link. Each iterative machine's doc comment should name and
point to its oracle twin, e.g. on `causal_cmp`:

```
/// The iterative, offset-threaded form of [`oracle::Version::leq`] run in both
/// directions at once. Read that 8-line recursive version first — this is the same
/// algorithm with the call stack made explicit and the two `leq` directions fused.
```

…and similarly `ev_join` → `oracle::Version::join_off`, `fill` → `oracle::Version::fill`,
`grow_probe`/`grow_emit` → `oracle::Version::grow`, `compare` → `oracle::Party::contains`
(×2), `sum` → `oracle::Party::sum`, `split` → `oracle::Party::split`. A reader then climbs
a ladder: recursive spec → "made iterative by threading" → the machine. This is pure
documentation, zero risk, and it's how I'd want to *learn* this code. (The oracle is
`cfg(test)`/`oracle`-feature only, so intra-doc links to it won't resolve in a default
`cargo doc`; use a plain-text reference, or gate a richer link under the feature.)

---

## 2. Meso level — within-module patterns

### S1. Group the symmetric `a`/`b` bookkeeping into a `Side` struct *(biggest in-code clarity gain)*

`ev_join` ([version/event.rs:148-194](file:///Users/oxide/src/rumors/crates/itc/src/version/event.rs))
is the clearest case. `JoinJob::Right` carries **eight** fields:

```rust
Right {
    a_internal: bool, a_sum: u64, a_pos: usize, a_off: u64,
    b_internal: bool, b_sum: u64, b_pos: usize, b_off: u64,
}
```

and the same a/b mirror logic is written out four times (left-child in `Eval`,
right-child in `Right`, twice). The asymmetry it encodes — *an internal side threads /
descends; a leaf side re-broadcasts in place* — is the actual idea, and it's currently
implicit in the longhand `if a_internal { (a_next, a_sum) } else { (a_pos, a_off) }`.

Grouping makes the symmetry a *type* and names the idea:

```rust
/// One input's state at a node in the two-tree walk.
#[derive(Clone, Copy)]
struct Side { internal: bool, pos: usize, off: u64, sum: u64, next: usize }

impl Side {
    /// Where this side's left child starts: descend if a node, else re-broadcast the leaf.
    fn left(self) -> (usize, u64) { if self.internal { (self.next, self.sum) } else { (self.pos, self.off) } }
    /// Where this side's right child starts, given where its left subtree ended (`ret`).
    fn right(self, threaded_end: usize) -> (usize, u64) {
        if self.internal { (threaded_end, self.sum) } else { (self.pos, self.off) }
    }
}
```

`JoinJob::Right { a: Side, b: Side }` — two fields, not eight — and the broadcast rule
appears *once*. The same `Side` cleans up `causal_cmp`'s three `Right*` variants
([compare.rs:107-139](file:///Users/oxide/src/rumors/crates/itc/src/version/compare.rs)),
where `RightLockstep` / `RightBroadcastB` / `RightBroadcastA` are three spellings of the
identical "thread the internal side, pin the leaf side." This is the partial-unification
that M4 endorses: it compresses the bookkeeping without touching control flow, early-exit,
or the `O(n+m)` guarantee. I'd rank it the most worthwhile code change in the review.

### S2. The job enums are well-documented but field-heavy — order and naming are good; consider `#[derive(Clone, Copy)]` consistency

Variants are already in traversal order and per-field doc comments are excellent. After S1
the field counts drop sharply. Two small consistency notes:

- Enum names vary: `Job` (compare) vs `JoinJob`/`FillJob`/`SumJob`/`ProbeJob`/`EmitJob`
  (everywhere else) vs `Pair` (is_disjoint/compare in party::ops). Standardize on
  `<Machine>Job` (or `…Step`) so the reader knows "this is the step enum for machine X"
  by name alone. `Pair` in particular ([party/ops.rs:48](file:///Users/oxide/src/rumors/crates/itc/src/party/ops.rs))
  doesn't signal that it's the same job-stack idiom.
- The register structs (`Joined`, `Built`, `Probed`, `Ends`, `Summed`) are a nice
  consistent family — each is "the thread register for machine X." A one-line convention
  note (they all play the `ret` role described in `event.rs`'s module doc) at each
  definition keeps them recognizable; most already have it.

### S3. Add `idbits::is_empty` to match `is_full`; retire the inline `(false, false)` test

[idbits.rs:46](file:///Users/oxide/src/rumors/crates/itc/src/idbits.rs) provides
`is_full`, and the module doc explicitly notes emptiness is "an inline `(false, false)`
header test." That inline test — `!a_node && !a_val` / `let (a_full, a_empty) = (!a_node
&& a_val, !a_node && !a_val)` — recurs in `is_disjoint`, `compare`, `sum`, `fill`, and
`grow`. It's exactly the kind of magic bit-pattern the project's "named constants over
magic numbers" instinct argues against. A symmetric `is_empty(bits, at) -> bool` (and
perhaps `is_full_or_empty`) makes those sites read as `if id.is_empty(pos)` rather than
`if !id_node && !id_val`, and documents the normal-form invariant (empty ≡ `0` leaf) in
one place instead of five.

### S4. A single generic `skip`, not two hand-synced copies

[idbits::skip](file:///Users/oxide/src/rumors/crates/itc/src/idbits.rs) and
[compare::skip](file:///Users/oxide/src/rumors/crates/itc/src/version/compare.rs) are the
*same* pending-counter algorithm on two header shapes, and the docs literally say "keep
the two in step." Whenever the docs ask the reader to keep two functions manually
synchronized, that's a signal to express the shared part once. The pending-counter loop
generalizes cleanly over "give me `(is_internal, next)` at a position":

```rust
fn skip(mut at: usize, mut header: impl FnMut(usize) -> (bool, usize)) -> usize {
    let mut pending: i64 = 1;
    while pending > 0 {
        let (internal, next) = header(at);
        at = next;
        pending += if internal { 1 } else { -1 };
    }
    at
}
```

`Builder::copy` ([event.rs:93](file:///Users/oxide/src/rumors/crates/itc/src/version/event.rs))
runs the same loop while also emitting — it can keep its own copy (it does extra work) but
should reference the shared one. Low risk, removes a stated maintenance hazard.

### S5. Name the `header` return instead of returning a positional triple

Every walk opens with `let (a_node, a_val, a_next) = header(a, a_pos);` /
`let (a_internal, a_base, a_next) = view.header(a_pos);`. The triple is unlabelled at the
type level; only the destructuring names rescue it, and they vary
(`a_node`/`a_internal`, `a_val`/`a_base`). A named struct documents the shape once:

```rust
struct IdHeader  { node: bool, val: bool, next: usize }
struct EvHeader  { internal: bool, base: u64, next: usize }
```

Counterpoint worth weighing: the destructuring is concise and the names *are* descriptive
per-site, so this is a genuine taste call — named fields cost a little verbosity
(`h.node`, `h.next`) for a lot of type-level documentation. I lean toward the struct for
`EvView::header` (the `(bool, u64, usize)` triple is the easiest to misread), and would
accept leaving `idbits::header` as a triple. Decide once and apply uniformly.

### S6. `Builder::close_node` — the crown jewel, slightly under-narrated at the truncate

`close_node` ([event.rs:118-131](file:///Users/oxide/src/rumors/crates/itc/src/version/event.rs))
is the single place normalization lives, and the adjacency-precondition doc comment is
exemplary. One micro-gap: the `truncate(node)` + re-`leaf` dance ("discards exactly those
three slots") is correct but reads as a small sleight of hand. A one-line assert pinning
the invariant the prose promises would make it self-checking:

```rust
debug_assert_eq!(right, node + 2, "collapse precondition: both children are adjacent leaves");
```

(This complements, not duplicates, REVIEW_FINDINGS — that doc verifies the invariant
*holds*; this makes the *code* state it.)

---

## 3. Micro level — local idiom & legibility

### µ1. `encode_int`'s `#[allow(dead_code)]` looks stale

[codec.rs:26-27](file:///Users/oxide/src/rumors/crates/itc/src/codec.rs):

```rust
// Used by the cfg(test) oracle bridge now and by `repack` from Phase 2 onward.
#[allow(dead_code)]
pub(crate) fn encode_int(out: &mut Bits, n: u64) {
```

`repack` ([version/working.rs:58](file:///Users/oxide/src/rumors/crates/itc/src/version/working.rs))
is always compiled and calls `encode_int`, so the function is *not* dead in a normal
build. The `allow` and its "from Phase 2 onward" comment are almost certainly vestigial
from staged construction. Verify clippy stays clean without them, then delete both — a
lingering `allow(dead_code)` on live code trains the reader to distrust such annotations.

### µ2. `Cur`'s `b`/`i` field names

[codec.rs:385-388](file:///Users/oxide/src/rumors/crates/itc/src/codec.rs) — `struct Cur {
b: &[u8], i: usize }`. The parser cursor is small and local, but `bytes`/`pos` (or `at`)
costs nothing and matches the naming density of the rest of the file. The methods
(`skip_ws`, `peek`, `bump`) are well-chosen; only the fields are terse.

### µ3. Comment indentation slip in `grow_probe`

[grow.rs:274-275](file:///Users/oxide/src/rumors/crates/itc/src/version/event/grow.rs) —
the `// Strict <` comment is indented to hang off the previous statement rather than
aligned with the `let left_chosen` it describes:

```rust
                let right = ret; // the right child's probe report
                                 // Strict `<` makes a tie favor the right child (see [`Cost`]).
                let left_chosen = left_cost < right.cost;
```

Move it to its own line above `left_chosen`. Tiny, but it's the one spot `cargo fmt`
won't catch and the eye snags on.

### µ4. Stale item names in docs (`contains`)

- [idbits.rs:2](file:///Users/oxide/src/rumors/crates/itc/src/idbits.rs) — "shared by the
  party operations (`split`/`sum`/`is_disjoint`/`contains`)". There is no `contains` in
  the impl; the function is `compare`. Same drift in `party/ops.rs`'s framing and in
  `test_support.rs`'s `contain_stress_pair` doc, which narrates "`contains` returns
  `true`" for a path that actually exercises `compare`.

These are the kind of small inaccuracies that erode trust in otherwise-precise docs.
Rename to `compare` (or note "the containment direction of `compare`").

### µ5. The `(false, false)` / unreachable match arms

`compare` and `causal_cmp` both end with:

```rust
match (le, ge) {
    (true, true) => Some(Ordering::Equal),
    (true, false) => Some(Ordering::Less),
    (false, true) => Some(Ordering::Greater),
    (false, false) => None, // Unreachable: both-false returns None inside the loop.
}
```

The comment is good. Consider `unreachable!()` with the same message instead of silently
returning `None` for the impossible arm — it documents the invariant as an assertion and
would catch a future edit that breaks the early-exit. (Marginal; the current form is also
defensible as "total and harmless." Taste.)

### µ6. `pub(crate) fn empty()` on `Party` — naming vs. the linearity story

[party.rs:75](file:///Users/oxide/src/rumors/crates/itc/src/party.rs) — `Party::empty()`
mints the anonymous identity purely as a `mem::replace` placeholder in `sync`. The doc is
clear that it's transient and never escapes. Two thoughts: (a) the name `empty` collides
conceptually with `codec::id_is_empty`; `anonymous()` would
signal "not a real value" at the call site
([clock.rs:229](file:///Users/oxide/src/rumors/crates/itc/src/clock.rs)). (b) This is also
the crux of REVIEW_FINDINGS' open editorial question (PAP-2: "is an empty-region Party
possible?") — worth resolving the name and the invariant together.

### µ7. Inherent `Version::new()` vs `Default` — good as-is

Both exist, `Default` delegates to `new`, `new` is the documented constructor. Idiomatic.
Same for the `From<&mut _>` → `batch()` conveniences. No change.

---

## 4. Pervasive trait-impl boilerplate

### B1. The `{Version, Batch}²` comparison matrix

[version.rs:224-260](file:///Users/oxide/src/rumors/crates/itc/src/version.rs) — six
`PartialEq`/`PartialOrd` impls, each a one-liner delegating to `causal_cmp(&self.view(),
&o.view())`. The symmetry is invisible because it's typed out six times. A small local
declarative macro makes the *pattern* the thing you read:

```rust
macro_rules! causal_cmp_impls {
    ($($lhs:ty, $rhs:ty);* $(;)?) => { $(
        impl PartialEq<$rhs> for $lhs {
            fn eq(&self, o: &$rhs) -> bool { causal_cmp(&self.view(), &o.view()) == Some(Ordering::Equal) }
        }
        impl PartialOrd<$rhs> for $lhs {
            fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> { causal_cmp(&self.view(), &o.view()) }
        }
    )* };
}
causal_cmp_impls! { Version, Batch<'_>; Batch<'_>, Version; Batch<'_>, Batch<'_>; }
```

Trade-off to weigh honestly: a macro hurts grep-ability and IDE "go to impl." For *six*
near-identical impls behind one obvious helper, I think the legibility win (you see the
matrix as a matrix) outweighs it — but it's a preference call, and the current explicit
form is not wrong.

### B2. The `BitOr` families on `Version` and `Clock`

[version.rs:203-222](file:///Users/oxide/src/rumors/crates/itc/src/version.rs) and
[clock.rs:264-290](file:///Users/oxide/src/rumors/crates/itc/src/clock.rs) — same
observation, smaller scale. The by-value/by-ref/assign variations are deliberate
(linearity), so they're less mechanical than B1; I'd leave these *explicit* precisely
because the by-value-consumes-the-party subtlety deserves to be read, not generated. The
excellent comment at clock.rs:261 ("a borrowing form would duplicate its party") is the
reason to keep them longhand. Mentioning for completeness, not recommending change.

### B3. Three hand-rolled error types

`OverlapError`, `DecodeError`, `ParseError` each hand-implement `Display` + `Error`
([lib.rs:103-168](file:///Users/oxide/src/rumors/crates/itc/src/lib.rs)). The project's
stated philosophy is "prefer libraries over hand-rolling — would rather add a dependency
than own maintenance burden," which points at `thiserror`:

```rust
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("input ended mid-tree")]
    Truncated,
    #[error("trailing or nonzero padding bits after a complete tree")]
    TrailingBits,
    #[error("input is well-formed but not in canonical normal form")]
    NotCanonical,
}
```

Human editor note: great, let's do it.

---

## 5. Tests & benches (structure only)

Surveyed via a dedicated pass; details folded here. The suite is well above average —
every test has an English-invariant doc comment (per `CLAUDE.md`), proptest strategies are
clean, `prop_assert!`/`prop_assert_eq!` are used over bare `assert!`. The structural notes:

- **T1. Inconsistent section dividers.**
  [clock/tests.rs](file:///Users/oxide/src/rumors/crates/itc/src/clock/tests.rs) uses
  `// ──── section ────` dividers and is a joy to scan; `party/tests.rs`,
  `version/tests.rs`, `oracle/tests.rs` don't, and with dozens of small functions they're
  hard to navigate. Apply the divider convention uniformly.
- **T2. Duplicated test helpers.** `steps_of`, `le`, and `MIN_SCALE`/`TREE` are
  copy-defined in both `party/tests.rs` and `version/tests.rs` (and `le` again inline in
  `oracle/tests.rs`). Lift `steps_of` and the complexity-scale constants into
  `test_support`. The `le` helpers are type-specific; a small generic or a macro would do,
  but they're short enough that duplication is defensible — at minimum unify the *name* and
  the constant value (one file calls it `MIN_SCALE = 64`, another `TREE = 64`, unrelated
  by name).
- **T3. A few tests lack the project-mandated doc comment** — e.g.
  `parse_bare_notation` (party/tests.rs) and `gamma_costs` (codec/tests.rs) state *what*
  but not the *invariant/why* (the Elias-gamma cost table). Cheap to add.
- **T4. `benches/common/mod.rs` helpers are undocumented.** The module-level algorithm
  comment is great, but `impl_parties`/`oracle_parties`/`impl_clocks` have no per-fn doc.
  Add one line each referencing the module algorithm.
- **T5. Naming refers to retired plan doc.** Where appropriate, names should refer to the
  paper directly, or be merely descriptive without a reference to a doc.

---

## 6. Synthesized step-by-step plan

Ordered by **risk-ascending and dependency**: documentation-only first (zero behavioral
risk, immediate legibility payoff), then mechanical local refactors behind the test suite,
then the judgement-call items. Each step is a self-contained commit; run the verification
gate (fmt → clippy → `nextest`, per `CLAUDE.md`) before each. Nothing here changes runtime
behavior except where noted; the differential harness guards the refactors.

**Phase A — Documentation coherence (no code change).**
1. **M1**: Resolve the deleted-plan question. Strip or re-target the ~20 `plan §`/`Phase`
   citations; keep paper (`reference/itc2008.txt`) citations. Update `CLAUDE.md` if the
   plan is retired. *(Touches doc comments across ~10 files; no logic.)*
2. **M5**: Add "iterative form of [`oracle::…`]" cross-references to each tricky machine's
   doc (`causal_cmp`, `ev_join`, `fill`, `grow_probe`/`grow_emit`, `compare`, `sum`,
   `split`). *(Doc only; highest readability-per-effort in the whole review.)*
3. **µ4**: Fix stale `contains` → `compare` references in `idbits.rs`, `party/ops.rs`,
   `test_support.rs`.
4. **M3**: Add the traversal-idiom taxonomy note (one place, e.g. `version/event.rs`
   module doc or a short `src/internals.md`).

**Phase B — Mechanical micro-cleanups (trivially verifiable).**
5. **µ1**: Remove the stale `#[allow(dead_code)]` + comment on `encode_int` (confirm
   clippy clean).
6. **µ3**: Fix the `grow_probe` comment indentation.
7. **µ2**: Rename `Cur { b, i }` → `{ bytes, pos }`.
8. **T3/T4**: Add the missing test/bench doc comments.
9. **T1**: Add section dividers to `party/`, `version/`, `oracle/` test files.

**Phase C — Local refactors behind the harness (low risk, real structural payoff).**
10. **S3**: Add `idbits::is_empty`; replace the inline `(false,false)` tests in
    `is_disjoint`/`compare`/`sum`/`fill`/`grow`. *(Pure substitution; harness + complexity
    tests cover it.)*
11. **S4**: Extract the generic pending-counter `skip`; point `idbits::skip`,
    `compare::skip`, and `Builder::copy` at the shared core.
12. **S5**: (If adopted) introduce `EvHeader` (and optionally `IdHeader`) named returns;
    update call sites. Do this in one sweep so the codebase stays uniform.
13. **T2**: Lift `steps_of` + scale constants into `test_support`; unify `MIN_SCALE`/`TREE`.

**Phase D — The featured refactor (do after C, with the harness warm).**
14. **S1**: Introduce `Side` and fold the eight-field `JoinJob::Right` and the three
    `causal_cmp` `Right*` variants into `a: Side, b: Side` with named `left`/`right`
    helpers. This is the change that most makes the tricky algorithm *read* like its idea.
    Land it alone, lean on the differential + complexity proptests, and confirm step-count
    linearity is unchanged (it should be — same control flow).
15. **S2**: While in there, standardize the job-enum names (`<Machine>Job`), including
    `Pair` → `CompareJob`/`DisjointJob`.
16. **S6**: Add the `close_node` adjacency `debug_assert!`.

**Phase E — Judgement calls (decide, then do or explicitly defer).**
17. **B1**: The `causal_cmp_impls!` macro for the comparison matrix — adopt only if you
    prefer matrix-as-macro over six explicit impls.
18. **B3**: `thiserror` for the three error types — adopt if the dependency is acceptable;
    aligns with the stated "libraries over hand-rolling" philosophy.
19. **µ5/µ6**: `unreachable!` for the both-false arm; `Party::empty` → `placeholder`
    (coordinate with REVIEW_FINDINGS PAP-2).
20. **M3 (stretch)**: Re-express `ev_max` in the dominant job-stack idiom *only if* it
    doesn't grow.

**Explicit non-goal:** the grand unified threaded-DFS trait (M4). Record the decision so a
future reader (or agent) doesn't re-litigate it: the machines' divergence (early-exit,
lazy-skip, broadcast, output-fold, asymmetric close, two-pass coupling) makes a single
abstraction cost more clarity and efficiency than it saves.

---

## 7. What I verified vs. inferred

- **Verified by reading**: every claim about a specific `file:line` (field counts, the
  `header` triple shapes, the duplicated `skip`, the dangling plan references via `git
  status` + `grep`, `encode_int`'s live use by `repack`, the inline empty/full tests).
- **Inferred / taste**: that `Side` preserves the `O(n+m)` guarantee (it changes only
  field grouping, not control flow — but confirm via the existing complexity proptests
  after the change); that `thiserror` is `no_std`-compatible at 2.x (check before
  adopting); that the plan deletion is intentional (resolve with you — M1 assumes nothing).
- **Not assessed** (covered by REVIEW_FINDINGS): correctness, the `u64` overflow defect,
  test coverage adequacy, paper conformance.
