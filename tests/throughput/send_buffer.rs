//! Unsent bytes in the sending kernel do not slow a divergent session,
//! whether or not its budget binds.
//!
//! # The question
//!
//! A kernel send buffer larger than the bandwidth-delay product holds bytes
//! the application has handed over and the wire has not taken, and no
//! application priority reorders them. In a divergent session, each side's
//! replies carry its own supplies and its *requests*: empty queries for
//! subtrees only it holds, which the peer answers with supplies. Bytes
//! queued ahead of a request delay it, and with it the bulk it invites.
//!
//! Each request a side asks holds room in its ledger for the reply it
//! invites until that reply is consumed, so the side's budget bounds its
//! outstanding requests, `K`. By Little's law, the bulk those requests
//! invite moves at most at `min(bandwidth, K · r / L)`, where `r` is the
//! bytes a request invites and `L` is the round trip plus the time its
//! requests spend queued.
//!
//! One analysis holds that the queue stays full. If the side sending the
//! requests saturates its direction while its send buffer holds `Q` of
//! unsent bytes beyond the product, the invited bulk runs at
//! `f = min(1, c / (1 + Q / RTT))` of the bandwidth, where `c = K · r / BDP`
//! is the requests' coverage of the bare link, and a session moving `D`
//! bytes each way takes about `(2 − f) · D / bandwidth`: up to half again as
//! long with a full product of unsent bytes.
//!
//! # What the sessions show
//!
//! The queue does not stay full. A side's requests and its own supplies
//! leave in the same replies, and its walk asks no further question while
//! the budget holding its outstanding requests is full. When queued bytes
//! stretch `L` until the budget binds, the side's supplies slow with its
//! requests, its direction stops saturating, and its queue drains. The
//! queue settles where the requests' latency just matches what the budget
//! keeps outstanding, so the invited bulk runs at the rate it has with no
//! unsent bytes at all, and the session takes as long.
//!
//! The argument needs a side's bulk to share replies with the requests its
//! budget holds back. Bulk from replies that carry none of them, such as
//! another level's, could keep a queue full while those requests wait. In
//! these fully divergent pairs nearly all of both ride one level's
//! replies.
//!
//! # The measurement
//!
//! Each test runs fully divergent pairs on a LAN, first with an unlimited
//! budget, which reads `Λ`, the most either
//! side's ledger holds outside its floor slots when nothing waits for
//! admission (see the [no-slowdown suite](crate::no_slowdown)). Each
//! session then runs at a budget of `Λ`, where no unit waits, and at half
//! of it, where units wait for admission, once with an unsent limit of zero
//! and once with one bandwidth-delay product. Each such budget is what the
//! ledgers admit against, configured beside the floor allowance
//! ([`admitting`](rumors_testkit::bench::session::admitting)).
//!
//! # Thresholds
//!
//! A session with unsent bytes may take at most its duration without them
//! plus the model's resolution, one segment's serialization
//! ([`SEGMENT_BYTES`](rumors_testkit::bench::link::SEGMENT_BYTES)) per hop
//! of the critical path ([`resolution`]). The analysis's largest predicted
//! excess is half the transfer's serialization, which the test checks is
//! far beyond that allowance, so the verdict cannot rest on the allowance's
//! size. A comparison means something only if the unsent queue fills, and
//! at half of `Λ` only if the budget binds there; the tests check both.

use rumors::DEFAULT_SYNC_MEMORY_BUDGET;
use rumors_testkit::bench::link::{LinkConfig, Side};

use rumors_testkit::bench::session::{self as fixture, Case, LAN_BDP, Session, Shape, resolution};

/// Messages each side holds that the other lacks.
const MESSAGES: u64 = 10_000;

/// The budget of the run that reads `Λ`: unlimited.
const ORACLE_BUDGET: usize = usize::MAX;

/// The LAN with an unsent limit of zero and of one product.
fn links() -> [LinkConfig; 2] {
    let link = LinkConfig::with_bdp(LAN_BDP);
    [link, link.with_unsent_limit(LAN_BDP)]
}

