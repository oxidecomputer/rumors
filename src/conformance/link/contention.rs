//! Independence between control traffic and data streams under shared pressure.

use super::*;

/// Control traffic must flow beside stalled data streams, and data streams
/// must flow beside stalled control traffic, in both directions at once.
///
/// This exposes shared buffer limits that separate control and data probes
/// may miss. Buffers larger than the probes exercise can still hide coupling;
/// see the [suite's limits](crate::conformance::link#what-the-suite-cannot-see).
///
/// The deadline covers pair construction and this check.
pub async fn check_control_data_independence<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab, D>(
    pair: impl AsyncFnOnce() -> (Link<CRa, CWa, Ca, Aa>, Link<CRb, CWb, Cb, Ab>),
    deadline: impl FnOnce() -> D,
) where
    CRa: AsyncRead + Unpin + Send,
    CWa: AsyncWrite + Unpin + Send,
    Ca: Connector,
    Aa: Acceptor,
    CRb: AsyncRead + Unpin + Send,
    CWb: AsyncWrite + Unpin + Send,
    Cb: Connector,
    Ab: Acceptor,
    D: Future<Output = ()>,
{
    timed("check_control_data_independence", deadline(), async {
        let (a, b) = pair().await;
        let mut a = a.into_parts();
        let mut b = b.into_parts();
        let ((mut ab, b_held), (mut ba, a_held)) = join(
            open_streams(&a.connector, &mut b.acceptor, true),
            open_streams(&b.connector, &mut a.acceptor, true),
        )
        .await;
        let pressure = join(
            join_all(ab.iter_mut().map(press)),
            join_all(ba.iter_mut().map(press)),
        );
        let control = join(
            duplex_exchange(
                &mut a.control_read,
                &mut a.control_write,
                CONTROL_DUPLEX_TAG_AB,
                CONTROL_DUPLEX_TAG_BA,
            ),
            duplex_exchange(
                &mut b.control_read,
                &mut b.control_write,
                CONTROL_DUPLEX_TAG_BA,
                CONTROL_DUPLEX_TAG_AB,
            ),
        );
        // Poll pressure first so small shared pools fill before control tries to
        // use them. Receivers stay alive and unread until pressure is cancelled.
        finish_beside(pressure, control).await;
        drop((ab, ba));
        // Return consumed-byte credit before the next probe. It must not depend
        // on a transport reclaiming unread buffers merely because a stream closes.
        join(
            join_all(a_held.into_iter().map(drain)),
            join_all(b_held.into_iter().map(drain)),
        )
        .await;

        let pressure = join(press(&mut a.control_write), press(&mut b.control_write));
        let data = join(
            probe_concurrency(&a.connector, &mut b.acceptor, true),
            probe_concurrency(&b.connector, &mut a.acceptor, true),
        );
        finish_beside(pressure, data).await;
    })
    .await;
}

/// Drain an aborted pressure stream and return its buffer credit.
async fn drain<R: AsyncRead + Unpin>(mut read: R) {
    tokio::io::copy(&mut read, &mut tokio::io::sink())
        .await
        .expect("contract: an aborted stream drains to EOF");
}

/// Require live work to finish while pressure is polled on every wake.
async fn finish_beside<P: Future, L: Future>(pressure: P, live: L) {
    match select(pin!(pressure), pin!(live)).await {
        Either::Left(_) => unreachable!("pressure runs until the live work finishes"),
        Either::Right(_) => {}
    }
}
