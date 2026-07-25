//! Attempting `&clock | &clock` must fail: no such `BitOr` impl exists.
//!
//! There is deliberately no `Clock | Clock` (that is the fallible `Clock::join`), and
//! no borrowing `&Clock | &Clock` — a borrowing join would duplicate a party.
use before::Clock;

fn main() {
    let a = Clock::seed();
    let b = Clock::seed();
    let _merged = &a | &b;
}
