RESULT=PASS-NATIVE
# Sound-off pass 4 report

Branch `mp1/sound-off-default`, worktree `/home/julien/Projects/TuRTS-mp1-sound-off`.
Head: `3b9fe6fa01fc8b0c545379fa010b2ace19f5390c` (pushed). Code commit: `7ecb09afcfbffa9b18cbdeaaceb970186462a277`.
Base of the pass: `ee9bd0dd`. Items 1 to 5 are done. Owed: the merge of `mp1/forward-mc` (item 6) and the headed rows (item 7).
Nothing headed ran. No Chrome ran in this pass.

## Items

### Item 1, H1: F10 and the fallback (option (b) + guard)
- Core (every edition): a plain F10 gives `menu` before any binding of the mod or the player.
  `crates/turts_interaction/src/bindings.rs:7` (`RESERVED_MENU_KEY`) and `:131-137` (`action_for_key`). Escape through `cancel` is unchanged.
- Packer (edition 0.2.4 only): `menu_keeps_f10` in `crates/turts_mod_runtime/src/mp1_validation.rs`
  (replaces `menu_keeps_a_key`). It refuses: `menu` in `engine_overrides` with no plain F10; F10 on another engine action;
  F10 on an authored action; an authored action named `menu`. Each text names package, file, field and the fix.
- Tests, red first (run `soundoff4-red-1`, log `p4-red-core.log` in the scratchpad: 3 of 4 red, the control green):
  `crates/turts_interaction/tests/f10_reserved.rs` (reviewer packages `a1` = `menu` on `KeyM` with `cancel` unbound, `a5` = `menu` on `ShiftLeft`
  alone, F10 before the bindings of mod and player, default keys, and `only_a_plain_f10_is_reserved`); `crates/turts_mod_runtime/tests/val1.rs`
  `f10_is_reserved_for_the_settings_menu_in_a_package_of_the_edition_0_2_4` and `no_shipped_package_binds_f10`. Both a1 and a5 are
  refused at pack time and open the menu with F10 through the real `Core::step`.
- 0.2.3 package: the packer does not check it. At run time F10 now opens the menu and a 0.2.3 action on a plain F10 does not fire.
  The six shipped packages bind no F10 (test `no_shipped_package_binds_f10`). Stated in the erratum E-SND-5.
- Fallback (1c): `sync_sound_switch` (`crates/turts_client/src/client_shell/notices.rs:~207-232`) shows the engine switch in each frame in which no
  `toggle_sound` button is visible (`WidgetButtonAction` plus `InheritedVisibility`). No static check. Red run `soundoff4-red-1`
  (log `p4-red-client.log`: 3 shell tests red). Tests in `crates/turts_client/src/client_shell/tests.rs`:
  `the_engine_draws_its_switch_in_each_frame_in_which_no_mod_switch_is_visible` (includes `[GuideOpen, Not(GuideOpen)]`),
  `a_visible_mod_switch_of_a_pair_hides_the_engine_switch_and_another_action_does_not`,
  `the_six_shipped_layouts_place_the_switch_and_a_visible_one_hides_the_engine_switch`.
- The three places that state the opt-out are updated: rustdoc of `ButtonAction::Input` (`crates/turts_mod_api/src/interface.rs`), `turts-pack --help`
  (`crates/turts_pack/src/main.rs`; test `pack_cli_help_states_the_sound_switch_rule` was red first, run `soundoff4-red-help`, then green), and
  erratum E-SND-1, -5 (`docs/research/mp1/errata-sound.md`).
- "F10 is reserved for the settings menu" is recorded in E-SND-5 as Baseline, for the `ux-designer` to confirm in design revision 6.3 (D66).

### Item 2, M1: the probe starts a silent Chrome
`tests/browser/click-without-hover-probe.js`: `args: [...chromeArgs without the null device, ...SILENT_ARGS]` and `env: silentEnv(folder)` from
`tests/browser/harness/silent-chrome.js`; the folder is removed in `finally`. Proof: a scratch copy of the forward-mc scan
(`git show origin/mp1/forward-mc:tests/browser/harness/silent-chrome-scan.test.js`, only the two root constants and the scope list changed,
because `docs/measurements/probes` is not on this branch) was RED for the probe (`.launch( has no silentEnv and SILENT_ARGS`) before the edit and
has no line for the probe after it. No second scan was added to the repository. The real scan runs after the forward merge.

