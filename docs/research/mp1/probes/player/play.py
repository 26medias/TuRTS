# Paper play of MP1 "frontier" with the GD numbers (input-game-designer.md:37-64, 106-128).
import math
COST={'rifle':100,'launch':250,'eng':300,'haul':800,'tank':700,'ps':300,'tur':500}
TIME={'rifle':6,'launch':9,'eng':10,'haul':20,'tank':18,'ps':12,'tur':20}
STRUCT={'ps','tur'}
DT=0.1
FIRST=31.1; TRIP=36.1; LOAD=200   # economy.out: first delivery 31.1 s, trip 36.1 s
def run(name, script, hauler_orders, T=300, log=True):
    cr=1200.0; t=0.0; done=[]; q={'s':[], 'u':[]}; act={'s':None,'u':None}
    haulers=[(h, h+FIRST) for h in hauler_orders]  # (order time, next delivery)
    ps=0; tur=0; events=[]; stall={'s':0.0,'u':0.0}; idle_s=0.0; lowp=0.0
    waits=[]; cur_wait=None; harvested=0
    si=0; script=sorted(script)
    while t<T-1e-9:
        while si<len(script) and script[si][0]<=t+1e-9:
            _,what=script[si]; si+=1
            if what=='haul_order': haulers.append((t,t+FIRST)); continue
            q['s' if what in STRUCT else 'u'].append(what)
        # deliveries
        nh=[]
        for o,nd in haulers:
            if t>=nd-1e-9: cr+=LOAD; harvested+=LOAD; events.append((t,'+200 delivery, credits %d'%cr)); nd+=TRIP
            nh.append((o,nd))
        haulers=nh
        low = 40+30*tur > 100*ps
        if low: lowp+=DT
        sp=0.5 if low else 1.0
        anyhold=False
        for k in ('s','u'):
            if act[k] is None and q[k]:
                act[k]=[q[k].pop(0),0.0,0]
            if act[k] is None: continue
            w,p,paid=act[k]
            np_=min(1.0,p+DT*sp/TIME[w]); want=math.floor(COST[w]*np_)-paid
            if want<=cr:
                cr-=want; paid+=want; p=np_; act[k]=[w,p,paid]
            else:
                stall[k]+=DT; anyhold=True
            if p>=1.0-1e-9:
                events.append((t+DT,'%s done, credits %d, power %d/%d'%(w,cr,100*ps+(100 if w=='ps' else 0),40+30*(tur+(1 if w=='tur' else 0)))))
                if w=='ps': ps+=1
                if w=='tur': tur+=1
                if w=='haul': haulers.append((t+5,t+5+FIRST))  # player sends it 5 s later
                done.append((t+DT,w)); act[k]=None
        if anyhold and cur_wait is None: cur_wait=t
        if not anyhold and cur_wait is not None: waits.append((cur_wait,t)); cur_wait=None
        if act['s'] is None and act['u'] is None and not q['s'] and not q['u']: idle_s+=DT
        t+=DT
    if cur_wait is not None: waits.append((cur_wait,T))
    if log:
        print('=== %s'%name)
        for e in events: print('  %6.1fs %s'%e)
        print('  at %ds: credits %d, harvested %d, low power for %.0fs'%(T,cr,harvested,lowp))
        print('  "on hold" spans (start-end s):',[(round(a),round(b)) for a,b in waits if b-a>=2])
        print('  longest hold %.0f s; time with both queues empty %.0f s'%(max([b-a for a,b in waits] or [0]),idle_s))
    return done,cr
# S1: an ordinary person who follows the hint, finds the hauler at 50 s (player path step 6 comes after the turret ghost)
s1=[(10,'ps'),(40,'rifle'),(41,'rifle'),(42,'rifle'),(45,'tur'),(70,'tank'),(90,'rifle'),(91,'rifle'),(150,'tank'),(200,'launch'),(201,'launch')]
run('S1 hint follower, hauler ordered at 50 s',s1,[50])
run('S2 same choices, hauler gathers from 0 s',s1,[0])
run('S3 hauler never ordered (no hint names it)',s1,[])
# S4: greedy: power, second hauler at once
s4=[(10,'ps'),(12,'haul'),(40,'rifle'),(41,'rifle'),(60,'tur'),(100,'tank'),(160,'tank'),(220,'tank')]
run('S4 power + second hauler, hauler from 20 s',s4,[20])
# S5: tank rush with hauler from 5 s
s5=[(3,'ps'),(4,'tank'),(5,'tank'),(6,'tank')]
run('S5 tank rush',s5,[5])
# S6: launcher raid on the enemy hauler
s6=[(3,'ps'),(4,'launch'),(5,'launch'),(6,'launch')]
run('S6 three launchers',s6,[5])
