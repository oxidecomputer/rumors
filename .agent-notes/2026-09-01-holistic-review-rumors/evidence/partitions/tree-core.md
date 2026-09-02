# Partition tree-core: The sparse Merkle radix trie: Tree, its tests and generators, the traversals (act, join, unknown)

## Partition summary

This partition is the in-memory content tree and everything that mutates it. `src/tree.rs` wraps a `Root` (a `Version` ceiling riding beside an optional height-32 typed node) in a `Tree<T>` facade whose `T` is a `PhantomData<fn() -> T>` witness over erased `Message` storage. It exposes the read faces (`hash`, `get`, `iter`, `range`, `range_owned`, `len`, `latest`, `earliest`), two commit paths (`act`, which ticks and keys a local batch and hands it to the private `react`; and `join`, the in-memory merge every gossip commit takes), and two test-only thread-local instruments (a root-hash read meter and a panic fuse). `src/tree/traverse/` holds three height-inductive traversals as polymorphic-recursive traits over the `S<H>`/`Z` ladder: `act` (batch apply with a per-key observer that decides the changed flag and the ceiling movement), `join` (lockstep merge with pointer-or-hash pruning), and `Unknown` (the deletion-honoring filter over memoized `[floor, ceiling]` spans that `join`'s one-sided arms call). `mod tree` is private (`src/lib.rs:322`); only `MERKLE_HASH_LEN` is re-exported and `Iter` leaks through `Snapshot`'s `IntoIterator`, so nearly every `pub` doc here is maintainer-facing and was judged at that altitude. `arb.rs` is the generator and fixture module every tree-adjacent suite in the crate imports; `tests.rs` is the tree's property suite.

The production code is structurally sound. No traversal recurses on input-controlled depth (all three recurse on the type-level height, bounded at 32 by construction); the one production `assert!` (version reuse, act.rs:169-176) and the one `unreachable!` (same-position leaves, join.rs:230-232) each carry a one-line argument that holds against `Hash::leaf`'s suffix-only preimage and `Node`'s pointer-or-hash equality; both commit sections defend panic atomicity by statement order with fuse-injected and destructor-source pins for each; and the changed flags are decided by the traversals, stated in both directions, with the one conservative case constructed rather than argued. No lens found a reachable panic, overflow, or torn state from any input the crate admits. `reference_hash` in `tests.rs` is an independent oracle, and `arb.rs` justifies each fixture by the shape it reaches and why version addressing cannot reach it otherwise.

The dominant issues are prose that outlived three events and a few fixed-sign performance deletions. Version addressing (961f63c6) left content-addressed language in `arb.rs` and content-era helper signatures and docs in `tests.rs`; the V1 mirror's retirement (368da2a50) left `Levels`, the zipper, `join_matches_mirror`'s transitive-coverage claim, and the "same filter" premise in three module docs; the sync-then-erasure commits (262568f9e, b524e406) left `T: Send + Sync` bounds and a hand-written `PartialEq` with nothing to avoid. The SHA3 swap (4f18c347) broke a re-derivation discipline d800957e8 had followed for the geometry fixture's "attempt 1581". On the code side, `act` sorts its action list at every one of the 32 heights and re-sorts each touched fan on reassembly where one stable sort at entry would do; `join`'s divergent arm clones the fan three times over and hand-rolls a merge that `itertools::merge_join_by` (already used in `materialized/work/answer.rs`) spells in a `match`; and `Tree::hash` clones the whole root pair to borrow a field. The one verification finding of substance is that `join_associative`'s doc claims redaction-associativity coverage that no suite provides, and that no property in the tree states join's survivor formula directly, so the three algebraic laws would pass with the leaf verdict inverted.

Lines read: 4067 across the nine partition files, plus the out-of-partition lines each finding cites. Test code: `src/tree/tests.rs` (1897), `src/tree/arb.rs` (699), `src/tree/traverse/join/tests.rs` (52), `src/tree/traverse/unknown/tests.rs` (156), and the `#[cfg(test)]` `meter` and `panic_injection` modules inside `src/tree.rs` (616-712). Production: the rest of `src/tree.rs`, `traverse.rs` (19), `act.rs` (194), `join.rs` (238), `unknown.rs` (94). No cargo, just, or test command was run; every claim is by reading, grep, or read-only git.

## Findings

### tree-core-1: Three module docs say the wire mirror delegates deletion honoring to `traverse::unknown`; the mirror has its own filter, pinned differentially
- Where: src/tree.rs:58-61 (related: src/tree/traverse/join.rs:8-11, src/tree/traverse/unknown.rs:9-11, src/tree/mirror/streaming/materialized/unknown.rs:4-5 and 43-45, src/tree/mirror/streaming/materialized/unknown/tests.rs:15 and 76, src/tree/mirror/streaming/tests.rs:154-156)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep for `traverse::unknown|unknown::Unknown|Unknown::unknown` outside `src/tree/traverse/` returns only rustdoc links and the differential test's import and call at `materialized/unknown/tests.rs:15,76`; read `materialized/unknown.rs:1-45`, which defines its own `known` predicate and dominance classifier)
- Seen by: correctness; refutation: confirmed; history: deliberate-but-expired (true while the V1 mirror called `traverse::unknown::Unknown`; 368da2a50 deleted V1 on 2026-09-01, and the retirement plan's lattice item 6 scheduled `unknown.rs`'s doc restatement, which did not land)
- Owner-gated: no

Three module docs state that `join` and the wire mirror share one deletion filter, and `tree.rs` draws the crate's testing strategy from that premise ("every convergence property can be tested in-memory and trusted on the wire"). The streaming mirror does not call `traverse::unknown`: `materialized/unknown.rs` is a separate, erased, backend-generic implementation, and the only consumer of `Unknown::unknown` outside `traverse` is the differential test that uses it as an oracle. The observational identity is established by `agrees_with_materialized_oracle` and `streaming_matches_join_oracle`, not by shared code, so a maintainer changing `traverse::unknown` would wrongly believe the wire follows. Prose speaks in the present tense and states what IS; the stated basis of the test strategy must be the actual one.

Evidence:

    src/tree.rs
    58    //! [`mirror`] reconciles two trees over a wire. `join` and `mirror` are
    59    //! observationally identical — both delegate deletion honoring to the same
    60    //! filter — so every convergence property can be tested in-memory and
    61    //! trusted on the wire.

    src/tree/traverse/join.rs
    8     //! merged union once. It is observationally identical to mirroring two local
    9     //! trees, producing the same merged [`Root`](crate::tree::Root), because it
    10    //! delegates all version filtering to the same [`Unknown`] traversal the
    11    //! mirror uses.

    src/tree/traverse/unknown.rs
    9     //! Both the in-memory [`join`](mod@super::join) and the wire
    10    //! [`mirror`](super::mirror) delegate their version filtering here, which
    11    //! is what makes them observationally identical.

    src/tree/mirror/streaming/materialized/unknown.rs
    4     //! This is the streaming counterpart of
    5     //! [`traverse::unknown`](crate::tree::traverse::unknown): it prunes a single

Resolution: Reword the three sites to state the actual relationship: `join` and the streaming mirror implement the same deletion-honoring predicate (a subtree causally at or before the counterparty's version drops out) in two walks, and their agreement is pinned differentially (`agrees_with_materialized_oracle` against `traverse::unknown`; `streaming_matches_join_oracle` against `Tree::join`). In `unknown.rs`, state the module's present role: the in-memory filter `join` calls and the oracle the materialized pruner is checked against. Route the same rewording to `streaming/tests.rs:154-156` (out of partition). Acceptance: no prose under `src/tree/` says the mirror delegates to or uses `traverse::unknown`; each site names the differential test that carries the identity.

### tree-core-2: `#[derive(Debug, Eq)]` on `Tree<T>` and the four derives on `Snapshot<T>` impose `T` bounds no impl can use
- Where: src/tree.rs:90-91 (related: src/tree.rs:93-98, src/tree.rs:137-156, src/snapshot.rs:12-17, src/message.rs:533-539, src/tree/typed/untyped.rs:684-695, src/tree/typed/height.rs:16-21, src/peer.rs:182-195, src/rumors.rs:92 and 354)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: assessed (read; the bound each derive adds is language-defined: `#[derive(Clone)]` on `Foo<T>` expands to `impl<T: Clone> Clone for Foo<T>`, which `height.rs:16-21` states in the crate's own words)
- Seen by: perfapi (45), correctness (41); refutation: confirmed; history: no-rationale-found (300e2298 deliberately hand-wrote `PartialEq` for `Tree` to avoid a `T` bound and left `Debug`/`Eq` derived; `Snapshot`'s derives arrived in a WIP commit, 36df73797, and were never revisited)
- Owner-gated: yes: relaxing the bounds on public trait impls of `Snapshot<T>` is a public-API change, even though it is strictly widening

`Tree<T>` hand-writes `Clone`, `PartialEq`, and `Default` with no bound on `T` (137-156) and holds `PhantomData<fn() -> T>` so that "auto-traits never descend into `T`" (96-97), yet `#[derive(Debug, Eq)]` adds `impl<T: Debug> Debug` and `impl<T: Eq> Eq`, so `Tree<T>: PartialEq` for every `T` while `Tree<T>: Eq` only for `T: Eq`. Neither derived impl can inspect `T`: `Message`'s `Debug` prints only the serialized hex and `Node` equality is pointer-or-hash. The user-visible consequence lands in `Snapshot<T>`, whose `#[derive(Clone, Debug, PartialEq, Eq)]` makes `Snapshot<T>: Clone` require `T: Clone`, although `Rumors::snapshot` (an impl with no `T` bounds, rumors.rs:92) hands one out for any payload `Peer::seed` accepts (`Serialize + DeserializeOwned + Eq + Send + Sync + 'static`, no `Clone`) and its doc promises cloning "shares structure with the live set rather than copying it". Types-first, and the crate's own rule at `height.rs:16-21`: derived impls carry inherited `T: Trait` bounds, which is why the phantom is `fn() -> T`.

Evidence:

    src/tree.rs
    90    #[derive(Debug, Eq)]
    91    pub struct Tree<T> {
    ...
    95        /// Storage is erased ([`Message`] holds `dyn Any`); the facade's `T`
    96        /// names the type its faces downcast to (as `fn() -> T`, so
    97        /// auto-traits never descend into `T`).
    98        payload: PhantomData<fn() -> T>,

    src/snapshot.rs
    16    #[derive(Clone, Debug, PartialEq, Eq)]
    17    pub struct Snapshot<T> {

    src/tree/typed/height.rs
    16    // Hand-rolled trait impls below rather than `#[derive(...)]` because each
    17    // derived impl would carry an inherited `T: Trait` bound (e.g.
    18    // `#[derive(Clone)]` expands to `impl<T: Clone> Clone for S<T>`), which

Resolution: Replace the derives with bound-free impls: `impl<T> Debug for Tree<T>` (format `root` only) and `impl<T> Eq for Tree<T> {}`; the same four for `Snapshot<T>` (`Clone`, `Debug`, `PartialEq`, `Eq`, each without a `T` bound). Add a compile test (a doctest or a `tests/` unit) that clones and compares a `Snapshot<NotClone>` where `NotClone: Serialize + DeserializeOwned + Eq` only. Acceptance: the compile test builds; `cargo doc` shows `impl<T> Clone for Snapshot<T>` with no `T: Clone`; `Tree<T>` implements the same trait set for every `T`.

### tree-core-3: `Root.root` makes every read-path accessor spell `self.root.root` among three other things named `Root`
- Where: src/tree.rs:108-111 (related: src/tree.rs:226, 231, 236, 248, 266, 290, 308, 322, 343, 363; src/tree/arb.rs:113-116 and 517-522; src/tree/mirror/streaming/backend/local.rs:232-243)
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified (`grep -c 'self\.root\.root' src/tree.rs` = 10; `typed::node::Root` and `typed::height::Root` are both imported in this partition, act.rs:6 and arb.rs:6)
- Seen by: structure (10); refutation: confirmed; history: no-rationale-found (60a3b0569 introduced `Root { version, root }`; 02bf7e346 renamed `version` to `ceiling` and left `root`)
- Owner-gated: no

The node field of `tree::Root` is itself named `root`, so with `Tree.root: Root` every accessor reads `self.root.root` while `typed::node::Root` and `typed::height::Root` are also in scope, and the destructure in `backend/local.rs` shadows its own parameter. The cost is the disambiguation a reader performs on every line of the read path. The struct's own doc describes it as "the node structure ... and the causal ceiling that rides outside it", which `Root { ceiling, node }` would read as.

Evidence:

    108    pub struct Root {
    109        ceiling: Version,
    110        root: Option<typed::node::Root>,
    111    }
    ...
    226        self.root.root.as_ref().map(Node::floor)

Resolution: Rename the field to `node`, updating `tree.rs`, `arb.rs` (113-116, 518-521), and `backend/local.rs` (234-242). The struct is crate-internal, so nothing public moves. Acceptance: `self.root.root` no longer appears in the crate; `local.rs`'s destructure no longer shadows.

### tree-core-4: Erasure residue in `tree.rs`: a derivable hand-written `PartialEq for Root` and three unused `T: Send + Sync` where-clauses
- Where: src/tree.rs:131-135 (related: src/tree.rs:107-111, 419-423, 477-481, 567-570; src/tree/traverse/act.rs:39-43; src/tree/traverse/join.rs:57-63)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read the three method bodies, 424-452, 483-540, 571-612: none names `T`; both callees `traverse::act` and `traverse::join` are non-generic in `T`; `Root` is non-generic, so the manual `eq` is field-for-field what the derive produces)
- Seen by: structure (1), perfapi (53), correctness (41); refutation: confirmed; history: deliberate-but-expired (the bounds made the then-async traversal futures `Send`, edea11ac and 329d891b; 262568f9e made the traversals synchronous and b524e406 erased `T` from storage, both leaving the bounds; the manual `PartialEq` was written in 300e2298 for the then-generic `Root<P, T>` and b524e406 made `Root` bare while keeping the impl)
- Owner-gated: no

`Tree::act`, `Tree::react`, and `Tree::join` each carry `where T: Send + Sync` that nothing in their bodies uses: the messages they move were constructed already, and the traversal entries take `Option<Node<Root>>` and `Message` with no `T` in sight. `Root` gained `#[derive(Clone, Debug, Eq)]` when it lost its type parameter, but the hand-written `PartialEq` beside it stayed, and a manual impl beside a derive line invites the reader to look for a subtlety that is not there (contrast `Tree<T>`'s manual impls, which exist to avoid `T` bounds). Machinery outlives the constraint that justified it; the audit recurs at each phase boundary.

Evidence:

    107    #[derive(Clone, Debug, Eq)]
    108    pub struct Root {
    ...
    131    impl PartialEq for Root {
    132        fn eq(&self, other: &Self) -> bool {
    133            self.ceiling == other.ceiling && self.root == other.root
    134        }
    135    }
    ...
    419        pub fn act<I>(&mut self, party: &before::Party, actions: I) -> bool
    420        where
    421            T: Send + Sync,
    422            I: IntoIterator<Item = Action>,
    ...
    477        fn react<M, I>(&mut self, reactions: I) -> bool
    478        where
    479            T: Send + Sync,
    ...
    567        pub fn join(&mut self, other: Tree<T>) -> bool
    568        where
    569            T: Send + Sync,
    570        {

Resolution: Change `Root`'s attribute to `#[derive(Clone, Debug, PartialEq, Eq)]` and delete lines 131-135; delete the three `where T: Send + Sync` clauses. Leave `Tree<T>`'s manual `Clone`/`PartialEq`/`Default` (137-156), which the phantom justifies. Acceptance: the crate compiles; the production callers `batch.rs:143` and `peer/gossip.rs:869` are untouched; no method in `tree.rs` names a bound its body does not use.

### tree-core-5: `Iter` is unnameable from outside the crate yet is the public `IntoIterator::IntoIter` of `&Snapshot<T>`
- Where: src/tree.rs:174-176 (related: src/snapshot.rs:7, 105-112, 172-178; src/lib.rs:317, 322, 347)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read `src/lib.rs:304-349`: `mod snapshot` and `mod tree` are private and `Iter` appears in no `pub use`; `snapshot.rs:7` re-exports it only inside the private module; `snapshot.rs:172-174` names it as `type IntoIter = Iter<'a, T>` while `Snapshot::iter` at 105-107 returns an opaque `impl`)
- Seen by: perfapi (49); refutation: confirmed; history: deliberate-and-holds for the opacity (8dc0596ed made `Snapshot::iter` opaque and dropped `Iter` from the root to hide engine internals) but that same commit left `IntoIterator for &Snapshot` naming the now-hidden type; the finding brings that new evidence and is kept, owner-gated
- Owner-gated: yes: either resolution changes the public API (re-export `Iter`, or make `into_iter`'s type opaque, or re-export and return the concrete type from `iter`)

`Snapshot::iter` returns an opaque `impl DoubleEndedIterator + ExactSizeIterator + Send + Sync` while `(&snapshot).into_iter()` returns the concrete `Iter<'a, T>`, which no downstream path can name (`rumors::Iter` does not exist). A user who wants to hold the iterator in a struct must write `<&'a Snapshot<T> as IntoIterator>::IntoIter`. The two entry points to one walk have different, and in one case unnameable, types; std's convention (`Vec::iter` returns `slice::Iter`, and `IntoIterator for &Vec` uses the same type) is the idiom the design at ddc57045f originally followed. `Iter` also implements none of `Clone`, `Debug`, or `FusedIterator`.

Evidence:

    src/tree.rs
    174    /// An [`ExactSizeIterator`] (the live-message count is known up front) and a
    175    /// [`DoubleEndedIterator`].
    176    pub struct Iter<'a, T>(typed::Iter<'a>, PhantomData<fn() -> T>);

    src/snapshot.rs
    105        pub fn iter(
    106            &self,
    107        ) -> impl DoubleEndedIterator<Item = (&Version, Arc<T>)> + ExactSizeIterator + Send + Sync
    ...
    172    impl<'a, T: Send + Sync + 'static> IntoIterator for &'a Snapshot<T> {
    173        type Item = (&'a Version, Arc<T>);
    174        type IntoIter = Iter<'a, T>;

    src/lib.rs
    317    mod snapshot;
    ...
    347    pub use snapshot::Snapshot;

Resolution: Owner decision between (a) re-exporting `Iter` at the crate root, having `Snapshot::iter` return `Iter<'_, T>` concretely, and adding `Debug`/`Clone` where `typed::Iter` supports them and `FusedIterator` if `typed::Iter` keeps returning `None` after exhaustion; or (b) keeping opacity everywhere by boxing or wrapping `into_iter`'s type so no public signature names a hidden type. (a) matches std and 8dc0596ed's goal can be met by keeping `typed::Iter` private inside the wrapper. Acceptance: `Snapshot::iter` and `(&snapshot).into_iter()` have the same type; if that type is public, `cargo doc` lists it and a doctest stores it in a struct field by name.

### tree-core-6: `latest`/`earliest` read as a symmetric pair but bound different sets, and the tree-level docs do not say so
- Where: src/tree.rs:219-227 (related: src/snapshot.rs:40-56)
- Class / severity / confidence: api-surprise / nit / medium
- Provenance: verified (read both bodies: `latest` returns `Root::ceiling`, `earliest` the live leaves' floor; `snapshot.rs:52-53` carries the "unlike `latest`" disclaimer)
- Seen by: perfapi (55); refutation: confirmed; history: deliberate-and-holds (02bf7e346 introduced the pair as designed; the asymmetry is documented on `Snapshot` but not on `Tree`)
- Owner-gated: yes: any rename or new accessor is a public-API decision

`latest` is the causal ceiling, advanced by every send and redaction and, after any redaction, stamping no live message (`get(latest())` is `None`); `earliest` is the floor of the live leaves. Paired names promise paired semantics, and `Snapshot`'s doc has to disclaim the asymmetry. The tree-level docs ("Returns the latest version for the tree") do not state it at all, so a maintainer reading `Tree` alone gets no warning. The design is deliberate and documented at the public face, so this is a nit; the recorded rationale lives only in the commit message.

Evidence:

    219        /// Returns the latest version for the tree.
    220        pub fn latest(&self) -> &Version {
    221            &self.root.ceiling
    222        }
    223
    224        /// Returns the earliest version present in the tree.
    225        pub fn earliest(&self) -> Option<&Version> {
    226            self.root.root.as_ref().map(Node::floor)
    227        }

Resolution: At minimum, state at 219 and 224 what each bounds (the causal ceiling of every action; the floor of the live leaves) so the tree-level docs match `Snapshot`'s. Owner option: rename the ceiling accessor (`frontier()`) so `latest`/`earliest` can be a symmetric live pair, or expose the live ceiling under a distinct name. Acceptance: `Tree`'s two docs state the asymmetry; if renamed, `Snapshot`'s docs no longer need an "unlike `latest`" clause.

### tree-core-7: `Tree::hash` clones the whole `Root` through a one-caller `From` impl to borrow a field
- Where: src/tree.rs:273-278 (related: src/tree.rs:113-117, src/tree/typed/node.rs:409-417)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep for `Option<typed::node::Root>` and `root.clone().into()` over `src/`: the `From<Root> for Option<typed::node::Root>` impl has exactly one consumer, tree.rs:277; the other root `.into()` calls target `StreamingRoot<Local>`; `Node::root_hash` takes `&Option<Root>`, node.rs:410)
- Seen by: structure (2), correctness (40), perfapi (46); refutation: confirmed; history: no-rationale-found (both the line and the impl date to 60a3b0569, whose message does not mention the conversion)
- Owner-gated: no

`hash()` clones the `Root` (a `Version` handle bump and a node `Arc` bump, then two drops), converts it by value through the `From` impl, and borrows the result, when `&self.root.root` is already exactly the `&Option<typed::node::Root>` that `Node::root_hash` takes. The `From` impl exists only to serve this line. `hash()` is the read the crate meters inside commit critical sections (616-623), so gratuitous work here runs in the wrong place, and the conversion reads as if it did something.

Evidence:

    113    impl From<Root> for Option<typed::node::Root> {
    114        fn from(value: Root) -> Self {
    115            value.root
    116        }
    117    }
    ...
    273        /// Returns the root hash for the tree.
    274        pub fn hash(&self) -> [u8; MERKLE_HASH_LEN] {
    275            #[cfg(test)]
    276            meter::record_root_hash_read();
    277            Node::root_hash(&self.root.clone().into()).into()
    278        }

Resolution: `Node::root_hash(&self.root.root).into()`; delete the `From<Root> for Option<typed::node::Root>` impl. Acceptance: no `.into()` on a `tree::Root` remains in the crate; `hash()` performs no clone; `empty_tree_hash_matches_reference` and the `root_hash_read_meter_is_live` pins in `crate::tests` stay green (the meter call precedes the read).

### tree-core-8: `warm_caches` says it forces every memo but skips `version_bytes`, which the greeting reads
- Where: src/tree.rs:297-313 (related: src/tree/typed/untyped.rs:147-158, src/tree/mirror/streaming/backend.rs:400-413, src/tree/mirror/streaming/materialized.rs:521 and 577, src/snapshot.rs:163-169, benches/gossip_fixed.rs, benches/gossip_grid.rs)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (read `untyped.rs:147-158`: a branch carries `bounds`, `version_bytes`, and the hash as `OnceLock`s, and `version_bytes` "forces" the bounds; grep shows `Root::<B>::max_version_bytes` at `backend.rs:408-411` reads `node.version_bytes()` for the greeting at `materialized.rs:521,577`; `warm_caches` forces `hash`, `ceiling`, `floor` only)
- Seen by: perfapi (50); refutation: confirmed; history: deliberate-but-expired (611b325de forced the three memos that existed; 206c288ae made `version_bytes` a lazy `OnceLock` without extending `warm_caches`)
- Owner-gated: no

The doc says the method forces "every lazily-memoized structural value"; a branch carries four (hash, the bounds span, `version_bytes`) and the body forces three. `version_bytes` is production-reachable: the streaming greeting reads it once per tree lineage, so the gossip benches that call `warm_caches` (through `Rumors::warm_caches`) before timing still pay one O(branches) fold inside their first timed session. Criterion's warm-up likely absorbs it, so the practical harm is small; the defect is a calibration helper whose contract ("every") its body does not meet. Statement faithfulness: never stronger than shown.

Evidence:

    297        /// Forces every lazily-memoized structural value — the observable hash
    298        /// and the ceiling/floor version bounds — for the whole tree.
    ...
    307        pub fn warm_caches(&self) {
    308            if let Some(root) = &self.root.root {
    309                let _ = root.hash();
    310                let _ = root.ceiling();
    311                let _ = root.floor();
    312            }
    313        }

    src/tree/typed/untyped.rs
    155            /// Like the bounds span (which it forces), this must be reset
    156            /// whenever the branch's children change, but not when its
    157            /// prefix does.
    158            version_bytes: OnceLock<usize>,

Resolution: Add `let _ = root.version_bytes();` (which forces the bounds span, so the `ceiling`/`floor` calls become redundant and can go), and word the doc as the list of memos it forces so a future memo cannot fall outside "every" unnoticed; mirror the wording at `snapshot.rs:163-165`. Acceptance: a test calls `warm_caches` then observes the memo set (below). Construction: add a `#[cfg(test)]` probe on `untyped::Node` returning whether the branch's `version_bytes` `OnceLock` is populated (`get().is_some()`); build a tree of at least two leaves, call `warm_caches()`, assert the probe is true. The assertion fails at this commit and passes after the fix.

### tree-core-9: The root ceiling has three names: "version vector", "causal ceiling", "version"
- Where: src/tree.rs:370-371 (related: src/tree.rs:374, 456, 549; src/tree/traverse/join.rs:17, 19, 45, 111; src/tree/tests.rs:610; contrast src/tree.rs:102, 109, 407, 561)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (`grep -rn 'version vector' src/` returns exactly the nine sites listed, all in this partition; the field is `Root::ceiling` and the same docs say "causal ceiling" at 102, 407, 561)
- Seen by: prose (28); refutation: confirmed; history: deliberate-but-expired (60a3b0569 named the field `version`; 02bf7e346 renamed it `ceiling` without sweeping the prose)
- Owner-gated: no

`act`'s doc calls `Root::ceiling` the tree's "internal version vector" while the same doc's changed-flag section calls it "the causal ceiling", and `join.rs` uses "version vector" throughout. `before` does use "version vector" for the usage pattern, so the term is defensible, but one concept should carry one name, and the code's own name is `ceiling`.

Evidence:

    370        /// Applies the specified actions as a batch to the tree, advancing its
    371        /// internal version vector once per action.

Resolution: Use "ceiling" (or "causal ceiling") for `Root::ceiling` at the nine sites; reserve "version" for a leaf's stamp. Acceptance: `grep -rn 'version vector' src/tree` returns nothing.

### tree-core-10: `act`'s rustdoc states a measured "2-3x" that no committed bench produces
- Where: src/tree.rs:387-390 (related: benches/in_memory.rs:23-39 and 91-92)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '2-3x' src/ benches/ README.md` finds tree.rs:389 only; `benches/in_memory.rs` has a `batch_insert` group and no one-at-a-time counterpart; no bench file calls `.act(`)
- Seen by: prose (22), correctness (42), perfapi (51); refutation: confirmed; history: deliberate-but-expired (8231541a0 landed the figure with a bench comparing `act/insert_batch_into_empty` against `act/insert_one_by_one`; 94da12b59 deleted that bench and the figure survived two rewordings)
- Owner-gated: no

The batching speedup is given a number that nothing in the tree can re-derive; it once had a bench and no longer does, so it rots as the fan, hashing, or allocator change (and tree-core-27's sort deletion would move it). A number you were handed is a hypothesis; an approximation survives in prose only with its measurement named.

Evidence:

    387        /// A batch is applied to the tree in a single traversal, which is more
    388        /// efficient than applying its actions one at a time: in theory an
    389        /// O(log n) speedup over one-by-one insertion, in practice about 2-3x
    390        /// since the log base is 256.

Resolution: Either drop the figure and keep the qualitative claim (one traversal instead of one per action, shared spine work amortized), or add a `single_insert` column beside `batch_insert` in `benches/in_memory.rs` and cite it by name. Acceptance: the doc either states no figure or names a committed bench that produces it.

### tree-core-11: The leaf-level contract prose does not match `Act for Z`: the "morally associative" version clause, the observer's "once per effectual action", and the `# Panics` reach
- Where: src/tree.rs:392-397 (related: src/tree/traverse/act.rs:17-22, 30-38, 139-190; src/tree/tests.rs:512-548)
- Class / severity / confidence: documentation / medium / high
- Provenance: assessed (traced `Act for Z`, act.rs:139-192, against the three prose sites; the two committed pins at tests.rs:530 and 547 agree with the trace and contradict the paragraph at 392-397)
- Seen by: prose (12), correctness (37); refutation: confirmed (37 states the exact rule; the refutation's new item on the `# Panics` reach is folded in here); history: deliberate-but-expired (1f83e9c74 wrote the paragraph when forgets did not tick, per 114cc9998; fc1ea02d4 and 9ebd1b8d2 amended around it without re-deriving; the observer wording at act.rs:19-22 is from 262568f9e while the once-per-key observation with `greatest_version` is 052d1f95b, never reconciled)
- Owner-gated: no

`Z::act` joins every action's version at a key into `greatest_version` (line 148, before the skip), and fires `on_action` once per key group, with that join, iff the leaf existed before or exists after (186-190). Three prose sites describe something else. (a) `Tree::act` says the version "is incremented once per changed key, regardless of how many actions pertain to it": `[Insert, Forget(same path)]` in one batch leaves the ceiling untouched (no observation when the net effect is nil; pinned at tests.rs:547 `Version::new()`), while the same pair across two calls advances it by both ticks (tests.rs:530 `version_for(&party, 2)`), and `[Insert, Forget(absent), Insert]` leaves it three ticks along with two changed keys. The actual rule: the ceiling absorbs the join of each observed group, that is, the tick of the last observed action; ticks of trailing unobserved actions are reissued by the next batch. (b) `traverse::act` says `on_action` "fires once per *effectual* action ... with that action's version": it fires per key group, with the joined version, and it fires for a group whose every action was skipped as causally prior and for an identical re-insert (neither effectual; tree.rs:409-417 acknowledges the first as the flag's conservative case), while it does not fire for an effectual Insert+Forget pair on a fresh key. (c) The `# Panics` section says an insert landing on a live leaf "disagreeing with it on version or payload" panics, but the causal skip at 152-159 runs before the identity check at 169-176, so an insert whose version is strictly prior to the resident leaf's is dropped, never asserted; `act_destructor_unwind_leaves_tree_byte_identical` (tests.rs:1683-1720) relies on exactly that. The observer is the sole source of the changed flag and of ceiling movement, so its stated semantics are load-bearing for `Batch::commit`'s wakeup; and a maintainer reading the doc and the two pins today gets two answers.

Evidence:

    src/tree.rs
    392        /// This function is "morally associative": partitioning a sequence of
    393        /// actions across multiple `act` calls produces the same tree as a
    394        /// single `act` over their concatenation, except possibly for the tree's
    395        /// version when several actions address the same key. In that case the
    396        /// version is incremented once per changed key, regardless of how many
    397        /// actions pertain to it.

    src/tree/traverse/act.rs
    19    /// `on_action` fires once per *effectual* action — a leaf inserted, replaced,
    20    /// or removed — with that action's version. A forget of a leaf that never
    21    /// existed observes nothing, which is what lets the caller join versions only
    22    /// for actions that changed the tree.
    ...
    32    /// Panics if an insert lands on a live leaf disagreeing with it on
    33    /// version or payload: version reuse. No input reaches that state —
    ...
    148            greatest_version |= &version;
    ...
    152            if version
    153                < node
    154                    .as_ref()
    155                    .map(|n| n.ceiling())
    156                    .unwrap_or(&Version::default())
    157            {
    158                continue;
    159            }
    ...
    185        // Observe the action, provided that the net action wasn't nil
    186        match (existed_before, &node) {
    187            // The node stayed empty
    188            (false, None) => {}
    189            _ => on_action(&greatest_version),
    190        }

Resolution: Restate tree.rs:392-397 as the implemented rule: the ceiling advances to the version of the last action whose key was observed (a key whose leaf existed before or exists after the batch); a batch whose trailing actions touch only never-present keys leaves those ticks unabsorbed, so the next batch reissues them (harmless: nothing carried them). Drop the scare-quoted coinage or define it in the sentence. Restate act.rs:19-22 as: fires once per key the batch touches whose leaf existed before or exists after, with the join of that key's action versions. Qualify act.rs:32-33: the panic covers an insert not causally prior to the resident leaf; a strictly prior insert is skipped. Add a unit test for `[Insert, Forget(absent path), Insert]` pinning `latest()` at three ticks. Acceptance: the paragraph at 392-397 predicts both `insert_then_delete_is_empty` and `insert_and_delete_same_batch_is_empty` without an exception clause; the observer doc names the key-group rule; the `# Panics` section names the skip's precedence; the new test is committed.

### tree-core-12: Moralized and dialect vocabulary: "honest" for trees, ticks, iterators, a simulation, and a lint; intensifier "real"/"genuinely"; "seam", "cashed out", "priced"
- Where: src/tree.rs:412-413 (related: src/tree.rs:29, 261, 521, 635, 675; src/tree/traverse/join.rs:55, 104; src/tree/traverse/act.rs:79; src/tree/traverse/unknown.rs:4; src/tree/arb.rs:303, 336, 396, 419, 429, 461, 479, 524, 531, 540; src/tree/tests.rs:248, 427, 750, 1024, 1036, 1124, 1133, 1306, 1546)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep for `honest`, `\b(real|really|genuinely)\b`, `seam|cashed out|priced` over the partition; sites as listed)
- Seen by: prose (27); refutation: confirmed (calibration: "honestly built tree" is a defensible extension of the model's "honest peer"; the clearly moralized uses are the lint, the iterator, and the simulation); history: no-rationale-found
- Owner-gated: no

AGENTS.md reserves "honest" for the peer model (authenticated-honest-peer). Here it is borrowed for trees ("every honestly built tree", where "conforming", already used at tree.rs:89, is the precise word), for ticks a test performs, for an `ExactSizeIterator` ("honest" for exact), for a simulation, and twice for a clippy allow ("keeps `-D warnings` honest"). "Real"/"genuinely"/"really" appear as intensifiers (join.rs:55 "really means"; tests.rs:248 "real CRDT semantics", 750, 1024, 1036, 1546) beside legitimate real-versus-injected contrasts (tree.rs:521, tests.rs:1498) that would read better with the contrast spelled out. "The local join seam" (arb.rs:524), "cashed out" (unknown.rs:4), and "priced outside" (tree.rs:261) are unanchored metaphors.

Evidence:

    412        ///   ceiling — which `act` and `join` both maintain, so every honestly
    413        ///   built tree qualifies — because each action ticks strictly above the
    ...
    635        // `-D warnings` honest on every platform the gate runs.

    src/tree/tests.rs
    426        /// yield `n` leaves, and `is_empty` track `n == 0`. `iter` is moreover an
    427        /// honest `ExactSizeIterator`: its reported length starts at `n` and falls

Resolution: Trees: "conforming" or "act/join-built". Ticks: "in-protocol ticks". Lint and simulation: "keeps `-D warnings` clean", "keeps the simulation in agreement with the builder". Iterator: "exact". Delete intensifier uses; where a contrast is meant, name it ("the destructor source, as opposed to the injected fuse"). "seam" to "the in-memory join path"; "cashed out" to "implemented"; "priced" to "accounted". Acceptance: `grep -rn -i '\bhonest' src/tree.rs src/tree/arb.rs src/tree/traverse src/tree/tests.rs` returns nothing outside the peer-model sense; intensifier uses are gone.

### tree-core-13: `act`'s body comments are fragmented and partly redundant, and both `Action::Insert` docs misstate where the version comes from
- Where: src/tree.rs:424-446 (related: src/tree.rs:161, 380; src/tree/traverse/act.rs:11)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read)
- Seen by: prose (23); refutation: confirmed; history: no-rationale-found (the blocks accreted across 114cc9998, fc1ea02d4, and 262568f9e; the `Insert` clause "tagged at the current version" was accurate when a batch shared one version and expired at 114cc9998)
- Owner-gated: no

Four comment blocks say overlapping things: 424-430 (tick per action, forgets above inserts, the deletion-honoring rationale) runs without a blank line into 431-434 (running version, lazy reactions); 437-439 restates uniqueness with a different reason ("wrongly early-aborts when versions compare equal" is the mirror's equal-ceiling short circuit, not per-action uniqueness); 443-446 carries rustdoc link syntax inside a `//` comment where it cannot resolve; and the rustdoc at 380 defers to "the body comment" for a reason 376-379 already gives. Separately, tree.rs:161 says an insert is "tagged at the current version by your own party" (it is tagged at the post-tick version, and the second person is odd in a maintainer doc), and act.rs:11 says "tagged by a version at a party" though the version rides in the action tuple, not the `Message`. Comments state what the code cannot show, once, at the branch that needs it.

Evidence:

    161        /// Insert some value, tagged at the current version by your own party.
    ...
    431            // The running version, advanced in place per action; each action
    432            // clones the post-tick value as the committed version that keys
    433            // its leaf. The reactions flow into `react` lazily; the whole
    434            // chain materializes only once, at the traversal's radix sort.
    ...
    437                // Advance the version. It must be unique for every action
    438                // applied to the tree; otherwise the mirror protocol
    439                // wrongly early-aborts when versions compare equal.
    ...
    443                // Convert unversioned, unlocalized actions into reactions
    444                // independent of our party and current version. The path is
    445                // derived from the post-tick version, which is unique per
    446                // insert (see [`typed::Path::for_leaf`]).

Resolution: Merge 424-446 into one comment: one paragraph on why every action ticks (a fresh path per insert; forgets strictly above any prior insert so the ceiling moves and equal-ceiling early completion cannot hide a redaction), one line on the lazy chain materializing at the walk's sort; drop the link syntax. At 380, delete "see the body comment". Reword 161 to "Insert a message; `act` stamps it with the version the batch ticks to" and act.rs:11 to "Insert the message; the version rides alongside in the action tuple". Acceptance: one comment block precedes `self.react(`; no `[`...`]` link syntax inside `//` comments in `tree.rs`; both `Insert` variant docs describe where the version comes from.

### tree-core-14: Insert-or-forget has three spellings on one call path, and `react`'s `M: Into<Option<Message>>` serves only test call sites
- Where: src/tree.rs:477-497 (related: src/tree.rs:158-165, 436-452; src/tree/traverse/act.rs:8-15; src/tree/arb.rs:5, 133, 194, 307; src/tree/tests.rs:98-105)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (`grep -rn '\.react(' src tests` outside `tree/tests.rs`: `Tree::react`'s only production caller is `act`, tree.rs:436; the `levels.rs` hits are `resolver.react`, a different method; `Into<Option<` appears once in the crate, tree.rs:480; `arb.rs` imports `traverse::{Action, act}` at line 5 and re-imports `crate::tree::{Action, Tree}` inside three bodies)
- Seen by: structure (5), perfapi (54); refutation: confirmed; history: deliberate-but-expired (47c9c9013 added the generic so callers of the then-public `react` could pass `unknown()`'s output directly; `react` went private in 8e348feff)
- Owner-gated: no

`tree::Action { Insert(Message), Forget(Path) }` is converted by `act` into `Option<Message>` (447-451), which `react` accepts through a generic `M: Into<Option<Message>>` and converts back into `traverse::Action { Insert(Message), Forget }` (493-496) before the walk. Production has one caller of `react` (`act`, with `M = Option<Message>`); the generic exists so tests can pass a bare `Message`. The middle representation and the type parameter exist only to bridge the other two, and the two enums sharing the name `Action` force qualified paths and function-local imports wherever both are in scope.

Evidence:

    477        fn react<M, I>(&mut self, reactions: I) -> bool
    478        where
    479            T: Send + Sync,
    480            M: Into<Option<Message>>,
    481            I: IntoIterator<Item = (typed::Path, Version, M)>,
    ...
    491            let actions: Vec<_> = reactions
    492                .into_iter()
    493                .map(|(path, version, message)| match message.into() {
    494                    None => (path, version, traverse::Action::Forget),
    495                    Some(value) => (path, version, traverse::Action::Insert(value)),
    496                })
    497                .collect();

    src/tree/arb.rs
    5     use crate::tree::traverse::{Action, act};
    ...
    133        use crate::tree::{Action, Tree};

Resolution: Have `react` take `I: IntoIterator<Item = (typed::Path, Version, traverse::Action)>` and drop `M`; build `traverse::Action::Insert(value)` / `traverse::Action::Forget` directly in `act` at 447-450 so the `match` at 493-496 becomes a plain `collect()`. Tests then construct `traverse::Action` in the `insert_at`/`event` helpers (the dozen inline `(path, version, msg)` tuples at tests.rs:1701, 1712, 1810, 1813, 1839, 1841, 1865, 1867, 1882, 1883, 1895, 1896 and the `event` closures at 258-264 and 280-284 change). The reverse direction (drop `traverse::Action` for `Option<Message>`, since they are isomorphic) also removes one spelling. Whichever enum survives, give the internal one a distinct name so `arb.rs` needs no function-local imports. Acceptance: `react` has one type parameter; no `Into<Option<Message>>` in the crate; `arb.rs` has no `use crate::tree::{Action, Tree};` inside function bodies.

### tree-core-15: Em-dashes in `//` comments at 21 sites in the partition
- Where: src/tree.rs:487-487 (related: src/tree.rs:504, 508, 533, 604; src/tree/tests.rs:302, 303, 1093, 1708, 1709, 1748; src/tree/traverse/act.rs:165; src/tree/traverse/join.rs:133, 142, 143; src/tree/traverse/unknown.rs:39, 40; src/tree/traverse/unknown/tests.rs:50; src/tree/arb.rs:330, 331, 371)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -n '^\s*//[^/!].*—'` over the nine files returns exactly the 21 sites; over all of `src/` it returns 116, so this is a crate-wide sweep item)
- Seen by: prose (26); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The owner's register rule is spaced double-hyphens in code comments and chat, true em-dashes in rendered prose (`///`, `//!`), for terminal compatibility. Rustdoc sites are correctly excluded here.

Evidence:

    487            // mechanism — the commit section below defends against every unwind,

Resolution: Replace `—` with ` -- ` (or a colon or semicolon where the sentence reads better) at the 21 `//` sites; leave `///` and `//!` alone. Best done as one crate-wide sweep. Acceptance: the grep in the provenance line returns nothing for the partition.

### tree-core-16: `react`'s atomicity comment attributes its mid-walk destructor source to a "wire-apply path" that does not exist
- Where: src/tree.rs:507-513 (related: src/tree/tests.rs:669-672 and 1689-1691; src/tree/traverse/act.rs:36-37; src/peer/gossip.rs:869; src/batch.rs:143)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`grep -rn wire-apply src/` returns tree.rs:511 and tests.rs:1690 only; `grep -rn '\.react('` shows `act` as `Tree::react`'s only production caller; the wire path installs merged trees through `inner.tree.join(merged)` at gossip.rs:869 and local batches through `inner.tree.act(party, actions)` at batch.rs:143; act.rs:36-37 states "no wire-derived leaf passes through this walk")
- Seen by: prose (11); refutation: confirmed (at 2d3a86d3f, the commit that introduced the phrase, `act` was already the sole caller, so the sentence was inaccurate when written); history: deliberate-but-expired (`react` was the foreign-apply entry in April, 47c9c9013; that role ended with `Tree::join`, 329d891bf, and `react` went private in 8e348feff; f3fef7bc1 the next day stated the opposite premise in the same walk)
- Owner-gated: no

The commit-section comment justifies the mid-walk `T`-destructor hazard by "the wire-apply path" where "those messages are freshly deserialized". No wire leaf passes through `react`. The mechanism itself is real on the `act` path (a `Batch` holds the only handles to its `Message`s, so a displaced insert or causally-skipped action dropped mid-walk is the last handle), so the conclusion stands while its stated premise is a ghost that contradicts act.rs:36-37 fourteen lines into the same walk. The same ghost appears in the destructor pin's doc ("exactly the wire-apply shape") and in `react_idempotent`'s rationale (re-delivery "in the face of retries or out-of-order transport"). A comment describing a code path the code does not have is a ghost reference, and here it misstates the premise a correctness argument rests on.

Evidence:

    507            // Panic atomicity: nothing of `self` mutates until the commit point
    508            // below, whatever the unwind's origin — a user type's destructor or
    509            // our own bug. Unwind sources survive inside this walk: the leaf
    510            // level drops causally-skipped action messages and batch-internal
    511            // displaced inserts mid-walk, and on the wire-apply path those
    512            // messages are freshly deserialized, so the drop is the last handle
    513            // and runs `T`'s destructor.

    src/tree/tests.rs
    1689    /// skips it and drops the action's message mid-walk — and that message is
    1690    /// the payload's last handle, exactly the wire-apply shape, where every
    1691    /// incoming message is freshly deserialized. The caught panic must be the

    src/tree/traverse/act.rs
    36    /// party linearity keeps regions disjoint), and no wire-derived leaf
    37    /// passes through this walk — so the panic marks a bug in this crate,

Resolution: Re-state the unwind-source premise in terms of the path that exists: `act` moves the batch's `Message`s into the walk (a `Batch` holds the only handles), so a displaced insert or a causally-skipped action dropped mid-walk is the last handle and runs `T`'s destructor. Delete the wire-apply clause at tree.rs:511-512 and tests.rs:1689-1691; reword tests.rs:669-672 so `react_idempotent` speaks of re-applying a versioned batch, not of transport retries. Leave act.rs:36-37 as is. Acceptance: `grep -rn 'wire-apply' src/` returns nothing; the react comment and both test docs name only the local batch path as the destructor source; act.rs's `# Panics` premise and the react comment agree.

### tree-core-17: "proves the defense total" attributes to a fixed-point fuse what the commit-point structure provides
- Where: src/tree.rs:660-661 (related: src/tree/tests.rs:1641-1643; src/tree/traverse/act.rs:44-52 and 77-81; src/tree/traverse/join.rs:64-70 and 102-106)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read: the fuse fires at the walk's entry and once per branch-level step; a pin armed at one depth demonstrates the defense at that fire point)
- Seen by: prose (29); refutation: confirmed; history: deliberate-and-holds for the intent (2d3a86d3f's message says the fuse pin is "proving the defense total against arbitrary internal panics"); the finding is a wording judgment about "proves", kept as a nit
- Owner-gated: no

Totality follows from the structural argument the comments already make (nothing mutates before the commit point; replace, assign, then drop). The fuse pins demonstrate that argument at chosen fire points; "proves ... total" attributes to the tests what the structure provides. Statement faithfulness: never stronger than shown, in informal claims as in formal ones.

Evidence:

    659        /// earlier fire points ran: deep enough to land after copy-on-write work
    660        /// has begun. The fuse stands in for an arbitrary internal bug and proves
    661        /// the defense total; the destructor-source pins beside the fuse pins

    src/tree/tests.rs
    1641    /// ceiling must both come through unchanged. Together with the
    1642    /// destructor-source pin beside it, this proves the defense total: a
    1643    /// panic of any origin inside the walk publishes nothing.

Resolution: "demonstrates the defense at an arbitrary internal fire point; totality is the commit-point structure above, which these pins exercise" at both sites. Acceptance: neither site claims the tests prove totality; each names the structural argument as its source.

### tree-core-18: Two `tests.rs` helper docs state preconditions that are false: `distinct_bytes` buys no path distinctness, and `idx` handles no generated strings
- Where: src/tree/tests.rs:29-33 (related: src/tree/tests.rs:56-65, 265-271, 714-718; src/tree/typed/path.rs:35-38; src/tree.rs:11-12)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`Path::for_leaf` hashes `version.as_bytes()` alone, path.rs:35-38; every `party_of(` argument in tests.rs is a literal, a `&party` bound to a literal, `[b'a' + (i % 5) as u8]` at 1037, or `label` from a literal array at 1460; `idx` is first byte lowercased minus `b'a'` mod 16)
- Seen by: prose (20, 21); refutation: confirmed; history: deliberate-but-expired for `distinct_bytes` (at c6a448972 `leaf_path(party, scalar, &value)` hashed the payload; 961f63c6 removed payload bytes from paths without this doc); no-rationale-found for `idx` (the "proptest-generated strings" clause described no caller at birth, 1af0ab9aa)
- Owner-gated: no

`distinct_bytes`'s doc says deduplication makes "every element map to a unique leaf path" and that properties need "no two inserts collide by path". Paths derive from versions, never bytes, so distinct bytes buy no placement; two inserts at one version collide whatever their bytes (and trip the version-reuse assert when the bytes differ, tests.rs:1877). Distinctness is still needed, for a different reason: the payload-keyed maps `index_of` (265-271) and `meta_by_value` (714-718) must be injective. `idx`'s doc says distinct labels "or proptest-generated strings" map to distinct indices; no caller passes a generated string, and the first-byte-mod-16 mapping sends "ab"/"ac" or "a"/"q" to one index, so the claim would be false if exercised. A helper's stated precondition must be one the code enforces or the callers respect.

Evidence:

    29    /// Generate a vector of distinct `Bytes`, deduplicated so every element maps
    30    /// to a unique leaf path when inserted under the same party and version.
    31    ///
    32    /// Many of the hash-invariance properties below are only meaningful when no two
    33    /// inserts collide by path; collision semantics are exercised separately.
    ...
    56    /// Map a human-readable party label to a small disjoint-party index.
    57    ///
    58    /// The distinct labels the tests use ("A"/"B"/"C"/"P", or proptest-generated
    59    /// strings) map to distinct indices, so [`party_of`] yields mutually
    60    /// disjoint parties.
    61    fn idx(label: impl AsRef<[u8]>) -> usize {
    62        label.as_ref().first().map_or(0, |b| {
    63            (b.to_ascii_lowercase().wrapping_sub(b'a') as usize) % 16
    64        })
    65    }

Resolution: `distinct_bytes`: "Generate a vector of distinct `Bytes`. Paths derive from versions, so distinctness buys nothing for placement; it keeps the payload-keyed maps the shuffle properties build (`index_of`, `meta_by_value`) injective." `idx`: drop "or proptest-generated strings" and state the rule: "Labels are single letters; the index is the letter's alphabet position mod 16, so the letters one test mixes must be distinct mod 16." Acceptance: both docs state the reason the helper is actually needed and no claim about paths or generated strings.

### tree-core-19: Test-helper idiom nits in `tests.rs`: `insert_at` carries two parameters that only re-derive the first, a wrapper that only renames, qualified `arb` paths, and a stray import
- Where: src/tree/tests.rs:98-105 (related: src/tree/tests.rs:10, 1005-1010, 260, 887, 970, 995, 1183, 1201, 1220, 1242, 1311, 1312, 1350, 1351; src/tree.rs:255-271)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (all nine `insert_at` call sites read, 364, 416, 642, 651, 682, 723, 729, 759, 769: each passes `version_for(party, scalar)` or a value equal to it, 723/729 via `meta_by_value` whose entries are `(version_for(party, i+1), i+1)`, 759/769 via `recorded`, the single-party `latest()` ticked once; `naive_max_version_bytes` is `tree.max_bound_bytes()`; `use serde::Serialize;` at line 10 is used at 1606)
- Seen by: structure (7), prose (31); refutation: confirmed; history: deliberate-but-expired for `insert_at` (at c6a448972 the path was `leaf_path(party, scalar, &value)`, content-derived; 961f63c6 rewrote 517 lines of tests.rs without dropping the redundant parameters) and for the wrapper (222026c12 gave it an independent body; 206c288ae moved the walk into `Tree::max_bound_bytes` and left the wrapper as a rename)
- Owner-gated: no

`insert_at(version, party, scalar, value)` derives the path from `(party, scalar)` while also taking `version`; every caller passes `version_for(party, scalar)`, so the two extra parameters re-derive what the first already is and admit a (path, version) pair the tree would reject. `naive_max_version_bytes` is a one-line rename of `tree.max_bound_bytes()` whose doc claims a "direct walk" the body does not perform (the walk and its doc live at tree.rs:255-271, and the two docs even apply "oracle" to opposite sides). `crate::tree::arb::` is spelled at eight sites and `super::arb::` at four while `use super::*;` is in scope. `use serde::Serialize;` sits outside the import block, glued to the next item's doc comment.

Evidence:

    98    fn insert_at(
    99        version: Version,
    100       party: impl AsRef<[u8]>,
    101       scalar: u64,
    102       value: Bytes,
    103   ) -> (Path, Version, Message) {
    104       (leaf_path(party, scalar), version, msg(value))
    105   }
    ...
    1005  /// The maximum canonical encoding over every version bound a tree holds
    1006  /// — leaf versions and every branch's ceiling and floor — recomputed by
    1007  /// direct walk: the oracle `Tree::max_version_bytes` must match.
    1008  fn naive_max_version_bytes(tree: &Tree<Bytes>) -> usize {
    1009      tree.max_bound_bytes()
    1010  }

Resolution: `fn insert_at(version: Version, value: Bytes) -> (Path, Version, Message) { (Path::for_leaf(&version), version, msg(value)) }` and simplify the nine call sites (several `party`/`scalar` locals then go). Inline `tree.max_bound_bytes()` at its four call sites, or keep the wrapper with a doc that makes no independent claim ("the oracle `Tree::max_bound_bytes`, under the name the assertions read"). Add `use super::arb;` once and drop the qualified spellings. Move the `serde` import into the import block. Acceptance: `insert_at` has two parameters; no `crate::tree::arb::` or `super::arb::` at a use site in the file; imports form one block.

### tree-core-20: `span_door_traffic`'s docs lean on `before`-internal metaphors a rumors maintainer cannot decode
- Where: src/tree/tests.rs:1385-1397 (related: src/tree/tests.rs:1420-1425, 1449-1455; crates/before/src/laws.rs and meter.rs for the vocabulary's home)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (the terms "door", "rung", "pair hull", "fringe" are `before`'s: `crates/before/src/laws.rs` has 24 hits for `door`, `meter.rs` 27 for rung/pair hull/fringe; none is defined or linked in `tests.rs`)
- Seen by: prose (33); refutation: confirmed (some borrowed vocabulary is inherent because the counters read are `before::meter::span_traffic`); history: no-rationale-found (b70f7d149 wrote the doc from its own commit message)
- Owner-gated: no

"bounds-memo door", "span-ladder rung", "pair hull", "fringe regime", "emitting walk" appear in the first sentence with no definition or link, so a reader cannot tell which counters the test reads or why the two verdicts differ. A doc comment's first sentence stands alone in a module listing; unanchored metaphors promoted to jargon are a dialect tell.

Evidence:

    1385    /// The pair-hull traffic mix at the tree's bounds-memo door, in
    1386    /// `before`'s span-ladder rung counters.

Resolution: Open with the mechanism in plain terms: "How the tree's bounds memos exercise `before`'s span-combining paths: comparable pairs take the fast comparison path, concurrent pairs take the emitting walk; the counters are `before::meter::span_traffic`." Link each borrowed term to its `before` definition on first use or drop it. Acceptance: the first sentence is readable without `before`'s internals; each remaining borrowed term links to its definition.

### tree-core-21: Generator docs in `arb.rs` narrate the streaming-deadlock incident instead of stating the geometry
- Where: src/tree/arb.rs:174-175 (related: src/tree/arb.rs:185, 293, 300-302; the incident record at .agent-notes/2026-07-17-streaming-wire-deadlock/)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the sites; the geometry itself is stated present-tense at 295-298 and 367-371)
- Seen by: prose (19); refutation: confirmed; history: no-rationale-found (b3b877d9b, the deadlock fix, wrote the narrative; the link-transport review's R36 later flagged line 185 for a design-doc section citation and 023546e4a produced the current "closes the proxy tier's generator gap" wording, so that ruling addressed citation form, not incident framing, and this finding does not reopen it)
- Owner-gated: no

`arb_wide_divergent_pair` and `early_first_child_dispute_pair` are documented as incident artifacts ("the streaming wire deadlock's trigger geometry"; "closes the proxy tier's generator gap"; "the streaming wire deadlock's counterexample skeleton, made permanent at the tier that should have owned it"). What a reader of a generator needs is the shape it produces and the property it stresses, which lines 295-298 and 367-371 already state well. History, blame, and gap-closing narratives belong in git and the decision record, not at the declaration site.

Evidence:

    174    /// [`arb_divergent_pair`] at a budget wide enough to reach the streaming
    175    /// wire deadlock's trigger geometry.
    ...
    185    /// This strategy closes the proxy tier's generator gap on *budget* only,
    ...
    300    /// This is the streaming wire deadlock's counterexample skeleton, made
    301    /// permanent at the tier that should have owned it. Content
    302    /// addressing means the shape cannot be dictated, so it is *searched*: insert

Resolution: At 174-175 and 185-192, describe the budget and what the wide pairs reach (multi-level disputes mixed with provisions in the opening reply) without "deadlock" or "gap". At 300-302: "The shape stresses whole-subtree provisions queued behind a dispute on one reply stream. Version addressing means it cannot be dictated, so it is searched: ...". Acceptance: `arb.rs` contains no incident nouns ("deadlock", "gap", "should have"); each generator doc states shape and stressed property in the present tense.

### tree-core-22: `arb_divergent_pair` and `arb_wide_divergent_pair` duplicate a 30-line generator body, and `ESCAPE_MARGIN` is declared twice
- Where: src/tree/arb.rs:193-230 (related: src/tree/arb.rs:132-172, 427-431, 534-541; src/tree/mirror/streaming/remote/proxy/tests.rs:26, 435, 539)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (read both bodies: 142-171 and 203-229 are identical apart from the comment at 147-149; only the five tuple-strategy literals at 136-140 versus 197-201 differ; `grep ESCAPE_MARGIN src/` finds two local `const ESCAPE_MARGIN: usize = 64;` at 431 and 541 with near-identical docs; `arb_wide_divergent_pair` has two callers in the proxy suite)
- Seen by: structure (4); refutation: confirmed; history: no-rationale-found (b3b877d9b added the wide variant as a verbatim copy; the two constants came in separate same-day commits, d29485fd0 and 7c9175a0b)
- Owner-gated: no

The `side` closure encodes the redaction semantics every merge property and every streaming differential suite depends on, and it exists twice; one edit to it must be made twice or the two generators diverge without anyone noticing. Named constants over repeated literals applies to the two `ESCAPE_MARGIN`s, whose docs already say the same thing.

Evidence:

    193    pub fn arb_wide_divergent_pair() -> BoxedStrategy<(crate::tree::Root, crate::tree::Root)> {
    194        use crate::tree::{Action, Tree};
    195
    196        (
    197            0usize..12,                // shared inserts (the common base)
    198            0usize..40,                // a-only inserts
    199            0usize..40,                // b-only inserts
    200            vec(any::<bool>(), 0..12), // which shared keys side a redacts
    201            vec(any::<bool>(), 0..12), // which shared keys side b redacts
    202        )
    203            .prop_map(|(n_shared, n_a, n_b, a_redact, b_redact)| {
    ...
    215                let side = |party: &Party, n: usize, redact: &[bool]| {
    216                    let mut t = base.clone();
    217                    t.act(party, (0..n).map(|_| Action::Insert(Message::new(()))));

Resolution: Introduce one private builder, e.g. `fn divergent_pair(shared: Range<usize>, per_side: Range<usize>) -> BoxedStrategy<(Root, Root)>` whose redact-mask length tracks `shared.end`, and make `arb_divergent_pair()` and `arb_wide_divergent_pair()` one-line wrappers keeping their current docs (the wide doc's budget-versus-bias argument is the valuable part). Hoist `ESCAPE_MARGIN` to one module-level constant with one doc. Acceptance: one `side` closure in the file; both public names keep their signatures and docs; the proxy suite compiles unchanged; one `ESCAPE_MARGIN`.

### tree-core-23: `arb.rs` describes paths as content-addressed and as functions of `(version, payload)`; the tree is version-addressed
- Where: src/tree/arb.rs:235-236 (related: src/tree/arb.rs:301-302, 327-329; src/tree/typed/path.rs:35-38; src/tree.rs:11-12; out of partition: src/tree/mirror/streaming/stats.rs:45, src/tree/mirror/streaming/tests/fixtures.rs:322)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`Path::for_leaf` is `PathHash::of(version.as_bytes())`, path.rs:35-38; tree.rs:11-12 says "Message bytes enter no path and no digest"; `git show 4f18c347 -- src/tree/arb.rs` is a one-line hunk renaming the hash at line 235 while keeping the noun)
- Seen by: prose (13); refutation: confirmed; history: deliberate-but-expired (both sentences were accurate when written, b3b877d9b and 48d5253d9; 961f63c6 derived leaf identity from the version alone and edited `arb.rs` without them; 4f18c347 then edited line 235 and kept the wrong noun)
- Owner-gated: no

The search comment at 327 states "Paths are functions of (version, payload) and payloads are unit", the very function the search simulates, and it is wrong: the path is a function of the version alone. "Content-addressed generators" (235) and "Content addressing means the shape cannot be dictated" (301-302) carry the same expired design. AGENTS.md's hard rule: nothing in the codebase refers to a design the code no longer has.

Evidence:

    235    /// Content-addressed generators cannot produce this shape — SHA3-256 scatters
    236    /// their keys at the root fan, so a merge's divergent descent below the
    ...
    301    /// permanent at the tier that should have owned it. Content
    302    /// addressing means the shape cannot be dictated, so it is *searched*: insert
    ...
    327        // Paths are functions of (version, payload) and payloads are unit, so a
    328        // candidate pair is fully determined by where each side's version chain
    329        // *starts*: `Tree::act` ticks from the root ceiling, so seeding a built

    src/tree/typed/path.rs
    35        pub fn for_leaf(version: &Version) -> Self {
    36            Self {
    37                height: PhantomData,
    38                hash: PathHash::of(version.as_bytes()).into(),

Resolution: Line 327: "Paths are functions of the version alone, so a candidate pair is fully determined by where each side's version chain starts" (the "payloads are unit" clause then goes). Lines 235 and 301: "Version-addressed generators" / "Version addressing". Route the two out-of-partition sites to the streaming partition. Acceptance: `grep -n -i 'content-address\|content addressing\|(version, payload)' src/tree/arb.rs` returns nothing; every statement of the path function in `arb.rs` agrees with `Path::for_leaf`.

### tree-core-24: `ATTEMPTS`'s doc hand-maintains "the winning window is attempt 1581", which the SHA3 swap did not re-derive, and calls the `unreachable!` an assert
- Where: src/tree/arb.rs:318-325 (related: src/tree/arb.rs:413)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git log -S'1581' -- src/tree/arb.rs` yields only d800957e8, 2026-08-06, whose message records "winning attempt 1581, budget 2048" and whose diff replaced the earlier "attempt 622, so 1024"; `git show 4f18c347 --stat -- src/tree/arb.rs` is one insertion and one deletion at line 235, so lines 318-325 were untouched while every `Path::for_leaf` output moved; line 413 is `unreachable!`. Whether 1581 still wins under SHA3-256 was not run)
- Seen by: structure (0), prose (14), perfapi (52); refutation: confirmed; history: deliberate-but-expired (the number was part of an explicit re-derivation discipline that d800957e8 followed for the previous leaf-hash change and 4f18c347 did not; "the assert below" has been wrong since b3b877d9b wrote it beside an `unreachable!`)
- Owner-gated: no

The doc asserts a specific search outcome and calls 2048 "exact headroom, not a guess". The figure was measured under BLAKE3; the swap to SHA3-256 re-derived every leaf path and changed only the hash's name in this file, so the sentence now describes a search the code no longer performs. Nothing checks which attempt wins (exhaustion is an `unreachable!`, not "the assert below"), so the claim is unverifiable in the tree and has already been hand-updated once. No hand-maintained counts: a number that matters lives in a mechanically enforced place that prose may cite by name.

Evidence:

    318        /// Attempt budget; the assert below turns exhaustion into a loud failure.
    319        ///
    320        /// The precompute below is proportional to this bound, so it directly
    321        /// prices the fixture. Hashing is deterministic and the winning window
    322        /// is attempt 1581, so 2048 is exact headroom, not a guess; if hashing
    323        /// or the leaf encoding ever changes, the search either finds another
    324        /// window within the budget or fails loudly here.
    325        const ATTEMPTS: usize = 2048;
    ...
    413        unreachable!("the deterministic geometry search must terminate");

Resolution: Delete "and the winning window is attempt 1581, so 2048 is exact headroom, not a guess" and keep the structural statement (a deterministic search under a budget whose exhaustion panics). If the headroom matters, make it mechanical: have the fixture return the winning attempt (or log it with the crate's `MEASURED` idiom) and assert a margin in a test the doc cites by name, which restates the discipline d800957e8 followed instead of relying on memory. Change "the assert below" to name the `unreachable!`, or make the guard an `assert!` whose message prices the budget. Acceptance: no literal attempt index in `arb.rs` prose, or a committed test asserts the winning attempt and the doc cites it; the guard's description matches its spelling.

### tree-core-25: `arb.rs` and `act.rs` open without a module doc; `arb.rs` ends in an inline `mod test` inside an already `cfg(test)` module; `arb_root_node` is `pub` with one same-file caller
- Where: src/tree/arb.rs:672-673 (related: src/tree/arb.rs:1, 59, 96; src/tree/traverse/act.rs:1; src/tree.rs:714-715)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (tree.rs:714-715 declares `#[cfg(test)] pub(crate) mod arb;`; `grep -rn '\barb_root_node\b' src/ tests/` returns arb.rs:59 (definition), :87 (doc link), :96 (the one caller); act.rs:1 is `use itertools::Itertools;` and arb.rs:1 is `use before::Party;` while `join.rs` and `unknown.rs` open with `//!`)
- Seen by: structure (8), prose (32); refutation: confirmed; history: no-rationale-found (the inline module predates c6a448972's "Standardize test modules on tests.rs" by eight days and that commit did not touch `arb.rs`, so the exception is an omission)
- Owner-gated: no

AGENTS.md's "Writing tests" puts tests in a sibling `tests.rs`; `arb.rs` has an inline `mod test` (singular) under a redundant `#[cfg(test)]`. `act.rs` (production) and `arb.rs` (the generator layer) have no first sentence for a reader entering from `traverse` or from a suite's imports. `arb_root_node` is wider than its use.

Evidence:

    src/tree/arb.rs
    672    #[cfg(test)]
    673    mod test {

    src/tree/traverse/act.rs
    1      use itertools::Itertools;

Resolution: Move `distinct_indices_are_pairwise_disjoint` to `src/tree/arb/tests.rs` behind `mod tests;` and drop the inner cfg; make `arb_root_node` private. Open act.rs with `//! The batch-apply traversal: one pass applies a materialized, version-stamped action list, observing once per key whose leaf changed.` and arb.rs with `//! Proptest strategies and deterministic fixtures for trees and divergent pairs; every party comes from `nth_party`, so independently drawn trees are causally concurrent.` Acceptance: no inline `mod test` in `arb.rs`; `arb_root_node` is not `pub`; both files open with `//!`; the gate's `testdoc` still sees the moved test's doc.

### tree-core-26: Ghosts of the retired V1 mirror in `traverse.rs`, `join.rs`, and `tree.rs`: `Levels`, the zipper, a "free function" claim `unknown` does not meet, and a trio that names `mirror`
- Where: src/tree/traverse.rs:7-19 (related: src/tree/traverse.rs:1-5; src/tree.rs:53-58; src/tree/traverse/join.rs:4-8 and 121; src/tree/traverse/unknown.rs:20-27; src/tree/traverse/act.rs:5; src/tree/traverse/join.rs:39; src/tree/traverse/unknown.rs:17; src/tree/mirror/streaming/materialized/unknown/tests.rs:2 and 15)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn '\bLevels\b' src/` returns traverse.rs:10 only; `grep -rni zipper src/` returns join.rs:6 only; grep for `traverse::act::` or `act::Act` outside `traverse/` returns nothing, while `unknown::Unknown` is imported and linked by `materialized/unknown/tests.rs:2,15`; `unknown.rs` exports only the `Unknown` trait and join.rs:121 calls `Unknown::unknown` directly; `use super::*;` at traverse.rs:7 is consumed by nothing in that file)
- Seen by: structure (6), prose (17, 18); refutation: confirmed (with provenance: `Levels` lived in the removed `traverse/mirror/local.rs`, "Each side keeps a [`Levels`] zipper"); history: deliberate-but-expired (ba8da96c3 widened `act` and `unknown` to `pub(crate)` for links from `typed/levels.rs`; 368da2a50 deleted `levels.rs` and the last `act::Act` linker; "run a zipper" and the act/join/mirror trio described the V1 mirror, which lived under `traverse/` until a4df5b2f6)
- Owner-gated: no

The visibility comment justifies `pub(crate)` on `act` and `unknown` by "rustdoc elsewhere (e.g. the `Levels` docs)". No `Levels` item exists, and nothing outside `traverse` links into `act` (every user goes through the `pub use act::{Action, act}` facade); `unknown` does have a live linker (the materialized pruner's differential test), so that half is real but misattributed. The module doc says "Each traversal is exposed as a free function", but `unknown` exposes only the trait. `tree.rs:55-58` lists the `traverse` trio as act, join, mirror, while `traverse` holds act, join, unknown and `mirror` is a sibling module whose streaming walk is erased, not height-inductive. `join.rs:6` says the mirror must "run a zipper", the V1 mechanism. The glob `use super::*;` exists only so the children can write `use super::typed::*;` and link `super::mirror`, which reads as if `typed` and `mirror` were children of `traverse`. No ghost references; a visibility rationale that names a nonexistent dependent is circular justification for the visibility it explains.

Evidence:

    src/tree/traverse.rs
    3     //! Each traversal is exposed as a free function so callers need not import a
    4     //! trait, though under the hood all are implemented by polymorphic recursion
    5     //! through traits.
    6
    7     use super::*;
    8
    9     // `act` and `unknown` are `pub(crate)` so rustdoc elsewhere (e.g. the
    10    // `Levels` docs) can link to the traversal traits inside them: a private
    11    // `mod` is unnameable from outside `traverse`, so the links would not
    12    // resolve. The free-function facade below remains the API.
    13    pub(crate) mod act;
    14    pub use act::{Action, act};
    15
    16    pub(crate) mod unknown;

    src/tree.rs
    55    //! All mutation and reconciliation is three inductive traversals over the
    56    //! same structure ([`traverse`]): [`act`](Tree::act) applies a local batch
    57    //! in one pass; [`join`](Tree::join) merges two in-memory trees;
    58    //! [`mirror`] reconciles two trees over a wire. `join` and `mirror` are

    src/tree/traverse/join.rs
    5     //! protocol: where the mirror reconciles two replicas by exchanging messages
    6     //! (and so must serialize, run a zipper, and build the union on both sides),

Resolution: `mod act;` (private) with the facade unchanged; re-state the comment to the actual dependent ("`unknown` is `pub(crate)` because the materialized pruner's tests import the `Unknown` trait as their oracle and link it from rustdoc"). Rewrite traverse.rs:3-5: "`act` and `join` are free functions over their per-height traits; `Unknown` is used as a trait by `join` and by the materialized pruner's tests." Drop `use super::*;` and have the children import `crate::tree::typed::*` (as `arb.rs` and `unknown/tests.rs` already do) and link `crate::tree::mirror`. At tree.rs:55-58, list the `traverse` trio as act/join/unknown and introduce `mirror` as the wire counterpart of `join`. At join.rs:6: "(over the wire, pairing queries with replies and building the union on both sides)". Acceptance: `cargo doc` (the gate's `doclint`) resolves every intra-doc link with `mod act` private; `grep -rn '\bLevels\b' src/` and `grep -rni zipper src/` return nothing; `traverse.rs` has no glob import; tree.rs's trio matches `traverse.rs`'s contents.

### tree-core-27: `act` sorts and re-materializes the action list at every one of the 32 heights, and re-sorts each touched fan on reassembly, where one stable sort at entry would do
- Where: src/tree/traverse/act.rs:86-93 (related: src/tree/traverse/act.rs:101-109 and 128-129; src/tree.rs:483-497; src/tree/typed/path.rs:49-58 and 86-90; src/tree/typed/untyped.rs:229-258 and 276-340; src/tree/typed/untyped/fan.rs:178-200; benches/in_memory.rs:91-92 and 137-138; src/tree/tests.rs:1655-1660)
- Class / severity / confidence: performance / medium / high
- Provenance: verified for the mechanism (read `itertools-0.14.0/src/lib.rs:3101-3110`: `sorted_by_key` is `Vec::from_iter(self)` then `sort_by_key`, a fresh `Vec` and a comparison sort per call; `Path<H>::Ord` compares the unconsumed suffix, path.rs:86-90, and `pop` takes its first byte, path.rs:49-58; `Fan::from_iter` detects a non-ascending run and pays `sort_by_key` plus a dedup copy, fan.rs:184-200); assessed for the magnitude (no bench was run)
- Seen by: perfapi (44, 48); refutation: confirmed (the typed walk visits every height even through compressed spines, since `into_children` on a prefixed node pops one prefix byte into `Fan::unit`, untyped.rs:230-239, so a fresh leaf pays this at all 32 levels); history: no-rationale-found (the per-level `sorted_by_key` dates to fe3612311 and was never justified; 262568f9e named the grouping "the radix sort" and the `react` comment inherited the phrase; `fan.rs:181-183` asserts "Every reassembly in the crate feeds pairs already strictly ascending", which act.rs:129 falsifies)
- Owner-gated: no for the sort deletion (adoptable now, no API or wire change); the larger slice-recursion redesign is an owner decision, listed under open questions

At each of the 32 heights, `S<H>::act` runs `sorted_by_key` (a fresh `Vec` and a comparison sort of the whole group) and then `collect()`s every radix group into another `Vec`, so each action's `(Path, Version, Action)` tuple is copied twice and sorted once per level, and a fresh leaf's spine costs about three heap allocations per level, on the path every `send`, `send_all`, `redact`, and `Batch` commit takes inside the watch lock. Because `Path<S<H>>`'s order is lexicographic on the unconsumed suffix and `pop` takes the suffix's first byte, one stable sort by full path at entry leaves every level's groups contiguous and each group already sorted by `Path<H>`, so `chunk_by` alone suffices at every level; stability preserves the documented last-action-on-a-path-wins order. Separately, the reassembly `updated.into_iter().chain(existing_children)` is ascending only when every updated radix is below every untouched one; otherwise `Fan::from_iter` pays a sort and a second buffer, which the root fan does on a typical commit. Both are strict deletions of redundant work with a fixed sign (denominator: per action per height; per touched branch per commit). The `react` comment (tree.rs:488-490) says the up-front `Vec` is "one Vec the radix sort immediately consumes"; `sorted_by_key` builds its own, and the sort is a comparison sort.

Evidence:

    86            let by_radix = actions
    87                .into_iter()
    88                .map(|(path, version, action)| {
    89                    let (child, path) = path.pop();
    90                    (child, path, version, action)
    91                })
    92                .sorted_by_key(|(child, _, _, _)| *child)
    93                .chunk_by(|(child, _, _, _)| *child);
    ...
    107                let actions: Vec<_> = group
    108                    .map(|(_, path, version, action)| (path, version, action))
    109                    .collect();
    ...
    128            // Re-assemble: updated children + untouched existing children.
    129            Node::branch(updated.into_iter().chain(existing_children).collect())

    src/tree/typed/path.rs
    86    impl<H: Height> Ord for Path<H> {
    87        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    88            self.hash[32 - H::HEIGHT..].cmp(&other.hash[32 - H::HEIGHT..])

    src/tree/typed/untyped/fan.rs
    181    /// [`insert`](Fan::insert). Every reassembly in the crate feeds pairs
    182    /// already strictly ascending and duplicate-free, which this recognizes in
    183    /// one pass; anything else pays one stable sort.

Resolution: Adoptable now: in `Tree::react` (tree.rs:491-497) sort the collected `Vec` once, stably, by path (`actions.sort_by_key(|(path, ..)| *path)`), state "sorted by `Path<Self>`" as `Act::act`'s precondition, and replace `.sorted_by_key(..).chunk_by(..)` with `.chunk_by(..)` alone; fix the `react` comment to describe the one sort that remains. For the reassembly, merge the two ascending runs (`updated.into_iter().merge_by(existing_children, |a, b| a.0 < b.0)`; radixes are disjoint because each updated child was `remove`d) or insert recursed children back into `existing_children` and drop `updated`, so `Fan::from_iter` takes its one-pass branch and fan.rs:181-183 becomes true. Acceptance: `benches/in_memory.rs` `batch_insert` and `redact` measured at the parent commit and after, identical-or-improved at every N, with fewer allocations (an allocation count per commit can be pinned the way `tests/encode_alloc.rs` pins encode); `react_batch_partitioning_preserves_hash`, `tree_shape_is_canonical_in_the_leaf_set`, and `insert_and_delete_same_batch_is_empty` stay green, proving order semantics survived; a debug assertion or test that the reassembled iterator is ascending.

### tree-core-28: Comments that narrate the next line, and a mutable-accumulator fan build where `collect` reads directly
- Where: src/tree/traverse/act.rs:95-95 (related: src/tree/traverse/act.rs:111, 128, 178, 187; src/tree/traverse/unknown.rs:34, 64-73, 79, 88; src/tree/traverse/unknown/tests.rs:55-63; src/tree/typed/node.rs:88-110)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read each cited comment against its statement; `Children<H>: FromIterator<(u8, Node<H>)>` at node.rs:88-96 and `IntoIterator` at 102-110 make the `filter_map(..).collect()` form available, and act.rs:129 already uses `collect()`)
- Seen by: structure (9), prose (30); refutation: confirmed (act.rs:187 labels a match arm and is the most defensible of the set); history: no-rationale-found (the one-liners date to the April and May origins; the `Children::default()` plus `insert` loop predates `FromIterator` on `Children`)
- Owner-gated: no

act.rs:95 "Explode the node into its children" over `node.map(|n| n.into_children())`, 111 "Mutably pull the existing child out of the parent:", 128 "Re-assemble: updated children + untouched existing children.", 178 "Set the node"; unknown.rs:34 and 79 "If the node doesn't exist, we can't return information about it" over `let node = node?;`, 64 "Recursively process each child, re-assembling only the unknown children", 88 "Otherwise, the node is causally unknown: return it". Comments state what the code cannot show, never what the next line does. The surviving-children fan in `unknown.rs` is built with `Children::default()` and a `for` loop with `insert`; the `TwoPass` oracle mirrors the same block.

Evidence:

    src/tree/traverse/act.rs
    95            // Explode the node into its children
    96            let mut existing_children = node.map(|n| n.into_children()).unwrap_or_default();

    src/tree/traverse/unknown.rs
    64            // Recursively process each child, re-assembling only the unknown children
    65            Node::branch({
    66                let mut children = Children::default();
    67                for (radix, child) in node.into_children() {
    68                    if let Some(child) = Unknown::unknown(Some(child), known) {
    69                        children.insert(radix, child);
    70                    }
    71                }
    72                children
    73            })

Resolution: Delete the pure narrations (keep act.rs:114, the short-circuit's why, and 142-144, the tie-break rule). Rewrite the fan as `Node::branch(node.into_children().into_iter().filter_map(|(radix, child)| Unknown::unknown(Some(child), known).map(|kept| (radix, kept))).collect())`, and let the `TwoPass` oracle follow. Acceptance: no `//` comment in `act.rs` or `unknown.rs` paraphrases the single statement that follows it; `unknown.rs` has no mutable `Children` accumulator.

### tree-core-29: A no-op `act` rebuilds the root spine under a new handle and discards its memos; `Unknown for S<H>` rebuilds a `Between` node whose every child survived
- Where: src/tree/traverse/act.rs:96-96 (related: src/tree/traverse/act.rs:115-121 and 128-129; src/tree.rs:384-385 and 526; src/tree/typed/untyped.rs:212-221 and 229-258; src/batch.rs:129-131; src/tree/traverse/unknown.rs:65-73)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (traced: `react` hands the walk `self.root.root.clone()`, so the root `Arc` is shared; `S<H>::act` calls `into_children()` unconditionally, which `Arc::make_mut`s the inner and takes the fan, untyped.rs:236-239 and 247-254; `Node::branch` allocates a fresh `NodeInner` with fresh `OnceLock`s, untyped.rs:212-221; an empty batch, or a batch of forgets for absent keys where every group `continue`s at 115-121, therefore commits a memo-less root under a new handle while returning `false`)
- Seen by: correctness (39); refutation: confirmed (the observable contract, hash and ceiling and content unchanged, holds, so "contract breached" would overread; the cost is one root-fan re-fold on the next read and a missed `ptr_eq` against earlier snapshots); history: no-rationale-found (no `actions.is_empty()` guard has ever existed; 2d3a86d3f priced the fresh spine for effectual batches, not for the empty case)
- Owner-gated: no

`Tree::act` promises "An empty batch is a complete no-op ... the tree is unchanged", and `Batch::commit` relies on that sentence to skip a special case. The promise holds observationally, but the root is replaced by a memo-less copy: the next `hash()` or `earliest()` re-folds the root fan (up to 256 child hashes and spans), and `ptr_eq` short-circuits against snapshots taken earlier fail at the root, falling back to the hash. The memos are the tree's amortization story (tree.rs:38-51), and the root-hash meter exists because re-hashing the spine happens under the watch lock. The same shape recurs in `unknown.rs:65-73`, which rebuilds a `Between` subtree even when every child survives, losing sharing for the kept side of `join`'s asymmetric arm.

Evidence:

    src/tree/traverse/act.rs
    96            let mut existing_children = node.map(|n| n.into_children()).unwrap_or_default();
    ...
    129            Node::branch(updated.into_iter().chain(existing_children).collect())

    src/tree.rs
    384        /// An empty batch is a complete no-op: nothing ticks, the tree is
    385        /// unchanged, and the returned flag is `false`.

    src/batch.rs
    129            // An empty action list needs no special case: `Tree::act`
    130            // documents an empty batch as a complete no-op, and its false
    131            // changed flag suppresses the wakeup.

Resolution: In `Tree::react`, return `false` before the walk when `actions.is_empty()` (fixed sign, trivial). In `S<H>::act`, keep a handle to the incoming node and return it when no group produced an update and no existing child was removed. Optionally, in `Unknown for S<H>`, return the original node when the rebuilt fan has the same length and every child is `ptr_eq` to the original. Acceptance: a committed test warms caches, runs an empty `act` and an all-absent-forgets `act`, and asserts the root node handle is pointer-identical (a `#[cfg(test)]` `ptr_eq` on typed `Node` delegating to `untyped::Node::ptr_eq`) and that a subsequent `hash()` is answered from the memo.

### tree-core-30: `Z::act` stores the running join as the leaf's version rather than the applied action's, and `react`'s "causally latest action wins" holds only for causally ascending sequences
- Where: src/tree/traverse/act.rs:179-182 (related: src/tree/traverse/act.rs:145-159; src/tree.rs:283-284, 447-451, 461-465)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (traced: `greatest_version` joins every action's version at the key, line 148, before the skip; an Insert stores that join, line 181, while the path was derived from the action's own `version`, tree.rs:449; under `act` the two coincide because one party's ticks from the ceiling form a chain and an insert is the first action at its fresh path)
- Seen by: correctness (38); refutation: confirmed, and its new item 2 (the `[Forget(v2), Insert(v1)]` counterexample to the `react` contract) is folded in here; history: no-rationale-found (052d1f95b changed `Node::leaf(version, value)` to `Node::leaf(greatest_version.clone(), value)` in a WIP commit with no comment, and no later commit discusses it)
- Owner-gated: no

The invariant the tree rests on, "the set is the tree" (tree.rs:16-19) and `get`'s premise that "the hit's version is the queried one" (283-284), requires the stored version to be the one the path was derived from. The storage site does not say so: it stores `greatest_version`, the join over every action at the key including skipped ones, and the coupling holds only by a caller-side chain property stated nowhere near it. Through the private `react` (test-only today) the two decouple: a Forget then an Insert at one path carrying concurrent versions stores their join, which hashes to neither path, so `iter` yields a version `get` cannot find. The same trace refutes `react`'s doc (tree.rs:461-463): `[Forget(v2), Insert(v1)]` at one key on a fresh node with `v1 < v2` sets `node = None`, then the Insert's skip test compares `v1` against `Version::default()` and passes, so the causally earlier insert lands, stamped with the forget's version `v2`; the doc predicts the forget wins. Finished code should be obviously, reviewably correct at the site where the property lives.

Evidence:

    145            for (_, version, action) in actions {
    146                // Join by reference: `version` is still needed for the causality
    147                // comparison just below, and the join doesn't consume it.
    148                greatest_version |= &version;
    ...
    179                node = match action {
    180                    Action::Forget => None,
    181                    Action::Insert(value) => Some(Node::leaf(greatest_version.clone(), value)),
    182                };

    src/tree.rs
    461        /// If multiple actions refer to the same leaf of the tree, the causally
    462        /// latest action wins, with order of specification breaking concurrency
    463        /// and version ties. Each item is keyed by its version-derived path, so

Resolution: Store the applied action's version: `Some(Node::leaf(version.clone(), value))` (move `version` into the arm; it is not needed after the comparison), keeping `greatest_version` for the observer only; this is equivalent under `act` and makes the path/version coupling hold at the storage site. Then either qualify `react`'s doc ("for a causally ascending sequence at each key, which `act` guarantees") or track the per-key ceiling across a Forget so the stated rule holds for arbitrary sequences. Acceptance: the leaf's stored version is by construction the one its path was derived from; existing `act`/`react` suites unchanged; a committed test pins the two constructions below. Construction: (1) tree holds a leaf at synthetic path `p` with version A@1; `tree.react([(p, B@1, None::<Message>), (p, C@1, Some(msg))])` with A, B, C disjoint parties: the Forget at B@1 is concurrent with A@1, not `<`, so the leaf is removed; the Insert at C@1 lands on `None`; the stored version is `B@1 | C@1`. Assert `tree.iter().next().unwrap().0 == &C@1`, which fails at this commit. (2) fresh tree, `p = Path::for_leaf(&v1)`, `v1 < v2` on one party: `tree.react([(p, v2, None::<Message>), (p, v1, Some(msg))])`. The doc predicts an empty tree; `tree.len()` is 1 and the stored version is `v2`, so `Path::for_leaf(stored) != p`.

### tree-core-31: `join`'s divergent arm clones the fan three times over and hand-rolls the two-way merge that `itertools::merge_join_by` spells in a `match`
- Where: src/tree/traverse/join.rs:150-159 (related: src/tree/traverse/join.rs:160-195; src/tree/typed/node.rs:41-86; src/tree/typed/untyped/fan.rs:14-20 and 154-160; src/tree/mirror/streaming/materialized/work/answer.rs:1, 58-86, 129-146; src/tree/traverse/unknown.rs:65-73)
- Class / severity / confidence: simplification / low / high
- Provenance: verified for the mechanism (`merged = ours.clone()` bumps k handles; `Children::iter` clones every child, node.rs:81-85, on both fans, including pairs then pruned at 179-183; `merged.insert`/`remove` are binary-search edits; `Children` exposes `insert`/`remove`/`iter` but no `push`, while `Fan::push` exists, fan.rs:154-160; `merge_join_by`/`EitherOrBoth` is already the crate's spelling of this shape in `answer.rs`); assessed for magnitude
- Seen by: structure (3), perfapi (47); refutation: confirmed (one rewrite satisfies both: a consuming `merge_join_by` feeding `push`); history: no-rationale-found for the hand-rolled loop (f5426c9d6 replaced imbl's `OrdMap::diff` after jneem/imbl#161 without considering itertools); deliberate-but-expired for the comment (written for a persistent `OrdMap` where `merged = ours.clone()` was O(1); f70f9559a replaced `OrdMap` with the non-persistent `Fan` whose doc states the premise the comment now contradicts)
- Owner-gated: no

For every divergent branch the walk clones `ours`' whole fan into `merged`, iterates both fans through `Children::iter` (which clones every child again, including pairs it then prunes as equal and drops), and patches `merged` by `insert`/`remove` with element shifts. The comment justifies starting from `ours` by "structural sharing", but sharing lives on the node handles, not the fan: `fan.rs:14-16` says `Fan` "is deliberately *not* a persistent map" and cloning it is "one refcount bump per child". The loop itself is two `Peekable`s, a four-arm `match` on `(peek, peek)`, `min`, two `next_if` calls, and `ours`/`theirs` shadowed from fan to iterator: the one place in the file where a reader simulates iterator state to trust the walk. Both fans are ascending by construction, which is exactly `merge_join_by`'s precondition, and itertools is already a dependency this crate uses for the same shape. A fresh fan built by ascending `push` from consuming iterators holds the identical handles with no extra bumps and no `insert`/`remove`, and reads as evidently correct.

Evidence:

    150                    // The walk reads both fans directly rather than diffing the
    151                    // two maps against each other: the merged map starts from
    152                    // *ours* and only divergent radixes are rewritten, so every
    153                    // shared child persists by structural sharing.
    154                    let ours = ours.into_children();
    155                    let theirs = theirs.into_children();
    156
    157                    let mut merged = ours.clone();
    158                    let mut ours = ours.iter().peekable();
    159                    let mut theirs = theirs.iter().peekable();
    160                    loop {
    161                        let (radix, our_child, their_child) = match (ours.peek(), theirs.peek()) {
    162                            (None, None) => break,
    ...
    169                            (Some((ours_radix, _)), Some((theirs_radix, _))) => {
    170                                let radix = (*ours_radix).min(*theirs_radix);

    src/tree/typed/untyped/fan.rs
    14    //! [`Fan`] is deliberately *not* a persistent map. Structural sharing lives
    15    //! one level up, on the node handles the fan stores (each entry is one
    16    //! `Arc` reference): cloning a fan is one refcount bump per child, and

Resolution: Expose `push` on `Children<H>` (delegating to `Fan::push`), start `merged` as `Children::default()`, and drive the loop as `for pair in ours.into_iter().merge_join_by(theirs.into_iter(), |(r, _), (s, _)| r.cmp(s))` with a three-arm `match` on `EitherOrBoth::{Both, Left, Right}`: push `our_child` for an equal pair, push `Join::join(..)`'s `Some` result otherwise. Rewrite the comment to state the invariant (both fans ascending; merged built ascending). Apply `push` (or `collect`) to `unknown.rs:65-73` too. Acceptance: `join_idempotent`, `join_commutative`, `join_associative`, both changed-flag properties, and `join_unwind_leaves_tree_byte_identical` (fuse count per branch level, unaffected) stay green; the loop-with-peekables is gone; a `testing::node_census` reading around one `Tree::join` of a wide divergent pair shows the peak drop by about two handles per divergent child.

### tree-core-32: Collision detection is attributed to "`react`'s occupied-path arms" and called "ingestion"; the check lives in `Act for Z`, and "ingestion" already means the wire
- Where: src/tree/traverse/join.rs:224-229 (related: src/tree/traverse/act.rs:161-176; src/tree/tests.rs:1301, 1800-1804, 1830-1832; out of partition: src/tree/typed/hash.rs:102-103)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn occupied src/tree/` shows no such arms in `Tree::react`, tree.rs:477-541; the identity assert is `Act for Z`, act.rs:169-176; `hash.rs:103` even links `[react](crate::tree::Tree::react)`; tests.rs:1301 uses "ingestion" for session ingestion)
- Seen by: prose (16); refutation: confirmed; history: no-rationale-found (f3fef7bc1, the commit that dissolved the collision machinery, deleted react's occupied-path doc clause in the same diff that wrote "`react`'s occupied-path arms" into join.rs, so the pointer was imprecise from birth, and its message uses "ingestion" for the apply walk, colliding with the session-ingestion sense from 0116a0817)
- Owner-gated: no

A pointer to the wrong site costs the maintainer a search, and a term with two meanings (wire ingestion versus local apply) costs a misreading. The same pointer recurs in two test docs and in `hash.rs`.

Evidence:

    224                // Two leaves at one position share the path, and a leaf digest
    225                // is a pure function of its path (`Hash::leaf`), so the pair
    226                // hashes equal and the level above carries it over verbatim
    227                // without recursing. Collision detection is ingestion's job
    228                // (`react`'s occupied-path arms), where both leaves are in
    229                // hand; the merge walk trusts path derivation.

Resolution: "Collision detection is the apply walk's job (`Act for Z`'s insert-on-live-leaf assert), where both leaves are in hand"; the same wording at tests.rs:1800-1804 and 1830-1832, and at `hash.rs:102-103` when that file is touched. Acceptance: `grep -rn "occupied-path\|ingestion's job" src/tree` returns nothing; each pointer names `Act for Z` or the act.rs assert.

### tree-core-33: `join/tests.rs` says it covers deletion honoring, but every test in the file is direction-blind; no property in the tree states join's survivor formula
- Where: src/tree/traverse/join/tests.rs:1-7 (related: src/tree/traverse/unknown.rs:84-86; src/tree/traverse/join.rs:119-129 and 135; src/tree/tests.rs:294-299, 1182-1207, 1241-1252, 1310-1335, 1349-1383, 1746-1760; src/tree/mirror/streaming/materialized/unknown/tests.rs:70-83; tests/multi_peer.rs; tests/common/oracle.rs)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (traced, not run: `join_idempotent` exits at `ours == theirs`, join.rs:135, before any filter; `join_commutative` holds because each one-sided subtree is filtered against the other side's ceiling whichever side is `ours`; `join_associative` over disjoint parties is vacuous under an inverted leaf verdict, both orders empty; both changed-flag biconditionals compute `changed` from actual leaf-count movement, join.rs:122, so a wrong drop or gain moves flag and hash together)
- Seen by: correctness (35); refutation: reframed and lowered from medium (the direction is pinned elsewhere: `deep_divergent_join_changed_flag_is_exact`'s redaction leg, `join_destructor_unwind_leaves_tree_byte_identical`, the two poisoned-store pins, Route C of `tree_shape_is_canonical_in_the_leaf_set`, the differential `agrees_with_materialized_oracle` and `streaming_matches_join_oracle`, and end-to-end `tests/multi_peer.rs` against `tests/common/oracle.rs` all fail under inversion; what remains is the module-doc overclaim and the missing local property); history: already-known in part (the independent family-level check was `join_matches_mirror`, deleted by 368da2a50; the v1-retirement record acknowledges the loss and deferred the scoped-mutants check that would have surfaced this)
- Owner-gated: no

The module doc says the file covers "its deletion honoring by version dominance" and that these laws ground the join oracle the streaming suite relies on, but inverting the leaf arm at `unknown.rs:84` (keep leaves `<= known`, drop the rest) leaves all three laws green, and no property anywhere in the tree states what join's survivor set is. Family coverage of the direction exists only end-to-end through gossip against the spec oracle in `tests/`, plus point fixtures here. An oracle that would agree with a wrong implementation is a blind spot; a property over `arb_divergent_pair` asserting the set formula directly would make the module doc true and ground the streaming oracle non-circularly.

Evidence:

    1    //! `Tree::join`'s algebraic laws and its deletion honoring by version
    2    //! dominance.
    3    //!
    4    //! Join is the in-memory oracle the wire reconciliation is differentially
    5    //! tested against (the streaming suites' join-oracle properties), so these
    6    //! laws — with the route-equivalence property in the tree's own suite —
    7    //! are what ground that oracle.

    src/tree/traverse/unknown.rs
    84            if causally::before(known).contains(node.ceiling()) {
    85                return None;
    86            }

Resolution: Add a proptest over `arb_divergent_pair()` and `arb_deep_divergent_pair()` comparing `join_tree(a, b)`'s leaf view to the set formula: a leaf with version `v` survives iff it is in both, or it is in `a` and `!causally::before(&Vb).contains(v)`, or it is in `b` and `!causally::before(&Va).contains(v)`; and the merged ceiling equals `Va | Vb`. Update the module doc to name it. Acceptance: the new proptest is committed, and under the reversible string swap at unknown.rs:84 (`contains` to `!contains`) it fails while the three laws still pass, demonstrating that it discriminates where they do not. Construction: build the expected view as a `BTreeMap<Vec<u8>, ()>` keyed by `version.as_bytes()` from `Tree::from_root(a).iter()` and `Tree::from_root(b).iter()` filtered by the formula; compare to the joined tree's leaf view; compare `joined.root.ceiling` to `a.ceiling | b.ceiling`. To confirm the blind spot first: apply the swap, run `just test tree::`, record which tests fail (the point fixtures and differentials listed above, none of the three laws), restore, verify `git diff` empty.

### tree-core-34: `join_associative`'s doc claims redaction-associativity coverage that no suite provides, by an argument that is now circular
- Where: src/tree/traverse/join/tests.rs:35-41 (related: src/tree/traverse/join/tests.rs:42-51; src/tree/arb.rs:92-119 and 132-172; src/tree/mirror/streaming/tests.rs:152-174)
- Class / severity / confidence: test-quality / medium / high
- Provenance: verified (`grep -rn associativ src/ tests/` returns tree.rs:392 and join/tests.rs:35,43 only; the streaming property `streaming_matches_join_oracle` uses `Tree::join` as its oracle, streaming/tests.rs:152-161, so "join matches the mirror" cannot establish a law of join; the test's generators `arb_tree_root(0|1|2, 0..6)` put the three trees on disjoint parties, so no ceiling contains any other side's leaf and deletion honoring never fires)
- Seen by: prose (15), correctness (34); refutation: confirmed (the proposed three-way redacting generator passes by trace: a shared leaf redacted by one side drops in both association orders; side-only leaves survive both); history: deliberate-but-expired (the parenthetical was written in 329d891bf beside `join_matches_mirror`, a differential against the independent V1 mirror over a redacting generator, so the transitive argument was then defensible; 368da2a50 deleted that test and reversed the oracle direction, and the retirement plan named "associativity" among join's anchors without noticing this test covers only disjoint parties)
- Owner-gated: no

Every test's doc comment states its invariant and must be accurate; an inaccurate testdoc is a bug in the test (AGENTS.md). This one asserts coverage that does not exist and vouches for the oracle by what it is the oracle of. The property is a family claim the CRDT semantics rest on, and the case it skips is exactly where ceiling-based deletion inference could go wrong.

Evidence:

    35        /// The merge is associative over three mutually-disjoint trees.
    36        ///
    37        /// (Uses `arb_tree_root` on three distinct party indices so the three are
    38        /// pairwise disjoint; `arb_divergent_pair` bakes in parties 0/1/2 and so
    39        /// cannot be composed three-way. Associativity in the presence of redactions
    40        /// is covered transitively: `join` matches the mirror, which proves it under
    41        /// its own redacting generators.)
    42        #[test]
    43        fn join_associative(
    44            a in arb_tree_root(0, 0..6),
    45            b in arb_tree_root(1, 0..6),
    46            c in arb_tree_root(2, 0..6),

Resolution: Add a three-way divergent generator to `arb.rs` (a common base of shared inserts on party 0; three forks on parties 1, 2, 3, each with its own inserts and an arbitrary redaction subset of the shared keys, ceilings built by `Tree::act`) and a `join_associative_with_redactions` proptest asserting `join_tree(join_tree(a, b), c) == join_tree(a, join_tree(b, c))` (`Root` equality covers ceiling and content). Rewrite the testdoc of `join_associative` to state what it tests and drop the transitive-coverage sentence. Acceptance: `grep -rn associativ src tests` shows the new property; the testdoc for `join_associative` names no coverage it does not provide and makes no appeal to the mirror-versus-join differential. Construction: generator `base.act(p0, n_shared inserts)`; for each side i in 1..=3, `t = base.clone(); t.act(p_i, n_i inserts); t.act(p_i, forgets of drawn shared keys)`. Assert both association orders equal, and optionally all six permutations (commutativity composed).

### tree-core-35: `unknown/tests.rs` frames its live oracle as something replaced and overstates what the meter pins; one "now" in `tests.rs` dates a sentence
- Where: src/tree/traverse/unknown/tests.rs:1-3 (related: src/tree/traverse/unknown/tests.rs:24-25, 116-129, 151-155; src/tree/tests.rs:1300-1301 and 1341-1343)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read the sites; the `assert!` at 151-155 pins only `fused_scan < two_pass_scan`; tests.rs:1342-1343 reads "which ingestion now / rejects" across a line break, so a single-line grep for "now rejects" misses it)
- Seen by: prose (25, 24), correctness (43); refutation: confirmed; history: no-rationale-found (28742fac1's message already states the present-tense role, "the two-pass spelling survives only as this comparison's in-test cost oracle"; the "one probe decode against two" sentence transcribes that commit's measured claim rather than what the assertion pins; the "now" at tests.rs:1342 was written by 0116a0817, the commit that landed the enforcement it refers to)
- Owner-gated: no

The two-pass shape exists in the file as the cost oracle; say what it is, not what it was. The test doc's "The pinned inequality is one probe decode per classified node against the two-pass shape's two" describes a ratio while the assertion pins a strict inequality. "Which ingestion now rejects" is dated rationale; the companion doc at 1300-1301 states the same fact without the adverb.

Evidence:

    src/tree/traverse/unknown/tests.rs
    1     //! The classifier's fused-walk cost: the dominance face against the
    2     //! two-pass placement shape it replaces, compared on a wide-divergence
    3     //! tree with deterministic meters.
    ...
    24    /// The two-check classification the fused dominance face replaced, kept
    25    /// as this comparison's cost oracle.
    ...
    127    /// subtract. The pinned inequality is one probe decode per classified
    128    /// node against the two-pass shape's two, with the dominance bail
    129    /// landing at the first refuting interval.

    src/tree/tests.rs
    1341    /// With a leaf whose version dominates the tree's ceiling (the shape only
    1342    /// a nonconforming implementation can transmit, and which ingestion now
    1343    /// rejects as `UncontainedSupply`), a forget of its key ticks from the

Resolution: "the two-pass placement shape kept as its cost oracle"; "A two-check spelling of the classifier, kept as this comparison's cost oracle"; "The pinned inequality is strict: the fused classifier scans fewer bits, because it decodes the probe once per classified node where the two-pass shape decodes it twice." Delete "now" at tests.rs:1342. Acceptance: no "replaces"/"replaced" in `unknown/tests.rs`; the test doc's stated pin matches the `assert!`; a multi-line-aware check finds no "ingestion now rejects".

## Positives

- The commit sections in `Tree::react` and `Tree::join` (tree.rs:507-541, 576-613) defend panic atomicity by construction (the walk runs on an O(1) structural clone, the ceiling folds into a local, the commit is replace, assign, then drop) and state the hazard concretely: an emptied root under a live ceiling, "byte-for-byte the shape of 'everything was redacted'". Each is pinned from three directions: caller-stream unwind, fuse-injected mid-walk unwind after copy-on-write work has begun (with the arming depth derived from the fire-point arithmetic, tests.rs:1655-1659), and the one caller-reachable source, a panicking `T` destructor on a last-handle drop, with the caught panic's message checked so the pin provably exercises that source (tests.rs:1683-1794). Every test the comments name exists.
- The changed flags are decided by the traversals, not by hashing, so no root hash is read inside a critical section; both directions are stated with their differing promises (tree.rs:399-417, 552-565), the one conservative case is constructed rather than argued (`act_changed_flag_is_conservative_only_in_a_poisoned_store`), and the biconditionals are pinned at the root fan and at full depth (`join_changed_flag_tracks_the_root_hash{,_at_depth}`, `deep_divergent_join_changed_flag_is_exact`), with `ceiling_only_join_reports_unchanged` pinning the deliberate exclusion of ceiling movement from join's flag.
- No traversal in the partition recurses on input-controlled depth: `act`, `join`, and `Unknown` are polymorphic recursions over the Peano height that bottom out at `Z` by type, and `Path::pop`'s index `32 - S::<H>::HEIGHT` is in `0..=31` by construction. The one production `assert!` (act.rs:169-176) is justified as a trust-boundary detector with both legs pinned by `#[should_panic]` tests and the identical-reinsert idempotence pinned separately; the one `unreachable!` (join.rs:230-232) carries a valid argument from `Hash::leaf`'s suffix-only preimage.
- `traverse::act`'s `# Panics` section (act.rs:30-38) is close to the one-line proof the doctrine asks of an assert (fresh ticks dominate the ceiling; party linearity; no wire-derived leaf on this path), and the monomorphization boundary is a stated decision at the code that enforces it (act.rs:24-28: a concrete `Vec` and `&mut dyn FnMut` so the per-height tower compiles once).
- `reference_hash` (tests.rs:107-181) is an independent ground truth: literal tag bytes, its own compression rule, its own preimage assembly, re-deriving the canonical shape from the sorted path set without calling back into the implementation. `tree_shape_is_canonical_in_the_leaf_set` routes one leaf set four ways (single batch, shuffled split with a redacted detour, disjoint join, bulk `from_sorted_leaves`) and pins the hash's version-set purity and the leaf view's payload mapping as separate facets.
- `arb.rs` justifies every generator and fixture by the shape it reaches and why version addressing cannot reach it otherwise; `arb_deep_divergent_pair`, `leaf_parent_dispute_pair`, and `leaf_parent_redaction_pair` construct the hash-prefix-collision analogue deliberately so the divergent descent and the `S<Z>` arms are reached at every depth; `early_first_child_dispute_pair` cross-checks its path simulation against the built trees (397-409) so the search cannot disagree with the builder unnoticed; `nth_party`'s disjointness invariant is stated (10-19) and checked mechanically.
- `unknown/tests.rs` is a model meter test: a retained known-worse shape as the cost oracle, a verdict-equality assertion before the cost comparison, a liveness floor on both counters, and a `MEASURED` line for the record.
- join.rs's module doc (1-35) is a model maintainer doc: the four-case analysis, why equal hashes mean equal version sets, and a candid complexity note about enumerating a divergent branch's full fan; unknown.rs's fused-classification comment (37-62) explains each `Dominance` verdict as a prune decision and why the floor-first exit is the common case.

## Open questions for Finch

- `Tree::react` documents a general versioned-apply contract (concurrent and tied versions, order-of-specification tie-breaking) that only tests exercise; its sole production caller `act` supplies one party's ascending chain. Should `react` stay a generic versioned-apply (then tree-core-30's storage-site fix and a per-key ceiling across Forgets are the right repairs), or should it collapse into `act`'s commit section with the fixtures reaching `traverse::act` directly as `arb.rs` already does, and the multi-action semantics documented once at the leaf level? Recommendation: keep `react` as `act`'s commit section, document it as such, and let tree-core-14 remove the middle spelling.
- The larger `act` redesign behind tree-core-27 (recurse on `&mut [(Path, Version, Action)]` slices with a depth index as `from_sorted_leaves` does, and build a fresh subtree under an absent child with `Node::from_sorted_leaves` in one shot instead of 30 levels of `branch`/`beneath`) changes the fuse fire-point structure that `act_mid_walk_unwind_leaves_tree_byte_identical` derives its arming depth from. Recommendation: land the single-sort tier first (fixed sign, measured with the existing benches), and decide the redesign only on those measurements.
- tree-core-5 reopens 8dc0596ed's opacity decision on `Snapshot::iter`. Recommendation: re-export `Iter` and return it concretely, keeping `typed::Iter` private inside it; that meets the "hide engine internals" goal and matches std's `IntoIterator for &Vec` convention.
- tree-core-6: keep `latest`/`earliest` as named and state the asymmetry at the tree level, or rename the ceiling accessor (`frontier`) so the pair can be symmetric? Recommendation: state it at the tree level now; the rename is worth doing before the first release if at all.
- After the SHA3 swap, does `early_first_child_dispute_pair`'s search still terminate at attempt 1581, or at another attempt within 2048? One test run answers it; tree-core-24 asks that the prose stop carrying the number either way.
- The `.agent-notes/2026-08-21-unknown-pruning-survivor/` handoff (a mutant inverting the streaming filter's leaf verdict survives the suite) is out of this partition, but its hypothesis that fixtures never build a mixed-knowledge parent at height 1 also describes `traverse::unknown`'s `Z` arm here, which only `leaf_parent_redaction_pair` reaches. A shared generator forcing `S<Z>` parents with interleaved known, unknown, and deleted leaves would serve both partitions; should it live in `src/tree/arb.rs`?
- The associativity-under-redactions law (tree-core-34) holds under the in-model invariant that a ceiling containing a leaf's tick contains the whole leaf version (a consequence of ceilings advancing only by own ticks and joins of whole ceilings). Is that invariant stated anywhere a maintainer would find it (crate docs or `reconciliation`)? If not, the new property's testdoc is a reasonable home for it.

## Dropped

- Candidate 14 (prose) and 52 (perfapi), "attempt 1581" in `ATTEMPTS`'s doc: duplicates of tree-core-24.
- Candidate 40 (correctness) and 46 (perfapi), `Tree::hash` clones the whole `Root`: duplicates of tree-core-7.
- Candidate 41 (correctness), bounds hygiene: its derived `Debug`/`Eq` half is tree-core-2; its `Send + Sync` and `Root: PartialEq` halves are tree-core-4.
- Candidate 53 (perfapi), `Root`'s hand-written `PartialEq`: duplicate of tree-core-4.
- Candidate 54 (perfapi), `react`'s `M` generic: duplicate of tree-core-14.
- Candidate 17 (prose), `traverse.rs` ghost `Levels` and misdescribed trio, and 18 (prose), the zipper: merged into tree-core-26 as one pattern (ghosts of the V1 mirror).
- Candidate 30 (prose), comments that narrate the next line: duplicate of tree-core-28.
- Candidate 34 (correctness), `join_associative` testdoc: duplicate of tree-core-34.
- Candidate 37 (correctness), observer contract and "once per changed key": merged into tree-core-11, which states its rule; the refutation pass's new item on the `# Panics` reach is folded in there as well.
- Candidate 42 (correctness) and 51 (perfapi), the "2-3x" figure: duplicates of tree-core-10.
- Candidate 43 (correctness), `TwoPass` framing, and 24 (prose), "now rejects": merged into tree-core-35 as temporal framing in test prose.
- Candidate 47 (perfapi), join clones the fan three times: merged into tree-core-31 with candidate 3; one rewrite satisfies both.
- Candidate 48 (perfapi), act's reassembly pays `Fan::from_iter`'s sort fallback: merged into tree-core-27 as the second redundant sort in the same walk.
- Candidate 31 (prose), `naive_max_version_bytes` is a renamed call: merged into tree-core-19.
- Candidate 32 (prose), missing module docs and inline `mod test`: merged into tree-core-25.
- Candidate 21 (prose), `idx`'s doc: merged into tree-core-18 with candidate 20 as the same pattern (helper preconditions that are false).
- The refutation pass's new item 2 (`react`'s "causally latest wins" fails for `[Forget(v2), Insert(v1)]`): folded into tree-core-30 as the second construction rather than a separate finding.
- The refutation pass's new item 3 (`streaming/stats.rs:45` and `streaming/tests/fixtures.rs:322` say "content-addressed"; `streaming/tests.rs:154-156` repeats the shared-filter claim): out of this partition; noted as related sites under tree-core-23 and tree-core-1 for the streaming partition to pick up.
- Nothing was dropped as refuted: the refutation pass confirmed every candidate and reframed one (35, now tree-core-33 at low severity).