/// Prints one session's per-direction throughput and unsent queue.
fn report(label: &str, session: &Session) {
    let duration = session.duration();
    let mut line = format!(
        "{label}: unsent limit {} B, budget {} B, {duration} byte-times, {} units waited",
        session.case.link.unsent_limit(),
        session.case.budget,
        session.admitted.waited(),
    );
    for side in Side::BOTH {
        let sent = session.report.sent_by(side);
        line.push_str(&format!(
            "; {side:?} sent {} B, {:.3} of the bandwidth over the session, \
             mean unsent {:.0} B, most {} B",
            sent.bytes(),
            sent.bytes() as f64 / duration as f64,
            sent.unsent_area_between(0, duration) as f64 / duration as f64,
            sent.max_unsent(),
        ));
    }
    println!("{line}");
}

/// Asserts that `with` took no longer than `without` plus `allowance`, and
/// that one of `with`'s unsent queues reached its limit, so the comparison
/// tested a buffer that held unsent bytes.
fn assert_unharmed(without: &Session, with: &Session, allowance: u64) {
    let limit = with.case.link.unsent_limit();
    assert!(
        Side::BOTH
            .into_iter()
            .any(|side| with.report.sent_by(side).max_unsent() >= limit),
        "an unsent queue must reach its limit of {limit}, or the comparison tests nothing",
    );
    assert!(
        with.duration() <= without.duration() + allowance,
        "unsent bytes slow the session: {} byte-times against {} without, beyond the \
         resolution of {allowance}",
        with.duration(),
        without.duration(),
    );
}

/// A fully divergent session takes no longer with a bandwidth-delay
/// product of unsent bytes than with none, both at a budget of `Λ` and at
/// half of it, where units wait for admission.
///
/// At half of `Λ` the budget binds, which is where the analysis in the
/// module docs predicts that queued requests cost the session.
#[test]
fn unsent_bytes_leave_a_symmetric_session_unchanged() {
    let shape = Shape::symmetric(MESSAGES);
    let [bare, buffered] = links();
    let case = |budget, link| Case {
        shape,
        budget,
        link,
    };
    let [oracle] = fixture::run_all([case(ORACLE_BUDGET, bare)]);
    let peak = oracle.admitted.budget_peak();
    let [
        critical_path,
        full_bare,
        full_buffered,
        half_bare,
        half_buffered,
    ] = fixture::run_all([
        case(ORACLE_BUDGET, bare.latency_only()),
        case(fixture::admitting(peak), bare),
        case(fixture::admitting(peak), buffered),
        case(fixture::admitting(peak / 2), bare),
        case(fixture::admitting(peak / 2), buffered),
    ]);
    let allowance = resolution(&critical_path);
    for (label, session) in [
        ("Λ, no unsent", &full_bare),
        ("Λ, unsent", &full_buffered),
        ("Λ / 2, no unsent", &half_bare),
        ("Λ / 2, unsent", &half_buffered),
    ] {
        report(label, session);
    }
    let transfer = full_bare.busier_direction_bytes();
    let predicted_excess = transfer / 2;
    assert!(
        predicted_excess > 10 * allowance,
        "the analysis's predicted excess of {predicted_excess} must dwarf the allowance \
         of {allowance}, so the verdict does not rest on the allowance",
    );
    assert_eq!(
        full_bare.admitted.waited(),
        0,
        "no unit waits at a budget of Λ on the bare link",
    );
    assert!(
        half_bare.admitted.waited() > 0 && half_buffered.admitted.waited() > 0,
        "units must wait for admission at half of Λ, where queued requests would cost",
    );
    assert_unharmed(&full_bare, &full_buffered, allowance);
    assert_unharmed(&half_bare, &half_buffered, allowance);
}

/// A catch-up, in which one side holds every message and the other none,
/// takes no longer with a bandwidth-delay product of unsent bytes than with
/// none.
///
/// The bulk crosses one way and the requests the other, so no queued bulk
/// can delay a request; both the analysis and the mechanism predict no
/// effect. Both peers run the default budget.
#[test]
fn unsent_bytes_leave_a_catch_up_unchanged() {
    let shape = Shape::catch_up(MESSAGES);
    let [bare, buffered] = links();
    let case = |link| Case {
        shape,
        budget: DEFAULT_SYNC_MEMORY_BUDGET,
        link,
    };
    let [critical_path, without, with] = fixture::run_all([
        Case {
            shape,
            budget: ORACLE_BUDGET,
            link: bare.latency_only(),
        },
        case(bare),
        case(buffered),
    ]);
    report("catch-up, no unsent", &without);
    report("catch-up, unsent", &with);
    assert_unharmed(&without, &with, resolution(&critical_path));
}
