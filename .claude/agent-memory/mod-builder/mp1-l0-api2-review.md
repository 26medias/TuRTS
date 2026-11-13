---
name: mp1-l0-api2-review
description: L0-API2 review (fa7984b, 2026-11-12): validation column verified by 47 probes; stale l0-raw plant records; K8 research arm untested; H15 slot count ruling
metadata:
  type: project
---

Review of L0-API2 at fa7984b (on 9044820), 2026-11-12. VERDICT PASS, 0 High.

Verified by runs:
- 47 one-fault packages on the val1 fixture via `check_api_0_2_4`: every refusal names package/file/path;
  reference and key lookups give "did you mean" + allowed list; font lookups do not.
- K7/K8 lifted from validate_m3; version rows (archive.rs:283) run before it, so ^0.2.3 is still refused
  with "`removal with a queue` needs engine api 0.2.4"; ^0.2.4 gets UNBUILT (L-DES).
- Edition gate works on the pack path: cedar `queue_ful`, muster `damage: 0` exit 0 at ^0.2.3, exit 2 at ^0.2.4.
- `mp1_plants.py --all` exit 2: l0-raw.json[4,5,8,10] stale since L0-API1 (9044820), [9] stale by L0-API2.
  Builder ran plants with a private --plants-dir, which hides this. Check `--all` at every review.
- K8 research arm (variants.rs:124) removable with all tests green; K2 gap boundary untested.
- H15 counts owner_slot of placements; a one-slot map passes with 1 colour though SLOT_COUNT is 2.

**Why:** consumer lens on the 0.2.4 validation that every mod meets.
**How to apply:** at L0-EXIT, check the re-pin of the 5 records, the K8 arm tests, the H15 ruling, font hint.
Related: [[mp1-l0-api1-review]].
