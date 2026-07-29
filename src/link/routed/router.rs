//! The router: one loop that owns the listener and never blocks on
//! anyone.
//!
//! Structurally the router is a single loop funneling connections to
//! many links — the same shape whose head-of-line failure the link
//! contract exists to preclude, one layer up. The discipline here is
//! the module's core invariant:
//!
//! - **Never await a per-link queue.** Delivery is `try_send` into a
//!   bounded channel; a full queue evicts that link (see
//!   [`StreamAcceptor`]'s docs for what its owner observes), and the
//!   loop moves on.
//! - **Hand off connections, not bytes.** After the header read a
//!   connection never touches the router again, so flow control stays
//!   end-to-end between the peers of each stream. A router that
//!   proxied bytes would be a mux, and would inherit every coupling
//!   problem the one-connection-per-stream shape exists to avoid.
//! - **Bound the header reads.** The header read is the router's only
//!   I/O on a connection, bounded in size by the wire format and in
//!   count by eviction: reads run as concurrent futures inside the
//!   drive future (the crate spawns nothing), and past
//!   [`Config::pending_headers`](super::Config::pending_headers) the
//!   oldest pending read is aborted, its connection dropped. A peer
//!   that dials and stalls mid-header therefore occupies one slot
//!   until displaced, and can never park the loop. Wall-clock bounds
//!   belong to the caller's [`Listen`]/[`Conn`](super::Conn) wrappers,
//!   where a runtime's clock lives.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};

use futures::future::{AbortHandle, Abortable};
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::io::{AsyncWriteExt, split};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;

use super::endpoint::{Arrival, LinkInfo};
use super::header::{self, Header, Token};
use super::stream::{StreamAcceptor, StreamConnector};
use super::{Dial, Link, Listen};
use crate::link::STREAM_COUNT;

/// The routing table: each live link's token, mapped to the bounded
/// queue its acceptor drains.
///
/// Shared between the router (inserts on inbound links, removes on
/// eviction), the endpoint (inserts on outbound links), and every
/// link's [`Registration`] (removes on drop). The mutex is never held
/// across an await.
pub(super) type Table<C> = Arc<Mutex<HashMap<Token, mpsc::Sender<C>>>>;

/// Lock the table, riding through a poisoning panic.
///
/// Every critical section is a single map operation, so a panic
/// elsewhere cannot leave the map torn; continuing lets the surviving
/// side of a panicked test observe eviction rather than a poison
/// cascade.
fn entries<C>(
    table: &Mutex<HashMap<Token, mpsc::Sender<C>>>,
) -> MutexGuard<'_, HashMap<Token, mpsc::Sender<C>>> {
    table
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A link's claim on its token: dropping it revokes the route.
///
/// Held by the link's [`StreamAcceptor`], so a
/// link that is dropped — poisoned, finished, or abandoned — stops
/// routing at that moment, whether or not the router ever hears
/// another byte. Connections that arrive for a revoked token are
/// dropped on sight.
pub(super) struct Registration<C> {
    table: Table<C>,
    token: Token,
}

impl<C> Registration<C> {
    /// Claim `token` in `table`; the claim ends when the value drops.
    pub(super) fn new(table: Table<C>, token: Token) -> Self {
        Registration { table, token }
    }
}

impl<C> Drop for Registration<C> {
    fn drop(&mut self) {
        entries(&self.table).remove(&self.token);
    }
}

/// Register a fresh outbound link: mint an unclaimed token and claim
/// it.
///
/// Collisions are astronomically unlikely at the token's width; the
/// loop makes the vacancy structural rather than probabilistic.
pub(super) fn register<C>(table: &Table<C>) -> (Token, Registration<C>, mpsc::Receiver<C>) {
    let (sender, receiver) = mpsc::channel(STREAM_COUNT);
    let mut sender = Some(sender);
    let token = loop {
        let token = Token::new();
        if let Entry::Vacant(vacancy) = entries(table).entry(token) {
            vacancy.insert(sender.take().expect("the loop ends at the first vacancy"));
            break token;
        }
    };
    (token, Registration::new(table.clone(), token), receiver)
}

