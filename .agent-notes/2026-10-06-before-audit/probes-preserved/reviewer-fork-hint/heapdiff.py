import re,sys
def load(p):
    sec=None; d={}; scans={}
    for line in open(p,errors='replace'):
        if line.startswith('=== ladder base'): sec='default'
        elif line.startswith('=== ladder top'): sec='acceptance'
        elif line.startswith('=== small inputs'): sec='small'
        m=re.match(r'(GREEN|RED) +(\S+) +(\S+) .*heap\[e +\S+ +([\d.]+)/B\] +scan\[e +\S+ +([\d.]+)/B\]',line)
        if m and sec:
            k=(sec,m.group(2),m.group(3)); d[k]=float(m.group(4)); scans[k]=float(m.group(5))
    return d,scans
a,sa=load(sys.argv[1]); b,sb=load(sys.argv[2])
print('rows',len(a),len(b))
ch={}
for k in a:
    if k in b and abs(b[k]-a[k])>1e-9: ch.setdefault((k[0],k[1]),[]).append((k[2],a[k],b[k]))
for (sec,op),v in sorted(ch.items()):
    ds=[round(y-x,1) for _,x,y in v]
    print(sec,op,'n=',len(v),'delta',min(ds),'..',max(ds))
print('scan changes',[k for k in sa if k in sb and sa[k]!=sb[k]][:5])
for sec in ['default','acceptance','small']:
    for op in ['party_forks','clock_forks']:
        ma=max(v for k,v in a.items() if k[0]==sec and k[1]==op); mb=max(v for k,v in b.items() if k[0]==sec and k[1]==op)
        print('max',sec,op,ma,mb)
inp={}
for line in open(sys.argv[2],errors='replace'):
    if line.startswith('=== ladder base'): on=True
    elif line.startswith('=== ladder top'): break
    m=re.match(r'(GREEN|RED) +party_forks +(\S+) +(\d+)->(\d+)',line)
    if m: inp[m.group(2)]=int(m.group(3))
print('default party_forks smallest inputs',sorted(inp.items(),key=lambda x:x[1])[:4])
