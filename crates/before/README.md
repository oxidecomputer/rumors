# before

<!-- cargo-rdme start -->

`before` implements [*Interval Tree Clocks* (Almeida, Baquero &
Fonte, 2008)](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf) (ITCs)
using an efficient and compact representation.

Interval tree clocks use much less space than traditional representations of
version vectors and vector clocks, often by more than an order of magnitude.
In dynamic settings where participants arrive and leave, they can also
*recycle identifiers* via a `join` without violating
causality, so they avoid the unbounded growth that affects naïve sparse
clocks and vectors.

### Efficiency

At 100 parties and 1,000,000 events, the expected size of a `Party` is
about 3 bytes and the expected size of a `Version` is about 100 bytes.
These figures assume static membership; continually `fork`ing
and `join`ing causes these to grow, but with reasonable
bounds. Under sustained membership churn, those same 100 parties will each
stabilize at around 50 bytes (linear in `N`) and their corresponding
versions at around 2,000 bytes (roughly `N²`).

![Space consumption of `before`'s interval-tree versions](https://raw.githubusercontent.com/oxidecomputer/rumors/HEAD/crates/before/results/space_consumption/itc_space_consumption.svg)

This crate implements cache-friendly, optimized versions of the operations
in the original paper, in addition to a host of useful operations not
described therein. Compared to a 1-to-1 transliteration of the paper into
Rust, `before` is between 2–20× faster.

### The types

| Type                | Is                                              | Core operations                                                                                                                                                   |
|---------------------|-------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `Party`           | a distinct entity which may emit events         | `tick`, `fork`(`s`), `join`, `is_disjoint`                                     |
| `Version`         | a causal timestamp (history of known events)    | `tick`, `PartialOrd` (`<`, `<=`, `concurrent`), join (`\|`), `rank`                                    |
| `Clock`           | a `Party` paired with its current `Version` | `tick`, `fork`(`s`), `join`, `send`, `recv`, join (`\|`) a `Version` |
| `Rank`/`Ranked` | the total causal ordering for `Version`       | `Ord` (`<`, `==`, `>`, etc.), summation (`+`), `checked_sub`                                                                               |

`Party`s and `Clock`s are linear (`!Clone`); `Version`s are
freely `Clone`able.

### A conceptual sketch

The insight of the original ITC paper is that a `Party` can be represented
as a *tree* denoting a non-empty set of subintervals of `[0, 1)`, giving
both compact representation and dynamic membership. The initial `Party`,
`Party::seed`, is `{ 0, 1) }`; a [`fork` splits an interval
in half, so the first fork yields `{ 0, 1/2) }` and `{ [1/2, 1) }`.
Disjoint interval sets are [`join`ed by set union, merging
adjacent intervals: `{ [0, 1/2), [5/8, 3/4) }` ∪ `{ [3/4, 1) }` = `{ [0,
1/2), [5/8, 1) }`. Parties can therefore be minted and recycled freely while
their representations stay small.

A `Version` is then a function from `[0, 1)` to the natural numbers, also
represented as a tree, with the initial `Version` the constantly-zero
function. To register an event for a `Party`, it suffices to increment the
function over any non-empty region owned by that party. Any such choice
yields a valid causal timestamp, and the freedom of choice lets the
implementation simplify a `Version`'s tree on `tick`. As
parties and versions are forked and joined over the lifetime of a
distributed system, their typical size therefore stays small: hundreds to
low thousands of bytes, even for hundreds of communicating processes and
millions of events.

That lattice is the `Version` API: the partial order `<=` tests whether
one version's history is contained in another's, the join `|` combines two
histories into their least upper bound, and `tick` moves
strictly upward. Two histories with no containing order are
*`concurrent`*.

By packaging a `Version` and a `Party` together into a `Clock`, we get
a causal clock which may be `tick`ed,
`fork`ed, and `join`ed, in addition to derived
operations like `send`, `recv`, and
`sync`. This is sufficient to implement *both* [*version
vectors*](https://en.wikipedia.org/wiki/Version_vector) and [*vector
clocks*](https://en.wikipedia.org/wiki/Vector_clock), depending on how you
use it.

### Additional utilities

The `causally` module gives convenience constructors for causal orderings
in terms of `Version`s: `since`,
`before`, `delta`, and friends build
a `causally::Range` (a `RangeBounds<Version>`
whose membership predicate is causal containment). Where a *total* order
over versions is needed, `Rank` is the exact, strictly-monotone causal
rank: ordering by `(Rank, tiebreak)` linearly extends causality, so
concurrent versions can be sequenced deterministically. `Ranked` packages
a version with its rank as a ready-made totally ordered key, tiebroken by
canonical bytes.

### Example

Depending on whether you want the semantics of [*version
vectors*](https://en.wikipedia.org/wiki/Version_vector) or [*vector
clocks*](https://en.wikipedia.org/wiki/Vector_clock), you use
`before` slightly differently.

#### ... as a Version Vector

[*Version vectors*](https://en.wikipedia.org/wiki/Version_vector) give a
causal ordering to **data**. Participants **do not** record a local event
when sending and receiving messages; only when modifying data.

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

// Bob's clock now dominates or is equal to the message, and also Alice's version
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

#### ... as a Vector Clock

[*Vector clocks*](https://en.wikipedia.org/wiki/Vector_clock) give a causal
ordering to **processes**. Participants **do** record a local event when
sending and receiving messages, *as well as* when modifying data.

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

// Bob incorporates Alice's version, then marking a "recv" event locally
bob.recv(&msg);

// Bob's clock now dominates the message, and also Alice's version
assert!(bob.version() > msg);
assert!(bob.version() > alice.version());

// But if Alice now records another local event unknown to Bob ...
alice.tick();
// ... then their versions are now incomparable (i.e. concurrent)
assert!(bob.version().concurrent(alice.version()));

// Unlike with version vectors, there is no way to re-synchronize two
// versions to become strictly equal by sending or receiving messages,
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
*disjoint* from it. The caller must ensure both:

1. **Singularity.** A system of clocks has one `Clock::seed` (or
   `Party::seed`), created once, from which every `Clock` and
   `Party` in the system descends. One `Party` may be reused with
   multiple `Version`s, and multiple "universes" may coexist, each
   descended from its own `seed`, as long as clocks from
   different seeds never interact.

2. **Linearity.** Operations on `Clock`s and `Party`s are strictly
   linear: once a `Clock` or `Party` has been
   `fork`ed, a copy of the pre-fork value must not come
   back into play. The crate helps by making `Party` and `Clock`
   `!Clone`, but at serialization boundaries and across
   processes, linearity is the caller's responsibility.

### Testing

Every operation is verified differentially against the paper's naive
recursive implementation as well as a nondeterministic function-space
semantics, alongside exhaustive small-scope enumeration of clock shapes,
algebraic-law property suites, and fuzzed codecs (`decode`'s strict
canonicality is asserted inline in the fuzz targets).

<!-- cargo-rdme end -->
