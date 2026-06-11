# The asymptotic complexity of the mirror reconciliation protocol

A first-principles derivation of the per-side computation, bandwidth, and
latency of `rumors`' mirror protocol, written for a reader who knows basic
probability and big-O but has never seen this codebase. Every modeling
assumption is pinned to a code location; every approximation is named where
it is used. All file references are relative to the repository root (this
worktree); line numbers are as of commit `a8a3b7a`.

**Headline results** (derived in §6, constants from the code):

| Axis | Result (expected, uniform leaves) | Empirical fit it refines |
|---|---|---|
| Per-side computation | `Θ( min(256·D, N) · (1 + log₂₅₆(N/D)) )` child-hash comparisons; equivalently `Θ(D·(1 + log(N/D)))` disputed nodes visited, ×256 sibling fan-out each | `√(C·D)`: a true upper bound (§7), loose by up to `√(C/D)/ln(C/D)` |
| Bandwidth | `Θ(D)` message bodies + the same `min(256·D, N)·(1 + log₂₅₆(N/D))` hash entries (~36–64 B each) + ≤ 36 frames of framing | `~D`: correct up to the slowly-varying log factor |
| Latency | `½·log₂₅₆(2·D·N) + O(1)` round trips, hard ceiling ≈ 18 RTT (36 frames), floor 1 RTT after the preamble for identical versions | `log₂₅₆(C+D)`: same at `D ≈ N`, ~2× high for `D ≪ N` (both tiny) |

where `N = C + D`. One meta-finding up front, developed in §7: at every
realistic scale (`N ≤ 10⁷`), the *exact* level-by-level sum tracks `√(C·D)`
to within a factor of ~2–4 across almost the whole `(C, D)` plane. The
smooth `D·log(N/D)` form is the asymptote; the discrete base-256 staircase
underneath it locally mimics a square root. The empirical fit was not an
accident, and wall-clock benchmarks at these scales genuinely cannot refute
it — §8 gives measurements that can.

> **Source-of-data flag.** `benches/hash_recon.rs` is *not* a reconciliation
> benchmark: it measures worst-case root-hash *reconstruction* (32 iterated
> 8 KiB BLAKE3 wraps vs. 34-byte commitments), motivating the per-node hash
> memo. The reconciliation measurements live in `benches/gossip_grid.rs` and
> `benches/in_memory.rs` over the divergence grid in
> `benches/support/grid.rs`. Everything below assumes the `√(C·D)` fit came
> from that grid.

---

## 1. The system, in one page

Each replica holds a **sparse Merkle radix trie**: branching factor 256,
fixed depth 32, one path byte per level (`src/tree.rs:18-26`,
`src/tree/typed/height.rs:110-117`). A leaf's 32-byte path is
`blake3( blake3(version) ‖ blake3(value) )` — the *version is folded into
the path* (`src/tree/typed/path.rs:32-44`), so every insert, even of
identical content, lands at a fresh, computationally-uniform position.
Single-child runs are path-compressed into a prefix byte string on the node
(`src/tree/typed/untyped.rs:25-47`, invariant "branches have ≥ 2 children"
at `untyped.rs:78-81, 128-150`). Each node lazily memoizes its subtree hash
(`untyped.rs:266-288`) and its version ceiling/floor
(`untyped.rs:298-360`); the memos survive copy-on-write clones
(`untyped.rs:100-123`). Hashing is compression-invariant: a compressed
prefix byte wraps the child hash exactly as a materialized one-child branch
would (`src/tree/typed/hash.rs:54-79`, `untyped.rs:268-287`). Leaves hash
to a constant (`hash.rs:45-51`) — all leaf-distinguishing content lives in
the *path*.

The **mirror protocol** reconciles two replicas' trees over a wire. After a
25-byte raw preamble (`src/tree/traverse/mirror/remote.rs:11-17`) and a
framed version handshake, equal versions terminate immediately
(`src/tree/traverse/mirror/local.rs:230-241, 268-281`;
`remote.rs:279-285`). Otherwise both sides run a synchronized descent. Each
side keeps a *zipper* of per-height levels (`src/tree/typed/levels.rs`);
each message a side sends descends its own zipper by **two heights**
(`local.rs:395-415`; `partition.rs:354-356`), and because the two sides
alternate strictly, the conversation's frontier advances **one depth per
message, two per round trip**.

