# Sweep api-audit: Public API surface audit

## Method and coverage

The sweep enumerated the public surface from the rendered rustdoc and read
every item's definition. This finalization pass disputed each of its
nineteen findings against the tree at 9e5784fb (working tree clean):

- Read every cited site with line numbers (`awk`/`sed -n` ranges over
  `src/error.rs`, `src/snapshot.rs`, `src/bookmark.rs`, `src/protocol.rs`,
  `src/peer.rs` whole, `src/rumors.rs` whole, `src/peer/gossip.rs`,
  `src/peer/bootstrap.rs`, `src/link.rs`, `src/link/routed.rs`,
  `src/link/routed/endpoint.rs`, `src/link/routed/stream.rs`, `src/tree.rs`,
  `src/message.rs`, `src/observe.rs`, `src/rumors/{unordered,causal,changes}.rs`,
  `src/tree/mirror.rs`, `src/tree/mirror/handshake.rs`, the streaming
  `remote/error.rs`, `codec/error.rs`, `codec/signal.rs`, `codec/frame.rs`,
  `adapter/error.rs`, `materialized/error.rs`, `proxy/error.rs`,
  `streams.rs`, `stats.rs`, `window.rs`, `Cargo.toml`, the `justfile`, and
  `tests/api_send_bounds.rs`).
- Extracted the `impl` headers from the rendered pages under
  `target/doc/rumors/` (index mtime 2026-09-01 19:17, after HEAD's commit
  time 18:37; the tree is clean, so the pages render this commit) with a
  short Python scan, and checked which pages exist under `error/`.
