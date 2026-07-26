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
//!   `Version`, `Party`, `Clock`, or `Rank`. Ops that consume an operand
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

use before::{Clock, Party, Rank, Version};

/// One register-file slot: any value the public surface produces.
enum Val {
    V(Version),
    P(Party),
    C(Clock),
    R(Rank),
}

thread_local! {
    /// The register file. wasm32-unknown-unknown is single-threaded, so a
    /// thread-local `RefCell` is an uncontended, unsafe-free global.
    static REGS: RefCell<Vec<Option<Val>>> = const { RefCell::new(Vec::new()) };
    /// The staging buffer for bulk byte transfer across the ABI.
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

/// `Version` projection (`/`): `dst = &v / &p`, the output-dominated row.
#[no_mangle]
pub extern "C" fn ff_version_project(dst: u32, v: u32, p: u32) -> i32 {
    let projected = with_v(v, |ver| with_p(p, |party| ver / party));
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
    code(with_v(src, |v| {
        let rank = v.rank();
        put(dst, Val::R(rank));
        OK
    }))
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

/// `Version::min_ticks`; the value returns through `ff_stage_len`-style
/// reads being unnecessary — it is the return value.
#[no_mangle]
pub extern "C" fn ff_version_min_ticks(src: u32) -> i64 {
    match with_v(src, |v| v.min_ticks()) {
        Some(n) => n as i64,
        None => -1,
    }
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
    let shares = with_p_mut(src, |p| p.forks(n as usize).collect::<Vec<_>>());
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

/// `Clock::own_version` into a `Version` register (output-dominated row).
#[no_mangle]
pub extern "C" fn ff_clock_own_version(dst: u32, src: u32) -> i32 {
    let own = REGS.with_borrow(|regs| match regs.get(src as usize) {
        Some(Some(Val::C(c))) => Some(c.own_version()),
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
    let sum = with_r(b, |rb| ra.clone() + rb);
    match sum {
        Some(s) => {
            put(dst, Val::R(s));
            OK
        }
        None => {
            put(a, Val::R(ra));
            ERR_REG
        }
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