Each steady-state message carries three channels
(`src/tree/traverse/mirror/message.rs:50-70`, asymmetry matrix at
`local.rs:46-58`):

| Channel | Content | Receiver's action |
|---|---|---|
| `uncertain` | `(prefix, hash)` of the sender's disputed-frontier subtrees | merge-join against own children: equal hash ⇒ drop; differ ⇒ explode grandchildren (recurse); we-lack ⇒ `request`; they-lack ⇒ `provide` |
| `requested` | prefixes the sender lacks entirely | answer next message by exploding that node's children into `providing` (`partition.rs:91-142`) |
| `providing` | whole subtrees, full structure on the wire (`message.rs:50-58`) | insert at the named prefix (`partition.rs:57-85`) |

All per-round set operations are single linear merges over sorted flat
vectors (`partition.rs:100-115, 199-203`;
`src/tree/typed/levels/level.rs`), so per-round work is linear in the
frontier and message sizes — no hidden log factors beyond the `imbl`
`OrdMap` child maps (an `O(log 256)` constant per child operation, absorbed
below).

**Termination is in-band**: a side is done when its outgoing `requested`
and `uncertain` are both empty (`partition.rs:393-410`;
`remote.rs:34-43, 364-374, 398-409`); a sender that just emitted an
empty-empty message does not even await a reply (`remote.rs:398-409`). The
schedule has a hard ceiling — `mirror_connected`
(`src/tree/traverse/mirror.rs:126-137`) is `initiator` + `responder` +
`open_initiator` + a `seq!` loop of **14 exchange pairs** +
`close_initiator` + `complete_responder` + `complete_initiator`. Counting
frames: 2 handshake + `Initiate`(depth 0) + `Opening`(depth 1) +
`Exchange`(depth 2) + 28 exchanges (depths 3–30) + one more responder
exchange (depth 31) + `Closing` + `Complete` = **36 frames maximum**,
covering all 32 byte-levels (the chain lengths are pinned at the type level
in `src/tree/traverse/mirror/protocol.rs:477-488`).

**Deletions** leave no tombstones. Every subtree shipped through
`providing` is first filtered by `Unknown::unknown`
(`src/tree/traverse/unknown.rs`): a subtree whose version *ceiling* is `≤`
the counterparty's version vector is something they have already seen — its
absence on their side means they deleted it — so it is dropped in `O(1)`
(one memoized-ceiling comparison, `unknown.rs:28-34`), *and the local copy
is dropped too* (the Left arm keeps a node only if it survives the filter,
`partition.rs:232-239`). A floor check keeps wholly-unknown subtrees intact
without rebuilding them (`unknown.rs:38-52`).

---

## 2. Model and assumptions

- **(A1) Uniform leaf placement.** Leaf paths are BLAKE3 outputs over
  `(version, value)` (`path.rs:32-44`), and versions are unique per action
  (`src/tree.rs:299-318`). In the random-oracle model the `N` leaf paths
  are i.i.d. uniform 32-byte strings, *regardless of insertion order,
  batching, or payload skew* — an adversary would need structured BLAKE3
  preimages to cluster them. This is the load-bearing probabilistic
  assumption; see §9.4.
- **(A2) Parameters.** Branching `b = 256`, depth 32 (`tree.rs:21`,
  `height.rs:110-117`). Hash size `h = 32` bytes (`hash.rs:17`).
- **(A3) Warm memos.** Subtree hashes and version bounds are memoized
  reads (`untyped.rs:266-288`); we count a hash *comparison* as `O(1)`.
  First-touch costs after local mutation are a separate, additive term
  (§9.1) — the benches warm them in untimed setup
  (`benches/gossip_grid.rs:119-160`, `src/tree.rs:182-196`).
