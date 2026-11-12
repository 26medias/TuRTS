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

Plan 5.3 note (review L3; decided in the review of `L0-API2`, question 4): the style of `world.float_text` (the
colour, font and size of the "+N" float) is owned by `L-FX-PRES`; its `sound` stays with `FX2` (plan:509). Plan 5.3
needs the line "`world.float_text` | `L-FX-PRES`". Before this, no plan task named the style. The line is in
`crates/turts_mod_runtime/src/unbuilt.rs`, owner `L-FX-PRES`, so a modder never writes a field with no effect.

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

## E-API-7. Texts that the design does not give (judged by the review of `L0-API2`)

The design gives an error text for some rows only. `L0-API2` wrote these texts in the form of 6.10
(`<package>/<file>: <field path>: <message>`). Each is a test in `crates/turts_mod_runtime/tests/val1.rs`. The
review of `L0-API2` accepted them, with the changes below (applied in the code).

- Row O9: replace the design text `units: a `Surrender` order lists no units` with ``units: a `Surrender` order
  must list no units; write `units: []` ``. The check refuses a list that is NOT empty; the old text read as an
  error for an empty list.
- Row K1, a production in both lists: ``constructor.buildings[1]: `val1:train_hauler` is also in
  `producer.recipes` of this unit; a production is a recipe or a building, not both`` (and `of `<unit>`
  (<file>)` for another unit).
- Row K6, no match file: `exports.match: a package with a constructor needs a match file that sets
  `open_sites_per_slot``; `open_sites_per_slot: must be at least 1`.
- Rows U8 and M3, the target kind: ``spawn_order.target: `val1:order_move` takes a `Point` target, not a `Node`
  target`` (the kinds are in backticks, as the convention of 6.10 says).
- Row KEY1 (b): ``input/bindings.ron: actions[7].default[0]: `KeyM` is also the default key of `key_move`; each
  default key must be unique`` (the index of the second action and of its binding is in the field path; a chord
  is written `Ctrl+KeyP`).
- Rows U4 to U7, R1, H4 to H6, H16, H20, K2, K3, M1, H14: the "must be finite and at least 0, found <x>" and
  "must be positive" forms follow the example of row H20; the exact wording of each is in the tests.
- Every name lookup gives the hint and the allowed list, a font lookup included (`unknown font asset `ui_fnt`;
  did you mean `ui_font`? (allowed: mono, ui_font)`; test `h11_h13_h24_an_unknown_font_gets_the_hint_and_the_allowed_list`).
  Row H11, H13 and H24 say "font exists"; add "font asset" to the list of lookups in the conventions of 6.10.

## E-API-9. Row H15: the number of colours is the number of seats (orchestrator ruling on the review of `L0-API2`, M3)

Design 6.10 row H15 (D:1595) and 6.7.4 (D:893-895) say "one different colour for each player slot" and do not
say where the number comes from. Replace with: "the number of player slots is the number of seats that the
lobby offers, the engine constant `SLOT_COUNT` (2 in MP1; design 6.7.5, D:998: "The seat count is the player-slot
count"). A match file or a map file does not declare it. The packer counts `SLOT_COUNT`, not the slots that the map
places units for: a map with units for slot 0 only, with one colour, is refused with `start: `AllSeated` needs 2
different slot colours in the interface, found 1`."

Where the count comes from, in the code: the design does not put a seat count in `MatchDef` (6.7.4 lists three
fields) or in the map. The constant was `pub(crate) const SLOT_COUNT: u8 = 2` in `crates/turts_sim/src/slots.rs:7`.
It is now `pub const SLOT_COUNT` in `crates/turts_mod_api/src/lib.rs`, and `turts_sim::slots` re-exports it (no
behaviour change; `crates/turts_sim/src/plugin.rs` reads it as before). `turts_mod_runtime` cannot depend on
`turts_sim`, and both depend on `turts_mod_api`. Test `m1_m2_h15_the_match_file`, case "H15 the seats of the lobby,
not the slots of the placements". Status: ruled by the orchestrator. Row LOB2 ("two seats get one default
colour") is the row that this closes.

## E-API-10. Four rules that the code has and the design does not say (review of `L0-API2`, L4; accepted)

Add to the validation column of 6.10:

- Row R1: each `repairer.cost[].resource` is a `Balance` resource (``repairer.cost[0].resource: `val1:ore` is not
  a `Balance` resource``).
- Row U2: a `weapon.damage_type` in a package that has no damage table is refused (`weapon.damage_type: the
  package declares no damage table`).
- Rows C1 and C2: each entry of `damage_types` and `armour_classes` is a nonempty local id without `:` (`must be a
  nonempty local id without `:``).
- Row M1: `KeyUnitsLost` with an empty list is refused (``victory.defeat[0]: `KeyUnitsLost` needs at least one
  unit``; the design has this text in 6.7.4, D:900; add it to the validation column of row M1).

## E-API-8. Not done in `L0-API2`, with the reason (NOT TRACED items)

- **Open plan item, no owner task:** rows A1 ("one damage table in the resolved package set") and A2 ("the root
  Game package owns the match file") on the resolved set, and the u64 overflow of the job `total` (design 6.4).
  The packer sees one package, so only the files of that package are counted (A1, A2), and it does not know the
  `tick_hz` of `server.ron` (the `total`). A grep of the plan for "resolved package set", "one damage table" and
  "root Game" finds nothing. In MP1 a Game cannot have a dependency (rule LR3), so the result is the same today.
  The orchestrator gives these three checks an owner before MP2 (the server serves libraries then).
- Row A1 (detail): the check on the resolved set needs the resolver (`resolve::resolve`).
- Row A2, "the root Game package owns the match file": a Library that exports a match file is not refused
  (needs the resolved set).
- Row P1 / design 6.4, "a `total` that overflows u64": the packer does not know the `tick_hz` of `server.ron`
  (design U7 note), so it checks `Q` (at most 65,536) and not `total`.
- Rows V1 and V2 (the GLB checks): owned by `PRES-MODELS` (plan 668).
- Rows H11 to H13 `font` and `sound` exist: checked against the `assets` files of the same package only.
