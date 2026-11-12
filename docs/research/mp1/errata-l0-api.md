# Errata text for design revision 6.3, from `L0-API1` and `L0-API2` (2026-11-12)

The text below is ready to paste. The builder did not edit the design or the plan (builders may not).
Each item names the design line, the old text, the new text, the evidence (`path:line`), and the status.
Source of the items: the review of `L0-API1` (`docs/research/mp1/review-l0-api1.md`) and the build of
`L0-API2` (branch `mp1/l0-api2`).

## E-API-1. Row H11: the fields of each world style (review M2)

Design 6.10 row H11 (D:1591) says "optional styles (colours `#RRGGBB`, sizes, font id)" and gives the fields
of `placement` only. Replace the cell "Type" of row H11 with the text below. The code is in
`crates/turts_mod_api/src/interface.rs` (`WorldStyle`, `PlacementStyle`, `LineStyle`, `BarStyle`,
`FloatTextStyle`). The parse test is `h11_world_styles_in_the_some_form` in
`crates/turts_mod_api/tests/api_0_2_4.rs`. Every style is `Option<...>` in `world` and defaults to `None`
(the engine style). A field that the list does not hold is refused (`deny_unknown_fields`). Every colour is
`#RRGGBB` (the packer refuses another form; `L0-API2`, test `h11_h24_world_styles`).

| Field of `world` | Type | Fields (R = required) | RON example |
| --- | --- | --- | --- |
| `placement` | `Option<PlacementStyle>` | `valid` (R, colour), `invalid` (R, colour), `stripes` (R, bool) | `placement: Some((valid: "#56B4E9", invalid: "#D55E00", stripes: true))` |
| `build_area_ring` | `Option<LineStyle>` | `colour` (R, colour), `width` (R, f32, pixels, finite and above 0) | `build_area_ring: Some((colour: "#FFFFFF", width: 2.0))` |
| `gap_ring` | `Option<LineStyle>` | as `build_area_ring`; the ring also draws the clearance ring of a node | `gap_ring: Some((colour: "#FFFFFF", width: 1.0))` |
| `rally` | `Option<LineStyle>` | as `build_area_ring`; the line to a rally point | `rally: Some((colour: "#56B4E9", width: 2.0))` |
| `progress_bar` | `Option<BarStyle>` | `width` (R, u16, above 0), `height` (R, u16, above 0), `background` (R, colour), `fill` (R, colour) | `progress_bar: Some((width: 40, height: 5, background: "#000000", fill: "#56B4E9"))` |
| `cargo_bar` | `Option<BarStyle>` | as `progress_bar` | `cargo_bar: Some((width: 32, height: 3, background: "#000000", fill: "#E69F00"))` |
| `float_text` | `Option<FloatTextStyle>` | `colour` (R, colour), `font` (`Option<String>`, default `None`, a font asset), `size` (R, u16), `sound` (`Option<String>`, default `None`, a sound asset) | `float_text: Some((colour: "#FFFFFF", font: None, size: 14, sound: None))`; the short form `float_text: Some((colour: "#FFFFFF", size: 14))` is the same value |

A guess that the review made, `build_area_ring: Some((colour: "#ffffff", size: 2))`, is refused: a line style
has `width`, not `size` (test `h11_world_styles_in_the_some_form`, the negative cases).

Status: the design left the field lists out (needs the orchestrator to apply the text). The code shapes copy
`WorldHealthBarStyle` (`BarStyle`) and `WorldTooltipStyle` (`FloatTextStyle`). The packer checks: colour
`#RRGGBB`, a font and a sound that exist, `width` above 0, a bar of a size above 0
(`crates/turts_mod_runtime/src/mp1_validation.rs`, `world_styles`).

Plan 5.3 note (review L3): no plan task reads the `float_text` style. Plan 5.3 gives only its `sound` to
`FX2`, and the block of `L-FX-PRES` renders the "+N" float (X14, R14) and names no style. `L0-API2` gave the
field its own `UNBUILT_FEATURES` line, owner `L-FX-PRES` (`crates/turts_mod_runtime/src/unbuilt.rs`). Needs
decision: which task reads the colour, font and size of `world.float_text`. Until then the line holds the
field back, so a modder never writes a field with no effect.

## E-API-2. Row H7: `Value` and `Source` (review M3; ruling option (a))

