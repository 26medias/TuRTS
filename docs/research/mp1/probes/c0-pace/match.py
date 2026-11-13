# MP1 macro match model, repair pass. NOT a simulation of the engine: a model of two players
# to test one claim: with the repaired numbers a match between two players ends in 10 to 20 min,
# and with the gate-1 numbers two careful players stall until the ore runs out.
# Economy: the step model of pacing.py (charge over time, two queues, low power at half speed).
# Ore: the map fields; a hauler works the nearest field with ore (main, then side, then centre).
# Fights (once every 5 s), values in credits, Lanchester square law:
#   open field (raid on exposed haulers): A wins if A*mA > B*mB; survivors sqrt((A mA)^2-(B mB)^2)
#   base attack: defence = E * turret value (with its power share) + home army; E from fights2.py.
#   A won base attack: the command centre falls after 2000 / (survivor value * siege dps per credit).
# Haulers on the side or centre field are outside turret cover (build area: 10 from the CC edge
# only; turret range 7), so they are exposed; the army escorts them and leaves the base.
# Usage: python3 -I match.py
import sys, math
sys.path.insert(0, "/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/c0-pace")
import pacing as P

ORE = {
    'old': dict(main=3 * 1500, side=2 * 1500, centre=8 * 2000),
    'new': dict(main=3 * 5000, side=2 * 3000, centre=8 * 3000),
}
NODES = {'main': (14, 22), 'side': (14, 58), 'centre': (54, 14)}
SIEGE = {'old': 20.0 / 700 * 0.7, 'new': 25.0 / 700 * 0.7}   # dps per credit of a mixed army
TUR_VALUE = {'old': 500 + 90, 'new': 600 + 90}
CC_HP = {'old': 2000, 'new': 3000}
POLICY = {
    # structures queue order; unit queue: haulers to `haul`, then an army cycle
    'careful':   dict(s=['ps', 'tur', 'ps', 'tur', 'tur'], haul=3, pre=['rifle', 'rifle', 'haul', 'tank'], army=['tank', 'rifle', 'rifle', 'launch'], attack=1.3, raid=1.1),
    'turtle':    dict(s=['ps', 'tur', 'ps', 'tur', 'tur', 'ps', 'tur', 'tur', 'ps', 'tur'], haul=2, pre=['haul'], army=['rifle', 'launch', 'eng'], attack=2.0, raid=9.9),
    'aggressor': dict(s=['ps', 'tur'], haul=2, pre=['tank', 'tank'], army=['tank', 'tank', 'launch', 'rifle', 'rifle'], attack=1.05, raid=1.0),
}

class Player:
    def __init__(s, name, pol, p, ore, skill, hauler_delay):
        s.name, s.pol, s.p, s.skill = name, POLICY[pol], p, skill
        s.cr = float(p['start']); s.q = {'s': list(s.pol['s']), 'u': []}; s.act = {'s': None, 'u': None}
        s.ore = {'main': ore['main'], 'side': ore['side']}
        s.haulers = [0.0 + P.trip(p, NODES['main'])[1]]   # D-F: gathers from tick 0
        s.hfield = ['main']; s.ps = 0; s.tur = 0; s.army = 0.0; s.ai = 0; s.spent = 0.0
        s.hauler_delay = hauler_delay; s.alive = True; s.pre_i = 0
    def field(s, centre):
        for f in ('main', 'side'):
            if s.ore[f] > 0: return f
        return 'centre' if centre[0] > 0 else None
    def next_unit(s, t):
        if s.pre_i < len(s.pol['pre']):
            w = s.pol['pre'][s.pre_i]; s.pre_i += 1
            if w != 'haul' or t >= s.hauler_delay: return w
        n_h = len(s.haulers) + sum(1 for k in ('u',) if s.act[k] and s.act[k][0] == 'haul')
        if n_h < s.pol['haul'] and t >= s.hauler_delay: return 'haul'
        w = s.pol['army'][s.ai % len(s.pol['army'])]; s.ai += 1; return w

def step(pl, t, dt, centre, log):
    p = pl.p
    nh = []
    for i, nd in enumerate(pl.haulers):
        if t >= nd - 1e-9:
            f = pl.hfield[i]
            pool = centre if f == 'centre' else None
            have = pool[0] if pool else pl.ore.get(f, 0)
            got = min(p['cap'], have)
            if pool: pool[0] -= got
            elif f: pl.ore[f] -= got
            pl.cr += got
            f2 = pl.field(centre); pl.hfield[i] = f2
            nd += P.trip(p, NODES[f2])[0] if f2 else 1e9
        nh.append(nd)
    pl.haulers = nh
    pl.cr += p['trickle'] * dt
    low = 40 + 30 * pl.tur > 100 * pl.ps
    sp = 0.5 if low else 1.0
    for k in ('s', 'u'):
        if pl.act[k] is None:
            if k == 'u': pl.act[k] = [pl.next_unit(t), 0.0, 0]
            elif pl.q['s']: pl.act[k] = [pl.q['s'].pop(0), 0.0, 0]
        if pl.act[k] is None: continue
        w, pr, paid = pl.act[k]
        np_ = min(1.0, pr + dt * sp / p['time'][w]); want = math.floor(p['cost'][w] * np_) - paid
        if want <= pl.cr:
            pl.cr -= want; paid += want; pr = np_; pl.act[k] = [w, pr, paid]; pl.spent += want
        if pr >= 1.0 - 1e-9:
            if w == 'ps': pl.ps += 1
            elif w == 'tur': pl.tur += 1
            elif w == 'haul': pl.haulers.append(t + P.trip(p, NODES[pl.field(centre) or 'main'])[1]); pl.hfield.append(pl.field(centre))
            else: pl.army += p['cost'][w]
            pl.act[k] = None

