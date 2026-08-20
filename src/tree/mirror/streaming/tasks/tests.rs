use futures::FutureExt;

use super::park_after_published_error;

/// The park is the mechanism that removes the published-error race, not
/// a courtesy.
///
/// For `failed = true` it never completes, so a task that already
/// published an error can never contribute a successful completion to
/// its session; for `failed = false` it completes immediately and the
/// pump proceeds.
#[test]
fn park_after_published_error_parks_only_on_failure() {
    assert!(
        park_after_published_error(false).now_or_never().is_some(),
        "an unfailed pump proceeds",
    );
    assert!(
        park_after_published_error(true).now_or_never().is_none(),
        "a failed pump parks forever",
    );
}
