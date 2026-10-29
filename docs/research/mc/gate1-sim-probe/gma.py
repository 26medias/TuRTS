"""GMA red-build check: two groups of 20 troopers cross paths with avoidance OFF
(straight-line M1 integrator, spreading ON). Count pairs that overlap by more than 20 % of r
for more than 10 consecutive ticks. GMA is red only if that count > 0."""
import math
R, STEP, S = 0.5, 3.0 / 20, 2 * 0.5 * 1.05
def hexs(p, n):
    out = [(p[0] + S * (i + 0.5 * j), p[1] + S * math.sqrt(3) / 2 * j) for i in range(-8, 9) for j in range(-8, 9)]
    out.sort(key=lambda q: (round(math.dist(q, p), 9), math.atan2(q[1] - p[1], q[0] - p[0])))
    return out[:n]
def assign(starts, p):
    sl = hexs(p, len(starts)); taken = set(); res = {}
    for i in sorted(range(len(starts)), key=lambda i: (math.dist(starts[i], p), i)):
        k = min((k for k in range(len(sl)) if k not in taken), key=lambda k: (math.dist(starts[i], sl[k]), k))
        taken.add(k); res[i] = sl[k]
    return [res[i] for i in range(len(starts))]
def case(label, a_start, a_tgt, b_start, b_tgt):
    A = [(a_start[0] + 1.2 * (k % 5), a_start[1] + 1.2 * (k // 5)) for k in range(20)]
    B = [(b_start[0] + 1.2 * (k % 5), b_start[1] + 1.2 * (k // 5)) for k in range(20)]
    pos = [list(q) for q in A + B]; tg = assign(A, a_tgt) + assign(B, b_tgt)
    run = {}; worst = 0; bad = 0
    for t in range(600):
        for i in range(40):
            dx, dy = tg[i][0] - pos[i][0], tg[i][1] - pos[i][1]; d = math.hypot(dx, dy)
            if d <= STEP: pos[i] = list(tg[i])
            else: pos[i][0] += dx / d * STEP; pos[i][1] += dy / d * STEP
        for i in range(40):
            for j in range(i + 1, 40):
                if math.dist(pos[i], pos[j]) < 2 * R - 0.2 * R:
                    run[(i, j)] = run.get((i, j), 0) + 1; worst = max(worst, run[(i, j)])
                else:
                    if run.get((i, j), 0) > 10: bad += 1
                    run[(i, j)] = 0
    bad += sum(1 for v in run.values() if v > 10)
    rest = sum(1 for i in range(40) for j in range(i + 1, 40) if math.dist(pos[i], pos[j]) < 2 * R - 1e-6)
    print(f"{label}: longest overlap run={worst} ticks, pairs over 10 ticks={bad}, overlaps at rest={rest} -> GMA {'RED' if bad or rest else 'GREEN'}")
case("perpendicular", (10, 20), (40, 22), (24, 6), (26, 36))
case("head-on", (10, 20), (40, 22), (40, 20), (10, 22))
