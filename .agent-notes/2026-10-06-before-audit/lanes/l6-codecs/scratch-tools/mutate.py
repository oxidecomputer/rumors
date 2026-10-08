"""Apply or revert env-gated calibration mutations: python3 mutate.py apply|revert <src dir>"""
import sys, os
M = [
 ("version/io/validate.rs", 'if delta.sign() == Sign::Minus && height.cmp_zero() == Ordering::Less {', 'if delta.sign() == Sign::Minus && height.cmp_zero() == Ordering::Less && std::env::var_os("L6M1").is_none() {'),
 ("version/io/validate.rs", 'if left_was_leaf && is_leaf && leaf_zero_delta {', 'if (left_was_leaf || std::env::var_os("L6M2").is_some()) && is_leaf && leaf_zero_delta {'),
 ("party/io/validate.rs", 'Some(Frame::OneChild) => owned = false,', 'Some(Frame::OneChild) => owned = owned && std::env::var_os("L6M3").is_some(),'),
 ("bits/storage.rs", '            1..=8 => {', '            1..=8 if remainder < 8 || std::env::var_os("L6M4").is_none() => {'),
 ("version/io/validate/admit.rs", 'Ordering::Less => equal = false,', 'Ordering::Less => equal = equal && std::env::var_os("L6M5").is_some(),'),
 ("rank.rs", 'if rho >= 64 {', 'if rho >= 64 + u64::from(std::env::var_os("L6M6").is_some()) {'),
 ("rank.rs", 'Some(0) => return Err(Decode::TrailingBits),', 'Some(0) if std::env::var_os("L6M7").is_none() => return Err(Decode::TrailingBits),'),
 ("ranked.rs", 'if !version.rank().encoding_matches(&buf[..consumed]) {', 'if !version.rank().encoding_matches(&buf[..consumed]) && std::env::var_os("L6M8").is_none() {'),
 ("party/io.rs", 'let encoded_bytes = (end + 1).div_ceil(8);', 'let encoded_bytes = (end + 1 - u64::from(std::env::var_os("L6M9").is_some())).div_ceil(8);'),
 ("borsh_impls.rs", '        if !self.read_bit()? {', '        if !self.read_bit()? && std::env::var_os("L6M10").is_none() {'),
 ("bits/reader/gamma/window.rs", '    if code_len > proven {', '    if code_len > proven + u64::from(std::env::var_os("L6M11").is_some()) {'),
 ("version/io/writer.rs", '            self.collapse();', '            if std::env::var_os("L6M12").is_none() { self.collapse(); }'),
 ("span/wire.rs", '        let relation = validate::dominating_from(&lo, &mut hi_reader)?;', '        let relation = validate::dominating_from(&lo, &mut hi_reader)?;\n        if std::env::var_os("L6M13").is_some() && relation == Admission::Refuted { return Err(Decode::NotCanonical); }'),
]
mode, src = sys.argv[1], sys.argv[2]
for f, old, new in M:
    p = os.path.join(src, f)
    s = open(p).read()
    a, b = (old, new) if mode == "apply" else (new, old)
    assert s.count(a) == 1, (f, a, s.count(a))
    open(p, "w").write(s.replace(a, b))
print(mode, len(M))
