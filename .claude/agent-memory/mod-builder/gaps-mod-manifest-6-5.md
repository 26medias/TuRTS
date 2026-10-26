---
name: gaps-mod-manifest-6-5
description: Manifest field gaps filed at gate 1 on spec 5.3/5.5/6.5 (v0.5 to v0.6 repair, 2026-10-26); which closed, which remain as section 15 open items
metadata:
  type: project
---

Verified by grep on 2026-10-26 (no Cargo.toml, no runtime):
- v0.5: 5.3 and 5.5 gave two different manifest field lists. 2 of 13 fields fillable.
- v0.6 repair CLOSED: one field list owned by 5.5 (line 196); two named documents, "package manifest"
  (author-written, never holds hash/size/URL) and "server lockfile" (5.3 line 184); 6.5 workflow takes
  the lockfile (line 255). Format gaps now LISTED as open in section 15 "Mod packaging" row
  (hash algorithm, identifier format, version scheme/constraint syntax, package kind values with a
  cosmetic-only kind, capability names). Status of those: Open, not Baseline.
- Residual after repair: 6.4 line 241 "Publish manifest" and line 247 "server manifest" still use the old
  word for the server advertisement.
- Checklist: scratchpad sample-manifest-0.6.md of that session.

**Why:** the modder cannot fill identity/kind/capabilities/hash fields until section 15 items are decided.
**How to apply:** at the next 5.x or 6.5 gate, re-run the 9-field checklist and check whether 15's open
formats were decided; cite status as Open until then.
