//! The stack a measured session runs over, and the socket operations each
//! side makes beneath it.
//!
//! Each side turns its TCP connection into the two halves the library takes
//! ([`establish`]), plain TCP or TLS 1.3 as the [`Transport`] point says,
//! set up as the library advises: Nagle's algorithm off, and the read half
//! wrapped in a `BufReader`.
//!
//! A [`Counting`] wrapper sits directly on the socket, beneath TLS and the
//! read buffer, so its counts are the operations the socket itself sees: on a
//! plain TCP socket each counted write is one `write` or `writev` system
//! call that moved bytes, and each counted read one `read` that delivered
//! bytes or end of stream. A system call that finds the socket not ready
//! (`EAGAIN`) moves nothing and is not counted. The wrapper forwards
//! vectored writes, so a message the library writes as one vectored write,
//! as it writes any message at least its write buffer's size, stays one
//! system call.
//!
//! The tally starts once the connection is set up, so it counts the
//! session alone, provided no setup read takes a byte of the peer's
//! session. A read through a buffering layer takes whatever has arrived,
//! up to the layer's buffer, so the setup is ordered to leave nothing of a
//! session to take:
//!
//! - On a warm connection, [`establish`] carries the warm-up bytes each way
//!   beneath the read buffer, side B first. Over plain TCP it reads the
//!   socket itself, whose reads take only the bytes asked for. TLS reads
//!   records ahead of what is asked, so once side B has read all of side
//!   A's warm-up bytes, it writes one more byte beneath TLS, and side A
//!   reads that byte from the socket itself before it starts its session.
//!   Side B's last warm-up read ends with side A's warm-up bytes, since side
//!   A sends nothing more until that byte arrives; side A's last read
//!   through TLS ends with side B's warm-up bytes, since side B writes
//!   nothing more until it has read them.
//! - On a fresh TLS connection, side B may send its first session records
//!   right behind its last handshake message, and side A's last handshake
//!   read can take them, so they count before the tally starts. Over plain
//!   TCP, nothing is read before the session.
//!
//! # TLS
//!
//! Side A serves TLS with a self-signed certificate it generates for the
//! process; side B accepts any certificate, but still verifies the
//! handshake's signatures with the provider's algorithms, so the handshake
//! does the cryptographic work a verifying client does. Only TLS 1.3 is
//! offered. The handshake runs before the session's clock starts on side
//! A, and within side B's connect time.

use std::io::{self, IoSlice};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll};

use anyhow::{Context as _, Result};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, ServerConfig, SignatureScheme};
use serde::{Deserialize, Serialize};
use tokio::io::{
    AsyncBufRead, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, ReadBuf,
};
use tokio::net::TcpStream;
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::point::Transport;

/// The name side B asks for and side A's certificate carries.
pub const SERVER_NAME: &str = "rumors-netperf";

/// Which end of the connection a side is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    /// The side that accepted the connection: side A, the TLS server.
    Accepted,
    /// The side that connected: side B, the TLS client.
    Connected,
}

/// The operations one side's socket saw, and the bytes they moved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocketCounts {
    /// Bytes written to the socket.
    pub sent_bytes: u64,
    /// Bytes read from the socket.
    pub received_bytes: u64,
    /// Writes the socket accepted bytes from: on plain TCP, one `write` or
    /// `writev` system call each. A call that finds the socket full is not
    /// counted.
    pub writes: u64,
    /// Reads that delivered bytes or end of stream: on plain TCP, one
    /// `read` system call each. A call that finds nothing to read is not
    /// counted.
    pub reads: u64,
}

/// The shared tally a [`Counting`] wrapper adds to.
#[derive(Debug, Default)]
pub struct Tally {
    /// Bytes written.
    sent_bytes: AtomicU64,
    /// Bytes read.
    received_bytes: AtomicU64,
    /// Writes that accepted bytes.
    writes: AtomicU64,
    /// Reads that delivered bytes or end of stream.
    reads: AtomicU64,
}

impl Tally {
    /// The counts so far.
    pub fn counts(&self) -> SocketCounts {
        SocketCounts {
            sent_bytes: self.sent_bytes.load(Ordering::Relaxed),
            received_bytes: self.received_bytes.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
            reads: self.reads.load(Ordering::Relaxed),
        }
    }

    /// Starts every count again from zero.
    pub fn reset(&self) {
        for count in [
            &self.sent_bytes,
            &self.received_bytes,
            &self.writes,
            &self.reads,
        ] {
            count.store(0, Ordering::Relaxed);
        }
    }
}

