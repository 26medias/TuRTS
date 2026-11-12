//! The ECS system that runs the movement frame.
//!
//! The movement system may write only these fields (pre-MP1 slice, A1c repair; the replay
//! validator checked this for each movement phase, and the ledger audit has no movement model):
//! - the components `Position`, `MoveTarget`, `MoveProgress`, `PlainMoveProgress` (until A2b2)
//!   and `PhysicalEscape`, and the resource `MovementPools`;
//! - `PendingMovementOrders::next_pool_id`, the id counter of the pools, one step for each
//!   fallback pool of one that it adds to `MovementPools` (task A2b1; `fallback.rs`).
//!
//! It reads `UnitOrders` (for the resting set only), `UnitType`, `Owner`, `UnitId`, `MapBounds`,
//! `SimClock` and `Time<Fixed>`. It writes nothing else: no order state, no owner, no map
//! bound, no inventory, no clock. The test `the_movement_system_writes_only_movement_state`
//! calls the system on the installed world and between ticks of two real scenarios (the Muster
//! field, and a blocked plain move that ends in a physical escape) and compares the full audit
//! state, so a write to any other audited field is red. The test also counts each arm of the
//! write loop (insert and remove of each movement component, and the fallback pools) and fails if
//! one arm never runs (the A1c re-check: plant S2 in the escape arm was green with the Muster
//! scenario alone). Since task A2b1 no code makes a plain progress, so its insert arm is dead
//! code until task A2b2 deletes it; the test requires that it never runs.
//! Known gap: a write of a
//! constant that already holds at every snapshot after tick 0 is not seen; the outcome tests of
//! the orders and of the group movement hold those values. The writer scan (`turts_checks/tests/
//! movement_writers.rs`) pins the writes of the movement files by count.

use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use bevy_math::Vec2;
use bevy_time::{Fixed, Time};

use super::fallback::assign_fallback_pools;
use super::frame::{MovementFrame, MovementStep, step_frame_with_fallbacks};
use super::plain::PlainMoveProgress;
use super::{MoveProgress, MovementPools, PhysicalEscape};
use crate::registry::UnitTypes;
use crate::world::{MoveTarget, Position, UnitType};

