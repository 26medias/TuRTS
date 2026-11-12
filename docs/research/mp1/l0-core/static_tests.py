#!/usr/bin/env python3
"""L0-CORE: the test functions of two commits. Nothing may be removed; the added ones are listed.
usage: static_tests.py BASE_COMMIT NEW_COMMIT   (it reads `#[test] fn NAME` of every crates/**/*.rs)"""
import re, subprocess, sys
base, head = sys.argv[1], sys.argv[2]
def names(rev):
    files = subprocess.check_output(["git", "ls-tree", "-r", "--name-only", rev], text=True).split("\n")
    out = set()
    for f in files:
        if not f.endswith(".rs") or not f.startswith("crates/"): continue
        text = subprocess.check_output(["git", "show", f"{rev}:{f}"], text=True, errors="replace")
        for m in re.finditer(r"#\[test\]\s*(?:#\[[^\]]*\]\s*)*(?:async\s+)?fn\s+(\w+)", text):
            out.add((f, m.group(1)))
    return out
b, h = names(base), names(head)
print("tests at base:", len(b), "at head:", len(h))
print("removed:", sorted(b - h))
print("renamed or moved: none" if not (b - h) else "")
print("added:", len(h - b))
for x in sorted(h - b): print("  +", x)
sys.exit(1 if b - h else 0)
