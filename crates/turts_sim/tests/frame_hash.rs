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
//! The first version drove the removed movement kernel `step_frame` and was deleted with it
//! (4ad52808), which left the CI step with no test. This version drives the flock through
//! the ECS movement system, the only movement path, on the authored `muster` world.
//!
//! The hash is FNV-1a over the bits of each position. There is no pinned value, on purpose:
//! the movement code uses the platform libm, and a later task that changes movement on
//! purpose changes the hash. The comparison of the script needs no pin (both runs are on
//! one machine). This file never names the feature.

use turts_mod_api::{ContentId, OrderDef, PackageId, SourceDef};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{
    Command, CommandKind, CommandQueue, MoveTarget, OrderCommand, OrderTargetSim, Position,
    QueueMode, SlotId, UnitId, Vec2, headless_app, install_package, stage_package,
};

const SEEDS: u64 = 8;
const TICKS: usize = 200;
/// Units 1 to MOVERS of the `muster` map get the seeded single moves.
const MOVERS: u32 = 40;

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
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * ((self.next() >> 40) as f32 / (1u64 << 24) as f32)
    }
}

/// The authored `muster` world (64 m x 48 m) and its catalog.
fn muster() -> (bevy_app::App, turts_mod_api::ContentCatalog) {
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
    (app, catalog)
}

/// Every unit pose of the tick, by unit id, into the hash. Returns the count of units with a
/// move target.
fn feed_tick(app: &mut bevy_app::App, hash: &mut Fnv) -> usize {
    let mut rows: Vec<(u32, Vec2, bool)> = app
        .world_mut()
        .query::<(&UnitId, &Position, Option<&MoveTarget>)>()
        .iter(app.world())
        .map(|(id, position, target)| (id.0, position.0, target.is_some()))
        .collect();
    rows.sort_by_key(|row| row.0);
    for (id, position, moving) in &rows {
        hash.bytes(&id.to_le_bytes());
        hash.vec2(*position);
        hash.bytes(&[u8::from(*moving)]);
    }
    rows.iter().filter(|row| row.2).count()
}

/// Seeded chains: in each, units 1 to `MOVERS` get single moves to seeded points every
/// 40 ticks, so the flock steps, pushes, waits and settles.
fn frame_chains_hash() -> (u64, usize, usize) {
    let mut hash = Fnv::new();
    let (mut moving_ticks, mut arrived_ticks) = (0, 0);
    for seed in 1..=SEEDS {
        let (mut app, _) = muster();
        let mut rng = Rng(seed.wrapping_mul(0x1234_5678_9abc_def1) | 1);
        let mut seq = 0;
        for tick in 0..TICKS {
            if tick % 40 == 0 {
                for id in 1..=MOVERS {
                    if !rng.next().is_multiple_of(3) {
                        seq += 1;
                        let target = Vec2::new(rng.range(4.0, 60.0), rng.range(4.0, 44.0));
                        app.world_mut()
                            .resource_mut::<CommandQueue>()
                            .0
                            .push_back(Command {
                                slot: SlotId(0),
                                seq,
                                kind: CommandKind::MoveTo {
                                    unit_ids: vec![id],
                                    target,
                                },
                            });
                    }
                }
            }
            app.update();
            let moving = feed_tick(&mut app, &mut hash);
            moving_ticks += moving;
            arrived_ticks += usize::from(tick % 40 == 39 && moving < MOVERS as usize);
        }
    }
    (hash.0, moving_ticks, arrived_ticks)
}

#[test]
fn frame_chains() {
    let (first, moving, arrived) = frame_chains_hash();
    let (second, _, _) = frame_chains_hash();
    assert_eq!(
        first, second,
        "the same seeds give the same hash in one process"
    );
    // The chains must both move units and let units arrive, else the hash proves nothing
    // about the movement.
    assert!(
        moving > 0,
        "no unit had a move target: the chains move nothing"
    );
    assert!(
        arrived > 0,
        "no unit arrived: the chains never settle a unit"
    );
    println!("\nframe_chains={first:016x} moving_unit_ticks={moving} arrival_windows={arrived}");
}

/// The authored `muster` world through the ECS movement system: a group order and a
/// crowd of single moves.
#[test]
fn muster_world() {
    let (mut app, catalog) = muster();
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
        feed_tick(&mut app, &mut hash);
    }
    println!("\nmuster_world={:016x}", hash.0);
}
