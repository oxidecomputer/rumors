# rumors: orientation map (commit 9e5784fb)

Provenance marks: **[V]** verified mechanically (grep, script, or read line by line); **[R]** assessed by reading prose. Every `file:line` cites a line I read. Scratch artifacts: `scratchpad/map/{moddocs.txt,deps2.txt,no_moddoc.txt}`.

## 1. Layering, top to bottom

**L0: the replica API** (`src/lib.rs:304-349` declares modules and re-exports [V]).
- `peer` (`src/peer.rs`): `Peer<T,B>`, the `!Clone` identity anchor holding `network`, `window: WindowConfig`, `run_budget: RunBudget`, `inner: watch::Sender<Inner<T>>`, `bookmark`, `codec: PayloadCodec`, `observe: Attachment` (`peer.rs:145-169`); `Inner<T>` is `party: Option<Party>` plus `tree: Tree<T>` (`peer.rs:177-180`). Owns the config knobs and the local mutation path, every one a `Batch` (`peer.rs:627-676`).
- `peer::bootstrap` (`src/peer/bootstrap.rs`): the `Bootstrap`/`BookmarkedBootstrap` builders and `Joined`.
- `peer::gossip` (`src/peer/gossip.rs`): the wire-session drivers for bootstrap, gossip, retire, and the `gossip_when` driver; preamble/epilogue constants; `Reconciliation`; `PartyGuard` (`gossip.rs:1-6`).
- `rumors` (`src/rumors.rs` + `causal.rs`, `changes.rs`, `unordered.rs`): `Rumors<T,B>`, the cloneable working handle wrapping a `Peer` (`rumors.rs:27-31`); the set observers `UnorderedMessages`, `CausalMessages`, `Changes`.
- `batch` (`src/batch.rs`): `Batch`, one commit of sends and redacts. `snapshot` (`src/snapshot.rs`): `Snapshot<T>`, a point-in-time `Tree` view.
- `bookmark` (`src/bookmark.rs`, `bookmark/format.rs`): identity persistence (`Bookmark`, `NoBookmark`) and the self-checking CBOR frame.
- `observe` (`src/observe.rs`): the rumors-blind wire hook (`Observer`/`SessionObserver`/`StreamObserver`) plus the crate-internal `Attachment`/`SessionHandle`.
- `error` (`src/error.rs`): `Error<B>` and the re-exported diagnostic taxonomy (`error.rs:38-51`).
- Foundational value types: `message` (`Message`, `PayloadCodec`, `PayloadDepthLimit`, `EncodeError`), `network` (`Network`, 16 bytes), `protocol` (`Protocol`, now the single variant `V2`, `src/protocol.rs:15-19`), `tags` (CBOR tag constants).
- Docs-only: `reconciliation`, `tutorial`. Gated: `conformance` (`conformance::link` public suite; `conformance::backend` in-crate), `testing` (`test-internals`, `doc(hidden)`).

**L1: transport contract**: `link` (`src/link.rs`): `Link`, `Connector`, `Acceptor`, `Done`, `SessionState`, `STREAM_COUNT`, `memory()`; `link::erased` (`pub(crate)`, `link.rs:626`) is the monomorphization funnel; `link::routed` (+`endpoint`, `header`, `router`, `stream`) adapts accept/connect transports.

**L2: session framing** (`src/tree/mirror.rs` and siblings): `mirror::Error<C,S>` and `contained` (`mirror.rs:37-39`); `cbor` (canonical head primitives); `framing` (`read_payload`/`resume_payload`); `handshake` (the 30-byte `Preamble`, `Staged`, `Intent`); `party` (trailing identity hand-off).

**L3: the streaming mirror** (`src/tree/mirror/streaming.rs:10-24` names the layers [V]): `protocol` (type-level phase schedule), `message` (`Greeting`, `Reply`, `Reaction`, `initiates`), `backend` (+`local`: `Backend`, `Leaf`, `Local`, `Root<B>`), `materialized` (the walk, deadlock-freedom argument; `work/{levels,assembly,queues,resolver,answer}`, `unknown`, `progress`, `transcript`), `remote` (the proxy: `codec/*`, `proxy/{start,state,work/*}`, `adapter/*`, `streams`), `window`, `erased`, `convert`, `driver`, `channel`, `tasks`, `stats`.

