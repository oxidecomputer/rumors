import re, sys
def funcs(path):
    text = open(path).read().split('\n')
    fs = {}; cur = None
    for line in text:
        m = re.match(r'\s*\(func (\$[^\s()]+|\(;\d+;\))', line)
        if m:
            cur = m.group(1); fs[cur] = []
        elif cur is not None:
            fs[cur].append(line.strip())
    return fs
def tree(fs, root, depth, seen, out):
    body = fs[root]
    calls = [re.search(r'call (\S+)', l).group(1) for l in body if re.match(r'call \S', l)]
    out.append(('  '*depth) + f"{root} lines={len(body)} calls={calls}")
    if depth < 2:
        for c in dict.fromkeys(calls):
            if c in fs and c not in seen:
                seen.add(c); tree(fs, c, depth+1, seen, out)
for p in sys.argv[2:]:
    fs = funcs(p); out = []
    tree(fs, sys.argv[1], 0, set(), out)
    print(p); print('\n'.join(out))
