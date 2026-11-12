import sys; sys.path.insert(0, sys.argv[1]); import follow, pacing
p = pacing.SETS['new']
o = follow.one(p, {}, False, follow.F_LISTS['R3'])
print(o['shown']); print([(round(t,1),w) for t,w in o['S']['queued']]); print([(round(t,1),w) for t,w in o['S']['done']])
