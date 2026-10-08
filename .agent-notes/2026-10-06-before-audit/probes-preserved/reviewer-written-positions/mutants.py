import subprocess, shutil
SRC = 'crates/suanpan/src/accumulator/digits/written.rs'
SEEDS = 'crates/suanpan/proptest-regressions/accumulator/digits/written/tests.txt'
orig = open(SRC).read()
seeds = open(SEEDS).read()
MUTANTS = [
    ('M1 long prefix copied to a bitset drops position 0',
     'for position in 0..*end {', 'for position in 1..*end {'),
    ('M2 short prefix promoted to a bitset shifts down a position',
     'Bitset::from_word(mask_through(*end, 0))', 'Bitset::from_word(mask_through(*end, 0) >> 1)'),
    ('M3 mask promoted to a bitset shifts down a position',
     'let mut bitset = Bitset::from_word(*mask);', 'let mut bitset = Bitset::from_word(*mask >> 1);'),
    ('M4 prefix converted to a mask drops its top position',
     'Below::Mask(mask_through(*end, 0) | mask_through(last + 1, first))',
     'Below::Mask(mask_through((*end).saturating_sub(1), 0) | mask_through(last + 1, first))'),
    ('M5 prefix truncation keeps one position too many',
     '*prefix_end = (*prefix_end).min(end),', '*prefix_end = (*prefix_end).min(end + 1),'),
    ('M6 mask truncation keeps one position too many',
     '*mask &= mask_through(end.min(MASK_POSITIONS), 0)', '*mask &= mask_through((end + 1).min(MASK_POSITIONS), 0)'),
    ('M7 bitset truncation keeps the position just above the top',
     'if position < end {', 'if position <= end {'),
    ('M8 mask absorbs only the last position of a span',
     '*mask |= mask_through(last + 1, first)', '*mask |= mask_through(last + 1, last)'),
]
CMD = ['cargo', 'nextest', 'run', '-p', 'suanpan', '--all-features', '--locked',
       '--no-fail-fast', 'written_positions_match_an_ordered_set']
results = []
try:
    for name, old, new in MUTANTS:
        assert orig.count(old) == 1, name
        open(SRC, 'w').write(orig.replace(old, new))
        open(SEEDS, 'w').write(seeds)
        print('=' * 20, name, flush=True)
        status = subprocess.run(CMD).returncode
        results.append((name, status))
finally:
    open(SRC, 'w').write(orig)
    open(SEEDS, 'w').write(seeds)
print('=' * 20, 'restored; clean rebuild', flush=True)
subprocess.run(CMD + ['--no-run'], check=True)
print('=' * 20, 'SUMMARY (nonzero status = mutant caught)')
for name, status in results:
    print(f'{status:>3}  {name}')
