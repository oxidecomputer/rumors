# Partition link: The Link transport contract, the in-memory link, erased links, and the routed (TCP) link

## Partition summary

This partition is the crate's transport boundary. `src/link.rs` states the six-clause contract every transport must satisfy (control duplex, per-stream independence, receiver-paced flow control at any positive capacity, `STREAM_COUNT` concurrency, completion through `Done`, tolerance of dropped accepts), defines the `Connector` and `Acceptor` traits, the `Link` bundle with its sealed `SessionState` (an epoch counter and a poison latch), the `LinkParts` decoration path, and the in-memory reference instantiation (`memory`, `MemoryConnector`, `MemoryAcceptor`). `src/link/erased.rs` is the monomorphization funnel: a `Bundle<H>` keeps a stream half together with its `Done` behind `TxDyn`/`RxDyn`, `DynConnector` erases behind an `Arc`, and `DynAcceptor<'a>` erases behind a `&mut dyn`. `src/link/routed/` adapts accept/connect transports (TCP and everything shaped like it) onto the contract by giving every link stream its own connection: `header.rs` is the 28-byte connect header and the `Addr` boundary with its stock `SocketAddr` instantiation, `router.rs` the per-endpoint listener loop with its routing table, `Registration` drop guard, and count-bounded `Abortable` header reads, `stream.rs` the per-link `StreamConnector`/`StreamAcceptor`, and `endpoint.rs` the `Endpoint`, `Incoming`, `Config`, and error types.

I read all 3204 lines of the partition with line numbers (production: `link.rs` 630, `erased.rs` 163, `routed.rs` 288, `endpoint.rs` 302, `header.rs` 321, `router.rs` 281, `stream.rs` 101; test code: `src/link/tests.rs` 180, `src/link/routed/header/tests.rs` 195, `src/link/routed/tests.rs` 743), plus the neighbors the findings rest on (`peer/gossip.rs` erasure aliases and `for_session` callers, `proxy/start.rs`'s `open`, `remote/streams.rs`'s `AcceptDriver::new` and its test callers, `codec/signal.rs`, `conformance/link/tests.rs`, `tests/common/routed_tcp.rs`, `tests/routed_link.rs`, `codec/encode/async_io.rs`). No cargo, just, or test command was run for this report; every "verified" item below is a grep or a read, and the two constructions were traced, not executed, when it was finalized. The witness pass afterwards ran link-28's memory-network construction: under `pending_headers: 1`, one stalled arrival evicted the pooled connection and the next stream open on a healthy link failed with `BrokenPipe` (`witness/results.md`); the TCP-pool hang variant and the four-peer figure under the default `Config` stay assessed by tracing.

The code is in good shape. The contract states goal beside mechanism for every clause; every `expect`/`unreachable!` in production code is a one-line proof from a check on the preceding lines; cancellation of `connect`, `accept`, `Incoming::accept`, and the router's `select!` arms is safe by construction; the poison latch is set before any wire traffic and cleared only by the funnels; routing revocation falls out of ownership (`Registration`'s `Drop`) with no teardown API; the router's no-await discipline is structural (`try_send`, `try_reserve`, abortable per-connection futures). Test hygiene is strong: every test carries an invariant docstring, the header parser has an every-prefix truncation sweep and a proptest, and the routed suite is shaped as a negative control for each failure mode the module docs list. I found no vestige of the V1 protocol or BLAKE3 here, and the one candidate vestige (the blanket `Acceptor for &mut A`) turned out to have five in-crate consumers.

Three items rise above nits and lows. The routed adapter's single `pending_headers` bound treats two populations with opposite eviction preferences as one, so a pooling `Dial`'s already-admitted idle connections are evicted before a fresh stalled header, and the default is sized for the non-pooling case: a liveness failure reachable by conforming peers at four pooling peers with the default `Config`, which the code's own docs name as a sizing duty rather than closing structurally (link-28). The routed TCP example and the `Conn` docs never mention `TCP_NODELAY`, and the routed shape (unidirectional connections, one to three `write_all` pieces per frame) is the canonical Nagle-plus-delayed-ACK stall (link-14). And the routing table has no type of its own, so the token-claim invariant is spelled twice, two `#[allow(clippy::type_complexity)]` stand in for a newtype, and a maintainer comment's premise is false as written (link-26). The rest are documentation accuracy refinements, dialect, and small API-surface gaps, most owner-gated.

## Findings

### link-1: Four small accuracy fixes in `link.rs` docs: an expired "cheap", a self-contradicting "stated where they arise", a hand-maintained "8 KiB", a missing blank line
- Where: src/link.rs:19-22 (related: src/link.rs:298-299, src/link.rs:547-549, src/link.rs:376-377, src/link.rs:532, src/link/routed.rs:80-84)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read)
- Seen by: prose (22, 24, 20); refutation: confirmed (22, 20), confirmed (24, with a misquote corrected); history: 22 "cheap" deliberate-but-expired at b16a800b; 24 deliberate (3f33afab states the exception at every surface); 20 no rationale
- Owner-gated: no (the option of making `MEMORY_STREAM_CAPACITY` public is noted, not required)

"Data streams are session-scoped and cheap" reads as a property the contract requires of the transport; the routed adapter pays a dial per open, and its own module doc says so. "Three qualified exceptions, stated where they arise:" is followed by the three exceptions stated in full. "Each stream buffers 8 KiB" restates the private `MEMORY_STREAM_CAPACITY`, which the public doc cannot link, so the two drift independently. `poisoned()` and `begin`'s doc abut with no blank line. Prose speaks in the present tense and states structure, not tallies.

Evidence:

    19	//! Data streams are session-scoped and cheap: a session opens them lazily
    20	//! and sparsely (up to [`STREAM_COUNT`], typically far fewer) and ends

    298	///   repair. "Unchanged" has three qualified exceptions, stated where they
    299	///   arise:

    547	/// use. Each stream buffers 8 KiB; use [`memory_with_capacity`] to pick

    376	    }
    377	    /// Open one session, returning the epoch that labels its data streams.

    (routed.rs)
    80	//! - Every lazy stream open pays one dial. Where the dial itself is

Resolution: line 19: "Data streams are session-scoped: a session opens them lazily and sparsely ...". Lines 298-299: "three qualified exceptions (each also stated where it arises):" or drop the clause. Line 547: drop the figure ("Each stream has a fixed buffer; [`memory_with_capacity`] picks another size, down to one byte") or make the constant public and cite it. Insert a blank line between 376 and 377. Acceptance: no "cheap" as a transport property; the "stated where they arise" clause no longer precedes the list it disclaims; the literal buffer size appears once, at the constant; `just doclint` clean.

### link-2: The pooled-flow-control section narrates a past observation that a committed test now pins
- Where: src/link.rs:107-113 (related: src/conformance/link/tests.rs:1017-1034)
- Class / severity / confidence: documentation / nit / medium
- Provenance: assessed (read)
- Seen by: prose (15); refutation: confirmed; history: deliberate-and-holds (Finch wrote the observed-not-promised sentence in 9c184576 and c5f1210a, paired with the 64-byte pin in 32813f55c)
- Owner-gated: yes: reopens owner phrasing

The paragraph says floor sessions "have been *observed* live" and calls that "observed behavior of the current protocol", a dated measurement report in public rustdoc. The negative-space content (below the bound liveness is not promised, and why) is the part that matters and survives a present-tense restatement; the observation itself is pinned by `starved_pool_degrades_latency_not_liveness`, whose doc refers back to this paragraph by description ("the link docs' measured-tolerance sentence").

Evidence:

    107	//! Sessions at the serialization floor (a one-subtree-in-flight window)
    108	//! have been *observed* live over far smaller pools, down to tens of
    109	//! bytes, with latency degradation. That is observed behavior of the
    110	//! current protocol at that window shape, not a promise: a window wide
    111	//! enough to fill several streams at once can leave a sub-bound pool in
    112	//! a cycle of waits, each stream waiting on pool credit the others hold.
    113	//! Size pools to the bound.

Resolution: restate in the present tense with the pin named: floor sessions stay live over far smaller pools (the conformance suite pins a 64-byte pool), with latency degradation; that is behavior at that window shape, not a promise, because a window wide enough to fill several streams at once can cycle on pool credit; size pools to the bound. In the conformance test's doc (outside this partition), name what it pins in its own words rather than pointing at a sentence by description. Acceptance: no past-tense observation and no "current protocol" in the section; the test doc no longer points at a sentence by description.

### link-3: `STREAM_COUNT` is a literal 17 in two layers, held equal by a pin test, and the stated derivation is never asserted
- Where: src/link.rs:161-169 (related: src/link/tests.rs:11-19, src/tree/mirror/streaming/remote/codec/signal.rs:15-32, src/tree/mirror/streaming/remote/streams.rs:71, src/tree/mirror/streaming/remote.rs:87-91, src/tree/mirror/streaming/window.rs:124)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: `= 17` appears at link.rs:169 and signal.rs:32 only; `codec_stream_count` is a `#[cfg(test)]` accessor whose sole caller is link/tests.rs:17; `crate::link::STREAM_COUNT` is imported by window.rs:124 and window/tests.rs:9, so the existing layering arrow runs tree to link)
- Seen by: structure (1), correctness (33); refutation: confirmed both, merged; history: deliberate-and-holds for the pin mechanism (b3b877d9), no rationale for literal over derivation
- Owner-gated: no (crate-internal; the resolution reaches into the codec partition)

