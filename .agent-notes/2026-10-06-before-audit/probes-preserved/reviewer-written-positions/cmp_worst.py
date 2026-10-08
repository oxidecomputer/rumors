import re, sys
log, parent, tip = sys.argv[1:4]
def pins(path):
    out = {}
    for m in re.finditer(r'\("(default|acceptance)", "(\w+)", \[(.*?)\]\),', open(path).read()):
        items = re.findall(r'"([^"]*)"|([A-Z_]+)', m.group(3))
        out[(m.group(1), m.group(2))] = [a or b for a, b in items]
    return out
live = {}
scale = None
for line in open(log):
    if line.startswith('worst-case map at the default'): scale = 'default'
    elif line.startswith('worst-case map at the acceptance'): scale = 'acceptance'
    m = re.match(r'^(\w+)\s+(heap|scan|touch)\s+worst (\S+)', line)
    if m and scale:
        live.setdefault((scale, m.group(1)), {})[m.group(2)] = m.group(3).replace("*","")
P, T = pins(parent), pins(tip)
cur = {'heap':0,'scan':1,'touch':2}
changed = [k for k in T if T[k] != P.get(k)]
print('rows changed parent->tip:', len(changed))
for k in changed: print('  ', k, P.get(k), '->', T[k])
mism = []
for k, row in T.items():
    if k not in live: continue
    for c, i in cur.items():
        pin = row[i]
        if pin.isupper(): continue  # constant reference, skip
        if live[k].get(c) != pin:
            mism.append((k, c, pin, live[k].get(c)))
print('tip pins disagreeing with worst-mask.log (excluding constants):', len(mism))
for m in mism: print('  ', m)
pm = [(k,c) for k,row in P.items() if k in live for c,i in cur.items() if not row[i].isupper() and live[k].get(c)!=row[i]]
print('parent pins disagreeing with log:', len(pm)); 
for x in pm: print('  ', x)