/// FixedUpdate: move each unit with a `MoveTarget` by `speed * delta` toward its target.
/// A unit that is within one step of its target lands on it and loses the `MoveTarget`.
pub(crate) fn integrate_movement(world: &mut World) {
    let delta = world.resource::<Time<Fixed>>().delta_secs();
    let types = world.resource::<UnitTypes>();
    let mut units: Vec<_> = world
        .iter_entities()
        .filter_map(|entity| {
            let unit_type = entity.get::<UnitType>()?;
            let position = entity.get::<Position>()?;
            let entry = types.get(unit_type.0)?;
            Some((
                entity
                    .get::<crate::world::UnitId>()
                    .map_or(u32::MAX, |id| id.0),
                entity.id(),
                crate::movement::MovementFrameUnit {
                    unit_id: entity
                        .get::<crate::world::UnitId>()
                        .map_or(u32::MAX, |id| id.0),
                    owner_slot: entity
                        .get::<crate::world::Owner>()
                        .map_or(0, |owner| owner.0.0),
                    unit_type: unit_type.0.0,
                    position: position.0,
                    radius: entry.radius,
                    speed: entry.movement.as_ref().map(|movement| movement.speed),
                    target: entity.get::<MoveTarget>().map(|target| target.0),
                    progress: entity.get::<MoveProgress>().cloned(),
                    plain_progress: entity.get::<PlainMoveProgress>().cloned(),
                    physical_escape: entity.get::<PhysicalEscape>().cloned(),
                },
            ))
        })
        .collect();
    units.sort_unstable_by_key(|(id, ..)| *id);
    let bounds = world
        .get_resource::<crate::world::MapBounds>()
        .map_or(Vec2::ZERO, |bounds| bounds.size);
    let tick_hz = world
        .get_resource::<crate::SimClock>()
        .map_or(20, |clock| clock.tick_hz);
    let frame = MovementFrame {
        bounds,
        tick_hz,
        units: units.iter().map(|(_, _, unit)| unit.clone()).collect(),
        pools: world.resource::<MovementPools>().0.clone(),
    };
    let preserve_resting_units = units
        .iter()
        .filter_map(|(unit_id, entity, _)| {
            world
                .entity(*entity)
                .get::<crate::orders::UnitOrders>()
                .and_then(|orders| orders.current.as_ref())
                .is_some_and(|order| {
                    order.behaviour == turts_mod_api::Behaviour::Attack
                        && matches!(order.target, crate::orders::OrderTargetSim::Point(_))
                })
                .then_some(*unit_id)
        })
        .collect();
    let MovementStep {
        mut outputs,
        fallbacks,
    } = step_frame_with_fallbacks(&frame, delta, &preserve_resting_units);
    #[cfg(feature = "test-probes")]
    let fallback_units: Vec<u32> = fallbacks.iter().map(|request| request.unit_id).collect();
    // The fallback pools of one take their ids from the counter of the order pools, in unit-id
    // order (task A2b1; `fallback.rs`). The resource is borrowed only when a fallback exists,
    // so that a frame with no fallback leaves the change tick of the counter as it was.
    let fallback_pools = if fallbacks.is_empty() {
        Vec::new()
    } else {
        if !world.contains_resource::<crate::orders::PendingMovementOrders>() {
            world.init_resource::<crate::orders::PendingMovementOrders>();
        }
        let mut pending = world.resource_mut::<crate::orders::PendingMovementOrders>();
        assign_fallback_pools(&mut outputs, fallbacks, &mut pending.next_pool_id)
    };
    #[cfg(feature = "test-probes")]
    super::probe::store_frame_flags(world, &outputs);
    #[cfg(feature = "test-probes")]
    {
        let inputs: Vec<_> = units
            .iter()
            .map(|(unit_id, _, unit)| (*unit_id, unit.target))
            .collect();
        super::probe::record_fallback_trips(world, &inputs, &outputs, &fallback_units);
    }
    for ((_, entity, _), output) in units.into_iter().zip(outputs) {
        let mut entity_mut = world.entity_mut(entity);
        if let Some(target) = output.target {
            entity_mut.insert(MoveTarget(target));
        } else {
            entity_mut.remove::<MoveTarget>();
        }
        if let Some(mut position) = entity_mut.get_mut::<Position>() {
            position.set_if_neq(Position(output.position));
        }
        if let Some(progress) = output.progress {
            entity_mut.insert(progress);
        } else {
            entity_mut.remove::<MoveProgress>();
        }
        if let Some(progress) = output.plain_progress {
            entity_mut.insert(progress);
        } else {
            entity_mut.remove::<PlainMoveProgress>();
        }
        if let Some(escape) = output.physical_escape {
            entity_mut.insert(escape);
        } else {
            entity_mut.remove::<PhysicalEscape>();
        }
    }
    if !fallback_pools.is_empty() {
        let mut pools = world.resource_mut::<MovementPools>();
        for pool in fallback_pools {
            pools.0.insert(pool.pool_id, pool);
        }
    }
    let active_pools: std::collections::BTreeSet<_> = world
        .iter_entities()
        .filter_map(|entity| {
            entity
                .get::<MoveProgress>()
                .map(|progress| progress.pool_id)
        })
        .collect();
    world
        .resource_mut::<MovementPools>()
        .0
        .retain(|pool_id, _| active_pools.contains(pool_id));
}

#[cfg(test)]
mod tests {
    use super::integrate_movement;
    use crate::{
        Command, CommandKind, CommandQueue, OrderCommand, OrderTargetSim, QueueMode, SlotId, Vec2,
    };
    use turts_mod_api::{ContentId, OrderDef, PackageId, SourceDef};
    use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

    /// The `AuditState` fields that the movement system may change (see the module doc).
    const MOVEMENT_FIELDS: [&str; 3] = [
        "movement_progress",
        "plain_movement_progress",
        "physical_escapes",
    ];

