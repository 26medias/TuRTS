VERDICT=pass-with-changes FINDINGS=8 HIGH=2 RUNS=11 NOT_CHECKED=6

D = the design. Crate paths are in `~/.cargo/registry/src/*/`. Evidence: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/gate1-bevy-evidence.md`.

## Findings

1. D:88 (K12), High. The limits cap the PNG file bytes, not the decoded memory. A flat 2048 px PNG is 19,771 B, so 848 of them fit in 16 MiB. Decoded, that is 13.2 GiB, and wasm32 has 4 GiB. Bevy keeps a CPU copy too (`bevy_asset-0.20.0/src/render_asset.rs:40-51`). Failure: a valid package crashes every client. Fix: cap the decoded bytes per package (w x h x 4, from IHDR). Use `RENDER_WORLD`, except for cursors (`bevy_winit-0.20.0/src/cursor/mod.rs:144-170`). Cap fonts and sounds per package.
2. D:943 (HUD4), High. This pixels test cannot go red. A CSS cursor gave 0 red px in the screenshot (the control gave 1024). Fix: the oracle reads `turts_debug.cursor` (context and image id).
3. D:206-208, D:521, Medium. On `CursorLeft`, Bevy sets the position to `None` (`bevy_winit-0.20.0/src/state.rs:323-325`). `InputEvent` has no leave event, so edge scroll can stop at the top edge. Fix: add `PointerLeft` and a CAM1 case.
4. D:213-215, D:220, Medium. No test compares the core projection with Bevy. Fix: compare with `Camera::world_to_viewport` (1 px, three viewports). Copy the FOV to `Projection`.
5. D:648-664, Medium. The design gives no draw method and no record at 5,000 units (D57; 13.4 Baseline). `render.rs:383,390` write every transform each frame. Fix: a shared ring mesh, one material per slot, bar fill by child scale, and writes only on change. Add a recorded Chrome WebGPU row at 5,000 units: frame time, draw calls and memory. Placement ghost: out of scope (D:64).
6. D:714-715, Medium. The sink opens when the plugin builds (`bevy_audio-0.20.0/src/audio_output.rs:20-31`). cpal calls `resume()` only in `play()` (`cpal-0.17.3/src/host/webaudio/mod.rs:430`). bevy_audio has no resume call. 1.7.5: no automatic resume. Fix: name the mechanism: a web-entry hook resumes each `AudioContext` on the click.
7. `render.rs:344`, D:530, Medium, needs decision. The light is hardcoded, and `CameraDef` has no light. D37 is Decided. The lighting API is Open (spec 15). Options: (a) a `LightDef` in MC; (b) defer to a named milestone.
8. D:585, Low. Bevy already multiplies the device factor by `UiScale` (`bevy_ui-0.20.0/src/update.rs:57-62`). Fix: `UiScale` = player scale only.

Lens answers:
- Q1: nine primitives map to Bevy 0.20 (run 1). Tooltip needs glue code around `bevy_ui_widgets` popover. `MinimapView` needs custom work.
- Q3: `audio` = bevy_audio + vorbis (lewton). Cost: +412,304 B after wasm-opt, +68,027 B after brotli (+1.3 %).
- Q6: headless Chrome takes the lock, and `movementX` carries the deltas. 1.7.6 is traced; CAM2 can run headless. The engine cursor must sum `MouseMotion` (`winit-0.30.13/src/platform_impl/web/event_loop/runner.rs:283`).
- Q7: no gameplay rule in presentation; no Bevy type in 2.2.

## What I ran

| command | claim | refute | observed |
|---|---|---|---|
| wasm32 check, UI probe | D:553-586 | error | ok |
| same, audio | D:707 | error | ok |
| two wasm-release builds | 1.7.4 | n/a | +68 KB br |
| png_probe.py | K12 | under 4 GiB | 13.2 GiB |
| cursor probe and control | HUD4; 1.7.6 | red > 0; no lock | 0 (1024); lock |
| grep resume | D:715 | resume on gesture | none |
| grep CursorLeft | D:521 | kept | None |
| grep UiScale | D:585 | no multiply | multiplies |
| grep RenderAssetUsages | F1 | render-only | both |
| grep D: batch, light | D:648 | plan present | absent |
| fetch MDN Autoplay | D:714 | n/a | Firefox: sticky |

## Not checked

These are NOT TRACED: browser draw calls at 5,000 units; font atlas memory (`bevy_text-0.20.0/src/text.rs:686`); Ctrl+digit on real keys; minimap cost; `UiTransform` hit tests; Firefox and Safari behaviour.

## What I changed

none

## Open questions for the orchestrator

1. Finding 7 needs a CEO decision.
2. Who sets the decoded-memory cap number for finding 1?
