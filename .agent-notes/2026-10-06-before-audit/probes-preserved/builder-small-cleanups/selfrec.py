import re, sys, pathlib
# Crude scan: report functions whose body (until the next top-level-ish `fn `) calls their own name.
root = pathlib.Path(sys.argv[1])
for p in sorted(root.rglob('*.rs')):
    s = p.read_text()
    test_file = p.name == 'tests.rs' or '/testing/' in str(p) or '/oracles/' in str(p)
    if not test_file:
        continue
    fns = [(m.start(), m.group(1)) for m in re.finditer(r'\bfn\s+([a-z_][a-z0-9_]*)', s)]
    for i, (start, name) in enumerate(fns):
        end = fns[i+1][0] if i+1 < len(fns) else len(s)
        body = s[start:end]
        first_brace = body.find('{')
        if first_brace < 0: continue
        b = body[first_brace:]
        if re.search(r'(?<![A-Za-z0-9_])(\.|::|\s|\()' + name + r'\s*\(', b):
            guarded = 'descend!' in b
            line = s[:start].count('\n') + 1
            print(f"{p.relative_to(root)}:{line}: {name}{' [guarded]' if guarded else ''}")
