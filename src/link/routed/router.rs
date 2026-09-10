//! Routes whole connections without forwarding their payload bytes.
//!
//! [`Router::run`] polls new arrivals, pending routing work, and returned
//! connections concurrently, without giving one source fixed priority.
//! It yields between routing events so an always-ready listener cannot keep
//! the executor from polling the application's other tasks.
//! It starts [`Router::route`] for each fresh connection to read its header,
//! then [`Router::dispatch`] establishes a link or delivers a data stream to
//! an existing one. The router handles routing bytes; the link carries session
//! traffic.
//!
//! [`Config::pending_headers`] limits fresh connections whose initial routing
//! has not finished. This includes reading the header and, for a new link,
//! sending its acknowledgement. At capacity, the router pauses acceptance
//! until an attempt finishes or fails. Admitted attempts continue to be polled;
//! connections already handed off no longer count. Each attempt races the
//! listener's deadline, which is dropped when routing finishes.
//!
//! A completed stream's `Done` callback puts its connection in the owning
//! [`Route::returned`] and notifies the router. Notifications coalesce: the
//! next wake collects all returns and starts [`Router::reuse`] to send READY
//! and read their next headers. Reused connections enter `dispatch` again.
//!
//! Queued returns and connections waiting for reuse share a separate
//! [`STREAM_COUNT`] limit per link. Fresh arrivals cannot displace them:
//! they remain until reused, closed by the transport, or released with
//! their link.
//!
//! Delivery never waits for a link's consumer. A full stream queue evicts
//! that link so overflow reports a failure instead of losing a stream.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::future::Future;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::task::Poll;

use futures::stream::{FuturesUnordered, StreamExt};
use tokio::io::{AsyncWriteExt, split};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{Notify, mpsc, watch};

use super::endpoint::{Arrival, LinkInfo};
use super::header::{self, Header, Token};
use super::stream::{StreamAcceptor, StreamConnector};
use super::{Config, Dial, Done, Link, Listen};
use crate::link::STREAM_COUNT;

/// A link's stream queue and ownership of its idle incoming connections.
struct Route<C> {
    /// Data streams waiting for this link's acceptor.
    queue: mpsc::Sender<(C, Done<C>)>,
    /// Completed connections waiting for the router to send READY.
    returned: Vec<Returned<C>>,
    /// Receiver count bounds queued and reusable connections together.
    /// Dropping this sender cancels the router's waits for reuse.
    idle: watch::Sender<()>,
}

impl<C> Route<C> {
    /// Track a link's delivery queue with no idle connections yet.
    fn new(queue: mpsc::Sender<(C, Done<C>)>) -> Self {
        Route {
            queue,
            returned: Vec::new(),
            idle: watch::Sender::new(()),
        }
    }
}

/// Maps each link's unique token to its routing queue and idle connections.
///
/// Owned by the router; endpoints and registrations hold weak references.
/// Never locked across an await.
pub(super) struct Table<C> {
    /// Shared by the router, outbound establishment, and stream completion.
    routes: Mutex<HashMap<Token, Route<C>>>,
}

impl<C> Table<C> {
    /// Create a routing table with no registered links.
    pub(super) fn new() -> Self {
        Table {
            routes: Mutex::new(HashMap::new()),
        }
    }

    /// Lock the routes, retaining access after a panic in another holder.
    fn entries(&self) -> MutexGuard<'_, HashMap<Token, Route<C>>> {
        self.routes.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Remove a route, dropping its connections after unlocking.
    fn remove(&self, token: &Token) {
        let removed = self.entries().remove(token);
        drop(removed);
    }

    /// Register an outbound link before its peer can open data streams.
    /// Its acceptor holds the returned registration, which removes the route
    /// on drop.
    #[allow(clippy::type_complexity)]
    pub(super) fn register(
        self: &Arc<Self>,
    ) -> (Token, Registration<C>, mpsc::Receiver<(C, Done<C>)>) {
        let (sender, receiver) = mpsc::channel(STREAM_COUNT);
        let token = loop {
            let token = Token::new();
            if let Entry::Vacant(vacancy) = self.entries().entry(token) {
                vacancy.insert(Route::new(sender));
                break token;
            }
        };
        (token, Registration::new(self, token), receiver)
    }
}

/// Revokes a link's route when its acceptor drops.
pub(super) struct Registration<C> {
    /// Removing a route must not keep a stopped router's table alive.
    table: Weak<Table<C>>,
    /// The link whose route this registration revokes.
    token: Token,
}

impl<C> Registration<C> {
    /// Arrange to remove `token` when the owning acceptor drops.
    fn new(table: &Arc<Table<C>>, token: Token) -> Self {
        Registration {
            table: Arc::downgrade(table),
            token,
        }
    }
}

impl<C> Drop for Registration<C> {
    /// Revoke the route if its router is still alive.
    fn drop(&mut self) {
        if let Some(table) = self.table.upgrade() {
            table.remove(&self.token);
        }
    }
}

/// A completed connection counted against its link's idle limit.
struct Returned<C> {
    /// Retained until reuse, transport failure, or link teardown.
    conn: C,
    /// Counts this connection against the idle limit and detects link teardown.
    released: watch::Receiver<()>,
}

/// Owns an endpoint's routing state for the lifetime of [`Self::run`].
/// The per-connection methods borrow it, so dropping `run` cancels them all.
pub(super) struct Router<D: Dial> {
    /// Cloned into incoming links so they can open streams back to their peer.
    dial: D,
    /// Owns the routes; endpoints and stream completions hold weak references.
    table: Arc<Table<D::Conn>>,
    /// New links waiting for the application's `Incoming::accept`.
    incoming: mpsc::Sender<Arrival<D>>,
    /// Coalesces completion callbacks; the connections stay in their routes.
    returned: Arc<Notify>,
    /// Fresh-routing capacity and outgoing reuse policy for incoming links.
    config: Config,
}

