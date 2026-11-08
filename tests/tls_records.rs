//! Whether a session over TLS 1.3, at the default write batch range, writes
//! records that each fit one TCP segment while its batches keep to the segment
//! size, and records within TLS's largest plaintext otherwise.
//!
//! The default write batch range ([`WRITE_BATCH_RANGE`]) starts at the most
//! plaintext one TLS 1.3 record can carry while the record fills one TCP
//! segment on a 1,500-byte Ethernet link over IPv4, and ends at the most
//! plaintext any TLS 1.3 record carries. This suite runs one session over
//! rustls, through `tokio-rustls`, on an in-process duplex stream, and parses
//! the ciphertext each side writes beneath TLS into records, from their
//! headers. It checks the claims the default rests on: every application-data
//! record of items that each fit the segment size fits one segment, and every
//! other record carries at most the largest batch, unless it carries a single
//! item larger than its ceiling.
//!
//! Whether the kernel then sends each record in one segment depends on the
//! socket and the path, which a library test cannot see.

use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use rumors::default::tls::WRITE_BATCH_RANGE;
use rumors::observe::{
    Direction, Observer, SessionInfo, SessionObserver, StreamInfo, StreamObserver,
};
use rumors::testing::CUTS_TO_GROW;
use rumors::{Rumors, default};
use rumors_testkit::bench::session::{Shape, pair};
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, DuplexStream, ReadBuf};
use tokio_rustls::{TlsAcceptor, TlsConnector};

/// Bytes each direction of the duplex stream buffers: more than the session
/// sends in one direction, so no write waits for its reader.
const CAPACITY: usize = 4 << 20;

/// The name the server's certificate is issued for.
const SERVER_NAME: &str = "localhost";

/// The TCP payload of one full-size segment on a 1,500-byte Ethernet link over
/// IPv4 with the TCP timestamps option: the MTU less 20 bytes of IPv4 header,
/// 20 of TCP header, and 12 of timestamps.
const TCP_SEGMENT_PAYLOAD: usize = 1_500 - 20 - 20 - 12;

/// The bytes of a TLS record header: content type, version, and length.
const RECORD_HEADER: usize = 5;

/// The bytes a TLS 1.3 record's payload adds to its plaintext: the inner
/// content type and the 16-byte AEAD tag.
const SEALING: usize = 1 + 16;

/// The outer content type of every TLS 1.3 record sent after the handshake:
/// application data.
const APPLICATION_DATA: u8 = 23;

/// The most plaintext one TLS record carries: rustls splits larger writes.
const MAX_FRAGMENT: usize = 1 << 14;

/// The segment size of the default write batch range: the ceiling of a batch
/// that starts with an item at most this large, while nothing waits.
const SEGMENT: usize = *WRITE_BATCH_RANGE.start();

/// The largest batch of the default write batch range: the ceiling of every
/// other batch.
const LARGEST: usize = *WRITE_BATCH_RANGE.end();

/// One TLS record as it crossed the wire.
#[derive(Clone, Copy, Debug)]
struct Record {
    /// The record's outer content type.
    content_type: u8,
    /// The record's payload length, from its header.
    payload: usize,
}

impl Record {
    /// The record's size on the wire, header included.
    fn wire(self) -> usize {
        RECORD_HEADER + self.payload
    }

    /// The plaintext an application-data record carries.
    fn plaintext(self) -> usize {
        self.payload - SEALING
    }
}

/// The records one side's ciphertext carried, parsed as it is written.
#[derive(Debug, Default)]
struct Records {
    /// The records whose headers have been read, in order.
    records: Vec<Record>,
    /// The bytes of a header still being read.
    header: Vec<u8>,
    /// The payload bytes of the current record not yet written.
    remaining: usize,
}

impl Records {
    /// Parse `bytes`, the next ciphertext written.
    fn parse(&mut self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            if self.remaining > 0 {
                let skipped = self.remaining.min(bytes.len());
                self.remaining -= skipped;
                bytes = &bytes[skipped..];
                continue;
            }
            let wanted = RECORD_HEADER - self.header.len();
            let taken = wanted.min(bytes.len());
            self.header.extend_from_slice(&bytes[..taken]);
            bytes = &bytes[taken..];
            if self.header.len() == RECORD_HEADER {
                let payload = usize::from(u16::from_be_bytes([self.header[3], self.header[4]]));
                self.records.push(Record {
                    content_type: self.header[0],
                    payload,
                });
                self.remaining = payload;
                self.header.clear();
            }
        }
    }
}

/// A transport beneath TLS that parses the records written through it.
struct Recorded {
    /// The duplex end beneath.
    inner: DuplexStream,
    /// The records written so far.
    records: Arc<Mutex<Records>>,
}

