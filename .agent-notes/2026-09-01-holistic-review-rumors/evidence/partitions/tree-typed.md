# Partition tree-typed: The typed tree layer: hashing, heights, nodes, paths, prefixes, the untyped view, fans, iteration

## Partition summary

This partition is the content tree's storage core and its height-typed veneer. `untyped::Node` is one `Arc<NodeInner>` shape for leaves and branches alike, carrying a deepest-first compressed prefix and three lazy memos: the single-preimage SHA3-256 hash, a `[floor, ceiling]` `Span`, and the maximum version-encoding width. `Fan` is a sorted `SmallVec` of `(radix, Node)` pairs with two inline slots. iter.rs holds a borrowing frontier walk (`Iter`, `Range`) and an owned constant-state spine walk (`RangeOwned`). Above that, height.rs gives Peano heights `Z`/`S<H>` with numbered aliases for the erased dispatch table, and `Node<H>`, `Children<H>`, `Path<H>`, `Prefix<H>`/`ErasedPrefix` re-tag the untyped values so traversals monomorphize per level. hash.rs fixes the 24-byte Merkle width, the 32-byte path width, and the tagged, length-delimited preimage layout. `mod tree` is private and `typed` is `pub(crate)`, so the only public surface here is `MERKLE_HASH_LEN`.

The code is in good shape. The two non-obvious tricks (the `PhantomData<fn() -> H>` auto-trait shortcut, and hash agreement resting on canonical shape rather than on the hash construction) are each explained and tested adversarially; the census funnel, the bulk `from_sorted_leaves` constructor, and the fan's inline-size pin state what they serve outside themselves; every panic site traces to programmer error, with the one trust boundary (`from_sorted_leaves`'s preconditions) enforced upstream by the decoder; recursion depth is structural everywhere. Verification is strong where it matters: literal-byte preimage pins, an independent `reference_hash` over the decomposition API, a virtual-level canonicity proptest against the bulk constructor, and a differential fan suite that checks handle identity.

