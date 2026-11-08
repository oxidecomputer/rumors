//! The session reader and its fixtures, each against an independent
//! answer.
//!
//! Fixture messages' encoded sizes are checked against an independent CBOR
//! encoder; a session's records over time, against its ledgers' totals;
//! and the reader's record of units waiting or admitted beside a waiting
//! unit of their kind and depth, against a model of each ledger's waiting
//! units.

use std::collections::BTreeMap;

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::testing::Admission;

use super::{
    Case, LAN_BDP, LedgerReading, READINGS, Shape, admitted, message, pair, record_admission, run,
    start_reading, text_chars,
};
use crate::bench::link::{Link, LinkConfig, Side};
use crate::common::budget::BudgetChoice;

/// The bytes `text` takes encoded as a CBOR text string.
fn encoded(text: &str) -> usize {
    let mut out = Vec::new();
    ciborium::into_writer(text, &mut out).expect("a string encodes");
    out.len()
}

proptest! {
    /// A message size names the characters of the one text string whose
    /// CBOR encoding takes exactly that many bytes, or none where no
    /// string's does, and a fixture message built at that size encodes in
    /// exactly that many bytes.
    ///
    /// A CBOR head takes one, two, three, five, or nine bytes, so the
    /// strings that could encode in `bytes` bytes are those of `bytes`
    /// less each head's length; the encoder judges each candidate.
    #[test]
    fn messages_encode_in_their_shape_s_size(bytes in 1..70_000_usize, index in 0..1_000_000_u64) {
        let fitting: Vec<usize> = [1, 2, 3, 5, 9]
            .into_iter()
            .filter_map(|head| bytes.checked_sub(head))
            .filter(|&chars| encoded(&"x".repeat(chars)) == bytes)
            .collect();
        prop_assert!(fitting.len() <= 1, "one string length per size: {fitting:?}");
        prop_assert_eq!(text_chars(bytes), fitting.first().copied());
        if let Some(chars) = fitting.first()
            && *chars > 7
        {
            prop_assert_eq!(encoded(&message('a', index, bytes)), bytes);
        }
    }
}

/// The excess `ledger`'s profile records, integrated from
/// its first step to its last.
fn profile_area(ledger: &LedgerReading) -> u128 {
    ledger
        .profile
        .windows(2)
        .map(|pair| pair[0].bytes as u128 * u128::from(pair[1].at - pair[0].at))
        .sum()
}

/// A session's records over time agree with the totals its ledgers keep
/// by other means, with and without a budget that holds units back.
///
/// Each side's profile of its excess integrates to its budget area and ends
/// at zero, as the session releases every charge. Its holders, built from
/// the moves the ledger reports, partition what it holds: their areas add
/// up to the places' areas, which the ledger's own per-place totals give.
/// Its waits number the units the ledger counted as waiting, each admitted
/// no earlier than it was queued and no wait longer than the time some unit
/// waited. Where a side made a share check, as some side does, the check
/// it dates as its share peak weighed exactly the peak the ledger kept, its
/// terms add up to that sum as the share's rule computes it, and it falls
/// within the session; where it made none, since every question it asked
/// was exempt, its share peak reads zero. What the holders held at
/// the budget peak adds up to what the ledger held then, which is at least
/// that peak excess, at a moment within the session. The two ledgers are
/// read as the two sides'.
///
/// The pair holds 10,000 messages a side, enough that some of its questions
/// overflow their kinds and depths' bounds and meet the share.
#[test]
fn records_over_time_agree_with_the_ledger_totals() {
    let shape = Shape::overlapping(10_000, 0);
    let link = LinkConfig::with_bdp(LAN_BDP);
    let unlimited = run(Case {
        shape,
        budget: BudgetChoice::Bytes(usize::MAX),
        link,
    });
    let budget = BudgetChoice::Ledger(unlimited.admitted.budget_peak() / 4);
    let budgeted = run(Case {
        shape,
        budget,
        link,
    });
    assert!(
        budgeted.admitted.waited() > 0,
        "a quarter of the peak holds units back"
    );
    for session in [&unlimited, &budgeted] {
        let mut sides: Vec<Option<Side>> = session
            .admitted
            .ledgers
            .iter()
            .map(|ledger| ledger.side)
            .collect();
        sides.sort_by_key(|side| side.map(Side::index));
        assert_eq!(sides, [Some(Side::A), Some(Side::B)]);
        assert!(
            session
                .admitted
                .ledgers
                .iter()
                .any(|ledger| ledger.share_peak_check.is_some()),
            "some side asks a question the share checks"
        );
        for ledger in &session.admitted.ledgers {
            match ledger.share_peak_check {
                Some(dated) => {
                    let check = dated.check;
                    assert_eq!(check.sum, ledger.share_peak);
                    let scaled = u128::from(check.pace_actual)
                        * (check.claimed + check.need) as u128
                        / u128::from(check.pace_predicted);
                    assert_eq!(
                        (check.invited + check.grant) as u128 + scaled,
                        check.sum as u128,
                    );
                    assert!(dated.at <= session.duration());
                }
                None => assert_eq!(ledger.share_peak, 0),
            }
            let at_peak: usize = ledger
                .budget_peak_holders
                .iter()
                .map(|(_, bytes)| bytes)
                .sum();
            assert_eq!(at_peak, ledger.budget_peak_held);
            assert!(ledger.budget_peak <= ledger.budget_peak_held);
            assert!(ledger.budget_peak_at <= session.duration());
            assert_eq!(profile_area(ledger), ledger.budget_area);
            assert_eq!(ledger.profile.last().map(|step| step.bytes), Some(0));
            let holders: u128 = ledger.holders.values().map(|held| held.area).sum();
            assert_eq!(holders, ledger.place_area.iter().sum::<u128>());
            assert_eq!(u64::try_from(ledger.waits.len()), Ok(ledger.waited));
            assert!(ledger.waiting_time <= session.duration());
            for wait in &ledger.waits {
                assert!(wait.queued <= wait.admitted);
                assert!(wait.admitted - wait.queued <= ledger.waiting_time);
            }
        }
    }
}