/// Fail on a write or flush that waits, passing every other outcome through.
///
/// After a wait, the session's batches grow to the largest batch whatever their
/// first item, which [`records_keep_to_the_default_batch_range`] does not
/// model; the duplex stream's capacity rules it out.
fn never_waits<T>(poll: Poll<T>) -> Poll<T> {
    assert!(
        poll.is_ready(),
        "the duplex stream never makes a write wait"
    );
    poll
}

/// Read through unchanged.
impl AsyncRead for Recorded {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

/// Write through, parsing the bytes each write accepts.
impl AsyncWrite for Recorded {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let poll = never_waits(Pin::new(&mut self.inner).poll_write(cx, buf));
        if let Poll::Ready(Ok(accepted)) = poll {
            self.records
                .lock()
                .expect("records lock")
                .parse(&buf[..accepted]);
        }
        poll
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        never_waits(Pin::new(&mut self.inner).poll_flush(cx))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// An observer that records the length of every item its sessions send, in the
/// order they are written.
///
/// Every sent stream's handler runs in the task that writes that stream's
/// items, so one shared list records the write order.
#[derive(Clone, Default)]
struct ItemLog(Arc<Mutex<Vec<usize>>>);

/// Record every session's sent items.
impl Observer for ItemLog {
    fn session(&self, _: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        Some(Box::new(self.clone()))
    }
}

/// Record every sent stream's items, and no received stream's.
impl SessionObserver for ItemLog {
    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
        (stream.direction == Direction::Sent).then(|| Box::new(self.clone()) as _)
    }
}

/// Append each item's length.
impl StreamObserver for ItemLog {
    fn message(&mut self, bytes: &[u8]) {
        self.0.lock().expect("item log lock").push(bytes.len());
    }
}

/// A server and a client configuration for TLS 1.3 alone, the client trusting
/// the server's freshly generated certificate.
///
/// The server sends no session tickets, so every record after the handshake
/// carries the session's bytes.
fn configs() -> (Arc<ServerConfig>, Arc<ClientConfig>) {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let certified = rcgen::generate_simple_self_signed(vec![SERVER_NAME.to_owned()])
        .expect("generate a certificate");
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
        certified.signing_key.serialize_der(),
    ));
    let mut server = ServerConfig::builder_with_provider(Arc::clone(&provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .expect("the ring provider supports TLS 1.3")
        .with_no_client_auth()
        .with_single_cert(vec![certified.cert.der().clone()], key)
        .expect("install the certificate");
    server.send_tls13_tickets = 0;
    let mut roots = RootCertStore::empty();
    roots
        .add(certified.cert.der().clone())
        .expect("trust the certificate");
    let client = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .expect("the ring provider supports TLS 1.3")
        .with_root_certificates(roots)
        .with_no_client_auth();
    (Arc::new(server), Arc::new(client))
}

/// What one side wrote: its items in order, and the application-data records
/// its ciphertext carried after the handshake.
struct Side {
    /// The length of each item the session wrote, in order.
    items: Vec<usize>,
    /// The records written after the handshake.
    records: Vec<Record>,
}

/// Gossip once over `stream`, a completed TLS stream, observing what the
/// session writes; `records` holds what the side wrote beneath TLS, and
/// `handshake` how many of those records the handshake wrote.
async fn gossip<S>(
    peer: Rumors<String>,
    stream: S,
    records: Arc<Mutex<Records>>,
    handshake: usize,
) -> Side
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let items = ItemLog::default();
    let peer = peer
        .try_into_peer()
        .await
        .expect("a fixture peer has one handle")
        .observe(Arc::new(items.clone()))
        .into_rumors();
    let (read, write) = tokio::io::split(stream);
    peer.gossip_once(BufReader::new(read), write)
        .await
        .expect("a session over TLS completes");
    let items = items.0.lock().expect("item log lock").clone();
    let records = records.lock().expect("records lock").records[handshake..].to_vec();
    Side { items, records }
}

/// Reconcile two disjoint sets of 5,000 messages over TLS 1.3 at the default
/// write batch range, and return what each side wrote.
fn session() -> [Side; 2] {
    let (a, b) = pair(Shape::symmetric(5_000), default::SYNC_MEMORY_BUDGET);
    let (server, client) = configs();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("build a current-thread runtime");
    runtime.block_on(async {
        let (near, far) = tokio::io::duplex(CAPACITY);
        let client_records = Arc::new(Mutex::new(Records::default()));
        let server_records = Arc::new(Mutex::new(Records::default()));
        let near = Recorded {
            inner: near,
            records: Arc::clone(&client_records),
        };
        let far = Recorded {
            inner: far,
            records: Arc::clone(&server_records),
        };
        let name = ServerName::try_from(SERVER_NAME).expect("a valid server name");
        let (client, server) = tokio::join!(
            TlsConnector::from(client).connect(name, near),
            TlsAcceptor::from(server).accept(far),
        );
        let mut client = client.expect("the client completes the handshake");
        let mut server = server.expect("the server completes the handshake");
        client.flush().await.expect("flush the client's handshake");
        server.flush().await.expect("flush the server's handshake");
        let client_handshake = client_records.lock().expect("records lock").records.len();
        let server_handshake = server_records.lock().expect("records lock").records.len();
        let a = tokio::spawn(gossip(a, client, client_records, client_handshake));
        let b = tokio::spawn(gossip(b, server, server_records, server_handshake));
        [
            a.await.expect("peer A's task completes"),
            b.await.expect("peer B's task completes"),
        ]
    })
}

