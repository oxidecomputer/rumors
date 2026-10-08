//! Identity operations at depth 100,000 on a small thread stack.
//!
//! Every walk over party trees must be iterative. A recursive walk overflows a
//! 256 KiB stack long before depth 100,000.

use before::{Clock, Count, Party, Version};

/// Depth of the constructed parties.
const D: usize = 100_000;
/// The probe thread's stack.
const STACK: usize = 256 * 1024;

/// Pack a two-bit tag stream into canonical party bytes.
fn pack(tags: impl IntoIterator<Item = (bool, bool)>) -> Vec<u8> {
    let mut bits = Vec::new();
    for (l, r) in tags {
        bits.push(l);
        bits.push(r);
    }
    bits.push(true);
    while bits.len() % 8 != 0 {
        bits.push(false);
    }
    bits.chunks(8)
        .map(|c| c.iter().fold(0u8, |acc, &b| (acc << 1) | u8::from(b)))
        .collect()
}

/// The deep cell `[1 - 2^-d, 1)`: a right spine over an owned terminal.
fn right_cell(d: usize) -> Party {
    let tags = std::iter::repeat_n((false, true), d).chain([(false, false)]);
    Party::decode(&pack(tags)[..]).expect("canonical right cell")
}

/// The deep cell `[0, 2^-d)`.
fn left_cell(d: usize) -> Party {
    let tags = std::iter::repeat_n((true, false), d).chain([(false, false)]);
    Party::decode(&pack(tags)[..]).expect("canonical left cell")
}

/// The complement of `right_cell(d)`: `[0, 1 - 2^-d)`, a comb of owned left
/// halves along the right spine.
fn right_comb(d: usize) -> Party {
    let mut tags = Vec::with_capacity(2 * d);
    for _ in 0..d - 1 {
        tags.push((true, true));
        tags.push((false, false));
    }
    tags.push((true, false));
    tags.push((false, false));
    Party::decode(&pack(tags)[..]).expect("canonical comb")
}

/// Alternating owned siblings down a zigzag: left-heavy at even levels.
fn zigzag(d: usize) -> Party {
    let mut tags = Vec::with_capacity(2 * d);
    for i in 0..d {
        if i % 2 == 0 {
            tags.push((true, false));
        } else {
            tags.push((false, true));
        }
    }
    tags.push((false, false));
    Party::decode(&pack(tags)[..]).expect("canonical zigzag")
}

fn on_small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("the deep probe completed without overflow or panic");
}

/// Every identity operation completes at depth 100,000 on a 256 KiB stack.
#[test]
fn deep_identity_operations_are_iterative() {
    on_small_stack(|| {
        let cell = right_cell(D);
        let comb = right_comb(D);
        let zz = zigzag(D);
        let lcell = left_cell(D);

        // Predicates.
        assert!(Party::seed().covers(&cell));
        assert!(!cell.covers(&Party::seed()));
        assert!(!comb.covers(&cell));
        assert!(comb.covers(&comb.dangerously_alias()));
        assert!(comb.is_disjoint(&cell));
        assert!(!comb.is_disjoint(&zz));
        assert!(zz.covers(&zz.dangerously_alias()));
        assert!(!zz.is_disjoint(&zz.dangerously_alias()));
        assert!(lcell.is_disjoint(&cell));

        // Difference, both the disjoint fast path and the walk.
        let rest = Party::seed().without(&cell).expect("the comb remains");
        assert_eq!(rest, comb);
        assert!(comb.dangerously_alias().without(&comb).is_none());
        assert_eq!(comb.dangerously_alias().without(&cell), Some(comb.dangerously_alias()));
        let z_minus = zz.dangerously_alias().without(&lcell);
        assert!(z_minus.is_some());
        let c_minus = comb.dangerously_alias().without(&zz);
        assert!(c_minus.is_some());

        // Join collapses 100,000 levels to the seed.
        let mut whole = comb.dangerously_alias();
        whole.join(cell.dangerously_alias()).expect("disjoint");
        assert!(whole.is_seed());

        // Sync of the two deep halves.
        let mut a = Clock::from_parts(comb.dangerously_alias(), Version::new());
        let mut b = Clock::from_parts(cell.dangerously_alias(), Version::new());
        a.tick();
        b.tick();
        a.sync(&mut b).expect("disjoint");
        assert!(a.party().is_disjoint(b.party()));

        // Borrowing fork iterators: a full drain, a partial drop, a wide count.
        for p in [comb.dangerously_alias(), cell.dangerously_alias(), zz.dangerously_alias()] {
            let original = p.dangerously_alias();
            let mut keeper = p;
            let shares: Vec<Party> = keeper.forks(5u8).collect();
            keeper.join_all(shares).expect("shares rejoin");
            assert_eq!(keeper, original);
            let taken: Vec<Party> = keeper.forks(1000u16).take(3).collect();
            keeper.join_all(taken).expect("shares rejoin");
            assert_eq!(keeper, original);
            let wide = Count::from(u128::MAX) + Count::from(1u8);
            let one: Vec<Party> = keeper.forks(wide).take(2).collect();
            keeper.join_all(one).expect("shares rejoin");
            assert_eq!(keeper, original);
            // Consuming split and n-ary rejoin.
            let [s0, s1, s2, s3, s4]: [Party; 5] = keeper.into();
            let mut s0 = s0;
            s0.join_all([s1, s2, s3, s4]).expect("shares rejoin");
            assert_eq!(s0, original);
        }

        // Clock forks and sync_all over deep parties.
        let mut c = Clock::from_parts(comb.dangerously_alias(), Version::new());
        c.tick();
        let mut kids: Vec<Clock> = c.forks(4u8).collect();
        let mut d = Clock::from_parts(cell.dangerously_alias(), Version::new());
        d.tick();
        kids.push(d);
        c.sync_all(kids.iter_mut()).expect("disjoint");
        let mut all = c;
        all.join_all(kids).expect("disjoint");
        assert!(all.party().is_seed());

        // Shape walks drain.
        assert_eq!(cell.shape().count(), D + 1);
        assert_eq!(comb.shape().count(), D + 1);
        assert_eq!(zz.shape().count(), D + 1);
        let mut clock = Clock::from_parts(zz.dangerously_alias(), Version::new());
        clock.tick();
        assert!(clock.shape().count() >= D + 1);
        let mut clock = Clock::from_parts(comb.dangerously_alias(), Version::new());
        clock.tick();
        clock.recv(&a.version().clone());
        assert!(clock.shape().count() >= D);

        // Hash, Eq, Debug, codec.
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        comb.hash(&mut h);
        let _ = h.finish();
        assert!(!format!("{comb:?}").is_empty());
        assert_eq!(Party::decode(&comb.encode()[..]).unwrap(), comb);
    });
}
