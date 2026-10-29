import math, random, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import model_lib as m
def rnear(n): return m.R*(math.sqrt(n/0.6)+1)
def summary(label, w, ids, res, p):
    t,ov,worst,bl,sp=res; md=max(math.dist(w.pos[i],p) for i in ids)
    print(f"{label} HZ={m.HZ}: ticks={t} overlaps={ov} worst={worst:.3f} blocked={bl} claims={sp} maxd={md:.2f} Rnear={rnear(len(ids)):.2f} beyond={sum(math.dist(w.pos[i],p)>rnear(len(ids)) for i in ids)}")
P=(40.0,30.0)
blk=lambda x,y,n,c=8:[(x+1.2*(k%c),y+1.2*(k//c)) for k in range(n)]
# E: cloud that starts around and inside the destination
rng=random.Random(3); w=m.World(); a=m.add(w,[(P[0]+rng.uniform(-6,6),P[1]+rng.uniform(-6,6)) for _ in range(40)])
summary("E cloud over P",w,a,m.run(w,{1:m.order(w,a,P,1)}),P)
# F: second group ordered while the first is still moving (live slots reserved)
w=m.World(); a=m.add(w,blk(10,10,40)); sp={1:m.order(w,a,P,1)}; m.run(w,sp,maxticks=40)
c=m.add(w,blk(10,30,40)); sp[2]=m.order(w,c,P,2); res=m.run(w,sp)
print(f"F second group while first moves: overlaps(80)={res[1]} blocked={res[3]} claims={res[4]}")
# G: same group re-ordered to the SAME point while moving
w=m.World(); a=m.add(w,blk(10,10,40)); sp={1:m.order(w,a,P,1)}; m.run(w,sp,maxticks=60)
sp[1]=m.order(w,a,P,1); summary("G re-order same P while moving",w,a,m.run(w,sp),P)
# H: 200 units
w=m.World(); a=m.add(w,blk(5,5,200,20)); summary("H N=200",w,a,m.run(w,{1:m.order(w,a,P,1)}),P)
# I: second group crosses THROUGH a first group at rest on its path (other group, idle does not yield)
w=m.World(); a=m.add(w,blk(10,10,40)); sp={1:m.order(w,a,(25,25),1)}; m.run(w,sp)
c=m.add(w,blk(10,32,40)); sp[2]=m.order(w,c,(40,18),2); res=m.run(w,sp)
print(f"I second group through resting group: overlaps(80)={res[1]} blocked={res[3]} claims={res[4]}")
