"""Stdlib-only GLB 2.0 writer: a low-poly tank (hull, turret node, muzzle node), 2 materials."""
import json, struct, sys

def box(cx, cy, cz, sx, sy, sz):
    # 24 vertices (flat normals), 12 triangles, min y = cy - sy/2
    p, n, idx = [], [], []
    faces = [((1,0,0),(0,1,0),(0,0,1)), ((-1,0,0),(0,1,0),(0,0,-1)),
             ((0,1,0),(0,0,1),(1,0,0)), ((0,-1,0),(0,0,-1),(1,0,0)),
             ((0,0,1),(1,0,0),(0,1,0)), ((0,0,-1),(-1,0,0),(0,1,0))]
    h = (sx/2, sy/2, sz/2)
    for normal, u, v in faces:
        base = len(p)
        c = [cx + normal[0]*h[0], cy + normal[1]*h[1], cz + normal[2]*h[2]]
        for su, sv in ((-1,-1),(1,-1),(1,1),(-1,1)):
            p.append([c[i] + su*u[i]*h[i] + sv*v[i]*h[i] for i in range(3)])
            n.append(list(normal))
        idx += [base, base+1, base+2, base, base+2, base+3]
    return p, n, idx

def merge(*parts):
    P, N, I = [], [], []
    for p, n, i in parts:
        off = len(P); P += p; N += n; I += [k + off for k in i]
    return P, N, I

bin_ = bytearray(); views = []; accs = []
def add(data, fmt, comps, ctype, target, minmax=False):
    while len(bin_) % 4: bin_.append(0)
    off = len(bin_)
    for row in data:
        bin_.extend(struct.pack("<" + fmt * comps, *(row if comps > 1 else [row])))
    views.append({"buffer": 0, "byteOffset": off, "byteLength": len(bin_) - off, "target": target})
    acc = {"bufferView": len(views) - 1, "componentType": ctype, "count": len(data),
           "type": {1: "SCALAR", 3: "VEC3"}[comps]}
    if minmax:
        acc["min"] = [min(r[i] for r in data) for i in range(3)]
        acc["max"] = [max(r[i] for r in data) for i in range(3)]
    accs.append(acc); return len(accs) - 1

def prim(geo, material):
    p, n, i = geo
    return {"attributes": {"POSITION": add(p, "f", 3, 5126, 34962, True),
                           "NORMAL": add(n, "f", 3, 5126, 34962)},
            "indices": add(i, "H", 1, 5123, 34963), "material": material}

# footprint radius 0.9 (D 8.1 example): XZ radius must be 0.72..1.08
hull_body = merge(box(0, 0.2, 0, 1.1, 0.4, 1.5), box(-0.55, 0.15, 0, 0.18, 0.3, 1.5), box(0.55, 0.15, 0, 0.18, 0.3, 1.5))
hull_team = box(0, 0.42, -0.4, 0.9, 0.04, 0.5)          # team stripe on top of hull
turret_team = box(0, 0.12, 0, 0.6, 0.24, 0.6)          # turret top = team colour
barrel_body = box(0, 0.12, 0.55, 0.1, 0.1, 0.6)
meshes = [{"name": "hull", "primitives": [prim(hull_body, 0), prim(hull_team, 1)]},
          {"name": "turret", "primitives": [prim(turret_team, 1), prim(barrel_body, 0)]}]
nodes = [{"name": "tank", "children": [1, 2]},
         {"name": "hull", "mesh": 0},
         {"name": "turret", "mesh": 1, "translation": [0, 0.4, 0], "children": [3]},
         {"name": "muzzle", "translation": [0, 0.12, 0.85]}]
doc = {"asset": {"version": "2.0", "generator": "turts make_tank.py"},
       "scene": 0, "scenes": [{"nodes": [0]}], "nodes": nodes, "meshes": meshes,
       "materials": [{"name": "body", "pbrMetallicRoughness": {"baseColorFactor": [0.45, 0.42, 0.38, 1], "metallicFactor": 0, "roughnessFactor": 0.9}},
                     {"name": "team", "pbrMetallicRoughness": {"baseColorFactor": [1, 1, 1, 1], "metallicFactor": 0, "roughnessFactor": 0.8}}],
       "buffers": [{"byteLength": 0}], "bufferViews": views, "accessors": accs}
while len(bin_) % 4: bin_.append(0)
doc["buffers"][0]["byteLength"] = len(bin_)
js = json.dumps(doc, separators=(",", ":")).encode()
js += b" " * ((4 - len(js) % 4) % 4)
out = struct.pack("<III", 0x46546C67, 2, 12 + 8 + len(js) + 8 + len(bin_))
out += struct.pack("<II", len(js), 0x4E4F534A) + js + struct.pack("<II", len(bin_), 0x004E4942) + bin_
open(sys.argv[1], "wb").write(out)
print(sys.argv[1], len(out), "bytes")
