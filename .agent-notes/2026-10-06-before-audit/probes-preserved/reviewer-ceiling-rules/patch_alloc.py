"""Throwaway diagnostic: log >= 4 KiB allocator events during armed board measurements."""
import sys
p = sys.argv[1]
t = open(p).read()
old_alloc = "#[global_allocator]\nstatic HEAP: PeakAlloc = PeakAlloc;"
assert old_alloc in t
t = t.replace(old_alloc, r'''static HEAP: PeakAlloc = PeakAlloc;
mod diag {
    use super::HEAP;
    use std::alloc::{GlobalAlloc, Layout};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
    pub struct Logged;
    static ARMED: AtomicBool = AtomicBool::new(false);
    const CAP: usize = 4096;
    static REC: [AtomicUsize; CAP * 3] = [const { AtomicUsize::new(0) }; CAP * 3];
    static N: AtomicUsize = AtomicUsize::new(0);
    static BASE: AtomicUsize = AtomicUsize::new(0);
    fn rec(kind: usize, old: usize, new: usize) {
        if ARMED.load(Relaxed) && (old >= 4096 || new >= 4096) {
            let i = N.fetch_add(1, Relaxed);
            if i < CAP {
                REC[3 * i].store(kind, Relaxed);
                REC[3 * i + 1].store(old, Relaxed);
                REC[3 * i + 2].store(new, Relaxed);
            }
        }
    }
    unsafe impl GlobalAlloc for Logged {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 { rec(1, 0, l.size()); unsafe { HEAP.alloc(l) } }
        unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 { rec(1, 0, l.size()); unsafe { HEAP.alloc_zeroed(l) } }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) { rec(2, l.size(), 0); unsafe { HEAP.dealloc(p, l) } }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 { rec(3, l.size(), n); unsafe { HEAP.realloc(p, l, n) } }
    }
    pub fn arm() { N.store(0, Relaxed); BASE.store(HEAP.current_usage(), Relaxed); ARMED.store(true, Relaxed); }
    pub fn disarm_dump(peak: usize) {
        ARMED.store(false, Relaxed);
        let n = N.load(Relaxed).min(CAP);
        let base = BASE.load(Relaxed);
        eprint!("DIAG peak-base={} events={}:", peak.saturating_sub(base), N.load(Relaxed));
        for i in 0..n {
            let k = ["?", "A", "D", "R"][REC[3 * i].load(Relaxed)];
            eprint!(" {}{}>{}", k, REC[3 * i + 1].load(Relaxed), REC[3 * i + 2].load(Relaxed));
        }
        eprintln!();
    }
}
#[global_allocator]
static LOGGED: diag::Logged = diag::Logged;''')
for a, b in [("reset_peak: || HEAP.reset_peak_usage(),", "reset_peak: || { diag::arm(); HEAP.reset_peak_usage() },"),
             ("peak: || HEAP.peak_usage(),", "peak: || { let p = HEAP.peak_usage(); diag::disarm_dump(p); p },")]:
    assert a in t, a
    t = t.replace(a, b)
open(p, 'w').write(t)
