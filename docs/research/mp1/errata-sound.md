# Errata text for design revision 6.3, from the task `mp1/sound-off-default` (2026-11-12)

The text below is ready to paste. The builder did not edit the design or the plan (builders may not).
Each item names the design line, the old text, the new text, the evidence (`path:line`), and the status.
Source: the CEO decision "the game makes no sound by default" and the review of the branch
(`docs/research/mp1/review-sound-off.md`: H1, M1, section 5).

## E-SND-1. Design 6.10, new row H25: the sound switch of a mod (review H1; ruling of the orchestrator)

Why: D67 is Decided ("the engine fixes no layout and reserves no screen region"). The play screen is the
HUD, and the HUD is mod data. The first build drew one engine button at a fixed place. A control that stays
on the play screen is now a widget that the mod places. The engine gives the action, the words and the
predicate. The engine draws its own switch in each frame in which no mod switch is visible.

Add this row after row H24 of the table of API 0.2.4 (the columns are those of rows H23 and H24):

| Column | Text |
| --- | --- |
| Row | H25 |
| Field | the engine input action `toggle_sound` in `ButtonAction::Input`; the value `Value::SoundState`; the predicate `Predicate::SoundOn` |
| Type | the action is a string, like `leave_match`. `SoundState` is a unit variant of `Value`. `SoundOn` is a unit variant of `Predicate` (it works inside `Not(...)`). |
| Default | none: a mod that does not use them keeps the 0.2.3 behaviour and gets the engine switch (the fallback rule below) |
| Validation | `toggle_sound` is an engine action, like `menu` and `toggle_guide` (`is_engine_action`, `crates/turts_mod_runtime/src/content_mc_validation.rs`, `ENGINE_ACTION_NAMES`), so the input map does not declare it. An unknown input action of a button now gets the hint and the allowed list, the declared actions and the engine actions. |
| Error text | at `^0.2.3`: `cedar/manifest.ron: engine_api: `engine action toggle_sound` needs engine api 0.2.4; `^0.2.3` admits 0.2.3 (written in interface/hud.ron: root[hud_order_cancel_button].widget.action)`. The same form for `Value::SoundState` (path `...widget.label`) and `Predicate::SoundOn` (path `...visible`). A misspelt action: `cedar/interface/hud.ron: root[4].widget.action: unknown input action `toggle_sond`; did you mean `toggle_sound`? (allowed: add_modifier, ..., toggle_sound, ...)` |
| Version-table rows | `engine action toggle_sound`, `Value::SoundState`, `Predicate::SoundOn`: each VAR, `since_api` 0.2.4 (`crates/turts_mod_runtime/src/version_table.rs`; the collector is `variants.rs`; the row count of the test `every_variant_row_names_a_use_that_the_collector_lists` is 60). They have no `UNBUILT` line: they are built (`unbuilt.rs`, test `the_rows_with_no_line_are_the_rows_that_lane_0_builds`). |

The meaning of the three names:

- `Input("toggle_sound")` on a `Button`: one activation flips the sound switch of the player. It is a client
  preference: the action never goes to the interaction core (`crates/turts_client/src/widgets/activation.rs`).
- `Bind(SoundState)`: the words `Sound: on` or `Sound: off`. `Sound: on` only when the player has switched the sound
  on and the browser lets audio run (`crates/turts_client/src/client_shell/notices.rs`, `sound_shown_on`). In a browser
  that keeps audio locked after a click, the words stay `Sound: off`.
- `SoundOn`: true when `SoundState` reads `Sound: on`. A mod uses it to style the switch.

RON example (the six shipped packages use this, with their own place and fill). The switch is a frame (a `Box`
with a fill for each state) round a `Button`:

    (id: "sound_switch_off", at: Some((anchor: Bottom, offset: (166.0, -8.0), pivot: None)), size: (w: Px(124.0), h: Px(28.0)), layer: 50, visible: [Not(SoundOn)], widget: Box(fill: Some(Colour("#4d1f1f")), border: None, children: [
        (id: "sound_off_button", at: Some((anchor: TopLeft, offset: (2.0, 2.0), pivot: None)), size: (w: Px(120.0), h: Px(24.0)), widget: Button(icon: None, label: Some(Bind(SoundState)), action: Input("toggle_sound"), hotkey_label: false)),
    ])),
    (id: "sound_switch_on", ... visible: [SoundOn], fill "#1f6138" ... the same button ...),

