//! The valid side of the balanced-array split: converting a `Party` or `Clock`
//! into a *non-empty* array compiles and runs.
//!
//! This pass case also has a structural job. trybuild runs `cargo check` for a
//! suite of only `compile_fail` cases, but `cargo check` does not monomorphize,
//! so the post-monomorphization `const { assert!(N >= 1) }` guard on the zero
//! length conversions never fires. The presence of one `pass` case flips
//! trybuild to `cargo build`, under which that guard surfaces — which is what
//! makes the sibling `*_into_empty_array.rs` cases fail to compile as intended.

use before::{Clock, Party};

fn main() {
    let [_a, _b, _c]: [Party; 3] = Party::seed().into();
    let [_x, _y]: [Clock; 2] = Clock::seed().into();
}
