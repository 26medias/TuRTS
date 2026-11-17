#!/usr/bin/env python3
"""B11 license check: every transitive normal dependency of each candidate build, both targets.
Writes OUT/meta/licenses.json. A package passes when one OR branch of its SPDX expression has
only permissive licenses."""
import json, re, subprocess, sys

PERMISSIVE = {"MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "Zlib", "Unlicense", "ISC",
              "CC0-1.0", "Unicode-3.0", "Unicode-DFS-2016", "BSL-1.0", "0BSD",
              "Apache-2.0 WITH LLVM-exception", "MIT-0"}

def ok(expr):
    if not expr:
        return False
    e = expr.replace("/", " OR ").replace("(", " ").replace(")", " ")
    for branch in re.split(r"\s+OR\s+", e):
        parts = [p.strip() for p in re.split(r"\s+AND\s+", branch) if p.strip()]
        if parts and all(p in PERMISSIVE for p in parts):
            return True
    return False

def tree(args):
    r = subprocess.run(["cargo", "tree", "-e", "normal", "--prefix", "none", "--format", "{p}|{l}"] + args,
                       capture_output=True, text=True, check=True)
    pk = {}
    for line in r.stdout.splitlines():
        if "|" in line:
            p, l = line.rsplit("|", 1)
            pk[p.replace(" (*)", "").strip()] = l.strip()
    return pk

out = sys.argv[1]
res = {}
for c in ["rhai", "rune", "lua"]:
    pk = {}
    pk.update(tree(["-p", "ms_web", "--target", "wasm32-unknown-unknown", "--features", c]))
    pk.update(tree(["-p", "ms_native", "--no-default-features", "--features", c]))
    bad = {p: l for p, l in pk.items() if not ok(l) and not p.startswith("ms_")}
    res[c] = {"packages": len(pk), "not_permissive": bad, "pass": not bad}
json.dump(res, open(f"{out}/meta/licenses.json", "w"), indent=1)
print(json.dumps({c: (v["packages"], v["pass"], v["not_permissive"]) for c, v in res.items()}))
