//! FM2, Jade's return crossing, on the server path (Marco's FM probe, 2026-11-26).
//!
//! quarry_ridge boots as the host boots it (package archive, private seed file). The 36 own
//! units go east to (50, 24); one walker digs the ridge (x 30-33, y 20-27, level 0); 120 ticks
//! after all are idle, the 36 go back to (22, 24) through the dug passage. All must be idle
//! within 60 s (1200 ticks) of the return order.
//!
//! The browser sends the ground point under the pointer, so the order points differ a little from
//! run to run, and the crowd reacts to that. The cases below are the seed of the failing browser
//! run and the order-point offsets for which a6d54cee left 11 to 32 units "Moving" and still at
//! the east mouth: E1 held both units of a pair in which a fast small unit closed in on a slow
//! large one ahead, and the revert spread through the crowd on every tick.
use super::{boot, config, orders};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{OrderEnvelope, OrderTarget, PlayerCommand, Pos, QueueMode, UnitId};
use turts_sim::{CommandQueue, SlotId, UnitOrders, headless_app, initialize_completed_world};

/// The private seed of the failing browser run (quarry_ridge, 2026-11-26 17:35).
const SEED: u64 = 681_927_706_610_217_123;
/// The offset keys for which a6d54cee did not settle (10 of the first 40).
const FAILING_KEYS: [u64; 10] = [1, 2, 3, 5, 12, 13, 17, 19, 26, 38];

/// Offsets of up to 0.05 m on the two order points, from one key (xorshift).
fn offsets(key: u64) -> [f32; 4] {
    let mut state = key.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
    std::array::from_fn(|_| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        ((state >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.1
    })
}

/// The ticks from the return order to the tick at which all 36 are idle, or the units that are
/// not idle after 1200 ticks.
fn return_crossing(key: u64) -> Result<u32, Vec<(u32, turts_sim::Vec2)>> {
    let dir = std::env::temp_dir().join(format!("turts-fm2-{}-{key}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let package = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/quarry_ridge");
    let bytes = pack_dir(&package).unwrap();
    let archive_path = dir.join("quarry_ridge.tar");
    std::fs::write(&archive_path, &bytes).unwrap();
    std::fs::write(dir.join("seed.txt"), SEED.to_string()).unwrap();
    let catalog = PackageArchive::parse(&bytes).unwrap().content().unwrap();
    let host = config::HostConfig {
        terrain_seed_file: Some(dir.join("seed.txt").display().to_string()),
        terrain_provenance: Some(dir.join("provenance.json").display().to_string()),
        game_package: archive_path.display().to_string(),
        http_listen: "127.0.0.1:0".into(),
        ws_listen: "127.0.0.1:0".into(),
        public_url: "http://127.0.0.1:7000".into(),
        tick_hz: 20,
        static_dir: None,
        start_policy: None,
        bot_limits: Default::default(),
        bots: vec![],
    };
    let mut app = headless_app(20);
    boot::boot(app.world_mut(), &host, &bytes).unwrap();
    let viewers: Vec<_> = (0..turts_mod_api::SLOT_COUNT).collect();
    turts_sim::visibility::initialize_context(app.world_mut(), 0x474d_0001, &viewers).unwrap();
    initialize_completed_world(app.world_mut());
    let _ = std::fs::remove_dir_all(&dir);
    let units: Vec<u32> = turts_sim::snapshot(app.world_mut())
        .iter()
        .filter(|unit| unit.owner_slot == 0)
        .map(|unit| unit.unit_id)
        .collect();
    assert_eq!(units.len(), 36, "quarry_ridge places 36 own units");
    let mut seq = 0;
    let mut send = |app: &mut bevy_app::App, envelope: OrderEnvelope| {
        seq += 1;
        let command = PlayerCommand {
            seq,
            order: envelope,
        };
        let resolved = orders::to_sim(SlotId(0), &command, &catalog).unwrap();
        app.world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .extend(resolved);
    };
    let idle = |app: &bevy_app::App, unit: u32| {
        let world = app.world();
        let entity = turts_sim::unit_entity(world, unit).unwrap();
        world.get::<turts_sim::MoveTarget>(entity).is_none()
            && world
                .get::<UnitOrders>(entity)
                .is_none_or(|orders| orders.current.is_none())
    };
    let pose = |app: &mut bevy_app::App, unit: u32| {
        turts_sim::snapshot(app.world_mut())
            .into_iter()
            .find(|row| row.unit_id == unit)
            .unwrap()
            .pos
    };
    let move_to = |x: f32, y: f32, group: u32| OrderEnvelope {
        order: "quarry_ridge:order_move".into(),
        units: units.iter().map(|&id| UnitId(id)).collect(),
        target: OrderTarget::Point(Pos { x, y }),
        queue: QueueMode::Replace,
        group,
    };
    let [out_x, out_y, back_x, back_y] = offsets(key);
    // The outbound order of the browser run came at tick 98.
    for _ in 0..98 {
        app.update();
    }
    send(&mut app, move_to(50.0 + out_x, 24.0 + out_y, 1));
    // The small units cross the gap and stand idle east of the ridge; the large ones wait.
    let small: Vec<u32> = units.iter().copied().filter(|&unit| unit <= 24).collect();
    for _ in 0..4000 {
        app.update();
        if small
            .iter()
            .all(|&unit| pose(&mut app, unit).x >= 34.0 && idle(&app, unit))
        {
            break;
        }
    }
    for _ in 0..30 {
        app.update();
    }
    send(
        &mut app,
        OrderEnvelope {
            order: "quarry_ridge:order_work".into(),
            units: vec![UnitId(units[0])],
            target: OrderTarget::TerrainWork(turts_mod_api::TerrainDesignation {
                tool: "quarry_ridge:spade".into(),
                cells: turts_mod_api::TerrainRect {
                    min: turts_mod_api::TerrainCell { x: 30, y: 20 },
                    max: turts_mod_api::TerrainCell { x: 34, y: 28 },
                },
                target_elevation: 0,
            }),
            queue: QueueMode::Replace,
            group: 0,
        },
    );
    let mut all_idle = false;
    for _ in 0..6000 {
        app.update();
        if units.iter().all(|&unit| idle(&app, unit)) {
            all_idle = true;
            break;
        }
    }
    assert!(
        all_idle,
        "key {key}: the dig and the crossing of the large units did not end"
    );
    for _ in 0..120 {
        app.update();
    }
    send(&mut app, move_to(22.0 + back_x, 24.0 + back_y, 3));
    for tick in 1..=1200 {
        app.update();
        if units.iter().all(|&unit| idle(&app, unit)) {
            return Ok(tick);
        }
    }
    let mut left = Vec::new();
    for &unit in &units {
        if !idle(&app, unit) {
            left.push((unit, pose(&mut app, unit)));
        }
    }
    Err(left)
}

#[test]
fn fm2_the_return_through_the_dug_passage_settles_all_36_within_60_s() {
    let failures: Vec<_> = FAILING_KEYS
        .iter()
        .filter_map(|&key| {
            return_crossing(key)
                .err()
                .map(|left| (key, left.len(), left.first().copied()))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "units are not idle 60 s after the return order (key, count, first): {failures:?}"
    );
}
