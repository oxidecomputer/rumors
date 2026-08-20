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
//! Every message is stored at one address: the BLAKE3 hash of the
//! [`Version`](crate::Version) stamped on it at send time. Nothing else
//! enters the address; it rests on the invariant the protocol already
//! requires everywhere — no two sends ever share a version (a replica's
//! version advances on every send, and disjoint parties can never produce the
//! same one). Two consequences are deliberate.
//!
//! - **Every send is a distinct message.** Sending byte-identical content
//!   twice creates two versions, hence two leaves; redacting one never
//!   touches the other, and re-sending redacted content is a new message,
//!   neither resurrected nor suppressed by the redaction that came before
//!   it.
//! - **Message bytes enter no address and no digest.** The payload
//!   encoding needs no canonical form — one value may have many valid
//!   encodings without splitting identity — and no author of content can
//!   steer where anything lands, or what any digest reads, by choosing
//!   bytes. What that buys is stated under
//!   [Twenty-four-byte digests](#twenty-four-byte-digests).
//!
//! Version reuse — the only way two messages could claim one address —
//! cannot arise: every send creates a fresh version (a tick strictly above
//! everything the replica has ever held), and the linearity of parties
//! keeps replicas' versions disjoint. Producing a reused version at all
//! requires violating the linearity invariant the crate docs' safety
//! rules state, a regime that is already fatal to causal gossip.
//!
//! The 32-byte address is also the message's *location*: addresses are
//! the paths of a 256-ary radix trie, one byte per level, 32 levels deep,
//! with single-child runs compressed away. Hashing spreads paths
//! uniformly, and the trie's shape is a pure function of its membership:
//! two replicas holding the same set of messages hold the *same tree*,
//! whatever order they learned it in. Each interior node memoizes two
//! summaries of its subtree: a digest (a 24-byte truncation of BLAKE3;
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
//! the disputed node's own children (a radix byte and a digest each) and
//! the comparison recurses one level deeper. And where one side holds a
//! subtree the other holds nothing under, the
//! descent stops. Its destination is the *disjoint frontier*: the
//! horizontal cut across the tree where every node along the cut is the
//! highest node held exclusively by one side or the other.
//!
//! Without redaction, reconciliation would end there: ship each exclusive
//! subtree's messages to the side that lacks them (*supplies*), splice, and
//! both replicas hold the union. The work is proportional to the difference,
//! not to the holdings: uniform paths thin disputes geometrically with depth,
//! so the disputed paths of two replicas differing in `D` of `N` total
//! messages separate in about `log₂₅₆(2·D·N)` levels (in expectation,
//! derived from uniform version hashing) — about five levels for two
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
//! all. The only requirement to honor deletions is already present in the two
//! versions the greeting already exchanged.
//!
//! At each point on the disjoint frontier, the exclusive holder of a subtree
//! filters that subtree against the *lacking* side's top-level causal version.
//! This is the *causal sieve*:
//!
//! - A leaf whose version is **contained in** (`<=`) the lacking side's
//!   version records a send that side's history already covers: the lacking
//!   side has necessarily *seen* the message, but no longer holds it. Seen
//!   but absent means it must have been deleted — so the holder drops the
//!   leaf locally too, honoring a deletion it was never told about directly.
//! - A leaf concurrent with, or in the causal future of, the lacking
//!   side's version cannot have been seen there yet: the holder keeps it, and
//!   ships it to the side that lacks it.
//!
//! The seen-implies-deleted inference is sound because a replica's frontier
//! ([`Snapshot::latest`](crate::Snapshot::latest)) advances on every redaction
//! as well as every send: redaction is itself causal, so a redacter's version
//! contains the version of the since-redacted sent message, and every replica
//! that catches up to it inherits that containment. (A further and more
//! technical soundness note: this also only works because addresses name
//! causally-unique versions, which means that there are no A-B-A problems in
//! play.) The two versions exchanged in the greeting are sufficient causal
//! context to locally filter every subtree on the disjoint frontier: this means
//! we need no tombstones, no per-message deletion metadata, and no grace
//! windows, and a redacted message's cost everywhere falls to zero once the
//! redaction has propagated. It is also why a classic tombstone-system anxiety
//! does not translate: "what if the deletion arrives before the message?" is
//! unrepresentable — there is nothing to arrive.
//!
//! To optimize this filtration, the version bounds memoized on every interior
//! node decide whole subtrees at once: a subtree whose version ceiling the
//! counterparty's version contains is settled *even though its digest differs*
//! (nothing under it can be news to the counterparty; whatever it holds and the
//! counterparty lacks, the counterparty deleted), and a subtree whose version
//! floor is concurrent with or in the causal future of the counterparty's
//! version ships whole with no further comparison. Only mixed subtrees descend
//! toward individual leaves. This subtree-wise pruning is semantically
//! identical to sieving each leaf one by one; what it changes is the price,
//! keeping redaction-processing proportional to divergence rather than to the
//! size of the set redacted. Note that the pruning rule handles exactly the
//! plain Merkle repair's blind spot: a protocol comparing digests alone would
//! recurse into a dominated subtree and faithfully resurrect the counterparty's
//! deletions.
//!
//! The outcome of a session: both replicas hold every message either held and
//! neither had redacted when the session began.
//!
//! # Twenty-four-byte digests
//!
//! The digests the descent compares are 24-byte truncations of BLAKE3 (BLAKE3
//! is designed to be truncated: any prefix is itself a cryptographic hash).
//! Digest bytes dominate every dispute listing on the wire — they are the
//! protocol's main metadata price — so the width is spent deliberately;
//! leaf addresses in the tree remain full 32-byte hashes.
//!
//! The width prices a specific, severe failure. A false-equal — two
//! differing subtrees whose digests read equal at the same prefix — is not
//! a delay: the descent settles that comparison, neither side ships what
//! the other lacks, and the session still merges the two causal frontiers.
//! On every later comparison, each unshipped divergent message sits below
//! the frontier of a replica that does not hold it, so the causal sieve
//! reads it as seen-but-absent — the signature of a redaction — and the
//! holder deletes it. A landed false-equal permanently deletes the
//! divergent messages fleet-wide.
//!
//! The acceptance is priced on the accident bound: a digest at prefix `P`
//! is only ever compared against the counterparty's digest at the same
//! `P`, so a false-equal is a per-interior-comparison event at 2⁻¹⁹² —
//! pairwise, never birthday-amplified across the tree's population — and
//! at that bound it does not occur by accident at any realistic session
//! volume.
//!
//! Every compared digest is a pure function of the *version set*: a leaf's
//! digest commits its address and its version, a branch's commits its
//! children, and message bytes appear nowhere. An author of message
//! content therefore contributes zero bits to any compared quantity — the
//! offline content-grinding route to a collision is structurally gone, not
//! merely priced. What could still contribute bits is influence over which
//! versions get created (an actor steering gossip schedules steers the
//! version set); against any such actor, the 24-byte width keeps the
//! offline birthday floor at 2⁹⁶ evaluations, an unconditional bound that
//! rests on no premise about capabilities. Hostile *peers* remain
//! off-model entirely (see [when *shouldn't* you use
//! it](crate#when-shouldnt-you-use-it)): peers hold write authority
//! already, so no width buys anything against a member and none is priced
//! here.
//!
//! # The bytes on the wire
//!
//! A session's transport is a [`Link`](crate::Link): one persistent
//! bidirectional *control stream*, plus unidirectional *data streams*
//! opened lazily as the descent needs them, at most
//! [`STREAM_COUNT`](crate::link::STREAM_COUNT) per direction.
//!
//! The asymmetry is demand-shaped. The control stream carries what every
//! session unconditionally exchanges — the preamble, the greeting, and the
//! closing [`Error::Epilogue`](crate::Error::Epilogue) confirmation — so it is
//! the one stream worth holding open across sessions: those fixed phases
//! dominate a short session's latency, and re-dialing per session would put
//! transport dial time and its failure modes inside every one of them. The
//! persistent control stream is also what gives a link its stable identity
//! between sessions (its session counter and its poison state). Data streams
//! carry the descent into the tree, whose traffic exists only where divergence
//! does: a converged session opens none, so they are opened on demand and are
//! worth recycling back to a transport's connection pool. They are
//! unidirectional because that is all the protocol demands — demanding less of
//! the transport leaves implementations more room, and a link is free to split
//! one bidirectional stream into two unidirectional roles.
//!
//! In *theory*, the maximum number of streams needed on either side of the link
//! is 17, though in practice, far fewer will ever be needed. Why 17? A 32-byte
//! key gives the descent 32 levels; the schedule of traversal asks each side to
//! hop down the tree by 2 levels at a time, so at most 16 data streams plus a
//! control stream are ever needed.
//!
//! Streams are independently flow-controlled: one stream's backpressure is
//! invisible to every other stream, so the two levels sharing a stream
//! serialize against each other and levels on different streams do not. That
//! per-stream independence is *vital*: it grounds the protocol's
//! deadlock-freedom argument. With this to rely on, the session pipelines many
//! disputes at multiple levels concurrently, which permits maximal utilization
//! of the connection between two synchronizing peers, with neither peer ever
//! needing to twiddle its thumbs awaiting a message.
//!
//! # Memory: the budget and the window
//!
//! This pipelining is bought with memory: every dispute in flight holds decoded
//! tree state until its concomitant reply resolves it. Rather than let a very
//! divergent session's in-flight state grow indefinitely with the divergence,
//! each session derives a window (i.e. fixed per-tree-level capacities) from a
//! caller-set byte budget
//! ([`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget)) and from
//! both sides' exact set sizes and version-size bounds, so a statistical
//! worst-case memory utilization can be statically capped before the descent
//! begins, by enforcing the correct amount of backpressure. Divergence wider
//! than a level's assigned buffer capacity drains in capacity-sized waves: a
//! smaller budget costs increased latency, and any caller-set budget — down to
//! zero — leaves the session deadlock-free at one dispute in flight per level.
//! Message bodies are governed separately: supplies stream outside the window
//! as size-targeted runs, with at most one run in hand per stream per direction
//! ([`Peer::target_message_size`](crate::Peer::target_message_size)). The
//! budget's full details — what it prices, the closed form for choosing one,
//! and the measured trade-off table — lives at
//! [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget).
//!
//! # Two protocols
//!
//! [`Protocol`](crate::Protocol) names two wire dialects for the same
//! reconciliation: they process the same tree, based on the same logical
//! concepts of the disjoint frontier and its causal sieve.
//! [`Protocol::V1`](crate::Protocol::V1), the strictly alternating original
//! protocol, exchanges the dispute frontier a whole tree-level at a time: each
//! message carries an entire level's listings and supplies, built in full
//! before it ships. That shape is simple and hand-verifiably correct, but its
//! message size is unbounded; at high divergence a level's message grows with
//! the divergence itself, so a session can transiently hold a second copy of
//! much of the set, doubling the replica's memory footprint in the worst case.
//! [`Protocol::V2`](crate::Protocol::V2), the default, runs the descent using
//! the bounded-memory streaming approach described above. V1 remains selectable
//! (the `protocol-v1` cargo feature) as the simpler behavioral oracle the
//! streaming implementation is checked against.