/// At the default write batch range, a session's TLS 1.3 records that keep to
/// the segment size each fit one TCP segment, and its other records carry at
/// most the largest batch.
///
/// A record carrying a single item larger than its ceiling is the one exception
/// to either bound.
///
/// Each record's plaintext is the record's payload less its inner content type
/// and AEAD tag, and must be whole items the session wrote, in order: each
/// flush of the session's buffer reaches TLS as one write, which rustls seals
/// as one record while it is at most a fragment. No write beneath TLS waits
/// ([`never_waits`]), so a batch's ceiling follows from its first item and the
/// batches before it: the segment size when the item fits it and fewer than
/// [`CUTS_TO_GROW`] batches in a row before it were cut at their ceiling, and
/// the largest batch otherwise. A batch was cut at its ceiling only if the item
/// after it would not have fitted beside it under that ceiling, so only if it
/// would not have fitted under the smallest ceiling the batch could have had:
/// the segment size when its first item fits it. The test counts the records in
/// a row that may have been cut by that measure, which are at least as many as
/// were, and predicts the segment size for every record whose first item fits
/// it and that follows fewer than [`CUTS_TO_GROW`] of them: where it predicts
/// the segment size, the session must have used it, and elsewhere the largest
/// batch bounds either ceiling. A record predicted to keep to the segment size
/// must fit the segment payload on the wire, header included, or carry its one
/// item alone; any other record must carry at most the largest batch of
/// plaintext, or one item. The segment payload is derived here from the link's
/// headers, independently of the library's constant, so the test also checks
/// that derivation.
///
/// So that the bound cannot hold vacuously, the session must write many
/// records, and some must be ones the segment size ended, where the next item
/// would not have fitted: batches reach the ceiling, rather than each record
/// carrying a frame or two.
#[test]
fn records_keep_to_the_default_batch_range() {
    let sides = session();
    let mut sizes = Vec::new();
    let mut ended_by_segment = 0;
    let mut large = 0;
    for side in &sides {
        assert!(
            side.items.iter().all(|&item| item <= MAX_FRAGMENT),
            "every item fits one record's fragment, so no item spans records"
        );
        let mut items = side.items.iter().copied().peekable();
        let mut possible_cuts = 0;
        for &record in &side.records {
            assert_eq!(record.content_type, APPLICATION_DATA, "{record:?}");
            let plaintext = record.plaintext();
            let mut carried = Vec::new();
            while carried.iter().sum::<usize>() < plaintext {
                carried.push(items.next().expect("a record carries only items written"));
            }
            assert_eq!(
                carried.iter().sum::<usize>(),
                plaintext,
                "a record carries whole items: {record:?}"
            );
            let smallest = if carried[0] <= SEGMENT {
                SEGMENT
            } else {
                LARGEST
            };
            let possibly_cut = items
                .peek()
                .is_some_and(|&next| plaintext + next > smallest);
            if carried[0] <= SEGMENT && possible_cuts < CUTS_TO_GROW {
                assert!(
                    record.wire() <= TCP_SEGMENT_PAYLOAD || carried.len() == 1,
                    "a record of {} bytes on the wire carries {} items: {carried:?}",
                    record.wire(),
                    carried.len()
                );
                if possibly_cut {
                    ended_by_segment += 1;
                }
            } else {
                assert!(
                    plaintext <= LARGEST || carried.len() == 1,
                    "a record of {plaintext} bytes of plaintext carries {} items: {carried:?}",
                    carried.len()
                );
                large += 1;
            }
            possible_cuts = if possibly_cut {
                (possible_cuts + 1).min(CUTS_TO_GROW)
            } else {
                0
            };
            sizes.push(record.wire());
        }
        assert!(items.next().is_none(), "every item reached a record");
    }
    sizes.sort_unstable();
    println!(
        "{} application-data records, largest {} bytes, median {} bytes, {} ended by the segment size, {} past it",
        sizes.len(),
        sizes.last().copied().unwrap_or(0),
        sizes.get(sizes.len() / 2).copied().unwrap_or(0),
        ended_by_segment,
        large
    );
    assert!(sizes.len() > 100, "{} records", sizes.len());
    assert!(ended_by_segment > 0, "no record reached the segment size");
}