The dominant issues are retirement residue in prose rather than design debt. The V1 protocol retirement and the leaf-preimage change left four sites describing a node codec that no longer exists, a `LEAF_TAG` doc that lists a field the preimage no longer commits, and a crate-private copy of the 24-byte width argument that has drifted from the reconciliation doc and now prices an actor the same docstring says contributes nothing. Below those: two mechanisms for one goal (`Height`'s supertraits beside hand-rolled impls that claim to avoid them), the number 32 hand-maintained three ways with no `Root::HEIGHT` tie, a handful of one-caller wrappers and bypassed impls that would read better dissolved, a performance contract in `Fan::from_iter`'s doc that the commit path violates, and a set of nits (expect messages, long qualified paths, a sentinel-encoded cursor, testdocs carrying a stale record width).

Lines read: 4164 across the fourteen partition files (2736 production, 1428 test: hash/tests.rs, height/tests.rs, path/tests.rs, untyped/fan/tests.rs, untyped/tests.rs), plus the external anchors each finding cites (act.rs, encode.rs, frame.rs, decode.rs, backend.rs, local.rs, tree.rs, lib.rs, reconciliation.rs, conformance.rs, testing.rs, both unknown.rs, join.rs, erased.rs, the bench header, the mutants config, the two agent-notes rulings, and the pinned smallvec and tinyvec sources). No cargo or just command was run; every "verified" below means grep, git, or a read of the cited lines.

## Findings

### tree-typed-1: Mixed `pub`/`pub(crate)` spelling inside a subtree nothing outside the crate can reach
- Where: src/tree/typed.rs:27-28 (related: src/lib.rs:322, src/tree.rs:67, src/tree/typed/node.rs:155-166, src/tree/typed/node.rs:268-294, src/tree/typed/untyped/iter.rs:184-191, src/tree/typed/untyped/iter.rs:368-395, src/tree/typed/prefix.rs:32, src/tree/typed/prefix.rs:166)
- Class / severity / confidence: idiom / low / medium
- Provenance: verified (read `mod tree;` at lib.rs:322 and `pub(crate) mod typed;` at tree.rs:67; conformance.rs:14-19 exposes only `pub mod link`, with `backend` `pub(crate)` and `#[cfg(test)]`; testing.rs:6-16 re-exports nothing under `typed`; spellings counted per file by grep). The dead-code-lint consequence is assessed, not compiled.
- Seen by: structure; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`mod tree` is private and `typed` is `pub(crate)`, so every item under `typed` has crate-only reach, yet the subtree splits items between `pub` and `pub(crate)` with no rule: `ErasedPrefix` and `Prefix::erase` are `pub(crate)` beside a `pub` `Prefix`; `Node::from_untyped`, `into_untyped`, `leaves`, `from_sorted_leaves` are `pub(crate)` beside `pub` `ceiling`, `hash`, `branch`; every constructor in iter.rs is `pub(crate)` while the walk types are `pub`. The modifier reads as an API boundary that does not exist (working default: visibility no wider than use, and one convention so a modifier carries information).

Evidence:

        27	pub(crate) use prefix::ErasedPrefix;
        28	pub use prefix::Prefix;

    src/lib.rs:
       322	mod tree;

    src/tree.rs:
        67	pub(crate) mod typed;

Resolution: pick one convention for src/tree/typed: either `#![warn(unreachable_pub)]` at the crate root with every item under `tree` spelled `pub(crate)`, or plain `pub` throughout with the `pub(crate)` modifiers dropped. The typed.rs:19-21 comment about rustdoc link resolution concerns `mod untyped` and stays valid under either. Acceptance: one spelling per item kind across the subtree; if `unreachable_pub` is adopted, `just clippy` is clean under it.

### tree-typed-2: `MERKLE_HASH_LEN`'s user-facing doc opens its second paragraph with a verbless fragment
- Where: src/tree/typed/hash.rs:6-12 (related: src/lib.rs:348, src/snapshot.rs:80)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (lib.rs:348 `pub use tree::MERKLE_HASH_LEN;`; snapshot.rs:80 returns `[u8; crate::MERKLE_HASH_LEN]`)
- Seen by: prose; refutation: reframed (the "names the tree" half dropped: `crate::reconciliation` is public and describes the tree at length); history: no-rationale-found
- Owner-gated: yes: user-facing prose, Finch's final say under the documentation change policy

This is the partition's only public rustdoc: the constant is re-exported at the crate root and is the width of `Snapshot::hash`'s return. Its second paragraph is a noun phrase with no verb; a public doc's sentences must stand alone.

Evidence:

         6	/// Width in bytes of the tree's Merkle hashes.
         7	///
         8	/// The subtree-comparison digests that gossip exchanges, surfaced as
         9	/// [`Snapshot::hash`](crate::Snapshot::hash). Narrower than the 32-byte
        10	/// version-derived leaf path; the width argument is in [the reconciliation
        11	/// docs](crate::reconciliation).

Resolution: proposed text: "Width in bytes of the digests gossip compares, as returned by [`Snapshot::hash`]. Narrower than a message's 32-byte address; the width argument is in [the reconciliation docs](crate::reconciliation#twenty-four-byte-digests)." Acceptance: the rendered doc for `rumors::MERKLE_HASH_LEN` has no fragment.

### tree-typed-3: `Hash`'s copy of the 24-byte width argument has drifted from the doc of record and prices an actor the same docstring says contributes nothing
- Where: src/tree/typed/hash.rs:39-43 (related: src/tree/typed/hash.rs:9-11, src/tree/typed/hash.rs:26-47, src/tree/typed/hash.rs:100-102, src/reconciliation.rs:164-172)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read reconciliation.rs:138-177 clause by clause against hash.rs:26-47; `git log -S'Why 24 bytes here'` names 2d1e6ea51 alone, whose body says "reconciliation.rs and hash.rs carry the re-derived argument"; `git show 9c73d7b46 -- src/tree/typed/hash.rs` touched only hunks at lines 7-11 and 92-110, leaving the Grinding bullet as written)
- Seen by: structure, prose; refutation: confirmed; history: two homes were owner intent at 2d1e6ea51 (deliberate-and-holds for the shape), while the content expired at 9c73d7b46, which re-derived reconciliation.rs only
- Owner-gated: yes for the dedupe (the two-home decision is recorded in 2d1e6ea51's message); no for the minimum fix (re-sync the bullet)

The `# Why 24 bytes here, and 32 for content` section restates the argument that `MERKLE_HASH_LEN`'s doc (lines 9-11) and the section itself (lines 31-32) already delegate to `crate::reconciliation`. Its Grinding bullet prices a content author's "colliding content pair" at 2⁹⁶, but the same docstring says at lines 100-102 that a content author contributes no bit to any compared quantity, and reconciliation.rs:166-169 says the content-grinding route is "structurally gone, not merely priced" and that the 2⁹⁶ floor prices influence over which versions are created. One argument in two homes has drifted, and the crate-private copy now misidentifies the actor it prices (Principle 5, one statement of record; a pricing argument that names the wrong actor is a prose correctness defect).

Evidence:

        39	/// - **Grinding.** For an author of message *content* who is not a peer —
        40	///   the one adjacent actor the trust model admits — the offline birthday
        41	///   floor for assembling any colliding content pair is 2⁹⁶ hash
        42	///   evaluations, which closes that vector unconditionally, with no
        43	///   premise about what an attempt would cost the attacker.

       100	    /// leaf commits no message bytes: a content author contributes no bit
       101	    /// to any compared quantity — digests are content-blind by design, a
       102	    /// modeled trade.

    src/reconciliation.rs:
       166	//! children, and message bytes appear nowhere. An author of message
       167	//! content therefore contributes zero bits to any compared quantity — the
       168	//! offline content-grinding route to a collision is structurally gone, not
       169	//! merely priced. What could still contribute bits is influence over which
       170	//! versions get created (an actor steering gossip schedules steers the
       171	//! version set); against any such actor, the 24-byte width keeps the
       172	//! offline birthday floor at 2⁹⁶ evaluations, an unconditional bound that

Resolution: preferred: cut lines 26-47 down to what is local to the type (a Merkle hash is an equality probe between subtrees at one prefix; a false-equal's cost and the width that prices it live in `crate::reconciliation#twenty-four-byte-digests`), keeping the SP 800-107 sentence at 19-24 and the link. Minimum: rewrite the Grinding bullet to match reconciliation.rs: content contributes zero bits by construction, and the 2⁹⁶ floor is the unconditional bound against an actor steering which versions are created. Acceptance: the crate states the 24-byte pricing derivation once, and `Hash`'s doc asserts nothing about content authors that lines 100-102 contradict.

### tree-typed-4: `std::fmt::Formatter`, `std::fmt::Result`, and `std::cmp::Ordering` spelled in full at every manual impl
- Where: src/tree/typed/hash.rs:52-56 (related: src/tree/typed/height.rs:38, 56, 62-63; src/tree/typed/path.rs:81, 87, 95; src/tree/typed/prefix.rs:98, 223, 229, 235; src/tree/typed/untyped.rs:113-114; src/tree/typed/untyped/fan.rs:74-75; src/tree/typed/node.rs:143)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep of the partition; seven short-form `fmt::` sites elsewhere in src, so the crate mixes both spellings)
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Every manual `Debug`/`Ord` impl in the partition writes the long paths, even where `use std::fmt::Debug` is already imported at the top of the file. The doctrine names `fmt::Result` and `fmt::Formatter` as the informative spelling; `std::fmt::Result` is not.

Evidence:

        52	impl Debug for Hash {
        53	    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

Resolution: `use std::fmt;` (and `use std::cmp::Ordering;` where needed) per file, then `fmt::Formatter<'_>`, `fmt::Result`, `Ordering::Equal`. Keep `std::hash::Hash` qualified at height.rs:43-44: the module's neighbour type named `Hash` makes that qualification informative. Acceptance: `grep -rn 'std::fmt::Formatter\|std::fmt::Result\|std::cmp::Ordering' src/tree/typed/` returns nothing.

### tree-typed-5: `LEAF_TAG`'s doc says the leaf preimage commits the version's encoding; `Hash::leaf` commits the suffix alone
- Where: src/tree/typed/hash.rs:60-64 (related: src/tree/typed/hash.rs:88-89, src/tree/typed/hash.rs:110-118, src/tree/typed/hash/tests.rs:30-32)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git show 961f63c6:src/tree/typed/hash.rs` line 116 reads `pub fn leaf(suffix: &[u8], version: &crate::Version)`; f3fef7bc is "tree: leaf digests commit the suffix alone"; `git blame -L 60,64` attributes all five lines to 961f63c6, unchanged since; `Hash::leaf`'s body read at 110-118)
- Seen by: prose, correctness; refutation: confirmed; history: deliberate-but-expired (accurate when written; the owner-ruled f3fef7bc updated `Hash::leaf`'s doc and the layout test but not this tag's)
- Owner-gated: no

The tag's docstring lists two committed fields, suffix and version encoding. The code hashes `LEAF_TAG ‖ suffix_len ‖ suffix` and nothing else, `Hash::leaf`'s own doc says so at line 89, and `leaf_preimage_layout` states the opposite of this doc ("never any version or message bytes"). Two docs in one module disagree about what a wire-visible digest commits (statement faithfulness; the AGENTS.md hard rule on prose describing removed behavior).

Evidence:

        60	/// Leaves are version-addressed (the path is the full-width hash of the
        61	/// leaf's version; see [`Path::for_leaf`](super::Path::for_leaf)), so a
        62	/// leaf's preimage commits its compressed suffix — path bytes — and its
        63	/// version's canonical encoding, never its message bytes: every compared
        64	/// digest in the tree is a pure function of the version set.

        88	    /// The hash of a leaf observed from the top of its compressed `suffix`:
        89	    /// `sha3_256(LEAF_TAG ‖ suffix_len ‖ suffix)`.

    src/tree/typed/hash/tests.rs:
        30	/// A leaf commits to exactly `LEAF_TAG ‖ suffix_len ‖ suffix` — its
        31	/// compressed suffix, length-tagged, and never any version or message
        32	/// bytes.

Resolution: restate: the preimage commits the compressed suffix alone; because a leaf's path is the full-width hash of its version, the suffix (with the prefix above it) commits the version transitively, and no version or message bytes enter. Acceptance: `LEAF_TAG`'s doc, `Hash::leaf`'s doc, and `leaf_preimage_layout`'s doc name the same field list.

### tree-typed-6: Both hash preimages heap-allocate a `Vec` per computation although each is statically bounded
- Where: src/tree/typed/hash.rs:110-118 (related: src/tree/typed/hash.rs:173-177, src/tree/typed/untyped.rs:464-479, benches/branch_hash.rs:14-18)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read; not measured)
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found (the bench header says `contiguous` includes the allocation to measure the end-to-end cost, not as a verdict on the buffer type)
- Owner-gated: no

`Hash::leaf` allocates a `Vec` for at most 2 + 32 bytes and `Hash::branch` a `Vec` for at most 4 + 32 + 256 × `CHILD_RECORD_LEN` bytes. Both run inside `Node::hash`'s memo fill, so the allocation is paid once per node-hash computation: every fresh spine node after a commit, and every virtual level that `into_children`/`beneath` exposes (each resets the memo). A stack buffer strictly deletes the malloc/free without changing a preimage byte (fixed sign; wire snapshots untouched). The committed bench never compared against this alternative.

Evidence:

       113	        let mut buf = Vec::with_capacity(2 + suffix.len());

       176	        let mut buf =
       177	            Vec::with_capacity(4 + prefix.len() + CHILD_RECORD_LEN * children.size_hint().0);

    benches/branch_hash.rs:
        16	//! which the hash tests pin byte-for-byte. `contiguous` reproduces the
        17	//! shipped form including its per-call buffer allocation, so the measured
        18	//! difference is the end-to-end cost a caller sees, not the hash core alone.

Resolution: in `Hash::leaf`, assemble into a `tinyvec::ArrayVec<[u8; 34]>` (tinyvec is already used by untyped.rs and prefix.rs); in `Hash::branch`, a `SmallVec` sized for the modal fan (fan.rs:5-8: interior branches rarely carry more than a handful of children), so hot small nodes stay allocation-free and the saturated case spills. Add a `stack` curve to `benches/branch_hash.rs` so the shipped form's claim stays re-measurable against this alternative. Acceptance: hash/tests.rs and the wire snapshots unchanged; the bench shows the stack arm at or below `contiguous` at every fan-out, recorded in the commit message.

### tree-typed-7: No committed demonstration that the layer's `debug_assert!` guards fire
- Where: src/tree/typed/hash.rs:196-210 (related: src/tree/typed/untyped/fan.rs:155-158, src/tree/typed/prefix.rs:45-49, src/tree/typed/untyped.rs:280-291)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified (`grep -rn 'should_panic\|catch_unwind' src/tree/typed/` returns nothing; .cargo/mutants.toml:33-35 says the campaign runs the dev/test profile so debug assertions are part of the observer)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found, scope disputed in part (an inverted guard panics on good inputs and fails the committed suites; only a weakened comparison is uncovered)
- Owner-gated: no

The layer's structural guards are debug assertions: ascending radix order and the one-child ban in `Hash::branch`, `Fan::push`'s order check, `ErasedPrefix::assume`'s length-vs-height witness, and `from_sorted_leaves`'s ascending-run and bare-leaf checks. Nothing under `src/tree/typed/**` uses `should_panic` or `catch_unwind`. A guard whose comparison weakens (`<=` for `<`, a dropped conjunct) passes every committed test while protecting nothing, and cargo-mutants does not mutate inside macro invocations, so the campaign does not see these conditions either (assessed). Adequacy defense: a criterion needs a committed demonstration that a known-bad input fails it.

Evidence:

       196	            debug_assert!(
       197	                previous.is_none_or(|previous| previous < radix),
       198	                "branch children must arrive in strictly ascending radix order",
       199	            );
    ...
       207	        debug_assert!(
       208	            count != 1,
       209	            "a one-child branch is unrepresentable under the canonical-shape invariant",
       210	        );

Resolution: one `#[cfg(debug_assertions)] #[should_panic(expected = "...")]` test per guard in the guarded function's sibling tests.rs (prefix.rs gets a new `prefix/tests.rs` and `mod tests;`). Acceptance: each new test fails when its guard is deleted or weakened to `<=`, and passes with it present; the gate's `testdoc` sees a doc comment on each.
Construction: hash/tests.rs: `Hash::branch(&[], [(2u8, Hash::default()), (1u8, Hash::default())])` expecting "strictly ascending radix order"; `Hash::branch(&[], [(0u8, Hash::default())])` expecting "one-child branch is unrepresentable". fan/tests.rs: `let mut f = Fan::new(); f.push(3, child()); f.push(3, child());` expecting "not greater than the current last". untyped/tests.rs: two entries sharing one `[u8; 32]` path through `Node::from_sorted_leaves(0, &mut entries)` expecting "strictly ascending by path"; a one-entry run whose leaf was pre-wrapped with `.beneath(0)` expecting "supplies bare leaf nodes". prefix/tests.rs: `Prefix::<Root>::new().erase().assume::<Z>()` expecting "re-tags at the height it was erased at".

### tree-typed-8: `Hash`'s bytes are reachable three ways, and `From<[u8; MERKLE_HASH_LEN]> for Hash` has no caller
- Where: src/tree/typed/hash.rs:224-240 (related: src/tree/typed/hash.rs:50, src/tree/mirror/streaming/remote/codec/frame.rs:478, src/tree.rs:277)
- Class / severity / confidence: simplification / nit / medium
- Provenance: verified as far as grep reaches (`Hash::from(` has no caller in src, tests, or benches; the one production construction site writes `Hash(hash)` on the tuple field; an inferred `.into()` cannot be excluded by grep)
- Seen by: structure; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The array is exposed as the `pub` field `.0` (frame.rs:478 and tests), via `as_bytes()` (hash.rs:202 and tests), and via `From<Hash> for [u8; 24]` (tree.rs:277). The fourth spelling, the array-to-`Hash` `From`, is bypassed at its one natural site. One way to do a thing; an impl with no caller is the circular-justification tell.

Evidence:

       224	    /// Reference to the raw [`MERKLE_HASH_LEN`] bytes.
       225	    pub fn as_bytes(&self) -> &[u8; MERKLE_HASH_LEN] {
       226	        &self.0
       227	    }
       228	}
       229	
       230	impl From<[u8; MERKLE_HASH_LEN]> for Hash {
       231	    fn from(bytes: [u8; MERKLE_HASH_LEN]) -> Self {
       232	        Hash(bytes)
       233	    }
       234	}

    src/tree/mirror/streaming/remote/codec/frame.rs:
       478	        self.children.push((radix, Hash(hash)));

Resolution: either make the field private and route construction through `Hash::from` (frame.rs:478 and the test literals become `Hash::from([...])`), or keep the `pub` field and delete the unused `From` impl. Acceptance: one construction spelling and one read spelling for `Hash`'s bytes across src.

### tree-typed-9: Testdocs out of step with the code: 17-byte child records, every-node openers on root-only tests, "version" for "ceiling"
- Where: src/tree/typed/hash/tests.rs:10-12 (related: src/tree/typed/untyped/tests.rs:460-462, 216-223, 242-249, 118-127, 172-177; src/tree/typed/hash.rs:79)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (`CHILD_RECORD_LEN = 1 + MERKLE_HASH_LEN` with `MERKLE_HASH_LEN = 24` at hash.rs:12 and 79; `git log -S'17-byte'` shows the phrase written at 5a6dd8a21 under the 16-byte width; `git show 2d1e6ea51 | grep 17-byte` shows the widening commit removed two other "17-byte" mentions and left these two; the root-only bodies read at untyped/tests.rs:225-240 and 251-270)
- Seen by: prose ([30]), prose/history ([32] second half); refutation: reframed ("17-byte" is wrong, not merely unnamed; severity raised); history: [30] deliberate-but-expired (`version()` became `ceiling()` at f7257af49 and the helper docs were not reworded)
- Owner-gated: no

Two testdocs describe the child record as 17 bytes; it is 25 (`CHILD_RECORD_LEN`), the 17 surviving from the 16-byte-digest era. `version_is_join_of_leaf_versions` and `floor_is_meet_of_leaf_versions` open with "Every node's ..." but check only `tree.ceiling()` and `tree.floor()`; the per-node claim is what `bounds_are_the_leaf_fold_at_every_node` checks. The helper docs at 118-127 and 172-177 call a node's bound its "version" and say "branch versions are recomputed by `Node::branch`", though bounds are memoized lazily under the names `ceiling`/`floor`. AGENTS.md: an inaccurate testdoc is a bug in the test.

Evidence:

        10	/// The prefix is length-tagged in one byte, the child count is a big-endian
        11	/// `u16`, then 17-byte records follow in the iteration order given, with no
        12	/// other framing or padding.

    src/tree/typed/untyped/tests.rs:
       462	    /// ascending 17-byte `radix ‖ hash` records.

       216	    /// Every node's ceiling is the join of its descendant leaves' versions.

       125	    /// leaves pass their original version back into `Node::leaf`, and branch
       126	    /// versions are recomputed by `Node::branch` from the same per-child
       127	    /// versions we started with.

Resolution: replace "17-byte" with "`CHILD_RECORD_LEN`-byte" (or "`radix ‖ hash`") at both sites; open the two root tests with the root claim ("The root's ceiling is the join of every leaf's version") and point at `bounds_are_the_leaf_fold_at_every_node` for the per-node statement; replace "version", "branch versions", and "root version" in the helper docs with "ceiling" or "bounds", and replace "recomputed by `Node::branch`" with "memoized lazily from the rebuilt children". Acceptance: `grep -rn '17-byte' src/tree/typed` is empty; each testdoc's first sentence is checked by its own body; no helper doc in the file calls a bound a "version".

### tree-typed-10: `#[repr(C)]` on the zero-sized height markers
- Where: src/tree/typed/height.rs:13-14 (related: src/tree/typed/height.rs:69-70, src/lib.rs:296)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (`git show 847f772e0:src/tree/typed/height.rs` has `#[repr(C)] pub struct S<T>(pub T);`; lib.rs:296 `#![cfg_attr(not(test), forbid(unsafe_code))]`)
- Seen by: structure; refutation: confirmed; history: no-rationale-found (arrived with a value-carrying `S<T>`; edea11aca hollowed the type and left the attribute)
- Owner-gated: no

`S<T>` and `Z` carry `#[repr(C)]`. Both are phantom markers, the crate forbids `unsafe`, and nothing transmutes or crosses FFI, so the attribute constrains nothing and states no purpose; height/tests.rs already pins size 0 and align 1 independently (Principle 3: an attribute earns its place by naming what it serves).

Evidence:

        13	#[repr(C)]
        14	pub struct S<T>(PhantomData<fn() -> T>);

        69	#[repr(C)]
        70	pub struct Z;

Resolution: remove both attributes. Acceptance: height.rs has no `repr` attribute; height/tests.rs passes.

### tree-typed-11: `Height`'s `Debug + Clone + Default` supertraits and the hand-rolled impls that "avoid bounds on `H`" are two mechanisms for one goal
- Where: src/tree/typed/height.rs:79 (related: src/tree/typed/path.rs:61-69, src/tree/typed/prefix.rs:202-212, src/tree/typed/node.rs:23-39, src/tree/typed/node.rs:130-137)
- Class / severity / confidence: vestigial / low / medium
- Provenance: assessed by grep, not compiled (no `#[derive` within six lines above any `H`-generic header anywhere in src; no `H::default()`, `H: Debug`, `H: Clone`, or `H: Default` bound outside typed; every height marker is `PhantomData<fn() -> H>`; height/tests.rs:18 calls `S::<Z>::default()` on the concrete type)
- Seen by: structure; refutation: confirmed (with the `Copy` nuance); history: deliberate-but-expired (at 847f772e0 `#[derive(Clone, Debug)] pub struct Node<.., H: Height>` consumed the supertraits; the derives became manual impls at 8231541a0/fe3612311 and the bounds stayed)
- Owner-gated: no

`Height` requires `Debug + Clone + Default`, yet `Node<H>`, `Children<H>`, `Path<H>`, and `Prefix<H>` hand-write `Clone`/`Default`/`Debug` with the stated reason "so we don't require unnecessary bounds on `H`". Because `H: Height` already implies those bounds, the manual impls avoid nothing they claim to, and nothing I can find consumes the supertraits. One of the two is dead weight, and the rationale comments are false either way. Only `Copy`, which `Height` does not imply, genuinely needs the manual impls.

Evidence:

        79	pub trait Height: Debug + Clone + Default + sealed::Sealed + 'static {

    src/tree/typed/path.rs:
        61	// Manual copy/clone impls so we don't require unnecessary bounds on `H`:

    src/tree/typed/prefix.rs:
       202	// Manual clone/comparison impls so we don't require unnecessary bounds on `H`.

Resolution: reduce the trait to `pub trait Height: sealed::Sealed + 'static { const HEIGHT: usize; }` and run `just check`; the manual impls then do what their comments say. If the compile reveals a consumer, keep the bounds and rewrite the three comments to say the manual impls exist for `Copy` (and, for `S<T>`, the reason at height.rs:16-21). Acceptance: either `Height` names no `Debug + Clone + Default` and `just check` is clean, or the comments at path.rs:61, prefix.rs:202, and node.rs:23-39 state a true reason.

### tree-typed-12: The number 32 is hand-maintained: three enumerations in height.rs, a bare literal at every path-width site, and no `Root::HEIGHT` tie
- Where: src/tree/typed/height.rs:125-133 (related: src/tree/typed/height.rs:116-123, 154-166; literal sites: hash.rs:253, 273, 278; node.rs:301, 304; path.rs:17, 50, 76, 88, 102, 111; prefix.rs:20, 33, 47, 59, 119, 125, 175; untyped.rs:278, 310; iter.rs:301, 387, 473; independent local names for the same width: src/tree/mirror/streaming/window.rs:137 `KEY_DEPTH`, src/bookmark/format.rs:78 `HASH_LEN`, src/tree/mirror/streaming/remote/codec/tests.rs:66 `MAX_ARBITRARY_SUFFIX_LEN`)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (the only compile-time pin is height.rs:166 over `H0`/`H32`; `grep -rn 'Root::HEIGHT' src` finds only runtime uses; `grep -rn 'const .*: usize = 32;' src` finds the three local names and no shared constant; erased.rs:215-238 splices `H~N` aliases and never names `Root`)
- Seen by: structure ([3], [4]), perfapi ([51]); refutation: confirmed (all three); history: no-rationale-found (bf1a5b4bc added `alias_heights!` and the endpoint assert but left `Root` a literal; its doc at 135 asserts "H32 (= [`Root`])" in prose only)
- Owner-gated: no

Heights are enumerated by `impl_heights!` over 32 `_` tokens, by `alias_heights!` over 33 names, and by `Root` as a hand-counted 32-deep `S<` chain; the `const _` ties the aliases to their numbers but `Root` to nothing, so a 31-deep `Root` compiles (H31 has a `Height` impl) and surfaces only through runtime tests. Separately, `MERKLE_HASH_LEN` names the 24-byte width, but the 32-byte path width, which is both SHA3-256's output and `Root::HEIGHT`, is a literal at every site: `[u8; 32]` types, `32 - H::HEIGHT` depth arithmetic, `(depth..32)`, `Vec::with_capacity(32)`, and the depth assert; three modules elsewhere have each coined a local name for the same fact. Principle 5 (no hand-maintained counts) and the working default (named constants over magic numbers); under Principle 6 the cheapest artifact passing today's check is a miscounted `Root`.

Evidence:

       116	// 32 `_` tokens => heights 0..=32 inclusive (33 impls).
    ...
       127	#[rustfmt::skip]
       128	pub type Root =
       129	// Laid out for your counting convenience in two rows of 16:
       130	    S<S<S<S<S<S<S<S<S<S<S<S<S<S<S<S< // 0
       131	    S<S<S<S<S<S<S<S<S<S<S<S<S<S<S<S< // 1
       132	//  0 1 2 3 4 5 6 7 8 9 a b c d e f
       133	    Z>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>;
    ...
       166	const _: () = assert!(H0::HEIGHT == 0 && H32::HEIGHT == 32);

    src/tree/typed/path.rs:
        50	        let byte = self.hash[32 - S::<H>::HEIGHT];

    src/tree/typed/prefix.rs:
        59	        32 - self.hash.len()

    src/tree/typed/untyped.rs:
       310	        let branch_at = (depth..32)

    src/tree/typed/untyped/iter.rs:
       387	            spine: Vec::with_capacity(32),

Resolution: (1) add `pub const PATH_LEN: usize = 32;` beside `MERKLE_HASH_LEN` in hash.rs with a one-line doc (the SHA3-256 output width and the root height), and use `[u8; PATH_LEN]` and `PATH_LEN - H::HEIGHT` at the listed sites; optionally give `Height` a derived `const DEPTH: usize = PATH_LEN - Self::HEIGHT;` so `Path::pop` and `Prefix` read `H::DEPTH`. (2) Merge the two macros into one `heights!(H0 H1 … H32)` that emits both the `Height` impl and the numbered alias per name, using the `$n + 1` accumulator `impl_heights!` already has, then `pub type Root = H32;` and `const _: () = assert!(Root::HEIGHT == PATH_LEN);`. If the two-row layout is wanted as pedagogy, keep it and add only the `Root::HEIGHT` assert. Acceptance: height.rs has one enumeration of the heights (or the literal plus an assert on `Root::HEIGHT`); `grep -n '\b32\b' src/tree/typed/*.rs src/tree/typed/untyped/*.rs` outside doc comments returns only the constant's definition; erased.rs's `at_parent_height!` compiles unchanged.

### tree-typed-13: The `PhantomData<fn() -> H>` argument is written out twice, and the typed wrappers restate the untyped docs they delegate to
- Where: src/tree/typed/node.rs:116-123 (related: src/tree/typed/height.rs:6-12, src/tree/typed/path.rs:9-13, src/tree/typed/prefix.rs:13-16, src/tree/typed/node.rs:184-195, 202-212, 225-234, 244-258, src/tree/typed/untyped.rs:481-515)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); the history verified by `git log -L` (all three phantom paragraphs written at edea11aca; 8f87ddd01 re-justified the node.rs copy "on its own terms")
- Seen by: prose; refutation: confirmed; history: no-rationale-found, noting the node.rs copy is the owner-touched one
- Owner-gated: no

The auto-trait argument appears in full on `S` (height.rs:6-12) and again on `Node` (node.rs:116-123), while `Path` and `Prefix` point at `Node` rather than at `S`, where the chain originates. Likewise the one-line delegating wrappers `dominance`, `version_bytes`, `is_leaf`, and `hash` on `Node<H>` re-explain the untyped semantics instead of stating what the height tag adds and linking the method of record, and untyped.rs:481-515 carries near-verbatim twin paragraphs for `ceiling` and `floor`. Duplicated rationale drifts (finding 3 is one realized instance); Principle 5: one statement of record per argument.

Evidence:

       116	/// The height marker is held as `PhantomData<fn() -> H>` rather than
       117	/// `PhantomData<H>`. Function pointers are unconditionally `Send + Sync`,
       118	/// so any auto-trait obligation on `Node` discharges without descending
       119	/// the `S<S<S<...S<Z>...>>>` peano-style height chain: a bare
       120	/// `PhantomData<H>` would send the trait solver walking 32 levels of
       121	/// `S<…>: Sync` on every `Send`/`Sync` check, even though the type
       122	/// variable `H` is purely phantom and never constructs anything that
       123	/// could fail to be `Send`/`Sync`.

    src/tree/typed/height.rs:
         6	/// The inner marker is `PhantomData<fn() -> T>` rather than `T` (or
         7	/// `PhantomData<T>`) so the auto-trait check on `S<T>` does not descend
         8	/// into `T`. Without this, proving `S<S<…S<Z>…>>: Sync` recurses 32
         9	/// levels deep every time a downstream crate asks an auto-trait question
        10	/// about a type that names `Root`. Function pointers are unconditionally
        11	/// `Send + Sync` regardless of their return type, so this marker
        12	/// short-circuits the recursion without unsafe `Send` / `Sync` impls.

Resolution: keep the full argument once, on `S` in height.rs, and have `Node`, `Path`, and `Prefix` say "`PhantomData<fn() -> H>`; see [`S`]" (the owner last re-justified the node.rs copy at 8f87ddd01; if that wording is preferred, move it to `S`). For each typed wrapper, one sentence stating the typed contract plus a link to the untyped method of record; make `floor`'s doc "The dual of [`ceiling`]" and keep the memo paragraph once. Acceptance: "Function pointers are unconditionally `Send + Sync`" occurs once under src/tree/typed/; each typed wrapper doc is a few lines and links its untyped counterpart.

### tree-typed-14: `compressed_prefix_len`: the typed wrapper has no caller, and both docs say a leaf's count is zero when compressed leaves are the common stored shape
- Where: src/tree/typed/node.rs:236-242 (related: src/tree/typed/untyped.rs:645-651, 292-301, 659-664; src/tree/typed/untyped/tests.rs:334, 343, 590-594)
- Class / severity / confidence: documentation / low / high
- Provenance: verified for the dead wrapper (grep: the only callers, untyped/tests.rs:334 and 343, take an untyped `Node` from `arb_tree`; `git log -S'.map(|n| n.compressed_prefix_len())'` shows the wrapper's only caller removed at fceb55f98); assessed (read) for the doc: `beneath` pushes onto any node, `from_sorted_leaves` extends a lone leaf's whole remaining spine, and `node_hash_preimage_is_in_path_order` hashes a leaf carrying `[0xAA, 0xBB]`
- Seen by: structure ([12]), prose ([24]); refutation: confirmed (both); history: [12] deliberate-but-expired (added for the borsh round-trip tests, dead since fceb55f98); [24] no-rationale-found
- Owner-gated: no

The `#[cfg(test)]` typed wrapper delegates to the untyped method but every caller invokes the untyped one directly, so it is dead test surface. Both docstrings say "Zero for a leaf or a non-compressed branch", which is false for every leaf under a collapsed spine, the shape `nested_singleton_wraps_extend_the_committed_prefix` drives through this very accessor; the module elsewhere (`is_leaf`: "regardless of any path-compressed prefix above it") already says leaves can be compressed.

Evidence:

       236	    /// Number of path-compressed prefix bytes on this node — i.e., the
       237	    /// count of singleton virtual-branch levels collapsed above the node's
       238	    /// actual content. Zero for a leaf or a non-compressed branch.
       239	    #[cfg(test)]
       240	    pub fn compressed_prefix_len(&self) -> usize {
       241	        self.inner.compressed_prefix_len()
       242	    }

    src/tree/typed/untyped/tests.rs:
       593	    let wrapped = leaf.beneath(0xAA).beneath(0xBB);
       594	    assert_eq!(wrapped.hash(), super::Hash::of(&[LEAF_TAG, 2, 0xBB, 0xAA]));

Resolution: delete node.rs:236-242; at untyped.rs:645-647 replace the last sentence with "Zero for an uncompressed node of either kind." Acceptance: `grep -rn compressed_prefix_len src` finds only the untyped definition and its two test callers; no docstring asserts that leaves have a zero prefix length.

### tree-typed-15: `super::Prefix` qualified at five use sites instead of imported
- Where: src/tree/typed/node.rs:268-294 (related: src/tree/typed/node.rs:7-10)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the imports at 7-10 and the sites at 270, 271, 277, 292, 293)
- Seen by: structure; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

node.rs imports `super::hash::Hash`, `super::height::{...}`, and `super::untyped`, but not `Prefix`, so `leaves` and `from_sorted_leaves` spell `super::Prefix` five times in twenty-five lines where the qualification carries no information.

Evidence:

       270	        prefix: &super::Prefix<H>,
       271	    ) -> impl Iterator<Item = (super::Prefix<Z>, Node<Z>)> + Send + use<H> {
    ...
       277	                    super::Prefix::from(key),
    ...
       292	        prefix: &super::Prefix<H>,
       293	        run: Vec<(super::Prefix<Z>, Node<Z>)>,

Resolution: `use super::prefix::Prefix;` beside the other `super::` imports; drop the qualifiers. Acceptance: no `super::Prefix` in node.rs.

### tree-typed-16: `from_sorted_leaves` re-checks a typed fact with `try_from(...).expect` while the infallible `From<Prefix> for [u8; 32]` sits unused
- Where: src/tree/typed/node.rs:301-308 (related: src/tree/typed/prefix.rs:119-123, src/tree/typed/untyped.rs:276-279)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (every `<[u8; 32]>::from(` call in src takes a `Path`; prefix.rs:119-123 provides the `Prefix`-to-array impl with default `H = Z`; `git show 9c73d7b46 -- src/tree/typed/prefix.rs` shows the impl retargeted from a retired `Key` type to `[u8; 32]` without revisiting this call site)
- Seen by: structure ([5]), perfapi ([48]); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

A `Prefix<Z>` is 32 bytes by its type (`32 - Z::HEIGHT`), and the crate's one site converting `Prefix<Z>` to an array bypasses the `From` impl for a fallible `try_from` with an `expect` asserting what the type already fixes (doctrine on guards: a runtime check must name a failure the types cannot prevent; a trait impl bypassed at its one natural site is the circular-justification tell). The same loop also re-collects the incoming `Vec<(Prefix<Z>, Node<Z>)>` into a second `Vec<([u8; 32], Option<untyped::Node>)>`, one allocation per supplied run, because the untyped entry takes exactly that slice shape.

Evidence:

       301	        let mut entries: Vec<([u8; 32], Option<untyped::Node>)> = run
       302	            .into_iter()
       303	            .map(|(path, leaf)| {
       304	                let path = <[u8; 32]>::try_from(path.as_bytes())
       305	                    .expect("a leaf prefix is a full 32-byte path");
       306	                (path, Some(leaf.into_untyped()))
       307	            })
       308	            .collect();

    src/tree/typed/prefix.rs:
       119	impl From<Prefix> for [u8; 32] {
       120	    fn from(value: Prefix) -> Self {
       121	        Path::from(value).into()
       122	    }
       123	}

Resolution: replace lines 304-305 with `let path = <[u8; 32]>::from(path);`, and spell the impl directly (`value.hash.into_inner()`) rather than routing through `Path`. The second `Vec` is worth removing only if a run-heavy bench moves; it is small next to the run's node allocations. Acceptance: no `try_from` in `from_sorted_leaves`; `From<Prefix> for [u8; 32]` has a caller; `every_virtual_level_hashes_canonically` and the backend/local suites pass unchanged.

### tree-typed-17: `Node<Root>::get` takes `&[u8]`, and the untyped walk spends two arms on lengths the type could exclude
- Where: src/tree/typed/node.rs:370-374 (related: src/tree/typed/untyped.rs:365-390, src/tree.rs:289-293)
- Class / severity / confidence: api-surprise / nit / medium
- Provenance: assessed (read: the sole caller, tree.rs:289-293, holds a `[u8; 32]` from `Path::for_leaf`)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found
- Owner-gated: no (crate-private signature)

The doc promises a lookup by full 32-byte path and the only caller has one, yet the parameter is a slice, and the untyped descent handles a leftover tail at a leaf (`path.is_empty().then_some`) and exhaustion at a branch (`split_first()?`) that a `&Path` or `&[u8; 32]` parameter would rule out at the boundary (types-first: make the contract the type).

Evidence:

       370	    /// Look up the live leaf whose full 32-byte path is `path`, by a single
       371	    /// `O(depth)` descent.
       372	    pub fn get(&self, path: &[u8]) -> Option<(&Version, &Message)> {

    src/tree/typed/untyped.rs:
       378	                // A full 32-byte path lands exactly at a leaf; a leftover
       379	                // tail means the path was deeper than the tree.
       380	                Children::Leaf { version, message } => {
       381	                    return path.is_empty().then_some((version, message));

Resolution: `get(&self, path: &Path)` (or `&[u8; 32]`) on the typed root, converting once; keep the untyped walk over a slice and restate its length arms as the invariant they then are. Acceptance: `Tree::get` passes the `Path` it already computes; the "leftover tail" comment is gone or restated as an invariant.

### tree-typed-18: `root_hash` takes `&Option<Root>` where its siblings take `Option<&Self>`
- Where: src/tree/typed/node.rs:409-417 (related: src/tree/typed/node.rs:390-407, src/tree.rs:113-117, src/tree.rs:277, src/tree/traverse/unknown/tests.rs:142-143, src/tree/mirror/streaming/materialized/unknown/tests.rs:80-81)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the three signatures and every caller by grep)
- Seen by: perfapi; refutation: reframed (the clone at tree.rs:277 is a call-site choice, not forced by the signature: `self.root.root` is already an `Option<typed::node::Root>`); history: no-rationale-found
- Owner-gated: no

`range` and `range_owned` take `Option<&Self>`, the shape clippy's `ref_option` prefers; `root_hash` takes `&Option<Root>`. Separately, the one production caller (tree.rs:277, in the tree-root partition) clones the whole `Root` pair per hash read to build a temporary to borrow, although `&self.root.root` already fits the signature.

Evidence:

       409	    /// The observable hash of a possibly-absent root.
       410	    pub fn root_hash(node: &Option<Root>) -> Hash {

       390	    pub fn range_owned<P: causally::Polarity>(
       391	        node: Option<&Self>,

    src/tree.rs:
       277	        Node::root_hash(&self.root.clone().into()).into()

Resolution: `pub fn root_hash(node: Option<&Root>) -> Hash`; callers pass `.as_ref()`. Hand the tree.rs:277 clone to the tree-root partition: `Node::root_hash(self.root.root.as_ref())` removes a `Version` clone and an `Arc` bump per `Tree::hash`/`Snapshot::hash` read. Acceptance: the three `Node<Root>` constructors share one `Option<&_>` shape; no `.clone()` in `Tree::hash`.

### tree-typed-19: Prose describing the retired node codec survives at four sites
- Where: src/tree/typed/node.rs:428-447 (related: src/tree/typed/hash.rs:16-17, src/tree/typed/hash.rs:91-92, src/tree/typed/hash.rs:128-129; the live framing at src/tree/mirror/streaming/remote/codec/frame.rs:469 and 521-522; out of partition: src/tree.rs:36 "the node serializer relies on")
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (`grep -rn 'tree::wire\|serialize_to\|DecodeNode\|count_minus_two\|Read::chain' src tests benches examples` matches only node.rs:428-440; `git log --diff-filter=D --name-only -- src/tree/wire.rs` names 368da2a5 "retire Protocol::V1 and the alternating mirror"; `git blame -L 428,447` gives 7d95be5b, f2b74a97, b524e406, all before the retirement; `git log -S'never length-prefixed'` names f2b74a97, whose `impl Encode for Hash` in the deleted `src/tree/wire.rs` wrote raw bytes; frame.rs:521-522 writes `cbor::write_head(out, MAJOR_BSTR, MERKLE_HASH_LEN as u64)` and frame.rs:469 rejects any other head; `grep -rn serializer src` finds no node serializer, only the payload one in message.rs)
- Seen by: structure ([0]), prose ([20]), correctness ([34], [35]), perfapi ([47]); refutation: confirmed, plus its new item (hash.rs:92, 129); history: contradicts-hard-rule; the V1 retirement plan (.agent-notes/2026-09-01-v1-retirement/README.md:199-201) scoped node.rs to "the gated codec/ingress items and their malformed-input tests" and did not list these comments, so this is a sweep blind spot, not a ruling
- Owner-gated: no

node.rs ends with a twenty-line comment documenting node serialization through `crate::tree::wire`, `untyped::Node::serialize_to`, `DecodeNode`, a `count_minus_two` field, and `std::io::Read::chain` recursion; none exists, and the design it describes (whole nodes travel, untagged) contradicts the live protocol, which ships leaf runs and listings through the CBOR codec. `Hash`'s type doc says the digest travels as raw bytes "never length-prefixed", but the codec writes every hash as a CBOR byte string with a 24-length head and the decoder rejects any other head. `Hash::leaf`'s and `Hash::branch`'s docs describe path order "as the node serializer emits it"; no node serializer exists. AGENTS.md hard rule: nothing refers to code that no longer exists; the block is a plain `//` comment, so `just doclint` cannot see its dangling links.

Evidence:

       428	// Wire format (see [`crate::tree::wire`]). Serialization is
       429	// height-uniform: every typed `Node<H>` delegates to
       430	// [`untyped::Node::serialize_to`], which emits the in-memory
       431	// representation directly (prefix length, head bytes, then either a leaf
       432	// body or a `count_minus_two` + children list). No leaf-vs-branch tag is
    ...
       436	// Deserialization at typed height `H` ([`DecodeNode`]) reads `prefix_len`, then either

    src/tree/typed/hash.rs:
        16	/// A newtype over a fixed-size byte array; on the wire it travels as its
        17	/// raw bytes, never length-prefixed (the width is pinned by the type).

        91	    /// `suffix` is the leaf's path-compressed span in **path order** —
        92	    /// shallowest byte first, as the node serializer emits it — and

    src/tree/mirror/streaming/remote/codec/frame.rs:
       521	        cbor::write_head(out, MAJOR_BSTR, MERKLE_HASH_LEN as u64);
       522	        out.extend_from_slice(hash.as_bytes());

Resolution: delete node.rs:428-447 (the file then ends after the `PartialEq` impl; the one surviving fact, that singletons never materialize and reconstruct through `beneath`, already lives at `Hash::branch`'s canonicity section and `untyped::Node::branch`). At hash.rs:16-17 either drop the wire sentence (the codec module owns framing) or restate it: the digest travels as a 24-byte CBOR byte string whose head the decoder checks against `MERKLE_HASH_LEN`. At hash.rs:92 and 129 cut "as the node serializer emits it"; the sentence already says "shallowest byte first". Hand tree.rs:36 to the tree-root partition. Acceptance: `grep -rn 'tree::wire\|serialize_to\|DecodeNode\|count_minus_two\|Read::chain\|node serializer\|never length-prefixed' src` returns nothing; `just doclint` clean.

### tree-typed-20: `Debug` renderings disagree with what `Eq` compares and with the documented path order
- Where: src/tree/typed/path.rs:94-98 (related: src/tree/typed/path.rs:74-90, src/tree/typed/untyped.rs:113-119, src/tree/typed/untyped.rs:466-470)
- Class / severity / confidence: idiom / nit / medium
- Provenance: assessed (read)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`Debug for Path<H>` prints all 32 stored bytes while `PartialEq`/`Ord` compare only `hash[32 - H::HEIGHT..]`, so two paths that compare equal can print differently in a shrunk counterexample. `Debug for untyped::Node` prints the prefix in storage order (deepest byte first) while every doc, the preimage, and `get` speak in path order. Failure output misleads exactly when someone is reading a failing test.

Evidence:

        94	impl<H: Height> Debug for Path<H> {
        95	    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        96	        self.hash.fmt(f)
        97	    }
        98	}

    src/tree/typed/untyped.rs:
       116	            .field("prefix", &hex::encode(&self.inner.prefix))

Resolution: `Path`: format `&self.hash[32 - H::HEIGHT..]`. `Node`: format the prefix reversed into path order, or label the field `prefix_deepest_first`. Acceptance: a proptest in path/tests.rs: after one pop, `format!("{:?}", ra) == format!("{:?}", rb)` iff `ra == rb`; `node_hash_preimage_is_in_path_order`'s `wrapped` node prints `bbaa`.

### tree-typed-21: `Prefix::containing` builds its bytes by slice-iterate-collect where `ArrayVec::from_array_len` says it in O(1)
- Where: src/tree/typed/prefix.rs:172-180
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified in the pinned dependency (tinyvec 1.11.0 arrayvec.rs:543 `pub fn from_array_len(data: A, len: usize) -> Self`, panicking only when `len > CAPACITY`, impossible for `32 - H::HEIGHT`; `PartialEq`/`Ord` on `ArrayVec` compare the slice view, so bytes past `len` are inert)
- Seen by: structure; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The prefix at height `H` is the path's first `32 - H::HEIGHT` bytes; the code slices, iterates, copies, and collects. tinyvec's `from_array_len` takes the full array and a length and reads as "this array, truncated to the height's depth".

Evidence:

       172	    pub fn containing(path: &Path) -> Self {
       173	        Prefix {
       174	            height: PhantomData,
       175	            hash: <[u8; 32]>::from(*path)[..32 - H::HEIGHT]
       176	                .iter()
       177	                .copied()
       178	                .collect(),
       179	        }
       180	    }

Resolution: `hash: ArrayVec::from_array_len(<[u8; 32]>::from(*path), 32 - H::HEIGHT)` (or `PATH_LEN - H::HEIGHT` once finding 12 lands). Acceptance: `containing` has no iterator chain; the adapter property suites, heavy `containing` users, pass.

### tree-typed-22: Two `expect` messages do not prove the impossibility they guard, and one inverts the geometry
- Where: src/tree/typed/prefix.rs:188-191 (related: src/tree/typed/prefix.rs:88-92, src/tree/typed/node.rs:361-366)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read against the impl bounds: `pop` at 184-187 requires `S<H>: Height`, so `H` is strictly below `Root`; `ErasedPrefix::pop` at 92 has the direction right; `git log -L188,191` shows the line survived the 9ebd1b8d2 accuracy sweep)
- Seen by: structure ([10]), prose ([28]), correctness ([40]); refutation: reframed (untyped.rs:237's `expect("non-empty prefix")` sits directly under its guard and is not a defect); history: no-rationale-found
- Owner-gated: no

`Prefix<H>::pop` says "a prefix above height Root has at least one byte to pop"; nothing is above `Root`, and the bound places `H` below it. The sibling at line 92 states it correctly. `Node<Z>::message` says "typed leaf failed to be a leaf", which names the failure rather than why it cannot happen (compare iter.rs:341 "a Leaf wraps a leaf node, by construction"). The codebase's standard: every `expect` message is a one-line proof; an inverted direction reads as true and is worse than none.

Evidence:

       188	        let byte = self
       189	            .hash
       190	            .pop()
       191	            .expect("a prefix above height Root has at least one byte to pop");

        92	            .expect("a prefix below the root has at least one byte to pop");

    src/tree/typed/node.rs:
       363	        self.inner
       364	            .as_leaf()
       365	            .expect("typed leaf failed to be a leaf")

Resolution: prefix.rs:191: "a prefix below the root has at least one byte to pop" (matching line 92). node.rs:365: "a `Node<Z>` wraps a leaf: the height-zero constructors admit nothing else". Acceptance: both messages state the reason the branch is unreachable; the two `pop` messages agree on direction.

### tree-typed-23: The compressed prefix is a heap `Vec<u8>` per node; an inline `ArrayVec<[u8; 32]>` would delete the allocation (measure first)
- Where: src/tree/typed/untyped.rs:78-87 (related: src/tree/typed/untyped.rs:213, 236, 299, 331, 345, 470, 661; src/tree/typed/untyped/tests.rs:876-893)
- Class / severity / confidence: performance / low / medium
- Provenance: assessed (read; not measured)
- Seen by: perfapi; refutation: uncertain (a resource trade, not a strict deletion: `NodeInner` grows for every node and the `<= 208` pin moves); history: no-rationale-found (`prefix: Vec<u8>` is original; .agent-notes/2026-07-18-node-hash-preimage records allocation in the node path moving a session number)
- Owner-gated: yes: the size pin's own doc says growth must be a deliberate, reviewed decision

Every `NodeInner` stores its prefix as a `Vec<u8>`, so every compressed node (in a uniform-hash tree nearly every leaf, whose spine runs about 28 to 30 bytes) owns a heap block besides its `Arc`, and `beneath`, `from_sorted_leaves`, and `into_children` under sharing touch the allocator for it. The prefix is bounded at 32 by the height cap, and `hash()` already reverses it into a stack `ArrayVec<[u8; 32]>`. Against that, `ArrayVec<[u8; 32]>` is 34 bytes to `Vec`'s 24, so `NodeInner` grows about 16 bytes after padding for every node, leaves included, and the sign is workload-dependent. Doctrine: resource trades get measure-first gating.

Evidence:

        87	    prefix: Vec<u8>,

       470	            let prefix: ArrayVec<[u8; 32]> = self.inner.prefix.iter().rev().copied().collect();

    src/tree/typed/untyped/tests.rs:
       892	    assert!(std::mem::size_of::<super::NodeInner>() <= 208);

Resolution: construct and measure: change `prefix` to `ArrayVec<[u8; 32]>` (`Vec::new()` becomes `ArrayVec::new()` at 213 and 345; the `extend`/`push`/`pop`/`collect` sites keep their API; the reverse-collect at 470 can iterate `rev()` directly), then compare at the parent commit and after with `benches/in_memory.rs`, the `stats_alloc` pattern from `tests/decode_alloc.rs` (allocations per inserted leaf), and `testing::node_census`. Land only on measured evidence, updating `node_inner_stays_within_budget` deliberately with the new size named in the commit. Acceptance: a committed before/after comparison with the pin updated in the same commit, or the proposal recorded as declined with the numbers.

### tree-typed-24: Manual `Clone`/`Default` impls on `NodeInner`, untyped `Children`, and `Fan` are what `#[derive]` generates
- Where: src/tree/typed/untyped.rs:103-111 (related: src/tree/typed/untyped.rs:164-187, src/tree/typed/untyped/fan.rs:58-72)
- Class / severity / confidence: idiom / low / high
- Provenance: verified by reading the field types' impls in the pinned dependencies and siblings (smallvec 1.15.1 lib.rs:2125 `impl<A: Array> Default for SmallVec<A>`, 2160 `impl<A: Array> Clone for SmallVec<A>`; `#[derive(Debug, Clone, PartialEq, Eq)] pub struct Span` at crates/before/src/span.rs:104; `#[derive(Clone)] pub struct Message` at src/message.rs:47; std's `OnceLock<T: Clone>: Clone`); not compiled
- Seen by: structure; refutation: confirmed; history: deliberate-but-expired (at b524e406^ the types were generic over the payload `T` and `impl<T> Clone` avoided a `T: Clone` bound; item erasure dropped `T` and kept the bodies)
- Owner-gated: no

Three non-generic types hand-write impls whose bodies are field-wise clones or defaults, about thirty-five lines a reader must check for a deviation that is not there. Every field type derives or implements the trait already. The untyped `Node`'s manual `Clone` is different and stays: it is the census funnel. Machinery outlived the constraint (payload genericity) that justified it.

Evidence:

       103	impl Clone for NodeInner {
       104	    fn clone(&self) -> Self {
       105	        Self {
       106	            prefix: self.prefix.clone(),
       107	            hash: self.hash.clone(),
       108	            children: self.children.clone(),
       109	        }
       110	    }
       111	}

    src/tree/typed/untyped/fan.rs:
        58	impl Default for Fan {
        59	    fn default() -> Self {
        60	        Self {
        61	            entries: SmallVec::new(),
        62	        }
        63	    }
        64	}

Resolution: `#[derive(Clone)] struct NodeInner`; `#[derive(Debug, Clone)] enum Children`, moving the memo-carrying comment at untyped.rs:171-173 above the derive; `#[derive(Default, Clone)] pub struct Fan`. Acceptance: the three manual blocks are gone; `just check` and the untyped and fan proptests pass unchanged.

### tree-typed-25: Untyped `Children` (a leaf-or-branch body) and typed `Children<H>` (a radix fan) share a name one module apart
- Where: src/tree/typed/untyped.rs:122-134 (related: src/tree/typed/node.rs:15-21, src/tree/typed/untyped/iter.rs:16)
- Class / severity / confidence: modularity / nit / medium
- Provenance: assessed (read)
- Seen by: structure; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The untyped enum named `Children` has a `Leaf` variant holding no children: it is the node's body kind. The typed `Children<H>` is a fan of children. iter.rs imports one while node.rs defines the other; a reader crossing the layer must remember which is in scope, and the untyped name misdescribes its `Leaf` variant.

Evidence:

       122	/// The children of a node.
       123	#[derive(Debug)]
       124	enum Children {
       125	    /// A direct leaf, at the true bottom of the tree.
       126	    Leaf {

    src/tree/typed/node.rs:
        18	pub struct Children<H: Height> {

Resolution: rename the untyped enum (`Body` or `Content`), a mechanical change confined to untyped.rs and iter.rs. Acceptance: one type named `Children` under src/tree/typed.

### tree-typed-26: `Node::branch`'s `None` arm and singleton collapse are undocumented at both layers
- Where: src/tree/typed/untyped.rs:201-211 (related: src/tree/typed/node.rs:317-324, src/tree/traverse/unknown.rs:65-73, src/tree/traverse/act.rs:129, src/tree/traverse/join.rs:195, src/tree/typed/untyped/tests.rs:150-160)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: the body returns `None` for an empty fan and `beneath` for one child; unknown.rs:65-73, act.rs:129, and join.rs:195 rely on an emptied subtree vanishing)
- Seen by: prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Both `branch` docs say only "Construct a new branch node from ... children" over an `Option<Self>` return. The `None` arm is what makes redaction pruning delete empty subtrees, and the one-child arm is the canonical-shape collapse the hash convention rests on; a maintainer must read the body to learn either (documentation altitude: state every return arm).

Evidence:

       201	    /// Construct a new branch node from a list of children with distinct
       202	    /// indices (inverse to [`Node::into_children`]).
       203	    pub fn branch(children: Fan) -> Option<Self> {
       204	        match children.len() {
       205	            0 => None,
       206	            1 => {
    ...
       210	                Some(node.beneath(index))

Resolution: add to both docs: "`None` when `children` is empty: an empty subtree has no node. A single child is not materialized as a branch; it is returned collapsed into its compressed prefix (see [`beneath`]), so every materialized branch has at least two children." Acceptance: both `branch` docstrings name the empty and singleton arms; `empty_branch_is_none`'s testdoc can cite the contract rather than restate it.

### tree-typed-27: `from_sorted_leaves` panics on an empty run and on a consumed slot but has no `# Panics` section
- Where: src/tree/typed/untyped.rs:260-279 (related: src/tree/typed/untyped.rs:285-287, 308-312; src/tree/typed/node.rs:284-294; src/tree/mirror/streaming/remote/adapter/decode.rs:504-528; src/tree/mirror/streaming/backend/local.rs:201-220)
- Class / severity / confidence: documentation / low / high
- Provenance: verified for reachability (decode.rs:508-527 rejects out-of-scope and non-ascending leaves as `LeafOutsideScope`/`LeafOrder` before assembly; `Local::assemble` at local.rs:201-220 builds only non-empty runs); assessed for the doc gap
- Seen by: prose; refutation: confirmed, plus its new item (the release-mode duplicate-path behavior lands on the `expect` at 312 because the ascending check at 280 is debug-only); history: no-rationale-found
- Owner-gated: no

The preconditions (non-empty, strictly ascending, bare leaves, each consumed once) are stated in running prose while the body `expect`s at 287 (consumed slot) and 308-309 (empty run), and in a release build a run with duplicate paths reaches `expect("distinct 32-byte paths diverge before the bottom")` at 312 rather than the stated precondition. Every other hazard in the partition (`Hash::leaf`/`branch`, `ErasedPrefix::push`/`pop`, `Leaf::value`) has a named `# Panics` section, and all of these panics are programmer-error only: the decoder and `Local::assemble` uphold the preconditions at the trust boundary.

Evidence:

       262	    /// `leaves` pairs each full 32-byte path with its bare (prefix-free)
       263	    /// leaf node, **strictly ascending by path**, every path sharing its
       264	    /// first `depth` bytes; the run must be non-empty, and each node is
       265	    /// consumed exactly once (the `Option` lets the recursion move nodes
       266	    /// out of a shared slice).

       308	        let first = leaves.first().expect("a leaf run is non-empty").0;
    ...
       312	            .expect("distinct 32-byte paths diverge before the bottom");

Resolution: add `# Panics` to both docstrings: on an empty run, on a `None` slot, and (release) on duplicate paths, each with the one-line reason the wire path cannot produce it (the decoder rejects non-ascending and out-of-scope leaves; `assemble` builds non-empty runs); note that ordering and bareness are debug-asserted. Acceptance: both `from_sorted_leaves` docstrings carry a `# Panics` section matching the `expect` sites.

### tree-typed-28: "door" is `before`'s maintainer coinage, used in rumors without a definition
- Where: src/tree/typed/untyped.rs:520-525 (related: src/tree/typed/untyped.rs:548-554, src/tree/typed/untyped/tests.rs:282)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (`grep -rn '\bdoor\b' src` finds these sites plus tree/tests.rs and a different sense in link/routed/header.rs; the word is dense in crates/before/src/laws.rs, absent from before's public docs, and defined nowhere in rumors)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (entered rumors through commit messages describing the span memo work)
- Owner-gated: no

"the trusted door", "the span door", and "a broken hull door" mean a constructor or entry point (`Span::at`, the `From` impls); a rumors maintainer reading `span`'s doc has to guess (writing rule: a coinage earns reuse only once it names an artifact the reader can find).

Evidence:

       522	    /// and a leaf's bounds coincide at its version, the coincident span
       523	    /// through the trusted door (`version <= version` holds
       524	    /// reflexively). Reading either forces the same memo

Resolution: name the thing: "via [`Span::at`], whose coincident form needs no validating comparison"; at 548 "where [`Span::dominance`]'s coincident case collapses ..."; in tests.rs:282 "a broken coincident-span constructor". Acceptance: `grep -rn '\bdoor\b' src/tree/typed` is empty.

### tree-typed-29: `dominance` is a one-line delegate at both layers for one caller, and the sibling module already inlines it
- Where: src/tree/typed/untyped.rs:535-557 (related: src/tree/typed/node.rs:184-195, src/tree/traverse/unknown.rs:48, src/tree/mirror/streaming/materialized/unknown.rs:66)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn '\.dominance(' src` finds two production callers, unknown.rs:48 through the wrapper and materialized/unknown.rs:66 as `node.span().dominance(known)`; `git show 4874f527 -- src/tree/typed/untyped.rs` shows the leaf arm collapsing to the delegate once `before` gained the coincident-span rung)
- Seen by: structure; refutation: confirmed; history: deliberate-but-expired (the wrapper had a real body, a leaf arm saving a decode, until 4874f527)
- Owner-gated: no

The untyped method is `self.span().dominance(probe)` under twenty lines documenting `Span::dominance`'s internals; the typed method delegates to it under twelve more. Two wrappers and thirty lines of prose about a dependency's method, for one call site, while the streaming counterpart spells the same classification directly.

Evidence:

       555	    pub fn dominance(&self, probe: &Version) -> Dominance {
       556	        self.span().dominance(probe)
       557	    }

    src/tree/traverse/unknown.rs:
        48	        match node.dominance(known) {

    src/tree/mirror/streaming/materialized/unknown.rs:
        66	    node.span().dominance(known)

Resolution: delete both `dominance` methods; change unknown.rs:48 to `match node.span().dominance(known)`. The one rumors-specific sentence (routing through `span` pays a leaf one decode per stream) moves onto `span`'s doc. Acceptance: `grep -rn 'fn dominance' src/tree/typed` is empty; both `unknown` modules call `span().dominance`.

### tree-typed-30: `Fan::from_iter`'s "every reassembly arrives ascending" claim is false for the commit path, which pays the sort
- Where: src/tree/typed/untyped/fan.rs:178-200 (related: src/tree/traverse/act.rs:92, 112, 129; src/tree/typed/node.rs:88-96)
- Class / severity / confidence: performance / low / high
- Provenance: verified by reading act.rs:86-129 (`sorted_by_key` makes `updated` ascending; `existing_children.remove(radix)` at 112; `updated.into_iter().chain(existing_children).collect()` at 129 is ascending only when every updated radix is below every untouched one: existing {1, 5} with an update at 5 yields [5, 1]); `git log -S'Every reassembly in the crate feeds pairs'` names f70f9559a ("Landed dark"), and the swap that wired act.rs to the fan (8f87ddd01) does not mention act.rs's chain order
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found (the claim was never checked against act.rs)
- Owner-gated: no

The doc states a performance contract (one pass for every crate reassembly) that the hottest reassembly, one per spine node per commit batch, violates: the concatenation at act.rs:129 is unsorted in general, so the fast check fails and the slow path runs a stable sort plus a second `SmallVec` (heap-allocated whenever the fan exceeds `FAN_INLINE` = 2) that rebuilds the entries even when no duplicate exists. Both inputs are individually ascending, so a two-way merge restores the one-pass path with a fixed sign.

Evidence:

       180	/// Later pairs displace earlier ones at the same radix, matching repeated
       181	/// [`insert`](Fan::insert). Every reassembly in the crate feeds pairs
       182	/// already strictly ascending and duplicate-free, which this recognizes in
       183	/// one pass; anything else pays one stable sort.

    src/tree/traverse/act.rs:
       112	            let existing_child = existing_children.remove(radix);
    ...
       128	        // Re-assemble: updated children + untouched existing children.
       129	        Node::branch(updated.into_iter().chain(existing_children).collect())

Resolution: in act.rs:129 merge the two ascending sequences by radix (`itertools::merge_by`, itertools already being used in act.rs; or `Children::insert` the updated children into `existing_children`, a binary-search insert), which makes the fan doc true; if act.rs stays as is, reword the doc to name it as the site that takes the sort path. Optionally skip the `deduped` rebuild when a post-sort adjacent scan finds no equal radixes. Acceptance: a fan/tests.rs test constructing the `updated ++ existing` interleaving that shows the fast path is taken from act.rs, or the doc reworded to match the code.

### tree-typed-31: `Fan::iter` hand-rolls a named `Iter<'a>` nothing outside fan.rs names, while `values` returns `impl Trait`
- Where: src/tree/typed/untyped/fan.rs:203-226 (related: src/tree/typed/untyped/fan.rs:162-175, src/tree/typed/node.rs:104)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (`grep -rn 'fan::Iter\b' src` finds nothing; `fan::IntoIter` is named at node.rs:104 and must stay; every consumer needs only `DoubleEndedIterator + ExactSizeIterator`, which `slice::Iter::map` preserves; the crate is edition 2024 so the RPIT lifetime capture is automatic)
- Seen by: structure; refutation: confirmed; history: no-rationale-found (both spellings were written together at f70f9559a)
- Owner-gated: no

Two styles for the same thing in one file, one of them twenty-four lines longer; the exact `size_hint` that `Hash::branch` sizes its buffer from is preserved by the adaptor.

Evidence:

       166	    pub fn iter(&self) -> Iter<'_> {
       167	        Iter {
       168	            inner: self.entries.iter(),
       169	        }
       170	    }
       171	
       172	    /// Iterate the children alone, in ascending radix order.
       173	    pub fn values(&self) -> impl DoubleEndedIterator<Item = &Node> + ExactSizeIterator {
       174	        self.entries.iter().map(|(_, child)| child)
       175	    }

Resolution: `pub fn iter(&self) -> impl DoubleEndedIterator<Item = (u8, &Node)> + ExactSizeIterator { self.entries.iter().map(|(radix, child)| (*radix, child)) }`; delete `Iter<'a>` and its three impls. Acceptance: fan.rs defines one named iterator type (`IntoIter`); `size_hint_is_exact_from_both_ends` passes.

### tree-typed-32: Local prose slips in the maintainer docs
- Where: src/tree/typed/untyped/iter.rs:1-6 (related: src/tree/typed/untyped/iter.rs:270, 287-288, 376-378; src/tree/typed/hash.rs:218-219; src/tree/typed/prefix.rs:23-24; src/tree/typed/untyped.rs:568-569; src/tree/typed/node.rs:385)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git log --follow --name-status -- src/tree/typed/untyped/iter.rs` shows `R100 src/tree/typed/untyped/node/iter.rs -> src/tree/typed/untyped/iter.rs`; untyped.rs:11 `mod iter;`; `git log -S'this replaces decoded'` names eb21c4d2d; `git log -S'struct Frozen'` names ef4239f87, renamed at 933505cde; `git show 9c73d7b46 -- src/tree/typed/untyped/iter.rs` shows the "32-byte" doubling arriving with the `Key` retirement's rewrap; the rest read)
- Seen by: structure ([9]), prose ([29]), correctness ([40]), perfapi ([54]); refutation: confirmed; history: iter.rs:5 deliberate-but-expired (true before the 48dc4323e move), untyped.rs:568-569 contradicts-hard-rule, the rest no-rationale-found; the "undercounts its walks" clause is disputed (the doc counts shells of the shared frontier engine, and `RangeOwned` has its own spine engine; omitting it is a completeness item)
- Owner-gated: no

Each is a small factual slip a reader trips on: the module doc names its parent `node` (the parent is `untyped`; `typed::node` is a different module) and omits `RangeOwned` from its listing sentence; `within`'s doc doubles "32-byte" across a line break; `RangeOwned`'s doc lacks a paragraph break at 287-288 so two paragraphs render as one; hash.rs:218 calls a `LazyLock` value "A compile-time constant"; prefix.rs:24 says an erased prefix's length "*is* the height" and then gives the complement; untyped.rs:568-569 is ungrammatical and refers to a replaced implementation ("the split folds this replaces"), which the hard rule sends to git; the scare-quoted "frozen" (iter.rs:270) and "Freeze" (node.rs:385) are the retired `Frozen` type's name surviving as vocabulary where "owned" already says it.

Evidence:

         1	//! Leaf iterators over the untyped tree: a shared frontier walk and its two
         2	//! shells, [`Iter`] (the unfiltered, exact-size walk) and [`Range`]
         3	//! (the walk filtered to a causal [`causally::Query`]).
         4	//!
         5	//! A child module of [`node`](super) so the walk can match on the parent's
         6	//! private [`Children`] variants and path-compression internals directly.

       376	    /// each leaf still reconstructs a full 32-byte
       377	    /// 32-byte path. `path.len()` plus the height of

    src/tree/typed/hash.rs:
       218	        // A compile-time constant: memoize it rather than re-hashing the
       219	        // four fixed bytes on every empty-root read.

    src/tree/typed/prefix.rs:
        23	/// A prefix with its height tag forgotten: the same accumulated path
        24	/// bytes, whose length *is* the height (`32 - height` bytes at `height`).

    src/tree/typed/untyped.rs:
       568	    ///   join legs sharing every operand decode — where the split folds
       569	    ///   this replaces decoded each version once per lattice direction.

Resolution: iter.rs:1-6: "Leaf walks over the untyped tree: a shared borrowing frontier engine beneath [`Iter`] and [`Range`], and the owned spine walk [`RangeOwned`]. A child module of [`untyped`](super) so the walks can match the parent's private `Children` variants directly." Delete one "32-byte" at 376-377; insert the blank `///` at 287-288; hash.rs:218: "A fixed value: compute it once, on first read, rather than re-hashing the four bytes on every empty-root read"; prefix.rs:24: "whose length determines the height (`32 - height` bytes at `height`)"; untyped.rs:568-569: a present-tense statement of the fused hull's cost (one decode per operand serving both lattice directions); drop the quoted "frozen" and "Freeze" for "owned". Acceptance: each cited line reads correctly; `grep -rn 'this replaces\|"frozen"' src/tree/typed` is empty.

### tree-typed-33: `Iter`'s doc names `unknown` and `Tree::join` as callback consumers of its order; neither takes a callback or uses `Iter`
- Where: src/tree/typed/untyped/iter.rs:171-174 (related: src/tree.rs:567, src/tree/traverse/unknown.rs:26, src/tree/traverse/join.rs:154-159, src/tree/traverse/act.rs:42)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (tree.rs:567 `pub fn join(&mut self, other: Tree<T>) -> bool`; unknown.rs:26 takes no callback; join.rs:154-159 walks `Children::iter` fans in lockstep and never names `Iter`; `git log -S'deterministic callback delivery'` names f7257af49, when `Tree::join` took `on_recv`/`on_send`; 262568f9e "retire the observation callbacks")
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired
- Owner-gated: no

The parenthetical describes a dependency that was retired with the observation callbacks; it sends a maintainer looking for a callback-delivering join that does not exist (Principle 5).

Evidence:

       171	/// it. (The public observers on [`Rumors`](crate::Rumors) still promise
       172	/// nothing about order, but [`unknown`](crate::tree::traverse::unknown)
       173	/// and `Tree::join` lean on the ascending forward order for their own
       174	/// deterministic callback delivery.)

Resolution: delete the parenthetical, or replace it with the consumer that actually relies on `Iter`'s order, if one does. Acceptance: every consumer the `Iter` doc names calls `Tree::iter` or `Iter::root`, or is removed from the sentence.

### tree-typed-34: The owned walk's cursor is a `u16` with a 256 sentinel and two `as` casts where `Option<u8>` carries the same state
- Where: src/tree/typed/untyped/iter.rs:311-312 (related: src/tree/typed/untyped/iter.rs:411-413, 429)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read)
- Seen by: structure; refutation: reframed (the `step(back: bool)` half is a documented private parameter, taste only); history: no-rationale-found
- Owner-gated: no

`Level.next` encodes "next radix to probe, or exhausted" with `256` as the sentinel, needing `level.next <= u8::MAX as u16` and `radix as u16 + 1`. `Option<u8>` says it without a sentinel or casts: `None` is exhausted, `Some(r)` probes `successor(r)`, and the advance is `radix.checked_add(1)`.

Evidence:

       311	    /// The smallest child radix not yet visited; `256` means exhausted.
       312	    next: u16,

       411	                        Children::Branch { children, .. } if level.next <= u8::MAX as u16 => {
       412	                            children
       413	                                .successor(level.next as u8)
    ...
       429	                            level.next = radix as u16 + 1;

Resolution: `next: Option<u8>` initialised `Some(0)`; the probe becomes `level.next.and_then(|at| children.successor(at))`, the advance `level.next = radix.checked_add(1)`. Acceptance: no `as u16`/`as u8` in iter.rs; the sentinel comment is gone.

### tree-typed-35: `RangeOwned` has an inherent `next` but no `Iterator` impl, so `Node::leaves` wraps it in `iter::from_fn` and observers hand-drive it
- Where: src/tree/typed/untyped/iter.rs:393-395 (related: src/tree/typed/node.rs:272-281, src/rumors/causal.rs:99, src/rumors/unordered.rs:211)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (callers by grep; `git log -S'Iterator for RangeOwned'` and for its former names is empty; the one nearby rationale for an inherent `next`, ef4239f87's "no standard lending-iterator trait", is about the `Messages` observer that lends borrows, not this walk, whose item is owned)
- Seen by: structure ([6]), correctness ([38]), perfapi ([50]); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The method has `Iterator::next`'s exact shape and an owned item; the borrowing `Range` in the same file implements `Iterator`. Without the trait, `Node::leaves` builds a `std::iter::from_fn` closure to get `.map`, and the subscription drains call `.next()` in `while let`/`if let` by hand. Implementing the trait removes the shim and gives every caller the adaptors; `Send` and the constant-size state are unaffected, and `size_hint` can report `(0, None)`.

Evidence:

       393	    /// Advance to the next passing leaf. The same classification as the
       394	    /// borrowing walk, with the leaf handed out by value.
       395	    pub(crate) fn next(&mut self) -> Option<([u8; 32], Leaf)> {

    src/tree/typed/node.rs:
       274	        std::iter::from_fn(move || {
       275	            walk.next().map(|(key, leaf)| {
       276	                (
       277	                    super::Prefix::from(key),
       278	                    Node::from_untyped(leaf.into_node()),
       279	                )
       280	            })
       281	        })

Resolution: `impl<P: Polarity> Iterator for RangeOwned<P> { type Item = ([u8; 32], Leaf); fn next(&mut self) -> Option<Self::Item> { /* body */ } }`, delete the inherent method, and reduce `Node::leaves` to `RangeOwned::within(...).map(|(key, leaf)| (Prefix::from(key), Node::from_untyped(leaf.into_node())))`. Acceptance: `iter::from_fn` is gone from node.rs; causal.rs and unordered.rs compile unchanged (method syntax resolves to the trait); `range_and_freeze_match_the_naive_filter` passes.

## Positives

- The `PhantomData<fn() -> H>` auto-trait shortcut is explained where the chain originates (height.rs:6-12), extended at height.rs:16-21 to why `S<T>`'s impls are hand-rolled (a derive would reintroduce the `T: Trait` bound), and pinned for size and alignment in height/tests.rs. This is how a non-obvious design decision should be documented; finding 13 is only about the second copy.
- `Hash::branch`'s canonicity section (hash.rs:143-156) locates hash agreement in canonical shape rather than in the hash construction, and `every_virtual_level_hashes_canonically` (untyped/tests.rs:470-485) pins exactly that claim at every virtual level, mid-spine included, against the from-scratch bulk constructor.
- hash/tests.rs `prefix_len_separates_boundary_shifts` (71-115) constructs the collision pair the length tag exists to prevent and asserts its premise (the untagged preimages coincide) before asserting the conclusion: a model adversarial pin. `saturated_fan_count_uses_the_high_byte` pins the one reason the count is a `u16`.
- The fan differential suite (fan/tests.rs:46-97, 107-132) checks handle identity via `ptr_eq` in both iteration directions, all 256 point lookups, and every successor probe against a `BTreeMap` oracle after every step; `size_hint` exactness is checked under consumption from both ends. `fan_is_forty_bytes` and `node_inner_stays_within_budget` make per-node cost a reviewed number with the growth rule stated in the testdoc.
- `Node::from_inner` (untyped.rs:190-199) is the single construction funnel that makes the test-only `census` an exact residency count, and its doc names what it serves outside itself: checking the session window's in-flight-reference bound against reality.
- `from_sorted_leaves`'s strict-ascent precondition is enforced at the trust boundary (decode.rs:508-527 rejects non-ascending and out-of-scope leaves before assembly), so its debug asserts are correctly scoped as programmer-error guards; the `Option` slots let the recursion move nodes out of a shared slice without cloning, and `branch_at` strictly increasing toward 32 bounds the recursion structurally.
- `RangeOwned`'s constant-state spine walk (one `Level` per materialized branch, siblings never enumerated, `successor` by binary search) is documented at the type with the memory argument the session window relies on; `Walk::step`'s two-ended re-push order (iter.rs:129-133) is stated precisely enough to check by reading.
- The `NodeInner`/`Children` field docs state the memo-invalidation invariants exactly (hash resets on any prefix or children change; bounds and `version_bytes` reset on children change only), and every mutation site (`into_children`, `beneath`, `from_sorted_leaves`) carries the matching reset with a comment saying why.
- `# Panics` sections with one-line reasons at `Hash::leaf`/`branch`, `ErasedPrefix::push`/`pop`, and `Leaf::value`; `ErasedPrefix::assume`'s doc traces why a cross-height re-tag is programmer error and never peer input.
- fan.rs's module doc argues the data-structure choice from the population shape and says where structural sharing actually lives and why `Fan` is deliberately not a persistent map: a design defended once, in writing, at the type.
- The `PartialEq` comment at untyped.rs:686-693 explains why hash equality is content equality and names the one scenario in which it would not be, tying it to the linearity invariant.

