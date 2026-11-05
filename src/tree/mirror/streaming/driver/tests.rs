use std::convert::identity;
use std::pin::pin;
use std::task::Poll;

use futures::future;

use super::{error_routes, race_session};

/// A response error published while polling wins over its terminal symptom.
#[test]
fn routed_error_precedes_same_poll_session_error() {
    let (route, _other, first_error) = error_routes(identity::<&str>, identity::<&str>);
    let session = future::poll_fn(move |_| {
        route.report("primary");
        Poll::Ready(Err::<(), _>("secondary"))
    });

    assert_eq!(
        pollster::block_on(race_session(pin!(session), first_error)),
        Err("primary")
    );
}

/// A terminal error remains authoritative when no response error preceded it.
#[test]
fn standalone_session_error_is_preserved() {
    let (_left, _right, first_error) = error_routes(identity::<&str>, identity::<&str>);
    let session = future::ready(Err::<(), _>("terminal"));

    assert_eq!(
        pollster::block_on(race_session(pin!(session), first_error)),
        Err("terminal")
    );
}

/// The longest report sequence [`first_report_wins_across_routes`] exhausts.
const MAX_REPORTS: usize = 4;

/// For every sequence of reports from either route, up to [`MAX_REPORTS`]
/// long, the session fails with the first report, and no later report
/// blocks its reporter.
///
/// Each sequence is the bits of `mask` below its `length`: a set bit reports
/// through the right route, a clear bit through the left. Each report
/// carries its position, so the result names which report won. Every report
/// is a synchronous call that returns before the session is first polled, so
/// a report that waited for room would hang the test rather than return.
#[test]
fn first_report_wins_across_routes() {
    for length in 1..=MAX_REPORTS {
        for mask in 0..(1u32 << length) {
            let (left, right, first_error) = error_routes(identity::<usize>, identity::<usize>);
            for position in 0..length {
                let route = if mask >> position & 1 == 0 {
                    &left
                } else {
                    &right
                };
                route.report(position);
            }
            let session = future::pending::<Result<(), usize>>();

            assert_eq!(
                pollster::block_on(race_session(pin!(session), first_error)),
                Err(0),
                "report sequence {mask:0length$b} of length {length}",
            );
        }
    }
}