- **(A4) Dispute drives descent.** Only the *Both-and-hashes-differ* cell
  of the asymmetry matrix recurses (`partition.rs:243-254`). The Left cell
  ships a whole subtree in the same message; the Right cell is requested
  and answered whole one message later (`partition.rs:91-142`). So
  one-sided differences cost `O(1)` additional messages once they separate
  from two-sided content.
- **(A5) Two heights per send, one depth per message.** Verified at
  `local.rs:395-415` and the alternation in `mirror.rs:126-137`.
- **(A6) No level skipping.** Path compression does **not** shorten the
  descent: exploding a compressed node pops exactly one prefix byte and
  yields a singleton child map (`untyped.rs:156-167`). Compression saves
  hashing and memory, never rounds.
- **(A7) No hash collisions.** A false `uncertain` match (probability
  `≤ pairs·2⁻²⁵⁶`) would silently prune; ignored.
- **(A8) Wire sizes.** An `uncertain` entry at depth `k` is `k` raw prefix
  bytes + 32 hash bytes (`prefix.rs:127-156`, `message.rs:61-63`); a
  `providing` node is its full borsh structure: per leaf, a version, the
  payload, and ≤ 32 compressed-prefix bytes (`message.rs:21-58`,
  `untyped.rs:401-438`). Frames carry a 4-byte length prefix
  (`remote.rs:26-33`).

## 3. Notation

| Symbol | Meaning |
|---|---|
| `b` | branching factor, 256 |
| `C` | live messages present on **both** sides |
| `D` | messages present on **exactly one** side, summed over both sides (`D = D_L + D_R`); redacted-on-one-side leaves count (§9.3) |
| `N` | `C + D`, the size of the union tree |
| `k` | depth = prefix length in bytes, `0 ≤ k ≤ 32`; height `H = 32 − k` |
| `Δ_k` | number of **disputed** depth-`k` prefixes: prefixes containing ≥ 1 differing leaf *and* ≥ 1 leaf on the counterparty's side |
| `f_k` | expected occupied-children count of a depth-`k` node, `≈ min(b, max(1, N/b^k))` |
| `j_D`, `j_N` | `log_b D`, `log_b N` (depths where `b^k` crosses `D`, `N`) |
| `k*` | separation depth: deepest disputed prefix |
| `W` | per-side work / total `uncertain` entries: `Σ_k Δ_k·f_k` |
| `s̄`, `s_v` | mean payload size; serialized `Version` size (not `O(1)`, §9.5) |

Logs are base `b = 256` unless written `ln`. One decimal decade is
`log₂₅₆ 10 ≈ 0.415` levels; one level spans 2.4 decades. Keep that coarse
quantization in mind — it is the source of the staircase in §7.

## 4. Warm-up: `D = 1`, `N = 10⁶`

One side holds a single extra leaf; the trees otherwise agree on `10⁶`
messages. Trace the protocol:

1. **Handshake** (2 frames): versions differ, descend.
2. **`Initiate`** (depth 0): one root hash. Differs.
3. **`Opening`** (depth 1): the responder *unconditionally* explodes its
   root and lists all its children (`local.rs:333-353`) — at `N = 10⁶`,
   all 256 slots are occupied (≈ 3,906 leaves each): **256 entries**.
4. The initiator merge-joins: 255 child hashes match and drop; the one
   slot containing the extra leaf differs (Both-case), so its 256
   grandchildren are exploded and listed: **256 entries** at depth 2.
5. The responder finds one depth-2 hash differing; that node has
   `≈ 10⁶/256² ≈ 15` occupied children: **≈ 15 entries** at depth 3.
6. At depth 3 the extra leaf sits alone in its slot with probability
   `≈ 1 − N/256³ ≈ 0.94`: a **Left case**. The whole leaf (a
   path-compressed spine node carrying its remaining ≈ 29 path bytes,
   version, and payload) ships in `providing`; nothing is exploded;
   the outgoing `uncertain` and `requested` are empty ⇒ **Done**, in-band.

