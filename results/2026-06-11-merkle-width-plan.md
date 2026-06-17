# Plan: truncate the Merkle hash to 16 bytes (content hash stays 32)

*2026-06-11. Companion to `results/2026-06-11-hash-width-transcript.md`, which
holds the security analysis. This document is the execution plan, grounded in
a code audit of the current tree.*

> **Executed 2026-06-11** on branch `merkle-width-16` (worktree
> `.claude/worktrees/merkle-width-16`), two commits: `d4eeb8f` (the width
> split, compiler chase, snapshot re-accepts) and `7d84d9d` (docs pass and
> the two convention-pinning tests). `just gate` and `just all` clean.
> Deviations from the plan as written: no `PROTOCOL_VERSION` bump (no
> deployed users — version 1 means the new format); `Snapshot::hash()`
> turned out to be public API and narrows to `[u8; MERKLE_HASH_LEN]`, with
> the constant newly exported at the crate root; the full-width primitive
> became `ContentHash` with `truncate()` as the only bridge between the
> widths.

## Decision (recap)

- **Merkle hash** (`tree::typed::Hash`, subtree comparison in the mirror
  protocol): **32 → 16 bytes.** It is a comparison signal, never identity; a
  false-equal is a transient, self-healing liveness wound. Collision floor
  drops to 2⁶⁴, but the only adversary who could exploit it (a member) gets
  the same effect for free, and a non-member cannot grind branch preimages
  offline.
- **Content hash** (the `Key`/`Path`, leaf identity): **stays 32 bytes.** It
  carries *all* content integrity (the leaf Merkle hash is a constant), a
  collision is permanent silent split-brain, and the grind against it is
  offline. Attacker-influenced value bytes cannot be ruled out a priori.
- **Tree height: unchanged** (depth 32, path = full content hash). Examined
  and rejected: shortening the tree requires shortening the identity, which
  reopens the 2⁶⁴-offline permanent-split-brain footgun to save ~16 B per
  shipped leaf — under 1% of session traffic after the Merkle win is banked.

## Verified code facts the plan rests on

- `Hash([u8; 32])` lives in `src/tree/typed/hash.rs`; crate-internal
  (`mod tree` is private; only `Key` is re-exported from `lib.rs`). **No
  public API change anywhere in this plan.**
- `Key` (`src/tree/key.rs`) and `Path` (`src/tree/typed/path.rs`) already
  store `[u8; 32]` independently of `Hash` — the type split mostly exists.
- **The one trap:** `Path::for_leaf` builds the 32-byte path *using*
  `Hash::of` / `Hasher` — the depth-1 Merkle `H(H(version) ‖ H(value))`.
  Both the inner hashes and the outer finalize must remain full-width: a
  16-byte inner `H(value)` would bottleneck path collision resistance at
  2⁶⁴ even with a 32-byte output, recreating exactly the failure the
  32-byte content hash exists to prevent. `Hasher` has no other user
  (`path.rs`, plus one test in `src/tree/tests.rs` that re-derives a path).
- Branch preimages: `CHILD_RECORD_LEN = 1 + 32` in `hash.rs`; the
  one-shot-buffer rationale (SIMD multi-block) is width-independent.
- Wire: `Hash` serializes as raw bytes (borsh, no length prefix), so the
  `uncertain` channels (`Initiate`, `Opening`, `Exchange`) shrink
  automatically once the type does. The saturated `Opening` goes
  256 × 33 ≈ 8.4 KB → 256 × 17 ≈ 4.4 KB.
- Version gate: `PROTOCOL_VERSION: u16 = 1` in `src/peer/gossip.rs:26`;
  the preamble (`src/tree/traverse/mirror/remote/preamble.rs`) rejects a
  mismatch before any framed traffic is trusted. **No bump needed:** there
  are no deployed users yet, so no old/new pair ever meets; version 1
  simply means the new format. (Post-release, an incompatible wire change
  like this one would require a bump.)
- Snapshots pinning the wire format: `src/tree/traverse/mirror/snapshots/`
  (insta) and `tests/gossip_snapshot.rs`. Re-accepting them is the
  sanctioned procedure for a deliberate protocol change (project CLAUDE.md).

