//! The fuzz-fit wasm guest: `before`'s public operations behind a C-ABI
//! register machine.
//!
//! The host (the `fuzzfit-harness` crate) drives this module under wasmtime
//! fuel metering: every export prefixed `ff_` is callable with u32/i64
//! scalars, values live in a register file inside the guest, and bulk bytes
//! (canonical encodings, display text) cross the boundary through a staging
//! buffer in linear memory. Each *measured* export performs exactly one
//! public `before` operation on registers, so the fuel consumed by one call
//! is the instruction count of one public operation (plus a constant
//! register-machine dispatch overhead the calibration's intercept absorbs).
//!
//! Contract with the harness (which is the only caller):
//!
//! - Registers are dense indices into a growable file; a slot holds a
//!   `Version`, `Party`, `Clock`, `Rank`, or `Span`. Ops that consume an operand
//!   (`join`, `without`, fold drains) take it out of its slot — the register
//!   file is linear exactly where the API is linear, so a generator that
//!   replays a valid program here cannot alias a `Party`.
//! - Every export returns `0` for success and a negative code for a misuse
//!   (missing register, wrong type, operation error). The harness treats any
//!   nonzero return as a harness bug and aborts the case: its generators
//!   construct programs that are valid by construction.
//! - Staging (`ff_stage_prepare` plus a host-side memory write,
//!   `ff_stage_ptr` plus a host-side read) executes no measured code; the
//!   measured kernels (`*_decode`, `*_encode`, `*_display`, `*_fromstr`)
//!   then read or write the staged bytes inside the fuel window.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt::Write as _;

use before::causally::{Dominance, Endpoint, Placement, Span};
use before::{Clock, Party, Rank, Version};

