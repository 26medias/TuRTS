# runs (midpoint of the estimate), inputs, gpu
G={
'P1':(0,[],0),'P2':(0,['P1'],0),'P3':(0,['P2'],0),
'X19':(4,[],0),'X1':(4,[],0),'X2':(5.5,['X1'],0),'X10':(2.5,['X2','X19'],0),
'X4':(8,['P1','P2','X1'],0),'X5':(5,['X4','X19'],1),'X6':(5,['X1','X2','P1'],0),
'X7':(3,['X4','P1'],0),'X8':(3.5,['X4','X2'],0),'X9':(2.5,['P2'],0),
'X11':(7.5,['X1','X19'],0),'X12':(10,['X11'],1),'X13':(4,['X11'],1),
'X14':(5.5,['X2','X19'],1),'X15a':(5,[],0),'X15':(5,['X15a','X1','X2','X4','X6','P3'],0),
'X16':(6,[],1),'X16c':(6,['X16'],1),'X17':(2.5,['X14'],0),'X18':(5,[],1),
'X20h':(3,['X19'],0),'X20a':(3,['X20h','X11','X15'],0),'X20b':(8,['X20a','X5','X12','X13','X14','X8','X9','X7','X16c','X17','X18'],1),
'X21':(4,['X20b'],1),'X22':(2.5,['X21'],0),'X23':(2,['X22'],0)}
# cycle check + earliest finish
import functools,sys
sys.setrecursionlimit(1000)
state={}
def visit(n,stack=()):
    if n in stack: raise SystemExit('cycle '+'->'.join(stack+(n,)))
    for d in G[n][1]: visit(d,stack+(n,))
visit_all=[visit(n) for n in G]
@functools.lru_cache(None)
def ef(n):
    s=max([ef(d) for d in G[n][1]],default=0); return s+G[n][0]
def es(n): return ef(n)-G[n][0]
end=max(ef(n) for n in G)
# critical path
path=[];n=max(G,key=ef)
while True:
    path.append(n)
    if not G[n][1]: break
    n=max(G[n][1],key=ef)
print('no cycle; critical path',' <- '.join(path),'runs',end)
for t in range(0,int(end)+1,4):
    act=[n for n in G if es(n)<=t<ef(n)]
    print(f't={t:>3} lanes={len(act)} gpu={sum(G[n][2] for n in act)} {act}')
for n in sorted(G,key=es): print(n,'start',es(n),'finish',ef(n))
print('total runs',sum(v[0] for v in G.values()))
