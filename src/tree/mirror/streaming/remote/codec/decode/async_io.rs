//! Exact asynchronous input for the self-delimiting frame grammar.

use std::io::ErrorKind;

use tokio::io::{AsyncRead, AsyncReadExt};

use crate::observe::{CaptureRead, StreamObserver};

use super::super::{
    budget::RunBudget,
    error::{DecodeError, DecodeErrorKind, FramePart},
    frame::{Frame, LeafRun, ListingBuilder, Reaction, WireFrame, listing_entry_len},
    signal::{Signal, Speaker, WireSignal},
};
use super::{
    OpenerItem, check_arity, decode_signal, frame_arity, head_error, listing_issue, opener_item,
    query_listing, run_head,
};
use crate::tree::{
    mirror::cbor::{self, HeadError, HeadReadError},
    mirror::framing::{read_payload, resume_payload},
    typed::{Hash, hash::MERKLE_HASH_LEN},
};

/// Bytes a listed child occupies when its radix takes a one-byte key
/// head: the fewest any canonical entry can occupy.
const NARROW_ENTRY_LEN: usize = listing_entry_len(0);

/// Bytes a listed child occupies when its radix takes a two-byte key
/// head: every entry after the first such radix, keys being strictly
/// ascending.
const WIDE_ENTRY_LEN: usize = listing_entry_len(u8::MAX);

/// The smallest radix whose key head takes two bytes.
const FIRST_WIDE_RADIX: u8 = 24;

/// Bytes a canonical frame opener occupies: the array head of a two- or
/// three-item frame, then the stream and state items.
const OPENER_LEN: usize = cbor::head_len(3) + WireSignal::ENCODED_LEN;

/// Async frame reader over one speaker's transport direction.
///
/// EOF before a frame's array head is a clean direction close and returns
/// `None`. Once that head arrives, a missing component is a contextual
/// truncation. Variable bodies are read at their declared size and
/// validated exactly once, with supply bodies additionally held to the
/// session's run budget before they are buffered
/// ([`DecodeErrorKind::OverbatchedRun`]).
///
/// # Reads
///
/// Every transport read takes only bytes the frame grammar guarantees to
/// exist given what has already been parsed, so a valid frame is consumed
/// exactly and no byte of the next frame is touched. Within that bound a
/// read takes as many bytes as it can: a frame's opener — its array head
/// and its stream and state items, one byte each — arrives in one read,
/// and a listing's entries arrive in bulk reads sized to the fewest bytes
/// the remaining entries can occupy. How
/// bytes are batched never changes how a frame is judged: components are
/// validated in wire order over the bytes that arrived, so a defect is
/// classified exactly as a reader fetching one head at a time would
/// classify it, and a frame the transport cuts short is truncated at the
/// same component.
pub struct FrameRead<R> {
    speaker: Speaker,
    /// The session's negotiated run budget, enforced on every supply frame
    /// this direction delivers.
    budget: RunBudget,
    read: R,
    /// The directed stream's observer, if any: handed each accepted
    /// frame's exact wire bytes, and costing one branch when absent.
    observe: Option<Box<dyn StreamObserver>>,
    /// Scratch for a listing's bulk reads, retained across frames so a
    /// query frame allocates only when it outgrows every earlier one.
    listing: Vec<u8>,
}

impl<R> FrameRead<R> {
    /// Bind `read` to the direction spoken by `speaker`, enforcing `budget`
    /// on the supply frames it delivers.
    pub fn new(speaker: Speaker, budget: RunBudget, read: R) -> Self {
        Self {
            speaker,
            budget,
            read,
            observe: None,
            listing: Vec::new(),
        }
    }

    /// Deliver every accepted frame to `observe`, when one is attached.
    pub fn observed(mut self, observe: Option<Box<dyn StreamObserver>>) -> Self {
        self.observe = observe;
        self
    }

    /// Recover the transport half. The reader holds no transport bytes
    /// between frames (every read is exact), so between frames the half
    /// rests exactly at a frame boundary.
    pub fn into_inner(self) -> R {
        self.read
    }
}

