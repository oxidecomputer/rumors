# Partition streaming-backend-window: Streaming backends (local, adversarial), channels, leaf conversion, the window, and the failing/faulting test doubles

## Partition summary

This partition is the streaming mirror's materiality and sizing layer. `backend.rs` defines what a session node is and what holding one costs: the `Backend` trait (erase/assume, `node_bytes`, `parent`, `children`, and the two bulk overrides `leaves`/`assemble`), the `Node`/`ErasedNode`/`Leaf` observation traits, the `NodeStream` alias, and the generic `Root<B>`. `backend/local.rs` is the only production implementation, mapping the traits onto the crate's `Arc`-handled typed tree and overriding the bulk paths so path-compressed spines are walked and rebuilt without per-virtual-level work; under `cfg(test)` every `Local` operation is wrapped by the poll scheduler in `local/adversarial.rs`. `convert.rs` is the level-by-level default chain those overrides are held equivalent to (explode to leaves through `children`, fold back up through `parent`). `window.rs` turns one byte budget plus the two greetings' set sizes and version-size bounds into per-height channel capacities, using integer Chernoff and Poisson-type occupancy envelopes and a binary search over a saturating charge; its constants are pinned by recomputation in `window/tests.rs`. `channel.rs` names the session's bounded edges and swaps in an instrumented, schedulable Tokio wrapper (`channel/instrumented.rs`) under test. `testing/failing.rs` and `testing/faulting.rs` are the fault-injecting backend and protocol decorators.

I read all thirteen files in full, 3588 lines. Production code is `backend.rs` (414), `backend/local.rs` (244), `convert.rs` (146), `window.rs` (763), and the `cfg(not(test))` half of `channel.rs` (90). Test code is `backend/local/adversarial.rs` (157), `backend/local/tests.rs` (188), `channel/instrumented.rs` (318), `convert/tests.rs` (104), `window/tests.rs` (414), `testing.rs` (13), `testing/failing.rs` (301), and `testing/faulting.rs` (436).

The production code is in good shape. Every panic site I traced is either provably unreachable or a backend-contract enforcement with its proof stated; wire input cannot reach `from_sorted_leaves` unsorted or uncontained because the decoder rejects those shapes as session errors first; the window solve is total and floor-preserving under saturating arithmetic; the integer-envelope inequalities check out (I re-derived `bernstein`, `small_mean_quantile`, and the bit-length figures with a replica); and every quoted figure the prose carries is pinned by a committed recomputation. The differential discipline is applied where it matters most: `Local`'s bulk overrides are held to the default chain by observational-equivalence proptests over both deep-spine and wide-fan shapes.

The findings are mostly residue and drift. Three retirements left prose behind: the two-backend session design (convert.rs's module doc describes a converter that no longer exists; `Root`'s manual `Clone` cites a `T` parameter erasure removed; a pass-through generator in `assemble` is the shell of a removed watermark filter), a scheduling axis that was never wired (`Role`), and one testdoc whose figures were re-pinned beneath it without the prose moving. Two verification gaps stand out: the integer-envelope dominance that underwrites the 2^-40 claim is certified only by an example no recipe runs, and that example by its own header checks a different family than the shipped pair-based functions; and a 22-line comment proves the leaf-request term identically zero where a one-line proptest could pin it. The rest is test-scaffold duplication, hand-counted byte constants beside siblings whose docs forbid hand counting, and altitude and dialect nits in otherwise careful prose.

## Findings

### streaming-backend-window-1: `Backend` family documented for an implementer the crate cannot admit
- Where: src/tree/mirror/streaming/backend.rs:1-20 (related: src/tree/mirror/streaming/backend.rs:158-162, 195-197; src/lib.rs:322, 328-345; src/conformance.rs:16-19; src/conformance/backend.rs:41-47)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: `mod tree;` is private at lib.rs:322; lib.rs:328-345 re-exports no member of the `Backend`/`Node`/`Leaf`/`ErasedNode`/`Root`/`Convert` family; `conformance::backend` is `#[cfg(test)] pub(crate)` at conformance.rs:18-19)
- Seen by: prose, perfapi; refutation: confirmed; history: already-known (the sync-budget record, `.agent-notes/2026-07-22-sync-budget/sync-budget.md:577-581`, states the trait is crate-internal today and goes public with the conformance suite)
- Owner-gated: yes: publishing the seam reopens a recorded deferral; the one-sentence status fix does not

The module doc and the trait docs address a third-party storage implementer ("a storage engine of its own"; the conformance suite "is how an implementation proves its account"), yet no member of the family is reachable outside the crate, and only `conformance/backend.rs` says so. Documentation altitude: prose written for an audience the API does not reach, in the one module a maintainer adding a backend reads first.

Evidence:

         3	//! A [`Backend`] decides what a tree node physically is — a value carried
         4	//! in the node's handle, or a reference into storage the backend owns —
         5	//! and the streaming protocol is generic over that decision: an
         6	//! implementation may hold its tree entirely in memory or in a storage
         7	//! engine of its own, and the session schedule is identical either way.
    ...
        18	//! backend conformance suite (`crate::conformance::backend`, compiled as
        19	//! this crate's own test gate; see [`crate::conformance`]) is how an
        20	//! implementation proves its account.

Resolution: Add one sentence to the module doc stating the boundary is crate-internal and `Local` is the sole production implementation (the wording at conformance/backend.rs:43-47 already exists). Keep the implementer guidance that serves the maintainer adding a backend. When the seam is published, do the trait-shape pass then: `parent` takes an owned `Vec`, `assemble`'s signature names the `pub(crate)` alias `BoxNodeStream`, and `Error: Send + 'static` carries no `Debug` or `std::error::Error` bound. Acceptance: backend.rs states the seam's status in its module doc; no sentence in the family addresses an external implementer without that framing.

### streaming-backend-window-2: Bare `pub` on crate-internal items beside `pub(crate)` in the same layer
- Where: src/tree/mirror/streaming/backend.rs:46-46 (related: src/tree/mirror/streaming/backend.rs:360, 389; src/tree/mirror/streaming/window.rs:132, 286; src/tree/mirror/streaming/channel.rs:10; src/tree/mirror/streaming/backend/local.rs:92; src/lib.rs:322)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep: no `unreachable_pub` in src/lib.rs, Cargo.toml, or .cargo/; `mod tree;` private)
- Seen by: structure; refutation: confirmed; history: no-rationale-found (the sync-budget note explains why nothing is exported, not why items say `pub`)
- Owner-gated: yes: a crate-wide convention choice

`Backend`, `Node`, `ErasedNode`, `Leaf`, `NodeStream`, `Root`, `Local`, `QueueKind`, and the instrumented types are `pub`, while `BoxNodeStream`, `Root::len`, `Window`, and `FAN` in the same files are `pub(crate)`. Under a private `mod tree` both spellings mean the same thing, so the keyword carries no information about the API surface.

Evidence:

        46	pub trait Backend: Clone + Send + Sync + 'static
    ...
       360	pub(crate) type BoxNodeStream<'a, B, H> = Pin<Box<dyn NodeStream<B, H> + 'a>>;

Resolution: Decide once: enable `#![warn(unreachable_pub)]` and let the compiler mark every effectively-private `pub` as `pub(crate)`, or state in AGENTS.md that bare `pub` inside private modules is the crate's style and drop the scattered `pub(crate)` where it adds nothing. Acceptance: either the lint is on and clean, or one convention is written down and applied.

### streaming-backend-window-3: Default-dialect tells: "seam", "knob", "story" as jargon, moralized "honest"/"genuine"/"real", and the "X, not Y:" opener clustered in window.rs
- Where: src/tree/mirror/streaming/backend.rs:67-67 (related: backend.rs:130, 183, 232, 251, 273; backend/local.rs:108, 199; backend/local/tests.rs:5, 57, 75; backend/local/adversarial.rs:25; channel/instrumented.rs:215; testing.rs:3; window.rs:14, 64, 68, 86, 114, 210, 232, 242, 269, 574; window/tests.rs:21, 165, 188)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -nwiE 'knob|seam|genuine|genuinely|story|really'` and `grep -nwE 'honest|real'` over the thirteen files; the model-of-record uses of "honest" at window.rs:26, 36, 680 and throughout faulting.rs are correctly excluded)
- Seen by: prose; refutation: confirmed; history: no-rationale-found ("knob" is the owner's working vocabulary in commit messages and the sync-budget note; the crate has precedent for a wholesale purge of one dialect word, "mint")
- Owner-gated: no

Three habits recur. Unanchored metaphors as jargon: "seam" for an override point (backend.rs:183, 251; local/tests.rs:57, 75), "knob" (window.rs:14), "story" (backend.rs:273; local.rs:199). Moralized code outside the model's "honest peer" sense: "genuinely distinct" (backend.rs:67), "one real child" (backend.rs:130), "every honest meet/join pair" (backend.rs:232), "keeps them honest" (local/tests.rs:5), "keeps `-D warnings` honest" (adversarial.rs:25, instrumented.rs:215), "really is pointer-sized" (local.rs:108), "a genuine semantic violation" (testing.rs:3). The antithesis-plus-colon opener nine times in window.rs alone (64, 68, 86, 114, 210, 232, 242, 269, 574) and twice in window/tests.rs (21, 188). Each instance is harmless; the density makes the prose read as a voice rather than a specification.

Evidence:

        67	    /// genuinely distinct per-height representations supplies a sum of
    ...
    (window.rs)
       232	/// An anchor, not an input: nothing derives from it. The closed form
    ...
       269	/// Chosen, not derived: a round policy default. What any budget buys —

Resolution: "seam" to "override point" or "boundary"; "knob" to "setting"; "story" to "argument" or cut; non-model "honest" to "correctly computed" or cut; "genuine", "real", "really" cut or made concrete (backend.rs:130 means a `Some` child). Keep at most one or two colon-fronted openers per module and rewrite the rest in default word order. Acceptance: the greps return nothing outside the model-of-record sense; window.rs carries at most two "X, not Y:" openers.

### streaming-backend-window-4: backend.rs restates the same hazard three times and two phrases twice; a `///` doc sits on an anonymous `const`
- Where: src/tree/mirror/streaming/backend.rs:115-117 (related: backend.rs:10, 15-17, 45, 133-135, 195-197, 282-283, 323; backend/local.rs:107-110)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (the restatements accreted across aabdcea0, 206c288a, and two docs passes)
- Owner-gated: no

The memory-versus-latency hazard of an underpriced node is stated at the module doc (15-17), at `node_bytes` (115-117), and at `version_bytes` (282-283); "custody costs it nothing new" appears verbatim at 10 and 323; the "convicts a violating ... before a live session can meet it" sentence at 133-135 and 195-197. `Backend`'s first sentence (45) describes a value's clone cost rather than the trait's role. In local.rs, a `///` doc decorates `const _: ()`, which rustdoc never renders.

