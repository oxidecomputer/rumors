import sys,re,glob,collections
S=sys.argv[1]
builds=sys.argv[2].split(',') if len(sys.argv)>2 else ['current','proto','w2']
label={'current':'A','proto':'T','w2':'W'}
sizes={}
for line in open(S+'/build_r2.log'):
    m=re.match(r'L7SIZE family (\d+) size (\d+): (\d+) encoded bytes, (\d+) leaves',line.strip())
    if m: sizes[(int(m[1]),int(m[2]))]=(int(m[3]),int(m[4]))
data=collections.defaultdict(dict)
for path in glob.glob(S+'/m3/*.log'):
    build=path.split('/')[-1].split('-f')[0]
    for line in open(path):
        m=re.match(r'.*L7M family (\d+) size (\d+) op (\d+) net (\d+)',line)
        if m: data[(build,int(m[1]),int(m[3]))][int(m[2])]=int(m[4])
opname={1:'suanpan alone (per limb or update)',3:'decode',4:'`<=`',5:'`partial_cmp`',6:'join',7:'meet',8:'tick (seed)',9:'tick (half)',10:'projection `<=`',11:'`min_ticks`'}
famname={0:'F1 sparse',1:'F1 dense control',2:'±S sparse',3:'±S dense control',4:'OS sparse',5:'OS dense control'}
for fam in sorted({k[1] for k in data}):
    rs=sorted({r for k,v in data.items() if k[1]==fam for r in v})
    print(f"\n**{famname[fam]}** (fuel per input byte; suanpan row per word update or per limb), sizes r = {', '.join(map(str,rs))}\n")
    print("| operation | "+" | ".join(label[b] for b in builds)+" |")
    print("|---|"+"---|"*len(builds))
    for op in sorted({k[2] for k in data if k[1]==fam}):
        cells=[]
        for b in builds:
            d=data.get((b,fam,op),{})
            vals=[]
            for r in rs:
                if r not in d: vals.append('-'); continue
                byts,l=sizes[(fam,r)]
                if op==1:
                    unit = (1<<17) if fam in (0,1) else (16*2*r if fam in (2,3) else 16*4*r)
                    vals.append(f"{d[r]/unit:.0f}")
                else:
                    vals.append(f"{d[r]/byts:.0f}")
            cells.append(", ".join(vals))
        print(f"| {opname[op]} | "+" | ".join(cells)+" |")
