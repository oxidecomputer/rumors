use super::*;

/// Every height type, including the full `Root` chain, is zero-sized:
/// heights exist only at the type level and cost nothing in any struct
/// that carries one.
#[test]
fn zero_size() {
    static_assertions::assert_eq_size!(Z, ());
    static_assertions::assert_eq_size!(S<Z>, ());
    static_assertions::assert_eq_size!(Root, ());
}

/// Height *values* (not just the types) are zero-sized, so constructing
/// one is free.
#[test]
fn zero_size_val() {
    static_assertions::assert_eq_size_val!(Z, ());
    static_assertions::assert_eq_size_val!(S::<Z>::default(), ());
}

/// Heights have alignment 1, so a phantom height never pads the struct
/// it tags.
#[test]
fn one_align() {
    static_assertions::assert_eq_align!(Z, ());
    static_assertions::assert_eq_align!(S<Z>, ());
    static_assertions::assert_eq_align!(Root, ());
}
