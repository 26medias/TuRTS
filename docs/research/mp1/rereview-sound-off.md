VERDICT=PASS HIGH=0

# Re-review, pass 2: "sound off by default" (mod-builder, consumer and API lens)

Candidate: `mp1/sound-off-default` at `314e2722`, on `origin/mp1/integration` `54c39fdd`. Worktree:
`/home/julien/Projects/TuRTS-mp1-l0-api1-review`. Date: 2026-11-12. Mode: review. What I changed in the repository: none.
Scratch: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/sound-off-review2/`.

Counts: 6 findings (0 High, 2 Medium, 4 Low). 2 faults of my own (1 red, 1 green, which shows a test gap). 9 sample
packages. 8 of the 27 plant records sampled.

## Findings

### Medium

**M1. The press-release repair can make a button act when the press started somewhere else.**
- File: `crates/turts_client/src/press_release.rs:34-49`.
- What the repair does: when a pointer has a press and a release in the same frame, it adds the button under the
  pointer now to the hover map of the frame before. It does not compare the place of the press with the place of
  the release. `PointerInput` carries both places.
- My probe (temporary file `crates/turts_client/tests/zz_mb_press_probe.rs`, deleted after the run; log
  `soundoff-rev2-press-probe-2`) gives these results:

| Case | Without the repair | With the repair |
| --- | --- | --- |
| One frame: press at x=100 (the map), release at x=3 (the button), the frame hovers the button | 0 activations | **1 activation** |
| Drag over frames: press on the map in frame 1, release over the button in frame 2, the first frame that hovers the button | 0 | 0 |
| One-frame click on the button | 0 activations, 1 `PointerOver` | 1 activation, **0 `PointerOver`** |

- So a quick box-select that ends on a HUD button within one frame (more likely at a low frame rate) now
  activates the button.
- Bevy alone presses the button in that case (`bevy_picking-0.20.0/src/events.rs:1060` sends the press to this
  frame's hover map), but it does not click it.
- The tests of the task do not have this case. They have no drag that ends over a HUD button
  (`press_release.rs:143-201`; a grep of the diff finds no new drag test).
- Fix: apply the repair only when the press and the release of the frame are at the same place, or within a click
  distance. Add the one-frame drag case as a red test.

**M2. The opt-out from the engine switch is a side effect, not a stated rule.**
- The engine fallback is suppressed by any placed `Input("toggle_sound")` button, also one that is never visible
  (`InterfaceDef::places_input_button`, `crates/turts_mod_api/src/interface.rs:43-60`).
- My package `s7_hidden_optout_024` packs with exit 0, with only the 1 x 1 px warning. It holds a 1 x 1 button
  with `visible: [GuideOpen, Not(GuideOpen)]`, so the play screen has no sound control.
- My ruling on the CEO rule "silent by default, with a way to turn it on":
    - The rule needs one way that every player can always reach. It does not need a control on the play screen.
    - The settings menu always holds a switch (`crates/turts_client/src/volume_settings.rs:189`).
    - The menu opens with the engine action `menu`, which has the engine default keys F10 and Escape
      (`crates/turts_interaction/src/input_map.rs:95-106`). Whether a mod override can remove both keys is NOT TRACED.
    - D67 gives the play-screen layout to the mod. So a mod may opt out of the play-screen switch.
- The opt-out must be a documented rule, not a hack. Errata E-SND-1 line 48 says "A mod can hide its button with
  `visible`; it still counts as placed". Make that the stated opt-out in the API documentation.
- Also state that `menu` keeps at least one binding in every mod. If an override can unbind both keys, either
  forbid that, or keep the play-screen fallback for such a mod.

### Low

**L1. A misspelt action in a `^0.2.3` package gets a hint to a 0.2.4 name.**
- Probe `s8_typo_023`: the allowed list holds `toggle_sound` and `leave_match`, and the hint is
  ``did you mean `toggle_sound`?``.
- If the modder follows the hint, the next run gives the version refusal.
- Fix: filter the engine action list by the edition of the package (`content_mc_validation.rs:1522-1538`).

**L2. Two path forms for the same field.**
- The typo text uses `root[0].widget.children[0].widget.action` (an index form).
- The version text uses `root[snd_quiet].widget.children[snd_quiet_btn].widget.action` (an id form).
- A modder reads two forms for one field. This is pre-existing (MC validation); I record it.

**L3. The runtime 0.2.3 baseline is a strip at run time.**
- The helper `as_edition_0_2_3` (`crates/turts_mod_runtime/tests/common/mod.rs:42-68`) is called from every
  `read_files`.
- Today the strip gives the base-commit files byte for byte (see question 2).
- Nothing pins that equality. A later edit of a shipped `hud.ron` changes the "0.2.3 package" of the tests with
  no red.
- Fix: one test that compares the stripped files of each shipped package with a pinned sha256 of their
  `54c39fdd` form. The other fix is a committed 0.2.3 fixture.

**L4. No test that a release with no press in the same frame leaves Bevy's behaviour unchanged.**
- My fault 2 removes the guard `if pressed.contains(&input.pointer_id)` (`press_release.rs:40`). All 488
  `turts_client` tests stay green.
- With that fault, a press on a button, a move off, and a return with the release in the first hover frame back
  acts.

## Question 1: a package written from the documentation and the public types

Source: errata E-SND-1 and the rustdoc of `Value::SoundState` and `Predicate::SoundOn` (`interface.rs:236-241`,
`:353-355`). I wrote two frames with a fill each, with `visible: [Not(SoundOn)]` and `[SoundOn]`, round a
`Button(label: Some(Bind(SoundState)), action: Input("toggle_sound"))`, on a copy of `cedar` at `54c39fdd`.

| Probe | Result |
| --- | --- |
| `s1_switch_024` | exit 0 |
| `s2_switch_023` | `` cedar/manifest.ron: engine_api: `engine action toggle_sound` needs engine api 0.2.4; `^0.2.3` admits 0.2.3 (written in interface/hud.ron: root[snd_quiet].widget.children[snd_quiet_btn].widget.action) `` |
| `s4_value_only_023` | `` `Value::SoundState` needs engine api 0.2.4; ... (written in interface/hud.ron: root[snd_words].widget.text) `` |
| `s5_pred_only_023` | `` `Predicate::SoundOn` needs engine api 0.2.4; ... (written in interface/hud.ron: root[snd_mark].visible) `` |
| `s6_action_only_023` | the `toggle_sound` row, path `root[snd_plain].widget.action` |
| `s3_typo_024` | ``cedar/interface/hud.ron: root[0].widget.children[0].widget.action: unknown input action `toggle_sond`; did you mean `toggle_sound`? (allowed: ...)`` |
| `s7_hidden_optout_024` | exit 0, with the 1 x 1 px warning (M2) |

- Each name is refused at `^0.2.3` with the package, the file and the field. The text says which API the name
  needs.
- Where the fallback rule is written:
    - the rustdoc of `Value::SoundState` (`interface.rs:236-240`);
    - `places_input_button` (`:44-46`);
    - errata E-SND-1.
- The rule is not in `turts-pack --help`, and not on `ButtonAction::Input`. Those are the places where a modder
  meets the action name. This is part of M2.

## Question 2: the edition move and the 84 tests

- **Byte equality of the strip.** I applied the strip of `as_edition_0_2_3` to the six shipped packages at
  `314e2722`. For all six, the `hud.ron` and `manifest.ron` are byte-equal to `54c39fdd` (a Python compare with
  `git show`). The branch changes no other package file (`git diff --stat 54c39fdd 314e2722 -- mods`: 12 files,
  only `hud.ron` and `manifest.ron`).
- So every runtime test that reads a shipped package through `read_files` gets exactly the old 0.2.3 package.
  Its property is unchanged. My sample of 12, from 6 families:
    - `api_0_2_4.rs`: `a1_a2_a3_new_export_groups`, `h7_value_variants_in_a_text_widget`, and the `raw_case!` cases
      `u1_armour` and `h19_role`;
    - `pack.rs`: `pack_is_reproducible`, `pack_orders_manifest_first`;
    - `raw_presence.rs`: `the_version_table_speaks_before_the_content_checks`;
    - `val1.rs`: `t2_t3_t4_edition_pairs_and_the_control`, `key1_b_unique_default_keys`;
    - `visuals.rs`: `the_control_package_has_one_visual_per_unit`,
      `a_unit_without_a_visual_names_the_manifest_and_exports_visuals`;
    - `archive.rs`: one read.

    Each test reads through `read_files`, so it gets the stripped files, which equal the base files.
- **The 4 moved pins** are not edition properties, so the move hides no 0.2.3 regression:
    - `forge_package.rs:20`, `economy_package_boot.rs:459` and `mc_packages.rs:172` assert the shipped manifest;
    - `mc_packages.rs:292` (`a_new_order_binds_a_behaviour_by_data`) tests a data order.
- **Decision on a committed fixture:** keep the strip, but pin it (L3).
- **A real third-party `^0.2.3` package.** I extracted `cedar` at `54c39fdd` (`git archive`). It packs with exit 0
  on this head (probe `s0`). Its `hud.ron` has no `toggle_sound` (grep count 0), so `sync_sound_switch` draws the
  engine fallback for it (`client_shell/notices.rs:209-221`). The test
  `the_engine_draws_its_switch_only_when_the_interface_places_no_toggle_sound_button`
  (`client_shell/tests.rs:2904`) covers an interface with no switch.
- **No private shortcut.** The fallback depends only on the interface content, and no line of the diff names a
  shipped package id outside tests.

## Question 3: the frozen things

- `tests/m3/m2_operation_stream.json`: sha256 `bcbf72c2206251f6...`. The stream that the emitter writes on this
  head is `cmp`-equal to it.
- `m2_trace_writes_four_packages` (`TURTS_M2_TRACE_OUT` set to scratch) gives these body hashes (`tail -n +2`):

| Package | Body hash | Lines (`wc -l`) | Equal to the record |
| --- | --- | --- | --- |
| base | `d78dc09392e5b969` | 2,897 | yes |
| haul | `887ce95f69801702` | 4,819 | yes |
| acme | `ba1c849aaee46dd4` | 801 | yes |
| cedar | `09f637325c511972` | 4,711 | yes |

  The line counts are `wc -l` counts, and they agree with `repair3-l0-sim-report.md:33`.
- The header `emitter_sha256` `04caf579...` equals the sha256 of `m2_trace.rs`, which this branch does not change.
- The six shipped packages pack with exit 0.

## Question 4: the press-release repair

- I read `press_release.rs` and its four tests.
- **Fault 1 (red).** I changed `PointerAction::Press(_)` to `Press(Secondary)`.
  `cargo test -p turts_client --lib press_release` gave exit 101, with 2 tests failed:
  `a_press_and_release_with_no_earlier_hover_frame_activates_the_button` and
  `a_press_and_release_over_no_button_activate_nothing`. I restored the file from a copy: `cmp` 0 and git clean.
  The fresh `cargo test -p turts_client` gave 488 passed and 0 failed (`soundoff-rev2-faults-green`).
- **Fault 2 (green, which gives L4).** I removed the press guard. 488 passed. Restored exactly (`cmp` 0).
- **Cross-over cases.**
    - A press outside and a release over the button in a later frame: no activation, with or without the repair.
    - A press and a release in one frame at two places: activation with the repair (M1).
    - The drag-select over frames that ends on a HUD button is unchanged. No test of the task covers it.

## Question 5: the gates

All cargo commands ran through `exec_in_lane_v4.py --head 314e2722`. The labels are `soundoff-rev2-*`.

| Label | Command | Exit |
| --- | --- | --- |
| test-api | `cargo test -p turts_mod_api -p turts_mod_runtime -p turts_pack --locked --no-fail-fast` | 101: 3 tests in `turts_pack/tests/economy_variants.rs:36` say "build crates/turts_checks example m2_variants first". The cache was empty: this is a prerequisite, not a defect |
| test-api-2 | the same, after the workspace run built the example | 0 (620 passed) |
| build-pack | `cargo build -p turts_pack --locked` | 0 |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| test-workspace | `cargo test --workspace --locked --no-fail-fast` | 0 (2024 passed, 38 ignored) |
| m2-trace | `cargo test -p turts_pack --locked --test m2_trace -- --ignored` | 0 |
| press-probe, press-probe-2 | the temporary probe test (the first did not compile: the Bevy 0.20 name is `PointerOver`) | 101, 0 |
| fault1-red, fault2-noguard, faults-green | the fault runs | 101, 0, 0 |
| plants-sample | `mp1_plants.py --plants-dir <8 records> --all` | 0: 8 red and restored (1a, 2a, 3b, 8a, 8b, 8c, 8d, 8g) |

- I also ran `python3 -I -B scripts/qa/mp1_plants.py --check --all` without the wrapper: "301 record(s) apply,
  0 stale", exit 0.
- I ran `turts-pack` without the wrapper on 9 sample packages and on 6 copies of the shipped packages.
- The worktree is clean after each run (`git status --porcelain` 0).

## Question 6: the merge

- `git fetch origin`: the head of `origin/mp1/integration` is `54c39fdd`, which equals the merge base, so the
  branch merges fast-forward.
- `origin/mp1/l0-wire` (`67a3f388`, in repair) edits 3 of the same files: `crates/turts_client/src/debug.rs`,
  `widgets/sources.rs` and `tests/forge_research_tooltip.rs`.
- `git merge-tree --write-tree 314e2722 origin/mp1/l0-wire` gives exit 0, so there is no textual conflict today.
- The later of the two branches must re-run `turts_client` after the rebase.

## NOT TRACED

- Whether a mod `engine_overrides` entry can remove every key of `menu` (M2).
- Headed runs (out of scope): SND1, SND2, HUD2, click-without-hover.
- The 19 plant records that I did not run, and the node and headed plant records.
- Whether a key binding to `toggle_sound` does anything. The action is a button action only (`activation.rs:73`).
