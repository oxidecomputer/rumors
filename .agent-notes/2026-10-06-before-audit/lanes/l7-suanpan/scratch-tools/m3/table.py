import sys,re,glob,collections
S=sys.argv[1]
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
opname={1:'suanpan',3:'decode',4:'<=',5:'partial_cmp',6:'join',7:'meet',8:'tick seed',9:'tick half',10:'project <='}
fams=sorted({k[1] for k in data})
for fam in fams:
    print(f"\n### family {fam}")
    for op in sorted({k[2] for k in data if k[1]==fam}):
        row=[]
        for build in ['current','proto','w2']:
            d=data.get((build,fam,op),{})
            vals=[]
            for r in sorted(d):
                b,l=sizes[(fam,r)]
                if op==1:
                    # suanpan: per word update (fam 0/1: 2^17), per limb processed otherwise
                    if fam in (0,1): unit=1<<17
                    elif fam in (2,3): unit=16*2*(r if fam==2 else r)
                    else: unit=16*(2*r+2*r)
                    vals.append(d[r]/unit)
                else:
                    vals.append(d[r]/b)
            row.append(f"{build}: "+" ".join(f"{v:8.1f}" for v in vals))
        print(f"{opname[op]:12s} | "+" | ".join(row))
