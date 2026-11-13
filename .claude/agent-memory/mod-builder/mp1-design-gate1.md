---
name: mp1-design-gate1
description: MP1 design gate 1 (rev 3) and re-gate (rev 4, 5158f96, API 0.2.4 table 6.10): which gaps closed, verified packer facts, open items for gate 2
metadata:
  type: project
---

Rev 3 gate (f7e7ccb, 2026-11-11): 12 findings, 3 High (no API table; construction home; NC1 kinds).

Rev 4 re-gate (branch mp1-design, 5158f96, 2026-11-12): 6.10 table exists (design:1256-1446).
Closed: H1, H2, H3, M5, M6, M8, L10, L11, L12. Partial: M4 (RAW), M7 (error texts), M9 (caps = CEO D-M).

Verified by runs (packer code still MC, so these are today's facts):
- ModError form: `pkg/file: field: msg (allowed: ...)`; parse errors drop the field path and add
  `at line L column C` (error.rs:42-60). Map unit ref: no list, no hint. Order hotkey: "unknown input action".
- Weapon damage 0 packs; reason key typo packs (exit 0). Relation `Enemy`, resource `Supply` refused today.
- ron 0.12.2 `ron::Value`: struct fields and map keys are the same `Map(String)`; enum variant names drop
  (unit variant -> `Unit`, struct variant -> bare Map). So a RAW key walk cannot tell `Button.font` from
  `Text.font` (Text writes font/size in every hud.ron of forge and muster).
- Weapon is a work capability today (content_m3_validation.rs:392-418).
- Packer uses DEFAULT_TICK_HZ (turts_pack/src/lib.rs:25-27); server tick_hz is server.ron (config.rs:77).
Open at rev 4 (filed): alert place and "credits gathered" in frontier words (High); RAW on enum-variant
fields; State alerts exclude forge triggers; no result-reason text path; key lists point at engine source.

**Why:** consumer lens for 6.10; the re-gate asked if a second mod can be written from 5, 6.10, 6.7.4, 7.
**How to apply:** at gate 2, check that the plan fixes these, and re-run the RAW and reason-typo probes
once lane 0 code exists. Sample mod `tide` RON files were throwaway scratch.