## Open questions for Finch

- `Hash` derives `Default` (the all-zero digest); its only users are ten test sites under mirror/streaming/remote, and `root_hash`'s comment warns that the empty tree must not hash as that value. Recommendation: drop the derive and spell `Hash([0; MERKLE_HASH_LEN])` in the tests, so a production `Default::default()` on a digest cannot compile.
- `ErasedPrefix::assume` checks the length-vs-height witness only in debug builds (prefix.rs:45-49). In release, a cross-height re-tag to `Prefix<Z>` yields fewer than 32 bytes and `From<Prefix> for Path` (`into_inner`, prefix.rs:113-117) zero-fills the tail, a misplaced leaf rather than a crash. Programmer-error only, and all tests run in debug. Recommendation: promote the O(1) witness to a release `assert!`; the check is cheap and the failure mode is silent misplacement.
- `Root`'s two-row `S<` ruler (height.rs:127-133): pedagogy worth keeping, or `pub type Root = H32;`? Recommendation: the alias plus `const _: () = assert!(Root::HEIGHT == PATH_LEN);`; if the ruler stays, the assert alone closes the miscount (finding 12).
- Which copy of the `PhantomData<fn() -> H>` argument to keep (finding 13): the owner last re-justified the node.rs copy at 8f87ddd01. Recommendation: move that wording onto `S` in height.rs, where the chain originates, and point the three types at it.
- The supply path's bare-leaf rebuild (`Leaf::into_node`, iter.rs:345-362) costs one `Arc<NodeInner>` allocation, a `Version` clone, and a `Message` clone per supplied leaf, to uphold `Node<Z>`'s bare-leaf invariant that only `from_sorted_leaves` asserts (untyped.rs:288-291) and the supply encoder never needs (encode.rs:204-206 reads `span()` and `message()` only). The rationale is stated in code and holds, so it is not filed as a finding. Question: is the invariant worth the per-leaf allocation on the wire path, or should `Backend::leaves`'s item be a leaf handle distinct from `Node<Z>` (touches the `Backend` trait)? Recommendation: measure with `benches/gossip_fixed.rs` on a supply-heavy fixture before deciding.
- `Prefix<H: Height = Z>` defaults to leaf height while `Path<H: Height = Root>` defaults to root height (prefix.rs:18, path.rs:15). Both are internal; the asymmetry surprises a maintainer. Recommendation: document why leaf-height prefixes are the common case (they are the leaf keys), or drop the `Prefix` default.
- Why does the gate's `-D warnings` not flag the caller-less typed `compressed_prefix_len` (node.rs:236-242)? I could not compile. If it is a dead-code-lint gap for `pub` items in a private subtree, finding 1's `unreachable_pub` plus `pub(crate)` convention is the fix; if something reaches it, finding 14's dead-wrapper claim should be re-checked by compiling.
- Handoffs out of partition: reconciliation.rs:164-165 ("a leaf's digest commits its address and its version") carries the same drift as finding 5 in the public doc of record; tree.rs:36 ("the node serializer relies on") is the same ghost as finding 19; tree.rs:277 clones the `Root` pair per hash read (finding 18).