**The fallback (it is part of the API and must be in the API documentation; rule of pass 4, ruling of the orchestrator):**
the engine draws its own sound switch on the play screen (bottom, right of the centre, 124 x 28 px, solid fill) in each
frame in which no button with the action `toggle_sound` is visible. The check is a run-time check, not a static one: the
client reads the buttons that exist in the frame (`WidgetButtonAction` with `Input("toggle_sound")`) and their
`InheritedVisibility` (`crates/turts_client/src/client_shell/notices.rs`, `sync_sound_switch`). A mod that places a visible
switch owns the control and its place. A mod can hide its button with `visible`; a hidden button does not count, so the
engine switch shows. A button whose `visible` list can never hold (for example `[GuideOpen, Not(GuideOpen)]`) is
never visible, so the engine switch shows too; a static check of the lists would miss such a list. (Before pass 4 the rule
was "a hidden button still counts as placed": `InterfaceDef::places_input_button`, `crates/turts_mod_api/src/interface.rs`.
That function stays in the public API; the engine no longer uses it for this decision.) The rule keeps the silent default
reversible in every mod, also in a mod that does not know the action. The settings menu always holds its own switch
beside the volume sliders (it is a shell screen, not the play screen). Tests:
`the_engine_draws_its_switch_in_each_frame_in_which_no_mod_switch_is_visible`,
`a_visible_mod_switch_of_a_pair_hides_the_engine_switch_and_another_action_does_not`,
`the_six_shipped_layouts_place_the_switch_and_a_visible_one_hides_the_engine_switch`
(`crates/turts_client/src/client_shell/tests.rs`).

Package edition: the six shipped packages (`muster`, `forge`, `base`, `cedar`, `haul`, `acme`) place the switch, so
their `engine_api` moves from `^0.2.3` to `^0.2.4` (the version table refuses the three names at `^0.2.3`).

Which API do the names belong to: API 0.2.4. Evidence: `ENGINE_API_VERSION` is `0.2.4`
(`crates/turts_mod_api/src/lib.rs:63`) and lane 0 has not frozen it (the coordinator's message; the same step as
row H23, which added the action `leave_match`). A new API number would make a second edition for three additive names
with no behaviour change for a package that does not write them. Status: Baseline (ruled by the orchestrator, H1).

Evidence in tests: `h25_sound_switch_names_and_the_placement_question` (`crates/turts_mod_api/tests/api_0_2_4.rs`),
`h25_the_sound_switch_names_need_api_0_2_4_and_pack_at_0_2_4` (`crates/turts_mod_runtime/tests/val1.rs`),
`toggle_sound_button_flips_the_switch_and_its_words_and_predicate_follow_the_state` (`crates/turts_client/src/widgets/tests.rs`),
`the_engine_draws_its_switch_in_each_frame_in_which_no_mod_switch_is_visible`
(`crates/turts_client/src/client_shell/tests.rs`). Browser rows: SND1 (engine fallback, package with no switch),
SND2 (the button of `mods/muster`), HUD2 (the button of `mods/forge`).

## E-SND-2. Design 10.1: the sound state and the start overlay (revision 16, ruling O15)

Old text of the row "Sound notice" (10.1): "When audio stays locked after the first gesture: "Sound is off. Click or
press a key to turn it on." for 8 s, ... The menu shows "Sound: off" until audio runs. Alerts keep their text and
minimap ping."

