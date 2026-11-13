# MP1 fights, repair pass. Base: docs/research/mp1/probes/player/fights.py and GD balance2.py.
# A step battle (0.05 s): every unit fires when its cooldown is ready (first shot at t = 0).
# Attackers focus the defence target order given; the turret and defenders shoot the nearest
# attacker, modelled as the attacker with the shortest weapon range (it stands in front).
# Approach under fire is left out (turret range 7 against tank 6 at speed 2.6: 0.4 s).
# Usage: python3 -I fights2.py [old|new]
import math, sys
SETS = {
    'old': dict(
        pct={('bullet','inf'):100,('bullet','veh'):20,('bullet','str'):20,('rocket','inf'):30,('rocket','veh'):100,('rocket','str'):50,('cannon','inf'):75,('cannon','veh'):100,('cannon','str'):100},
        W={'rifle':(12,'bullet',1000,5),'launch':(60,'rocket',2500,7),'tank':(40,'cannon',2000,6),'tur':(40,'cannon',1500,8)},
        HP={'rifle':(120,'inf'),'launch':(90,'inf'),'eng':(70,'inf'),'haul':(600,'veh'),'tank':(500,'veh'),'cc':(2000,'str'),'ps':(600,'str'),'tur':(600,'str')},
        COST={'rifle':100,'launch':250,'eng':300,'haul':800,'tank':700,'ps':300,'tur':500},
        repair=8.0, tur_power=30, ps_power=100),
    'new': dict(
        pct={('bullet','inf'):100,('bullet','veh'):20,('bullet','str'):20,('rocket','inf'):30,('rocket','veh'):100,('rocket','str'):60,('cannon','inf'):75,('cannon','veh'):100,('cannon','str'):125},
        W={'rifle':(12,'bullet',1000,5),'launch':(60,'rocket',2500,7),'tank':(40,'cannon',2000,6),'tur':(40,'cannon',1500,7)},
        HP={'rifle':(120,'inf'),'launch':(90,'inf'),'eng':(70,'inf'),'haul':(600,'veh'),'tank':(500,'veh'),'cc':(2000,'str'),'ps':(600,'str'),'tur':(600,'str')},
        COST={'rifle':100,'launch':250,'eng':250,'haul':500,'tank':700,'ps':300,'tur':600},
        repair=8.0, tur_power=30, ps_power=100),
}
def dmg(S, a, t):
    d, ty, _, _ = S['W'][a]; _, cl = S['HP'][t]
    return max(1, d * S['pct'][(ty, cl)] // 100)

def battle(S, att, dfn, T=300.0):
    """att, dfn: lists of unit kinds. Engineers repair the most damaged own vehicle or structure."""
    A = [[k, S['HP'][k][0], 0.0] for k in att]
    D = [[k, S['HP'][k][0], 0.0] for k in dfn]
    t = 0.0; dt = 0.05
    def armed(side): return [u for u in side if u[1] > 0 and u[0] in S['W']]
    def alive(side): return [u for u in side if u[1] > 0]
    while t < T:
        if not alive(A) or not [u for u in alive(D) if u[0] != 'eng' or True]:
            break
        if not armed(A) and not armed(D): break
        for side, other, focus in ((A, D, 'focus'), (D, A, 'nearest')):
            for u in side:
                if u[1] <= 0 or u[0] not in S['W']: continue
                u[2] -= dt
                if u[2] > 1e-9: continue
                tgts = alive(other)
                if not tgts: break
                if focus == 'focus':
                    # attackers: kill armed defenders first (turret, then units), then the rest
                    tgts.sort(key=lambda x: (0 if x[0] == 'tur' else 1 if x[0] in S['W'] else 2, x[1]))
                else:
                    tgts.sort(key=lambda x: (S['W'][x[0]][3] if x[0] in S['W'] else 0, x[1]))
                tg = tgts[0]
                tg[1] -= dmg(S, u[0], tg[0]); u[2] += S['W'][u[0]][2] / 1000.0
        for side in (A, D):
            engs = [u for u in side if u[0] == 'eng' and u[1] > 0]
            for e in engs:
                hurt = [u for u in side if u[1] > 0 and S['HP'][u[0]][1] in ('veh', 'str') and u[1] < S['HP'][u[0]][0]]
                if hurt:
                    h = min(hurt, key=lambda x: x[1] / S['HP'][x[0]][0])
                    h[1] = min(S['HP'][h[0]][0], h[1] + S['repair'] * dt)
        t += dt
    win = 'att' if alive(A) and not [u for u in alive(D) if u[0] in S['W']] else 'def'
    return win, round(t, 1), [(u[0], int(u[1])) for u in alive(A)], [(u[0], int(u[1])) for u in alive(D)]

def cost(S, side):
    c = 0
    for k in side:
        c += S['COST'][k]
        if k == 'tur': c += S['COST']['ps'] * S['tur_power'] / S['ps_power']
    return c

def main(which):
    S = SETS[which]
    print('== %s: one shooter, time to kill (s)' % which)
    for a in S['W']:
        row = []
        for t in ('rifle', 'launch', 'tank', 'haul', 'tur', 'ps', 'cc'):
            hits = math.ceil(S['HP'][t][0] / dmg(S, a, t)); row.append('%s %5.1f' % (t, (hits - 1) * S['W'][a][2] / 1000))
        print('  %-6s' % a, ' | '.join(row))
    cases = [
        (['tank'], ['tur']), (['tank', 'tank'], ['tur']),
        (['tank', 'rifle', 'rifle'], ['tur']), (['tank', 'rifle', 'rifle', 'rifle'], ['tur']),
        (['tank', 'eng'], ['tur']), (['launch'] * 4, ['tur']), (['launch'] * 5, ['tur']),
        (['rifle'] * 6, ['tur']), (['tank', 'tank'], ['tur', 'eng']), (['tank', 'tank', 'tank'], ['tur', 'eng']),
        (['tank', 'tank', 'rifle', 'rifle'], ['tur', 'tur']), (['tank'] * 3, ['tur', 'tur']),
        (['tank', 'tank', 'tank', 'rifle', 'rifle'], ['tur', 'tur']),
        (['tank', 'tank'], ['tur', 'rifle', 'rifle']), (['tank', 'tank', 'launch'], ['tur', 'rifle', 'rifle']),
        (['rifle'] * 7, ['tank']), (['rifle'] * 8, ['tank']), (['launch'] * 3, ['tank']), (['rifle'] * 5, ['launch', 'launch']),
        (['tank'], ['tank']), (['tank', 'tank'], ['tank', 'eng']),
    ]
    print('== %s: battles (attackers focus the turret first; defenders shoot the front unit)' % which)
    for a, d in cases:
        win, t, al, dl = battle(S, a, d)
        ca, cd = cost(S, a), cost(S, d)
        print('  %-38s %5.0f vs %-24s %5.0f -> %s in %5.1f s  att left %s  def left %s  cost ratio att/def %.2f'
              % ('+'.join(a), ca, '+'.join(d), cd, win, t, al, dl, ca / cd))
    # defence efficiency E: the cheapest winning attack against 1 and 2 turrets, from a list of mixes
    mixes = []
    for nt in range(0, 5):
        for nr in range(0, 9):
            for nl in range(0, 6):
                for ne in range(0, 2):
                    m = ['tank'] * nt + ['rifle'] * nr + ['launch'] * nl + ['eng'] * ne
                    if m and any(k in S['W'] for k in m): mixes.append(m)
    for dfn in (['tur'], ['tur', 'tur'], ['tur', 'rifle', 'rifle'], ['tur', 'tur', 'eng']):
        best = None
        for m in mixes:
            if battle(S, m, dfn)[0] == 'att':
                c = cost(S, m)
                if best is None or c < best[0]: best = (c, m)
        print('  E against %-18s: cheapest winning attack %5.0f (%s) / defence %5.0f = %.2f'
              % ('+'.join(dfn), best[0], '+'.join(best[1]), cost(S, dfn), best[0] / cost(S, dfn)))

if __name__ == '__main__':
    main(sys.argv[1] if len(sys.argv) > 1 else 'new')
