# The routed link: accept/connect transports behind a per-process router

Status: determination 2026-07-29. This is the layer-A design of record
for transports with no innate substreams (TCP and everything shaped
like it), instantiating the router sketched in
`streaming-wire-deadlock.md` §8.4–8.5 as an in-crate generic adapter,
`rumors::link::routed`. The link contract itself (`src/link.rs`) is
unchanged; this document decides how an accept/connect byte-stream
transport satisfies it.

## 1. Problem and shape

A `Link` needs one persistent bidirectional control stream plus up to
`STREAM_COUNT` (17) unidirectional data streams per direction, opened
lazily mid-session, each independently flow-controlled and
half-closeable. QUIC provides all of that natively, one connection per
link. An accept/connect transport provides exactly one primitive:
dial a name, get a byte stream; listen, get byte streams.

The mapping is one connection per stream — per-stream flow control and
half-close come from the transport itself, which is precisely what the
independence clause asks for — behind a per-process **router**:

- Every dialed connection opens with a small **connect header** naming
  the link it belongs to, by token.
- One router task per process owns the listener, reads headers, and
  hands each connection *whole* to its link's bounded queue. After the
  header, the connection never touches the router again: a router that
  proxied bytes would become a mux and inherit the head-of-line
  problem the link contract exists to preclude.
- The first connection of a link (kind `LINK`) *is* the control
  stream; later connections (kind `STREAM`) are data streams.

The adapter is generic over the caller's transport (§2) and lives in
the crate because it is pure logic over the caller's connections: no
sockets, no spawning, no timers, no new dependencies — the crate's
no-runtime promise holds (the router is a future the caller drives).

## 2. The seam: `Addr`, `Conn`, `Dial`, `Listen`

```rust
pub trait Addr: Clone + Send + Sync + 'static {
    fn encode(&self) -> Vec<u8>;              // must fit MAX_ADDR_LEN (255)
    fn decode(bytes: &[u8]) -> Option<Self>;
}
pub trait Conn: AsyncRead + AsyncWrite + Unpin + Send + 'static {}  // blanket
pub trait Dial: Clone + Send + Sync + 'static {
    type Addr: Addr;
    type Conn: Conn;
    async fn dial(&self, addr: &Self::Addr) -> io::Result<Self::Conn>;
}
pub trait Listen: Send + 'static {
    type Conn: Conn;
    async fn accept(&mut self) -> io::Result<Self::Conn>;  // cancel-safe
}
```

`Addr` is a name in the *caller's* namespace — a socket address, a
Unix path, an overlay peer id. The router ferries it (opaquely, in the
`LINK` header) and never interprets it; only the caller's `Dial` does.
This is what unbinds the adapter from IP. An `Addr` impl for
`SocketAddr` ships with the module (18 bytes: v4-mapped 16-byte IP +
big-endian port) so socket transports don't each reinvent it.

Two `Conn` requirements the bounds cannot express, documented as
contract on the trait:

- `poll_write` makes bytes peer-visible without an explicit flush (the
  session and the conformance probes never flush before awaiting a
  peer's read);
- dropping the connection delivers already-written bytes and then EOF
  (drop is the transport half-close the session relies on).

`TcpStream` satisfies both; a buffering wrapper (a `BufWriter`, some
TLS rigs) does not, and fails the independence probe or truncates
final bytes. Security is the caller's layer, per the link contract's
security division: `Dial`/`Listen` are assumed pre-authenticated, and
the token routes — it does not authenticate.

## 3. The wire

All multi-byte integers big-endian. One header per dialed connection,
written by the dialer before any protocol byte; one ACK byte in reply
on `LINK` connections only.

```
magic    10  b"ROUTEDLINK"
version   1  0x01
kind      1  0x01 LINK | 0x02 STREAM
token    16  the link's identity, minted at link establishment

LINK only:
addr_len  1  length of the dialer's advertised name (1..=255)
addr      …  Addr::encode() of the name peers dial back

ACK       1  0x01, accept-side router → dialer, LINK connections only
```

Per-field rationale:

- **magic** denotes the thing itself — a routed-link connect header —
  so a connection misrouted across protocols fails at the first read
  with a precise error. It is deliberately not an abbreviation of the
  crate name: the session preamble already owns `b"RUMORS"`, and two
  near-identical magics would blunt exactly the misroute diagnosis a
  magic exists for.
- **version** is the compatibility door. A future header kind that
  needs more context (for instance a reusable-lease connection with
  private framing) arrives as a new version or kind, never a mutation
  of this one.
- **token** is 16 random bytes: wide enough that collision handling
  and coordination are non-problems; minted per *link*, not per
  session — sessions on a link are serialized and already disambiguate
  themselves by the epoch byte in the session's own 2-byte stream
  label. The router routes on the token alone.
- **addr** rides only on `LINK` because only link establishment needs
  it: the accept side must dial back for its own outgoing data
  streams, and the accepted connection's source is not the dialer's
  listener (ephemeral ports, NAT) — only the dialer knows its own
  reachable name. Length-prefixed and capped at 255 so the router's
  header read is bounded by construction.
- **ACK** closes the registration race, §5.

The fixed 28-byte prefix (magic through token) is one `read_exact`;
`LINK` reads `addr_len` and then exactly that many more bytes. The
maximum header is 284 bytes.

The header deliberately carries **no epoch and no stream index**,
deviating from the `(link token, epoch, stream index)` shape sketched
in `streaming-wire-deadlock.md` §8.4. That sketch predates the
contract's anonymous-streams clause: `Connector::connect()` takes no
arguments, so the adapter *cannot* learn the epoch or index — the
session writes them itself as the stream's first payload bytes and
validates them on the accepting side. The router has no routing use
for them, and duplicating the session's label would mint a consistency
obligation between two copies of one fact with no checker.

## 4. The router

One drive future per endpoint, returned by the constructor and driven
by the caller (spawn it, select over it — the adapter never spawns).
Its loop selects over `Listen::accept` and a `FuturesUnordered` of
per-connection futures; each accepted connection becomes one such
future, so the loop itself never awaits any per-connection I/O.

Each per-connection future owns its connection and does, bounded:

- read the header (`read_exact` of the fixed prefix, then the
  kind-determined remainder);
- `LINK`: register the token in the shared table → `try_reserve` an
  incoming-links permit (no permit: kill the connection, unregister)
  → write the ACK byte → split the connection into the control halves
  → assemble the accept-side `Link` (its connector dials the header's
  decoded `Addr`) → deliver `(LinkInfo, Link)` on the permit;
- `STREAM`: `try_send` the whole connection onto the token's queue.
  Unknown token, or a queue that is gone: drop the connection.

The §8.5 discipline, instantiated:

- **Never await a per-destination queue.** Per-link queues are bounded
  mpsc channels of capacity `STREAM_COUNT` (the control connection
  never routes through the queue — it *is* the `LINK` connection, so
  17 suffices; sessions are serialized, so an honest peer can never
  have more in flight). Delivery is `try_send`, never `send`.
- **Overflow kills the link, not the connection.** A full queue proves
  peer misbehavior or a local bug. Dropping just the overflowing
  connection would silently lose a delivery on a link that still looks
  healthy — the accepting session would wait forever for a labeled
  stream that never arrives, a hang where a poison was owed. Killing
  the link (unregister the token, drop the queue's sender so the
  acceptor's next `accept` errors) turns it into ordinary transport
  failure: session errs, link poisons, application re-links.
- **Sockets, not bytes.** After the header the router never touches
  the connection; flow control stays end-to-end per stream.
- **Header reads are bounded in size by construction and in count by
  eviction.** §8.4 sketched header reads as small spawned tasks with a
  time bound; the crate promises no spawning and no timers, so the
  determination is: per-connection futures live in the drive future's
  own `FuturesUnordered`, and the set is *count-bounded* — beyond the
  configured cap, the oldest pending header read is aborted (its
  connection drops). A connection that stalls mid-header therefore
  occupies one slot until fresh arrivals displace it, and can never
  park the loop. Callers who want wall-clock bounds put them where the
  clock lives: a deadline-wrapping `Listen`/`Conn` in their transport
  layer. The cap is hygiene, not a security boundary — the transport
  is pre-authenticated, and hostile-peer economics are off-model.

Token-table entries are removed eagerly: the link's acceptor half
holds a guard whose drop unregisters the token, so an application that
drops a link (poisoned or merely finished) revokes its routing at that
moment, whether or not the router ever hears another byte. Unknown
tokens are dropped on sight; after the ACK handshake below, the only
honest sources of one are a link the local application already
discarded, or dials chasing a stale address — both resolve as
transport failure on the dialer's side, which is the self-healing
path (poison, re-link).

The drive future resolves `Err` when `Listen::accept` fails, and the
failure is the caller's to interpret (their `Listen` owns retry policy
for transient conditions; the adapter treats what it is given as
fatal). It resolves `Ok(())` only if the endpoint is shut down.

## 5. Link establishment

`Endpoint::link(peer)`:

1. mint a token, build the link's queue, and **register the token
   locally, before anything touches the wire** — the peer's reverse
   dials can only follow its read of our header, so
   register-before-write makes them happens-after registration;
2. dial `peer`, write the `LINK` header (token + our advertised name);
3. await the ACK byte; EOF or a non-ACK byte is a crisp rejection
   (unregister and fail);
4. split the connection into the control halves and assemble the
   `Link`.

The ACK exists because of a real race on the accept side. Header reads
complete in any order (they are concurrent futures), and accept/connect
transports guarantee nothing about cross-connection ordering — so
without the ACK, a `STREAM` dial racing behind a `LINK` dial could
reach the peer's router before the `LINK` header finishes processing,
and die as an unknown token even though both ends are honest. The
protocol's own traffic happens to serialize past the window (the first
data stream follows speaker election, which follows control-stream
bytes), but the contract does not: `connect()` may legally be called
before any control byte, and the conformance suite does exactly that.
One ACK round trip, paid once per link (not per session, not per
stream), makes registration on both routers happens-before either
end's first possible `connect()` — the dialer's by rule 1, the
acceptor's because the ACK is written after registration.