/// One register-file slot: any value the public surface produces.
enum Val {
    V(Version),
    P(Party),
    C(Clock),
    R(Rank),
    S(Span<'static>),
}

// clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
// fallback-TLS lowering (illumos among the gate's targets) and denies
// initializers that already sit in `const` blocks; the allow keeps
// `-D warnings` honest on every platform the gate runs.
thread_local! {
    /// The register file. wasm32-unknown-unknown is single-threaded, so a
    /// thread-local `RefCell` is an uncontended, unsafe-free global.
    #[allow(clippy::missing_const_for_thread_local)]
    static REGS: RefCell<Vec<Option<Val>>> = const { RefCell::new(Vec::new()) };
    /// The staging buffer for bulk byte transfer across the ABI.
    #[allow(clippy::missing_const_for_thread_local)]
    static STAGE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Success.
const OK: i32 = 0;
/// A register was empty or held the wrong type.
const ERR_REG: i32 = -1;
/// A public operation reported an error (e.g. joining overlapping parties).
const ERR_OP: i32 = -2;
/// Staged bytes failed to decode or parse.
const ERR_CODEC: i32 = -3;

/// Store `val` into register `dst`, growing the file as needed.
fn put(dst: u32, val: Val) {
    REGS.with_borrow_mut(|regs| {
        let dst = dst as usize;
        if regs.len() <= dst {
            regs.resize_with(dst + 1, || None);
        }
        regs[dst] = Some(val);
    });
}

/// Move the value out of register `src`.
fn take(src: u32) -> Option<Val> {
    REGS.with_borrow_mut(|regs| regs.get_mut(src as usize).and_then(Option::take))
}

/// Move a `Version` out of `src`, or fail with `ERR_REG` via `None`.
fn take_v(src: u32) -> Option<Version> {
    match take(src) {
        Some(Val::V(v)) => Some(v),
        other => {
            // Put a non-version back rather than destroying it: the harness
            // aborts on the error return anyway, but keep the file honest.
            if let Some(val) = other {
                put(src, val);
            }
            None
        }
    }
}

/// Move a `Party` out of `src`.
fn take_p(src: u32) -> Option<Party> {
    match take(src) {
        Some(Val::P(p)) => Some(p),
        other => {
            if let Some(val) = other {
                put(src, val);
            }
            None
        }
    }
}

/// Move a `Clock` out of `src`.
fn take_c(src: u32) -> Option<Clock> {
    match take(src) {
        Some(Val::C(c)) => Some(c),
        other => {
            if let Some(val) = other {
                put(src, val);
            }
            None
        }
    }
}

/// Move a `Rank` out of `src`.
fn take_r(src: u32) -> Option<Rank> {
    match take(src) {
        Some(Val::R(r)) => Some(r),
        other => {
            if let Some(val) = other {
                put(src, val);
            }
            None
        }
    }
}

/// Run `f` with a borrowed `Version` in `reg`.
fn with_v<T>(reg: u32, f: impl FnOnce(&Version) -> T) -> Option<T> {
    REGS.with_borrow(|regs| match regs.get(reg as usize) {
        Some(Some(Val::V(v))) => Some(f(v)),
        _ => None,
    })
}

/// Run `f` with a borrowed `Party` in `reg`.
fn with_p<T>(reg: u32, f: impl FnOnce(&Party) -> T) -> Option<T> {
    REGS.with_borrow(|regs| match regs.get(reg as usize) {
        Some(Some(Val::P(p))) => Some(f(p)),
        _ => None,
    })
}

/// Run `f` with a borrowed `Rank` in `reg`.
fn with_r<T>(reg: u32, f: impl FnOnce(&Rank) -> T) -> Option<T> {
    REGS.with_borrow(|regs| match regs.get(reg as usize) {
        Some(Some(Val::R(r))) => Some(f(r)),
        _ => None,
    })
}

/// Run `f` with a borrowed `Span` in `reg`.
fn with_s<T>(reg: u32, f: impl FnOnce(&Span<'static>) -> T) -> Option<T> {
    REGS.with_borrow(|regs| match regs.get(reg as usize) {
        Some(Some(Val::S(s))) => Some(f(s)),
        _ => None,
    })
}

/// Run `f` with a mutable `Party` in `reg`.
fn with_p_mut<T>(reg: u32, f: impl FnOnce(&mut Party) -> T) -> Option<T> {
    REGS.with_borrow_mut(|regs| match regs.get_mut(reg as usize) {
        Some(Some(Val::P(p))) => Some(f(p)),
        _ => None,
    })
}

/// Run `f` with a mutable `Clock` in `reg`.
fn with_c_mut<T>(reg: u32, f: impl FnOnce(&mut Clock) -> T) -> Option<T> {
    REGS.with_borrow_mut(|regs| match regs.get_mut(reg as usize) {
        Some(Some(Val::C(c))) => Some(f(c)),
        _ => None,
    })
}

/// Lift an `Option<i32>` result-code computation into the ABI return code.
fn code(r: Option<i32>) -> i32 {
    r.unwrap_or(ERR_REG)
}

// ─── control (unmeasured) ────────────────────────────────────────────────────

/// Empty kernel: the fuel cost of one host→guest call, the measurement's
/// per-call overhead baseline.
#[no_mangle]
pub extern "C" fn ff_nop() -> i32 {
    OK
}

/// Clear the register file and the staging buffer.
#[no_mangle]
pub extern "C" fn ff_reset() -> i32 {
    REGS.with_borrow_mut(Vec::clear);
    STAGE.with_borrow_mut(Vec::clear);
    OK
}

/// Pre-reserve `n` register slots (unmeasured; the harness calls this once
/// per instance).
///
/// Without the reservation, a measured kernel whose `put` lands on a `Vec`
/// doubling boundary pays an O(file) reallocation inside its fuel window —
/// register-machine bookkeeping billed to a public operation. The
/// enforcement suite caught exactly that as a false above-band flag on
/// `ff_party_seed` (the committed seed in
/// `harness/tests/enforce.proptest-regressions` replays it); with the file
/// pre-reserved to the program budget, `put` fills at most one fresh slot
/// per call, O(1) forever.
#[no_mangle]
pub extern "C" fn ff_regs_reserve(n: u32) -> i32 {
    REGS.with_borrow_mut(|regs| regs.reserve(n as usize));
    OK
}

/// Resize the staging buffer to `len` and return its address; the host then
/// writes payload bytes directly into linear memory.
#[no_mangle]
pub extern "C" fn ff_stage_prepare(len: u32) -> u32 {
    STAGE.with_borrow_mut(|stage| {
        stage.clear();
        stage.resize(len as usize, 0);
        stage.as_ptr() as u32
    })
}

/// Current staging buffer address (for host-side reads of kernel output).
#[no_mangle]
pub extern "C" fn ff_stage_ptr() -> u32 {
    STAGE.with_borrow(|stage| stage.as_ptr() as u32)
}

/// Current staging buffer length.
#[no_mangle]
pub extern "C" fn ff_stage_len() -> u32 {
    STAGE.with_borrow(|stage| stage.len() as u32)
}

// ─── codecs (measured) ───────────────────────────────────────────────────────

/// Decode the staged bytes as a canonical `Version` into `dst`.
#[no_mangle]
pub extern "C" fn ff_version_decode(dst: u32) -> i32 {
    STAGE.with_borrow(|stage| match Version::decode(stage.as_slice()) {
        Ok(v) => {
            put(dst, Val::V(v));
            OK
        }
        Err(_) => ERR_CODEC,
    })
}

/// Decode the staged bytes as a canonical `Party` into `dst`.
#[no_mangle]
pub extern "C" fn ff_party_decode(dst: u32) -> i32 {
    STAGE.with_borrow(|stage| match Party::decode(stage.as_slice()) {
        Ok(p) => {
            put(dst, Val::P(p));
            OK
        }
        Err(_) => ERR_CODEC,
    })
}

/// Decode the staged bytes as a canonical `Clock` into `dst`.
#[no_mangle]
pub extern "C" fn ff_clock_decode(dst: u32) -> i32 {
    STAGE.with_borrow(|stage| match Clock::decode(stage.as_slice()) {
        Ok(c) => {
            put(dst, Val::C(c));
            OK
        }
        Err(_) => ERR_CODEC,
    })
}

/// Encode the `Version` in `src` into the staging buffer.
#[no_mangle]
pub extern "C" fn ff_version_encode(src: u32) -> i32 {
    code(with_v(src, |v| {
        let bytes = v.encode();
        STAGE.with_borrow_mut(|stage| *stage = bytes);
        OK
    }))
}

/// Encode the `Party` in `src` into the staging buffer.
#[no_mangle]
pub extern "C" fn ff_party_encode(src: u32) -> i32 {
    code(with_p(src, |p| {
        let bytes = p.encode();
        STAGE.with_borrow_mut(|stage| *stage = bytes);
        OK
    }))
}

/// Encode the `Clock` in `src` into the staging buffer.
#[no_mangle]
pub extern "C" fn ff_clock_encode(src: u32) -> i32 {
    REGS.with_borrow(|regs| match regs.get(src as usize) {
        Some(Some(Val::C(c))) => {
            let bytes = c.encode();
            STAGE.with_borrow_mut(|stage| *stage = bytes);
            OK
        }
        _ => ERR_REG,
    })
}

// ─── text I/O (measured) ─────────────────────────────────────────────────────

/// Render the `Version` in `src` to text in the staging buffer.
#[no_mangle]
pub extern "C" fn ff_version_display(src: u32) -> i32 {
    code(with_v(src, |v| {
        let mut s = String::new();
        write!(s, "{v}").expect("Display into String cannot fail");
        STAGE.with_borrow_mut(|stage| *stage = s.into_bytes());
        OK
    }))
}

/// Render the `Party` in `src` to text in the staging buffer.
#[no_mangle]
pub extern "C" fn ff_party_display(src: u32) -> i32 {
    code(with_p(src, |p| {
        let mut s = String::new();
        write!(s, "{p}").expect("Display into String cannot fail");
        STAGE.with_borrow_mut(|stage| *stage = s.into_bytes());
        OK
    }))
}

/// Parse the staged text as a `Version` into `dst`.
#[no_mangle]
pub extern "C" fn ff_version_fromstr(dst: u32) -> i32 {
    STAGE.with_borrow(|stage| {
        let Ok(text) = std::str::from_utf8(stage) else {
            return ERR_CODEC;
        };
        match text.parse::<Version>() {
            Ok(v) => {
                put(dst, Val::V(v));
                OK
            }
            Err(_) => ERR_CODEC,
        }
    })
}

/// Parse the staged text as a `Party` into `dst`.
#[no_mangle]
pub extern "C" fn ff_party_fromstr(dst: u32) -> i32 {
    STAGE.with_borrow(|stage| {
        let Ok(text) = std::str::from_utf8(stage) else {
            return ERR_CODEC;
        };
        match text.parse::<Party>() {
            Ok(p) => {
                put(dst, Val::P(p));
                OK
            }
            Err(_) => ERR_CODEC,
        }
    })
}

// ─── Version operations (measured) ───────────────────────────────────────────

/// `Version::tick`: tick the version in `ver` with the party in `party`.
#[no_mangle]
pub extern "C" fn ff_version_tick(ver: u32, party: u32) -> i32 {
    REGS.with_borrow_mut(|regs| {
        // Two disjoint borrows out of one file: split at the higher index.
        let (v_idx, p_idx) = (ver as usize, party as usize);
        if v_idx == p_idx || regs.len() <= v_idx.max(p_idx) {
            return ERR_REG;
        }
        let (lo, hi) = regs.split_at_mut(v_idx.max(p_idx));
        let (a, b) = if v_idx < p_idx {
            (&mut lo[v_idx], &mut hi[0])
        } else {
            (&mut hi[0], &mut lo[p_idx])
        };
        match (a, b) {
            (Some(Val::V(v)), Some(Val::P(p))) => {
                v.tick(p);
                OK
            }
            _ => ERR_REG,
        }
    })
}

/// `Version::ticks`: advance the version in `ver` by `n` events for the
/// party in `party` (the fused multi-tick walk, flat in the count).
#[no_mangle]
pub extern "C" fn ff_version_ticks(ver: u32, party: u32, n: u32) -> i32 {
    REGS.with_borrow_mut(|regs| {
        // Two disjoint borrows out of one file: split at the higher index
        // (the same discipline as `ff_version_tick`).
        let (v_idx, p_idx) = (ver as usize, party as usize);
        if v_idx == p_idx || regs.len() <= v_idx.max(p_idx) {
            return ERR_REG;
        }
        let (lo, hi) = regs.split_at_mut(v_idx.max(p_idx));
        let (a, b) = if v_idx < p_idx {
            (&mut lo[v_idx], &mut hi[0])
        } else {
            (&mut hi[0], &mut lo[p_idx])
        };
        match (a, b) {
            (Some(Val::V(v)), Some(Val::P(p))) => {
                v.ticks(p, u64::from(n));
                OK
            }
            _ => ERR_REG,
        }
    })
}

/// `Version` join (`|`): `dst = take(a) | &b` (the by-value/by-ref cell).
#[no_mangle]
pub extern "C" fn ff_version_join(dst: u32, a: u32, b: u32) -> i32 {
    let Some(va) = take_v(a) else {
        return ERR_REG;
    };
    // `va` moves into the closure: on the (harness-bug) error path the value
    // is lost, which is fine — the harness aborts the case on any nonzero
    // return. Keeping the happy path move-only keeps the measured window
    // exactly one public operation, no defensive clones.
    match with_v(b, |vb| va | vb) {
        Some(joined) => {
            put(dst, Val::V(joined));
            OK
        }
        None => ERR_REG,
    }
}

/// `Version` meet (`&`): `dst = take(a) & &b`.
#[no_mangle]
pub extern "C" fn ff_version_meet(dst: u32, a: u32, b: u32) -> i32 {
    let Some(va) = take_v(a) else {
        return ERR_REG;
    };
    match with_v(b, |vb| va & vb) {
        Some(met) => {
            put(dst, Val::V(met));
            OK
        }
        None => ERR_REG,
    }
}

/// `Version` projection, materialized: `dst = (&v / &p).to_version()`,
/// the output-dominated row (the view construction itself is O(1) and
/// prices nothing; this kernel prices the explicit materialization).
#[no_mangle]
pub extern "C" fn ff_version_project(dst: u32, v: u32, p: u32) -> i32 {
    let projected = with_v(v, |ver| with_p(p, |party| (ver / party).to_version()));
    match projected {
        Some(Some(out)) => {
            put(dst, Val::V(out));
            OK
        }
        _ => ERR_REG,
    }
}

/// `PartialOrd` on versions: returns 0 `Less`, 1 `Equal`, 2 `Greater`,
/// 3 concurrent (no ordering).
#[no_mangle]
pub extern "C" fn ff_version_cmp(a: u32, b: u32) -> i32 {
    let r = with_v(a, |va| with_v(b, |vb| va.partial_cmp(vb)));
    match r {
        Some(Some(ord)) => match ord {
            Some(Ordering::Less) => 0,
            Some(Ordering::Equal) => 1,
            Some(Ordering::Greater) => 2,
            None => 3,
        },
        _ => ERR_REG,
    }
}

/// `Version::concurrent`.
#[no_mangle]
pub extern "C" fn ff_version_concurrent(a: u32, b: u32) -> i32 {
    match with_v(a, |va| with_v(b, |vb| va.concurrent(vb))) {
        Some(Some(c)) => i32::from(c),
        _ => ERR_REG,
    }
}

/// `Version::rank` into a `Rank` register.
#[no_mangle]
pub extern "C" fn ff_version_rank(dst: u32, src: u32) -> i32 {
    // The rank is computed under the shared borrow and stored after it
    // ends: `put` needs the file mutably.
    match with_v(src, |v| v.rank()) {
        Some(rank) => {
            put(dst, Val::R(rank));
            OK
        }
        None => ERR_REG,
    }
}

/// `Version::distance` into a `Rank` register.
#[no_mangle]
pub extern "C" fn ff_version_distance(dst: u32, a: u32, b: u32) -> i32 {
    match with_v(a, |va| with_v(b, |vb| va.distance(vb))) {
        Some(Some(rank)) => {
            put(dst, Val::R(rank));
            OK
        }
        _ => ERR_REG,
    }
}

/// `Version::lag` into a `Rank` register.
#[no_mangle]
pub extern "C" fn ff_version_lag(dst: u32, a: u32, b: u32) -> i32 {
    match with_v(a, |va| with_v(b, |vb| va.lag(vb))) {
        Some(Some(rank)) => {
            put(dst, Val::R(rank));
            OK
        }
        _ => ERR_REG,
    }
}

/// `Version::min_ticks`; the return value is the count's decimal
/// digest, with `-1` reserved for a bad register.
///
/// The count itself is unbounded, so the `i64` channel carries a
/// nonnegative FNV-1a of its decimal rendering, computed identically
/// on the native side.
#[no_mangle]
pub extern "C" fn ff_version_min_ticks(src: u32) -> i64 {
    match with_v(src, |v| v.min_ticks()) {
        Some(n) => decimal_digest(&n.to_string()),
        None => -1,
    }
}

/// A nonnegative FNV-1a digest of a decimal rendering.
///
/// The `i64` channel's encoding for unbounded counts: both sides of the
/// differential compute it, so equality is equality of counts up to a
/// collision no fuel schedule can steer.
fn decimal_digest(text: &str) -> i64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (h & 0x7fff_ffff_ffff_ffff) as i64
}

/// `Version::join_all` over registers `src..src + n`, result into `dst`.
#[no_mangle]
pub extern "C" fn ff_version_join_all(dst: u32, src: u32, n: u32) -> i32 {
    let mut ops = Vec::with_capacity(n as usize);
    for i in 0..n {
        match take_v(src + i) {
            Some(v) => ops.push(v),
            None => return ERR_REG,
        }
    }
    put(dst, Val::V(Version::join_all(ops)));
    OK
}

/// `Version::meet_all` over registers `src..src + n`, result into `dst`.
#[no_mangle]
pub extern "C" fn ff_version_meet_all(dst: u32, src: u32, n: u32) -> i32 {
    let mut ops = Vec::with_capacity(n as usize);
    for i in 0..n {
        match take_v(src + i) {
            Some(v) => ops.push(v),
            None => return ERR_REG,
        }
    }
    match Version::meet_all(ops) {
        Some(met) => {
            put(dst, Val::V(met));
            OK
        }
        None => ERR_OP,
    }
}

/// `Version::span`: the pair's lattice hull `[a & b, a | b]` into a
/// span register (one fused pair walk feeds both endpoints; the
/// operands are read in place and the endpoints minted owned).
#[no_mangle]
pub extern "C" fn ff_version_span(dst: u32, a: u32, b: u32) -> i32 {
    let span = with_v(a, |va| with_v(b, |vb| va.span(vb)));
    match span {
        Some(Some(s)) => {
            put(dst, Val::S(s));
            OK
        }
        _ => ERR_REG,
    }
}

/// `Version::span_all`: the lattice hull of the versions in
/// `src..src + n` into `dst`.
///
/// The version in `src` is the receiver, the rest ride as the iterator,
/// feed order preserved; every operand is borrowed — the hull fold
/// reads them in place and mints its endpoints owned.
#[no_mangle]
pub extern "C" fn ff_version_span_all(dst: u32, src: u32, n: u32) -> i32 {
    if n == 0 {
        return ERR_REG;
    }
    let span = REGS.with_borrow(|regs| {
        let version = |i: u32| match regs.get(i as usize) {
            Some(Some(Val::V(v))) => Some(v),
            _ => None,
        };
        let receiver = version(src)?;
        let others: Vec<&Version> = (1..n).map(|i| version(src + i)).collect::<Option<_>>()?;
        Some(receiver.span_all(others))
    });
    match span {
        Some(s) => {
            put(dst, Val::S(s));
            OK
        }
        None => ERR_REG,
    }
}

/// The fused three-stream masked comparison `(v / p) ⋚ w`, no
/// materialization: returns 0 `Less`, 1 `Equal`, 2 `Greater`,
/// 3 concurrent (no ordering).
#[no_mangle]
pub extern "C" fn ff_own_version_cmp(v: u32, p: u32, w: u32) -> i32 {
    let r = with_v(v, |ver| {
        with_p(p, |party| {
            with_v(w, |other| (ver / party).partial_cmp(other))
        })
    });
    match r {
        Some(Some(Some(ord))) => match ord {
            Some(Ordering::Less) => 0,
            Some(Ordering::Equal) => 1,
            Some(Ordering::Greater) => 2,
            None => 3,
        },
        _ => ERR_REG,
    }
}

/// The fused four-stream masked comparison `(v₁ / p₁) ⋚ (v₂ / p₂)`, no
/// materialization: returns 0 `Less`, 1 `Equal`, 2 `Greater`,
/// 3 concurrent (no ordering).
#[no_mangle]
pub extern "C" fn ff_own_version_pair_cmp(v1: u32, p1: u32, v2: u32, p2: u32) -> i32 {
    let r = with_v(v1, |va| {
        with_p(p1, |pa| {
            with_v(v2, |vb| with_p(p2, |pb| (va / pa).partial_cmp(&(vb / pb))))
        })
    });
    match r {
        Some(Some(Some(Some(ord)))) => match ord {
            Some(Ordering::Less) => 0,
            Some(Ordering::Equal) => 1,
            Some(Ordering::Greater) => 2,
            None => 3,
        },
        _ => ERR_REG,
    }
}

// ─── Party operations (measured) ─────────────────────────────────────────────

/// `Party::seed` into `dst`.
#[no_mangle]
pub extern "C" fn ff_party_seed(dst: u32) -> i32 {
    put(dst, Val::P(Party::seed()));
    OK
}

/// `Party::fork`: split the party in `src`, the new share into `dst`.
#[no_mangle]
pub extern "C" fn ff_party_fork(dst: u32, src: u32) -> i32 {
    let forked = with_p_mut(src, |p| p.fork());
    match forked {
        Some(p) => {
            put(dst, Val::P(p));
            OK
        }
        None => ERR_REG,
    }
}

/// `Party::forks(n)`: balanced shares into `dst..dst + n` (replaces `src`:
/// the iterator borrows the source, which keeps its remainder).
#[no_mangle]
pub extern "C" fn ff_party_forks(dst: u32, src: u32, n: u32) -> i32 {
    let shares = with_p_mut(src, |p| p.forks(u64::from(n)).collect::<Vec<_>>());
    match shares {
        Some(shares) => {
            for (i, share) in shares.into_iter().enumerate() {
                put(dst + i as u32, Val::P(share));
            }
            OK
        }
        None => ERR_REG,
    }
}

/// `Party::join`: fold the party in `b` into `a` (consumes `b`).
#[no_mangle]
pub extern "C" fn ff_party_join(a: u32, b: u32) -> i32 {
    let Some(pb) = take_p(b) else {
        return ERR_REG;
    };
    let joined = with_p_mut(a, |pa| pa.join(pb));
    match joined {
        Some(Ok(())) => OK,
        Some(Err(rejected)) => {
            put(b, Val::P(rejected));
            ERR_OP
        }
        None => ERR_REG,
    }
}

/// `Party::join_all`: fold the parties in `src..src + n` into `a`
/// (consumes the range; the n-ary balanced fold).
///
/// On an overlap rejection the handed-back parties are dropped rather
/// than restored: the harness aborts the case on any nonzero return, and
/// the atlas's fold panels construct disjoint populations, so the error
/// path is a roster bug, never a measurement.
#[no_mangle]
pub extern "C" fn ff_party_join_all(a: u32, src: u32, n: u32) -> i32 {
    let mut ops = Vec::with_capacity(n as usize);
    for i in 0..n {
        match take_p(src + i) {
            Some(p) => ops.push(p),
            None => return ERR_REG,
        }
    }
    match with_p_mut(a, |pa| pa.join_all(ops)) {
        Some(Ok(())) => OK,
        Some(Err(_)) => ERR_OP,
        None => ERR_REG,
    }
}

/// `Party::is_disjoint`.
#[no_mangle]
pub extern "C" fn ff_party_is_disjoint(a: u32, b: u32) -> i32 {
    match with_p(a, |pa| with_p(b, |pb| pa.is_disjoint(pb))) {
        Some(Some(d)) => i32::from(d),
        _ => ERR_REG,
    }
}

/// `Party::covers`.
#[no_mangle]
pub extern "C" fn ff_party_covers(a: u32, b: u32) -> i32 {
    match with_p(a, |pa| with_p(b, |pb| pa.covers(pb))) {
        Some(Some(c)) => i32::from(c),
        _ => ERR_REG,
    }
}

/// `Party::without`: `dst = take(a).without(&b)`; returns `ERR_OP` when the
/// difference is empty (the operand is consumed either way, as the API does).
#[no_mangle]
pub extern "C" fn ff_party_without(dst: u32, a: u32, b: u32) -> i32 {
    let Some(pa) = take_p(a) else {
        return ERR_REG;
    };
    match with_p(b, |pb| pa.without(pb)) {
        Some(Some(diff)) => {
            put(dst, Val::P(diff));
            OK
        }
        Some(None) => ERR_OP,
        None => ERR_REG,
    }
}

// ─── Clock operations (measured) ─────────────────────────────────────────────

/// `Clock::seed` into `dst`.
#[no_mangle]
pub extern "C" fn ff_clock_seed(dst: u32) -> i32 {
    put(dst, Val::C(Clock::seed()));
    OK
}

/// `Clock::tick`.
#[no_mangle]
pub extern "C" fn ff_clock_tick(c: u32) -> i32 {
    code(with_c_mut(c, |clock| {
        clock.tick();
        OK
    }))
}

/// `Clock::fork`: the new clock into `dst`.
#[no_mangle]
pub extern "C" fn ff_clock_fork(dst: u32, src: u32) -> i32 {
    let forked = with_c_mut(src, |c| c.fork());
    match forked {
        Some(c) => {
            put(dst, Val::C(c));
            OK
        }
        None => ERR_REG,
    }
}

/// `Clock::join`: fold the clock in `b` into `a` (consumes `b`).
#[no_mangle]
pub extern "C" fn ff_clock_join(a: u32, b: u32) -> i32 {
    let Some(cb) = take_c(b) else {
        return ERR_REG;
    };
    let joined = with_c_mut(a, |ca| ca.join(cb).map(|_| ()));
    match joined {
        Some(Ok(())) => OK,
        Some(Err(rejected)) => {
            put(b, Val::C(rejected));
            ERR_OP
        }
        None => ERR_REG,
    }
}

/// `Clock::join_all`: fold the clocks in `src..src + n` into `a`
/// (consumes the range; the n-ary balanced fold over both halves).
///
/// On an overlap rejection the handed-back clocks are dropped rather
/// than restored, as in `ff_party_join_all`: the error path is a roster
/// bug, never a measurement.
#[no_mangle]
pub extern "C" fn ff_clock_join_all(a: u32, src: u32, n: u32) -> i32 {
    let mut ops = Vec::with_capacity(n as usize);
    for i in 0..n {
        match take_c(src + i) {
            Some(c) => ops.push(c),
            None => return ERR_REG,
        }
    }
    match with_c_mut(a, |ca| ca.join_all(ops).map(|_| ())) {
        Some(Ok(())) => OK,
        Some(Err(_)) => ERR_OP,
        None => ERR_REG,
    }
}

/// `Clock::send`.
#[no_mangle]
pub extern "C" fn ff_clock_send(c: u32) -> i32 {
    code(with_c_mut(c, |clock| {
        clock.send();
        OK
    }))
}

/// `Clock::recv`: receive the `Version` in `v` into the clock in `c`.
#[no_mangle]
pub extern "C" fn ff_clock_recv(c: u32, v: u32) -> i32 {
    REGS.with_borrow_mut(|regs| {
        let (c_idx, v_idx) = (c as usize, v as usize);
        if c_idx == v_idx || regs.len() <= c_idx.max(v_idx) {
            return ERR_REG;
        }
        let (lo, hi) = regs.split_at_mut(c_idx.max(v_idx));
        let (a, b) = if c_idx < v_idx {
            (&mut lo[c_idx], &mut hi[0])
        } else {
            (&mut hi[0], &mut lo[v_idx])
        };
        match (a, b) {
            (Some(Val::C(clock)), Some(Val::V(version))) => {
                clock.recv(version);
                OK
            }
            _ => ERR_REG,
        }
    })
}

/// `Clock::sync` between the clocks in `a` and `b`.
#[no_mangle]
pub extern "C" fn ff_clock_sync(a: u32, b: u32) -> i32 {
    REGS.with_borrow_mut(|regs| {
        let (a_idx, b_idx) = (a as usize, b as usize);
        if a_idx == b_idx || regs.len() <= a_idx.max(b_idx) {
            return ERR_REG;
        }
        let (lo, hi) = regs.split_at_mut(a_idx.max(b_idx));
        let (x, y) = if a_idx < b_idx {
            (&mut lo[a_idx], &mut hi[0])
        } else {
            (&mut hi[0], &mut lo[b_idx])
        };
        match (x, y) {
            (Some(Val::C(ca)), Some(Val::C(cb))) => match ca.sync(cb) {
                Ok(_) => OK,
                Err(_) => ERR_OP,
            },
            _ => ERR_REG,
        }
    })
}

/// `Clock::own_version`, materialized into a `Version` register (the
/// output-dominated row; the view is O(1), the materialization is what
/// this kernel prices).
#[no_mangle]
pub extern "C" fn ff_clock_own_version(dst: u32, src: u32) -> i32 {
    let own = REGS.with_borrow(|regs| match regs.get(src as usize) {
        Some(Some(Val::C(c))) => Some(c.own_version().to_version()),
        _ => None,
    });
    match own {
        Some(v) => {
            put(dst, Val::V(v));
            OK
        }
        None => ERR_REG,
    }
}

/// `Clock::version`, cloned into a `Version` register (the register machine's
/// bridge from clock programs to version operands).
#[no_mangle]
pub extern "C" fn ff_clock_version(dst: u32, src: u32) -> i32 {
    let version = REGS.with_borrow(|regs| match regs.get(src as usize) {
        Some(Some(Val::C(c))) => Some(c.version().clone()),
        _ => None,
    });
    match version {
        Some(v) => {
            put(dst, Val::V(v));
            OK
        }
        None => ERR_REG,
    }
}

/// `Clock::into_parts`: split the clock in `src` into a `Party` in `dst_p`
/// and a `Version` in `dst_v`.
#[no_mangle]
pub extern "C" fn ff_clock_into_parts(dst_p: u32, dst_v: u32, src: u32) -> i32 {
    let Some(clock) = take_c(src) else {
        return ERR_REG;
    };
    let (party, version) = clock.into_parts();
    put(dst_p, Val::P(party));
    put(dst_v, Val::V(version));
    OK
}

/// `Clock::from_parts`: assemble a clock in `dst` from the `Party` in `p`
/// and the `Version` in `v` (both consumed).
#[no_mangle]
pub extern "C" fn ff_clock_from_parts(dst: u32, p: u32, v: u32) -> i32 {
    let Some(party) = take_p(p) else {
        return ERR_REG;
    };
    let Some(version) = take_v(v) else {
        put(p, Val::P(party));
        return ERR_REG;
    };
    put(dst, Val::C(Clock::from_parts(party, version)));
    OK
}

// ─── Rank operations (measured) ──────────────────────────────────────────────

/// `Rank + &Rank` into `dst` (consumes `a`).
#[no_mangle]
pub extern "C" fn ff_rank_add(dst: u32, a: u32, b: u32) -> i32 {
    let Some(ra) = take_r(a) else {
        return ERR_REG;
    };
    // `ra` moves into the closure: on the (harness-bug) error path the value
    // is lost, which is fine — the harness aborts the case on any nonzero
    // return. Keeping the happy path move-only keeps the measured window
    // exactly one public operation, no defensive clones (the same
    // discipline as `ff_version_join`).
    match with_r(b, |rb| ra + rb) {
        Some(s) => {
            put(dst, Val::R(s));
            OK
        }
        None => ERR_REG,
    }
}

/// Render the `Rank` in `src` to text in the staging buffer (the harness's
/// end-of-program differential reads ranks as text; `Rank` has no codec).
#[no_mangle]
pub extern "C" fn ff_rank_display(src: u32) -> i32 {
    code(with_r(src, |r| {
        let mut s = String::new();
        write!(s, "{r}").expect("Display into String cannot fail");
        STAGE.with_borrow_mut(|stage| *stage = s.into_bytes());
        OK
    }))
}

/// `Ord` on ranks: 0 `Less`, 1 `Equal`, 2 `Greater`.
#[no_mangle]
pub extern "C" fn ff_rank_cmp(a: u32, b: u32) -> i32 {
    match with_r(a, |ra| with_r(b, |rb| ra.cmp(rb))) {
        Some(Some(Ordering::Less)) => 0,
        Some(Some(Ordering::Equal)) => 1,
        Some(Some(Ordering::Greater)) => 2,
        _ => ERR_REG,
    }
}

/// `Rank::checked_sub` into `dst`; `ERR_OP` when the difference underflows.
#[no_mangle]
pub extern "C" fn ff_rank_checked_sub(dst: u32, a: u32, b: u32) -> i32 {
    match with_r(a, |ra| with_r(b, |rb| ra.checked_sub(rb))) {
        Some(Some(Some(diff))) => {
            put(dst, Val::R(diff));
            OK
        }
        Some(Some(None)) => ERR_OP,
        _ => ERR_REG,
    }
}

// ─── Span operations (measured) ──────────────────────────────────────────────

/// `Span::place`: the nine-way placement of the version in `probe`
/// against the span in `s`, encoded 0..=8.
///
/// The encoding follows the variant order `Before, At(Start), At(End),
/// At(Both), Between, Concurrent(Start), Concurrent(End),
/// Concurrent(Both), After`.
#[no_mangle]
pub extern "C" fn ff_span_place(s: u32, probe: u32) -> i32 {
    let r = with_s(s, |span| with_v(probe, |p| span.place(p)));
    match r {
        Some(Some(placement)) => match placement {
            Placement::Before => 0,
            Placement::At(Endpoint::Start) => 1,
            Placement::At(Endpoint::End) => 2,
            Placement::At(Endpoint::Both) => 3,
            Placement::Between => 4,
            Placement::Concurrent(Endpoint::Start) => 5,
            Placement::Concurrent(Endpoint::End) => 6,
            Placement::Concurrent(Endpoint::Both) => 7,
            Placement::After => 8,
        },
        _ => ERR_REG,
    }
}

/// `Span::dominance`: the three-way dominance coarsening of the
/// version in `probe` against the span in `s`, encoded 0 `Before`,
/// 1 `Between`, 2 `After`.
///
/// The placement family's earliest exits live in this verdict, so its
/// fuel can undercut `ff_span_place`'s on refuting probes.
#[no_mangle]
pub extern "C" fn ff_span_dominance(s: u32, probe: u32) -> i32 {
    let r = with_s(s, |span| with_v(probe, |p| span.dominance(p)));
    match r {
        Some(Some(dominance)) => match dominance {
            Dominance::Before => 0,
            Dominance::Between => 1,
            Dominance::After => 2,
        },
        _ => ERR_REG,
    }
}

/// Encode the `Span` in `src` into the staging buffer (the meet's
/// canonical bytes, then the join's).
#[no_mangle]
pub extern "C" fn ff_span_encode(src: u32) -> i32 {
    code(with_s(src, |s| {
        let bytes = s.encode();
        STAGE.with_borrow_mut(|stage| *stage = bytes);
        OK
    }))
}

/// Decode the staged bytes as a canonical `Span` into `dst` (the one
/// forward pass whose second parse also proves, in the same walk, that
/// the pair is ordered).
#[no_mangle]
pub extern "C" fn ff_span_decode(dst: u32) -> i32 {
    STAGE.with_borrow(|stage| match Span::decode(stage.as_slice()) {
        Ok(s) => {
            put(dst, Val::S(s));
            OK
        }
        Err(_) => ERR_CODEC,
    })
}

/// `Span | Span`: the containment join of the spans in `a` and `b`
/// into `dst` (operands read in place; the union's endpoints are
/// minted owned).
#[no_mangle]
pub extern "C" fn ff_span_union(dst: u32, a: u32, b: u32) -> i32 {
    let r = with_s(a, |sa| with_s(b, |sb| sa | sb));
    match r {
        Some(Some(s)) => {
            put(dst, Val::S(s));
            OK
        }
        _ => ERR_REG,
    }
}

/// `Span & Span`: the containment meet of the spans in `a` and `b`
/// into `dst`.
///
/// Returns 1 with `dst` written when the segments share a version, 0
/// with `dst` untouched on the empty intersection — both measured
/// verdicts, not errors.
#[no_mangle]
pub extern "C" fn ff_span_intersect(dst: u32, a: u32, b: u32) -> i32 {
    let r = with_s(a, |sa| with_s(b, |sb| sa & sb));
    match r {
        Some(Some(Some(s))) => {
            put(dst, Val::S(s));
            1
        }
        Some(Some(None)) => 0,
        _ => ERR_REG,
    }
}

/// `Span + Span`: the pointwise join of the spans in `a` and `b` into
/// `dst`.
#[no_mangle]
pub extern "C" fn ff_span_sum(dst: u32, a: u32, b: u32) -> i32 {
    let r = with_s(a, |sa| with_s(b, |sb| sa + sb));
    match r {
        Some(Some(s)) => {
            put(dst, Val::S(s));
            OK
        }
        _ => ERR_REG,
    }
}

/// `Span * Span`: the pointwise meet of the spans in `a` and `b` into
/// `dst`.
#[no_mangle]
pub extern "C" fn ff_span_product(dst: u32, a: u32, b: u32) -> i32 {
    let r = with_s(a, |sa| with_s(b, |sb| sa * sb));
    match r {
        Some(Some(s)) => {
            put(dst, Val::S(s));
            OK
        }
        _ => ERR_REG,
    }
}

/// Borrow the spans in `src..src + n` as (receiver, items) and run
/// one n-ary span door over them.
///
/// The span in `src` is the receiver, the rest ride as the iterator,
/// feed order preserved; every operand is borrowed — the balanced
/// fold reads them in place and mints its endpoints owned.
fn span_fold<T>(
    src: u32,
    n: u32,
    door: impl FnOnce(&Span<'static>, Vec<&Span<'static>>) -> T,
) -> Option<T> {
    if n == 0 {
        return None;
    }
    REGS.with_borrow(|regs| {
        let span = |i: u32| match regs.get(i as usize) {
            Some(Some(Val::S(s))) => Some(s),
            _ => None,
        };
        let receiver = span(src)?;
        let others: Vec<&Span<'static>> = (1..n).map(|i| span(src + i)).collect::<Option<_>>()?;
        Some(door(receiver, others))
    })
}

/// `Span::union_all`: the containment join of the spans in
/// `src..src + n` into `dst` (the receiver-seeded balanced fold; the
/// span in `src` is the receiver, feed order preserved).
#[no_mangle]
pub extern "C" fn ff_span_union_all(dst: u32, src: u32, n: u32) -> i32 {
    match span_fold(src, n, |receiver, others| receiver.union_all(others)) {
        Some(s) => {
            put(dst, Val::S(s));
            OK
        }
        None => ERR_REG,
    }
}

/// `Span::intersect_all`: the containment meet of the spans in
/// `src..src + n` into `dst` (the receiver-seeded balanced fold).
///
/// Returns 1 with `dst` written on a shared segment, 0 with `dst`
/// untouched on the empty intersection — both measured verdicts.
#[no_mangle]
pub extern "C" fn ff_span_intersect_all(dst: u32, src: u32, n: u32) -> i32 {
    match span_fold(src, n, |receiver, others| receiver.intersect_all(others)) {
        Some(Some(s)) => {
            put(dst, Val::S(s));
            1
        }
        Some(None) => 0,
        None => ERR_REG,
    }
}

/// `Span::sum_all`: the pointwise join of the spans in `src..src + n`
/// into `dst` (the receiver-seeded balanced fold).
#[no_mangle]
pub extern "C" fn ff_span_sum_all(dst: u32, src: u32, n: u32) -> i32 {
    match span_fold(src, n, |receiver, others| receiver.sum_all(others)) {
        Some(s) => {
            put(dst, Val::S(s));
            OK
        }
        None => ERR_REG,
    }
}

/// `Span::product_all`: the pointwise meet of the spans in
/// `src..src + n` into `dst` (the receiver-seeded balanced fold).
#[no_mangle]
pub extern "C" fn ff_span_product_all(dst: u32, src: u32, n: u32) -> i32 {
    match span_fold(src, n, |receiver, others| receiver.product_all(others)) {
        Some(s) => {
            put(dst, Val::S(s));
            OK
        }
        None => ERR_REG,
    }
}

/// The span projection, materialized: `dst = (&span / &party).to_span()`
/// (the view construction is O(1) and prices nothing; this kernel
/// prices the two-endpoint materialization).
#[no_mangle]
pub extern "C" fn ff_span_project(dst: u32, s: u32, p: u32) -> i32 {
    let out = with_s(s, |span| with_p(p, |party| (span / party).to_span()));
    match out {
        Some(Some(projected)) => {
            put(dst, Val::S(projected));
            OK
        }
        _ => ERR_REG,
    }
}

/// `OwnSpan::place`: the nine-way placement of the version in `probe`
/// against the projection of the span in `s` by the party in `p`,
/// encoded 0..=8 as `ff_span_place` encodes its verdict.
#[no_mangle]
pub extern "C" fn ff_own_span_place(s: u32, p: u32, probe: u32) -> i32 {
    let r = with_s(s, |span| {
        with_p(p, |party| with_v(probe, |v| (span / party).place(v)))
    });
    match r {
        Some(Some(Some(placement))) => match placement {
            Placement::Before => 0,
            Placement::At(Endpoint::Start) => 1,
            Placement::At(Endpoint::End) => 2,
            Placement::At(Endpoint::Both) => 3,
            Placement::Between => 4,
            Placement::Concurrent(Endpoint::Start) => 5,
            Placement::Concurrent(Endpoint::End) => 6,
            Placement::Concurrent(Endpoint::Both) => 7,
            Placement::After => 8,
        },
        _ => ERR_REG,
    }
}

/// `OwnSpan::dominance`: the three-way dominance coarsening of the
/// version in `probe` against the projection of the span in `s` by
/// the party in `p`, encoded 0 `Before`, 1 `Between`, 2 `After`.
///
/// The coarse question buys the projected placement's early exit: a
/// probe the projected start refutes never walks the end, so its fuel
/// can undercut `ff_own_span_place`'s there.
#[no_mangle]
pub extern "C" fn ff_own_span_dominance(s: u32, p: u32, probe: u32) -> i32 {
    let r = with_s(s, |span| {
        with_p(p, |party| with_v(probe, |v| (span / party).dominance(v)))
    });
    match r {
        Some(Some(Some(dominance))) => match dominance {
            Dominance::Before => 0,
            Dominance::Between => 1,
            Dominance::After => 2,
        },
        _ => ERR_REG,
    }
}

// ─── instrument self-test (measured, deliberately not a kernel) ──────────────

/// A deliberately quadratic burner: `n²` iterations, each pinned by
/// `black_box` so no strength reduction can collapse the nest into a
/// closed form that reads linear.
///
/// Not a kernel: no `before` operation runs here, no strategy emits it,
/// and calibration never bands it. The enforcement suite calls it
/// directly as an instrument-liveness check — the full detection path
/// (wasm execution, fuel metering, band judgment) must flag this genuine
/// superlinear mechanism ABOVE a linear band, or the instrument is blind
/// whatever the real kernels read.
#[no_mangle]
pub extern "C" fn ff_selftest_quadratic(n: u32) -> i32 {
    let mut acc: u64 = 0;
    for i in 0..u64::from(n) {
        for j in 0..u64::from(n) {
            acc = std::hint::black_box(acc ^ i.wrapping_mul(31).wrapping_add(j));
        }
    }
    // Consume the accumulator so the whole nest is observable work.
    std::hint::black_box(acc);
    OK
}
