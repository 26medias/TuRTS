# Duel model: all units in range from t=0 (unless gap given), focus fire on one target,
# first shot at t=0, damage = max(1, floor(dmg * pct / 100)). Step 0.01 s.
import math
PCT = {  # damage type -> armour class -> percent
    "bullet": {"infantry": 100, "vehicle": 20, "structure": 20},
    "rocket": {"infantry": 30, "vehicle": 100, "structure": 50},
    "cannon": {"infantry": 75, "vehicle": 100, "structure": 100},
}
U = {
    # name: cost, hp, armour, dmg, type, range, cooldown_s
    "rifleman": (100, 120, "infantry", 12, "bullet", 5.0, 1.0),
    "launcher": (250, 90, "infantry", 60, "rocket", 7.0, 2.5),
    "tank":     (700, 500, "vehicle", 40, "cannon", 6.0, 2.0),
    "turret":   (500, 600, "structure", 40, "cannon", 8.0, 1.5),
    "engineer": (300, 70, "infantry", 0, None, 0, 1),
    "harvester":(800, 600, "vehicle", 0, None, 0, 1),
    "cc":       (0, 2000, "structure", 0, None, 0, 1),
    "power":    (300, 600, "structure", 0, None, 0, 1),
}
def hit(a, b):
    _, _, _, d, t, _, _ = U[a]
    return 0 if t is None else max(1, d * PCT[t][U[b][2]] // 100)
def duel(A, B, repair_b=0.0, dt=0.01, tmax=600):
    side = [[{"n": n, "hp": U[n][1], "cd": 0.0} for n in A], [{"n": n, "hp": U[n][1], "cd": 0.0} for n in B]]
    t = 0.0
    while t < tmax:
        for s in (0, 1):
            foe = [u for u in side[1 - s] if u["hp"] > 0]
            if not foe: return t, side
            tgt = foe[0]
            for u in side[s]:
                if u["hp"] <= 0 or U[u["n"]][4] is None: continue
                if u["cd"] <= 1e-9:
                    tgt["hp"] -= hit(u["n"], tgt["n"]); u["cd"] = U[u["n"]][6]
                    if tgt["hp"] <= 0:
                        foe = [x for x in side[1 - s] if x["hp"] > 0]
                        if not foe: return t, side
                        tgt = foe[0]
        if repair_b:
            for u in side[1]:
                if u["hp"] > 0 and U[u["n"]][2] != "infantry":
                    u["hp"] = min(U[u["n"]][1], u["hp"] + repair_b * dt); break
        for s in (0, 1):
            for u in side[s]: u["cd"] = max(0.0, u["cd"] - dt)
        t += dt
    return t, side
def report(label, A, B, **kw):
    t, side = duel(A, B, **kw)
    ca = sum(U[n][0] for n in A); cb = sum(U[n][0] for n in B)
    sa = [f'{u["n"]}:{max(0, round(u["hp"]))}' for u in side[0] if u["hp"] > 0]
    sb = [f'{u["n"]}:{max(0, round(u["hp"]))}' for u in side[1] if u["hp"] > 0]
    win = "A" if sa and not sb else "B" if sb and not sa else "none"
    print(f"{label:44} A={ca:5} B={cb:5} t={t:6.2f}s win={win:4} A_left={sa} B_left={sb}")
print("== single-target time to kill (s), first shot at t=0 ==")
for a in ["rifleman", "launcher", "tank", "turret"]:
    row = []
    for b in ["rifleman", "launcher", "engineer", "tank", "harvester", "turret", "power", "cc"]:
        h = hit(a, b); shots = math.ceil(U[b][1] / h); row.append(f"{b}={(shots-1)*U[a][6]:.1f}")
    print(f"{a:9}", " ".join(row))
print("== ratio 1: time to kill, equal credits ==")
report("7 rifleman vs 1 tank", ["rifleman"]*7, ["tank"])
report("5 rifleman vs 1 tank", ["rifleman"]*5, ["tank"])
report("3 launcher vs 1 tank", ["launcher"]*3, ["tank"])
report("5 rifleman vs 2 launcher", ["rifleman"]*5, ["launcher"]*2)
report("1 tank vs 3 launcher (tank first)", ["tank"], ["launcher"]*3)
print("== ratio 3: turret against tank ==")
report("1 tank vs 1 turret", ["tank"], ["turret"])
report("2 tank vs 1 turret", ["tank"]*2, ["turret"])
report("2 tank vs turret + 1 engineer (8 hp/s)", ["tank"]*2, ["turret"], repair_b=8.0)
report("2 tank vs turret + 2 engineer (16 hp/s)", ["tank"]*2, ["turret"], repair_b=16.0)
report("6 rifleman vs 1 turret", ["rifleman"]*6, ["turret"])
report("4 launcher vs 1 turret", ["launcher"]*4, ["turret"])
report("3 tank + 0 vs cc (siege time)", ["tank"]*3, ["cc"])
report("4 launcher vs cc", ["launcher"]*4, ["cc"])
report("8 rifleman vs 1 tank (800 vs 700)", ["rifleman"]*8, ["tank"])
report("2 tank vs 2 tank + 1 engineer (16hp/s->8)", ["tank"]*2, ["tank"]*2, repair_b=8.0)
report("1 tank vs 1 harvester", ["tank"], ["harvester"])
report("3 rifleman vs 1 harvester", ["rifleman"]*3, ["harvester"])
report("2 launcher vs 1 harvester", ["launcher"]*2, ["harvester"])
