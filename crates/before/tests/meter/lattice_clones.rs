//! Allocation parity of the join and meet entry points.
//!
//! A [`Version`] whose buffer was adopted from an exactly filled `Vec` (as
//! [`Version::decode`] adopts one of eight bytes or more) allocates a shared
//! reference count on its first clone. That allocation is one small, fixed-size
//! header, below anything the amplification board's peak-heap column resolves
//! at its input sizes, so this suite holds it exactly. Every join and meet
//! entry point must clone a borrowed operand only when the result *is* that
//! operand, and must move an owned operand rather than clone it.

use before::{Party, Version};

/// One lattice direction.
#[derive(Clone, Copy, Debug)]
enum Op {
    /// The join (`|`).
    Join,
    /// The meet (`&`).
    Meet,
}

/// How an entry point receives one of its operands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Form {
    /// By reference: returning the operand as the result takes a clone.
    Borrowed,
    /// By value: returning the operand as the result moves it.
    Owned,
}

/// Which value a case's result is, and so which operand, if any, an entry
/// point may clone.
#[derive(Clone, Copy, Debug)]
enum Yields {
    /// The left operand.
    Left,
    /// The right operand.
    Right,
    /// The empty version, which needs no allocation.
    Empty,
    /// A new version from the lattice kernel.
    Kernel,
}

/// A join and meet entry point under test, taking both operands by value so
/// that it can borrow or move each as its signature requires.
type Entry = fn(Op, Version, Version) -> Version;

/// Every join and meet entry point: the name its failure message reports, how
/// it receives its left and right operands, and the call.
const ENTRIES: [(&str, Form, Form, Entry); 7] = [
    (
        "named method",
        Form::Borrowed,
        Form::Borrowed,
        |op, l, r| match op {
            Op::Join => l.join(&r),
            Op::Meet => l.meet(&r),
        },
    ),
    (
        "borrowed operator",
        Form::Borrowed,
        Form::Borrowed,
        |op, l, r| match op {
            Op::Join => &l | &r,
            Op::Meet => &l & &r,
        },
    ),
    (
        "owned-right operator",
        Form::Borrowed,
        Form::Owned,
        |op, l, r| match op {
            Op::Join => &l | r,
            Op::Meet => &l & r,
        },
    ),
    (
        "owned-left operator",
        Form::Owned,
        Form::Borrowed,
        |op, l, r| match op {
            Op::Join => l | &r,
            Op::Meet => l & &r,
        },
    ),
    (
        "owned operator",
        Form::Owned,
        Form::Owned,
        |op, l, r| match op {
            Op::Join => l | r,
            Op::Meet => l & r,
        },
    ),
    (
        "borrowed assignment",
        Form::Owned,
        Form::Borrowed,
        |op, mut l, r| {
            match op {
                Op::Join => l |= &r,
                Op::Meet => l &= &r,
            }
            l
        },
    ),
    (
        "owned assignment",
        Form::Owned,
        Form::Owned,
        |op, mut l, r| {
            match op {
                Op::Join => l |= r,
                Op::Meet => l &= r,
            }
            l
        },
    ),
];

/// Decode a fresh twin of `bytes`: a buffer-distinct version no one has
/// cloned yet.
fn twin(bytes: &[u8]) -> Version {
    Version::decode(bytes).expect("fixture bytes are canonical")
}

/// Peak heap of the first clone of a fresh twin of `bytes`.
fn clone_cost(bytes: &[u8]) -> usize {
    let fresh = twin(bytes);
    super::peak_heap(move || fresh.clone()).0
}

/// Peak heap of the in-place assignment `x |= &r` or `x &= &r` on twins of
/// `lhs` and `rhs` that were cloned before the measurement, with its result.
///
/// The clones are held until after the measurement, so every clone the
/// assignment takes is free; the reading is the assignment's cost apart from
/// cloning.
fn clone_free_heap(op: Op, lhs: &[u8], rhs: &[u8]) -> (usize, Version) {
    let (mut x, r) = (twin(lhs), twin(rhs));
    let held = (x.clone(), r.clone());
    let measured = super::peak_heap(move || {
        match op {
            Op::Join => x |= &r,
            Op::Meet => x &= &r,
        }
        x
    });
    drop(held);
    measured
}

