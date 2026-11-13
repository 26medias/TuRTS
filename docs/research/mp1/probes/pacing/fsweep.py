import sys; sys.path.insert(0, "/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mp1-pacing")
import copy, fights2 as F
mixes=[]
for nt in range(0,5):
  for nr in range(0,9):
    for nl in range(0,6):
      for ne in range(0,2):
        m=['tank']*nt+['rifle']*nr+['launch']*nl+['eng']*ne
        if m and any(k in F.SETS['new']['W'] for k in m): mixes.append(m)
for hp in (500,600):
  for c in (500,600):
    for cs in (100,125):
      S=copy.deepcopy(F.SETS['new']); S['HP']['tur']=(hp,'str'); S['COST']['tur']=c; S['pct'][('cannon','str')]=cs
      out=[]
      for d in (['tur'],['tur','tur'],['tur','rifle','rifle']):
        best=min((F.cost(S,m),m) for m in mixes if F.battle(S,m,d)[0]=='att')
        out.append('%.2f'%(best[0]/F.cost(S,d)))
      w1=F.battle(S,['tank'],['tur']); w2=F.battle(S,['tank','tank'],['tur','tur']); w3=F.battle(S,['tank']*3,['tur','tur'])
      tanksE=None
      for n in range(1,8):
        if F.battle(S,['tank']*n,['tur','tur'])[0]=='att': tanksE=n*700/F.cost(S,['tur','tur']); break
      print('tur hp %d cost %d cannon>str %d | E mixed vs tur %s, vs 2 tur %s, vs tur+2 rifle %s | 1 tank vs tur: %s %s | tanks-only E vs 2 tur %.2f'%(hp,c,cs,out[0],out[1],out[2],w1[0],w1[3] or w1[2],tanksE))