    /// The audit state as JSON, with the fields that the movement system may write removed: the
    /// pools, the progress rows, the unit positions and the unit targets.
    fn without_movement_state(world: &mut bevy_ecs::world::World) -> serde_json::Value {
        let mut state = serde_json::to_value(crate::audit_state(world)).unwrap();
        let object = state.as_object_mut().unwrap();
        for field in MOVEMENT_FIELDS {
            object.remove(field);
        }
        object.remove("movement_pools");
        for unit in object["raw_units"].as_array_mut().unwrap() {
            let unit = unit.as_object_mut().unwrap();
            unit.remove("pos");
            unit.remove("move_target");
        }
        for unit in object["units"].as_array_mut().unwrap() {
            unit.as_object_mut().unwrap().remove("pos");
        }
        for unit in object["economy"]["units"].as_array_mut().unwrap() {
            unit.as_object_mut().unwrap().remove("pos");
        }
        // The pool id counter: `call` checks it apart (task A2b1). The audit state leaves out an
        // empty `pending_movement_orders` (a counter of 0 is part of "empty"), so the default
        // value stands in for it, and the counter is removed from both.
        let mut pending = object.remove("pending_movement_orders").unwrap_or_else(|| {
            serde_json::to_value(crate::orders::PendingMovementOrders::default()).unwrap()
        });
        pending.as_object_mut().unwrap().remove("next_pool_id");
        object.insert("pending_movement_orders".into(), pending);
        state
    }

    /// Call the movement system one time. The pool id counter may grow only by the fallback
    /// pools of one that the call adds: each new pool has a new id above the old counter, and the
    /// unit that holds it has a fallback progress. Returns the count of those pools.
    fn call(world: &mut bevy_ecs::world::World, label: &str, tick: u32) -> usize {
        let next_before = world
            .resource::<crate::orders::PendingMovementOrders>()
            .next_pool_id;
        integrate_movement(world);
        let next_after = world
            .resource::<crate::orders::PendingMovementOrders>()
            .next_pool_id;
        let new_pools: Vec<u64> = world
            .resource::<crate::movement::MovementPools>()
            .0
            .keys()
            .copied()
            .filter(|pool_id| *pool_id > next_before)
            .collect();
        let fallback_pools = world
            .iter_entities()
            .filter_map(|entity| entity.get::<crate::movement::MoveProgress>())
            .filter(|progress| progress.is_fallback() && progress.pool_id > next_before)
            .count();
        assert_eq!(
            (next_after - next_before) as usize,
            new_pools.len(),
            "{label} tick {tick}: the pool id counter moved by more than the new pools {new_pools:?}"
        );
        assert_eq!(
            fallback_pools,
            new_pools.len(),
            "{label} tick {tick}: a new pool {new_pools:?} is not the fallback pool of a unit"
        );
        new_pools.len()
    }

    /// How often each arm of the write loop of `integrate_movement` ran: for each movement
    /// component, the arm that inserts it and the arm that removes it. Each unit row of a call
    /// runs exactly one arm of each pair, and after the call the unit holds the component exactly
    /// when the insert arm ran, so the counts come from the components after each call.
    #[derive(Debug, Default)]
    struct WriteArms {
        /// The fallback pools of one that a call added (task A2b1).
        fallback_pools: usize,
        target_set: usize,
        target_removed: usize,
        progress_set: usize,
        progress_removed: usize,
        plain_set: usize,
        plain_removed: usize,
        escape_set: usize,
        escape_removed: usize,
    }

    impl WriteArms {
        /// Count the arms of the call that has just ended.
        fn count(&mut self, world: &mut bevy_ecs::world::World) {
            for entity in world.iter_entities() {
                if entity.get::<crate::world::UnitType>().is_none()
                    || entity.get::<crate::world::Position>().is_none()
                {
                    continue;
                }
                let pairs = [
                    (
                        entity.contains::<crate::world::MoveTarget>(),
                        &mut self.target_set,
                        &mut self.target_removed,
                    ),
                    (
                        entity.contains::<crate::movement::MoveProgress>(),
                        &mut self.progress_set,
                        &mut self.progress_removed,
                    ),
                    (
                        entity.contains::<crate::movement::PlainMoveProgress>(),
                        &mut self.plain_set,
                        &mut self.plain_removed,
                    ),
                    (
                        entity.contains::<crate::movement::PhysicalEscape>(),
                        &mut self.escape_set,
                        &mut self.escape_removed,
                    ),
                ];
                for (held, set, removed) in pairs {
                    if held {
                        *set += 1;
                    } else {
                        *removed += 1;
                    }
                }
            }
        }