/// Encode a pair of concurrent multi-party versions, each well over eight
/// bytes.
fn concurrent_pair() -> (Vec<u8>, Vec<u8>) {
    let mut root = Party::seed();
    let mut parties: Vec<Party> = (0..64).map(|_| root.fork()).collect();
    parties.push(root);
    let (mut v, mut w) = (Version::new(), Version::new());
    for (i, party) in parties.iter().enumerate() {
        for _ in 0..(i % 3 + 1) {
            party.tick(&mut v);
        }
        for _ in 0..((i + 1) % 4 + 1) {
            party.tick(&mut w);
        }
    }
    assert!(
        v.concurrent(&w),
        "the kernel cases need concurrent operands"
    );
    (v.encode(), w.encode())
}

/// Every join and meet entry point clones a borrowed operand only when the
/// result is that operand, and moves an owned one.
///
/// The cases cover every arm of the short-circuit ladder in both directions,
/// plus the kernel. Each case first measures the in-place assignment on
/// promoted twins, which clone for free: that allocates nothing unless the
/// kernel emits the result. Each entry point then runs on fresh twins, where
/// any clone of a nonempty operand allocates. It must allocate exactly the
/// clone-free reading, plus one clone of the operand the result is when the
/// entry point borrows that operand. An entry point that owns the operand the
/// result is moves it, and allocates no clone.
///
/// A floor first proves that a fresh twin's clone does allocate; without it,
/// a change in buffer sizing would make every comparison pass trivially.
#[test]
fn lattice_entry_points_clone_only_borrowed_results() {
    let (v, w) = concurrent_pair();
    let empty = Version::new().encode();
    assert!(
        clone_cost(&v) > 0,
        "the first clone of a fresh {}-byte twin must allocate its shared \
         reference count; at zero, every comparison below passes trivially",
        v.len(),
    );

    let (v, w, empty) = (&v[..], &w[..], &empty[..]);
    // (case, direction, left operand, right operand, the result)
    #[allow(clippy::type_complexity)]
    let cases: [(&str, Op, &[u8], &[u8], Yields); 8] = [
        ("concurrent", Op::Join, v, w, Yields::Kernel),
        ("concurrent", Op::Meet, v, w, Yields::Kernel),
        ("empty right", Op::Join, v, empty, Yields::Left),
        ("empty right", Op::Meet, v, empty, Yields::Empty),
        ("empty left", Op::Join, empty, v, Yields::Right),
        ("empty left", Op::Meet, empty, v, Yields::Left),
        ("equal twins", Op::Join, v, v, Yields::Left),
        ("equal twins", Op::Meet, v, v, Yields::Left),
    ];
    let mut violations = Vec::new();
    for (name, op, lhs, rhs, yields) in cases {
        let (clone_free, expected) = clone_free_heap(op, lhs, rhs);
        if !matches!(yields, Yields::Kernel) && clone_free != 0 {
            violations.push(format!(
                "{op:?} {name}: the assignment allocated {clone_free} bytes on \
                 promoted twins, not 0"
            ));
        }
        for (entry_name, left, right, entry) in ENTRIES {
            let clone = match yields {
                Yields::Left if left == Form::Borrowed => clone_cost(lhs),
                Yields::Right if right == Form::Borrowed => clone_cost(rhs),
                Yields::Left | Yields::Right | Yields::Empty | Yields::Kernel => 0,
            };
            let allowed = clone_free + clone;
            let (l, r) = (twin(lhs), twin(rhs));
            let (peak, out) = super::peak_heap(move || entry(op, l, r));
            assert_eq!(
                out, expected,
                "{op:?} {name}: the {entry_name} must agree with the assignment"
            );
            if peak != allowed {
                violations.push(format!(
                    "{op:?} {name}: the {entry_name} allocated {peak} bytes, not {allowed}"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "join and meet must clone a borrowed operand only when the result is \
         that operand, and must move an owned one; a larger reading means a \
         clone on a path that discards it or could move it:\n{}",
        violations.join("\n"),
    );
}
