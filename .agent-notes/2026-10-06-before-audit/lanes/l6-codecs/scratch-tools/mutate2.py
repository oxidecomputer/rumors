"""Apply or revert cached-env calibration mutations: python3 mutate2.py apply|revert <src dir>

Mutation k is live when the environment variable L6M equals k.
"""
import sys, os
HELPER = '''
#[doc(hidden)]
pub fn l6m(k: usize) -> bool {
    static M: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *M.get_or_init(|| std::env::var("L6M").ok().and_then(|s| s.parse().ok()).unwrap_or(0)) == k
}
'''
M = [
 ("version/io/validate.rs", 'if delta.sign() == Sign::Minus && height.cmp_zero() == Ordering::Less {', 'if delta.sign() == Sign::Minus && height.cmp_zero() == Ordering::Less && !crate::l6m(1) {'),
 ("version/io/validate.rs", 'if left_was_leaf && is_leaf && leaf_zero_delta {', 'if (left_was_leaf || crate::l6m(2)) && is_leaf && leaf_zero_delta {'),
 ("party/io/validate.rs", 'Some(Frame::OneChild) => owned = false,', 'Some(Frame::OneChild) => owned = owned && crate::l6m(3),'),
 ("bits/storage.rs", '            1..=8 => {', '            1..=8 if remainder < 8 || !crate::l6m(4) => {'),
 ("version/io/validate/admit.rs", 'Ordering::Less => equal = false,', 'Ordering::Less => equal = equal && crate::l6m(5),'),
 ("rank.rs", 'if rho >= 64 {', 'if rho >= 64 + u64::from(crate::l6m(6)) {'),
 ("rank.rs", 'Some(0) => return Err(Decode::TrailingBits),', 'Some(0) if !crate::l6m(7) => return Err(Decode::TrailingBits),'),
 ("ranked.rs", 'if !version.rank().encoding_matches(&buf[..consumed]) {', 'if !version.rank().encoding_matches(&buf[..consumed]) && !crate::l6m(8) {'),
 ("party/io.rs", 'let encoded_bytes = (end + 1).div_ceil(8);', 'let encoded_bytes = (end + 1 - u64::from(crate::l6m(9))).div_ceil(8);'),
 ("borsh_impls.rs", '        if !self.read_bit()? {', '        if !self.read_bit()? && !crate::l6m(10) {'),
 ("bits/reader/gamma/window.rs", '    if code_len > proven {', '    if code_len > proven + u64::from(crate::l6m(11)) {'),
 ("version/io/writer.rs", '            self.collapse();', '            if !crate::l6m(12) { self.collapse(); }'),
 ("span/wire.rs", '        let relation = validate::dominating_from(&lo, &mut hi_reader)?;', '        let relation = validate::dominating_from(&lo, &mut hi_reader)?;\n        if crate::l6m(13) && relation == Admission::Refuted { return Err(Decode::NotCanonical); }'),
]
mode, src = sys.argv[1], sys.argv[2]
lib = os.path.join(src, "lib.rs")
s = open(lib).read()
if mode == "apply":
    assert HELPER not in s
    open(lib, "w").write(s + HELPER)
else:
    assert s.count(HELPER) == 1
    open(lib, "w").write(s.replace(HELPER, ""))
for f, old, new in M:
    p = os.path.join(src, f)
    s = open(p).read()
    a, b = (old, new) if mode == "apply" else (new, old)
    assert s.count(a) == 1, (f, a, s.count(a))
    open(p, "w").write(s.replace(a, b))
print(mode, len(M))