**L4: the content tree** (`src/tree.rs`): `Tree<T>`, `Root` (node plus out-of-node `ceiling`, `tree.rs:101-111`), `Action`, `Iter`; `traverse/{act,join,unknown}`; `typed/{hash,height,node,path,prefix,untyped/{fan,iter}}`.

**L5: external**: `before` (`Party`, `Version`, `causally`; re-exported at `lib.rs:328-330`), `sha3`, `ciborium` (pinned `=0.2.2`, `Cargo.toml:36-46`), tokio's io traits only (`lib.rs:231-236`).

Module-doc coverage [V]: 133 of 183 `src` files open with a `//!` block; the 50 without are listed in `map/no_moddoc.txt`. Production files among them: `batch.rs`, `message.rs`, `rumors.rs` and its three children, `snapshot.rs`, `tree/traverse/act.rs`, all six `tree/typed/*.rs`, `streaming/backend/local.rs`, `materialized/{common,error}.rs`, `work/{answer,resolver}.rs`, `protocol/peer.rs`, `remote/adapter/{decode,encode,error,scope}.rs`, `proxy/work/progress/trace.rs`.

Retirement residue [R], for whoever deep-reads these: `.agent-notes/2026-09-01-v1-retirement/README.md` records V1's removal and the `.protocol()` builders' deletion (`README.md:5-8`), yet `src/protocol.rs:1` still reads "Selectable wire reconciliation protocols." over one variant; `src/peer.rs:604` still says "like changing the selected [`Protocol`]"; `src/error.rs:14` still advises "select the same [`Protocol`] at both ends"; `src/peer/gossip.rs:615-616` still says the observation handle is "inert unless a handler is attached and the dialect is observable" though the note says the dialect guard was dropped (`README.md:225-226`). The note itself carries two `Status:` blocks that disagree ("implemented", line 3; "in progress", line 43).

## 2. One gossip session, top to wire and back

