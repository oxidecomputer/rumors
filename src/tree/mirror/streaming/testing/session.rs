//! A whole remote session between two replicas over one in-memory pipe.
//!
//! [`over_pipe`] runs both endpoints' walks and proxies, each endpoint's two
//! halves sharing its window, under the closed-world poller, which reports a
//! stall whenever every task waits with no wake arranged. Suites drive
//! whole remote sessions through it rather than assembling their own.

use std::convert::Infallible;

use futures::join;
use serde::{Serialize, de::DeserializeOwned};

use crate::DEFAULT_TARGET_MESSAGE_SIZE;
use crate::message::{PayloadCodec, PayloadDepthLimit};
use crate::testing::{Quiescence, memory_connection, run_to_quiescence};
use crate::tree::Root;
use crate::tree::mirror::Error as MirrorError;
use crate::tree::mirror::streaming::materialized::{Error as MaterializedError, Handshaking};
use crate::tree::mirror::streaming::remote::{
    Error as RemoteError, Handshaking as RemoteHandshaking,
};
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::tree::mirror::streaming::{Local, mirror};

/// Bytes each direction of an ample pipe buffers: enough that the pipe never
/// holds back a frame in these suites' sessions, so only the walk's and the
/// proxy's own queues can hold a stage.
pub const AMPLE: usize = 1 << 20;

/// One endpoint's reconciled tree, or the failure of whichever of its walk
/// and proxy raised one.
pub type Endpoint =
    Result<Root, MirrorError<MaterializedError<Infallible>, RemoteError<Infallible>>>;

/// Reconcile a pair over one in-memory connection whose directions each
/// buffer `buffer` bytes; the poller's verdict when the session stopped
/// first.
///
/// Each endpoint's walk resolves its window from that endpoint's entry in
/// `windows`, and its proxy adopts that window; both endpoints decode
/// payloads as `T`.
pub fn over_pipe<T>(
    pair: (Root, Root),
    windows: [WindowConfig; 2],
    buffer: usize,
) -> Result<(Endpoint, Endpoint), Quiescence>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let (a, b) = pair;
    let (a_end, b_end) = memory_connection(buffer);
    let [a_window, b_window] = windows;
    let codec = PayloadCodec::new::<T>(PayloadDepthLimit::default());
    run_to_quiescence(async move {
        let size = DEFAULT_TARGET_MESSAGE_SIZE as u64;
        let a_walk = Handshaking::start(Local, a.into(), size).window(a_window);
        let b_walk = Handshaking::start(Local, b.into(), size).window(b_window);
        let a_proxy = RemoteHandshaking::start(Local, a_end.read, a_end.write, codec);
        let b_proxy = RemoteHandshaking::start(Local, b_end.read, b_end.write, codec);
        let (a, b) = join!(
            Box::pin(mirror(a_walk, a_proxy)),
            Box::pin(mirror(b_walk, b_proxy)),
        );
        (
            a.map(|(root, _pipe)| root.into()),
            b.map(|(root, _pipe)| root.into()),
        )
    })
}

/// Both endpoints' reconciled trees.
///
/// # Panics
///
/// Panics with the failure of either endpoint; a test that expects every
/// session it drives to succeed calls this on [`over_pipe`]'s outcome.
pub fn completed((a, b): (Endpoint, Endpoint)) -> (Root, Root) {
    (
        a.expect("the first endpoint's session failed"),
        b.expect("the second endpoint's session failed"),
    )
}