        /// The arms that must run. The insert arm of the plain progress is not in the list: no
        /// code makes a plain progress since task A2b1 (see `the_movement_system_writes_only_
        /// movement_state`).
        fn arms(&self) -> [(&'static str, usize); 8] {
            [
                ("fallback pool added", self.fallback_pools),
                ("target set", self.target_set),
                ("target removed", self.target_removed),
                ("progress set", self.progress_set),
                ("progress removed", self.progress_removed),
                ("plain progress removed", self.plain_removed),
                ("escape set", self.escape_set),
                ("escape removed", self.escape_removed),
            ]
        }
    }

    /// Run a scenario and call the movement system once more after each tick. Each extra call
    /// must leave the audit state unchanged, except the movement state. Returns the count of
    /// calls that changed the movement state.
    fn sweep(
        app: &mut bevy_app::App,
        label: &str,
        script: &[(u32, Command)],
        ticks: u32,
        arms: &mut WriteArms,
    ) -> usize {
        let mut changed_movement_state = 0;
        // The first call runs on the world as the package installed it, before any update. A
        // write of a constant (for example an owner) that the movement system makes in each
        // call is then seen as a change; between two ticks it would already be in the state.
        let installed = without_movement_state(app.world_mut());
        arms.fallback_pools += call(app.world_mut(), label, 0);
        arms.count(app.world_mut());
        assert_eq!(
            installed,
            without_movement_state(app.world_mut()),
            "{label} tick 0: the movement system changed an audited field that is not movement \
             state"
        );
        for tick in 1..=ticks {
            for (at, command) in script {
                if *at == tick {
                    app.world_mut()
                        .resource_mut::<CommandQueue>()
                        .0
                        .push_back(command.clone());
                }
            }
            app.update();
            let before_full = serde_json::to_value(crate::audit_state(app.world_mut())).unwrap();
            let before = without_movement_state(app.world_mut());
            arms.fallback_pools += call(app.world_mut(), label, tick);
            arms.count(app.world_mut());
            let after_full = serde_json::to_value(crate::audit_state(app.world_mut())).unwrap();
            let after = without_movement_state(app.world_mut());
            assert_eq!(
                before, after,
                "{label} tick {tick}: the movement system changed an audited field that is not \
                 movement state"
            );
            changed_movement_state += usize::from(before_full != after_full);
        }
        changed_movement_state
    }

    /// The blocked plain move of `group_movement.rs` (`full_width_post_app`, the test at `:2156`):
    /// a 7 x 5 map, a mover of radius 0.5 at (0.5, 2.5), a static post of radius 2 at (3.5, 2.5),
    /// and a one-unit move to (6.5, 2.5). The post leaves a gap of 0.5 to each map edge, so the
    /// move fails as `Blocked` while the body overlaps the post: the physical escape arm of the
    /// write loop runs (the A1c re-check: the Muster scenario alone makes no escape).
    fn blocked_post_app() -> (bevy_app::App, Vec<(u32, Command)>) {
        use turts_mod_api::{
            Footprint, MapDef, MapSize, MovementDef, Placement, UnitDef, UnitRegistry, WorldPos,
        };
        let unit = |id: &str, movement: Option<MovementDef>, radius: f32| UnitDef {
            id: id.into(),
            name: id.into(),
            movement,
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: None,
            footprint: Footprint::Circle { radius },
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: None,
            weapon: None,
            removal: None,
        };
        let mut app = crate::headless_app(20);
        let origin = PackageId("fixture_pkg".to_owned());
        app.world_mut()
            .resource_mut::<crate::UnitTypes>()
            .register_unit_type(
                &origin,
                unit("walker", Some(MovementDef { speed: 3.0 }), 0.5),
            )
            .unwrap();
        let post_type = app
            .world_mut()
            .resource_mut::<crate::UnitTypes>()
            .register_unit_type(&origin, unit("wide-post", None, 2.0))
            .unwrap();
        let map = MapDef {
            id: "field".into(),
            size: MapSize { x: 7.0, y: 5.0 },
            placements: vec![Placement {
                unit: "fixture_pkg:walker".into(),
                owner_slot: 0,
                pos: WorldPos { x: 0.5, y: 2.5 },
                contents: Vec::new(),
                facing: None,
            }],
            nodes: Vec::new(),
            slot_economy: Vec::new(),
            belts: Vec::new(),
        };
        crate::load_map(app.world_mut(), &origin, &map).unwrap();
        app.world_mut().spawn((
            crate::world::UnitId(2),
            crate::world::UnitType(post_type),
            crate::world::Owner(SlotId(0)),
            crate::world::Position(Vec2::new(3.5, 2.5)),
        ));
        let script = vec![(
            1,
            Command {
                slot: SlotId(0),
                seq: 1,
                kind: CommandKind::Order(Box::new(OrderCommand {
                    order: ContentId::parse("fixture_pkg:move").unwrap(),
                    behaviour: turts_mod_api::Behaviour::Move,
                    target_kind: turts_mod_api::TargetKind::PointOrUnit,
                    policy: None,
                    units: vec![1],
                    target: OrderTargetSim::Point(Vec2::new(6.5, 2.5)),
                    queue: QueueMode::Replace,
                    group: 0,
                    allowed_units: Vec::new(),
                    queue_depth: 16,
                })),
            },
        )];
        (app, script)
    }

