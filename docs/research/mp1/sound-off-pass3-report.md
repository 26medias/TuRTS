RESULT=PASS (AUD1 and HUD2 stay red from before this change; one probe check cannot go red, stated below)

# Pass 3 of `mp1/sound-off-default`: M1, L4, M2, L1, L3, L2

Branch `mp1/sound-off-default`, head `ee9bd0dd`, pushed, not merged. `origin/mp1/integration` moved
(L0-WIRE, `4553acc4`): merged in `4cbe008b` (one conflict, the node test list in `.github/workflows/ci.yml`: the union).
Labels `soundoff3-*` under `/home/julien/Projects/TuRTS/target/mc-evidence/`. Erratum: `docs/research/mp1/errata-sound.md`
(E-SND-3, E-SND-5, E-SND-6, E-SND-7 are new or changed in this pass).

## M1 and L4: the click repair

- The first repair merged the hover map. The new repair (`crates/turts_client/src/press_release.rs`) runs after
  `pointer_events` and sends the dropped `Click` and `Release` only for one click: same pointer and button; a press in the
  frame (a release with no press in the frame is left to Bevy); no drag by the rule of the core; a hit test of the press
  position and of the release position on the node of the entity (as `bevy_ui-0.20.0/src/picking_backend.rs:205`); and only
  for an entity that was not hovered in the frame before (Bevy clicks those itself).
- The drag rule that the client already uses: the literal `> 4.0` in `crates/turts_interaction/src/lib.rs:581`
  (`on_pointer_move`) and `:632` (`on_pointer_up`). The repair reads a new constant `CLICK_DRAG_PIXELS` (4.0, same file).
  My first try wrote the constant into those two lines; the L0-CORE proof `step_equal.py` then saw a different token and
  3 L0-CORE plants had no green baseline (`soundoff3-plants-others-1`). Reverted: the core keeps the literal, and the test
  `the_core_uses_the_click_drag_distance` (`crates/turts_interaction/tests/click_drag_rule.rs`) pins both literals to the
  constant (plant 7k). The 3 L0-CORE plants are red and restored again (`soundoff3-plants-4`).
- `PointerOver`: a grep of `crates` for `PointerOver`, `Pointer<Over>`, `PointerEnter`, `PointerOut`, `PointerLeave`,
  `PickingInteraction` finds nothing outside the repair. Widgets, tooltip and cursor read the `Hovered` component
  (`widgets/tooltip.rs:92`, `widgets/activation.rs:19`), which Bevy sets from the hover map. The new repair keeps Bevy's
  `Over`: a test compares a quick click with a normal hover-then-click and both give `over, press, click, activate, release`.
- Red tests (all in `press_release::tests`; they ran red against a naive body first, `soundoff3-red-2`): press on the map and
  release over a button (0); press on button A and release on button B (0); touching buttons (0); different buttons (0);
  a drag that comes back (0); a far press and release on one button (0); a release with no press in the frame (0, L4);
  press and release on one button in one frame (exactly 1); a button hovered before is clicked once, not twice (1).
- The headed row SND2 found a defect that the native tests did not: a click with a hover frame and a press and release in one
  later frame was clicked by Bevy and again by the repair (the sound switched twice; SND2 red in 2 runs,
  `soundoff3-headed-muster-1`). Fixed, with its test and plant 7j.
- Plants of the repair: 7a to 7k, all red and restored (`soundoff3-plants-1`, `-3`, `-4`). Mutants: no press required, no press
  hit test, no drag flag, no distance rule, no button compare, `Pressed` not given back, threshold 4000, core literal 5.0,
  Bevy click sent twice.

## Headed probe (`tests/browser/click-without-hover-probe.js`, `scripts/click-without-hover.sh`)

At the merged head `4cbe008b` (code of the repair unchanged since), under `mc_headed_v6.sh`:
- fixture (engine fallback switch): the one-frame click acts within 500 ms; the map-press check leaves the sound off
  (`soundoff3-merged-probe-1`).
- forge, `--guide-close`: the mod-placed switch acts, the guide Close button acts, the map-press check leaves it off
  (`soundoff3-merged-probe-2`).
- Plant 7b (registration `run_if(|| false)`): the click check is red, `acted:false`, exit 1 (`soundoff3-probe-plant7-1`).
- LIMIT, measured: the new map-press check cannot go red in this browser. With the press hit test, the distance rule and the
  drag rule all removed (4 runs, `soundoff3-probe-plant7i-1` to `-4`, the last with a page CPU throttle of 20 and 12 tries) the
  sound still stays off: the browser input does not put press-on-map and release-on-switch into one frame the way the native
  test does. The cause is NOT TRACED. The proof of that case is the native red tests; the probe only shows that the real
  build does not act on it. The record of plant 7i was dropped (a plant that stays green is not a plant). The probe says
  this in its header.