### Item 3, M2: version rule for a key bound to `toggle_sound`
`crates/turts_mod_runtime/src/variants.rs` (`input_uses`): `engine_overrides[i].name` and `actions[i].name` equal to `toggle_sound` use the row
`engine action toggle_sound`. Test `a_key_bound_to_toggle_sound_needs_api_0_2_4` (`val1.rs`): `^0.2.3` refused with the text
`... engine action toggle_sound needs engine api 0.2.4 ... (written in input/bindings.ron: engine_overrides[0].name)`; `^0.2.4` packs.
`leave_match` at `^0.2.3`: NOT changed. The same rule would give a second refusal at `^0.2.4` through its `UNBUILT` line
(`crates/turts_mod_runtime/src/unbuilt.rs:187`, owner `UI-RESULT`). This is read, not run. Recorded for `UI-RESULT` in E-SND-6; the test shows the gap.

### Item 4, L1 to L4
- L1: `try_insert(Pressed)` (`crates/turts_client/src/press_release.rs`). Test
  `a_button_that_despawns_itself_on_activate_is_not_pressed_again_in_the_same_frame`. Red: panic in `bevy_ecs-0.20.0/src/error/handler.rs:128` at `insert<Pressed>`.
- L2: a disabled button gets no `Pressed` and no click (`states: Query<(Has<UiButton>, Has<InteractionDisabled>)>`). Test
  `a_disabled_button_is_left_alone_by_the_repair`; red first.
- L3: tests `a_cancel_between_the_press_and_the_release_is_not_a_click` (F3) and `the_presses_of_two_pointers_do_not_mix` (F4). Plants 18c and 18d
  remove the Cancel arm and key the presses by one pointer: both red.
- L4: the map-press entry has `"evidence": false, "can_fail": false` and does not count in the exit code (`exitCode`, exported). Test in
  `tests/browser/harness/sound-switch-pixels.test.js`; plants 19a, 19b red.

### Item 5: plants
`docs/research/mp1/plants/sound-off.json`: 59 records (3 headed: 6a, 6b, 7b), pinned to `7ecb09af`. New: 8a, 8a2, 8a3 (fallback rule), 11a to 11e (F10 packer rule),
16a to 16c (core), 17a, 17b (version rule), 18a to 18d (L1 to L3), 19a, 19b (probe), 15b (help). Re-pinned: 3c, 7g, 8a, 11a, 11b, 15a text.
Runs, all red then restored, tree clean after each: `soundoff4-plants-1` (25 red-restored, 6 failed on placeholder red texts, fixed),
`soundoff4-plants-2` (22 of 22), `soundoff4-plants-3` (25 of 25; 2 headed skipped by the runner). `mp1_plants.py --check --all`: 487 records, 0 stale. `--self-test`: 64 of 64.

## Gate table (native, at the code commit `7ecb09af`; plants commit adds one JSON file)

| Gate | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --check` | exit 0 | direct run |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 | `soundoff4-clippy-2` (the first run was red on 4 findings, fixed) |
| `cargo test --workspace --locked` | 2121 passed, 0 failed, 38 ignored (was 2108) | `soundoff4-ws-1` |
| `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked` | exit 0 | `soundoff4-wasm-1` |
| `scripts/boundary.sh` | boundary ok | `p4-boundary.out` |
| six packages pack | all six exit 0, 0 stderr bytes | `pack6.sh` (fresh `turts-pack`, `soundoff4-buildpack-1`) |
| node unit tests | 350 pass, 0 fail (was 349) | `p4-node.out` |
| driver rule | self-test 47 cases PASS; tree exit 0 | direct run |
| plants | see item 5 | |

Evidence folders: `/home/julien/Projects/TuRTS/target/mc-evidence/soundoff4-*`. Scratchpad: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/sound-off/`.

## Item 6 and item 7 (owed)
`mp1/forward-mc` is not merged. A dry run with `git merge-tree` shows conflicts in `tests/browser/feedback-driver.js` and `tests/browser/feedback-oracle.js`
(and `.github/workflows/ci.yml`, as known). Headed rows (SND1, SND2, ORD6, ORD7, ALR1, HUD2, AUD1, the probe) did not run. They run after the merge,
under `mc_headed_v6.sh`, with the read-only `pw-dump` watcher.
`.github/workflows/ci.yml` is not changed in this pass (the new node test is inside the existing file `sound-switch-pixels.test.js`).

## NOT TRACED
- Headed behaviour of F10 and of the fallback switch in a browser. Only native tests ran.
- The silence of the probe's Chrome: shown by the scan only, not by a run and not by `pw-dump`.
- The `leave_match` claim about the `UNBUILT` line (read, not run).
- The wide forward scan on the real merged tree (the scratch copy covered `tests/browser` and `scripts` only).
- Whether a published 0.2.3 package binds F10 (no shipped package does).
