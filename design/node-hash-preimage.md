# Single-preimage node hashing: one compression per node

Status: implemented 2026-07-18 (commit `51f6ecd1`; revised from
the two-rule spine-wrap draft after review — see §5 for the
draft's shape and why this one won). This is the design note that
`design/streaming-latency-serialization.md` §10 lever E calls
for; the measurements referenced here are that document's (§8
cells, post-lever-A profile).

## 1. The problem

The Merkle convention (`src/tree/typed/hash.rs`) has two rules: a
leaf hashes to `blake3(LEAF_TAG)`, and a branch to
`blake3(BRANCH_TAG ‖ r₀ ‖ h₀ ‖ …)` over its children in ascending
radix order. Path compression is handled by *iterating* the branch
rule: `Node::hash` (`src/tree/typed/untyped.rs`) computes the
node's base hash, then wraps it once per compressed-prefix byte
with a single-child branch hash, `Hash::branch([(byte, hash)])`.
The elegance is that a one-child branch and a compressed byte hash
identically, so hashing is compression-invariant *by construction*.

The cost is one blake3 compression per compressed byte, and the
tree's shape makes that expensive exactly where the workload is
hot:

- Leaf paths are content hashes — 32 uniformly random bytes — so
  at I = 5000 divergence the branching frontier sits near depth 2
  and every leaf node carries a ~29-byte compressed suffix. Its
  first `hash()` therefore costs ~30 compressions where the
  content demands one [derived: frontier depth from 5 000 keys
  over 256² slots; suffix = 32 − depth − 1 radix byte].
- The memo does not survive descent. `into_children` pops the
  shallowest prefix byte and *clears* the hash memo (the hash
  genuinely changes one virtual level down), so a walk that
  descends a spine and re-reads hashes refolds the remaining spine
  at every virtual level: O(s²) compressions over a fully-walked
  s-byte spine [checked: `untyped.rs` `into_children`].
- Both protocols pay it — the convention is shared — so it moves
  absolute session time, not the V2−V1 gap.

Measured footprint [checked, post-lever-A profile, V2
I = 5000, d = 0]: blake3 is 27.9 % of in-session time,
≈ 8.3 ms/session, the largest single row — larger than all
remaining wire glue combined. Lever C's `Path::for_leaf`
derivation accounts for ~1 ms of that; most of the rest is spine
wrapping.

## 2. The design

One preimage per node, covering the compressed prefix and the
children together, with every variable-width field length-tagged
[decision, 2026-07-18]:

    leaf:    blake3(LEAF_TAG   ‖ prefix_len ‖ prefix)
    branch:  blake3(BRANCH_TAG ‖ prefix_len ‖ prefix
                               ‖ child_count ‖ r₀ ‖ h₀ ‖ …)

- `prefix` is the node's compressed prefix in **path order** —
  shallowest byte first, as the node serializer emits it
  (in-memory storage is shallowest-last). `prefix_len` is one
  byte. Correction from implementation [checked, 2026-07-18]: a
  prefix spans up to **32** bytes, not 31 — a lone-leaf root
  compresses the entire path (there is no radix byte above it);
  the leaf preimage is then 34 bytes, still one block.
- Children appear in ascending radix order as fixed 17-byte
  `radix ‖ hash` records. `child_count` is a big-endian `u16`:
  the count ranges over {0} ∪ [2, 256] — zero only for the empty
  root, and never one, by the path-compression invariant (§3) —
  which overflows a biased byte.
- The tags are the existing `LEAF_TAG = 0` / `BRANCH_TAG = 1`.
  Two tags are load-bearing, not legacy: a leaf may carry an
  *empty* suffix (its parent sits at depth 31), and under a
  single tag its preimage would be byte-identical to the empty
  root's. The kind byte is what separates them.
- The empty tree remains `blake3(BRANCH_TAG ‖ 0 ‖ 0)`, a
  constant. The leaf hash is **no longer a constant**: it commits
  its own suffix. (Today's constant leaf was sound — the spine
  wraps above committed the full path — but nothing depends on
  leaf-hash constancy beyond a memoization shortcut; see §7.)

Explicit lengths make preimage injectivity *locally* checkable:
no two distinct `(kind, prefix, children)` triples encode to the
same byte string, by inspection of the fields, rather than by a
global argument about fixed record widths and trailing-hash
parses. The redundancy costs three bytes.

