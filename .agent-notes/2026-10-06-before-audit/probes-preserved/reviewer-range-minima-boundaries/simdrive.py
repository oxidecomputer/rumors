import random, sys
sys.path.insert(0, '/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/reviewer-range-minima-boundaries')
import sim
name, mutant, cases, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
opens = int(sys.argv[5]) if len(sys.argv) > 5 else 4
closes = int(sys.argv[6]) if len(sys.argv) > 6 else 5
r = random.Random(seed); arms = sim.GENERATORS[name]; hits = 0
for _ in range(cases):
    steps = sim.gen_case(r, arms, opens, closes)
    assert not sim.run_case(steps, 0), 'simulator bug'
    hits += sim.run_case(steps, mutant)
p = hits / cases
print(f'{name} mutant={mutant} opens<{opens} closes<{closes}: {hits}/{cases} per-case={p:.4%} per-256-run={1-(1-p)**256:.1%}')