## Steps (dependency order)

1. **Split the hash primitives** in `src/tree/typed/hash.rs`:
   - `Hash` becomes `[u8; 16]`. Add a named width constant (e.g.
     `MERKLE_HASH_LEN: usize = 16`); derive `CHILD_RECORD_LEN = 1 +
     MERKLE_HASH_LEN`. `Hash::of` truncates the BLAKE3 output to 16 bytes
     (prefix truncation of an XOF is the sanctioned construction;
     security = min(w/2, 128)). `leaf()`, `branch()`, `empty_root()` are
     width-agnostic and follow.
   - Keep `Hasher` (and a full-width one-shot, e.g. `digest(&[u8]) ->
     [u8; 32]` or a `ContentHash` newtype) as the **full-width** content
     path: `for_leaf`'s inner `H(version)`, `H(value)` and outer finalize
     all stay 32 bytes. Document *why* in a comment at `for_leaf`: the
     path's collision resistance is the min over every component hash.
   - `From<Hash> for [u8; 32]` and friends become `[u8; 16]`; the compiler
     finds every coupling (the `hasher.finalize().into()` in `for_leaf`
     fails to compile until step 2 routes it to the full-width primitive —
     this is the safety net working, not an obstacle).
2. **Chase the compiler** through the internal users: `typed/node.rs`
   (memoized `hash()`/`root_hash`), `typed/untyped.rs`, `mirror/local.rs` +
   `local/partition.rs`, `mirror/message.rs`, `mirror/wire_snapshot.rs`,
   `src/tree/tests.rs`, `typed/hash/tests.rs`. Mostly mechanical; `Hash` is
   opaque almost everywhere. Update the hand-rolled child-record arithmetic
   in `benches/branch_hash.rs` to the constant.
3. **Re-accept wire snapshots** (insta + `tests/gossip_snapshot.rs`).
   Review the diffs as a checklist: expect *only* 33→17 child records and
   32→16 hash atoms; any other byte movement is a finding. Keep all
   `proptest-regressions/**` seeds — they are inputs, not outputs, and
   survive the width change.
4. **Docs pass** (the numbers that now lie):
   - `hash.rs` module/type docs: the two widths and the asymmetry argument
     (comparison signal vs. identity; self-healing vs. permanent), with the
     claim's provenance marked (derived; assumptions: prefix-paired
     comparisons, member-trust model).
   - `mirror/message.rs` wire-format header: "Hash: 32 raw bytes" → 16.
   - `lib.rs` bandwidth caveat (lines ~48–55): "10–25 KB of distinguishing
     hash traffic" and the "~20 KB" crossover roughly halve (~5–13 KB,
     crossover ~10 KB). Mark these as derived from the pre-change measured
     figures until re-measured.
   - `tree.rs` shape section if it states hash width.
5. **Add the negative test** the change makes meaningful: a unit test
   pinning `Hash` width and the new `leaf()`/`empty_root()` constants (so a
   future accidental width change trips loudly), and a doctest/test
   asserting `Path::for_leaf` output is still 32 bytes.
6. **Gate**: `just gate`, then `just all` (feature matrix, wasm, bench
   builds, fuzz targets). Re-run `cargo bench --bench gossip_grid` at
   leisure — the cost-model constants (`b_d`, `c_lat`) in
   `results/ANALYSIS.md` shift a little (17-byte child records); the model
   shapes don't. Note it in ANALYSIS.md rather than re-fitting immediately.

## Residual risks / open items

- **Audit assumption "path-equality ⇒ content-equality" consumers** above
  `join` and mirror placement (persistence layers, application code over
  `Key`). Unchanged by this plan (content hash stays 32) but worth the
  sweep while in here — it is the invariant the whole split rests on.
- The ~2⁻⁷⁵ fleet-lifetime accidental figure uses a derived per-session
  comparison count (`results/ANALYSIS.md` §2b), not an instrumented one.
  If the figure gets enshrined in rustdoc, label it derived.
- `before`/`rumormill` were checked: no dependency on the tree `Hash`.