    #[test]
    fn the_movement_system_writes_only_movement_state() {
        let package_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
        let archive = PackageArchive::parse(&pack_dir(&package_dir).expect("Muster packs"))
            .expect("Muster archive parses");
        let catalog = archive.content().expect("Muster catalog resolves");
        let (map_file, map) = archive
            .maps()
            .expect("Muster maps parse")
            .into_iter()
            .find(|(_, map)| map.id == "field")
            .expect("Muster field map exists");
        let mut app = crate::headless_app(20);
        let staged = crate::stage_package(
            app.world(),
            catalog.clone(),
            SourceDef {
                file: map_file,
                value: map,
            },
            20,
        )
        .expect("the authored Muster map stages");
        crate::install_package(app.world_mut(), staged);
        let order = |local: &str, units: Vec<u32>, group: u32| {
            let (id, definition): (ContentId, &OrderDef) = catalog
                .orders
                .iter()
                .find(|(id, _)| id.package.0 == "muster" && id.local == local)
                .map(|(id, definition)| (id.clone(), definition))
                .expect("the Muster order exists");
            Command {
                slot: SlotId(0),
                seq: group,
                kind: CommandKind::Order(Box::new(OrderCommand {
                    order: id,
                    behaviour: definition.behaviour,
                    target_kind: definition.target,
                    policy: None,
                    units,
                    target: OrderTargetSim::Point(Vec2::new(40.0, 30.0)),
                    queue: QueueMode::Replace,
                    group,
                    allowed_units: definition.units.clone(),
                    queue_depth: catalog.order_queue_depths[&PackageId("muster".into())],
                })),
            }
        };
        // A point attack, a move and a legacy single move run together.
        let script = vec![
            (1, order("rush", (21..=24).collect(), 1)),
            (1, order("order_move", (1..=4).collect(), 2)),
            (
                3,
                Command {
                    slot: SlotId(0),
                    seq: 3,
                    kind: CommandKind::MoveTo {
                        unit_ids: vec![41],
                        target: Vec2::new(34.0, 26.0),
                    },
                },
            ),
        ];
        let mut arms = WriteArms::default();
        let changed_movement_state = sweep(&mut app, "muster", &script, 130, &mut arms);
        assert!(
            changed_movement_state > 5,
            "the scenario moves units, so the movement state changes: {changed_movement_state}"
        );
        // A blocked plain move: the only scenario that reaches the physical escape arm.
        let (mut blocked, script) = blocked_post_app();
        let changed_movement_state = sweep(&mut blocked, "blocked post", &script, 200, &mut arms);
        assert!(
            changed_movement_state > 5,
            "the blocked move moves its unit, so the movement state changes: \
             {changed_movement_state}"
        );
        // Each arm of the write loop must run, else a write in an arm that the scenarios never
        // reach stays green (the A1c re-check, plant S2 in the escape arm).
        for (arm, count) in arms.arms() {
            assert!(
                count > 0,
                "the arm {arm:?} of the write loop never ran: {arms:?}"
            );
        }
        // No code makes a plain progress since task A2b1: its insert arm never runs.
        assert_eq!(arms.plain_set, 0, "a plain progress was made: {arms:?}");
    }
}
