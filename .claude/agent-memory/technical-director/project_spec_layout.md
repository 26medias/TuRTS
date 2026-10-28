---
name: spec-register-layout
description: Verified line anchors in RTS-Engine-Project-Specifications.md v0.7 (809 lines) for the register (now D01-D64), 4.1-4.3, 5.x, 7, 8, invariants, 14, 15
metadata:
  type: project
---

Verified 2026-10-27 on spec v0.7 (809 lines): register L20-89, D01 L24 ... D59 L82, plus
D60-D64 L83-87 (all Baseline variants; the agent body still says D01-D59). 4.1 modules L117-132;
4.2 L134-147 (turret L144, lab L143, smelter L141); 4.3 L149-165 (Combat row L159, Progression
L161, Match rules L163); 5.2 L185-189; 5.3 L191-197; 5.5 L205-209; 7.2 L402-406; 7.3 L410;
7.4 L412-416; 8 L422-428 (research open policies L428); invariants L535-552 (inv 1 = L539,
inv 5 = L543, inv 10 = L548, inv 13 = L551); 14 L670-694 (M3 L678, M7 L682, "not a committed
delivery schedule" L672); 15 L696-714 (Mod packaging version scheme Open L708); 16.2 L724-751.

Code facts (2026-10-27): ENGINE_API_VERSION "0.2.0" at crates/turts_mod_api/src/lib.rs:28;
engine_api check = semver VersionReq.matches in crates/turts_mod_runtime/src/manifest.rs:23-42,
commented "M1 placeholder"; schema.rs uses deny_unknown_fields (27 uses). NET_PROTOCOL_VERSION 2
at crates/turts_net_protocol/src/lib.rs:13. turts_net_protocol depends on turts_mod_api.
M2 validation: gatherer requires movement, producers static (turts_mod_runtime/src/content.rs:186, :314).

Gate-2 traps (2026-10-27, M3 plan review): schema structs (UnitDef, MapDef, Placement, Exports,
ResolvedUnit, ContentCatalog) derive no Default, so a new field breaks ~66 struct literal sites
in 11 files; `wire_reason` (turts_server/src/outbound.rs:55-83) is an exhaustive match, so any new
sim RejectReason needs a server edit; validate_manifest runs before content parse
(turts_mod_runtime/src/archive.rs:42); load_package boots the sim (turts_pack/src/lib.rs:13-38);
tests/browser/m2.js:358-364 asserts exact debug key sets; default boundary.sh run has no
turts_mod_runtime no-bevy rule.

**Why:** Gate reviews need `path:line` cites; these anchors save a re-scan.
**How to apply:** Re-check with grep before citing; the spec moves when it is edited.
