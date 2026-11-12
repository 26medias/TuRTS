//! The hash of the movement step, with and without the cargo feature `test-probes`
//! (pre-MP1 task Q1b, design section 9 item 20).
//!
//! The feature must change no result. Each test prints one line `<name>=<hash>`. The
//! proof is that the output of a run with the feature and the output of a run without
//! it are equal. `scripts/qa/probe_neutral.sh` runs both, compares the hash lines and
//! exits 1 on a difference (CI: the step "Probe neutrality" of the `check` job). The two
//! commands that it runs:
//!
//!     cargo test -p turts_sim --locked --test frame_hash -- --nocapture --test-threads=1
//!     cargo test -p turts_sim --locked --features test-probes --test frame_hash -- --nocapture --test-threads=1
//!
//! The hash is FNV-1a over the bits of each output. There is no pinned value, on purpose:
//! the fixture uses `cos` and `sin` of the platform libm, the movement code uses `atan2`
//! (`movement/mod.rs`, `spread.rs`), and only one machine was run, so a pinned value can
//! differ on another platform. A later task that changes movement on purpose changes the
//! hash. The comparison of the script needs no pin (both runs are on one machine). This file
//! never names the feature.

use std::collections::BTreeMap;

use turts_mod_api::{ContentId, OrderDef, PackageId, SourceDef};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::movement::{
    MoveProgress, MovementActivity, MovementFrame, MovementFrameOutput, MovementFrameUnit,
    MovementPool, step_frame,
};
use turts_sim::{
    Command, CommandKind, CommandQueue, OrderCommand, OrderTargetSim, Position, QueueMode, SlotId,
    UnitId, Vec2, headless_app, install_package, stage_package,
};

const SEEDS: u64 = 16;
const FRAMES: usize = 300;

struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn vec2(&mut self, point: Vec2) {
        self.bytes(&point.x.to_bits().to_le_bytes());
        self.bytes(&point.y.to_bits().to_le_bytes());
    }

    fn text(&mut self, text: &str) {
        self.bytes(text.as_bytes());
        self.bytes(&[0xff]);
    }
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }
}

fn start_frame(seed: u64) -> MovementFrame {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15 ^ seed.wrapping_mul(0x2545_f491_4f6c_dd1d) | 1);
    let bounds = Vec2::new(48.0, 36.0);
    let pool_point = Vec2::new(rng.range(8.0, 40.0), rng.range(8.0, 28.0));
    let slots: Vec<Vec2> = (0..12)
        .map(|index| {
            let angle = index as f32 * std::f32::consts::FRAC_PI_6;
            pool_point + Vec2::new(angle.cos(), angle.sin()) * (1.1 + 0.4 * (index / 6) as f32)
        })
        .collect();
    let pool = MovementPool {
        pool_id: 1,
        point: pool_point,
        spacing: 1.1,
        primary_count: 8,
        slots: slots.clone(),
    };
    let units = (0..36u32)
        .map(|index| {
            let position = Vec2::new(rng.range(2.0, 46.0), rng.range(2.0, 34.0));
            let pooled = index < 8;
            let slot = slots[index as usize % slots.len()];
            MovementFrameUnit {
                unit_id: index + 1,
                owner_slot: 0,
                unit_type: 1,
                position,
                radius: if index % 9 == 0 { 0.8 } else { 0.5 },
                speed: (index % 7 != 6).then_some(3.0 + (index % 3) as f32),
                target: if pooled {
                    Some(slot)
                } else {
                    (index % 4 != 3).then(|| Vec2::new(rng.range(2.0, 46.0), rng.range(2.0, 34.0)))
                },
                progress: pooled.then_some(MoveProgress {
                    pool_id: 1,
                    slot,
                    activity: MovementActivity::Moving,
                    pass_mode: false,
                    stalling_on_group: false,
                    stalled_ticks: 0,
                    best_distance: position.distance(slot),
                    claim_count: 0,
                    follow_origin: None,
                    gather_goal: None,
                    fallback: None,
                }),
                plain_progress: None,
                physical_escape: None,
            }
        })
        .collect();
    MovementFrame {
        bounds,
        tick_hz: 20,
        units,
        pools: BTreeMap::from([(1, pool)]),
    }
}

/// The next frame: the outputs become the inputs. A unit with no target gets a new
/// one now and then, so that the chain keeps moving, as `--synthetic-units` does.
fn next_frame(
    frame: &MovementFrame,
    outputs: &[MovementFrameOutput],
    rng: &mut Rng,
) -> MovementFrame {
    let mut next = frame.clone();
    next.units.sort_unstable_by_key(|unit| unit.unit_id);
    for (unit, output) in next.units.iter_mut().zip(outputs) {
        assert_eq!(unit.unit_id, output.unit_id);
        unit.position = output.position;
        unit.target = output.target;
        unit.progress = output.progress.clone();
        unit.plain_progress = output.plain_progress.clone();
        unit.physical_escape = output.physical_escape.clone();
        if unit.target.is_none() && unit.progress.is_none() && rng.next().is_multiple_of(16) {
            unit.target = Some(Vec2::new(rng.range(2.0, 46.0), rng.range(2.0, 34.0)));
        }
    }
    next
}

