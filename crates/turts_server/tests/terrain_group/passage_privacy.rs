//! GM6 in the passage rule (T-39 M1): a body that its owner may not see is never a passage
//! holder (`flock.rs`, the `unit_permitted` check of the holder search). Two worlds differ
//! only by one hidden enemy unit that stands in the gap. Up to the first body contact, the
//! own unit must move in the same frames in both worlds. A hidden holder would make it wait
//! at the mouth before any contact, and the hidden unit would leak through its motion.
use super::{boot, config::HostConfig, orders};
use std::{
    fs,
    path::PathBuf,
    process::Command as Process,
    sync::atomic::{AtomicU64, Ordering},
};
use turts_mod_api::{MapDef, Placement, UnitDef, VisibilityDef, VisionSharing, WorldPos};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{OrderEnvelope, OrderTarget, PlayerCommand, Pos, QueueMode, UnitId};
use turts_sim::{
    CommandQueue, SlotId, Vec2, headless_app, initialize_completed_world, unit_entity,
};

static NEXT: AtomicU64 = AtomicU64::new(0);
const PACKAGE: &str = "quarry";
/// GM2-narrow: the ridge is X 30..34, the gap is Y 23..25.
const OWN_START: [f32; 2] = [24.0, 24.0];
const ENEMY_AT: [f32; 2] = [32.0, 24.0];
const GOAL: [f32; 2] = [40.0, 24.0];
/// Own unit id 1. The enemy unit is placed last, so its id (2) shifts no own id.
const OWN: u32 = 1;
const ENEMY: u32 = 2;
const TICKS: u32 = 400;

/// GM2-narrow with fog: each slot sees with its own units only, and no unit has sight.
fn fog_world(with_enemy: bool) -> (bevy::app::App, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let dir = std::env::temp_dir().join(format!(
        "turts-passage-privacy-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    let output = Process::new("python3")
        .arg(root.join("tests/browser/terrain-group/generate.py"))
        .arg("--root")
        .arg(&root)
        .arg("--package")
        .arg(PACKAGE)
        .arg("--scene")
        .arg("GM2-narrow")
        .arg("--output")
        .arg(&dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "public generator: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let frozen: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("fixture.json")).unwrap()).unwrap();
    let small = frozen["bindings"]["small"].as_str().unwrap().to_owned();
    let public = dir.join(PACKAGE);
    for kind in ["small", "large"] {
        let name = frozen["bindings"][kind].as_str().unwrap();
        let path = public.join(format!("units/{name}.ron"));
        let mut unit: UnitDef = ron::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        unit.vision = None;
        fs::write(path, ron::ser::to_string(&unit).unwrap()).unwrap();
    }
    let mut map: MapDef =
        ron::from_str(&fs::read_to_string(public.join("maps/field.ron")).unwrap()).unwrap();
    map.visibility = Some(VisibilityDef {
        cell_size: 1.0,
        sharing: VisionSharing::OwnSlot,
        memory_limit: Some(65536),
    });
    let placement = |slot: u8, at: [f32; 2]| Placement {
        unit: format!("{PACKAGE}:{small}"),
        owner_slot: slot,
        pos: WorldPos { x: at[0], y: at[1] },
        contents: vec![],
        facing: None,
        start_order: None,
    };
    map.placements = vec![placement(0, OWN_START)];
    if with_enemy {
        map.placements.push(placement(1, ENEMY_AT));
    }
    map.slot_economy = vec![];
    fs::write(
        public.join("maps/field.ron"),
        ron::ser::to_string(&map).unwrap(),
    )
    .unwrap();
    let bytes = pack_dir(&public).unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    turts_pack::check_package(&archive).unwrap();
    let catalog = archive.content().unwrap();
    fs::write(dir.join("seed.txt"), "3457144321\n").unwrap();
    let config = HostConfig {
        terrain_seed_file: Some(dir.join("seed.txt").display().to_string()),
        terrain_provenance: Some(dir.join("private-provenance.json").display().to_string()),
        game_package: dir.join("package.tar").display().to_string(),
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
    boot::boot(app.world_mut(), &config, &bytes).unwrap();
    turts_sim::visibility::initialize_context(app.world_mut(), 0x474d0016, &[0, 1]).unwrap();
    initialize_completed_world(app.world_mut());
    let command = PlayerCommand {
        seq: 1,
        order: OrderEnvelope {
            order: format!("{PACKAGE}:order_move"),
            units: vec![UnitId(OWN)],
            target: OrderTarget::Point(Pos {
                x: GOAL[0],
                y: GOAL[1],
            }),
            queue: QueueMode::Replace,
            group: 1,
        },
    };
    let resolved = orders::to_sim(SlotId(0), &command, &catalog).unwrap();
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .extend(resolved);
    (app, dir)
}

fn position(app: &bevy::app::App, unit: u32) -> Vec2 {
    let entity = unit_entity(app.world(), unit).unwrap();
    app.world().get::<turts_sim::Position>(entity).unwrap().0
}

/// The own unit's position after each tick, and in the enemy world the enemy's position.
fn frames(with_enemy: bool) -> Vec<(Vec2, Option<Vec2>)> {
    let (mut app, dir) = fog_world(with_enemy);
    let mut out = Vec::new();
    for _ in 0..TICKS {
        app.update();
        out.push((
            position(&app, OWN),
            with_enemy.then(|| position(&app, ENEMY)),
        ));
    }
    let _ = fs::remove_dir_all(dir);
    out
}

#[test]
fn gm6_a_hidden_enemy_in_the_gap_does_not_change_own_motion_before_contact() {
    let clear = frames(false);
    let hidden = frames(true);
    // Two small bodies of radius 0.4 touch at 0.8 m. Compare up to 0.25 m before that.
    let contact = 0.8 + 0.25;
    let horizon = hidden
        .iter()
        .position(|(own, enemy)| own.distance(enemy.unwrap()) <= contact)
        .unwrap_or(hidden.len());
    for (tick, ((own, _), (alone, _))) in hidden.iter().zip(&clear).take(horizon).enumerate() {
        assert_eq!(
            own,
            alone,
            "tick {}: a hidden enemy in the gap changed the own unit before contact",
            tick + 1
        );
    }
    // The holder search looks 10 radii (4 m) ahead. The compared frames must include the
    // approach inside that range, or the test proves nothing about the passage rule.
    let near = hidden[..horizon]
        .iter()
        .filter(|(own, enemy)| own.distance(enemy.unwrap()) < 4.0)
        .count();
    eprintln!("passage privacy: contact horizon tick {horizon}, {near} frames within 4 m");
    assert!(
        near >= 10,
        "the own unit came within 4 m of the hidden unit in only {near} frames before contact"
    );
    let moved = clear[horizon.saturating_sub(1)]
        .0
        .distance(Vec2::from_array(OWN_START));
    assert!(
        moved > 2.0,
        "the own unit moved only {moved} m before the contact horizon"
    );
}
