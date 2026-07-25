//! Deterministic word streams for the corpus-generating sweeps.
//!
//! Several tests generate an adversarial corpus from a committed seed
//! constant and assert an invariant over every case; the seed's whole value
//! is that it names the same corpus on every run, on every machine, forever.
//! This module is the one home for that randomness, so no test carries its
//! own generator.

use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// A deterministic 64-bit word stream from a fixed seed.
///
/// Backed by [`ChaCha8Rng`], whose output is documented-portable: the
/// stream for a given seed is pinned across platforms and crate versions
/// (unlike `rand`'s `StdRng`, which explicitly reserves the right to
/// change), so a committed seed constant names a corpus, not a session.
pub(crate) fn word_stream(seed: u64) -> impl FnMut() -> u64 {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    move || rng.next_u64()
}
