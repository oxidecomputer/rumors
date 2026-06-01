# `itc` — Review Findings & Hardening Backlog

Synthesized from a meticulous review (2026-05-30, branch `itc-crate`) covering
implementation correctness, test coverage, and paper conformance. Inputs: a full manual
read of every source and test file by the lead reviewer; three independent review agents
(test-coverage-vs-catalog, adversarial line-by-line correctness, and paper-conformance);
and one empirically reproduced defect.

---

## Current state

- **All 78 tests pass**; `cargo clippy --all-targets --all-features -D warnings` and
  `cargo fmt --check` are clean. Proptest regression seeds are committed under
  `proptest-regressions/`.
- The core algorithms (`grow` probe/emit, `fill`, `split`, `sum`, `compare`,
  `is_disjoint`, `causal_cmp`, `ev_join`) were verified faithful to both the recursive
  oracle and the paper §5.3 — by manual trace *and* an adversarial agent. In particular:
  `grow`'s lexicographic `(expansions, depth)` cost and right-favoring tie-break match
  the oracle (including the `Expand`-folding accounting); the `Choices` keying cannot
  collide; `close_node`'s truncate-adjacency invariant holds; `split`'s min-start
  both-nonempty branch provably equals the oracle's recursive split point.
- `decode`'s normal-form validation is sound **for in-range inputs**: it enforces both
  event predicates (≥1 zero-base child; no equal-leaf collapse) and the id collapse rule,
  and Elias-gamma is a canonical prefix code — so byte-equality `Eq`/`Hash` is sound for
  any value the operations actually produce.
- No recursion on tree depth anywhere; the depth-100k stack-safety test passes.

The findings below are the **negative space** around that: a confirmed arithmetic defect
on adversarial (non-op-generated) inputs, a set of coverage/conformance gaps, and a
program to raise confidence to the level the design deserves.

---

## The one structural theme

Almost every gap traces to a single property of the suite: **every test input is
produced by the operation pipeline** (`world_strategy`: a seed clock, then ≤30
fork/tick/join/sync/send ops). Consequences:

1. Operations are tested only on the tree *shapes operations produce* — never on the
   full space of valid normal-form trees that `decode`/`try_from`/`from_parts` accept.
2. The sole ground truth is the recursive oracle, so any bug the impl and oracle *share*
   (same assumptions) is invisible.

`BUG-1` below is exactly this blind spot made real — and notably, the adversarial
correctness agent *also* missed it, because it reasoned within the same "inputs come from
ops" frame. The highest-value items (`PROG-1`, `PROG-2`) break that coupling.

---

## P0 — Confirmed defect

### BUG-1 — Unchecked `u64` path-sum overflow: debug panic / release causality inversion

**Priority:** P0 (high for any deployment that decodes untrusted `Version`/`Clock`).
**Status:** reproduced empirically through the safe public API on 2026-05-30.

**Problem.** The offset-threaded comparison and join accumulate root-to-leaf path sums
with unchecked `u64` addition. `decode` and `Version::try_from` accept any *normal-form*
tree, and `parse_ev` validates only **relative** bases — it never sums a path — so a
canonical tree whose path sums exceed `u64::MAX` is admitted, then overflows downstream.

- **Debug:** panics (`attempt to add with overflow`) — a DoS on any compare/merge of an
  untrusted message.
- **Release:** silently wraps, **inverting the causal order** — the worst failure for a
  logical clock (a wrong happens-before answer, not a crash).

**Evidence.**
- `src/version/compare.rs:166-167` — `let a_sum = a_off + a_base;` / `b_sum` (causal_cmp).
- `src/version/event.rs:231-232` (`ev_join` Eval), `:235` (leaf `a_sum.max(b_sum)`),
  `:342` (`ev_max` `offset + base`) — same unchecked adds; `fill` overflows via `ev_max`.
- `src/codec.rs:29` — `let m = n + 1;` in `encode_int` overflows when `n == u64::MAX`.
  `decode_int` caps decodable `n` at `u64::MAX − 1` (`codec.rs:52`), so `decode→encode` is
  safe, but `Version::try_from(u64::MAX)` / a literal `repack` of such a base panics. Treat
  as a sub-item of the same fix.