/// A shape's redactions take effect on the side that makes them and on
/// that side alone.
///
/// Before the session, each replica holds the shared messages, its own
/// insertions, and the messages its peer redacted, and none it redacted
/// itself. A session over the pair still gains each side exactly the
/// messages its peer inserted, since a redaction crosses as a version,
/// never as a message.
#[test]
fn redactions_hold_on_the_redacting_side_alone() {
    let shape = Shape::overlapping(2_000, 50).with_redactions(100, 300);
    let (a, b) = pair(shape, usize::MAX);
    assert_eq!(
        a.snapshot().len() as u64,
        shape.shared + shape.a + shape.redacted_b
    );
    assert_eq!(
        b.snapshot().len() as u64,
        shape.shared + shape.b + shape.redacted_a
    );
    drop((a, b));
    run(Case {
        shape,
        budget: BudgetChoice::Bytes(usize::MAX),
        link: LinkConfig::with_bdp(LAN_BDP),
    });
}

/// Each draw of a shape builds another tree for the same counts: every
/// replica holds as many messages at every draw, and its tree's hash
/// differs from draw to draw, since the draw moves every leaf's path.
#[test]
fn draws_change_the_tree_and_not_the_counts() {
    let shape = Shape::overlapping(500, 50).with_redactions(20, 30);
    let replicas: Vec<_> = (0..3)
        .map(|draw| pair(shape.with_draw(draw), usize::MAX))
        .collect();
    for (a, b) in &replicas {
        assert_eq!(
            a.snapshot().len() as u64,
            shape.shared + shape.a + shape.redacted_b
        );
        assert_eq!(
            b.snapshot().len() as u64,
            shape.shared + shape.b + shape.redacted_a
        );
    }
    let hashes: std::collections::BTreeSet<_> =
        replicas.iter().map(|(a, _)| a.snapshot().hash()).collect();
    assert_eq!(hashes.len(), replicas.len());
}

/// One report to the admission hook in a generated schedule.
///
/// The report is of a unit of `ledger`'s kind `question` at `depth`
/// starting to wait or admitted at once, or of the `pick`th of `ledger`'s
/// waiting units, counted modulo how many wait, taking its grant.
#[derive(Clone, Copy, Debug)]
enum Report {
    /// A unit starts to wait.
    Wait {
        /// The ledger it waits in.
        ledger: usize,
        /// Whether it is a question.
        question: bool,
        /// The depth of the scope it concerns.
        depth: usize,
    },
    /// A unit that never waited takes its grant.
    Grant {
        /// The ledger that admits it.
        ledger: usize,
        /// Whether it is a question.
        question: bool,
        /// The depth of the scope it concerns.
        depth: usize,
    },
    /// A waiting unit takes its grant.
    Admit {
        /// The ledger it waited in.
        ledger: usize,
        /// Which of the ledger's waiting units, modulo their number.
        pick: usize,
    },
}

