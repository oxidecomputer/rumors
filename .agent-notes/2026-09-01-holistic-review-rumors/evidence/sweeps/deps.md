# Sweep deps: Dependency and feature audit

## Method and coverage

This pass disputed the sweep's eleven findings against the tree at
9e5784fb4dce977cfbdfd1619886d1482b5ce764 (working tree clean, verified with
`git rev-parse HEAD` and `git status --porcelain`). For every finding I opened
the cited lines with line numbers (`cat -n`, `sed -n`), re-ran the use-site
census with `/usr/bin/grep -rn` over `src/`, `tests/`, `benches/`,
`examples/`, and every member `Cargo.toml` (`grep` on this machine is aliased
to ugrep, which rejects the bounded-repetition patterns; the census used the
system grep throughout), and read the registry sources the claims rest on:
static_assertions 1.1.0 (`assert_eq_size.rs`, `assert_eq_align.rs`), futures
0.3.32 and futures-util 0.3.32 (the versions Cargo.lock resolves), tokio
1.52.3, and serde 1.0.228. For the docs.rs finding I ran `strings` over
`librustc_driver-*.dylib` in the pinned nightly (nightly-2026-06-30) and in
stable 1.97.1 (the `rust-toolchain.toml` channel) and extracted the
removed-features neighborhoods. History came from `git log -S` on each
originating token (`static_assertions`, `forbid(unsafe_code)`,
`doc_auto_cfg`, `futures-util`, `tokio-stream`, `pub use ::before`,
`seed_rng`, `nightly_toolchain :=`, `toolchain: nightly`) and from a grep of
`.agent-notes/` and `design/` for every one of those tokens; the two prior
review packets record no ruling on any of them. The workspace-table census
(every `[workspace.dependencies]` key counted against `workspace = true`
inheritors across the root and member manifests) found no orphan.

What this pass could not see: no cargo, rustdoc, or `cargo tree` run was
permitted, so the docs.rs breakage is inferred from the compiler's embedded
feature table rather than demonstrated, and dependency-graph claims rest on
the manifests and Cargo.lock rather than on a resolved tree. Neither of the
two permitted `cargo nextest` invocations was spent: no finding turned on a
runtime behavior a test could settle. The `.claude/worktrees/` directory
holds another agent's worktree; it was excluded from every census and left
untouched.

## Findings

### deps-1: static_assertions is a normal dependency for one test module, and one redundant test in it is the sole reason forbid(unsafe_code) is conditional
- Where: Cargo.toml:128-128 (related: src/lib.rs:295-296, src/tree/typed/height.rs:166, src/tree/typed/height.rs:175-176, src/tree/typed/height/tests.rs:6-28, crates/before/Cargo.toml:28)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (grep census of `static_assertions` over src/, tests/, benches/, examples/ and every manifest; read the three macro bodies in the registry crate; grep of `unsafe` over src/, tests/, benches/, examples/)
- Verification: confirmed and sharpened; history: deliberate-but-expired (fceb55f98, 2026-06-05, "remove all unsafe code", introduced the conditional with the same comment; today its trigger is one redundant test)
- Owner-gated: no

`static_assertions` is declared under `[dependencies]`, so every user of rumors compiles it, while its only use sites are eight macro calls in the `#[cfg(test)]` module `src/tree/typed/height/tests.rs`. Of the three macros used, only `assert_eq_size_val!` (through `assert_eq_size_ptr!`) expands to `#[allow(... unsafe_code ...)] let _ = || unsafe { ... }`; `assert_eq_size!` and `assert_eq_align!` expand to `const _: fn() = || { ... }` with no `unsafe`, so the single test `zero_size_val` is what keeps `#![forbid(unsafe_code)]` from being unconditional, and that test asserts nothing `zero_size` does not already assert (a `Sized` value's size is its type's size). `height.rs:166` already states a type-level fact with the std idiom `const _: () = assert!(...)`, so the replacement pattern is in the same file.

