//! Count bytes written by each end of an in-memory session.

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use rumors::Rumors;
use rumors::link::{Connector, Done, Link, MemoryLink};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::AsyncWrite;

use super::wire::block_on;

/// Per-stream capacity large enough not to alter the measured batching.
const LINK_CAPACITY: usize = 8 * 1024 * 1024;

/// An async writer that counts bytes accepted by its inner writer.
struct CountingWrite<W> {
    /// The writer that receives the bytes.
    inner: W,
    /// Bytes accepted by this session end.
    written: Arc<AtomicUsize>,
}

/// Forward writes while counting their accepted bytes.
impl<W: AsyncWrite + Unpin> AsyncWrite for CountingWrite<W> {
    /// Count bytes accepted by a successful write.
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let poll = Pin::new(&mut self.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(accepted)) = &poll {
            self.written.fetch_add(*accepted, Ordering::Relaxed);
        }
        poll
    }

    /// Flush the inner writer.
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    /// Shut down the inner writer.
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// A connector that counts writes on every stream it opens.
#[derive(Clone)]
struct CountingConnector<C> {
    /// The connector that opens the stream.
    inner: C,
    /// Bytes accepted by this session end.
    written: Arc<AtomicUsize>,
}

/// Wrap each opened stream in a [`CountingWrite`].
impl<C: Connector> Connector for CountingConnector<C> {
    /// The counted data-stream writer.
    type Tx = CountingWrite<C::Tx>;

    /// Open and wrap one data stream.
    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        let (inner, _) = self.inner.connect().await?;
        Ok((
            CountingWrite {
                inner,
                written: self.written.clone(),
            },
            Done::discard(),
        ))
    }
}

/// Count every write made through one end of an in-memory link.
fn counting(
    link: MemoryLink,
    written: &Arc<AtomicUsize>,
) -> Link<
    tokio::io::DuplexStream,
    CountingWrite<tokio::io::DuplexStream>,
    CountingConnector<rumors::link::MemoryConnector>,
    rumors::link::MemoryAcceptor,
> {
    link.map_transport(|control_read, control_write, connector, acceptor| {
        (
            control_read,
            CountingWrite {
                inner: control_write,
                written: written.clone(),
            },
            CountingConnector {
                inner: connector,
                written: written.clone(),
            },
            acceptor,
        )
    })
}

/// Run one session and return bytes written as `(left to right, right to left)`.
pub fn session_wire_bytes<T>(left: &Rumors<T>, right: &Rumors<T>) -> (usize, usize)
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let left_written = Arc::new(AtomicUsize::new(0));
    let right_written = Arc::new(AtomicUsize::new(0));
    let (left_link, right_link) = rumors::link::memory_with_capacity(LINK_CAPACITY);
    let mut left_link = counting(left_link, &left_written);
    let mut right_link = counting(right_link, &right_written);
    block_on(async {
        let (left_result, right_result) = tokio::join!(
            left.gossip_once(&mut left_link),
            right.gossip_once(&mut right_link)
        );
        left_result.expect("left gossip over counting link");
        right_result.expect("right gossip over counting link");
    });
    (
        left_written.load(Ordering::Relaxed),
        right_written.load(Ordering::Relaxed),
    )
}
