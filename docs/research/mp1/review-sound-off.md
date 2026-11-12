VERDICT=FAIL HIGH=1

# Review of `mp1/sound-off-default` (head 99247bfa): the game is silent until the player turns the sound on

Seat: `ux-designer`, review mode, 2026-11-12. Worktree `/home/julien/Projects/TuRTS-mp1-q-cold-review` (detached at
99247bfa). Scratch: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/sound-off-review/`
(`R/` below). Not committed.

Summary: the default and the gate work. With the sound off, no player starts; turning it off stops a player that
already plays (my plant proves that the test can fail). The switch is readable, its words carry the state, and it
overlaps no HUD element of the six packages at 1280x720 or 1024x768. One High: the switch is an engine element at a
fixed place on the play screen. That reserves a screen region, against the Decided D67 constraint. The fix is small:
an engine action that a mod places, and the engine switch only as a fallback. The alert repeat of question 5 is
confirmed; the acceptance sentence for `UI-ALERT` is in section 5.

## Findings

| # | Sev | Where | Defect | Fix |
| --- | --- | --- | --- | --- |
| H1 | High | `crates/turts_client/src/client_shell/notices.rs` (`sync_sound_switch`, `SOUND_SWITCH_*` constants: x = centre + 104 px, 8 px from the bottom, 124x28) | D67 line (a) is Decided: "the engine fixes no layout and reserves no screen region for the mod interface" (`RTS-Engine-Project-Specifications.md:96`). The play screen is the HUD, which is mod-owned (D70, Proposed, `:99`). A permanent engine button there is a reserved region that a mod cannot move, restyle or remove. The settings menu is a shell screen, so the copy of the switch in the menu is fine. | Give mods the control, and keep the CEO's fix for every mod: (1) an engine input action `toggle_sound`, like `toggle_guide` (a `ButtonAction::Input`; add it to the engine action list); (2) a value for the state words (`Bind(SoundState)`: "Sound: on" only when the flag is on and the browser lets audio run, the same rule as `sound_shown_on`); (3) a predicate `SoundOn` for style. The engine draws its own switch only when the interface has no widget with the action `toggle_sound`. Cost: three additive API items, one validation entry, two tests (a mod switch at another place hides the engine switch; with no mod switch, the engine switch shows); about 1 run. |
| M1 | Medium | the switch button (engine `UiButton`) | When the pointer jumps onto the switch and presses in the same frame, the click is lost. Run `soundoff-rev-switch-2` (muster, `R/snaps2/muster-1280x720-log.json`): one click, then `sound_on: false` for 5 s while the audio context is `running` and `gesture_seen` is true; the second click turns it on. With a pointer move and a 300 ms rest first (run `soundoff-rev-switch-3`, `R/snaps3/`), one click acts within 500 ms. A real mouse nearly always gives a hover frame, so a person rarely meets this. But a quick move-and-click, a touch screen, or a test driver can, and the result is the CEO's "I clicked and nothing happened". Same cause: the 6-package hold (12 of 12 single clicks did nothing; `R/snaps/*-3-ui.json`). | Let a press on a button act with no hover frame (the press itself sets the hover), or record it as a known engine limit for every button. Q-COLD's buttons probe already rests 300 ms because of this; a probe with no rest would show it. Root cause NOT TRACED (the button interaction system). |
| L1 | Low | `volume_settings.rs:143-146` | The switch uses the engine default font (monospace in the screenshots), not the mod font. It looks like a debug label next to the mod HUD (`R/snaps/muster-1280x720-2-played.png`). | Settled by H1: a mod switch uses the mod font. |
| L2 | Low | my harness | The 6-package hold ended with receipt status `failed: exit 143`. The cause was my held script: its last line was `wait` on a server that it had stopped itself. All 12 page runs returned 0. Not a product fault. | - |

## Run table

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `mp1-play.sh --prepare R/prep-<p> --package mods/<p>` for muster, forge, base, cedar, haul, acme (outside the lock) | the six packages build at this head | a prepare that is not 0 | 6 of 6 are 0 (`R/prep-status.txt`) |
| `mc_headed_v6.sh --mode pre-merge ... --label soundoff-rev-snaps-1 -- bash R/held-snap.sh` (one hold; a fresh prepared server for each page; `R/snap.js`, my own script) | the switch shows "Sound: off" and overlaps nothing, at 1280x720 and 1024x768 | an overlap with a HUD widget, or no switch | 12 of 12 show `sound_switch` "Sound: off", at [744, 684, 124, 28] (1280) and [616, 732, 124, 28] (1024). Overlaps: none, except forge `guide` (the full-page guide that opens at the join; the switch draws on top of it and its words stay readable: `R/snaps/forge-1280x720-2-played.png`). Hold 456 s |
| same, label `soundoff-rev-switch-2` (muster; one click, then poll each 500 ms for 5 s, then a second click) | "one click on" (builder, SND1) | `sound_on` true after one click | false for 5 s; true after the second click (M1) |
| same, label `soundoff-rev-switch-3` (move 10 steps, rest 300 ms, one click) | one click acts when the pointer hovers first | false after one click | true at +500 ms; a second click turns it off |
| `exec_in_lane_v4.py ... --label soundoff-rev-green1 -- cargo test -p turts_client --locked --lib --test volume_settings -- sound` | the sound tests are green | a fail | 9 + 3 passed |
| plant in `audio.rs` `silence_while_sound_is_off`: `if settings.sound_on {` becomes `if true {` (a player that already plays keeps playing after "off"); label `soundoff-rev-plant1`, `--allow-dirty` | the tests catch "off does not stop a playing sound" | all green | exit 101: `turning_the_sound_off_stops_a_sound_that_is_playing` FAILED (`audio.rs:1274`) |
| `cp` restore; `cmp` with the saved copy; `git status --short` | exact restore | `cmp` not 0, or a dirty tree | `cmp` 0; tree clean |
| label `soundoff-rev-green2`, same command | green again | a fail | 9 + 3 passed |
| reading `feedback.rs:746-773`, `:817-840`, `:870-880`, `feedback/alert_state.rs:39-55`; `grep WAIT_ALERTED` | question 5 | a once-per-wait guard on this branch | no guard (0 hits; the MC PT1 repair `feedback.rs:858-869` of MC 9b3a7bd is not on this branch). Each evaluation pushes the waiting unit; only `cooldown.max(1.0)` limits it. The new triggers are stubs that return an empty list |

## 1. The control (question 1)

- **Place.** Bottom, right of centre (`R/snaps/*-2-played.png`). That is not where a person looks first: top-left and
  the centre take the first look. But it is where a status control is expected, it stays on screen all the time, and
  it is the only bright-on-dark-red block at the bottom. Accept the place as the engine default.
- **Readability.** 16 px text (`volume_settings.rs:143-146`). That is 24 px at 1080p: above the 18 px minimum of XAG
  101 for PC (https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/101). The text is
  white on the fill sRGB (0.30, 0.12, 0.12): contrast about 13.8:1 (relative luminance 0.026; computed from
  `volume_settings.rs:124-130`). The "on" fill (0.12, 0.38, 0.22) gives about 7.4:1. The words "Sound: off" and
  "Sound: on" carry the state; the colour only adds to it. The button edge has low contrast against the dark void
  around the map, but the text does not depend on the edge.
- **Overlap.** None with any HUD widget of the six packages, at either size (run table). Not checked: the MC forge
  PT1 layout (bottom objective box x 290-990, y 544-712), which is not on this branch. At 1280x720 the switch
  (x 744-868, y 684-712) would sit inside that box. That is one more reason for H1.
- **First-click rules.** The switch adds 2 words to the visible total. Totals with it: acme 10, base, cedar and haul
  21, muster 71 (the 66-word guide, condition M2), forge 239 (the guide). Under the "Click to play" overlay
  (`ZIndex(i32::MAX - 3)` against the overlay's `i32::MAX`), a first click on the switch goes to the overlay: the same
  lost first click as Q-COLD finding M4. After "Click to play" it acts in one click when the pointer hovers first
  (M1).

## 2. D67 (question 2)

Do not accept the switch as a shell control. The rule: **every control that stays on the play screen is a widget that
the mod places; the engine supplies the action, the value and a fallback that shows only when the mod places none.**
Shell overlays that come and go (the "Click to play" overlay, the sound-locked notice, the rejoin text) stay
engine-owned. The cost is in finding H1. This keeps the CEO's request (silent by default, one switch on the screen)
for every mod today, and gives a mod the full D67 freedom.

## 3. The silent default (question 3)

The switch words are the hint, and they are honest: "Sound: off" tells a first-time person that the game has sound
and that it is off. No one-time line is needed. A line would add words to the first screen, and the game is already
over the text budget in two packages. The 2 words of the switch count against the 60-word total, like any visible
text. Keep the sound-locked notice for its own case (flag on, browser still locked); the branch does this
(`notices.rs`, `sync_sound_off_notice`).

## 4. Tests and plant (question 4)

Green 9 + 3, red 1 on the plant, exact restore, green 9 + 3 (run table). The tests can fail on the fault "off does
not stop a sound that already plays".

## 5. The alert repeat (question 5)

Confirmed. `InserterWaiting` (`feedback.rs:746-773`) and `GathererWaiting` (`:817-840`) push the waiting unit in every
evaluation once the wait passes its threshold. After that, only the rule cooldown limits them:
`now - last < rule.cooldown.max(1.0)` (`:870-874`). A loader that waits plays one alert, and one sound with the sound
on, about each second for as long as it waits. The state triggers of API 0.2.4 are stubs that return an empty list
(`feedback/alert_state.rs:39-55`). The MC PT1 guard "one alert for each wait" is not on this branch (0 hits for
`WAIT_ALERTED`); the plan merges it forward before `UI-ALERT` (plan :82).

The acceptance sentence for `UI-ALERT` (add to row ALR2):

> With the sound switch on, a headless client whose replica holds one own unit that waits with no break for 120 s
> (one case `InserterWaiting`, one case `GathererWaiting`) records exactly one alert line and at most one alert sound
> for that wait in `turts_debug.audio`. A second sound comes only after the unit stops waiting and waits again, and
> only when the last sound of the rule is at least `cooldown` seconds old. In the first 60 s of the stage-1 match with
> no input after the sound is turned on, at most 3 alert sounds start (8.3 point 4). Plant: today's rule (the unit
> pushed on each evaluation, `cooldown.max(1.0)`) gives more than 100 sounds in the 120 s case: red.

## Not checked

- The settings-menu switch and its "Audio levels" line: not opened.
- The AUD1 and HUD2 reds the builder reports; the SND1, ORD6, ORD7 and ALR1 rows: not run.
- The native desktop client.
- The cause of M1.

## What I changed

None in the repository. The plant was applied and restored exactly (`cmp` 0; `git status` clean). Six prepared folders
and all evidence are in `R/`. My servers ended with each page run (the held script stops the serve process that it
started).
