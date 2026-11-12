//! The ECS system that runs the movement frame.
//!
//! The movement system may write only these fields (pre-MP1 slice, A1c repair; the replay
//! validator checked this for each movement phase, and the ledger audit has no movement model):
//! - the components `Position`, `MoveTarget`, `MoveProgress`, `PlainMoveProgress` (until A2b2)
//!   and `PhysicalEscape`, and the resource `MovementPools`.
//!
//! It reads `UnitOrders` (for the resting set only), `UnitType`, `Owner`, `UnitId`, `MapBounds`,
//! `SimClock` and `Time<Fixed>`. It writes nothing else: no order state, no owner, no map
//! bound, no inventory, no clock. The test `the_movement_system_writes_only_movement_state`
//! calls the system on the installed world and between ticks of a real scenario and compares the
//! full audit state, so a write to any other audited field is red. Known gap: a write of a
//! constant that already holds at every snapshot after tick 0 is not seen; the outcome tests of
//! the orders and of the group movement hold those values. The writer scan (`turts_checks/tests/
//! movement_writers.rs`) pins the writes of the movement files by count.

use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use bevy_math::Vec2;
use bevy_time::{Fixed, Time};

use super::frame::{MovementFrame, step_frame_preserving_resting};
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
    let outputs = step_frame_preserving_resting(&frame, delta, &preserve_resting_units);
    #[cfg(feature = "test-probes")]
    super::probe::store_frame_flags(world, &outputs);
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
        state
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
        let mut changed_movement_state = 0;
        // The first call runs on the world as the package installed it, before any update. A
        // write of a constant (for example an owner) that the movement system makes in each
        // call is then seen as a change; between two ticks it would already be in the state.
        let installed = without_movement_state(app.world_mut());
        integrate_movement(app.world_mut());
        assert_eq!(
            installed,
            without_movement_state(app.world_mut()),
            "tick 0: the movement system changed an audited field that is not movement state"
        );
        for tick in 1..=130_u32 {
            for (at, command) in &script {
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
            integrate_movement(app.world_mut());
            let after_full = serde_json::to_value(crate::audit_state(app.world_mut())).unwrap();
            let after = without_movement_state(app.world_mut());
            assert_eq!(
                before, after,
                "tick {tick}: the movement system changed an audited field that is not movement state"
            );
            changed_movement_state += usize::from(before_full != after_full);
        }
        assert!(
            changed_movement_state > 5,
            "the scenario moves units, so the movement state changes: {changed_movement_state}"
        );
    }
}