**Reproduction (verified, then removed).** A throwaway `examples/overflow_probe.rs`:
```rust
let big = 1u64 << 63;
// Normal form: root min(big,0)=0; left node min(0,1)=0. Left half's true value = 2^64.
let a = Version::try_from((big, (big, 0u64, 1u64), 0u64)).unwrap();
let b = Version::try_from(big).unwrap();           // constant 2^63
// True: a's left half (2^64) > 2^63 ⇒ cmp(a,b) == Greater.
a.partial_cmp(&b)   // debug: PANIC at compare.rs:167
                    // release: Some(Less)  ← causality inverted
```

**Recommended fix (preferred: saturating arithmetic).**
1. Replace the unchecked adds at the four threading sites with `saturating_add`.
   Comparison/join stay monotone and total; in-range answers are unchanged; pathological
   inputs degrade gracefully instead of wrapping. (`grow` already uses `saturating_add`
   for its cost — this extends the same discipline to base sums.)
2. Audit `encode_int`/`repack` for the `n == u64::MAX` edge; either saturate, or reject at
   decode (option below).

Alternatives considered: (b) bound base magnitude at `decode` and carry a "path sums fit
`u64`" invariant on `Version` — more invasive, changes what counts as canonical; (c)
`checked_add` propagating a `DecodeError`/sentinel — most correct but ripples through
signatures. Recommend (1); revisit (b) only if a max-magnitude bound is independently
desirable.

**Acceptance criteria.**
- Unit regression test using the witness above: debug does **not** panic; the answer is
  correct (or documented-saturated) in both profiles.
- Property test over arbitrary normal-form trees with bases drawn to include values near
  `u64::MAX` (see `PROG-1`): `partial_cmp`, `|`, `has_seen`, `tick`, `decode` never panic.
- `cargo nextest run --release --no-fail-fast --all-features` green; fmt/clippy clean.

---

## P1 — Test coverage gaps (against the plan's own catalog / Definition of Done)

### COV-2 — serde round-trip only exercises `serde_json`, never the binary `serialize_bytes` path
**Priority:** P1. **Effort:** small.
**Problem.** `serde_impls.rs` serializes via `serialize_bytes(&self.encode())` and
deserializes via `Vec<u8>::deserialize`. The only test uses **serde_json**, where
`serialize_bytes` emits a JSON number-array and `Vec<u8>::deserialize` reads it back via
`visit_seq` — so it passes but never validates the intended binary path. `Vec<u8>` does
**not** implement `visit_bytes`/`visit_byte_buf`, so a self-describing binary format
(MessagePack/CBOR) that emits a typed bytes value would **fail to deserialize**. Downstream
users on bincode/postcard/rmp are unguarded.
**Evidence.** `src/serde_impls.rs:12,18` (and Version/Clock); `src/clock/tests.rs:679-699`
(`serde_roundtrip`, serde_json only).
**Fix.** Add a round-trip test over a true binary format (add `bincode` or `postcard` as a
dev-dep) and over a self-describing one (`rmp-serde`/`ciborium`). If a binary format
fails, switch the impls to `serde_bytes::ByteBuf` (or a manual `visit_bytes`+`visit_seq`
visitor) so both paths round-trip.
**Acceptance.** Party/Version/Clock round-trip through at least one binary and one
self-describing format under `--features serde`.

### COV-3 — `decode` non-canonical reject suite missing the `(0, 0)` id case
**Priority:** P2. **Effort:** trivial.
**Problem.** The plan's A3 enumerates "uncollapsed `(0,0)`/`(1,1)` id" — only `(1,1)` is
fed to `Party::decode`. (`o15_id_normalization` checks the oracle *constructor* collapses
`(0,0)`, not that `decode` *rejects* a `(0,0)` byte stream.)
**Evidence.** `src/codec/tests.rs:107-116` (`a3_reject_noncanonical_id`).
**Fix.** Add a `(0,0)` denormal-id reject assertion (and, while there, a deep/nested
denormal so the bottom-up validator's recursion is exercised, plus a non-zero **intra-byte
padding** bit case to complement the whole-trailing-byte case).
**Acceptance.** Both `(0,0)` and `(1,1)` denormal ids → `Err(NotCanonical)`.

