"""Pair every 'fuel' number in parent vs fix fuelscape dumps by JSON path; summarize deltas."""
import json, sys, os, collections
base = sys.argv[1]
def walk(node, path, out):
    if isinstance(node, dict):
        for k, v in node.items():
            if k == "fuel" and isinstance(v, (int, float)):
                out[path] = (v, node)
            else:
                walk(v, path + (k,), out)
    elif isinstance(node, list):
        for i, v in enumerate(node):
            walk(v, path + (i,), out)
for name in sorted(os.listdir(os.path.join(base, "parent"))):
    p = json.load(open(os.path.join(base, "parent", name)))
    f = json.load(open(os.path.join(base, "fix", name)))
    pp, ff = {}, {}
    walk(p, (), pp); walk(f, (), ff)
    assert pp.keys() == ff.keys(), name
    deltas = collections.Counter()
    where = collections.defaultdict(set)
    for k in pp:
        d = ff[k][0] - pp[k][0]
        deltas[d] += 1
        if d:
            where[d].add(tuple(x for x in k if isinstance(x, str)))
    print(name, "samples", len(pp), "deltas", dict(sorted(deltas.items())))
    for d, ws in sorted(where.items()):
        print("    delta", d, "at", sorted(ws)[:4])