Evidence:

       115	    /// suite). Everywhere else in the budget derivation, mis-estimation
       116	    /// costs latency; an underpriced node is the one input that breaches
       117	    /// the *memory* envelope instead.
    ...
        15	//! The obligation is sharp because its failure mode is the odd one out:
        16	//! everywhere else in the budget derivation a mis-estimate costs latency,
        17	//! while an underpriced node breaches the *memory* envelope instead. The
    (local.rs)
       108	/// The handle really is pointer-sized: the window's per-reference price
       109	/// rests on it.
       110	const _: () = assert!(std::mem::size_of::<typed::Node<Z>>() == std::mem::size_of::<*const ()>());

Resolution: Keep the hazard at `node_bytes` only; in the module doc, one clause and a link. Drop the second "custody costs it nothing new" and the second conformance sentence. Open `Backend` with what it is ("Storage for a session's tree nodes; a value is a cheap cloneable handle to it."). Make local.rs:108-109 a `//` comment. Acceptance: each of the three phrases appears once in backend.rs; the `const _` carries a `//` comment.

### streaming-backend-window-5: Manual `Clone` for `Root<B>` justified by a `T: Clone` bound that erasure removed; `T` also named at line 51
- Where: src/tree/mirror/streaming/backend.rs:374-383 (related: src/tree/mirror/streaming/backend.rs:50-52, 367-372; src/message.rs:48)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 2fd590d57:src/tree/mirror/streaming/backend.rs` has `pub struct Root<B, T>` at line 142 with the identical comment at 152; `git show 48bc31df -- backend.rs` retypes `impl<B, T> Clone for Root<B, T>` to `impl<B: Backend<Node<Z>: Leaf>> Clone for Root<B>` and leaves the comment; the derive-compiles claim is assessed from the GAT bound at line 52, not compiled)
- Seen by: structure, prose; refutation: confirmed; history: deliberate-but-expired (true at 2fd590d5, expired at 48bc31df)
- Owner-gated: no

`Root<B>` has one type parameter, but the comment says the derive "would demand `T: Clone`". `Backend: Clone` (46) and `Node<H>: ... + Clone` (52) supply every bound `#[derive(Clone)]` needs for `ceiling: Version` and `root: Option<B::Node<height::Root>>`, so the manual impl and its rationale are leftovers. Line 51's "decode as `T` at the wire boundary" names the same erased parameter; `Message` is not generic. AGENTS.md hard rule: nothing in the tree refers to code that no longer exists.

Evidence:

       374	// Manual because the derive would demand `T: Clone`; nodes are cloneable
       375	// handles regardless of the message type they carry.
       376	impl<B: Backend<Node<Z>: Leaf>> Clone for Root<B> {
    ...
        50	    /// The type of nodes, indexed by height `H`; leaf payloads are
        51	    /// erased in storage and decode as `T` at the wire boundary.

Resolution: Replace the manual impl with `#[derive(Debug, Clone)]` on `Root<B>` and delete the comment; if the derive fails for a reason other than `T`, state that reason at the impl. At line 51, drop the clause or name what the wire produces ("decode as the peer's message type at the wire boundary"). Acceptance: no bare `T` in backend.rs prose; `Root<B>` is `Clone` by derive, or by a manual impl whose comment is true of today's code.

### streaming-backend-window-6: `Root<B>::max_version_bytes` narrates the in-memory backend's memoization at a backend-generic accessor
- Where: src/tree/mirror/streaming/backend.rs:396-407 (related: src/tree/typed/untyped.rs:400-421; src/tree/mirror/streaming/backend.rs:262-263)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read untyped.rs:400-421, which already carries the memo paragraph at the typed node)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (written at 206c288a; the same mechanism is documented on the typed node)
- Owner-gated: no

The accessor is generic over `B`, but its doc describes `Local`'s lazy memo ("every branch's bounds memo is forced tree-wide ... shared through the node handles across snapshots"). For a backend keeping `version_bytes` as a stored field, which `Node::version_bytes`'s doc at 262-263 invites, the paragraph is false. Documentation respects the abstraction boundary of the item it documents.

Evidence:

       400	    /// The root node's [`version_bytes`](Node::version_bytes) aggregate
       401	    /// — leaf versions and every interior ceiling and floor — or zero
       402	    /// when empty. The first read materializes it: every branch's
       403	    /// bounds memo is forced tree-wide, `O(#branches)` bound
       404	    /// folds, once per tree lineage — the memos are shared through the
       405	    /// node handles across snapshots, and a mutation invalidates only
       406	    /// its own spine. A fully converged pair pays this once, at
       407	    /// greeting time.

Resolution: Cut to the first sentence plus what the greeting carries; if the greeting-time cost matters to the maintainer, one clause pointing at `typed::Node::version_bytes`. Acceptance: the doc makes no claim that depends on `B = Local`.

### streaming-backend-window-7: Em-dashes in `//` comments (fourteen lines)
- Where: src/tree/mirror/streaming/backend/local.rs:44-44 (related: backend/local.rs:182; window.rs:395, 396, 402, 404, 410, 411, 435, 550, 561, 565, 568, 574)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -nE '^[[:space:]]*//[^/!].*—'` over the thirteen files returns exactly these fourteen lines; `///` and `//!` lines are rendered prose and exempt)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (the recorded house rule, prior review R76, covers assert messages; the comment-side preference is in the owner's global doctrine, not AGENTS.md)
- Owner-gated: no

The convention for code comments is the spaced double-hyphen or a colon; em-dashes belong to rendered prose.

Evidence:

        44	    // obligates is carried by construction — no per-read validation
    (window.rs)
       402	        // charged once (the level's queues — the walk's query and
       403	        // resolution queues and the proxy's flushed-question and
       404	        // next-scope queues — hold overlapping views of the same

Resolution: Replace each with ` -- `, a colon, or a semicolon as the sentence wants. Acceptance: the grep returns nothing under the partition.

### streaming-backend-window-8: `Local::node_bytes` exists twice: an inherent fn and a trait impl that delegates to it
- Where: src/tree/mirror/streaming/backend/local.rs:103-129 (related: src/testing.rs:229, 330)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (grep: the path form `Local::node_bytes` is used only at testing.rs:229 and :330)
- Seen by: structure; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The inherent `pub(crate) fn node_bytes` carries the doc and the body; the `Backend` impl delegates. The inherent one exists so the path resolves without `Backend` in scope at two test sites. One price, two definitions.

Evidence:

       103	    pub(crate) fn node_bytes(_children: usize, _version_bound: usize) -> usize {
       104	        std::mem::size_of::<typed::Node<Z>>()
       105	    }
    ...
       127	    fn node_bytes(children: usize, version_bound: usize) -> usize {
       128	        Local::node_bytes(children, version_bound)
       129	    }

Resolution: Move the doc onto the `Backend` impl's `node_bytes`, delete the inherent fn, and have testing.rs write `<Local as Backend>::node_bytes`. Acceptance: a single `node_bytes` on `Local`; testing.rs compiles.

### streaming-backend-window-9: `Local::children` deep-clones the shared node through `into_children` to yield its children
- Where: src/tree/mirror/streaming/backend/local.rs:136-141 (related: src/tree/typed/untyped.rs:164-187, 229-258; src/tree/typed/untyped/fan.rs:135-145; src/peer/gossip.rs:695, 1143)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read `into_children`: `Arc::make_mut` in both arms; `Children::clone` copies `bounds`, `version_bytes`, and the `Fan`; the session root is `inner.tree.clone()` handed to `start(Local, root.into())`, so every handle a session explodes is shared with the live tree; no allocation count was run)
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found (the latency note profiled `Arc::make_mut` under the conversion machinery; the fix at 88a76a71 addressed `leaves`/`assemble`, not the walk's `children`)
- Owner-gated: no

`parent.into_children()` reaches the fan through `Arc::make_mut`, which on a shared handle clones the whole `NodeInner` (bounds with two owned `Version`s, the memos, the fan, the prefix) and then `mem::take`s the fan out of the fresh copy, dropping the rest. Every disputed parent the walk explodes pays an allocation, two `Version` clones, and a fan heap allocation past two inline entries, all freed immediately. This is strict deletion of redundant work with a fixed sign; the doctrine says construct and measure. Denominator: per exploded parent.

Evidence:

       136	        let children = stream::iter(
       137	            parent
       138	                .into_children()
       139	                .into_iter()
       140	                .map(move |(radix, child)| Ok((prefix.push(radix), child))),
       141	        );

Resolution: Give the in-memory backend a non-copying child cursor for uncompressed branches: hold the parent handle and resume with `Fan::successor(radix)` (fan.rs:140-145, already documented as the resume point of a suspended ascending walk), cloning one child handle per step; keep the `into_children` path for path-compressed nodes, where popping a prefix byte needs a new node. Measure with a `stats_alloc` meter or the census counters on a disputed-fan corpus at the parent commit and after. Acceptance: a committed meter shows no `NodeInner` allocation per `Local::children` call on a shared uncompressed branch; the `local/tests.rs` equivalence proptests and the join-oracle suite stay green.

### streaming-backend-window-10: `Local::assemble`'s comment asserts the memory story is unchanged without pricing the run-proportional transient it introduces
- Where: src/tree/mirror/streaming/backend/local.rs:195-200 (related: src/tree/mirror/streaming/remote/adapter.rs:56-67; src/tree/mirror/streaming/remote/adapter/decode.rs:356-358; src/peer.rs:351-369; src/tree/typed/node.rs:301-308)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: a run is one supplied node's whole leaf set per adapter.rs:44-54 and encode.rs:185; each buffered entry is a 34-byte prefix padded beside an 8-byte handle, then re-collected at node.rs:301-308 into 40-byte entries; peer.rs:358-360 excludes the replica from the budget)
- Seen by: correctness (as a medium correctness finding); refutation: reframed to low (no committed promise is falsified: the budget's documented scope excludes replica storage, and the transient is a bounded fraction of the replica growth the same run causes); history: deliberate-and-holds for the buffering (88a76a71), but the memory clause was never measured, and the cbor review's memory check (`.agent-notes/2026-08-20-cbor-wire-review/REVIEW.md:976-979`) overlooked this buffer
- Owner-gated: yes: choosing between pricing the transient in prose and rebuilding the bulk path incrementally is the owner's call

The default `Convert::assemble` holds one open radix group per level. The override accumulates every leaf of a maximal same-prefix run before building, so for an early supply or a root-child answer the transient is proportional to the supplied subtree, about 88 bytes of bookkeeping per leaf beyond the handles, reclaimed when the run's node is built. The comment says the memory story is "unchanged" and the decoder's comment says "never a decoded vector of leaves"; both are true at the wire layer and neither quantifies this buffer, and nothing prices it (the supply-decode envelope prices `FAN + 1` leaves per stream). A maintainer reading either comment would conclude no run-proportional state exists.

Evidence:

       195	        // The bulk counterpart of `leaves`: buffer each maximal
       196	        // same-prefix run and build its subtree in one pass, rather than
       197	        // folding it up one virtual level at a time. The buffered run is
       198	        // transient state for a subtree this in-memory backend is about to
       199	        // hold whole anyway, so the streaming session's memory story is
       200	        // unchanged.

Resolution: Either (a) restate the comment with the quantity: the override trades the default chain's fan-per-level bookkeeping for one proportional to the run (state the per-leaf figure as a `size_of` expression, and that it is reclaimed at run end and falls under the budget's "replica itself" exclusion), and add one clause to decode.rs:356-358 saying the backend's own run buffer is the backend's custody; or (b) make the bulk build incremental with a radix-stack builder that emits each compressed subtree when its prefix closes. Acceptance: for (a), both comments describe the run buffer truthfully; for (b), a census-ledger measurement of one large supply run shows peak transient bounded by fan times depth rather than growing with run length.

### streaming-backend-window-11: `Local::assemble` buffers each run twice before building the subtree
- Where: src/tree/mirror/streaming/backend/local.rs:204-219 (related: src/tree/typed/node.rs:291-310; src/tree/mirror/streaming/remote/adapter/decode.rs:88, 408)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read node.rs:301-308: `from_sorted_leaves` immediately re-collects the run into `Vec<([u8; 32], Option<untyped::Node>)>` because `untyped::Node::from_sorted_leaves(depth, &mut entries)` wants that shape)
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found (88a76a71 discusses the virtual-level saving only)
- Owner-gated: no