## M2: the opt-out and the menu key

- Stated in 3 places: rustdoc of `ButtonAction::Input` (`crates/turts_mod_api/src/interface.rs`), `turts-pack --help`
  (test `pack_cli_help_states_the_sound_switch_rule`), erratum E-SND-5. The rule: the play screen is the mod's, a mod that places
  a `toggle_sound` button owns the control and may hide it with `visible`; the engine draws its own only when none is placed;
  every player can always reach the settings-menu switch, which opens with `menu`.
- Trace: the defaults of `menu` are F10 and Escape (`crates/turts_interaction/src/input_map.rs:91-103`). A mod CAN remove
  them: `engine_overrides: [(name: "menu", bindings: [])]`, or an authored action named `menu` with no key
  (`effective_actions`, `input_map.rs:109-125`, the last writer wins); nothing refused either.
- New 0.2.4 check `menu_keeps_a_key` (`crates/turts_mod_runtime/src/mp1_validation.rs`, in the KEY1 group; no version-table row).
  Text: `<package>/input/bindings.ron: engine_overrides[0].bindings: the engine action `menu` has no key; the settings menu holds
  the sound switch ...; bind a key to it, or leave `menu` out of `engine_overrides` (the keys are F10 and Escape)`.
  Test `the_engine_action_menu_keeps_a_key_in_a_package_of_the_edition_0_2_4` (empty, pointer-only, authored: refused; F9:
  packs; `^0.2.3`: not checked). Plants 11a, 11b.
- Key bound to `toggle_sound`: it passed the check of `engine_overrides` but did nothing (the core knew only `menu`,
  `toggle_guide`, `cancel` as shell actions). It now works: the core sends the shell action, the client flips the switch. Tests
  `a_key_bound_to_toggle_sound_sends_the_shell_action` and `the_shell_action_toggle_sound_flips_the_switch`; plants 14a, 14b.
  `leave_match` has the same gap (owner `UI-RESULT`), noted in E-SND-6.

## L1, L3, L2

- L1: the hint and the allowed list of a misspelt action offer only the names of the edition of the package
  (`ENGINE_ACTIONS_SINCE_0_2_4`); test `the_hint_for_a_misspelt_action_offers_only_the_names_of_the_edition`; plant 12a.
- L3: test `the_edition_0_2_3_baseline_is_the_base_commit_form_of_each_shipped_package` pins the sha256 of the stripped
  `interface/hud.ron` and `manifest.ron` of the six packages (equal to `54c39fdd`, taken with `git show`); plant 13a.
- L2: listed as open in the erratum (E-SND-7).

## Gates at the head before the last small commits and at `ee9bd0dd`

- `cargo fmt --check`, `clippy --workspace --all-targets --locked -D warnings`, `cargo test --workspace --locked` (2108
  passed, 0 failed, 38 ignored), wasm32 check, `scripts/boundary.sh`: all 0 (`soundoff3-last-*`). The API crates' tests
  (`turts_mod_api`, `turts_mod_runtime`, `turts_pack`, `turts_interaction`) are inside that run.
- The six packages pack with exit 0 and no stderr (`soundoff3-last-pack6-1`).
- `mp1_plants.py --check --all`: 470 records apply, 0 stale; `--self-test` 64 of 64.
- Node tests 349/349 at the merged head; driver rule self-test and tree pass.
- Plants: 42 records in `sound-off.json` (3 headed). All 39 non-headed red and restored in pieces: `soundoff3-plants-1`
  (34 of 38 at first; 7e, 7h, 11a, 11b were re-pinned and re-run red in `-2`), `-3` (the 8 of `7*` after a rename), `-4`
  (7h, 7k, 3 L0-CORE). The 55 older records on files I touched: 52 red at once; the 3 L0-CORE ones after the revert above.
- A rename was needed: `crates/turts_checks` (no-content scan) read the local name `press` as the forge unit `press`.
- Headed rows through `mc_headed_v6.sh`, prepared, 2 runs each, at the merged head `4cbe008b`: SND1, ORD6, ORD7, ALR1 pass
  (`soundoff3-merged-headed-1`, hold 60 s); SND2 passes (`-2`). The later commits changed no client code except the
  core literal revert and a test file.

## Still red, not from this change

AUD1 (stale slider coordinates, red at base `54c39fdd`) and HUD2 (Controls tail check "Type modifier"): not run in pass 3.

## NOT TRACED

- Why the browser does not give the one-frame press-on-map, release-on-switch sequence (probe limit above).
- Native desktop client; Firefox and Safari.
- AUD1 oracle additions end to end.
