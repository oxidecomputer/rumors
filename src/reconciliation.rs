//! How two replicas reconcile: the tree, the descent, and the causal sieve.
//!
//! This page explains the protocol behind [`gossip`](crate::Rumors::gossip)
//! forward — from what a replica stores to what crosses the wire — and ends
//! with how the design compares to its neighbors. Nothing here is required
//! to use the crate: the contracts live on the API surfaces and this page
//! carries the why behind them. For a doing-first introduction instead,
//! start with the [tutorial](crate::tutorial).
//!
//! # One hash binds identity, causality, and placement
//!
//! Every message is stored under a [`Key`](crate::Key): the BLAKE3 hash
//! binding the [`Version`](crate::Version) at which the message was sent to
//! the message's canonical [`borsh`] encoding. Both inputs are
//! deliberate.
//!
//! - **The version makes every send unique.** A replica's version advances
//!   on every send, so each send mints a key that no other send in the
//!   universe's history can mint again. Sending byte-identical content
//!   twice creates two messages under two keys; redacting one never touches
//!   the other; re-sending redacted content is a new message under a new
//!   key, neither resurrected nor suppressed by the redaction that came
//!   before it.
//! - **The content makes the address canonical.** A content address is only
//!   an address if one value has exactly one encoding. Borsh guarantees
//!   that by construction; serialization frameworks in general do not.
//!
//! The 32-byte key is also the message's *location*: keys are the paths of
//! a 256-ary radix trie, one key byte per level, 32 levels deep, with
//! single-child runs compressed away. Hashing spreads keys uniformly, and
//! the trie's shape is a pure function of its membership: two replicas
//! holding the same set of messages hold the *same tree*, whatever order
//! they learned it in. Each interior node memoizes two summaries of its
//! subtree: a digest (a 16-byte truncation of BLAKE3;
//! [`MERKLE_HASH_LEN`](crate::MERKLE_HASH_LEN)) and the ceiling and floor
//! of its leaves' versions. The digest answers "do we hold the same things
//! here?"; the version bounds answer "could anything here be news to a
//! peer at that causal position?". The whole protocol is those two
//! questions, asked recursively.
//!
//! # The descent to the disjoint frontier
//!
//! After a fixed transport preamble, a session opens with a *greeting*:
//! each side sends its version, its live-message count, its version-size
//! bound, its message-size target, and its root's child listing. Equal versions mean identical
//! replicas, and the session ends right there, before a single digest is
//! compared: the cheapest possible session, one exchange and no descent.
//!
//! Otherwise the two sides walk their trees downward together, level by
//! level, comparing children by digest. Equal digests prune: that subtree
//! is settled, however large it is. Where both sides hold something under a
//! child but the digests differ, the child is *disputed*: the reply lists
//! the disputed node's own children (a radix byte and a digest each, 17
//! bytes per child) and the comparison recurses one level deeper. And
//! where one side holds a subtree the other holds nothing under, the
//! descent stops. Its destination is the *disjoint frontier*: the
//! horizontal cut across the tree where every node along the cut is the
//! highest node held exclusively by one side or the other.
//!
//! Without redaction, reconciliation would end there: ship each exclusive
//! subtree's messages to the side that lacks them (*supplies*), splice, and
//! both replicas hold the union. The work is proportional to the difference,
//! not to the holdings: uniform keys thin disputes geometrically with depth,
//! so the disputed paths of two replicas differing in `D` of `N` total
//! messages separate in about `log₂₅₆(2·D·N)` levels (in expectation,
//! derived from uniform content addressing) — about five levels for two
//! fully divergent million-message replicas, three or four when a small
//! divergence sits in a large set. Round trips are governed by that depth,
//! not by `D`: every dispute at a level travels concurrently (the wire
//! shape below), so latency is paid per level. A bootstrap — everything on
//! one side, nothing on the other — needs no descent at all: the whole set
//! ships as supplies from the root, `O(1)` rounds.
//!
//! # The causal sieve
//!
//! Redaction is the twist. When a peer [`redact`](crate::Rumors::redact)s a
//! message, no record of the deletion is stored and none crosses the wire:
//! there is no tombstone, and no redaction object exists in the protocol at
//! all. Deletion honoring rides entirely on the two versions the greeting
//! already exchanged.
//!
//! At each frontier point, the holder filters its exclusive subtree leaf by
//! leaf against the *lacking* side's greeted version. This is the *causal
//! sieve*:
//!
//! - A leaf whose version is **contained in** (`<=`) the lacking side's
//!   version records a send that side's history already covers: the lacking
//!   side has necessarily *seen* the message. Seen but absent means
//!   deleted — so the holder drops the leaf locally too, honoring a
//!   deletion it was never told about.
//! - A leaf concurrent with, or in the causal future of, the lacking
//!   side's version cannot have been seen there: the holder keeps it, and
//!   ships it.
//!
//! The seen-implies-deleted inference is sound because a replica's frontier
//! ([`Snapshot::latest`](crate::Snapshot::latest)) advances on every
//! redaction as well as every send: deleting is itself causal history, so a
//! deleter's version contains the deleted send, and every replica that
//! catches up to the deleter inherits that containment. The two greeted
//! versions are sufficient causal context to filter every subtree on the
//! disjoint frontier — which is why no tombstones, no per-message deletion
//! metadata, and no grace windows exist anywhere in the system, and why a
//! redacted message's cost everywhere falls to zero once the redaction has
//! propagated. It is also why a classic tombstone-system anxiety does not
//! translate: "what if the deletion arrives before the message?" is
//! unrepresentable — there is nothing to arrive.
//!
//! The semantics are leaf-wise; the evaluation is not. The version bounds
//! memoized on every interior node decide whole subtrees at once — a
//! subtree whose version ceiling the counterparty's version contains is
//! settled *even though its digest differs* (nothing under it can be news
//! to the counterparty; whatever it holds and the counterparty lacks, the
//! counterparty deleted), and a subtree whose version floor is concurrent
//! with or in the causal future of the counterparty's version ships whole
//! with no further comparison. Only mixed subtrees
//! descend toward individual leaves. This subtree-wise pruning is
//! semantically identical to sieving each leaf one by one; what it changes
//! is the price, keeping deletion honoring divergence-proportional rather
//! than holdings-proportional. Note the pruning rule is exactly the plain
//! Merkle repair's blind spot: a protocol comparing digests alone would
//! recurse into a dominated subtree and faithfully resurrect the
//! counterparty's deletions.
//!
//! The outcome of a session, stated once: both replicas hold every message
//! either one held when the session began and neither had deleted.
//!
//! # Sixteen-byte digests
//!
//! The digests the descent compares are 16-byte truncations of BLAKE3
//! (BLAKE3 is designed to be truncated: any prefix is itself a
//! cryptographic hash). Digest bytes dominate every dispute listing, so
//! halving them roughly halves the descent's metadata; keys — and with
//! them leaf placement — remain full 32-byte hashes.
//!
//! What would a digest collision cost, and who could make one? Peers are
//! trusted in this crate's model (see [when *shouldn't* you use
//! it](crate#when-shouldnt-you-use-it)), and a compromised peer never
//! needs a collision: it already holds write authority over the set. The
//! residual party is an author of message *content* who is not a peer.
//! Such an author controls the content bytes but never the version half of
//! the hash input: every send is stamped with a fresh causal version whose
//! value under concurrent gossip is unpredictable, so no colliding
//! (version, content) pair can be assembled ahead of time. The version
//! stamp denies precomputation, and mining a collision online — against
//! digests that exist only once their versions are already stamped — is
//! infeasible at 128 bits.
//! Were a collision somehow landed, its blast radius is bounded: two
//! differing subtrees falsely read equal, so a message silently fails to
//! propagate across that comparison — transient shadowing that ends when
//! causal movement re-opens that part of the tree — never corruption of
//! what replicas hold.
//!
//! # The shape on the wire
//!
//! A session's transport is a [`Link`](crate::Link): one persistent
//! bidirectional *control stream*, plus unidirectional *data streams*
//! opened lazily as the descent needs them, at most
//! [`STREAM_COUNT`](crate::link::STREAM_COUNT) per direction.
//!
//! The asymmetry is demand-shaped. The control stream carries what every
//! session unconditionally exchanges — the preamble, the greeting, and the
//! closing confirmation ([`Error::Epilogue`](crate::Error::Epilogue)
//! documents the one residue that close cannot remove) — so it is the one
//! stream worth holding open across sessions: those fixed phases dominate a
//! short session's latency, and re-dialing per session would put transport
//! dial time and its failure modes inside every one of them. The persistent
//! control stream is also what gives a link its stable identity between
//! sessions — its session counter and its poison state. Data streams carry
//! the descent, whose traffic exists only where divergence does: a
//! converged session opens none, so they are opened on demand and are worth
//! recycling back to a transport's connection pool. They are unidirectional
//! because that is all the protocol demands — demanding less of the
//! transport leaves implementations more room, and a link is free to split
//! one bidirectional stream into two unidirectional roles.
//!
//! Seventeen is the wire schedule's own number. A 32-byte key gives the
//! descent 32 levels; the schedule assigns each data stream two adjacent
//! levels; and one more stream carries the opening supplies — the root
//! subtrees the crossed greeting listings already prove exclusive, shipped
//! a hop before they could be asked for. `⌈32 / 2⌉ + 1 = 17`.
//!
//! Streams are independently flow-controlled: one stream's backpressure is
//! invisible to every other stream, so the two levels sharing a stream
//! serialize against each other and levels on different streams do not.
//! That per-stream independence is the load-bearing clause of the
//! [`link`](crate::link) contract and the ground the protocol's
//! deadlock-freedom argument stands on; within it, the session pipelines
//! every dispute at a level concurrently, which is what buys the
//! pay-per-level latency above.
//!
//! # Memory: the budget and the window
//!
//! Pipelining is bought with memory: every dispute in flight holds decoded
//! tree state until its reply resolves it. Rather than let a very divergent
//! session's in-flight state grow with the divergence, each session derives
//! a window — fixed per-level capacities — from a caller-set byte budget
//! ([`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget)) and from
//! what the greeting put on the table (both sides' exact set sizes and
//! version-size bounds), so every input to the worst case is known before
//! the descent begins. Divergence wider than a level's capacity drains in
//! capacity-sized waves: a smaller budget costs latency, never correctness,
//! and any budget — down to zero — leaves the session deadlock-free at one
//! dispute in flight per level. Message bodies are governed separately:
//! supplies stream outside the window as size-targeted runs, with at
//! most one run in hand per stream per direction
//! ([`Peer::target_message_size`](crate::Peer::target_message_size)). The
//! budget's full contract — what it prices, the closed form for choosing
//! one, and the measured trade-off table — lives at
//! [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget).
//!
//! # Two protocols
//!
//! [`Protocol`](crate::Protocol) names two wire dialects for the same
//! reconciliation — same tree, same frontier, same sieve.
//! [`Protocol::V1`](crate::Protocol::V1), the strictly alternating
//! original, exchanges the dispute frontier a whole level at a time: each
//! message carries an entire level's listings and supplies, built in full
//! before it ships. That shape is simple, but its message size is
//! unbounded — at high divergence a level's message grows with the
//! divergence itself, so a session can transiently hold a second copy of
//! much of the set, doubling the replica's memory footprint at exactly the
//! moment (a large catch-up) it can least afford to.
//! [`Protocol::V2`](crate::Protocol::V2), the default, runs the descent as
//! fixed-memory streaming: disputes and supplies flow through the windowed
//! pipeline above, so a session's memory is set by the budget, not by the
//! divergence. V1 remains selectable (the `protocol-v1` cargo feature) for
//! wire compatibility with V1 peers and as the simpler behavioral oracle
//! the streaming implementation is checked against.
//!
//! # How it compares
//!
//! `rumors` is an *augmented Merkle repair*: Merkle set reconciliation over
//! a content-addressed store whose every node is annotated with an interval
//! tree clock version, under which the store forms a join-semilattice. Each
//! ingredient has honest prior art; this section places the design among
//! its neighbors — what it takes from each, and what each still does
//! better. In one line: Dynamo-style Merkle repair made exact and
//! tombstone-free by fusing it with an ORSWOT-style causal-dominance rule —
//! carried per subtree, not per element — over ITC versions, tuned to spend
//! bandwidth to buy round trips.
//!
//! ## Epidemic anti-entropy
//!
//! The founding work is [*Epidemic Algorithms for Replicated Database
//! Maintenance*](https://dl.acm.org/doi/10.1145/41840.41841) (Demers et
//! al., PODC 1987): replicated databases converge by pairwise anti-entropy
//! and rumor mongering, analyzed as epidemics. It is also the origin of
//! this crate's central problem. Demers et al. observe that deletion
//! cannot be represented by absence and introduce *death certificates* —
//! tombstones — plus a dormancy scheme to bound their retention, trading
//! resurrection risk against storage. `rumors` is a direct descendant in
//! framing (its anti-entropy is the whole protocol; there is no separate
//! rumor-mongering channel), and its redaction design resolves that
//! paper's open trade: absence *plus causal dominance* is a sound
//! representation of deletion, so no certificate is ever stored and
//! nothing retires on a timer. The loss case: Demers et al. engineered for
//! thin, expensive, lossy links, with spatial distributions to cut
//! cross-link traffic; `rumors` deliberately buys latency with bandwidth
//! and is the wrong protocol on metered or narrow links — [the crate
//! docs](crate#when-shouldnt-you-use-it) say so.
//!
//! ## Merkle repair in production stores
//!
//! [Dynamo](https://dl.acm.org/doi/10.1145/1294261.1294281) (DeCandia et
//! al., SOSP 2007) introduced per-key-range Merkle trees so replicas
//! compare digests and transfer only divergent ranges;
//! [Cassandra](https://docs.datastax.com/en/dse/6.8/architecture/database-architecture/anti-entropy-repair.html)
//! builds a fixed-depth hash tree over each partition range on demand, and
//! [Riak's active anti-entropy](https://docs.riak.com/riak/kv/latest/learn/concepts/active-anti-entropy/index.html)
//! maintains persistent hash trees updated on write. This is the closest
//! operational ancestor: hash-compare down, transfer at the frontier.
//! `rumors` gains three things in its regime. The tree *is* the store —
//! persistent, incrementally memoized, never rebuilt by scanning (Riak
//! approaches this; Cassandra pays a full-scan validation per repair).
//! Resolution is exact — one leaf per message, so repair ships exactly the
//! differing messages rather than streaming every range that disagrees.
//! And, decisively: digest-only repair cannot distinguish "you're missing
//! this" from "you deleted this", so these systems all carry tombstones
//! with garbage-collection grace windows and the resurrection hazard of a
//! replica that sleeps past its window; the causal sieve eliminates that
//! category. The loss cases: those systems shard — the tree covers a key
//! range, replicas hold partitions, datasets are disk-resident and
//! effectively unbounded — where every `rumors` peer holds the full set in
//! memory; they repair *mutable* keyed data where `rumors` replicates
//! immutable messages; and their per-repair metadata is constant-size
//! regardless of item count, while `rumors` keeps per-message tree state
//! resident.
//!
//! ## Canonical-shape search trees
//!
//! [Merkle Search Trees](https://inria.hal.science/hal-02303490) (Auvolat
//! & Taïani, SRDS 2019) are deterministically shaped search trees — an
//! item's layer derives from its hash, so structure is a pure function of
//! content — supporting Merkle anti-entropy over *ordered* keys; Bluesky's
//! [AT Protocol repository](https://atproto.com/specs/repository) is an
//! MST in production, and [Prolly
//! trees](https://www.dolthub.com/docs/architecture/storage-engine/prolly-tree)
//! (Noms/Dolt) get the same canonical-shape property from content-defined
//! chunking. `rumors`' trie shares the headline property — equal content
//! gives equal shape and equal digests on every replica, insert-order
//! independent. The difference is what the key order buys. An MST
//! preserves *application* key order, so it serves range scans and
//! tolerates non-uniform, even adversarial, key distributions; `rumors`
//! spends the key space entirely on uniformity — the path *is* the hash —
//! which makes its depth and occupancy statistics theorems rather than
//! expectations, and recovers range queries on the one axis it cares
//! about (causality) from the version-bound memos instead
//! ([`Snapshot::range`](crate::Snapshot::range)). What `rumors` adds in
//! its regime is the deletion story — MST anti-entropy alone reconciles
//! toward union and re-learns deletions; the MST paper pairs the tree
//! with CRDT value types and inherits whatever deletion metadata those
//! carry — and a shipped wire protocol: a pipelined, fixed-memory,
//! deadlock-argued session where the paper specifies a structure and a
//! compare-and-descend outline. The loss cases: ordered keys (syncing
//! "records in this collection" or "events in this time range"),
//! self-certification in open networks (exactly the untrusted regime
//! `rumors` declares off-model), and attacker-chosen keys; Prolly trees
//! additionally offer structural diff and merge for versioned data, a
//! different product than a gossip substrate.
//!
//! ## Range-based set reconciliation
//!
//! [Range-Based Set Reconciliation](https://arxiv.org/abs/2212.13567)
//! (Meyer, 2023) reconciles two sets over a total order: peers exchange
//! fingerprints of ranges, split ranges that disagree, and recurse until
//! disjoint items ship — `O(D·log N)` communication over `O(log N)`
//! sequential rounds with *no per-item auxiliary state*, everything
//! computed from a sorted index at sync time;
//! [negentropy](https://github.com/hoytech/negentropy) is the deployed
//! instantiation. This is the same divide-and-conquer skeleton as the
//! descent with the tree made implicit, and it is the honest
//! bandwidth-side benchmark: RBSR's metadata is leaner (no 256-way child
//! listings), it needs no resident tree, it works off any ordered
//! storage, and its total order lets a caller reconcile a *subrange* —
//! none of which `rumors` offers. Where `rumors` wins in its regime:
//! latency discipline (RBSR's rounds are inherently sequential per range
//! lineage, while `rumors` pipelines an entire level's disputes per hop
//! under a derived memory bound); incremental maintenance (memoized
//! digests make session compute frontier-proportional, with no full-index
//! fingerprint pass); and deletion (RBSR converges on union, full stop —
//! deleting through it requires tombstones in the item set or
//! application-level deletion records). Fingerprint soundness is also
//! structurally easier here: RBSR's composable range fingerprints must
//! resist crafted collisions in the homomorphic combine, where `rumors`
//! compares plain truncated BLAKE3 at fixed tree positions under a
//! trusted-peer model.
//!
//! ## Sketch-based reconciliation
//!
//! Invertible Bloom lookup tables encode a set into a sketch sized to the
//! *difference*: exchange sketches, subtract, peel out exactly the
//! differing items — `O(D)` communication independent of `N`, in one
//! round trip ([Eppstein et al., SIGCOMM
//! 2011](https://dl.acm.org/doi/10.1145/2018436.2018462)); [Rateless
//! IBLT](https://arxiv.org/abs/2402.02668) (Yang, Gilad & Alizadeh,
//! SIGCOMM 2024) removes the difference-estimation problem with a coded
//! symbol stream the receiver consumes until decoding succeeds. In pure
//! information-theoretic terms this family wins the bandwidth and
//! round-trip race outright — for *finding the difference*, `rumors` is
//! not competitive on bytes, and knows it. Its claim is different.
//! Semantics: a sketch yields the symmetric difference as items, and the
//! protocol still cannot say whether an item the peer lacks should be
//! sent or was deleted there — the deletion problem returns untouched.
//! Compute shape: sketch encoding touches every element (or maintains a
//! mutable sketch, which then *is* per-item auxiliary state) and peeling
//! is CPU-heavy at scale, where `rumors`' memoized tree keeps session
//! compute proportional to the frontier. Payloads: sketches reconcile
//! fixed-width identifiers, so message bodies need a retrieval round.
//! And determinism: decoding is probabilistic; the descent is
//! deterministic given the trees. Rateless IBLT is the right choice on
//! thin links, unknown divergence, and untrusted counterparties;
//! `rumors`' regime — fat trusted links, latency-bound, deletion-heavy —
//! is close to its complement.
//!
//! ## Tombstone-free deletion in CRDTs
//!
//! The nearest *semantic* neighbor is the [optimized
//! OR-set](https://arxiv.org/abs/1210.3368) (Bieniusa et al., 2012),
//! deployed as Riak's ORSWOT: elements carry birth dots, the replica
//! carries a version vector, and on merge an element present on one side
//! whose dot the other side's vector dominates is dropped — removal by
//! causal dominance, no tombstones. The causal sieve is recognizably this
//! inference. The lift is twofold. First, the "dot" is the message's send
//! version, which is simultaneously half the key's hash input — identity,
//! causality, and placement are one commitment rather than parallel
//! bookkeeping. Second, and substantively: ORSWOT states the rule
//! element-wise over full states or delta intervals, while `rumors` pushes
//! it into the reconciliation tree as per-subtree version bounds, so
//! dominance prunes whole subtrees mid-session and the deletion filter
//! *is* the sync protocol's pruning rule. On the adjacent path, [causal
//! stability](https://arxiv.org/abs/1710.04469) (Baquero, Almeida &
//! Shoker) discards operation metadata once causally stable — but needs
//! stability tracked across the whole membership, which churn stalls;
//! `rumors` has nothing retained to discard. The loss cases, stated
//! plainly: OR-set-family types support *concurrent add and remove of the
//! same element* with add-wins arbitration, which `rumors` sidesteps
//! rather than solves (messages are immutable and uniquely keyed, so the
//! race cannot arise — but neither can "re-add the same element"
//! semantics); and tombstone-carrying designs (Yjs's integrated
//! tombstones, Automerge's retained operation history) buy richer types —
//! sequences, maps, text — plus partial replication, history, and
//! auditability in untrusted settings. `rumors` buys zero-residue
//! deletion by restricting the data model to an unordered set of
//! immutable messages.
//!
//! ## The version algebra
//!
//! Version vectors need one entry per actor forever: under dynamic
//! membership they either leak entries or need out-of-band retirement.
//! [Dotted version vectors](https://arxiv.org/abs/1011.5808) (Preguiça,
//! Baquero et al.) solve a different problem — precise per-write dots
//! against sibling explosion — and still assume a stable actor-id space.
//! [Interval tree clocks](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf)
//! (Almeida, Baquero & Fonte, 2008) make the identity space itself a
//! lattice: identities fork and join, membership change is a local
//! operation, and a retired identity is *recycled*, not leaked. `rumors`
//! (via [`before`](crate::before)) builds this into its product surface —
//! [`seed`](crate::Peer::seed), [`bootstrap`](crate::Peer::bootstrap), and
//! [`retire`](crate::Peer::retire) *are* ITC fork and join — and every
//! message version is an ITC stamp, which is also what keeps the greeted
//! causal context compact under membership churn. The loss cases: ITC
//! identities are structural, not nameable — "what did replica R write?"
//! needs a registry outside the clock; version sizes degrade under
//! sustained fork/join churn ([`before`](crate::before)'s docs put a
//! version at ~2 KB at equilibrium churn across 100 parties, roughly
//! `N²`, against a version vector's predictable `N` entries); linearity
//! of identities is a sharp global safety invariant (see [the crate
//! docs](crate)) that vector-clock systems never have to state; and a
//! peer that crashes without retiring strands an identity interval,
//! widening stamps slightly forever, where a version-vector entry can be
//! garbage-collected by fiat.
//!
//! ## What is new here
//!
//! Every ingredient above has prior art; the combination is the claim. We
//! know of no prior system that reconciles a content-addressed Merkle
//! structure under a causal-dominance sieve: annotating every node of a
//! canonical set index with the lattice ceiling and floor of its
//! subtree's versions, so that the descent prunes simultaneously by hash
//! equality and by causal dominance, and deletion propagates with zero
//! stored deletion state — the two session versions are the entire causal
//! context, and the deletion filter is literally the pruning rule. ORSWOT
//! has the sieve without the tree; MSTs, Prolly trees, and RBSR have the
//! tree or the recursion without causal annotation, so they converge on
//! union; [Merkle-CRDTs](https://arxiv.org/abs/2004.00107) (Sanjuán et
//! al., 2020) put CRDTs in a Merkle DAG, but as a grow-only causal
//! history whose removals need tombstone-bearing value types — nearly the
//! dual of the move made here; and the production Merkle repairs carry
//! tombstones and grace windows. The choice of ITCs as the version
//! algebra compounds the claim only mildly — any causal lattice would
//! support the sieve — but it is load-bearing for the membership story
//! and for keeping the greeting small under churn.