Design 6.10 row H7 (D:1587) lists the new variants of "`Value` and `Source`" together. Replace the cell "Type"
with: "new variants of `Value`: `Supply(ContentRef, Provided or Consumed)`, `Connection`, `Outcome`,
`OutcomeReason`, `MatchStats(StatField)`, `MatchClock`, `Hint`. New variants of `Source` (the list sources):
`ConstructionQueue`, `Sites`, `Notices`, `SelectionTypes`. A list name is in `Source` only: `Bind(Sites)`,
`Format("{0}", [Sites])` and `tooltip: Some(Sites)` are refused with the parse error `Unexpected variant named
`Sites` in enum `Value`, expected one of ...` (the file and the line are in the text)."

Evidence: `crates/turts_mod_api/src/interface.rs` (`enum Value`, `enum Source`); test
`h7_the_four_list_names_are_not_values` in `crates/turts_mod_api/tests/api_0_2_4.rs`. Design 7.5 (D:1901-1904)
names the `Value::` variants of that list and none of the four. The version rows and the `UNBUILT` lines of
`Value::ConstructionQueue`, `Value::Sites`, `Value::Notices` and `Value::SelectionTypes` are removed (the
`UI-HUD` block of plan 5.3 has 15 lines, not 19). Status: decided by the orchestrator (option (a)).

## E-API-3. Row H14: the RON notation of a hint condition (review, "Errata judgement")

Design 6.10 row H14 (D:1594) writes `HintShownFor(id, secs: f32)`: a positional field and a named field in one
variant. RON cannot parse that form (probe `p14_hint_024`: `cannot parse: expected `,` at line 26 column
111`). The code is right. Replace the notation with:

- `HintDone("build_power")` and `StepCurrent("build_power")`: a newtype variant with one string (the quotes are
  part of the RON);
- `HintShownFor(id: "h1", secs: 2.0)`: a struct variant (named fields only);
- `JobOnHold(secs: 2.0)`: a struct variant.

The error-text example of the row keeps `HintDone(build_powr)` without quotes: it is a message, and the
message prints the condition in that form. Add one sentence: "The RON source writes `HintDone("build_powr")`."
The packer text is `<package>/<file>: hints[2].starts: `HintDone(build_powr)` names no hint; did you mean
`build_power`? (allowed: build_power, ...)` (test `h14_hints`).

Evidence: `crates/turts_mod_api/src/interface.rs` (`HintCondition`); test `h14_every_hint_condition` in
`crates/turts_mod_api/tests/api_0_2_4.rs`. Status: the design notation is wrong, the code is right.

## E-API-4. The `STATE_KEYS` list has 23 keys (review, "Errata judgement")

Design 6.10, "The full key lists" (D:1622-1628), cites `crates/turts_net_protocol/src/economy.rs:90-108` for the
`GatherStage` values and counts 22 keys. The enum `GatherWaitReason` ends at `economy.rs:109`
(`Blocked,`), one line after the cited range. The simulation sends it
(`crates/turts_sim/src/economy/gather.rs:66`, `crates/turts_server/src/export.rs:269`). Replace the
text with: "The wire `GatherStage` values (`crates/turts_net_protocol/src/economy.rs:90-109`): `gather.to_node`,
`gather.gathering`, `gather.to_drop_off`, `gather.unloading`, `gather.idle`, `gather.waiting.no_drop_off`,
`gather.waiting.destination_full`, `gather.waiting.destination_overflow`, `gather.waiting.no_deposit`,
`gather.waiting.depleted`, `gather.waiting.blocked`. Total: 23 keys."

Also correct plan line 463 (`STATE_KEYS` (22, D:1616-1625)) to 23. Evidence: `crates/turts_mod_api/src/keys.rs`
(`STATE_KEYS`, 23 entries); `crates/turts_client/tests/api_keys.rs` (the list against the wire enums); test
`h1_h2_h3_the_key_lists` in `crates/turts_mod_runtime/tests/val1.rs` (the counts 49 and 23). Status: the code
is right.

## E-API-5. The refusal text of an unbuilt feature (review M1; ruling)

Design 6.8, "One mod API step" (D:1394-1396) and 6.7.4 give the text `<package>/<file>: <field path>:
`<feature>` is not built in this engine build`. Replace "with the text" by: "with the text
`<package>/<file>: <field path>: `<feature>` is not built in this engine build; leave it out until an engine
build supports it`. The text names no plan task: the owner task of each line is a comment in the code
(`crates/turts_mod_runtime/src/unbuilt.rs`)." Evidence: tests `every_raw_line_refuses_with_its_file_path_and_owner`
(`unbuilt.rs`) and `check` in `crates/turts_mod_runtime/tests/api_0_2_4.rs`. `turts-pack --help` lists the text
and the edition rule (test `pack_cli_help_lists_the_unbuilt_refusal_and_the_edition_rule`).

