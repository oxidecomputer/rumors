# How it compares

`rumors` is an *augmented Merkle repair*: Merkle set reconciliation over a
content-addressed store whose every node is annotated with an interval tree
clock version, under which the store forms a join-semilattice. Each
ingredient has prior art; this section places the design among its neighbors
— what it takes from each, and what each still does better. In one line:
Dynamo-style Merkle repair made exact and tombstone-free by fusing it with
an ORSWOT-style causal-dominance rule — carried per subtree, not per element
— over ITC versions, tuned to spend bandwidth to buy round trips.

## Epidemic anti-entropy

The founding work is [*Epidemic Algorithms for Replicated Database
Maintenance*](https://dl.acm.org/doi/10.1145/41840.41841) (Demers et
al., PODC 1987): replicated databases converge by pairwise anti-entropy
and rumor mongering, analyzed as epidemics. It is also the origin of
this crate's central problem. Demers et al. observe that deletion
cannot be represented by absence and introduce *death certificates* —
tombstones — plus a dormancy scheme to bound their retention, trading
resurrection risk against storage. `rumors` is a direct descendant in
framing (its anti-entropy is the whole protocol; there is no separate
rumor-mongering channel), and its redaction design resolves that
paper's open trade: absence *plus causal dominance* is a sound
representation of deletion, so no certificate is ever stored and
nothing retires on a timer. The loss case: Demers et al. engineered for
thin, expensive, lossy links, with spatial distributions to cut
cross-link traffic; `rumors` deliberately buys latency with bandwidth
and is the wrong protocol on metered or narrow links — [the crate
docs](crate#when-shouldnt-you-use-it) say so.

## Merkle repair in production stores

[Dynamo](https://dl.acm.org/doi/10.1145/1294261.1294281) (DeCandia et
al., SOSP 2007) introduced per-key-range Merkle trees so replicas
compare digests and transfer only divergent ranges;
[Cassandra](https://docs.datastax.com/en/dse/6.8/architecture/database-architecture/anti-entropy-repair.html)
builds a fixed-depth hash tree over each partition range on demand, and
[Riak's active anti-entropy](https://docs.riak.com/riak/kv/latest/learn/concepts/active-anti-entropy/index.html)
maintains persistent hash trees updated on write. This is the closest
operational ancestor: hash-compare down, transfer at the frontier.
`rumors` gains three things in its regime. The tree *is* the store —
persistent, incrementally memoized, never rebuilt by scanning (Riak
approaches this; Cassandra pays a full-scan validation per repair).
Resolution is exact — one leaf per message, so repair ships exactly the
differing messages rather than streaming every range that disagrees.
And, decisively: digest-only repair cannot distinguish "you're missing
this" from "you deleted this", so these systems all carry tombstones
with garbage-collection grace windows and the resurrection hazard of a
replica that sleeps past its window; the causal sieve eliminates that
category. The loss cases: those systems shard — the tree covers a key
range, replicas hold partitions, datasets are disk-resident and
effectively unbounded — where every `rumors` peer holds the full set in
memory; they repair *mutable* keyed data where `rumors` replicates
immutable messages; and their per-repair metadata is constant-size
regardless of item count, while `rumors` keeps per-message tree state
resident.

## Canonical-shape search trees

[Merkle Search Trees](https://inria.hal.science/hal-02303490) (Auvolat
& Taïani, SRDS 2019) are deterministically shaped search trees — an
item's layer derives from its hash, so structure is a pure function of
content — supporting Merkle anti-entropy over *ordered* keys; Bluesky's
[AT Protocol repository](https://atproto.com/specs/repository) is an
MST in production, and [Prolly
trees](https://www.dolthub.com/docs/architecture/storage-engine/prolly-tree)
(Noms/Dolt) get the same canonical-shape property from content-defined
chunking. `rumors`' trie shares the headline property — equal content
gives equal shape and equal digests on every replica, insert-order
independent. The difference is what the key order buys. An MST
preserves *application* key order, so it serves range scans and
tolerates non-uniform, even adversarial, key distributions; `rumors`
spends the key space entirely on uniformity — the path *is* the hash —
which makes its depth and occupancy statistics theorems rather than
expectations, and recovers range queries on the one axis it cares
about (causality) from the version-bound memos instead
([`Snapshot::range`](crate::Snapshot::range)). What `rumors` adds in
its regime is the deletion story — MST anti-entropy alone reconciles
toward union and re-learns deletions; the MST paper pairs the tree
with CRDT value types and inherits whatever deletion metadata those
carry — and a shipped wire protocol: a pipelined, fixed-memory,
deadlock-argued session where the paper specifies a structure and a
compare-and-descend outline. The loss cases: ordered keys (syncing
"records in this collection" or "events in this time range"),
self-certification in open networks (exactly the untrusted regime
`rumors` declares off-model), and attacker-chosen keys; Prolly trees
additionally offer structural diff and merge for versioned data, a
different product than a gossip substrate.

## Range-based set reconciliation

[Range-Based Set Reconciliation](https://arxiv.org/abs/2212.13567)
(Meyer, 2023) reconciles two sets over a total order: peers exchange
fingerprints of ranges, split ranges that disagree, and recurse until
disjoint items ship — `O(D·log N)` communication over `O(log N)`
sequential rounds with *no per-item auxiliary state*, everything
computed from a sorted index at sync time;
[negentropy](https://github.com/hoytech/negentropy) is the deployed
instantiation. This is the same divide-and-conquer skeleton as the
descent with the tree made implicit, and it is the honest
bandwidth-side benchmark: RBSR's metadata is leaner (no 256-way child
listings), it needs no resident tree, it works off any ordered
storage, and its total order lets a caller reconcile a *subrange* —
none of which `rumors` offers. Where `rumors` wins in its regime:
latency discipline (RBSR's rounds are inherently sequential per range
lineage, while `rumors` pipelines an entire level's disputes per hop
under a derived memory bound); incremental maintenance (memoized
digests make session compute frontier-proportional, with no full-index
fingerprint pass); and deletion (RBSR converges on union, full stop —
deleting through it requires tombstones in the item set or
application-level deletion records). Fingerprint soundness is also
structurally easier here: RBSR's composable range fingerprints must
resist crafted collisions in the homomorphic combine, where `rumors`
compares plain truncated BLAKE3 at fixed tree positions under a
trusted-peer model.

## Sketch-based reconciliation

Invertible Bloom lookup tables encode a set into a sketch sized to the
*difference*: exchange sketches, subtract, peel out exactly the
differing items — `O(D)` communication independent of `N`, in one
round trip ([Eppstein et al., SIGCOMM
2011](https://dl.acm.org/doi/10.1145/2018436.2018462)); [Rateless
IBLT](https://arxiv.org/abs/2402.02668) (Yang, Gilad & Alizadeh,
SIGCOMM 2024) removes the difference-estimation problem with a coded
symbol stream the receiver consumes until decoding succeeds. In pure
information-theoretic terms this family wins the bandwidth and
round-trip race outright — for *finding the difference*, `rumors` is
not competitive on bytes, and knows it. Its claim is different.
Semantics: a sketch yields the symmetric difference as items, and the
protocol still cannot say whether an item the peer lacks should be
sent or was deleted there — the deletion problem returns untouched.
Compute shape: sketch encoding touches every element (or maintains a
mutable sketch, which then *is* per-item auxiliary state) and peeling
is CPU-heavy at scale, where `rumors`' memoized tree keeps session
compute proportional to the frontier. Payloads: sketches reconcile
fixed-width identifiers, so message bodies need a retrieval round.
And determinism: decoding is probabilistic; the descent is
deterministic given the trees. Rateless IBLT is the right choice on
thin links, unknown divergence, and untrusted counterparties;
`rumors`' regime — fat trusted links, latency-bound, deletion-heavy —
is close to its complement.

## Tombstone-free deletion in CRDTs

The nearest *semantic* neighbor is the [optimized
OR-set](https://arxiv.org/abs/1210.3368) (Bieniusa et al., 2012),
deployed as Riak's ORSWOT: elements carry birth dots, the replica
carries a version vector, and on merge an element present on one side
whose dot the other side's vector dominates is dropped — removal by
causal dominance, no tombstones. The causal sieve is recognizably this
inference. The lift is twofold. First, the "dot" is the message's send
version, which is simultaneously half the key's hash input — identity,
causality, and placement are one commitment rather than parallel
bookkeeping. Second, and substantively: ORSWOT states the rule
element-wise over full states or delta intervals, while `rumors` pushes
it into the reconciliation tree as per-subtree version bounds, so
dominance prunes whole subtrees mid-session and the deletion filter
*is* the sync protocol's pruning rule. On the adjacent path, [causal
stability](https://arxiv.org/abs/1710.04469) (Baquero, Almeida &
Shoker) discards operation metadata once causally stable — but needs
stability tracked across the whole membership, which churn stalls;
`rumors` has nothing retained to discard. The loss cases, stated
plainly: OR-set-family types support *concurrent add and remove of the
same element* with add-wins arbitration, which `rumors` sidesteps
rather than solves (messages are immutable and uniquely keyed, so the
race cannot arise — but neither can "re-add the same element"
semantics); and tombstone-carrying designs (Yjs's integrated
tombstones, Automerge's retained operation history) buy richer types —
sequences, maps, text — plus partial replication, history, and
auditability in untrusted settings. `rumors` buys zero-residue
deletion by restricting the data model to an unordered set of
immutable messages.

## The version algebra

Version vectors need one entry per actor forever: under dynamic
membership they either leak entries or need out-of-band retirement.
[Dotted version vectors](https://arxiv.org/abs/1011.5808) (Preguiça,
Baquero et al.) solve a different problem — precise per-write dots
against sibling explosion — and still assume a stable actor-id space.
[Interval tree clocks](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf)
(Almeida, Baquero & Fonte, 2008) make the identity space itself a
lattice: identities fork and join, membership change is a local
operation, and a retired identity is *recycled*, not leaked. `rumors`
(via [`before`](crate::before)) builds this into its product surface —
[`seed`](crate::Peer::seed), [`bootstrap`](crate::Peer::bootstrap), and
[`retire`](crate::Peer::retire) *are* ITC fork and join — and every
message version is an ITC stamp, which is also what keeps the greeted
causal context compact under membership churn. The loss cases: ITC
identities are structural, not nameable — "what did replica R write?"
needs a registry outside the clock; version sizes degrade under
sustained fork/join churn ([`before`](crate::before)'s docs put a
version at ~2 KB at equilibrium churn across 100 parties, roughly
`N²`, against a version vector's predictable `N` entries); linearity
of identities is a sharp global safety invariant (see [the crate
docs](crate)) that vector-clock systems never have to state; and a
peer that crashes without retiring strands an identity interval,
widening stamps slightly forever, where a version-vector entry can be
garbage-collected by fiat.

## What is new here

Every ingredient above has prior art; the combination is the claim. We
know of no prior system that reconciles a content-addressed Merkle
structure under a causal-dominance sieve: annotating every node of a
canonical set index with the lattice ceiling and floor of its
subtree's versions, so that the descent prunes simultaneously by hash
equality and by causal dominance, and deletion propagates with zero
stored deletion state — the two session versions are the entire causal
context, and the deletion filter is literally the pruning rule. ORSWOT
has the sieve without the tree; MSTs, Prolly trees, and RBSR have the
tree or the recursion without causal annotation, so they converge on
union; [Merkle-CRDTs](https://arxiv.org/abs/2004.00107) (Sanjuán et
al., 2020) put CRDTs in a Merkle DAG, but as a grow-only causal
history whose removals need tombstone-bearing value types — nearly the
dual of the move made here; and the production Merkle repairs carry
tombstones and grace windows. The choice of ITCs as the version
algebra compounds the claim only mildly — any causal lattice would
support the sieve — but it is load-bearing for the membership story
and for keeping the greeting small under churn.