Each run is accumulated as `Vec<(Prefix<Z>, typed::Node<Z>)>` and then handed to `from_sorted_leaves`, which allocates a second same-length vector in the builder's entry shape. One extra allocation per run plus a per-leaf move on the decode path; strict deletion, fixed sign.

Evidence:

       204	            let mut run: Vec<(Prefix<Z>, typed::Node<Z>)> = Vec::new();
       205	            while let Some(item) = leaves.next().await {
       206	                let (prefix, leaf) = item?;
       207	                let target = Prefix::<H>::containing(&Path::from(prefix));
    ...
       213	                        typed::Node::from_sorted_leaves(&finished, mem::take(&mut run)),

Resolution: Accumulate the builder's own entry shape directly (push `([u8; 32], Option<untyped::Node>)`, or let `typed::Node::from_sorted_leaves` take the run by `&mut [..]` and build once), moving the containment `debug_assert!` to the byte form. `Prefix::<H>::containing(&Path::from(prefix))` per leaf can become a slice comparison of the first `32 - H::HEIGHT` bytes against the current run's key. Acceptance: one buffer allocation per run, checked by an allocation-count test over a fixed multi-run stream; `full_height_roundtrip_matches_default`, `multi_run_grouping_matches_default`, and `leaf_height_assembly_is_identity` unchanged and green.

### streaming-backend-window-12: `adversarial::Role` is threaded through three layers and never read; its recorded heights disagree between `children` and `leaves`
- Where: src/tree/mirror/streaming/backend/local/adversarial.rs:10-15 (related: adversarial.rs:47-62, 101-125; backend/local.rs:142-145, 165-173, 185-188, 222-225; testing/failing.rs:28-33, 251-253)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (the only consumer is `next_delay(_role: Role)`, which binds the parameter with an underscore and reads only the thread-local; `Role` appears nowhere outside adversarial.rs and local.rs)
- Seen by: structure, correctness; refutation: confirmed; history: no-rationale-found (`_role` has been unread since the file's creation at 065356abe, a one-line commit; nothing records what it was reserved for)
- Owner-gated: no

`Role` is constructed at four sites in local.rs, stored on `DelayedFuture`/`DelayedStream`, passed to `delay()`, and discarded by `next_delay`. Meanwhile `Local::children<H>` records `Role::Children { height: H::HEIGHT }` where `H` is the child height, while `Local::leaves<H>` records the same variant where `H` is the source height, and `failing::Operation::Children` documents the source height. The inconsistency is harmless only because nothing reads the field. Circular justification: the type exists to be passed to a function that ignores it, and each `Local` method carries a four-line `cfg` split whose only test-specific content is the unused label.

Evidence:

        10	/// One typed asynchronous surface of [`Local`](super::Local).
        11	#[derive(Clone, Copy, Debug, Eq, PartialEq)]
        12	pub(super) enum Role {
        13	    Children { height: usize },
        14	    Parent { height: usize },
        15	}
    ...
       115	fn next_delay(_role: Role) -> u8 {

Resolution: Delete `Role` and the parameter from `future`/`stream`/`delay`/`next_delay`; optionally compile `adversarial` unconditionally with `#[cfg(not(test))]` identity versions of `future`/`stream` so each `Local` method ends in one line. If a role-keyed schedule is wanted instead, settle the height convention (source height, matching `failing::Operation`) and read the field. Acceptance: `Role` is read by the scheduler or is gone; local.rs's four `Backend` methods no longer construct a label.

### streaming-backend-window-13: The poll-delay scheduler exists in two copies, the countdown in three, the `.min(2)` cap in three files unnamed, and the `RoleStats` merge twice
- Where: src/tree/mirror/streaming/backend/local/adversarial.rs:17-44 (related: adversarial.rs:101-125; channel/instrumented.rs:34-45, 120-129, 159-167, 207-238, 288-294, 308-318; src/testing/transport.rs:180; src/tree/mirror/streaming/tests.rs:84-85)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep `min(2)`: adversarial.rs:123, instrumented.rs:316, transport.rs:180, none commented; the `Schedule`/`with_schedule`/`next_delay` blocks read identical modulo the ignored `Role`; streaming/tests.rs:84-85 nests `with_schedule` inside `with_local_schedule`)
- Seen by: structure, prose, perfapi; refutation: confirmed (adds the third cap at transport.rs:180); history: no-rationale-found (the scheduler entered inline at c9b8f38b0 and was copied at 065356abe, both one-line commits)
- Owner-gated: no

adversarial.rs and instrumented.rs each define an identical `Schedule { delays, step }`, an identical `with_schedule` with a `Restore` drop guard, and an identical `next_delay` including an unexplained `delay.min(2)` cap; the take-a-delay-then-self-wake countdown is written three times (adversarial.rs `delay()`, `Sender::send`, `Receiver::poll_recv`); a third uncommented copy of the cap sits in `src/testing/transport.rs`. Separately, `ChannelReport::kind` and `with_observation` fold `RoleStats` field by field with the same six rules. The two schedule cells are deliberately independent (streaming/tests.rs nests them), but the mechanism need not be duplicated to keep two cells. Duplicated logic drifts; the cap encodes a decision about schedule expressiveness that nothing states.

Evidence:

        17	struct Schedule {
        18	    delays: Vec<u8>,
        19	    step: usize,
        20	}
    ...
       121	        let delay = current.delays.get(current.step).copied().unwrap_or(0);
       122	        current.step += 1;
       123	        delay.min(2)
    (instrumented.rs)
       316	        delay.min(2)
    ...
        37	                total.channels += stats.channels;
        38	                total.effective_capacity = total.effective_capacity.max(stats.effective_capacity);

Resolution: Extract one test-support module (for example `streaming/testing/schedule.rs`) with a `ScheduleCell` around a `thread_local!` `RefCell<Option<Schedule>>` exposing `with` and `next`, plus a `Countdown(Option<u8>)` whose one method performs the wake-and-`Pending` step; instantiate two cells (backend, channel). Name the cap (`MAX_SCHEDULED_DELAY: u8 = 2`) with one sentence on why delays are bounded, and use it at transport.rs:180 too. Add `RoleStats::absorb(&mut self, other: RoleStats)` and call it from both folds. Acceptance: one definition each of the schedule struct, the restore guard, the next-delay read, the countdown step, and the `RoleStats` merge; both cells still independently settable; no bare `.min(2)`.

### streaming-backend-window-14: Inline `mod tests {}` blocks against the sibling-file convention
- Where: src/tree/mirror/streaming/backend/local/adversarial.rs:127-128 (related: src/tree/mirror/streaming/testing/failing.rs:267-268; src/testing.rs; src/tree/mirror/streaming/driver.rs; src/testing/transport.rs)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (`grep -rln '^mod tests {' src` returns five files: the two in this partition plus src/testing.rs, driver.rs, and testing/transport.rs)
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found (the convention entered AGENTS.md at dfd19c447, after both blocks, and was not applied retroactively; the rule carries no exemption clause)
- Owner-gated: no

AGENTS.md: "Unit and protocol tests live in a sibling file: `mod tests;` in the source, `tests.rs` next to it." Both files here are test-only themselves, so the reading-brevity cost is small, but the rule is stated without exception and the `testdoc` gate leg does not check placement.

Evidence:

       127	#[cfg(test)]
       128	mod tests {

Resolution: Move each block to a `tests.rs` sibling, or amend the convention to exempt test-only modules explicitly (my recommendation; see open questions). Acceptance: either no `mod tests {` block remains under src/, or AGENTS.md states the exemption.

### streaming-backend-window-15: Testdoc claims the wrappers self-wake; the body's noop waker cannot observe a wake
- Where: src/tree/mirror/streaming/backend/local/adversarial.rs:137-156 (related: adversarial.rs:107)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read: the context is built from `Waker::noop()` at line 140, so `cx.waker().wake_by_ref()` at 107 is unobservable; the assertions check only the Pending/Ready sequence)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (the doc line was added in the bulk testdoc sweep 358c6b1a over an unchanged body)
- Owner-gated: no

The doc states the property that keeps a delayed poll from stranding its task; the body cannot detect its loss. Every test doc states the invariant it protects and must be accurate.

Evidence:

       137	    /// Scheduled wrappers self-wake for every delay, then preserve the backend result or item.
       138	    #[test]
       139	    fn future_and_stream_delays_self_wake_then_complete() {
       140	        let waker = Waker::noop();
       141	        let mut cx = Context::from_waker(waker);

Resolution: Poll with a counting waker (a `std::task::Wake` impl over an `AtomicUsize`) and assert the count equals the scheduled delays (3 for `vec![2, 1]`), or narrow the doc to the Pending/Ready sequence it checks. Acceptance: the body asserts a wake count, or the doc no longer claims self-waking.
Construction: Delete line 107 (`cx.waker().wake_by_ref();`) and run this test: it stays green, while any scheduled streaming test hangs. A counting waker turns that into a failing assertion here.

### streaming-backend-window-16: `QueueKind::ALL`/`PROXY` are hand-maintained rosters the coverage tests trust; the type's summary names only the materialized graph
- Where: src/tree/mirror/streaming/channel.rs:8-57 (related: src/tree/mirror/streaming/tests/capacity.rs:125-135; src/tree/mirror/streaming/remote/proxy/tests.rs:601-606, 629)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (counted seventeen variants at lines 11-27 against the fourteen-plus-three roster entries; grep: the only consumers are capacity.rs:125 and proxy/tests.rs:601, 629; no exhaustive `match` over `QueueKind` exists; `strum` is absent from Cargo.toml)
- Seen by: structure, correctness, perfapi, prose (the doc half); refutation: confirmed, severity down to low (the instrument catches every listed edge; the escape is one variant added without its roster entry); history: no-rationale-found
- Owner-gated: no

`capacity_stress_covers_every_queue_role` and the proxy coverage test iterate these arrays to assert every edge was constructed and exercised. A variant added to `QueueKind` but not to a roster compiles and passes both tests vacuously: the coverage instrument cannot detect its own roster's incompleteness. The type's first sentence says "the materialized protocol's channel graph" while three variants are proxy edges. Doctrine: no hand-maintained enumerations of facts the code can change without touching them.

Evidence:

         8	/// One semantic edge in the materialized protocol's channel graph.
    ...
        31	    /// Every materialized semantic edge, for its coverage assertions.
        32	    #[cfg(test)]
        33	    pub const ALL: [Self; 14] = [
    ...
        50	    /// Every remote-proxy semantic edge, for its coverage assertions.
        51	    #[cfg(test)]
        52	    pub const PROXY: [Self; 3] = [

Resolution: Lightest: a `#[cfg(test)]` test with an exhaustive `match` over every variant asserting `ALL.contains(&kind) || PROXY.contains(&kind)` and that the rosters are disjoint; the compiler then forces the roster update. Or derive the roster (`strum::EnumIter` as a dev-dependency) partitioned by an exhaustive `fn side(self) -> Side`. Reword line 8 to name both graphs. Acceptance: adding a variant to `QueueKind` without updating a roster fails to compile or fails a committed test.

### streaming-backend-window-17: instrumented.rs has no module doc and `RoleStats`'s fields, the vocabulary of the capacity assertions, are undocumented
- Where: src/tree/mirror/streaming/channel/instrumented.rs:16-25 (related: instrumented.rs:1, 130-134, 186)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read: the file opens with `use` at line 1; the field meanings are defined at the increment sites)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (the module was split out of materialized/channel.rs at 065356abe without acquiring a doc)
- Owner-gated: no

The capacity and proxy tests assert against these counters, yet a reader must find line 130 to learn that `blocked_send_polls` counts send polls made while the channel had no free slot, and line 186 to learn `effective_capacity` is the post-`with_kind_capacity` cap.

Evidence:

        16	/// Aggregated observations for every channel created with one role.
        17	#[derive(Clone, Copy, Debug, Default)]
        18	pub struct RoleStats {
        19	    pub channels: usize,
        20	    pub effective_capacity: usize,
        21	    pub sends: usize,
        22	    pub receives: usize,
        23	    pub blocked_send_polls: usize,
        24	    pub high_water: usize,
        25	}

Resolution: One line per field and a two-sentence `//!` naming the three hooks (schedule, kind cap, observation). Acceptance: `cargo doc --document-private-items` shows every field documented and the module with a summary.

### streaming-backend-window-18: Instrumented channel occupancy counter can wrap, then overflow, under a multi-threaded executor
- Where: src/tree/mirror/streaming/channel/instrumented.rs:77-86 (related: instrumented.rs:117-142, 158-173; tests/routed_link.rs:226, 286; tests/disruption.rs:45)
- Class / severity / confidence: test-quality / nit / medium
- Provenance: assessed (read: `sent()` runs after the send future resolves at 138-140, `received()` after the poll yields at 169-171; grep `multi_thread`: only integration tests, which link the production channel, so the wrapper never meets one today)
- Seen by: correctness; refutation: confirmed (latent, test-only); history: no-rationale-found
- Owner-gated: no

A receive landing between the inner send completing and `sent()` running makes `fetch_sub` wrap to `usize::MAX`, and the next `fetch_add(1) + 1` overflows in debug builds. Not live today (every in-crate driver is single-threaded), but the wrapper is `Send` and nothing at `Stats` states the premise; the first `#[tokio::test(flavor = "multi_thread")]` around a scheduled mirror would see intermittent panics attributed to the channel.

Evidence:

        77	    fn sent(&self) {
        78	        self.sends.fetch_add(1, Ordering::Relaxed);
        79	        let occupancy = self.occupancy.fetch_add(1, Ordering::Relaxed) + 1;
        80	        self.high_water.fetch_max(occupancy, Ordering::Relaxed);
        81	    }
        82	
        83	    fn received(&self) {
        84	        self.receives.fetch_add(1, Ordering::Relaxed);
        85	        self.occupancy.fetch_sub(1, Ordering::Relaxed);
        86	    }

Resolution: Use `wrapping_add(1)` on the returned old value, or document at `Stats` that the counters assume a single-threaded poller and are approximate otherwise. Acceptance: no arithmetic on an atomic's returned value that can overflow, or the premise stated at `Stats`.
Construction: Run `scheduled_streaming_mirror` under `tokio::runtime::Builder::new_multi_thread().worker_threads(4)` with client and server as separate tasks, a few hundred iterations in debug; watch for `attempt to add with overflow` in `Stats::sent`.

### streaming-backend-window-19: convert.rs's module doc describes a dormant cross-backend converter; the wire adapter runs the module on every supplied node
- Where: src/tree/mirror/streaming/convert.rs:1-8 (related: src/tree/mirror/streaming.rs:22; src/tree/mirror/streaming/remote/adapter.rs:42-67; src/tree/mirror/streaming/remote/adapter/encode.rs:185; src/tree/mirror/streaming/remote/adapter/decode.rs:88, 408; src/tree/mirror/streaming/erased.rs:299-334)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep: erased.rs:308 `backend.leaves::<H>(...)` and :330 `.assemble::<H>(leaves)` dispatch to the trait methods whose defaults are `Convert::explode`/`Convert::assemble`; encode.rs:185 and decode.rs:88, 408 run them on every supplied node; `grep -rn Converted src` is empty; adapter.rs:44-58 documents exactly this path)
- Seen by: structure, prose; refutation: confirmed; history: deliberate-but-expired (the paragraph was written for `convert::Converted` at 01643fa5, deleted at 7c32f0e2; the adapter became the consumer at cbfe1aff3)
- Owner-gated: no

The doc frames `Convert` as what lets "a heterogeneous pair meet" and says "a homogeneous session pays nothing" and "the protocol itself converts nowhere". Today the wire carries supplies as leaf records, so every supplied node is exploded on encode (`Backend::leaves`) and reassembled on decode (`Backend::assemble`), within one backend, on every session; the supply-decode memory charge rides on this fold. A maintainer looking for where leaf runs become nodes again would not recognize the module from its doc. AGENTS.md hard rule: nothing in the tree describes code that no longer exists; the parent's one-line summary (streaming.rs:22, "the leaf conversion boundary between backends") is already closer to the truth.

Evidence:

         1	//! Re-represent nodes from one backend in the node types of another.
         2	//!
         3	//! A node converts by exploding to leaves in the source backend and
         4	//! reassembling in the target.
         5	//!
         6	//! The protocol itself converts nowhere: both parties of a session name one
         7	//! backend, and a homogeneous session pays nothing. This module is what lets a
         8	//! heterogeneous pair meet, by re-representing each node-carrying message.

Resolution: Rewrite around what the module does: explode a height-`H` node stream to the prefix-ordered leaf stream beneath it and reassemble height-`H` nodes from one, level by level through `Backend::children`/`Backend::parent`; the wire carries leaves, never nodes, so the encoder runs `Backend::leaves` and the decoder `Backend::assemble` on every supplied node, and a backend may override both in bulk. One sentence may note that leaf records are backend-neutral, so two peers' backends need not coincide. Renaming the module is optional taste. Acceptance: the module doc names the encode/decode call sites as consumers and no longer states that a homogeneous session pays nothing.

### streaming-backend-window-20: `S<H>::assemble` relays `fold_parents` through a pass-through generator left over from watermark stripping
- Where: src/tree/mirror/streaming/convert.rs:83-91
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (`git show 748325407^:src/tree/mirror/streaming/convert.rs` line 95 is `Box::pin(fold_parents(backend, below))`; 748325407 introduced the generator as `if let (prefix, Some(node)) = item? { yield (prefix, node); }`; the watermark removal reduced the body to the pass-through and kept the shell)
- Seen by: structure, correctness, perfapi; refutation: confirmed (history settles the lifetime doubt); history: deliberate-but-expired
- Owner-gated: no

The generator pulls each item out and yields it unchanged: an extra generator, pin, and `?`/re-wrap per assembled node on the default chain. `Local` overrides `assemble`, so the cost lands only on tests and any future backend; the legibility cost lands on every reader who looks for what the relay does.

Evidence:

        83	        let below = H::assemble(backend.clone(), leaves);
        84	        let folded = fold_parents(backend, below);
        85	        Box::pin(try_stream! {
        86	            let mut folded = pin!(folded);
        87	            while let Some(item) = folded.next().await {
        88	                let (prefix, node) = item?;
        89	                yield (prefix, node);
        90	            }
        91	        })

Resolution: `Box::pin(fold_parents(backend, H::assemble(backend.clone(), leaves)))`, the form the parent of 748325407 compiled. Acceptance: no generator in `assemble`; `folds_to_exactly_the_group_parents` and the `local/tests.rs` equivalence suite green.

### streaming-backend-window-21: testing.rs's summary describes only `Faulting`'s reply-corruption face
- Where: src/tree/mirror/streaming/testing.rs:3-4 (related: src/tree/mirror/streaming/testing/faulting.rs:29-42)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read faulting.rs:35-41: `Fault::Greeting` "Fires at the handshake; the phase countdown does not apply")
- Seen by: prose; refutation: confirmed; history: deliberate-but-expired (the sentence predates `Fault::Greeting`; 50c8b0a3 added the greeting-lie axis and changed only the `pub use` line here)
- Owner-gated: no

A doc comment's first sentence stands alone in a module listing and must cover every variant of the item it introduces.

Evidence:

         3	//! [`Faulting<P>`] wraps a protocol state and manufactures a genuine semantic
         4	//! violation in one outgoing phase. [`Failing<B>`] wraps its materialized

Resolution: "[`Faulting<P>`] wraps a protocol state and injects one fault: a semantic violation in a selected outgoing reply phase, or a lie in the greeting it sends." Acceptance: the sentence describes both `Fault` variants.

### streaming-backend-window-22: `Failing<B>` deliberately leaves `leaves`/`assemble` to the default chain, and says so nowhere
- Where: src/tree/mirror/streaming/testing/failing.rs:198-206 (related: src/conformance/backend.rs:379-386, 430-437; src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:203-213)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read: the impl forwards `erase`, `assume`, `node_bytes`, `parent`, `children` only; the conformance `Charged<B>` forwards both bulk methods with "Delegate to the wrapped backend's own override" comments; backend_errors.rs:210 iterates `for height in 1..32` expecting every level's operation to surface its injected error)
- Seen by: structure; refutation: confirmed; history: deliberate-and-holds (88a76a71: "defaulting to the level-by-level Convert fold, which Failing and any future database backend keep"; the rationale lives only in history)
- Owner-gated: no

The non-override is load-bearing: it routes `Failing<Local>` through every level's `children`/`parent`, so the per-level injection suite sees each checkpoint, and it means `Failing<Local>` sessions never exercise `Local`'s overrides. A maintainer comparing this impl with `Charged<B>` cannot tell intent from oversight. Undocumented deliberate choice: state the rationale at the site.

Evidence:

       198	impl<B> Backend for Failing<B>
       199	where
       200	    B: Backend<Node<Z>: Leaf>,
       201	{
       202	    type Node<H: Height> = FailingNode<B::Node<H>>;
       203	    // Erasure passes through the wrapper: fault injection targets the
       204	    // traversal operations, and re-tagging is not one.
       205	    type Erased = FailingNode<B::Erased>;
       206	    type Error = Failure<B::Error>;

Resolution: Add a two-line comment in the impl: `leaves`/`assemble` intentionally keep the default chain so injection sees every level's `children`/`parent`; forwarding to the inner overrides would bypass the checkpoint. Acceptance: the impl states the intent at the site.

### streaming-backend-window-23: faulting.rs: a bare `unreachable!()`, dated "current fixture" language over an unasserted premise, a type-admitted rejected `Fault`, and a duplicated corruption check
- Where: src/tree/mirror/streaming/testing/faulting.rs:227-240 (related: faulting.rs:32-34, 193-203, 259-269, 416-420)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read; the 0xff premise itself was not re-verified against every fixture)
- Seen by: structure, prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Line 240's `unreachable!()` carries no message where line 238's arm gives its proof (the proof here is the early returns at 193-203). The `UncontainedSupply` comment says "every current fixture satisfies" the 0xff premise: dated language, and a premise held in prose that a fixture growing a 0xff child would turn into a different violation without a message. `Fault::Reply(Violation::OverdrawnSupply)` is a value the type admits and the function rejects by panic. `fault_phase` (259) and `complete_responder` (416) each spell the `(0, Some(Fault::Reply(violation)))` test.

Evidence:

       230	                // its escaped version is at fault. Radix 0xff assumes no
       231	                // fixture holds that child, which every current fixture
       232	                // satisfies.
    ...
       237	            Violation::OverdrawnSupply => {
       238	                unreachable!("a set-length overrun is a greeting lie (Fault::Greeting), never a reply corruption")
       239	            }
       240	            Violation::UnaskedReply | Violation::UnansweredQuery => unreachable!(),

Resolution: Give line 240 its proof ("handled by the early returns above"); reword to "no fixture holds a 0xff child" and `debug_assert!` it against the honest reply; consider a `ReplyCorruption` enum for `Fault::Reply` so `OverdrawnSupply` is unrepresentable there; factor `fn corrupt_if_due(responses, remaining, fault)` and call it from both sites. Acceptance: no bare `unreachable!()`; no "current" in the comment; one site tests the corruption countdown.

### streaming-backend-window-24: The flushed-question derivation cites proxy internals by file path from a module that cannot link them
- Where: src/tree/mirror/streaming/window.rs:110-121 (related: window.rs:78-109; src/tree/mirror/streaming/remote/proxy/work/queues.rs:25-42)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (read queues.rs:34-36, which defers to the window docs as "the canonical derivation"; `encode.rs` and `pump.rs` are siblings of queues.rs, so from there both premises link as `super::encode`/`super::pump`)
- Seen by: prose; refutation: confirmed; history: deliberate-and-holds for the placement (b76a31f3 wrote the derivation into window.rs as the argument that the edge's window-wide capacity is necessary, and reconciled queues.rs to defer); what remains is the citation form
- Owner-gated: yes: moving the section reverses a recorded placement

The proof of the `ProxyLocalQuestions` occupancy bound names its premises by file path and explains inside the doc why intra-doc links cannot resolve. Cite by stable name, never file path: refactors orphan paths, and a doc that must explain its own citation mechanics is placed one module away from its subject.

Evidence:

       112	//! - the encoder flushes one complete wire reply, then publishes that
       113	//!   reply's entire question batch, before dequeuing its next scope
       114	//!   (`remote/proxy/work/encode.rs`; file paths, not intra-doc links,
       115	//!   because `proxy` is private to `remote` and unresolvable from here);
       116	//! - the decoder dequeues question-first and retires exactly one entry
       117	//!   per decoded reply, in wire order (`remote/proxy/work/pump.rs`);

Resolution: Move the "Sizing the flushed-question edge" section onto `queues::local_questions`, where the premises become resolving links and `Scope` is in scope, and reduce window.rs to one sentence pointing there; invert the pointer at queues.rs:34-36. Acceptance: no file path in window.rs prose; the derivation lives beside `local_questions`.

### streaming-backend-window-25: `KEY_DEPTH` restates `height::Root::HEIGHT` as a fresh literal
- Where: src/tree/mirror/streaming/window.rs:134-137 (related: src/tree/typed/height.rs:79-81, 125-133)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep: `KEY_DEPTH` is defined and used only in window.rs and window/tests.rs; height.rs:80 documents `HEIGHT` as "`Z` is 0; [`Root`] is 32" and the `Root` alias at 128-133 is thirty-two `S` layers)
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

Two independent 32s can drift; one should derive from the other. Named constants over magic numbers.

Evidence:

       134	/// Radix levels in the trie: one byte of a 32-byte leaf path per
       135	/// level. Typed heights run from `Z = 0` (leaves) to `Root = KEY_DEPTH`;
       136	/// the *depth* of the children discussed at height `h` is `KEY_DEPTH − h`.
       137	const KEY_DEPTH: usize = 32;

Resolution: `const KEY_DEPTH: usize = <height::Root as Height>::HEIGHT;`, keeping the local name for the depth-arithmetic prose, or at minimum a `const _: () = assert!(KEY_DEPTH == <height::Root as Height>::HEIGHT);`. Acceptance: one definition of the depth, tied to the typed height by construction or assertion.

### streaming-backend-window-26: `SCOPE_FIXED_BYTES` and `LEAF_REQUEST_BYTES` are hand-counted, name no types, and sit beside siblings whose docs state the rule against hand counting
- Where: src/tree/mirror/streaming/window.rs:158-163 (related: window.rs:139-156, 165-176, 243, 265; src/tree/mirror/streaming/materialized.rs:265-279; src/tree/mirror/streaming/remote/adapter/scope.rs:12-16; src/tree/typed/prefix.rs:17-34; src/tree/mirror/streaming/materialized/work/queues.rs:254; src/peer.rs:431, 452; window/tests.rs:24)
- Class / severity / confidence: idiom / low / medium
- Provenance: assessed (layout arithmetic from source, not compiled: `Prefix`/`ErasedPrefix` wrap `tinyvec::ArrayVec<[u8; 32]>`, which in the locked tinyvec 1.11.0 is `#[repr(C)] { len: u16, data: A }`, 34 bytes at alignment 2; `Query<E> { prefix: ErasedPrefix, ours: Vec<(u8, E)> }` and `Resolution<E> { prefix: ErasedPrefix, resolved: Vec<(u8, Resolve<E>)> }` each come to 64; the proxy `Scope { parent: ErasedPrefix, children: Vec<u8>, next: usize }` to 72; the leaf-request channel item at queues.rs:254 is a bare `Prefix<Z>`)
- Seen by: structure, prose, correctness; refutation: confirmed (adds `Resolution` as a third prefix-carrying container); history: already-known (the un-executed remainder of prior review R16, whose fix derived `REFERENCE_SLOT_BYTES` only; R16's padding argument lives only in the review note)
- Owner-gated: yes: deriving may move the pinned `SCOPE_ENVELOPE_BYTES` (5_431) and the "5431" figure at peer.rs:431

`REFERENCE_SLOT_BYTES` (139-156) and `FAN_SLOT_BYTES` (165-176) derive from `size_of` with the rationale that a hand-counted total goes stale. The two constants between them are literals whose docs name no containers, so the count cannot be audited against the code. Reading the candidates: if the two prefixes are `Query`'s and `Resolution`'s, 128 is exact today; if the second is the proxy `Scope` (whose `next: usize` the count omits), the fixed cost is 136; and a buffered scope has three prefix-carrying containers whose coexistence the doc does not settle. The 40-byte leaf request is the pointer-padded footprint of a 34-byte prefix, safe but not what the doc says. The 62,500 design-corpus figure is likewise restated by hand at window.rs:243 and peer.rs:452 beside its defining constant in window/tests.rs:24.

Evidence:

       158	/// Fixed in-memory bytes per buffered scope beyond its per-child slots:
       159	/// two inline prefixes (40 B each) and the container `Vec` headers.
       160	const SCOPE_FIXED_BYTES: usize = 2 * 40 + 2 * 24;
       161	
       162	/// In-memory bytes of one buffered leaf request: an inline leaf prefix.
       163	const LEAF_REQUEST_BYTES: usize = 40;
    ...
       142	/// Derived from `size_of` of the real slot types under the in-memory
       143	/// backend, so a layout change moves the price with it instead of
       144	/// leaving a hand-counted byte total stale: the pointer-aligned

Resolution: Name the containers and derive: `SCOPE_FIXED_BYTES = size_of::<Query<E>>() + size_of::<Resolution<E>>()` (or whichever pair is meant, at the `Local` erased instantiation) and `LEAF_REQUEST_BYTES = size_of::<Prefix<Z>>()` or its padded form with the padding intent stated; keep the pointer-class caveat the sibling docs carry. If the derived value differs from 128, re-pin `SCOPE_ENVELOPE_BYTES`, `tradeoff.md`, and the peer.rs figures in one deliberate commit naming the layout attribution. Cite `DESIGN_SESSION_MESSAGES` by name for the 62,500 restatements, or move that constant out of the test module. Acceptance: all four slot constants are `size_of` expressions over named types; `scope_envelope_matches_the_derivation` passes; if the pin moved, the commit states the attribution.

### streaming-backend-window-27: The public constant's first sentence overstates the budget's scope and names a cfg-gated private constant
- Where: src/tree/mirror/streaming/window.rs:267-275 (related: window.rs:264-265; src/peer.rs:20, 308, 351-369; src/lib.rs:341)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep: the re-export chain peer.rs:20 and lib.rs:341 makes `DEFAULT_SYNC_MEMORY_BUDGET` public API; `SCOPE_ENVELOPE_BYTES` is `#[cfg(any(test, feature = "test-internals"))] pub(crate)` at 264-265; `Peer::sync_memory_budget`'s own summary at peer.rs:308 is "Bound the memory a synchronization may spend on pipelining", with exclusions at 351-369)
- Seen by: prose; refutation: confirmed, severity down to low; history: no-rationale-found (written in the 2026-07-24 docs pass; the prior review verified the constant's arithmetic, not its public wording)
- Owner-gated: yes: public rustdoc

The first sentence, "Worst-case memory one synchronization may spend by default", drops the qualifier the method's own summary carries ("on pipelining") and contradicts the method's "What this does not bound" list (wire messages in hand, the replica, observers, other sessions). The doc also points the library user at `SCOPE_ENVELOPE_BYTES`, which the API does not reach. Public rustdoc names nothing the API does not reach, and a first sentence stands alone in a module listing.

Evidence:

       267	/// Worst-case memory one synchronization may spend by default: 512 MiB.
    ...
       272	/// [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget); the
       273	/// decomposition behind the accuracy band is recorded beside the pinned
       274	/// per-scope envelope (`SCOPE_ENVELOPE_BYTES`).
       275	pub const DEFAULT_SYNC_MEMORY_BUDGET: usize = 512 * 1024 * 1024;

Resolution: First sentence: "The default budget for the memory a synchronization may spend on pipelining: 512 MiB." Keep the link to `Peer::sync_memory_budget`. Move the `SCOPE_ENVELOPE_BYTES` pointer into a `//` maintainer comment or drop it (its own doc carries the decomposition). Acceptance: the public first sentence is consistent with the method's exclusions and no cfg-gated or crate-private item is named.

### streaming-backend-window-28: `from_budget`/`resolve` take four positional `u64`s where a length and a version-bytes bound can be swapped silently
- Where: src/tree/mirror/streaming/window.rs:338-345 (related: window.rs:524-531; src/tree/mirror/streaming/materialized.rs:624-630; src/tree/mirror/streaming/remote/proxy/start.rs:177-183; window/tests.rs:76, 88, 99)
- Class / severity / confidence: idiom / low / medium
- Provenance: assessed (read: the body at 346-358 uses only `max`, product, and sum of each pair, so local/remote order is immaterial; both production callers interleave fields from two greetings)
- Seen by: structure, perfapi; refutation: confirmed and reframed (the hazard is a length/version-bytes confusion, not side order); history: no-rationale-found (the signature accreted one argument at a time across d27cb5aa, 1988de6d, aabdcea0)
- Owner-gated: no

A `len` swapped with a `version_bytes` type-checks and mis-sizes the window, and an under-priced window is the one failure the docs call a memory breach rather than latency. Types-first: newtypes over same-typed positional runs. The test suite repeats the `(X, X, 0, 0, budget, f)` shape about fifteen times with no field names.

Evidence:

       338	    pub(crate) fn from_budget(
       339	        local_messages: u64,
       340	        remote_messages: u64,
       341	        local_version_bytes: u64,
       342	        remote_version_bytes: u64,
       343	        budget_bytes: usize,
       344	        node_bytes: impl Fn(usize, usize) -> usize,
       345	    ) -> Self {

Resolution: Introduce a small `Copy` struct (`Corpus { messages: u64, version_bytes: u64 }`, or reuse the greeting's pair) and take `local: Corpus, remote: Corpus`; derive it from `Root<B>` on the walk side and from the two greetings on the proxy side; shrink the test helpers accordingly. Acceptance: no call site passes four bare `u64`s to the solve; window/tests.rs compiles against the struct form and stays green.

### streaming-backend-window-29: Four-point monotonicity `debug_assert` in `from_budget` duplicates the conformance suite's grid sweep over a crate-internal boundary
- Where: src/tree/mirror/streaming/window.rs:360-370 (related: src/tree/mirror/streaming/backend.rs:110-117; window.rs:334-337; src/conformance/backend.rs:571-615, 651; window/tests.rs:316-318, 330-332)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (read: `node_bytes_monotone::<B>()` at conformance/backend.rs:584-615 sweeps every adjacent fan pair up to `FAN` crossed with a bound grid and runs at :651 for every backend the suite exercises; its doc at 580-581 says it exists because the derivation's check is "a four-point `debug_assert`, compiled out of release"; lib.rs:322 keeps `tree` private so no external `node_bytes` reaches the assert)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (prior review R22 asked for the sweep; 8d959048 added it and kept the assert, naming both in the trait doc without saying why)
- Owner-gated: no

A guard earns its place by naming a constructible failure the committed tests cannot catch; for a deterministic pure function, a runtime spot check samples the same space the sweep samples deliberately. The one space the sweep does not reach is the ad-hoc closures window/tests.rs hands to `from_budget` directly (`materializing_node_bytes`, `|_, _| usize::MAX`), which are test inputs rather than backends. Three prose sites cite the assert as a safeguard on the production contract, which overstates what it does.

Evidence:

       360	        #[cfg(debug_assertions)]
       361	        for window in [0usize, 1, 16, FAN].windows(2) {
       362	            debug_assert!(
       363	                node_bytes(window[0], version_bound) <= node_bytes(window[1], version_bound),
       364	                "node_bytes must be monotone in the child count",
       365	            );

Resolution: Delete the assert and re-state the three prose references (backend.rs:113 "debug-asserted when a session derives its window", window.rs:337 "spot-checked here in debug builds", conformance/backend.rs:580-581) to name the conformance sweep alone; or keep it and say at the site that it guards test-supplied pricing closures, which is the only space the sweep misses. Acceptance: either no `debug_assert` on `node_bytes` in `from_budget` and the three sites cite `node_bytes_monotone`, or the assert's site names what it catches.

### streaming-backend-window-30: The leaf-request charge term is identically zero for every u64 corpus; the proof lives in a 22-line comment with no committed pin
- Where: src/tree/mirror/streaming/window.rs:409-443 (related: window.rs:162-163, 649-666, 750-760; window/tests.rs:57-58, 177-186, 210)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (Python replica of window.rs:580-760 at `scratchpad/final/envelope.py`: `stage_population(n, pair, 32)` is 0 at (u64::MAX, u64::MAX), (2^63, 2^63), (10^10, 10^10), (62_500, 62_500), (1, 1), and (u64::MAX, 1); at the maximal pair every depth from 25 down is zero, depth 24 is 12; raising `UNION_TAIL_BITS` to 55 makes depth 25 nonzero (12) and to 111 makes depth 32 nonzero (12), while lowering it to 8 widens the zero region)
- Seen by: structure, correctness; refutation: confirmed, with the demonstration's sign corrected (the guard at 658 is `num_bits + t + 2 < den_bits`, so a smaller `t` makes more populations zero); history: deliberate-and-holds for keeping the term (655d2ae9 chose to charge "the identical expression the grant uses" so both sides of the function state one bound; e82b862e added the exact thresholds); the absence of a pin is the un-ruled half
- Owner-gated: no: adding the pin reopens nothing; deleting the term would reopen 655d2ae9 and is the owner's call

`population[KEY_DEPTH]` multiplies `jointly_occupied(n, pair, 30)`, whose quantile is zero whenever `bitlen(pair) + 50 < 241`, and `pair` is a product of two `u64`s. So `LEAF_REQUEST_BYTES` never contributes, heights 0 through 7 always receive the floor, and the comment exists to prove it. Nothing tests the claim; `deep_levels_are_sparse` asserts `<= 16` at heights 0..=22 for one corpus. A change to `UNION_TAIL_BITS`, the `+ 2` headroom, or `TAIL_DEPTH_CAP` would falsify the comment's proof with nothing failing. Doctrine: every invariant held in prose becomes a committed check, and the solve should read as evidently right without a page of argument about a line that does nothing.

Evidence:

       419	        // the granted statistic is `jointly_occupied(n, pair, 30)`
       420	        // times the per-parent fan, whose quantile is zero for every
       421	        // representable corpus (`small_mean_quantile` at j = 30 has 241
       422	        // denominator bits against at most 128 for a product of two u64
       423	        // corpora; nonzero needs pair ≥ 2¹⁹⁰). The entries that could
    ...
       442	            total.saturating_add(population[KEY_DEPTH].min(k) * LEAF_REQUEST_BYTES as u128)

Resolution: Add a proptest `deep_stage_populations_are_zero(a in any::<u64>(), b in any::<u64>())` asserting `stage_population(n, pair, d) == 0` for `d in 25..=KEY_DEPTH` with `n = max(a, b)` and `pair = a * b`, cite it from the comment, and shrink the comment to the invariant (two sentences: the edge is charged at the width it is granted; that width is zero for every u64 pair, pinned by the test, so the edge floors at one slot). Keep the term for formula totality per 655d2ae9. The test-side `charge()` replica (tests.rs:57-58) and `scope_envelope_matches_the_derivation` (tests.rs:210) keep their matching term. Acceptance: the committed test fails when `UNION_TAIL_BITS` is raised to 111 (depth 32 becomes nonzero at the maximal pair) and passes at HEAD; the comment block is reduced to the invariant; `SCOPE_ENVELOPE_BYTES` and `tradeoff.md` are unchanged.
Construction: Temporarily set `UNION_TAIL_BITS = 111` and run the proposed test with `a = b = u64::MAX`: `stage_population(n, pair, 32)` returns 12. At `UNION_TAIL_BITS = 55`, the depth-25 assertion fails first (population 12).

### streaming-backend-window-31: `Window::capacity` clamps an out-of-range height that only programmer error can produce; two `unwrap_or`s guard conversions that cannot fail
- Where: src/tree/mirror/streaming/window.rs:478-480 (related: window.rs:380, 447, 737-741; src/tree/mirror/streaming/materialized/work/levels.rs:197, 313-326, 362-371, 557-559; src/tree/mirror/streaming/remote/proxy/work/pump.rs:77-350; src/testing.rs:231-232, 332; window/tests.rs:49, 205)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (grep of every `.capacity(` caller: levels.rs and pump.rs pass typed `HEIGHT` consts or `asked_height + {1, 2}` where `internal_level<H>` supplies `H::HEIGHT` under `S<S<H>>: Height`; testing.rs iterates `0..=32`; `children_quantile` is `.min(FAN)` at 737-741 so the `try_into` at 380 is infallible; `population` is a 33-element array so `.max()` is never `None`)
- Seen by: correctness, perfapi; refutation: confirmed; history: no-rationale-found (the clamp has been there since d27cb5aa introduced the per-height table, uncommented)
- Owner-gated: no

A silent clamp reads as if out-of-range heights were expected input and maps a mispairing to the root's capacity instead of surfacing it. Panics are the sanctioned outcome for programmer error.

Evidence:

       478	    pub(crate) fn capacity(&self, height: usize) -> usize {
       479	        self.capacities[height.min(KEY_DEPTH)]
       480	    }
    ...
       380	            let held = children_quantile(n, depth).try_into().unwrap_or(usize::MAX);
    ...
       447	        let ceiling = population.iter().copied().max().unwrap_or(1).max(1);

Resolution: Index directly with a one-line comment that typed heights bound the argument (or `debug_assert!(height <= KEY_DEPTH)` and index); replace the dead `unwrap_or`s with `expect` naming the bound, or restructure so the fallible conversion disappears. Acceptance: no `.min(KEY_DEPTH)` in `capacity`; no `unwrap_or` on a provably infallible conversion.

### streaming-backend-window-32: Integer-envelope dominance is certified only by an example no recipe runs, and that example checks a different family than the shipped code
- Where: src/tree/mirror/streaming/window.rs:570-576 (related: examples/envelope_sim.rs:17-31; justfile:774, 889-973; .github/workflows; window/tests.rs:354-381)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (grep of the justfile and .github/workflows: the only examples run are `window_tradeoff` and `before`'s `amp_board`; the justfile's "envelope suite" at :655 is `before`'s amplification envelope, not this one; envelope_sim.rs:28-31 states it certifies "the one-corpus `N` forms" while window.rs implements "the pair-based `A·B` adaptation"; window/tests.rs, read in full, never compares an integer quantile to an exact tail: `envelopes_are_consistent` checks internal ordering only)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (ded24eb3 named the example the certifying tool of record after deleting the Python simulator, but no commit or note decides whether it runs under the gate; the scope limitation is stated in the example's own header and was never closed)
- Owner-gated: yes: the pin itself is not gated, but the disposition of `envelope_sim` (gate leg, or superseded and retired) touches a recorded decision and gate policy

The module's 2^-40 per-session bound rests on each integer quantile dominating its exact-Chernoff counterpart. The comment cites the example as the certificate; nothing runs it, and its own header says it certifies a different family than `jointly_occupied`/`stage_population`. A change to `UNION_TAIL_BITS`, `BERNSTEIN_TAIL`, `TAIL_DEPTH_CAP`, or the `b >= 5` / `t/(b-3)+2` constants passes the gate unchallenged today. Principle 2: any quantity computable two ways gets a committed comparison, and a status board nothing enforces is decoration.

Evidence:

       570	// The functions below bound the resulting occupancy statistics from
       571	// above with pure integer arithmetic. Each integer quantile is
       572	// constructed to dominate its exact-Chernoff counterpart, and the
       573	// dominance is verified by `examples/envelope_sim.rs` over a dense
       574	// sampled sweep of (N, depth) — sampled, not exhaustive — so on that
       575	// certificate the integer envelopes inherit the exact tails' joint
       576	// ≥ 1 − 2⁻⁴⁰ per-session bound (UNION_TAIL_BITS counts the union). All

Resolution: Add a committed proptest in window/tests.rs over log-uniform `(a, b)` corpora and `depth in 1..=KEY_DEPTH` asserting each integer quantile is at least the exact-distribution quantile at its stated tail level: `leaves_quantile(n, j)` against the least `q` with `P(Binomial(n, 256^-j) >= q) <= 2^-(48 + 8 min(j, 40))` via a log-space survival function; `jointly_occupied(n, pair, j)` against the Poisson upper tail at mean `pair / 256^j` and level 2^-48; `child_slots_quantile` against the occupied-child-slot count. Then decide the example's fate: a `just` leg (`cargo run --release --example envelope_sim`) or retirement once the in-tree test demonstrably catches what the example caught. Acceptance: a committed test fails when any integer quantile is lowered below its exact counterpart (demonstrate by changing `+ 2` to `+ 1` in `small_mean_quantile`, or `BERNSTEIN_TAIL` to 20, and observing the failure), and the module comment cites that test by name.
Construction: For fixed grids first (`n` in {10^3, 62_500, 10^6, 10^10, 2^40, 2^63}, `j` in 1..=32): compute `log_sf(q) = log P(X >= q)` for `X ~ Binomial(n, 256^-j)` by summing `exp(lgamma terms)` from `q` to `n` (or the Poisson with `mu = n * p`, which upper-bounds the binomial tail for small `p`), find the least `q` with `log_sf(q) <= -t ln 2`, and assert `leaves_quantile(n, j) >= q`; then generalize to the proptest.

### streaming-backend-window-33: `pow256`'s safety argument rests on a premise its call sites falsify
- Where: src/tree/mirror/streaming/window.rs:580-584 (related: window.rs:674, 698, 700, 714, 730-731)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read: the saturated value is a divisor at 698 `pair / pow256(j)` and 714 `n / pow256(j)`, and a multiplicand at 730 `pow256(j).saturating_mul(fan)` before dividing at 731; the `min` uses at 674 and 700 are the only comparisons)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (the premise was already false at d27cb5aa)
- Owner-gated: no

The doc says the saturated value is "only ever compared against" bounded values. The conclusion holds for a different reason: division by the saturated value floors to zero and every caller pads `+ 1`, and `min` with it is the identity, so saturation only narrows an envelope. A safety argument whose premise is false trains the reader to distrust the file's other arguments, which are careful.

Evidence:

       580	/// `256^j`, saturating at `u128::MAX` (only ever compared against values
       581	/// bounded by `u64` inputs, so saturation is always on the safe side).
       582	fn pow256(j: usize) -> u128 {

Resolution: State the operative argument: saturating for `j >= 16`; as a `min` operand saturation is the identity, and as a divisor it floors the mean to zero, which every caller pads with `+ 1`, so saturation only ever narrows an envelope. Acceptance: the premise is true of every use in the file.

### streaming-backend-window-34: `BERNSTEIN_TAIL` is a hand-derived literal with a maintenance instruction where `tail_exponent(0)` computes the same number
- Where: src/tree/mirror/streaming/window.rs:601-606 (related: window.rs:622-624, 685)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (replica: `tail_exponent(0) = ceil(7 * 48 / 10) = 34 = BERNSTEIN_TAIL`; `jointly_occupied`'s doc at 685 already calls it the flat level)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (prior review R75 asked that the tail literals be named so a change cannot strand a companion; b0304e71 named them as literals with a reminder rather than deriving)
- Owner-gated: no

"A changed tail level must move this with it" asks a human to do what the compiler can. R75's own goal favors the derivation.

Evidence:

       601	/// The Bernstein exponent delivering the union tail: `e⁻ᵗ ≤ 2⁻⁴⁸` needs
       602	/// `t ≥ UNION_TAIL_BITS × ln 2 ≈ 33.3`, rounded up.
       603	///
       604	/// Derived from [`UNION_TAIL_BITS`]; a changed tail level must move
       605	/// this with it.
       606	const BERNSTEIN_TAIL: u128 = 34;

Resolution: Make `tail_exponent` a `const fn` (replace `j.min(TAIL_DEPTH_CAP)` with an `if`; `u128::div_ceil` is const) and define `const BERNSTEIN_TAIL: u128 = tail_exponent(0);` with the doc reading "the flat union level: `tail_exponent` at zero sharpening". Drop the maintenance sentence. Acceptance: `BERNSTEIN_TAIL` has no literal initializer; `envelopes_are_consistent` and the window tests pass unchanged.

### streaming-backend-window-35: `child_slots_quantile`'s doc attributes the evaluation-point slack to the `+ 1` when the two named losses can exceed one
- Where: src/tree/mirror/streaming/window.rs:721-727 (related: window.rs:626-638)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (replica: the supremum of `1024x / (255 (2 + x)^2)` is about 0.502 at `x = 2`, slightly over the doc's "half a slot"; the floor loss is strictly under one; the two together exceed the `+ 1` when the floored quotient's fractional part exceeds about 0.498)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (the paragraph is the fix for prior review R49, which ruled that the comment should record the actual argument; this continues that ruling)
- Owner-gated: no

The conclusion holds through the Bernstein slack `(t - 2) s` at `t >= 34`, which the sentence's last clause says merely "rides on top". A maintainer re-deriving from this comment would misattribute which term carries the slack.

Evidence:

       723	/// e^(−x′)) ≤ 256 × 2x′/(2 + x′)`. The code evaluates that form at
       724	/// `x = Np` instead of `x′` — an understatement of at most half a slot
       725	/// for `p ≤ 1/256`, absorbed (with the floor division's sub-unit loss)
       726	/// by the `+ 1` on `mean_hi`; the Bernstein slack at `t ≥ 34` rides on
       727	/// top.

Resolution: Reword: the `+ 1` covers the floor loss; the sub-slot understatement (at most about 0.502) is covered by the Bernstein `+ t` term, whose sufficiency argument in `bernstein` leaves `(t - 2) s` of slack. Acceptance: each named loss is matched to a term at least as large.

### streaming-backend-window-36: Testdoc claims every mid-depth level pipelines; the body checks one height
- Where: src/tree/mirror/streaming/window/tests.rs:150-167 (related: window.rs:458-472)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read)
- Seen by: prose; refutation: confirmed (and notes the universal is doubtful on its own terms: a level whose population is 2 to 16 gets capacity equal to its population, not "well past" the floor); history: no-rationale-found
- Owner-gated: no

The doc states a universal over every level whose population exceeds one; the body asserts `capacity(KEY_DEPTH - 4) > FAN` at one height. The comment inside the body is accurate about that point.

Evidence:

       150	/// The default pairing pipelines where population lives: every mid-depth
       151	/// level whose population exceeds one gets capacity well past the
       152	/// serialization floor.
    ...
       166	    assert!(window.capacity(KEY_DEPTH - 4) > FAN);

Resolution: Either iterate the depths and assert `capacity(KEY_DEPTH - depth) > 1` wherever `stage_population(n, n * n, depth) > 1` (the helpers are imported), or reword the doc to the point it checks: depth 4, the first stage whose population outgrows the structural caps, receives capacity above a full fan under the default budget. Acceptance: doc and assertions describe the same set of heights.

### streaming-backend-window-37: Testdoc quotes 60 B / 65,404 / ~4.3× while its assertions and `Peer::sync_memory_budget` say 52 B / 91,941 / ~2.6×
- Where: src/tree/mirror/streaming/window/tests.rs:266-310 (related: src/peer.rs:421-429, 441)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`git show 4dd2053c^:...window/tests.rs` has `Some(60)` and `65_404` under these doc lines; `git show 4dd2053c:...` has `Some(52)` and `91_941` with the doc untouched; `git blame` puts lines 269 and 273 in ba8045c5 and lines 299, 307, 308 in 4dd2053c; peer.rs:422, 425, 441 quote 52 B and ~2.6×)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed; history: no-rationale-found (4dd2053c's message says it corrected the assert message's stale "~4.6x" note and re-pinned the figures, and its diffstat includes peer.rs; the doc comment above was not touched)
- Owner-gated: no

The test exists to stop quoted figures drifting, and its own header is the drifted figure. AGENTS.md: every test's doc comment states its invariant and review holds it to the standard; an inaccurate testdoc is a bug in the test.

Evidence:

       269	/// `m* = 60 B` (quoted at `Peer::sync_memory_budget`) is the
       270	/// smallest record size whose self-consistent corpus — the spec BDP in
       271	/// `m`-size records, per side — fits entirely inside the window the
       272	/// default budget derives at that corpus; the u64 column's BDP-scale
       273	/// corpus derives a 65,404-scope window, the quoted ~4.3× figure.
    ...
       299	        Some(52),
    ...
       307	        91_941,
       308	        "the u64 BDP-scale window moved: update the ~2.6x figure quoted at \

Resolution: Rewrite the doc without the literals ("the crossover record size and the u64 BDP-scale window are the solve's own numbers, pinned here and quoted at `Peer::sync_memory_budget`") so the next re-pin cannot reopen the gap; the assertions and their messages remain the record. If figures must stay in the doc, name them once as constants used by both the assertion and the message. Acceptance: no number in the doc comment disagrees with the assertions below it or with peer.rs:422-441.

### streaming-backend-window-38: Uniform `u64` proptest strategies for corpus size effectively never sample small corpora
- Where: src/tree/mirror/streaming/window/tests.rs:359-362 (related: window/tests.rs:376, 403-404)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (proptest's `RangeFrom<u64>` and `Range<u64>` strategies sample uniformly, so values below 2^32 arrive with probability 2^-32; the fixed-point tests at 62,500, 10^6, 10^10, and 2^24 are the only small-corpus coverage)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`window_stays_inside_the_budget` and `envelopes_are_consistent` draw `messages in 1u64..` and `0u64..`, landing in [2^62, 2^64) three quarters of the time. The family where near-root populations are sub-fan and the Poisson branch of `small_mean_quantile` fires at shallow depths, which is where real sessions live and where an off-by-one in the bit-length comparison would show, is a point test with extra steps. Every window proptest also passes symmetric pairs.

Evidence:

       359	    fn window_stays_inside_the_budget(
       360	        messages in 1u64..,
       361	        budget in 0usize..=1 << 44,
       362	    ) {

Resolution: Use a log-uniform strategy (for example `(0u32..64).prop_flat_map(|bits| { let lo = 1u64 << bits; lo..=lo.saturating_mul(2).saturating_sub(1) })`) for both tests, and draw asymmetric `(a, b)` pairs in `window_stays_inside_the_budget`. Acceptance: both strategies produce corpus sizes spanning 1..2^63 with roughly equal mass per octave; asymmetric pairs are drawn.

## Positives

- backend/local/tests.rs holds `Local`'s bulk `leaves`/`assemble` overrides to the level-by-level `Convert` default by observational equivalence over generated leaf runs shaped to stress both compression (tiny-alphabet deep spines) and fan (full-alphabet wide fans), comparing hash, len, floor, ceiling, and the otherwise-unserialized `version_bytes` aggregate per node, and round-tripping the leaves both ways. The module doc states exactly why the overrides exist and what keeps them equivalent. This is the differential-oracle discipline applied at the right boundary.
- window.rs derives `REFERENCE_SLOT_BYTES` and `FAN_SLOT_BYTES` from `size_of` of the real slot types with the rationale inline, and local.rs:110 pins the `Local` handle at pointer size with a compile-time assertion the window's per-reference price rests on: layout facts the compiler checks rather than prose.
- window/tests.rs pins every figure the prose quotes to a recomputation (`scope_envelope_matches_the_derivation`, `supply_decode_envelope_matches_the_charge`, `tradeoff_table_matches_the_derivation`, `default_crossover_matches_the_solve`), each assertion message naming the doc site to update; `pathological_pricing_saturates_to_the_floor` demonstrates the saturating solve at `u64::MAX` corpora under `usize::MAX` pricing. The one drift I found (finding 37) is in a testdoc, not in any pinned figure.
- `Window::from_budget` is total and floor-preserving: saturating arithmetic where a u64 population times a near-`usize::MAX` price passes u128, plain multiplication only where provably below 2^70, and a binary search that terminates at capacity one when even the floor exceeds the budget. The integer-envelope arithmetic checks out line by line: `small_mean_quantile`'s bit-length test is a strictly stronger form of `num * 2^(t+2) < 256^j`, `bernstein`'s `(t - 2) s` slack argument is correct, and the bit-length figures in the `from_budget` comment (241 and 249 denominator bits) match my replica.
- `WindowConfig::default` is unconditional on cargo features, with the reason stated at the site (features are additive and unify across a build graph) and pinned by `default_is_the_budget_unconditionally`; `Window::FLOOR` is an explicit test opt-in rather than a build-shape accident.
- backend.rs:77-88 (`Backend::assume`) follows its unspecified-behavior clause with a one-line proof of why only programmer error can reach it, naming the witness (an erased prefix's byte length is its height) and why peer input cannot reach a mispairing. `Leaf::leaf` carries uniform `# Errors` and `# Cancel safety` sections, and the cancel-safety text reasons through the reclaimable-garbage case rather than asserting safety. `Backend::node_bytes` names the one asymmetric failure mode precisely and points at the suite that convicts a violating implementation.
- The wire path cannot reach `from_sorted_leaves` with unsorted, uncontained, duplicate, or oversized-version leaves: the decoder rejects those as session errors before a leaf enters assembly, so the backend's preconditions hold by construction and its panics are backend-contract enforcement with stated proofs, exactly as backend.rs promises.
- The test doubles compose cleanly: `Failing` layers keep independent countdowns and distinguish their errors (`failing_backends_compose`); `Faulting` injects both reply corruptions and greeting lies, and the greeting-lie suite checks the tolerated (inflated) directions as well as the detectable (shrunken) ones, so the guards are tested for false positives too; every `with_*` scope in the instrumented channel and the adversarial scheduler restores prior state through a `Drop` guard so nested scopes unwind correctly.
- convert/tests.rs:74-78 calls out the one case a naive fold misses (the final group, flushed at input end) and builds its expected value through `Backend::parent` directly, not through the fold under test.

## Open questions for Finch

- Which two containers does `SCOPE_FIXED_BYTES` price (finding 26)? My layout reading: `Query<E>` plus `Resolution<E>` gives exactly 128 today, `Query<E>` plus the proxy `Scope` gives 136 because of `Scope::next`, and a buffered scope has all three. Recommendation: name `Query` and `Resolution` in the doc, derive both constants from `size_of`, and state in one sentence whether the proxy `Scope` is covered by the "overlapping views" argument at window.rs:401-406 or should be priced. If the derived value is 128 nothing re-pins.
- Is a persistent `Backend` still the plan (finding 1)? Recommendation: add the one-sentence "crate-internal today" status to backend.rs now, and schedule the trait-shape pass (owned `Vec` in `parent`, the `pub(crate)` alias in `assemble`'s signature, the `Error` bounds) for the publication commit rather than now.
- `examples/envelope_sim.rs` (finding 32): gate leg, or supersede with an in-tree dominance proptest over the shipped pair-based functions and retire it? Recommendation: the in-tree proptest, because the example's own header says it certifies a different family; then retire the example under the same discipline as landing an instrument, once the proptest demonstrably catches a lowered quantile.
- `Local::assemble`'s run buffer (finding 10): price it in prose or rebuild incrementally? Recommendation: prose now, stating the per-leaf bookkeeping as a `size_of` expression and that it falls under the budget's "replica itself" exclusion; build the radix-stack builder only if a census or allocation meter over a large single-run supply shows it matters.
- The leaf-request term (finding 30): keep it per 655d2ae9's symmetry ruling with the committed pin, or delete it? Recommendation: keep, pin, and cut the comment to the invariant.
- `TAIL_DEPTH_CAP = 40` (window.rs:608-611) never binds: every caller passes `j <= 32`. Is it deliberate future-proofing against a `KEY_DEPTH` change (then its doc should say so), or dissolvable? Recommendation: dissolve, or tie it to `KEY_DEPTH` with a stated reason.
- Bare `pub` versus `pub(crate)` under the private `mod tree` (finding 2): enable `unreachable_pub`, or state the convention? Recommendation: enable the lint; it makes the API surface legible from the keyword.
- Moving the flushed-question derivation onto `queues::local_questions` (finding 24) reverses b76a31f3's placement. Recommendation: move it; the file-path citations are the cost of the current placement, and queues.rs already defers to it.
- Inline `mod tests {}` in test-only modules (finding 14): move to siblings, or exempt? Recommendation: exempt test-only modules explicitly in AGENTS.md; a `tests.rs` beside a module that is itself `cfg(test)` is ceremony.
- backend/local/tests.rs:104-105 says the `version_bytes` aggregate is "not serialized, so this equivalence is its only bulk-path coverage", but the root's aggregate is serialized in the greeting. Is "not serialized" meant per node? Recommendation: reword to "no node's wire form carries its aggregate" so the coverage claim reads precisely.

## Dropped

- `impl Stream for` the instrumented `Receiver` has no consumer (candidate 8): refuted. erased.rs:143-144 declares `#[cfg(test)] type ReceiverStreamOf<E> = Receiver<E>;` and `ReplyResultStream::poll_next` polls it through `Stream::poll_next` (erased.rs:133-138); common.rs:52 calls `rx.map(Ok)` on the instrumented receiver under `cfg(test)`. Deleting the impl breaks the test build.
- Two hand-rolled run-grouping loops share one shape (candidate 12): refuted. The flushes differ in kind: `fold_parents` awaits a fallible `backend.parent` that may return `None` and must be filtered (convert.rs:104-119, 132-135), while `Local::assemble` builds synchronously and infallibly (local.rs:211-214). A generic `runs_by` would leave each caller a tail comparable in length to the loop it replaces.
- Candidates 17, 36, 50 (testdoc figures): duplicates of streaming-backend-window-37.
- Candidate 22 (convert doc): duplicate of streaming-backend-window-19.
- Candidates 41, 56 (rosters): duplicates of streaming-backend-window-16; candidate 27's channel.rs half is folded there and its testing.rs half is streaming-backend-window-21.
- Candidate 19 (ghost `T`): duplicate of streaming-backend-window-5.
- Candidate 42 (`Role` heights): duplicate of streaming-backend-window-12, which carries its height-convention observation.
- Candidates 32, 55 (scheduler duplication and cap) and 13 (`RoleStats` merge): folded into streaming-backend-window-13.
- Candidate 39 (leaf-term pin): duplicate of streaming-backend-window-30, with the refutation pass's sign correction applied.
- Candidates 20, 44 (hand-counted constants): duplicates of streaming-backend-window-26.
- Candidate 52 (positional `u64`s): duplicate of streaming-backend-window-28.
- Candidates 46, 58 (pass-through generator): duplicates of streaming-backend-window-20.
- Candidate 57 (capacity clamp): duplicate of streaming-backend-window-31.
- Candidate 37 as a medium correctness finding: reframed to documentation at low (streaming-backend-window-10); no committed promise is falsified because the budget's documented scope excludes the replica.
- Candidate 51's "decide the seam's status": reopens a recorded deferral without new evidence; narrowed to the status sentence and the trait-shape notes in streaming-backend-window-1.
