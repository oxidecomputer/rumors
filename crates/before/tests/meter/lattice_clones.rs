//! Allocation parity of the join and meet entry points that borrow their left
//! operand.
//!
//! A [`Version`] whose buffer was adopted from an exactly filled `Vec` (as
//! [`Version::decode`] adopts one of eight bytes or more) allocates a shared
//! reference count on its first clone. That allocation is one small, fixed-size
//! header, below anything the amplification board's peak-heap column resolves
//! at its input sizes, so this suite holds it exactly. Every join and meet
//! entry point must clone an operand only when the result *is* that operand.

use before::{Party, Version};

/// One lattice direction.
#[derive(Clone, Copy, Debug)]
enum Op {
    /// The join (`|`).
    Join,
    /// The meet (`&`).
    Meet,
}

impl Op {
    /// Apply the in-place assignment form: the reference every borrowed-left
    /// entry point is held to.
    fn assign(self, lhs: &mut Version, rhs: &Version) {
        match self {
            Op::Join => *lhs |= rhs,
            Op::Meet => *lhs &= rhs,
        }
    }
}

/// Which value a case's result is, and so which clone, if any, the entry
/// points may take.
#[derive(Clone, Copy, Debug)]
enum Yields {
    /// The left operand: the assignment leaves it in place, and a
    /// borrowed-left entry point clones it to return it.
    Left,
    /// The right operand: every entry point clones it once.
    Right,
    /// The empty version, which needs no allocation.
    Empty,
    /// A new version from the lattice kernel.
    Kernel,
}

/// A borrowed-left entry point under test, taking the right operand by value
/// so that each entry point can borrow or move it as its signature requires.
type Entry = fn(Op, &Version, Version) -> Version;

/// Every entry point that reads a borrowed left operand and returns a new
/// version, with the name its failure message reports.
const ENTRIES: [(&str, Entry); 3] = [
    ("named method", |op, l, r| match op {
        Op::Join => l.join(&r),
        Op::Meet => l.meet(&r),
    }),
    ("borrowed operator", |op, l, r| match op {
        Op::Join => l | &r,
        Op::Meet => l & &r,
    }),
    ("owned-right operator", |op, l, r| match op {
        Op::Join => l | r,
        Op::Meet => l & r,
    }),
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

/// Peak heap of the in-place assignment on twins of `lhs` and `rhs`, with
/// its result.
///
/// With `promoted`, both twins are cloned before the measurement and the
/// clones held until after it, so every clone the assignment takes is free;
/// the reading is then the assignment's cost apart from cloning.
fn assignment_heap(op: Op, lhs: &[u8], rhs: &[u8], promoted: bool) -> (usize, Version) {
    let (mut x, r) = (twin(lhs), twin(rhs));
    let held = promoted.then(|| (x.clone(), r.clone()));
    let measured = super::peak_heap(move || {
        op.assign(&mut x, &r);
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

/// The in-place and borrowed-left join and meet entry points clone an
/// operand only when the result is that operand.
///
/// Each case runs on fresh twins, where any clone of a nonempty operand
/// allocates, and on promoted twins, which clone for free; the difference
/// between the two readings is the cost of the clones a path takes. The
/// cases cover every arm of the short-circuit ladder in both directions,
/// plus the kernel, and hold three readings per case:
///
/// - On promoted twins, the in-place assignment (`x |= &r`, `x &= &r`)
///   allocates nothing unless the kernel emits the result.
/// - On fresh twins, the assignment allocates exactly what it does on
///   promoted twins, plus one clone of the right operand when the result is
///   the right operand.
/// - Each borrowed-left entry point allocates exactly what the assignment
///   does on fresh twins, plus one clone of the left operand when the result
///   is the left operand, since it must hand back an owned value sharing
///   that buffer.
///
/// A floor first proves that a fresh twin's clone does allocate; without it,
/// a change in buffer sizing would make every comparison pass trivially.
#[test]
fn lattice_entry_points_clone_only_their_result() {
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
        let (clone_free, _) = assignment_heap(op, lhs, rhs, true);
        if !matches!(yields, Yields::Kernel) && clone_free != 0 {
            violations.push(format!(
                "{op:?} {name}: the assignment allocated {clone_free} bytes on \
                 promoted twins, not 0"
            ));
        }
        let (reference, expected) = assignment_heap(op, lhs, rhs, false);
        let bound = match yields {
            Yields::Right => clone_free + clone_cost(rhs),
            Yields::Left | Yields::Empty | Yields::Kernel => clone_free,
        };
        if reference != bound {
            violations.push(format!(
                "{op:?} {name}: the assignment allocated {reference} bytes, not {bound}"
            ));
        }
        let allowed = match yields {
            Yields::Left => reference + clone_cost(lhs),
            Yields::Right | Yields::Empty | Yields::Kernel => reference,
        };
        for (entry_name, entry) in ENTRIES {
            let (l, r) = (twin(lhs), twin(rhs));
            let (peak, (out, _l)) = super::peak_heap(move || (entry(op, &l, r), l));
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
        "join and meet must clone an operand only when the result is that \
         operand; a larger reading means a clone on a path that discards it:\n{}",
        violations.join("\n"),
    );
}
