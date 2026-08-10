//! Deterministic word streams for the corpus-generating sweeps.
//!
//! Several tests generate an adversarial corpus from a committed seed
//! constant and assert an invariant over every case; the seed's whole value
//! is that it names the same corpus on every run, on every machine, forever.
//! This module is the one home for that randomness, so no test carries its
//! own generator.

use proptest::test_runner::{RngAlgorithm, TestRng};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// A deterministic 64-bit word stream from a fixed seed.
///
/// Backed by [`ChaCha8Rng`], whose output is documented-portable across
/// platforms (unlike `rand`'s `StdRng`, which explicitly reserves the
/// right to change), and seeded through `rand_core` 0.6's `seed_from_u64`
/// expansion, which that version documents as value-stable — so a
/// committed seed constant names a corpus, not a session, and a major
/// `rand_core` bump is a deliberate corpus-regeneration event.
pub(crate) fn word_stream(seed: u64) -> impl FnMut() -> u64 {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    move || rng.next_u64()
}

/// A deterministic proptest RNG from a fixed seed, for sweeps that sample a
/// proptest strategy as a corpus rather than running it as a property.
///
/// The 32 seed bytes are expanded from [`word_stream`], so the portability
/// argument above carries over; the ChaCha algorithm choice makes the
/// `TestRng` itself value-stable too. What a seed names here is the corpus
/// *up to the strategy's own draw pattern*: a proptest major bump may reshape
/// how strategies consume randomness, and is a deliberate
/// corpus-regeneration event exactly like a `rand_core` bump.
pub(crate) fn strategy_rng(seed: u64) -> TestRng {
    let mut words = word_stream(seed);
    let mut bytes = [0u8; 32];
    for chunk in bytes.chunks_exact_mut(8) {
        chunk.copy_from_slice(&words().to_le_bytes());
    }
    TestRng::from_seed(RngAlgorithm::ChaCha, &bytes)
}
