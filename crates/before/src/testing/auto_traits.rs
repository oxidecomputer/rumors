//! Compile-time pins on the auto traits of every public API type.

use static_assertions::assert_impl_all;
use std::hash::Hash;

assert_impl_all!(crate::Party: Send, Sync, Unpin);
assert_impl_all!(crate::Version: Send, Sync, Unpin);
assert_impl_all!(crate::Clock: Send, Sync, Unpin);
assert_impl_all!(crate::OwnVersion<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::Rank: Send, Sync, Unpin);
assert_impl_all!(crate::Ranked<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::Count: Send, Sync, Unpin);
assert_impl_all!(crate::Span<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::OwnSpan<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::Dominance: Send, Sync, Unpin);
assert_impl_all!(crate::Endpoint: Send, Sync, Unpin);
assert_impl_all!(crate::Placement: Send, Sync, Unpin);
assert_impl_all!(crate::Precedence: Send, Sync, Unpin);

assert_impl_all!(crate::Span<'static>: Hash);
assert_impl_all!(crate::shape::Plateau: Hash);
assert_impl_all!(crate::shape::Rise: Hash);
assert_impl_all!(crate::shape::Region: Hash);
assert_impl_all!(crate::shape::Cell<2>: Hash);

assert_impl_all!(crate::causally::Ceiling<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Coverage: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Floor<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Query<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Query<'static, crate::causally::Down>: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Query<'static, crate::causally::Up>: Send, Sync, Unpin);

assert_impl_all!(crate::error::Crossed: Send, Sync, Unpin);
assert_impl_all!(crate::error::Decode: Send, Sync, Unpin);
assert_impl_all!(crate::error::Overlap: Send, Sync, Unpin);
assert_impl_all!(crate::error::ParseRank: Send, Sync, Unpin);

assert_impl_all!(crate::iter::PartyForks<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::iter::ClockForks<'static>: Send, Sync, Unpin);
