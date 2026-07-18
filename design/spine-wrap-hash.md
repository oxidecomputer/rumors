# Spine-wrap hashing: one compression per compressed prefix

Status: proposed, 2026-07-18. This is the design note that
`design/streaming-latency-serialization.md` §10 lever E calls for;
the measurements referenced here are that document's (§8 cells,
post-lever-A profile). Nothing below is implemented.

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

Add a third rule. A node whose compressed prefix is non-empty
hashes in **one** compression over the whole spine:

    hash(node) = blake3(SPINE_TAG ‖ prefix ‖ base)      prefix ≠ ε
    hash(node) = base                                    prefix = ε

where `base` is the node's existing base hash (the leaf constant,
or the branch rule over its children), `SPINE_TAG = 2` extends the
`LEAF_TAG = 0` / `BRANCH_TAG = 1` domain-separation family, and
`prefix` is written in **path order** — shallowest byte first, the
same order the node serializer emits (in-memory storage is
shallowest-last; the implementation iterates reversed).

The preimage is at most 1 + 31 + 16 = 48 bytes — under blake3's
64-byte block — so the wrap is exactly one compression for every
possible spine [derived: `MERKLE_HASH_LEN` = 16, prefix ≤ 31
within a 32-byte path]. Parsing is unambiguous: the child hash is
always the trailing 16 bytes, so equal-length preimages split
identically, and the tag separates the three domains.

Mid-spine (virtual-level) hashes remain well-defined and get
cheaper: the node one virtual level down the spine hashes as
`blake3(SPINE_TAG ‖ prefix[1..] ‖ base)` — one compression per
level instead of today's O(remaining spine) refold after the memo
clear. The descent machinery (`into_children`) is unchanged in
shape; only `Node::hash`'s fold and its memo recompute change.

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
  yields equal shape, hence equal spines, hence equal wraps.
- This invariant is **already load-bearing on the wire**: the node
  serializer discriminates leaf from branch *by shape*, untagged,
  and documents "multi-child branches always carry at least two
  children, by the path-compression invariant"
  (`untyped.rs::serialize_to`). The wrap adds a second dependent
  of an invariant the codec already relies on, not a new
  obligation.
- Cross-peer mid-spine comparisons are the interesting case: peer
  A mid-spine at depth d versus peer B's materialized node at
  depth d. If their contents under the scope are equal, B's
  canonical shape below d equals A's virtual suffix, and the two
  wraps agree; if contents differ, the hashes differ with the
  usual 2⁻¹²⁸ pairwise false-equal probability, unchanged.

What is *lost*: the convention no longer defines a hash for a
one-child branch at all. That is deliberate negative space — a
rule for an unrepresentable shape is documentation of the
impossible — but it means any future change that relaxes the
compression invariant breaks hashing silently. The implementation
should pin the invariant with its own proptest (see §6) so it
fails loudly instead.

## 4. What it breaks

Every non-leaf hash value in every tree, in both protocols.
Concretely:

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
  no stored-state migration — the break is wire-visible only.
- Mixed-version sessions. A pre-break and post-break peer with
  identical content disagree on every internal hash: each session
  walks the full tree, matches every leaf, transfers nothing, and
  terminates — convergent but pathologically expensive, *every*
  session, forever. This must not be reachable silently.
  [decision needed]: gate the break in the session preamble,
  which already rejects incompatible peers on its
  magic/protocol/network check — either new `Protocol` wire
  discriminants for the post-break formats or a hash-convention
  byte alongside them. Recommendation: new discriminants; the
  preamble then fails fast with the existing incompatible-peer
  error and no new mechanism.

For a deployed universe this is a flag-day change. That is
acceptable now — the formats are ours, nothing persists — and the
cost of deferring grows with every deployment, which is an
argument for landing E before stability promises, not after.

## 5. Alternatives considered

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
  argument — where the wrap is a hashing change with wire-value
  fallout only. Recorded as the natural successor if spine hops
  ever dominate (see the §11 hop ledger); it composes with lever
  C's path shipping and should be designed against A's run frame
  if taken up.
- **Fold the prefix into the parent's child record** (listings
  carry `(radix run, hash)`). Moves compression into the wire
  listing format, breaking the fixed `QUERY_CHILD_LEN` record and
  the capture/budget arithmetic built on it, to buy the same
  compressions the wrap buys locally. Rejected.

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
- The mirror differential and conformance suites run unchanged: a
  wrong wrap rule surfaces as spurious disputes (equal content,
  unequal hashes) or missed ones, both of which the
  oracle-vs-streaming comparisons catch.
- Snapshot re-accepts in one isolated commit, reviewed to confirm
  only hash-valued bytes moved.
- `tests/hop_trace.rs` pins hop counts unchanged — the wrap does
  not touch the schedule.
- Acceptance: the §8 sweep. Expected [derived, envelope]: both
  protocols' d = 0 insertion cells improve by roughly the spine
  share of their blake3 row — for V2 ~6 ± 1 ms/session of the
  8.3 ms measured, V1 proportionally — with the V2/V1 ratio
  roughly preserved (E is parity-neutral by design). A result far
  outside that envelope means the cost model is wrong; stop and
  re-profile before merging.

## 7. Open questions (deliverables for the implementation)

1. The preamble gating mechanism (§4): new `Protocol`
   discriminant values, or a convention byte? Decide before the
   snapshot re-accept, so the break and its gate land together.
2. `Hash::branch`'s public doc promises the one-rule-everywhere
   story ("whether the branch is a fully-materialized multi-child
   node or a single-child virtual level"); it and `Node::hash`'s
   docs need the three-rule rewrite, and the crate-level hashing
   story should name the canonicity dependence explicitly.
3. Whether `before-viz` or any tooling renders tree hashes and
   needs re-pinning (believed no [unverified]).