1. `Rumors::gossip(&mut link)` (`rumors.rs:489-501`) forwards to `Peer::gossip` (`gossip.rs:459-488`), which calls `erase(link)` (`gossip.rs:1239-1254`): `SessionState::begin` fails fast on a poisoned link, sets the poison latch, and advances the epoch (`link.rs:385-393`); the link becomes `DynLinkParts` (`gossip.rs:85`), erased so the protocol towers codegen once (`gossip.rs:56-70`).
2. `gossip_inner(Intent::Remain, staged, parts)` (`gossip.rs:598-905`). A `Recorder` and a `SessionHandle` are created (`gossip.rs:614-622`). `handshake::preamble` exchanges the fixed 30-byte `55799(["rumors", version, network, intent])` item (`handshake.rs:12-17`; `gossip.rs:625-629`). Mutual retire bails via `epilogue` (`gossip.rs:637-643`).
3. One `watch` critical section under the bookmark mutex: reclaim into the bookmark, snapshot `prior_tree`, and speculatively take/fork the party into a `PartyGuard` (`gossip.rs:671-716`; guard semantics `gossip.rs:1379-1404`).
4. `Reconciliation::reconcile` (`gossip.rs:1080-1168`), boxed and `inline(never)` so the tower stays in this crate (`gossip.rs:1118-1126`). It builds the two participants: the walk, `materialized::Handshaking::start(Local, root)` with `.window/.target_message_size/.stats`, and the proxy, `streaming_remote::Handshaking::start(Local, carrier, codec)` with `.window/.stats/.observe` (`gossip.rs:1143-1151`), where `carrier = Link::for_session(...)` (`link.rs:441-461`).
5. `streaming::handshake(local, proxy)` (`streaming.rs:156-181`): `Client::connect` yields our `Greeting`; `Server::accept` sends it and returns the peer's; `complete_connect` hands the peer greeting to the walk. The `Greeting` carries `version`, `set_len`, `max_version_bytes`, `target_message_size`, `payload_depth_limit`, `listing` (`message.rs:62-118`); its wire spelling is one tag-24 item (`codec/greeting.rs:1-22`). Then the newborn check for a bootstrap claimant or the `NetworkMismatch` check (`gossip.rs:1155-1163`).
6. `Handshaken::reconcile` → `descend` (`streaming.rs:110-134`, `184-217`): equal versions resolve both sides via `complete_equal` with no descent; otherwise `message::initiates` elects the initiator (smaller `set_len`, tie broken by canonical version bytes, `message.rs:141-156`) and `mirror_connected` runs the schedule (`driver.rs:152-172`): `i.initiator; r.responder; 15 × (i.reply; r.reply); i.reply; r.complete_responder; i.complete_initiator`. The typed chain is `Connect → CompleteConnect → {CompleteEqual, Initiator, Responder} → Reply… → CompleteInitiator/CompleteResponder` (`protocol.rs:56-200`); `Peer<I>`/`Client<I>`/`Server<I>` spell the chain at 15 initiator and 14 responder rounds (`protocol/peer.rs:108-115`). Each `Reply` phase descends two heights (`protocol.rs:142-161`), hence `STREAM_COUNT = ceil(32/2) + 1 = 17` (`link.rs:161-169`).
7. Inside a phase: the walk pairs each incoming `Reply` positionally with its pending `Query`, resolves scopes (`materialized.rs:12-24`), and answers through `Work::respond`; the proxy encodes replies to frames (`[stream, state, body]`, `codec.rs:3-27`), attaches the new `Scope` only after the frame flushes (`adapter.rs:37-40`), opens each logical stream lazily on its first frame (`streams.rs:10-19`), and decodes the peer's frames back into `Reply`s, reconstructing supplied nodes from leaf runs (`adapter.rs:42-67`). The initiator's opening question rides the greeting; its exclusive root children ship as early supplies (`remote.rs:43-54`).
8. Back in `gossip_inner`: `party::receive` if the peer is retiring, or `bookmark_donate` then `party::send` if we donate (`gossip.rs:753-803`); then the commit critical section: absorb the party, `inner.tree.join(merged)`, wake observers on retire, content change, or ceiling advance (`gossip.rs:816-871`); persist an absorbed retiree (`gossip.rs:884-886`); exchange the epilogue marker `"."` concurrently (`gossip.rs:54`, `1285-1317`, `897-899`). On `Ok`, `link.session.finish()` clears the poison (`gossip.rs:476-478`) and the caller receives `Gossiped { converged, led: Led::Local, stats }` (`gossip.rs:483-487`).

`gossip_when` (`gossip.rs:913-1068`) wraps the same `gossip_inner` in an `unfold` that selects between `Staged::fill` (remote preamble) and the `when` cue stream, suppressing `WhenChanged` cues when `latest == converged` (`gossip.rs:998-1013`). Bootstrap runs the same machinery from `Root::default()` with `Network::BOOTSTRAP` (`gossip.rs:286-295`, `1186-1231`), then `party::receive` and the epilogue (`gossip.rs:320-327`).

## 3. Key invariants and the model of record, as the crate states them

