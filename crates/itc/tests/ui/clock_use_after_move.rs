//! The `|` (`BitOr`) operator consumes its `Clock` operand by value: a borrowing
//! form would duplicate the party. Reusing a `Clock` after `|` must therefore be
//! rejected as a use-after-move.

use itc::{Clock, Version};

fn main() {
    let clock = Clock::seed();
    let ev = Version::new();
    let _merged = clock | ev;
    // `clock` was moved into the `|` above; touching it again is illegal.
    let _again = clock.version();
}