Cost: the preimage for a frontier leaf (1 + 1 + 29 bytes) or a
small-fan leaf-parent (1 + 1 + s + 2 + 17c, c ≤ 3) stays under
blake3's 64-byte block — **one compression per node** where today
costs 1 + s ≈ 30 [derived]. A saturated 256-child branch ingests
~4.4 KB either way; the children bytes are irreducible and its
prefix is empty in practice, so large-fan cost is unchanged.

Mid-spine (virtual-level) hashes remain well-defined: the node
one virtual level down the spine hashes with `prefix[1..]` and
its length, children unchanged. Each such hash is a fresh
single-shot preimage — including re-ingesting the children
records, since the prefix leads the preimage — which is one
compression for the small-fan nodes that actually sit under long
spines, and never worse than today's O(remaining spine) refold
[derived; see §5 for the rejected layout that could reuse a
children-side memo].

## 3. Why it stays correct

The property the per-byte fold bought — peers with equal content
under a prefix compute equal hashes there, however their nodes are
compressed — does not disappear; it moves from *by construction*
to *by canonicity*:

- The tree's shape is a function of its content. `Node::branch`
  collapses a singleton branch into its child's prefix at
  construction (`untyped.rs`), `from_sorted_run` builds maximally
  compressed subtrees, and there is no other branch constructor:
  a one-child branch node cannot exist. Equal content therefore
  yields equal shape, hence equal `(prefix, children)` fields,
  hence equal hashes.
- This invariant is **already load-bearing on the wire**: the node
  serializer discriminates leaf from branch *by shape*, untagged,
  and documents "multi-child branches always carry at least two
  children, by the path-compression invariant"
  (`untyped.rs::serialize_to`). The hash rule adds a second
  dependent of an invariant the codec already relies on, not a
  new obligation.
- Cross-peer mid-spine comparisons are the interesting case: peer
  A mid-spine at depth d versus peer B's materialized node at
  depth d. If their contents under the scope are equal, B's
  canonical shape below d equals A's virtual suffix, and the two
  preimages agree field-for-field; if contents differ, the hashes
  differ with the usual 2⁻¹²⁸ pairwise false-equal probability,
  unchanged.
- Path commitment survives the leaf rule change: along any
  root-to-leaf chain, each node commits its own prefix and each
  parent commits the child's radix, so the concatenation commits
  the full 32-byte path exactly as the wraps did.

What is *lost*: the convention no longer defines a hash for a
one-child branch at all. That is deliberate negative space — a
rule for an unrepresentable shape is documentation of the
impossible — but it means any future change that relaxes the
compression invariant breaks hashing silently. The implementation
must pin the invariant with its own proptest (see §6) so it fails
loudly instead.

## 4. What it breaks

Every hash value in every tree, in both protocols. Concretely:

- All insta snapshots that embed hashes: `gossip_snapshot` (V1
  and V2 — unlike lever A, both change), bootstrap/retire
  fixtures, the codec atlas/corpus/error-atlas rows containing
  query listings. One deliberate, isolated re-accept commit; the
  byte *layout* of every frame is unchanged — only 16-byte hash
  values differ.
