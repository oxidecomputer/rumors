# Forking API ideas

A tangent from the [presentation note](README.md). Building a probe, Claude
assumed `Party::forks(k)` partitions the whole party into `k` shares. It
does not, and its docs say so: "`self` retains one residual share", and
`From<Party>` for `[Party; N]` forks into exactly `N` shares with no residual.
Finch proposed two additions. Neither is designed or built; both are
proposals for later triage.

## `into_forks(n)`: exact shares, count chosen at run time

`Party::into_forks(n)` and `Clock::into_forks(n)` consume the receiver and
yield exactly `n` balanced shares with no residual: the dynamic counterpart
of `From<Party> for [Party; N]` and `From<Clock> for [Clock; N]`, whose count
is a compile-time constant.

Points to settle:

- **`n = 0`.** A `Party` is never empty, so zero shares would destroy
  identity. A nonzero count type rules it out by construction; a panic would
  be the fallback for a count only known at run time.
- **Dropping the iterator early.** `forks(k)` borrows the receiver, which
  keeps every share the iterator has not returned. A consuming iterator has
  nowhere to return them, so dropping it early discards identity. Options:
  `#[must_use]` and documentation; yield an exact-size collection rather than
  a lazy iterator; or offer a method that hands back the unreturned
  remainder as one `Party`.
- **Clock.** Each share carries a clone of the version, as
  `From<Clock> for [Clock; N]` does.

## `forking()`: choose the count during iteration

The idea: an iterator hands out handles, and each handle resolves into a
balanced fork of the original only after iteration ends. The caller can then
decide how many forks to take while iterating, for instance one per item of
a stream of unknown length.

### Why handles must be promises

A balanced partition cannot be built incrementally. Handed-out shares are
owned `Party` values (identity linearity), so they cannot later be split
further. The first share is handed out before the total `n` is known. If it
were a real party of width `w`, then for large enough `n`, `w` would exceed
every other share by far more than any balance bound allows. So a handle must
be a promise, and the shares materialize only once `n` is fixed. This also
answers "resolved only once the iterator is dropped". `Drop` cannot return
values, so the ordering has to be expressed by moving the iterator into the
step that fixes `n`.

### Shape

```rust
let mut forking = party.forking();       // borrows `party` mutably
let first = forking.next().unwrap();     // a Pending handle: not Clone
let rest: Vec<_> = stream.map(|_| forking.next().unwrap()).collect();
let mut forks = forking.finish();        // consumes `forking`: n is now fixed
let first: Party = forks.redeem(first);  // consumes the handle
// Dropping `forks` joins every unredeemed share back into `party`.
```

The move semantics give the ordering. `finish` consumes the minting side,
so no handle is minted after `n` is fixed. `redeem` exists only on what
`finish` returns, so no handle resolves before `n` is fixed. Handles are not
`Clone`, so each resolves at most once.

**Mismatched sessions** (redeeming a handle against another `forking`'s
result) need ruling out. Two options:

- **Runtime check.** Each handle carries a session identity, and a mismatch
  panics. It is a programmer error, so a panic is admissible, but the types
  do not prevent it.
- **Branded lifetime.** A closure-scoped invariant lifetime, as `GhostCell`'s
  token or the `generativity` crate provide, makes a handle from one session
  ill-typed with another:
  `party.forking(|mut session| { let a = session.fork(); …; let mut forks = session.finish(); forks.redeem(a) })`.
  Handles then cannot escape the closure (the redeemed parties can).

**Leftovers.** With a borrowed receiver, a share whose handle was dropped
unredeemed, and any share left unredeemed when the result drops, rejoin the
receiver, so no identity is lost. As with `forks(k)`, the receiver keeps a
residual share, since a `Party` is never empty. A consuming variant has no
such home and inherits `into_forks`'s early-drop question.

**Cost.** A handle is an index (plus a session identity in the runtime-check
variant). Redemption needs share `i` of a balanced `n`-way split
constructed directly from `(n, i)`, rather than by walking shares in order as
`PartyForks` does; whether the current split admits that cheaply is to be
checked.

### Drawbacks, and when it is worth having

- A handle is not a party: it cannot tick, be sent, or be joined until
  `finish`.
- Redemption is a second step, and the branded variant confines handles to a
  closure.
- Most callers can count first and call `forks(k)`, which yields usable
  parties immediately.

The motivating case is a source whose length is unknown and that cannot be
buffered, or code that must hand out shares before it knows how many it
will need. That makes `forking()` an alternate API beside the bounded
iterator, not a replacement. Its name could be `forking` or something else.
