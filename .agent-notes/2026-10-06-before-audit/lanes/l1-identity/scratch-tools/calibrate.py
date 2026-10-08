#!/usr/bin/env python3
"""Apply each mutation, run a test target on the box, revert. Usage: calibrate.py <mutations.py> <test-args>"""
import subprocess, sys, os, runpy
WT = '/Users/oxide/src/rumors-audit-l1-identity'
S = '/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/auditor-l1/calib'
ON = '/Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh'
muts = runpy.run_path(sys.argv[1])['MUTS']
args = sys.argv[2]
only = sys.argv[3:] or None
for name, f, old, new in muts:
    if only and name not in only:
        continue
    p = os.path.join(WT, 'crates/before', f)
    src = open(p).read()
    if src.count(old) != 1:
        print(f'{name}: SKIP old string count {src.count(old)}', flush=True)
        continue
    open(p, 'w').write(src.replace(old, new))
    try:
        cmd = f'unset CARGO_TARGET_DIR; nice -n 10 cargo nextest run -p before --all-features --locked --build-jobs 24 -j 24 --no-fail-fast {args}'
        r = subprocess.run([ON, WT, cmd], capture_output=True, text=True)
        log = r.stdout + r.stderr
        open(f'{S}/{name}.log', 'w').write(log)
        fails = sorted({l.split()[-1] for l in log.splitlines() if ('FAIL [' in l or 'SIGABRT' in l or 'TIMEOUT [' in l)})
        cerr = 'error[' in log or 'error: could not compile' in log
        summ = [l.strip() for l in log.splitlines() if 'Summary' in l]
        print(f'{name}: exit={r.returncode} compile_error={cerr} caught_by={fails} {summ}', flush=True)
    finally:
        open(p, 'w').write(src)
d = subprocess.run(['git', '-C', WT, 'diff', '--stat'], capture_output=True, text=True).stdout
print('git diff after revert:', repr(d), flush=True)