fn feed_output(hash: &mut Fnv, output: &MovementFrameOutput) {
    hash.bytes(&output.unit_id.to_le_bytes());
    hash.vec2(output.position);
    match output.target {
        Some(target) => {
            hash.bytes(&[1]);
            hash.vec2(target);
        }
        None => hash.bytes(&[0]),
    }
    // `Debug` of `f32` prints the shortest text that reads back to the same bits.
    hash.text(&format!("{:?}", output.progress));
    hash.text(&format!("{:?}", output.plain_progress));
    hash.text(&format!("{:?}", output.physical_escape));
    hash.bytes(&[u8::from(output.failed_blocked)]);
}

/// 16 seeded chains of 300 frames of the shared kernel `step_frame`.
fn frame_chains_hash() -> (u64, usize, usize) {
    let mut hash = Fnv::new();
    let mut plain_frames = 0;
    let mut pooled_frames = 0;
    for seed in 1..=SEEDS {
        let mut rng = Rng(seed.wrapping_mul(0x1234_5678_9abc_def1) | 1);
        let mut frame = start_frame(seed);
        for _ in 0..FRAMES {
            let outputs = step_frame(&frame, 0.05);
            for output in &outputs {
                feed_output(&mut hash, output);
                plain_frames += usize::from(output.plain_progress.is_some());
                pooled_frames += usize::from(output.progress.is_some());
            }
            frame = next_frame(&frame, &outputs, &mut rng);
        }
    }
    (hash.0, plain_frames, pooled_frames)
}

#[test]
fn frame_chains() {
    let (first, plain, pooled) = frame_chains_hash();
    let (second, _, _) = frame_chains_hash();
    assert_eq!(
        first, second,
        "the same seeds give the same hash in one process"
    );
    // The chains must reach both the pool branch and the plain fallback, else the
    // hash proves nothing about them.
    assert!(
        plain > 0,
        "no unit reached a plain progress: the chains miss that branch"
    );
    assert!(
        pooled > 0,
        "no unit held a pool: the chains miss that branch"
    );
    println!("\nframe_chains={first:016x} plain_unit_frames={plain} pooled_unit_frames={pooled}");
}

/// The authored `muster` world through the ECS movement system: a group order and a
/// crowd of single moves. This is the path where the feature stores its probe flags.
#[test]
fn muster_world() {
    let package_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
    let archive = PackageArchive::parse(&pack_dir(&package_dir).expect("Muster packs"))
        .expect("Muster archive parses");
    let catalog = archive.content().expect("Muster catalog resolves");
    let (map_file, map) = archive
        .maps()
        .expect("Muster maps parse")
        .into_iter()
        .find(|(_, map)| map.id == "field")
        .expect("Muster field map exists");
    let mut app = headless_app(20);
    let staged = stage_package(
        app.world(),
        catalog.clone(),
        SourceDef {
            file: map_file,
            value: map,
        },
        20,
    )
    .expect("the authored Muster map stages");
    install_package(app.world_mut(), staged);
    let (order, definition): (ContentId, &OrderDef) = catalog
        .orders
        .iter()
        .find(|(id, _)| id.package.0 == "muster" && id.local == "order_move")
        .map(|(id, definition)| (id.clone(), definition))
        .expect("Muster order_move exists");
    let group = Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order,
            behaviour: definition.behaviour,
            target_kind: definition.target,
            policy: None,
            units: (1..=20).collect(),
            target: OrderTargetSim::Point(Vec2::new(40.0, 30.0)),
            queue: QueueMode::Replace,
            group: 5,
            allowed_units: definition.units.clone(),
            queue_depth: catalog.order_queue_depths[&PackageId("muster".into())],
        })),
    };
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(group);
    for id in 21..=40u32 {
        app.world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .push_back(Command {
                slot: SlotId(0),
                seq: 100 + id,
                kind: CommandKind::MoveTo {
                    unit_ids: vec![id],
                    target: Vec2::new(60.0, 10.0 + id as f32 * 0.5),
                },
            });
    }
    let mut hash = Fnv::new();
    for _ in 0..120 {
        app.update();
        let mut rows: Vec<(u32, Vec2)> = app
            .world_mut()
            .query::<(&UnitId, &Position)>()
            .iter(app.world())
            .map(|(id, position)| (id.0, position.0))
            .collect();
        rows.sort_by_key(|row| row.0);
        for (id, position) in rows {
            hash.bytes(&id.to_le_bytes());
            hash.vec2(position);
        }
    }
    println!("\nmuster_world={:016x}", hash.0);
}
