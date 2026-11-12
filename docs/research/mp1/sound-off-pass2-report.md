RESULT=PASS (two old reds stay: AUD1 and HUD2, same causes as pass 1; nothing new red)

# Pass 2 of `mp1/sound-off-default`: H1, M1, the alert acceptance sentence

Branch `mp1/sound-off-default`, head `314e2722`, pushed, not merged. `origin/mp1/integration` had not moved
(`git log HEAD..origin/mp1/integration` is empty at the end), so there is no merge commit.
Commits of pass 2: `815ef626` (H1 and M1), `6e8999f3`, `53fac829`, `ed5f9b80`, `714e551f`, `314e2722`.
Labels: `soundoff2-*` under `/home/julien/Projects/TuRTS/target/mc-evidence/`.

## H1: the switch is a widget that the mod places (D67)

- API 0.2.4, row H25 (3 additive names): the engine action `toggle_sound` (`ButtonAction::Input`), `Value::SoundState`
  ("Sound: on" or "Sound: off", rule of pass 1: on only when the flag is on and the browser lets audio run) and
  `Predicate::SoundOn` (`crates/turts_mod_api/src/interface.rs`).
- Pattern of L0-API1: 3 VAR rows in `version_table.rs` (`since_api` 0.2.4; the row count test 57 becomes 60), the collector
  in `variants.rs`, the refusal at `^0.2.3` with its path, the engine action list (`ENGINE_ACTION_NAMES` in
  `content_mc_validation.rs`), a hint and an allowed list for an unknown input action of a button. No `UNBUILT` line
  (they are built; `unbuilt.rs` lists them in the test of the rows with no line).
- Which API: 0.2.4. `ENGINE_API_VERSION` is `0.2.4` (`crates/turts_mod_api/src/lib.rs:63`), lane 0 has not frozen it, and
  row H23 (`leave_match`) took the same step. A new number would be a second edition for three additive names.
- Client: the action flips the switch and never goes to the core (`widgets/activation.rs`); the value and the predicate read
  one state (`widgets/sources.rs`, `values.rs`, `tree.rs`).
- Fallback: the engine draws its own switch only when the active interface places no widget with `toggle_sound`
  (`InterfaceDef::places_input_button`, at any depth of Box, List and Grid; `client_shell/notices.rs`). It is stated in the
  rustdoc of `Value::SoundState` and in the erratum. The settings-menu switch stays (shell screen).
- The six packages place the switch in their `hud.ron`: a 124 x 28 frame (2 px, red `#4d1f1f` for off, green `#1f6138` for
  on, one widget for each state with `Not(SoundOn)` / `SoundOn`) round a 120 x 24 button, anchor `Bottom`, offset
  (166, -8), `layer: 50` (above the forge guide). That is the place measured free in pass 1. The button is 24 px high
  because the packer warns below 24 x 24 (the `cli` test caught it). Font: the button label uses the tooltip style font
  of the package (muster_ui, forge_ui); base, cedar, haul and acme ship no font, so they use the engine default.
- Cost that the ruling did not name: the six packages need `engine_api: "^0.2.4"` (the version table refuses the names at
  `^0.2.3`). 84 tests used the shipped packages as the 0.2.3 baseline. `crates/turts_mod_runtime/tests/common`
  `as_edition_0_2_3` gives them that baseline (strips the two widgets, restores `^0.2.3`); 4 pins in `turts_pack` and
  `forge_package` moved to `^0.2.4`. New test `the_shipped_packages_place_the_sound_switch_and_pack_at_0_2_4` packs each
  shipped package as it is. The six packs exit 0 with no stderr (`turts-pack`, from copies, `soundoff2-pack6-1`).
- Tests: `h25_sound_switch_names_and_the_placement_question` (api), `h25_the_sound_switch_names_need_api_0_2_4_and_pack_at_0_2_4`
  (runtime), `toggle_sound_button_flips_the_switch_...` (widgets),
  `the_engine_draws_its_switch_only_when_the_interface_places_no_toggle_sound_button` (shell; covers no interface, the
  fixture, and each of the six packages). Honest note: for H1 I wrote the tests after the code; the red proof is the
  plants below.
- Rows: pixel finder now finds the switch by fill and size anywhere (solid fallback or frame). SND1 (engine fallback, fixture
  package), new SND2 (`mods/muster`, the button of the package; the oracle asserts `kind: button`), HUD2 (`mods/forge`).
- Erratum: `docs/research/mp1/errata-sound.md` on the branch (E-SND-1 row H25, E-SND-2 design 10.1, E-SND-3 M1 facts,
  E-SND-4 ALR2 sentence).

