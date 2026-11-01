import sys, json, time, random
import numpy as np
import colour
from colour.blindness import matrix_cvd_Machado2009
import warnings; warnings.filterwarnings("ignore")
MATS = {k: matrix_cvd_Machado2009(kind, 1.0) for k, kind in [("protan","Protanomaly"),("deutan","Deuteranomaly"),("tritan","Tritanomaly")]}
I, J = np.triu_indices(8, 1)
def labs_for(rgb):
    lin = colour.models.eotf_sRGB(rgb)
    out = {"normal": colour.XYZ_to_Lab(colour.sRGB_to_XYZ(rgb))}
    for k, m in MATS.items():
        sim = np.clip(lin @ m.T, 0, 1)
        out[k] = colour.XYZ_to_Lab(colour.sRGB_to_XYZ(colour.models.eotf_inverse_sRGB(sim)))
    return out
LMIN, LMAX = 30.0, 90.0
def score(rgb):
    labs = labs_for(rgb)
    Ln = labs["normal"][:,0]
    pen = float(np.clip(LMIN-Ln,0,None).sum() + np.clip(Ln-LMAX,0,None).sum())
    mins = {}
    for k, L in labs.items():
        d = colour.delta_E(L[I], L[J], method="CIE 2000")
        mins[k] = float(d.min())
    return min(mins.values()) - pen, mins
def hx(rgb): return ["#%02X%02X%02X" % tuple(int(round(c*255)) for c in row) for row in rgb]
def from_hex(hs): return np.array([[int(h[i:i+2],16) for i in (1,3,5)] for h in hs], float)/255
okabe = ["#0072B2","#E69F00","#009E73","#D55E00","#56B4E9","#CC79A7","#F0E442","#000000"]
s, mins = score(from_hex(okabe)); print("okabe-ito min", round(s,2), {k: round(v,2) for k,v in mins.items()}, flush=True)
budget = float(sys.argv[1]) if len(sys.argv) > 1 else 120
rng = np.random.default_rng(int(sys.argv[2]) if len(sys.argv) > 2 else 1)
best, best_s = None, -1
t0 = time.time()
while time.time() - t0 < budget:
    cur = from_hex(okabe) if (len(sys.argv)>3 and sys.argv[3]=='seed') else rng.random((8,3)); cur_s, _ = score(cur); T = 4.0
    for it in range(4000):
        cand = cur.copy(); k = rng.integers(8); cand[k] = np.clip(cand[k] + rng.normal(0, 0.12, 3), 0, 1)
        cs, _ = score(cand)
        if cs > cur_s or rng.random() < np.exp((cs-cur_s)/T): cur, cur_s = cand, cs
        T *= 0.999
        if cur_s > best_s: best, best_s = cur.copy(), cur_s
        if time.time() - t0 > budget: break
    print("restart done; best so far", round(best_s,2), flush=True)
s, mins = score(best)
print(json.dumps({"best_min": round(s,3), "per_sim": {k: round(v,3) for k,v in mins.items()}, "palette": hx(best)}))
