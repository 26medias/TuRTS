"""Pure-move probe: the multiset of top-level items (text, whitespace and visibility normalised)
of the old files must equal that of the new files, except `mod`/`use` items.
usage: item_equal.py --old f1 [f2 ...] --new g1 [g2 ...]"""
import re, sys, collections
VIS = re.compile(r"\bpub(\((crate|super|self|in [^)]*)\))?\s+")
START = re.compile(r"^(#\[|#!\[|///|//!|pub|fn |async |const |static |struct |enum |impl|trait |type |mod |use |unsafe |macro_rules!|extern )")
def items(text):
    out, cur = [], []
    lines = text.split("\n")
    # unwrap an inline `mod x {` ... `}` whose body moved to its own file: caller passes the body file
    for line in lines:
        if line and not line[0].isspace() and START.match(line) and not (cur and (cur[-1].startswith(("#[", "///", "//!", "#![")))):
            if cur: out.append(cur)
            cur = [line]
        else:
            cur.append(line)
    if cur: out.append(cur)
    norm = []
    for it in out:
        body = [VIS.sub("", l.strip()) for l in it if l.strip() and not l.strip().startswith("//")]
        if not body: continue
        head = next((l for l in body if not l.startswith(("#[", "#!["))), "")
        if re.match(r"^(mod \w+;|use .*;)$", head) and len(body) <= 3: continue
        norm.append("\n".join(body))
    return collections.Counter(norm)
def load(paths): 
    c = collections.Counter()
    for p in paths: c.update(items(open(p).read()))
    return c
a = sys.argv.index("--old"); b = sys.argv.index("--new")
old, new = load(sys.argv[a+1:b]), load(sys.argv[b+1:])
lost, gained = old - new, new - old
for t in lost: print("ONLY-OLD:", t.split("\n")[0][:100], f"({len(t.splitlines())} lines)")
for t in gained: print("ONLY-NEW:", t.split("\n")[0][:100], f"({len(t.splitlines())} lines)")
print("RESULT", "EQUAL" if not lost and not gained else "DIFFERENT", f"old_items={sum(old.values())} new_items={sum(new.values())}")
sys.exit(0 if not lost and not gained else 1)
