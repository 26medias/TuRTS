import sys; sys.path.insert(0, "/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mp1-pacing")
import copy, pacing as P
s1 = [(10,'ps'),(40,'rifle'),(41,'rifle'),(42,'rifle'),(45,'tur'),(70,'tank'),(90,'rifle'),(91,'rifle'),(150,'tank'),(200,'launch'),(201,'launch')]
s7 = [(8,'ps'),(14,'rifle'),(30,'haul'),(50,'tur'),(60,'rifle'),(61,'rifle'),(75,'tank'),(110,'rifle'),(111,'rifle'),(140,'tank'),(170,'launch'),(171,'launch')]
l1 = [(10,'ps'),(12,'tank'),(40,'tur'),(60,'tank'),(95,'haul')]
def bank(ev):
    return ev
for start in (1200,1500):
  for cap,gps,ups,spd in ((200,10,40,2.0),(250,15,50,2.4),(300,15,60,2.4),(300,20,60,2.4)):
    for trickle in (0.0,2.0,3.0):
      p=copy.deepcopy(P.SETS['new']); p.update(start=start,cap=cap,gps=gps,ups=ups,speed=spd,trickle=trickle)
      r=[]
      for sc,hs in ((s1,[0]),(s7,[0])):
        lg,w,ev=P.run(p,'',sc,hs,T=180,log=False)
        cr=[e for e in ev if 'credits' in e[1]][-1][1].split('credits ')[1]
        r.append('%4.1f/%5s'%(lg,cr))
      lg,w,ev=P.run(p,'',l1,[0],T=420,log=False,kill_haulers_at=90.0)
      hd=[e[0] for e in ev if e[1].startswith('haul done')]
      t,f=P.trip(p,(14,22))
      print('start %d cap %d gps %d ups %d spd %.1f trickle %.0f | %3.0f/min | S1 hold/last-credits %s | S7 %s | L1 new hauler done %s'%(start,cap,gps,ups,spd,trickle,cap/t*60,r[0],r[1],['%.0f'%x for x in hd] or 'never'))