impl<D: Dial> Router<D> {
    /// Assemble routing state; connection processing begins in [`Self::run`].
    pub(super) fn new(
        dial: D,
        table: Arc<Table<D::Conn>>,
        incoming: mpsc::Sender<Arrival<D>>,
        config: Config,
    ) -> Self {
        Router {
            dial,
            table,
            incoming,
            returned: Arc::new(Notify::new()),
            config,
        }
    }

    /// Poll fresh routing and reuse concurrently until the listener fails.
    /// Completed connections are collected together on each notification; their
    /// reuse waits have separate capacity from fresh routing attempts.
    pub(super) async fn run(self, mut listen: impl Listen<Conn = D::Conn>) -> io::Result<()> {
        let mut pending = FuturesUnordered::new();
        let mut idle = FuturesUnordered::new();
        loop {
            // An immediately ready listener and routing work can keep the
            // select below ready indefinitely. Yield independently of the
            // transport and runtime so other tasks still get polled.
            let mut yielded = false;
            std::future::poll_fn(|cx| {
                if std::mem::replace(&mut yielded, true) {
                    Poll::Ready(())
                } else {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            })
            .await;
            // Keep the select fair: a steady arrival of fresh connections must
            // not starve returned connections or completed header reads.
            let conn = tokio::select! {
                accepted = listen.accept(), if pending.len() < self.config.pending_headers => accepted?,
                _ = self.returned.notified() => {
                    for (&token, route) in self.table.entries().iter_mut() {
                        for conn in route.returned.drain(..) {
                            idle.push(self.reuse(token, conn));
                        }
                    }
                    continue;
                }
                Some(()) = pending.next() => continue,
                Some(()) = idle.next() => continue,
            };
            pending.push(self.route(conn, listen.routing_deadline()));
        }
    }

    /// Await reuse or link teardown, independently of fresh routing attempts.
    async fn reuse(&self, token: Token, returned: Returned<D::Conn>) {
        let Returned {
            mut conn,
            mut released,
        } = returned;
        // Teardown takes priority when the next header is ready at the same time.
        let header = tokio::select! {
            biased;
            _ = released.changed() => None,
            header = async {
                conn.write_all(&[header::READY]).await?;
                conn.flush().await?;
                header::read::<D::Addr, _>(&mut conn).await
            } => header.ok(),
        };
        // Release admission before delivery can hand the connection back again.
        drop(released);
        if let Some(Header::Stream { token: next }) = header
            && next == token
        {
            let _ = self.dispatch(conn, Header::Stream { token }).await;
        }
    }

    /// Route a fresh connection within the listener's deadline. Cancellation
    /// also drops any registration and incoming backlog reservation.
    async fn route(&self, mut conn: D::Conn, deadline: impl Future<Output = ()> + Send) {
        // Once the deadline has expired, do not advance this attempt further.
        tokio::select! {
            biased;
            _ = deadline => {}
            _ = async {
                if let Ok(header) = header::read::<D::Addr, _>(&mut conn).await {
                    let _ = self.dispatch(conn, header).await;
                }
            } => {}
        }
    }

    /// Hand off a data stream, or register and acknowledge a new link.
    /// A stream's `Done` returns its connection to the route and wakes `run`.
    async fn dispatch(&self, mut conn: D::Conn, header: Header<D::Addr>) -> io::Result<()> {
        match header {
            Header::Stream { token } => {
                let Some(queue) = self
                    .table
                    .entries()
                    .get(&token)
                    .map(|route| route.queue.clone())
                else {
                    return Ok(());
                };
                let returned = Arc::clone(&self.returned);
                let table = Arc::downgrade(&self.table);
                let done = Done::new(move |conn| {
                    let Some(table) = table.upgrade() else { return };
                    {
                        let mut entries = table.entries();
                        let Some(route) = entries.get_mut(&token) else {
                            return;
                        };
                        // Include connections already waiting for reuse in this limit.
                        if route.idle.receiver_count() >= STREAM_COUNT {
                            return;
                        }
                        route.returned.push(Returned {
                            conn,
                            released: route.idle.subscribe(),
                        });
                    }
                    returned.notify_one();
                });
                match queue.try_send((conn, done)) {
                    Ok(()) => {}
                    Err(TrySendError::Full(overflow)) => {
                        drop(overflow);
                        self.table.remove(&token);
                    }
                    Err(TrySendError::Closed(orphan)) => drop(orphan),
                }
            }
            Header::Link { token, peer } => {
                let (sender, receiver) = mpsc::channel(STREAM_COUNT);
                {
                    let mut entries = self.table.entries();
                    let Entry::Vacant(vacancy) = entries.entry(token) else {
                        return Ok(());
                    };
                    vacancy.insert(Route::new(sender));
                }
                let registration = Registration::new(&self.table, token);
                // Reserve before acknowledging so a full backlog rejects the
                // establishment while the dialer is still waiting for an answer.
                let Ok(slot) = self.incoming.try_reserve() else {
                    return Ok(());
                };
                conn.write_all(&[header::ACK]).await?;
                conn.flush().await?;
                let (control_read, control_write) = split(conn);
                let link = Link::new(
                    control_read,
                    control_write,
                    StreamConnector::new(
                        self.dial.clone(),
                        peer.clone(),
                        token,
                        self.config.pooling,
                    ),
                    StreamAcceptor::new(receiver, registration),
                );
                slot.send((LinkInfo { peer, token }, link));
            }
        }
        Ok(())
    }
}
