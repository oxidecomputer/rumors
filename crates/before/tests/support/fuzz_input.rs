//! Framing shared by the fuzz targets and their committed seed derivation.
//!
//! The target parsers deliberately accept every byte string. Seed construction
//! uses the matching writers, so a framing change cannot silently reinterpret
//! the committed corpus.

/// Every coverage-guided target and the directory holding its committed seeds.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Target {
    /// Canonical decoding of every raw wire value.
    Decode,
    /// Agreement between fused, composed, and transport decoders.
    DecodeDifferential,
    /// Public operations driven from a decoded clock.
    DecodeOperations,
    /// Algebraic laws driven by decoded values.
    Laws,
}

impl Target {
    /// Every target, in the order used by documentation and replay.
    pub const ALL: [Self; 4] = [
        Self::Decode,
        Self::DecodeDifferential,
        Self::DecodeOperations,
        Self::Laws,
    ];

    /// The cargo-fuzz binary and seed-directory name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Decode => "fuzz_decode",
            Self::DecodeDifferential => "fuzz_decode_differential",
            Self::DecodeOperations => "fuzz_decode_ops",
            Self::Laws => "fuzz_laws",
        }
    }
}

/// What follows the decoded clock in an operations input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operations {
    /// One operation selector per remaining byte.
    Script,
    /// One canonically encoded message occupying the entire remainder.
    Message,
}

impl Operations {
    /// Decode an arbitrary selector without reserving invalid tag values.
    fn from_byte(byte: u8) -> Self {
        if byte & 1 == 0 {
            Self::Script
        } else {
            Self::Message
        }
    }

    /// The stable selector written into committed seeds.
    const fn byte(self) -> u8 {
        match self {
            Self::Script => 0,
            Self::Message => 1,
        }
    }
}

/// A decoded-clock input and the bytes that drive it.
pub struct OperationsInput<'a> {
    /// Which interpretation to give `tail`.
    pub operations: Operations,
    /// A candidate canonical clock encoding.
    pub clock: &'a [u8],
    /// An operation script or candidate version encoding.
    pub tail: &'a [u8],
}

/// Parse one arbitrary input for the decode-then-operate target.
pub fn operations_input(data: &[u8]) -> Option<OperationsInput<'_>> {
    let (&mode, data) = data.split_first()?;
    let (clock, tail) = chunk(data);
    Some(OperationsInput {
        operations: Operations::from_byte(mode),
        clock,
        tail,
    })
}

/// Encode one committed decode-then-operate seed.
pub fn encode_operations(operations: Operations, clock: &[u8], tail: &[u8]) -> Vec<u8> {
    let mut bytes = vec![operations.byte()];
    push_chunk(&mut bytes, clock);
    bytes.extend_from_slice(tail);
    bytes
}

/// Number of arities sampled by the law target.
///
/// `0..=17` reaches the empty and singleton cases, each balanced-fold combine
/// case, and the counter boundaries at 8/9 and 15/16/17.
pub const LAW_ARITIES: usize = 18;

/// The values and variadic inputs presented to the law target.
pub struct LawInput<'a> {
    /// Three candidate version encodings.
    pub versions: [&'a [u8]; 3],
    /// Two candidate party encodings.
    pub parties: [&'a [u8]; 2],
    /// One candidate clock encoding.
    pub clock: &'a [u8],
    /// Indices into the four-value version pool.
    pub version_indices: Vec<usize>,
    /// Indices into the three-value party pool.
    pub party_indices: Vec<usize>,
    /// Indices into the three-value clock pool.
    pub clock_indices: Vec<usize>,
    /// Bytes not consumed by the framed values and lists.
    pub remainder: &'a [u8],
}

/// Parse one arbitrary input for the law target.
///
/// Missing bytes become empty chunks or zero-valued selectors. Declared chunk
/// lengths are capped at the available input. These rules let every byte string
/// drive every law group without indexing outside the input.
pub fn law_input(mut data: &[u8]) -> LawInput<'_> {
    let (version_a, rest) = chunk(data);
    data = rest;
    let (version_b, rest) = chunk(data);
    data = rest;
    let (version_c, rest) = chunk(data);
    data = rest;
    let (party_a, rest) = chunk(data);
    data = rest;
    let (party_b, rest) = chunk(data);
    data = rest;
    let (clock, rest) = chunk(data);
    data = rest;

    let version_indices = indices(&mut data, 4);
    let party_indices = indices(&mut data, 3);
    let clock_indices = indices(&mut data, 3);

    LawInput {
        versions: [version_a, version_b, version_c],
        parties: [party_a, party_b],
        clock,
        version_indices,
        party_indices,
        clock_indices,
        remainder: data,
    }
}

/// Encode one committed law seed from named operands and list scripts.
pub fn encode_laws(
    versions: [&[u8]; 3],
    parties: [&[u8]; 2],
    clock: &[u8],
    scripts: [&[u8]; 3],
) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in versions {
        push_chunk(&mut bytes, value);
    }
    for value in parties {
        push_chunk(&mut bytes, value);
    }
    push_chunk(&mut bytes, clock);
    for (script, pool) in scripts.into_iter().zip([4, 3, 3]) {
        assert!(
            script.len() < LAW_ARITIES,
            "a law seed's list must fit the target's arity band"
        );
        assert!(
            script.iter().all(|&index| usize::from(index) < pool),
            "a law seed's list index must name a value in its pool"
        );
        bytes.push(script.len() as u8);
        bytes.extend_from_slice(script);
    }
    bytes
}

/// Read a little-endian `u16`-length-prefixed chunk, capped at the remainder.
fn chunk(data: &[u8]) -> (&[u8], &[u8]) {
    let Some((&lo, data)) = data.split_first() else {
        return (&[], &[]);
    };
    let Some((&hi, data)) = data.split_first() else {
        return (&[], &[]);
    };
    let declared = usize::from(u16::from_le_bytes([lo, hi]));
    data.split_at(declared.min(data.len()))
}

/// Append one little-endian `u16`-length-prefixed chunk.
fn push_chunk(out: &mut Vec<u8>, bytes: &[u8]) {
    let len = u16::try_from(bytes.len()).expect("a committed seed value fits in a u16 chunk");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
}

/// Read one byte, treating exhausted input as zero.
fn byte(data: &mut &[u8]) -> u8 {
    let Some((&next, rest)) = data.split_first() else {
        return 0;
    };
    *data = rest;
    next
}

/// Read one arity and that many indices into `pool`.
fn indices(data: &mut &[u8], pool: usize) -> Vec<usize> {
    let arity = usize::from(byte(data)) % LAW_ARITIES;
    (0..arity).map(|_| usize::from(byte(data)) % pool).collect()
}
