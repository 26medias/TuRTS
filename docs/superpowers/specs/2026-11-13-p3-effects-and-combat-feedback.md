# Stage P3: effects and combat feedback

Date: 2026-11-13. Status: ready to build. Roadmap: `2026-11-13-presentation-roadmap.md`, P3.
Track: the package contract (section 3) is Foundation; the client work is the light path.
Process: this spec, the build, one independent review that runs code. No plan document.
Base: branch `codex/models-stage1`. It can run at the same time as P2; the two meet only at
the death of a unit and at the shot moment (sections 4.4 and 4.5).

## 1. What the player sees

1. A Crossbowman that shoots sends a visible bolt to its target. The bolt takes a short time
   to fly. A spark shows where it hits.
2. A unit that takes damage flashes for a moment.
3. A unit that dies leaves a short burst of dust. A building that is destroyed breaks with a
   larger burst, debris and smoke, and leaves a scorch mark on the ground for some time.
4. A damaged building smokes below half health and burns below a quarter.
5. A Villager that chops wood sends out wood chips. A building site gives dust.
6. When the player's units or buildings are attacked outside the view, the player gets a
   warning: a text "Your base is under attack" or "Your units are under attack", a pulse on the
   screen edge in the direction of the attack, and a sound if sound is on. A click on the
   warning, or the Space key, moves the camera there.
7. With effects set to `low`, the same match plays and all information stays readable: bolts,
   hits and warnings still show, only smaller and fewer.

## 2. Facts, traced (branch `codex/models-stage1`)

- The mod API has a visual effects file: `VisualEffectsDef` with `VisualEffectDef { id, kind,
  colour, size, ms, sound }` and `EffectKind` = `Flash`, `Spark`, `Tracer`, `Burst`, `Smoke`.
  A unit visual can name effects: `VisualEffectRefs { fire, hit, hit_reduced, death, damaged }`
  (`crates/turts_mod_api/src/schema.rs`, API rows V3 and V4).
- No client code draws an effect. The skirmish package declares none, has no sounds
  (`mods/skirmish/assets/assets.ron`: `sounds: []`) and no alerts
  (`mods/skirmish/interface/hud.ron`: `alerts: []`).
- The client gets `Effect::Fired { unit, target, weapon }`, `Effect::Destroyed { unit }` and
  `Effect::Credited` (`crates/turts_net_protocol/src/records.rs:115-130`). Effects are not
  replayed after a resync. Health changes arrive in the unit records.
- The audio engine and its alert rules exist (`crates/turts_client/src/audio.rs`). Sound starts
  off (CEO rule).
- NOT TRACED: which alert fields the HUD schema has today, and if the server sends an
  "attacked" fact for units outside the view of the owner. Codex traces both first and states
  the result. A player always knows about damage to the units that the player owns.

## 3. Package contract

This is roadmap level A (engine effect types with parameters) and the start of level B
(composition in data). No mod code. All additions are optional with defaults and version-table
rows; the five kinds of today keep their meaning.

### 3.1 Effect definition

An effect is a list of layers. Each layer is one emitter type with parameters:

```
(id: "bolt_hit", layers: [
    (kind: Sparks(count: 8, speed: 3.0, spread_deg: 60, gravity: 9.0), colour: "#ffd27a", size: 0.08, ms: 250),
    (kind: Flash(radius: 0.5), colour: "#fff2c0", ms: 80),
], sound: Some("skirmish:hit_wood"), cost: Small)
```

Layer kinds for this stage:

| Kind | What it draws | Main parameters |
| --- | --- | --- |
| `Flash` | a short bright sprite, optional light | radius, light (on or off) |
| `Sparks` | fast small particles with gravity | count, speed, spread, gravity |
| `Burst` | an expanding soft puff | radius start and end |
| `Smoke` | slow rising particles; can loop while a condition holds | rate, rise, spread |
| `Fire` | flame particles; can loop | rate, height |
| `Debris` | a few tumbling small meshes or sprites that fall and fade | count, speed, model or image |
| `Tracer` | a thin line that fades | width |
| `Projectile` | a model or sprite that flies from source to target | model or image, speed, arc height, trail |
| `Trail` | a ribbon behind a moving thing | width, ms |
| `Decal` | a picture on the ground that fades | image, size, ms |
| `Shake` | a camera shake, limited by a client setting | strength, ms |

Common layer parameters: `colour` (and optional end colour), `size` (and optional end size),
`ms`, `delay_ms`, `image` (a package image; absent: an engine soft dot), `blend` (`Alpha` or
`Add`), `offset` (from the anchor).

Rules:

- `cost` (`Small`, `Medium`, `Large`) is declared by the author and checked at pack time
  against the particle count and duration. The client budget of 4.6 uses it.
