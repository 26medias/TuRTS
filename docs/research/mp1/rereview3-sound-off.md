VERDICT=FAIL HIGH=1

# Re-review, pass 3: "sound off by default" (mod-builder, consumer and API lens)

Candidate: `mp1/sound-off-default` at `ee9bd0dd`, which has `origin/mp1/integration` `4553acc4` merged in `4cbe008b`.
Worktree: `/home/julien/Projects/TuRTS-mp1-l0-api1-review`, detached. Date: 2026-11-12. What I changed in the repository:
none. I recreated the empty cache folder `target/mc-mp1-l0-api1-review`, because it was gone (the disk is 95 % full).
Scratch: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/sound-off-review3/`.

Counts: 7 findings (1 High, 2 Medium, 4 Low). 4 faults of my own (2 red, 2 green, which show test gaps). 12 sample
packages, 2 temporary probe test files (deleted after the run), and 6 of the 42 plant records.

## Findings

### High

**H1. `menu_keeps_a_key` counts the keys of `menu`, not the keys that open the menu.**
- File: `crates/turts_mod_runtime/src/mp1_validation.rs:1376-1414`.
- **What packs.** Two `^0.2.4` packages pack with exit 0 and leave the player no way to reach a sound switch:
    - `a1_menu_shadowed`: `engine_overrides: [(name: "menu", bindings: [Key(code: "KeyM")]), (name: "cancel", bindings: [])]`.
      `cedar` already binds `key_move` to `KeyM`.
    - `a5_menu_modifier_key`: `menu` on `ShiftLeft`, and `cancel` with no binding.
    - In both, the shipped switch of `cedar` has `visible: [GuideOpen, Not(GuideOpen)]`. That widget is never visible,
      and it still turns off the engine fallback.
- **Core probe** (temporary test in `crates/turts_interaction/tests`, the shipped `Core::step`, the package input
  map): `KeyM`, `ShiftLeft`, `Escape` and `F10` give no `ShellAction { action: "menu" }`. The control package
  (`a6`: switch hidden, `menu` not overridden) gives `menu` on `Escape` and `F10`.
- **Why it happens:**
    - `action_for_key` takes the first match in the name order of a `BTreeMap`
      (`crates/turts_interaction/src/bindings.rs:126-132`). `key_move` comes before `menu`, so `KeyM` is never `menu`.
    - The rule for unique default keys compares the authored `actions` only. It does not compare `engine_overrides`
      (`mp1_validation.rs:1416-1440`).
    - `Escape` reaches the menu only through `cancel` (`crates/turts_interaction/src/lib.rs:894-911`), and the package
      can remove that binding.
- **Error text:** none, because these packages pack. The pointer-only case (`a4`) gets a good text that a modder can
  act on: `` cedar/input/bindings.ron: engine_overrides[0].bindings: the engine action `menu` has no key; ... (the keys are F10 and Escape) ``.
- **Fix: needs decision.** Options:
    - (a) In a package of edition 0.2.4, `menu` keeps at least one key that no other effective action binds with the
      same code and modifiers, and that is not a modifier key alone. The rule must also compare the overrides with the
      authored actions.
    - (b) F10 stays bound to `menu` always: the engine does not let a mod override it.
    - (c) The engine draws its fallback switch unless the switch of the mod can become visible. This is hard to prove
      on a `visible` predicate list.

  I recommend (b), or (a) plus a red package for each case of H1.

### Medium

**M1. The probe starts a Chrome with no sound isolation of its own.**
- File: `tests/browser/click-without-hover-probe.js:61-68`.
- The probe calls `chromium.launch`. Its default arguments, when `MC_CHROME_ARGS_JSON` is not set, do not have
  `--alsa-output-device=null`, and the call has no `silentEnv`.
- The probe is silent only under the headed wrapper, which exports the argument and `ALSA_CONFIG_PATH`
  (`target/mc-evidence/gate-runner/mc_headed_v6.sh:749`, `:752`).
- `scripts/click-without-hover.sh` only tells the reader to use the wrapper. Nothing enforces it.
- The probe also turns the sound ON (its click check).
- Neither scan sees this file:
    - `tests/browser/harness/silent-chrome-scan.test.js` reads only `*.test.js` files;
    - its script scan reads only `scripts/`.
- Fix: put `SILENT_ARGS` and `silentEnv(...)` (`tests/browser/harness/silent-chrome.js`) in the probe itself, and let
  the scan read every `tests/browser/**/*.js` that starts a Chrome.
- I did not start a browser.

**M2. A `^0.2.3` package can bind a key to a 0.2.4 engine action, and it packs.**
- Probes `b2_023_toggle_sound_key` and `b3_023_leave_match_key`: `cedar` from `54c39fdd` with
  `engine_overrides: [(name: "toggle_sound" | "leave_match", bindings: [Key(code: "KeyT")])]`. Both give exit 0.
- The version rows look only at the `Button` action (`variants.rs`: `engine action toggle_sound`,
  `engine action leave_match`). They do not look at `engine_overrides[].name`.
- A released 0.2.3 engine does not know these names.
- `toggle_sound` is new in this branch. `leave_match` (H23) had the same gap before it.
- Fix: one version row for each name in `engine_overrides` and in an authored action name.

### Low

**L1. The repair can make the client panic.**
- `commands.entity(*entity).insert(Pressed)` is at `crates/turts_client/src/press_release.rs:173-175`.
- Probe `p2`: a double click in one frame on a button whose `Activate` observer despawns that button panics in Bevy's
  error handler (`bevy_ecs-0.20.0/src/error/handler.rs:128`): "`insert<Pressed>` ... Entity despawned".
- No current `On<Activate>` observer of the client despawns its button in the same command flush
  (`widgets/activation.rs:52`, `client_screens.rs:600`, `client_shell/settings.rs:481`,
  `client_shell/bindings_editor.rs:785`, `volume_settings.rs:154`), so this is latent.
- Fix: `try_insert`.

**L2. The repair presses a disabled button.**
- Probe `p1`: a button with `InteractionDisabled` has `Pressed` after a one-frame click, and keeps it. There is no
  activation.
- Bevy never presses a disabled button (`bevy_ui_widgets-0.20.0/src/button.rs:91-94`). Its release handler does not
  take `Pressed` off a disabled button (`:108-110`).
- After the button is enabled again, a normal click acts once (`p1 after enable: activations=1`). The effect is a
  stale look.
- Fix: skip an entity with `InteractionDisabled`.

**L3. Two of my faults stay green (test gaps).**
- F3: I removed the `PointerAction::Cancel` arm (`press_release.rs:100-102`). All 509 `turts_client` tests stay green.
- F4: I keyed the presses by one pointer, not by `pointer_id` (`:68-87`). All 509 tests stay green.
- The repair's behaviour in both cases is right today: probe `p5` (press, cancel, release: 0 activations) and probe
  `p4` (touch press and mouse release: 0; two clicks interleaved: 2). But no test makes them red.

**L4. The map-press check has no "not evidence" mark in its output.**
- The header (`click-without-hover-probe.js:11-18`) and the report (`sound-off-pass3-report.md:46-50`) state the
  limit honestly.
- The check is in no row and no plant: plant 7b runs the script without `--map-press-first`.
- But the JSON entry (`:113`) has the same form as a real check, and it counts in the exit code of the probe.
- Fix: mark the entry (for example `"evidence": false, "can_fail": false`), or leave it out of the exit code.

## Check 1: the press-release repair (review M1, L4)

- **Closed:** review M1 (the crossing click) and review L4 (a release with no press).
    - The tests `a_press_on_the_map_and_a_release_over_a_button_in_one_frame_activate_nothing`,
      `a_press_on_one_button_and_a_release_on_another_in_one_frame_activate_neither` and
      `a_release_with_no_press_in_the_frame_is_left_to_bevy` (`press_release.rs:402-566`) cover them.
    - My pass-2 probe case (press at the map, release on the button, one frame) is now a red test.
- **My faults** (not among 7a to 7k):

| Fault | Change | Result | Restore |
| --- | --- | --- | --- |
| F1 | `Release` triggered before `Click` (`:176-189`) | red, exit 101: `a_press_and_release_on_one_button_in_one_frame_activate_it_once`, `a_press_on_one_button_and_a_release_on_the_button_next_to_it_click_neither`, `a_drag_that_comes_back_to_the_button_in_one_frame_is_not_a_click` | `cmp` 0 |
| F2 | the `PointerRelease` trigger removed | red, exit 101: `a_press_and_release_on_one_button_in_one_frame_activate_it_once` (the button stays pressed) | `cmp` 0 |
| F3 | the `Cancel` arm removed | green, 509/0 (L3) | `cmp` 0 |
| F4 | the presses keyed by one pointer | green, 509/0 (L3) | `cmp` 0 |

  The fresh run on the clean tree after all four restores: 509 passed, 0 failed (`soundoff-rerev3-faults-green`).
- **Attempts at a double activation or a lost click** (probe `crates/turts_client/tests/zz_mb_click_probe.rs`,
  deleted after the run):
    - a disabled button: L2;
    - a despawn between two clicks: L1 (panic);
    - a double click in one frame: 2 activations, with or without a hover frame. This is consistent: Bevy itself
      gives 2;
    - two pointers: right;
    - a cancel between press and release: right.

    I found no double activation.
- **SND2.** Plant 7j removes the check of the hover map of the frame before (`:158-163`). The test
  `a_button_that_was_hovered_before_is_clicked_once_by_bevy_not_twice` then goes red with "one activation, not two"
  (my plant sample). So the new test is red without the repair of that defect.
- **The guide and modal layers:** NOT TRACED. They need a headed run.

## Check 2: the probe limit

The probe header and the builder report both say "this check only shows that the real build does not act on it".
The check is not in a row and not in a plant. Its output entry is not marked (L4). I count it as no evidence.

## Check 3: the menu rule (review M2)

| Package | Result |
| --- | --- |
| `a1_menu_shadowed` | exit 0, no reachable switch (H1) |
| `a2_menu_fake_key` | `` engine_overrides[0].bindings[0].code: unknown physical key code `NoSuchKey` `` |
| `a3_menu_reserved` | `engine_overrides[0].bindings[0]: Ctrl+KeyW is reserved by the browser` |
| `a4_menu_pointer` | the menu-key text above, with the action to take |
| `a5_menu_modifier_key` | exit 0, no reachable switch (H1) |
| `a6_hidden_switch_menu_default` | exit 0; `menu` opens on F10 and Escape (allowed: the settings switch is reachable) |
| `a7_menu_two_overrides` | exit 0; the last override wins in the rule and at run time (`input_map.rs:108-122`) |
| `a8_023_menu_unbound` (`^0.2.3`) | exit 0. The column does not run for edition 0.2.3, but a 0.2.3 package cannot place `toggle_sound`, so the engine fallback switch shows. Reachable |

The rule is stated in `turts-pack --help` (lines 28-34 of the output) and in the rustdoc of `ButtonAction::Input`.

## Check 4: the key bound to `toggle_sound`

- `b1_toggle_sound_key` (`^0.2.4`, `toggle_sound` on `KeyT`) packs. The core gives
  `ShellAction { action: "toggle_sound" }` on `KeyT` (core probe).
- The client flips the switch for that effect (`crates/turts_client/src/widgets/mod.rs:146-152`, test
  `the_shell_action_toggle_sound_flips_the_switch`).
- **What `leave_match` still lacks** (owner `UI-RESULT`):
    - its activation is a stub that sends nothing (`crates/turts_client/src/widgets/activation/leave.rs:8-13`);
    - a key bound to it does nothing: the core has no arm for it (`turts_interaction/src/lib.rs:881`, `:951`);
    - a `^0.2.3` package can bind a key to it (M2).

## Check 5: review L1 and L3

- **L1: closed.**
    - `s8_typo_023` now gets ``unknown input action `toggle_sond` (allowed: ...)``. The allowed list has no 0.2.4
      names and the text gives no hint (`content_mc_validation.rs:1526-1580`).
    - `s3_typo_024` keeps the hint ``did you mean `toggle_sound`?``.
- **L3: closed.** The test `the_edition_0_2_3_baseline_is_the_base_commit_form_of_each_shipped_package`
  (`crates/turts_mod_runtime/tests/val1.rs:1759`) pins the sha256 of each file. I hashed two packages myself with
  `git show 54c39fdd:...`, and both match the pins:

| Package | `hud.ron` | `manifest.ron` |
| --- | --- | --- |
| cedar | `d036dfa250abae51` | `9bff7b914e43fb54` |
| muster | `79e532ea199cca15` | `c4b8ab7c62a87de9` |

  Plant 13a is red in my sample.

## Check 6: the sound rule

- `scripts/mp1-play.sh` and `scripts/cold-play.sh` have the silent form, and the node script scan says so.
- `click-without-hover-probe.js` does not have it (M1).
- `scripts/click-without-hover.sh` does not start a Chrome itself.
- I started no browser.

## Check 7: the gates at `ee9bd0dd`

All commands ran through `exec_in_lane_v4.py --head ee9bd0dd`. The labels are `soundoff-rerev3-*`.

| Label | Command | Exit |
| --- | --- | --- |
| fmt-b | `cargo fmt --all --check` | 0 |
| build-pack-b | `cargo build -p turts_pack --locked` | 0 |
| test-client-b | `cargo test -p turts_client --locked` | 0 (509 passed) |
| test-workspace-b | `cargo test --workspace --locked --no-fail-fast` | 0 (2108 passed, 38 ignored) |
| clippy-b | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| wasm-b | `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked` | 0 |
| boundary-b | `bash scripts/boundary.sh` | 0 |
| plants-check | `mp1_plants.py --check --all` | 0, "470 record(s) apply, 0 stale" |
| plants-sample | `--plants-dir` with 7g, 7j, 11a, 11b, 12a, 13a | 0, 6 red and restored |
| node-unit | `node --test` on `harness/sound-switch-pixels.test.js` and `harness/silent-chrome-scan.test.js` | 0, 16 of 16 pass |
| probes | the two temporary probe tests | 0 |
| f1-order, f2-norelease, f3-nocancel, f4-onepointer, faults-green | the fault runs | 101, 101, 0, 0, 0 |

- The six shipped packages pack with exit 0.
- The node tests that start a Chrome (`cold-play.test.js`, `cold-probe.test.js`, `wait-sync.test.js`,
  `mp1-capture.test.js`) did not run (the no-browser rule), and `node_modules` is not in the review tree.
- The first gate chain (labels without `-b`) did not start, because the cache folder was missing. It recorded no
  result.

**Merge.**
- `origin/mp1/integration` is `4553acc4`. It is an ancestor of `ee9bd0dd`, and `merge-tree` gives exit 0.
- `origin/mp1/forward-mc` is `f4020d1f`, with merge base `4553acc4`. `merge-tree` gives exit 1, with content
  conflicts in `.github/workflows/ci.yml`, `tests/browser/feedback-driver.js` and `tests/browser/feedback-oracle.js`.
  The branch that merges second must resolve them.

## NOT TRACED

- The guide and modal-layer cases of check 1 (they need a headed run).
- Why the browser does not give the one-frame map press (the builder also leaves this NOT TRACED).
- The node tests that start a Chrome.
- The headed rows SND1, SND2 and HUD2.
- 36 of the 42 plant records.
- Whether a player can unbind every `menu` key in the bindings editor. That editor is inside the menu.
