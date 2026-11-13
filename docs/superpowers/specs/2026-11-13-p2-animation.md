# Stage P2: animation

Date: 2026-11-13. Status: ready to build. Roadmap: `2026-11-13-presentation-roadmap.md`, P2.
Track: the package contract (section 3) is Foundation; the client work is the light path.
Process: this spec, the build, one independent review that runs code. No plan document.
Base: branch `codex/models-stage1` (stage P1, API 0.2.5).

## 1. What the player sees

1. A Villager and a Crossbowman stand with a small idle motion.
2. They walk with their legs when they move. The feet do not slide in a way that a player
   notices at the default camera.
3. A Crossbowman raises the crossbow and shoots at the moment that the shot happens. Between
   shots it keeps the weapon up while it has a target.
4. A Villager shows a work motion when it gathers at the trees and when it builds.
5. A unit that dies falls down, stays on the ground a short time, then sinks into the ground
   and is gone. It does not vanish at the moment of death.
6. A building under construction rises from the ground with its progress.
7. Two hundred animated units in one view keep the game smooth.

## 2. Facts, traced (branch `codex/models-stage1`)

- `ModelDef.animations` exists with the fields `idle`, `move`, `attack`, `death`, `work`
  (`crates/turts_mod_api/src/schema.rs:786`, `:810-816`). The stage 1 client ignores it and
  draws the bind pose.
- The pack-time check accepts a model with a skin and animations (acceptance row MD4 of P1).
- The client knows the state of each unit from the wire: `Activity` = `Idle`, `Pending`,
  `Moving`, `Attacking`, `Holding`, `Gathering`, `Blocked`
  (`crates/turts_net_protocol/src/messages.rs:116-124`).
- The client gets the events `Effect::Fired { unit, target, weapon }` and
  `Effect::Destroyed { unit }` (`crates/turts_net_protocol/src/records.rs:115-124`). Effects are
  not replayed after a resync.
- Animated files for the two people are ready, CC0:
  `target/mc-evidence/art/skirmish-fantasy/soldier_anim.glb` and `worker_anim.glb`. Their
  clips: `Idle`, `Walking_A`, `Running_A`, `Death_A`, `Interact`, `PickUp`,
  `2H_Ranged_Aiming`, `2H_Ranged_Shoot`, `2H_Ranged_Reload`, `1H_Melee_Attack_Chop`, `Hit_A`.
  In the `2H_Ranged_*` clips the body turns about 42 degrees to the left of the aim line.
  The root scale is not applied.
- P1 measured 2,000 static detailed units at the display limit (about 58 frames each second)
  on the RTX 3080 (`docs/measurements/models.md`). Animated cost is not measured.

## 3. Package contract

Additions are optional, with defaults and version-table rows. Old packages load unchanged.

```
animations: Some((
    idle:   Some((clip: "Idle")),
    move:   Some((clip: "Walking_A", stride: 1.6)),
    attack: Some((clip: "2H_Ranged_Shoot", fire_at: 0.25, hold: Some("2H_Ranged_Aiming"), yaw_offset_deg: 42.3)),
    death:  Some((clip: "Death_A")),
    work:   Some((clip: "Interact")),
    hit:    None,
))
```

- Each entry names a clip of the model file. The pack-time check refuses a clip name that the
  file does not have, and lists the names that exist, with a `did you mean` hint.
- `stride`: the distance in map units that the unit covers in one loop of the `move` clip at
  the model's scale. The engine sets the playback speed from the real speed of the unit, so the
  feet match the ground. Absent: the clip plays at its authored speed.
- `fire_at`: the fraction of the `attack` clip at which the shot leaves. The engine aligns that
  moment with the `Fired` event (section 4.2).
- `hold`: a looping clip for a unit that has a target between shots. Absent: `idle`.
- `yaw_offset_deg`: a yaw correction applied while `attack` or `hold` plays, for clips whose
  body turns away from the aim line.
- `hit`: optional short reaction clip. Not used by the skirmish in this stage.
- A missing entry falls back: `move`, `attack`, `work`, `death` to `idle`; `idle` to the bind
  pose.
- The form of today (`idle: Some("Idle")`, a plain string) must still parse, or the version
  rules must cover the change. Choose the way that the version table supports; state it.
- A LOD level can be a skinned file or a static file. A static LOD level shows no animation;
  that is the cheap far level.
- Limits (pack time): 128 joints for one skin (P1), 64 clips for one file, 60 s for one clip.
  A package does not run code for each unit; the engine selects the clip.

## 4. Engine rules

### 4.1 State to clip

| Unit state (client) | Clip |
| --- | --- |
| `Moving` (the position changes) | `move` |
| `Attacking`, at a `Fired` event | `attack`, one time |
| `Attacking`, between shots | `hold` |
| `Gathering` at the node, and a constructor at its site | `work` |
| `Destroyed` event | `death`, one time, then the body rule of 4.3 |
| all other states | `idle` |

- A change of clip blends over 0.15 s. No snap.
- Units of one type do not move in step: each unit gets a start phase from its unit id.
- Movement decides `move` from the presented motion, not from the order, so a unit that waits
  in a queue does not walk in place.

### 4.2 Shot timing

- The server decides the shot. The client learns it from `Fired`. The client must not delay
  the visible effect of the shot to wait for the clip.
