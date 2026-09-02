# Performance

This document collects every finding of the holistic review of `rumors` at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 whose primary class is performance: where the crate does avoidable work, per what denominator, and how sure we are. No benchmark was run in this review. Every cost below was established by reading the code and, where the finding says so, the pinned dependency sources; the one sweep that ran the test suite (suite-economics) ran it once, on a loaded machine, and its timings are indicative, as its own section says. Each entry therefore carries a sign: *fixed* means the change is a strict removal of redundant work, whose direction cannot depend on workload, and the recommendation is to adopt it and confirm with the named meter or bench; *workload-dependent* means the change trades one resource for another, and the recommendation is to measure first with the named instrument and land only on measured evidence. Ids have the form `<partition or sweep key>-<n>`; the full record for each, including the lens reports it was distilled from, lives in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md` beside this file. Severities are the finalizers': *high* (a defect that breaks a stated contract or masks a failure), *medium* (a cost or gap worth scheduling on its own), *low* (worth fixing when the file is next open), *nit* (a small consistency or legibility item). Provenance is stated per entry: *verified* means run or mechanically checked (grep, arithmetic, a read of the pinned dependency source); *assessed* means established by reading the crate's code; *demonstrated* means a constructed test exists (no performance entry reached that level, and none appears in the review's witness file). Findings of other classes that bear on a cost are cited by id under "See also" and are not restated here; their records live in the sibling documents.

## Highest-value items

1. The commit path every `send`, `send_all`, `redact`, and `Batch` takes inside the watch write lock sorts and re-materializes its action list at each of the 32 heights and then re-sorts each touched fan on reassembly, where one stable sort at entry and a two-way merge would do (tree-core-27; the fan-doc contradiction is tree-typed-30, and the no-op commit that discards the root's memos is tree-core-29).
2. Every live message retains its encoding vector's power-of-two slack plus a `bytes::Shared` header, about 60 percent overhead on the serialized cache at the design record's message size, against a headline claim that memory scales with the live set (api-core-10).
3. Every supplied leaf record decodes its version atom through the general CBOR reader, paying a 4 KiB stack zero and a `Vec` allocation that the head primitive the greeting already uses would delete; the change also closes a canonicality gap and reopens the B2 ruling, so it is owner-gated (remote-codec-24).
4. The routed TCP example, the `Conn` docs, and both TCP test harnesses leave Nagle enabled on unidirectional connections that write one to three small pieces per frame, the canonical delayed-ACK stall; the fix is `set_nodelay(true)` and one sentence in the docs (link-14).
5. `Local::children` deep-clones a shared node through `Arc::make_mut` to yield its children and drops the copy at once, once per disputed parent the walk explodes (streaming-backend-window-9).
6. Every decoded reply, supply-free disputes included, builds a FAN-slot channel, two boxed streams, and a `join`, and the capacity is defended as "load-bearing for liveness" with no mechanism named; the claim needs a construction and the shape needs a meter before any redesign (remote-adapter-streams-6, remote-proxy-27).
7. The swarm example, the crate's showcase and documented measurement tool, encodes its `Vec<u8>` payloads as CBOR integer arrays, about 1.9 wire bytes per random payload byte, where `bytes::Bytes` encodes as a byte string (swarm-example-5).
8. The geometry-search fixture recomputes about 262 thousand SHA3 hashes at opt-level 0 on every call, fifteen calls across ten proxy tests, about 24 CPU-seconds per suite pass on a value that never changes (suite-economics-2; the hand-maintained "attempt 1581" is suite-economics-3 and tree-core-24).
9. Four proptests draw 256 cases from input spaces of 10 to 32 points, about 16 CPU-seconds where exhaustive loops cost under one second and prove more (suite-economics-4).
10. The suite's wall-clock critical path is one 19.9 s capacity test whose 32-parent width no record justifies; a run at 4 parents either shrinks it about eightfold or produces the missing sentence (suite-economics-7).
11. Four `multi_peer` properties, `sanity`'s panic-freedom test, a census test, and a corners test repeat identical generated executions to assert different things, roughly 20 CPU-seconds per pass (suite-economics-5; the same sites from the partition side are tests-lifecycle-9 and tests-lifecycle-29).
12. Both hash preimages and every node's compressed prefix live in heap `Vec`s though each is statically bounded; the leaf preimage is a strict deletion, the branch buffer and the prefix are trades to measure with `benches/branch_hash.rs`, `benches/in_memory.rs`, and the node census (tree-typed-6, tree-typed-23).

## Crate-wide patterns

Six patterns recur across the entries below. Each is stated once here with its full site list; the per-module entries carry the site-specific claim and resolution.

**Bounded data in heap vectors.** Several values whose size is fixed by the tree's geometry are stored or built in a `Vec`: the two hash preimages (tree-typed-6, `src/tree/typed/hash.rs:113` and `:176-177`), the compressed prefix on every node (tree-typed-23, `src/tree/typed/untyped.rs:87`), the retained question's radix list (remote-adapter-streams-16, `src/tree/mirror/streaming/remote/adapter/scope.rs:14`), the causal observer's tiebreak key (api-core-29, `src/rumors/causal.rs:101`), and the version atom of every supplied record (remote-codec-24, `src/tree/mirror/streaming/remote/codec/frame.rs:364-365`). The message cache's slack (api-core-10, `src/message.rs:394`) is the same pattern from the other side: a vector kept after it stopped growing. Each is one allocation per item on a hot path; the fixed-size replacement is a strict deletion except where the inline form grows every instance (tree-typed-23) or would put kilobytes on the stack in a recursion (the branch half of tree-typed-6).

**Deep clones through `Arc::make_mut` on shared handles.** `into_children` clones the whole `NodeInner` when the handle is shared and then takes the fan out of the copy. The walk pays it once per exploded parent (streaming-backend-window-9, `src/tree/mirror/streaming/backend/local.rs:138`), and the commit path pays it at every spine node even when no group produces an update (tree-core-29, `src/tree/traverse/act.rs:96`). Related clones of other classes: `Tree::hash` clones the whole `Root` to borrow a field (tree-core-7), `join`'s divergent arm clones the fan three times (tree-core-31), `Handshaken` keeps a cloned `Greeting` to read two scalars (mirror-common-16), and a `Version` is cloned needlessly at `gossip.rs:681-694` (session-bookmark-11) and `examples/swarm.rs:618-622` (swarm-example-13).

**One site, two findings.** Three costs were found independently from both sides of a boundary and their resolutions should land once: the reassembly at `src/tree/traverse/act.rs:129` violates the fan doc's one-pass promise at `src/tree/typed/untyped/fan.rs:181-183` (tree-core-27 and tree-typed-30); the per-reply channel at `src/tree/mirror/streaming/remote/adapter/decode.rs:288-291` is paid at the pump's call site `src/tree/mirror/streaming/remote/proxy/work/pump.rs:227-235` (remote-adapter-streams-6 and remote-proxy-27); and the repeated executor passes in `tests/multi_peer.rs` and `tests/sanity.rs` were filed by the sweep (suite-economics-5) and by the partition (tests-lifecycle-9, tests-lifecycle-29), with the `window_corners` and `window_census` halves of suite-economics-5 also filed as tests-resource-link-window-26 and tests-resource-link-window-19.

**Meters exist for the codec and nowhere else.** `tests/encode_alloc.rs` and `tests/decode_alloc.rs` hold the frame writer and the declared-length reads to allocation ceilings with liveness floors. No committed number holds message residency (api-core-10), the walk's per-scope allocations (materialized-30), a supply-free reply's fixed decode allocations (remote-proxy-27), the commit path's allocations per action (tree-core-27), or `Local::children`'s allocations per exploded parent (streaming-backend-window-9). Every one of those entries therefore begins its resolution with a `stats_alloc` region in the style of the existing meters, committing the current number before the change tightens it.

**Verification that repeats itself.** Identical generated executions repeated to assert different things (suite-economics-5, tests-lifecycle-9, tests-lifecycle-29, remote-proxy-tests-12, remote-proxy-tests-18, remote-adapter-tests-21, tests-bookmark-13); finite input spaces of a dozen points sampled 256 times (suite-economics-4); a deterministic fixture recomputed on every call (suite-economics-2); and a 2.2-million-case enumeration with a fresh allocation per case (remote-capture-atlas-28). Two of these name the same amplifier: `rumors` and `sha3` compile at opt-level 0 in the dev profile (Cargo.toml raises it only for `before` and `suanpan`), so every hash-heavy fixture pays for the profile choice the manifest documents.

**Performance claims in prose with no committed instrument.** The one in-class instance is `Fan::from_iter`'s doc promising one pass for every reassembly in the crate (tree-typed-30). The sibling documents hold the others: `act`'s "2-3x" (tree-core-10), "the winning window is attempt 1581" (tree-core-24, suite-economics-3), the version-bounds pruning the benches advertise (benches-envelope-13), the dominance certificate `window.rs` cites (benches-envelope-32), `Local::assemble`'s unpriced transient (streaming-backend-window-10), a testdoc quoting 60 B / 65,404 / ~4.3x where the assertions say 52 B / 91,941 / ~2.6x (streaming-backend-window-37), and hop counts recorded in comments beside bands no assertion enforces (tests-resource-link-window-24).

## Crate root and public surface

### api-core-29: `CausalMessages` copies each staged version's bytes into a fresh `Vec` for the tiebreak key
- Where: src/rumors/causal.rs:99-102 (related: src/rumors/causal.rs:54-63, crates/before/src/version.rs:91-95, crates/before/src/codec/bits.rs:91-97, crates/before/src/version/rank.rs:882, crates/before/src/version/ranked.rs:372-383, tests/causal.rs:146-153)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read; `Version` is `#[derive(Clone)]` over a refcounted `Bytes`, so its clone is O(1) and `as_bytes` is a borrow; `Rank: Ord`; not measured)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (9c73d7b4 chose the `(Rank, bytes)` key and documents the order, not the copy; `Version::clone` was already O(1) then)
- Owner-gated: no
- Sign: fixed (a strict deletion of a copy; the order is unchanged). Confirm with a `stats_alloc` region around one ingest pass.

Per staged leaf, `ingest` allocates a `Vec<u8>` copy of `version.as_bytes()` as the rank tiebreak in the `BTreeMap<(Rank, Vec<u8>), Leaf>` key, while the `Leaf` already owns the version and cloning it is a refcount bump. Denominator: one heap allocation and a version-length copy per message delivered through `CausalMessages`, plus a `Vec` header in every staged key. Materializing `Rank` once per leaf stays justified: `Ranked::cmp` is a rank co-sweep per comparison.

Evidence:

    99	        while let Some((_, leaf)) = walk.next() {
   100	            let version = leaf.version();
   101	            staged.insert((version.rank(), version.as_bytes().to_vec()), leaf);
   102	        }

    63	    staged: BTreeMap<(Rank, Vec<u8>), Leaf>,

