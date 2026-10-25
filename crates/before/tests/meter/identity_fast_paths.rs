//! The heap cost of the byte-equality fast path.
//!
//! The fast paths' fuel costs are claims in the `focused-fuel` harness's
//! identity suite; this check holds the heap the byte-equal `join_all` path
//! allocates, which fuel does not separate from the rest of its work.

use before::testing::meter::registry::Shape;

/// Peak heap allowed for `join_all`'s fixed-size fold bookkeeping.
const EQUAL_JOIN_PEAK_HEAP: usize = 440;

/// Peak heap `join_all` must read at least, so a meter that stops counting
/// fails instead of passing under the ceiling.
///
/// `join_all` reduces through a balanced fold that pushes its first operand,
/// the receiver, onto a freshly allocated stack whose entries each carry a
/// `usize` weight. That allocation happens for any operands, and the borrowed
/// operands free nothing before it, so the peak holds at least one weight.
const EQUAL_JOIN_PEAK_FLOOR: usize = size_of::<usize>();

/// Joining byte-equal versions returns shared storage without copying either
/// operand.
///
/// The operands are decoded independently, so pointer identity cannot take
/// the shortcut. Growing both streams fourfold must leave the fold's peak heap
/// unchanged: only fixed-size bookkeeping may be allocated before the
/// byte-equality check returns a clone. That bookkeeping always includes the
/// fold's stack, so the peak must also reach [`EQUAL_JOIN_PEAK_FLOOR`].
#[test]
fn equal_join_all_has_fixed_heap_cost() {
    let run = |depth: usize| {
        let encoded = Shape::Dense.build1(depth);
        let a = super::version_of(&encoded);
        let b = super::version_of(&encoded);
        let (peak, out) = super::peak_heap(|| a.join_all([&b]));
        assert_eq!(out, a, "joining equal versions preserves their value");
        assert!(
            peak >= EQUAL_JOIN_PEAK_FLOOR,
            "join_all over equal versions read {peak} transient bytes, below the \
             {EQUAL_JOIN_PEAK_FLOOR}-byte fold stack it always allocates: the heap \
             meter is not counting this thread's allocations"
        );
        assert!(
            peak <= EQUAL_JOIN_PEAK_HEAP,
            "join_all over equal versions used {peak} transient bytes; the fixed-size \
             bookkeeping ceiling is {EQUAL_JOIN_PEAK_HEAP} bytes"
        );
        peak
    };

    let small = run(1_000);
    let large = run(4_000);
    assert_eq!(
        small, large,
        "growing equal operands must not increase join_all's transient heap"
    );
}