- Rule: at `Fired`, the `attack` clip starts at the time `fire_at` minus a lead of at most
  0.1 s, so the release moment shows within 0.1 s of the event. The stage P3 muzzle effect and
  projectile start at the event time.
- Facing: the unit faces its target while it attacks (P1 rule), plus `yaw_offset_deg`.

### 4.3 Death and bodies

- At `Destroyed`, the server removes the unit. The client keeps a visual body that is not a
  unit: it cannot be selected, has no health bar, no ring, no collision, and no pick volume.
- The body plays `death`, stays 3 s, sinks 1.0 map unit over 1.5 s, and is removed.
- At most 64 bodies exist. A new body beyond that removes the oldest.
- A unit that leaves the view for another reason (not `Destroyed`) is removed with no body.
- After a resync no body appears for a unit that died during the gap (effects are not
  replayed).
- A building has no `death` clip in this stage: it uses the stage P3 destruction effect. Until
  P3 exists, it is removed as today.

### 4.4 Construction rise

- A site shows the final model raised from below ground level in proportion to its progress,
  with the part below ground hidden. At 100 % it is at its final place.
- If a site visual exists in the package, the package visual wins.

### 4.5 Cost and quality

- The client `quality` setting controls animation: `high` animates all visible units each
  frame; `medium` and `low` update far units (small on screen) at a lower rate, and can draw
  the smallest LOD level static.
- Units outside the view do not animate.
- If the measurement of section 6 shows that 400 animated units at `high` are below 30 frames
  each second on the play-test machine, the stage is not complete: select and build a cheaper
  crowd method (a lower update rate by screen size, shared poses for units in the same clip
  phase, or baked vertex animation), and state the choice in the report.

## 5. Skirmish content

- The Crossbowman and the Villager use `soldier_anim.glb` and `worker_anim.glb` with the
  mapping of section 3 (Villager: `idle` = `Idle`, `move` = `Walking_A`, `work` = `Interact`,
  `death` = `Death_A`; no `attack`).
- Measure the `stride` of `Walking_A` from the file; do not guess it.
- Keep the scale and footprint fit of stage 1 (Crossbowman scale 0.88).
- Update `mods/skirmish/assets/CREDITS.md` if a file changes.

## 6. Measurement

Add rows to `docs/measurements/models.md`, same machine data and method as P1, quality `high`:

| Scene | Record |
| --- | --- |
| 200 animated Crossbowmen in view, all walking | frame time p50 and p95, wasm high-water |
| 400 animated, all walking | the same |
| 400 animated, mixed states, with 20 deaths each second for 10 s | the same |
| 1,000 animated, all walking | the same |

The display limits the result to about 60 frames each second. If a method to measure below
that limit exists in the harness (frame work time, not presentation interval), record it too.

## 7. Acceptance

Each row must be able to fail on the build before the change.

| Id | Check | Level |
| --- | --- | --- |
| AN1 | A unit that moves plays `move`; when it stops, `idle` within 0.3 s. | client test |
| AN2 | Foot match: over 5 s of straight movement, the `move` clip advances `distance / stride` loops within 5 %. | client test |
| AN3 | At a `Fired` event the `attack` clip is at `fire_at` within 0.1 s. | client test |
| AN4 | A Villager at the trees plays `work`; one that walks back plays `move`. | client test or browser |
| AN5 | At `Destroyed` a body plays `death`, cannot be picked, and is gone after 3 s + 1.5 s (within 0.3 s). The 65th body removes the first. | client test |
| AN6 | After a resync, no body appears for a unit that died in the gap. | client test |
| AN7 | Browser, pixels: a walking Crossbowman changes its silhouette between frames (legs move); an idle bind-pose build fails this. | browser |
| AN8 | Pack: a clip name that the file does not have is refused with the list of names; a P1 package with no `animations` packs unchanged. | pack test |
| AN9 | A site at 50 % shows about half of the model height above ground; at 100 % all of it. | client test or browser |
| AN10 | The scripted player probes of `docs/qa/cold-play.md` pass. | browser |
| AN11 | The measurement table of section 6 is recorded. | document |

Every test browser runs with `--alsa-output-device=null` and a fresh profile.

## 8. Review

One reviewer, after the build. The reviewer runs AN1 to AN10 and plants three faults:
(a) the clip never changes from `idle`, (b) the body is removed at the `Destroyed` event,
(c) the shot clip starts at 0 at the event (the release shows late). Each must turn a row red.

## 9. Out of scope

Turret and muzzle node aim for vehicles (with the first vehicle), animation events authored in
the file, inverse kinematics, ragdolls, facial animation, building idle animation, the hit
reaction in the skirmish.

## Implementation notes (2026-11-13)

- The wire had `Fired` and `Destroyed`, but no simulation writer filled them.
  The completed tick now derives them from committed shot and destruction transactions.
  No damage rule or wire field changes. Recovery never replays these visual events.
- Clip entries accept both legacy strings and metadata records. The metadata fields require
  API 0.2.6. Existing string mappings and packages remain valid.
- Animated models use their authored idle pose for footprint and body-picking bounds.
  The bind pose is too wide to describe a standing body. Static-model bounds stay unchanged.
- The supplied Crossbowman prop penetrated the ground in Idle. Its local crossbow scale is
  halved. The body stays at scale 0.88 and the collision footprint stays unchanged.
