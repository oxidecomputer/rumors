//! A `Clock` owns a `Party`, so cloning a `Clock` would duplicate that party's
//! share. `Clock` must therefore not be `Clone`.

use before::Clock;

fn main() {
    let c = Clock::seed();
    let _dup = c.clone();
}