- Pack-time limits: 8 layers for one effect, 64 particles for one layer, 10 s for a layer that
  does not loop, 512 px for an effect image. Refusals follow the `ModError` style.
- A `Decal` is a picture only. It never changes terrain or pathing.
- An effect that names a sound follows the audio rules of today. No sound plays while sound is
  off.

### 3.2 Where effects attach

`VisualEffectRefs` on a unit visual, extended:

| Field | When | Anchor |
| --- | --- | --- |
| `fire` | at `Fired`, on the shooter | the `muzzle_node` of the model if named, else a point at 0.7 of the model height, in front |
| `projectile` (new) | at `Fired`, from shooter to target | flies from the fire anchor to the target body centre |
| `hit` | when the projectile arrives; with no projectile, at `Fired` | the target body centre |
| `hit_reduced` | as `hit`, when the damage was reduced | the same |
| `death` | at `Destroyed` | the unit position |
| `damaged` (a list now; the single form still parses) | while health is below a ratio | the model top, or named nodes |
| `work` (new) | each loop of the work motion, or each second if no animation | a point in front of the unit |
| `site` (new) | while a site is under construction | the footprint |

A missing field means no effect. The engine supplies no default effect; the default mod
declares its own.

### 3.3 Alerts

The package declares alerts in the HUD file: for each alert, the trigger, the text, the sound,
and the least time between two alerts of one kind. Triggers for this stage:
`UnitAttacked` (a unit of the player takes damage) and `BuildingAttacked`. If the HUD schema
has alert fields already, use them; add only what is missing.

## 4. Engine rules

### 4.1 Presentation only

Effects never change a match. The server does not read them. A projectile is a picture: the
damage is applied by the server at its own time, and the client shows it.

### 4.2 Projectile timing

- The server applies the damage at the tick of `Fired` (as today). The health bar of the target
  must not drop before the bolt arrives on screen.
- Rule: the client holds the visible health change of that hit, the `hit` effect and the
  damage flash until the projectile arrives, for at most 300 ms. The flight time is the
  distance divided by the projectile speed, limited to 300 ms. Then the bar shows the server
  value.
- A unit that is destroyed by the hit: its `death` starts when the projectile arrives, within
  the same 300 ms limit.
- State this limit in the report with a measured example. If a held health value can make a
  player decide wrongly (for example a unit shown alive that is dead), reduce the limit.

### 4.3 Damage flash

A unit that takes damage shows a white tint for 100 ms. This is an engine behaviour for all
mods, with a client setting to switch it off. Team colour returns after the flash.

### 4.4 Deaths (meeting point with P2)

- P2 owns the body and the `death` animation. P3 owns the `death` effect. Both start at the
  same moment. If P2 is not merged yet, the unit is removed as today and only the effect shows.
- A building that is destroyed: the model is removed at the moment of the effect peak
  (`delay_ms` of the first `Burst` layer, default 0), then the `Decal` stays.

Terminal-state correction, traced during acceptance: the server sends the final tick as a
reliable full snapshot. That message has no `Fired` or `Destroyed` facts. On the first snapshot
with a Victory or Draw outcome, the client may show death pictures for previously known actors
that the snapshot newly removes. Actors already waiting for a known projectile death are excluded. This requires the same session and a coherent prior tick at most
4 ticks earlier. The pictures start at receipt; no shot is inferred. Ordinary recovery,
late joins, stale gaps, and repeated terminal snapshots do not start old death pictures.
A Defeat result alone is not a terminal match: other teams can still be playing.

### 4.5 Shot moment (meeting point with P2)

The `fire` effect and the projectile start at the `Fired` event time. P2 aligns its clip to
that time. P3 does not wait for an animation.

### 4.6 Budget and quality

- One pool for all particles. Client `quality` sets the pool size: `low` 512, `medium` 2,048,
  `high` 8,192 particles. The pool never grows.
- When the pool is full: effects of cost `Small` near the camera centre win; `Large` effects
  far from the view centre are dropped first. An effect that is dropped whole is better than
  one that shows a part.
- `fire`, `projectile` and `hit` of units in view are never dropped whole at any quality: at
  the least the engine draws the projectile as a plain line and the hit as one sprite. These
  carry game information.
- Effects outside the view are not simulated, except looping `Smoke` and `Fire`, which start
  in their steady state when they enter the view.
- Decals: at most 128; the oldest fades first.
- `Shake`: limited by a client setting with the values off, half, full. Default: half.
- No effect older than 4 ticks before the presentation tick is started (after a resync there
  is no burst of old effects).
