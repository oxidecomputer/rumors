//! A process-local accept/connect network for exercising the routed
//! link without sockets.
//!
//! [`MemoryNet`] is a registry of named listeners; dialing a name
//! creates one [`tokio::io::duplex`] connection and delivers one end
//! to the listener, exactly the accept/connect primitive the
//! [`routed`](crate::link::routed) adapter builds on. Everything is
//! channels and buffers, so suites run deterministically under a
//! single-poll executor. Names are plain strings, so they cannot be mistaken
//! for network addresses.

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::io::{DuplexStream, duplex};
use tokio::sync::mpsc;

use crate::link::routed::{Addr, Dial, Listen, Unencodable};

/// Bytes each connection buffers per direction before its writer
/// blocks on its reader.
const CONNECTION_CAPACITY: usize = 8 * 1024;

/// A name on a [`MemoryNet`]: an arbitrary string, encoded as its
/// UTF-8 bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryName(pub String);

impl MemoryName {
    /// Name a peer on the memory network.
    pub fn new(name: impl Into<String>) -> Self {
        MemoryName(name.into())
    }
}

/// Encode names as UTF-8 for routed-link headers.
impl Addr for MemoryName {
    // UTF-8 carries every string faithfully, so encoding never
    // refuses; the endpoint's construction-time length check still
    // binds the name to the header's bounds.
    /// Encode this name as UTF-8 bytes.
    fn encode(&self) -> Result<Vec<u8>, Unencodable> {
        Ok(self.0.clone().into_bytes())
    }

    /// Decode a UTF-8 name.
    fn decode(bytes: &[u8]) -> Option<Self> {
        String::from_utf8(bytes.to_vec()).ok().map(MemoryName)
    }
}

/// A closed-world accept/connect network: named listeners, in-memory
/// connections.
///
/// Clones share the network. Bind listeners with
/// [`listen`](Self::listen), dial them through the [`Dial`] handle
/// from [`dial`](Self::dial).
#[derive(Clone, Default)]
pub struct MemoryNet {
    listeners: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<DuplexStream>>>>,
}

/// Hide the listener registry while making the test network debuggable.
impl std::fmt::Debug for MemoryNet {
    /// Format an opaque network handle.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryNet").finish_non_exhaustive()
    }
}

impl MemoryNet {
    /// An empty network.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a listener at `name`, displacing any earlier binding.
    pub fn listen(&self, name: &MemoryName) -> MemoryListen {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.registry().insert(name.0.clone(), sender);
        MemoryListen { conns: receiver }
    }

    /// A dialer onto this network.
    pub fn dial(&self) -> MemoryDial {
        MemoryDial { net: self.clone() }
    }

    /// Lock the listener registry, riding through a poisoning panic:
    /// each critical section is a single map operation, so the map is
    /// never torn.
    fn registry(&self) -> MutexGuard<'_, HashMap<String, mpsc::UnboundedSender<DuplexStream>>> {
        self.listeners
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// The [`Dial`] half of a [`MemoryNet`].
#[derive(Clone)]
pub struct MemoryDial {
    net: MemoryNet,
}

/// Hide the shared network while making the dialer debuggable.
impl std::fmt::Debug for MemoryDial {
    /// Format an opaque dialer handle.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryDial").finish_non_exhaustive()
    }
}

/// Dial listeners registered in the shared memory network.
impl Dial for MemoryDial {
    type Addr = MemoryName;
    type Conn = DuplexStream;

    /// Create and deliver one in-memory connection.
    async fn dial(&self, addr: &MemoryName) -> io::Result<Self::Conn> {
        let listener = self.net.registry().get(&addr.0).cloned().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "nothing listens at this name",
            )
        })?;
        let (dialed, accepted) = duplex(CONNECTION_CAPACITY);
        // Delivery is unbounded because this is a test network. A dial never
        // waits on the listener's accept pace, matching the routed adapter's
        // requirement that one open not block another.
        listener.send(accepted).map_err(|_| {
            io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "the listener at this name was dropped",
            )
        })?;
        Ok(dialed)
    }
}

/// The [`Listen`] half of one [`MemoryNet`] name.
pub struct MemoryListen {
    conns: mpsc::UnboundedReceiver<DuplexStream>,
}

/// Summarize a listener without exposing its channel implementation.
impl std::fmt::Debug for MemoryListen {
    /// Format the number of connections waiting for acceptance.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryListen")
            .field("queued", &self.conns.len())
            .finish_non_exhaustive()
    }
}

/// Accept connections delivered to this bound memory listener.
impl Listen for MemoryListen {
    type Conn = DuplexStream;

    /// Wait for the next delivered connection.
    async fn accept(&mut self) -> io::Result<Self::Conn> {
        self.conns
            .recv()
            .await
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "this listener is unbound"))
    }
}
