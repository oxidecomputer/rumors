//! A practical guide: using `rumors` to build an eventually-consistent
//! replicated KV store.
//!
//! In this tutorial, we will start from scratch and build up a complete,
//! running application, which implements a key-value store using `rumors` to
//! synchronize state. To follow along, paste each Rust code block successively
//! into your `main.rs`.
//!
//! # Step 1: dependencies
//!
//! Start a fresh binary crate (`cargo new kv`) and add three dependencies:
//!
//! ```toml
//! [dependencies]
//! rumors = "0.1"
//! futures = "0.3"
//! rustls = "0.23"
//! tokio = { version = "1", features = [
//!     "io-util", "macros", "net", "rt-multi-thread", "time",
//! ] }
//! tokio-rustls = "0.26"
//! ```
//!
//! We drive everything with Tokio for convenience only; the crate itself is
//! [runtime-independent](crate#runtime-independence) — sessions and observers
//! are plain futures and streams.
//!
//! ```ignore
//! use std::error::Error;
//! use std::path::Path;
//! use std::sync::Arc;
//!
//! use rustls::pki_types::pem::PemObject;
//! use rustls::pki_types::{CertificateDer, PrivateKeyDer};
//! use rustls::server::WebPkiClientVerifier;
//! use rustls::{ClientConfig, RootCertStore, ServerConfig};
//! use tokio_rustls::{TlsAcceptor, TlsConnector};
//!
//! fn tls(dir: &Path) -> Result<(TlsConnector, TlsAcceptor), Box<dyn Error>> {
//!     let mut network_ca = RootCertStore::empty();
//!     for authority in CertificateDer::pem_file_iter(dir.join("ca.pem"))? {
//!         network_ca.add(authority?)?;
//!     }
//!     let network_ca = Arc::new(network_ca);
//!     let cert = CertificateDer::pem_file_iter(dir.join("cert.pem"))?
//!         .collect::<Result<Vec<_>, _>>()?;
//!     let key = PrivateKeyDer::from_pem_file(dir.join("key.pem"))?;
//!
//!     let client = ClientConfig::builder()
//!         .with_root_certificates(network_ca.clone())
//!         .with_client_auth_cert(cert.clone(), key.clone_key())?;
//!     let verifier = WebPkiClientVerifier::builder(network_ca).build()?;
//!     let server = ServerConfig::builder()
//!         .with_client_cert_verifier(verifier)
//!         .with_single_cert(cert, key)?;
//!     Ok((Arc::new(client).into(), Arc::new(server).into()))
//! }
//! ```
//!
//! ```ignore
//! use std::collections::BTreeMap;
//! use std::sync::Mutex;
//! use std::time::Duration;
//!
//! use rumors::{Peer, Rumors};
//!
//! fn session_deadline() -> tokio::time::Sleep {
//!     tokio::time::sleep(Duration::from_secs(30))
//! }
//!
//! struct State {
//!     rumors: Option<Rumors<String>>,
//!     kv: BTreeMap<String, String>,
//! }
//!
//! struct Node {
//!     state: Mutex<State>,
//! }
//!
//! impl Node {
//!     fn new() -> Self {
//!         let peer = Peer::seed().session_deadline(session_deadline);
//!         let state = State {
//!             rumors: Some(peer.into_rumors()),
//!             kv: BTreeMap::new(),
//!         };
//!         Self {
//!             state: Mutex::new(state),
//!         }
//!     }
//! }
//! ```
//!
//! ```ignore
//! use rumors::error::Mismatch;
//!
//! fn loses_to_remote(error: &rumors::Error) -> bool {
//!     let rumors::Error::Mismatch(Mismatch::Network {
//!         local_network,
//!         local_min_events,
//!         remote_network,
//!         remote_min_events,
//!         ..
//!     }) = error
//!     else {
//!         return false;
//!     };
//!     (local_min_events, local_network) < (remote_min_events, remote_network)
//! }
//! ```
//!
//! ```ignore
//! use rumors::Joined;
//! use tokio::io::{AsyncRead, AsyncWrite, BufReader};
//!
//! impl Node {
//!     async fn gossip_over(
//!         &self,
//!         stream: impl AsyncRead + AsyncWrite + Send + Unpin,
//!     ) -> Result<(), Box<dyn Error>> {
//!         let (read, write) = tokio::io::split(stream);
//!         let read = BufReader::new(read);
//!
//!         let current = self.state.lock().unwrap().rumors.clone();
//!         let (rumors, read, write) = match current {
//!             Some(rumors) => (rumors, read, write),
//!             None => {
//!                 let bootstrap =
//!                     Peer::bootstrap().session_deadline(session_deadline);
//!                 match bootstrap.join(read, write).await {
//!                     Joined::Joined { peer, connection } => {
//!                         let rumors = peer.into_rumors();
//!                         self.state.lock().unwrap().rumors =
//!                             Some(rumors.clone());
//!                         let (read, write) = connection.into_halves()?;
//!                         (rumors, read, write)
//!                     }
//!                     other => Err(format!("join failed: {other:?}"))?,
//!                 }
//!             }
//!         };
//!
//!         let result = rumors.gossip(read, write).await;
//!         if let Err(error) = &result
//!             && loses_to_remote(error)
//!         {
//!             let mut state = self.state.lock().unwrap();
//!             state.rumors.take_if(|c| c.network() == rumors.network());
//!         }
//!         result?;
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ```ignore
//! use std::net::SocketAddr;
//!
//! use rustls::pki_types::ServerName;
//! use tokio::net::TcpStream;
//!
//! impl Node {
//!     async fn connect(
//!         &self,
//!         tls: &TlsConnector,
//!         addr: SocketAddr,
//!         name: ServerName<'static>,
//!     ) -> Result<(), Box<dyn Error>> {
//!         let tcp = TcpStream::connect(addr).await?;
//!         tcp.set_nodelay(true)?;
//!         let stream = tls.connect(name, tcp).await?;
//!         self.gossip_over(stream).await
//!     }
//!
//!     async fn accept(
//!         &self,
//!         tls: &TlsAcceptor,
//!         tcp: TcpStream,
//!     ) -> Result<(), Box<dyn Error>> {
//!         tcp.set_nodelay(true)?;
//!         let stream = tls.accept(tcp).await?;
//!         self.gossip_over(stream).await
//!     }
//! }
//! ```
//!
//! ```ignore
//! use tokio::net::TcpListener;
//!
//! impl Node {
//!     async fn dial(
//!         self: Arc<Self>,
//!         tls: TlsConnector,
//!         addr: SocketAddr,
//!         name: ServerName<'static>,
//!     ) {
//!         loop {
//!             if let Err(e) = self.connect(&tls, addr, name.clone()).await {
//!                 eprintln!("connection to {addr} ended: {e}");
//!             }
//!             tokio::time::sleep(Duration::from_secs(1)).await;
//!         }
//!     }
//!
//!     async fn serve(
//!         self: Arc<Self>,
//!         tls: TlsAcceptor,
//!         listener: TcpListener,
//!     ) -> std::io::Result<()> {
//!         loop {
//!             let (tcp, addr) = listener.accept().await?;
//!             let (node, tls) = (self.clone(), tls.clone());
//!             tokio::spawn(async move {
//!                 if let Err(e) = node.accept(&tls, tcp).await {
//!                     eprintln!("connection from {addr} ended: {e}");
//!                 }
//!             });
//!         }
//!     }
//! }
//! ```
//!
//! ```ignore
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn Error>> {
//!     let (connector, acceptor) = tls(Path::new("tls"))?;
//!     let node = Arc::new(Node::new());
//!
//!     let peers = [
//!         ("peer-2.example.com", "192.0.2.2:4433"),
//!         ("peer-3.example.com", "192.0.2.3:4433"),
//!     ];
//!     for (name, addr) in peers {
//!         let name = ServerName::try_from(name)?;
//!         let addr = addr.parse()?;
//!         tokio::spawn(node.clone().dial(connector.clone(), addr, name));
//!     }
//!
//!     let listener = TcpListener::bind("0.0.0.0:4433").await?;
//!     node.serve(acceptor, listener).await?;
//!     Ok(())
//! }
//! ```
