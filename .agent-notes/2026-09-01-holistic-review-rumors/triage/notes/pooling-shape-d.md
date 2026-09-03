<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch on 2026-09-04 from a subagent's read of oxidecomputer/sush, sprockets, and qorb (clones in the session scratchpad; every quoted fact re-checked by the coordinator in those clones); not authored, audited, or endorsed by Finch. -->

# The routed link's pooling seam, evaluated against sush

The question (p2-link stop 3, T164 item 1 reopened): how a transport's
connection pool keys by link, given `Dial::dial(&self, addr)` carries
no link identity. Shape (d): pooling owned by the adapter's per-link
`StreamConnector`, `Dial` shrinking to `dial`.

## What sush does today (verified in the clone)

- `sush/Cargo.toml` pins `rumors` at `b2675dbe` and uses
  `rumors::link::routed` over sprockets (`server/src/link.rs`); its
  module doc: a fresh connection is a full attested handshake, about a
  second with the RoT in the loop, and the RoT serializes them, so the
  dialer pools through `Dial::recycle` into one qorb pool per peer
  address (`pools: HashMap<SocketAddrV6, Arc<Pool<PooledConn>>>`).
- qorb 0.4.1 sizes each pool to `claimed + spares_wanted` (2) at every
  60 s rebalance and removes idle slots above it, so a session's
  recycled complement survives at most until the next tick, then is
  culled to two; a refused or released return (the `READY` byte never
  arrives) marks the slot for a background re-handshake. sush's pooling
  is weaker than its own comments say.
- One logical link per peer pair by policy (an address tiebreak), but
  links are replaced on every session failure, migration, or driver
  replacement, and the per-peer pool spans those transitions: the old
  link's clean connections stay pooled while the peer's router releases
  them, and the next `Endpoint::link` or stream open to that peer draws
  one and fails, one per retry tick. The release hazard is on sush's
  ordinary re-link path, not only in a two-links case.
- sprockets has no session resumption or anything cheaper than a full
  attestation (`grep -rniE 'resum|ticket|psk' tls/src` is empty; a
  fresh `ClientConfig` per connect; the attestation exchange runs
  unconditionally after the TLS handshake).
- The shipped `sush` binary does not gossip yet (`main.rs` uses
  `isolated(seed_gossip())`); the transport exists and its conformance
  test is `#[ignore]`d.

## Shape (d) against that

- Works with a plain `dial`: sush's `dial` becomes its `PoolConnector::connect`
  body (spawn the handshake under a timeout; a non-cancel-safe handshake
  must be spawned, which `Dial::dial`'s doc then states). The adapter's
  noop-waker poll on a sprockets stream yields the `READY` byte, `Pending`,
  end-of-stream after `close_notify` (released or refused), or an error
  after a reset: exactly take, keep, discard.
- Dials per session (one side, `k` streams opened, one link per pair):
  today `k - 1` on the first session, `0..k` within 60 s of the last,
  `k - 2` plus two refills after a longer idle, and on re-link up to 17
  failed draws one per 5 s tick plus a refill handshake each; under (d)
  `k` on the first session, `0..k` thereafter for the link's life, and 1
  on re-link with zero stale draws (the old link's pool drops with it).
  Shape (a) matches (d) in steady state but leaks a complement of
  attested connections per dead link unless the transport also learns
  the link ended, which nothing in (a) provides.
- Leaves the public surface: `Dial::recycle`, the `READY` contract, the
  must-not-block caution, the per-link admission rule as a
  transport-facing statement, the per-peer hazard, the pool-sizing
  corollary. sush deletes about 150 of its 525 link lines and the qorb
  dependency.
- The one cost: idle socket count. qorb culls to two per peer after
  60 s; (d) holds up to a complement per link per direction for the
  link's life (about 31 x 17 outbound on a 32-sled rack, plus the same
  inbound at the router), bounded by the router's admission and warm
  for the next session. A per-link cap in `Config` (at most
  `STREAM_COUNT`) is the knob if it ever matters.
- The noop-waker poll misses a `READY` still in flight at the moment of
  an open (a fresh dial that once; the connection serves the next
  open); today's shape has the same window.

## Recommendation

(d), landed by the p2-link lane as a re-scope of link-28 under a new
ruling: `Dial` shrinks to `dial` (public signature change, ruled),
pooling becomes a `Config` bit defaulting on, the per-link pool lives in
`StreamConnector` and drops with the link, the router's per-link
admission and `READY` byte stay as the internal protocol between the
two ends, and the two-links test lands as an adapter test.
