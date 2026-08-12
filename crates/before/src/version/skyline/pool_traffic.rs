//! Process-global counter of anchor-web pool misses: leases the
//! accumulator pool could not serve.
//!
//! The anchored-minimum web recycles every dying accumulator into a pool
//! (`MinWeb::retire`) and re-arms from it (`MinWeb::lease`), so range
//! churn allocates nothing in steady state: once the pool has grown to
//! the walk's peak simultaneous demand, every later lease is a recycled
//! buffer. No other meter can see that property — a heap meter reads
//! peak *live* bytes, and skipping the recycle entirely leaves the peak
//! untouched (each dropped buffer's bytes are released before the fresh
//! allocation that replaces it) with every touch and limb reading
//! byte-identical, since a fresh accumulator and a reset one fold
//! identically. This counter records the one observable the claim is
//! made of: a lease that found the pool empty, which is exactly an
//! allocation the pool could not serve. Steady-state churn reads a
//! constant bounded by the walk's peak outstanding leases (the fill
//! phase); churn-proportional misses mean the recycle is dead. The
//! seam-stop pool row in `tests/meter.rs` pins both directions.
//!
//! The recording compiles to nothing without the `limb-meter` feature —
//! its siblings' idiom (`codec::limb_meter`, `suanpan::touch_meter`) —
//! and the reading is process-global with the same isolation requirement
//! as every other meter: meaningful one scenario per process (nextest's
//! model) or under a single-threaded caller. The read surface is
//! `meter::pool_misses` / `meter::reset_pool_misses`.

#[cfg(feature = "limb-meter")]
mod counter {
    use core::sync::atomic::{AtomicU64, Ordering};

    static MISSES: AtomicU64 = AtomicU64::new(0);

    /// Count one lease the pool could not serve.
    pub(super) fn record_miss() {
        MISSES.fetch_add(1, Ordering::Relaxed);
    }

    /// The misses recorded since the last [`reset`].
    pub(crate) fn misses() -> u64 {
        MISSES.load(Ordering::Relaxed)
    }

    /// Reset the miss counter to zero.
    pub(crate) fn reset() {
        MISSES.store(0, Ordering::Relaxed);
    }
}

#[cfg(feature = "limb-meter")]
pub(crate) use counter::{misses, reset};

/// Count one lease the pool could not serve.
///
/// Compiles to nothing without the `limb-meter` feature, so the lease
/// path can call it unconditionally.
#[inline(always)]
pub(crate) fn record_miss() {
    #[cfg(feature = "limb-meter")]
    counter::record_miss();
}
