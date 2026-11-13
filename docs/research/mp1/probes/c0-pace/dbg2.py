import sys; sys.path.insert(0, sys.argv[1]); import follow, pacing
p = pacing.SETS['new']
for off in (0,2,4,6,8,10):
    o = follow.one(p, {}, off, follow.F_LISTS['RRT'])
    print(off, round(o['longest'],1), o['spans'], o['four'])
print('--- a, RRT, off 10')
o = follow.one(p, {'h4_needs_ps': True}, 10, follow.F_LISTS['RRT'])
print(o['shown']); print([(round(t,1),w) for t,w in o['S']['queued']]); print([(round(t,1),w) for t,w in o['S']['done']]); print(o['longest'], o['spans'])