- `Snapshot::hash`, the public observable. Its contract ("two
  snapshots with equal hashes represent the exact same set")
  survives; its values do not. Nothing in the crate persists
  hashes [checked: bookmark and snapshot surfaces], so there is
  no stored-state migration.
- Compatibility: **nothing has ever deployed either protocol
  version — there are no existing peers, so no version gate is
  needed** [decision, 2026-07-18]. The break is a snapshot
  re-accept and nothing more. For the record: after first
  deployment, any future hash-convention change would need a
  preamble-level gate, because cross-convention peers with equal
  content walk the full tree transferring nothing on every
  session — convergent but pathological, and silent.

## 5. Alternatives considered

- **Two-rule spine wrap** (this note's first draft): keep the
  branch rule as the node's *base*, add
  `blake3(SPINE_TAG ‖ prefix ‖ base)` for non-empty prefixes.
  One compression for the wrap, but the base is its own
  compression, so small nodes cost two where the unified preimage
  costs one — a ~2× tax on the dominant term (fresh-node memos).
  Its sole advantage: the base is a convention-level object a
  memo can retain across `into_children`'s prefix pops, making
  mid-spine hashes O(1) even for large-fan nodes. That path is
  rare (a dispute descending a dense-fan deep spine) and the
  unified rule is never worse there than the status quo. Rejected
  for the hot-path factor.
- **Children-first preimage layout** (`… ‖ children ‖ prefix`):
  would let an implementation hash the children once into a
  cloneable `blake3::Hasher` and finalize per virtual level,
  recovering the two-rule draft's mid-spine reuse without its
  extra compression. Rejected: it optimizes the rare path by
  leaking hash-function internals into the convention's layout
  and abandoning path order. Recorded here in case mid-spine
  recompute ever measures hot.
- **Cache per-virtual-level hashes on the node** (a `Vec<Hash>`
  alongside the memo). Keeps the convention and kills the O(s²)
  descent refold, but spends 16 bytes × spine length per node and
  does nothing about construction cost — the dominant term. The
  right fix for a different problem.
- **Commit spines only at branch points and let descent skip
  them** (variable-depth queries). Strictly stronger: it would
  remove the virtual levels from the protocol schedule too,
  shrinking hop counts through sparse regions. It is also a
  protocol redesign — scope naming, phase schedule, deadlock
  argument — where this change has wire-value fallout only.
  Recorded as the natural successor if spine hops ever dominate
  (see the §11 hop ledger); it composes with lever C's path
  shipping and should be designed against A's run frame if taken
  up.

## 6. Test and acceptance strategy

- **Canonicity proptest** (new, and now load-bearing for hashing):
  the same leaf set built by different insertion orders, merge
  shapes, and redaction sequences yields structurally identical
  trees — every branch ≥ 2 children, maximal prefixes — and
  identical hashes.
- **Virtual-level consistency proptest** (new): for a random tree
  and every spine position, the exploded child's hash equals the
  hash of a canonically constructed tree over the same content at
  that depth. This is the property mixed-shape peer comparison
  rests on, and under the old convention it held by construction —
  it must not regress to untested.
- **Preimage-injectivity unit tests**: the collision pairs the
  lengths and tags exist to prevent, pinned explicitly —
  empty-suffix leaf vs empty root, prefix-byte vs child-record
  boundary shifts.
- The mirror differential and conformance suites run unchanged: a
  wrong rule surfaces as spurious disputes (equal content,
  unequal hashes) or missed ones, both of which the
  oracle-vs-streaming comparisons catch.
- Snapshot re-accepts in one isolated commit, reviewed to confirm
  only hash-valued bytes moved.
- `tests/hop_trace.rs` pins hop counts unchanged — the rule does
  not touch the schedule.
- Acceptance: the §8 sweep. Expected [derived, envelope]: both
  protocols' d = 0 insertion cells improve by roughly the spine
  share of their blake3 row — for V2 ~6 ± 1 ms/session of the
  8.3 ms measured, V1 proportionally — with the V2/V1 ratio
  roughly preserved (E is parity-neutral by design). A result far
  outside that envelope means the cost model is wrong; stop and
  re-profile before merging.

  **Measured [checked, 2026-07-18]** — outside the envelope, on
  the high side: V2 insertions d = 0 recovered **10.3 ms**
  (29.5 → 19.25) and V1 **10.4 ms** (19.6 → 9.20); redactions
  −2.3/−2.0 ms; the d = 1..100 insertion cells all shifted by the
  same ~10 ms absolute. The V2−V1 gap is unchanged (9.9 → 10.1 ms)
  — parity-neutral exactly as designed — though the *ratio* reads
  worse (1.51× → 2.09×) because the shared term collapsed; the
  gap, not the ratio, is the parity metric. The re-profile the
  envelope rule demanded explains the excess: blake3 compression
  fell 27.9 % → 8.0 % of the session (≈ 6.8 ms, in-envelope), and
  the unmodeled ~3.5 ms was the fold's *freight* — one `Vec`
  allocation per spine wrap (malloc ≈ −1.6 ms/session) plus
  per-call hash setup/finalize beyond the compressions. Lesson
  for future envelopes: cost a per-item loop by the whole call,
  not the arithmetic inside it.

## 7. Open questions (deliverables for the implementation)

1. `Hash::branch`/`Hash::leaf`'s public docs promise the
   one-rule-everywhere story ("whether the branch is a
   fully-materialized multi-child node or a single-child virtual
   level"); the API becomes something like
   `Hash::node(prefix, children)` and the docs need the
   single-preimage rewrite, with the canonicity dependence named
   at the crate-level hashing story.
2. Audit for anything that relies on today's *constant* leaf hash
   beyond the `LazyLock` memo shortcut (believed nothing
   [unverified]: leaf identity travels as the path, and listings
   compare leaf hashes only under equal paths).
3. Whether `before-viz` or any tooling renders tree hashes and
   needs re-pinning (believed no [unverified]).