- **Model of record** (`window.rs:25-27`): "Content addresses are uniform 32-byte strings (the model of record: uniform-hash, authenticated-honest-peer)". AGENTS.md's hard rules add that "hostile-peer regimes are off-model" and "Violation/fail-fast machinery is a conformance bug detector, not a security boundary" (echoed at `mirror.rs:11-14`, `link.rs:115-139`).
- **Trust** (`lib.rs:63-69`): peers "trust one another: the protocol rejects malformed and mismatched sessions, but it is not Byzantine-tolerant."
- **Version uniqueness and linearity** (`reconciliation.rs:31-36`): "Version reuse … cannot arise: every send creates a fresh version … and the linearity of parties keeps replicas' versions disjoint." `traverse::act` panics on reuse as "a bug in this crate, never an environmental failure" (`act.rs:30-37`).
- **The set is the tree** (`tree.rs:16-19`): leaf placement "is fully determined by the version stamped on it, so two replicas holding the same messages hold the same tree". Digests are "a pure function of the version set, blind to message bytes" (`tree.rs:20-22`).
- **join ≡ mirror** (`tree.rs:58-61`): "`join` and `mirror` are observationally identical — both delegate deletion honoring to the same filter"; `traverse/unknown.rs:9-11` names that filter.
- **Redaction without tombstones** (`reconciliation.rs:96-103`, `135-136`): a leaf whose version is `<=` the lacking side's version "must have been deleted"; outcome: "both replicas hold every message either held and neither had redacted when the session began."
- **Deletion honoring depends on strict forget ticks** (`tree.rs:377-380`, `424-430`) and on a bootstrap claimant's empty greeting version (`gossip.rs:1256-1274`).
- **24-byte digests** (`reconciliation.rs:157-162`): false-equal "is a per-interior-comparison event at 2⁻¹⁹²", and a landed false-equal "permanently deletes the divergent messages fleet-wide" (`reconciliation.rs:147-155`).
- **Link contract** (`link.rs:24-68`): control duplex, per-stream independence, receiver-paced flow control at any positive capacity, up to `STREAM_COUNT` concurrent streams, completion via `Done`, accept-cancellation tolerance.
- **Deadlock freedom** (`materialized.rs:26-63`): "Every await … is for the k-th item of one specific stream"; "wire before internal publication" and "resolution before dependent work" make "one slot *sufficient* for every query and resolution channel"; independence is "an interface obligation, supplied by the link or not at all" (`materialized.rs:65-77`).
- **Window** (`window.rs:53-60`): exceeding a capacity means "that stage serializes … Degradation is latency, never memory growth and never deadlock". Assembly fan queues are a correctness floor, not tunable (`window.rs:67-76`).
- **Session promise** (`link.rs:279-321`): `Ok` certifies both sides committed; `Err` leaves the replica unchanged and the link poisoned, with three qualified exceptions; cancellation counts as `Err`.
- **Commit atomicity** (`tree.rs:507-523`, `576-590`): nothing mutates until the commit point; fuse-injected unwind pins named there.
- **Wire stability** (`lib.rs:286-288`; AGENTS.md hard rules): snapshots re-accepted only for deliberate, owner-ruled format changes; `ciborium` pinned because "depth admission and wire ingress run the same compiled deserializer" (`Cargo.toml:36-40`).

## 4. Oracles and instruments

**Differential oracles [V]:**
- `Tree::join` is the streaming mirror's oracle: `join_oracle(a: Root, b: Root) -> Root` (`streaming/tests.rs:152-157`) drives `streaming_matches_join_oracle` (`:261`) and three expected values in `tests/capacity.rs:41,175,375`. Join's own laws ground it (`traverse/join/tests.rs:1-7`), together with a route-equivalence property in `tree/tests.rs` that I know only from that doc comment (`join/tests.rs:5-6`) [R].
- `traverse::unknown` is the oracle for the streaming `materialized::unknown` pruner (`materialized/unknown/tests.rs:1-3`).
- `Local`'s bulk `leaves`/`assemble` overrides are checked against the level-by-level `Convert` default (`backend/local/tests.rs:1-8`).
- Integration suites use a spec-shaped `Oracle` of `BTreeMap`/`BTreeSet` keyed by `EventIdx` (`tests/common/oracle.rs:1-9`): `async_wire, bootstrap, membership, multi_peer, pairwise, partition, redaction, retire, sanity, session_overlap, shadow_validity, target_message_size`. `shadow_validity.rs` is the meta-test that the shadow simulator agrees with the executor.
- Formal bridges: `streaming/tests/{skeleton,wedge,local_eq,announced}.rs` tie real sessions to the Lean model's `Skel`, `Mux.wedge`, `LocalEq`, and the payload-independence premise B5 (module docs at each).
- Session counters against a constructed corpus: `streaming/tests/stats.rs:1-16`, re-checked publicly in `tests/session_stats.rs`.

**Snapshot pins [V]:** `insta` in `tests/{gossip,bootstrap,retire}_snapshot.rs` (22 `.snap` files under `tests/snapshots/`, provenance-swept by `tests/snapshot_liveness.rs`), `bookmark/format/tests.rs`, `codec/{tests.rs,signal/tests.rs,tests/error_atlas.rs}`. The renderer is `codec/capture.rs` (a CBOR reflection rendering argued injective on wire bytes, `capture.rs:12-30`). Rules for re-accepting: AGENTS.md hard rules.