impl<R: AsyncRead + Unpin> FrameRead<R> {
    /// Read and decode one frame without consuming any byte of the next.
    ///
    /// # Cancel safety
    ///
    /// Not cancel safe. A dropped `frame` future may already have consumed
    /// part of a frame — the exact reads do not give bytes back — leaving
    /// the direction mid-frame, where the next call would parse body bytes
    /// as a frame head. Either retain the in-flight future across polls
    /// until it resolves, or read nothing further from this direction after
    /// a cancellation.
    pub async fn frame(&mut self) -> Result<Option<WireFrame>, DecodeError> {
        match &mut self.observe {
            None => read_frame(&mut self.read, self.speaker, self.budget, &mut self.listing).await,
            Some(observe) => {
                // Retain the consumed bytes so the observer sees the
                // frame's true wire spelling, never a re-encoding. Only
                // an accepted whole frame is delivered: a clean close
                // consumed nothing, and an error leaves a fragment.
                let mut capture = CaptureRead::new(&mut self.read);
                let result =
                    read_frame(&mut capture, self.speaker, self.budget, &mut self.listing).await;
                if let Ok(Some(_)) = &result {
                    observe.message(capture.bytes());
                }
                result
            }
        }
    }
}

/// Read and decode one frame from `read`; the contract is
/// [`FrameRead::frame`]'s.
async fn read_frame<R: AsyncRead + Unpin>(
    read: &mut R,
    speaker: Speaker,
    budget: RunBudget,
    listing: &mut Vec<u8>,
) -> Result<Option<WireFrame>, DecodeError> {
    let direction = |kind| DecodeError::direction(speaker, kind);
    let mut exact = Exact { read };
    // Every frame opens with its array head, its stream item, and its
    // state item, each a one-byte head when canonical, so one read may
    // take all three. A close before the first byte is the clean end of
    // the direction; anything shorter after it is judged in wire order
    // below, each item taking what the read fetched ahead of it.
    let mut opener = [0u8; OPENER_LEN];
    let arrived = exact.fill(&mut opener).await;
    if arrived.filled == 0 {
        return match arrived.failure {
            None => Ok(None),
            Some(source) => Err(direction(DecodeErrorKind::Read {
                part: FramePart::FrameHead,
                source,
            })),
        };
    }
    let (arity, index, state) = {
        let mut head = Pending::new(FramePart::FrameHead);
        head.take(&opener[..arrived.filled]);
        let (head, rest) = exact.head(&mut head).await.map_err(direction)?;
        let arity = frame_arity(head).map_err(direction)?;
        let mut stream = Pending::new(FramePart::Signal);
        stream.take(rest);
        let (stream_head, rest) = exact.head(&mut stream).await.map_err(direction)?;
        let index = opener_item(stream_head, OpenerItem::Stream).map_err(direction)?;
        let mut state = Pending::new(FramePart::Signal);
        state.take(rest);
        let (state_head, _) = exact.head(&mut state).await.map_err(direction)?;
        let state = opener_item(state_head, OpenerItem::State).map_err(direction)?;
        (arity, index, state)
    };
    let (stream, signal) = decode_signal(speaker, index, state)?;
    let mut decoder = AsyncFrameDecoder {
        exact,
        budget,
        listing,
    };
    let frame = async {
        check_arity(signal, arity)?;
        decoder.body(signal).await
    }
    .await
    .map_err(|kind| DecodeError::stream(speaker, stream, kind))?;
    Ok(Some((stream, frame)))
}

/// What one transport read attempt delivered: the bytes filled before the
/// buffer was full, the transport closed, or it failed.
struct Arrived {
    filled: usize,
    /// The transport's failure, if the read ended in one rather than in a
    /// full buffer or a close.
    failure: Option<std::io::Error>,
}

impl Arrived {
    /// Type a short delivery by the part left incomplete: a close is a
    /// truncation, a failure a read error.
    fn short(self, part: FramePart) -> DecodeErrorKind {
        match self.failure {
            Some(source) => DecodeErrorKind::Read { part, source },
            None => DecodeErrorKind::Truncated {
                missing: part,
                source: ErrorKind::UnexpectedEof.into(),
            },
        }
    }
}

/// A head being assembled from bytes that may have arrived ahead of it.
struct Pending {
    bytes: [u8; cbor::MAX_HEAD_LEN],
    have: usize,
    part: FramePart,
}