### COV-4 — Deep-tree (H33) stack-safety test omits `sync`, `receive`/`send`, and clock observers
**Priority:** P2. **Effort:** small.
**Problem.** The plan's H33 says "every op + encode/decode" at ≥100k depth. The test
exercises `encode`/`decode`, `tick`, `partial_cmp`, `|`, `fork`, `join`, `Debug` — but not
`sync` (the most complex composite: fork+join+merge), `receive`/`send`, or
`has_seen`/`happens_before`/`concurrent_with` at depth.
**Evidence.** `src/clock/tests.rs:385-421` (`h33_deep_tree_stack_safety`).
**Fix.** Extend the deep test to drive `sync` between two deep clocks, a `receive`, and
each clock observer; assert no overflow.
**Acceptance.** Every public op runs against a ≥100k-depth structure without overflow.

### COV-5 — No compile-fail (`trybuild`) tests guarding linearity and the locked API
**Priority:** P2. **Effort:** small.
**Problem.** Plan §8 / `CLAUDE.md` state linearity (`Party`/`Clock` `!Clone`, by-value
`BitOr`) is "a type-level guarantee checked by compile-fail tests" — none exist. Nothing
fails if a future edit adds `Clone` to `Party` or a borrowing `BitOr` for `Clock`. The
"public API matches Appendix B exactly" DoD item has no automated guard.
**Fix.** Add `trybuild` dev-dep with `compile_fail` cases: cloning a `Party`/`Clock`;
using a `Clock` after a by-value `|`; a borrowing `Clock | Clock`. Optionally a doctest
`compile_fail` for the same.
**Acceptance.** Each forbidden pattern fails to compile with a stable error; the suite
catches a deliberately-introduced `#[derive(Clone)]` on `Party`.

### COV-6 / COV-7 — Minor
- **COV-6 [P3]:** A2 canonical injectivity is checked for `Version` and `Party` but not
  `Clock` directly (Clock has no `PartialEq`; covered only transitively via the harness).
  Consider an explicit Clock byte-injectivity property. `src/codec/tests.rs:83-102`.
- **COV-7 [P3]:** `h34_decode_never_panics` (256 cases, 0..512 bytes) is a thin panic net
  and does not assert `is_normal` on accepted values directly (only re-encode stability).
  Fold into `PROG-5` (coverage-guided fuzzing) and add the explicit
  `decode(b) == Ok(x) ⟹ is_normal(x)` assertion via the oracle lowering.
  `src/clock/tests.rs:423-438`.

---

## P1/P2 — Paper-conformance gaps

### PAP-1 — Event **minimality** / `grow` **optimality** is untested (the defining causality property)
**Priority:** P1 (it is *the* property ITC exists for). **Effort:** medium.
**Problem.** The paper's event condition (§3 L94-99, §5.3.4) requires `e < e'` **and**
that `e'` dominate no more than needed (`e' ≰ x`; `x < e' ⇒ x ≤ e`), delivered by `grow`
choosing the **cost-minimal** inflation. The suite tests strict advance (`o4`, `c15`),
impl==oracle (`tick_matches_oracle`), and grow's *linear time* (`grow_bushy_is_linear`) —
but **nothing asserts the chosen inflation is minimal**. The entire safety net is "impl ==
oracle," and the oracle's own minimality is never independently established.
**Evidence.** `src/version/tests.rs` (`tick_matches_oracle`, `grow_bushy_is_linear`);
paper `reference/itc2008.txt:94-99,443-462`.
**Fix.** (a) A brute-force oracle property: enumerate every feasible single-region
inflation of `(id, e)`, compute each one's true `(expansions, depth)` cost, and assert
`grow`'s output has globally minimal cost with the correct root-ward tie-break. (b) A
metamorphic property: for generated `x`, `x < e.tick(p) ⇒ x ≤ e`. Run both against the
impl (and the oracle).
**Acceptance.** Both properties green over the decoupled generators (`PROG-1`).

### PAP-2 — Anonymous-stamp (`id = 0`) event precondition is silent, not enforced/tested
**Priority:** P2. **Effort:** small.
**Problem.** The paper (L199, L404) forbids `event` on an anonymous stamp. The impl makes
`tick` on an empty-region id a **silent no-op** (never inflates), rather than rejecting.
At the `Clock` level this is near-unreachable, but `Version::tick(&Party)` is public and
*can* be handed an empty-region party (e.g. one extracted as a `(0, …)` sub-tree). The
behavior there is untested and unspecified.

Human editorial comment: Can it? I thought it was impossible to create an empty-region Party.
If this is not so, it should be made so.