/// A socket, or one half of it, that adds every operation it completes to a
/// shared [`Tally`].
#[derive(Debug)]
pub struct Counting<T> {
    /// The wrapped socket or half.
    inner: T,
    /// Where the counts go.
    tally: Arc<Tally>,
}

impl<T> Counting<T> {
    /// Wraps `inner`, counting into `tally`.
    pub fn new(inner: T, tally: Arc<Tally>) -> Self {
        Self { inner, tally }
    }

    /// Counts one write's outcome, passing it through.
    fn wrote(&self, poll: Poll<io::Result<usize>>) -> Poll<io::Result<usize>> {
        if let Poll::Ready(Ok(written)) = &poll {
            self.tally.writes.fetch_add(1, Ordering::Relaxed);
            self.tally
                .sent_bytes
                .fetch_add(*written as u64, Ordering::Relaxed);
        }
        poll
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for Counting<T> {
    /// Reads through, counting the read and its bytes.
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        let poll = Pin::new(&mut self.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = &poll {
            let read = buf.filled().len() - before;
            self.tally.reads.fetch_add(1, Ordering::Relaxed);
            self.tally
                .received_bytes
                .fetch_add(read as u64, Ordering::Relaxed);
        }
        poll
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for Counting<T> {
    /// Writes through, counting the write and its bytes.
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let poll = Pin::new(&mut self.inner).poll_write(cx, buf);
        self.wrote(poll)
    }

    /// Writes the slices through as one vectored write, counting it once.
    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        let poll = Pin::new(&mut self.inner).poll_write_vectored(cx, bufs);
        self.wrote(poll)
    }

    /// Whether the wrapped socket takes vectored writes whole.
    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    /// Flushes through.
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    /// Shuts down through.
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// A read half as the library takes it: buffered.
pub type ReadHalf = Box<dyn AsyncBufRead + Unpin + Send>;

/// A write half as the library takes it.
pub type WriteHalf = Box<dyn AsyncWrite + Unpin + Send>;

/// Turns `stream` into the halves of `transport`'s stack, and the tally of
/// what its socket sees from here on.
///
/// With `warm_bytes`, the connection first carries that many bytes each
/// way, side B first, ordered as the module docs describe, so the tally
/// counts nothing of the warm-up and the session alone after it.
///
/// # Errors
///
/// If disabling Nagle's algorithm fails, the TLS handshake does, or the
/// warm-up does.
pub async fn establish(
    mut stream: TcpStream,
    transport: Transport,
    role: Role,
    warm_bytes: Option<u64>,
) -> Result<(ReadHalf, WriteHalf, Arc<Tally>)> {
    stream
        .set_nodelay(true)
        .context("disabling Nagle's algorithm")?;
    let tally = Arc::new(Tally::default());
    let (read, write): (Box<dyn AsyncRead + Unpin + Send>, WriteHalf) = if transport.tls {
        let socket = Counting::new(stream, Arc::clone(&tally));
        match role {
            Role::Accepted => {
                let mut tls = TlsAcceptor::from(server_config()?)
                    .accept(socket)
                    .await
                    .context("serving the TLS handshake")?;
                if let Some(bytes) = warm_bytes {
                    warm_up(&mut tls, role, bytes).await?;
                    beneath_tls_warmed(&mut tls.get_mut().0, role).await?;
                }
                let (read, write) = tokio::io::split(tls);
                (Box::new(read), Box::new(write))
            }
            Role::Connected => {
                let name = ServerName::try_from(SERVER_NAME).expect("the server name is valid");
                let mut tls = TlsConnector::from(client_config())
                    .connect(name, socket)
                    .await
                    .context("completing the TLS handshake")?;
                if let Some(bytes) = warm_bytes {
                    warm_up(&mut tls, role, bytes).await?;
                    beneath_tls_warmed(&mut tls.get_mut().0, role).await?;
                }
                let (read, write) = tokio::io::split(tls);
                (Box::new(read), Box::new(write))
            }
        }
    } else {
        if let Some(bytes) = warm_bytes {
            warm_up(&mut stream, role, bytes).await?;
        }
        let (read, write) = stream.into_split();
        (
            Box::new(Counting::new(read, Arc::clone(&tally))),
            Box::new(Counting::new(write, Arc::clone(&tally))),
        )
    };
    let read: ReadHalf = Box::new(BufReader::new(read));
    tally.reset();
    Ok((read, write, tally))
}

/// The buffer the warm-up writes from and reads into.
const WARM_CHUNK: usize = 1 << 16;

/// The byte side B writes beneath TLS once it has read all of side A's
/// warm-up bytes.
const WARMED: u8 = b'W';

/// Sends `bytes` each way through `stack`, side B first, so the
/// connection's congestion window is open when the session starts.
///
/// Each side reads exactly the peer's warm-up bytes: the side that reads
/// first writes only after it has read them, and the side that reads last
/// receives nothing after them until the setup ends.
async fn warm_up(
    stack: &mut (impl AsyncRead + AsyncWrite + Unpin),
    role: Role,
    bytes: u64,
) -> Result<()> {
    let exchange = async {
        match role {
            Role::Connected => {
                push_bytes(stack, bytes).await?;
                pull_bytes(stack, bytes).await
            }
            Role::Accepted => {
                pull_bytes(stack, bytes).await?;
                push_bytes(stack, bytes).await
            }
        }
    };
    exchange.await.context("warming the connection")
}

/// Ends a TLS warm-up beneath TLS: side B writes [`WARMED`] to `socket`,
/// and side A reads it there, exactly, before either starts its session.
///
/// Side B writes it only after its last read through TLS, and side A
/// starts its session only once it has read it, so neither TLS layer can
/// read ahead into the peer's session. Neither TLS layer sees the byte:
/// side B's has nothing queued to send after its flushed warm-up bytes,
/// and side A's reads nothing more until its session starts.
async fn beneath_tls_warmed(
    socket: &mut (impl AsyncRead + AsyncWrite + Unpin),
    role: Role,
) -> Result<()> {
    match role {
        Role::Connected => {
            socket.write_all(&[WARMED]).await?;
            socket.flush().await?;
        }
        Role::Accepted => {
            let mut byte = [0];
            socket.read_exact(&mut byte).await?;
            anyhow::ensure!(
                byte == [WARMED],
                "side B ended the warm-up with {byte:?}, not {WARMED}"
            );
        }
    }
    Ok(())
}

/// Writes `bytes` zero bytes and flushes.
async fn push_bytes(write: &mut (impl AsyncWrite + Unpin), bytes: u64) -> io::Result<()> {
    let chunk = [0u8; WARM_CHUNK];
    let mut left = bytes;
    while left > 0 {
        let n = left.min(WARM_CHUNK as u64) as usize;
        write.write_all(&chunk[..n]).await?;
        left -= n as u64;
    }
    write.flush().await
}

/// Reads and discards exactly `bytes` bytes.
async fn pull_bytes(read: &mut (impl AsyncRead + Unpin), bytes: u64) -> io::Result<()> {
    let mut sink = vec![0u8; WARM_CHUNK];
    let mut left = bytes;
    while left > 0 {
        let n = left.min(WARM_CHUNK as u64) as usize;
        read.read_exact(&mut sink[..n]).await?;
        left -= n as u64;
    }
    Ok(())
}

/// The crypto provider every TLS session uses.
fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// The server's TLS configuration, with a certificate generated once per
/// process.
pub fn server_config() -> Result<Arc<ServerConfig>> {
    static CONFIG: OnceLock<Arc<ServerConfig>> = OnceLock::new();
    if let Some(config) = CONFIG.get() {
        return Ok(Arc::clone(config));
    }
    let certified = rcgen::generate_simple_self_signed(vec![SERVER_NAME.to_string()])
        .context("generating the TLS certificate")?;
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
        certified.signing_key.serialize_der(),
    ));
    let config = ServerConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .context("choosing TLS 1.3")?
        .with_no_client_auth()
        .with_single_cert(vec![certified.cert.der().clone()], key)
        .context("installing the TLS certificate")?;
    Ok(Arc::clone(CONFIG.get_or_init(|| Arc::new(config))))
}

/// The client's TLS configuration: any certificate, verified signatures.
pub fn client_config() -> Arc<ClientConfig> {
    static CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    Arc::clone(CONFIG.get_or_init(|| {
        let provider = provider();
        let config = ClientConfig::builder_with_provider(Arc::clone(&provider))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .expect("the ring provider supports TLS 1.3")
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AnyCertificate(provider)))
            .with_no_client_auth();
        Arc::new(config)
    }))
}

/// A certificate verifier that trusts any certificate but checks the
/// handshake's signatures, so a measured handshake does a verifying
/// client's signature work without a certificate authority.
#[derive(Debug)]
struct AnyCertificate(Arc<CryptoProvider>);

impl ServerCertVerifier for AnyCertificate {
    /// Accepts the certificate: the tool measures, it does not authenticate.
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    /// Verifies a TLS 1.2 handshake signature with the provider.
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    /// Verifies a TLS 1.3 handshake signature with the provider.
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    /// The provider's signature schemes.
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

#[cfg(test)]
mod tests;
