use proptest::collection::vec;
use proptest::prelude::*;

use super::{Charge, Ledger};
use crate::counter;
use crate::tree::mirror::streaming::holding::Holding;

/// One operation on a ledger and the charges it has issued.
#[derive(Clone, Debug)]
enum Op {
    /// Charge this many bytes for the place at this index of
    /// [`Holding::ALL`].
    Charge(usize, usize),
    /// Drop the live charge at this index, modulo the live count.
    Drop(usize),
    /// Split this many bytes of the live charge at this index into a new
    /// charge for the place at that index.
    Split(usize, usize, usize),
    /// Build an item of this many bytes for the place at that index out of
    /// the live charge at this index, charged in full however much the
    /// charge holds.
    Take(usize, usize, usize),
}

fn arb_op() -> impl Strategy<Value = Op> {
    let place = 0..Holding::ALL.len();
    prop_oneof![
        (place.clone(), 0..1_000usize).prop_map(|(place, bytes)| Op::Charge(place, bytes)),
        any::<usize>().prop_map(Op::Drop),
        (any::<usize>(), 0..1_000usize, place.clone())
            .prop_map(|(at, bytes, place)| Op::Split(at, bytes, place)),
        (any::<usize>(), 0..1_000usize, place)
            .prop_map(|(at, bytes, place)| Op::Take(at, bytes, place)),
    ]
}

proptest! {
    /// Through any sequence of charges, splits, items built out of them,
    /// and drops, a ledger holds exactly what its live charges hold,
    /// in all and per place.
    ///
    /// Its peak is the most it has held, and it holds nothing once every
    /// charge is dropped.
    ///
    /// The charges are made outside admission and outside floor slots, so
    /// the budget counts every byte they hold, whatever an item needed
    /// beyond the charge it was built out of; the bytes items needed beyond
    /// their charges add up to the ledger's shortfall.
    #[test]
    fn a_ledger_holds_exactly_its_live_charges(ops in vec(arb_op(), 0..64)) {
        let ledger = Ledger::default();
        let mut live: Vec<(Holding, Charge)> = Vec::new();
        let mut peak = 0;
        let mut short = 0;
        for op in ops {
            match op {
                Op::Charge(place, bytes) => {
                    let place = Holding::ALL[place];
                    live.push((place, ledger.charge(place, bytes)));
                }
                Op::Drop(at) if !live.is_empty() => {
                    let at = at % live.len();
                    live.swap_remove(at);
                }
                Op::Split(at, bytes, place) if !live.is_empty() => {
                    let at = at % live.len();
                    let place = Holding::ALL[place];
                    let part = live[at].1.split(place, bytes);
                    live.push((place, part));
                }
                Op::Take(at, bytes, place) if !live.is_empty() => {
                    let at = at % live.len();
                    let place = Holding::ALL[place];
                    short += bytes.saturating_sub(live[at].1.bytes());
                    let item = live[at].1.take(place, bytes);
                    prop_assert_eq!(item.bytes(), bytes);
                    live.push((place, item));
                }
                _ => {}
            }
            let held: usize = live.iter().map(|(_, charge)| charge.bytes()).sum();
            peak = peak.max(held);
            prop_assert_eq!(ledger.held(), held);
            prop_assert_eq!(ledger.normal(), held);
            prop_assert_eq!(ledger.short(), counter::widen(short));
            for place in Holding::ALL {
                let expected: usize = live
                    .iter()
                    .filter(|(other, _)| *other == place)
                    .map(|(_, charge)| charge.bytes())
                    .sum();
                prop_assert_eq!(ledger.held_for(place), expected);
            }
        }
        prop_assert!(ledger.peak() >= peak);
        drop(live);
        prop_assert_eq!((ledger.held(), ledger.normal()), (0, 0));
    }
}

/// A charge on no ledger holds nothing, and splitting it moves nothing.
#[test]
fn the_default_charge_holds_nothing() {
    let mut charge = Charge::default();
    assert_eq!(charge.split(Holding::Resolutions, 10).bytes(), 0);
    assert_eq!(charge.bytes(), 0);
}