## E-API-6. Rows K7 and K8, and the order of the checks (`L0-API2`)

- Row K7 (`health without removal`) and row K8 (`removal with a queue`): the two M3 rules are lifted
  (`crates/turts_mod_runtime/src/content_m3_validation.rs`). Each is now a COMBO row of the version table, so a
  package of an older edition gets `` `health without removal` needs engine api 0.2.4; `^0.2.1` admits 0.2.3
  (written in units/turret.ron: health) `` and the edition 0.2.4 gets the `UNBUILT` line of `L-DES`. The two
  tests `health_requires_removal` and `removal_on_unit_with_queue` in
  `crates/turts_mod_runtime/tests/content_m3.rs` change their observation to that text (a permitted change:
  observation change).
- The "`constructor` beside `producer` exemption at `content_m3_validation.rs:418`" of the plan has no code to
  exempt: that check reads the resolved unit (`ResolvedUnit`), which has no `constructor` field. The rule of
  row K1 is checked on the parsed unit (`mp1_validation.rs`, `constructor`), with the text of the design.
- Order of the checks in `PackageArchive::content` (new text for 6.10, "Conventions"): version table (the RAW
  rows), `UNBUILT_FEATURES` (the RAW lines), the old validation, the version table (the parsed rows),
  `UNBUILT_FEATURES` (the parsed lines), then the validation column of the 0.2.4 table (`check_api_0_2_4`). A
  package that uses a feature that is not built hears that first. A test reaches the checks while the line
  stands through `PackageArchive::check_api_0_2_4`. The checks run for the edition 0.2.4 only.
- Row K4: `crates/turts_mod_runtime/src/content.rs` lifts the rule "output must be mobile and cannot be a
  producer" for the production ids that a constructor lists as buildings (the plan lists no file for it; the
  design says "it lifts `content.rs:557` for these ids only").

## E-API-7. Texts that the design does not give (needs decision)

The design gives an error text for some rows only. `L0-API2` wrote these texts in the form of 6.10
(`<package>/<file>: <field path>: <message>`). Each is a test in `crates/turts_mod_runtime/tests/val1.rs`.
The orchestrator or `game-designer` confirms them or gives the design text.

- Row O9: the design text `units: a `Surrender` order lists no units` is used as written. The validation
  column says `units: []`, so the text reads as an error for a LIST THAT IS EMPTY, while the check refuses a
  list that is NOT empty. Needs decision: change the text to `units: a `Surrender` order must list no units`
  (the code and the test use the design text).
- Row K1, a production in both `producer.recipes` and `constructor.buildings`:
  `` constructor.buildings[1]: `val1:train_hauler` is also in `producer.recipes` of this unit `` (and `of
  `<unit>` (<file>)` for another unit).
- Row K6, no match file: `exports.match: a package with a constructor needs a match file that sets
  `open_sites_per_slot``; `open_sites_per_slot: must be at least 1`.
- Row U8 and M3, target kind: `` spawn_order.target: `val1:order_move` takes a Point target, not a Node target ``.
- Rows U4 to U7, R1, H4 to H6, H16, H20, K2, K3, M1, H14: the "must be finite and at least 0, found <x>" and
  "must be positive" forms follow the example of row H20; the exact wording of each is in the tests.
- Row H15: "one different colour for each player slot" counts the distinct `owner_slot` values of the map
  placements. Needs decision: the design does not say where the number of player slots comes from.

## E-API-8. Not done in `L0-API2`, with the reason (NOT TRACED items)

- Row A1, "one damage table in the resolved package set": the packer sees one package, so only the files of
  that package are counted. The resolved-set check needs the resolver (`resolve::resolve`) and belongs to the
  server path (`L-DMG`).
- Row A2, "the root Game package owns the match file": a Library that exports a match file is not refused
  (needs the resolved set).
- Row P1 / design 6.4, "a `total` that overflows u64": the packer does not know the `tick_hz` of `server.ron`
  (design U7 note), so it checks `Q` (at most 65,536) and not `total`.
- Rows V1 and V2 (the GLB checks): owned by `PRES-MODELS` (plan 668).
- Rows H11 to H13 `font` and `sound` exist: checked against the `assets` files of the same package only.
