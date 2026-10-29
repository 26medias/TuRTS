---
name: spec-register-layout
description: Verified line anchors in RTS-Engine-Project-Specifications.md v0.8 (856 lines): register D01-D74, terms, 3.1 goal, 4.x, 6.x, 14 milestone order, 15, 16.2, 17.1; plus API version code and plan-review compile traps
metadata:
  type: project
---

Verified 2026-10-29 on spec v0.8 after the repair pass (I wrote the 0.8 amendment). D65/D67/D73 use mixed status "Decided line (G6 B / G5 a / G7 yes); design Proposed (G16)". Invariant 15 (replay on one build) L581. Section 14 player-check rule L703, terms L705-712, rows L721-737. Older anchors below are from before the repair and drift by up to +15 after L563: Terms (presentation, player
input, player interface) L18-22. Register L26-105: D01 L30, D18 L47 (lightyear selected, G13), D41 L70
(reopened, server-in-process, G2), D43 L72 (editor trigger note, G4), D48 L77 (Rust bot framework note,
G8), D52 L81 (shared-control note, G3), D65-D74 L94-103 all Proposed with seats. 3.1 goal (G1, Decided)
L113. 4.1 L137 (module row renamed "Presentation and player interface" L142); 4.3 L169 (row "Player
interface" L184). 6.2 L263 (G9 same-build determinism L267); 6.3 L273 (D41 text L277); 6.10 L368 (G13
paragraph L380); 6.11 L386 (replay = command log L392). Invariants L563-580. 14 L698 (order QA0, MC, MP1,
MP2, M4, M4b, M5, MS, MB, M6, M7, M8-M13; M1-M3 Done; column "Acceptance a player can fail"). 15 L733
(rows Player input, HUD, Order model, Fog of war L743-746; fog has no D row). 16.2 L765 (MP1 note L794).
17.1 L800 (G8 L804). 18 L833. G1-G16 live in docs/research/review-2026-10/00-gap-register.md section 3.

Code facts (2026-10-27): ENGINE_API_VERSION "0.2.0" at crates/turts_mod_api/src/lib.rs:28;
engine_api check = semver VersionReq.matches in crates/turts_mod_runtime/src/manifest.rs:23-42,
commented "M1 placeholder"; schema.rs uses deny_unknown_fields (27 uses). NET_PROTOCOL_VERSION 2
at crates/turts_net_protocol/src/lib.rs:13. turts_net_protocol depends on turts_mod_api.
M3 validation rules that block 16.2: crates/turts_mod_runtime/src/content_m3_validation.rs:375 (health
requires removal), :382 (weapons must be static), :427 (queue cannot be removed) (verified 2026-10-29).

Gate-2 traps (2026-10-27, M3 plan review): schema structs (UnitDef, MapDef, Placement, Exports,
ResolvedUnit, ContentCatalog) derive no Default, so a new field breaks ~66 struct literal sites
in 11 files; `wire_reason` (turts_server/src/outbound.rs:55-83) is an exhaustive match, so any new
sim RejectReason needs a server edit; validate_manifest runs before content parse
(turts_mod_runtime/src/archive.rs:42); load_package boots the sim (turts_pack/src/lib.rs:13-38);
tests/browser/m2.js:358-364 asserts exact debug key sets; default boundary.sh run has no
turts_mod_runtime no-bevy rule.

**Why:** Gate reviews need `path:line` cites; these anchors save a re-scan.
**How to apply:** Re-check with grep before citing; the spec moves when it is edited.