Both ends may `link()` toward each other concurrently; the result is
two independent links, each with its own token and control stream.
The adapter does not deduplicate — which link (or links) to gossip on
is application policy, as it is for every other transport.

## 6. Contract mapping

| Clause (src/link.rs) | How the routed link satisfies it |
| --- | --- |
| Control duplex | The control stream is one transport connection, split; both directions are the transport's own full duplex. |
| Independence | One connection per stream; flow control and buffering are per-connection in the transport. Nothing is shared: no window pool, no mux. |
| Flow control | Receiver-paced by the transport per connection, at whatever capacity the caller's transport grants (the contract needs only "positive"). |
| Concurrency | `connect()` dials a fresh connection; opens share no limiter, so no open ever serializes behind another stream's progress. The 17-per-direction complement is 35 connections worst case, well inside any listener backlog the caller configures. |
| Half-close | `Tx` *is* the connection; dropping it closes it, delivering written bytes then EOF (the documented `Conn` requirement). The control connection is held by the link itself and outlives the session's data connections structurally. |
| Cancellation | `Acceptor::accept` is a bounded-mpsc `recv()`, cancel-safe by construction: an undelivered connection stays queued for the next call. |
| Anonymous streams | The acceptor yields connections in arrival order with no routing logic; the session's own label does the pairing. |
| Security division | Inherited whole: authentication, integrity, confidentiality, freshness are the caller's transport (`Dial`/`Listen`), below this layer. The token is routing state, not a credential. |

Accepted failure modes, stated rather than hidden:

- A dial that reaches the wrong process (stale address, restarted
  peer) dies at the header or as an unknown token; the dialing session
  poisons; the application re-links. Self-healing, at session
  granularity.
- A silently dead path (no reset in flight) hangs a `connect()` or a
  write until the caller's session timeout cancels the session — the
  same backstop every transport relies on; keepalive belongs in the
  caller's `Dial`.
- Every lazy stream open pays one dial (connection setup + 28-byte
  header). That is the compatibility trade named in
  `streaming-wire-deadlock.md` §8.4: this instantiation exists for
  deployments that cannot take QUIC, not to win latency contests.

## 7. Decision records