The doc derives `ceil(32 / 2) + 1 = 17` and says the value is "pinned against the wire codec by test", but the pin compares two literals: `Stream::COUNT` is `17` at signal.rs:32, not computed from `STREAMED_HEIGHT_COUNT` and `STREAM_HEIGHT_STRIDE` beside it. The derivation is enforced only indirectly (the codec's height arithmetic would misbehave in its own tests at 16 or 18). No hand-maintained counts: a quantity computable from constants already in scope should be computed, and the pin test plus its test-only accessor then dissolve.

Evidence:

    165	/// protocol's own, fixed by its wire schedule (the descent's 32 tree
    166	/// heights at a two-height stride per stream, plus the shared opening
    167	/// stream: `ceil(32 / 2) + 1 = 17`) and pinned against the wire codec by
    168	/// test, so it cannot drift silently.
    169	pub const STREAM_COUNT: usize = 17;

    (src/link/tests.rs)
    13	#[test]
    14	fn stream_count_matches_the_codec() {
    15	    assert_eq!(
    16	        STREAM_COUNT,
    17	        usize::from(crate::tree::mirror::streaming::remote::codec_stream_count()),
    18	    );
    19	}

    (signal.rs)
    32	    pub const COUNT: u8 = 17;

Resolution: keep ownership in `link` (the direction the existing imports already run) and have the codec cite it: `pub const COUNT: u8 = crate::link::STREAM_COUNT as u8;` with `const _: () = assert!(STREAMED_HEIGHT_COUNT.div_ceil(STREAM_HEIGHT_STRIDE) + 1 == crate::link::STREAM_COUNT);` beside the schedule constants in signal.rs, so the compiler checks the arithmetic the doc states. Delete `stream_count_matches_the_codec` and the `#[cfg(test)] codec_stream_count()` accessor; consider retiring the private `STREAM_COUNT` alias at streams.rs:71. Flag to the codec partition's reviewer. Acceptance: `grep -rn '= 17' src/link.rs src/tree/mirror/streaming/remote/codec/signal.rs` returns exactly one line; a compile-time assertion ties the value to the stride and height constants; `codec_stream_count` is gone; `just gate` clean.

### link-4: `Connector::connect` and `Endpoint::link` have no `# Cancel safety` section; their accept-side duals do
- Where: src/link.rs:220-227 (related: src/link.rs:252-257, src/link.rs:64-68, src/link/routed/endpoint.rs:246-252, src/link/routed/endpoint.rs:295-298, src/link/routed/router.rs:269-277, src/link/routed/tests.rs:412-461, src/tree/mirror/streaming/remote/streams.rs:159-168)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read both trait docs, both endpoint methods, and the router's ACK path)
- Seen by: perfapi (41), correctness (29a); refutation: confirmed both; history: no rationale (0eff1021 scoped the cancellation clause to accept drops; `Incoming::accept` got its section in b16a800b while `link` did not)
- Owner-gated: no

Hazards get uniform named sections. `Acceptor::accept` and `Incoming::accept` each carry `# Cancel safety`; `Connector::connect` and `Endpoint::link` are silent, which a reader takes as "nothing to say". Both have something to say. For `connect`: the session retains a pending open until it resolves and drops one only at teardown, an implementation must leave the link usable after a drop at any stage, and a completed-then-dropped open surfaces to the peer as an empty stream (the routed test `cancelled_opens_leave_the_link_usable` asserts exactly this, with no clause to cite). For `Endpoint::link`: dropping the future after the peer's router has written ACK and delivered through `slot.send` leaves the peer's application holding a link whose control connection closed at the drop, so its first session fails as transport failure; nothing leaks (the dialer's `Registration` drops with the future), and "nothing needs cleaning up" at endpoint.rs:251 is true of the `Err` arms only.

Evidence:

    220	    /// Open one outgoing unidirectional stream, paired with where the
    221	    /// half goes at its clean end.
    222	    ///
    223	    /// # Errors
    224	    ///
    225	    /// Fails only for transport reasons (the link is gone); the session
    226	    /// treats any error as fatal to the session, never retries.
    227	    fn connect(&self) -> impl Future<Output = io::Result<(Self::Tx, Done<Self::Tx>)>> + Send;

    (endpoint.rs)
    251	    /// Either way no link exists and nothing needs cleaning up; retry
    252	    /// policy is the caller's.

Resolution: add `# Cancel safety` to `Connector::connect` stating the retention discipline and the empty-stream residue, and mirror it in the contract's cancellation clause (link.rs:64-68), which today speaks only of `accept`. Add `# Cancel safety` to `Endpoint::link` stating the peer-side residue (a delivered link whose control stream is already closed; the peer's first session fails as transport failure and it re-links). The routed test's doc can then cite the clause it exercises. Acceptance: both sections present; `just doclint` clean.

### link-5: No `Debug` on `Link`, `LinkParts`, `Endpoint`, `Incoming`, or the four supply types; smaller common-trait gaps
- Where: src/link.rs:322-329 (related: src/link.rs:201-205, src/link.rs:482-500, src/link.rs:589-593, src/link.rs:610-612, src/link/routed/endpoint.rs:141-143, src/link/routed/endpoint.rs:287-289, src/link/routed/stream.rs:24-28, src/link/routed/stream.rs:66-70, src/link/routed/header.rs:97-98, src/link/routed/header.rs:129)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read every `derive` and `impl ... for` in the seven production files; no `Debug` impl exists for the listed types; `Token` derives `Clone, Copy, PartialEq, Eq, Hash` without `Ord`; `SessionState` and `Config` derive without `PartialEq`)
- Seen by: perfapi (42); refutation: confirmed; history: no rationale (no `missing_debug_implementations` lint; never ruled)
- Owner-gated: yes: public trait impls

`Done` has a hand-written `Debug` (link.rs:201-205), so the crate wants its transport types debuggable, yet `Link`, `LinkParts`, `MemoryConnector`, `MemoryAcceptor`, `Endpoint`, `Incoming`, `StreamConnector`, and `StreamAcceptor` implement none; a user struct holding a `RoutedLink<TcpDial>` cannot `#[derive(Debug)]`. Smaller: `Token` lacks `Ord` (cannot key a `BTreeMap`), `SessionState`, `Config`, and `LinkInfo` lack `PartialEq`, and `LinkInfo<A>: Debug` is conditional on an `A: Debug` bound `Addr` never asks for. The crate's own convention for opaque types is a manual `Debug` with `finish_non_exhaustive()` (message.rs, rumors.rs, peer.rs), which needs no bounds on the type parameters.

Evidence:

    322	pub struct Link<CR, CW, C, A> {
    323	    pub(crate) control_read: CR,
    324	    pub(crate) control_write: CW,
    325	    pub(crate) connector: C,
    326	    pub(crate) acceptor: A,
    327	    /// This link's session counter and poison latch.
    328	    pub(crate) session: SessionState,
    329	}

Resolution: manual `Debug` impls printing what is type-agnostic (`Link { session, .. }`, `LinkParts { session, .. }`, `MemoryConnector { capacity, .. }`, `Endpoint { local_addr, .. }` bounded on `D::Addr: Debug`, unit-style for `MemoryAcceptor`, `Incoming`, `StreamAcceptor`, `StreamConnector { token, .. }`), all via `finish_non_exhaustive()`. Derive `PartialEq, Eq` on `SessionState`, `Config`, `LinkInfo` and `PartialOrd, Ord` on `Token`. Consider `Debug` as an `Addr` supertrait. Acceptance: a test-only `#[derive(Debug)] struct Holder(RoutedLink<MemoryDial>)` compiles; `just gate` clean.

### link-6: `SessionState` has public getters but no path to read them without consuming the link
- Where: src/link.rs:363-376 (related: src/link.rs:328, src/link.rs:470-478, src/link.rs:491-499)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (`grep -n 'fn session' src/link.rs` is empty; `grep -rn '\.session\b'` outside src/link shows every public-tier read going through `parts.session` after `into_parts`, e.g. proxy/work/tests.rs:48, conformance/link.rs:1004, testing/transport.rs:533)
- Seen by: perfapi (35); refutation: confirmed; history: no rationale (ee67e4cb added the getters when sealing the fields; a `Link::session()` accessor was never raised)
- Owner-gated: yes: public API addition

`epoch()` and `poisoned()` are `pub`, but `Link::session` is `pub(crate)` and the only public route to a `SessionState` is `into_parts`, which consumes the link. A connection manager that wants check-then-act (re-link instead of firing a session that fails with `Error::LinkPoisoned`) must dismantle and reassemble the link to ask. A public getter on a public type implies a public way to hold a reference to that type. link-9, if adopted, dissolves this.

Evidence:

    365	    pub fn epoch(&self) -> u8 {
    366	        self.epoch
    367	    }

    374	    pub fn poisoned(&self) -> bool {
    375	        self.poisoned
    376	    }

    328	    pub(crate) session: SessionState,

Resolution: add `pub fn session(&self) -> &SessionState` on `Link` in the unbounded impl block beside `into_parts`. No wire or behavior change. Acceptance: a unit test reads `link.session().poisoned()` on a fresh link (false) and after an interrupted session (true) without calling `into_parts`.

### link-7: `Link::new` states its precondition as "at the same time"; the invariant is "before any session"
- Where: src/link.rs:418-420 (related: src/link.rs:339-340, src/link.rs:427-430)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read); the history claim verified via `git show c5f1210a -- src/link.rs`
- Seen by: prose (21); refutation: confirmed; history: no rationale, and the founding wording (b3b877d9) carried the correct explanation ("the epoch counters start at zero on both sides and advance in lockstep"), which c5f1210a replaced with a pointer
- Owner-gated: no

What `SessionState`'s lockstep needs is that both ends wrap the same transport with no session yet run on it, so both start at epoch zero with the latch clear. Simultaneity is neither necessary nor what any code checks; a public precondition stating a different condition than the one relied on misleads the implementer.

Evidence:

    418	    /// Both ends of a connection must construct their links around the
    419	    /// same fresh transport at the same time; the bookkeeping the two ends
    420	    /// then keep in step is [`SessionState`]'s.

Resolution: "Both ends wrap the same transport before any session runs on it, so both start at epoch zero with the latch clear; the bookkeeping the two ends then keep in step is [`SessionState`]'s." Acceptance: the doc states the fresh-transport condition and not simultaneity.

### link-8: `Link` doubles as the one-session carrier, so `for_session` builds a `Link` whose poison flag is admittedly inert
- Where: src/link.rs:441-461 (related: src/peer/gossip.rs:85, src/peer/gossip.rs:1147, src/peer/gossip.rs:1206, src/peer/gossip/tests.rs:209, src/tree/mirror/streaming/remote/proxy/start.rs:69, src/tree/mirror/streaming/remote/proxy/start.rs:420-427)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (grep: three `for_session` callers; proxy/start.rs:420-427 destructures the carrier and reads only `session.epoch()`; `DynLinkParts` is a positional 5-tuple at gossip.rs:85)
- Seen by: structure (4); refutation: confirmed; history: no rationale (the inert-flag comment has admitted the mismatch since the flag existed)
- Owner-gated: no (crate-internal)

`Handshaking::start` takes a `Link`, so the funnels rebuild one from the erased 5-tuple and `open` reads only the epoch. The type means two things: the long-lived link with a live `SessionState`, and a per-session carrier where half of that state is meaningless, as the comment says. A field meaningless in one of a type's two uses is two types sharing one name; a carrier struct also gives `DynLinkParts` field names.

Evidence:

    453	            // The carrier's own poison flag is inert: it lives for one
    454	            // session and is discarded; the long-lived link's state is the
    455	            // one the funnels consult and clear.
    456	            session: SessionState {
    457	                epoch,
    458	                poisoned: false,
    459	            },

