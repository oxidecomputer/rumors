//! Count the bytes each end of an in-memory session writes.

use rumors::Rumors;
use rumors::testing::memory_connection;
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::fault::metered;
use super::wire::block_on;

/// Per-direction capacity large enough not to alter the measured batching.
const CONNECTION_CAPACITY: usize = 8 * 1024 * 1024;

/// Run one session and return bytes written as
/// `(left to right, right to left)`.
///
/// Each count is every byte one end wrote to its write half: the preamble, the
/// greeting, every frame, and the closing items. The counts come from
/// [`metered`] ends, whose budgets never run out.
pub fn session_wire_bytes<T>(left: &Rumors<T>, right: &Rumors<T>) -> (usize, usize)
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let (left_end, right_end) = memory_connection(CONNECTION_CAPACITY);
    let (left_end, left_meter) = metered(left_end);
    let (right_end, right_meter) = metered(right_end);
    block_on(async {
        let (left_result, right_result) = tokio::join!(
            left.gossip_once(left_end.read, left_end.write),
            right.gossip_once(right_end.read, right_end.write)
        );
        left_result.expect("left gossip over metered connection");
        right_result.expect("right gossip over metered connection");
    });
    (left_meter.written(), right_meter.written())
}
