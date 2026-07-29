//! A process-local accept/connect network for exercising the routed
//! link without sockets.
//!
//! [`MemoryNet`] is a registry of named listeners; dialing a name
//! creates one [`tokio::io::duplex`] connection and delivers one end
//! to the listener, exactly the accept/connect primitive the
//! [`routed`](crate::link::routed) adapter builds on. Everything is
//! channels and buffers, so suites run deterministically under a
//! single-poll executor, and names are plain strings, which keeps the
//! address seam honest: nothing here resembles an IP address.

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};

use tokio::io::DuplexStream;
use tokio::sync::mpsc;

use crate::link::routed::{Addr, Dial, Listen};

/// Bytes each connection buffers per direction before its writer
/// blocks on its reader.
const CONNECTION_CAPACITY: usize = 8 * 1024;

/// Dialed-but-unaccepted connections a listener holds; past it, dials
/// are refused, as a full accept backlog refuses connections on a real
/// transport.
const LISTEN_BACKLOG: usize = 64;

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

impl Addr for MemoryName {
    fn encode(&self) -> Vec<u8> {
        self.0.clone().into_bytes()
    }

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
    listeners: Arc<Mutex<HashMap<String, mpsc::Sender<DuplexStream>>>>,
}

impl MemoryNet {
    /// An empty network.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a listener at `name`, displacing any earlier binding.
    pub fn listen(&self, name: &MemoryName) -> MemoryListen {
        let (sender, receiver) = mpsc::channel(LISTEN_BACKLOG);
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
    fn registry(&self) -> std::sync::MutexGuard<'_, HashMap<String, mpsc::Sender<DuplexStream>>> {
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

impl Dial for MemoryDial {
    type Addr = MemoryName;
    type Conn = DuplexStream;

    async fn dial(&self, addr: &MemoryName) -> io::Result<Self::Conn> {
        let listener = self.net.registry().get(&addr.0).cloned().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "nothing listens at this name",
            )
        })?;
        let (dialed, accepted) = tokio::io::duplex(CONNECTION_CAPACITY);
        // A full or abandoned backlog refuses the dial outright; a
        // dial never waits on the listener's accept pace, mirroring
        // the routed adapter's requirement that opens not serialize
        // behind anyone else's progress.
        listener.try_send(accepted).map_err(|_| {
            io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "the listener's backlog is full",
            )
        })?;
        Ok(dialed)
    }
}

/// The [`Listen`] half of one [`MemoryNet`] name.
pub struct MemoryListen {
    conns: mpsc::Receiver<DuplexStream>,
}

impl Listen for MemoryListen {
    type Conn = DuplexStream;

    async fn accept(&mut self) -> io::Result<Self::Conn> {
        self.conns
            .recv()
            .await
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "the memory network is gone"))
    }
}