Resolution: design proposal, crate-internal: a `pub(crate) struct Carrier<CR, CW, C, A> { control_read, control_write, connector, acceptor, epoch: u8 }` in `link` (or `link::erased`), with `Link::carrier(self)` for tests that hand a fresh `memory()` link to the proxy; `Handshaking::start` takes the `Carrier`; `DynLinkParts<'a>` becomes `Carrier<DynRead<'a>, DynWrite<'a>, DynConnector, DynAcceptor<'a>>`; `for_session` and its comment go. Steelman for the status quo: 25 lines, zero test ceremony; the change touches proxy/start.rs, the proxy harness, gossip.rs, and gossip/tests.rs. Acceptance: no `Link` is constructed with a `SessionState` no session consults; the proxy's entry point names the epoch as a field.

### link-9: `LinkParts` is a field-for-field twin of `Link` that exists to publish the fields
- Where: src/link.rs:481-500 (related: src/link.rs:322-329, src/link.rs:463-479, src/link.rs:502-519, src/link.rs:346-361)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (grep: `into_parts()` appears on 63 lines across 26 files in src, tests, and examples, every one a decorate-and-rebuild; `SessionState`'s fields are private at link.rs:352,360)
- Seen by: structure (5); refutation: confirmed (migration larger than "about twenty sites"); history: deliberate-and-holds (created in b3b877d9 "for wrapper-building"; R40 sealed `SessionState` while keeping the parts type; why a twin rather than `pub` fields was never argued)
- Owner-gated: yes: public API change

`Link`'s fields are `pub(crate)`; `LinkParts` has the same five fields `pub`, and `into_parts`/`into_link` move them across. With `Link`'s fields public, the same decoration is a destructure-and-rebuild of one type, and the twin, the two methods, and their docs dissolve. Nothing new becomes forgeable: `SessionState` stays sealed, so a `Link` literal can only carry a `SessionState` obtained from a real link, exactly as `LinkParts` permits today. The one reason to decline is representation freedom for `Link`; if that is the ruling, the reason belongs in `Link`'s doc so the twin has a stated justification.

Evidence:

    481	/// The dismantled pieces of a [`Link`]; see [`Link::into_parts`].
    482	pub struct LinkParts<CR, CW, C, A> {
    483	    /// The control stream's read half.
    484	    pub control_read: CR,
    485	    /// The control stream's write half.
    486	    pub control_write: CW,
    487	    /// The outgoing stream supply.
    488	    pub connector: C,
    489	    /// The incoming stream supply.
    490	    pub acceptor: A,

Resolution: either make `Link`'s five fields `pub`, move `LinkParts`' field docs (especially the `session` preservation warning at 491-498) onto them under a `# Decorating a link` section, delete `LinkParts`/`into_parts`/`into_link`, and update the 63 sites mechanically; or keep `LinkParts` and state in `Link`'s doc why the fields stay private. Acceptance: `grep -rn LinkParts src tests examples` is empty, or `Link`'s doc states the reason the parts type exists beyond publishing the fields.

### link-10: The erasure funnel's vocabulary and rationale are split between `erased.rs` and `gossip.rs`
- Where: src/link/erased.rs:5-10 (related: src/peer/gossip.rs:56-76, src/conformance/backend/tests.rs:67)
- Class / severity / confidence: modularity / nit / high
- Provenance: verified (grep: `type DynRead`/`type DynWrite` are private aliases at gossip.rs:71,76 with the cost argument at 56-70; the erasure argument is written out in full at erased.rs:3-16 and again at gossip.rs:56-70, differing in detail)
- Seen by: structure (10), prose (16, duplication half); refutation: confirmed; history: accretion (the aliases predate `erased.rs`, 83edcd944 vs b3b877d9); the "+0.7 GiB" figure is a recorded owner ruling (punch list item 3) and is not part of this finding
- Owner-gated: no

`link::erased` owns `DynConnector`/`DynAcceptor` and says it "mirrors" `DynRead`/`DynWrite`, but those live in `peer/gossip.rs` with their own full copy of the funnel's cost argument; the two copies already differ (this file counts two allocations per open, gossip.rs mentions none), and the backtick mention cannot be an intra-doc link. One concept, one module, one statement of record.

Evidence:

    7	//! tower instantiation — the reason this funnel is load-bearing. Every
    8	//! session entry point therefore erases the link's stream supply here
    9	//! — mirroring the `DynRead`/`DynWrite` erasure of the control halves —
    10	//! and the towers instantiate once per payload type.

    (gossip.rs)
    71	type DynRead<'a> = &'a mut (dyn AsyncRead + Unpin + Send + 'a);
    76	type DynWrite<'a> = &'a mut (dyn AsyncWrite + Unpin + Send + 'a);

Resolution: move `DynRead<'a>`/`DynWrite<'a>` into `link::erased` as `pub(crate)`, keep one statement of the erasure argument (this module doc), have gossip.rs import them and point here in a sentence, and turn the mention at line 9 into intra-doc links. Acceptance: gossip.rs imports the aliases from `crate::link::erased`; the erasure argument is written out once.

### link-11: Four in-memory link unit tests restate conformance clauses at lower strength
- Where: src/link/tests.rs:21-114 (related: src/conformance/link/tests.rs:26-39, src/lib.rs:306-307, src/conformance/link.rs:158-435)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (lib.rs:306 gates `conformance` on `any(test, feature = "conformance")`; `memory_link_conforms` and `one_byte_windows_conform` at conformance/link/tests.rs:28-39 run the full suite on `memory()` at default and one-byte capacity under `cfg(test)`)
- Seen by: correctness (31); refutation: confirmed (with the counterpoint that a unit failure localizes faster); history: no rationale (born in the same commit as the suite, never justified)
- Owner-gated: no

`control_carries_bytes_both_ways`, `connect_delivers_an_ordered_half_closing_stream`, `a_stalled_stream_does_not_couple_its_siblings`, and `stream_writes_block_on_their_own_reader` are weaker instances of `check_control`, `check_control_duplex`, `check_streams`, and `check_independence`, which the suite runs on the same `memory()` in the same `cargo test`. Anything these catch the suite catches, and a reworded clause now has two homes. The other three tests in the file (supply failure, which the suite says it cannot observe, and the two `SessionState` tests) are unique and belong.

Evidence:

    66	/// Streams are independent: a stream whose reader never drains does not
    67	/// stop a later stream from opening, transferring, and closing.
    68	#[test]
    69	fn a_stalled_stream_does_not_couple_its_siblings() {

    (conformance/link/tests.rs)
    28	#[test]
    29	fn memory_link_conforms() {
    30	    run_to_quiescence(super::check(async || memory())).expect("the suite stays live");
    31	}

Resolution: delete the four; keep `dropping_the_peer_fails_the_supply`, `epochs_count_and_wrap`, `an_unfinished_session_poisons_every_later_begin`; retitle the module doc from "Contract tests for the in-memory link" to what remains. If faster localization is wanted, that is a reason to keep them, stated in the module doc. Acceptance: src/link/tests.rs holds the three unique tests (or a stated reason for the others); the gate stays green.

### link-12: Test docs claim properties the bodies do not observe (blocking, independence, allocation)
- Where: src/link/tests.rs:94-95 (related: src/link/tests.rs:113, src/link/tests.rs:21-22, src/link/tests.rs:141-142, src/link/routed/tests.rs:118-119, src/link.rs:349-351)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read each doc against its body)
- Seen by: prose (19), correctness (32); refutation: confirmed both, merged; history: no rationale for 19; for "never allocates" the intended sense is recoverable (the epoch "allocates no identity", the crate's identity vocabulary; link.rs:351 says "a label tripwire rather than an identity")
- Owner-gated: no

An inaccurate testdoc is a bug in the test. `stream_writes_block_on_their_own_reader` says a writer past capacity "blocks until its own reader drains", but the body only joins a six-byte write against a read at capacity two, which a non-blocking pipe passes identically (the `.expect` at 113 states what is checked). `control_carries_bytes_both_ways` claims "two independent ordered byte pipes" over a strictly sequential ping/pong; independence is `check_control_duplex`'s job. `epochs_count_and_wrap` says the counter "never allocates", which read as a memory claim about a `u8` is unasserted and odd. routed/tests.rs:118-119 says bytes cross "independently" over a sequential exchange. If link-11 lands, only the epochs doc and the routed comment remain.

Evidence:

    94	/// Backpressure is per-stream and receiver-paced: a writer past the buffer
    95	/// capacity blocks until its own reader drains, then completes.

    113	    .expect("a two-byte window still carries six bytes");

    141	/// Epochs advance one per begun session and wrap: the label tripwire never
    142	/// allocates, only counts. `finish` marks each session's clean end.

    (routed/tests.rs)
    118	            // The control stream is the establishment connection:
    119	            // bytes cross in both directions independently.

Resolution: restate each to what the body checks: "a two-byte window still carries six bytes to completion"; "the control halves carry bytes in both directions, intact and in order"; "Epochs advance one per begun session and wrap at `u8::MAX`; the epoch is a mismatch check, never an identity. `finish` marks each session's clean end."; "bytes cross in both directions". If blocking is meant to be tested, add the observation (`now_or_never` on the over-capacity write before draining). Acceptance: each listed doc names only behavior its body asserts.

### link-13: Dialect tells across the partition's prose: "honest"/"misbehavior" where "conforming" is the claim, "real"/"genuine", and metaphors promoted to jargon
- Where: src/link/routed.rs:68-70 (related: src/link.rs:538, src/link/routed/router.rs:234-235, src/link/routed/stream.rs:91-92, src/link/routed/tests.rs:183-184, src/link/routed/tests.rs:196, src/link/routed.rs:189, src/link/routed/tests.rs:436, src/link/routed.rs:214, src/link/routed/endpoint.rs:30, src/link/routed/header.rs:28-30, src/link/routed/header/tests.rs:58-59, src/link.rs:351, src/link/tests.rs:141)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep `honest|misbehav|\breal\b|genuine` and `seam|knob|compatibility door|tripwire|reusable-lease` over the partition; the listed sites are the complete sets)
- Seen by: prose (17, 18), perfapi (46); refutation: confirmed 17; reframed 18 ("still" at link.rs:150 is logical, not temporal, and is not a finding); confirmed 46 as a subset; history: deliberate-and-holds (every word is the owner's own vocabulary: "honest" is the model of record's name in AGENTS.md, "seam"/"compatibility door" are the design record's, "knobs"/"tripwire" are Finch's)
- Owner-gated: no (owner taste on owner vocabulary; batch for a prose pass)

Two precision points and one taste point. Precision: at routed.rs:69-70 and router.rs:234-235 the claim is about conformance ("never has more than a session's complement in flight"), and a trusted peer can be nonconforming by bug, so "conforming" is the exact word where "honest" (the model's name for the non-adversarial premise) is used; "the real contract" (routed.rs:189) and "the genuine payload" (routed/tests.rs:436) are moralized words with no adversary in sight. Taste: "seam" for a trait boundary, "knobs" for config fields, "compatibility door" for a version byte (twice), "label tripwire" for a mismatch check (twice), and a speculative future kind named in passing ("a reusable-lease kind, say") each have a plain mechanism word; the brief lists them as tells.

Evidence:

    68	//! - A link whose stream queue overflows is evicted wholesale: an
    69	//!   honest peer never has more than a session's worth of streams in
    70	//!   flight, so overflow proves misbehavior (or a local bug), and

    189	/// Blanket-implemented for every type with the bounds; the real

    214	/// This is the adapter's outgoing seam, and the place transport policy

    (endpoint.rs)
    30	/// Capacity knobs of an endpoint's router; [`Config::default`] suits

    (header.rs)
    28	//! with no checker. The version byte is the compatibility door: any
    29	//! future shape (a reusable-lease kind, say) arrives as a new version

Resolution: routed.rs:69-70 and router.rs:234-235: "a conforming peer never has more than a session's complement in flight, so overflow is a conformance bug (the peer's or a local one)"; routed.rs:189: "the contract the bounds cannot express"; routed/tests.rs:436: "the written payload"; routed.rs:214: "the adapter's outgoing boundary"; endpoint.rs:30: "Capacity bounds of an endpoint's router"; header.rs:28-30 and header/tests.rs:58-59: "The version byte is what lets the format change: any future shape arrives as a new version or kind" (drop the hypothetical kind); link.rs:351 and link/tests.rs:141: "a mismatch check rather than an identity". Acceptance: the listed words are gone from the listed lines and their replacements read as mechanism.

### link-14: The routed TCP guidance never mentions `TCP_NODELAY`; the routed shape is the Nagle-plus-delayed-ACK worst case
- Where: src/link/routed.rs:118-125 (related: src/link/routed.rs:193-197, src/link/routed.rs:206-207, src/link/routed.rs:213-216, src/link/routed/stream.rs:51-52, src/tree/mirror/streaming/remote/streams.rs:209, src/tree/mirror/streaming/remote/codec/encode/async_io.rs:80-96, tests/common/routed_tcp.rs:34-43, tests/common/routed_tcp.rs:57-67, tests/common/tcp.rs:50-52)
- Class / severity / confidence: performance / medium / medium
- Provenance: verified (absence: `grep -rn -i 'nodelay|nagle' src tests examples benches justfile Cargo.toml` returns nothing; write structure: `write_encoding` writes the frame head and then zero, one, or two body pieces as separate `write` calls before `flush`, and a stream open writes the 28-byte header at stream.rs:52 and the label at streams.rs:209 as separate writes). The latency magnitude is assessed from the mechanism, not measured; no network experiment was run.
- Seen by: perfapi (34); refutation: confirmed, "medium stands conditionally"; history: no rationale (the design record deprioritized dial cost, not per-frame ACK stalls; no note mentions Nagle)
- Owner-gated: no

The module's TCP example dials a raw `TcpStream::connect`, the `Conn` docs say `tokio::net::TcpStream` satisfies the connection obligations with no caveat, and the `Dial` docs list socket options as the implementation's business without naming the one that governs this protocol's latency. Tokio leaves Nagle enabled by default. A routed data stream is a unidirectional connection carrying frames written as one to three `write_all` pieces each, preceded at open by a header write and a label write; on a unidirectional connection no reverse traffic piggybacks ACKs, so every small piece after the first waits for the peer's delayed-ACK timer (tens of milliseconds on common stacks). The sign is fixed for this shape because `FrameWrite::frame` already flushes at every frame boundary and the `Conn` contract forbids user-space write buffering, so Nagle can only add waiting. Users copy the example as their deployment template.

Evidence:

    118	//! impl Dial for TcpDial {
    119	//!     type Addr = SocketAddr;
    120	//!     type Conn = TcpStream;
    121	//!
    122	//!     async fn dial(&self, addr: &SocketAddr) -> io::Result<TcpStream> {
    123	//!         TcpStream::connect(*addr).await
    124	//!     }
    125	//! }

    206	/// `tokio::net::TcpStream` satisfies both, as does anything else whose
    207	/// writes land in the transport as they are accepted.

    (async_io.rs)
    84	    write(out, FramePart::FrameHead, encoding.head.as_slice()).await?;
    85	    match &encoding.body {
    86	        BodyEncoding::Empty => {}
    87	        BodyEncoding::Listing(listing) => {
    88	            write(out, FramePart::QueryChildren, listing).await?;
    89	        }
    90	        BodyEncoding::Supply { head, run } => {
    91	            write(out, FramePart::SupplyLength, head.as_slice()).await?;
    92	            write(out, FramePart::SupplyRun, run.as_bytes()).await?;
    93	        }
    94	    }

Resolution: in the example `TcpDial::dial` call `stream.set_nodelay(true)?` before returning, and in the example `TcpListen::accept` for the accepted connection (whose writes are the ACK/READY bytes and the control stream's frames). Name `TCP_NODELAY` in the `Dial` docs' policy list (routed.rs:214-216) and add a sentence to the `Conn` docs (routed.rs:193-197) that Nagle is the kernel-side form of the hidden write buffering the clause warns about: liveness survives, latency does not. Apply the same to `tests/common/routed_tcp.rs` (both dials) and `tests/common/tcp.rs` so the crate's own TCP runs measure the intended configuration. Acceptance: the example, the `Dial` docs, and both TCP test harnesses set or name `TCP_NODELAY`; a before/after wall-clock timing of `tests/routed_link.rs`'s pooled mutual-gossip test over loopback shows the per-frame stall gone. If the measurement shows no difference, the finding reduces to the documentation change alone.
Construction: two processes over real TCP with the example `TcpDial`; capture a data-stream connection with tcpdump during a reply phase and observe the gap between the frame-head segment and its body segment equal to the receiver's delayed-ACK timeout; repeat with `set_nodelay(true)` and observe the gap vanish.

### link-15: `Dial::recycle` obligates pooling implementers around a router-written byte whose value the public contract never states
- Where: src/link/routed.rs:252-259 (related: src/link/routed/header.rs:66-75, src/link/routed/header.rs:12-22, src/link/routed/tests.rs:534-535, tests/common/routed_tcp.rs:76-77)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`READY` is `pub(super)` at header.rs:75; the header module's layout block at header.rs:14-22 lists only dialer-to-router bytes; both in-tree pooling dials accept any single byte: routed/tests.rs:534-535 checks only `buf.filled().len() == 1`, routed_tcp.rs:76-77 `read_exact`s one byte)
- Seen by: prose (13); refutation: confirmed, severity down to nit; history: no rationale (b2675dbe's message and doc both say "one byte" without naming the value)
- Owner-gated: no (the "expose `READY`" alternative is in open questions)

The public doc tells a pooling `Dial` to "reuse only connections whose byte has arrived" without saying whether any byte suffices or whether a value must be checked; `READY` is invisible to the implementer, and the wire layout describes only the dialer-to-router direction. A wire signal the implementer must consume is part of the contract. The in-tree implementations take the any-byte reading, which is what the prose implies.

Evidence:

    252	    /// The peer's router writes one byte on the connection once it is
    253	    /// ready for the next stream. A stream sent earlier is not
    254	    /// delivered until the previous stream's consumer lets the
    255	    /// connection go. Consume the byte off the dialing path and reuse
    256	    /// only connections whose byte has arrived, dialing fresh

    (header.rs)
    75	pub(super) const READY: u8 = 2;

Resolution: add to `recycle`'s doc that the byte's value is unspecified and must be consumed and discarded (making the any-byte reading the contract), and add the two router-to-dialer bytes (`ACK`, `READY`) to the header module's layout block so the wire is described in both directions. Acceptance: a reader of `Dial::recycle` alone can write a correct pooling dial without reading `header.rs`; the layout block lists every byte either side writes.

### link-16: Router evictions are unobservable except through the affected link's next error
- Where: src/link/routed/endpoint.rs:52-55 (related: src/link/routed/router.rs:159-163, src/link/routed/router.rs:218-223, src/link/routed/router.rs:239-242, src/link/routed/router.rs:252-257)
- Class / severity / confidence: feature-gap / low / medium
- Provenance: assessed (read every eviction and drop site in router.rs; none counts, logs, or calls out)
- Seen by: perfapi (45); refutation: confirmed; history: no rationale (no note considers router observability; `src/observe.rs` has no router hook)
- Owner-gated: yes: new public surface

The `pending_headers` doc concedes that eviction of an idle recovered connection is silent and advises sizing generously; a stream-queue overflow and a pending-header eviction likewise leave no trace except a later transport error on one link. An operator cannot size `pending_headers` from data, detect a nonconforming peer, or tell a pool-corruption pattern from ordinary churn. "Size generously" is a guess standing in for a measurement. If link-28's structural fix lands, the admitted-connection eviction class disappears and the remaining evictions are conformance-bug signals worth counting.

Evidence:

    52	    /// expects. Eviction of an idle recovered connection is silent: no
    53	    /// invalidation reaches the dialer's pool, and the next stream
    54	    /// drawn on the dead entry fails, or hangs to the caller's session
    55	    /// timeout. Size generously.

Resolution: owner decision on shape. Smallest: a `RouterStats` of atomic counters (headers read, `LINK` accepted, `LINK` rejected, `STREAM` routed, unknown token, queue-overflow evictions, pending-header evictions) shared between the router and `Endpoint::stats(&self)`. Acceptance: `pending_header_bound_evicts_oldest` and `queue_overflow_evicts_the_link` assert the corresponding counter moved from 0 to 1; a counter that no known-bad scenario moves is decoration and is not added.

### link-17: `EndpointError` and `LinkError` lack `#[non_exhaustive]`, as do many other public enums; a crate-wide ruling is missing
- Where: src/link/routed/endpoint.rs:82-83 (related: src/link/routed/endpoint.rs:111-112, src/error.rs:61-63, src/message.rs:117, src/tree/mirror.rs:43, src/tree/mirror/streaming/remote/streams.rs:102)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep census: 24 `non_exhaustive` sites under src, none in src/link; public enums without the attribute include `EncodeError` (message.rs:117), the mirror `Error` (tree/mirror.rs:43), `ReplyFrameError`, `SendError`, `HeadError`, and a dozen codec/adapter error enums)
- Seen by: perfapi (37); refutation: reframed (the "only two" premise is false; crate-wide inconsistency); history: no rationale (both enums predate 5048a368's stated principle that "the crate marks open diagnostic taxonomies non-exhaustive"; neither was ever classified open or closed)
- Owner-gated: yes: crate-wide policy

These two enums are plausible growth points (a `Busy` variant for `Rejected`, see link-18; a new `Config` bound), and the crate has stated a principle for open taxonomies, but the attribute is absent across much of the public error surface, so adding it here alone establishes no convention. Pre-release is the cheap moment to rule crate-wide.

Evidence:

    82	#[derive(Debug, thiserror::Error)]
    83	pub enum EndpointError {

    111	#[derive(Debug, thiserror::Error)]
    112	pub enum LinkError {

Resolution: rule crate-wide which public enums are open taxonomies; if these two are, add `#[non_exhaustive]` to both (the in-crate tests match with `matches!`, unaffected). Acceptance: the ruling is recorded; the two enums carry the attribute or a stated reason not to.

### link-18: `LinkError::Rejected` and `Endpoint::link`'s `# Errors` misstate the causes: the router never "answers" a rejection, and the address-decode mismatch is unnamed
- Where: src/link/routed/endpoint.rs:117-121 (related: src/link/routed/endpoint.rs:246-252, src/link/routed/endpoint.rs:262-270, src/link/routed/endpoint.rs:285-286, src/link/routed/router.rs:159-163, src/link/routed/router.rs:252-257, src/link/routed/router.rs:266-268, src/link/routed/header.rs:312-313, src/link/routed/tests.rs:320-346, src/link/routed/tests.rs:378-389)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (traced every path that ends in `Rejected`)
- Seen by: prose (12), correctness (29b), perfapi (38); refutation: confirmed all three, merged; history: the EOF-to-`Rejected` mapping is deliberate (design record §5 "EOF or a non-ACK byte is a crisp rejection"), the two-cause doc was imprecise from b16a800b, distinguishing busy from foreign was never considered
- Owner-gated: no for the doc fix; the NACK-byte option is a wire decision and is in open questions

The variant says the peer's router "answered but did not accept", yet the router never writes a non-ACK byte in any rejection path: it drops the connection on a full backlog (router.rs:266-268), a dropped `Incoming` (same `try_reserve`), a duplicate token (router.rs:252-257), a pending-header eviction of the `LINK` connection (router.rs:159-163), and any header it cannot parse, including an advertised name the peer's `Addr::decode` returns `None` for (header.rs:312-313). That last cause is what a deployer sees when two endpoints disagree on their `Addr` type or encoding, and neither doc names it. Separately, a full backlog (transient; retry) and a foreign listener (permanent; fix the deployment) reach the dialer identically as `UnexpectedEof`, so the "retry policy is the caller's" it hands over cannot be informed. Public `# Errors` prose must state every arm accurately against the code.

Evidence:

    117	    /// The peer's router answered but did not accept the link: its
    118	    /// application is not accepting links, or the listener is not a
    119	    /// routed-link router at all.
    120	    #[error("the peer's router rejected the link")]
    121	    Rejected,

    248	    /// [`LinkError::Io`] for transport failure, [`LinkError::Rejected`]
    249	    /// when the peer answered without acknowledging (its application's
    250	    /// backlog is full, or the listener does not speak this wire).

    264	            // A clean close or a non-acknowledgement byte is the
    265	            // peer's router declining; transport trouble stays Io.
    266	            Ok(_) => return Err(LinkError::Rejected),
    267	            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
    268	                return Err(LinkError::Rejected);
    269	            }

    (router.rs)
    266	            let Ok(slot) = incoming.try_reserve() else {
    267	                return Ok(());
    268	            };

Resolution: rewrite the variant doc and `Endpoint::link`'s `# Errors` around what the dialer observes (the peer closed the connection, or wrote a byte other than the acknowledgement, before acknowledging) and name the causes the router has: its application's backlog is full or `Incoming` was dropped, the advertised name did not decode at the peer (the two endpoints' `Addr` types or encodings disagree), the router was at its pending-header bound, or the listener is not a routed-link router. Drop "answered". State that the busy and foreign cases are indistinguishable by design unless the owner takes the NACK-byte option. Acceptance: the doc lists no cause the code does not produce and omits none it does; the address-decode cause appears by name; `just doclint` clean.

### link-19: Hand-written `Clone` impls where `#[derive(Clone)]` produces the same impl
- Where: src/link/routed/endpoint.rs:145-151 (related: src/link/routed/stream.rs:37-45, src/link/routed.rs:224, src/link/routed/header.rs:129, src/link.rs:589-590, src/link/erased.rs:114-115)
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (Rust semantics: `Dial: Clone` at routed.rs:224 and `Addr: Clone` at header.rs:129 supply the bounds the derive needs; not compiled)
- Seen by: structure (6), perfapi (43); refutation: confirmed, duplicates merged; history: no rationale (b16a800b, never revisited; `MemoryConnector` and `DynConnector` derive)
- Owner-gated: no

`Endpoint<D: Dial>` (an `Arc<Inner<D>>`) and `StreamConnector<D: Dial>` (fields `D`, `D::Addr`, `Token`) implement `Clone` by hand. The derive adds a `D: Clone` bound that `Dial: Clone` already implies, and `self.peer.clone()` resolves through `Addr: Clone`. Seven lines each that say nothing the derive would not; a reader pauses to look for the reason the derive was avoided and finds none.

Evidence:

    145	impl<D: Dial> Clone for Endpoint<D> {
    146	    fn clone(&self) -> Self {
    147	        Endpoint {
    148	            inner: Arc::clone(&self.inner),
    149	        }
    150	    }
    151	}

    (stream.rs)
    37	impl<D: Dial> Clone for StreamConnector<D> {
    38	    fn clone(&self) -> Self {
    39	        StreamConnector {
    40	            dial: self.dial.clone(),
    41	            peer: self.peer.clone(),
    42	            token: self.token,
    43	        }
    44	    }
    45	}

Resolution: replace both with `#[derive(Clone)]` on the struct definitions (endpoint.rs:141, stream.rs:24). If the derive fails to prove `D::Addr: Clone` on the pinned toolchain, keep the manual impl on `StreamConnector` with a one-line comment saying why. Acceptance: both impl blocks gone; `just clippy` clean; the routed tests pass.

### link-20: `Config`'s two zero-bound errors are what `NonZeroUsize` fields make unrepresentable
- Where: src/link/routed/endpoint.rs:208-213 (related: src/link/routed/endpoint.rs:41, src/link/routed/endpoint.rs:56, src/link/routed/endpoint.rs:100-107, src/link/routed/endpoint.rs:215, src/link/routed/router.rs:141, src/link.rs:556-560, src/link/routed/tests.rs:710-743)
- Class / severity / confidence: idiom / nit / medium
- Provenance: assessed (read; the checks are load-bearing because `tokio::sync::mpsc::channel` panics at capacity zero)
- Seen by: structure (11), perfapi (39); refutation: confirmed both, merged; history: deliberate-and-holds for the typed-error shape (b23d19c0 "every arm stated"; `memory_with_capacity` is the in-crate `usize`-plus-check precedent; R79 ruled against byte-quantity newtypes on `Peer`/`Bootstrap`; `NonZeroUsize` never weighed)
- Owner-gated: yes: public API and a recorded taste ruling nearby

Both fields are counts whose zero value is meaningless, and `Endpoint::new` turns each into an error variant with a test. `NonZeroUsize` fields delete both variants, both checks, their doc paragraphs, and `zero_config_bounds_fail_construction`; the same applies to `memory_with_capacity(capacity: usize)` and its `# Panics`. The named cost: `NonZeroUsize` literals are clumsy at call sites, and the crate's precedent (and R79's spirit) is bare `usize` plus validation. Both shapes are defensible; the current one is correct and documented.

Evidence:

    208	        if config.incoming_backlog == 0 {
    209	            return Err(EndpointError::ZeroIncomingBacklog);
    210	        }
    211	        if config.pending_headers == 0 {
    212	            return Err(EndpointError::ZeroPendingHeaders);
    213	        }

Resolution: owner call. If adopted: `pub incoming_backlog: NonZeroUsize`, `pub pending_headers: NonZeroUsize`, defaults as `NonZeroUsize` constants, delete the two `Zero*` variants, checks, and test, pass `.get()` to the channel constructors; likewise `memory_with_capacity(capacity: NonZeroUsize)`. If declined, no change. Acceptance: decision recorded; if changed, `EndpointError` has two variants and no zero check remains.

### link-21: The dialing side of a routed link cannot observe its `Token`
- Where: src/link/routed/endpoint.rs:253-279 (related: src/link/routed/endpoint.rs:124-133, src/link/routed/header.rs:87-98, src/link/routed/stream.rs:24-35, src/link/routed/stream.rs:66-84)
- Class / severity / confidence: feature-gap / low / high
- Provenance: verified (read endpoint.rs and stream.rs in full: `link` returns `Ok(Link::new(...))` with no token; neither `StreamConnector` nor `StreamAcceptor` has an accessor)
- Seen by: perfapi (36); refutation: confirmed; history: no rationale (the design record fixes that tokens are "observed through LinkInfo" and never discusses the dialer's view)
- Owner-gated: yes: public API change

`Endpoint::link` draws the token and returns only the `RoutedLink<D>`; the accepting side receives `LinkInfo { peer, token }`. The token is the one identifier both processes share for a link, so the dialer cannot log it, correlate a poisoned link with the peer's view, or tell which of several links to the same peer failed. The two ends of one object are asymmetric for no stated reason.

Evidence:

    253	    pub async fn link(&self, peer: D::Addr) -> Result<RoutedLink<D>, LinkError> {
    254	        // Register before the header goes out: the peer's reverse
    255	        // dials can only follow its read of the header, so they always
    256	        // find the token routable.
    257	        let (token, registration, streams) = router::register(&self.inner.table);

    131	    /// The link's routing identity, unique per link on this endpoint.
    132	    pub token: Token,

Resolution: return `(LinkInfo<D::Addr>, RoutedLink<D>)` from `Endpoint::link`, mirroring `Incoming::accept` (the `peer` field echoes the argument), or add `pub fn token(&self) -> Token` on `StreamConnector`/`StreamAcceptor`. The tuple return keeps the ends symmetric and is the smaller surface. Acceptance: `establishment_connects_control_and_streams` asserts the dialer-side token equals `info.token` on the accepting side.

### link-22: `PREFIX_LEN` is `pub(super)` but used only inside `header`, and the layout it heads is spelled three times by offset arithmetic
- Where: src/link/routed/header.rs:59 (related: src/link/routed/header.rs:247-257, src/link/routed/header.rs:260-267, src/link/routed/header.rs:286-300, src/link/routed/header/tests.rs:63, src/link/routed/header/tests.rs:73, src/link/routed/header/tests.rs:86-87)
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (grep: `PREFIX_LEN` outside header.rs appears only at header/tests.rs:86-87, a child module that reaches parent-private items through `use super::*` regardless of visibility; no sibling module names it)
- Seen by: structure (2, 8); refutation: confirmed both (2 down to nit: the round-trip and truncation tests pin encoder/decoder agreement); history: no rationale (b16a800b, unchanged)
- Owner-gated: no

The constant is exported to the parent module with no sibling user, and it carries a bare `2` for the two one-byte fields. The fixed prefix is assembled by `push`/`extend` in `link_header`, by indexed copies at `MAGIC.len()`, `MAGIC.len() + 1`, `MAGIC.len() + 2` in `stream_header`, and parsed by the same arithmetic in `read`; the tests repeat the offsets. Named constants over magic numbers and one spelling per layout; the round-trip tests make this legibility rather than drift risk.

Evidence:

    59	pub(super) const PREFIX_LEN: usize = MAGIC.len() + 2 + TOKEN_LEN;

    262	    bytes[..MAGIC.len()].copy_from_slice(MAGIC);
    263	    bytes[MAGIC.len()] = VERSION;
    264	    bytes[MAGIC.len() + 1] = KIND_STREAM;
    265	    bytes[MAGIC.len() + 2..].copy_from_slice(&token.0);

    295	    let kind = prefix[MAGIC.len() + 1];
    296	    let token = Token(
    297	        prefix[MAGIC.len() + 2..]

Resolution: drop `pub(super)`; name the offsets (`const VERSION_AT: usize = MAGIC.len(); const KIND_AT: usize = VERSION_AT + 1; const TOKEN_AT: usize = KIND_AT + 1; const PREFIX_LEN: usize = TOKEN_AT + TOKEN_LEN;`); add `fn prefix(kind: u8, token: &Token) -> [u8; PREFIX_LEN]` that `stream_header` returns and `link_header` extends from; `read` indexes by the named offsets; the tests at header/tests.rs:63,73 use them too. Acceptance: `PREFIX_LEN` is private; `MAGIC.len() + 1` and `MAGIC.len() + 2` no longer appear in header.rs; the literal `2` is gone; header tests pass unchanged.

### link-23: Encoding is attributed to the router; the endpoint encodes
- Where: src/link/routed/header.rs:186-187 (related: src/link/routed/tests.rs:663-664, src/link/routed/endpoint.rs:204, src/link/routed.rs:17-18)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep `.encode()` under src/link: the only production call is `advertised.encode()?` at endpoint.rs:204 in `Endpoint::new`; the router only decodes, via header.rs:312 from router.rs:216)
- Seen by: prose (14); refutation: confirmed; history: no rationale (3a306e3c's own message used the loose attribution while the call already lived in `Endpoint::new`)
- Owner-gated: no

Both the `impl Addr for SocketAddr` doc and a test doc say construction is "the one place the router encodes"; in this module's own vocabulary the router is the listener-owning loop (routed.rs:17-18), which never encodes. A maintainer-facing attribution naming the wrong component sends the reader to the wrong file.

Evidence:

    186	/// advertised name is a configuration bug surfaced at endpoint
    187	/// construction (the one place the router encodes). A link-local

    (routed/tests.rs)
    663	/// dials a different peer), and construction is the one place the
    664	/// router encodes, so the refusal surfaces exactly here.

Resolution: "(the one place the adapter encodes: `Endpoint::new`)" at both sites. Acceptance: no prose in the partition says the router encodes.

### link-24: `SocketAddr::decode` hand-checks the length and then `expect`s twice
- Where: src/link/routed/header.rs:211-215
- Class / severity / confidence: idiom / nit / high
- Provenance: assessed (read; `rust-toolchain.toml` pins 1.97.1, so `split_first_chunk` is available)
- Seen by: structure (9); refutation: confirmed; history: no rationale
- Owner-gated: no

The manual `len != SOCKET_ADDR_LEN` check followed by two `try_into().expect("length checked above")` can be one fallible split that makes the length check structural and removes both `expect`s. The current messages carry a valid one-line proof, so this is a preference for structural totality over a proven assert.

Evidence:

    211	        if bytes.len() != SOCKET_ADDR_LEN {
    212	            return None;
    213	        }
    214	        let ip: [u8; 16] = bytes[..16].try_into().expect("length checked above");
    215	        let port = u16::from_be_bytes(bytes[16..].try_into().expect("length checked above"));

Resolution: `let (ip, port) = bytes.split_first_chunk::<16>()?; let port: &[u8; 2] = port.try_into().ok()?;` then `Ipv6Addr::from(*ip)` and `u16::from_be_bytes(*port)`; the exact-length requirement becomes the `try_into` on the two-byte tail. `SOCKET_ADDR_LEN` stays for `encode`'s capacity and the tests. Acceptance: no `expect` in `SocketAddr::decode`; the round-trip, flowinfo, and truncation tests pass.

### link-25: `link_header` truncates the advertised-name length with a debug-only guard
- Where: src/link/routed/header.rs:247-256 (related: src/link/routed/endpoint.rs:204-207, src/link/routed/endpoint.rs:259)
- Class / severity / confidence: correctness / nit / high
- Provenance: assessed (read; the single caller passes the construction-validated `encoded`)
- Seen by: correctness (27); refutation: confirmed; history: no rationale (b23d19c0 moved validation to `Endpoint::new` and left the debug guard)
- Owner-gated: no

The length byte is written with `as u8` behind a `debug_assert!`; in release a caller passing more than 255 bytes would emit a truncated length and a header the peer parses as a shorter name followed by garbage. Today the only caller passes validated bytes, so the breach is programmer error, but the guard that says so is not total, and an `as` cast that can truncate is a wrong-behavior path rather than an argued unreachability.

Evidence:

    248	    debug_assert!((1..=MAX_ADDR_LEN).contains(&addr.len()));

    254	    bytes.push(addr.len() as u8);

Resolution: `bytes.push(u8::try_from(addr.len()).expect("the endpoint validates the advertised name's length at construction"));` and drop the `debug_assert!`; or carry the validated encoding as a newtype so the bound is in the type. Acceptance: no `as u8` in header.rs; the `expect` message names the construction-time check.

### link-26: The routing table has no type of its own: the token claim is spelled twice, two clippy allowances stand in for a newtype, and a maintainer comment's premise is false as written
- Where: src/link/routed/router.rs:102-115 (related: src/link/routed/router.rs:46-69, src/link/routed/router.rs:249-260, src/link/routed/router.rs:58-59, src/link/routed/endpoint.rs:214, src/link/routed/tests.rs:188-232)
- Class / severity / confidence: modularity / medium / high
- Provenance: assessed (read both spellings, the alias, `entries()`, both `#[allow(clippy::type_complexity)]`, and the constructor at endpoint.rs:214)
- Seen by: structure (0), prose (23), perfapi (44); refutation: confirmed 0 (severity to low; noted the untested outbound-queue capacity), confirmed 23 (subsumed), reframed 44 (subsumed: a second alias to dodge the lint is the coining doctrine discourages); history: no rationale (both spellings from b16a800b; the allowances arrived with 4be4b830)
- Owner-gated: no

Claiming a token (create the `STREAM_COUNT`-bounded channel, insert the sender if the token is vacant, wrap in a `Registration`) is written once for outbound links in `register`, with an `Option`/`take()`/`expect` dance because the channel is created outside the loop, and once more, differently, in `deliver`'s `Link` arm, which holds the guard across `contains_key` then `insert`. That second spelling falsifies the premise of `entries`' doc ("Every critical section is a single map operation"); neither operation panics, so the conclusion holds, but the proof obligation as written is unmet. `Table<C>` is a semantic alias, `entries()` and `register` each carry `#[allow(clippy::type_complexity)]`, and `Endpoint::new` spells `Arc::new(Mutex::new(HashMap::new()))`, all symptoms of the table having no type. Newtypes over type synonyms; one spelling of one invariant. Drift exposure: `queue_overflow_evicts_the_link` floods only the inbound-created queue, so a capacity divergence in `register`'s copy would pass the suite today. I hold this at medium rather than the refutation's low because the false comment premise is a standing maintainer-facing inaccuracy, not only taste, and the same change fixes it.

Evidence:

    58	/// Every critical section is a single map operation, so a panic
    59	/// elsewhere cannot leave the map torn; continuing lets the surviving

    105	    let (sender, receiver) = mpsc::channel(STREAM_COUNT);
    106	    let mut sender = Some(sender);
    107	    let token = loop {
    108	        let token = Token::new();
    109	        if let Entry::Vacant(vacancy) = entries(table).entry(token) {
    110	            vacancy.insert(sender.take().expect("the loop ends at the first vacancy"));
    111	            break token;
    112	        }
    113	    };

    249	            let (sender, receiver) = mpsc::channel(STREAM_COUNT);
    250	            {
    251	                let mut entries = entries(table);
    252	                if entries.contains_key(&token) {
    253	                    // A duplicate establishment is a peer bug (tokens
    254	                    // are drawn fresh per link); dropping it leaves
    255	                    // the live link undisturbed.
    256	                    return Ok(());
    257	                }
    258	                entries.insert(token, sender);
    259	            }

Resolution: introduce `pub(super) struct Table<C>(Arc<Mutex<HashMap<Token, mpsc::Sender<(C, Done<C>)>>>>)` with `Default`, `Clone`, and methods: `fn claim(&self, token: Token) -> Option<(Registration<C>, mpsc::Receiver<(C, Done<C>)>)>` (creates the channel under one `Entry` critical section, `None` if occupied), `fn route(&self, token: &Token) -> Option<mpsc::Sender<..>>`, and `fn revoke(&self, token: &Token)` used by `Registration::drop`. `register` becomes `loop { let token = Token::new(); if let Some(claimed) = table.claim(token) { return (token, claimed); } }`; `deliver`'s `Link` arm becomes `let Some((registration, receiver)) = table.claim(token) else { return Ok(()); };`. `entries()` moves inside as a private method; both allowances go; `Endpoint::new` calls `Table::default()`; the `entries` doc premise becomes literally true. Add an overflow witness for the outbound (`register`-created) queue. Acceptance: `mpsc::channel(STREAM_COUNT)` and `Registration::new` each appear once in router.rs; no `#[allow(clippy::type_complexity)]` remains; endpoint.rs no longer names `HashMap`/`Mutex`; every guard scopes exactly one map call; `just gate` clean.

### link-27: Per-connection micro-costs in the router and stream connector, all strict deletions
- Where: src/link/routed/router.rs:152-157 (related: src/link/routed/router.rs:146, src/link/routed/router.rs:159-163, src/link/routed/router.rs:172, src/link/routed/router.rs:274, src/link/routed/stream.rs:53-55)
- Class / severity / confidence: performance / nit / high
- Provenance: assessed (read)
- Seen by: perfapi (40); refutation: confirmed, severity down to nit (each is paid beside a dial per open); history: no rationale
- Owner-gated: no

Three fixed-sign deletions: (a) the drive loop retires a finished header read with `order.retain(..)`, a linear scan over up to `pending_headers` entries, per accepted connection, and `pending_headers` is the bound the docs tell users to size generously; ids are monotonic, so a `BTreeMap<u64, AbortHandle>` (`pop_first` to evict, `remove(&id)` to retire) is both O(log n) and simpler than a `VecDeque` plus `retain`. (b) `dial.clone()` at router.rs:172 runs for every accepted connection but only the `Header::Link` arm uses it. (c) `StreamConnector::connect` clones `dial` and `peer` per open to build the recycle `Done`. Magnitude is small at defaults; (a) is the one that grows with a user-chosen constant.

Evidence:

    152	            Some(finished) = pending.next() => {
    153	                if let Ok(id) = finished {
    154	                    order.retain(|(pending_id, _)| *pending_id != id);
    155	                }
    156	                continue;
    157	            }

Resolution: (a) `order: BTreeMap<u64, AbortHandle>`; evict with `pop_first()`, retire with `remove(&id)`. (b) pass `&dial` into `deliver` and clone only in the `Link` arm. (c) optional: hold `Arc<(D, D::Addr)>` in `StreamConnector`. Acceptance: behavior-preserving; `pending_header_bound_evicts_oldest`, `stalled_header_does_not_park_the_router`, `queue_overflow_evicts_the_link` pass unchanged; no new meter warranted at this magnitude.

### link-28: The router's single count bound evicts a pooling dialer's already-admitted idle connections first, and the default is sized for the non-pooling case
- Where: src/link/routed/router.rs:159-163 (related: src/link/routed/router.rs:135-141, src/link/routed/router.rs:151, src/link/routed/router.rs:213-215, src/link/routed/router.rs:229-231, src/link/routed/endpoint.rs:42-56, src/link/routed/endpoint.rs:63-65, src/link/routed.rs:74-79, src/link/routed.rs:246-250, src/link/routed/tests.rs:498-596, tests/common/routed_tcp.rs:49-86)
- Class / severity / confidence: correctness / medium / high
- Provenance: assessed (traced by reading: a completed inbound stream's `Done` returns its connection at router.rs:229-231, the drive loop pulls it at 151, `deliver` writes `READY` at 213-215 before the header read begins, so the connection then sits in `order` beside fresh mid-header arrivals and is evicted oldest-first at 159-163). Not run.
- Seen by: correctness (26, and 30c for the missing witness); refutation: confirmed step by step, severity unchanged, plus two new observations folded in below; history: already-known (4be4b830 added both the recovered-connection population and the "Size generously" paragraph; b2675dbe made `READY` precede the header read; landed via PR #13 with no owner ruling; `DEFAULT_PENDING_HEADERS`'s rationale dates from b16a800b and was never re-denominated)
- Owner-gated: yes: design change to the adapter; the design record's DECIDED 2026-07-29 entry deferred pooling to the `Dial` without pricing this interaction

The router keeps one bound (`pending_headers`) over two populations with opposite eviction preferences: fresh connections stalled mid-header (the hygiene target) and recovered idle connections a pooling `Dial` has already admitted, because `READY` is written before the header read. Eviction is oldest-first, so admitted idle connections predating a fresh stalled arrival go first, and each link can park up to `STREAM_COUNT` recovered connections per session, so `DEFAULT_PENDING_HEADERS = 64` overflows at four pooling peers in ordinary operation. An evicted admitted connection is discovered only by the next stream drawn on it: over memory the `STREAM` header write fails with `BrokenPipe` and `connect` errors on a link whose peer is healthy; over TCP a small stream's header, label, frame, and end control can all land in the socket buffer before the RST, so the dialing session sees no failure and the peer's session waits for a stream that never arrives, until the caller's timeout. `Config::pending_headers`'s own doc names that outcome and defers it to the caller; `DEFAULT_PENDING_HEADERS`'s rationale speaks only of "simultaneous dials". Two riders: the module doc at routed.rs:77-79 says a dead pooled connection "surfaces as the recycled stream's transport failure and heals at session granularity", which understates the hang its own `Config` doc names; and the `returns` channel's comment at router.rs:135-140 describes a bound ("bounded by the same budget as the pending reads it feeds") that never constrains the idle population, because the drive loop frees each slot as it pulls the connection. Correct at all scales: peer count is an input, and the crate's own shape for resource pressure is "degradation is latency, never deadlock". A documented sizing duty is a hand-maintained invariant where a structural one is available: a pool should never be able to admit a connection the router will not serve.

Evidence:

    159	        if order.len() >= pending_headers
    160	            && let Some((_, oldest)) = order.pop_front()
    161	        {
    162	            oldest.abort();
    163	        }

    213	    if recovered {
    214	        conn.write_all(&[header::READY]).await?;
    215	    }

    135	    // Connections coming back from completed streams, to await their
    136	    // next header beside fresh arrivals. The queue is bounded by the
    137	    // same budget as the pending reads it feeds, and its send never
    138	    // blocks: a return finding it full is dropped, the eviction the
    139	    // pending bound would deal it anyway. (Declared before `pending`,

    (endpoint.rs)
    52	    /// expects. Eviction of an idle recovered connection is silent: no
    53	    /// invalidation reaches the dialer's pool, and the next stream
    54	    /// drawn on the dead entry fails, or hangs to the caller's session
    55	    /// timeout. Size generously.

    63	/// Default [`Config::pending_headers`]: comfortably past a full
    64	/// session complement of simultaneous dials from several peers.
    65	const DEFAULT_PENDING_HEADERS: usize = 64;

    (routed.rs)
    77	//!   the caller's [`Dial`] — as does discovering that a pooled
    78	//!   connection died while idle, which surfaces as the recycled
    79	//!   stream's transport failure and heals at session granularity.

Resolution: separate the two populations. Give recovered connections their own bound and apply it before `READY` is written: a return that finds the recovered budget full is dropped pre-`READY` (the dialer's pool then never admits it, since the pre-`READY` drop path already exists in embryo at router.rs:229-231), and an admitted connection is thereafter closed only by the dialer or by transport failure, never by count. The smallest form is a counter of recovered connections currently in `pending`, checked before the `READY` write. Keep oldest-first count eviction for fresh mid-header connections only, whose dialer is still inside its open and observes the drop as a failed open. Make routed.rs:77-79 and endpoint.rs:52-55 agree whichever design is chosen. If the owner prefers one bound, at minimum re-denominate `DEFAULT_PENDING_HEADERS`'s rationale for the pooled idle population and land the constructed test as the documented failure's witness. Acceptance: a committed test with a pooling `Dial` in which recovered idle connections exceed `pending_headers` completes a later session with no failed or hung stream (new design), plus a negative control showing a pre-`READY` drop leaves the pool without that connection; or, under the current design, the constructed test pins the failure and the default's doc states the pooled sizing rule.
Construction: in src/link/routed/tests.rs with the `PoolingDial` fixture (498-578) and `settle()` (582-596): build endpoint `a` with `Config { pending_headers: 1, ..Config::default() }` and `b` with the default; `establish(&b, "a", ..)`; `transfer_completed(&at_b, &mut at_a, ..)` once; `settle()` so `a`'s router writes `READY` and `admit` pools the connection; dial one raw connection to `a` with `net.dial()` and write `b"ROU"` (stalls mid-header; as the newer arrival it evicts the recovered connection, the oldest, dropping the router's `DuplexStream` end); then `at_b.connector.connect()`: the pool hands out the dead connection, the `STREAM` header write returns `BrokenPipe`, and `connect` returns `Err` on a link whose peer is healthy. For the hang variant, the same shape with `PoolingTcpDial` (tests/common/routed_tcp.rs:49-86), a small mutual gossip session, and `tokio::time::timeout`.

### link-29: The router's read-id counter should wrap rather than overflow
- Where: src/link/routed/router.rs:165-166 (related: src/link/routed/router.rs:143-147, src/link/routed/router.rs:152-155)
- Class / severity / confidence: correctness / nit / high
- Provenance: assessed (read)
- Seen by: correctness (28); refutation: confirmed; history: no rationale
- Owner-gated: no

`next_id += 1` on a `u64` panics in debug and wraps in release after 2^64 arrivals. That is the tolerated infeasible-work corner, but even there the doctrine picks the most benign behavior; wrapping is correct because ids need only be distinct among the at most `pending_headers` live entries in `order` (saturating would collide every id at `u64::MAX` and make `retain` retire all of them).

Evidence:

    165	        let id = next_id;
    166	        next_id += 1;

Resolution: `next_id = next_id.wrapping_add(1);` with a one-line comment that at most `pending_headers` ids are live at once. Acceptance: no unchecked increment on the id counter.

### link-30: `route` exists only to discard `deliver`'s error and echo an id
- Where: src/link/routed/router.rs:188-202 (related: src/link/routed/router.rs:164-179)
- Class / severity / confidence: simplification / nit / high
- Provenance: assessed (read)
- Seen by: structure (7); refutation: confirmed, with a mechanical note (a non-`move` async block at the push site would capture the loop-local `id` by reference, so the working form is `deliver(..).map(move |_| id)`); history: no rationale
- Owner-gated: no

`route` duplicates `deliver`'s six-parameter signature, adds a seventh, and its whole body is `let _ = deliver(..).await; id`. The id is threaded through only so the drive loop can retire the matching abort handle; a helper with one caller reads better inlined at the push site where the id is created.

Evidence:

    188	async fn route<D: Dial>(
    189	    conn: D::Conn,
    190	    recovered: bool,
    191	    dial: D,
    192	    table: &Table<D::Conn>,
    193	    incoming: &mpsc::Sender<Arrival<D>>,
    194	    returns: &mpsc::Sender<D::Conn>,
    195	    id: u64,
    196	) -> u64 {
    197	    // A failure here is a connection that never became anyone's
    198	    // stream: the dialer observes the drop as transport failure, and
    199	    // there is no one else to tell.
    200	    let _ = deliver(conn, recovered, dial, table, incoming, returns).await;
    201	    id
    202	}

Resolution: at router.rs:168-179, `pending.push(Abortable::new(deliver(conn, recovered, dial.clone(), &table, &incoming, &returns).map(move |_| id), registration));`, moving the discarded-error comment to that site; delete `route`. Acceptance: `route` is gone; `deliver` is the only per-connection async fn in router.rs.

### link-31: The routed-adapter roster never constructs the accepted maximum advertised name
- Where: src/link/routed/tests.rs:694-708 (related: src/link/routed/endpoint.rs:205, src/link/routed/header.rs:304-311)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (grep: the only `MAX_ADDR_LEN` use in tests is `"x".repeat(header::MAX_ADDR_LEN + 1)` at routed/tests.rs:696; no test constructs or parses a 255-byte name)
- Seen by: correctness (30a); refutation: confirmed (30b, `link()` over a recycled connection, judged low value because the router path after `READY` is identical for both header kinds; 30c is link-28's construction)
- Owner-gated: no

`out_of_bound_advertised_name_fails_construction` tries 0 and `MAX_ADDR_LEN + 1`, so the inclusive upper bound is never exercised: a 255-byte name is never accepted at endpoint.rs:205 nor parsed through `vec![0; len]` at header.rs:310 and dialed back. Boundary roster: empty, one, capacity, capacity plus one.

Evidence:

    696	    for name in [String::new(), "x".repeat(header::MAX_ADDR_LEN + 1)] {

Resolution: add a test that constructs an endpoint with a 255-byte `MemoryName`, establishes a link toward it from a peer, and asserts the peer's reverse `transfer` dials the full name (the `LINK` header round-trips the maximum). Acceptance: a new test in src/link/routed/tests.rs with a doc comment stating the boundary; the gate stays green.
Construction: `MemoryName::new("x".repeat(header::MAX_ADDR_LEN))` as `a`'s advertised name; `establish(&b, <that name>, &mut a_incoming)`; assert `info.peer` equals the full name and `transfer(&at_a, &mut at_b, ..)` succeeds (the reverse dial uses the name carried in the header).

## Positives

- The contract section (link.rs:24-68) states, for every clause, the mechanism, the failure it precludes, and the concrete carrier shape that would violate it (half-duplex turn protocols, close-credited pools, buffered writers), so an implementer knows what to build without reading the protocol; and every clause has a conformance check with a committed negative control.
- Routing revocation is ownership, not protocol: `Registration`'s `Drop` (router.rs:90-94), held by `StreamAcceptor` (stream.rs:66-70), means a dropped link stops routing at that instant with no teardown API, on every early return of `deliver` too; `dropping_a_link_revokes_its_token` pins exactly that.
- `Done<Half>` (link.rs:171-199) is one small mechanism that lets single-use transports write `Done::discard()` and recycling transports hand the connection back, and it composes through the erasure layer without a second concept: `Bundle<H>` keeps the concrete half and its `Done` together so completion through the box reaches the transport's own recovery hook (erased.rs:28-87).
- The router's three disciplines are stated once (router.rs:9-27) and each is visible in the code as written: `try_send` for delivery, `try_reserve` for the backlog, `try_send` for returns, the table mutex never held across an await, and all per-connection I/O inside abortable futures evicted oldest-first by count.
- Every `expect`/`unreachable!` in the partition's production code argues its unreachability from a check on the preceding lines: "length checked above" (header.rs:214-215), "prefix layout fixes the token width" (header.rs:299), "the loop ends at the first vacancy" (router.rs:110).
- `SessionState` is sealed exactly right for a value wrappers must carry: `Copy`, private fields, public getters, `pub(crate)` mutators (link.rs:345-407); the poison latch is set in `begin` before any wire traffic and cleared only by `finish`; and `LinkParts::session`'s doc (link.rs:491-498) explains the hazard of substituting a stale copy.
- The explicit-dispatch comment at erased.rs:158-161 documents a non-obvious recursion trap (method syntax would resolve to the blanket `AcceptDyn` impl) at the one site where a later reader would otherwise reintroduce it.
- Hazard sections are complete where they exist: `Endpoint::new`'s `# Errors` names every arm with its trigger (endpoint.rs:175-190); `Listen`'s `# Cancel safety` matches the router's `select!` re-creation exactly (routed.rs:277-281); `memory_with_capacity`'s `# Panics` carries the one-line reason (link.rs:556-558).
- The advertised name is encoded and length-validated once at construction (endpoint.rs:159-161, 204-207); `stream_header` returns a stack array (header.rs:260-267), so a routed stream open allocates nothing in the adapter itself.
- Test hygiene is strong: every test carries an invariant docstring; `truncation_is_rejected` sweeps every prefix length rather than sampling one; `socket_addr_roundtrips` is a proptest that states its canonicalization caveat rather than overclaiming equality; the routed tests run under `pollster` with the routers as a `select` arm so a router that resolves is itself a failure (routed/tests.rs:38-48), and they are shaped as negative controls for every failure mode the module docs list.
- No vestige of the V1 protocol, BLAKE3, or the height/item erasure work exists in this partition; the link layer is protocol-agnostic and reads as written today.

## Open questions for Finch

- Is a pooling `Dial` a first-class deployment shape for the routed adapter, or an opt-in optimization whose sizing duty the caller accepts? This decides link-28's shape. Recommendation: first-class; the design record already names `Dial` as the pooling boundary, and a recovered-connection budget applied before `READY` is a small change that removes a whole failure class.
- Should the router distinguish a full backlog from a foreign listener with a reason byte in place of the single `ACK` (link-18)? Pre-release the wire is free to change and the byte costs one octet per link. Recommendation: only if a caller wants different retry policy for the two; otherwise document the conflation as deliberate.
- Should `READY` (and `ACK`) become public constants so pooling implementers can validate the byte, or should the public contract state that any single byte is the ready signal (link-15)? Recommendation: state the unspecified-value contract; do not widen the surface.
- Which site is the statement of record for the session promise's three qualified exceptions: `Link#what-a-session-promises` (the anchor every cross-reference targets) or `Rumors::gossip`? 3f33afab deliberately states the third at every reachable surface. Recommendation: keep `Link` canonical, trim `Rumors::gossip` to a one-line summary plus the anchor, and drop the "stated where they arise" clause either way (link-1 handles the clause).
- The "+0.7 GiB of rustc peak memory" figure (erased.rs:6, conformance/backend/tests.rs:67) was ruled inline (punch list item 3) and was measured on stable 1.96.1; the pin is now 1.97.1 and no meter re-measures it. Recommendation: soften to the structural claim ("a second full protocol tower per instantiation") in rustdoc and keep the number in the decision record, or add a memwatch-style recipe that re-measures it on toolchain bumps.
- Keep `LinkParts` for representation freedom, or publish `Link`'s fields and dissolve the twin (link-9)? Recommendation: dissolve, pre-release, unless a representation change for `Link` is on the horizon; either way, state the reason in `Link`'s doc.
- `NonZeroUsize` for `Config`'s counts and `memory_with_capacity` (link-20)? Recommendation: leave as is, consistent with the existing `usize`-plus-check shape and R79's spirit.
- A crate-wide `#[non_exhaustive]` ruling for public enums (link-17): which taxonomies are open? Recommendation: rule once, pre-release, and apply mechanically.
- Should `MEMORY_STREAM_CAPACITY` be public so docs can cite it, or is the fixed size an implementation detail the docs should stop quoting (link-1)? Recommendation: stop quoting it.
- `starved_pool_degrades_latency_not_liveness`'s doc (conformance/link/tests.rs:1026-1027, another partition) points at link.rs:107-113 by description ("the link docs' measured-tolerance sentence"); if link-2's restatement lands, that pointer should name what it pins in its own words.

## Dropped

- Blanket `impl Acceptor for &mut A` has no use site (structure 3): refuted. `AcceptDriver::new` takes `acceptor: A` by value with `A: Acceptor` (streams.rs:660-668) and `streams/tests.rs` passes `&mut b.acceptor` at lines 85, 161, 209, 398, 432, which type-check only through this impl; the history pass's "no consumer" grep missed those sites.
- The scoped-IPv6 refusal rationale is written out at three public sites (prose 25): below the bar. One full statement (header.rs:180-189) plus two abbreviated pointers at the altitude of an error variant and a constructor's `# Errors`, both of which must keep every arm named per b23d19c0.
- `Endpoint::link` over a recycled connection is never exercised (correctness 30b): below the bar. The router-side path after `READY` is identical for `LINK` and `STREAM` headers, so the shape adds little over the existing `STREAM` coverage.
- The "+0.7 GiB" figure at a declaration site (prose 16, figure half): recorded owner ruling (punch list item 3, executed in 33bd7d56); moved to open questions with the toolchain-drift observation.
- "still" at link.rs:150 as a temporal marker (prose 18, one item): misread; "The core still ships no network code" is the logical "nonetheless".
- "item-granular" as undefined jargon (prose 22, one item): deliberate owner vocabulary (5048a368 added the buffering sentence with its rationale; "item" is the CBOR wire's established term).
- Manual `Clone` impls (perfapi 43): duplicate of link-19.
- `Config` zero bounds as runtime validation (perfapi 39): duplicate of link-20.
- "never allocates" testdoc (correctness 32): duplicate of link-12.
- Dialect tells in two public summaries (perfapi 46): duplicate of link-13.
- `STREAM_COUNT` derivation pinned literal-to-literal (correctness 33): merged into link-3.
- `Endpoint::link` under-enumerates `Rejected` (correctness 29b) and `Rejected` conflates busy with foreign (perfapi 38): merged into link-18.
- `entries` doc premise false for the `LINK` arm (prose 23) and `register`'s `Option::take().expect` dance (perfapi 44): subsumed by link-26's `Table` newtype.
- `Connector::connect` lacks `# Cancel safety` (perfapi 41) and `Endpoint::link` lacks it (correctness 29a): merged into link-4.
- `PREFIX_LEN` visibility (structure 8): merged into link-22.
- Control-half erasure aliases live in gossip.rs (structure 10) and the erasure argument written twice (prose 16, duplication half): merged into link-10.
- Encoding attributed to the router at the test site (prose 14, second site): folded into link-23.
- Module-doc "cheap", the blank line at 376/377 (prose 22), "8 KiB" (prose 20), and "stated where they arise" (prose 24 residue): batched into link-1.
