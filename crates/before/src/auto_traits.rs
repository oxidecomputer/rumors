//! Compile-time pins on the auto traits of every public API type.

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
assert_impl_all!(crate::Dominance: Send, Sync, Unpin);
assert_impl_all!(crate::Endpoint: Send, Sync, Unpin);
assert_impl_all!(crate::Placement: Send, Sync, Unpin);
assert_impl_all!(crate::Precedence: Send, Sync, Unpin);

assert_impl_all!(crate::causally::Ceiling<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Coverage: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Floor<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Query<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Query<'static, crate::causally::Down>: Send, Sync, Unpin);
assert_impl_all!(crate::causally::Query<'static, crate::causally::Up>: Send, Sync, Unpin);

assert_impl_all!(crate::error::Crossed: Send, Sync, Unpin);
assert_impl_all!(crate::error::Decode: Send, Sync, Unpin);
assert_impl_all!(crate::error::Overlap: Send, Sync, Unpin);
assert_impl_all!(crate::error::Parse: Send, Sync, Unpin);

assert_impl_all!(crate::iter::Party<'static>: Send, Sync, Unpin);
assert_impl_all!(crate::iter::Clock<'static>: Send, Sync, Unpin);
