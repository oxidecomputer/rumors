//! A `Party` is a linear share of the id space: cloning it would duplicate that
//! share, breaking disjointness. `Party` must therefore not be `Clone`.

use itc::Party;

fn main() {
    let p = Party::seed();
    let _dup = p.clone();
}