Evidence:

    Cargo.toml
       128	static_assertions = { workspace = true }

    src/lib.rs
       295	// Static assertions uses #[allow(unsafe_code)], so we allow it only in tests
       296	#![cfg_attr(not(test), forbid(unsafe_code))]

    src/tree/typed/height.rs
       166	const _: () = assert!(H0::HEIGHT == 0 && H32::HEIGHT == 32);
       175	#[cfg(test)]
       176	mod tests;

    src/tree/typed/height/tests.rs
        13	/// Height *values* (not just the types) are zero-sized, so constructing
        14	/// one is free.
        15	#[test]
        16	fn zero_size_val() {
        17	    static_assertions::assert_eq_size_val!(Z, ());
        18	    static_assertions::assert_eq_size_val!(S::<Z>::default(), ());
        19	}

    ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/static_assertions-1.1.0/src/assert_eq_size.rs
        34	macro_rules! assert_eq_size {
        36	        const _: fn() = || {
        37	            $(let _ = $crate::_core::mem::transmute::<$x, $xs>;)+
        76	macro_rules! assert_eq_size_ptr {
        78	        #[allow(unknown_lints, unsafe_code, forget_copy, useless_transmute)]
        79	        let _ = || unsafe {
       119	macro_rules! assert_eq_size_val {

Resolution: Delete `zero_size_val` (its testdoc claims a value/type distinction that does not exist for `Sized` types, and `zero_size` covers both of its checks). Replace the remaining six assertions with `const _: () = assert!(size_of::<Z>() == 0 && align_of::<Z>() == 1);` and likewise for `S<Z>` and `Root`, either in `height.rs` beside line 166 (they are compile-time facts and need no `#[test]` wrapper to fire) or in `tests.rs` with the two testdocs kept. Remove `static_assertions` from rumors' `[dependencies]` (the workspace table entry stays; `crates/before` inherits it at its line 28). Make the crate attribute an unconditional `#![forbid(unsafe_code)]` and delete the comment at lib.rs:295. Acceptance: rumors' `[dependencies]` has no `static_assertions` entry; lib.rs carries `#![forbid(unsafe_code)]` with no `cfg_attr`; the three layout facts are `const _` assertions that fail compilation if a height type gains size or alignment; `just gate` is clean.

### deps-2: tokio-stream patches a Stream gap channel.rs leaves open, and two consumers carry duplicated cfg(test) duals because of it
- Where: src/tree/mirror/streaming/channel.rs:75-76 (related: src/tree/mirror/streaming/erased.rs:46-47, src/tree/mirror/streaming/erased.rs:141-159, src/tree/mirror/streaming/materialized/common.rs:4-5, src/tree/mirror/streaming/materialized/common.rs:47-65, src/tree/mirror/streaming/materialized/work/assembly.rs:6-7, src/tree/mirror/streaming/materialized/work/assembly.rs:73, src/tree/mirror/streaming/materialized/work/assembly.rs:84, src/tree/mirror/streaming/remote/adapter/decode.rs:7-8, src/tree/mirror/streaming/remote/adapter/decode.rs:83-84, src/tree/mirror/streaming/remote/adapter/decode.rs:399-404, src/tree/mirror/streaming/channel/instrumented.rs:152-182, Cargo.toml:138)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (grep census of `tokio_stream` over src/; read channel.rs in full, instrumented.rs:140-190, erased.rs:105-165, common.rs:1-70, assembly.rs:1-20 and 60-90, decode.rs:1-30 with its `ReceiverStream`/`mpsc` sites; grep of `mpsc::channel|mpsc::Receiver|mpsc::Sender` and of `.recv()|.try_recv(` over the streaming module; read futures-util 0.3.32 `stream/stream/mod.rs` for `fn next` (273) and `fn fuse` (1245))
- Verification: reframed in two places (below); history: no-rationale-found (tokio-stream arrived with 61ddf3a5 "WIP: streaming protocol, mostly reorg"; the 2c73d032 table migration moved it without pruning, its message stating `cargo tree` was byte-identical)
- Owner-gated: no

channel.rs exists to make the test and production channel types interchangeable, but its production side re-exports tokio's `Receiver` bare, which does not implement `Stream`, while the instrumented `Receiver` does (instrumented.rs:176). Two consumers therefore carry `#[cfg(test)]`/`#[cfg(not(test))]` duals of a type alias plus a function body each, wrapping the production receiver in `tokio_stream::wrappers::ReceiverStream` (erased.rs:141-159, common.rs:47-65), and assembly.rs imports `tokio_stream::StreamExt` for `.fuse()` and `.next()`, methods `futures::StreamExt` (imported in the same crate everywhere else, and `futures::{Stream, stream}` is imported in that file) provides identically. Two corrections to the sweep: the crate's tokio-stream footprint is five sites in four files, not four, because decode.rs wraps two raw `tokio::sync::mpsc` receivers (lines 84 and 404) that never pass through channel.rs; and the `try_recv` sites the sweep listed as forwarding surface (driver.rs:80, streams.rs:570) are on raw `tokio::sync::mpsc` receivers (`FirstError(mpsc::Receiver<E>)` at driver.rs:66, `receive: mpsc::Receiver<StreamError>` at streams.rs:529), not channel.rs's type, and the instrumented `Receiver` itself exposes only `recv` and `Stream`, so the production wrapper's forwarding surface is `recv` alone.

Evidence:

    src/tree/mirror/streaming/channel.rs
        75	#[cfg(not(test))]
        76	pub use tokio::sync::mpsc::{Receiver, Sender};

    src/tree/mirror/streaming/erased.rs
       141	/// The channel receiver as a stream, uniform across the test and
       142	/// production channel types.
       143	#[cfg(test)]
       144	type ReceiverStreamOf<E> = Receiver<E>;
       145	/// The channel receiver as a stream, uniform across the test and
       146	/// production channel types.
       147	#[cfg(not(test))]
       148	type ReceiverStreamOf<E> = ReceiverStream<E>;
       149	
       150	fn receiver_stream<E: Send>(receiver: Receiver<E>) -> ReceiverStreamOf<E> {
       151	    #[cfg(test)]
       152	    {
       153	        receiver
       154	    }
       155	    #[cfg(not(test))]
       156	    {
       157	        ReceiverStream::new(receiver)
       158	    }
       159	}

    src/tree/mirror/streaming/materialized/common.rs
        60	/// The type of a receiver stream wrapping items in `Ok`.
        61	#[cfg(test)]
        62	pub type OkReceiverStream<T, E> = stream::Map<Receiver<T>, fn(T) -> Result<T, E>>;
        63	/// The type of a receiver stream wrapping items in `Ok`.
        64	#[cfg(not(test))]
        65	pub type OkReceiverStream<T, E> = stream::Map<ReceiverStream<T>, fn(T) -> Result<T, E>>;

    src/tree/mirror/streaming/materialized/work/assembly.rs
         6	use futures::{Stream, stream};
         7	use tokio_stream::StreamExt;
        73	        let mut level = pin!(level.fuse());
        84	                        next_or_cancelled(level.next()).await?

    src/tree/mirror/streaming/remote/adapter/decode.rs
         7	use tokio::sync::mpsc;
         8	use tokio_stream::wrappers::ReceiverStream;
        83	        let (tx, rx) = mpsc::channel::<Result<(Prefix<Z>, B::Node<Z>), B::Error>>(FAN);
        84	        let leaves = ReceiverStream::new(rx);
       404	    let leaves = ReceiverStream::new(leaves);

    src/tree/mirror/streaming/channel/instrumented.rs
       152	impl<T> Receiver<T> {
       153	    /// Receive one item after the scheduled suspension points.
       154	    pub async fn recv(&mut self) -> Option<T> {
       176	impl<T> Stream for Receiver<T> {

Resolution: Move the seam into channel.rs. (a) Minimal: channel.rs exports under both cfgs a `pub type ReceiverStream<T>` (tokio-stream's wrapper in production, `Receiver<T>` under test) and `pub fn into_stream<T>(rx: Receiver<T>) -> ReceiverStream<T>`; erased.rs:141-159 and common.rs:47-65 collapse to one line each and lose their duals; assembly.rs:7 becomes `futures::StreamExt`. (b) Full: in production channel.rs defines `pub struct Receiver<T>(tokio::sync::mpsc::Receiver<T>)` implementing `Stream` via `poll_recv` and forwarding `recv` (the only method channel.rs consumers call: materialized.rs:874, levels.rs:140/381/565/662, encode.rs:59/95, pump.rs:191/289/383), which makes both flavors `Stream`; decode.rs's two raw channels then either route through channel.rs (which also brings the adapter's leaf channel under the instrumented channel's test observation) or take a local `poll_recv`-based wrapper, after which `tokio-stream` leaves rumors' `[dependencies]` and the workspace table. Acceptance: erased.rs and common.rs contain no `#[cfg(test)]`/`#[cfg(not(test))]` pair around the receiver-stream type; `grep -rn 'tokio_stream::StreamExt' src/` is empty; under (b), `grep -rn tokio_stream src/` is empty and `cargo tree -p rumors -e normal --depth 1` omits tokio-stream.

### deps-3: the docs.rs configuration names a feature gate the toolchain records as removed, and no gate or ci leg builds under `--cfg docsrs`
- Where: src/lib.rs:297-299 (related: Cargo.toml:89-94, justfile:256-258, justfile:268-270, justfile:1000)
- Class / severity / confidence: correctness / medium / medium
- Provenance: assessed (read; `strings` over `librustc_driver-66ceb92b8eba7452.dylib` in nightly-2026-06-30 and over the 1.97.1 stable dylib; no rustdoc run was permitted)
- Verification: confirmed as far as static evidence permits; history: no-rationale-found (dfd19c447, 2026-07-24, "round 4: the style doctrine, applied" added both lines with the message fragment "the Cargo features section and docsrs auto-cfg"; Rust 1.92.0 predates that commit, so the attribute has never been accepted by a then-current nightly)
- Owner-gated: no

The crate enables `doc_auto_cfg` under `cfg(docsrs)`, and Cargo.toml's docs.rs metadata passes `--cfg docsrs`. The removed-features table embedded in both the pinned nightly and the 1.97.1 stable compiler carries an entry with reason "merged into `doc_cfg`" at version 1.92.0; `doc_auto_cfg` is the rustdoc feature RFC 3631 merged into `doc_cfg`, and the same binaries carry the successor syntax's diagnostics ("`#[doc(auto_cfg)]` is experimental"). The string pool does not place the table's name column beside its reason column, which is why the mapping is inferred rather than read, and why confidence is medium. On a post-1.92 nightly, which is what docs.rs builds with, `#![feature(doc_auto_cfg)]` is an error (feature has been removed). Locally, `docs` and `docs-internal` run stable `cargo doc` without `--cfg docsrs`, and no ci leg passes it either, so nothing in the tree ever compiles the docsrs path.

Evidence:

    src/lib.rs
       297	// docs.rs builds pass `--cfg docsrs` (see Cargo.toml's docs.rs metadata), so
       298	// every feature-gated item self-labels its gate there; inert on stable builds.
       299	#![cfg_attr(docsrs, feature(doc_auto_cfg))]

    Cargo.toml
        89	# docs.rs renders every feature-gated module the crate docs advertise
        90	# (`conformance`), matching the gate's all-features rustdoc passes. The
        91	# `docsrs` cfg turns on lib.rs's `doc_auto_cfg`, so gated items self-label.
        92	[package.metadata.docs.rs]
        93	all-features = true
        94	rustdoc-args = ["--cfg", "docsrs"]

    justfile
       258	    RUSTDOCFLAGS="-D warnings --html-in-header {{ justfile_directory() }}/crates/before/docs/fuelscape-header.html" cargo doc --workspace --all-features --no-deps

    strings, nightly-2026-06-30 librustc_driver (removed-features neighborhood, one line of the pool)
    ...replaced by `CoercePointee`1.84.0renamed to `diagnostic_on_unmatched_args`CURRENT_RUSTC_VERSIONmerged into `doc_cfg`1.92.0merged into `#![feature(rustdoc_in...

    strings, 1.97.1 librustc_driver
    ...replaced by `CoercePointee`1.84.0merged into `doc_cfg`1.92.0merged into `#![feature(rustdoc_i...

Resolution: Change lib.rs:299 to `#![cfg_attr(docsrs, feature(doc_cfg))]` (under RFC 3631, `doc_cfg` labels gated items automatically; `#[doc(auto_cfg = false)]` opts out) and reword Cargo.toml:91 and lib.rs:297-298 to name `doc_cfg`. Add a leg to `ci` (and consider `gate`; it is one nightly rustdoc of one crate): `RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +{{ nightly_toolchain }} doc -p rumors --all-features --no-deps --target-dir target/doc-docsrs`. Acceptance: a committed justfile leg runs rustdoc for rumors under the pinned nightly with `--cfg docsrs` and `-D warnings`, `just ci` includes it, and lib.rs and Cargo.toml name the gate that leg accepts.
Construction: at HEAD, `RUSTDOCFLAGS="--cfg docsrs" cargo +nightly-2026-06-30 doc -p rumors --all-features --no-deps`. Expected: `error[E0557]: feature has been removed` naming `doc_auto_cfg`, with the note "merged into `doc_cfg`". After the fix the same command succeeds and `conformance` renders with its feature label. This one command settles the medium confidence either way.

### deps-4: futures-util is fully shadowed by futures at all six sites
- Where: Cargo.toml:139-140 (related: Cargo.toml:52-53, src/bookmark.rs:17, src/tree/mirror/handshake.rs:329, src/tree/mirror/streaming/remote/proxy/start.rs:219, src/peer/gossip.rs:13, src/peer/gossip.rs:946, src/peer/gossip.rs:1313)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep census of `futures_util` over src/, tests/, benches/, examples/; grep of `futures-util` over every manifest in the workspace; read futures-0.3.32/src/lib.rs re-export lines and its `[features]` table; Cargo.lock resolves futures and futures-util both to 0.3.32)
- Verification: confirmed; history: no-rationale-found (60a3b056 added it; 2c73d032 moved it into the table unchanged)
- Owner-gated: no

Every `futures_util` path the crate uses (`FutureExt`, `StreamExt`, `future::try_join`, `stream::unfold`) is re-exported at the root of futures 0.3.32 (`pub use futures_util::future::{FutureExt, TryFutureExt};` at lib.rs:101, `pub use futures_util::stream::{StreamExt, TryStreamExt};` at 106, `pub use futures_util::{future, never, sink, stream, task};` at 131), and futures' `std` feature enables `futures-util/std`, so the second declaration adds a second spelling for the same items and one more table row. No other manifest inherits `futures-util`.

Evidence:

    Cargo.toml
        52	futures = { version = "0.3", default-features = false, features = ["std", "async-await"] }
        53	futures-util = { version = "0.3", default-features = false, features = ["std"] }
       139	futures = { workspace = true }
       140	futures-util = { workspace = true }

    src/peer/gossip.rs
        13	use futures_util::StreamExt;
    src/tree/mirror/handshake.rs
       329	    futures_util::future::try_join(write, read).await?;
    src/bookmark.rs
        17	use futures_util::FutureExt;

Resolution: Replace `futures_util::` with `futures::` at the six sites; remove `futures-util` from rumors' `[dependencies]` and from `[workspace.dependencies]`. Acceptance: `grep -rn futures_util src/` is empty; the workspace table has no `futures-util` row; `cargo tree -p rumors -e normal --depth 1` lists futures and not futures-util.

### deps-5: rand's RngCore reaches the public surface through an ungated, doc-hidden constructor that only tests call
- Where: src/peer.rs:210-213 (related: src/peer.rs:8, src/peer.rs:207, src/peer.rs:480-481, src/peer.rs:726-727, src/rumors.rs:420-421, src/network.rs:62, design/rumors-frame-fuzz.md:78-79, eleven `tests/*.rs` call sites)
- Class / severity / confidence: api-surprise / low / high
- Provenance: verified (grep of `seed_rng` over src/, tests/, benches/, examples/, crates/rumors-tracing, fuzz workspaces; grep of `pub use rand` over src/ (empty); read peer.rs:195-225, 475-490, 720-735, rumors.rs:415-425, network.rs:50-80)
- Verification: confirmed; history: deliberate-and-holds for the constructor itself (0f4461932 introduced it), no-rationale-found for its gating differing from its siblings
- Owner-gated: yes: any change to a `pub` item, hidden or not, is a public API decision

`Peer::seed_rng<R: RngCore + ?Sized>` is `pub` and `#[doc(hidden)]`, takes rand 0.8's `RngCore` (rand is not re-exported, so a caller must depend on the same rand major), and outside `seed()` is called only from eleven test files. Every other test seam on `Peer` and `Rumors` is gated `#[cfg(any(test, feature = "test-internals"))]`; this one is not. The planned fuzz harness (design/rumors-frame-fuzz.md:78-79) pairs `Peer::seed_rng` with `sync_window_floor()`, which is gated, so that consumer must enable `test-internals` regardless and loses nothing if `seed_rng` is gated the same way.

Evidence:

    src/peer.rs
       210	    /// Like [`seed`](Self::seed), but draws the universe's [`Network`]
       211	    /// identifier from a caller-supplied RNG instead of [`OsRng`].
       212	    #[doc(hidden)]
       213	    pub fn seed_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self {
       480	    #[cfg(any(test, feature = "test-internals"))]
       481	    #[doc(hidden)]
       726	    #[cfg(any(test, feature = "test-internals"))]
       727	    #[doc(hidden)]

    tests/retire_snapshot.rs
        41	    Peer::seed_rng(&mut SmallRng::seed_from_u64(0))

Resolution: Either gate it like its siblings (`#[cfg(any(test, feature = "test-internals"))]`, keeping or dropping `#[doc(hidden)]` to match them), which keeps rand off the shipped surface; or make it a documented constructor over a dependency-free input (`seed_with_network_bytes([u8; 16])`, rejecting the all-zero sentinel as `Network::from_rng` does). The test call sites adapt either way. Acceptance: either `seed_rng` compiles only under test/test-internals, or the seeded constructor's signature names no rand type.

### deps-6: `pub use ::before;` puts a whole crate on the public surface beside an explicit re-export list, with the choice unrecorded at the site
- Where: src/lib.rs:328-330 (related: src/lib.rs:93, src/bookmark.rs:149, src/rumors.rs:422, src/peer.rs:728)
- Class / severity / confidence: api-surprise / low / medium
- Provenance: assessed (read lib.rs in full; grep of `pub fn|pub trait|pub struct|pub enum|pub type` mentioning `Clock|Party|Dominance|Ticks|Version` over non-test src with `pub(crate)`/`pub(super)` excluded; grep of `rumors::before` over tests/, benches/, examples/, crates/rumors-tracing/src (empty; the tests import `before::...` from the dev-dependency directly))
- Verification: reframed (severity lowered from medium; the rustdoc-rendering claim dropped, see Dropped); history: deliberate-and-holds with a thin record (8dc0596ed, 2026-06-11, "Also re-export `before` (and Version/causally from it) at the crate root"; the same commit added `pub use ::borsh;`, since removed with borsh, so the whole-crate pattern has already contracted once)
- Owner-gated: yes: public API shape

Line 328 re-exports all of `before` as `rumors::before`; line 330 re-exports three of its items by name. The whole-crate form makes every public path of `before` part of rumors' API, so a `before` breaking change is a rumors breaking change, and users have two spellings for the named items. The standard reason for such a re-export holds here: rumors' ungated public signatures name `Version` (Rumors, Batch, Snapshot, the checkpoints), `Ticks`, `causally`, and `Clock` (`BookmarkIo` returns `BTreeMap<Network, Vec<Clock>>`), and a user implementing `BookmarkIo` needs the exact `before` version rumors compiled against; `Party` reaches the ungated surface nowhere (both `dangerously_alias_party` exits are test-internals-gated). Nothing at the site says which consideration won, and the explicit list on the next line reads as the intended shape.

Evidence:

    src/lib.rs
       328	pub use ::before;
       329	pub use batch::Batch;
       330	pub use before::{Ticks, Version, causally};

    src/bookmark.rs
       149	    ) -> impl Future<Output = Result<BTreeMap<Network, Vec<Clock>>, BookmarkIo<Self::Error>>> + Send;

Resolution: Owner decision between (1) keeping the whole-crate re-export as the version-pinning path for `before` types in rumors' signatures, recorded in a one-line comment at line 328, and then either dropping the duplicate spellings at line 330 or saying why both exist; or (2) replacing line 328 with an enumerated `pub mod before { pub use ::before::{Clock, Ticks, Version, causally, ...}; }` covering the items reachable from rumors' ungated signatures and their methods' return types, with the crate docs' [`before`] link (lib.rs:93) retargeted. Acceptance: line 328 carries either a comment stating the choice or an enumerated module; the named re-exports at 330 are either unique spellings or explained.

### deps-7: ci.yml installs toolchains the recipes no longer invoke, and its comment cites a `cargo +nightly` call the justfile does not make
- Where: .github/workflows/ci.yml:54-74 (related: .github/workflows/ci.yml:17-20, justfile:23-40, justfile:123, justfile:368, justfile:555-559, rust-toolchain.toml:28-31)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read ci.yml:1-110, justfile:20-42, rust-toolchain.toml; grep of `nightly` over the justfile; `git log -p -S 'nightly_toolchain :='` shows the variable born as `"nightly"` in 788b5366f (2026-07-15) and pinned to `"nightly-2026-06-30"` in e7a4b7b0 (2026-08-04), whose stat touches AGENTS.md, the fuzzfit bands, the justfile, and rust-toolchain.toml but not ci.yml; ci.yml's only later commits are Dependabot SHA bumps)
- Verification: raised in this verification pass, not in the sweep's report; history: deliberate-but-expired (the comment was accurate when the variable was `"nightly"`; both pins landed afterward without touching ci.yml)
- Owner-gated: no

The workflow installs a floating `nightly` (line 67) with a comment saying "The doctest and fuzz recipes invoke `cargo +nightly`", and installs `stable` (line 72) so that "the gate's bare `cargo fmt`/`clippy`/`check` must hit stable". Neither is what runs: every nightly recipe invokes `cargo +{{ nightly_toolchain }}` with the variable pinned to `nightly-2026-06-30`, and `rust-toolchain.toml` pins `1.97.1`, which rustup resolves for every bare `cargo` regardless of the default. Both pinned toolchains reach the runner only through rustup's implicit auto-install on first use, which the workflow neither states nor controls, so CI's toolchain provenance is the opposite of what its comments claim and rests on a rustup default the tree does not pin. The justfile's own comment (23-33) gives the argument against a floating nightly that this workflow step contradicts.

Evidence:

    .github/workflows/ci.yml
        54	      # Install nightly *first* so the stable install below wins the default:
        55	      # each dtolnay/rust-toolchain step runs `rustup default`, last one sticks.
        56	      # The doctest and fuzz recipes invoke `cargo +nightly`, so nightly only
        57	      # needs to exist, not be the default — whereas the gate's bare
        58	      # `cargo fmt`/`clippy`/`check` must hit stable.
        64	      - name: Install nightly toolchain (merged doctests and fuzz build)
        65	        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
        66	        with:
        67	          toolchain: nightly
        69	      - name: Install stable toolchain (default)
        70	        uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772 # v1
        71	        with:
        72	          toolchain: stable

    justfile
        40	nightly_toolchain := "nightly-2026-06-30"
       368	    {{ justfile_directory() }}/tools/memwatch cargo +{{ nightly_toolchain }} fuzz build --target {{ host_triple }}

    rust-toolchain.toml
        28	[toolchain]
        29	channel = "1.97.1"

Resolution: Install the pinned toolchains by name in the workflow (the stable step names `1.97.1`, or is dropped if the action honors `rust-toolchain.toml` without a `toolchain:` input; the nightly step names `nightly-2026-06-30`, ideally read from the justfile so the two cannot diverge, or the justfile's pin comment names ci.yml as the second site to move), and rewrite the comments at 17-20 and 54-58 to describe the pins. Acceptance: the workflow names no floating channel; `grep -n 'cargo +nightly' .github/workflows/ci.yml` is empty; the nightly pin appears in exactly one place or its two sites name each other.

### deps-8: bytes' `serde` feature is enabled but no serde path touches a `Bytes`
- Where: Cargo.toml:126-126 (related: src/message.rs:7, src/message.rs:47-51, src/message.rs:565-568, src/tree/mirror/streaming/remote/codec/frame.rs:370)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (grep of `Bytes` over non-test src: the only `Bytes` field is `Message.serialized`; `Message` derives `Clone` alone and its `Serialize` is hand-written; no `derive(Serialize)`/`derive(Deserialize)` struct in src holds a `Bytes`)
- Verification: confirmed; history: deliberate-but-expired (f2b74a97 "wire: retire borsh for CBOR payloads" carried the feature into the CBOR era; 2c73d032 moved it unchanged)
- Owner-gated: no

The only serde traffic involving `Bytes` is `serializer.serialize_bytes(&self.serialized)`, a `&[u8]` deref that never calls bytes' own `Serialize` impl. Feature unification through `before`'s bytes dependency (which requests no features) does not light it either, so the selection declares a need the code does not have.

Evidence:

    Cargo.toml
       126	bytes = { workspace = true, features = ["serde"] }

    src/message.rs
        47	#[derive(Clone)]
        48	pub struct Message {
        49	    message: Arc<dyn Any + Send + Sync>,
        50	    serialized: Bytes,
        51	}
       565	impl Serialize for Message {
       566	    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
       567	        serializer.serialize_bytes(&self.serialized)

Resolution: `bytes = { workspace = true }`. Acceptance: the bytes entry declares no features and `just gate` is clean.

### deps-9: the tokio dev-dependency's `default-features = true` is vacuous
- Where: Cargo.toml:153-153 (related: Cargo.toml:12-13, Cargo.toml:78, Cargo.toml:130)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (read tokio-1.52.3/Cargo.toml:70 `default = []` in the registry; Cargo.lock resolves tokio 1.52.3; for contrast read serde-1.0.228/Cargo.toml:67 `default = ["std"]`, which makes the serde entry's identical key at line 130 meaningful)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The workspace table sets `default-features = false` for tokio and the dev-dependency widens it back; tokio's default feature set is empty, so the widening enables nothing and the explicit list on lines 154-162 is the whole selection. The same key on the serde entry (line 130) does real work, which is what makes the vacuous one misleading.

Evidence:

    Cargo.toml
        78	tokio = { version = "1", default-features = false }
       153	tokio = { workspace = true, default-features = true, features = [

    ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/Cargo.toml
        70	default = []

Resolution: Delete `default-features = true` from the tokio dev-dependency entry. Acceptance: the entry carries `workspace = true` and `features = [...]` only.

### deps-10: the crate docs' feature list claims completeness and omits `meter`
- Where: src/lib.rs:274-282 (related: Cargo.toml:117-122, README.md:278-286, src/tree/tests.rs:1398, src/tree/traverse/unknown/tests.rs:5, src/tree/typed/untyped/tests.rs:644)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read lib.rs:274-282 and README.md:274-290; grep of `feature = "meter"` over src/, tests/, benches/, examples/)
- Verification: confirmed; history: deliberate-but-expired (dfd19c447 wrote the section; `meter` is gated at three suites)
- Owner-gated: no

The `# Cargo features` section opens with "Every feature is off by default." and lists two of the three features Cargo.toml defines; `meter` is visible on crates.io and docs.rs like any feature, and the README mirrors the omission because it is derived by `tools/readme`.

Evidence:

    src/lib.rs
       274	//! # Cargo features
       275	//!
       276	//! Every feature is off by default.
       277	//!
       278	//! - `conformance`: the public validation suite for caller-built [`link`]
       279	//!   instantiations (the [`conformance::link`] module). Enable it from a
       280	//!   dev-dependency; it is safe, though pointless, in an application.
       281	//! - `test-internals`: this crate's own test scaffolding, enabled through
       282	//!   its self-referential dev-dependency. Never enable it in an application.

    Cargo.toml
       122	meter = ["before/limb-meter", "before/scan-meter"]

Resolution: Add a `meter` bullet ("this crate's own metering suites; lights `before`'s deterministic counters. Test-only, like `test-internals`; never enable it in an application.") and run `just readme`. Acceptance: lib.rs and the regenerated README name all three features; `just readme-check` is clean.

### deps-11: the `meter` feature comment names one suite as if it were the roster
- Where: Cargo.toml:117-118 (related: src/tree/tests.rs:1398, src/tree/traverse/unknown/tests.rs:5, src/tree/typed/untyped/tests.rs:644)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep of `feature = "meter"` over src/: three gated sites)
- Verification: confirmed; history: deliberate-but-expired (the parenthetical named the only suite at the time it was written; two more now share the gate)
- Owner-gated: no

The parenthetical "(the branch bound memos' fold-cost comparison)" names one of three suites gated on the feature, so it reads as the enumeration and will rot as suites are added.

Evidence:

    Cargo.toml
       117	# Lights `before`'s deterministic resource counters for this crate's own
       118	# metering tests (the branch bound memos' fold-cost comparison). Test-only

Resolution: Name the class: "for this crate's own metering suites (the tree's deterministic cost comparisons, gated `#[cfg(feature = \"meter\")]`)". Acceptance: the comment names no individual suite.

### deps-12: rumors' `[package]` alone in the workspace lacks `description` and `license`
- Where: Cargo.toml:83-87 (related: crates/before/Cargo.toml:5-6, crates/rumors-tracing/Cargo.toml:5-6, crates/suanpan/Cargo.toml:5-6, crates/surface-scan/Cargo.toml:5-6, crates/before-viz/Cargo.toml:5-6)
- Class / severity / confidence: feature-gap / nit / high
- Provenance: verified (read every member `[package]` table)
- Verification: confirmed and sharpened; history: no-rationale-found
- Owner-gated: yes: the description is published prose in Finch's name

Every sibling crate carries `description` and `license = "MPL-2.0"` (surface-scan and before-viz additionally `publish = false`); rumors, the crate the workspace exists to ship, carries neither, and `cargo publish` refuses a crate without `license` and `description`. The license is therefore not an open decision but a workspace convention rumors has not joined.

Evidence:

    Cargo.toml
        83	[package]
        84	name = "rumors"
        85	version = "0.1.0"
        86	edition = "2024"
        87	readme = "README.md"

    crates/before/Cargo.toml
         5	description = "Interval Tree Clocks (Almeida, Baquero & Fonte, 2008): packed bit-stream storage, transient fixed-width working form, linear-typed API."
         6	license = "MPL-2.0"

Resolution: Add `license = "MPL-2.0"` and a one-sentence `description` (the crate doc's first sentence is the natural source); add `repository` when the crate's public home is settled. Acceptance: `cargo publish --dry-run -p rumors` passes the manifest checks.

## Positives

- The workspace dependency table is the single version of record and the census confirms it: every `[workspace.dependencies]` key has at least one `workspace = true` inheritor across the root and member manifests (verified by script), and `tools/manifestlint`'s header states both the rule and the reason cargo cannot check it itself.
- The tokio normal-dependency feature set (`io-util`, `macros`, `sync`) matches what the shipped library uses: `tokio::select!` at six non-test sites (tasks.rs:26, materialized.rs:841, driver.rs:73, remote/proxy/work.rs:215, peer/gossip.rs:966, link/routed/router.rs:149), and no `tokio::task`, `tokio::spawn`, `tokio::time`, or `tokio::runtime` outside doctests and test scaffolding (verified by grep). The crate docs' runtime-independence claim is true of the manifest.
- `conformance = []` adds no tokio features; the suite's only mention of `tokio::task::yield_now` is the contrast sentence at conformance/link.rs:684, and the `features` leg (justfile:522) checks the feature alone.
- `height.rs:166` already states a type-level fact with `const _: () = assert!(...)`: the idiom deps-1 asks for is in the file.
- The ciborium exact pin (Cargo.toml:36-46) and the cbor-diag fork comment (26-35) state the invariant and the bump procedure at the right altitude; the `[lib] bench = false` comment (97-106) names the concrete cost it avoids.
- The sibling crates' `[package]` tables are complete and consistent (description, MPL-2.0, `publish = false` where test-only), which is what makes deps-12 a one-line fix rather than a decision.
- The justfile's nightly-pin comment (23-38) gives the reproducibility argument in full and names the paired bump procedure; deps-7 is about ci.yml not having followed it, not about the argument.

## Open questions for Finch

1. decode.rs builds its leaf channels on raw `tokio::sync::mpsc` (lines 83 and 288) rather than channel.rs's `channel(QueueRole, ..)`, so in tests that edge escapes the instrumented channel's capacity limits, delay schedule, and coverage assertions (`QueueKind::PROXY` lists three proxy edges; this one is not among them). Is that deliberate (the adapter's internal fan-out is not a protocol edge) or an omission the streaming partition should pick up? It bears on which option deps-2 takes.
2. rand is on the 0.8 line workspace-wide while 0.9 changed the core traits. Nothing in rumors forces a bump; `seed_rng` (deps-5) is the one place the choice leaks to users, and gating it makes the bump a private matter.
3. The `meter` feature reaches `before::meter` two ways (rumors' feature forwards to `before/limb-meter` and `before/scan-meter`, each implying `before/meter`; the dev-dependency lights `before/meter` directly for `encoded_bits`). Sound, but one sentence at Cargo.toml:117-122 saying which route each consumer relies on would save the next reader the derivation.
4. deps-3's construction command is one nightly rustdoc; running it before any of this lands settles the medium confidence and shows whether docs.rs has ever built this crate's docs.
5. For deps-7: do you want CI to read the nightly pin from the justfile (a `just --evaluate nightly_toolchain` step), or to carry a second copy with the two sites naming each other?

## Dropped

- The sweep's claim under the `pub use ::before;` finding that "docs.rs renders before's surface under rumors": rustdoc renders an external-crate re-export as a `pub use before;` line linking to that crate's own docs and does not inline it without `#[doc(inline)]`; the semver coupling and duplicate spellings stand, the rendering claim does not (assessed; no rustdoc run permitted).
- The sweep's count of tokio-stream's footprint as "four sites": decode.rs:8 (uses at 84 and 404) is a fifth, on raw tokio channels, and changes what dissolving the dependency costs (folded into deps-2).
- The sweep's forwarding surface for the production `Receiver` wrapper, which listed `try_recv` at driver.rs:80 and streams.rs:570: both are raw `tokio::sync::mpsc` receivers outside channel.rs, and the instrumented `Receiver` exposes only `recv`; the wrapper forwards `recv` alone (folded into deps-2).
- The `pub use ::before;` finding's medium severity: lowered to low, because the re-export has the standard version-pinning rationale for `before` types in rumors' public signatures and the actionable remainder is recording the choice.
