# before: low-hanging-fruit sweep (2026-07-18)

Companion to `design/streaming-latency-serialization.md` §10.1 (lever D).
Method: AND-filtered session profiles (`v2_i5000_d0_fixed`, `v1_i5000_d0`,
sampling floor ≈ 0.3 % of a session ≈ 0.1 ms) plus an exhaustive file-by-file
read of the hot surface (`version/compare.rs`, `version/event/*`,
`version.rs`, `version/batch.rs`, `version/working.rs`, `codec/{gamma,base,
bits,tree,cursor}.rs`, `idbits.rs`, `borsh_impls.rs`) and a pattern skim of
the cold surface (`party.rs`, `clock.rs`, `codec/text.rs`, tick/rank paths).

## Headline

After D1–D3, **no ms-scale in-session fruit remains in `before` at
I = 5000** [checked]: compare walks are allocation-free in practice
(session ∩ `version::compare` ∩ malloc = 0 samples), `Base` Small-path
arithmetic never allocates and every Big spill is lazy
(`unwrap_or_else`), the only eager error values on hot paths are D1's two
sites, and `Batch`'s unpack-once/repack-once amortization measures 0.5 %
of a session. The finds below are API-quality wins and riders, not new
levers.

## Finds (impact-per-risk order)

1. **`Version` equality walks to prove *inequality*** —
   `version.rs` `causal_cmp_impls!` (eq cells). `causal_cmp`
   short-circuits `Equal` by memcmp (`trivially_eq`) but answers "not
   equal" only after the full `O(n+m)` gamma-decoding walk — yet for
   same-form operands, canonical normal form means byte-inequality
   already *is* the answer. Session-warm only in V1 (two `==` per
   exchange, `alternating/backend/local.rs:242,280`) and the once-per-
   session streaming version-equal check: sub-µs each [derived]. Worth
   landing for the public API (Versions as map keys, dedup, `Eq`-heavy
   consumers). Fix: eq cells compare `as_bits()` when both views are
   packed (and `topo`/`base` when both working); mixed forms keep the
   walk. Behavior unchanged — byte-equality ⟺ equality is already the
   crate's documented invariant. Risk ≈ 0.
2. **`Version::is_empty` allocates and walks** — `version.rs:91`:
   `*self == Version::new()` costs a `BitVec` malloc + encode +
   `causal_cmp` + drop for an O(1) question (the canonical empty version
   is exactly the 2-bit leaf `[0, 1]`). Cold in rumors' session path;
   public API. Fix: test `len == 2 && bits == [0,1]` inline, with a
   debug_assert against `Version::new()`. Risk ≈ 0.
3. **Per-call parse stacks** — `codec/tree.rs:37,107`: `parse_id_from`/
   `parse_ev_from` allocate a fresh `Vec<Frame>` per call; one call per
   wire-decoded `Version` (~10 k/session). Measured jointly with D3's
   buffer growth at ~7 ms of 3 276 session-ms (≈ 0.07 ms/session)
   [checked] — under the sampling floor individually. Fold into the D3
   branch (a `smallvec` with ~16 inline frames covers real event-tree
   depths) rather than landing standalone.
4. **No lattice-identity short-circuit in `join_view`/`meet_view`** —
   `version/batch.rs:82–107`: joining into an empty current (the
   ceiling-memo seed pattern, rumors `untyped.rs:319`) runs a full
   combine + repack where `0 | v = v` is a bits copy; dually
   `0 & v = 0` could return immediately. Bounded by the total
   in-session combine+repack footprint of ~0.6 % [checked], so
   ≤ ~0.2 ms/session; cheap to add (an O(1) is-empty test on either
   view) and it also helps non-gossip fold workloads
   (`join_all`/`Sum` seed with `Version::new()`).
5. **`encode_int` emits bit-by-bit** — `codec/gamma.rs:33–41`: `k`
   zero-pushes plus `k+1` mantissa pushes per integer on every repack.
   Repack is invisible in-session (0 malloc samples; `Batch` family
   0.5 %). A word-wise emit (build the `2k+1`-bit code in a `u64`,
   `store_be` once) is the natural *encode-side rider on D2*; not
   standalone.

## Swept and found CLEAN (don't re-audit)

- Eager error/default arguments: only D1's two sites
  (`cursor.rs:30`, `gamma.rs:60`); `map_err(Decode::Io)` sites are
  once-per-decode; `text.rs` is cold and fieldless.
- Capacity-observing channel/alloc ops, `try_*` probes: none.
- `Base` arithmetic (`codec/base.rs`): Small paths allocation-free;
  every `BigUint` fallback behind `unwrap_or_else`/match; `Ord` is
  discriminant-first. Working-form reads clone `Base` per node —
  an enum copy unless Big (rare by normal form).
- `BitWriter`/`pack_to_writer`: chunked, whole-byte fast path when
  aligned; once per encode.
- `trivially_eq`: exists, runs first in `causal_cmp`, and bitvec's
  specialized slice-eq (`sp_eq`) does word compares.
- Compare/combine walks: no per-node allocation on the Small path
  [checked: 0 malloc samples under `version::compare` in-session];
  offsets threaded by reference; `EvNode` by value.
- `WorkingVersion::unpack`/`repack` push-growth: deliberate
  (allocator size-class recycling, commented) and measured cheap.
- `Builder` (combine output): single pre-sized allocation via
  `node_capacity_bound`.
- `idbits::skip_subtree` / `skip_int`: counter loop, no decode, no
  stack.
- Recursion guards (`descend!`) present on every depth recursion in
  the swept files; iterative exceptions documented.
- Tick/rank/project paths (`fill.rs`, `grow.rs`, `project.rs`,
  `rank.rs`): session-cold; clones there are `Base` enum copies or
  deliberate (`dangerously_alias`); `rank.rs` `num.clone() << k` is
  BigUint-shift semantics, cold.

## Suggested disposition

Land finds 1–2 as one small `before` PR (API-quality; zero risk);
attach 3 to the D3 branch and 5 to the D2 branch; take 4
opportunistically. None of these move the gossip-session needle at
I = 5000 — that story remains D1–D3 (§10.1).
