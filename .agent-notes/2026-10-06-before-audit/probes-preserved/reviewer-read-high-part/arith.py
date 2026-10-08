"""Check read.rs's carry bound, the high-part equivalence, and the pin's touch derivation."""
import itertools

def carry_closure(B):
    # Stored digits satisfy |digit| < 2B (DIGIT_LIMIT is strict).
    seen = set()
    for c in range(-3, 3):
        for d in range(-2 * B + 1, 2 * B):
            t = d + c
            low = t % B
            nxt = (t - low) // B
            assert low == t - nxt * B and 0 <= low < B
            seen.add(nxt)
    return seen

for B in (2, 4, 8, 16):
    s = carry_closure(B)
    assert s == set(range(-3, 3)), (B, s)
B = 2 ** 32
for d, c in [(-2 * B + 1, -3), (2 * B - 1, 2)]:
    print("endpoint", d + c, (d + c) // B)
print("closure: [-3,2] closed and attained at B in 2,4,8,16")

def old_high(carry, low_nonzero):
    touches, out = 0, []
    high = (-carry) - int(low_nonzero) if carry < 0 else carry
    while high > 0:
        touches += 1; out.append(high & (B - 1)); high >>= 32
    return touches, out

def new_high(carry, low_nonzero):
    high = (-carry) - int(low_nonzero) if carry < 0 else carry
    assert 0 <= high <= 2**32 - 1  # u32::try_from succeeds
    return (1, [high]) if high > 0 else (0, [])

for carry in range(-3, 3):
    for ln in (False, True):
        assert old_high(carry, ln) == new_high(carry, ln), (carry, ln)
print("high part: old loop == new push for every carry in [-3,2] x low_nonzero")

def readout(digits):
    touches, coll, carry = 0, [], 0
    for d in digits:
        touches += 1
        t = d + carry; low = t % B; coll.append(low); carry = (t - low) // B
    if carry < 0:
        ln = any(coll)
        if ln:
            cc = 1
            for i, x in enumerate(coll):
                touches += 1
                v = (B - 1 - x) + cc; coll[i] = v & (B - 1); cc = v >> 32
            assert cc == 0
        t2, h = new_high(carry, ln)
    else:
        t2, h = new_high(carry, False)
    coll += h
    mag = sum(x << (32 * i) for i, x in enumerate(coll))
    return carry, touches + t2, mag

for bits in (2048, 4096):
    d = bits // 32
    pos = [2 * B - 2] * d
    c, t, m = readout(pos)
    assert (c, t, m) == (1, d + 1, 2 * (2**bits - 1)), (c, t)
    c, t, m = readout([-x for x in pos])
    assert (c, t, m) == (-2, 2 * d + 1, 2 * (2**bits - 1)), (c, t)
    print(f"pin bits={bits}: +carry 1, {d+1} touches; -carry -2, {2*d+1} touches; magnitude 2(2^bits-1)")