Totals: ≈ `256 + 256 + 15 ≈ 527` hash entries on the wire and compared
(~18 KB), 7–8 frames ≈ 4 round trips including the preamble, one message
body. Three things to notice, which the general analysis formalizes:

- The *visited disputed nodes* number only ≈ 3 (one per level until
  separation) — `D·(1 + log_b(N/D))` with `D = 1`.
- But each visited node costs its **full sibling fan** (≤ 256 entries):
  the per-node constant is `f_k ≈ min(b, N/b^k)`. This factor-256 is the
  price of the radix-256 tradeoff: few rounds, fat rounds.
- Separation happened at depth `≈ log_b(D·N) = log_b 10⁶ ≈ 2.5` — not
  `log_b N` levels of *messages saved*, because termination is the in-band
  emptiness predicate, not bottoming out at depth 32.

## 5. Two lemmas

**Lemma 1 (prefix occupancy / sharing).** Throw `m` uniform leaves into
the `b^k` bins at depth `k`. The expected number of occupied bins is
`b^k·(1 − (1 − b^{−k})^m)`, which is `Θ(min(b^k, m))` — at least
`(1 − 1/e)·min(b^k, m)`, at most `min(b^k, m)`.

*Proof sketch.* Linearity of expectation over bins; `(1−1/x)^x ≤ 1/e`.
Applied to the `D` differing leaves, this bounds how many distinct
ancestors they have at each depth: their root-to-leaf paths share prefixes
exactly while `b^k < D`, and are essentially disjoint below. Summing,
the union of the `D` differing root-to-separation paths has
`Σ_k min(b^k, D) = Θ(D·(1 + log_b(N/D)))` distinct nodes (the geometric
head contributes `Θ(D)`; the plateau contributes `D` per level from
`j_D` to the dispute cutoff of Lemma 2). ∎

**Lemma 2 (dispute persistence).** A depth-`k` prefix is disputed —
i.e. recursed into rather than resolved — iff it contains a differing leaf
*and* the counterparty owns at least one leaf under it (A4: otherwise it is
a Left/Right case and resolves in `O(1)` messages). Under A1,

```
E[Δ_k]  ≤  min( b^k ,  D ,  2·D·N·b^{−k} )
```

and `P(any dispute survives depth k) ≤ 2·D·N·b^{−k}`.

*Proof sketch.* The first two bounds are Lemma 1 applied to the differing
leaves. For the third: a dispute at depth `k` requires some differing leaf
`d` and some leaf `x` on the side opposite `d`'s holder to share a `k`-byte
prefix. Each such (ordered) pair shares it with probability `b^{−k}`
(uniform, independent paths), and there are at most
`D_L·(C + D_R) + D_R·(C + D_L) ≤ 2·D·N` pairs. Union bound / linearity. ∎

**Corollary (separation depth).** `k* ≤ log_b(2·D·N) + t` with probability
`≥ 1 − b^{−t}`; `E[k*] ≤ log_b(2DN) + b/(b−1)·(1/ln b) ≈ log_b(2DN) + 1`.
The fixed 32-level ceiling is unreachable probabilistically: hitting it
would need `D·N ≈ 256³⁰`. (It exists because paths are 32 bytes; even a
full-depth descent is *correct* — equal full paths imply equal
`(version, value)` by construction, `tree.rs:22-26`.)

**Lemma 3 (fan-out).** A depth-`k` node in the union tree of `N` uniform
leaves has expected occupied-children count `f_k = Θ(min(b, max(1,
N/b^k)))`, and the *total* children over any set of `Δ` depth-`k` nodes is
`≤ Δ·b` always and `Θ(Δ·f_k)` in expectation (children counts across
disjoint subtrees are negatively associated; we only ever need linearity).
*Caution (Jensen):* `E[min(X,Y)] ≤ min(E X, E Y)`, so products of the
`min` envelopes used below are upper envelopes of expectations; the
matching lower bounds hold within the `(1−1/e)` constants of Lemma 1.

## 6. The level-by-level sum

Per round at depth `k`, the side processing the counterparty's `uncertain`
does (all single-pass, sorted merges — `partition.rs:159-304`):

