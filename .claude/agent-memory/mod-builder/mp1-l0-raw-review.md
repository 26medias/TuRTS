---
name: mp1-l0-raw-review
description: L0-RAW review (cdff88c, 2026-11-12): RAW presence verified facts, gaps filed (list elements, two presence parsers, Manifest( form, explicit None in variant)
metadata:
  type: project
---

Review of L0-RAW, branch mp1/l0-raw, cdff88c (2026-11-12). Verified by packer runs:

- Explicit `[]` and one entry of exports.orders/interface/input/assets at ^0.2.2 are refused:
  `acme/manifest.ron: engine_api: \`exports.X\` needs engine api 0.2.3; \`^0.2.2\` admits 0.2.2`. Left out packs.
- Block comment, line comment, string value: not counted. Comment before key, whitespace, r#orders,
  implicit_some, `Manifest(`/`Exports(` names: counted. Duplicate key: typed parse refuses first.
- Content cross-checks run before the version table (orders without assets: "unknown image asset").
- Gaps filed: has_path does not walk lists (raw_presence.rs:115-121), so design RAW rows M3, H5, H6,
  H21, H22 (`alerts[].x`, `placements[].start_order`) cannot be written; validate_m3_presence
  (content_m3_validation.rs:602-716) is a second presence parser with its own error text and it
  refuses a `Manifest(` named struct with "Expected struct `ExportPresence`"; a Parsed row for a
  variant field (Button font) accepts an explicit `font: None` (scratch probe); version error names
  manifest.ron, not the unit file that writes the field.
- `dependencies` exists since 13c472a (first schema): the 0.2.2 feature is a non-empty map, so the
  builder's Parsed detection is right; design 6.10 line "RAW applies to every other row" is wrong for it.

Re-check at e86cc16 (2026-11-12): M1, M2, M3, L4, L5, L7 closed by runs. M3 rows equal old verdicts
for ^0.2, ^0.2.1, *, >=0.1.0, >0.2.0, ~0.2, 0.2 (old binary from cdff88c). No accept-to-refuse.
Open Low: `Written<T>` absent key serialises as `font:None` and re-parses as written (no
skip_serializing_if); first call skips parsed rows, so `weapon with movement` at ^0.2 now reports
`exports.recipes`/`unit.health` first; a parsed row names no file. Merge-tree on 052beb1: clean.

**Why:** consumer lens on the version table that every 0.2.4 row will use.
**How to apply:** at L0-API2 / VAL1, check list look-through, one presence source, and the file in the error.
Related: [[mp1-design-gate1]].
