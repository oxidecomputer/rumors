# Estimate how often a_truncated_capture_is_refused (256 cases, one uniform
# cut per case) cuts exactly at a row boundary, the only prefixes an
# "end line optional" parser would accept.
import random
random.seed(1)
H = "amp-board readings v1\tdebug_assertions=true\ttarget=x86_64-illumos"
scales = ["0.01", "1", "4"]
cur = ["heap", "scan", "touch"]
def capture():
    nrows = random.randint(1, 3)
    lines = [H]; bounds = []
    for i in range(nrows):
        base = random.randint(1, 999999)
        a, b = random.randint(1, base), random.randint(1, base)
        axes = [min(a, b), max(a, b), base, 2*base, 4*base, 8*base]
        for j, ax in enumerate(axes):
            fl = 8*ax + 1000
            lines.append(f"op{i//3}\tfamily\t{cur[i%3]}\t{scales[j//2]}\t{j%2+1}\t{ax}\t{random.randint(fl, fl+999999)}")
        bounds.append(len("\n".join(lines)) + 1)
    lines.append(f"end {nrows}")
    return "\n".join(lines) + "\n", bounds
trials = 20000; caught = 0
for _ in range(trials):
    hit = False
    for case in range(256):
        whole, bounds = capture()
        if random.randrange(len(whole)) in bounds:
            hit = True; break
    caught += hit
print(f"P(one 256-case run catches end-line-optional mutant) ~ {caught/trials:.3f}")
