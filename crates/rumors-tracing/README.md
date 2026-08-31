# rumors-tracing

<!-- cargo-rdme start -->

A `tracing` adapter for `rumors` gossip sessions: spans per
session and per directed stream, one structured event per observed
wire item.

`rumors` exposes its wire traffic through the bytes-level
observation hook in `rumors::observe`. This crate is that hook's
bridge into the `tracing` ecosystem: attach a `TracingObserver`
to a peer, install whatever `tracing` subscriber your application
already uses, and every session the peer enters becomes a span tree
with each protocol message a structured event inside it. Because
the wire is CBOR end to end, the adapter needs no knowledge of the
protocol's message vocabulary: items render generically through the
`cbor-diag` crate — deep inspection of application payloads comes
free, since they are the application's own CBOR, and rumors' own
tags read back through the `rumors::tags` registry.

## Quickstart

```rust
use std::sync::Arc;

use rumors::Peer;
use rumors_tracing::TracingObserver;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), rumors::Error> {
    // Install your subscriber first (tracing_subscriber::fmt(),
    // a test collector, anything): the adapter emits, it never
    // installs.
    let alice = Peer::<String>::seed()
        .observe(Arc::new(TracingObserver::new()))
        .into_rumors();
    alice.send("the meeting is at noon".to_string());

    let (mut near, mut far) = rumors::link::memory();
    let (served, joined) = tokio::join!(alice.gossip(&mut far), async {
        Peer::<String>::bootstrap().join(&mut near).await
    });
    served?;
    joined?.expect("alice is established, not herself bootstrapping");
    Ok(())
}
```

## What the adapter emits

Everything is emitted under the target `rumors`, so one directive
(`rumors=debug`) scopes it in any filter. Field values that name
protocol vocabulary (`SessionKind`,
`Role`, …) are recorded in their debug form.

- **One `session` span per observed session** (level `INFO`):
  fields `kind` (`Gossip`, `Bootstrap`, `Retire`), `protocol`, and
  `ordinal` — the adapter's own count of the sessions it has
  observed, so concurrent sessions stay distinguishable. (The hook
  deliberately carries no session number; numbering is the
  observer's concern, and this adapter counts internally.)
- **One `role elected` event** (level `INFO`, inside the session
  span) when the session's role election is decided, with the
  elected `role`. Sessions whose greetings carry equal versions
  hold no election and emit no such event.
- **One `stream` span per directed stream** (level `DEBUG`, child
  of the session span): fields `kind` (`control` or `data`) and
  `direction`, plus `speaker` and `index` for data streams.
- **One `message` event per wire item** (level `DEBUG`, inside its
  stream span): fields `ordinal` (see below), `len` (the item's
  exact wire size in bytes), and `item` — the item rendered by the
  `cbor-diag` crate in RFC 8949 extended diagnostic notation:
  structure unfolded, encoding indicators where a head is not the
  immediate form, embedded-CBOR tags (24, 63) shown as `<<…>>`,
  byte strings as hex, and tags by number (`rumors::tags` is the
  decoder ring for rumors' own). The rendering is bounded: nesting
  depth by `cbor-diag`'s default depth limit (embedded unfolds past
  the budget fall back to plain byte strings) and rendered length
  by a local cap with an explicit elision mark, so events stay
  cheap under megabyte supply runs.

## Ordering across streams

A session's streams pump concurrently and the hook deliberately
imposes no cross-stream ordering, so subscriber-side timestamps
interleave events only as precisely as your subscriber's clock.
The `ordinal` field is the sharper tool: the adapter stamps every
`message` event from one session-scoped atomic counter, so sorting
a session's events by `ordinal` reconstructs the observed
interleaving exactly — the consumer-side pattern the hook's
documentation recommends, built in.

## Cost and back-pressure

Handlers run synchronously inside the session's stream tasks (see
the hook's back-pressure contract in `rumors::observe`): the
adapter therefore does bounded work per item — one parse plus one
capped rendering, carried lazily so it runs only when a subscriber
actually formats the `item` field — and only when the `message`
event is enabled by your subscriber; a disabled target costs the
enabled check plus one relaxed atomic increment (the ordinal must
advance even unobserved).
A subscriber that blocks inside `event` stalls the emitting stream,
exactly as any slow `StreamObserver` would: keep slow sinks
behind a channel.

## When not to use it

- **To watch the replica's *contents*** — what messages exist,
  in what order — use `rumors`' own pull-based observers
  (`rumors::Rumors::unordered_messages` and friends); the wire
  view is the wrong altitude for application state.
- **To capture sessions for byte-exact replay or assertion**,
  implement `rumors::observe::Observer` directly and keep the
  bytes: this adapter renders for human eyes and elides by design.

<!-- cargo-rdme end -->
