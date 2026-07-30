use super::PopStack;

/// The pop-able bit stack round-trips arbitrary pushes through pops in
/// LIFO order — zero included (it stores one real value bit) and the
/// full word width — so every consumer's coordinate comes back exactly
/// as pushed.
#[test]
fn pop_stack_round_trips_lifo() {
    let mut stack = PopStack::new();
    let vals = [0u64, 1, 7, 64, 3, 1, 100_000, 2, u64::MAX];
    for &v in &vals {
        stack.push(v);
    }
    for &v in vals.iter().rev() {
        assert_eq!(stack.pop(), v);
    }
}
