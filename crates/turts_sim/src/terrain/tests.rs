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
        Self::configured(rate, capacity, initial, |_, _| {})
    }
    fn configured(
        rate: u64,
        capacity: u64,
        initial: i32,
        configure: impl FnOnce(&mut ContentCatalog, &mut MapDef),
    ) -> Self {
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
        configure(&mut catalog, &mut map);
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
                .filter_map(|entity| {
                    let kind = entity.get::<crate::UnitType>()?;
                    if app
                        .world()
                        .resource::<crate::UnitTypes>()
                        .get(kind.0)?
                        .content_id
                        != id(name).to_string()
                    {
                        return None;
                    }
                    let position = entity.get::<crate::Position>()?.0;
                    Some((position, entity.get::<crate::UnitId>()?.0))
                })
                .min_by(|left, right| {
                    left.0
                        .x
                        .total_cmp(&right.0.x)
                        .then_with(|| left.0.y.total_cmp(&right.0.y))
                })
                .unwrap()
                .1
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
        self.command_units(vec![self.worker], behaviour, target, queue)
    }
    fn command_units(
        &mut self,
        units: Vec<u32>,
        behaviour: Behaviour,
        target: OrderTargetSim,
        queue: QueueMode,
    ) {
        self.command_slot(SlotId(0), units, behaviour, target, queue)
    }
    fn command_slot(
        &mut self,
        slot: SlotId,
        units: Vec<u32>,
        behaviour: Behaviour,
        target: OrderTargetSim,
        queue: QueueMode,
    ) {
        self.sequence += 1;
        let target_kind = match &target {
            OrderTargetSim::TerrainWork { .. } => TargetKind::TerrainDesignation,
            OrderTargetSim::Unit(_) => TargetKind::Unit,
            OrderTargetSim::Point(_) => TargetKind::Point,
            _ => TargetKind::None,
        };
        let command = Command {
            slot,
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
                units,
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
    fn unit_at(&self, x: f32, y: f32) -> u32 {
        self.app
            .world()
            .iter_entities()
            .find_map(|entity| {
                (entity.get::<crate::Position>()?.0 == crate::Vec2::new(x, y))
                    .then(|| entity.get::<crate::UnitId>().unwrap().0)
            })
            .unwrap()
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

#[test]
fn immutable_phase_surface_and_world_sidecar_never_read_later_columns_or_serialize_truth() {
    let mut fixture = Fixture::new(20, 100, 20);
    fixture.work("dig", rect(3, 1, 1), 17, QueueMode::Replace);
    fixture.tick();
    let tactical = phase(fixture.app.world(), ObservationPhase::Tactical).unwrap();
    let completed = phase(fixture.app.world(), ObservationPhase::Completed).unwrap();
    let cell = TerrainCell { x: 3, y: 1 };
    assert_eq!(tactical.observe_cell(0, cell).unwrap().elevation, 20);
    assert_eq!(completed.observe_cell(0, cell).unwrap().elevation, 19);
    assert_eq!(
        completed.observe_cell(0, cell).unwrap().provenance,
        Some(TerrainProvenance::Natural)
    );
    let world_view = crate::world_view(fixture.app.world_mut());
    assert!(std::sync::Arc::ptr_eq(
        world_view.terrain.as_ref().unwrap(),
        &completed
    ));
    fixture.tick();
    assert_eq!(fixture.height(3, 1), 18);
    assert_eq!(tactical.observe_cell(0, cell).unwrap().elevation, 20);
    assert_eq!(completed.observe_cell(0, cell).unwrap().elevation, 19);
    assert_eq!(
        world_view
            .terrain
            .as_ref()
            .unwrap()
            .observe_cell(0, cell)
            .unwrap()
            .elevation,
        19
    );
    let json = serde_json::to_value(&world_view).unwrap();
    assert!(
        json.get("terrain").is_none(),
        "the immutable cache must never serialize unprojected top surfaces"
    );
    let decoded: crate::WorldView = serde_json::from_value(json).unwrap();
    assert!(decoded.terrain.is_none());
    assert_eq!(
        decoded, world_view,
        "the skipped sidecar is not authoritative state"
    );
    let audit = crate::audit_state(fixture.app.world_mut());
    assert!(audit.raw_terrain.is_some());
    let decoded: crate::AuditState =
        serde_json::from_slice(&serde_json::to_vec(&audit).unwrap()).unwrap();
    assert_eq!(
        decoded, audit,
        "private raw terrain remains part of authoritative equality"
    );
    let debug = format!("{completed:?}");
    assert!(
        !debug.contains("stone")
            && !debug.contains("elevations")
            && !debug.contains("private_seed")
    );
}
#[test]
fn unknown_surface_has_no_material_work_or_foreign_anchor_but_own_contact_remains() {
    let mut fixture = Fixture::configured(20, 100, 20, |_, map| {
        map.visibility = Some(VisibilityDef {
            cell_size: 2.0,
            sharing: VisionSharing::OwnSlot,
            memory_limit: None,
        });
    });
    crate::visibility::initialize_context(fixture.app.world_mut(), 77, &[0, 1]).unwrap();
    let old = phase(fixture.app.world(), ObservationPhase::Completed).unwrap();
    assert!(old.observe_cell(0, TerrainCell { x: 3, y: 1 }).is_none());
    assert_eq!(old.ground_anchor(0, fixture.worker).unwrap().elevation, 20);
    assert!(old.ground_anchor(1, fixture.worker).is_none());
    fixture.work("dig", rect(3, 1, 1), 17, QueueMode::Replace);
    fixture.tick();
    assert_eq!(fixture.height(3, 1), 20);
    let completed = phase(fixture.app.world(), ObservationPhase::Completed).unwrap();
    let row = completed.work_for(0).next().unwrap();
    assert_eq!(row.wait, TerrainWorkWait::Unknown);
    assert_eq!(row.required_work, None);
    assert_eq!(row.completed_work, 0);
    assert_eq!(completed.work_for(1).count(), 0);
}

fn wall(catalog: &mut ContentCatalog, map: &mut MapDef, gap: bool) {
    let terrain = map.terrain.as_mut().unwrap();
    terrain.initial_heights = if gap {
        vec![
            TerrainHeightRect {
                cells: TerrainRect {
                    min: TerrainCell { x: 3, y: 0 },
                    max: TerrainCell { x: 4, y: 4 },
                },
                elevation: 40,
            },
            TerrainHeightRect {
                cells: TerrainRect {
                    min: TerrainCell { x: 3, y: 5 },
                    max: TerrainCell { x: 4, y: 8 },
                },
                elevation: 40,
            },
        ]
    } else {
        vec![TerrainHeightRect {
            cells: TerrainRect {
                min: TerrainCell { x: 3, y: 0 },
                max: TerrainCell { x: 4, y: 8 },
            },
            elevation: 40,
        }]
    };
    let TerrainGenerationDef::Version1 { bands } = &mut terrain.generation;
    bands[0].top = 40;
    map.placements[0].pos = WorldPos { x: 1.5, y: 4.5 };
    map.placements[1].pos = WorldPos { x: 6.5, y: 6.5 };
    catalog.units.get_mut(&id("harvester")).unwrap().movement = Some(MovementDef { speed: 100.0 });
}
#[test]
fn fast_real_move_cannot_jump_a_one_cell_cliff_and_keeps_its_order() {
    let mut fixture = Fixture::configured(20, 100, 20, |catalog, map| wall(catalog, map, false));
    fixture.command(
        Behaviour::Move,
        OrderTargetSim::Point(crate::Vec2::new(5.5, 4.5)),
        QueueMode::Replace,
    );
    for _ in 0..10 {
        fixture.tick();
    }
    let entity = crate::unit_entity(fixture.app.world(), fixture.worker).unwrap();
    assert!(
        fixture
            .app
            .world()
            .get::<crate::Position>(entity)
            .unwrap()
            .0
            .x
            < 2.75
    );
    assert!(
        fixture
            .app
            .world()
            .get::<crate::MoveTarget>(entity)
            .is_some()
    );
    let retained = &fixture
        .app
        .world()
        .resource::<crate::orders::PlainMoveOrderIdentities>()
        .0[&fixture.worker];
    assert_eq!(
        retained.target,
        OrderTargetSim::Point(crate::Vec2::new(5.5, 4.5))
    );
    assert!(
        fixture
            .app
            .world()
            .get::<navigation::TerrainMotionWait>(entity)
            .is_some()
    );
}
#[test]
fn real_footprints_keep_large_rover_out_of_a_worker_width_gap() {
    let mut small = Fixture::configured(20, 100, 20, |catalog, map| wall(catalog, map, true));
    let mut large = Fixture::configured(20, 100, 20, |catalog, map| {
        wall(catalog, map, true);
        catalog.units.get_mut(&id("harvester")).unwrap().footprint =
            Footprint::Circle { radius: 0.75 };
    });
    for fixture in [&mut small, &mut large] {
        fixture.command(
            Behaviour::Move,
            OrderTargetSim::Point(crate::Vec2::new(5.5, 4.5)),
            QueueMode::Replace,
        );
        for _ in 0..30 {
            fixture.tick();
        }
    }
    let position = |fixture: &Fixture| {
        fixture
            .app
            .world()
            .get::<crate::Position>(
                crate::unit_entity(fixture.app.world(), fixture.worker).unwrap(),
            )
            .unwrap()
            .0
    };
    assert!(
        position(&small).x > 4.5,
        "a half-width worker crosses the gap"
    );
    assert!(
        position(&large).x < 3.0,
        "centre-only traversal must not admit the rover"
    );
}
#[test]
fn real_class_step_and_grade_both_apply_even_to_a_fast_crossing() {
    for (max_step, grade_denominator, crosses) in [(0, 1, false), (20, 100, false), (20, 50, true)]
    {
        let mut fixture = Fixture::configured(20, 100, 20, |catalog, map| {
            wall(catalog, map, false);
            catalog.terrain_movement_classes.insert(
                id("walker_class"),
                ResolvedTerrainMovementClass {
                    id: id("walker_class"),
                    name: "Walker".into(),
                    max_step,
                    max_grade: Ratio {
                        numerator: 1,
                        denominator: grade_denominator,
                    },
                },
            );
            catalog
                .units
                .get_mut(&id("harvester"))
                .unwrap()
                .terrain_class = Some(id("walker_class"));
        });
        fixture.command(
            Behaviour::Move,
            OrderTargetSim::Point(crate::Vec2::new(5.5, 4.5)),
            QueueMode::Replace,
        );
        for _ in 0..30 {
            fixture.tick();
        }
        let position = fixture
            .app
            .world()
            .get::<crate::Position>(
                crate::unit_entity(fixture.app.world(), fixture.worker).unwrap(),
            )
            .unwrap()
            .0;
        assert_eq!(
            position.x > 4.5,
            crosses,
            "step{max_step}, grade1/{grade_denominator}, pose{position:?}"
        );
    }
}
#[test]
fn ordinary_dig_opens_a_cached_blocked_route_but_distant_edit_does_not_invalidate_it() {
    let mut fixture = Fixture::configured(20, 1000, 20, |catalog, map| {
        wall(catalog, map, false);
        map.placements[0].pos = WorldPos { x: 1.5, y: 1.5 };
        let mut walker = map.placements[0].clone();
        walker.pos = WorldPos { x: 1.5, y: 4.5 };
        map.placements.push(walker);
    });
    let walker = fixture.unit_at(1.5, 4.5);
    fixture.command_units(
        vec![walker],
        Behaviour::Move,
        OrderTargetSim::Point(crate::Vec2::new(5.5, 4.5)),
        QueueMode::Replace,
    );
    fixture.tick();
    let before = privileged_navigation_diagnostics(fixture.app.world());
    fixture.work("dig", rect(6, 0, 1), 19, QueueMode::Replace);
    fixture.tick();
    fixture.tick();
    let distant = privileged_navigation_diagnostics(fixture.app.world());
    assert_eq!(distant.invalidations, before.invalidations);
    assert_eq!(distant.searches, before.searches);
    fixture.work("dig", rect(3, 4, 1), 20, QueueMode::Replace);
    for _ in 0..45 {
        fixture.tick();
    }
    assert_eq!(fixture.height(3, 4), 20);
    let after = privileged_navigation_diagnostics(fixture.app.world());
    assert!(after.invalidations > distant.invalidations);
    let position = fixture
        .app
        .world()
        .get::<crate::Position>(crate::unit_entity(fixture.app.world(), walker).unwrap())
        .unwrap()
        .0;
    assert!(
        position.x > 4.5,
        "the original retained move crosses the newly opened passage: {position:?}"
    );
}
#[test]
fn real_allocator_inversion_keeps_physical_work_claim_winner() {
    let mut result = Vec::new();
    for reverse in [false, true] {
        let mut fixture = Fixture::configured(20, 100, 20, |_, map| {
            let mut other = map.placements[0].clone();
            other.pos = WorldPos { x: 1.5, y: 2.5 };
            map.placements.push(other);
            if reverse {
                map.placements.reverse();
            }
        });
        let first = fixture.unit_at(1.5, 1.5);
        let later = fixture.unit_at(1.5, 2.5);
        fixture.command_units(
            vec![later, first],
            Behaviour::TerrainWork,
            OrderTargetSim::TerrainWork {
                tool: id("dig"),
                cells: rect(3, 1, 1),
                target_elevation: 17,
            },
            QueueMode::Replace,
        );
        for _ in 0..5 {
            fixture.tick();
        }
        assert_eq!(fixture.ore(first), 3);
        assert_eq!(fixture.ore(later), 0);
        result.push((first, later, fixture.height(3, 1)));
    }
    assert_ne!(
        result[0].0, result[1].0,
        "real placement order changed the physical actor's allocated identity"
    );
    assert_eq!(result[0].2, result[1].2);
}
#[test]
fn a_later_closer_worker_cannot_steal_existing_partial_work_claim() {
    let mut fixture = Fixture::configured(1, 100, 20, |_, map| {
        let mut other = map.placements[0].clone();
        other.pos = WorldPos { x: 2.5, y: 1.5 };
        map.placements.push(other);
    });
    fixture.work("dig", rect(4, 1, 1), 19, QueueMode::Replace);
    fixture.tick();
    let later = fixture.unit_at(2.5, 1.5);
    fixture.command_units(
        vec![later],
        Behaviour::TerrainWork,
        OrderTargetSim::TerrainWork {
            tool: id("dig"),
            cells: rect(4, 1, 1),
            target_elevation: 19,
        },
        QueueMode::Replace,
    );
    for _ in 0..22 {
        fixture.tick();
    }
    assert_eq!(fixture.ore(fixture.worker), 1);
    assert_eq!(fixture.ore(later), 0);
}
#[test]
fn missing_second_fill_input_cannot_charge_first_input_or_change_column() {
    let mut fixture = Fixture::configured(20, 100, 20, |_, map| {
        for slot in &mut map.slot_economy {
            for balance in &mut slot.balances {
                balance.amount = 0;
            }
        }
    });
    fixture.work("dig", rect(3, 1, 1), 17, QueueMode::Replace);
    for _ in 0..5 {
        fixture.tick();
    }
    assert_eq!(fixture.ore(fixture.worker), 3);
    let before = privileged_audit(fixture.app.world()).unwrap().columns_state;
    fixture.work("fill", rect(3, 1, 1), 20, QueueMode::Replace);
    for _ in 0..5 {
        fixture.tick();
    }
    assert_eq!(fixture.ore(fixture.worker), 3);
    assert_eq!(fixture.credits(), 0);
    assert_eq!(
        privileged_audit(fixture.app.world()).unwrap().columns_state,
        before
    );
    assert_eq!(
        work::rows_for(fixture.app.world(), 0)[0].wait,
        TerrainWorkWait::MissingInputs
    );
}

fn source_death_fixture(rate: u64, work_density: u32) -> Fixture {
    let mut fixture = Fixture::configured(rate, 100, 20, |catalog, map| {
        map.visibility = Some(VisibilityDef {
            cell_size: 2.0,
            sharing: VisionSharing::OwnSlot,
            memory_limit: None,
        });
        catalog
            .terrain_materials
            .get_mut(&id("stone"))
            .unwrap()
            .work_per_world_cube
            .numerator = work_density;
        let mut scout = catalog.units[&id("rifleman")].clone();
        scout.id = id("sight_scout");
        scout.name = "Scout".into();
        scout.vision = Some(VisionDef { radius: 6.0 });
        scout.health = Some(HealthDef { max: 100 });
        scout.removal = Some(RemovalDef {
            by_command: true,
            contents: ContentsPolicy::Destroy,
        });
        catalog.units.insert(scout.id.clone(), scout);
        let mut enemy = catalog.units[&id("harvester")].clone();
        enemy.id = id("enemy_tool");
        enemy.name = "Enemy tool".into();
        enemy.vision = Some(VisionDef { radius: 6.0 });
        enemy.movement = None;
        enemy.terrain_foundation = Some(TerrainFoundation::ForbidEdit);
        enemy.weapon = Some(ResolvedWeapon {
            damage_type: None,
            bonus: vec![],
            range: 2.0,
            range_mode: WeaponRangeMode::Centre,
            cooldown_ms: 50,
            damage: 100,
            ammo: None,
            targets: TargetFilter {
                slots: SlotRelation::Other,
            },
            when_short: vec![],
        });
        catalog.units.insert(enemy.id.clone(), enemy);
        for (unit, owner, x, y) in [("sight_scout", 0, 1.5, 2.5), ("enemy_tool", 1, 2.5, 2.5)] {
            map.placements.push(Placement {
                unit: id(unit).to_string(),
                owner_slot: owner,
                pos: WorldPos { x, y },
                contents: vec![],
                facing: None,
                start_order: None,
            });
        }
    });
    crate::visibility::initialize_context(fixture.app.world_mut(), 77, &[0, 1]).unwrap();
    fixture
}
#[test]
fn source_death_redacts_partial_work_and_hidden_edits_do_not_refresh_shared_memory() {
    let mut fixture = source_death_fixture(31, 10_000);
    let scout = fixture.unit_at(1.5, 2.5);
    let enemy = fixture.unit_at(2.5, 2.5);
    let cell = TerrainCell { x: 3, y: 1 };
    fixture.work("dig", rect(3, 1, 1), 17, QueueMode::Replace);
    fixture.tick();
    assert!(
        crate::unit_entity(fixture.app.world(), scout).is_none(),
        "ordinary combat killed the sight source after work"
    );
    let runtime = &fixture
        .app
        .world()
        .resource::<work::TerrainWorkRuntime>()
        .units[&fixture.worker];
    assert_eq!(runtime.done, 1);
    assert_eq!(runtime.carry, 11);
    assert_eq!(runtime.required, Some(10));
    let completed = phase(fixture.app.world(), ObservationPhase::Completed).unwrap();
    let row = completed.work_for(0).next().unwrap();
    assert_eq!(row.wait, TerrainWorkWait::Unknown);
    assert_eq!(row.required_work, None);
    assert_eq!(row.completed_work, 0);
    let remembered = completed.surface(0, cell).unwrap();
    assert_eq!(remembered.elevation, 20);
    assert!(!remembered.current);
    assert_eq!(remembered.last_seen_tick, 1);
    let chunk = completed
        .chunk_for(turts_rules::terrain::geometry::Cell { x: 3, y: 1 })
        .unwrap();
    let revision = completed.knowledge_revision(0, chunk);
    fixture.command_slot(
        SlotId(1),
        vec![enemy],
        Behaviour::TerrainWork,
        OrderTargetSim::TerrainWork {
            tool: id("dig"),
            cells: rect(3, 1, 1),
            target_elevation: 19,
        },
        QueueMode::Replace,
    );
    for _ in 0..10 {
        fixture.tick();
    }
    assert_eq!(
        fixture.height(3, 1),
        19,
        "the other owner's ordinary hidden work committed"
    );
    let after = phase(fixture.app.world(), ObservationPhase::Completed).unwrap();
    assert_eq!(after.surface(0, cell), Some(remembered));
    assert_eq!(
        after.knowledge_revision(0, chunk),
        revision,
        "hidden edit counters cannot invalidate permitted route caches"
    );
    assert_eq!(after.surface(1, cell).unwrap().elevation, 19);
}
#[test]
fn committed_surface_witness_survives_a_later_same_tick_sight_source_death() {
    let mut fixture = source_death_fixture(20, 1000);
    fixture.work("dig", rect(3, 1, 1), 17, QueueMode::Replace);
    fixture.tick();
    assert_eq!(fixture.ore(fixture.worker), 1);
    let completed = phase(fixture.app.world(), ObservationPhase::Completed).unwrap();
    let remembered = completed.surface(0, TerrainCell { x: 3, y: 1 }).unwrap();
    assert_eq!(remembered.elevation, 19);
    assert!(!remembered.current);
    assert_eq!(remembered.last_seen_tick, 1);
    assert_eq!(remembered.provenance, Some(TerrainProvenance::Natural));
    assert!(completed.observe_cell(0, remembered.cell).is_none());
    assert_eq!(
        committed_surface_observations(fixture.app.world(), 0)[0]
            .surface
            .elevation,
        remembered.elevation
    );
}
#[test]
fn unchanged_current_surface_retains_sample_tick_without_refresh_traffic() {
    let mut fixture = Fixture::new(20, 100, 20);
    let cell = TerrainCell { x: 3, y: 1 };
    let first = phase(fixture.app.world(), ObservationPhase::Completed)
        .unwrap()
        .surface(0, cell)
        .unwrap();
    for _ in 0..4 {
        fixture.tick();
    }
    let stable = phase(fixture.app.world(), ObservationPhase::Completed)
        .unwrap()
        .surface(0, cell)
        .unwrap();
    assert_eq!(stable, first);
    fixture.work("dig", rect(3, 1, 1), 19, QueueMode::Replace);
    fixture.tick();
    let changed = phase(fixture.app.world(), ObservationPhase::Completed)
        .unwrap()
        .surface(0, cell)
        .unwrap();
    assert_eq!(changed.elevation, 19);
    assert_eq!(changed.last_seen_tick, 5);
    for _ in 0..4 {
        fixture.tick();
    }
    assert_eq!(
        phase(fixture.app.world(), ObservationPhase::Completed)
            .unwrap()
            .surface(0, cell),
        Some(changed)
    );
}

#[test]
fn paired_unseen_ridge_worlds_plan_and_report_equally_until_physical_contact() {
    let create = || {
        let mut fixture = Fixture::configured(4000, 1000, 20, |catalog, map| {
            map.visibility = Some(VisibilityDef {
                cell_size: 2.0,
                sharing: VisionSharing::OwnSlot,
                memory_limit: None,
            });
            map.placements[0].pos = WorldPos { x: 1.5, y: 4.5 };
            map.placements[1].pos = WorldPos { x: 1.5, y: 6.5 };
            let mut enemy = catalog.units[&id("harvester")].clone();
            enemy.id = id("private_worker");
            enemy.name = "Private worker".into();
            enemy.vision = Some(VisionDef { radius: 6.0 });
            catalog.units.insert(enemy.id.clone(), enemy);
            map.placements.push(Placement {
                unit: id("private_worker").to_string(),
                owner_slot: 1,
                pos: WorldPos { x: 6.5, y: 4.5 },
                contents: vec![ItemStack {
                    item: id("ore").to_string(),
                    amount: 160,
                }],
                facing: None,
                start_order: None,
            });
        });
        crate::visibility::initialize_context(fixture.app.world_mut(), 77, &[0, 1]).unwrap();
        fixture
    };
    let mut flat = create();
    let mut ridge = create();
    let enemy = ridge.unit_at(6.5, 4.5);
    ridge.command_slot(
        SlotId(1),
        vec![enemy],
        Behaviour::TerrainWork,
        OrderTargetSim::TerrainWork {
            tool: id("fill"),
            cells: TerrainRect {
                min: TerrainCell { x: 3, y: 0 },
                max: TerrainCell { x: 4, y: 8 },
            },
            target_elevation: 40,
        },
        QueueMode::Replace,
    );
    for _ in 0..8 {
        flat.tick();
        ridge.tick();
    }
    assert_eq!(flat.height(3, 4), 20);
    assert_eq!(ridge.height(3, 4), 40);
    for fixture in [&mut flat, &mut ridge] {
        fixture.command(
            Behaviour::Move,
            OrderTargetSim::Point(crate::Vec2::new(5.5, 4.5)),
            QueueMode::Replace,
        );
    }
    let owned = |fixture: &Fixture| {
        let entity = crate::unit_entity(fixture.app.world(), fixture.worker).unwrap();
        (
            fixture
                .app
                .world()
                .get::<crate::Position>(entity)
                .unwrap()
                .0,
            fixture
                .app
                .world()
                .get::<crate::MoveTarget>(entity)
                .map(|target| target.0),
            fixture
                .app
                .world()
                .get::<navigation::TerrainMotionWait>(entity)
                .is_some(),
        )
    };
    for tick in 1..=6 {
        flat.tick();
        ridge.tick();
        assert_eq!(
            owned(&flat),
            owned(&ridge),
            "unknown ridge must not change an early route or wait before contact, tick{tick}"
        );
        let left = phase(flat.app.world(), ObservationPhase::Completed).unwrap();
        let right = phase(ridge.app.world(), ObservationPhase::Completed).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(
                    left.surface(0, TerrainCell { x, y }),
                    right.surface(0, TerrainCell { x, y })
                );
            }
        }
    }
    for _ in 0..40 {
        flat.tick();
        ridge.tick();
    }
    assert!(owned(&flat).0.x > 3.5);
    assert!(
        owned(&ridge).0.x <= 2.75,
        "physical certification prevents crossing the unseen cliff"
    );
    assert!(
        phase(ridge.app.world(), ObservationPhase::Completed)
            .unwrap()
            .surface(0, TerrainCell { x: 3, y: 4 })
            .is_none(),
        "contact does not grant a neighboring cell's height/material"
    );
}

#[test]
fn append_unload_preserves_nonzero_work_and_fraction_then_resumes_exactly_once() {
    let mut fixture = Fixture::configured(401, 5, 1000, |_, map| {
        map.placements[1].pos = WorldPos { x: 5.5, y: 4.5 };
    });
    fixture.work("dig", rect(3, 1, 2), 990, QueueMode::Replace);
    fixture.tick();
    let before = fixture
        .app
        .world()
        .resource::<work::TerrainWorkRuntime>()
        .units[&fixture.worker]
        .clone();
    assert!(before.done > 0 && before.carry > 0);
    assert_eq!(fixture.ore(fixture.worker), 5);
    let original = fixture
        .app
        .world()
        .get::<crate::orders::UnitOrders>(
            crate::unit_entity(fixture.app.world(), fixture.worker).unwrap(),
        )
        .unwrap()
        .current
        .clone();
    fixture.command(
        Behaviour::UnloadCargo,
        OrderTargetSim::Unit(fixture.depot),
        QueueMode::Append,
    );
    let mut suspended_ticks = 0;
    for _ in 0..150 {
        fixture.tick();
        if pause::is_suspended(fixture.app.world(), fixture.worker) {
            suspended_ticks += 1;
            let actual = &fixture
                .app
                .world()
                .resource::<work::TerrainWorkRuntime>()
                .units[&fixture.worker];
            assert_eq!(
                actual, &before,
                "utility must not earn, lose or replace work/carry"
            );
            let orders = fixture
                .app
                .world()
                .get::<crate::orders::UnitOrders>(
                    crate::unit_entity(fixture.app.world(), fixture.worker).unwrap(),
                )
                .unwrap();
            assert_eq!(orders.queue.front(), original.as_ref());
            assert_eq!(orders.queue.len(), 1);
        }
        if fixture.height(3, 1) == 990 {
            break;
        }
    }
    assert!(suspended_ticks > 0);
    assert_eq!(fixture.height(3, 1), 990);
    assert_eq!(fixture.ore(fixture.depot), 5);
    assert_eq!(fixture.ore(fixture.worker), 5);
    assert!(!pause::has_record(fixture.app.world(), fixture.worker));
    assert_eq!(
        fixture.height(4, 1),
        1000,
        "resuming must finish the original cell before the now-nearer cell"
    );
    fixture.command(Behaviour::Stop, OrderTargetSim::None, QueueMode::Replace);
    fixture.tick();
    assert!(
        !fixture
            .app
            .world()
            .resource::<work::TerrainWorkRuntime>()
            .units
            .contains_key(&fixture.worker)
    );
}

#[test]
fn active_and_suspended_work_cancel_on_ownership_loss_and_explicit_stop_or_replace() {
    for suspended in [false, true] {
        for action in 0..3 {
            let mut fixture = Fixture::new(401, 5, 1000);
            fixture.work("dig", rect(3, 1, 1), 990, QueueMode::Replace);
            fixture.tick();
            if suspended {
                fixture.command(
                    Behaviour::UnloadCargo,
                    OrderTargetSim::Unit(fixture.depot),
                    QueueMode::Append,
                );
                fixture.tick();
                assert!(pause::is_suspended(fixture.app.world(), fixture.worker));
            }
            let height = fixture.height(3, 1);
            let entity = crate::unit_entity(fixture.app.world(), fixture.worker).unwrap();
            match action {
                0 => {
                    fixture
                        .app
                        .world_mut()
                        .entity_mut(entity)
                        .insert(crate::Owner(SlotId(1)));
                }
                1 => fixture.command(Behaviour::Stop, OrderTargetSim::None, QueueMode::Replace),
                _ => fixture.command(
                    Behaviour::UnloadCargo,
                    OrderTargetSim::Unit(fixture.depot),
                    QueueMode::Replace,
                ),
            }
            fixture.tick();
            assert!(!pause::has_record(fixture.app.world(), fixture.worker));
            assert!(
                !fixture
                    .app
                    .world()
                    .resource::<work::TerrainWorkRuntime>()
                    .units
                    .contains_key(&fixture.worker)
            );
            for _ in 0..5 {
                fixture.tick();
            }
            assert_eq!(
                fixture.height(3, 1),
                height,
                "old authority or canceled work must never resume"
            );
        }
    }
}

#[test]
fn real_allocator_inversion_keeps_collision_safe_terrain_routes_and_bounded_retry() {
    fn scene(reverse: bool) -> (Fixture, [u32; 2]) {
        let fixture = Fixture::configured(20, 100, 20, |catalog, map| {
            wall(catalog, map, true);
            let mut follower = map.placements[0].clone();
            follower.pos = WorldPos { x: 0.5, y: 4.5 };
            map.placements.push(follower);
            if reverse {
                map.placements.reverse();
            }
        });
        let actors = [fixture.unit_at(1.5, 4.5), fixture.unit_at(0.5, 4.5)];
        (fixture, actors)
    }
    let (mut forward, left) = scene(false);
    let (mut reverse, right) = scene(true);
    assert_ne!(
        left, right,
        "the ordinary allocator must assign different IDs"
    );
    for (fixture, actors) in [(&mut forward, left), (&mut reverse, right)] {
        fixture.command_units(
            actors.into(),
            Behaviour::Move,
            OrderTargetSim::Point(crate::Vec2::new(5.5, 4.5)),
            QueueMode::Replace,
        );
    }
    let positions = |fixture: &Fixture, actors: [u32; 2]| {
        actors.map(|unit| {
            fixture
                .app
                .world()
                .get::<crate::Position>(crate::unit_entity(fixture.app.world(), unit).unwrap())
                .unwrap()
                .0
        })
    };
    for _ in 0..40 {
        let before = privileged_navigation_diagnostics(forward.app.world());
        forward.tick();
        reverse.tick();
        let after = privileged_navigation_diagnostics(forward.app.world());
        assert!(after.solver_retries - before.solver_retries <= 8);
        let physical = positions(&forward, left);
        assert_eq!(
            physical,
            positions(&reverse, right),
            "allocation order must not change physical routing or collision results"
        );
        assert!(
            physical[0].distance(physical[1]) >= 0.5 - 1e-5,
            "terrain certification must never roll a leader back into its follower"
        );
    }
    assert!(
        positions(&forward, left).iter().any(|point| point.x > 4.5),
        "a real worker must cross the permitted gap"
    );
}

#[test]
fn zero_health_cancels_active_and_paused_work_inside_the_health_transaction() {
    for suspended in [false, true] {
        let mut fixture = Fixture::new(401, 5, 1000);
        fixture.work("dig", rect(3, 1, 1), 990, QueueMode::Replace);
        fixture.tick();
        if suspended {
            fixture.command(
                Behaviour::UnloadCargo,
                OrderTargetSim::Unit(fixture.depot),
                QueueMode::Append,
            );
            fixture.tick();
        }
        let height = fixture.height(3, 1);
        let unit = fixture.worker;
        crate::economy::transaction::transact(fixture.app.world_mut(), |tx| {
            tx.stage_health(unit, 0);
            Ok(())
        })
        .unwrap();
        assert!(!pause::has_record(fixture.app.world(), unit));
        assert!(
            !fixture
                .app
                .world()
                .resource::<work::TerrainWorkRuntime>()
                .units
                .contains_key(&unit)
        );
        let orders = fixture
            .app
            .world()
            .get::<crate::orders::UnitOrders>(
                crate::unit_entity(fixture.app.world(), unit).unwrap(),
            )
            .unwrap();
        assert!(orders.current.is_none() && orders.queue.is_empty());
        for _ in 0..3 {
            fixture.tick();
        }
        assert_eq!(fixture.height(3, 1), height);
    }
}

fn output_cliff(catalog: &mut ContentCatalog, map: &mut MapDef) {
    let terrain = map.terrain.as_mut().unwrap();
    terrain.initial_heights = vec![TerrainHeightRect {
        cells: TerrainRect {
            min: TerrainCell { x: 3, y: 0 },
            max: TerrainCell { x: 4, y: 8 },
        },
        elevation: 40,
    }];
    let TerrainGenerationDef::Version1 { bands } = &mut terrain.generation;
    bands[0].top = 40;
    let recipe = catalog.production.get_mut(&id("build_harvester")).unwrap();
    recipe.duration_ms = 50;
    recipe.costs.clear();
    catalog
        .units
        .get_mut(&id("command_center"))
        .unwrap()
        .producer
        .as_mut()
        .unwrap()
        .spawn_offsets = vec![WorldPos { x: 1.5, y: 0.0 }, WorldPos { x: 0.0, y: 1.0 }];
}

#[test]
fn production_skips_a_cliff_straddling_exit_and_commits_the_next_supported_exit() {
    let mut fixture = Fixture::configured(20, 100, 20, output_cliff);
    crate::economy::production::queue_production(
        fixture.app.world_mut(),
        SlotId(0),
        fixture.depot,
        id("build_harvester"),
    )
    .unwrap();
    for _ in 0..16 {
        fixture.tick();
        if fixture
            .app
            .world()
            .iter_entities()
            .filter(|entity| entity.contains::<crate::UnitId>())
            .count()
            > 2
        {
            break;
        }
    }
    let outputs = fixture
        .app
        .world()
        .iter_entities()
        .filter_map(|entity| {
            let unit = entity.get::<crate::UnitId>()?.0;
            (unit != fixture.worker && unit != fixture.depot)
                .then(|| entity.get::<crate::Position>().unwrap().0)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        outputs,
        vec![crate::Vec2::new(1.5, 5.5)],
        "the first collision-free candidate straddles a cliff; choose the next genuinely supported exit"
    );
}

#[test]
fn construction_rejects_cliff_straddling_foundation_before_charge_and_allocator_change() {
    let mut fixture = Fixture::configured(20, 100, 20, |catalog, map| {
        output_cliff(catalog, map);
        let mut building = catalog.units[&id("command_center")].clone();
        building.id = id("terrain_building");
        building.producer = None;
        building.inventory = None;
        building.drop_off = None;
        building.placement = Some(SitePlacementDef {
            gap: 0.0,
            node_clearance: 0.0,
            in_build_area: false,
            site_health: Ratio {
                numerator: 1,
                denominator: 2,
            },
        });
        building.health = Some(HealthDef { max: 100 });
        catalog.units.insert(building.id.clone(), building);
        let mut recipe = catalog.production[&id("build_harvester")].clone();
        recipe.id = id("build_terrain_building");
        recipe.output_unit = id("terrain_building");
        recipe.duration_ms = 1000;
        recipe.costs = vec![ResolvedCost {
            resource: id("credits"),
            amount: 1,
            source: CostSource::SlotBalance,
        }];
        catalog.production.insert(recipe.id.clone(), recipe);
        catalog.units.get_mut(&id("harvester")).unwrap().constructor = Some(ResolvedConstructor {
            buildings: vec![id("build_terrain_building")],
            concurrent_sites: 1,
            queue_capacity: 2,
            progress: ConstructProgress::Autonomous,
            when_short: vec![],
        });
    });
    fixture
        .app
        .insert_resource(crate::economy::construction::ConstructionLimit(2));
    let before = crate::audit_state(fixture.app.world_mut());
    assert_eq!(
        crate::economy::construction::construct(
            fixture.app.world_mut(),
            SlotId(0),
            fixture.worker,
            id("build_terrain_building"),
            crate::Vec2::new(3.0, 3.0)
        ),
        Err(crate::RejectReason::SiteBlocked)
    );
    assert_eq!(
        before,
        crate::audit_state(fixture.app.world_mut()),
        "refused support must not charge, allocate, reserve or create a site"
    );
    crate::economy::construction::construct(
        fixture.app.world_mut(),
        SlotId(0),
        fixture.worker,
        id("build_terrain_building"),
        crate::Vec2::new(5.5, 3.0),
    )
    .unwrap();
    assert_eq!(
        fixture
            .app
            .world()
            .resource::<crate::economy::construction::ConstructionState>()
            .sites
            .len(),
        1
    );
}
