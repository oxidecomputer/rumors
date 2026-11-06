//! The floor/budget substitution in admission, and the carry that undoes
//! it, at pairs of budgets either side of a depth-1 answer's grant.
//!
//! # The mechanism
//!
//! In a disjoint pair, the responder's depth-1 questions take grants larger
//! than these budgets, so each goes through its floor slot, and the slot
//! frees only once the scope it asked about is assembled: the 256 depth-1
//! subtrees run one after another. The initiator admits its depth-1 answer
//! for a subtree as the subtree's question arrives, which at some subtree
//! boundaries is the instant the previous subtree's answer finishes: the
//! answer's slot is still taken, and frees within that instant.
//!
//! At a ledger budget below the answer's grant, the answer waits for the
//! slot and takes it. At a budget the grant fits, the answer is admitted
//! against the budget first. Were its bytes to stay there, they would hold
//! the budget through the whole subtree, away from the subtree's deeper
//! questions, for no parallelism the serialized subtrees could use, and the
//! subtree would run about twice as long. A slot that frees carries those
//! bytes instead (admission's module docs, "Carrying"), so the answer holds
//! the budget for no link time at all.
//!
//! # The claims
//!
//! Each cell is a disjoint pair on a LAN at two ledger budgets, `B` and
//! `B′`, between which some depth-1 answer's grant falls: it waits for its
//! slot at `B` and fits the budget at `B′`. At both budgets:
//!
//! 1. The responder's depth-1 questions all take their floor slot, the
//!    premise that serializes the subtrees.
//! 2. Every depth-1 answer the initiator admits against the budget is
//!    carried by its slot at the same link time, so none holds the budget
//!    for any time; and at `B′` some answer is, so the cell exercises the
//!    substitution.
//!
//! And the session at `B′` takes at most one round trip longer than at `B`,
//! the bound the budget criterion holds every pair of budgets to. Without
//! the carry, the substituted answers double their subtrees' time, which
//! costs thousands of one-way delays, so the bound detects a substitution
//! the carry misses. With it, a larger budget can still repack the units
//! after the first difference by less than a round trip, as the budget
//! criterion's module docs describe.

use rumors_testkit::bench::link::LinkConfig;
use rumors_testkit::bench::session::{
    self as fixture, Case, LAN_BDP, LedgerReading, Session, Shape,
};
use rumors_testkit::common::budget::BudgetChoice;

/// One cell: a disjoint pair of `messages` per side, at two ledger budgets,
/// the smaller first.
#[derive(Clone, Copy, Debug)]
struct Cell {
    /// Messages on each side, none shared.
    messages: u64,
    /// The ledger budgets `B` and `B′`.
    budgets: [usize; 2],
}

/// The cells, each a pair of budgets that some of the initiator's depth-1
/// answers' grants fall between:
///
/// - 3,000 messages per side at 2,560 B and 3,072 B. The depth-1 answers'
///   grants here rise from 2,112 B in steps of 152 B, one step per
///   reaction, so these budgets bracket the answers of several consecutive
///   reaction counts.
/// - 200,000 messages per side, one byte either side of the saturated
///   depth-1 answer's grant, 39,208 B: the grant of an answer with a full
///   fan of reactions, which some answers at this size carry.
const CELLS: [Cell; 2] = [
    Cell {
        messages: 3_000,
        budgets: [2_560, 3_072],
    },
    Cell {
        messages: 200_000,
        budgets: [39_207, 39_208],
    },
];

/// The case of `cell` at its budget `which`, over a LAN.
///
/// The cells' ledger budgets lie below the largest grant, which the public
/// setter's minimum keeps every session above, so they are set through the
/// test path.
fn case(cell: Cell, which: usize) -> Case {
    Case {
        shape: Shape::symmetric(cell.messages),
        budget: BudgetChoice::Ledger(cell.budgets[which]),
        link: LinkConfig::with_bdp(LAN_BDP),
    }
}

/// The initiator's and the responder's ledgers in `session`, told apart by
/// the root question only the initiator asks.
fn ledgers(session: &Session) -> [&LedgerReading; 2] {
    let ledgers = &session.admitted.ledgers;
    let initiator = ledgers
        .iter()
        .position(|ledger| ledger.questions[0].units > 0)
        .expect("one side asks the root question");
    assert_eq!(ledgers.len(), 2, "a session reports two ledgers");
    [&ledgers[initiator], &ledgers[1 - initiator]]
}

/// Checks the claims at one budget of `cell`, returning the grants of the
/// initiator's depth-1 answers and how many it admitted against the budget.
fn check(cell: Cell, session: &Session) -> (Vec<usize>, usize) {
    let [initiator, responder] = ledgers(session);
    let asked = responder.questions[1];
    assert_eq!(
        asked.floor, asked.units,
        "the responder's depth-1 questions all take their slot, which serializes the \
         subtrees: {cell:?} at {:?}",
        session.case.budget,
    );
    let answers = || {
        initiator
            .dated
            .iter()
            .filter(|admission| !admission.question && admission.depth == 1)
    };
    let carried: std::collections::BTreeSet<u64> = initiator
        .carries
        .iter()
        .filter(|carry| !carry.unit.question && carry.unit.depth == 1)
        .map(|carry| carry.at)
        .collect();
    let budgeted: Vec<u64> = answers()
        .filter(|admission| !admission.floor)
        .map(|admission| admission.at)
        .collect();
    for at in &budgeted {
        assert!(
            carried.contains(at),
            "a depth-1 answer admitted against the budget at {at} was not carried then: \
             {cell:?} at {:?}",
            session.case.budget,
        );
    }
    (
        answers().map(|admission| admission.bytes).collect(),
        budgeted.len(),
    )
}

/// At each cell's two budgets, the substitution happens and is carried at
/// once, and the larger budget's session is at most one round trip slower.
///
/// See the module docs for the mechanism and the claims. Each cell's
/// budgets straddle some grant of the initiator's depth-1 answers, which is
/// what makes the step matter.
#[test]
fn the_substitution_is_carried_and_the_inversion_is_gone() {
    let sessions = fixture::run_all(std::array::from_fn::<_, 4, _>(|index| {
        case(CELLS[index / 2], index % 2)
    }));
    for (cell, pair) in CELLS.iter().zip(sessions.chunks(2)) {
        let [smaller, larger] = [&pair[0], &pair[1]];
        let (grants, _) = check(*cell, smaller);
        let (_, substituted) = check(*cell, larger);
        let [b, b_prime] = cell.budgets;
        assert!(
            grants.iter().any(|&grant| b < grant && grant <= b_prime),
            "no depth-1 answer's grant falls between the budgets: {cell:?}",
        );
        assert!(
            substituted > 0,
            "no depth-1 answer entered the budget at the larger budget, so the cell does \
             not exercise the substitution: {cell:?}",
        );
        let round_trip = 2 * smaller.case.link.one_way_delay();
        assert!(
            larger.duration() <= smaller.duration() + round_trip,
            "the session at {b_prime} B lasts {} byte-times, more than one round trip \
             ({round_trip}) beyond {} at {b} B: {cell:?}",
            larger.duration(),
            smaller.duration(),
        );
    }
}
