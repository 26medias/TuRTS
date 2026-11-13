---
name: mp1-l0-api1-review
description: L0-API1 review (7c51ac0, 2026-11-12): verified 0.2.4 packer texts, gaps filed (UNBUILT text has task id, H11 style shapes, H7 Value/Source split, client relation test gap)
metadata:
  type: project
---

Review of L0-API1 at 7c51ac0 (base 51f1223), 2026-11-12. VERDICT PASS, 0 High.

Verified by turts-pack runs on copies of mods/base and mods/muster:
- Version refusal: `base/manifest.ron: engine_api: \`acquire\` needs engine api 0.2.4; \`^0.2.3\` admits 0.2.3 (written in units/rifleman.ron: acquire)`; same at `>=0.2.3`; VAR rows name the file too (resources/power.ron: kind; units/trooper.ron: weapon.targets.slots).
- UNBUILT: `base/<file>: <path>: \`<feature>\` is not built in this engine build (task \`<owner>\`)` exit 2.
- `slots: Enemy` packs at ^0.2.4 (built via turts_rules). `states` with a bad key packs at ^0.2.4 (T2 is L0-API2).
- Design form `HintShownFor(id, secs: f32)` is not RON (parse error); code form `HintShownFor(id: "h", secs: 2.0)`.
- STATE_KEYS 23 is right: GatherWaitReason::Blocked at economy.rs:109 (design cites :90-108).

Filed (re-check at L0-API2 / L0-EXIT): M1 task id in the modder message, no advice (unbuilt.rs:270-277);
M2 H11 styles LineStyle/BarStyle/FloatTextStyle not in the design, no Some-form test; M3 H7 needs decision
(4 list names in both Value and Source); L1 client relation.rs test cannot fail for Ally/Enemy under OWN_TEAM;
L2 turts-pack --help omits UNBUILT and edition; L3 world.float_text has no owner in plan 5.3.

**Why:** consumer lens on the 0.2.4 surface that frontier and third-party mods will write.
**How to apply:** at L0-API2 and L0-EXIT, check that these closed. Related: [[mp1-l0-raw-review]], [[mp1-design-gate1]].
