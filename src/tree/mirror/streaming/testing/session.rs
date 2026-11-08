//! A whole remote session between two replicas over one in-memory pipe.
//!
//! [`over_pipe`] runs both endpoints' walks and proxies, each endpoint's walk
//! admitting its work against that endpoint's budget, under the closed-world
//! poller, which reports a stall whenever every task waits with no wake
//! arranged. Suites drive whole remote sessions through it rather than
//! assembling their own.

use std::convert::Infallible;

use futures::join;
use serde::{Serialize, de::DeserializeOwned};

use crate::default;
use crate::message::{PayloadCodec, PayloadDepthLimit};
use crate::observe::{Attachment, SessionKind};
use crate::testing::{Quiescence, memory_connection, run_to_quiescence};
use crate::tree::Root;
use crate::tree::mirror::Error as MirrorError;
use crate::tree::mirror::streaming::Backend;
use crate::tree::mirror::streaming::Leaf;
use crate::tree::mirror::streaming::ledger::Ledger;
use crate::tree::mirror::streaming::materialized::{
    Error as MaterializedError, Handshaking, Start,
};
use crate::tree::mirror::streaming::remote::{
    Error as RemoteError, Handshaking as RemoteHandshaking,
};
use crate::tree::mirror::streaming::{Local, mirror};
use crate::tree::typed::height::Z;

/// Bytes each direction of an ample pipe buffers: enough that the pipe never
/// holds back a frame in these suites' sessions, so only the walk's and the
/// proxy's own queues can hold a stage.
pub const AMPLE: usize = 1 << 20;

/// What one endpoint's ledger admits its work against in a test session.
///
/// Suites name the configuration at every session they drive, since a budget of
/// zero with the walk's bounds and the serial configuration admit differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionLedger {
    /// This budget, beside the walk's bound for each kind and depth of unit
    /// ([`Handshaking::budget`]): a budget of zero still pipelines within the
    /// bounds.
    Budget(usize),
    /// The serial configuration ([`Handshaking::serial`]): a budget of zero
    /// with every bound zero, admitting one unit of each kind and depth at a
    /// time, the configuration the liveness argument rests on at its most
    /// serial.
    Serial,
}

impl SessionLedger {
    /// Give `walk` this ledger.
    pub fn apply<B: Backend<Node<Z>: Leaf>>(
        self,
        walk: Handshaking<B, Start>,
    ) -> Handshaking<B, Start> {
        match self {
            Self::Budget(bytes) => walk.budget(bytes),
            Self::Serial => walk.serial(),
        }
    }
}

/// One endpoint's reconciled tree, or the failure of whichever of its walk and
/// proxy raised one.
pub type Endpoint =
    Result<Root, MirrorError<MaterializedError<Infallible>, RemoteError<Infallible>>>;

/// Reconcile a pair over one in-memory connection whose directions each buffer
/// `buffer` bytes; the poller's verdict when the session stopped first.
///
/// Each endpoint's walk admits its work against the ledger its entry in
/// `budgets` gives ([`SessionLedger`]); both endpoints decode payloads as `T`.
pub fn over_pipe<T>(
    pair: (Root, Root),
    budgets: [SessionLedger; 2],
    buffer: usize,
) -> Result<(Endpoint, Endpoint), Quiescence>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    observed_over_pipe::<T>(pair, budgets, buffer, &Attachment::default())
}

/// [`over_pipe`], with both endpoints' sessions observed by `observers`.
pub fn observed_over_pipe<T>(
    pair: (Root, Root),
    budgets: [SessionLedger; 2],
    buffer: usize,
    observers: &Attachment,
) -> Result<(Endpoint, Endpoint), Quiescence>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    session::<T>(pair, budgets, buffer, observers).0
}

/// [`over_pipe`], also returning both endpoints' ledgers, in the pair's order,
/// which outlive the session for a test to read.
pub fn over_pipe_keeping_ledgers<T>(
    pair: (Root, Root),
    budgets: [SessionLedger; 2],
    buffer: usize,
) -> (Result<(Endpoint, Endpoint), Quiescence>, [Ledger; 2])
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    session::<T>(pair, budgets, buffer, &Attachment::default())
}

/// The session [`observed_over_pipe`] and [`over_pipe_keeping_ledgers`] run,
/// with both endpoints' ledgers beside its outcome.
fn session<T>(
    pair: (Root, Root),
    budgets: [SessionLedger; 2],
    buffer: usize,
    observers: &Attachment,
) -> (Result<(Endpoint, Endpoint), Quiescence>, [Ledger; 2])
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let (a, b) = pair;
    let (a_observe, b_observe) = (
        observers.begin(SessionKind::Gossip),
        observers.begin(SessionKind::Gossip),
    );
    let (a_end, b_end) = memory_connection(buffer);
    let [a_budget, b_budget] = budgets;
    let codec = PayloadCodec::new::<T>(PayloadDepthLimit::default());
    let size = default::TARGET_MESSAGE_SIZE as u64;
    let a_walk = a_budget.apply(Handshaking::start(
        Local,
        a.into(),
        Some(crate::network::NetworkKeys::fixture()),
        size,
    ));
    let b_walk = b_budget.apply(Handshaking::start(
        Local,
        b.into(),
        Some(crate::network::NetworkKeys::fixture()),
        size,
    ));
    let ledgers = [a_walk.ledger(), b_walk.ledger()];
    let outcome = run_to_quiescence(async move {
        let a_proxy = RemoteHandshaking::start(
            Local,
            a_end.read,
            a_end.write,
            codec,
            Some(crate::network::NetworkKeys::fixture()),
        )
        .observe(a_observe);
        let b_proxy = RemoteHandshaking::start(
            Local,
            b_end.read,
            b_end.write,
            codec,
            Some(crate::network::NetworkKeys::fixture()),
        )
        .observe(b_observe);
        // Each endpoint's walk and proxy run inside its own future, so the
        // channel observation can tell the two sides' queues apart.
        let (a, b) = join!(
            Box::pin(mirror(a_walk, a_proxy)),
            Box::pin(mirror(b_walk, b_proxy)),
        );
        (
            a.map(|(root, _pipe)| root.into()),
            b.map(|(root, _pipe)| root.into()),
        )
    });
    (outcome, ledgers)
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
