VERDICT=pass-with-changes FINDINGS=6 HIGH=0 RUNS=7 NOT_CHECKED=3

D = the revision 2 design. Crate paths are under `~/.cargo/registry/src/*/`. Evidence: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/gate1-bevy-evidence.md`.

## Original findings

- B1: fixed with new defects (N2, N3). D:96, D:667-669, D:971. `RENDER_WORLD` is sound: Bevy takes `data` and keeps the size (`bevy_render-0.20.0/src/texture/gpu_image.rs:39-45`).
- B2: fixed with a new defect (N1). D:1064-1069, D:1145.
- B3: fixed. D:240-244, D:634, D:1139. A synthetic move to y = -10 fires `pointerout`, so CAM1 can go red headless.
- B4: fixed with a new defect (N4). D:255, D:1141.
- B5: fixed. D:788-799, D:1148.
- B6: fixed with a new defect (N5). D:854-859.
- B7: fixed as call K23 (Open). D:107, D:644. The fields exist (`bevy_light-0.20.0/src/ambient_light.rs:21`, `directional_light.rs:86`).
- B8: fixed. D:703-705.
- K27 (D:910-920): it adds no new screen. A reload resets the user activation, and the focus overlay covers that. No defect.

## New findings

- N1 Medium. Cursor images can be up to 2048 px (D:96, D:971). Browsers ignore a CSS cursor above 128 px (MDN). winit uses a CSS cursor (`winit-0.30.13/src/platform_impl/web/cursor.rs:336`). So HUD4 passes while the system cursor shows. Fix: refuse a cursor above 128 px. The bridge reports the applied `CursorIcon`.
- N2 Medium. The 256 MiB cap is per package (D:96). N locked packages can use N x 256 MiB. Fix: cap the locked set too.
- N3 Low. Sounds have no byte cap (D:96). Bevy keeps the file bytes (`bevy_audio-0.20.0/src/audio_source.rs:8-15`). Fix: add a byte cap.
- N4 Low. PROJ1 says "headless (windowed test app)" (D:1141), but hosted CI has no display. Fix: use an `Image` target (NOT TRACED).
- N5 Low. D:858-859 names no gate for "no sound is queued". Fix: spawn no player while the context is `suspended`.
- N6 Low. A second load of a path returns the existing handle (`bevy_asset-0.20.0/src/server/mod.rs:324`). A cursor that is also an icon can lose its CPU copy. Fix: load cursors on their own.

## What I ran

| command | claim | refute | observed |
|---|---|---|---|
| Playwright leave probe | D:1139 | no event | `pointerout` |
| fetch MDN `cursor` | D:96 | no limit | 128 px |
| grep winit `cursor.rs` | N1 | no CSS | CSS |
| read `gpu_image.rs`, `render_asset.rs:346-368` | D:667 | asset removed | data only |
| grep `bevy_light` | D:644 | absent | present |
| grep asset server | N6 | per settings | per path |
| read D, B1-B8 | D:1292-1299 | fix absent | present |

## Not checked

These are NOT TRACED: cpal while the context is suspended; PROJ1 on an `Image` target; GPU texture memory.

## What I changed

none