- WebGPU is the reference. Use the particle path that the renderer supports there with
  instancing; do not create one entity with its own material for each particle.

### 4.7 Attack warning

- Trigger: a unit or building that the player owns takes damage, and it is outside the camera
  view (its body centre is not in the viewport).
- One warning for each kind (`units`, `base`) at most every 10 s. Damage in view gives no
  warning; the player sees it.
- The warning shows the text of the package, a pulse on the screen edge in the direction of
  the place, and a mark on the minimap if a minimap exists.
- Space, or a click on the warning text, moves the camera to the place of the last warning.
  Check the input map of the package first; if Space is taken, use the key that the package
  binds to `jump_to_alert`, and add that binding to the skirmish.
- The sound follows the alert rules of the audio engine. Silent while sound is off.

## 5. Skirmish content

Effects in `mods/skirmish`, theme medieval fantasy, low-poly, no gore:

| Effect | Use |
| --- | --- |
| bolt | `Projectile`: a thin dark bolt with a short pale `Trail`; speed 30 map units each second |
| bolt release | `fire`: a small pale `Flash`, no light |
| bolt hit | `hit`: 6 to 8 `Sparks` in wood colour and a small `Flash` |
| unit death | `death`: a dust `Burst` in ground colour |
| building death | `death`: a large `Burst`, 12 `Debris` pieces, `Smoke` for 3 s, a `Decal` scorch mark for 60 s, `Shake` |
| building damaged | `damaged`: `Smoke` below 1/2 health; `Fire` and `Smoke` below 1/4 |
| chop | `work` on the Villager at the trees: 4 wood-chip `Sparks` |
| site dust | `site`: slow dust `Smoke` at the footprint edge |

Alerts: "Your castle is under attack" for buildings, "Your units are under attack" for units.

Sounds: optional in this stage. If used, only CC0 files, recorded in
`mods/skirmish/assets/CREDITS.md`. The orchestrator can supply them; ask for them in the report
if wanted. Images for effects: the engine soft dot serves; no new image is required.

## 6. Measurement

Add to `docs/measurements/models.md`, same method as P1, quality `high` and `low`:

| Scene | Record |
| --- | --- |
| 100 Crossbowmen against 100, all shooting | frame time p50 and p95, live particle count, wasm high-water |
| The same with 10 buildings that burn | the same |
| 400 against 400 | the same, and the number of dropped effects by cost class |

## 7. Acceptance

Each row must be able to fail on the build before the change.

| Id | Check | Level |
| --- | --- | --- |
| FX1 | At `Fired`, a projectile appears at the shooter and reaches the target; flight time = distance / speed within 20 %, never above 300 ms. | client test |
| FX2 | The visible health of the target does not change before the projectile arrives, and shows the server value at most 300 ms after `Fired`. | client test |
| FX3 | Browser, pixels: during a shot, pixels of the bolt colour appear between shooter and target; at the hit, spark pixels appear at the target. | browser |
| FX4 | A unit that takes damage shows the white tint in at least one frame and the team colour again within 300 ms. | browser or client test |
| FX5 | A building below 1/2 health emits smoke; below 1/4 fire; above 1/2 nothing. | client test |
| FX6 | A destroyed building leaves a decal that is gone after its `ms`; the 129th decal removes the first. | client test |
| FX7 | Budget: with the pool full at `low`, `fire`, `projectile` and `hit` of units in view still draw in their plain form; `Large` effects are dropped first. The pool size never exceeds its limit. | client test |
| FX8 | After a resync, no effect starts for an event older than 4 ticks. | client test |
| FX9 | Warning: damage to an owned building outside the view shows the text and the edge pulse; damage in view does not; a second warning of the same kind inside 10 s does not show. The jump key moves the camera so that the place is in view. | browser |
| FX10 | Pack: an effect above a limit of 3.1 is refused with its text; a package with the five kinds of today packs unchanged. | pack test |
| FX11 | Effects change no result: the same scripted match with effects on and with quality `low` gives the same server state hash at the end. | server and client test |
| FX12 | The scripted player probes of `docs/qa/cold-play.md` pass, and one probe wins a fight with effects on. | browser |
| FX13 | The measurement table of section 6 is recorded. | document |

Every test browser runs with `--alsa-output-device=null` and a fresh profile.

## 8. Review

One reviewer, after the build. The reviewer runs FX1 to FX12 and plants three faults: (a) the
health bar changes at `Fired` and not at arrival, (b) the pool grows past its limit, (c) the
warning shows for damage in view. Each must turn a row red.

## 9. Out of scope

Custom shader code from mods (roadmap P5), layered materials, weather, water, post-processing
(P4), area damage visuals, shields, beam weapons, voice lines, music.
