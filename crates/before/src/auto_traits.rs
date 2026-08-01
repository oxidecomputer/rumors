//! Compile-time pins on the auto traits of every public API type.
//!
//! Every type on the public API surface is `Send`, `Sync`, and `Unpin`,
//! and callers (executors, work-stealing pools, containers that demand
//! `Send` values) are entitled to rely on it. Auto traits are inferred
//! from fields, so a field addition can silently revoke them; these
//! assertions turn that revocation into a compile error at the field, in
//! every build of the crate. Dropping one of these guarantees is a
//! deliberate API event: it must change this file, where the reviewer
//! sees it by name.
//!
//! The roster below covers the API surface — the root types (the `span`
//! module's among them, pinned at their root re-export names), the
//! `causally`, `error`, and `iter` modules. The feature-gated instrument
//! trees (`oracle`, `meter`, `surface`, `laws`) are bench/test-only
//! surfaces outside the API stability promise and carry no pins here.
//! The surface-totality gate's impl census (`crates/before/surfacecheck`)
//! excludes compiler-synthesized auto-trait impls from its roster
//! because this module is where those guarantees are pinned.
//!
//! Borrowing types are asserted at `'static`: auto traits of these types
//! do not depend on the lifetime, so the `'static` instance stands in
//! for all of them.

use static_assertions::assert_impl_all;

assert_impl_all!(crate::Party: Send, Sync, Unpin);
assert_impl_all!(crate::Version: Send, Sync, Unpin);
assert_impl_all!(crate::Clock: Send, Sync, Unpin);
assert_impl_all!(crate::OwnVersion<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::Rank: Send, Sync, Unpin);
assert_impl_all!(crate::Ranked<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::Ticks: Send, Sync, Unpin);
assert_impl_all!(crate::Span<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::OwnSpan<'static>: Send, Sync, Unpin);

assert_impl_all!(crate::causally::Bounded: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Dominance: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Endpoint: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Placement: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Range<'static>: Send, Sync, Unpin);

assert_impl_all!(crate::error::Crossed: Send, Sync, Unpin);
assert_impl_all!(crate::error::Decode: Send, Sync, Unpin);
assert_impl_all!(crate::error::Overlap: Send, Sync, Unpin);
assert_impl_all!(crate::error::Parse: Send, Sync, Unpin);

assert_impl_all!(crate::iter::Party<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::iter::Clock<'static>: Send, Sync, Unpin);