- drain its frontier once: `O(|frontier_k|)`;
- for each disputed parent, merge-join its own occupied children against
  the listed hashes: `O(own fan + listed entries)`, each comparison a
  memoized-hash `memcmp` (A3);
- explode each hash-differing child's grandchildren into the next level
  and emit them as the next `uncertain` (`partition.rs:243-254, 364-373`).

So the entries *listed at depth `k`* are exactly the occupied children of
the disputed depth-`(k−1)` parents, and each side's per-session work, the
total `uncertain` traffic, and the zipper/`collapse` churn
(`levels.rs:148-197`, linear in total level entries) are all `Θ` of the
same master sum:

```
W  =  Σ_{k=0}^{32}  Δ_k · f_k   ≈   Σ_k  min(b^k, D, 2DN·b^{−k}) · min(b, max(1, N/b^k))
```

Evaluate by regimes (write `j_D = log_b D`, `j_N = log_b N`; assume first
`D ≤ N/b²` so all three regimes are non-empty):

| Regime | Depths | Term | Contribution |
|---|---|---|---|
| **Head** (paths shared) | `k ≤ j_D` | `b^k · b` | geometric, `≈ b·D·(1 + 1/b + …) ≈ b·D` |
| **Plateau** (paths distinct, fans saturated) | `j_D < k ≤ j_N − 1` | `D · b` | `b·D·(log_b(N/D) − 1)` |
| **Tail** (fans thin out, disputes die) | `k > j_N − 1` | `D·N·b^{−k}·O(1)` | geometric, `≈ Θ(D·b)` at the shoulder, then ÷256 per level |

**Theorem 1 (per-side computation).** Under A1–A6, the expected per-side
work (hash comparisons + level-entry handling, each `O(1)`) and the
expected total `uncertain` entry count are

```
W  =  Θ( min(b·D, N) · (1 + log_b(N/D)) )
```

- for `D ≤ N/b`:  `W = Θ(b·D·(1 + log_b(N/D)))` — the prompt-level form
  `Θ(D·(1+log(N/D)))` counts *disputed nodes visited*; the protocol pays a
  `×b` sibling fan on each (Lemma 3, §4);
- for `N/b < D ≤ N` (heavily diverged / disjoint): the plateau vanishes
  and the peak term is `Θ(N)`; `W = Θ(N) = Θ(D)·O(b)` — linear;
- additive `Θ(D·(1 + s_v))` for absorbing/filtering the provided subtrees
  and the final collapse, and `Θ(1)` per round of fixed pipeline cost. ∎

Cross-checks: `D = 1, N = 10⁶` gives `W ≈ 256·(1 + log_b(N/b)) ≈ 640` vs.
the §4 hand count of 527 ✓. Disjoint trees (`C = 0, N = D`) give
`Θ(N)` ✓ (matches the one-shot character of bootstrap, §9.2).

**Theorem 2 (bandwidth).** Total bytes on the wire, both directions
(each depth's `uncertain` is listed by exactly one side — the alternation
splits levels between them):

```
bytes  =  W·(h + k̄)            [uncertain: 32-byte hash + k-byte prefix, k̄ ≤ j_N+1]
        + Θ(D)·(s̄ + s_v + ≤32) [providing: payloads, versions, compressed spines]
        + O(D)·k̄               [requested: one prefix per separated they-only subtree]
        + ≤ 36·4 + 2·25 + 2·s_v [framing, preamble, handshake]
```

In `Θ`-terms: `Θ(D·(s̄ + s_v)) + Θ(min(b·D, N)·(1 + log_b(N/D))·h)`. The
empirical "`~ D`" is right in its `D`-scaling; the per-`D` constant is the
interesting part: at `N/D = 10²…10⁴` the hash channel runs ~10–40 KB *per
differing message* (e.g. §4: 18 KB to move one message). For small
payloads, **hash traffic dominates bandwidth**; bodies dominate only when
`s̄ ≳ 36·W/D` bytes. ∎