**Meters [V]:** allocator meters `tests/decode_alloc.rs`, `tests/encode_alloc.rs` (`stats_alloc`, `Cargo.toml:169-172`); `tests/party_conservation.rs` reads `before`'s `encoded_bits` (`Cargo.toml:146-149`); `tests/dispute_wire.rs` pins the 43 B dispute overhead cited at `peer.rs:391-394`; the window suites (`window_census`, `window_corners`, `window_knee`, `window_operator`, `window_sweep`, `gossip_pipelining`, `tradeoff_probe` ignore-gated) assert on virtual wire time (`.config/nextest.toml:28-35`); `tree.rs:616-650` meters root-hash reads per commit path; `hop_trace.rs` counts hops.

**Property tests [V]:** 22 `tests/*.rs` files carry `proptest!`; 33 committed seed files under `proptest-regressions/` (anchored by the empty `tests/main.rs`, swept by `tests/seed_liveness.rs`).

**Liveness harness:** `testing::run_to_quiescence` (`testing.rs:375`) turns a wire stall into an error under a closed-world poller; nextest kills after three 60 s periods (`.config/nextest.toml:25-26`).

**Conformance:** `conformance::link::check` for caller-built links (`conformance/link.rs:10-19`), validated in both directions (`conformance/link/tests.rs`); `conformance::backend` prices node residency against `Backend::node_bytes` (`conformance/backend.rs:1-24`).

**The gate [V]:** `gate: gate-lints gate-streams` (`justfile:400`). `gate-lints` = `fmt-check doclint testdoc workflowlint manifestlint digestshare mutants-list readme-check` (`justfile:403`). `gate-streams` runs eight concurrent streams (`justfile:464-471`): `workspace` = `clippy clippy-default docs test-all citecheck`; `doctest`; `board`; `wasm`; `fuzz`; `surface`; `internal-docs`; `audit` = `supply-chain`. Of these, `board`, `wasm`, `fuzz`, `surface` exercise `crates/before` artifacts, not rumors [R, from the recipe comments opening at `justfile:354, 561, 609, 646, 906`]. `ci` (`justfile:1000`) is the no-rot sweep GitHub runs (`.github/workflows/ci.yml:107-108`); its `instruments` job re-runs `supply-chain`, `amp-board-acceptance`, `worst-cases-pin`, `surface-totality` (`ci.yml:171-184`); `coverage` runs `coverage-kernel{,-branch}` (`ci.yml:226-230`). `.cargo/mutants.toml` sets `test_tool = "nextest"`, `test_workspace = true`, `--all-features` (`:78-80`); every one of its exclusion patterns names a `crates/before` file, none a rumors `src/` path [V].

## 5. Module dependency graph

Generated by `map/deps2.txt` from `use crate::` and `use super::` statements only (not qualified paths in expressions), test and scaffold modules separated [V].

Layered by what each imports:
- **Leaves** (import nothing crate-internal beyond the root): `link`, `message`, `network`, `protocol`, `tags`, `observe`, `bookmark`, `reconciliation`, `tutorial`, `tree::mirror`, `tree::mirror::cbor`, `tree::mirror::framing`, `tree::typed`, `tree::typed::hash`, `streaming::{channel, tasks, stats, remote}`.
- **Tree core**: `typed::{height, path, prefix, node, untyped, untyped::{fan,iter}}` → `typed::*`, `message`, `causally`; `traverse::{act, join, unknown}` → `typed`, `unknown`; `tree` → `typed`, `message`.
- **Session framing**: `handshake` → `cbor`, `observe`; `party` → `cbor`, `tags`, `observe`; `bookmark::format` → `tags`, `mirror::cbor`.
- **Streaming**: `message`, `protocol`, `backend`, `convert`, `erased`, `window`, `materialized/*`, `remote/*`, `driver` → each other and `typed`, `link`, `observe`, `message`.
- **API**: `peer`, `peer::bootstrap`, `peer::gossip`, `rumors/*`, `batch`, `snapshot`, `error` → everything below.

