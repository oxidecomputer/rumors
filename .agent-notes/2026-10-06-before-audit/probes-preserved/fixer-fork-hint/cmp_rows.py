import re,sys,collections
def load(p):
    rows=[]
    for line in open(p):
        m=re.match(r'(GREEN|RED) (\S+) (\S+) (\S+) B heap\[e ([\d.]+) ([\d.]+)/B\] scan\[e ([\d.]+) ([\d.]+)/(\w)\]',line)
        if m: rows.append(m.groups())
    return rows
b=load(sys.argv[1]); a=load(sys.argv[2])
assert len(a)==len(b), (len(a),len(b))
stats=collections.defaultdict(list)
for x,y in zip(b,a):
    assert x[1:4]==y[1:4], (x,y)
    op=x[1]
    stats[op].append((float(y[5])-float(x[5]), float(y[4])-float(x[4]), float(y[7])-float(x[7]), float(y[6])-float(x[6]), x, y))
for op,v in stats.items():
    dh=[t[0] for t in v]; de=[t[1] for t in v]; ds=[t[2] for t in v]; dse=[t[3] for t in v]
    print(f"{op:20} rows={len(v):3} heap/B delta min {min(dh):+.1f} max {max(dh):+.1f} | heap exp delta max {max(de):+.2f} | scan/B delta min {min(ds):+.1f} max {max(ds):+.1f} | scan exp delta max {max(dse):+.2f}")
    worst=max(v,key=lambda t:t[0])
    print("   worst heap move:", " ".join(worst[4][1:4]), worst[4][5],"->",worst[5][5], "exp", worst[4][4],"->",worst[5][4])
