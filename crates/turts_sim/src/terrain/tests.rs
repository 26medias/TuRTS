use super::*;
use crate::{
    Command, CommandKind, SlotId, UnitOrders, apply_command, headless_app,
    orders::{OrderCommand, OrderTargetSim, QueueMode},
};
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
fn id(local: &str) -> ContentId {
    ContentId::parse(&format!("base:{local}")).unwrap()
}
fn rect(x: u32, y: u32, width: u32) -> TerrainRect {
    TerrainRect {
        min: TerrainCell { x, y },
        max: TerrainCell {
            x: x + width,
            y: y + 1,
        },
    }
}
struct Fixture {
    app: bevy_app::App,
    worker: u32,
    depot: u32,
    sequence: u32,
}
impl Fixture {
    fn new(rate: u64, capacity: u64, initial: i32) -> Self {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
        let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
        let mut catalog = archive.content().unwrap();
        let (_, mut map) = archive.maps().unwrap().remove(0);
        let worker = catalog.units.get_mut(&id("harvester")).unwrap();
        worker.footprint = Footprint::Circle { radius: 0.25 };
        worker.gatherer = None; // Explicit cargo works without granting Gatherer.
        worker.terrain_cargo = Some(TerrainCargoDef {
            unload_per_second: 20,
            interaction_range: 0.25,
        });
        worker.terrain_tools = vec![id("dig"), id("fill")];
        worker.inventory = Some(ResolvedInventory {
            capacity,
            accepts: vec![id("ore")],
        });
        let depot = catalog.units.get_mut(&id("command_center")).unwrap();
        depot.footprint = Footprint::Circle { radius: 0.25 };
        depot.terrain_foundation = Some(TerrainFoundation::ForbidEdit);
        depot.inventory = Some(ResolvedInventory {
            capacity: 1_000_000,
            accepts: vec![id("ore")],
        });
        depot.drop_off = Some(ResolvedDropOff {
            rules: vec![ResolvedDropOffRule {
                item: id("ore"),
                effect: ResolvedDropOffEffect::Store,
            }],
        });
        let density = Ratio {
            numerator: 1000,
            denominator: 1,
        };
        catalog.terrain_materials.insert(
            id("stone"),
            ResolvedTerrainMaterial {
                id: id("stone"),
                name: "Stone".into(),
                surface_color: "#888888".into(),
                hardness: 1,
                work_per_world_cube: density,
                natural_outputs: vec![ResolvedTerrainResourceRate {
                    resource: id("ore"),
                    per_world_cube: density,
                }],
                fill_inputs: vec![
                    ResolvedTerrainResourceRate {
                        resource: id("ore"),
                        per_world_cube: density,
                    },
                    ResolvedTerrainResourceRate {
                        resource: id("credits"),
                        per_world_cube: density,
                    },
                ],
                placed_removal: TerrainPlacedRemoval::NoYield,
            },
        );
        for (name, operation) in [
            ("dig", ResolvedTerrainOperation::Excavate),
            (
                "fill",
                ResolvedTerrainOperation::Fill {
                    material: id("stone"),
                },
            ),
        ] {
            catalog.terrain_tools.insert(
                id(name),
                ResolvedTerrainTool {
                    id: id(name),
                    name: name.into(),
                    operation,
                    work_per_second: rate,
                    reach: 6.0,
                    allowed_materials: vec![id("stone")],
                    max_hardness: 1,
                    output_destination: TerrainOutputDestination::Inventory,
                },
            );
        }
        map.id = "terrain_fixture".into();
        map.size = MapSize { x: 8.0, y: 8.0 };
        map.nodes.clear();
        map.belts.clear();
        map.placements.clear();
        map.visibility = None;
        for (unit, x, y) in [("harvester", 1.5, 1.5), ("command_center", 1.5, 4.5)] {
            map.placements.push(Placement {
                unit: id(unit).to_string(),
                owner_slot: 0,
                pos: WorldPos { x, y },
                contents: vec![],
                facing: None,
                start_order: None,
            });
        }
        map.terrain = Some(TerrainDef {
            length_scale: 1000,
            cell_width: 1000,
            elevation_step: 1,
            columns: 8,
            rows: 8,
            chunk_width: 2,
            initial_elevation: initial,
            initial_heights: vec![],
            floor: 0,
            ceiling: initial + 1000,
            generation: TerrainGenerationDef::Version1 {
                bands: vec![TerrainDatumBand {
                    bottom: 0,
                    top: initial,
                    choices: vec![TerrainWeightedMaterial {
                        material: id("stone").to_string(),
                        weight: 1,
                    }],
                }],
            },
        });
        let resolved = turts_mod_runtime::terrain::validate_terrain_map(
            &PackageId("base".into()),
            "maps/terrain_fixture.ron",
            &map,
            &catalog,
        )
        .unwrap();
        let mut app = headless_app(20);
        let staged = crate::bootstrap::stage_package_with_terrain(
            app.world(),
            catalog,
            SourceDef {
                file: "maps/terrain_fixture.ron".into(),
                value: map,
            },
            20,
            id("terrain_fixture"),
            resolved,
            Some(PrivateTerrainSeed::new(77)),
        )
        .unwrap();
        crate::install_package(app.world_mut(), staged);
        let find = |name: &str| {
            app.world()
                .iter_entities()
                .find_map(|entity| {
                    let kind = entity.get::<crate::UnitType>()?;
                    (app.world()
                        .resource::<crate::UnitTypes>()
                        .get(kind.0)?
                        .content_id
                        == id(name).to_string())
                    .then(|| entity.get::<crate::UnitId>().unwrap().0)
                })
                .unwrap()
        };
        let worker = find("harvester");
        let depot = find("command_center");
        Self {
            app,
            worker,
            depot,
            sequence: 0,
        }
    }
    fn command(&mut self, behaviour: Behaviour, target: OrderTargetSim, queue: QueueMode) {
        self.sequence += 1;
        let target_kind = match &target {
            OrderTargetSim::TerrainWork { .. } => TargetKind::TerrainDesignation,
            OrderTargetSim::Unit(_) => TargetKind::Unit,
            _ => TargetKind::None,
        };
        let command = Command {
            slot: SlotId(0),
            seq: self.sequence,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: id(match behaviour {
                    Behaviour::TerrainWork => "terrain",
                    Behaviour::UnloadCargo => "unload",
                    Behaviour::Stop => "stop",
                    _ => "fixture",
                }),
                behaviour,
                target_kind,
                policy: None,
                units: vec![self.worker],
                target,
                queue,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        };
        let result = apply_command(self.app.world_mut(), &command);
        assert!(result.outcome.is_ok(), "{result:?}");
    }
    fn work(&mut self, tool: &str, cells: TerrainRect, level: i32, queue: QueueMode) {
        self.command(
            Behaviour::TerrainWork,
            OrderTargetSim::TerrainWork {
                tool: id(tool),
                cells,
                target_elevation: level,
            },
            queue,
        );
    }
    fn tick(&mut self) {
        self.app.update();
    }
    fn ore(&self, unit: u32) -> u64 {
        self.app
            .world()
            .resource::<crate::EconomyLedger>()
            .inventories
            .get(&unit)
            .and_then(|inventory| inventory.items.get(&id("ore")))
            .map_or(0, |account| account.on_hand)
    }
    fn credits(&self) -> u64 {
        self.app.world().resource::<crate::EconomyLedger>().balances[&(0, id("credits"))].on_hand
    }
    fn height(&self, x: u32, y: u32) -> i32 {
        let terrain = self.app.world().resource::<TerrainState>();
        terrain.columns[terrain
            .index(turts_rules::terrain::geometry::Cell { x, y })
            .unwrap()]
        .surface()
    }
}
#[test]
fn real_ticks_high_rate_batch_work_is_conserved_across_cell_completion_and_pause() {
    let mut fixture = Fixture::new(401, 100_000, 1000);
    fixture.work("dig", rect(3, 1, 2), 0, QueueMode::Replace);
    for ticks in 1u64..=70 {
        fixture.tick();
        let state = fixture
            .app
            .world()
            .resource::<work::TerrainWorkRuntime>()
            .units
            .get(&fixture.worker)
            .unwrap();
        let output = fixture.ore(fixture.worker);
        assert_eq!(
            output + state.done,
            ticks * 401 / 20,
            "whole work is either committed or retained at tick{ticks}"
        );
        assert_eq!(state.carry, ticks * 401 % 20);
        assert_eq!(
            output,
            u64::try_from(2000 - fixture.height(3, 1) - fixture.height(4, 1)).unwrap()
        );
    }
    assert_eq!(fixture.height(3, 1), 0);
    assert!(fixture.height(4, 1) < 1000);
}
#[test]
fn full_cargo_pauses_exact_work_then_ordinary_unload_and_queued_work_resume() {
    let mut fixture = Fixture::new(401, 5, 1000);
    fixture.work("dig", rect(3, 1, 1), 0, QueueMode::Replace);
    fixture.tick();
    assert_eq!(fixture.ore(fixture.worker), 5);
    let before = work::rows_for(fixture.app.world(), 0);
    let state = fixture
        .app
        .world()
        .resource::<work::TerrainWorkRuntime>()
        .units[&fixture.worker]
        .clone();
    assert_eq!(state.done, 15);
    assert_eq!(state.carry, 1);
    for _ in 0..5 {
        fixture.tick();
    }
    let paused = &fixture
        .app
        .world()
        .resource::<work::TerrainWorkRuntime>()
        .units[&fixture.worker];
    assert_eq!(paused.done, state.done);
    assert_eq!(paused.carry, state.carry);
    assert_eq!(before[0].current_cell, paused.cell);
    let credits = fixture.credits();
    fixture.command(
        Behaviour::UnloadCargo,
        OrderTargetSim::Unit(fixture.depot),
        QueueMode::Replace,
    );
    fixture.work("dig", rect(3, 1, 1), 990, QueueMode::Append);
    for _ in 0..150 {
        fixture.tick();
        if fixture.ore(fixture.depot) == 5 && fixture.height(3, 1) == 990 {
            break;
        }
    }
    assert_eq!(fixture.ore(fixture.depot), 5);
    assert_eq!(fixture.ore(fixture.worker), 5);
    assert_eq!(
        fixture.credits(),
        credits,
        "unloading stores cargo without resource conversion"
    );
    assert_eq!(fixture.height(3, 1), 990);
}
#[test]
fn fill_pays_mixed_inputs_atomically_and_placed_removal_yields_nothing() {
    let mut fixture = Fixture::new(20, 100, 20);
    fixture.work("dig", rect(3, 1, 1), 17, QueueMode::Replace);
    for _ in 0..5 {
        fixture.tick();
    }
    assert_eq!(fixture.ore(fixture.worker), 3);
    let credits = fixture.credits();
    fixture.work("fill", rect(3, 1, 1), 20, QueueMode::Replace);
    for _ in 0..5 {
        fixture.tick();
    }
    assert_eq!(fixture.height(3, 1), 20);
    assert_eq!(fixture.ore(fixture.worker), 0);
    assert_eq!(fixture.credits(), credits - 3);
    fixture.work("dig", rect(3, 1, 1), 17, QueueMode::Replace);
    for _ in 0..5 {
        fixture.tick();
    }
    assert_eq!(fixture.ore(fixture.worker), 0);
    assert_eq!(fixture.height(3, 1), 17);
    let terrain = fixture.app.world().resource::<TerrainState>();
    let column = &terrain.columns[terrain
        .index(turts_rules::terrain::geometry::Cell { x: 3, y: 1 })
        .unwrap()];
    assert_eq!(column.natural_top(), 17);
    assert!(column.placed_runs().is_empty());
}
#[test]
fn cancellation_discards_only_uncommitted_work_and_same_context_keeps_natural_depletion() {
    let mut fixture = Fixture::new(1, 100, 20);
    fixture.work("dig", rect(3, 1, 1), 19, QueueMode::Replace);
    for _ in 0..10 {
        fixture.tick();
    }
    let before = privileged_audit(fixture.app.world()).unwrap();
    fixture.command(Behaviour::Stop, OrderTargetSim::None, QueueMode::Replace);
    for _ in 0..5 {
        fixture.tick();
    }
    assert!(
        fixture
            .app
            .world()
            .resource::<work::TerrainWorkRuntime>()
            .units
            .is_empty()
    );
    assert_eq!(
        privileged_audit(fixture.app.world()).unwrap().columns_state,
        before.columns_state
    );
    assert_eq!(fixture.ore(fixture.worker), 0);
    assert!(
        fixture
            .app
            .world()
            .get::<UnitOrders>(crate::unit_entity(fixture.app.world(), fixture.worker).unwrap())
            .unwrap()
            .current
            .is_none()
    );
}