- Grepped for use sites (`fn rumors`, `into_rumors`, `select`,
  `missing_docs`, `warm_caches`/`seed_rng`, `non_exhaustive`, `Debug for`,
  `tests/` inside doc comments, doc-comment `.send(..);` statements,
  `.flip()`, the `pub` constructors' callers).
- Checked history with `git log -S` for `BookmarkError`, `repr(u16)`,
  `Two deliberate boundaries`, `peer.rumors()`, `# Choosing a budget`,
  the `Iter` re-export, and `warm_caches`; read the commit message of
  1e458d69 (the `non_exhaustive` policy) and the notes under
  `.agent-notes/2026-09-01-v1-retirement/`,
  `.agent-notes/2026-08-20-cbor-wire-review/`,
  `.agent-notes/2026-07-22-sync-budget/`.

What this pass could not see: no cargo, rustc, or `cargo doc` invocation
was permitted, so the derive-bound claims rest on the rendered pages plus
the standard semantics of `#[derive]` (a bound on every type parameter),
and the availability of the `unnameable_types` lint on the pinned
toolchain was not checked. No test invocation was needed: none of the
surviving findings is a correctness claim.

## Findings

### api-audit-1: Derived trait impls on wrapper types demand bounds the wrapped values never need
- Where: src/error.rs:61-63 (related: src/snapshot.rs:16-17, src/tree.rs:90-91, src/tree.rs:137-150, src/peer/gossip.rs:93-94, src/peer/gossip.rs:149-150, src/peer/bootstrap.rs:365-366, src/peer.rs:182-193, src/rumors.rs:79-90, src/peer/bootstrap.rs:81-94, src/bookmark.rs:92)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: verified (impl headers extracted from the rendered pages `error/enum.Error.html`, `struct.Snapshot.html`, `enum.Retire.html`, `enum.Joined.html`, `struct.Unbookmarked.html`, `struct.Peer.html` under `target/doc/rumors/`)
- Verification: confirmed; history: no-rationale-found (the derives date from WIP commits; the crate's own `Bootstrap` shows the manual-impl pattern with its rationale)
- Owner-gated: yes: relaxing impl bounds widens the public API, which the owner lands

`Error<B>`, `Retire<T, B>`, `Joined<T, B>`, `Unbookmarked<T, B>`, and
`Snapshot<T>` derive their impls, so the derive's implicit bounds leak into
the API: `rumors::Error<B>` is `Debug`, and therefore `std::error::Error`,
only when the user's bookmark type is `Debug`, a bound `Bookmark` neither
states nor needs (no `B` value is ever held, only `B::Error`); `Snapshot<T>`
is `Clone` only when `T: Clone`, although it is an `Arc`-sharing view whose
`Tree<T>` has an unbounded manual `Clone`. `Peer` and `Rumors` implement
`Debug` by hand for all `T`, and `Bootstrap` carries a comment explaining
exactly this pattern, so the outcome types undo a standard the crate set.

Evidence:

    src/error.rs
    61	#[non_exhaustive]
    62	#[derive(Debug, thiserror::Error)]
    63	pub enum Error<B: BookmarkError = NoBookmark> {

    src/snapshot.rs
    16	#[derive(Clone, Debug, PartialEq, Eq)]
    17	pub struct Snapshot<T> {

    src/tree.rs
    90	#[derive(Debug, Eq)]
    91	pub struct Tree<T> {
    ...
    137	impl<T> Clone for Tree<T> {

    src/peer/bootstrap.rs (the crate already states the rule)
    81	// A manual, unbounded impl: the payload type is phantom (the builder
    82	// holds configuration only), so the `T: Clone` bound `derive` would add
    83	// has nothing to constrain.
    84	impl<T> Clone for Bootstrap<T> {

    rendered impl headers (target/doc/rumors)
    impl<B: Debug + BookmarkError> Debug for Error<B> where B::Error: Debug
    impl<B: BookmarkError> Error for Error<B> where BookmarkIo<B::Error>: Error, Self: Debug + Display
    impl<T: Clone> Clone for Snapshot<T>
    impl<T: Debug> Debug for Snapshot<T>
    impl<T: Debug, B: Debug + BookmarkError> Debug for Retire<T, B>
    impl<T: Debug, B: Debug + BookmarkError> Debug for Joined<T, B>
    impl<T, B: BookmarkError> Debug for Peer<T, B>

Resolution: replace the derives with manual impls mirroring the wrapped
types' bounds: `impl<B: BookmarkError> Debug for Error<B> where
BookmarkIo<B::Error>: Debug` (thiserror's `Error` impl then holds for every
bookmark), the same for `Retire`, `Joined`, `Unbookmarked`; `impl<T> Clone
for Snapshot<T>` and `impl<T> Debug for Snapshot<T>` delegating to `Tree`,
which needs its own manual `Debug` (its derive also binds `T: Debug`
through the `PhantomData`). Add a compile test beside
`tests/api_send_bounds.rs` instantiating `Error<B>` as `Box<dyn
std::error::Error>` for a `Bookmark` whose type is not `Debug`, and cloning
a `Snapshot<P>` for a non-`Clone` payload. Acceptance: the rendered headers
read `impl<T, B: BookmarkError> Debug for Retire<T, B>`, `impl<B:
BookmarkError> Error for Error<B>` without `B: Debug`, and `impl<T> Clone
for Snapshot<T>`; the compile test is committed.

Construction: `struct NoDebugBookmark; impl BookmarkError for
NoDebugBookmark { type Error = std::convert::Infallible; } impl Bookmark
for NoDebugBookmark { /* as NoBookmark */ }` then `fn is_error<E:
std::error::Error>() {} is_error::<rumors::Error<NoDebugBookmark>>();`
fails to compile today. `#[derive(PartialEq, Eq, Serialize, Deserialize)]
struct P(u64); fn is_clone<T: Clone>() {}
is_clone::<rumors::Snapshot<P>>();` fails to compile today.

### api-audit-2: Public signatures name types with no public path: Snapshot's IntoIter, Signal, and a second StreamError
- Where: src/snapshot.rs:172-174 (related: src/snapshot.rs:4-7, src/snapshot.rs:105-112, src/tree.rs:167-176, src/lib.rs:347, src/tree/mirror/streaming/remote/codec/signal.rs:40-47, src/tree/mirror/streaming/remote/codec/signal.rs:105-110, src/tree/mirror/streaming/remote/codec/signal.rs:192-200, src/tree/mirror/streaming/remote/codec/signal.rs:374-377, src/tree/mirror/streaming/remote/error.rs:11-18, src/tree/mirror/streaming/remote/streams.rs:264-266)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: verified (`find target/doc/rumors -name '*Iter*'` returns nothing; every `href` in `struct.Snapshot.html` containing `Iter` points at `IntoIterator`/`DoubleEndedIterator`/`ExactSizeIterator` in std, none at an `Iter` page; `error/struct.Stream.html` renders `Result<Self, StreamError>` with `StreamError` as unlinked text; `error/` holds 36 pages and no `Signal` page)
- Verification: confirmed; history: no-rationale-found (the `Iter` re-export dates from ddc57045f, before `snapshot` became a private module)
- Owner-gated: yes: the fix exports a type or removes public methods

`impl IntoIterator for &Snapshot<T>` sets `IntoIter = Iter<'a, T>`, but
`Iter` lives in the private `tree` module and its `pub use` sits in the
private `snapshot` module (`lib.rs` re-exports only `Snapshot`), so no user
can write the type to store the iterator or name it in a signature, the
doc comments written for it render nowhere, and `Snapshot::iter()` returns
an `impl` type, so the two faces of one iterator disagree. The same hole
exists twice in `rumors::error`: `InvalidSignalPlacement::signal()` returns
`Signal`, and `Stream::new()` returns `Result<Self, StreamError>` where
that `StreamError` is `signal::StreamError`, an unexported enum sharing its
name with the exported `rumors::error::StreamError` from `streams.rs`.

Evidence:

    src/snapshot.rs
    4	/// The iterator of [`Snapshot::iter`], re-exported from the tree internals:
    5	/// every live message as `(&Version, Arc<T>)`, unspecified order,
    6	/// exact-size and double-ended.
    7	pub use crate::tree::Iter;
    ...
    172	impl<'a, T: Send + Sync + 'static> IntoIterator for &'a Snapshot<T> {
    173	    type Item = (&'a Version, Arc<T>);
    174	    type IntoIter = Iter<'a, T>;

    src/lib.rs
    347	pub use snapshot::Snapshot;

    src/tree/mirror/streaming/remote/codec/signal.rs
    41	    pub fn new(index: u8) -> Result<Self, StreamError> {
    ...
    107	pub enum StreamError {
    ...
    375	    pub fn signal(self) -> Signal {

    src/tree/mirror/streaming/remote/error.rs (neither Signal nor signal::StreamError is exported)
    11	pub use super::codec::{
    12	    DecodeError as CodecDecodeError, DecodeErrorKind as CodecDecodeErrorKind, DecodeLeafError,
    13	    DecodeSignalError, EncodeError as CodecEncodeError, EncodeErrorKind as CodecEncodeErrorKind,
    14	    FramePart, GreetingError, HeadError, InvalidSignalPlacement, LeafRunError, ListingIssue,
    15	    Origin, QueryOrderError, Speaker, Stream, StreamClass,
    16	};
    17	pub use super::proxy::Error as RemoteError;
    18	pub use super::streams::{AcceptError, ReplyFrameError, SendError, StreamError};

Resolution: export `Iter` at the crate root (for example `pub use
snapshot::{Snapshot, SnapshotIter}` with the alias at snapshot.rs:7) and
have `Snapshot::iter()` return it by name so both faces agree. In `error`,
make `Stream::new`, `Stream::at_height`, and `Stream::height` `pub(crate)`
(see api-audit-14) so `signal::StreamError` leaves the public surface, or
export it under a distinct name; make `InvalidSignalPlacement::signal`
`pub(crate)` or export `Signal`, `Flow`, and `End`. Then close the class
mechanically: enable rustc's allow-by-default `unnameable_types` lint in
`lib.rs` (`#![warn(unnameable_types)]`, denied by the clippy leg's `-D
warnings`) if the pinned toolchain carries it. Acceptance: every type in a
rendered public signature links to a page (`struct.Snapshot.html`'s `type
IntoIter` links to an `Iter` page; `error/struct.Stream.html` and
`error/struct.InvalidSignalPlacement.html` show no unlinked type), and the
lint is on.

### api-audit-3: Payload bounds are re-demanded at every method although the crate docs promise 'demanded once'
- Where: src/lib.rs:240-242 (related: src/peer.rs:145, src/peer.rs:195, src/peer.rs:290-292, src/peer/bootstrap.rs:247-252, src/rumors.rs:27, src/rumors.rs:165-168, src/rumors.rs:208-210, src/rumors.rs:244-247, src/rumors.rs:279-282, src/rumors.rs:337-340, src/rumors.rs:362-365, src/rumors.rs:382-385, src/rumors.rs:489-494, src/rumors.rs:617-623, src/snapshot.rs:17, src/snapshot.rs:90-93, src/snapshot.rs:105-110, src/snapshot.rs:153-158, src/rumors/unordered.rs:167, src/rumors/unordered.rs:191, src/rumors/causal.rs:128, src/rumors/causal.rs:151, src/rumors/changes.rs:112-114, src/rumors/changes.rs:127)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read every `impl` block on `Peer`, `Rumors`, `Snapshot`, and the observers; the structs carry no `T` bound, and the only constructors, `Peer::seed` and the two `join`s, each require all six bounds)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes for the structural fix (struct bounds are API); the one-sentence doc correction is not

The crate docs say the payload type's six bounds are demanded once at
construction. `Serialize + DeserializeOwned + Eq` are; `Send + Sync +
'static` are repeated as `where` clauses on `Peer::retire`, every `Rumors`
mutator and session entry, `Snapshot::{get, iter, range}`, and the observer
impls, because `Peer<T, B>`, `Rumors<T, B>`, and `Snapshot<T>` declare no
bound on `T`. The set varies by method (`send` wants `'static`, `redact`
does not), which reads as a per-method requirement and is not: no `Peer<T>`
exists for a `T` failing any of them.

Evidence:

    src/lib.rs
    240	//! Your message type `T` needs [`serde::Serialize`],
    241	//! [`serde::de::DeserializeOwned`], [`Eq`], [`Send`], [`Sync`], and
    242	//! `'static`, all demanded once, at peer construction. Payloads are

    src/peer.rs
    145	pub struct Peer<T, B: BookmarkError = NoBookmark> {
    ...
    290	    pub async fn retire<CR, CW, C, A>(self, link: &mut Link<CR, CW, C, A>) -> Retire<T, B>
    291	    where
    292	        T: Send + Sync + 'static,

    src/rumors.rs
    208	    pub fn redact(&self, version: &Version)
    209	    where
    210	        T: Send + Sync,

Resolution: minimal: reword lib.rs:240-242 so the serde and `Eq`
obligations are the ones demanded once, while `Send + Sync + 'static`
accompany the methods that move or share the payload. Structural: put `T:
Send + Sync + 'static` on the definitions of `Peer`, `Rumors`, `Snapshot`,
and the observers (`Batch` already carries `Send + Sync`) and delete the
per-method clauses, making the sentence true as written. Acceptance: either
the sentence matches the signatures, or `grep -n 'T: Send + Sync'
src/rumors.rs src/peer.rs src/snapshot.rs` returns only struct-level
bounds.

### api-audit-4: `BookmarkError` is a misnamed supertrait implemented by bookmarks, not by errors
- Where: src/bookmark.rs:26-31 (related: src/bookmark.rs:48-49, src/bookmark.rs:92, src/bookmark.rs:144, src/bookmark.rs:199-201, src/peer.rs:145, src/rumors.rs:27, src/error.rs:63, src/peer/gossip.rs:94, src/peer/gossip.rs:150, src/peer/bootstrap.rs:366)
- Class / severity / confidence: api-surprise / low / medium
- Provenance: assessed (read bookmark.rs in full; every constructor path that yields a `Peer<T, B>` requires `B: Bookmark`: peer.rs:272-277, bootstrap.rs:212, and `seed` fixes `NoBookmark`)
- Verification: confirmed; history: no-rationale-found (`git log -S'pub trait BookmarkError'` attributes it to cbfc1571e, "Unify async/sync interfaces, parameterizing by defaulted type", whose message records no reason for the split)
- Owner-gated: yes: renames or removes a public trait

The trait named `BookmarkError` is implemented by bookmark types and
carries only `type Error`; its first sentence describes the associated
type, not the trait. Every public generic reads `B: BookmarkError`, telling
a reader that `B` is an error type. Since every constructor requires `B:
Bookmark`, the split buys nothing a `type Error` on `Bookmark` would not;
the crate-private `Persist` supertrait would carry its own `type Error`,
normalized to `Bookmark::Error` by the blanket impl.

Evidence:

    src/bookmark.rs
    26	/// The error a [`Bookmark`] reports when persistence fails.
    27	pub trait BookmarkError {
    28	    /// What a [`load`](Bookmark::load) or [`store`](Bookmark::store)
    29	    /// reports when it fails.
    30	    type Error: std::error::Error + Send + Sync + 'static;
    31	}
    ...
    92	pub trait Bookmark: BookmarkError {
    ...
    199	impl BookmarkError for NoBookmark {
    200	    type Error = std::convert::Infallible;
    201	}

    src/peer.rs
    145	pub struct Peer<T, B: BookmarkError = NoBookmark> {

Resolution: fold `type Error` into `Bookmark`, bound the structs and
`Error<B>` on `B: Bookmark`, and delete `BookmarkError`, so a user
implementing a bookmark implements one trait. If a reason for the split
exists, keep the trait under a name that describes it and rewrite its
first sentence to describe the trait rather than the associated type.
Acceptance: no public bound reads `B: BookmarkError`, or the trait's name
and first sentence describe what implements it.

### api-audit-5: Prose still speaks of 'selecting' a Protocol that no API selects, and the enum carries a vestigial repr
- Where: src/protocol.rs:1-19 (related: src/error.rs:14, src/error.rs:76, src/error.rs:179, src/error.rs:201, src/peer.rs:603-605, src/tree/mirror/handshake.rs:102, src/tree/mirror/handshake.rs:125, tests/handshake.rs:59)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -n -i 'select' src/protocol.rs src/error.rs src/peer.rs`; no public function takes or sets a `Protocol`; `git log -S'repr(u16)'` attributes the repr to 83edcd944, a WIP commit; the only casts are `Protocol::V2 as u64` on the wire and `as u8`/`as u16` in tests)
- Verification: confirmed; history: deliberate-and-holds for the enum itself (the V1 retirement note's decision 2 keeps `Protocol` public and `#[non_exhaustive]` as wire vocabulary), no-rationale-found for the surviving "select" prose and the repr
- Owner-gated: no

After the V1 removal (368da2a5) `Protocol` has one variant and no public
function selects one, yet the module doc calls the protocols "Selectable",
the error table tells the caller to "select the same Protocol at both
ends", the `VersionMismatch` display says "we selected", two `Error` docs
speak of "the selected dialect", and `Peer::payload_depth_limit` compares
its knob to "changing the selected Protocol". A reader goes looking for a
`.protocol(..)` builder that does not exist. The enum also carries
`#[repr(u16)]` while the wire writes the discriminant as a CBOR uint and
`VersionMismatch` reports `remote_version: u64`; nothing in the crate
depends on the repr.

Evidence:

    src/protocol.rs
    1	//! Selectable wire reconciliation protocols.
    ...
    12	#[repr(u16)]

    src/error.rs
    14	//! | [`Error::VersionMismatch`] | unchanged | select the same [`Protocol`] at both ends; if both already do, the selected protocol's wire version differs across the two releases: align crate versions |
    ...
    76	    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]
    ...
    179	    /// The peer opened as a rumors stream of the selected dialect, but a
    ...
    201	        /// The selected dialect's full preamble width.

    src/peer.rs
    603	    /// is therefore a fleet-coordinated configuration event, like
    604	    /// changing the selected [`Protocol`](crate::Protocol), never a
    605	    /// per-peer tuning parameter.

    src/tree/mirror/handshake.rs
    102	        cbor::write_head(&mut bytes, MAJOR_UINT, Protocol::V2 as u64);

Resolution: rewrite protocol.rs:1 as the dialect the crate speaks (one
today; a wire change adds a variant), the error.rs:14 row as "the peers run
releases speaking different wire versions: align crate versions", the
display at error.rs:76 as "we speak", error.rs:179 and 201 as "the rumors
dialect", and peer.rs:603-605 as "like a wire-version upgrade"; drop
`#[repr(u16)]` (or state at the enum why a repr matters when the wire
carries a CBOR uint), adjusting tests/handshake.rs:59 accordingly.
Acceptance: `grep -rn -i 'select' src/protocol.rs src/error.rs
src/peer.rs` finds no protocol-selection prose; the `Protocol` doc states
that the crate speaks one dialect.

### api-audit-6: The routed-link example names a `peer.rumors()` method that does not exist
- Where: src/link/routed.rs:159-160 (related: src/peer.rs:616-625)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn 'fn rumors\b' src tests benches examples` returns nothing; the only conversion is `Peer::into_rumors` at peer.rs:623; `git log -S'peer.rumors()'` attributes the line to b16a800b5)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The TCP instantiation example closes by telling the reader to run sessions
with `peer.rumors().gossip(&mut link_at_b)`. `Peer` has no `rumors()`; the
conversion is `into_rumors`. The call sits in a comment, so the doctest
passes and nothing catches it: the one place a transport author reads
first carries a ghost method.

Evidence:

    src/link/routed.rs
    159	//! // Each side now runs sessions on its link:
    160	//! // `peer.rumors().gossip(&mut link_at_b)`, etc.
    161	//! # let _ = (&mut link_at_a, &mut link_at_b);

Resolution: write `rumors.gossip(&mut link_at_b)` with `let rumors =
Peer::<String>::seed().into_rumors();` introduced earlier in the example,
or make the trailing line live code (for example a closure over
`rumors::Rumors<u64>` that calls `gossip`). Acceptance: the example's
session line is compiled code or names `into_rumors`.

### api-audit-7: The public error taxonomy has undocumented variants, fields, and methods, and no lint guards against it
- Where: src/tree/mirror/streaming/remote/codec/error.rs:40-53 (related: src/tree/mirror/streaming/remote/codec/error.rs:12-27, src/tree/mirror/streaming/remote/codec/error.rs:56-61, src/tree/mirror/streaming/remote/codec/error.rs:63-76, src/tree/mirror/streaming/remote/codec/error.rs:78-85, src/tree/mirror/streaming/remote/codec/error.rs:103-109, src/tree/mirror/streaming/remote/codec/error.rs:111-159, src/tree/mirror/streaming/remote/codec/error.rs:161-168, src/tree/mirror/streaming/remote/codec/signal.rs:105-117, src/tree/mirror/streaming/remote/codec/signal.rs:137-154, src/tree/mirror/streaming/remote/codec/signal.rs:396-397, src/tree/mirror/streaming/materialized/error.rs:1-8, src/tree/mirror/streaming/remote/streams.rs:237-257, src/tree/mirror/streaming/remote/streams.rs:264-295, src/tree/mirror/streaming/remote/streams.rs:805-826, src/error.rs:72-80, src/error.rs:231-233, src/tree/mirror/handshake.rs:37, src/tree/mirror/handshake.rs:178, src/lib.rs:295-302, Cargo.toml:83-107)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read the listed files; lib.rs:295-302 carries only `forbid(unsafe_code)`, `doc_auto_cfg`, and `deny(clippy::large_futures)`; `grep -rn 'missing_docs\|missing_debug' src Cargo.toml justfile .cargo .config .github` returns nothing; `tools/doclint` checks summary length and `include_str!` separation only)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Items rendered under `rumors::error` lack doc comments on many public
members: every `FramePart` variant, the `Origin::Stream` fields,
`QueryOrderError`'s fields, `EncodeErrorKind::{Write, Flush,
SupplyTooLarge}`, `CodecEncodeError`'s fields, `DecodeLeafError::{Version,
Message}`, the `Read`, `InvalidSignal`, `Truncated`, `QueryOutOfOrder`,
`InvalidRun`, `OverbatchedRun`, and `TrailingBytes` variants of
`CodecDecodeErrorKind` and most of their fields, `CodecDecodeError`'s
fields, `MaterializedError::{Backend, Violation}`, `Speaker::{Initiator,
Responder}`, four of five `StreamClass` variants,
`DecodeSignalError::Placement`, the `origin`/`source` fields throughout
`SendError`, `StreamError`, and `AcceptError`, and at the top level
`Error::MagicMismatch { remote_magic: [u8; 6] }`, `VersionMismatch`'s two
fields, and `IntentInvalid { byte }`. The `6` is a magic number whose named
constant (`MISMATCH_PREVIEW_LEN`) is private, so the field's meaning is
unrecoverable from the public page. No `missing_docs` lint exists anywhere
in the tree, so the gate cannot notice.

Evidence:

    src/tree/mirror/streaming/remote/codec/error.rs
    40	/// The absent or malformed component of a frame.
    41	#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
    42	pub enum FramePart {
    43	    #[error("frame head")]
    44	    FrameHead,
    45	    #[error("signal")]
    46	    Signal,
    47	    #[error("query child listing")]
    48	    QueryChildren,
    49	    #[error("supply run head")]
    50	    SupplyLength,
    51	    #[error("supply run")]
    52	    SupplyRun,
    53	}

    src/error.rs
    72	    #[error("peer is not a rumors stream (leading bytes: {remote_magic:x?})")]
    73	    MagicMismatch { remote_magic: [u8; 6] },

    src/tree/mirror/handshake.rs
    37	const MISMATCH_PREVIEW_LEN: usize = 6;
    ...
    178	    MagicMismatch { remote_magic: [u8; 6] },

Resolution: add `#![warn(missing_docs)]` to lib.rs (the clippy leg's `-D
warnings` then enforces it) and write the missing sentences, or make the
item `pub(crate)` where the sentence would be "internal" (api-audit-14
covers the constructors). Replace both `[u8; 6]` with `[u8;
MISMATCH_PREVIEW_LEN]` and either export the constant or document the
field's width in words. Acceptance: `cargo clippy -p rumors --lib
--all-features -- -D warnings` passes with `#![warn(missing_docs)]` in
lib.rs.

### api-audit-8: Public rustdoc cites test files and a `cfg(test)` constant a library user cannot see
- Where: src/peer.rs:391-394 (related: src/peer.rs:414-417, src/peer.rs:428-429, src/peer.rs:444-445, src/peer.rs:456-457, src/tree/mirror/streaming/window.rs:264-265, src/tree/mirror/streaming/window.rs:272-274, src/tree/mirror/streaming/remote/codec/error.rs:150-153, src/tree/mirror/streaming/remote/codec/budget.rs:86)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`grep -rn -E '^\s*(///|//!).*tests/' src`; `SCOPE_ENVELOPE_BYTES` is `#[cfg(any(test, feature = "test-internals"))] pub(crate)` at window.rs:264-265; `SUPPLY_FRAME_OVERHEAD` is `pub const` in the private `codec::budget` module and is not re-exported)
- Verification: reframed: the sweep also listed link.rs:49-50, link.rs:167-168, and the adapter `DecodeError` docs naming `max_version_bytes` and `set_len`; the first two already state provenance without a path ("the crate's own tests", "pinned ... by test"), which is the form the resolution asks for, and the last two are the greeting's wire field names (greeting.rs:16-19), observable by any wire observer, so all four are dropped from the list; history: no-rationale-found
- Owner-gated: no

The public docs of `Peer::sync_memory_budget` name five test artifacts
(`tests/dispute_wire.rs`, `tests/tradeoff_probe.rs`,
`default_crossover_matches_the_solve`, `tests/window_operator.rs`,
`tests/window_knee.rs`); `DEFAULT_SYNC_MEMORY_BUDGET`'s doc cites the
`cfg(test)` constant `SCOPE_ENVELOPE_BYTES`; `CodecDecodeErrorKind::
OverbatchedRun`'s field doc cites the private `SUPPLY_FRAME_OVERHEAD`.
None is reachable from the API, and nothing checks the test paths when a
suite is renamed (the gate's `citecheck` covers `before`'s rosters, not
doc-to-test paths).

Evidence:

    src/peer.rs
    391	    /// a 5431 B envelope (recomputed exactly by test), and each disputed
    392	    /// message costs 43 B of wire overhead on top of its record
    393	    /// (calibrated by deterministic byte counts,
    394	    /// `tests/dispute_wire.rs`).

    src/tree/mirror/streaming/window.rs
    264	#[cfg(any(test, feature = "test-internals"))]
    265	pub(crate) const SCOPE_ENVELOPE_BYTES: usize = 5_431;
    ...
    272	/// [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget); the
    273	/// decomposition behind the accuracy band is recorded beside the pinned
    274	/// per-scope envelope (`SCOPE_ENVELOPE_BYTES`).

    src/tree/mirror/streaming/remote/codec/error.rs
    150	        /// The frame's charged wire size — its run body plus the
    151	        /// `SUPPLY_FRAME_OVERHEAD` envelope at its widest — which may

Resolution: in the public docs, state the provenance without the path
("measured; pinned by test", as link.rs already does) and keep the numbers
with their validity bands; move the measurement narrative to maintainer
docs (a private module doc or the note under
`.agent-notes/2026-07-22-sync-budget/`). Replace the private-constant
names with the quantities they denote. Acceptance: `grep -rn 'tests/' src
--include='*.rs' | grep -E '^[^:]+:[0-9]+:\s*(///|//!)'` returns hits only
in private docs (`src/testing.rs`, test-module docs), and no public doc
names an item `cargo doc` does not render.

### api-audit-9: `Peer::sync_memory_budget` carries a 160-line operator sizing guide on a setter
- Where: src/peer.rs:308-470 (related: src/peer.rs:371-373, src/peer/bootstrap.rs:120-136, src/reconciliation.rs:216-235, src/tree/mirror/streaming/window.rs:267-275, src/tree/mirror/streaming/window/tradeoff.md)
- Class / severity / confidence: documentation / low / high
- Provenance: assessed (read peer.rs:308-470; the contract paragraphs run 308-369, "# Choosing a budget" runs 371-464, and the included table is 11 lines)
- Verification: confirmed; history: deliberate-but-expired: the placement was chosen with the knob (1988de6da) and the sync-budget note's 2026-07-23 amendment records "worked in `Peer::sync_memory_budget`'s docs"; the crate has since grown explanation modules (`reconciliation`, `tutorial`) that give the guide a natural home it lacked then
- Owner-gated: yes: moves public documentation the owner placed

The doc comment on one builder method runs from line 308 to an included
trade-off table at 465: the contract (what is bounded, that any budget is
deadlock-free, per session and not wire-visible, follows the peer) occupies
the first sixty lines, and the remaining hundred plus the table are a
sizing methodology with a closed form, an accuracy band, worked figures,
and measurement provenance. A reader who came for "what does this knob do"
scrolls past the derivation; a reader who came for the derivation cannot
link to it except through the method. `Bootstrap::sync_memory_budget`
already models the alternative by pointing at the contract.

Evidence:

    src/peer.rs
    308	    /// Bound the memory a synchronization may spend on pipelining.
    ...
    371	    /// # Choosing a budget
    372	    ///
    373	    /// The intuition: the budget buys parallelism on the wire. A
    ...
    465	    #[doc = include_str!("tree/mirror/streaming/window/tradeoff.md")]
    466	    #[must_use]
    467	    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {

Resolution: move "# Choosing a budget" and the table into a public
explanation module beside `reconciliation` (or a section of it), leave the
method doc with the contract plus one pointer, and have
`DEFAULT_SYNC_MEMORY_BUDGET` and reconciliation.rs:233-235 point at the new
page. Acceptance: the method doc states the contract and links to the
sizing page; the sizing page renders the table.

### api-audit-10: The `#[non_exhaustive]` policy is stated only in a commit message, so the split reads as accident in the tree
- Where: src/message.rs:116-117 (related: src/error.rs:1-28, src/link/routed/endpoint.rs:82-83, src/link/routed/endpoint.rs:111-112, src/tree/mirror/streaming/remote/streams.rs:238-239, src/tree/mirror/streaming/remote/adapter/error.rs:43-44, src/tree/mirror/streaming/remote/codec/error.rs:41-42, src/tree/mirror/streaming/remote/codec/error.rs:64-65, src/tree/mirror/streaming/remote/codec/error.rs:103-104, src/tree/mirror/streaming/materialized/error.rs:2-3)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (`grep -rn -A2 'non_exhaustive' src` lists the annotated types; the remaining public enums were read at their definitions; `git show -s 1e458d69` records the policy)
- Verification: reframed: the sweep read the mix as accident; commit 1e458d69 states a deliberate rule ("the contract-outcome and wire-grammar enums stay exhaustive deliberately"), and the cbor-wire review packet applied it to `SessionKind` (REVIEW.md:1013-1026, since done). Two of the sweep's examples do not hold: routed `Config` is a public-field struct with `Default`, where `#[non_exhaustive]` would remove literal construction and `..Config::default()` already survives new fields, and `EncodeError`'s doc does not describe admission as a growing set. What remains is that the rule lives nowhere in the tree; history: deliberate-and-holds
- Owner-gated: no for recording the rule; yes for any reclassification

`EncodeError` (three admission outcomes), `EndpointError`, `LinkError`,
`SendError`, `ReplyEncodeError`, `FramePart`, `HeadError`, and the other
exhaustive enums are exhaustive by the rule in 1e458d69, while their
siblings are open; no sentence in the tree says which class an enum
belongs to, so a downstream author cannot tell a deliberate closed grammar
from an omission. Pre-release is the moment this is cheap to record.

Evidence:

    src/message.rs
    116	#[derive(Debug, thiserror::Error)]
    117	pub enum EncodeError {

    commit 1e458d69 (message)
    A downstream wildcard arm is the correct response to an unknown variant in
    every one of these; the contract-outcome and wire-grammar enums stay
    exhaustive deliberately.

Resolution: state the rule once in the `error` module doc (open
taxonomies that grow with enforcement are `#[non_exhaustive]`; contract
outcomes and closed wire grammars are exhaustive so a match is total), and
name `EncodeError`'s class at its definition. Acceptance: the `error`
module doc carries the rule; every public exhaustive enum is either a
contract outcome or a wire grammar by that sentence.

### api-audit-11: Session settings cannot be read back
- Where: src/peer.rs:467-467 (related: src/peer.rs:549, src/peer.rs:611, src/peer.rs:182-193, src/rumors.rs:79-90, src/peer/bootstrap.rs:96-105)
- Class / severity / confidence: feature-gap / nit / medium
- Provenance: verified (`grep -n 'pub fn' src/peer.rs src/rumors.rs` shows no getter for any setting; `Bootstrap`'s manual `Debug` prints all three, `Peer`'s and `Rumors`' print none)
- Verification: reframed: the sweep also flagged the argument shapes (`usize` for the two byte budgets, a newtype for the depth limit) as inconsistent; the asymmetry has a reason the code shows: `PayloadDepthLimit` crosses the wire, is compared for equality in the handshake, and appears in `Error::PayloadDepthMismatch`, while the byte budgets are local-only and never leave the process, so bytes-as-`usize` is the std idiom there. The getter gap stands; history: no-rationale-found
- Owner-gated: yes: adds public methods

No getter exists for `sync_memory_budget`, `target_message_size`, or
`payload_depth_limit` on `Peer`, `Rumors`, or the builders, and the `Peer`
and `Rumors` `Debug` impls omit them, so a configured peer's effective
settings cannot be logged or asserted without the caller carrying the
values it passed.

Evidence:

    src/peer.rs
    467	    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {
    ...
    549	    pub fn target_message_size(mut self, bytes: usize) -> Self {
    ...
    611	    pub fn payload_depth_limit(mut self, limit: PayloadDepthLimit) -> Self {

    src/peer/bootstrap.rs (the builder already prints what the peer cannot)
    99	        f.debug_struct("Bootstrap")
    100	            .field("window", &self.window)
    101	            .field("run_budget", &self.run_budget)
    102	            .field("payload_depth_limit", &self.payload_depth_limit)

Resolution: add the three settings to `Peer`'s and `Rumors`' `Debug`
output at minimum; consider getters (or one settings view) if a caller
needs to assert them. Acceptance: a test reads each setting back from a
configured `Rumors`, or the `Debug` output shows them.

### api-audit-12: Twelve public types have no `Debug` impl
- Where: src/link.rs:322-329 (related: src/link.rs:482, src/link.rs:589-590, src/link.rs:610, src/batch.rs:27, src/rumors/unordered.rs:33, src/rumors/causal.rs:37, src/rumors/changes.rs:44, src/link/routed/endpoint.rs:141, src/link/routed/endpoint.rs:287, src/link/routed/stream.rs:24, src/link/routed/stream.rs:66, src/link.rs:345-346, src/link.rs:201-205)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`grep -rn -E 'impl(<[^>]*>)? (std::fmt::|fmt::)?Debug for|derive\([^)]*Debug' src` lists no impl in batch.rs, the three observer files, endpoint.rs, or stream.rs, and in link.rs only `Done` (201) and `SessionState` (345))
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: adds impls

`Link`, `LinkParts`, `MemoryConnector`, `MemoryAcceptor`, `Batch`,
`UnorderedMessages`, `CausalMessages`, `Changes`, `routed::Endpoint`,
`routed::Incoming`, `routed::StreamConnector`, and `routed::StreamAcceptor`
implement no `Debug`, so none can sit in a `#[derive(Debug)]` struct or an
`expect` message without a wrapper. The crate knows how to do this for
opaque types (`Done`, `Bootstrap`, `Peer`, `Rumors` print summaries), and
`SessionState` is already the summary a `Link` would print.

Evidence:

    src/link.rs
    322	pub struct Link<CR, CW, C, A> {
    323	    pub(crate) control_read: CR,
    324	    pub(crate) control_write: CW,
    325	    pub(crate) connector: C,
    326	    pub(crate) acceptor: A,
    327	    /// This link's session counter and poison latch.
    328	    pub(crate) session: SessionState,
    329	}
    ...
    345	#[derive(Clone, Copy, Debug)]
    346	pub struct SessionState {

    src/link.rs
    589	#[derive(Clone)]
    590	pub struct MemoryConnector {

Resolution: add manual `Debug` impls printing a summary independent of the
type parameters (`Link { session: .. }`, `UnorderedMessages { checkpoint:
.. }`, `Changes { seen: .. }`, `Batch { queued: .. }`), and enable
`#![warn(missing_debug_implementations)]` so the class stays closed.
Acceptance: `cargo clippy -p rumors --lib -- -D warnings` passes with
`#![warn(missing_debug_implementations)]` in lib.rs.

### api-audit-13: `MirrorError`'s public shape carries meaning the user cannot read off it
- Where: src/error.rs:52-53 (related: src/error.rs:283-289, src/tree/mirror.rs:41-50, src/tree/mirror/streaming/remote/proxy/error.rs:31-41, src/tree/mirror/streaming/materialized/error.rs:1-8, src/tree/mirror/streaming/remote/adapter/error.rs:44-47, src/tree/mirror/streaming/remote/adapter/error.rs:59-62, src/peer/gossip.rs:1154, src/peer/gossip.rs:1165, src/peer/gossip.rs:1212, src/peer/gossip.rs:1224, src/peer/gossip.rs:1406-1431)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (`grep -rn 'Error::Mirror\|streaming_error\|MirrorError' src` outside tests shows `streaming_error` as the only constructor of `Error::Mirror` and the four `.map_err(streaming_error)` sites as its only callers; `.flip()` has no non-test caller; `error/type.MirrorError.html` shows the aliased enum's variants without a mapping sentence)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: changes the public error shape or its docs

Three facts about the mirror taxonomy are invisible from its public docs.
The alias fixes `Client` to `MaterializedError` (the local participant: a
`Violation` it detected in the peer's replies) and `Server` to
`RemoteError` (the wire proxy), but neither the alias doc nor
`Error::Mirror` says so, and the variant docs speak of "position", an
internal vocabulary. Three backend-error slots are fixed to `Infallible`
(`MaterializedError::Backend`, `ReplyEncodeError::Backend`,
`ReplyDecodeError::Backend`), so an exhaustive match writes `Backend(never)
=> match never {}` arms for variants that cannot occur. And
`RemoteError::PayloadDepthMismatch` is rendered as a public variant while
every session path lifts it into `Error::PayloadDepthMismatch` before
returning, so a match on it under `Error::Mirror` is dead code.

Evidence:

    src/error.rs
    52	/// The concrete production mirror failure, retaining its detecting side.
    53	pub type MirrorError = mirror::Error<MaterializedError<Infallible>, RemoteError<Infallible>>;

    src/tree/mirror.rs
    43	pub enum Error<C, S> {
    44	    /// The protocol participant supplied in the client position failed.
    45	    #[error("mirror client failed")]
    46	    Client(#[source] C),
    47	    /// The protocol participant supplied in the server position failed.
    48	    #[error("mirror server failed")]
    49	    Server(#[source] S),

    src/peer/gossip.rs
    1419	    // The depth-limit mismatch is a configuration diagnosis, not a
    1420	    // reconciliation failure: surface it as its own top-level variant.
    1421	    // Only the proxy (the server side of every production handshake)
    1422	    // detects it; the materialized participant has no wire.
    1423	    if let tree::mirror::Error::Server(streaming_remote::Error::PayloadDepthMismatch {
    1424	        local,
    1425	        remote,
    1426	    }) = error
    1427	    {
    1428	        return Error::PayloadDepthMismatch { local, remote };
    1429	    }
    1430	    Error::Mirror(error)

Resolution: document at `MirrorError` which variant is the local
participant and which the wire proxy, in caller vocabulary. Then the owner
chooses: (a) keep the alias and add a sentence to
`RemoteError::PayloadDepthMismatch` saying the peer surface lifts it to
`Error::PayloadDepthMismatch`; or (b) present a concrete, non-generic
public `MirrorError` enum (local violation | wire failure) constructed at
`streaming_error`, keeping the generic sum crate-internal, which removes
the `Infallible` arms and the unreachable variant together. Acceptance:
the `MirrorError` docs state the mapping; either the unreachable variant's
doc says where it surfaces, or no publicly matchable variant is uninhabited
or unreachable.

### api-audit-14: Diagnostic types expose constructors and protocol-schedule helpers as public API
- Where: src/tree/mirror/streaming/remote/codec/error.rs:19-27 (related: src/tree/mirror/streaming/remote/codec/error.rs:87-94, src/tree/mirror/streaming/remote/codec/error.rs:170-185, src/tree/mirror/streaming/remote/codec/signal.rs:30-91, src/tree/mirror/streaming/remote/codec/signal.rs:119-126, src/reconciliation.rs:201-205)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (read the definitions; the callers of `Origin::direction`/`stream`, `EncodeError::new`, `DecodeError::direction`/`stream`, `Stream::new`/`at_height`/`height`, and `Speaker::other` are all inside `src/tree/mirror/streaming`, across `streams.rs`, `erased.rs`, the adapter, proxy, and codec modules)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: narrows public methods

`Origin::{direction, stream}`, `CodecEncodeError::new`,
`CodecDecodeError::{direction, stream}`, `error::Stream::{new, at_height,
height, COUNT, MAX}`, and `Speaker::other` are `pub` and rendered under
`rumors::error`, so a user can construct codec errors or compute which
logical stream carries a tree height, neither of which any public operation
accepts. `at_height`/`height` bind the public API to the wire schedule (two
heights per stream) that `reconciliation` describes as an internal choice,
and `Stream::new` is what drags the unexported `signal::StreamError` into a
public signature (api-audit-2). Their only callers are sibling modules, so
`pub(crate)` is mechanical.

Evidence:

    src/tree/mirror/streaming/remote/codec/error.rs
    19	impl Origin {
    20	    pub fn direction(speaker: Speaker) -> Self {
    21	        Origin::Direction(speaker)
    22	    }
    23	
    24	    pub fn stream(speaker: Speaker, stream: Stream) -> Self {
    25	        Origin::Stream { speaker, stream }
    26	    }
    27	}

    src/tree/mirror/streaming/remote/codec/signal.rs
    54	    /// Find the stream carrying nodes at `height` for `speaker`.
    55	    pub fn at_height(speaker: Speaker, height: usize) -> Option<Self> {
    ...
    79	    /// Find the node height carried by this stream for `speaker`.
    80	    pub fn height(self, speaker: Speaker) -> usize {

Resolution: make the constructors and schedule helpers `pub(crate)`; keep
the read accessors (`Stream::index`, `Speaker::role`,
`InvalidSignalPlacement::{stream, class}`). Acceptance: the rendered
`error/enum.Origin.html`, `error/struct.CodecEncodeError.html`,
`error/struct.CodecDecodeError.html`, and `error/struct.Stream.html` list
no constructors and no height arithmetic.

### api-audit-15: `#[doc(hidden)]` bench hooks are not feature-gated like their siblings
- Where: src/peer.rs:713-716 (related: src/peer.rs:210-213, src/rumors.rs:413-416, src/snapshot.rs:166-169, src/peer.rs:480-483, src/peer.rs:726-728, Cargo.toml:144-145, justfile:129-131)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (`grep -rln 'warm_caches\|seed_rng' benches examples tests src crates` lists only `benches/*.rs`, `tests/*.rs`, and the definitions; `seed_rng`'s callers are ten `tests/*.rs` files; Cargo.toml:145 gives the self-referential dev-dependency `features = ["test-internals", "conformance"]`, and the justfile comment at 129-131 confirms the dev-dependency cycle lights those features on the lib for every test build; benches build with dev-dependencies too)
- Verification: confirmed; history: no-rationale-found (`warm_caches` dates from cb69fc951, before `test-internals` existed as a gate)
- Owner-gated: yes: removes items from default builds

`Peer::warm_caches`, `Rumors::warm_caches`, `Snapshot::warm_caches`, and
`Peer::seed_rng` are `pub` and hidden but compiled into every build, while
the neighbouring test hooks (`sync_window_floor`,
`dangerously_alias_party`) are gated on `cfg(any(test, feature =
"test-internals"))`. The only callers are benches and integration tests,
which already build with the dev-dependency's `test-internals`, so the gate
costs nothing; hidden items are still callable and semver-relevant, and
the crate docs call `test-internals` "this crate's own test scaffolding".

Evidence:

    src/peer.rs
    713	    #[doc(hidden)]
    714	    pub fn warm_caches(&self) {
    715	        self.inner.borrow().tree.warm_caches();
    716	    }

    src/peer.rs (the sibling that is gated)
    480	    #[cfg(any(test, feature = "test-internals"))]
    481	    #[doc(hidden)]
    482	    #[must_use]
    483	    pub fn sync_window_floor(mut self) -> Self {

    Cargo.toml
    145	rumors = { workspace = true, features = ["test-internals", "conformance"] }

Resolution: add `#[cfg(any(test, feature = "test-internals"))]` to the
four items. Acceptance: `cargo doc -p rumors --no-deps` (no features)
exposes neither `warm_caches` nor `seed_rng`; `cargo bench --no-run` and
`cargo nextest run --no-run` still build.

### api-audit-16: Hazard sections are named and placed inconsistently across the public API
- Where: src/rumors.rs:563-563 (related: src/rumors.rs:133-170, src/rumors.rs:215-250, src/rumors.rs:441-501, src/peer/bootstrap.rs:219-259, src/link.rs:247-258, src/link/routed/endpoint.rs:175-190, src/link/routed/endpoint.rs:246-252, src/link/routed/endpoint.rs:295-299, src/link/routed.rs:270-281, src/peer.rs:265-271)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read each cited doc block)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`Rumors::gossip_when` titles its cancellation contract `# Cancellation`,
while `Acceptor::accept`, `Incoming::accept`, and `Listen` use `# Cancel
safety`. `Rumors::send`, `send_all`, `gossip`, and `Bootstrap::join` return
`Result`s whose failure semantics are running prose with no `# Errors`
heading, while `Peer::bookmark`, `Endpoint::new`, `Endpoint::link`,
`Connector::connect`, and `Acceptor::accept` carry one.

Evidence:

    src/rumors.rs
    563	    /// # Cancellation

    src/link.rs
    252	    /// # Cancel safety

    src/rumors.rs (an error contract without its heading)
    457	    /// On `Err`, the replica is unchanged and the link is poisoned:
    458	    /// discard it and reconnect. This is enforced, not advisory, since
    459	    /// every subsequent session on the link fails fast with

Resolution: rename `# Cancellation` to `# Cancel safety`; add `# Errors`
above the existing prose in `send`, `send_all`, `gossip`, `gossip_when`,
and `Bootstrap::join`; consider `clippy::missing_errors_doc` in the clippy
leg to hold the line. Acceptance: every public `Result`-returning method
has an `# Errors` section; every cancellation contract is titled `# Cancel
safety`.

### api-audit-17: `SessionStats` doc announces 'Two deliberate boundaries' and lists one
- Where: src/tree/mirror/streaming/stats.rs:33-38 (related: src/tree/mirror/streaming/stats.rs:14-17)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read stats.rs in full; `git show 487e17ea3:src/tree/mirror/streaming/stats.rs` shows the second bullet was "`Protocol::V1` sessions report zero in every field", deleted by the V1 retirement with the lead-in left unchanged)
- Verification: confirmed; history: leftover of 368da2a5
- Owner-gated: no

The type doc promises two boundaries and gives one bullet; the second was
the V1 zeros clause, removed with V1, and the count stayed.

Evidence:

    src/tree/mirror/streaming/stats.rs
    33	/// Two deliberate boundaries:
    34	///
    35	/// - **No duration field.** The caller owns the clock: wrap the `gossip`
    36	///   call (or the `gossip_when` stream's polls) in whatever timing
    37	///   instrument the application already uses. A duration measured inside
    38	///   the crate would bake in one notion of time and satisfy nobody's.

Resolution: change the lead-in to "One deliberate boundary:", or restore a
second bullet the module doc already implies ("Nothing here touches the
wire": no exchanged or peer-reported numbers). Acceptance: the lead-in's
count matches the bullets.

### api-audit-18: Two doc examples ignore `send`'s `Result`
- Where: src/snapshot.rs:140-143 (related: src/rumors/unordered.rs:139, src/rumors/unordered.rs:156, src/rumors.rs:228-236, src/lib.rs:295-302, justfile:122-123)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (`grep -rn -E '^\s*//[!/].*\.send\(.*\);\s*$' src` returns exactly these five lines; lib.rs carries no `#![doc(test(attr(..)))]`)
- Verification: reframed: the sweep attributed the silence to the doctest leg lacking `-D warnings`; the mechanism is rustdoc's own `#![allow(unused)]` injected into every doctest, which in-source overrides command-line lint levels, so the fix is a crate-root `#![doc(test(attr(deny(unused_must_use))))]`, not a RUSTDOCFLAGS change; history: no-rationale-found
- Owner-gated: no

`Snapshot::range`'s and `UnorderedMessages::checkpoint`'s examples call
`rumors.send(..)` as a statement, discarding the `Result<(), EncodeError>`
that every other example propagates with `?`, and teaching the one pattern
the admission contract exists to prevent.

Evidence:

    src/snapshot.rs
    140	    /// rumors.send("first".to_string());
    141	    /// let then = rumors.snapshot().latest().clone();
    142	    /// rumors.send("second".to_string());
    143	    /// rumors.send("third".to_string());

    src/rumors/unordered.rs
    139	    /// rumors.send("one".to_string());
    ...
    156	    /// rumors.send("two".to_string());

Resolution: append `?` and give the examples a `Result` return (as
`Rumors::send_all`'s example does with `# Ok::<(), EncodeError>(())`);
add `#![doc(test(attr(deny(unused_must_use))))]` to lib.rs so the class
stays closed. Acceptance: the grep above returns nothing and the attribute
is present.

## Positives

- The outcome types that carry a live `Peer` (`Retire`, `Joined`,
  `Unbookmarked`) are `#[must_use]` with messages that say exactly what
  dropping loses (gossip.rs:92, 148; bootstrap.rs:364): the compiler
  enforces the identity-custody model at the API edge.
- `Bootstrap` versus `BookmarkedBootstrap` is a typestate that gives each
  `join` only its own outcomes (bootstrap.rs:262-273), so a persist failure
  after a successful session cannot be mistaken for success.
- `Peer`, `Rumors`, and `Bootstrap` implement `Debug` by hand, print a
  summary, and say why (peer.rs:182-193, rumors.rs:79-90, bootstrap.rs:96-
  105); `Bootstrap`'s manual `Clone` carries a two-line comment naming the
  derive-bound trap (bootstrap.rs:81-84). The standard api-audit-1 asks the
  rest of the crate to meet is one the crate already wrote down.
- The `error` module doc is a table mapping every top-level variant to the
  replica's state and the caller's next action (error.rs:10-28).
- `Link`'s "What a session promises" (link.rs:279-321) states the
  `Ok`/`Err`/cancellation contract, names the three qualified exceptions to
  "unchanged", and puts the timeout with the caller; the model-of-record
  statement at link.rs:115-139 is careful to say the validation machinery
  is not a security boundary.
- `Gossiped`, `SessionStats`, `SessionInfo`, `StreamInfo`, and `StreamId`
  are `#[non_exhaustive]` data carriers with public fields, and commit
  1e458d69 records a coherent rule for which enums are open.
- `PayloadDepthLimit` is a proper newtype with `const fn new`/`get`,
  `Display`, `Default`, and a named default constant; `Network` is opaque,
  `Copy`, totally ordered, and serializes as its 16 raw bytes.
- The conformance suite's `yield_once` (conformance/link.rs:680-696) avoids
  `tokio::task::yield_now`, so the suite runs under any executor;
  `link::memory` is deterministic and closed-world.
- The tutorial is a sequence of complete programs with their exact output,
  and the crate-level example is shown whole; both compile as doctests.
- `Rumors::batch` documents and demonstrates with two `compile_fail`
  doctests that the scope handle cannot escape the closure
  (rumors.rs:320-336).
- `Snapshot::range` takes anything `Into<causally::Query>` and states that
  a small causal delta costs work proportional to the delta
  (snapshot.rs:114-161).
- The V1 retirement note records its rulings, the verification lattice,
  and an honest "what is genuinely lost" section; the retirement itself
  left few ghosts (api-audit-5, api-audit-17 are the residue).

## Open questions for Finch

1. Decision 2 of the V1 note keeps `Protocol` public as wire vocabulary.
   Does that ruling also keep `Error::VersionMismatch.local_protocol` as
   the enum (so api-audit-5 is purely a prose rewrite), or should the
   variant carry the local wire version as a number like `remote_version`?
2. Should `BookmarkError` be folded into `Bookmark` (one trait for
   implementors, `B: Bookmark` everywhere), or is there a reason for the
   split that should be written at the trait? No rationale was found in
   git or the notes.
3. Is the full depth of `rumors::error` (roughly thirty codec, adapter,
   stream, and proxy types) meant to be public API, or should the public
   surface stop at a concrete `MirrorError` with the generic sum, its
   `Infallible` slots, constructors, and schedule helpers kept
   crate-private? api-audit-13 and api-audit-14 hinge on this.
4. Does the operator sizing guide belong on `Peer::sync_memory_budget`
   (where the 2026-07-23 amendment placed it), or in an explanation module
   beside `reconciliation` now that the crate has that shape?
5. Should `Rumors` expose direct `len`/`is_empty`/`latest` readers and
   `Snapshot` a `contains(&Version)`, or is "read through a snapshot" the
   intended contract, to be stated in the `Rumors` type doc? The tutorial
   reads `snapshot().len()` five times, which suggests the direct readers
   would be used; the cost of the current path is one `Arc` bump.

## Dropped

- Sweep [18] (`Rumors` has no `len`/`is_empty`/`latest`, `Snapshot` no
  `contains`): a design question with no named cost beyond an `Arc` bump;
  moved to open question 5.
- Sweep [7]'s citations of link.rs:49-50 and link.rs:167-168: those docs
  already state provenance without a path, the form the resolution asks
  for.
- Sweep [7]'s citations of adapter/error.rs:84 and 99 (`max_version_bytes`,
  `set_len`): those are the greeting's wire field names (greeting.rs:16,
  19), observable through the `observe` hook, not private items.
- Sweep [9]'s `Config` example: a public-field struct with `Default` is
  the std idiom; `#[non_exhaustive]` would remove literal construction and
  `..Config::default()` already survives new fields.
- Sweep [9]'s claim that `EncodeError`'s docs describe admission as a
  growing set: message.rs:108-139 describes one mechanism (decode and
  compare) with three outcomes; the finding was reframed to the unrecorded
  policy (api-audit-10).
- Sweep [10]'s newtype-versus-`usize` asymmetry: `PayloadDepthLimit`
  crosses the wire and is compared in the handshake; the byte budgets are
  local-only, so the shapes differ for a reason the code shows.
- Sweep [17]'s mechanism (the doctest leg lacking `-D warnings`): rustdoc
  injects `#![allow(unused)]` into doctests, which command-line lint levels
  do not override; reframed in api-audit-18.