**Theorem 3 (latency).** Messages strictly alternate and the conversation
advances one depth per message (A5). Disputes empty at depth `k*`
(Lemma 2 corollary), plus `O(1)` messages to flush the final
`requested → providing` exchange; the in-band predicate then stops the
session (A6 — no draining of the remaining schedule):

```
RTT  =  ½·k* + O(1)  =  ½·log₂₅₆(2·D·N) + c,    c ≈ 3–4
```

(preamble ≈ 1, handshake 1, `Initiate`/`Opening` 1, closing tail ~1),
hard-ceilinged at 36 frames ≈ 18 RTT, floored at ~2 RTT total for equal
versions. Path compression does not reduce this (A6). Numerically the
*descent* portion is tiny at any scale — 1.8 levels ≈ 1 RTT for
`(N, D) = (10⁶, 1)` up to 6.1 levels ≈ 3 RTT for `(10⁷, 10⁷)`; the `O(1)`
dominates. The empirical `log₂₅₆(C+D)` agrees at `D ≈ N` (where
`½ log DN = log N`) and is ≈ 2× the true descent depth for `D ≪ N`, a
difference of < 2 RTT at any realistic scale — unfalsifiable from timing,
trivially checkable by counting frames (§8). ∎

## 7. Against the `√(C·D)` fit

**Proposition (the fit is a true upper bound).** For `1 ≤ D ≤ C`:

```
D·ln(C/D)  ≤  (2/e)·√(C·D),    with equality iff D = C/e².
```

*Proof.* Set `x = D/C ∈ (0,1]` and maximize `g(x) = √x·ln(1/x)`:
`g′(x) = (ln(1/x) − 2)/(2√x) = 0` at `x = e⁻²`, `g(e⁻²) = 2/e`. Then
`D ln(C/D) = C·x ln(1/x) = C√x·g(x)·…` — directly,
`D·ln(C/D) = √(CD)·√(x)·ln(1/x) ≤ (2/e)√(CD)`. ∎

In base 256: `D·log₂₅₆(C/D) ≤ (2/(e·ln 256))·√(CD) ≈ 0.133·√(CD)`. With
Theorem 1's constants (`b/ln b ≈ 46.2`), for `D ≤ C`:

```
W  ≤  b·D·(2 + ln 2) + (2/e)·(b/ln b)·√(CD)  ≈  690·D + 34·√(C·D)  =  O(√(C·D)),
```

since `D ≤ √(CD)` when `D ≤ C`. So the empirical `√(C·D)` is a *correct*
upper bound on per-side computation — but loose: the ratio
`√(CD)/(D·log(C/D)) = √(C/D)/log(C/D)` grows without bound as `D/C → 0`,
and the bound is tight (constant-factor) only in the band
`D/C ∈ [~10⁻², ~½]` where `√x·ln(1/x)` stays within 1.5× of its max.

**Why the fit nevertheless matched the data: the staircase.** The smooth
form `b·D·(1+log_b(N/D))` is the envelope of a *staircase*: each level of
the sum spans a factor of 256 ≈ 2.4 decades. Inside one span the dominant
term is `t_k = D·N/b^k` — **linear in `N` at fixed `D`** — and the term
above it, `min(b,D)·b^{k}`-shaped, is *flat* in `D` once `D > b`. The
exact sum's local log-log slopes therefore wobble around the sqrt's 0.5
instead of sitting at the asymptotic 0 (in `N`) and 1 (in `D`):

Exact-sum values of `W` (children examined ≈ `uncertain` entries; computed
from the master sum, expectations, constants ±2× from occupancy
corrections), against `√(C·D)`:

| `N` | `D` | `W` | `√(C·D)` | `W/√(CD)` | `k*` | RTT (descent+c) |
|---|---|---|---|---|---|---|
| 10⁴ | 1 | 295 | 100 | 3.0 | 1.8 | ~3.9 |
| 10⁴ | 10² | 4.2 k | 1.0 k | 4.2 | 2.6 | ~4.3 |
| 10⁴ | 10⁴ | 26 k | 10 k | 2.6 | 3.6 | ~4.8 |
| 10⁵ | 10² | 26 k | 3.2 k | **8.2** | 3.0 | ~4.5 |
| 10⁵ | 10³ | 67 k | 10 k | 6.8 | 3.5 | ~4.7 |
| 10⁶ | 1 | 0.53 k | 1.0 k | **0.53** | 2.5 | ~4.2 |
| 10⁶ | 10² | 27 k | 10 k | 2.7 | 3.5 | ~4.7 |
| 10⁶ | 10³ | 81 k | 32 k | 2.6 | 3.9 | ~4.9 |
| 10⁶ | 10⁴ | 220 k | 100 k | 2.2 | 4.3 | ~5.1 |
| 10⁶ | 10⁵ | 1.08 M | 316 k | 3.6 | 4.7 | ~5.3 |
| 10⁷ | 1 | 0.67 k | 3.2 k | **0.21** | 2.9 | ~4.4 |
| 10⁷ | 10⁴ | 1.6 M | 316 k | 5.1 | 4.7 | ~5.3 |
| 10⁷ | 10⁵ | 10.2 M | 1.0 M | **10.2** | 5.1 | ~5.6 |
| 10⁷ | 10⁷ | 29 M | 10 M | 2.9 | 6.1 | ~6.0 |

Read the ratio column: over five orders of magnitude in each axis it stays
within `[0.2, 10]` — a one-constant sqrt fit (say anchored at the central
cells, `a ≈ 2.6`) leaves residuals of only 0.1×–4×, and the worst residuals
sit at the extreme corners (`D = 1` at huge `N`; `D ≈ b·N/b²`) where wall
time is dominated by fixed per-session costs (preamble, handshake, ~8
frames, thread/pipe synchronization in the bench harness) or by the `Θ(D)`
body-handling term. Local log-log slopes of the exact sum, vs. sqrt's
uniform 0.5:

- in `D` at fixed `N = 10⁶`: 0.75, 0.96, **0.47, 0.43**, 0.69 per decade
  (`D: 1→10→10²→10³→10⁴→10⁵`) — *in the densest benchmark range the true
  slope is ≈ 0.45, indistinguishable from 0.5*;
- in `N` at fixed `D = 10³`: 0.80, **0.08**, 0.43 per decade
  (`N: 10⁴→10⁵→10⁶→10⁷`) — the `10⁵→10⁶` decade is the smoking gun: the
  true cost is nearly flat there (it sits between staircase steps:
  `b² = 65,536`, `b³ = 16.8 M`), while sqrt demands ×3.16.

**Conclusion.** `√(C·D)` and `Θ(min(bD,N)(1+log_b(N/D)))` are
empirically indistinguishable from wall time on diagonal or `D`-sweeps at
grid scales; they separate cleanly on **fixed-`D`, `N`-sweeps across the
`10⁵→10⁶` decade**, and on any *count*-based (rather than time-based)
measurement.

## 8. An experiment that distinguishes them

All using existing infrastructure; no protocol changes.

1. **Byte counting (noise-free, decisive).** In
   `benches/gossip_grid.rs`, wrap the `PipeReader`/`PipeWriter` ends of
   `Wire` (`gossip_grid.rs:43-99`) in trivial `std::io::Read`/`Write`
   adapters that increment `AtomicU64`s, and record bytes per session per
   cell. Bandwidth is `Θ(36·W + body terms)` by Theorem 2 and is exactly
   measurable. Regress `log(bytes − a·D)` on `log D` and `log(N/D)` over
   cells with `D ≥ 100`: the sqrt model predicts elasticity ≈ 0.5 in
   `log(N/D)`; Theorem 1 predicts ≈ 0 (a log-of-log). Frame *counts* from
   the same adapter (count 4-byte headers) verify Theorem 3 against
   `½·log₂₅₆(2DN) + c` to ±1 frame.
