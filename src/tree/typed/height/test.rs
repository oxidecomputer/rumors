use super::*;

#[test]
fn zero_size() {
    static_assertions::assert_eq_size!(Z, ());
    static_assertions::assert_eq_size!(S<Z>, ());
    static_assertions::assert_eq_size!(Root, ());
}

#[test]
fn zero_size_val() {
    static_assertions::assert_eq_size_val!(Z, ());
    static_assertions::assert_eq_size_val!(S::<Z>::default(), ());
}

#[test]
fn one_align() {
    static_assertions::assert_eq_align!(Z, ());
    static_assertions::assert_eq_align!(S<Z>, ());
    static_assertions::assert_eq_align!(Root, ());
}
