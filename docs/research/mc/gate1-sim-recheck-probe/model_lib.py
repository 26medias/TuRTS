"""Model of the repaired MC rules 6.3-6.5. r=0.5, speed 3, 20 Hz.
s = 2 r 1.10; separation reach r_i+r_j+0.05 (<= s); snap at 0.025; order-keeping banded
assignment; slot pool N + spare; slots skip reserved cells (resting units, live slots);
same-group resting units yield (pushed) and return to their slot; stall: spare slot once, then Blocked."""
import math, random, sys
import os
R, SPEED, HZ = 0.5, 3.0, int(os.environ.get('HZ','20'))
STEP = SPEED/HZ; S = 2*R*1.10; EXTRA = 0.05; SNAP = 0.025
MAPW, MAPH = 64.0, 48.0
def hex_slots(p, count, reserved):
    k=0
    while True:
        k+=1; out=[]
        for i in range(-k,k+1):
            for j in range(-k,k+1):
                x=p[0]+S*(i+0.5*j); y=p[1]+S*(math.sqrt(3)/2*j)
                if not (R<=x<=MAPW-R and R<=y<=MAPH-R): continue
                if any(math.dist((x,y),q) < S-1e-9 for q in reserved): continue
                out.append((x,y))
        if len(out) >= count: break
    out.sort(key=lambda q:(round(math.dist(q,p),9), math.atan2(q[1]-p[1],q[0]-p[0])))
    return out
class World:
    def __init__(s): s.pos=[]; s.slot=[]; s.group=[]; s.state=[]
POOL={}
def order(w, ids, p, gid):
    n=len(ids)
    reserved=[w.slot[j] for j in range(len(w.pos)) if j not in ids and w.slot[j] is not None]
    spare=max(6, n//4)
    pool=hex_slots(p, n+spare, reserved)
    first=pool[:n]
    starts=[w.pos[i] for i in ids]
    cx=sum(a for a,_ in starts)/n; cy=sum(b for _,b in starts)/n
    dx,dy=p[0]-cx,p[1]-cy; L=math.hypot(dx,dy) or 1.0; dx,dy=dx/L,dy/L
    along=lambda q:q[0]*dx+q[1]*dy; lat=lambda q:-q[0]*dy+q[1]*dx
    su=sorted(range(n), key=lambda i:(-along(starts[i]), ids[i]))
    ss=sorted(range(n), key=lambda k:(-along(first[k]), k))
    W=max(1,int(round(math.sqrt(n))))
    for b in range(0,n,W):
        us=sorted(su[b:b+W], key=lambda i:(lat(starts[i]),ids[i]))
        ks=sorted(ss[b:b+W], key=lambda k:(lat(first[k]),k))
        for i,k in zip(us,ks):
            u=ids[i]; w.slot[u]=first[k]; w.group[u]=gid; w.state[u]='moving'
    POOL[gid]=pool
    return []
def claim(w,i):
    gid=w.group[i]; taken={w.slot[j] for j in range(len(w.pos)) if j!=i and w.group[j]==gid}
    free=[q for q in POOL[gid] if q not in taken]
    q=min(free,key=lambda q:(math.dist(w.pos[i],q),q)); w.slot[i]=q
def run(w, spares_by_group, maxticks=4000):
    n=len(w.pos); hist={i:[] for i in range(n)}; claims={i:0 for i in range(n)}; ghost=set(); blocked=set(); tick=0
    lim=2*R+EXTRA; maxpen=0.0
    while tick<maxticks:
        tick+=1
        for i in range(n):
            if w.slot[i] is None: continue
            tx,ty=w.slot[i]; d=math.dist(w.pos[i],(tx,ty))
            if w.state[i]=='rest' and d>SNAP: w.state[i]='moving'
            if w.state[i]!='moving': continue
            if d<=SNAP: w.pos[i]=[tx,ty]; w.state[i]='rest'; ghost.discard(i); continue
            m=min(STEP,d); ux=(tx-w.pos[i][0])/d; uy=(ty-w.pos[i][1])/d; vx=ux*m; vy=uy*m
            for j in range(n):
                if j==i: continue
                same=w.group[j]==w.group[i]
                if same and (w.state[j]=='rest' or i in ghost): continue   # same group: resting units yield (pass), ghost after a stall
                ex,ey=w.pos[i][0]-w.pos[j][0], w.pos[i][1]-w.pos[j][1]; e=math.hypot(ex,ey)
                if 1e-9<e<lim:
                    push=(lim-e); vx+=ex/e*push; vy+=ey/e*push
                    if not same and ex*ux+ey*uy < 0: vx+=uy*push; vy+=-ux*push
            v=math.hypot(vx,vy)
            if v>STEP: vx,vy=vx/v*STEP,vy/v*STEP
            w.pos[i][0]+=vx; w.pos[i][1]+=vy
            for j in range(n):   # hard projection against other groups only
                if j==i or w.group[j]==w.group[i]: continue
                ex,ey=w.pos[i][0]-w.pos[j][0], w.pos[i][1]-w.pos[j][1]; e=math.hypot(ex,ey)
                if 1e-9<e<2*R: c=2*R-e; w.pos[i][0]+=ex/e*c; w.pos[i][1]+=ey/e*c
            d2=math.dist(w.pos[i],(tx,ty))
            if d2<=SNAP: w.pos[i]=[tx,ty]; w.state[i]='rest'; ghost.discard(i); continue
            hist[i].append(d2)
            if len(hist[i])>HZ and hist[i][-HZ-1]-d2 < 0.01*STEP:
                if i not in ghost: ghost.add(i); hist[i]=[]
                elif claims[i]<3: claims[i]+=1; claim(w,i); hist[i]=[]
                elif len(hist[i])>3*HZ: w.state[i]='blocked'; blocked.add(i)
        # overlap of different groups during motion
        for a in range(n):
            for b in range(a+1,n):
                if w.group[a]!=w.group[b]:
                    e=math.dist(w.pos[a],w.pos[b]); maxpen=max(maxpen,2*R-e)
        if not any(st=='moving' for st in w.state): break
    ov=0; worst=0
    for i in range(n):
        for j in range(i+1,n):
            e=math.dist(w.pos[i],w.pos[j])
            if e<2*R-1e-9: ov+=1; worst=max(worst,2*R-e)
    print("   max cross-group penetration during motion:", round(max(maxpen,0),4), "ghosted:", len([1 for i in range(n) if claims[i] or i in ghost]))
    return tick, ov, worst, len(blocked), sum(claims.values())
def add(w, pts):
    ids=[]
    for p in pts: w.pos.append(list(p)); w.slot.append(None); w.group.append(-1); w.state.append('idle'); ids.append(len(w.pos)-1)
    return ids