2. **The flat decade.** Fix `differing = 1,000`, `redacted = 0`, sweep
   `common ∈ {10³, 10⁴, 10⁵}` (cells already exist; extend `COMMON` in
   `benches/support/grid.rs:74` to `10⁶` for the decisive decade,
   accepting the 10-sample floor). Predicted wall-time ratios
   `common 10⁵ → 10⁶`: sqrt ×3.16; Theorem 1 ×1.2 (the `0.08`-slope
   decade). One afternoon of Criterion time.
3. **Direct hash-compare counts.** Drive two `local::Exchange`s
   in-process through the `#[cfg(test)]` `mirror::mirror` entry
   (`mirror.rs:337-353`) with grid-shaped trees and count `uncertain`
   entries (e.g. instrument in a test, or just count from the byte totals:
   `entries ≈ (uncertain bytes)/(34…64)`). This measures `W` itself,
   bypassing the `α·D + fixed` wall-time floor entirely.

## 9. Caveats and edge regimes

1. **Lazy memoization (first session after writes).** A batch of `m`
   local changes leaves cold `OnceLock`s on the `Θ(m·(1+log_b(N/m)))`
   ancestor nodes of the changed paths (`untyped.rs:40-47`; memos survive
   `Arc`-sharing, so *only* changed paths are cold,
   `untyped.rs:100-123`). The first `hash()` then pays one branch preimage
   per cold node — up to `1 + 33·256 ≈ 8.4` KB of BLAKE3 for a saturated
   branch, but only 34 bytes per compressed-spine level
   (`untyped.rs:283-287`; both conventions measured in
   `benches/hash_recon.rs`). This cost is additive, amortizes across
   sessions, and is excluded from the benches by `warm_caches()`
   (`gossip_grid.rs:154-160`).
2. **Bootstrap / one side empty (`D = N`).** No prefix has content on
   both sides, so there are *zero* disputes: the empty responder sends an
   empty `Opening` (`local.rs:337-341`), and the initiator's root-level
   drain Left-provides every root child in one message
   (`partition.rs:276-294`), filtered through `Unknown` against the empty
   version (everything survives). Cost: `Θ(N)` transfer and absorption,
   `O(1)` rounds. Theorem 1's formula degrades gracefully to `Θ(N)` here,
   but the *mechanism* is different — the dispute machinery never engages.
3. **Deletion honoring.** A redacted leaf (present on A, deleted on B)
   keeps hashes differing along its path, so it costs descent exactly like
   a differing leaf — **count redactions in `D`** (the grid's throughput
   accounting already does: `grid.rs:115-127`). But it costs no body
   bytes: at separation, A's Left-provide is killed by the `O(1)` ceiling
   check (`unknown.rs:28-34`) and A deletes its own copy
   (`partition.rs:232-239`) — deletion *propagates* through the version
   ceiling, with no tombstone and no transfer. A whole
   missing-but-dominated subtree dies the same way: one memoized
   comparison, no descent below it.
4. **Skew.** A1 is enforced by content addressing, not hoped for: because
   the version is folded into the leaf path (`path.rs:32-44`), even a
   million-message single-party version-batch spreads uniformly —
   concentration under few prefixes would require engineered BLAKE3
   outputs. (Contrast key-ordered Merkle trees, where a contiguous batch
   *does* cluster.) Even granting an adversary clustered paths, the depth
   ceiling caps the damage at `W ≤ 32·b·D` and 18 RTT.
5. **Version sizes.** ITC `Version`s are not constant-size: they ride the
   handshake, every provided leaf, and every `ceiling/floor ≤` comparison
   (`unknown.rs:31, 46-49`), contributing the `s_v` factors in Theorems 1–2.
   For long-lived universes with many party splits, `s_v` is the term to
   watch, orthogonal to the tree combinatorics.
6. **The radix constant is the design tradeoff.** Per disputed node the
   protocol ships a full sibling fan (up to 256 × 36ish bytes ≈ 9 KB) to
   buy a 2.4-decade depth reduction per level. `b = 256` puts every
   realistic deployment within ~3 levels of separation (RTT-optimal) at
   the cost of the `×b` in `W` — which is exactly the constant that made a
   `√(C·D)` fit land in the right numeric range in the first place.
