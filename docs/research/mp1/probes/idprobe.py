import re, sys, pathlib
root = pathlib.Path(sys.argv[1])
idents, lits = set(), set()
for p in (root/"crates").rglob("*.rs"):
    if "tests" in p.parts: continue
    t = p.read_text(errors="ignore")
    lits.update(re.findall(r'"((?:[^"\\]|\\.)*)"', t))
    t2 = re.sub(r'"((?:[^"\\]|\\.)*)"', ' ', t)
    idents.update(re.findall(r'\b(?:r#)?([A-Za-z_][A-Za-z0-9_]*)\b', t2))
cands = sys.argv[2].split(",")
for c in cands:
    print(f"{c:22} ident={'HIT' if c in idents else 'no ':3} literal={'HIT' if c in lits else 'no'}")
