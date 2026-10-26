//! Runs property tests whose case count is a fraction of the configured one.
//!
//! A property whose cases each cost about `divisor` ordinary cases runs the
//! configured case count divided by `divisor`, rounded up
//! ([`divided_cases`]). The default count then keeps it inside nextest's time
//! limit in a debug build, and a run that sets `PROPTEST_CASES` scales it in
//! proportion.
//!
//! `proptest!` cannot express this. Every form of the macro passes its
//! configuration through proptest's `contextualize_config`, which re-reads the
//! `PROPTEST_*` environment variables, so a set `PROPTEST_CASES` replaces
//! whatever case count the configuration carried. [`run`] starts instead from
//! the default configuration, which has already read the environment, and
//! divides its count afterwards. [`divided_proptest!`] wraps [`run`] in
//! `proptest!`'s syntax.

use proptest::strategy::Strategy;
use proptest::test_runner::{Config, TestCaseError, TestRunner};

/// The case count for a property whose cases each cost about `divisor`
/// ordinary cases: `configured` divided by `divisor`, rounded up.
///
/// # Panics
///
/// Panics if `divisor` is zero.
pub(crate) fn divided_cases(configured: u32, divisor: u32) -> u32 {
    configured.div_ceil(divisor)
}

/// Runs `test` on [`divided_cases`] of the configured case count, drawing
/// each case from `strategy`.
///
/// The run is the one `proptest!` performs, but for the case count:
/// persisted failures replay first, and a new failure shrinks and persists.
/// `test_name` and `source_file` play their `proptest!` roles. Pass the
/// test's `module_path!()`-qualified name and its own `file!()`, which
/// locates the persisted failures, as [`divided_proptest!`] does.
///
/// # Panics
///
/// Panics with proptest's report when a case fails, and if `divisor` is
/// zero.
pub(crate) fn run<S: Strategy>(
    divisor: u32,
    test_name: &'static str,
    source_file: &'static str,
    strategy: S,
    test: impl Fn(S::Value) -> Result<(), TestCaseError>,
) {
    let mut config = Config::default();
    config.cases = divided_cases(config.cases, divisor);
    config.test_name = Some(test_name);
    config.source_file = Some(source_file);
    let mut runner = TestRunner::new(config);
    if let Err(failure) = runner.run(&strategy, test) {
        panic!("{failure}\n{runner}");
    }
}

/// Declares property tests that run [`divided_cases`] of the configured case
/// count, in `proptest!`'s syntax with `#![divisor(...)]` in place of
/// `#![proptest_config(...)]`.
///
/// A test draws its arguments as one tuple strategy, which consumes the
/// random stream exactly as `proptest!`'s argument list does, so seeds
/// persisted by `proptest!` replay the same inputs.
macro_rules! divided_proptest {
    (
        #![divisor($divisor:expr)]
        $(
            $(#[$meta:meta])*
            fn $name:ident($($parm:pat in $strategy:expr),+ $(,)?) $body:block
        )+
    ) => {
        $(
            $(#[$meta])*
            fn $name() {
                $crate::testing::property::run(
                    $divisor,
                    ::core::concat!(::core::module_path!(), "::", ::core::stringify!($name)),
                    ::core::file!(),
                    ($($strategy,)+),
                    |($($parm,)+)| {
                        let (): () = $body;
                        ::core::result::Result::Ok(())
                    },
                );
            }
        )+
    };
}

pub(crate) use divided_proptest;

#[cfg(test)]
mod tests;