New text: "The game makes no sound until the player turns it on (CEO decision, 2026-11-12). The switch is stored as
`audio.sound_on` in the settings document (default `false`; a document with no flag reads as off; a flag that is not a
boolean is a corrupt value with the usual notice). While it is off no sound player exists and a request for a sound is
dropped, not queued; alert lines and minimap pings are unchanged. One control shows the state in words (`Sound: off`,
`Sound: on`) and switches it with one click: the widget that the mod places (row H25), or the engine fallback, and the
copy in the settings menu. `Sound: on` shows only when the switch is on and the browser lets audio run. The notice
"Sound is off. Click or press a key to turn it on." shows only when the switch is on and the browser keeps audio locked
after the first gesture (for 8 s, from 0.5 s after the first frame with locked audio)."

Evidence: `crates/turts_client/src/volume_settings.rs` (`VolumeSettings::sound_on`),
`crates/turts_client/src/audio.rs` (`spawn_requested_sounds`, `silence_while_sound_is_off`),
`crates/turts_client/src/client_shell/notices.rs` (`sync_sound_off_notice`). Status: Decided by the CEO.

## E-SND-3. The click that the pointer reaches in one frame (review M1, pass 3): not a design change

Facts, for the design notes of 10.1 ("every engine button acts on one click"): `bevy_picking` 0.20.0 sends the press to
this frame's hover map and the release and click to the map of the frame before, so a press and a release in the
frame where the pointer reached a button give no click (`bevy_picking-0.20.0/src/events.rs:1060` and `:1091-1092`).
Every `bevy_ui_widgets::Button` of the engine had it. The repair is `crates/turts_client/src/press_release.rs`.

Pass 3 changed the repair after the re-review (M1, L4). It no longer edits Bevy's previous hover map. After `pointer_events`
it sends the click that Bevy dropped (`Click`, then `Release`) to each hovered entity, and only for one click:

- the same pointer and the same button, the press and the release in the frame's input, and a press in the frame (a
  release with no press in the frame is left to Bevy);
- no drag by the rule of the interaction core: 4 px (`crates/turts_interaction/src/lib.rs:581` and `:632`, the literal
  `> 4.0` that tells a click from a box select). The client repair reads the constant `CLICK_DRAG_PIXELS` (4.0) of the same
  file. The core keeps its literal because the L0-CORE proof `step_equal.py` compares `step` token by token with the code
  before the split (a first try that wrote the constant there made three L0-CORE plants non-green); the test
  `the_core_uses_the_click_drag_distance` (`crates/turts_interaction/tests/click_drag_rule.rs`) pins the two literals to the
  constant. No input between the press and the release is farther than that from the press, and the release is not farther;
- a hit test of the press position and of the release position on the node of the entity (the way
  `bevy_ui-0.20.0/src/picking_backend.rs:205` hits a node), so a press on the map and a release over a button, or a
  press on button A and a release on button B, click nothing (red tests: the three cases of the re-review).

The events that a button sees for a quick click are the events of a normal hover-then-click, in the same order: `Over`,
`Press`, `Click` (with `Activate`), `Release` (test `a_press_and_release_on_one_button_in_one_frame_activate_it_once`
compares the two orders). The first repair (a merge into the previous hover map) dropped the `Over`. No code depends on
`PointerOver`: a grep of `crates` for `PointerOver`, `Pointer<Over>`, `PointerEnter`, `PointerOut`, `PointerLeave`,
`PickingInteraction` finds nothing outside the repair. The widgets, the tooltip and the cursor read the `Hovered`
component, which Bevy sets from the hover map and not from these events (`widgets/tooltip.rs:92`,
`widgets/activation.rs:19`). The test `without_the_repair_bevy_loses_the_click_and_leaves_the_button_pressed` fails when
Bevy fixes the cause: the repair can then go.

## E-SND-4. Row ALR2: the acceptance sentence for `UI-ALERT` (from the review, section 5, verbatim)

> With the sound switch on, a headless client whose replica holds one own unit that waits with no break for 120 s
> (one case `InserterWaiting`, one case `GathererWaiting`) records exactly one alert line and at most one alert sound
> for that wait in `turts_debug.audio`. A second sound comes only after the unit stops waiting and waits again, and
> only when the last sound of the rule is at least `cooldown` seconds old. In the first 60 s of the stage-1 match with
> no input after the sound is turned on, at most 3 alert sounds start (8.3 point 4). Plant: today's rule (the unit
> pushed on each evaluation, `cooldown.max(1.0)`) gives more than 100 sounds in the 120 s case: red.

