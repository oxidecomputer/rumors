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
    static_assertions::assert_eq_size_val!(S(Z), ());
}

#[test]
fn one_align() {
    static_assertions::assert_eq_align!(Z, ());
    static_assertions::assert_eq_align!(S<Z>, ());
    static_assertions::assert_eq_align!(Root, ());
}

/// `Pred` strips exactly one `S` from a height: `S<Z>` ↦ `Z`, `S<S<Z>>` ↦
/// `S<Z>`, and (witnessing the ladder reaches the top) `S<Root::Pred>` is `Root`.
#[test]
fn pred_strips_one_successor() {
    static_assertions::assert_type_eq_all!(<S<Z> as Pred>::Pred, Z);
    static_assertions::assert_type_eq_all!(<S<S<Z>> as Pred>::Pred, S<Z>);
    static_assertions::assert_type_eq_all!(S<<Root as Pred>::Pred>, Root);
}

/// `Pred` is undefined on `Z`: zero has no predecessor.
#[test]
fn pred_undefined_on_zero() {
    static_assertions::assert_not_impl_any!(Z: Pred);
}