impl Pending {
    fn new(part: FramePart) -> Self {
        Self {
            bytes: [0; cbor::MAX_HEAD_LEN],
            have: 0,
            part,
        }
    }

    /// Accept bytes already fetched that begin this head.
    fn take(&mut self, bytes: &[u8]) {
        debug_assert!(self.have == 0 && bytes.len() <= self.bytes.len());
        self.bytes[..bytes.len()].copy_from_slice(bytes);
        self.have = bytes.len();
    }
}

/// Exact reads over one frame: every read takes only bytes the grammar
/// guarantees to exist given what has already been parsed.
struct Exact<'a, R> {
    read: &'a mut R,
}

impl<'a, R: AsyncRead + Unpin> Exact<'a, R> {
    /// Read into `buf` until it is full, the transport closes, or it
    /// fails, reporting what arrived. The caller judges the bytes in wire
    /// order.
    async fn fill(&mut self, buf: &mut [u8]) -> Arrived {
        let mut filled = 0;
        while filled < buf.len() {
            match self.read.read(&mut buf[filled..]).await {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(source) => {
                    return Arrived {
                        filled,
                        failure: Some(source),
                    };
                }
            }
        }
        Arrived {
            filled,
            failure: None,
        }
    }

    /// Append up to `want` bytes to `scratch` likewise.
    async fn fill_vec(&mut self, scratch: &mut Vec<u8>, want: usize) -> Arrived {
        let start = scratch.len();
        scratch.resize(start + want, 0);
        let arrived = self.fill(&mut scratch[start..]).await;
        scratch.truncate(start + arrived.filled);
        arrived
    }

    /// Fill `buf` exactly, attributing a short delivery to `part`.
    async fn fill_exact(&mut self, buf: &mut [u8], part: FramePart) -> Result<(), DecodeErrorKind> {
        let arrived = self.fill(buf).await;
        if arrived.filled < buf.len() {
            return Err(arrived.short(part));
        }
        Ok(())
    }

    /// Complete and parse one canonical head, returning it with any bytes
    /// fetched ahead of it that belong to the next item.
    ///
    /// The initial byte is read if it has not arrived; its extension is
    /// read only as far as the bytes in hand fall short of it. Bytes in
    /// hand past the head are the next item's.
    async fn head<'p>(
        &mut self,
        pending: &'p mut Pending,
    ) -> Result<(cbor::Head, &'p [u8]), DecodeErrorKind> {
        let part = pending.part;
        if pending.have == 0 {
            self.fill_exact(&mut pending.bytes[..1], part).await?;
            pending.have = 1;
        }
        let extension = cbor::extension_len(pending.bytes[0]).map_err(|e| head_error(part, e))?;
        let len = 1 + extension;
        if pending.have < len {
            self.fill_exact(&mut pending.bytes[pending.have..len], part)
                .await?;
            pending.have = len;
        }
        let head = parse_head(&pending.bytes[..len], part)?;
        Ok((head, &pending.bytes[len..pending.have]))
    }

    /// Read one canonical head with nothing fetched ahead of it.
    async fn fresh_head(&mut self, part: FramePart) -> Result<cbor::Head, DecodeErrorKind> {
        let mut pending = Pending::new(part);
        let (head, _) = self.head(&mut pending).await?;
        Ok(head)
    }
}

/// Parse a complete head's bytes, typing a deterministic-contract
/// violation by the part it interrupted.
fn parse_head(bytes: &[u8], part: FramePart) -> Result<cbor::Head, DecodeErrorKind> {
    let mut input = bytes;
    cbor::read_head(&mut input).map_err(|e| head_error(part, HeadReadError::Malformed(e)))
}

/// Reads a body after its signal has established the frame grammar.
struct AsyncFrameDecoder<'a, R> {
    exact: Exact<'a, R>,
    /// The session's run budget, gating supply-body buffering.
    budget: RunBudget,
    /// Scratch for a listing's bulk reads.
    listing: &'a mut Vec<u8>,
}

/// How far one listing entry parsed from the bytes in hand.
enum Entry {
    /// The entry is complete and valid: its radix, digest, and width.
    Complete(u8, [u8; MERKLE_HASH_LEN], usize),
    /// Every component that arrived whole is valid; the rest has not
    /// arrived.
    ///
    /// At least `need` more bytes are owed before it can: the rest of the
    /// entry once its key head is whole (its width is then known
    /// exactly), otherwise the rest of that head.
    Incomplete { need: usize },
}

