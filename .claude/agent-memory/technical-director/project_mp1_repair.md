---
name: mp1-design-repair-facts
description: Verified 2026-11-11 facts from the MP1 design rev 3 repair (TuRTS-mp1, mp1-design d729a97): ron::Value keeps keys not variants, version-table presence, reason/cursor key sets, admission order, hot-site lines, choices made in 6.10
metadata:
  type: project
---

Verified 2026-11-11 in /home/julien/Projects/TuRTS-mp1 (mp1-design, d729a97; code = MC integration):
- Probe (scratch td-ronprobe, ron =0.12.2): `ron::Value` keeps struct keys incl. `armour: None` and
  `orders: []`, but enum unit variants become `Unit`. So "presence = written" works for fields only;
  variants need the typed value (rule: no default yields a new variant).
- Version table has `weapon with movement` since 0.2.3 (version_table.rs:52-56); presence from parsed
  values (content_mc_validation.rs:65-68). All repo packages require ^0.2.3.
- Reason keys: wire set = messages.rs `as_str`; client-only keys no_applicable_order, queue_disabled,
  already_researching (forge hud.ron). Cursor contexts in client: default, select, move, attack, gather,
  forbidden (widgets/cursor.rs:403-410). Validation checks cursor asset ids only (content_mc_validation.rs:527).
- Admission order: BadSequence session.rs:403-412, production_precheck :414-418, bucket :419-424.
- Damage today: base = damage + research modifier (combat.rs:371-374), applied = min(health) (:376);
  Fired staged to shooter slot (:393-402). choose_target fn at :297; call :201; cooldown :229.
- try_ms_to_ticks at boot_m3.rs:591. capture_operation at economy/transaction.rs:105 (slice plan says :90).
- Charge model check (scratch td-repair/charge.py): floor(cost*done/total) differences, 20,000 cases, 0 bad.
- Repair choices I wrote into the design: constructor queue + producer queue (no named producer queues),
  edition rule = lowest admitted API, `Supply(scope, counts)` kind, `match` export file, acquire on unit.

**Why:** gate re-check of MP1 rev 4 and lane 0 cite these.
**How to apply:** re-check lines after the slice (S2/S3 split orders.rs and movement.rs). See
[[mp1-architecture-input-facts]], [[mc-architecture-audit-facts]].
