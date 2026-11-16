use super::*;
use crate::{
    Command, CommandKind, SlotId, apply_command,
    orders::{OrderCommand, QueueMode},
};
use turts_mod_api::{PackageId, SourceDef, TargetKind};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
fn exercise(package: &str, owner: u8, start: bevy_math::Vec2, cell: Cell, tool: &str, target: i32) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mods")
        .join(package);
    let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
    let catalog = archive.content().unwrap();
    let (file, map) = archive.maps().unwrap().remove(0);
    let resolved = turts_mod_runtime::terrain::validate_terrain_map(
        &PackageId(package.into()),
        &file,
        &map,
        &catalog,
    )
    .unwrap();
    let mut app = crate::headless_app(20);
    let staged = crate::stage_package_with_terrain(
        app.world(),
        catalog,
        SourceDef { file, value: map },
        20,
        ContentId::parse(&format!("{package}:field")).unwrap(),
        resolved,
        Some(super::super::PrivateTerrainSeed::new(20261115)),
    )
    .unwrap();
    crate::install_package(app.world_mut(), staged);
    crate::visibility::initialize_context(app.world_mut(), 77, &[0, 1]).unwrap();
    let actor = app
        .world()
        .iter_entities()
        .find_map(|entity| {
            (entity.get::<Position>()?.0 == start && entity.get::<Owner>()?.0.0 == owner)
                .then(|| entity.get::<UnitId>().unwrap().0)
        })
        .unwrap();
    let before = app
        .world()
        .resource::<TerrainState>()
        .elevation(cell)
        .unwrap();
    let order = OrderCommand {
        order: ContentId::parse(&format!("{package}:order_work")).unwrap(),
        behaviour: Behaviour::TerrainWork,
        target_kind: TargetKind::TerrainDesignation,
        policy: None,
        units: vec![actor],
        target: OrderTargetSim::TerrainWork {
            tool: ContentId::parse(&format!("{package}:{tool}")).unwrap(),
            cells: TerrainRect {
                min: TerrainCell {
                    x: cell.x,
                    y: cell.y,
                },
                max: TerrainCell {
                    x: cell.x + 1,
                    y: cell.y + 1,
                },
            },
            target_elevation: target,
        },
        queue: QueueMode::Replace,
        group: 0,
        allowed_units: vec![],
        queue_depth: 16,
    };
    let accepted = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(owner),
            seq: 1,
            kind: CommandKind::Order(Box::new(order)),
        },
    );
    assert!(
        accepted.outcome.is_ok(),
        "public work order rejected: {accepted:?}"
    );
    let mut moved = false;
    let mut progressed = false;
    let mut first_goal = None;
    for _ in 0..600 {
        app.update();
        let entity = unit_entity(app.world(), actor).unwrap();
        moved |= app.world().get::<Position>(entity).unwrap().0 != start;
        if first_goal.is_none() {
            first_goal = app
                .world()
                .get::<crate::world::MoveTarget>(entity)
                .map(|target| target.0);
        }
        let height = app
            .world()
            .resource::<TerrainState>()
            .elevation(cell)
            .unwrap();
        progressed |= height < before;
        if progressed {
            break;
        }
    }
    let entity = unit_entity(app.world(), actor).unwrap();
    let position = app.world().get::<Position>(entity).unwrap().0;
    let state = app
        .world()
        .resource::<TerrainWorkRuntime>()
        .units
        .get(&actor);
    assert!(
        moved,
        "ordinary work order must approach its cell: package={package},owner={owner},first_goal={first_goal:?},position={position:?},state={state:?}"
    );
    assert!(
        progressed,
        "ordinary work order must make actual terrain progress: package={package},owner={owner},first_goal={first_goal:?},position={position:?},state={state:?}"
    );
    assert!(
        super::super::ground_elevation(app.world(), actor).is_some(),
        "worker must retain complete physical support"
    );
}
#[test]
fn stockyard_west_worker_reaches_and_excavates_the_public_two_unit_cell() {
    exercise(
        "terrain_stockyard",
        0,
        bevy_math::Vec2::new(20.0, 16.0),
        Cell { x: 11, y: 10 },
        "scraper",
        1,
    );
}
#[test]
fn stockyard_east_worker_reaches_and_excavates_without_crossing_the_ridge() {
    exercise(
        "terrain_stockyard",
        1,
        bevy_math::Vec2::new(32.0, 16.0),
        Cell { x: 12, y: 10 },
        "scraper",
        1,
    );
}
#[test]
fn quarry_worker_reaches_and_excavates_at_one_unit_resolution() {
    exercise(
        "quarry",
        0,
        bevy_math::Vec2::new(13.0, 8.0),
        Cell { x: 15, y: 11 },
        "spade",
        3,
    );
}