impl<'a, R: AsyncRead + Unpin> AsyncFrameDecoder<'a, R> {
    async fn body(&mut self, signal: Signal) -> Result<Frame, DecodeErrorKind> {
        let frame = match signal {
            Signal::Match(flow) => Frame::Reaction(Reaction::Match, flow),
            Signal::QueryEmpty(flow) => Frame::Reaction(Reaction::Query(Vec::new()), flow),
            Signal::Query(flow) => Frame::Reaction(Reaction::Query(self.query().await?), flow),
            Signal::Supply(flow) => Frame::Reaction(Reaction::Supply(self.supply().await?), flow),
            Signal::End(end) => Frame::End(end),
        };
        Ok(frame)
    }

    /// Read a listing map: its head, then its entries in bulk.
    ///
    /// Each bulk read takes the fewest bytes the entries still owed can
    /// occupy — the narrow entry width per entry until a radix of 24 or
    /// more has been seen, the wide width thereafter — so a canonical
    /// listing is consumed exactly, in a bounded number of reads
    /// independent of its fan. Entries are judged in wire order as their
    /// bytes arrive; a defect is reported before any later entry is
    /// examined, and a delivery that ends mid-entry is a truncation only
    /// once everything whole before it has passed.
    async fn query(&mut self) -> Result<Vec<(u8, Hash)>, DecodeErrorKind> {
        const PART: FramePart = FramePart::QueryChildren;
        let head = self.exact.fresh_head(PART).await?;
        let mut listing = query_listing(head)?;
        let count = usize::try_from(head.value).expect("the listing fits the radix space");
        let scratch = &mut *self.listing;
        scratch.clear();
        let mut parsed = 0;
        let mut entries = 0;
        let mut wide = false;
        // Bytes the entry cut short by the last read still needs; the
        // first read owes nothing beyond the entries' minimum.
        let mut need = 0;
        while entries < count {
            let entry_min = if wide {
                WIDE_ENTRY_LEN
            } else {
                NARROW_ENTRY_LEN
            };
            let buffered = scratch.len() - parsed;
            let owed = ((count - entries) * entry_min)
                .saturating_sub(buffered)
                .max(need);
            let arrived = self.exact.fill_vec(scratch, owed).await;
            loop {
                match parse_entry(&scratch[parsed..], &listing)? {
                    Entry::Complete(radix, digest, width) => {
                        listing.entry(radix, digest);
                        parsed += width;
                        entries += 1;
                        wide |= radix >= FIRST_WIDE_RADIX;
                        if entries == count {
                            break;
                        }
                    }
                    Entry::Incomplete { need: more } => {
                        need = more;
                        break;
                    }
                }
            }
            if entries < count && arrived.filled < owed {
                return Err(arrived.short(PART));
            }
        }
        debug_assert_eq!(
            parsed,
            scratch.len(),
            "a canonical listing's bulk reads take exactly its entries' bytes"
        );
        Ok(listing.finish())
    }

    async fn supply(&mut self) -> Result<LeafRun, DecodeErrorKind> {
        let tag = self.exact.fresh_head(FramePart::SupplyLength).await?;
        let body = self.exact.fresh_head(FramePart::SupplyLength).await?;
        let len = run_head(tag, body)?;
        let run = if self.budget.covers(len) {
            read_payload(&mut *self.exact.read, len)
                .await
                .map_err(|source| classify(FramePart::SupplyRun, source))?
        } else {
            // The frame outsizes the budget the peer's encoder flushes
            // within, so the one shape an honest encoder can still have
            // produced is a single record spanning the whole body (the
            // minimum-one-record overhang). That is decidable from the
            // first record's heads alone, so nothing beyond them is
            // read until the frame is known legal: a violating frame is
            // rejected before its body is buffered, keeping the decode
            // inside the memory envelope the budget priced. A body too
            // short to hold a record's heads cannot be a lone record and
            // is rejected on the declared length alone.
            let budget = self.budget;
            let overbatched = move || DecodeErrorKind::OverbatchedRun {
                declared: super::super::budget::SUPPLY_FRAME_OVERHEAD.saturating_add(len),
                budget: budget.bytes(),
            };
            if len < super::super::frame::RECORD_TAG_LEN + 1 {
                return Err(overbatched());
            }
            let Some((prefix, record)) = self.record_prefix().await? else {
                return Err(overbatched());
            };
            if !super::super::frame::lone_record_spans(len, record) {
                return Err(overbatched());
            }
            // Legal lone record: resume the body read behind the heads
            // already consumed, in the same single buffer.
            resume_payload(&mut *self.exact.read, prefix, len)
                .await
                .map_err(|source| classify(FramePart::SupplyRun, source))?
        };
        Ok(LeafRun::from_encoded(run)?)
    }