The cause, in code (not repaired here): `crates/turts_client/src/feedback.rs:746-773` (`InserterWaiting`) and
`:817-840` (`GathererWaiting`) push the waiting unit in every evaluation; only `cooldown.max(1.0)` limits it
(`:870-880`). The state triggers of API 0.2.4 are stubs (`crates/turts_client/src/feedback/alert_state.rs`).

## E-SND-5. The play-screen switch is optional for a mod; the settings menu is not; F10 is reserved (re-review M2, re-check H1)

Rule (add to row H25 and to 6.10 "Conventions"): the game is silent until the player turns the sound on, so every player
must be able to reach a switch. The play screen is the mod's (D67), so a mod may leave the play-screen switch out. The
way that every player can always reach is the switch in the settings menu (a shell screen, the "Audio levels" line). The
menu opens with the engine action `menu`.

- The opt-out is stated, not a side effect: the engine draws its own play-screen switch in each frame in which no
  `Button` with `Input("toggle_sound")` is visible; a mod that places a visible one owns the control and may show it or
  hide it with `visible` (a hidden button, or one whose `visible` list never holds, does not count: the engine switch then
  shows; see E-SND-1). It is written in the rustdoc of `ButtonAction::Input`
  (`crates/turts_mod_api/src/interface.rs`), in `turts-pack --help` (test `pack_cli_help_states_the_sound_switch_rule`),
  and here.
- **F10 is reserved for the settings menu.** Status: Baseline (ruled by the orchestrator, option (b) of the re-check H1).
  The `ux-designer` confirms it in design revision 6.3 (D66, the control model). Before this rule, a package could keep a
  key on `menu` that was no use: `menu` on `KeyM` (which another action also uses), or on `ShiftLeft` alone, and the
  first-match lookup `action_for_key` (map order by name) gave the key to the other action. A count of keys on `menu` did
  not prove that the player could open the menu. Now:
  - Run time, for every edition: the interaction core reads a plain F10 (no modifier held) for `menu` before any binding of
    the mod or the player (`crates/turts_interaction/src/bindings.rs`, `action_for_key`, constant `RESERVED_MENU_KEY`).
    Escape keeps working through `cancel`, as before (`crates/turts_interaction/src/lib.rs`, the `cancel` arm sends
    the `menu` shell action when no order is pending). Tests through the real `Core::step`
    (`crates/turts_interaction/tests/f10_reserved.rs`): the package `a1` (`menu` on `KeyM`, which `key_move` uses, `cancel`
    unbound), the package `a5` (`menu` on `ShiftLeft` alone), F10 against the bindings of the mod and of the player, and a
    control for the default keys.
  - Pack time, edition 0.2.4 only (`crates/turts_mod_runtime/src/mp1_validation.rs`, `menu_keeps_f10`, in the KEY1 group;
    no version-table row): the packer refuses (1) an `engine_overrides` entry for `menu` that has no plain F10:
    `<package>/input/bindings.ron: engine_overrides[0].bindings: the engine action `menu` must keep F10; F10 is reserved
    for the settings menu, which holds the sound switch that every player must be able to reach. Add `Key(code: "F10",
    mods: [])` to these bindings, or leave `menu` out of `engine_overrides``; (2) F10 on another engine action:
    `...engine_overrides[0].bindings[0]: F10 is reserved for the settings menu (the engine action `menu`); bind another
    key to `zoom_in``; (3) F10 on an authored action: `...actions[5].default[0]: ...bind another key to `key_own``; (4)
    an authored action named `menu`: `...actions[5].name: `menu` is an engine action; an authored action cannot have
    its name. Change its keys in `engine_overrides` (it keeps F10)`. Test
    `f10_is_reserved_for_the_settings_menu_in_a_package_of_the_edition_0_2_4` (`crates/turts_mod_runtime/tests/val1.rs`; `a1`,
    `a5`, an empty list, a pointer-only list, F9 without F10: refused; F10 alone and F10 with F9: packs).
  - What changes for a package of the edition 0.2.3 (or older) that bound F10 to its own action: the packer does not
    check it (the same test shows a `^0.2.3` package that binds F10 to its own action still packs), and at run time F10 now
    opens the settings menu and its own action does not fire on a plain F10. The six shipped packages bind no F10
    (test `no_shipped_package_binds_f10`, `crates/turts_mod_runtime/tests/val1.rs`), so no shipped package changes. A
    published 0.2.3 package that bound F10 must move F10 to another key. This is a change of behaviour for that package and
    not a change of API.
  - The player can still unbind keys in the settings menu of the game (the player's choice; F10 is read before the player's
    bindings, so a player cannot lose the menu either).