- **DECIDED (2026-07-29): in-crate module, not a sibling crate.** The
  adapter is generic over `Dial`/`Listen` and needs zero new
  dependencies; only its *instantiations* (TCP, TLS) touch a network
  stack, and those stay caller-side (the crate's tests instantiate
  over `tokio::net` from dev-dependencies). The sibling-crate ruling
  in `streaming-wire-deadlock.md` §8.9 governs network bindings; an
  adapter with no network dependency is core-eligible logic.
- **DECIDED (2026-07-29): caller-driven router future; count-bounded
  header reads.** The crate promises no runtime, no spawning, no
  timers; the router is therefore a future the caller drives, and
  pending header reads are bounded by count with oldest-first
  eviction instead of by wall clock. Time bounds live in the caller's
  `Listen`/`Conn` wrappers, next to the runtime that can measure them.
- **DECIDED (2026-07-29): per-link token, token-only routing.**
  Granularity: sessions are serialized per link and self-labeled
  (epoch byte), so routing state is per-link. Content: no epoch or
  stream index in the header — `connect()` cannot supply them, the
  router cannot use them, and duplicating the session label creates an
  unchecked consistency obligation. Supersedes the header sketched in
  §8.4, which predates the contract's anonymous-streams clause.
- **DECIDED (2026-07-29): one-RTT LINK ACK.** Registration on both
  routers must happen-before either end's first `connect()`; the
  contract permits a `connect()` before any control byte, so control
  traffic cannot be the fence. Cost: one round trip per link
  establishment, amortized over every session the link carries.
- **DECIDED (2026-07-29): single-use connections; no reuse at stream
  boundaries.** Reuse (the pooled-lease idea in §8.3) requires layer-A
  private framing to delimit stream end without a transport close,
  forfeiting the one thing connection-per-stream gets free: EOF *is*
  the half-close, kernel-enforced, no trailing-byte ambiguity. It also
  needs clean/dirty lease tracking and abort plumbing. The genuine
  future case is per-connection security-handshake cost; the header's
  version/kind fields are the door, and the change would be a new
  kind, never a mutation.
- **DECIDED (2026-07-29): no connection pooling in v1; the `Dial`
  trait is the pooling seam.** A warm-dial supply (pre-established
  connections to hide dial latency on lazy opens) composes behind
  `Dial` without touching header, router, or link code, and should be
  motivated by latency measurements, not anticipation. qorb 0.4.1 was
  evaluated as the supply: its pools are homogeneous (`Pool::claim()`
  cannot target a backend, so per-peer means one pool per peer), its
  `claim::Handle` cannot detach a connection (Deref/DerefMut/Drop
  only), and single-use claims fight the recycle machinery (an
  `on_recycle` error is the only way to retire a spent connection,
  which reads as perpetual backend failure in its stats and
  rebalancer). A qorb-backed `Dial` would hold each claim only long
  enough to extract the connection (`Option::take` inside the pooled
  type), never across a stream's lifetime — holding claims for live
  streams re-couples stream opens to pool capacity, which is exactly
  the cross-stream wait the concurrency clause forbids (and qorb's
  default `max_slots = 16` is below one session's worst-case 18).
  Recorded here so the future evaluation starts from these facts.
- **DECIDED (2026-07-29): no peer discovery.** `link()` takes an
  explicit `Addr`. Discovery composes above the adapter (any resolver
  feeding addresses in the caller's namespace) and below it (a `Dial`
  that resolves names at dial time); building either in would bind the
  adapter to a discovery model the seam deliberately leaves open.
- **REJECTED: deriving the dial-back address from the accepted
  connection.** The accepted connection's source is an ephemeral port
  (and possibly a NAT); only the dialer knows its reachable name, so
  the `LINK` header carries it, opaquely, in the caller's namespace.
- **REJECTED: per-session listeners (the `tests/common/tcp.rs`
  shape) as the production story.** One listener per session avoids
  the router entirely and is the right trade for a test harness, but
  a process gossiping with P peers over S serialized sessions would
  churn P listeners per round and advertise a moving target; the
  per-process router with a stable advertised name is the deployable
  shape, as `tests/common/tcp.rs`'s own docs state.