Resolution: A private `struct StageKey { rank: Rank, version: Version }` whose `Ord` compares `(rank, version.as_bytes())` lexicographically and whose `Eq` is derived (`Version`'s `Eq` is canonical byte equality, so the two agree); key `staged` by it, cloning the leaf's `Version`. `before` needs no change. Acceptance: a `stats_alloc` region around one ingest pass shows one fewer allocation per delivered message; tests/causal.rs's `(rank, bytes)` ordering assertion still passes.

See also: tests-observation-3 (whether the `(rank, canonical bytes)` order becomes a public promise), which decides how the key's doc is worded but not the copy.

## Session and bookmark

The one entry here carries an `api-core` id: the api-core partition filed it because `Batch::send` is its only call site in that partition, while its anchor is `src/message.rs`, which this section owns.

### api-core-10: Every stored message retains its encoding `Vec`'s slack capacity plus a `bytes::Shared` header
- Where: src/message.rs:393-396 (out of the api-core partition's file list; related: src/message.rs:285-290, src/message.rs:341, src/batch.rs:71, src/lib.rs:14-16, tests/encode_alloc.rs:1-10)
- Class / severity / confidence: performance / medium / high
- Provenance: verified (read bytes 1.11.1 src/bytes.rs:960-993, the version Cargo.lock pins: `From<Vec<u8>>` takes the allocation-free boxed-slice path only when `len == cap`, and otherwise allocates a `Box<Shared>` and retains the vector's full `cap`; read `to_vec` and both `Bytes::from(Vec)` sites; the slack figure is arithmetic on the doubling sequence, and no meter was run)
- Seen by: perfapi; refutation: confirmed (correcting the reader's bytes version from 1.12.1 to the locked 1.11.1; the code is identical); history: no rationale found (no note or meter covers message residency; tests/encode_alloc.rs meters the frame writer)
- Owner-gated: no
- Sign: fixed for the memory term (strictly fewer resident bytes and one fewer allocation per send); roughly neutral for CPU (one shrinking realloc replaces one header allocation). Confirm with the residency meter the resolution lands.

The per-send path (`Batch::send` at batch.rs:71, then `PayloadCodec::message`, then `Message::try_from_arc`) serializes into a `Vec` that starts at `Vec::new()` and grows by doubling as ciborium writes piecewise, then stores it with `Bytes::from(serialized)`. Nearly every live message therefore carries its encoding's power-of-two slack plus a header for its whole lifetime: at the design record `m = 172` (peer.rs:431) that is a 256-byte capacity, so 84 bytes of slack plus a header of roughly 24 bytes, about 60 percent overhead on the serialized cache the crate keeps per message so gossip can supply cached bytes. The wire path copies the cache with `serialize_bytes(&self.serialized)` (message.rs:565-568) rather than cloning the `Bytes`, so the header is a pure resident cost today. Denominator: per live message, resident bytes, plus one extra allocation per send. The crate's headline claim is that memory scales with the live set (lib.rs:14-16), and this is redundant work with a fixed sign, so under the doctrine it defaults to construct-and-measure.

Evidence:

    285	fn to_vec<T: Serialize>(value: &T) -> Vec<u8> {
    286	    let mut buf = Vec::new();

    393	        Ok(Message {
    394	            serialized: Bytes::from(serialized),
    395	            message: arc,
    396	        })

Resolution: At message.rs:394 and 341 write `Bytes::from(serialized.into_boxed_slice())`: `into_boxed_slice` shrinks the allocation (usually in place) and the boxed-slice path uses the promotable vtable with no header allocation and no slack. Land a `stats_alloc` meter in the style of tests/encode_alloc.rs around one `Rumors::send` (or a `Batch::commit` of N sends) pinning bytes retained per message to the payload `Arc` plus the exact encoding length plus tree nodes, committing the current bad number first per the metering practice. Acceptance: the committed meter shows zero retained slack and one fewer allocation per send than the parent commit's baseline; `benches/in_memory` `batch_insert` is neutral or better.

See also: benches-envelope-12 (the `batch_insert` bench's timed body includes dropping the tree and an `OsRng` draw, which the acceptance measurement should exclude first); the api-core partition's dropped note that `Message::from_wire` storing sliced run buffers is the ingress twin of this cost and was left for the streaming decoder's reviewer, where no entry filed it.

## Link

### link-14: The routed TCP guidance never mentions `TCP_NODELAY`; the routed shape is the Nagle-plus-delayed-ACK worst case
- Where: src/link/routed.rs:118-125 (related: src/link/routed.rs:193-197, src/link/routed.rs:206-207, src/link/routed.rs:213-216, src/link/routed/stream.rs:51-52, src/tree/mirror/streaming/remote/streams.rs:209, src/tree/mirror/streaming/remote/codec/encode/async_io.rs:80-96, tests/common/routed_tcp.rs:34-43, tests/common/routed_tcp.rs:57-67, tests/common/tcp.rs:50-52)
- Class / severity / confidence: performance / medium / medium
- Provenance: verified (absence: `grep -rn -i 'nodelay|nagle' src tests examples benches justfile Cargo.toml` returns nothing; write structure: `write_encoding` writes the frame head and then zero, one, or two body pieces as separate `write` calls before `flush`, and a stream open writes the 28-byte header at stream.rs:52 and the label at streams.rs:209 as separate writes). The latency magnitude is assessed from the mechanism, not measured; no network experiment was run.
- Seen by: perfapi (34); refutation: confirmed, "medium stands conditionally"; history: no rationale (the design record deprioritized dial cost, not per-frame ACK stalls; no note mentions Nagle)
- Owner-gated: no
- Sign: fixed for this shape (the crate already flushes at every frame boundary and forbids user-space write buffering, so Nagle can only add waiting); the magnitude is unmeasured, so confirm with the wall-clock comparison the acceptance names, and if it shows nothing the finding reduces to the documentation change.

The module's TCP example dials a raw `TcpStream::connect`, the `Conn` docs say `tokio::net::TcpStream` satisfies the connection obligations with no caveat, and the `Dial` docs list socket options as the implementation's business without naming the one that governs this protocol's latency. Tokio leaves Nagle enabled by default. A routed data stream is a unidirectional connection carrying frames written as one to three `write_all` pieces each, preceded at open by a header write and a label write; on a unidirectional connection no reverse traffic piggybacks ACKs, so every small piece after the first waits for the peer's delayed-ACK timer (tens of milliseconds on common stacks). The sign is fixed for this shape because `FrameWrite::frame` already flushes at every frame boundary and the `Conn` contract forbids user-space write buffering, so Nagle can only add waiting. Users copy the example as their deployment template. Denominator: per frame piece after the first on each TCP data stream, in wall-clock latency.

Evidence:

    118	//! impl Dial for TcpDial {
    119	//!     type Addr = SocketAddr;
    120	//!     type Conn = TcpStream;
    121	//!
    122	//!     async fn dial(&self, addr: &SocketAddr) -> io::Result<TcpStream> {
    123	//!         TcpStream::connect(*addr).await
    124	//!     }
    125	//! }

    206	/// `tokio::net::TcpStream` satisfies both, as does anything else whose
    207	/// writes land in the transport as they are accepted.

    (async_io.rs)
    84	    write(out, FramePart::FrameHead, encoding.head.as_slice()).await?;
    85	    match &encoding.body {
    86	        BodyEncoding::Empty => {}
    87	        BodyEncoding::Listing(listing) => {
    88	            write(out, FramePart::QueryChildren, listing).await?;
    89	        }
    90	        BodyEncoding::Supply { head, run } => {
    91	            write(out, FramePart::SupplyLength, head.as_slice()).await?;
    92	            write(out, FramePart::SupplyRun, run.as_bytes()).await?;
    93	        }
    94	    }

Resolution: in the example `TcpDial::dial` call `stream.set_nodelay(true)?` before returning, and in the example `TcpListen::accept` for the accepted connection (whose writes are the ACK/READY bytes and the control stream's frames). Name `TCP_NODELAY` in the `Dial` docs' policy list (routed.rs:214-216) and add a sentence to the `Conn` docs (routed.rs:193-197) that Nagle is the kernel-side form of the hidden write buffering the clause warns about: liveness survives, latency does not. Apply the same to `tests/common/routed_tcp.rs` (both dials) and `tests/common/tcp.rs` so the crate's own TCP runs measure the intended configuration. Acceptance: the example, the `Dial` docs, and both TCP test harnesses set or name `TCP_NODELAY`; a before/after wall-clock timing of `tests/routed_link.rs`'s pooled mutual-gossip test over loopback shows the per-frame stall gone. If the measurement shows no difference, the finding reduces to the documentation change alone.
Construction: two processes over real TCP with the example `TcpDial`; capture a data-stream connection with tcpdump during a reply phase and observe the gap between the frame-head segment and its body segment equal to the receiver's delayed-ACK timeout; repeat with `set_nodelay(true)` and observe the gap vanish.

See also: the remote-codec partition's dropped candidate that supply frames cost three transport writes plus a flush (refuted as workload-dependent and deliberate for `FramePart` attribution); with `TCP_NODELAY` set the piecewise writes cost only syscalls, which is the regime that refutation assumed.

### link-27: Per-connection micro-costs in the router and stream connector, all strict deletions
- Where: src/link/routed/router.rs:152-157 (related: src/link/routed/router.rs:146, src/link/routed/router.rs:159-163, src/link/routed/router.rs:172, src/link/routed/router.rs:274, src/link/routed/stream.rs:53-55)
- Class / severity / confidence: performance / nit / high
- Provenance: assessed (read)
- Seen by: perfapi (40); refutation: confirmed, severity down to nit (each is paid beside a dial per open); history: no rationale
- Owner-gated: no
- Sign: fixed (three strict deletions); no meter warranted at this magnitude.

Three fixed-sign deletions: (a) the drive loop retires a finished header read with `order.retain(..)`, a linear scan over up to `pending_headers` entries, per accepted connection, and `pending_headers` is the bound the docs tell users to size generously; ids are monotonic, so a `BTreeMap<u64, AbortHandle>` (`pop_first` to evict, `remove(&id)` to retire) is both O(log n) and simpler than a `VecDeque` plus `retain`. (b) `dial.clone()` at router.rs:172 runs for every accepted connection but only the `Header::Link` arm uses it. (c) `StreamConnector::connect` clones `dial` and `peer` per open to build the recycle `Done`. Magnitude is small at defaults; (a) is the one that grows with a user-chosen constant. Denominator: per accepted connection (a, b) and per stream open (c).

Evidence:

    152	            Some(finished) = pending.next() => {
    153	                if let Ok(id) = finished {
    154	                    order.retain(|(pending_id, _)| *pending_id != id);
    155	                }
    156	                continue;
    157	            }

Resolution: (a) `order: BTreeMap<u64, AbortHandle>`; evict with `pop_first()`, retire with `remove(&id)`. (b) pass `&dial` into `deliver` and clone only in the `Link` arm. (c) optional: hold `Arc<(D, D::Addr)>` in `StreamConnector`. Acceptance: behavior-preserving; `pending_header_bound_evicts_oldest`, `stalled_header_does_not_park_the_router`, `queue_overflow_evicts_the_link` pass unchanged; no new meter warranted at this magnitude.

See also: link-28 (the single `pending_headers` bound evicting pooled idle connections, a correctness finding whose fix reshapes the same `order` structure and should land with or before (a)); link-26 (the routing table's missing newtype).

## Tree core

### tree-core-27: `act` sorts and re-materializes the action list at every one of the 32 heights, and re-sorts each touched fan on reassembly, where one stable sort at entry would do
- Where: src/tree/traverse/act.rs:86-93 (related: src/tree/traverse/act.rs:101-109 and 128-129; src/tree.rs:483-497; src/tree/typed/path.rs:49-58 and 86-90; src/tree/typed/untyped.rs:229-258 and 276-340; src/tree/typed/untyped/fan.rs:178-200; benches/in_memory.rs:91-92 and 137-138; src/tree/tests.rs:1655-1660)
- Class / severity / confidence: performance / medium / high
- Provenance: verified for the mechanism (read `itertools-0.14.0/src/lib.rs:3101-3110`: `sorted_by_key` is `Vec::from_iter(self)` then `sort_by_key`, a fresh `Vec` and a comparison sort per call; `Path<H>::Ord` compares the unconsumed suffix, path.rs:86-90, and `pop` takes its first byte, path.rs:49-58; `Fan::from_iter` detects a non-ascending run and pays `sort_by_key` plus a dedup copy, fan.rs:184-200); assessed for the magnitude (no bench was run)
- Seen by: perfapi (44, 48); refutation: confirmed (the typed walk visits every height even through compressed spines, since `into_children` on a prefixed node pops one prefix byte into `Fan::unit`, untyped.rs:230-239, so a fresh leaf pays this at all 32 levels); history: no-rationale-found (the per-level `sorted_by_key` dates to fe3612311 and was never justified; 262568f9e named the grouping "the radix sort" and the `react` comment inherited the phrase; `fan.rs:181-183` asserts "Every reassembly in the crate feeds pairs already strictly ascending", which act.rs:129 falsifies)
- Owner-gated: no for the sort deletion (adoptable now, no API or wire change); the larger slice-recursion redesign is an owner decision, listed under open questions
- Sign: fixed for both deletions (one sort at entry replaces 32; a merge replaces the fan's fallback sort). Confirm with `benches/in_memory.rs` `batch_insert` and `redact` at the parent commit and after, and with a per-commit allocation count.

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

Resolution: Adoptable now: in `Tree::react` (tree.rs:491-497) sort the collected `Vec` once, stably, by path (`actions.sort_by_key(|(path, ..)| *path)`), state "sorted by `Path<Self>`" as `Act::act`'s precondition, and replace `.sorted_by_key(..).chunk_by(..)` with `.chunk_by(..)` alone; fix the `react` comment to describe the one sort that remains. For the reassembly, merge the two ascending runs (`updated.into_iter().merge_by(existing_children, |a, b| a.0 < b.0)`; radixes are disjoint because each updated child was `remove`d) or insert recursed children back into `existing_children` and drop `updated`, so `Fan::from_iter` takes its one-pass branch and fan.rs:181-183 becomes true. Acceptance: `benches/in_memory.rs` `batch_insert` and `redact` measured at the parent commit and after, identical-or-improved at every N, with fewer allocations (an allocation count per commit can be pinned the way `tests/encode_alloc.rs` pins encode); `react_batch_partitioning_preserves_hash`, `tree_shape_is_canonical_in_the_leaf_set`, and `insert_and_delete_same_batch_is_empty` still pass, proving order semantics survived; a debug assertion or test that the reassembled iterator is ascending.

See also: tree-typed-30 (the same reassembly from the fan's side; one merge satisfies both); tree-core-29 (the same walk's no-op cost); tree-core-10 (`act`'s rustdoc quotes a "2-3x" no committed bench produces, so the acceptance bench doubles as that figure's re-derivation); benches-envelope-12 (`batch_insert`'s timed body includes the tree drop and an `OsRng` draw); async-hazards-3 (payload destructors also run inside this lock, so shortening the hold is the shared goal).

### tree-core-29: A no-op `act` rebuilds the root spine under a new handle and discards its memos; `Unknown for S<H>` rebuilds a `Between` node whose every child survived
- Where: src/tree/traverse/act.rs:96-96 (related: src/tree/traverse/act.rs:115-121 and 128-129; src/tree.rs:384-385 and 526; src/tree/typed/untyped.rs:212-221 and 229-258; src/batch.rs:129-131; src/tree/traverse/unknown.rs:65-73)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (traced: `react` hands the walk `self.root.root.clone()`, so the root `Arc` is shared; `S<H>::act` calls `into_children()` unconditionally, which `Arc::make_mut`s the inner and takes the fan, untyped.rs:236-239 and 247-254; `Node::branch` allocates a fresh `NodeInner` with fresh `OnceLock`s, untyped.rs:212-221; an empty batch, or a batch of forgets for absent keys where every group `continue`s at 115-121, therefore commits a memo-less root under a new handle while returning `false`)
- Seen by: correctness (39); refutation: confirmed (the observable contract, hash and ceiling and content unchanged, holds, so "contract breached" would overread; the cost is one root-fan re-fold on the next read and a missed `ptr_eq` against earlier snapshots); history: no-rationale-found (no `actions.is_empty()` guard has ever existed; 2d3a86d3f priced the fresh spine for effectual batches, not for the empty case)
- Owner-gated: no
- Sign: fixed (an early return and a handle-preserving arm delete work without changing any observable). Confirm with the pointer-identity test the acceptance names.

`Tree::act` promises "An empty batch is a complete no-op ... the tree is unchanged", and `Batch::commit` relies on that sentence to skip a special case. The promise holds observationally, but the root is replaced by a memo-less copy: the next `hash()` or `earliest()` re-folds the root fan (up to 256 child hashes and spans), and `ptr_eq` short-circuits against snapshots taken earlier fail at the root, falling back to the hash. The memos are the tree's amortization argument (tree.rs:38-51), and the root-hash meter exists because re-hashing the spine happens under the watch lock. The same shape recurs in `unknown.rs:65-73`, which rebuilds a `Between` subtree even when every child survives, losing sharing for the kept side of `join`'s asymmetric arm. Denominator: per no-op commit (one root-fan re-fold deferred to the next read, plus one lost pointer-equality short-circuit per later comparison); per fully surviving `Between` subtree in `join`.

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

See also: streaming-backend-window-9 (the same `into_children` clone on a shared handle, paid by the walk); tree-core-31 (`join`'s divergent arm's clones, whose rewrite is the natural place for the `Unknown` arm's handle-preserving return).

## Tree typed

### tree-typed-6: Both hash preimages heap-allocate a `Vec` per computation although each is statically bounded
- Where: src/tree/typed/hash.rs:110-118 (related: src/tree/typed/hash.rs:173-177, src/tree/typed/untyped.rs:464-479, benches/branch_hash.rs:14-18)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read; not measured)
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found (the bench header says `contiguous` includes the allocation to measure the end-to-end cost, not as a verdict on the buffer type)
- Owner-gated: no
- Sign: fixed for `Hash::leaf` (a 34-byte inline buffer strictly deletes the allocation); for `Hash::branch` a `SmallVec` sized for the modal fan is a small stack-for-heap trade whose spill point must be chosen, so measure it with the `stack` curve the resolution adds to `benches/branch_hash.rs`.

`Hash::leaf` allocates a `Vec` for at most 2 + 32 bytes and `Hash::branch` a `Vec` for at most 4 + 32 + 256 × `CHILD_RECORD_LEN` bytes. Both run inside `Node::hash`'s memo fill, so the allocation is paid once per node-hash computation: every fresh spine node after a commit, and every virtual level that `into_children`/`beneath` exposes (each resets the memo). A stack buffer strictly deletes the malloc/free without changing a preimage byte (fixed sign; wire snapshots untouched). The committed bench never compared against this alternative. Denominator: one allocation per node-hash computation.

Evidence:

       113	        let mut buf = Vec::with_capacity(2 + suffix.len());

       176	        let mut buf =
       177	            Vec::with_capacity(4 + prefix.len() + CHILD_RECORD_LEN * children.size_hint().0);

    benches/branch_hash.rs:
        16	//! which the hash tests pin byte-for-byte. `contiguous` reproduces the
        17	//! shipped form including its per-call buffer allocation, so the measured
        18	//! difference is the end-to-end cost a caller sees, not the hash core alone.

Resolution: in `Hash::leaf`, assemble into a `tinyvec::ArrayVec<[u8; 34]>` (tinyvec is already used by untyped.rs and prefix.rs); in `Hash::branch`, a `SmallVec` sized for the modal fan (fan.rs:5-8: interior branches rarely carry more than a handful of children), so hot small nodes stay allocation-free and the saturated case spills. Add a `stack` curve to `benches/branch_hash.rs` so the shipped form's claim stays re-measurable against this alternative. Acceptance: hash/tests.rs and the wire snapshots unchanged; the bench shows the stack arm at or below `contiguous` at every fan-out, recorded in the commit message.

Synthesis note: the sizing in the resolution matters for the branch half. `Node::hash`'s memo fill recurses down an unmemoized spine, so a buffer sized for the saturated fan (several KiB) would sit on the stack once per unmemoized level, up to 32 deep; the modal-fan `SmallVec` is what keeps this a deletion rather than a stack cost, and the `stack` bench curve is the right instrument for the spill point.

See also: benches-envelope-1 (the bench restates `Hash::branch`'s preimage by hand with nothing tying the copy to the shipped function; adding a curve to it inherits that gap).

### tree-typed-23: The compressed prefix is a heap `Vec<u8>` per node; an inline `ArrayVec<[u8; 32]>` would delete the allocation (measure first)
- Where: src/tree/typed/untyped.rs:78-87 (related: src/tree/typed/untyped.rs:213, 236, 299, 331, 345, 470, 661; src/tree/typed/untyped/tests.rs:876-893)
- Class / severity / confidence: performance / low / medium
- Provenance: assessed (read; not measured)
- Seen by: perfapi; refutation: uncertain (a resource trade, not a strict deletion: `NodeInner` grows for every node and the `<= 208` pin moves); history: no-rationale-found (`prefix: Vec<u8>` is original; .agent-notes/2026-07-18-node-hash-preimage records allocation in the node path moving a session number)
- Owner-gated: yes: the size pin's own doc says growth must be a deliberate, reviewed decision
- Sign: workload-dependent (one allocation and one heap block per compressed node deleted, against about 16 bytes added to every `NodeInner`, leaves included). Measure first with `benches/in_memory.rs`, a `stats_alloc` count per inserted leaf, and `testing::node_census`.

Every `NodeInner` stores its prefix as a `Vec<u8>`, so every compressed node (in a uniform-hash tree nearly every leaf, whose spine runs about 28 to 30 bytes) owns a heap block besides its `Arc`, and `beneath`, `from_sorted_leaves`, and `into_children` under sharing touch the allocator for it. The prefix is bounded at 32 by the height cap, and `hash()` already reverses it into a stack `ArrayVec<[u8; 32]>`. Against that, `ArrayVec<[u8; 32]>` is 34 bytes to `Vec`'s 24, so `NodeInner` grows about 16 bytes after padding for every node, leaves included, and the sign is workload-dependent. Doctrine: resource trades get measure-first gating. Denominator: per node, resident bytes and allocations.

Evidence:

        87	    prefix: Vec<u8>,

       470	            let prefix: ArrayVec<[u8; 32]> = self.inner.prefix.iter().rev().copied().collect();

    src/tree/typed/untyped/tests.rs:
       892	    assert!(std::mem::size_of::<super::NodeInner>() <= 208);

Resolution: construct and measure: change `prefix` to `ArrayVec<[u8; 32]>` (`Vec::new()` becomes `ArrayVec::new()` at 213 and 345; the `extend`/`push`/`pop`/`collect` sites keep their API; the reverse-collect at 470 can iterate `rev()` directly), then compare at the parent commit and after with `benches/in_memory.rs`, the `stats_alloc` pattern from `tests/decode_alloc.rs` (allocations per inserted leaf), and `testing::node_census`. Land only on measured evidence, updating `node_inner_stays_within_budget` deliberately with the new size named in the commit. Acceptance: a committed before/after comparison with the pin updated in the same commit, or the proposal recorded as declined with the numbers.

See also: the tree-typed partition's open question on the supply path's bare-leaf rebuild (one `Arc<NodeInner>`, a `Version` clone, and a `Message` clone per supplied leaf, deliberate and documented, not filed as a finding), which is the other per-node allocation on the wire path and would be measured by the same instruments.

### tree-typed-30: `Fan::from_iter`'s "every reassembly arrives ascending" claim is false for the commit path, which pays the sort
- Where: src/tree/typed/untyped/fan.rs:178-200 (related: src/tree/traverse/act.rs:92, 112, 129; src/tree/typed/node.rs:88-96)
- Class / severity / confidence: performance / low / high
- Provenance: verified by reading act.rs:86-129 (`sorted_by_key` makes `updated` ascending; `existing_children.remove(radix)` at 112; `updated.into_iter().chain(existing_children).collect()` at 129 is ascending only when every updated radix is below every untouched one: existing {1, 5} with an update at 5 yields [5, 1]); `git log -S'Every reassembly in the crate feeds pairs'` names f70f9559a ("Landed dark"), and the swap that wired act.rs to the fan (8f87ddd01) does not mention act.rs's chain order
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found (the claim was never checked against act.rs)
- Owner-gated: no
- Sign: fixed (a two-way merge of two ascending inputs restores the one-pass path; if the code is left as is, the doc must change instead).

The doc states a performance contract (one pass for every crate reassembly) that the hottest reassembly, one per spine node per commit batch, violates: the concatenation at act.rs:129 is unsorted in general, so the fast check fails and the slow path runs a stable sort plus a second `SmallVec` (heap-allocated whenever the fan exceeds `FAN_INLINE` = 2) that rebuilds the entries even when no duplicate exists. Both inputs are individually ascending, so a two-way merge restores the one-pass path with a fixed sign. Denominator: per spine node per commit batch.

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

See also: tree-core-27 (the same site from the walk's side; land the merge once and let both findings close).

## Streaming backend and window

### streaming-backend-window-9: `Local::children` deep-clones the shared node through `into_children` to yield its children
- Where: src/tree/mirror/streaming/backend/local.rs:136-141 (related: src/tree/typed/untyped.rs:164-187, 229-258; src/tree/typed/untyped/fan.rs:135-145; src/peer/gossip.rs:695, 1143)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read `into_children`: `Arc::make_mut` in both arms; `Children::clone` copies `bounds`, `version_bytes`, and the `Fan`; the session root is `inner.tree.clone()` handed to `start(Local, root.into())`, so every handle a session explodes is shared with the live tree; no allocation count was run)
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found (the latency note profiled `Arc::make_mut` under the conversion machinery; the fix at 88a76a71 addressed `leaves`/`assemble`, not the walk's `children`)
- Owner-gated: no
- Sign: fixed (a non-copying cursor over the shared fan deletes the clone outright). Confirm with a `stats_alloc` meter or the census counters on a disputed-fan corpus at the parent commit and after.

`parent.into_children()` reaches the fan through `Arc::make_mut`, which on a shared handle clones the whole `NodeInner` (bounds with two owned `Version`s, the memos, the fan, the prefix) and then `mem::take`s the fan out of the fresh copy, dropping the rest. Every disputed parent the walk explodes pays an allocation, two `Version` clones, and a fan heap allocation past two inline entries, all freed immediately. This is strict deletion of redundant work with a fixed sign; the doctrine says construct and measure. Denominator: per exploded parent.

Evidence:

       136	        let children = stream::iter(
       137	            parent
       138	                .into_children()
       139	                .into_iter()
       140	                .map(move |(radix, child)| Ok((prefix.push(radix), child))),
       141	        );

Resolution: Give the in-memory backend a non-copying child cursor for uncompressed branches: hold the parent handle and resume with `Fan::successor(radix)` (fan.rs:140-145, already documented as the resume point of a suspended ascending walk), cloning one child handle per step; keep the `into_children` path for path-compressed nodes, where popping a prefix byte needs a new node. Measure with a `stats_alloc` meter or the census counters on a disputed-fan corpus at the parent commit and after. Acceptance: a committed meter shows no `NodeInner` allocation per `Local::children` call on a shared uncompressed branch; the `local/tests.rs` equivalence proptests and the join-oracle suite still pass.

See also: tree-core-29 (the commit path's unconditional `into_children` on the same shared handles); async-hazards-4 (the first greeting on a cold tree hashes and bounds the whole tree in one poll through this same `children` entry).

### streaming-backend-window-11: `Local::assemble` buffers each run twice before building the subtree
- Where: src/tree/mirror/streaming/backend/local.rs:204-219 (related: src/tree/typed/node.rs:291-310; src/tree/mirror/streaming/remote/adapter/decode.rs:88, 408)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read node.rs:301-308: `from_sorted_leaves` immediately re-collects the run into `Vec<([u8; 32], Option<untyped::Node>)>` because `untyped::Node::from_sorted_leaves(depth, &mut entries)` wants that shape)
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found (88a76a71 discusses the virtual-level saving only)
- Owner-gated: no
- Sign: fixed (one of two same-length buffers per run is deleted). Confirm with an allocation-count test over a fixed multi-run stream.

Each run is accumulated as `Vec<(Prefix<Z>, typed::Node<Z>)>` and then handed to `from_sorted_leaves`, which allocates a second same-length vector in the builder's entry shape. One extra allocation per run plus a per-leaf move on the decode path; strict deletion, fixed sign. Denominator: per assembled run, one allocation; per supplied leaf, one move.

Evidence:

       204	            let mut run: Vec<(Prefix<Z>, typed::Node<Z>)> = Vec::new();
       205	            while let Some(item) = leaves.next().await {
       206	                let (prefix, leaf) = item?;
       207	                let target = Prefix::<H>::containing(&Path::from(prefix));
    ...
       213	                        typed::Node::from_sorted_leaves(&finished, mem::take(&mut run)),

Resolution: Accumulate the builder's own entry shape directly (push `([u8; 32], Option<untyped::Node>)`, or let `typed::Node::from_sorted_leaves` take the run by `&mut [..]` and build once), moving the containment `debug_assert!` to the byte form. `Prefix::<H>::containing(&Path::from(prefix))` per leaf can become a slice comparison of the first `32 - H::HEIGHT` bytes against the current run's key. Acceptance: one buffer allocation per run, checked by an allocation-count test over a fixed multi-run stream; `full_height_roundtrip_matches_default`, `multi_run_grouping_matches_default`, and `leaf_height_assembly_is_identity` unchanged and passing.

See also: streaming-backend-window-10 (the same function's comment asserts the memory story unchanged without pricing the run-proportional transient this buffer is; pricing it is the documentation half of this cost).

## Materialized

### materialized-30: Fan-bounded per-scope vectors grow from empty, and no meter prices the walk's allocations
- Where: src/tree/mirror/streaming/materialized/work/answer.rs:50-52 (related: work/answer.rs:121-123; work/resolver.rs:54; src/tree/mirror/streaming/materialized/common.rs:29; work/levels.rs:97, 107-108; tests/encode_alloc.rs:1-10; tests/decode_alloc.rs)
- Class / severity / confidence: performance / nit / medium
- Provenance: assessed (read; `tests/encode_alloc.rs` and `tests/decode_alloc.rs` meter the codec only per their module docs)
- Seen by: perfapi; refutation: confirmed, with one caveat (reserving `ours.len() + theirs.len()` over-reserves by up to 2x on all-`Both` scopes, so the reservation is a small memory trade rather than a pure deletion); history: no rationale found
- Owner-gated: no
- Sign: the meter is the finding; the reservation that follows is a small trade (up to a 2x over-reserve on all-`Both` scopes) and lands only when the meter shows the ceiling moving down.

Per answered scope, `internal` and `leaf_parent` build three vectors whose final lengths are bounds of the inputs, and `Resolver` starts `resolved` at zero capacity. The cost is small; what is missing is the instrument: no committed number holds the walk's per-scope allocation count, and instruments come before cures. Denominator: allocation events per answered scope.

Evidence:

    50	    let mut reactions = Vec::new();
    51	    let mut asked = Vec::new();
    52	    let mut resolved = Vec::new();

Resolution: First land a `stats_alloc` meter (same shape as `tests/encode_alloc.rs`) over an in-process `streaming::mirror` session of a fixed disputed shape (e.g. `full_depth_comb_pair`), committing the current allocation-event count as the ceiling with a liveness floor. Then reserve (`with_capacity(ours.len())` in `Resolver::new`, `with_capacity(theirs.len())` for `asked`, a bound for `reactions`/`resolved`) and tighten the ceiling in the same commit. Acceptance: a committed meter whose ceiling the reservation commit lowers, before and after numbers in the commit message.

See also: the materialized partition's open question "A walk-side allocation meter: wanted at all?" (Open questions below); remote-proxy-27 (the ingress twin of the same missing meter).

### materialized-35: `Resolver` owns its `Recorder` and is constructed per resolved scope, so the walks clone the `Arc` once per query
- Where: src/tree/mirror/streaming/materialized/work/resolver.rs:35-38 (related: work/resolver.rs:27-34, 49; work/levels.rs:430, 571, 668; src/tree/mirror/streaming/stats.rs:162-165)
- Class / severity / confidence: performance / nit / high
- Provenance: verified (read the struct: `their_version` and `ledger` are `&'v`, `stats` is owned; the three `Resolver::new` calls pass `stats.clone()` inside the per-query loop; `Recorder` is `Arc<Counters>`, stats.rs:162-165)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (487e17ea added the owned recorder when the struct's only borrowed input was `their_version`; 50c8b0a3 then added `ledger` as a borrow)
- Owner-gated: no
- Sign: fixed (two atomic read-modify-writes per disputed scope deleted). Confirm by grep, as the acceptance says.

Two atomic RMWs (clone and drop) per disputed scope that a `&'v Recorder` field removes; the struct already spells its per-session inputs with one lifetime and the owned recorder is the odd one out. Denominator: per disputed scope.

Evidence:

    35	    /// The session's stats recorder: each absorbed supply credits its
    36	    /// exact live-leaf count as
    37	    /// [`messages_gained`](crate::SessionStats::messages_gained).
    38	    stats: Recorder,

Resolution: `stats: &'v Recorder`; constructor parameter `&'v Recorder`; pass `&stats` at the three sites. Acceptance: `grep -n "stats.clone()" levels.rs` shows only the per-stage clones at the top of each walk body, none inside a `while let Some(query)` loop.

## Remote codec

### remote-codec-12: Listing bulk reads zero-fill their scratch before reading, unlike the body reader's policy
- Where: src/tree/mirror/streaming/remote/codec/decode/async_io.rs:270-276 (related: src/tree/mirror/framing.rs:69-71 and :100-107)
- Class / severity / confidence: performance / nit / high
- Provenance: assessed (read)
- Seen by: perfapi (49); refutation: confirmed (bound 256 × 27 = 6912 bytes per query frame); history: today's 18527932; nothing chooses `resize` over `read_buf`
- Owner-gated: no
- Sign: fixed (a memset of up to about 7 KiB per query frame deleted; consistency more than speed).

`fill_vec` grows the retained listing scratch with `resize(start + want, 0)` and then reads into it, so a query frame memsets up to about 7 KiB before its bytes arrive, while the same codec's body reader (`resume_payload`) reads into spare capacity and documents "no zero fill". Two body readers in one codec use two policies. Denominator: per query frame.

Evidence:

    270	    async fn fill_vec(&mut self, scratch: &mut Vec<u8>, want: usize) -> Arrived {
    271	        let start = scratch.len();
    272	        scratch.resize(start + want, 0);
    273	        let arrived = self.fill(&mut scratch[start..]).await;
    274	        scratch.truncate(start + arrived.filled);
    275	        arrived
    276	    }

Resolution: `scratch.reserve(want)` and loop on `read.read_buf(scratch)` until `scratch.len() - start` reaches `want`, EOF, or error, mirroring `resume_payload`; or factor one bounded-read helper both callers use. Acceptance: `fill_vec` contains no `resize`; the listing proptests and `truncated_bodies_are_rejected` still pass.

See also: remote-codec-14 (the over-budget lone-record read through `resume_payload` can consume bytes of the next frame because `read_buf` fills spare capacity; a shared bounded-read helper must carry that clamp, so the two changes belong together); remote-codec-8 (decode fragments duplicated between the async reader and the sync oracle).

### remote-codec-24: Every supplied record's version atom is decoded through the general CBOR reader: a 4 KiB stack zero and a `Vec` allocation per leaf, and an unjudged head spelling
- Where: src/tree/mirror/streaming/remote/codec/frame.rs:364-365 (related: frame.rs:73-84 and :337-345; src/tree/mirror/streaming/remote/codec.rs:10-16; src/tree/mirror/streaming/remote/codec/greeting.rs:158-178; src/tree/mirror/streaming/remote/codec/decode/tests.rs:467-534; crates/before/src/serde_impls.rs:39-43)
- Class / severity / confidence: performance / medium / high
- Provenance: assessed (read at the locked versions: ciborium-0.2.2 `src/de/mod.rs:825-831` is `pub fn from_reader(...) { let mut scratch = [0; 4096]; from_reader_with_buffer(reader, &mut scratch) }`; `crates/before/src/serde_impls.rs:41-42` is `let bytes = <Vec<u8>>::deserialize(d)?; Version::decode(&bytes[..])`; not measured)
- Seen by: perfapi (41); refutation: confirmed; history: already-known and reopens a ruling: REVIEW.md B2a proposed exactly this resolution and Finch ruled "B2: enforcement alternative declined — the prose rescope plus boundary-pinning witnesses is the resolution of record"; 60a6d191 and 18fbc92f committed the two pins, each saying flipping to rejection is "a deliberate contract change, not drift"; the ruling rested on the detector never firing against any existing encoder, and the performance argument was not before Finch
- Owner-gated: yes (wire acceptance changes; reopens B2)
- Sign: fixed (the scratch zero, the intermediate `Vec`, the serde dispatch, and `de_error` are deleted outright); the owner decision is about wire acceptance, not about the sign. Confirm with the allocator meter over `records()` the acceptance names.

`parse_record` hand-parses the version-atom tag, then hands the byte string to `ciborium::de::from_reader`, which zeroes a 4 KiB stack scratch per call, after which `before`'s `Deserialize for Version` allocates a `Vec<u8>` for the atom bytes before `Version::decode`. Both costs are strictly redundant with the head primitive the crate already uses for the same shape in the greeting (`read_head`, require `MAJOR_BSTR`, `split`, `Version::decode`), and `de_error` exists only to convert the reader's error. The same choice is what leaves the atom's byte-string head unjudged for shortest form and definiteness, documented at frame.rs:77-84 and codec.rs:12-16 and pinned by two tests, so the wire has one canonicality rule for the greeting's atom and a weaker one for a record's. Denominator: per supplied record. This reopens B2 on grounds the ruling did not weigh, and the finding says so.

Evidence:

    364	    let version: Version =
    365	        ciborium::de::from_reader(&mut input).map_err(|e| DecodeLeafError::Version(de_error(e)))?;

    greeting.rs:165	                let head = cbor::read_head(&mut input).map_err(GreetingError::Head)?;
    greeting.rs:166	                if head.major != MAJOR_BSTR {
    greeting.rs:178	                version = Some(Version::decode(atom).map_err(GreetingError::Version)?);

Resolution: In `parse_record`, after the version-tag check, `cbor::read_head(&mut input)`, require `MAJOR_BSTR`, split `head.value` bytes, and `Version::decode(atom)`; report the decoder's error as `DecodeLeafError::Version` (mapping into the existing `io::Error`, or changing the variant to carry `before::error::Decode` as `GreetingError::Version` does). Re-state `widened_version_atom_head_is_not_spelling_judged` and `indefinite_version_atom_head_is_not_spelling_judged` as rejections, and re-denominate the exception sentences at frame.rs:77-84 and codec.rs:12-16 to name only the application payload as the general-reader position. Name the B2 ruling and the wire-acceptance change in the commit. Acceptance: `parse_record` contains no `ciborium::de` call; the two former pins assert rejection with the typed head error; the gossip and codec snapshots are byte-identical (the encoder already emits shortest form); an allocator meter over `records()` on a run of N records shows no per-record allocation beyond the `Version`, `Arc`, and `Bytes` it constructs.

See also: the remote-codec partition's open question "Reopen B2?" (Open questions below); tests/decode_alloc.rs's supply-read meter (tests-resource-link-window-7 notes what that meter does and does not exercise), which is the natural home for the acceptance count.

## Remote capture and codec tests

### remote-capture-atlas-28: The bounded corpus test runs about 2.2 million codec round-trips at opt-level 0 with a fresh allocation per case, unmeasured
- Where: src/tree/mirror/streaming/remote/codec/tests.rs:487-497 (related: src/tree/mirror/streaming/remote/codec/tests.rs:34, src/tree/mirror/streaming/remote/codec/tests.rs:375, src/tree/mirror/streaming/remote/codec/tests.rs:389, Cargo.toml:174-187, .config/nextest.toml:25-26)
- Class / severity / confidence: performance / low / medium
- Provenance: assessed (read; `EXHAUSTIVE_FRAME_CASES = 1_118_600` is asserted at 389 and I recomputed it: per stream, 2 flows x (Match + Supply + C(256,0) + C(256,1) + C(256,2) queries) + 2 ends = 65,800; x17 = 1,118,600; `check_both` runs each case for both speakers; Cargo.toml's dev profile raises opt-level only for `before` and `suanpan`; nextest's slow period is 60 s. No wall time is recorded anywhere I searched and I did not run it)
- Seen by: perfapi; refutation: uncertain (facts verified; cost unmeasured); history: no rationale found
- Owner-gated: no
- Sign: fixed for the two named items (buffer reuse across the loop; `sha3` at opt-level 2 in the dev profile, following the manifest's precedent); whether the enumeration itself stays is the owner's call. Measure the wall time first, as the resolution says.

Per case the loop allocates a fresh `Vec::new()`, encodes, decodes through `decode_exact` (which allocates the frame), and feeds SHA3, with `children.to_vec()` per query case; `rumors` and `sha3` compile at opt-level 0 in the unit-test binary. The denominator (2.2 million cases per run) is large enough that this single test plausibly sits on the suite's critical path, but no committed number says so. Instruments before cures: measure, then land the fixed-sign items.

Evidence:

    490	                let mut encoded = Vec::new();
    491	                encode(speaker, &frame, &mut encoded).unwrap();
    492	                accepted[direction] += 1;
    493	                assert_eq!(
    494	                    decode_exact(speaker, RunBudget::default(), &encoded).unwrap(),
    495	                    frame
    496	                );
    497	                bucket.accept(&encoded);

Resolution: Measure first (`just test bounded_corpus_manifest_snapshot`, wall time under the dev profile, recorded). If material, land the two fixed-sign items: reuse one encode buffer across the loop (`encoded.clear()`), and `[profile.dev.package.sha3] opt-level = 2` following the manifest's own precedent; re-measure. Whether the 1.1-million-case enumeration earns its remaining cost (given `frame_round_trips` samples the family and the atlas pins the 340 placements) is the owner's call and would move the manifest snapshot. Acceptance: a before/after wall time is recorded; the manifest snapshot is byte-identical after the fixed-sign changes.

Synthesis note: the suite-economics sweep's run log ranks the suite's slowest tests and names the capacity test at 19.86 s as the longest (suite-economics-7); this test does not appear in that ranking, so it is unlikely to sit on the critical path today. The two fixed-sign items still stand, and the `sha3` opt-level change also serves suite-economics-2.

## Remote adapter and streams

### remote-adapter-streams-6: The decode channel's capacity is justified as "load-bearing for liveness" in two places with no mechanism named, and none is derivable; the pull-based alternative is a measure-first proposal
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:278-291 (related: src/tree/mirror/streaming/window.rs:129-132, src/tree/mirror/streaming/window.rs:390-399, src/tree/mirror/streaming/window.rs:178-192, src/tree/mirror/streaming/remote/adapter/decode.rs:83-131, src/tree/mirror/streaming/remote/adapter/decode.rs:380, src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:130-183, src/tree/mirror/streaming/backend/local.rs:201-221, src/tree/mirror/streaming/convert.rs:79-92)
- Class / severity / confidence: performance / medium / medium
- Provenance: assessed (read both `assemble` implementations and the joined drive; verified the history mechanically: `git show 8aeed2dd3 -- decode.rs` shows the sentence "it exists to amortize the reader/assembler waker round trip" replaced by "The capacity is load-bearing for liveness", with no liveness argument in the commit)
- Seen by: perfapi ([33]); refutation: reframed (no stall derivable from the joined-drive shape; settling it needs a sub-FAN construction, a code change; the pull-based reader is a separate backend-dependent trade); history: no-rationale-found for the liveness sentence (8aeed2dd's message discusses custody and pricing, never liveness); the pricing half is grounded (d6537a7b pinned FAN + 1 as a never-exceeds premise)
- Owner-gated: yes: the window's supply-decode pre-charge term (window.rs:397-399) would move, and the reader/assembler shape is a documented design
- Sign: workload- and backend-dependent for the pull-based reader (one fan of read-ahead can overlap wire parsing with a slow persistent `assemble`; under `Local` it buys nothing), so measure first on `benches/gossip_fixed.rs` with a supply-heavy and a supply-free fixture plus a per-reply allocation meter. Settling the liveness claim is a construction, not a trade, and comes first.

The comment at 280-282 and window.rs:395-396 say the FAN-slot capacity "is load-bearing for liveness" and that "no configuration may shrink it", and window.rs:129-131 calls it a "hard capacity floor". No mechanism is named. Both halves are driven by one task (`futures::future::join` at 291; the `poll_fn` at 103-116), `ReceiverStream` drains one item per poll, `Local::assemble` (local.rs:205-217) and the `fold_parents` chain (convert.rs:83-91) each pull one leaf at a time, so a full channel yields to the drain and progress does not depend on the capacity for any capacity of at least one; capacity governs wakeups (the amortization the previous comment stated) and read-ahead, not progress. Meanwhile the channel's residency is pre-charged into every session budget as the flat supply-decode envelope (`STREAM_COUNT * (FAN + 1) * ...`, window.rs:191-192, 397-399), every decoded reply, supply-free disputes included, allocates a FAN-slot channel plus two boxed streams and a `join`, and every supplied record pays a bounded-channel send/recv pair. A stated invariant whose only defense is itself is the circular-justification tell; a number that governs a budget term is a hypothesis until its premise is named. Steelman of the channel: one fan of read-ahead lets a slow persistent `assemble` overlap wire parsing; under `Local` it buys nothing. Denominator: per decoded reply (the fixed allocations), per supplied record (the channel hop), and per session (the pre-charged budget term).

Evidence:

       278	    // One fan of buffered leaves, amortizing the reader/assembler waker
       279	    // round trip over runs of consecutive leaves instead of paying it per
       280	    // leaf. The capacity is load-bearing for liveness: the channel must
       281	    // admit one full fan of records while the assembler holds a parent
       282	    // group open, so no configuration may shrink it. Its residency is
        ...
       288	    let (tx, rx) = mpsc::channel::<Result<(Prefix<Z>, B::Node<Z>), B::Error>>(FAN);
       289	    let read = read_reply::<B, _, _, _>(version_bytes, &ledger, scope, frames, question, tx, codec);
       290	    let assemble = assemble_supplies::<B>(backend, children_height, rx);
       291	    let (read, assembled) = futures::future::join(read, assemble).await;

    window.rs:
       394	        // so `node_bytes(0, ·)` is its whole resident price). Width
       395	        // cannot shrink this term — the fan capacity is load-bearing for
       396	        // liveness — so it comes off the budget before the solve.

Resolution: 1. Settle the liveness claim: construct `decode` with capacity 1 (a test-only constant or a parameter) and drive an eager source of `4 * FAN` records under `Local`; if it completes, rewrite decode.rs:280-282 and window.rs:394-396 to the true rationale (waker amortization and one fan of read-ahead, priced as a never-exceeds ceiling pinned by `fan_occupancy.rs`), and if it stalls, name the stall in the comment and commit the construction as a test. 2. Then, as an owner decision, prototype the pull-based reader (`read_reply` as a stream of `(Prefix<Z>, B::Node<Z>)` the assembler polls directly, skeleton and questions parked in a slot read after assembly ends; `early_supplies` becomes the same stream fed to `ops::assemble`), measure at the parent commit and after on `benches/gossip_fixed.rs` for a supply-heavy and a supply-free workload plus a per-reply allocation meter, and if adopted re-derive the window's supply-decode term and replace the fan-occupancy pins with a meter for whatever residency the new shape has. Acceptance: either a committed test constructing the stall a sub-FAN capacity causes and a comment naming it, or the two comments state the amortization/read-ahead rationale and drop "liveness"; the pull-based reader lands only with before/after numbers and a re-derived pricing term.
Construction: For the liveness question only (no defect asserted): `mpsc::channel(1)` in `decode` with `stream::iter(frames(&leaves(4 * FAN)))` under `Local`, on the `fan_occupancy.rs` runtime; completion refutes the liveness claim for the joined shape.

See also: remote-proxy-27 (the pump's call site, where the per-reply cost is paid; its meter is step zero for this entry's measurement); deps-2 and the module-graph sweep's open question 4 (this channel is a raw `tokio::sync::mpsc` outside `streaming::channel`'s instrumentation, so the schedule machinery cannot perturb it); the remote-adapter-tests partition's open question on whether the `FAN + 1` equality pin rests on tokio's cooperative budget; conformance-28 and tests-resource-link-window-20 (the census checks that would detect a mis-priced pre-charge both resolve to the floor window today and pass vacuously, so the pre-charge term this entry would move has no live meter behind it).

### remote-adapter-streams-16: `Scope`'s positional radix list is a heap `Vec` per question; a 256-bit set makes `Scope` `Copy` and allocation-free
- Where: src/tree/mirror/streaming/remote/adapter/scope.rs:12-26 (related: src/tree/mirror/streaming/remote/adapter/scope.rs:40-44, src/tree/mirror/streaming/remote/adapter/scope.rs:55-67, src/tree/mirror/streaming/remote/adapter/encode.rs:103, src/tree/mirror/streaming/remote/adapter/decode.rs:223, src/tree/mirror/streaming/remote/codec/error.rs:55-61)
- Class / severity / confidence: performance / low / high
- Provenance: assessed (read; the precondition that wire listings are strictly ascending by radix is enforced by the codec's `QueryOrderError`, codec/error.rs:55-61, so a bitset walked in ascending order reproduces `Scope::next`'s positional order)
- Seen by: perfapi ([34]); refutation: confirmed; history: no-rationale-found (`children: Vec<u8>` dates to 00b29d32; d8bef16b rewrote `Scope`'s type parameters and doc and kept the representation without comment)
- Owner-gated: no (private type)
- Sign: fixed (one allocation per question on each side deleted; the 34-byte inline form is smaller than the `Vec` header plus its block). Confirm with a per-question allocation count.

Every question created on either side (`Scope::new` from the encoder's `derive` and the decoder's question closures, plus `Scope::opening` and `Scope::leaf`) collects the listing's radices into a `Vec<u8>`: one heap allocation per disputed scope per peer, and `Scope` is `Clone` rather than `Copy`. The radices are strictly ascending bytes consumed positionally, which is a 256-bit set walked by a cursor. Strict deletion of redundant work with a fixed sign: a `[u64; 4]` plus a `u16` cursor holds the same information in 34 bytes inline, `next()` becomes a `trailing_zeros` walk over at most four words, `Scope` gains `Copy`, and the two proxy scope queues move a fixed-size value. Denominator: one allocation per question on each side; small beside the listing `Vec<(u8, Hash)>` the codec allocates for the same question, hence low.

Evidence:

        12	pub struct Scope {
        13	    parent: ErasedPrefix,
        14	    children: Vec<u8>,
        15	    next: usize,
        16	}
        ...
        20	    pub fn new(parent: ErasedPrefix, listing: &[(u8, Hash)]) -> Self {
        21	        Self {
        22	            parent,
        23	            children: listing.iter().map(|(radix, _)| *radix).collect(),
        24	            next: 0,
        25	        }
        26	    }

Resolution: Replace `children: Vec<u8>` with a `RadixSet([u64; 4])` newtype (`insert`, `next_above(cursor)`), keep the cursor as `u16`, derive `Copy`, keep `is_request` as `set.is_empty()`. Acceptance: `Scope` is `Copy`; `adapter/tests/properties.rs` and `malformed.rs` pass unchanged; a per-question allocation meter (or a `stats_alloc` inspection of `decode_reply` on a fan-wide `Query` reply) shows one fewer allocation per question.

See also: the streaming-backend-window partition's open question on which two containers `SCOPE_FIXED_BYTES` prices (a `Copy` `Scope` of fixed size changes the `size_of` the pricing would read); remote-adapter-streams-5 (the four copies of the scope-derivation rule, whose consolidation is where a representation change lands once).

### remote-adapter-tests-21: two of the six height sweeps are strict special cases of two others
- Where: src/tree/mirror/streaming/remote/adapter/tests/properties.rs:1033-1062 (related: src/tree/mirror/streaming/remote/adapter/tests/properties.rs:1064-1119, 154-173, 419-443)
- Class / severity / confidence: performance / nit / medium
- Provenance: assessed (read; the runtime share is not measured, and no timing run was made)
- Seen by: api-economics; refutation: confirmed (the `btree_set(.., 0..=8)` generator produces the empty case at a useful rate, so the degenerate sweeps add mostly naming value); history: the specific sweeps came first (00b29d32) and the general ones were added later (07504b2e) without retiring them; 24b187a8 added the sentinel to `matches` and `mixed` and not to `positioned`
- Owner-gated: no
- Sign: fixed for the deletion (two of six sweeps, each 256 cases × 32 heights, removed with no sampled space lost); the naming value is the counterweight, and the resolution offers a reduced case count as the middle path. Measure the file's runtime once before and after on a quiet machine.

`supplied_leaf_is_lossless_at_every_height` is `mixed_reactions_are_lossless_at_every_height` with `radixes` empty; `matches_and_boundaries_are_lossless_at_every_height` is `positional_reactions_are_lossless_at_every_height` with `queries == 0`, except that only the former checks the trailing sentinel. Each sweep is 256 cases × 32 heights of encode plus decode with a fresh channel and boxed async stream. The degenerate shapes do have naming value as seeds, so this is a judgment call. Denominator: per test run, 2 × 256 × 32 encode-decode pairs.

Evidence:

    1033	    /// For every reply height, assembling one supplied wire leaf and then
    1034	    /// exploding the resulting backend node reproduces the exact frame.
    1035	    #[test]
    1036	    fn supplied_leaf_is_lossless_at_every_height(
    1037	        value in any::<u64>(),
    1038	        ticks in any::<u8>(),
    1039	    ) {

Resolution: fold the sentinel check into `positioned_reactions` and drop the two special-case sweeps, or keep them at a documented reduced case count since their generators vary only the leaf. Acceptance: four sweeps at full case count, or six with the two degenerate ones at a documented reduced count; the file's runtime measured once before and after on a quiet machine.

See also: remote-adapter-tests-20 (each law stated twice, for `Z` and `S<H>`, with textually identical bodies; the same file's structural duplication).

## Remote proxy

### remote-proxy-27: Every decoded reply pays a FAN-slot channel and two boxed streams whether or not it carries a supply (adapter call, cross-partition)
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:227-235 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:290-298, src/tree/mirror/streaming/remote/proxy/work/pump.rs:384-392, src/tree/mirror/streaming/remote/adapter/decode.rs:277-291, src/tree/mirror/streaming/erased.rs:319-334, tests/decode_alloc.rs:1-12)
- Class / severity / confidence: performance / low / medium
- Provenance: assessed (read decode.rs:288-291: every `decode` builds `mpsc::channel(FAN)` and joins reader and assembler; read tests/decode_alloc.rs's module doc: it meters declared-length payload reads and supply reads, not the per-reply fixed allocations. Not measured.)
- Seen by: perfapi; refutation: confirmed (with the caveat that lazy channel creation changes the join's control flow, so it is measure-first); history: no-rationale-found (4d55d4844 prices the FAN capacity, not its construction per supply-free reply)
- Owner-gated: no
- Sign: measure first (lazy channel creation changes the join's control flow, so the deletion is not free of structure change); the meter is the fixed first step.

In a dispute descent most replies are `Match`/`Query` only; for them the channel, the assembler, and the join are overhead. Denominator: per decoded reply. The crate already holds the encoder to zero allocations per body-free frame (`tests/encode_alloc.rs`); the decoder's per-reply fixed allocations are the ingress twin of that number and are unmetered. Instruments before cures: this is a candidate to meter first. The resolution lives in the adapter partition; the call site is here.

Evidence:

    227	                let Decoded { reply, questions } = decode_reply::<B, _>(
    228	                    backend.clone(),
    229	                    version_bytes,
    230	                    ledger.clone(),
    231	                    scope,
    232	                    &mut incoming,
    233	                    codec,
    234	                )
    235	                .await?;

    decode.rs:288	    let (tx, rx) = mpsc::channel::<Result<(Prefix<Z>, B::Node<Z>), B::Error>>(FAN);
    decode.rs:290	    let assemble = assemble_supplies::<B>(backend, children_height, rx);
    decode.rs:291	    let (read, assembled) = futures::future::join(read, assemble).await;

Resolution: first, add a `stats_alloc` region around a `Match`-only reply decode (the `tests/decode_alloc.rs` pattern) and commit the current count as the baseline. Then, in the adapter: create the leaf channel and assembler lazily on the first `Supply` record, or let the pump own one channel per stage and hand it in so the allocation amortizes. Acceptance: a committed allocation meter for a supply-free reply decode; after the change its count excludes the channel and assembler allocations.

Construction: wrap `decode_reply` over a frame stream of one `Frame::Reaction(Match, Flow::End)` in a `stats_alloc::Region` and read `allocations`; expect a nonzero count today attributable to the channel, the `ReceiverStream` box, and the assembler box.

See also: remote-adapter-streams-6 (the same channel from the adapter's side, with the liveness question that must be settled before its shape changes); remote-proxy-24 (the per-reply `backend.clone()` and `ledger.clone()` visible at lines 228 and 230, folded there as a related cost); remote-adapter-streams-4 (the three loose decode premises these arguments carry).

### remote-proxy-tests-12: `context_registration_is_causal` reruns `wire_reconciliation_matches_local`'s exact session to add one assertion
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:407-427 (related: tests.rs:388-405, 609-623)
- Class / severity / confidence: performance / nit / high
- Provenance: verified (read: identical generators and `instrumented_reconcile` call; the second adds only `trace.assert_registration_causality()`)
- Seen by: api-economics; refutation: confirmed, downgraded (separate properties give separate failure attribution and seeds); history: no rationale (675e2f53 argues for the assertion, not for a separate case budget)
- Owner-gated: no
- Sign: fixed for the deletion (256 instrumented sessions per run removed); the counterweight is failure attribution by name and a separate seed file, which the merged assertion messages can carry.

Both properties draw `(a, b) in arb_divergent_pair()` and `schedule in vec(0_u8..=2, 0..128)` and call `instrumented_reconcile`; at the default 256 cases the second is 256 redundant instrumented sessions whose only independent value is its (good) doc paragraph, which can ride the merged test. The counterweight is real but small: a separate property attributes a failure to registration causality by name and persists its own seed. Denominator: per test run, 256 instrumented wire sessions.

Evidence:

       417	    #[test]
       418	    fn context_registration_is_causal(
       419	        (a, b) in arb_divergent_pair(),
       420	        schedule in vec(0_u8..=2, 0..128),
       421	    ) {
       422	        let (result, _channels, trace) = instrumented_reconcile(a, b, schedule);
        ...
       426	        trace.assert_registration_causality();

Resolution: Call `trace.assert_registration_causality()` inside `wire_reconciliation_matches_local` after `trace.assert_valid()`, move the receive-side ordering paragraph into that doc, and delete the second property; or keep both and accept the cost knowingly. Acceptance: one property over `instrumented_reconcile` asserts oracle equality, channel bounds, send-side ordering, and registration causality, and its doc names all four.

See also: suite-economics-5 (the same pattern in the integration suites, with the sampling trade stated there); the remote-proxy-tests partition's open question on the seed file whose entries may predate the `schedule` draw.

### remote-proxy-tests-18: The transport-failure property's recovery run reconciles immutable inputs over a fresh link and pins nothing new
- Where: src/tree/mirror/streaming/remote/proxy/tests/failures.rs:255-266 (related: failures.rs:153, 158-166, 191-205; tests/harness.rs:449-468)
- Class / severity / confidence: performance / low / high
- Provenance: verified (read: `harness::reconcile` calls `memory_with_capacity` afresh on every call at harness.rs:456; `left`/`right` are owned roots whose earlier uses took clones; the doc at 158-166 names no recovery claim)
- Seen by: api-economics; refutation: confirmed, downgraded (32 cases, so 32 redundant sessions); history: no rationale (the block was born with the property in 77674c9c, already pinning nothing beyond the clean run; 6410212a's rework left it untouched)
- Owner-gated: no
- Sign: fixed (32 sessions per run whose result is already asserted twice deleted; nothing sampled is lost).

After a fault fires, the property runs a third full session with both plans default. Nothing from the faulted session can reach it: it is a clean reconcile of the same inputs, already asserted by the `clean` run and by `successful_io_adversity_matches_materialized`. The claim a recovery run would pin (a failed session leaves the replica and link usable) lives at the `Peer`/`Link` tier, where poison and epoch state persist; at this tier the harness makes it vacuously true. Principle 6: the worst passing artifact here is any implementation that passes the clean run. Denominator: per test run, 32 wire sessions.

Evidence:

       255	            let recovered = run_to_quiescence(harness::reconcile(
       256	                left,
       257	                right,
       258	                17,
       259	                IoPlan::default(),
       260	                IoPlan::default(),
       261	            ))
       262	            .map_err(|stopped| TestCaseError::fail(format!(
       263	                "clean recovery became quiescent: {stopped:?}",
       264	            )))?;
       265	            prop_assert_eq!(recovered.left.as_ref().ok(), Some(&expected.0));
       266	            prop_assert_eq!(recovered.right.as_ref().ok(), Some(&expected.1));

    harness.rs:
       456	    let (left_link, right_link) = memory_with_capacity(capacity.max(1));

Resolution: Delete the `recovered` block. If the intent was to reuse the same link after the fault, that is a different test: hand the faulted link with its `SessionState` to a second session and assert the poison/epoch behavior the link contract promises, beside the link-tier tests. Acceptance: the property runs at most three sessions per case (oracle, clean, faulted) and its doc names every claim it asserts.

See also: the remote-proxy-tests partition's open question on the literal `17` at line 258 (it predates `STREAM_COUNT` by a day and should become the harness's `TRANSPORT_CAPACITY`); remote-proxy-tests-25 (the captured-`Done` mechanism a link-tier recovery test would need).

## Integration tests

### tests-bookmark-13: `heal` pays a confirming full-mesh round that `sim::quiesce` no longer does, and its doc says it mirrors it
- Where: tests/bookmark_causality.rs:758-776 (related: tests/bookmark_causality.rs:734; tests/common/sim.rs:891-898; tests/bookmark_transmit_window.rs:447-465)
- Class / severity / confidence: performance / low / high
- Provenance: verified (read `sim::quiesce` at HEAD and at `0d48b153^` via `git show`; 0d48b153's message names the criterion change and its diff touched bookmark_causality.rs only for doc re-wraps)
- Seen by: api-economics; refutation: confirmed; history: deliberate but expired (0d48b153 changed sim only)
- Owner-gated: no
- Sign: fixed (with equal fingerprints across live peers every pending emission is already propagated, so the confirming mesh exchanges nothing; the trailing `secure` sweep still promotes it).

`World::heal` fingerprints, runs the full mesh, then compares, so a converged fleet pays one mesh (six sessions at n = 4) and a changed fleet pays two. `sim::quiesce` tests all-equal fingerprints before any round and returns without one. Across 2 x 256 cases this is a fixed per-case cost the claim does not need, and "Mirrors `sim::quiesce`" is inaccurate in exactly this respect. Denominator: per proptest case, one full mesh of sessions (six at n = 4) beyond what the claim needs.

Evidence:

       734	    /// reachable. Mirrors `sim::quiesce`, plus the network-convergence step.
        ...
       760	        let rounds = MAX_HEAL_ROUNDS_PER_PEER * self.n();
       761	        for _ in 0..rounds {
       762	            let before = self.fingerprints();
       763	            for a in 0..self.n() {
       764	                for b in (a + 1)..self.n() {
       765	                    self.clean_gossip(a, b);
       766	                }
       767	            }
       768	            if self.fingerprints() == before {

    tests/common/sim.rs:
       892	        // Identical fingerprints are the fixed point itself: peers with
       893	        // equal content and version exchange nothing, so no confirming
       894	        // mesh round is owed.
       895	        let first: ([u8; rumors::MERKLE_HASH_LEN], Version) = fingerprint(&peers[0]);
       896	        if peers[1..].iter().all(|p| fingerprint(p) == first) {
       897	            return;

Resolution: at the top of each round, if all live fingerprints are equal, run the final `secure` sweep and return; otherwise run the mesh. Better, once `sim::quiesce` is bookmark-generic (tests-bookmark-3), call it and keep only the network-collapse step here. Acceptance: a plan whose fleet is converged before heal performs zero heal sessions (a debug counter on `clean_gossip` shows it); both proptests and the reconstructed tests still pass.

See also: tests-bookmark-3 (the suite-local drivers that reimplement `common::wire` for bookmarked peers; generalizing them is the resolution's second branch); tests-bookmark-12 (the same harness swallows session errors, so a heal round that fails is invisible today, which matters once heal rounds are counted).

### tests-lifecycle-9: multi_peer runs four executor passes per case for properties one pass proves, two of them implied by a third
- Where: tests/multi_peer.rs:40-100 (related: tests/multi_peer.rs:106-124, tests/multi_peer.rs:168-200, tests/membership.rs:60-70, tests/common/peer.rs:78-87, tests/redaction.rs:3-5)
- Class / severity / confidence: performance / low / high
- Provenance: verified (all four properties draw `(schedule_u64(), arb_window_assignment())` and call `execute_and_quiesce` at lines 44, 62, 85, 110; `diff` of membership.rs:53-58 against multi_peer.rs:86-91 is identical modulo the binding name)
- Seen by: api-economics; refutation: confirmed with a caveat (the multiset check is implied by the canonical-map check only while `resolved_versions` is injective, which the harness's `insert_one` assert at peer.rs:85 enforces) and downgraded to low; history: deliberate-and-holds (80a3155f41 chose one named test per invariant "for auditability"; the rationale lives only in that commit message and never weighed the executor cost or the implication)
- Owner-gated: yes: reopens a recorded design choice (one named test per invariant)
- Sign: fixed for the deletion of the two implied properties (512 executor-plus-quiesce runs per pass removed with no sampled space lost, given the injectivity premise the harness enforces). Note the trade suite-economics-5 states for the fuller merge: fusing all four properties onto one execution reduces the distinct schedules the suite draws per pass from about 1024 to 256.

If every peer's `readout` equals the canonical map (`versions_stable_across_peers`), then all readouts are pairwise equal (`all_peers_converge_after_quiesce`) and the value multiset of the canonical map is exactly `expected_live()` (`readout_matches_oracle_after_quiesce`), given that `resolved_versions` is injective, which `Peer::insert_one`'s exactly-one-new-observation assert enforces. The two implied tests therefore add no sampled space at 512 executor-plus-quiesce runs (up to 8 peers, 50 events, 256 cases each). The recorded rationale (each invariant named after what it tests) is a legibility argument and still holds; it is not stated in the module, and it never considered fusing assertions over one result with distinct messages. Denominator: per suite pass, 512 executor-plus-quiesce runs.

Evidence:

        40	    fn all_peers_converge_after_quiesce(
        41	        schedule in schedule_u64(),
        42	        windows in arb_window_assignment(),
        43	    ) {
        44	        let result = execute_and_quiesce(&schedule, &windows);

        58	    fn readout_matches_oracle_after_quiesce(
        59	        schedule in schedule_u64(),
        60	        windows in arb_window_assignment(),
        61	    ) {
        62	        let result = execute_and_quiesce(&schedule, &windows);

        81	    fn versions_stable_across_peers(
        82	        schedule in schedule_u64(),
        83	        windows in arb_window_assignment(),
        84	    ) {
        85	        let result = execute_and_quiesce(&schedule, &windows);

Resolution: Owner's choice between (a) fusing: delete the two implied tests, fold the observation-log check into the canonical-map test over the same `result` with distinct assertion messages, switch the `_at_floor` and `_string` legs to the canonical-map form (stronger at equal cost), and note beside the fused test that the harness's `insert_one` assert carries the injectivity premise; or (b) keeping the split and stating the one-test-per-invariant policy in the module doc so the cost is recorded as a choice. Either way, redaction.rs:3-5 and multi_peer.rs:182 name `readout_matches_oracle_after_quiesce` and must follow the rename. Apply the same reasoning to membership.rs:62-69, where the multiset assert is implied by the canonical one that follows it. Acceptance: for (a), `execute_and_quiesce` runs at most once per generated `(schedule, windows)` case in the u64 swept leg and every property the deleted tests held is asserted in a survivor; for (b), the module doc states the policy.

See also: suite-economics-5 (the sweep's record of the same site with run-log timings of 3.97 to 4.17 s per property, and the census and corners halves); tests-lifecycle-29 (the `sanity.rs` property on the same generator); the tests-lifecycle partition's open question on whether the schedule-executor binaries' default 256 cases were chosen or inherited.

### tests-lifecycle-29: sanity.rs's panic-freedom property is strictly subsumed by multi_peer, and its doc's premise is false
- Where: tests/sanity.rs:19-29 (related: tests/sanity.rs:15-16, tests/multi_peer.rs:21-22, tests/multi_peer.rs:40-44, tests/sanity.rs:31-69, tests/sanity.rs:72-93, .config/nextest.toml)
- Class / severity / confidence: performance / low / high
- Provenance: verified (sanity.rs:15-16 and multi_peer.rs:21-22 define identical `N_PEERS` (2..=8) and `MAX_EVENTS` (50); sanity.rs:25-28 draws `arb_schedule(any::<u64>(), ..)` with `arb_window_assignment()` and calls `execute_and_quiesce`, exactly what every multi_peer property does before asserting more; tests run as independent processes under nextest and as independent functions under libtest)
- Seen by: api-economics; refutation: confirmed; history: no-rationale-found (the premise was never literally true even in the original single-binary suite; the split into per-file binaries and nextest made it less so)
- Owner-gated: no
- Sign: fixed (256 executor passes per suite pass deleted; every failure it could report is already a failure of a stronger committed test).

`arbitrary_schedules_dont_panic` adds 256 executor passes and no sampled space: any panic it would catch already fails every multi_peer property on the same generator. Its doc says "if this fails, the others cannot run", but the others run and fail with the same panic. A test whose every failure is already a failure of a stronger committed test is pure cost, and an inaccurate testdoc is a bug in the test. With it gone, `forked_gossip_matches_direct_gossip` belongs with the pairwise laws and `quiesce_handles_zero_or_one_peer` is a harness self-test, so the binary dissolves. Denominator: per suite pass, 256 executor-plus-quiesce runs (3.5 s in the sweep's run log).

Evidence:

        20	    /// Arbitrary schedules complete without panicking and produce a
        21	    /// finite converged state. The safety net for every other
        22	    /// invariant in the suite — if this fails, the others cannot run.
        23	    #[test]
        24	    fn arbitrary_schedules_dont_panic(
        25	        schedule in arb_schedule(any::<u64>(), N_PEERS, MAX_EVENTS),
        26	        windows in arb_window_assignment(),
        27	    ) {
        28	        let _ = execute_and_quiesce(&schedule, &windows);
        29	    }

Resolution: Delete the test. Move `forked_gossip_matches_direct_gossip` into pairwise.rs and `quiesce_handles_zero_or_one_peer` beside the harness's other self-checks (shadow_validity.rs, or wherever the harness self-tests are collected), then delete sanity.rs. Acceptance: tests/sanity.rs no longer exists or contains no test another binary's property implies; both surviving tests still run.

See also: suite-economics-5 (the same test from the sweep's side); suite-economics-8 (`sanity.rs` is one of the sixty link units, so dissolving the binary also removes one compile of `tests/common`).

## Benches and examples

### benches-envelope-18: grid::build harvests the shared prefix's versions for every cell though only redaction cells consume them
- Where: benches/support/grid.rs:156-172 (related: benches/support/grid.rs:42-44, benches/support/grid.rs:54, benches/support/grid.rs:120-137)
- Class / severity / confidence: performance / low / high
- Provenance: verified (`shared` is collected at 159 unconditionally and read only at 170-171 under `if redacted > 0` at 165; `REDACTED[0] == 0` at 54; `cells()` admits `redacted = 0` for every `(common, differing)`; the module doc at 42-44 names fixture building as the wall-time bottleneck)
- Seen by: perfapi; refutation: confirmed; history: no-rationale-found (the collect predates 9c73d7b463, which turned each element from a `Copy` key into a `Version` clone plus `Arc` traffic without moving it under the branch)
- Owner-gated: no
- Sign: fixed (dead work in setup deleted; measured bodies unchanged).

For every zero-redaction cell (the whole `REDACTED[0]` column plus every cell the `common < 2 * redacted` filter empties), the harvest is dead work: up to 100,000 `Version` clones, each paying the `Arc<dyn Any>` clone, downcast, and drop of `Snapshot::iter`, per fixture build, per Criterion iteration, in the stage the module calls its bottleneck. Untimed setup still costs wall time; deleting redundant work has a fixed sign. Denominator: per fixture build per Criterion iteration on a zero-redaction cell, up to 100,000 `Version` clones.

Evidence:

   156	    // The shared prefix's versions, for carving the redaction blocks; order
   157	    // is immaterial (the blocks only need to be disjoint and deterministic,
   158	    // and the snapshot iterates in a stable order).
   159	    let shared: Vec<Version> = left.snapshot().iter().map(|(v, _)| v.clone()).collect();
   160	
   161	    let right = wire::bootstrap_fork(&left);
   162	    send_units(&left, differing);
   163	    send_units(&right, differing);
   164	
   165	    if redacted > 0 {

Resolution: `let shared = (redacted > 0).then(|| left.snapshot().iter().map(|(v, _)| v.clone()).collect::<Vec<Version>>());` at the same position (it must precede the post-fork sends at 162-163), and read it inside the branch. Acceptance: non-redaction cells perform no snapshot iteration in setup; cell ids and measured bodies unchanged.

See also: benches-envelope-19 (harvesting versions from a `Snapshot` pays an `Arc` clone, downcast, and drop per element because no versions-only enumeration exists; that is why the dead harvest is expensive, and the redaction cells that do need it would benefit from the accessor); benches-envelope-11 and benches-envelope-5 (the same harvest duplicated in `in_memory.rs` and `seeded_with_versions`).

### swarm-example-5: `Payload = Vec<u8>` encodes as a CBOR integer array, nearly doubling wire and decode cost per payload byte
- Where: examples/swarm.rs:185-188 (related: examples/swarm.rs:853-857, examples/swarm/tests.rs:13, Cargo.toml:126, tests/dispute_wire.rs:33-36, src/message.rs:25-27)
- Class / severity / confidence: performance / medium / high
- Provenance: verified (vendored ciborium-0.2.2 src/ser/mod.rs:165-168 emits `Header::Bytes` for `serialize_bytes` and :226-227 emits `Header::Array` for `serialize_seq`; serde's `Vec<T>` serializes through `serialize_seq`; Cargo.toml:126 enables `bytes` with `serde`; the 1.906 bytes-per-uniform-byte figure is arithmetic from RFC 8949 major type 0, not measured by running the example)
- Seen by: perfapi; refutation: confirmed (severity lowered to low: the bandwidth row reports true wire bytes); history: deliberate but expired (`Vec<u8>` was chosen under Borsh, which wrote it as a blob; f2b74a97 switched the codec and acb556fc rewrote the doc without changing the type)
- Owner-gated: no
- Sign: fixed (a byte string is strictly shorter than an integer array of the same bytes and decodes in one step rather than per element; the cached `Message` bytes shrink likewise).

serde serializes `Vec<u8>` element by element, so ciborium emits a CBOR array of unsigned integers: one byte for values 0..=23 and two for 24..=255, about 1.9 wire bytes per uniformly random payload byte, plus a per-element decode on every receive and on every send's self-decode admission check, and the cached `Message` bytes inflated likewise. The doc's "small per-element constant" is literally true (about 0.9 bytes per element) and hides a near-doubling relative to the payload it prices. `bytes::Bytes`, already a regular dependency with the `serde` feature and the type the crate's own wire tests use for exactly this reason, encodes as a byte string. The finalizer held this at medium rather than the refutation's low because the example is the crate's showcase and its documented measurement tool, and the crate docs' payload-type section says nothing about byte payloads, so this is the one place a user learns the idiom. Denominator: per payload byte, on the wire, in the resident cache, and in decode work at both ends.

Evidence:

    185	/// Message payload type: opaque, randomized bytes. CBOR encodes `Vec<u8>`
    186	/// as an integer array, so the wire cost tracks the message size (within
    187	/// a small per-element constant).
    188	type Payload = Vec<u8>;

    ciborium-0.2.2 src/ser/mod.rs
    165	    fn serialize_bytes(self, v: &[u8]) -> Result<(), Self::Error> {
    166	        self.0.push(Header::Bytes(v.len().into()))?;
    226	    fn serialize_seq(self, length: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
    227	        self.0.push(Header::Array(length))?;

    tests/dispute_wire.rs
    33	//! Payload corpora are [`bytes::Bytes`], which serde carries as a CBOR
    34	//! byte string: a fixed-length payload has one deterministic encoded

Resolution: `type Payload = bytes::Bytes;` with `random_message` returning `Bytes::from(buf)`; restate the doc: a byte string costs its length plus a one-to-three-byte head. `TEST_MESSAGE_SIZE` and the controller test are unaffected. Acceptance: the `Payload` doc no longer says "integer array" or "per-element"; with `--headless-secs 5 --message-size 256`, wire bytes per learned message fall from roughly 490 plus framing to roughly 259 plus framing (observable once swarm-example-18 surfaces `messages_gained`, or from a one-off `SessionStats::bytes_sent` probe).

See also: swarm-example-18 (the example discards `Gossiped` and never reads `SessionStats`, which is why the acceptance figure has no readout today); swarm-example-16 (the "roundtrips/sync" row that reports a divergence-independent constant, the example's other dead instrument); the api-core partition's dropped note that the crate docs' payload-type section is silent on byte payloads, the documentation half of this idiom.

## The cost of verification (suite-economics sweep)

This section holds the performance-class findings of the suite-economics sweep, which measured what the test suite itself costs. The sweep's run of record is one `cargo nextest run -p rumors --all-features` at 9e5784fb: 836 tests across 60 binaries, 22.07 s wall on 16 workers, 2 skipped (both `#[ignore]` by design), no slow, retry, or flaky markers; the per-test rows sum to 295.142 s of test time. The machine was not quiet: load averages were 9.65/7.33/6.99 at run start and 19.09/9.70/7.85 at run end, and the build waited on the shared build-directory lock, so every timing below is indicative rather than a baseline. The finalization pass spent one of its two permitted invocations confirming that `future_size` compiles to zero tests (load 4.51 before and after); it ran nothing else. Per-test times quoted in the entries come from that one run.

The sweep's other findings belong to other classes and are recorded in the sibling documents: suite-economics-1 (the `future_size` budget tests compile to zero tests in every committed run; verification-gap), suite-economics-3 ("the winning window is attempt 1581" is a hand-maintained number the SHA3 swap left behind; documentation, the same site as tree-core-24), suite-economics-6 (`.config/nextest.toml` describes a wall-clock bound no test asserts; documentation), suite-economics-8 (sixty link units for 836 tests, `tests/common` compiled 42 times and `latency.rs` seven times, a build-time cost; modularity, with tests-common-8 the partition's view of it), and suite-economics-10 (the inter-process child reaping busy-polls with a 25 ms real-clock sleep; idiom). The partition reports filed further test-cost findings, kept in their own module sections above: remote-adapter-tests-21, remote-capture-atlas-28, remote-proxy-tests-12, remote-proxy-tests-18, tests-bookmark-13, tests-lifecycle-9, and tests-lifecycle-29. Other-class findings that price verification indirectly: tests-disruption-handshake-14 (five negative assertions rest on 100 ms wall-clock windows), tests-lifecycle-27 (`reuse.rs` takes a Tokio runtime and a wall-clock deadline where the closed-world poller would do), tests-resource-link-window-18 (the ignore-gated `tradeoff_probe` costs about 11 s in release per run and nothing schedules it), and verification-infra-3 (a mutation campaign is hours on ox-east-1 and has no recipe or cadence).

### suite-economics-2: early_first_child_dispute_pair reruns its 262k-tick geometry search on every call (15 calls across 10 tests, about 1.6 s each)
- Where: src/tree/arb.rs:318-359 (related: src/tree/mirror/streaming/remote/proxy/tests/malformed.rs:67, :97, :134, :168, :199; src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:179, :353; src/tree/mirror/streaming/remote/proxy/tests/greeting.rs:58, :72; src/tree/mirror/streaming/remote/proxy/tests.rs:581)
- Class / severity / confidence: performance / medium / high
- Provenance: verified (read the fixture and every call site; run-log clusters: one-call tests 1.625-1.704 s, two-call tests 3.256-3.434 s)
- Verification: reframed. The sweep counted 13 calls; there are 15: `malformed.rs` calls its `deep_pair` wrapper eight times in five tests (three of them inside `for corrupt_left in [false, true]`), and the sweep missed `phase_invalid_signal_propagates_through_the_full_proxy` (malformed.rs:97, 1.704 s). History: no-rationale-found for the per-call recompute; the comment prices the search but does not say why it runs per call.
- Owner-gated: no
- Sign: fixed (the hint window is the same deterministic value computed once instead of found by scanning 2048 windows; the fallback keeps the loud exhaustion failure). Confirm with the per-test times the acceptance names.

Each call precomputes `ATTEMPTS * STRIDE + leaves` first bytes per side (2 x 131k SHA3-256 hashes plus version ticks) and scans the windows before building the one winning pair. The precompute is a deterministic function of nothing, and it costs about 1.6 s per call in the dev profile (the rumors crate and the `sha3` dependency compile at opt-level 0; only `before` and `suanpan` are optimized). Ten proxy tests pay it, five of them twice: about 24 s of CPU per suite pass on a value that never changes. Wall time is untouched under 16-way parallelism; the cargo-mutants campaign (whole suite per mutant, `.cargo/mutants.toml:78-80`) and the llvm-cov legs pay it in full. Under nextest's process-per-test model a `LazyLock` recovers only the within-test duplicate, so memoization alone does not remove the cost. Denominator: per suite pass, about 24 CPU-seconds; per mutant in a campaign, the same again.

Evidence:

    321	    /// prices the fixture. Hashing is deterministic and the winning window
    322	    /// is attempt 1581, so 2048 is exact headroom, not a guess; if hashing
    323	    /// or the leaf encoding ever changes, the search either finds another
    324	    /// window within the budget or fails loudly here.
    325	    const ATTEMPTS: usize = 2048;

    356	    let f_a = firsts(&p_a, ATTEMPTS * STRIDE + LEFT_LEAVES);
    357	    let f_b = firsts(&p_b, ATTEMPTS * STRIDE + RIGHT_LEAVES);

    66	    for corrupt_left in [false, true] {
    67	        let (left, right) = deep_pair();

Anchor correction: the finalized record quoted the two `firsts` calls as lines 349-350 (corrected there as well); at this commit those lines hold the `burnt` closure's `Version::new()` and `version.ticks(party, ticks)`, and the `firsts` calls are lines 356-357, quoted above. The `Where` range 318-359 still contains them.

Resolution: try the known window first. Introduce a `HINT_ATTEMPT` constant (measured after the SHA3 swap: see suite-economics-3), compute only that window's firsts (`LEFT_LEAVES + RIGHT_LEAVES` ticks from `burnt(party, HINT_ATTEMPT * STRIDE)`), evaluate the same geometry predicate, and fall back to the full scan when it fails. The built-versus-simulated equality assert and the loud exhaustion failure stay; determinism is unchanged; the prose number becomes an executed constant. Optionally wrap the result in a `LazyLock` for the two-call tests. A small unit test that the hint window satisfies the predicate, and that a wrong hint falls through to the scan, keeps the fast path checked. Acceptance: the ten proxy tests drop from 1.6-3.4 s to well under 0.1 s each; the fixture still returns a pair satisfying the geometry predicate; the fallback path is exercised by a committed check.

See also: suite-economics-3 and tree-core-24 (the hand-maintained "attempt 1581" this resolution turns into an executed constant); remote-capture-atlas-28 (the other test whose cost is dominated by unoptimized SHA3, so `[profile.dev.package.sha3] opt-level = 2` serves both); the remote-proxy-tests partition's open question on reducing `wide_symmetric_accepts_reordered_match_local` to one deterministic case over this fixture.

### suite-economics-4: 256 proptest cases over finite input spaces of 10 to 32 points
- Where: src/tree/mirror/streaming/tests/faults.rs:115-118 (related: src/tree/mirror/streaming/tests/faults.rs:50-58, tests/party_conservation.rs:308, tests/party_conservation.rs:348-351, tests/redaction.rs:30-34)
- Class / severity / confidence: performance / medium / high
- Provenance: verified (read every generator; `GreetingLie` has exactly the five variants `arb_greeting_lie` lists (faulting.rs:52-75); no `PROPTEST_CASES` override exists in `.config/nextest.toml`, the justfile, or ci.yml, so the default 256 applies; run-log times 10.033 s, 3.598 s, 0.820 s, 1.413 s)
- Verification: confirmed; history: no-rationale-found (the reduced-case blocks elsewhere in the tree carry their reason at the declaration; these four carry none)
- Owner-gated: no
- Sign: fixed (exhaustive enumeration covers every point of each finite space on every run, strictly more than 256 draws with replacement, at about one twenty-fifth of the cost). Confirm with the combined time the acceptance names.

`greeting_lies_classify_exactly` draws from a 5-arm `prop_oneof![Just(..)]` and `any::<bool>()`: ten distinct inputs, run 256 times, each a full-depth comb session plus a second baseline session for the two benign lies, 10.03 s, the fifth-longest test in the suite. proptest does not deduplicate, so exhaustive enumeration is strictly stronger (every point, every run) and about 25x cheaper. The same shape recurs: `party_returns_to_baseline_under_sequential_cycles(k in 1usize..=12)` is 12 points whose k = 12 case asserts every smaller prefix inside its own loop (3.60 s); `party_returns_to_baseline_under_interleaved_cycles` draws a shuffle of 2 to 4 elements, 2! + 3! + 4! = 32 permutations (0.82 s); `redaction_propagates_from_any_peer` has 2 + 3 + 4 + 5 + 6 = 20 behavioral points (`n_peers` 2..=6 times `redactor_idx % n_peers`; `value` does not enter the property) (1.41 s). About 16 s of CPU for what four exhaustive loops do in under a second. Doctrine: state a family as a proptest invariant; a dozen-point finite family is not a sampling problem, and the docstring's "Every greeting lie classifies exactly ... in both orientations" is literally satisfiable by enumeration. Denominator: per suite pass, about 16 CPU-seconds across four tests.

Evidence:

    115	    #[test]
    116	    fn greeting_lies_classify_exactly(
    117	        lie in arb_greeting_lie(),
    118	        fault_client in any::<bool>(),

    308	    fn party_returns_to_baseline_under_sequential_cycles(k in 1usize..=12) {

    348	    fn party_returns_to_baseline_under_interleaved_cycles(
    349	        order in (2usize..=4)
    350	            .prop_flat_map(|m| Just((0..m).collect::<Vec<usize>>()).prop_shuffle()),
    351	    ) {

    30	    fn redaction_propagates_from_any_peer(
    31	        n_peers in 2usize..=6,
    32	        value in any::<u64>(),
    33	        redactor_idx in any::<usize>(),
    34	    ) {

Resolution: replace the `proptest!` wrappers with plain `#[test]` exhaustive loops: `for lie in GreetingLie::ALL { for fault_client in [false, true] { .. } }` (add an `ALL` const if absent), hoisting the comb fixture and its baseline sessions out of the loop; a single k = 12 run for the sequential-cycles test (or `for k in 1..=12` if independent fleets per k are wanted); `for m in 2..=4` over `Itertools::permutations` for the interleaved test; `for n in 2..=6 { for r in 0..n }` with one fixed value for the redaction test. Docstrings stand unchanged. Acceptance: the four tests keep their assertions and docstrings, cover every point of their input space deterministically, and finish in under 1 s combined (from about 16 s).

See also: streaming-tests-17 (the same enumeration proposed for the sampled rosters in `faults.rs` on determinism and legibility grounds, with the seed-file consequences streaming-tests-20 and the streaming-tests partition's first open question record); the suite-economics positives on reduced case counts that carry their rationale at the declaration, which these four sites lack.

### suite-economics-5: identical generated executions repeated across tests that differ only in their assertion
- Where: tests/multi_peer.rs:40-62 (related: tests/multi_peer.rs:81-85, tests/multi_peer.rs:106-110, tests/sanity.rs:20-29, tests/window_census.rs:124, tests/window_census.rs:273-280, tests/window_census.rs:89-96, tests/window_corners.rs:128-129, tests/window_corners.rs:139-140)
- Class / severity / confidence: performance / low / high
- Provenance: verified (read the four files in full and `execute_and_quiesce` in tests/common/schedule/executor.rs:76-85; `N_PEERS` and `MAX_EVENTS` are identical between multi_peer.rs:21-22 and sanity.rs:15-16; run-log times 3.976, 3.967, 4.170, 4.041 s for the four u64 multi_peer properties, 3.515 s for the sanity property, 3.799 s for the census duplicate, 1.139 s for the corners test)
- Verification: confirmed, with one trade stated that the sweep did not: merging the four multi_peer properties keeps 256 schedules per invariant but reduces the distinct schedules the suite draws per pass from about 1024 to 256. History: no-rationale-found.
- Owner-gated: no
- Sign: fixed for the `sanity` deletion and for the census and corners setup (redundant work with no sampled space lost); the `multi_peer` merge trades sampling breadth (about 1024 distinct schedules per pass down to 256) for about 12 CPU-seconds, and the resolution says to state the choice at the declaration whichever way it goes.

Four `multi_peer` properties run the same `schedule_u64()` and `arb_window_assignment()` generator through `execute_and_quiesce` at 256 cases each and differ only in what they assert on the result (the string variant differs in `T` and is a separate claim). `sanity::arbitrary_schedules_dont_panic` runs the identical generator and executor and asserts nothing; its docstring's justification ("if this fails, the others cannot run") is circular, since a panic inside `execute_and_quiesce` fails every multi_peer test the same way. `window_census::floor_overhead_is_bounded_by_content` recomputes the `overhead(0, DIVERGENT_WIDE)` measurement that `window_attributable_residency_stays_inside_admittance` computes at line 124, and does so by repeating the body of the `overhead` helper (lines 274-280 against 90-96) because it also needs `after`. `window_corners::zero_budget_serializes_but_completes` builds the pair and runs the session twice because `latency::session_hops` consumes the pair and returns only the hop count. Roughly 20 s of CPU per pass; wall time is unaffected under nextest parallelism, but cargo-mutants and llvm-cov pay CPU. Principle 3 for the sanity test; redundant setup for the rest. Denominator: per suite pass, roughly 20 CPU-seconds.

Evidence:

    40	    fn all_peers_converge_after_quiesce(
    41	        schedule in schedule_u64(),
    42	        windows in arb_window_assignment(),
    43	    ) {
    44	        let result = execute_and_quiesce(&schedule, &windows);

    58	    fn readout_matches_oracle_after_quiesce(
    59	        schedule in schedule_u64(),
    60	        windows in arb_window_assignment(),
    61	    ) {
    62	        let result = execute_and_quiesce(&schedule, &windows);

    20	    /// Arbitrary schedules complete without panicking and produce a
    21	    /// finite converged state. The safety net for every other
    22	    /// invariant in the suite — if this fails, the others cannot run.

    273	fn floor_overhead_is_bounded_by_content() {
    274	    let (left, right) = diverged(0, DIVERGENT_WIDE);
    275	    let before = node_census().live;
    276	    node_census_reset();
    277	    reconcile(&left, &right);
    278	    let peak = node_census().peak;
    279	    let after = node_census().live;
    280	    let overhead = peak.saturating_sub(before + after);

    128	    let measured = hops(pair(0, 2_048, divergence, divergence));
    129	    let (left, right) = pair(0, 2_048, divergence, divergence);

Resolution: multi_peer: one generated execution per case with the four u64 invariants asserted by named helper functions (the docstring enumerates them), keeping the `_at_floor` and string legs as separate distributions; if the wider draw is worth its cost, say so at the declaration and keep the split. sanity: delete `arbitrary_schedules_dont_panic` or re-aim it at a distribution nothing else executes. window_census: have `overhead` return `before` and `after` alongside the overhead and call it from both tests (removing the verbatim copy of its body), or assert the floor bound inside the admittance test's floor leg. window_corners: use `DelayedWire::round_trip_virtual` once (it returns the reconciled pair and the elapsed virtual time; `hops_on_lattice` converts the latter). Acceptance: every invariant currently asserted is still asserted on at least 256 generated schedules; one definition of the census differencing in window_census.rs; per-pass CPU for these binaries drops by roughly 20 s; `just gate` verdict unchanged.

See also: tests-lifecycle-9 and tests-lifecycle-29 (the partition's records of the `multi_peer` and `sanity` halves, with the injectivity premise and the owner-gating stated there); tests-resource-link-window-26 and tests-resource-link-window-19 (the `window_corners` and `window_census` halves from the partition's side); tests-resource-link-window-20 (the census admittance test this resolution would touch is vacuous today because its budget resolves to the floor, so land that fix first or together).

### suite-economics-7: the capacity witness runs three 8192-leaf sessions; the parent count may not be necessary
- Where: src/tree/mirror/streaming/tests/capacity.rs:171-195 (related: src/tree/mirror/streaming/tests/capacity.rs:77-83, src/tree/mirror/streaming/tests/capacity.rs:291-330, src/tree/mirror/streaming/window.rs:132, src/tree/mirror/streaming/materialized/work/queues.rs:93)
- Class / severity / confidence: performance / low / low
- Provenance: assessed (read; not run: the variant needs a file change)
- Verification: confirmed as a question, not a defect; history: already-known observation. The 2026-07-23 review packet recorded the test at 22.6 s as "within bounds, worth watching" (`review-link-transport-branch.md:18`); `.agent-notes/2026-07-18-parent-placement/parent-placement.md:66-68` cites the `[32,256]` pyramid and the 253/254 boundary as "254 = fan − 2", a per-scope property (`FAN = 256`, window.rs:132; `assembly_level_returns` is sized by `FAN`, queues.rs:93). Nothing recorded says why 32 parents rather than the stress matrix's 4.
- Owner-gated: no
- Sign: undetermined until constructed. If the `[4, 256]` pyramid reproduces all three assertions, shrinking the fixture is a strict deletion (the same assertions at about an eighth of the work); if it does not, nothing changes and the comment gains the sentence it lacks.

At 19.86 s this is the suite's longest test and sets its wall-clock critical path (everything else completes within the same 22 s on 16 workers). It runs one observed session plus two stall probes on `pyramid_pair(&[32, 256], 1, LeafOrder::Reversed)`, 8192 disputed leaves each. Its assertions concern the `AssemblyLevelReturns` high-water mark (at least 254) and the 253/254 stall boundary, both properties of one parent's fan; the stress matrix's "recursive full fan" case already reaches "the fan-sized inter-level return boundary" at `[4, 256]` (lines 77-83), and `parent_delay_no_cross_parent_backlog` (lines 291-330) pins that return backlog does not span sibling parents. If the 32 is not necessary, the critical path shrinks about 8x; if it is, the reason belongs in the comment, which states the claim but not why this width. Denominator: per suite pass, the wall-clock critical path (19.86 s of a 22.07 s run).

Evidence:

    173	fn capacity_stress_witness_requires_inter_level_fan() {
    174	    let (a, b) = pyramid_pair(&[32, 256], 1, LeafOrder::Reversed);
    175	    let expected = join_oracle(a.clone(), b.clone());
    176	    let (actual, report) =
    177	        with_observation(|| scheduled_streaming_mirror(a.clone(), b.clone(), vec![2; 16_384]));

    77	    // A full fan below four simultaneously disputed parents reaches every
    78	    // one-slot recursive query/resolution boundary and the fan-sized
    79	    // inter-level return boundary, with a sibling backlog behind it.

Anchor correction: the finalized record quoted the test body as lines 172-176 and the stress-matrix comment as lines 74-76 (both corrected there as well); at this commit line 172 is the `#[test]` attribute and the body runs 173-177, and the comment runs 77-79 (lines 74-76 hold the "root full fan" case). The `Where` range is widened from 171-194 to 171-195 to include the closing brace, and the related range 73-80 is corrected to 77-83, the "recursive full fan" case the finding names.

Resolution: construct, do not argue. Run the witness once at `pyramid_pair(&[4, 256], 1, LeafOrder::Reversed)` and check all three assertions (high water at least 254, stalls at 253, completes at 254). If they hold, shrink the fixture and add one sentence saying why four parents suffice; if not, record the mechanism that requires 32. Acceptance: either the test runs in about 2.5 s with the same three assertions, or its comment states why the stage must be 32 wide.
Construction: copy the test body with `&[4, 256]`, run it alone (`cargo nextest run -p rumors --all-features -E 'test(capacity_stress_witness)'`), and compare the report's `AssemblyLevelReturns` high water and the two `underbuffered_mirror_stalls` outcomes against the current assertions.

See also: streaming-tests-11 (the stall probes in `capacity.rs` collapse completion, violation, and poll-budget exhaustion into one boolean, so the "completes at 254" assertion passes on a protocol error; fix that first so the construction's verdict means what it says); the suite-economics sweep's positive on `.config/nextest.toml`'s timeout paragraph, which is what absorbs this test's length today.

## Positives

What the crate does well on this dimension, drawn from the partition and sweep reports and deduplicated. Each item names the site so it can be checked.

- Memory pricing rests on layout facts the compiler checks. `window.rs` derives `REFERENCE_SLOT_BYTES` and `FAN_SLOT_BYTES` from `size_of` of the real slot types with the rationale inline; `backend/local.rs:110` pins the `Local` handle at pointer size with a compile-time assertion the window's per-reference price rests on; `budget.rs:64-71` derives `DEFAULT_TARGET_MESSAGE_SIZE` from the wire constants rather than measuring it; and every figure the window prose quotes is pinned by a recomputation in `window/tests.rs` whose assertion message names the doc site to update.
- The codec allocates nothing for the frames a session writes most often. `Heads<const N: usize>` (encode.rs:46-78) renders the fixed heads on the stack with a capacity derived from the grammar's maxima, and `tests/encode_alloc.rs` enforces the zero-allocation claim rather than asserting it in prose; `LeafRun` stays encoded on both sides (frame.rs:58-71) so the per-frame bound is one run's bytes; the over-budget ingress gate (decode/async_io.rs:429-463) decides legality from the first record's heads before buffering the body, and `overbatched_supply_rejects_without_buffering_its_body` prices that premise under an allocation ceiling; `framing.rs`'s grow-as-bytes-arrive policy is priced by `tests/decode_alloc.rs`.
- The two allocator meters are built to catch the cheap fake. `tests/decode_alloc.rs` pairs every ceiling with a liveness floor, uses a non-power-of-two `HONEST_ODD_LEN` so a doubling overshoot cannot hide behind a power-of-two length, and its allocation-event ceiling catches a per-granule reservation policy whose byte reading would look identical; `tests/encode_alloc.rs` calibrates its own harness overhead with a committed test so the frame constants state the writer's allocations alone.
- The wire path avoids copies where they would be easy to add. The encoder serializes each leaf straight out of the borrowed node (encode.rs:197-206) with no `Version` clone and no `Arc` bump; `Frame`s move rather than clone along the whole path; `stream_header` returns a stack array (header.rs:260-267), so a routed stream open allocates nothing in the adapter; `PayloadCodec` is `Copy` (two fn pointers and a limit), so per-message wire decode is a direct call with no `dyn` dispatch.
- Monomorphization is bounded and its cost is written where it is paid. `DynRead`'s doc (gossip.rs:56-70) states the cost model (one vtable call per stream open and per poll beneath frame buffering); `Reconciliation::reconcile`'s doc (gossip.rs:1114-1126) explains why both the boxed `dyn` coercion and `inline(never)` are needed; the walk bodies take `Replies<E>` and instantiate once per backend, with the typed re-tags confined to `Work::respond`'s exit and two fixed-height root sites; and the prune recursion's one-future-per-node design (unknown.rs:17-23) was chosen on a measured 40.5 percent reduction in cumulative llvm-lines, recorded in bf1a5b4b.
- The commit path is designed to hold the watch lock briefly. The walk runs on an O(1) structural clone of the root; the changed flags are decided by the traversals, not by hashing, so no root hash is read inside a critical section; and `tree.rs:616-650` meters root-hash reads per commit path with a liveness leg (`root_hash_read_meter_is_live`) beside its two zero-ceiling pins. The entries above (tree-core-27, tree-core-29) shorten what the lock still covers; they do not contradict the design.
- Reads are cheap by mechanism, not by claim. A `Snapshot` clone is a `Bytes` refcount bump plus an `Arc` bump, `len` is a stored count, and hash and bounds are `OnceLock` memos; `RangeOwned` is a constant-state spine walk (one `Level` per materialized branch, siblings never enumerated, `successor` by binary search) documented with the memory argument the session window relies on; `from_sorted_leaves`'s `Option` slots let the recursion move nodes out of a shared slice without cloning; `fan_is_forty_bytes` and `node_inner_stays_within_budget` make per-node cost a reviewed number with the growth rule stated in the testdoc.
- `Local`'s bulk `leaves` and `assemble` overrides skip the per-virtual-level work of the default chain and are held to that chain by observational-equivalence proptests over both deep-spine and wide-fan shapes (backend/local/tests.rs), comparing hash, length, floor, ceiling, and the unserialized `version_bytes` aggregate per node.
- Test-only instrumentation costs nothing in release. `Progress` is a `Copy` type that is a ZST outside `cfg(test)`, passed by value through the proxy; the walk's `#[cfg(test)]` trace hooks live inside the `yield_resolve_query!` expansion, so the instrument cannot drift from the production publication order.
- The benches measure what they say they measure. Every Criterion group keeps fixture construction untimed (`iter_batched` at `PerIteration`) and warms lazy memos explicitly; `grid.rs` states its throughput denominator (per-side transfer, not the shared size); `benches/support/latency.rs:10-62` argues its measurement model as mechanism, turns wire delay into an exact, load-independent hop count on a paused clock, refuses to report a virtual figure on a wall-clock wire, and fails loudly off the delay lattice, which is what lets the window suites pass interleaved with everything else under load.
- The suite itself is economical where it has been priced. The sweep's run completed 836 tests in 22 s wall on a machine whose load average rose from 9.6 to 19, with no slow markers, retries, or flaky results. `.config/nextest.toml:14-24` names the one collision its timeout budget knowingly accepts and the recovery procedure. Reduced case counts carry their rationale at the declaration (tests/party_conservation.rs:386-392, proxy/tests.rs:524-529, capacity.rs's `arb_stress_widths`), and every proptest in the observation suites whose per-case cost is a wire session states why its count is pared. Cargo.toml:174-187 documents the dev profile's compile-time-for-test-time trade and why debug assertions must stay on, and `.cargo/mutants.toml:26-37` states the matching campaign profile. `tests/common/wire.rs:26-32` reuses one current-thread runtime per test thread across proptest cases, with the reason stated. `tests/disruption.rs`'s inter-process simulation costs 0.66 s while exercising real TCP and real process boundaries.
- Two model meter tests exist to copy. `unknown/tests.rs` keeps a retained known-worse shape as the cost oracle, asserts verdict equality before comparing cost, floors both counters, and records a `MEASURED` line; `tests/dispute_wire.rs`'s negative control (343-371) proves its counter alive on a session that disputes nothing and bounds the fixed overhead below one byte per message, which is what licenses its exact integer pins.
- Atomics are used at the cheapest ordering that is correct, with the reason written: `SupplyLedger::charge` uses one `Relaxed` `fetch_add` with a post-check that is race-correct for two ingestion sites, and `stats.rs` justifies its `Relaxed` counters by the single post-completion read.

## Open questions for Finch

Decisions only the owner can make that bear on cost, deduplicated across the partition and sweep reports, each with a recommendation.

1. Reopen B2 (remote-codec-24)? The ruling declined enforcement because the detector would never fire against an existing encoder; the per-record 4 KiB memset and `Vec` allocation were not before you. Recommendation: enforce, hand-parsing the atom as the greeting does, and name the ruling and the two flipped pins in the commit.
2. The decode channel's liveness claim and the pull-based reader (remote-adapter-streams-6, remote-proxy-27). Recommendation: construct the sub-FAN run first (one test-only parameter); if it completes, rewrite the two comments to the amortization and read-ahead rationale and treat the pull-based reader as a measure-first experiment gated on a persistent backend that actually wants read-ahead. The answer decides whether the roughly 205 KB supply-decode pre-charge is a floor or a cost of the current shape.
3. The larger `act` redesign behind tree-core-27 (recurse on `&mut [(Path, Version, Action)]` slices with a depth index as `from_sorted_leaves` does, and build a fresh subtree under an absent child in one shot) changes the fuse fire-point structure that `act_mid_walk_unwind_leaves_tree_byte_identical` derives its arming depth from. Recommendation: land the single-sort tier first, measured with the existing benches, and decide the redesign only on those measurements.
4. `multi_peer`'s one-named-test-per-invariant policy versus fusing (tests-lifecycle-9, suite-economics-5). Recommendation: fuse the two implied checks into the canonical-map test with distinct assertion messages, state the policy in the module doc for the tests that remain separate, and state the sampling-breadth trade (about 1024 distinct schedules per pass down to 256) at the declaration whichever way it goes.
5. The geometry-search fixture (suite-economics-2, suite-economics-3, tree-core-24): an executable `HINT_ATTEMPT` constant converts the prose number into a checked one and removes about 24 s of CPU per pass; the alternative is deleting the number and leaving the search as it is. Recommendation: the hint, measured after the SHA3 swap.
6. The capacity witness (suite-economics-7): does the `[4, 256]` pyramid reproduce the high-water mark and the 253/254 boundary? One measured run answers it and either shrinks the suite's critical path about eightfold or produces the sentence the test is missing. Fix streaming-tests-11 first so the run's verdict is trustworthy.
7. A walk-side allocation meter (materialized-30): wanted at all? Without it the vector-capacity reservations should stay recorded candidates rather than land. Recommendation: yes, one `stats_alloc` region over a fixed disputed shape; the same meter serves as the acceptance instrument for remote-proxy-27 and streaming-backend-window-9.
8. The `NodeInner` size pin and the inline prefix (tree-typed-23): the pin's own doc says growth must be a deliberate, reviewed decision, and the change is a trade, not a deletion. Recommendation: construct and measure with `benches/in_memory.rs`, a `stats_alloc` count per inserted leaf, and `testing::node_census`; land only on evidence, or record the decline with the numbers.
9. Lock scope of `Batch::commit` (raised by the api-core partition, not filed as a finding): the whole `Tree::act` runs inside `send_if_modified`'s write lock, so `snapshot()`, the `Debug` impls, and observer `borrow_and_update` calls block for the commit's duration, and a 10^5-message `send_all` is O(N log N) under the lock. Is a measured figure for the large-batch case wanted before persistent storage lands? Recommendation: yes, once tree-core-27 lands, since it changes the constant; pair it with async-hazards-3's cheap deferral of the pre-image drop past `send_if_modified`.
10. The supply path's bare-leaf rebuild (raised by the tree-typed partition, not filed): `Leaf::into_node` (iter.rs:345-362) costs one `Arc<NodeInner>` allocation, a `Version` clone, and a `Message` clone per supplied leaf to uphold `Node<Z>`'s bare-leaf invariant that only `from_sorted_leaves` asserts and the supply encoder never needs. Is the invariant worth the per-leaf allocation on the wire path, or should `Backend::leaves` yield a leaf handle distinct from `Node<Z>` (touches the `Backend` trait)? Recommendation: measure with `benches/gossip_fixed.rs` on a supply-heavy fixture before deciding.
11. `warm_caches` (async-hazards-4, api-audit-15, inventory-4): should a named public entry let applications pre-pay the first-greeting materialization, which today hashes and bounds the whole cold tree inside one poll? An API addition. Recommendation: the doc sentence is warranted either way; the entry is your call.
12. Bench economics (benches-envelope open questions): report `Throughput::Bytes` from `Gossiped.stats` beside elements so the gossip benches read as wire efficiency; add a `--sample-size 10 --measurement-time 1` smoke of each bench binary to `just all` so a panicking fixture cannot hide until someone runs `just bench`; and state at `CAPACITY` why `Wire` uses 64 KiB pipes while `DelayedWire` uses 8 MiB, since the two zero-latency intercepts measure different harness overheads. Recommendation: all three, when the next bench pass happens.
13. Binary layout and build time (suite-economics-8, tests-common-8, and the tests-bookmark partition's one-binary-or-four question): `tests/common` is compiled 42 times and `latency.rs` seven. Recommendation: measure `cargo build --tests --timings` at HEAD first; then consolidate the window family and the five single-test binaries, and leave the schedule-engine suites; adopt a path dev-dependency crate only if the number is material.
14. Case budgets (tests-lifecycle partition): none of the schedule-executor binaries set a `ProptestConfig`, so each property runs 256 cases at up to 8 peers and 50 events, while session_overlap (48), disruption (8), and party_conservation (32) budget explicitly. Chosen or inherited? Recommendation: if 256 is fine on the wall clock, say so in one module doc; otherwise budget the swept legs and keep the floor legs at 256.
15. Two ungated instruments (tests-resource-link-window-18, suite-economics-1): `tradeoff_probe` costs about 11 s in release per run and nothing schedules it, though `Peer::sync_memory_budget`'s rustdoc quotes its figure; `future_size` compiles to zero tests in every committed run. Recommendation: wire `tradeoff_probe` into the CI `instruments` job, and give `future_size` a debug-profile budget measured once rather than a release leg.
16. Observability for tuning (materialized-2, remote-codec-5, and the frame-counter questions from mirror-common and remote-adapter-streams): a `SessionStats::window_stalls` counter, a getter for the effective run budget, and `frames_sent`/`frames_received` would give a user the readouts that tell them which way to move `sync_memory_budget` and `target_message_size`. Recommendation: the stall counter as zero-versus-nonzero and the budget getter now; the frame counters only if the tuning question is your own.
17. Unrecorded wall times: the total sweep in `overlapped_install_never_loses_innocent_messages` (tests-observation) and `bounded_corpus_manifest_snapshot` (remote-capture-atlas-28) have no committed figure, and the mutation campaign (verification-infra-3) is hours on ox-east-1 with no recipe or cadence. Recommendation: one measured number in a note for each test, and a named hand-run recipe for the campaign.

## Counts

Thirty-one entries at commit 9e5784fb, dated 2026-09-02.

| Severity | Count |
|---|---|
| high | 0 |
| medium | 8 |
| low | 17 |
| nit | 6 |
| total | 31 |

| Module section | Entries | Ids |
|---|---|---|
| Crate root and public surface | 1 | api-core-29 |
| Session and bookmark | 1 | api-core-10 |
| Link | 2 | link-14, link-27 |
| Tree core | 2 | tree-core-27, tree-core-29 |
| Tree typed | 3 | tree-typed-6, tree-typed-23, tree-typed-30 |
| Streaming backend and window | 2 | streaming-backend-window-9, streaming-backend-window-11 |
| Materialized | 2 | materialized-30, materialized-35 |
| Remote codec | 2 | remote-codec-12, remote-codec-24 |
| Remote capture and codec tests | 1 | remote-capture-atlas-28 |
| Remote adapter and streams | 3 | remote-adapter-streams-6, remote-adapter-streams-16, remote-adapter-tests-21 |
| Remote proxy | 3 | remote-proxy-27, remote-proxy-tests-12, remote-proxy-tests-18 |
| Integration tests | 3 | tests-bookmark-13, tests-lifecycle-9, tests-lifecycle-29 |
| Benches and examples | 2 | benches-envelope-18, swarm-example-5 |
| The cost of verification (suite-economics sweep) | 4 | suite-economics-2, suite-economics-4, suite-economics-5, suite-economics-7 |

Sections with no performance entries and therefore omitted: conformance, mirror common, test scaffolding (`src/testing`, `src/tests`).

By sign: 22 entries are strict deletions of redundant work (adopt and confirm), 5 are trades or shape changes to measure first (tree-typed-23, remote-adapter-streams-6, remote-proxy-27, the reservation half of materialized-30, the branch half of tree-typed-6), 1 is undetermined until constructed (suite-economics-7), and 3 are deletions whose counterweight is a stated policy or naming value the owner may prefer to keep (tests-lifecycle-9, remote-proxy-tests-12, remote-adapter-tests-21). Owner-gated entries: remote-codec-24, remote-adapter-streams-6, tree-typed-23, tests-lifecycle-9, and the redesign half of tree-core-27.
