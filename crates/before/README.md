# before

<!-- cargo-rdme start -->

`before` implements [*Interval Tree Clocks* (Almeida, Baquero &
Fonte, 2008)](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf) (ITCs)
using an efficient and compact representation.

A causal clock answers the question wall-clock time cannot: given two
updates made on different machines, did one *know about* the other, or did
they happen concurrently? The classic answers — [*version
vectors*](https://en.wikipedia.org/wiki/Version_vector) and [*vector
clocks*](https://en.wikipedia.org/wiki/Vector_clock) — pay one counter per
participant, forever: an entry can never be safely removed once its
participant leaves, so under churn those clocks only grow.

Interval tree clocks give the same causal answers in much less space,
often by more than an order of magnitude, and in dynamic settings they
*recycle identity*: a departing participant `join`s its
clock into a surviving peer's, returning its share of the *id space* —
the range of identity the participants partition among themselves — and
its history, so the clocks avoid unbounded growth.

### Quickstart

A first session. Two informal definitions up front: the *seed* is the
first clock, owning the whole id space; a *fork* splits a clock in two,
each half a valid clock that acts independently from then on.

```rust
use before::Clock;

// Every system of clocks descends from one seed (see the safety rules).
let mut alice = Clock::seed();

// New participants fork off a live clock, never mint themselves.
let mut bob = alice.fork();

// Each participant ticks its own clock to record a local event.
alice.tick();
bob.tick();

// Neither history contains the other: the two events are concurrent.
assert!(alice.version().concurrent(bob.version()));

// Versions are the timestamps that travel. Joining a received version
// (`|=`) merges history only; bob then knows strictly more than alice —
// her whole history, plus his own tick that she has not seen.
bob |= alice.version();
assert!(bob.version() > alice.version());

// Joining a whole *clock* merges identity too: it retires bob, recycling
// his share of the id space into alice. It is fallible — the two parties
// must be disjoint — hence the `unwrap`.
alice.join(bob).unwrap();

// Clocks and versions cross process boundaries as canonical bytes: one
// byte spelling per value, so equal bytes mean equal values.
let bytes = alice.encode();
let restored = Clock::decode(&bytes[..]).unwrap();
assert_eq!(restored, alice);
// `restored` duplicates `alice`: treat encode-then-send as a move, and
// let exactly one of the two handles stay live (see the safety rules).
```

### The types

| Type                | Is                                              | Core operations                                                                                                                                                   |
|---------------------|-------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `Party`           | a distinct entity which may emit events         | `tick`, `fork`(`s`), `join`, `is_disjoint`                                     |
| `Version`         | a causal timestamp (history of known events)    | `tick`, `PartialOrd` (`<`, `<=`, `concurrent`), join (`\|`), meet (`&`), `rank`                        |
| `Clock`           | a `Party` paired with its current `Version` | `tick`, `fork`(`s`), `join`, `send`, `recv`, join (`\|`, `\|=`) a `Version` |
| `Rank`/`Ranked` | a total order extending the causal order       | `Ord` (`<`, `==`, `>`, etc.), summation (`+`), `checked_sub`, `encode`/`decode`                            |

`Party`s and `Clock`s are linear (`!Clone`): moved, never
duplicated, because duplicating identity is exactly what breaks a causal
clock. `Version`s are freely `Clone`able. A tick pairs the two
halves — `party.tick(&mut version)` and `version.tick(&party)` are the
same act from either receiver (`Party::tick`, `Version::tick`) — and
`Clock::tick` performs it on its own pair.
The `iter` module forks `n` peers in one balanced split.

### Version vector or vector clock?

`before` can play either classic role; the difference is
entirely in how you use it. [*Version
vectors*](https://en.wikipedia.org/wiki/Version_vector) order **data** —
they answer "has this replica seen that write?" — so participants record
an event only when the data changes. [*Vector
clocks*](https://en.wikipedia.org/wiki/Vector_clock) order **processes** —
they answer "could this event have influenced that one?" — so
participants record an event on every send and receive as well. Reach for
a version vector when reconciling replicated state; reach for a vector
clock when the messages themselves are the events. Pick one discipline
per protocol and keep to it — mixing them is not unsafe, but the ordering
then answers neither question exactly.

#### As a version vector

Participants **do not** record a local event when sending and receiving
messages; only when modifying data.

```rust
use before::Clock;

// Alice is the distinguished first party who creates the first clock
let mut alice = Clock::seed();

// Alice hands Bob a clock of his own
let mut bob = alice.fork();

// Alice marks an event locally
alice.tick();

// Bob marks an event locally
bob.tick();

// Alice sends her *current* version *without* recording another event locally
let msg = alice.version();

// Bob incorporates Alice's version *without* recording another event locally
bob |= msg;

// Bob's clock now dominates or equals the message and Alice's version.
// (`>=`, not `>`: incorporating a version records no event, so a Bob
// with no history of his own would end exactly equal to the message —
// contrast the vector-clock example below.)
assert!(bob.version() >= msg);
assert!(bob.version() >= alice.version());

// But if Alice now records another local event unknown to Bob ...
alice.tick();
// ... then their versions are now incomparable (i.e. concurrent)
assert!(bob.version().concurrent(alice.version()));

// Bob can send his version back to Alice, and vice-versa,
// for their versions to become equal again.
alice |= bob.version();
bob |= alice.version();
assert!(bob.version() == alice.version());
```

#### As a vector clock

Participants **do** record a local event when sending and receiving
messages, *as well as* when modifying data.

```rust
use before::Clock;

// Alice is the distinguished first party who creates the first clock
let mut alice = Clock::seed();

// Alice hands Bob a clock of his own
let mut bob = alice.fork();

// Alice marks an event locally
alice.tick();

// Bob marks an event locally
bob.tick();

// Alice marks a "send" event locally and then sends her version to Bob
let msg = alice.send();

// Bob incorporates Alice's version, then marks a "recv" event locally
bob.recv(&msg);

// Bob's clock now strictly dominates the message, and also Alice's
// version: the receive itself was an event.
assert!(bob.version() > msg);
assert!(bob.version() > alice.version());

// But if Alice now records another local event unknown to Bob ...
alice.tick();
// ... then their versions are now incomparable (i.e. concurrent)
assert!(bob.version().concurrent(alice.version()));

// Unlike with version vectors, there is no way to re-synchronize two
// versions to become equal by sending or receiving messages,
// because receiving a message records a local event unknown to the
// sender by definition -- so if Bob sends to Alice, then vice-versa,
// then Bob's version will strictly dominate Alice's, because he knows
// about one more event than her (his own local receive)
alice.recv(bob.send());
bob.recv(alice.send());
assert!(bob.version() > alice.version());
```

### Safety rules

Interval tree clocks are correct only under the Law of Disjointness: no
`Party` may ever interact with another `Party` that is not
*disjoint* from it. (A party is a set of id-space
intervals — see [How it works](#how-it-works); disjoint parties share
none.) Two parties *interact* when one is `join`ed or
`sync`ed (a mutual join) with the other, and whenever versions they tick
meet in a comparison or a join. Only the first kind is fenced: join and
sync do verify their operands, and refuse overlapping parties. The
second kind is where corruption lives — a version carries no record of
who ticked it, so a version written through a duplicated identity is
indistinguishable from a healthy one, and comparisons simply begin
reporting causal order that never happened. Nothing panics; the answers
are just wrong.

The two rules below are what make the Law hold. Each is stated by its
commonest trigger, and the discipline extends past the letter of both:
a `Clock` restored from bytes persisted *before* its last
`tick` brings an earlier state of the identity back
into play with no fork anywhere in the history — the restored party
overlaps its own descendant, and two distinct events receive one
version. An overlap error from a join or sync is always a symptom,
never the disease — some rule was already broken upstream (a stale
copy came back into play, or a second seed leaked in), and the fence
caught one visible consequence of it; the rest of the damage may
already be silent. The caller must ensure both:

1. **Singularity.** A system of clocks has one `Clock::seed` (or
   `Party::seed`), created once, from which every `Clock` and
   `Party` in the system descends. One `Party` may be reused with
   multiple `Version`s (`Party::tick` borrows the party, so one
   identity can stamp many histories), and multiple "universes" may
   coexist, each descended from its own `seed`, as long
   as clocks from different seeds never interact. Nothing in a value or
   its encoding identifies its universe, so that boundary is the
   caller's to police.

   *What about a universe tag, so mixups could be detected?* There is
   deliberately no mechanism for this. Naming a universe is the concern
   of the library that embeds `before`, which already has the
   protocol and structures a universe lives in — while a tag stamped
   into every value (a UUID, say) would add 128 bits to values that
   are often a few bytes long, paid across an entire corpus.

2. **Linearity.** Operations on `Clock`s and `Party`s are strictly
   linear: once a `Clock` or `Party` has been
   `fork`ed, a copy of the pre-fork value must not come
   back into play. The crate helps by making `Party` and `Clock`
   `!Clone`, but at serialization boundaries and across
   processes, linearity is the caller's responsibility.

### Replicating clocks between processes

Everything crosses process boundaries as canonical bytes:
`encode`/`encode_to` emit them, and
`decode` validates strictly — malformed or non-canonical
input is rejected, never repaired — so an accepted value is always in
normal form, and byte equality on encodings is exactly value equality.
The `serde` and `borsh` features serialize through the same encodings
(see [Crate features](#crate-features)).

Most protocols move `Version`s — the freely `Clone`able timestamps —
and keep each `Clock` pinned to its process. A whole `Clock` or
`Party` crosses the wire only when the identity itself must move: a
hand-off, or a retiring peer sending its clock to a survivor — any live
peer can absorb it, since `join` needs only disjointness.
At that boundary, linearity becomes your job (the second safety rule):
the type system cannot see that a clock whose bytes left the process now
exists twice, so treat encode-then-send as a *move* and stop using the
local handle.

### Comparing and ordering versions

Causality itself is the partial order on `Version`: `a <= b` tests
containment of history, and two versions with no containing order are
`concurrent`. Two tools extend it:

- **Filtering**: the `causally` module names causal ranges —
  `since(a)` is everything `a` does not already
  contain, `before(b)` everything strictly contained
  in `b`, and `delta(a, b)` everything known at `b`
  but not at `a`: the shape of "what my peer has not seen yet". Each is
  a `causally::Range`, a
  `RangeBounds<Version>` whose membership
  predicate is causal containment.
- **Sorting**: where a *total* order over versions is needed, `Rank`
  measures a version by a quantity that strictly grows with every tick
  — the exact area under the version's history function, drawn out in
  `implementation` — so `v < w` implies
  `v.rank() < w.rank()`: causes always sort before their effects. Only
  concurrent versions can tie, and any deterministic tiebreak then
  yields the same total order on every replica. `Ranked` builds
  that total order in: it views a version by its rank — comparisons
  run fused over the packed streams, no rank materialized — with the
  version's canonical bytes as the tiebreak, and its
  `encode` emits a composite byte key whose plain
  byte-wise order *is* the total order, `decode`
  recovering the version from the key. `Rank::encode` is the
  rank-only key form for stores that bring their own tiebreak;
  either way a sorted KV store gets a causal-ordering key with no
  rank-aware comparator on the store's side.

### How it works

The insight of the original ITC paper is that a `Party` can be
represented as a *tree* denoting a non-empty set of subintervals of
`[0, 1)` — each node splits its interval in half, and a leaf marks its
whole subinterval owned or not — giving both compact representation and
dynamic membership. The initial `Party`, `Party::seed`, is
`{ [0, 1) }`; a `fork` splits an interval in half, so the
first fork yields `{ [0, 1/2) }` and `{ [1/2, 1) }`. Disjoint interval
sets are `join`ed by set union, merging adjacent
intervals: `{ [0, 1/2), [5/8, 3/4) }` ∪ `{ [3/4, 1) }` = `{ [0, 1/2),
[5/8, 1) }`. Parties can therefore be minted and recycled freely while
their representations stay small.

A `Version` is then a function from `[0, 1)` to the natural numbers,
also represented as a tree, with the initial `Version` the
constantly-zero function. To register an event for a `Party`, it
suffices to increment the function over any non-empty region owned by
that party. Any such choice is valid because a successor timestamp only
has to dominate its predecessor — everywhere at least as high, somewhere
strictly higher — and disjointness guarantees no other party ever
increments the same region. That freedom lets the implementation prefer
the increment that *simplifies* the tree, flattening a party's region to
one plateau rather than stacking detail; ticks flatten and joins merge
fragments, which is why typical sizes stay small — hundreds to low
thousands of bytes even for hundreds of communicating processes and
millions of events (see [Efficiency](#efficiency)).

These functions form a *lattice* — a partial order in which any two
elements have a least combined upper bound — and that lattice is the
`Version` API: the partial order `<=` tests whether one version's
history is contained in another's, the join `|` combines two histories
into their least upper bound, the meet `&` keeps exactly the history two
versions share, and `tick` moves strictly upward. Two
histories with no containing order are
*`concurrent`*. Packaging a `Version` and a
`Party` together into a `Clock` gives a causal clock which may be
`tick`ed, `fork`ed, and
`join`ed, in addition to derived operations like
`send`, `recv`, and `sync`
(a mutual exchange: both clocks end holding the joined history).

How the crate makes all of this compact and fast — the packed
representation, the coding, and the sweeps every operation runs as — is
the `implementation` module's essay, with a worked example in hand.

### Efficiency

At 100 parties and 1,000,000 events, the expected size of a `Party` is
about 3 bytes and the expected size of a `Version` is about 100 bytes
(measured: the space-consumption experiment that draws the figure below).
These figures assume static membership; continually `fork`ing
and `join`ing causes these to grow, but with reasonable
bounds. Under sustained membership churn, those same 100 parties will each
stabilize at around 50 bytes (growing linearly in the number of parties
`N`) and their corresponding versions at around 2,000 bytes (roughly
`N²`).

![Space consumption of `before`'s interval-tree versions](https://raw.githubusercontent.com/oxidecomputer/rumors/HEAD/crates/before/results/space_consumption/itc_space_consumption.svg)

This crate implements cache-friendly, optimized versions of the operations
in the original paper, in addition to a host of useful operations not
described therein. The paper's recursive representation survives in-tree
as the differential-testing oracle, and the workspace bench suite times
every operation against it.

### Crate features

Every feature is off by default.

- **`serde`** — `Serialize`/`Deserialize` for `Party`, `Version`,
  `Clock`, `Rank`, `Ranked`, and `Span`, each as
  its canonical byte encoding; deserializing runs the same strict
  validation as `decode`.
- **`borsh`** — `BorshSerialize`/`BorshDeserialize`, likewise as the
  canonical encodings. The encodings are *prefix-free* — no value's
  encoding is a prefix of another's, so a decoder knows where each value
  ends — and values therefore compose inside larger borsh messages
  without a length prefix.
- **`doc-images`** — embeds the space-consumption diagram above into the
  rendered docs (`cargo doc --all-features`).
- **`oracle`**, **`meter`** (plus the meter's counter switches
  `limb-meter` and `scan-meter`), and **`laws`** — expose the crate's own
  verification instruments — the paper-faithful reference implementation,
  the input generators and resource meters behind the performance tests,
  and the named algebraic-law predicates the law proptests and fuzz
  target share — to its bench, metering, and fuzz suites. Never for
  production use.

### Testing

Every operation is verified differentially against the paper's naive
recursive implementation as well as a nondeterministic function-space
semantics (versions modeled as literal mathematical functions, with
tick's freedom of where to increment exercised nondeterministically),
alongside exhaustive small-scope enumeration of clock shapes,
algebraic-law property suites, and fuzzed codecs (`decode`'s strict
canonicality is asserted inline in the fuzz targets).

<!-- cargo-rdme end -->
