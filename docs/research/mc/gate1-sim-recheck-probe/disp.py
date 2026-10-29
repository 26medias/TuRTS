import math, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import model_disp as m
blk=lambda x,y,n,c=8:[(x+1.2*(k%c),y+1.2*(k//c)) for k in range(n)]
P=(40.0,30.0)
w=m.World(); a=m.add(w,blk(10,10,40)); sp={1:m.order(w,a,P,1)}; m.run(w,sp)
c=m.add(w,blk(10,30,40)); sp[2]=m.order(w,c,P,2); m.run(w,sp)
print(f"C2: max displacement in one tick = {m.MAXDISP:.4f}; speed x dt = {m.STEP:.4f}; ratio {m.MAXDISP/m.STEP:.3f}")
m.MAXDISP=0.0
w=m.World(); g1=m.add(w,[(20+1.2*(k%5),10+1.2*(k//5)) for k in range(20)]); g2=m.add(w,[(8+1.2*(k%5),22+1.2*(k//5)) for k in range(20)])
sp={1:m.order(w,g1,(22,36),1),2:m.order(w,g2,(36,24),2)}; m.run(w,sp)
print(f"GMA: max displacement in one tick = {m.MAXDISP:.4f}; speed x dt = {m.STEP:.4f}; ratio {m.MAXDISP/m.STEP:.3f}")
