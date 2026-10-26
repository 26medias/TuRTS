---
name: m1-design-gate1
description: Gaps filed at gate 1 on docs/superpowers/specs/2026-10-26-m1-foundation-design.md (third-party unit package path); re-check at gate 2
metadata:
  type: project
---

Filed 2026-10-26, review-spec, design draft (no Cargo.toml, no runtime). Verified by grep and a RON probe.
- HIGH: map stub in turts_sim places hard-coded `m1test:rifleman` (design:85, :165). A third-party unit registers but never spawns.
- HIGH: DEV-M1-01 client embeds the base archive (design:133). A third-party package needs a client rebuild. Needs decision.
- HIGH: T2 (design:175) runs only inside turts_mod_runtime. It cannot go red on the two items above.
- MED: T2 "one-byte change fails hash check" has no reference hash on the server side (server computes hash, design:128).
- MED: no archive layout, manifest file name, exports semantics, unit schema table, shape variants, diagnostic shape, server-config key.
- Manifest checklist: 9/9 field names of 5.5 present (design:119-121); formats still Open placeholders (design:61-63).
- Probe: serde-derived RON types from design text parse the design literals; an undeclared variant gives
  `NoSuchEnumVariant { expected: ["Circle"], found: "Rect", outer: Some("Footprint") }` (internal type name leaks).

**Why:** lens question was "can a third-party unit load with no engine change"; answer was no.
**How to apply:** at gate 2 / plan review, check each item closed; re-run the probe in scratchpad if a schema exists.

**Re-gate on revision 2 (2026-10-26):** all 8 gate-1 findings CLOSED. Evidence: map placement in package (design:165-166, 180, 223), client downloads from lockfile URLs (design:35-36, 199-202), T2a server binary with base.tar and acme.tar (design:233), T2d hash compare (design:236), layout and schema (design:138-170), shape variants (design:162, 169), ModError and server.ron (design:172-183), turts-pack (design:135).
Open after rev 2 (filed): MapDef placement rules and map selection not stated or tested; T2a oracle checks only content ids (sent by the server), not the downloaded visual; design:173 "no Rust type name in message" is untested and ron 0.12.2 raw errors do name `UnitDef`/`Footprint` (probe).
Probe facts: ron 0.12.2 rejects positional `Cuboid(1.0, 0.8, 1.4)` when the type has named fields.
