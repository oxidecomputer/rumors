//! About `rumors`: how it works, what it is, and what it isn't.
//!
//! This page is for reflection rather than action. If you're after a
//! guided lesson see [`crate::tutorial`]; if you want a recipe for a
//! specific task, see [`crate::guide`]. The [crate root](crate) is the
//! API reference. What follows is the conceptual picture that ties the
//! three together.
//!
//! # What `rumors` is
//!
//! `rumors` is a *stateless pairwise reconciler* for a CRDT-backed
//! gossip set. Given a connected reader/writer to one peer, it brings
//! both sides into agreement on which messages are live in the rumor
//! set, then returns. The internal protocol is a finite mirror exchange
//! between two replicas of a [versioned trie](crate::Local); there is
//! no persistent connection state, no session identifier, and no
//! background activity. A peer is just a [`Local<T>`](crate::Local) and
//! a way to talk to other peers.
//!
//! What it gives you is **unordered, at-most-once, eventually-consistent
//! delivery** of every message that any participant has originated and
//! not redacted. Each newly-observed message is surfaced exactly once
//! through the `on_message` callback during
//! [`process`](crate::Local::process) into the
//! [`Original`](crate::Original) for the party.
//!
//! # What `rumors` is not
//!
//! `rumors` is *not* a complete gossip protocol. It is meant to be
//! embedded inside a larger system that supplies the pieces it
//! deliberately omits:
//!
//! - **Not peer discovery.** You decide who talks to whom and on what
//!   schedule. The library has no notion of membership, no heartbeat,
//!   no failure detector.
//! - **Not a spanning-tree maintainer.** Gossip topology is the caller's
//!   problem. The reconciler converges any pair that talks; reaching
//!   eventual consistency across the whole network requires that the
//!   pairs you choose to reconcile form a connected graph over time.
//! - **Not a transport.** You hand it an
//!   [`AsyncRead`](tokio::io::AsyncRead) and an
//!   [`AsyncWrite`](tokio::io::AsyncWrite) (or the sync equivalents);
//!   TLS, framing, congestion control, and reconnect logic live above.
//! - **Not a causal-delivery primitive.** The `on_message` callbacks
//!   are emitted in arbitrary order, *including* orderings that violate
//!   the causal precedence captured by the [`Version`](crate::Version)
//!   threaded through them. A reply can arrive before the question it
//!   replies to. If your application needs causal or total ordering,
//!   build it on top: every callback carries the `Version` you need to
//!   sort by.
//! - **Not Byzantine-resistant.** Every peer is trusted to follow the
//!   protocol and to keep its party identifier unique across the whole
//!   network for all time. A malicious or buggy peer that reuses an
//!   identifier or fabricates version vectors can corrupt the rumor set
//!   contagiously; see [`Local::for_party`](crate::Local::for_party).
//!
//! If those omissions are showstoppers for you, `rumors` is the wrong
//! library. If they read as features — "yes, I want to bring my own
//! membership and transport and have a small, auditable reconciler in
//! the middle" — read on.
//!
//! # The mental model
//!
//! Every peer holds a *versioned set* of rumors. Internally, that set
//! is a sparse Merkle trie of branching factor 256 and depth 32, with
//! messages addressed by their Blake3 hash. Each leaf is tagged with
//! the [`Version`](crate::Version) at which the local replica observed
//! the message — a per-party event-count vector — and each interior
//! node carries a hash that summarises everything beneath it. A
//! redaction *removes* the leaf outright; nothing is kept in its place
//! (no tombstone), so the redactor's tree is genuinely smaller after a
//! redaction.
//!
//! The reconciler is a *mirror protocol*: it walks both peers' tries in
//! lockstep, descending only into subtrees whose root hashes disagree.
//! At each level one side names the children it has, the other replies
//! with the children it lacks and the children it can fill in, and
//! they recurse into the disagreeing subset. When every disagreement
//! has been resolved, the session terminates from within the protocol
//! itself — there is no out-of-band shutdown signal.
//!
//! # Why a 256-ary trie of depth 32?
//!
//! The shape is chosen deliberately for *bandwidth-plentiful,
//! latency-bottlenecked* environments — datacenter and intra-rack
//! links. Branching factor 256 discriminates one byte of the message
//! hash per level, so the trie is at most 32 levels deep and most real
//! diffs resolve in one or two round trips: typically the topmost
//! disagreement is many levels above the leaves, and one descent narrows
//! the search space by a factor of 256.
//!
//! The price is wider per-level messages: a single trie node enumerates
//! up to 256 child hashes (32 bytes each). For a workload where a small
//! diff might fit in one packet, this is wasteful. We trade that
//! bandwidth for round trips deliberately; if your workload inverts the
//! ratio — high-latency-tolerant but bandwidth-starved — `rumors` is
//! the wrong tool. A binary trie of depth 256 would shrink each message
//! by a factor of ~128 but multiply the round-trip count by ~8.
//!
//! # Redaction without tombstones
//!
//! Redactions are propagated by the same mirror protocol that
//! propagates inserts, but they do not leave tombstones. The redactor's
//! tree truly loses the leaf, and a redacted key occupies no space on
//! any peer that has caught up.
//!
//! The mechanism is *deletion inference via version vectors*. Every
//! action (insert or redact) bumps the local party's component of the
//! version vector by one; the new version is carried on whichever leaf
//! the action touched, and the per-party event count for the redactor
//! after a redact is strictly greater than it was at the moment of the
//! original insert. When two peers reconcile, the mirror protocol
//! compares version vectors and treats the side with the higher version
//! at a given position as authoritative — including authoritatively
//! *absent*. A peer with a leaf that another peer has redacted observes
//! that the other side's version dominates and that the other side has
//! no leaf there, and infers a deletion. This is what is meant by
//! "deletion inference": the wire carries no explicit delete record;
//! the absence of a leaf at a higher version *is* the delete.
//!
//! This gives redactions two specific advantages:
//!
//! - **Bounded memory.** A long-lived network does not accumulate
//!   redaction history. The cost of a redact is one event-counter bump
//!   per party, the same as a regular insert; the freed leaf is gone
//!   for good.
//! - **Bandwidth honest to the live set.** Reconciling two peers that
//!   have both seen the same redactions costs nothing for those
//!   redactions — there is nothing left for them to disagree about.
//!
//! Redaction is *contagious* by the same logic: there is no quorum, no
//! voting, no coordinator. One peer's local decision evicts the message
//! network-wide as long as the gossip graph is eventually connected.
//! The price is that *anyone* can issue the redaction — there is no
//! built-in authorization. Building access control around `redact` is
//! the caller's problem.
//!
//! # Delivery guarantees
//!
//! - **At-most-once.** Each `on_message` callback fires exactly once
//!   per message per local replica, the first (and only) time that
//!   replica observes the message.
//! - **Eventually consistent.** Two peers that gossip with each other
//!   (directly or transitively) converge on the same live set of
//!   messages once the gossip graph quiesces. There is no upper bound
//!   on convergence time.
//! - **Unordered.** Callback firing order within a single
//!   [`process`](crate::Local::process) or [`gossip`](crate::Local::gossip)
//!   call is unspecified; across calls it is also unspecified.
//!   Crucially, this includes pairs of messages that are causally
//!   ordered under their [`Version`](crate::Version)s. The version
//!   vector is *recorded*, not *enforced*: the protocol guarantees you
//!   eventually see every live message, but it tells you nothing about
//!   the order in which you will see them.
//!
//! # Boundaries and risks
//!
//! - **Liveness under partition.** During a partition, peers on
//!   different sides cannot converge. The library does not attempt to
//!   detect or recover; it relies on the caller's transport to
//!   eventually reconnect. Messages originated during the partition are
//!   simply delivered late, in arbitrary order.
//! - **Party identifier uniqueness.** The strongest invariant the
//!   library asks of the caller is that each party identifier is held
//!   by at most one process at a time, and that successive
//!   re-instantiations of the same party identifier monotonically
//!   advance the local event counter. Violating either causes silent,
//!   contagious corruption of the rumor set across the whole network.
//!   The runtime check in [`Local::for_party`](crate::Local::for_party)
//!   enforces uniqueness only within the current process; the
//!   cross-network half is on you.
//! - **Memory growth.** Redactions are free — no tombstones — but the
//!   version vector itself grows with the number of distinct parties
//!   ever observed. The library has no garbage-collection policy for
//!   retired parties; long-running networks with churn in party
//!   identifiers will accumulate version-vector entries linearly.
//!   Bounding this is a caller-side concern.
//!
//! # Where to next
//!
//! - For a runnable walkthrough of two peers gossiping in one process,
//!   see [`crate::tutorial`].
//! - For recipes covering common real-world tasks (persisting state,
//!   bridging to TCP, parallelizing across tasks, compressing the
//!   wire), see [`crate::guide`].
//! - For the API itself, start at the [crate root](crate).