/// Drive one endpoint's router until the listener fails.
///
/// The loop selects between accepting the next connection and
/// retiring finished header reads; all per-connection work (the
/// header read, the `LINK` acknowledgement, delivery) happens inside
/// the per-connection futures, so the loop itself never awaits any
/// one connection's progress.
pub(super) async fn drive<D, L>(
    mut listen: L,
    dial: D,
    table: Table<D::Conn>,
    incoming: mpsc::Sender<Arrival<D>>,
    pending_headers: usize,
) -> io::Result<()>
where
    D: Dial,
    L: Listen<Conn = D::Conn>,
{
    let mut pending = FuturesUnordered::new();
    // Insertion-ordered abort handles for the pending reads, so the
    // count bound evicts oldest-first; ids reconcile the two
    // collections because reads finish in arbitrary order.
    let mut order: VecDeque<(u64, AbortHandle)> = VecDeque::new();
    let mut next_id: u64 = 0;
    loop {
        tokio::select! {
            accepted = listen.accept() => {
                let conn = accepted?;
                if order.len() >= pending_headers
                    && let Some((_, oldest)) = order.pop_front()
                {
                    oldest.abort();
                }
                let (abort, registration) = AbortHandle::new_pair();
                let id = next_id;
                next_id += 1;
                order.push_back((id, abort));
                pending.push(Abortable::new(
                    route(conn, dial.clone(), &table, &incoming, id),
                    registration,
                ));
            }
            Some(finished) = pending.next() => {
                if let Ok(id) = finished {
                    order.retain(|(pending_id, _)| *pending_id != id);
                }
            }
        }
    }
}

/// Read one connection's header and route it; errors drop the
/// connection.
///
/// Returns the read's id so the drive loop can retire its abort
/// handle.
async fn route<D: Dial>(
    conn: D::Conn,
    dial: D,
    table: &Table<D::Conn>,
    incoming: &mpsc::Sender<Arrival<D>>,
    id: u64,
) -> u64 {
    // A failure here is a connection that never became anyone's
    // stream: the dialer observes the drop as transport failure, and
    // there is no one else to tell.
    let _ = deliver(conn, dial, table, incoming).await;
    id
}

/// The routing step behind [`route`]: parse, then attach or establish.
async fn deliver<D: Dial>(
    mut conn: D::Conn,
    dial: D,
    table: &Table<D::Conn>,
    incoming: &mpsc::Sender<Arrival<D>>,
) -> io::Result<()> {
    match header::read::<D::Addr, _>(&mut conn).await? {
        Header::Stream { token } => {
            let Some(queue) = entries(table).get(&token).cloned() else {
                // Unknown token: a revoked or never-registered link.
                // Dropping the connection surfaces as transport failure
                // on the dialing side, which owns the retry.
                return Ok(());
            };
            match queue.try_send(conn) {
                Ok(()) => {}
                // A full queue proves peer misbehavior (an honest peer
                // never exceeds a session's complement, and the queue
                // holds one): evict the link so its owner sees a
                // transport error instead of silently losing this
                // stream and hanging its session.
                Err(TrySendError::Full(overflow)) => {
                    drop(overflow);
                    entries(table).remove(&token);
                }
                // The acceptor is already gone; its registration is
                // being (or has been) revoked with it.
                Err(TrySendError::Closed(orphan)) => drop(orphan),
            }
        }
        Header::Link { token, peer } => {
            let (sender, receiver) = mpsc::channel(STREAM_COUNT);
            {
                let mut entries = entries(table);
                if entries.contains_key(&token) {
                    // A duplicate establishment is a peer bug (tokens
                    // are minted fresh per link); dropping it leaves
                    // the live link undisturbed.
                    return Ok(());
                }
                entries.insert(token, sender);
            }
            let registration = Registration::new(table.clone(), token);
            // Reserve the application's slot before acknowledging: a
            // full backlog rejects the link while the dialer is still
            // waiting on the acknowledgement, the crisp failure. The
            // registration guard revokes the token on every early
            // return.
            let Ok(slot) = incoming.try_reserve() else {
                return Ok(());
            };
            conn.write_all(&[header::ACK]).await?;
            let (control_read, control_write) = split(conn);
            let link = Link::new(
                control_read,
                control_write,
                StreamConnector::new(dial, peer.clone(), token),
                StreamAcceptor::new(receiver, registration),
            );
            slot.send((LinkInfo { peer, token }, link));
        }
    }
    Ok(())
}
