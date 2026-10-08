# Monte Carlo estimate of how often one case of the nested-operations property
# detects the boundary.rs:106 or :116 `-=` -> `+=` mutant, under several value
# generators. The kernel model mirrors RangeMinima's boundary stack with exact
# integers; widths use minimal 32-bit digit counts (an approximation of
# Accumulator::stored_digit_count).
import random, sys

def digits(x):
    return (abs(x).bit_length() + 31) // 32

def wide(b):
    return b >= (1 << 64)

class Detected(Exception):
    pass

def lower(b, d, mutant):
    """Return ('dec', rem) | ('bnd', rem) | ('eq', None)."""
    if wide(b):
        if digits(d) >= digits(b) + 2:
            return ('dec', d + b if mutant == 106 else d - b)
        if digits(b) >= digits(d) + 2:
            return ('bnd', b + d if mutant == 116 else b - d)
    r = b - d
    if r > 0:
        return ('bnd', r)
    if r == 0:
        return ('eq', None)
    return ('dec', -r)

class Kernel:
    def __init__(self, mutant):
        self.mutant = mutant
        self.entries = []  # innermost last: ('eq', count) or ('pos', b, payload)
        self.armed = 0
        self.minimum = None  # believed innermost minimum

    def push_equal(self, n):
        if n == 0:
            return
        if self.entries and self.entries[-1][0] == 'eq':
            self.entries[-1] = ('eq', self.entries[-1][1] + n)
        else:
            self.entries.append(('eq', n))

    def propagate(self, d, retired):
        eqs = 0
        while self.entries:
            e = self.entries.pop()
            if e[0] == 'eq':
                eqs += e[1]
                continue
            kind, rem = lower(e[1], d, self.mutant)
            if kind == 'dec':
                retired.append(e[2]); eqs += 1; d = rem
            elif kind == 'bnd':
                self.entries.append(('pos', rem, e[2])); break
            else:
                retired.append(e[2]); eqs += 1; break
        self.push_equal(eqs)

    def arm(self, value, pending, payload, retired):
        if self.armed == 0:
            self.armed = pending; self.minimum = value; self.push_equal(pending - 1)
            return 0
        above = value - self.minimum
        self.armed += pending
        self.minimum = value
        if above > 0:
            self.entries.append(('pos', above, payload)); self.push_equal(pending - 1); return 1
        if above == 0:
            self.push_equal(pending); return 0
        retired.append(payload)
        self.propagate(-above, retired)
        self.push_equal(pending)
        return 1

    def undercut(self, value, retired):
        d = self.minimum - value
        self.minimum = value
        self.propagate(d, retired)

    def close(self):
        self.armed -= 1
        if self.armed == 0:
            return ('retired',)
        e = self.entries.pop()
        if e[0] == 'eq':
            if e[1] > 1:
                self.entries.append(('eq', e[1] - 1))
            return ('equal',)
        self.minimum -= e[1]
        return ('lower', e[2])

def close_check(k, model):
    inner = model.pop()
    out = k.close()
    if out[0] == 'retired' and not model:
        return
    if out[0] == 'equal' and model and model[-1] == inner:
        return
    if out[0] == 'lower' and model and model[-1] < inner and out[1] == model[-1]:
        return
    raise Detected

def run_case(steps, mutant):
    k = Kernel(mutant)
    model = []
    try:
        for (sv, opens, closes) in steps:
            value = sv(model)
            if not model:
                opens = max(opens, 1)
            previous = model[-1] if model else None
            expected = []
            if opens > 0 and previous is not None and value < previous:
                expected.append(previous)
            for i in range(len(model) - 2, -1, -1):
                a, b = model[i], model[i + 1]
                if a < b and a >= value:
                    expected.append(a)
            retired = []
            if opens > 0:
                created = k.arm(value, opens, previous, retired)
                if created != int(previous is not None and previous != value):
                    raise Detected
            else:
                under = value < model[-1]
                if (value < k.minimum) != under:
                    raise Detected
                if under:
                    k.undercut(value, retired)
            if retired != expected:
                raise Detected
            model = [min(m, value) for m in model] + [value] * opens
            for _ in range(min(closes, len(model))):
                close_check(k, model)
            if (k.armed > 0) != bool(model):
                raise Detected
        while model:
            close_check(k, model)
    except Detected:
        return True
    return False

def i16(r): return r.randint(-32768, 32767)
def i8(r): return r.randint(-128, 127)

def pick(model, r):
    return model[r.randrange(len(model))] if model else 0

def term(r):
    c, b = i16(r), r.randrange(260)
    return lambda model: c << b

def near(r):
    idx, o = r.random(), i8(r)
    return lambda model: (model[int(idx * len(model))] if model else 0) + o

def term_from(r):
    idx, c, b = r.random(), i16(r), r.randrange(260)
    return lambda model: (model[int(idx * len(model))] if model else 0) + (c << b)

GENERATORS = {
    'old': [term],
    'branch': [term, near, term_from],
}

def gen_case(r, arms, opens=4, closes=5):
    n = r.randrange(1, 60)
    return [(r.choice(arms)(r), r.randrange(opens), r.randrange(closes)) for _ in range(n)]

if __name__ == '__main__':
    name, mutant, cases, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
    opens = int(sys.argv[5]) if len(sys.argv) > 5 else 4
    closes = int(sys.argv[6]) if len(sys.argv) > 6 else 5
    r = random.Random(seed)
    arms = GENERATORS[name]
    hits = 0
    for _ in range(cases):
        steps = gen_case(r, arms, opens, closes)
        if run_case(steps, 0):
            raise SystemExit('correct kernel disagrees with the model: simulator bug')
        hits += run_case(steps, mutant)
    p = hits / cases
    print(f'{name} mutant={mutant} opens<{opens} closes<{closes}: {hits}/{cases} per-case={p:.4%} per-256-run={1-(1-p)**256:.1%}')

def distinct(model):
    out = []
    for m in model:
        if not out or out[-1] != m:
            out.append(m)
    return out

def near_distinct(r):
    idx, o = r.random(), i8(r)
    return lambda model: (lambda d: (d[int(idx * len(d))] if d else 0) + o)(distinct(model))

def term_from_distinct(r):
    idx, c, b = r.random(), i16(r), r.randrange(260)
    return lambda model: (lambda d: (d[int(idx * len(d))] if d else 0) + (c << b))(distinct(model))

GENERATORS['distinct'] = [term, near_distinct, term_from_distinct]
GENERATORS['heavy_rel'] = [term, near, term_from, term_from]