Cycles and sideways edges worth a reviewer's eye:
- `peer` ↔ `rumors`: `peer.rs:22-24` imports `Rumors`; `rumors.rs:12` imports `Peer`. By design (the two faces of one replica, `lib.rs:113-126`), but a genuine cycle. `batch` reaches `Inner` through `lib.rs:339`.
- Within `streaming`: `materialized` ↔ `window` (`materialized` imports `window`; `window.rs:123` imports `materialized::Resolve`); `materialized` ↔ `erased` (`erased` imports `materialized::children_of`); `materialized` → `remote` for one constant (`materialized.rs:114`, `remote::DEFAULT_TARGET_MESSAGE_SIZE`) while `remote::{adapter::decode, proxy::work, proxy::work::pump}` → `materialized`. The walk depending on the proxy is the one edge that inverts the module doc's layer order (`streaming.rs:10-24`).
- `window` → `link::STREAM_COUNT` (`window.rs:124`): the memory model reaches the transport constant directly.
- `tree::mirror::handshake`, `party`, `remote::codec::{signal,capture}` → `observe` (`handshake.rs:27`, `party.rs:8`, `signal.rs:4`, `capture.rs:52`). Not upward in fact: `observe` imports only the crate root, so it is a foundation module that happens to be public.
- `streaming::backend::local` → `tree` (the in-memory backend wraps the `Tree`), and `tree` → `mirror` only by `pub mod` declaration (`tree.rs:73`).

## 6. Glossary (crate terms, with defining file)

- **universe / seed**: a gossip network created by one `Peer::seed` (`lib.rs:78-82`); `Network` is its 16-byte id (`network.rs:25`).
- **party**: a peer's ITC identity, `before::Party`; **linearity** of parties is the invariant everything rests on (`lib.rs:63-67`).
- **Version**: `before::Version`, the causal stamp; a leaf's address is SHA3-256 of it (`reconciliation.rs:12-13`, `typed/path.rs:15` `Path`).
- **ceiling / floor**: memoized version bounds per node; the root ceiling rides outside the nodes (`tree.rs:101-111`).
- **Merkle hash / `Hash`**: 24-byte truncated SHA3-256, `MERKLE_HASH_LEN` (`typed/hash.rs:12,50`); **`PathHash`**: the 32-byte address (`hash.rs:253`).
- **radix / fan**: a branch's children keyed by byte, `Fan` (`untyped/fan.rs:53`); `FAN = 256`.
- **path compression**: single-child spines collapsed (`tree.rs:31-36`).
- **act / join / unknown**: the traversal trio (`tree.rs:53-61`); `Unknown` is the deletion-honoring filter (`traverse/unknown.rs:22`).
- **causal sieve**: the seen-but-absent inference (`reconciliation.rs:84-103`).
- **disjoint frontier**: the cut where each node is held exclusively by one side (`reconciliation.rs:66-68`).
- **dispute / disputed scope**: both sides hold a child, digests differ (`reconciliation.rs:61-64`).
- **supply / run / `RunBudget`**: exclusive leaves shipped as byte-budgeted leaf runs (`remote.rs:33-41`, `codec/budget.rs:114`).
- **scope / stage**: the subtree one question names; one height's pairing loop (`materialized.rs:8-10`); **`Scope`** the retained question (`adapter/scope.rs:12`).
- **query / reply / reaction / return**: the three item kinds (`materialized.rs:12-24`; `message.rs:159-170`).
- **resolution / `Resolve::Pending`**: the slot structure assembly fills (`materialized.rs:274-282`).
- **the walk / the proxy**: in-process vs wire-bound participant (`streaming.rs:4-8`).
- **initiator / responder**: roles elected by `initiates` (`message.rs:121-156`); **client / server**: handshake positions (`protocol/peer.rs:78-104`).
- **greeting / preamble / handshake**: distinguished at `message.rs:34-36`; `Preamble` at `handshake.rs:90`.
- **epilogue / `EPILOGUE_MARKER`**: the completion certificate (`gossip.rs:45-54`).
- **early supplies**: the initiator's exclusive root children shipped at greeting time (`remote.rs:47-54`).
- **window / liveness floor / `WindowConfig`**: per-height capacities from a byte budget; capacity one is the floor (`window.rs:1-21`, `DEFAULT_SYNC_MEMORY_BUDGET` at `:275`).
- **envelope**: an integer tail bound at 2⁻⁴⁸ (`window.rs:47-51`); **`SUPPLY_DECODE_ENVELOPE_BYTES`** (`window.rs:75`).
- **backend / materiality / `Local`**: what a node is and costs (`backend.rs:1-20`; `local.rs:92`).
- **erased**: height-erased runtime values with the prefix length as witness (`erased.rs:5-25`).
- **link / control stream / data stream / session epoch / poison**: `link.rs:9-22`, `346-361`.
- **routed link / token / router / `Endpoint`**: `link/routed.rs:15-22`.
- **bookmark / `NoBookmark` / `BOOKMARK_FORMAT_VERSION`**: `bookmark.rs:1-11`, `format.rs:8-13`.
- **`Intent` (Remain/Retire)**: the preamble's session-intent field, defined in `handshake` (imported at `gossip.rs:30`; matched at `gossip.rs:618-621`).
- **`Led`, `Gossip` cue, `Gossiped`, `SessionStats`/`Recorder`**: `gossip.rs:159-226`; `stats.rs:41,163`.
- **checkpoint**: an observer's resume point (`lib.rs:201-204`).
- **skeleton / wedge / `LocalEq`**: formal-bridge vocabulary (`streaming/tests/skeleton.rs:1-27`).

