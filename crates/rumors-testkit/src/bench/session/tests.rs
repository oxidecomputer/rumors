//! Fixture messages' encoded sizes, against an independent CBOR encoder,
//! and a session's records over time, against its ledgers' totals.

use proptest::prelude::*;

use super::{Case, LAN_BDP, LedgerReading, Shape, message, pair, run, text_chars};
use crate::bench::link::{LinkConfig, Side};
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
