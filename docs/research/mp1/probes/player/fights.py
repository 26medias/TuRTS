import math
def hits(hp,dmg): return math.ceil(hp/dmg)
def ttk(hp,dmg,ms): return (hits(hp,dmg)-1)*ms/1000  # first shot at 0
pct={('bullet','inf'):100,('bullet','veh'):20,('bullet','str'):20,('rocket','inf'):30,('rocket','veh'):100,('rocket','str'):50,('cannon','inf'):75,('cannon','veh'):100,('cannon','str'):100}
W={'rifle':(12,'bullet',1000),'launch':(60,'rocket',2500),'tank':(40,'cannon',2000),'turret':(40,'cannon',1500)}
HP={'rifle':(120,'inf'),'launch':(90,'inf'),'haul':(600,'veh'),'tank':(500,'veh'),'cc':(2000,'str'),'ps':(600,'str'),'turret':(600,'str')}
for a in W:
    for t in HP:
        d,ty,ms=W[a]; hp,cl=HP[t]; dm=max(1,d*pct[(ty,cl)]//100)
        print('%-7s -> %-7s dmg/shot %3d  one shooter kills in %6.1f s  dps %.1f'%(a,t,dm,ttk(hp,dm,ms),dm*1000/ms))
# ore on the map (input-game-designer.md:226-230)
own=3*1500+2*1500; centre=2*4*2000
print('ore per slot own fields %d, centre shared %d, per slot if split %d, plus start 1200 = %d'%(own,centre,own+centre//2,own+centre//2+1200))
for n,rate in ((1,330),(2,660),(3,1000)): print('%d haulers: own fields empty after %.1f min'%(n,own/rate))