- A misspelt action in a `^0.2.3` package no longer gets a hint to a name of 0.2.4: the hint and the allowed list offer the
  names that the edition admits (`ENGINE_ACTIONS_SINCE_0_2_4`; test
  `the_hint_for_a_misspelt_action_offers_only_the_names_of_the_edition`).

## E-SND-6. A key bound to `toggle_sound` (re-review question; re-check M2)

Fact before this change: `toggle_sound` passed the check of `engine_overrides` (it is in `is_engine_action`), but the
interaction core knew only `menu`, `toggle_guide` and `cancel` as shell actions
(`crates/turts_interaction/src/lib.rs:881`, `:951`), so a key bound to it did nothing. Now it works: the core sends
`Effect::ShellAction { action: "toggle_sound" }`, and the client flips the switch (`widgets/mod.rs`,
`consume_shell_actions`). It has no default key; a mod binds one with `engine_overrides: [(name: "toggle_sound",
bindings: [Key(code: "F9", mods: [])])]`. Tests: `a_key_bound_to_toggle_sound_sends_the_shell_action`
(`crates/turts_interaction/tests/rebound_actions.rs`) and `the_shell_action_toggle_sound_flips_the_switch`
(`crates/turts_client/src/widgets/tests.rs`).

Version rule (pass 4, M2): a package of `^0.2.3` that binds a key to `toggle_sound` in `engine_overrides`, or that names an
authored action `toggle_sound`, is refused by the version table as for the other 0.2.4 names (the same row
`engine action toggle_sound`; `crates/turts_mod_runtime/src/variants.rs`, `input_uses`):
`cedar/manifest.ron: engine_api: `engine action toggle_sound` needs engine api 0.2.4; `^0.2.3` admits 0.2.3 (written in
input/bindings.ron: engine_overrides[0].name)` (path `actions[5].name` for the authored form). Test
`a_key_bound_to_toggle_sound_needs_api_0_2_4` (`crates/turts_mod_runtime/tests/val1.rs`).

`leave_match` (record for the task `UI-RESULT`): the same two gaps exist. A bound key does nothing, and a `^0.2.3` package
that binds a key to `leave_match` in `engine_overrides` packs (the last lines of the test
`a_key_bound_to_toggle_sound_needs_api_0_2_4` show it). It is NOT changed here. It is not the same one-line rule: the row
`engine action leave_match` has an `UNBUILT` line (owner `UI-RESULT`, `crates/turts_mod_runtime/src/unbuilt.rs:187`), so the
same detection would make the packer refuse the override at `^0.2.4` as well, as an unbuilt feature (read, not run: `unbuilt.rs:58` asks the collector for the row by name, `archive.rs:197-206`
and `:331-338` apply the lines). `UI-RESULT` must decide both together.

## E-SND-7. Open item, older than this task (re-review L2)

Two path forms name one field in the packer's messages: a validation error uses the index form
(`root[0].widget.children[0].widget.action`), and a version-table refusal uses the id form
(`root[snd_quiet].widget.children[snd_quiet_btn].widget.action`). The modder reads two forms for one field. Pre-existing
(MC validation against the version table). Status: open; no owner task. Not changed here.