## M1: root cause and repair

- Root cause (general, every `bevy_ui_widgets::Button`): `bevy_picking-0.20.0/src/events.rs:1060` sends `Pointer<Press>` to
  this frame's `HoverMap`; `events.rs:1091-1092` sends `Click` and `Release` to the entities of `PreviousHoverMap`. A pointer
  that reaches a button and presses and releases in that one frame has the button only in `HoverMap`. `Press` makes it
  `Pressed` (`bevy_ui_widgets-0.20.0/src/button.rs`, `button_on_pointer_down`); `Release` finds nobody, so no `Click`, no
  `Activate`, and `Pressed` stays on. The next click then acts: the reviewer's "the second click turns it on". It is the
  widget buttons, the shell buttons and the binding buttons. A touch screen or a fast click gives the same order.
- Repair: `crates/turts_client/src/press_release.rs`. Between `generate_hovermap` and `pointer_events`, when a pointer has a
  press then a release in this frame's input, the entities it hovers now are added to `PreviousHoverMap` for it. Added in
  `ClientShellPlugin`. No Bevy change.
- Native tests (`press_release::tests`): the browser order (press and release, no earlier hover frame) activates once and leaves
  nothing `Pressed`; the same without the repair loses the click and leaves the button `Pressed` (it pins the cause: it goes
  red when Bevy fixes it, then the repair can go); the person order (hover, press, later release) is unchanged; no hovered
  button, no activation; a button not hovered is not pressed.
- Headed check: `tests/browser/click-without-hover-probe.js` with `scripts/click-without-hover.sh` (instrument; own Chrome
  with the MC profile, under the wrapper). The pointer rests 300 ms far away, then move, press and release go in one batch;
  500 ms later one debug record (the 4 Hz cap stays; only SH1 may pass an interval) must show the effect.
  - fixture (engine fallback button): sound on, acted (`soundoff2-probe-final-1`, and at 815ef626).
  - forge, `--guide-close`: the mod-placed switch acted and the guide Close button acted (`soundoff2-probe-forge-2`).
  - Without the repair (plant 7b, the registration made `run_if(|| false)`): `acted:false`, exit 1 (`soundoff2-probe-plant7-1`).
  So M1 reproduces headed in this build and the repair removes it.

## ALR2

Reviewer sentence added verbatim to `errata-sound.md` (E-SND-4), with the cause in code. Not repaired.

## Gates (final head `314e2722`)

- `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`
  (2024 passed, 0 failed, 38 ignored), wasm32 check, `scripts/boundary.sh`: all 0.
- API crates: `turts_mod_api`, `turts_mod_runtime`, `turts_pack` tests green inside the workspace run.
- Node tests 288/288 (all files; `ci-node-list` green); driver rule self-test and tree pass.
- `mp1_plants.py --check --all`: 301 records apply, 0 stale. One old record had to be re-pinned (`VAL1 H23/a`, L0-API2: the
  `| "leave_match"` line moved into a constant list); proved red and restored (`soundoff2-plants-h23-1`).
- Plants: 27 records in `docs/research/mp1/plants/sound-off.json`. 24 non-headed: 24 red and restored through `--plants-dir`
  with only my file (`soundoff2-plants-2`). 3 headed: 6a and 6b proved in pass 1; 7b proved here. The 28 older records on the
  files I touched: 28 red and restored (`soundoff2-plants-others-1`). Plant 8g stayed green once (it removed one of the two
  buttons); fixed to remove both.
- Headed rows through `mc_headed_v6.sh` (prepared, 2 runs each): at the final head SND1, ORD6, ORD7, ALR1 pass
  (`soundoff2-headed-final-1`, hold 57 s). At `815ef626` also SND2 pass and HUD2, AUD1 as below.

## Still red, not from this change

- AUD1: red at base `54c39fdd` (pass 1, 2 runs); stale slider coordinates after my sound-on step. NOT run in pass 2.
- HUD2 (forge, mod-placed switch): my asserts pass (switch click proof, sound on, refusal sink); it fails at the Controls tail
  check "Type modifier" (`soundoff2-headed-hud2-1`, 2 runs). Not run at base.

## NOT TRACED

- Native desktop client; Firefox and Safari.
- The look of the six switches at 1024 x 768 and with the guide open in base, cedar, haul, acme (muster and forge were
  seen; the fill and size are pinned by a node test for all six).
- The oracle additions for AUD1 end to end.
- Whether `bevy_picking` 0.20.1 still has the cause (the lane pins `=0.20.0`).

## What I changed

Branch `mp1/sound-off-default` only (list: `git diff 99247bfa..314e2722 --stat`), and this report. No other branch.