/// Reports over two ledgers and few kinds and depths, so that units of one
/// kind and depth meet often, in one ledger and across the two.
fn report() -> impl Strategy<Value = Report> {
    prop_oneof![
        (1..=2_usize, any::<bool>(), 0..3_usize).prop_map(|(ledger, question, depth)| {
            Report::Wait {
                ledger,
                question,
                depth,
            }
        }),
        (1..=2_usize, any::<bool>(), 0..3_usize).prop_map(|(ledger, question, depth)| {
            Report::Grant {
                ledger,
                question,
                depth,
            }
        }),
        (1..=2_usize, any::<usize>()).prop_map(|(ledger, pick)| Report::Admit { ledger, pick }),
    ]
}

/// The hook's report of the unit `order` of `ledger`, of kind `question` at
/// `depth`, starting to wait if `waiting`, or else taking its grant after
/// waiting.
fn waiting_report(
    ledger: usize,
    order: u64,
    question: bool,
    depth: usize,
    waiting: bool,
) -> Admission {
    Admission {
        ledger,
        order: Some(order),
        waiting,
        question,
        depth,
        bytes: 0,
        exempt: false,
        waited: !waiting,
        leftover: false,
        partial: false,
    }
}

/// The hook's report of a unit of `ledger`, of kind `question` at `depth`,
/// taking its grant as it asked, without waiting.
fn granted_report(ledger: usize, question: bool, depth: usize) -> Admission {
    Admission {
        ledger,
        order: None,
        waiting: false,
        question,
        depth,
        bytes: 0,
        exempt: true,
        waited: false,
        leftover: false,
        partial: false,
    }
}

proptest! {
    /// The reader records a crowded kind and depth exactly when a unit
    /// starts to wait, or takes its grant, while its own ledger has another
    /// unit of the same kind and depth waiting.
    ///
    /// A unit admitted before the next starts waiting or is granted crowds
    /// nothing, and two ledgers' units never crowd each other.
    #[test]
    fn a_unit_waiting_beside_its_kind_and_depth_is_recorded(
        reports in vec(report(), 0..24),
    ) {
        start_reading(&Link::new(LinkConfig::with_bdp(LAN_BDP)));
        let mut waiting: BTreeMap<usize, Vec<(u64, bool, usize)>> = BTreeMap::new();
        let mut next = 0;
        let mut crowded = false;
        for report in reports {
            match report {
                Report::Wait { ledger, question, depth } => {
                    let units = waiting.entry(ledger).or_default();
                    crowded |= units.iter().any(|&(_, q, d)| q == question && d == depth);
                    next += 1;
                    units.push((next, question, depth));
                    record_admission(waiting_report(ledger, next, question, depth, true));
                }
                Report::Admit { ledger, pick } => {
                    let units = waiting.entry(ledger).or_default();
                    if units.is_empty() {
                        continue;
                    }
                    let (order, question, depth) = units.remove(pick % units.len());
                    crowded |= units.iter().any(|&(_, q, d)| q == question && d == depth);
                    record_admission(waiting_report(ledger, order, question, depth, false));
                }
                Report::Grant { ledger, question, depth } => {
                    let units = waiting.entry(ledger).or_default();
                    crowded |= units.iter().any(|&(_, q, d)| q == question && d == depth);
                    record_admission(granted_report(ledger, question, depth));
                }
            }
        }
        let recorded = READINGS.with(|readings| {
            readings.borrow().ledgers.values().any(|ledger| ledger.crowded.is_some())
        });
        prop_assert_eq!(recorded, crowded);
    }
}

/// Two questions at one depth waiting at once in one ledger fail the
/// session's reading as it ends.
#[test]
#[should_panic(expected = "while another unit of its kind and depth waited")]
fn two_waiting_units_of_one_kind_and_depth_fail_the_reading() {
    start_reading(&Link::new(LinkConfig::with_bdp(LAN_BDP)));
    record_admission(waiting_report(1, 1, true, 3, true));
    record_admission(waiting_report(1, 2, true, 3, true));
    admitted();
}

/// A unit admitted at once while an earlier unit of its kind and depth
/// still waits in its ledger fails the session's reading as it ends.
///
/// The admitted unit never waited itself, as an exempt unit need not, but
/// what its kind and depth holds rose while the earlier unit waited.
#[test]
#[should_panic(expected = "while another unit of its kind and depth waited")]
fn a_unit_admitted_past_a_waiting_one_of_its_kind_and_depth_fails_the_reading() {
    start_reading(&Link::new(LinkConfig::with_bdp(LAN_BDP)));
    record_admission(waiting_report(1, 1, true, 3, true));
    record_admission(granted_report(1, true, 3));
    admitted();
}
