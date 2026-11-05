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
//! Every message is stored at one address: the SHA-256 hash of the
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
//!   [Sixteen-byte digests](#sixteen-byte-digests).
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
//! summaries of its subtree: a 16-byte digest (a truncation of SHA-256)
//! and the ceiling and floor of its leaves' versions. The digest answers
//! "do we hold the same things
//! here?"; the version bounds answer "could anything here be news to a
//! peer at that causal position?". The whole protocol is those two
//! questions, asked recursively.
//!
//! # The descent to the disjoint frontier
//!
//! After a fixed transport preamble, a session opens with a *greeting*:
//! each side sends its version, live-message count, version-size bound,
//! message-size target, payload depth limit, root's children, and the
//! number of its root's grandchildren. The peers must agree on the payload
//! depth limit before descent. Equal versions mean identical replicas, and
//! the session ends before a single digest is compared: one exchange and
//! no descent.
//!
//! Otherwise the two sides walk their trees downward together, level by
//! level, comparing children by digest. Equal digests prune: that subtree
//! is settled, however large it is. Where both sides hold something under a
//! child but the digests differ, the child is *disputed*: the reply lists
//! the disputed node's own children, each with its radix byte, its digest,
//! and the number of its own grandchildren, which bounds the reply a
//! question about it can draw, and the comparison recurses one level
//! deeper. And where one side holds a subtree the other holds nothing
//! under, the descent stops. Its destination is the *disjoint frontier*: the
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
//! # Sixteen-byte digests
//!
//! The digests the descent compares are 16-byte truncations of SHA-256
//! (truncating an approved hash is itself an approved hash of the shorter
//! width: NIST SP 800-107 Rev. 1, §5.1). SHA-256 is the hash because
//! current x86 and Arm processors compute it in hardware.
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
//! The acceptance is priced on the accident bound. A digest at prefix `P`
//! is only ever compared against the counterparty's digest at the same
//! `P`, so under the uniform-hash model a false-equal is a
//! per-comparison event at 2⁻¹²⁸, independent across comparisons:
//! pairwise, never birthday-amplified across the tree's population. Take
//! a deliberately absurd lifetime: a million peers, each running ten
//! thousand sessions a day of a thousand comparisons each, for a century.
//! That is about 2⁵⁸ comparisons, and the chance that any one of them is a
//! false-equal is about 2⁻⁷⁰.
//!
//! Every compared digest is a pure function of the *version set*: a leaf's
//! digest commits its address and its version, a branch's commits its
//! children, and message bytes appear nowhere. An author of message
//! content therefore contributes zero bits to any compared quantity, and
//! only authorized peers' clocks create versions. Hostile *peers* remain
//! off-model entirely (see [when *shouldn't* you use
//! it](crate#when-shouldnt-you-use-it)): peers hold write authority
//! already, so no width buys anything against a member, and the threat
//! model leaves the accident bound as the whole calculation.
//!
//! # The bytes on the wire
//!
//! A session runs over one connection, a reliable, ordered duplex byte
//! stream, which it takes as two halves (see [Transport](crate#transport)).
//! Each direction carries, in order, the preamble, the greeting, the
//! descent's frames, and the closing items: an identity donation when one
//! changes hands, and the [`Phase::Completion`](crate::error::Phase::Completion)
//! confirmation. A bootstrap adds one control item to each direction, right
//! after its preamble: the joiner's ephemeral public key, and the network's
//! secret its provider seals for that key.
//!
//! The descent's frames travel on *logical streams*, one per tree level a
//! side answers at. Each frame names its logical stream, so the one
//! connection carries them all, their frames interleaved. A logical stream
//! carries frames only where divergence does: a converged session sends no
//! frame at all.
//!
//! Sharing one connection among levels could let a full queue at one level
//! stop delivery at another, which would break the walk's deadlock freedom.
//! A receiver prevents that by reading every frame as it arrives and holding
//! each reply until its walk takes it. A reply exists only in answer to the
//! receiver's own question, and the receiver set aside room for each reply
//! when it asked, so reading never waits on the walk.
//! Levels still pipeline across the connection, and whenever a sender has a
//! choice, a frame carrying the descent goes out ahead of supply runs
//! waiting at other levels.
//!
//! # Memory: the budget
//!
//! Each question in flight holds tree state, and room for its answer, until
//! its answer arrives and is taken. To keep that state bounded, each side
//! admits each question and answer it builds against a caller-set byte
//! budget ([`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget)),
//! counting what it holds and what it has set aside; work that does not fit
//! waits for earlier work to finish. Each level always has room for one
//! question and one answer, which the budget sets aside before it admits
//! anything else, so a smaller budget adds latency and never stops a
//! session: a zero budget asks one question at a time per level and remains
//! deadlock-free.
//! Message bodies are governed separately: supplies stream as
//! size-targeted runs, with at most four runs buffered per level sending,
//! and three per level receiving plus one more for the whole receiving
//! direction ([`Peer::target_message_size`](crate::Peer::target_message_size)).
//! The budget's contract is at
//! [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget);
//! [the sizing guide](crate::sizing) explains how to choose it.
//!
//! # Why streaming, not level-synchronous exchange
//!
//! The straightforward wire shape for this reconciliation exchanges the
//! dispute frontier a whole tree-level at a time: each message carries an
//! entire level's listings and supplies, built in full before it ships.
//! That shape is simple and hand-verifiably correct, but its message size
//! is unbounded; at high divergence a level's message grows with the
//! divergence itself, so a session can transiently hold a second copy of
//! much of the set, doubling the replica's memory footprint in the worst
//! case. The streaming descent above bounds that transient state. Tests
//! compare its result with the same in-memory reconciliation semantics across
//! generated trees, including redactions.
