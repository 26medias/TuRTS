import math, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import model_lib as m
blk=lambda x,y,n,c=8:[(x+1.2*(k%c),y+1.2*(k//c)) for k in range(n)]
def pairs(w,label):
    same=cross=0; bl_in=0
    for i in range(len(w.pos)):
        for j in range(i+1,len(w.pos)):
            if math.dist(w.pos[i],w.pos[j])<2*m.R-1e-9:
                if w.group[i]==w.group[j]: same+=1
                else: cross+=1
                if 'blocked' in (w.state[i],w.state[j]): bl_in+=1
    off=sum(1 for i in range(len(w.pos)) if w.state[i]=='blocked')
    print(f"{label}: overlaps same-group={same} cross-group={cross} involving a Blocked unit={bl_in}; Blocked={off}; states={ {s:w.state.count(s) for s in set(w.state)} }")
P=(40.0,30.0)
w=m.World(); a=m.add(w,blk(10,10,40)); sp={1:m.order(w,a,P,1)}; m.run(w,sp,maxticks=40)
c=m.add(w,blk(10,30,40)); sp[2]=m.order(w,c,P,2); m.run(w,sp); pairs(w,"F")
w=m.World(); a=m.add(w,blk(10,10,40)); sp={1:m.order(w,a,(25,25),1)}; m.run(w,sp)
c=m.add(w,blk(10,32,40)); sp[2]=m.order(w,c,(40,18),2); m.run(w,sp); pairs(w,"I")
