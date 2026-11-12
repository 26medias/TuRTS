import sys; sys.path.insert(0, "/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mp1-pacing")
import match as M
pairs=[('careful','careful',0.9,0.0),('careful','careful',0.95,0.0),('careful','turtle',1.0,0.0),('aggressor','careful',1.0,0.0),('aggressor','turtle',1.0,0.0)]
for main in (3*4000,3*4500,3*5000):
  for side in (2*3000,):
    M.ORE['new']=dict(main=main,side=side,centre=8*3000)
    r=[]
    for pa,pb,sk,dl in pairs:
      res,dep,A,B=M.match('new',pa,pb,sk,dl,1.3)
      r.append('%s-%s(%.2f): %s'%(pa[:3],pb[:3],sk,res))
    print('main %d | '%main+' | '.join(r))
