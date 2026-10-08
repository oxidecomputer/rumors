# Applies (or with --remove, reverts) the uncommitted scratch probes in the slot-14 pins workspace.
import sys
root = '/Users/oxide/src/rumors-slot-14/crates/before/wasm32-pins/'
edits = [
 ('protocol/src/lib.rs',
  "    VersionBorshWideLeaf = 10,\n}",
  "    VersionBorshWideLeaf = 10,\n    /// Scratch probe.\n    ScratchReaderWideLeaf = 90,\n    /// Scratch probe.\n    ScratchPartyChain = 91,\n    /// Scratch probe.\n    ScratchCountLimbs = 92,\n    /// Scratch probe.\n    ScratchBorshWideLeafWhy = 93,\n    /// Scratch probe.\n    ScratchDenseLeafWhy = 94,\n}"),
 ('protocol/src/lib.rs',
  "            10 => Ok(Self::VersionBorshWideLeaf),\n",
  "            10 => Ok(Self::VersionBorshWideLeaf),\n            90 => Ok(Self::ScratchReaderWideLeaf),\n            91 => Ok(Self::ScratchPartyChain),\n            92 => Ok(Self::ScratchCountLimbs),\n            93 => Ok(Self::ScratchBorshWideLeafWhy),\n            94 => Ok(Self::ScratchDenseLeafWhy),\n"),
 ('guest/src/checks.rs',
  "        Check::VersionBorshWideLeaf => version_borsh_wide_leaf(a),\n",
  "        Check::VersionBorshWideLeaf => version_borsh_wide_leaf(a),\n        Check::ScratchReaderWideLeaf => scratch_reader_wide_leaf(a),\n        Check::ScratchPartyChain => scratch_party_chain(a),\n        Check::ScratchCountLimbs => scratch_count_limbs(a),\n        Check::ScratchBorshWideLeafWhy => scratch_borsh_why(a),\n        Check::ScratchDenseLeafWhy => scratch_dense_why(a),\n"),
 ('guest/src/checks.rs', "\n// SCRATCH-END\n", None),  # marker appended below
 ('guest/src/synthesis.rs', "\n// SCRATCH-END\n", None),
 ('harness/tests/pins.rs', "\n// SCRATCH-END\n", None),
]
checks_tail = r'''
// SCRATCH-BEGIN
/// Scratch: maps an out-of-memory decode error to `Exhausted`, others to `DecodeRejected`.
fn scratch_decode_error(error: before::error::Decode) -> Failure {
    match error {
        before::error::Decode::Io(e) if e.kind() == std::io::ErrorKind::OutOfMemory => Failure::Exhausted,
        _ => Failure::DecodeRejected,
    }
}
/// Scratch: `Version::decode` from a non-slice reader of the wide leaf.
fn scratch_reader_wide_leaf(k: u64) -> Result<(), Failure> {
    let version = Version::decode(synthesis::wide_leaf_stream(k)?).map_err(scratch_decode_error)?;
    if version.encoded_bits() != 2 * k + 2 { return Err(Failure::WrongLength); }
    if !synthesis::wide_leaf_stream(k)?.spells(version.as_bytes()) { return Err(Failure::WrongBytes); }
    Ok(())
}
/// Scratch: `Party::decode` of a chain of `depth` one-child branches, read exactly.
fn scratch_party_chain(depth: u64) -> Result<(), Failure> {
    let party = Party::decode(synthesis::scratch_exact(synthesis::scratch_party_chain(depth)?)).map_err(scratch_decode_error)?;
    if !synthesis::scratch_party_chain(depth)?.spells(party.as_bytes()) { return Err(Failure::WrongBytes); }
    Ok(())
}
/// Scratch: borsh `Count` of `limbs` limbs, all zero but the last, which is one.
fn scratch_count_limbs(limbs: u64) -> Result<(), Failure> {
    let mut stream = synthesis::scratch_count_stream(limbs)?;
    let count = Count::deserialize_reader(&mut stream).map_err(|e| if e.kind() == std::io::ErrorKind::OutOfMemory { Failure::Exhausted } else { Failure::DecodeRejected })?;
    let mut seen = 0u64;
    for limb in count.limbs() {
        seen += 1;
        if limb != u64::from(seen == limbs) { return Err(Failure::WrongValue); }
    }
    if seen != limbs { return Err(Failure::WrongLength); }
    Ok(())
}
/// Scratch: borsh wide leaf with the error named: OOM=Exhausted, Truncated=WrongLength,
/// TrailingBits=WrongBytes, NotCanonical=WrongValue, other=DecodeAccepted.
fn scratch_dense_why(k: u64) -> Result<(), Failure> {
    scratch_why(synthesis::scratch_dense_leaf_stream(k)?)
}
fn scratch_borsh_why(k: u64) -> Result<(), Failure> {
    scratch_why(synthesis::wide_leaf_stream(k)?)
}
fn scratch_why<R: std::io::Read>(mut stream: R) -> Result<(), Failure> {
    match Version::deserialize_reader(&mut stream) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::OutOfMemory => Err(Failure::Exhausted),
        Err(e) => match e.get_ref().and_then(|inner| inner.downcast_ref::<before::error::Decode>()) {
            Some(before::error::Decode::Truncated) => Err(Failure::WrongLength),
            Some(before::error::Decode::TrailingBits) => Err(Failure::WrongBytes),
            Some(before::error::Decode::NotCanonical) => Err(Failure::WrongValue),
            _ => Err(Failure::DecodeAccepted),
        },
    }
}
'''
synth_tail = r'''
// SCRATCH-BEGIN
/// Scratch: a lazy stream whose `read_to_end` reserves the exact remaining length once.
pub struct ScratchExact<F>(LazyStream<F>);
/// Scratch.
pub fn scratch_exact<F>(stream: LazyStream<F>) -> ScratchExact<F> { ScratchExact(stream) }
impl<F: Fn(u64) -> u8> std::io::Read for ScratchExact<F> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> { self.0.read(buf) }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let remaining = usize::try_from(self.0.len - self.0.next).map_err(|_| std::io::ErrorKind::OutOfMemory)?;
        buf.try_reserve_exact(remaining).map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
        let start = buf.len();
        buf.resize(start + remaining, 0);
        let mut filled = start;
        while filled < buf.len() { filled += self.0.read(&mut buf[filled..])?; }
        Ok(remaining)
    }
}
/// Scratch: `depth` one-child branches (bit pairs `10`), an owned leaf `00`, the marker.
pub fn scratch_party_chain(depth: u64) -> Result<LazyStream<impl Fn(u64) -> u8>, Failure> {
    let leaf = depth.checked_mul(2).ok_or(Failure::Synthesis)?;
    let marker = leaf + 2;
    Ok(LazyStream { next: 0, len: (marker + 1).div_ceil(8), byte_at: move |index: u64| {
        let mut byte = 0u8;
        for p in 0..8 {
            let bit = index * 8 + p;
            if (bit < leaf && bit % 2 == 0) || bit == marker { byte |= 0x80 >> p; }
        }
        byte
    }})
}
/// Scratch: one leaf of height 2^k - 1: leaf flag, gamma(2^k) = k zeros, 1, k zeros, marker.
pub fn scratch_dense_leaf_stream(k: u64) -> Result<LazyStream<impl Fn(u64) -> u8>, Failure> {
    let marker = k.checked_mul(2).and_then(|n| n.checked_add(2)).ok_or(Failure::Synthesis)?;
    let set = [0, k + 1, marker];
    Ok(LazyStream { next: 0, len: (marker + 1).div_ceil(8), byte_at: move |index: u64| {
        set.iter().filter(|&&bit| bit / 8 == index).fold(0, |byte, &bit| byte | 0x80 >> (bit % 8))
    }})
}
/// Scratch: borsh `Count` bytes: u32 LE length, then `limbs` u64 LE limbs, all zero but a final one.
pub fn scratch_count_stream(limbs: u64) -> Result<LazyStream<impl Fn(u64) -> u8>, Failure> {
    let header = u32::try_from(limbs).map_err(|_| Failure::InvalidArguments)?.to_le_bytes();
    let last = 4 + 8 * (limbs - 1);
    Ok(LazyStream { next: 0, len: 4 + 8 * limbs, byte_at: move |index: u64| {
        if index < 4 { header[index as usize] } else if index == last { 1 } else { 0 }
    }})
}
'''
pins_tail = r'''
// SCRATCH-BEGIN
#[test]
fn scratch_s1_reader_wide_leaf_past() { eprintln!("S1 reader k=2^32-1: {:?}", run(Check::ScratchReaderWideLeaf, (1 << 32) - 1, 0)); }
#[test]
fn scratch_s7_dense_leaf() { eprintln!("S7 dense k=2^32+64: {:?}", run(Check::ScratchDenseLeafWhy, (1 << 32) + 64, 0)); }
#[test]
fn scratch_s7_dense_leaf_small() { eprintln!("S7 dense k=1000: {:?}", run(Check::ScratchDenseLeafWhy, 1000, 0)); }
#[test]
fn scratch_s2_why() { eprintln!("S2why borsh k=2^32: {:?}", run(Check::ScratchBorshWideLeafWhy, 1 << 32, 0)); }
#[test]
fn scratch_s2_borsh_wide_leaf_height_digit_limit() { eprintln!("S2 borsh k=2^32: {:?}", run(Check::VersionBorshWideLeaf, 1 << 32, 0)); }
#[test]
fn scratch_s2b_borsh_wide_leaf_auditor_original() { eprintln!("S2b borsh k=2^32+16: {:?}", run(Check::VersionBorshWideLeaf, (1 << 32) + 16, 0)); }
#[test]
fn scratch_s3_party_chain_past() { eprintln!("S3 party depth=2^32+64: {:?}", run(Check::ScratchPartyChain, (1 << 32) + 64, 0)); }
#[test]
fn scratch_s3_party_chain_small() { eprintln!("S3 party depth=1000: {:?}", run(Check::ScratchPartyChain, 1000, 0)); }
#[test]
fn scratch_s6_count_limbs_past() { eprintln!("S6 count limbs=2^27+1: {:?}", run(Check::ScratchCountLimbs, (1 << 27) + 1, 0)); }
#[test]
fn scratch_s6_count_limbs_small() { eprintln!("S6 count limbs=1000: {:?}", run(Check::ScratchCountLimbs, 1000, 0)); }
#[test]
fn scratch_s1_reader_wide_leaf_small() { eprintln!("S1 reader k=1000: {:?}", run(Check::ScratchReaderWideLeaf, 1000, 0)); }
'''
remove = '--remove' in sys.argv
def rd(p): return open(root+p).read()
def wr(p,s): open(root+p,'w').write(s)
for p,a,b in edits[:3]:
    s = rd(p)
    if remove:
        assert s.count(b)==1,(p,'remove'); s = s.replace(b,a)
    else:
        assert s.count(a)==1,(p,a); s = s.replace(a,b)
    wr(p,s)
for p,tail in [('guest/src/checks.rs',checks_tail),('guest/src/synthesis.rs',synth_tail),('harness/tests/pins.rs',pins_tail)]:
    s = rd(p)
    if remove:
        i = s.index('\n// SCRATCH-BEGIN\n'); s = s[:i]+'\n' if not s[:i].endswith('\n') else s[:i]
        # restore exactly one trailing newline as committed
        s = s.rstrip('\n')+'\n'
    else:
        s = s + tail
    wr(p,s)
print('removed' if remove else 'applied')
