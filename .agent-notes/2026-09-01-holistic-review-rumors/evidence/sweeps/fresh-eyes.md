# Sweep fresh-eyes: Fresh-eyes user: a scratch application from the public docs alone

## Method and coverage

The sweep read the crate as a first-time user (README, crate docs, tutorial,
every `pub` item's rustdoc) and wrote a 500-line application against those
docs alone; its artifacts sit in the shared scratchpad under
`sweeps/fresh-eyes/` (`check-1.log`, `run.log`, `scratch-app/`). This pass
disputed each of its ten findings against the tree at 9e5784fb (clean
working tree, verified with `git rev-parse HEAD` and `git status`):

- Opened every cited site with line numbers (`sed -n` ranges, `cat -n`) and
  quoted from the file, never from the sweep's report.
- Grepped use sites for every identifier a finding turns on (`STREAM_COUNT`,
  `Protocol`, `selected`, `BufReader`, `Bookmark for`, `fn rumors(`,
  `unnameable_types`, `SCOPE_ENVELOPE_BYTES`, `seam`).
- Read history where a finding might be a recorded ruling: `git show
  368da2a5` (the V1 retirement) and its message, `git log -S` on the
  affected phrases, `git show b16a800b5:src/peer.rs` (the routed-link
  commit), and the notes `.agent-notes/2026-09-01-v1-retirement/README.md`,
  `.agent-notes/2026-08-20-cbor-wire-review/REVIEW.md` (behavioral note
  N1), and `.agent-notes/2026-07-22-sync-budget/sync-budget.md`.
- Corroborated the sweep's two compiler probes against its own log:
  `check-1.log:77` carries `error[E0603]: module `snapshot` is private` and
  `check-1.log:88` carries `error[E0599]: no method named `protocol` found
  for struct `Peer<T, B>``. Its runtime observations (`run.log`) were read,
  not reproduced.
- Ran no cargo, just, or test command. Both permitted `nextest` invocations
  went unused: no finding rests on a runtime claim that reading did not
  settle (the one candidate, the stream count, is two literal constants and
  a pin test read in `src/link/tests.rs`).

What this pass could not see: docs.rs rendering. Every "reachable from the
public API" claim is inferred from `pub`/`pub(crate)` visibility and `mod`
privacy in `src/lib.rs`, not from a rendered page.

Outcome: all ten sweep findings survive. Two are widened (fresh-eyes-2 gains
sites and a recorded-ruling history; fresh-eyes-5 gains its cause), two are
reframed (fresh-eyes-7's mechanism and history; fresh-eyes-8's class), and
one nit is new (fresh-eyes-11). No correctness defect was found under this
lens.

## Findings

### fresh-eyes-1: reconciliation.rs and link.rs give different stream counts
- Where: src/reconciliation.rs:201-205 (related: src/link.rs:161-169, src/link.rs:93-97, src/reconciliation.rs:184, src/observe.rs:182-184, src/tree/mirror/streaming/remote/codec/signal.rs:31-32)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (read `pub const STREAM_COUNT: usize = 17;` at link.rs:169 and `pub const COUNT: u8 = 17;` on the codec's `Stream` newtype at signal.rs:32, whose indices `0..COUNT` are the data streams the session opens through `Connector`/`Acceptor`; the control stream is the separate `Link` halves; the pin test `stream_count_matches_the_codec` in src/link/tests.rs asserts the two constants equal; read, not run)
- Verification: confirmed; history: no-rationale-found (reconciliation.rs:184 already states the bound as `STREAM_COUNT` per direction; the paragraph at 201-205 is a second, unpinned derivation that arrives at a different total)
- Owner-gated: no

The reconciliation page says a side needs at most 17 streams in total, 16
data plus one control; `STREAM_COUNT` and the codec say 17 data streams per
direction beside the control stream. A transport author sizing a pool from
the reconciliation page under-provisions by one stream per direction, and
the `+ 1` in link.rs's pool formula reads as double counting under the
reconciliation page's account.

Evidence:

    src/reconciliation.rs
    201	//! In *theory*, the maximum number of streams needed on either side of the link
    202	//! is 17, though in practice, far fewer will ever be needed. Why 17? A 32-byte
    203	//! key gives the descent 32 levels; the schedule of traversal asks each side to
    204	//! hop down the tree by 2 levels at a time, so at most 16 data streams plus a
    205	//! control stream are ever needed.

    src/link.rs
    161	/// Logical data streams a session may open in one direction.
    162	///
    163	/// The protocol never opens more, and instantiations must admit this many
    164	/// concurrently (per direction, plus the control stream). The value is the
    165	/// protocol's own, fixed by its wire schedule (the descent's 32 tree
    166	/// heights at a two-height stride per stream, plus the shared opening
    167	/// stream: `ceil(32 / 2) + 1 = 17`) and pinned against the wire codec by
    168	/// test, so it cannot drift silently.
    169	pub const STREAM_COUNT: usize = 17;

    src/link.rs
    93	//! - Size the pool to at least **([`STREAM_COUNT`] + 1) × B** per
    94	//!   direction, where B is the per-stream buffering the transport grants:
    95	//!   every data stream plus the control stream (the +1), each sitting
    96	//!   full at the same moment.

    src/tree/mirror/streaming/remote/codec/signal.rs
    31	    /// Logical streams multiplexed into each transport direction.
    32	    pub const COUNT: u8 = 17;

Resolution: Rewrite reconciliation.rs:201-205 to state that a session opens
at most [`STREAM_COUNT`] data streams per direction beside the persistent
control stream, linking the constant for the derivation, and delete the
"16 data streams plus a control stream" arithmetic; one derivation, at the
constant. Acceptance: `grep -n "16 data\|is 17" src/reconciliation.rs` is
empty and the page states the bound only through `STREAM_COUNT`.

### fresh-eyes-2: Docs tell users to "select" a Protocol, but no API selects one
- Where: src/error.rs:14 (related: src/error.rs:76, src/error.rs:179, src/error.rs:201, src/protocol.rs:1, src/peer.rs:603-604, src/tree/mirror/handshake.rs:180, src/tree/mirror/handshake.rs:204, src/peer/gossip.rs:723-729)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (the sweep's probe `check-1.log:88`: `error[E0599]: no method named `protocol` found for struct `Peer<T, B>``; `git show 368da2a5` deletes three `pub fn protocol(mut self, protocol: Protocol) -> Self` setters from src/peer.rs, src/peer/bootstrap.rs and src/rumors.rs; `grep -rn Protocol src` finds no setter, and src/tree/mirror/handshake.rs:102, 125, 127 hard-code `Protocol::V2`; `Reconciliation` at gossip.rs:1080 has one method, `reconcile`)
- Verification: confirmed and widened (two more public sites at error.rs:179 and :201, and a private comment at gossip.rs:723-729 that still describes two protocol branches); history: deliberate-but-expired (the knob was removed by ruling: `.agent-notes/2026-09-01-v1-retirement/README.md` decision 2 and commit 368da2a5, whose message reads "a knob with one position; it returns with a V3"; the vocabulary survived the ruling. The one-variant `Protocol` enum itself is deliberate by that same message, so the fix is wording, not the type)
- Owner-gated: no

Four public pages and one Display message describe a protocol the user
"selected", and the `VersionMismatch` remedy is to "select the same
`Protocol` at both ends". Nothing selects one: the setters are gone and the
handshake hard-codes `Protocol::V2`. A user handed the error is given an
action the API does not afford, and a maintainer reading gossip.rs:723-729
is told about two branches that no longer exist.

Evidence:

    src/error.rs
    14	//! | [`Error::VersionMismatch`] | unchanged | select the same [`Protocol`] at both ends; if both already do, the selected protocol's wire version differs across the two releases: align crate versions |
    76	    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]
    179	    /// The peer opened as a rumors stream of the selected dialect, but a
    201	        /// The selected dialect's full preamble width.

    src/protocol.rs
    1	//! Selectable wire reconciliation protocols.

    src/peer.rs
    603	    /// is therefore a fleet-coordinated configuration event, like
    604	    /// changing the selected [`Protocol`](crate::Protocol), never a

    src/peer/gossip.rs
    723	        // Reconcile using this peer's selected protocol. Both branches meet at
    724	        // the lifecycle boundary the surrounding transaction needs: a local
    725	        // root plus raw transport halves positioned after reconciliation.
    726	        // The protocol bodies live behind the non-generic [`Reconciliation`],
    727	        // whose methods return their futures boxed: neither concrete
    728	        // protocol state machine becomes part of this outer session future,
    729	        // or of the consumer crate that instantiates it.

    .agent-notes/2026-09-01-v1-retirement/README.md (the ruling)
    decision 2 — remove the `.protocol()` builders (done; ...)
    ... (rewrite as the design rationale it is: the level-synchronous shape described as
    the naive alternative, not as a shipped selectable dialect)

Resolution: error.rs:14: keep only the second clause ("the two ends run
releases whose wire versions differ: align crate versions"). error.rs:76 and
handshake.rs:180: name the local wire version ("we speak
{local_protocol:?}"). error.rs:179, :201 and handshake.rs:204: "the
dialect" or "the wire's". protocol.rs:1: drop "Selectable" ("The wire
reconciliation protocol."). peer.rs:603-604: compare to a different
fleet-coordinated event, or cut the comparison. gossip.rs:723-729: state the
one body ("`Reconciliation::reconcile` returns its future boxed so the
protocol state machine stays in this crate's object code") and delete "Both
branches" and "neither concrete protocol state machine". Acceptance: `grep
-rn "select" src/protocol.rs src/error.rs src/peer.rs src/peer/gossip.rs
src/tree/mirror/handshake.rs` matches only settings setters (the
`payload_depth_limit` line at error.rs:109 and the builder docs), never
`Protocol`.

### fresh-eyes-3: sync_memory_budget rustdoc cites test files, a test fn, and internal concepts
- Where: src/peer.rs:393-457 (related: src/peer.rs:313, src/peer.rs:318, src/tree/mirror/streaming/window.rs:273-274, src/reconciliation.rs:248-249, src/snapshot.rs:4)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (the four cited files exist under tests/; `default_crossover_matches_the_solve` is at src/tree/mirror/streaming/window/tests.rs:278; `SCOPE_ENVELOPE_BYTES` is `pub(crate)` at window.rs:265 while `DEFAULT_SYNC_MEMORY_BUDGET` is public via peer.rs:20 and lib.rs:341; `mod tree` is private at lib.rs:322, so `Tree::join` is unreachable from the API. The altitude judgment is assessed by reading)
- Verification: confirmed; history: the trade-off table and closed form are deliberate (`.agent-notes/2026-07-22-sync-budget/sync-budget.md:150-156` records the table as "compiled into `sync_memory_budget`'s rustdoc ... the figure of record"); the test-file citations have no-rationale-found
- Owner-gated: no

The public rustdoc of one setter names four test files, one test function,
and "the storage backend's own cost function"; the public
`DEFAULT_SYNC_MEMORY_BUDGET` names a `pub(crate)` constant; the public
reconciliation page names the private `Tree::join`. None is reachable from
docs.rs or the API. They are provenance for maintainers inside a contract the
user reads for one number, and they lengthen a ~160-line doc.

Evidence:

    src/peer.rs
    313	    /// priced by the storage backend's own cost function — and
    318	    /// record per reply stream — ~0.2 MB under the in-memory backend, a
    393	    /// (calibrated by deterministic byte counts,
    394	    /// `tests/dispute_wire.rs`).
    417	    /// (`tests/tradeoff_probe.rs`).
    428	    ///   and pinned by `default_crossover_matches_the_solve`;
    444	    ///   solve). `tests/window_operator.rs` holds the wave model
    445	    ///   against measured sessions on a bandwidth-limited link.
    457	    /// `tests/window_knee.rs`, `tests/window_operator.rs`). One

    src/tree/mirror/streaming/window.rs
    273	/// decomposition behind the accuracy band is recorded beside the pinned
    274	/// per-scope envelope (`SCOPE_ENVELOPE_BYTES`).

    src/reconciliation.rs
    248	//! behavioral oracle in the test suite is the in-memory merge
    249	//! (`Tree::join`), which honors deletions through the same filter.

Resolution: Keep the contract, the closed form with its accuracy band, the
worked answers, and the table. Replace each "(`tests/x.rs`)" with the claim's
status alone ("measured", "pinned by test") and move the file names to the
tests' own doc comments or to a maintainer comment on the constant each
pins. Replace "the storage backend's own cost function" and "under the
in-memory backend" with what the user can see (per disputed subtree in
flight, at the default) or cut. window.rs:273-274: drop the parenthetical
naming `SCOPE_ENVELOPE_BYTES` from the public constant's doc (a `//`
maintainer comment beside the constant may keep it). reconciliation.rs:248-
249: cut the oracle sentence from the public page (it is a test-suite fact).
Acceptance: `grep -n "tests/\|_matches_the_solve\|SCOPE_ENVELOPE_BYTES\|Tree::join" src/peer.rs src/tree/mirror/streaming/window.rs src/reconciliation.rs` matches only lines that are not `///` or `//!` docs of public items.

### fresh-eyes-4: Snapshot's IntoIterator names an iterator type users cannot name
- Where: src/snapshot.rs:4-7 (related: src/snapshot.rs:105-107, src/snapshot.rs:172-179, src/lib.rs:317, src/lib.rs:347, src/tree.rs:176)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (the sweep's probe `check-1.log:77`: `error[E0603]: module `snapshot` is private`; read `mod snapshot;` at lib.rs:317 and `pub use snapshot::Snapshot;` at lib.rs:347, the only export; `pub struct Iter<'a, T>` at tree.rs:176 inside the private `mod tree`; no `[lints]` table in Cargo.toml and no `unnameable_types` attribute in src/lib.rs, so no committed check catches this class)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes (whether `Snapshot::iter` returns a named type and whether `Iter` joins the crate root are API decisions)

`Iter` is re-exported into the private `snapshot` module only, so it is
reachable solely as `<&Snapshot<T> as IntoIterator>::IntoIter`; a user cannot
write the type to hold the iterator in a struct. The doc sentence is also
inaccurate on the visible surface: `Snapshot::iter` returns an `impl` type,
not `Iter`, and "re-exported from the tree internals" names a layer the API
does not expose.

Evidence:

    src/snapshot.rs
    4	/// The iterator of [`Snapshot::iter`], re-exported from the tree internals:
    5	/// every live message as `(&Version, Arc<T>)`, unspecified order,
    6	/// exact-size and double-ended.
    7	pub use crate::tree::Iter;

    105	    pub fn iter(
    106	        &self,
    107	    ) -> impl DoubleEndedIterator<Item = (&Version, Arc<T>)> + ExactSizeIterator + Send + Sync

    172	impl<'a, T: Send + Sync + 'static> IntoIterator for &'a Snapshot<T> {
    173	    type Item = (&'a Version, Arc<T>);
    174	    type IntoIter = Iter<'a, T>;

    src/lib.rs
    317	mod snapshot;
    347	pub use snapshot::Snapshot;

Resolution: Owner's pick of shape: (a) re-export `Iter` at the crate root
and let `Snapshot::iter` return it, so the doc sentence becomes true; or (b)
keep the `impl` return and make `IntoIter` a nameable public type. Either
way, add `#![warn(unnameable_types)]` to src/lib.rs so the gate's clippy leg
fails the next unnameable public type: the committed check for this hole.
Rewrite snapshot.rs:4-6 without "tree internals". Acceptance: a doctest or
integration test names the iterator type in a `let x: rumors::... =
snapshot.into_iter();` binding and compiles; `unnameable_types` is enabled
and the gate is clean.

### fresh-eyes-5: SessionStats announces "Two deliberate boundaries" and lists one
- Where: src/tree/mirror/streaming/stats.rs:33-38 (related: src/peer/gossip.rs:174-179)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read stats.rs:26-41; `git log -S"Two deliberate boundaries"` finds the block's birth in 487e17ea3 with a second bullet, "[`Protocol::V1`](crate::Protocol::V1) sessions report zero in every field", which commit 368da2a5 deleted)
- Verification: confirmed, cause identified; history: deliberate-but-expired (the second bullet was the V1 one; the retirement removed it and left the count)
- Owner-gated: no

The public type's doc promises two bullets and delivers one. This is the
rot a hand-maintained count invites: the V1 bullet was deleted with V1, and
"Two" stayed.

Evidence:

    src/tree/mirror/streaming/stats.rs
    33	/// Two deliberate boundaries:
    34	///
    35	/// - **No duration field.** The caller owns the clock: wrap the `gossip`
    36	///   call (or the `gossip_when` stream's polls) in whatever timing
    37	///   instrument the application already uses. A duration measured inside
    38	///   the crate would bake in one notion of time and satisfy nobody's.
    39	#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]

Resolution: Drop the count ("One deliberate boundary:" or fold the bullet
into a sentence), or restore a second item if one is intended (the "every
count is local; the two ends report their own numbers" point at
gossip.rs:174-175 is the natural candidate). Acceptance: the heading's count,
if any, equals the bullets beneath it.

### fresh-eyes-6: Routed TCP example comment names a nonexistent `peer.rumors()`
- Where: src/link/routed.rs:159-160 (related: src/peer.rs:623)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn "fn rumors(" src tests examples` is empty; `pub fn into_rumors(self) -> Rumors<T, B>` at src/peer.rs:623; `git show b16a800b5:src/peer.rs`, the commit that wrote the example, already has `into_rumors` and no `rumors()`, so the comment was wrong when written, not orphaned by a rename)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The doctest's closing hint names a method that has never existed; being a
comment, `cargo test` cannot catch it, and it is the last line a first-time
TCP user reads before writing their own code.

Evidence:

    src/link/routed.rs
    159	//! // Each side now runs sessions on its link:
    160	//! // `peer.rumors().gossip(&mut link_at_b)`, etc.

Resolution: `//! // `peer.into_rumors().gossip(&mut link_at_b)`, etc.` (or
show `let rumors = peer.into_rumors();` first). Acceptance: every method
named in the routed example resolves (`grep -rn "fn into_rumors" src`
non-empty; `grep -rn "\.rumors()" src` empty).

### fresh-eyes-7: The BufReader advice states the mechanism without its reason, and the routed module inherits nothing
- Where: src/link.rs:76-78 (related: src/link/routed.rs:86-103, src/link/routed.rs:193-208, src/link/routed/endpoint.rs:20-25, src/tree/mirror/framing.rs:18-24)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (`grep -rn "BufReader\|BufWriter" src` finds only the two doc mentions, no code; framing.rs:18-24 and the cbor-wire review's N1 note read; the sweep's TCP session over a bare `TcpStream` completed, `run.log` line 43: `tcp: alice gained 0 shed 0 bytes 69/0; bob2 gained 1 shed 0 bytes 0/69`)
- Verification: reframed: the advice is a syscall-count mitigation (the codec reads CBOR heads byte-wise and digests in 32-byte `read_exact`s; payload chunks bypass an 8 KiB buffer), which the private framing.rs doc and the review packet both say and the public sentence does not; history: already-known (`.agent-notes/2026-08-20-cbor-wire-review/REVIEW.md` N1 placed the sentence in link.rs verbatim and recorded "If left undone, no invariant is at risk"; the routed module predates that ruling and was not considered by it)
- Owner-gated: no for the wording; yes for having the adapter wrap its own read halves

The public transport contract instructs "wrap the read half in
`tokio::io::BufReader`" without saying it is a throughput measure, so a
`Link` author cannot tell whether omitting it is a correctness risk. The
routed adapter builds `RoutedLink` from raw `ReadHalf<D::Conn>` halves, its
TCP example hands over a bare `TcpStream`, and its "What the transport must
provide" section is silent, so a routed-TCP user cannot tell whether the
adapter applied the advice, whether their `Conn` should, or whether it
matters. (tokio's `BufReader<T>` forwards `AsyncWrite` to `T` unbuffered, so
`Conn = BufReader<TcpStream>` satisfies the no-hidden-write-buffering rule;
assessed from tokio's API, not run.)

Evidence:

    src/link.rs
    76	//! Reads are exact and item-granular; on an unbuffered transport, wrap the
    77	//! read half in `tokio::io::BufReader` — caller-owned buffering outlives a
    78	//! session and is safe across session boundaries.

    src/tree/mirror/framing.rs (private module doc)
    //! The price is read batching: capacity-bounded payload reads instead of
    //! one large buffered read. A caller wanting fewer reads on a raw socket
    //! can wrap it in [`tokio::io::BufReader`] sized above
    //! [`PAYLOAD_CHUNK_LEN`] — at the default 8 KiB capacity nearly every
    //! payload read outsizes the buffer and bypasses it.

    src/link/routed/endpoint.rs
    20	pub type RoutedLink<D> = Link<
    21	    ReadHalf<<D as Dial>::Conn>,
    22	    WriteHalf<<D as Dial>::Conn>,

Resolution: At link.rs:76-78, add the reason in one clause ("a throughput
measure: the codec reads frame heads and digests item by item, so an
unbuffered socket pays a read syscall per item; correctness does not depend
on it"). In routed.rs's "What the transport must provide" section, one
sentence: the adapter does not buffer; a `Dial` whose `Conn` is a raw socket
may hand over `BufReader<TcpStream>` (write-through, so the `Conn` rule
holds). Owner option: have the adapter wrap its own read halves, since it
owns the split. Acceptance: both pages state whether the wrapper is
optional and why; the routed example either wraps or says why it does not.

### fresh-eyes-8: No worked Bookmark implementation is visible to users
- Where: src/bookmark.rs:104-124 (related: src/bookmark.rs:192-218, src/tutorial.rs:321, tests/bookmark_when.rs:103, tests/bookmark_transmit_window.rs:115, tests/common/flaky.rs:195)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (`grep -rn "Bookmark for" src tests examples` lists `NoBookmark` plus three test impls; `grep -n "# Example" src/bookmark.rs` is empty; the tutorial names `Bookmark` once, at line 321, without an impl; the sweep's file-backed impl at `scratch-app/src/main.rs:53-87` compiled on its first check and its record survived the simulated crash, `run.log` lines 4, 36, 38)
- Verification: confirmed; class reframed from feature-gap to documentation, since the resolution is an example and a shipped impl is a separate decision; history: no-rationale-found
- Owner-gated: no for the example; yes for shipping an implementation

The only `Bookmark` impl a user can see is `NoBookmark`. `store` takes a lent
boxed-future writer (`Serialized<'a>`) and carries an obligation the crate
cannot check, with "unspecified corruption" as the cost of getting it wrong,
and no example shows the temp-and-rename shape that satisfies it. The
sweep's impl compiled first try, so the docs are sufficient; the example is
what makes the cheapest artifact the intended one.

Evidence:

    src/bookmark.rs
    104	    /// Atomically replace the stored record.
    105	    ///
    106	    /// The crate serializes the framed record by calling `write` with a lent
    107	    /// writer. The implementor **must commit the written bytes atomically iff
    108	    /// `write` returns `Ok`** and must report an error rather than leave a
    109	    /// partial frame where the next [`load`](Self::load) could read it.
    110	    ///
    111	    /// Atomicity here is a safety obligation the crate cannot check, not
    112	    /// storage hygiene: a torn or reordered store whose next `load` yields
    113	    /// *valid but stale* bytes is indistinguishable from a record that never
    114	    /// covered the session, and its consequence is unspecified corruption:

Resolution: Add an `# Examples` block on `Bookmark` (or on `store`) with a
minimal file-backed impl: `load` reads the file into a `Cursor<Vec<u8>>`
(`Ok(None)` when absent), `store` buffers through the lent writer into a
`Vec<u8>`, writes a sibling temp file, and `rename`s over the target. Make
it a compiled doctest so it cannot rot. Owner option: ship that impl behind
a feature so the three test bookmarks and every user's copy collapse into
one. Acceptance: `cargo test --doc` compiles the example; the example is the
one the tutorial points to.

### fresh-eyes-9: Joined::Bailed/Failed return the bookmark but drop the configuration
- Where: src/peer/bootstrap.rs:345-353 (related: src/peer/bootstrap.rs:44-46, src/peer/bootstrap.rs:247-250, src/peer/bootstrap.rs:279-283, src/peer/bootstrap.rs:379-382, src/peer/bootstrap.rs:398-403)
- Class / severity / confidence: api-surprise / nit / medium
- Provenance: verified (read: `let Self { config, bookmark } = self;` at 345, `config` consumed by `config.join(link)` at 346 and absent from both retry arms; `Bootstrap::join` takes `self` at 247-250; `Bootstrap<T>` has a hand-written `Clone` (comment at line 82); `BookmarkedBootstrap<T, B>` at 279 derives nothing)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes

The plain builder's docs promise that after a bail or a failed session "a
clone of the same configuration retries", but `BookmarkedBootstrap::join`
consumes the builder, the type is not `Clone`, and the retry arms hand back
only the bookmark. A user who wants the promised retry must have cloned the
plain `Bootstrap<T>` before attaching the bookmark, which no page shows.

Evidence:

    src/peer/bootstrap.rs
    44	/// The builder is `Clone`: after a mutual-bootstrap bail
    45	/// ([`join`](Self::join)'s `Ok(None)`) or a failed session, a clone of
    46	/// the same configuration retries against another provider as-is.

    345	        let Self { config, bookmark } = self;
    346	        match config.join(link).await {
    347	            Ok(Some(peer)) => match peer.bookmark(bookmark).await {
    348	                Ok(peer) => Joined::Joined { peer },
    349	                Err(unbookmarked) => Joined::Unbookmarked(unbookmarked),
    350	            },
    351	            Ok(None) => Joined::Bailed { bookmark },
    352	            Err(error) => Joined::Failed { error, bookmark },
    353	        }

Resolution: Either return the whole `BookmarkedBootstrap<T, B>` in `Bailed`
and `Failed` (API change), or add one sentence to `Bootstrap::bookmark`
saying to clone the plain builder first when a retry is anticipated.
Acceptance: a retry after `Joined::Bailed` needs no state the outcome did
not return, or the docs say what to keep.

### fresh-eyes-10: error module doc misstates what reaches the user through Error::Mirror
- Where: src/error.rs:3-6 (related: src/error.rs:39-40, src/error.rs:186-189, src/error.rs:216-219, src/error.rs:289)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read: `PreambleMalformed { defect: PreambleDefect }` at 186-189 and `HandOffMalformed { defect: HandOffDefect }` at 216-219 carry two of the page's re-exports directly; `Mirror(#[from] MirrorError)` at 289)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The routing sentence at the top of the page says everything besides `Error`
and `EncodeError` is reachable through `Error::Mirror`; `PreambleDefect` and
`HandOffDefect` on the same page reach the user through two other variants.

Evidence:

    src/error.rs
    3	//! You handle [`Error`], and a send can return the local admission
    4	//! error [`EncodeError`] (also at the crate root); everything else on
    5	//! this page is the diagnostic taxonomy reachable through
    6	//! [`Error::Mirror`], for matching and bug reports. Every session `Err` poisons its link (discard it and

    186	    PreambleMalformed {
    187	        /// Which preamble field failed, and how.
    188	        defect: PreambleDefect,
    189	    },

Resolution: "reachable through `Error`'s variants (chiefly
`Error::Mirror`)", and while there, re-wrap line 6, whose sentence runs past
the page's column. Acceptance: every re-exported type on the page is
reachable through the variant the sentence names.

### fresh-eyes-11: "seam" in public rustdoc
- Where: src/tree/mirror/streaming/stats.rs:29 (related: src/tree/mirror/streaming/stats.rs:104, src/tree/mirror/streaming/stats.rs:122, src/peer/gossip.rs:177-178, src/link/routed.rs:214)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn seam src`; the listed sites are docs of `pub` items: `SessionStats`, its `bytes_sent` and `bytes_received` fields, `Gossiped::stats`, `trait Dial`)
- Verification: new in this pass (surfaced while confirming fresh-eyes-5; overlaps the prose-hygiene sweep's lens); history: no-rationale-found
- Owner-gated: no

"Seam" is maintainer vocabulary for a layer boundary; on the public
`SessionStats` and `Dial` pages a first-time reader has no anchor for it,
and stats.rs:105 already supplies the plain word ("the same boundary").

Evidence:

    src/tree/mirror/streaming/stats.rs
    29	/// Every count is taken locally, at the seam named in its field docs, while
    104	    /// this counter is taken exactly at that seam, between the codec and
    105	    /// the transport stream it writes: the same boundary the
    122	    /// Counted at the same codec seam as [`bytes_sent`](Self::bytes_sent),

    src/peer/gossip.rs
    177	    /// See [`SessionStats`] for each field's mechanism and the seam it is
    178	    /// counted at.

    src/link/routed.rs
    214	/// This is the adapter's outgoing seam, and the place transport policy

Resolution: "boundary" (or name the layer: "between the codec and the
transport") at the five public sites; the private-module uses may stay.
Acceptance: `grep -rn seam src` matches no `///` or `//!` line attached to a
`pub` item.

## Positives

- The public docs alone were enough to write a complete application: the
  sweep's custom `Bookmark`, three-level wire `Observer`, routed TCP
  `Dial`/`Listen` pair, stoppable `gossip_when` drivers, and all three set
  observers compiled on the first `cargo check`, with the only errors being
  its two deliberate probes (I read `check-1.log`: exactly two `error[` lines,
  at 77 and 88).
- Every `#[must_use]` on an outcome type says what a silent drop loses; I
  verified the messages at bootstrap.rs:58, 278, 364, gossip.rs:92, 148, 912,
  and rumors.rs:616. Line 364's "every `Joined` variant carries a peer or
  the bookmark; dropping it loses one or the other" is the model.
- `Changes`' docs pre-empt the naive `loop { changes.next(); gossip() }`
  deadlock and route the reader to `gossip_when` (src/rumors/changes.rs:37-
  43, verified). The `Error` table (error.rs:10-28) gives one actionable
  sentence per variant, and the two sites this pass found wanting
  (fresh-eyes-2, fresh-eyes-10) are wording, not structure.
- The V1 retirement's prose re-denomination was thorough: the commit message
  for 368da2a5 enumerates the surfaces it re-stated, and the leftovers found
  here (fresh-eyes-2's "select" vocabulary, fresh-eyes-5's count) are the
  residue of a sweep that caught nearly everything. The retirement note's
  numbered rulings made attributing each leftover to a decision immediate.
- The sweep's runtime observations (`run.log`, read not reproduced): both
  ends' drivers ended cleanly on stream end and on remote hang-up; concurrent
  one-shot `gossip` on both ends merged into one session with both sides
  reporting `Led::Local`, as rumors.rs and gossip.rs:184-185 say; one side's
  `bytes_sent` equalled the other's `bytes_received` over TCP (69/69);
  redactions issued on each side were honored on both in one session.

## Open questions for Finch

1. Bookmark size (the sweep's question, answered by reading): Bob's record
   was 68 bytes after his bookmarked join and 68 bytes after three driven
   sessions carrying his own sends. gossip.rs:681-715 re-records and writes
   the bookmark at every session start whenever the live `(party, version)`
   differs from what was last persisted (`is_current` false), and a send
   advances `inner.tree.latest()`, so those writes did run; the equal size
   means the advanced version encoded to the same width. Assessed from code,
   not run. If you want certainty, one `eprintln!` in the sweep's
   `FileBookmark::store` settles it.
2. fresh-eyes-4: which shape do you want, `Snapshot::iter` returning a
   nameable `Iter` re-exported at the root, or an `impl` return with a
   nameable `IntoIter`? Either way I recommend `#![warn(unnameable_types)]`
   as the committed check.
3. fresh-eyes-7: should the routed adapter wrap its own read halves in
   `BufReader`, since it owns the split, or stay a pass-through and document
   the option?
4. fresh-eyes-9: return the whole `BookmarkedBootstrap` from `Bailed`/
   `Failed`, or document the clone-before-bookmark pattern?
5. src/tutorial.rs:29 instructs `rumors = "0.1"`; Cargo.toml is 0.1.0. Fine
   for a crate that publishes as 0.1; worth a line if the tutorial is read
   before publication.

## Dropped

- Candidate: reconciliation.rs:237-249 "Why streaming, not level-synchronous exchange" as a ghost of V1. Dropped: the V1-retirement note rules it "the design rationale it is: the level-synchronous shape described as the naive alternative", and the text names no removed code; only its `Tree::join` sentence is flagged, under fresh-eyes-3.
- Candidate: dissolve the one-variant `Protocol` enum under Principle 3. Dropped: commit 368da2a5's message keeps it deliberately ("a knob with one position; it returns with a V3"); fresh-eyes-2 targets the vocabulary only.
- Candidate: run `stream_count_matches_the_codec` to settle fresh-eyes-1 mechanically. Dropped: both constants are literal `17`s read at their definitions, and the machine is shared with other reviewers.
- The sweep's classification of fresh-eyes-8 as feature-gap. Reframed to documentation rather than dropped; the shipped-impl half stays as an owner option.
