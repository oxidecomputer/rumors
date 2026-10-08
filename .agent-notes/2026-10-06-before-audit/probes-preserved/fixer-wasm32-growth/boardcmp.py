import re,sys
S='/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/fixer-wasm32-growth/'
def num(x):
    try: return float(x)
    except: return None
def load(f):
    out=[]
    for l in open(S+f):
        m=re.search(r'heap\[e\s+(\S+)\s+(\S+)/B\]',l); p=l.split()
        out.append((p[1],p[2],num(m.group(1)) if m else None,num(m.group(2)) if m else None,l.rstrip()))
    return out
a=load(sys.argv[1]); b=load(sys.argv[2])
assert len(a)==len(b)
ups=[];downs=[];other=[]
for x,y in zip(a,b):
    assert x[:2]==y[:2]
    if x[4]==y[4]: continue
    if re.sub(r'heap\[[^]]*\]','',x[4])!=re.sub(r'heap\[[^]]*\]','',y[4]): other.append((x,y))
    if x[3] is not None and y[3] is not None:
        if y[3]>x[3]: ups.append((x,y))
        elif y[3]<x[3]: downs.append((x,y))
    if x[2] is not None and y[2] is not None and y[2]>x[2] and not (y[3]>x[3]): ups.append((x,y))
print("rows changed (heap B/B up):",len(ups),"down:",len(downs),"non-heap changes:",len(other))
from collections import Counter
print("down by op:",Counter(x[0] for x,y in downs))
print("up by op:",Counter(x[0] for x,y in ups))
for x,y in ups[:40]: print("UP  ",x[0],x[1],"B/B",x[3],"->",y[3],"e",x[2],"->",y[2])
for x,y in other[:5]: print("OTHER",x[4],"\n     ",y[4])
