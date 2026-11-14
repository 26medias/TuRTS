# Independent foundation review: swordsman and Surface range

Reviewed head: `d2214625`. Baseline: `36608a74`.
Review worktree: `/home/julien/Projects/TuRTS-swordsman-review`.
Spec: `docs/superpowers/specs/2026-11-14-swordsman-and-surface-range.md`.

Verdict: PASS for the foundation change. No blocking findings.

## Source assessment

- Omitted weapon modes default to Centre. Centre preserves the old range value and serialization omits that default.
- Raw presence detection requires API 0.2.8 for either explicit mode. Resolution retains the mode.
- Surface firing uses the authoritative source and candidate radii. The broad phase includes the largest live radius. The exact filter uses each candidate's radius.
- Attack approach and attack-move engagement use the same reach. The existing bounded rounding correction remains in use.
- Mixed weapons keep separate reaches. Existing target priority and movement arbitration remain unchanged.
- Audit records omit Centre and default old records to Centre. Surface is recorded. The replay uses the new reach and reconstructs the approach rounding correction.
- The selected Surface range ring adds the source radius. Game command and snapshot structures are unchanged.
- Skirmish content declares Surface only for the swordsman. Its attack, move, and stop orders include the new type. Its visual declares no projectile effect.

## Executed code checks

All commands used `/tmp/codex-mp1-cargo` and the shared native cache. Every command exited 0.
Evidence paths below are relative to `/home/julien/Projects/TuRTS/target/mc-evidence/swordsman/`.

| Check | Result | Evidence |
| --- | --- | --- |
| `test -p turts_sim --lib surface` | Three tests pass. Includes castle reach, 16 bearings for attack and attack-move, no overlap, and separate mixed weapon reaches. | `review-sim.log` |
| `test -p turts_mod_runtime --test content_m3 range_mode` | One test passes. Old omission resolves as Centre. Both explicit variants need 0.2.8 and resolve there. | `review-api.log` |
| `test -p turts_audit --test surface_range` | One test passes. Actual mixed-unit commands replay. Repeated attack and attack-move runs produce identical records. | `review-audit.log` |
| Independent per-target radius case | Pass. A closer small enemy outside its own reach stays unharmed. The larger building in reach takes damage. | `review-independent.log` |
| Independent zero-gap Surface approaches | Pass from 64 bearings. The weapon reaches the building and fires. | `review-independent.log` |
| Independent old audit record compatibility | Pass. Centre fields are omitted. Removing the Surface field simulates an old record and defaults it to Centre on load. | `review-audit-compat.log` |
| `test -p turts_client --lib melee_without_projectile` | One test passes. No bolt is emitted. Health and hit flash release without a flight delay. | `review-melee-fx.log` |

The independent test additions are retained in `review-independent-tests.patch` and the review worktree. They do not change the author worktree.

## Limits

I did not run a browser, measure GPU performance, or test the launcher. The author runs those acceptance checks separately.
I did not repeat the full native suite. I ran the targeted checks above.
This review does not claim balance quality or a new human playtest.
