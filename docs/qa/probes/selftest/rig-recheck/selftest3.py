# Model of revision 1, section 3.4: set equality; red on (1) exempt spec without case,
# (2) rule R1..R8 without case, (3) fewer than 19 rows.
import csv, sys
EXEMPT = {"m1","m2","m3","m4"}; RULES = {f"R{i}" for i in range(1,10)}; MIN = 20
rows = list(csv.DictReader(open(sys.argv[1]), delimiter="\t")); bad = []
for r in rows:
    want = set() if r["expected_rules"] == "-" else set(r["expected_rules"].split(","))
    got = set() if r["got"] == "-" else set(r["got"].split(","))
    if want != got: bad.append(f"case {r['file']}: want {sorted(want)} got {sorted(got)}")
reds = [r for r in rows if r["expected_rules"] != "-"]
if EXEMPT - {r["file"] for r in reds}: bad.append(f"cond1 {sorted(EXEMPT - {r['file'] for r in reds})}")
cov = set().union(*[set(r["expected_rules"].split(",")) for r in reds]) if reds else set()
if RULES - cov: bad.append(f"cond2 {sorted(RULES - cov)}")
if len(rows) < MIN: bad.append(f"cond3 {len(rows)}<{MIN}")
print("self-test", "red" if bad else "green", bad); sys.exit(1 if bad else 0)
