# C0-PACE model. The step arithmetic of pacing.run (same constants, same DT, same merge rule), with a
# driver callback in place of a fixed script, so a hint follower can react to what the hint line shows.
# Usage: python3 -I sim.py <dir of pacing.py>   (self-test: static scripts must equal pacing.run)
import sys, math, copy
DT = 0.1
STRUCT = {'ps', 'tur'}
COMBAT = {'rifle', 'launch', 'tank'}

def run2(p, driver, T=240, quiet=90.0, clear_after=15.0, trip=None):
    TRIP, FIRST = trip
    COST, TIME = p['cost'], p['time']
    S = dict(t=0.0, cr=float(p['start']), q={'s': [], 'u': []}, act={'s': None, 'u': None}, ps=0, tur=0,
             haulers=[0 + FIRST], done=[], queued=[], hold=False, hold_since=None, ev=[])
    t = 0.0; waits = []; cur_wait = None
    sounds = []; state_on = False; clear_since = None; last_sound = -1e9
    while t < T - 1e-9:
        S['t'] = t
        for what in driver(t, S):
            S['q']['s' if what in STRUCT else 'u'].append(what); S['queued'].append((t, what))
        nh = []
        for nd in S['haulers']:
            if t >= nd - 1e-9:
                S['cr'] += p['cap']; S['ev'].append((t, 'delivery')); nd += TRIP
            nh.append(nd)
        S['haulers'] = nh
        S['cr'] += p['trickle'] * DT
        low = 40 + 30 * S['tur'] > 100 * S['ps']
        sp = 0.5 if low else 1.0
        anyhold = False
        for k in ('s', 'u'):
            if S['act'][k] is None and S['q'][k]:
                S['act'][k] = [S['q'][k].pop(0), 0.0, 0]
            if S['act'][k] is None: continue
            w, pr, paid = S['act'][k]
            np_ = min(1.0, pr + DT * sp / TIME[w]); want = math.floor(COST[w] * np_) - paid
            if want <= S['cr']:
                S['cr'] -= want; paid += want; pr = np_; S['act'][k] = [w, pr, paid]
            else:
                anyhold = True
            if pr >= 1.0 - 1e-9:
                S['done'].append((t + DT, w))
                if w == 'ps': S['ps'] += 1
                if w == 'tur': S['tur'] += 1
                if w == 'haul': S['haulers'].append(t + DT + FIRST)
                S['act'][k] = None
        if anyhold and cur_wait is None: cur_wait = t
        if not anyhold and cur_wait is not None: waits.append((cur_wait, t)); cur_wait = None
        S['hold'] = anyhold
        S.setdefault('trace', []).append((t, S['cr'], anyhold, len(S['haulers']), low))
        if anyhold:
            clear_since = None
            if not state_on:
                state_on = True
                if t - last_sound >= quiet: sounds.append(round(t)); last_sound = t
        elif state_on:
            if clear_since is None: clear_since = t
            if t - clear_since >= clear_after: state_on = False
        t += DT
    if cur_wait is not None: waits.append((cur_wait, T))
    merged = []
    for a, b in waits:
        if merged and a - merged[-1][1] < 2.0: merged[-1] = (merged[-1][0], b)
        else: merged.append((a, b))
    longest = max([b - a for a, b in merged] or [0])
    return longest, merged, S, sounds

def static_driver(script):
    script = sorted(script, key=lambda e: e[0]); st = {'i': 0}
    def d(t, S):
        out = []
        while st['i'] < len(script) and script[st['i']][0] <= t + 1e-9:
            out.append(script[st['i']][1]); st['i'] += 1
        return out
    return d

if __name__ == '__main__':
    sys.path.insert(0, sys.argv[1]); import pacing
    p = pacing.SETS['new']; tr = pacing.trip(p, (14, 22))
    V = {'B': [(8,'ps'),(14,'rifle'),(15,'haul'),(34,'tur'),(40,'rifle'),(41,'rifle'),(42,'rifle')],
         'C': [(8,'ps'),(14,'rifle'),(15,'haul'),(34,'tur'),(40,'rifle'),(41,'rifle'),(42,'tank')],
         'D': [(8,'ps'),(14,'rifle'),(30,'haul'),(50,'tur'),(60,'rifle'),(61,'rifle'),(75,'tank')],
         'S7': [(8,'ps'),(14,'rifle'),(30,'haul'),(50,'tur'),(60,'rifle'),(61,'rifle'),(75,'tank'),(110,'rifle'),(111,'rifle'),(140,'tank'),(170,'launch'),(171,'launch')]}
    ok = True
    for k, s in V.items():
        a = pacing.run(p, k, s, [0], T=180, log=False, quiet=90, clear_after=15)
        b = run2(p, static_driver(s), T=180, trip=tr)
        same = abs(a[0] - b[0]) < 1e-9 and [(round(x, 1), round(y, 1)) for x, y in a[1]] == [(round(x, 1), round(y, 1)) for x, y in b[1]]
        ok &= same
        print('%-3s pacing.run longest %.1f spans %s | run2 longest %.1f spans %s | %s' % (k, a[0], [(round(x), round(y)) for x, y in a[1] if y - x >= 2], b[0], [(round(x), round(y)) for x, y in b[1] if y - x >= 2], 'EQUAL' if same else 'DIFFER'))
    print('self-test', 'PASS' if ok else 'FAIL')
    sys.exit(0 if ok else 1)