## Dropped

- [42] `MAX_BRANCHING` never binds: deliberate and documented. The constant is the alphabet guard for `btree_set(any::<u8>(), 1..=max_n)` (a budget above 256 would otherwise ask for more distinct bytes than exist), and its doc hedges "subject to the leaf budget". The residual gap (no 256-child fan through `Node::hash` against `reference_hash`) is below the bar: the saturated count is pinned at `Hash::branch` and the fan's spilled storage by the differential suite.
- [44] supply walk rebuilds a bare leaf per compressed leaf: the rationale is stated in code (iter.rs:345-351) and holds; the encode.rs:197-200 comment is accurate about the encoder's own reads. Converted to the fifth open question.
- [52] `Path::pop` and `Prefix::pop` return their tuples in opposite orders: refuted. `Path::pop` removes the leading byte and returns `(byte, rest)`, the shape of `split_first`; the two prefix `pop`s remove the trailing byte and return `(rest, byte)`, the shape of `split_last`. The byte sits in the tuple where it sits in the path.
- [32] the bench magnitude in `Hash::branch`'s comment: reopens the sealed R30 ruling (.agent-notes/2026-07-23-review-link-transport: "bench named in the claim", the factor re-denominated for SHA3 at 4f18c347) with no new evidence. Its "17-byte" half is kept as finding 9.
- [14] the `step(back: bool)` half: a documented private parameter; taste, not a finding. The sentinel half is finding 34.
- [28] untyped.rs:237 `expect("non-empty prefix")`: sits directly under its guard at 230 and names the establishing fact; not a defect. The other two messages are finding 22.
- [33] the "names the tree" half: `crate::reconciliation` is public and describes the tree, so the concept is already established for the user. The fragment half is finding 2.
- [9] the "undercounts its walks" clause: the module doc counts shells of the shared frontier engine, and `RangeOwned` has its own spine engine; kept only as a completeness item within finding 32.
- Refutation's new item on "seam" (node.rs:151 "The streaming mirror's erasure seam"): the streaming layer's established name for `erased`, used across the crate; whether it is defined where a reader first meets it belongs to the streaming partitions.
- Duplicates merged: [20], [34], [47] and the refutation's "node serializer" item into finding 19; [36] into 5; [16] into 3; [51] and [4] into 12; [48] into 16; [38] and [50] into 35; [40] and [54] into 32 and 22; [12] into 14; [30] into 9; the refutation's release-mode duplicate-path item into 27.