def exposed(pl):
    return sum(1 for f in pl.hfield if f in ('side', 'centre'))

def options(a, b, which, E):
    """The actions that a may take against b, as (ratio, kind). Both sides are tested in the same
    step; only the action with the larger ratio happens, so the loop order does not decide."""
    A = a.army * a.skill; B = b.army * b.skill; out = []
    if A <= 0: return out
    if exposed(b) and (B == 0 or A / B >= a.pol['raid']) and A > B:
        out.append((A / max(B, 1.0), 'raid'))
    D = math.sqrt((E * b.tur * TUR_VALUE[which]) ** 2 + B ** 2)
    if D == 0 or (A / D >= a.pol['attack'] and A > D):
        out.append((A / max(D, 1.0), 'base'))
    return out

def act(a, b, kind, t, which, E, log):
    A = a.army * a.skill; B = b.army * b.skill
    if kind == 'raid':
        a.army = math.sqrt(A * A - B * B) / a.skill; b.army = 0.0
        k = [i for i, f in enumerate(b.hfield) if f in ('side', 'centre')]
        for i in reversed(k): del b.haulers[i]; del b.hfield[i]
        log.append('%5.0fs %s wins the field fight at the haulers of %s: %d haulers lost; army of %s left %.0f' % (t, a.name, b.name, len(k), a.name, a.army))
        return None
    D = math.sqrt((E * b.tur * TUR_VALUE[which]) ** 2 + B ** 2)
    surv = math.sqrt(A * A - D * D)
    kill = CC_HP[which] / max(surv * SIEGE[which], 1e-9)
    log.append('%5.0fs %s breaks the base of %s (army %.0f vs defence %.0f, %d turrets); CC falls in %.0f s' % (t, a.name, b.name, A, D, b.tur, kill))
    return t + kill

def match(which, pa, pb, skill_b=1.0, delay_b=0.0, E=1.3, T=40 * 60, verbose=False):
    p = P.SETS[which]; ore = ORE[which]
    centre = [ore['centre']]
    A = Player('A', pa, p, ore, 1.0, 0.0); B = Player('B', pb, p, ore, skill_b, delay_b)
    t = 0.0; dt = 0.5; log = []; end = None; depleted = None
    while t < T:
        for pl in (A, B): step(pl, t, dt, centre, log)
        if int(t * 2) % 10 == 0:
            cand = [(r, k, x, y) for x, y in ((A, B), (B, A)) for r, k in options(x, y, which, E)]
            if cand:
                r, k, x, y = max(cand, key=lambda c: c[0])
                if len([c for c in cand if abs(c[0] - r) < 1e-9 and c[2] is not x]) == 0:
                    e = act(x, y, k, t, which, E, log)
                    if e is not None: end = (e, x.name)
        if end: break
        if depleted is None and centre[0] <= 0 and A.ore['main'] + A.ore['side'] + B.ore['main'] + B.ore['side'] <= 0:
            depleted = t
        t += dt
    first_out = None
    if verbose:
        for l in log: print('   ', l)
    res = 'A wins at %.1f min' % (end[0] / 60) if end and end[1] == 'A' else 'B wins at %.1f min' % (end[0] / 60) if end else 'no end in %d min' % (T / 60)
    return res, depleted, A, B

if __name__ == '__main__':
    pairs = [('careful', 'careful', 1.0, 0.0), ('careful', 'careful', 0.95, 0.0), ('careful', 'careful', 0.9, 0.0), ('careful', 'careful', 0.8, 0.0), ('careful', 'careful', 1.0, 120.0),
             ('careful', 'turtle', 1.0, 0.0), ('aggressor', 'careful', 1.0, 0.0), ('aggressor', 'turtle', 1.0, 0.0),
             ('turtle', 'turtle', 0.9, 0.0)]
    for which, Es in (('old', (1.4, 1.8)), ('new', (1.1, 1.3))):
        p = P.SETS[which]
        for f in NODES:
            tt, _ = P.trip(p, NODES[f]); print('%s trip %s %.1f s = %.0f credits/min per hauler' % (which, f, tt, p['cap'] / tt * 60))
        for E in Es:
            for pa, pb, sk, dl in pairs:
                res, dep, A, B = match(which, pa, pb, sk, dl, E)
                print('%s E=%.1f  A %-9s vs B %-9s (B skill %.2f, B 3rd hauler after %3.0f s): %-22s ore gone at %s'
                      % (which, E, pa, pb, sk, dl, res, '%.1f min' % (dep / 60) if dep else 'not in 40 min'))
    print('--- log: new E=1.3 careful vs careful, B skill 0.9')
    match('new', 'careful', 'careful', 0.9, 0.0, 1.3, verbose=True)
    print('--- log: old E=1.8 careful vs careful, B skill 0.9')
    match('old', 'careful', 'careful', 0.9, 0.0, 1.8, verbose=True)