**Evidence.** `src/oracle.rs` (`fill`/`grow` empty-id arms), `src/version/event/grow.rs`
`debug_assert!(id_val)`; no test feeds an empty-region party to `tick`.
**Fix.** Decide and document the contract (no-op vs debug-assert vs typed rejection); add a
test pinning it: "tick with an empty-region party leaves the version unchanged and never
inflates."
**Acceptance.** A test exercises an empty-region-party `tick` and asserts the documented
behavior.

### PAP-3 / PAP-4 — Minor / aspirational
- **PAP-3 [P3]:** The §5.1 worked example tests assert qualitative outcomes and the final
  collapse, but not the paper's concrete intermediate tree values (the paper renders them
  only as figures, so there is nothing literal to match). Low value; note and move on.
  `src/clock/tests.rs:486-556`, `src/oracle/tests.rs` (`o15_*`).
- **PAP-4 [P3]:** The paper's §6 practical thesis — stamp size stabilizes under churn — is
  unverified. `bench-baselines/` exists. Consider a regression-style assertion ("after N
  fork/event/join churn iterations at fixed population, encoded size stays under a bound").
  Confidence-building, not correctness.

---

## P2 — Forward-looking adversarial program (raise confidence beyond the plan)

These are *new* techniques the original implementation plan never specified. `PROG-1` and `PROG-2` are the
highest-leverage and directly attack the structural blind spot.

### PROG-1 — Decouple input generation: arbitrary normal-form generators feeding every op
**Priority:** P1 (subsumes `ADV-1`; pins `BUG-1`'s class and `PAP-1`). **Effort:** medium.
**What.** proptest strategies that build *arbitrary* id trees and event trees directly
(random recursive shape; random bases, **including values near `u64::MAX`**), pushed
through the oracle's normalizing constructors so they are valid normal form, then fed to
**every** operation and diffed against the oracle:
- `causal_cmp` on arbitrary *unrelated* pairs (today's pairs are always causally related);
- `tick`/`fill`/`grow` on arbitrary `(id, event)` pairs with unrelated shapes — where the
  `Kind` arm selection, cost folding, and tie-breaks live;
- `split`/`sum`/`compare`/`is_disjoint` on arbitrary id pairs;
- `unpack`/`repack` and `decode∘encode` on arbitrary valid trees.
The bridge (`from_oracle_*`/`to_oracle_*`) already exists in `src/test_support.rs`.
**Why.** This is the single change that most broadens coverage and is the natural home for
the `BUG-1` regression (large-base inputs) and the `PAP-1` minimality checks.
**Acceptance.** New generators in `test_support.rs`; differential tests for each op family
green at high case counts; large-base variants exercise the `BUG-1` fix.

### PROG-2 — A second, semantically independent oracle (function-space sampling)
**Priority:** P2. **Effort:** medium/large.
**What.** Implement a reference straight from the paper §4: represent the event component
as a step function evaluated at a dense grid of rationals in `[0,1)`, and the id as a set
of owned intervals. Then `leq` = pointwise ≤ over samples, `join` = pointwise max,
`event` = inflate the smallest owned dyadic interval. Differential-test the tree impl
against it.
**Why.** It shares no code or representation with the tree recursion, so it catches bugs
the tree-oracle and impl get wrong *together* — precisely the failure mode that hid
`BUG-1`. Principled: it is the paper's own semantics.
**Acceptance.** A sampling oracle module (test-only); agreement with the impl over the
op-trace and decoupled generators.

### PROG-3 — Exhaustive small-scope checking
**Priority:** P2. **Effort:** small/medium.
**What.** Enumerate *all* normal-form id trees up to some threshold of depth and all event trees with
bases in `{0,1,2}` up to the same; for every tree and every pair, run every op and compare
to the oracle. The depth threshold could be pretty high -- remember that it's not *that* expensive
to check a fast property for every single 32-bit number, all things considered (though it might take a while).
**Why.** The small-scope hypothesis fits ITC: small trees are few, and exhaustive
enumeration catches edge cases (a `grow` tie at the root, an empty-child spine corner, the
`close_node` adjacency boundary) that random sampling under-hits.
**Acceptance.** An ignored-by-default (or fast-bounded) exhaustive test that passes.

### PROG-4 — `grow`-optimality brute-force oracle
Same as `PAP-1(a)`; listed here as the testing-program facet. Build once, reuse in
`PROG-1`/`PROG-3`.

### PROG-5 — Hostile decode fuzzing
**Priority:** P2. **Effort:** small (mutation tests) / medium (cargo-fuzz).
**What.** (a) A `cargo-fuzz`/libFuzzer target on `Party/Version/Clock::decode` (and a
second that decodes then runs the full op set). (b) Mutation tests: take a valid encoding
and flip each bit / truncate at each position / perturb each padding bit, asserting
`decode` either rejects or accepts-canonically. (c) Assert the keystone invariant
explicitly: for any bytes, `decode → Ok(x) ⟹ is_normal(x)` (via oracle lowering), not just
re-encode stability.
**Why.** Mutating valid streams probes validator edges far better than the current 256
uniform-random vectors; the `is_normal` assertion is what makes `Eq`/`Hash` byte-equality
sound.
**Acceptance.** Fuzz target builds and runs clean for a fixed budget; mutation tests green;
the `is_normal`-on-accept property added to `h34`.

### PROG-6 — Oracle-independent algebraic laws checked directly on the impl
**Priority:** P3. **Effort:** small.
**What.** Run `a|a==a`, commutativity/associativity of `|`, `a ≤ a|b`, antisymmetry,
transitivity, fork∘join round-trip, `decode∘encode==id`, split⊕sum disjointness — **on
the impl itself**, not via the oracle.
**Why.** These hold by the math regardless of the reference, so they catch bugs the oracle
shares.

### PROG-7 — Tooling & hygiene
**Priority:** P2/P3. **Effort:** small each.
- `cargo-llvm-cov` to confirm the `grow` probe/emit arms (`COST_MAX`, `FullEvNode`, both
  `left_chosen` branches) are actually reached; an uncovered branch tells you exactly which
  input shape the harness fails to generate.
- (Linearity compile-fail tests = `COV-5`.)

---

## Suggested session sequencing

Grouped so each session is a coherent, gate-passing unit (fmt → clippy → `nextest
--release` before each commit). Dependency-ordered.

1. **Session A — Fix `BUG-1` + regression.** Saturating adds at the four sites + the
   `encode_int` edge; add the witness unit test. Smallest diff, highest stakes. (Unblocks
   the large-base variants in later sessions.)
2. **Session B — Decoupled generators (`PROG-1`).** Build the arbitrary-normal-form
   strategies and the per-op differentials, *including* large-base inputs that exercise
   the `BUG-1` fix and the `grow`-minimality checks (`PAP-1`/`PROG-4`).
3. **Session C — Coverage quick wins.** `COV-1` (case count), `COV-3` ((0,0) reject),
   `COV-4` (deep ops), `COV-2` (serde binary formats), `PAP-2` (anonymous precondition).
4. **Session D — Robustness tooling.** `PROG-5` (fuzz + mutation + `is_normal`-on-accept),
   `COV-5` (trybuild linearity), `PROG-7` (Miri, llvm-cov, nightly soak).
5. **Session E — Deeper confidence (optional).** `PROG-2` (function-space oracle),
   `PROG-3` (exhaustive small-scope), `PROG-6` (impl-direct laws), `PAP-4` (size-growth
   regression).

P0 (`BUG-1`) should land first regardless. Sessions C–E are independent of each other and
can be reordered to taste.

---

## Appendix — finding → source index

| ID | Severity | Theme | Key location |
|----|----------|-------|--------------|
| BUG-1 | P0 | overflow / causality inversion | `version/compare.rs:166-167`, `version/event.rs:231-235,342`, `codec.rs:29` |
| COV-1 | P1 | harness case count | `clock/tests.rs:41`, `test_support.rs:38-51` |
| COV-2 | P1 | serde binary path | `serde_impls.rs`, `clock/tests.rs:679` |
| COV-3 | P2 | decode reject `(0,0)` | `codec/tests.rs:107` |
| COV-4 | P2 | deep-op coverage | `clock/tests.rs:385` |
| COV-5 | P2 | linearity compile-fail | (none — add `trybuild`) |
| COV-6/7 | P3 | Clock injectivity / fuzz net | `codec/tests.rs:83`, `clock/tests.rs:423` |
| PAP-1 | P1 | event minimality / grow optimality | `version/tests.rs`, paper L94-99,443-462 |
| PAP-2 | P2 | anonymous-stamp precondition | `oracle.rs`, `version/event/grow.rs` |
| PAP-3/4 | P3 | worked-example values / size growth | `clock/tests.rs:486`, `bench-baselines/` |
| PROG-1..7 | P1-P3 | adversarial program | new test infrastructure |
