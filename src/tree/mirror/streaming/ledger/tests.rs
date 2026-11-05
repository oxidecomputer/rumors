use proptest::collection::vec;
use proptest::prelude::*;

use super::{Charge, Ledger};
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
    /// Fold the live charge at the second index into the one at the first,
    /// when both are for one place.
    Absorb(usize, usize),
}

fn arb_op() -> impl Strategy<Value = Op> {
    let place = 0..Holding::ALL.len();
    prop_oneof![
        (place.clone(), 0..1_000usize).prop_map(|(place, bytes)| Op::Charge(place, bytes)),
        any::<usize>().prop_map(Op::Drop),
        (any::<usize>(), 0..1_000usize, place)
            .prop_map(|(at, bytes, place)| Op::Split(at, bytes, place)),
        (any::<usize>(), any::<usize>()).prop_map(|(into, from)| Op::Absorb(into, from)),
    ]
}

proptest! {
    /// Through any sequence of charges, splits, folds, and drops, a
    /// ledger holds exactly what its live charges hold, in all and per
    /// place, its peak is the most it has held, and it holds nothing once
    /// every charge is dropped.
    #[test]
    fn a_ledger_holds_exactly_its_live_charges(ops in vec(arb_op(), 0..64)) {
        let ledger = Ledger::new();
        let mut live: Vec<(Holding, Charge)> = Vec::new();
        let mut peak = 0;
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
                Op::Absorb(into, from) if live.len() > 1 => {
                    let (into, from) = (into % live.len(), from % live.len());
                    if into != from && live[into].0 == live[from].0 {
                        let (_, charge) = live.swap_remove(from);
                        let into = if into == live.len() { from } else { into };
                        live[into].1.absorb(charge);
                    }
                }
                _ => {}
            }
            let held: usize = live.iter().map(|(_, charge)| charge.bytes()).sum();
            peak = peak.max(held);
            prop_assert_eq!(ledger.held(), held);
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
        prop_assert_eq!(ledger.held(), 0);
    }
}

/// A charge on no ledger holds nothing, and splitting or folding it moves
/// nothing.
#[test]
fn the_default_charge_holds_nothing() {
    let mut charge = Charge::default();
    assert_eq!(charge.split(Holding::Resolutions, 10).bytes(), 0);
    charge.absorb(Charge::default());
    assert_eq!(charge.bytes(), 0);
}
