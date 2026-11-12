"""Summarize one G1 edge counter log: START lines (processes), HIT lines by thread."""
import collections, sys

path = sys.argv[1]
starts, hits, added, removed = [], collections.Counter(), collections.Counter(), collections.Counter()
try:
    lines = open(path).read().splitlines()
except FileNotFoundError:
    lines = []
for line in lines:
    fields = dict(part.split("=", 1) for part in line.split()[1:] if "=" in part)
    if line.startswith("START"):
        starts.append(fields.get("exe", "?"))
    elif line.startswith("HIT"):
        thread = fields.get("thread", "?")
        hits[thread] += 1
        added[thread] += int(fields["added"])
        removed[thread] += int(fields["removed"])
print(f"log {path}: processes {len(starts)} hit_queries {sum(hits.values())} "
      f"added {sum(added.values())} removed {sum(removed.values())}")
for thread in sorted(hits):
    print(f"    thread {thread}: hit_queries {hits[thread]} added {added[thread]} removed {removed[thread]}")