## 7. Public API surface (from `src/lib.rs:328-349`) [V]

Re-exports: `before` (crate), `Ticks`, `Version`, `causally`; `Batch`; `BOOKMARK_FORMAT_VERSION`, `Bookmark`, `BookmarkError`, `BookmarkIo`, `FormatError`, `FrameDefect`, `NoBookmark`, `RecordDefect`, `Serialized`; `Error`, `MirrorError`; `Acceptor`, `Connector`, `Link`; `EncodeError`; `Network`; `BookmarkedBootstrap`, `Bootstrap`, `DEFAULT_PAYLOAD_DEPTH_LIMIT`, `DEFAULT_SYNC_MEMORY_BUDGET`, `DEFAULT_TARGET_MESSAGE_SIZE`, `Gossip`, `Gossiped`, `Joined`, `Led`, `PayloadDepthLimit`, `Peer`, `Retire`, `Unbookmarked`; `Protocol`; `CausalMessages`, `Changes`, `Rumors`, `TryNext`, `TryTick`, `UnorderedMessages`; `Snapshot`; `MERKLE_HASH_LEN`; `SessionStats`.

Public modules: `error` (plus its re-exported taxonomy, `error.rs:38-51`), `link` (`Done`, `SessionState`, `LinkParts`, `MemoryLink`, `MemoryConnector`, `MemoryAcceptor`, `STREAM_COUNT`, `memory`, `memory_with_capacity`; `link::routed`: `Dial`, `Listen`, `Conn`, `Endpoint`, `Incoming`, `Config`, `RoutedLink`, `EndpointError`, `LinkError`, `LinkInfo`, `Addr`, `Token`, `MAX_ADDR_LEN`, `Unencodable`, `StreamAcceptor`, `StreamConnector`, `routed.rs:183-185`), `observe` (`Observer`, `SessionObserver`, `StreamObserver`, `SessionInfo`, `SessionKind`, `StreamInfo`, `StreamId`, `Direction`, `Role`), `reconciliation`, `tags` (`PARTY_TAG`, `VERSION_TAG`, `CLOCK_TAG`), `tutorial`; gated `conformance` (`conformance::link::check`) and `doc(hidden)` `testing`.

Key methods: `Peer::{seed, seed_rng (hidden), bootstrap, bookmark, retire, network, sync_memory_budget, sync_window_floor (gated), observe, target_message_size, payload_depth_limit, into_rumors, warm_caches (hidden)}`; `Bootstrap::{sync_memory_budget, target_message_size, payload_depth_limit, observe, bookmark, join}`; `Rumors::{send, redact, send_all, redact_all, batch, network, snapshot, unordered_messages(_since), causal_messages(_since), changes, try_into_peer, gossip, gossip_when}`; `Batch::{send, redact, send_all, redact_all}`; `Snapshot::{network, latest, earliest, is_empty, len, hash, get, iter, range}`. With the `.protocol()` builders gone (`README.md:5-8`), the uses of `Protocol` I found are `observe::SessionInfo` (`observe.rs:134`) and `Error::VersionMismatch` (`error.rs:14`); I did not sweep for others [R].
