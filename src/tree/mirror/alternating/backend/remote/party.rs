//! The trailing party hand-off after an alternating mirror descent.

use before::Party;
use tokio::io::{AsyncRead, AsyncWrite};

use super::{Error, FrameRead, FrameWrite, recv_msg, send_msg};

/// Ship a donated party after reconciliation has transferred all content.
///
/// Bootstrapping sends a freshly forked party from provider to newcomer;
/// retirement sends the retiree's whole party toward its absorber. Forking or
/// taking the party before the descent, but putting it on the wire only after
/// the descent succeeds, bounds the unavoidable two-generals uncertainty to
/// this final frame.
pub(crate) async fn send_party<W>(party: Party, writer: &mut FrameWrite<W>) -> Result<(), Error>
where
    W: AsyncWrite + Unpin,
{
    send_msg(writer, &party).await
}

/// Receive the party promised by a bootstrapping provider or retiring peer.
pub(crate) async fn recv_party<R>(reader: &mut FrameRead<R>) -> Result<Party, Error>
where
    R: AsyncRead + Unpin,
{
    recv_msg(reader).await
}