    /// Read the first record's heads inside an over-budget run, returning
    /// the exact bytes consumed and the record content length.
    ///
    /// `None` when they are not a record's heads: over budget, the
    /// distinction from malformed is moot — either way the frame is not
    /// the legal overhang.
    async fn record_prefix(&mut self) -> Result<Option<(Vec<u8>, u64)>, DecodeErrorKind> {
        let mut prefix = Vec::new();
        let tag = self.exact.fresh_head(FramePart::SupplyRun).await?;
        cbor::write_head(&mut prefix, tag.major, tag.value);
        if tag.major != cbor::MAJOR_TAG || tag.value != cbor::TAG_CBOR_SEQUENCE {
            return Ok(None);
        }
        let body = self.exact.fresh_head(FramePart::SupplyRun).await?;
        cbor::write_head(&mut prefix, body.major, body.value);
        if body.major != cbor::MAJOR_BSTR {
            return Ok(None);
        }
        Ok(Some((prefix, body.value)))
    }
}

/// Parse one listing entry from the front of `bytes`, judging each
/// component as soon as it is whole.
///
/// The key head is judged against the listing's order, the value head as
/// a digest's, then the digest bytes. Judging records nothing, so an
/// entry cut short by one delivery is judged again, identically, when
/// the next completes it.
fn parse_entry(bytes: &[u8], listing: &ListingBuilder) -> Result<Entry, DecodeErrorKind> {
    let mut rest = bytes;
    let Some(key) = partial_head(&mut rest)? else {
        // The key head's own width is known from its initial byte when
        // one arrived; with none, one byte is owed.
        let need = match bytes.first() {
            Some(&initial) => {
                1 + cbor::extension_len(initial)
                    .map_err(|e| head_error(FramePart::QueryChildren, e))?
                    - bytes.len()
            }
            None => 1,
        };
        return Ok(Entry::Incomplete { need });
    };
    let radix = listing.key(key).map_err(listing_issue)?;
    let need = listing_entry_len(radix).saturating_sub(bytes.len());
    let Some(value) = partial_head(&mut rest)? else {
        return Ok(Entry::Incomplete { need });
    };
    ListingBuilder::value_head(value).map_err(listing_issue)?;
    if rest.len() < MERKLE_HASH_LEN {
        return Ok(Entry::Incomplete { need });
    }
    let (digest, rest) = rest.split_at(MERKLE_HASH_LEN);
    Ok(Entry::Complete(
        radix,
        digest.try_into().expect("split at the digest width"),
        bytes.len() - rest.len(),
    ))
}

/// Parse one listing head off `input` if it has arrived whole: `None`
/// when the bytes end inside it, a listing defect when it is present but
/// not canonical.
fn partial_head(input: &mut &[u8]) -> Result<Option<cbor::Head>, DecodeErrorKind> {
    match cbor::read_head(input) {
        Ok(head) => Ok(Some(head)),
        Err(HeadError::Truncated) => Ok(None),
        Err(error) => Err(listing_issue(super::super::frame::ListingIssue::Head(
            error,
        ))),
    }
}

/// Type an I/O failure by the frame part it interrupted: end-of-stream is a
/// contextual truncation, anything else a plain read failure.
fn classify(part: FramePart, source: std::io::Error) -> DecodeErrorKind {
    match source.kind() {
        ErrorKind::UnexpectedEof => DecodeErrorKind::Truncated {
            missing: part,
            source,
        },
        _ => DecodeErrorKind::Read { part, source },
    }
}
