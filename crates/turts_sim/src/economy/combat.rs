//! Weapon and health state, plus the fixed-tick combat phase.

use std::collections::BTreeMap;

use bevy_app::{App, FixedUpdate};
use bevy_ecs::{prelude::Resource, schedule::IntoScheduleConfigs, world::World};
use turts_mod_api::{ContentId, SlotRelation, WeaponStat};

use super::{
    events::{EconomyEventKind, RemoveCause},
    ledger::Endpoint,
    power,
    research::{Disabled, ResearchState, modifier},
    transaction::{self, Transaction},
};
use crate::{
    Owner, RejectReason, SimClock, UnitId, UnitType, bootstrap::InstalledCatalog,
    registry::UnitTypes, world::Position,
};

/// Current weapon condition.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum WeaponStatus {
    #[default]
    Ready,
    Cooling,
    NoTarget,
    OutOfAmmo,
}

/// Cooldown and most recent targeting information for a weapon.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WeaponState {
    pub(crate) cooldown: u32,
    pub(crate) status: WeaponStatus,
    pub(crate) last_target: Option<u32>,
    pub(crate) ordered_target_fired: Option<u32>,
}

/// Weapon states indexed by unit id.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Weapons(pub(crate) BTreeMap<u32, WeaponState>);

/// Current hit points indexed by unit id.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Healths(pub(crate) BTreeMap<u32, u64>);

/// Install the Combat phase system.
pub(crate) fn install(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        run_combat
            .in_set(crate::plugin::SimSet::Combat)
            .run_if(not_stopped),
    );
}

fn not_stopped(
    stopped: Option<bevy_ecs::prelude::Res<crate::economy::events::MatchStopped>>,
) -> bool {
    stopped.is_none()
}

#[derive(Clone)]
struct WeaponParameters {
    owner: u8,
    range: f32,
    damage: u64,
    modifier: Result<u64, RejectReason>,
    cooldown_ticks: Result<u32, RejectReason>,
    ammo: Option<(ContentId, u64)>,
    targets: SlotRelation,
}

struct CombatIndex<'a> {
    grid: &'a crate::grid::UniformGrid,
    points: &'a [crate::grid::GridPoint],
    positions: &'a BTreeMap<u32, bevy_math::Vec2>,
    owners: &'a BTreeMap<u32, u8>,
}

/// Run each enabled weapon once, in ascending unit-id order.
pub(crate) fn run_combat(world: &mut World) {
    if !world.contains_resource::<Weapons>()
        || !world.contains_resource::<Healths>()
        || !world.contains_resource::<InstalledCatalog>()
        || !world.contains_resource::<UnitTypes>()
    {
        return;
    }
    let mut entities = BTreeMap::new();
    let mut points = Vec::new();
    let mut owners = BTreeMap::new();
    let mut positions = BTreeMap::new();
    for entity in world.iter_entities() {
        let (Some(id), Some(position), Some(owner)) = (
            entity.get::<UnitId>(),
            entity.get::<Position>(),
            entity.get::<Owner>(),
        ) else {
            continue;
        };
        entities.insert(id.0, entity.id());
        points.push(crate::grid::GridPoint {
            id: id.0,
            position: position.0,
        });
        owners.insert(id.0, owner.0.0);
        positions.insert(id.0, position.0);
    }
    points.sort_unstable_by_key(|point| point.id);
    let cell_size = world
        .resource::<InstalledCatalog>()
        .0
        .units
        .values()
        .filter_map(|unit| unit.weapon.as_ref().map(|weapon| weapon.range))
        .fold(f32::EPSILON, f32::max);
    let grid = crate::grid::UniformGrid::new(&points, cell_size);
    let combat_index = CombatIndex {
        grid: &grid,
        points: &points,
        positions: &positions,
        owners: &owners,
    };
    let mut weapon_ids = world
        .resource::<Weapons>()
        .0
        .keys()
        .copied()
        .collect::<Vec<_>>();
    weapon_ids.sort_unstable();
    for weapon in weapon_ids {
        if world.resource::<Disabled>().0.contains(&weapon) {
            continue;
        };
        let Some(entity) = entities.get(&weapon).copied() else {
            continue;
        };
        // Earlier weapons in this same phase may have destroyed a later
        // shooter. The phase index is deliberately built once, so validate
        // the indexed entity before reading its components.
        if world.get::<UnitId>(entity).is_none() {
            continue;
        }
        let Some(parameters) = weapon_parameters(world, world.entity(entity)) else {
            continue;
        };
        let order = world
            .entity(entity)
            .get::<crate::orders::UnitOrders>()
            .and_then(|orders| orders.current.as_ref())
            .or_else(|| {
                world
                    .get_resource::<crate::orders::PlainMoveOrderIdentities>()
                    .and_then(|orders| orders.0.get(&weapon))
            });
        let order_definition = order.and_then(|order| {
            world
                .resource::<InstalledCatalog>()
                .0
                .orders
                .get(&order.order)
                .filter(|definition| definition.behaviour == order.behaviour)
        });
        // An active authored identity is trusted only when it resolves in the
        // installed match catalog and its behavior agrees with the captured
        // order. Never infer a mutable or missing definition from the order.
        if order.is_some() && order_definition.is_none() {
            continue;
        }
        let fire_policy = order_definition.and_then(|definition| definition.fire);
        let suppressed_by_fire_policy = order.is_some_and(|order| {
            matches!(
                order.behaviour,
                turts_mod_api::Behaviour::Move | turts_mod_api::Behaviour::Hold
            )
        }) && fire_policy == Some(turts_mod_api::FirePolicy::Never);
        let stops_for_engage = order.is_some_and(|order| {
            order.behaviour == turts_mod_api::Behaviour::Attack
                && matches!(order.target, crate::orders::OrderTargetSim::Point(_))
                && order_definition.is_some_and(|definition| {
                    definition.engage == Some(turts_mod_api::EngagePolicy::StopAndFire)
                })
        });
        let suppressed_by_engage = order.is_some_and(|order| {
            order.behaviour == turts_mod_api::Behaviour::Attack
                && matches!(order.target, crate::orders::OrderTargetSim::Point(_))
        }) && !stops_for_engage;
        let origin = positions[&weapon];
        let focus = order.and_then(|order| match order.target {
            crate::orders::OrderTargetSim::Unit(target)
                if order.behaviour == turts_mod_api::Behaviour::Attack =>
            {
                Some(target)
            }
            _ => None,
        });
        let target = if suppressed_by_fire_policy
            || suppressed_by_engage
            || !power::weapon_enabled(world, weapon)
        {
            None
        } else {
            choose_target(world, origin, &parameters, &combat_index, focus)
        };
        if target.is_some()
            && stops_for_engage
            && world
                .entity(entity)
                .get::<crate::world::MoveTarget>()
                .is_some()
        {
            crate::economy::transaction::capture_post_movement_order_operation(
                world,
                weapon,
                |world| {
                    world
                        .entity_mut(entity)
                        .remove::<crate::world::MoveTarget>();
                },
            );
        }
        let fire_enabled = power::weapon_enabled(world, weapon);
        let _ = transaction::transact(world, |tx| {
            tx.mark_m3();
            let mut state = tx.weapon_state(weapon)?;
            // Attack-move needs acquisition state even when cooldown or ammo
            // blocks a shot. Keep the legacy M3 weapon history unchanged for
            // units without an authored StopAndFire point order.
            if stops_for_engage {
                state.last_target = target;
            }
            if state.cooldown > 0 {
                state.cooldown -= 1;
            }
            if state.cooldown > 0 {
                state.status = WeaponStatus::Cooling;
                tx.stage_weapon(weapon, state);
                return Ok(());
            }

            let Some(target) = target.filter(|_| fire_enabled) else {
                state.status = WeaponStatus::NoTarget;
                if focus.is_none() || state.last_target != focus {
                    state.last_target = None;
                }
                tx.stage_weapon(weapon, state);
                return Ok(());
            };
            fire(
                tx,
                weapon,
                target,
                &parameters,
                &mut state,
                focus == Some(target),
            )
        });
    }
}

fn weapon_parameters(
    world: &World,
    entity: bevy_ecs::world::EntityRef<'_>,
) -> Option<WeaponParameters> {
    let owner = entity.get::<Owner>()?.0.0;
    let unit_type = entity.get::<UnitType>()?.0;
    let type_entry = world.resource::<UnitTypes>().get(unit_type)?;
    let unit_def = ContentId::parse(&type_entry.content_id).ok()?;
    let definition = world
        .resource::<InstalledCatalog>()
        .0
        .units
        .get(&unit_def)?;
    let weapon_def = definition.weapon.as_ref()?;
    let tick_hz = u32::from(world.resource::<SimClock>().tick_hz);
    let cooldown_ticks = crate::boot_m3::try_weapon_cooldown_ticks(weapon_def.cooldown_ms, tick_hz)
        .ok_or(RejectReason::ArithmeticOverflow);
    let research = world.resource::<ResearchState>();
    let modifier = modifier(
        research,
        &world.resource::<InstalledCatalog>().0,
        owner,
        &unit_def,
        WeaponStat::Damage,
    );
    Some(WeaponParameters {
        owner,
        range: weapon_def.range,
        damage: weapon_def.damage,
        modifier,
        cooldown_ticks,
        ammo: weapon_def
            .ammo
            .as_ref()
            .map(|ammo| (ammo.item.clone(), ammo.per_shot)),
        targets: weapon_def.targets.slots,
    })
}

/// The one range rule of a weapon: `point` is in range of a weapon at `origin` when the squared
/// distance is at most the squared range. Target choice (`choose_target`) and the approach of an
/// Attack order (`weapon_stand_off`) both decide with this function, so a unit that ends its
/// approach is in range by the same test that lets its weapon fire.
pub(crate) fn within_weapon_range(
    origin: bevy_math::Vec2,
    point: bevy_math::Vec2,
    range: f32,
) -> bool {
    origin.distance_squared(point) <= range * range
}

/// The point from which a weapon of `range` can fire at `target`, on the line from `target` to
/// `from`: the ring point at distance `range` from `target`.
///
/// The ring point is a real number that `f32` cannot store, so its stored value can lie a few
/// rounding steps outside the range, and `within_weapon_range` then refuses it (a unit that
/// stopped there never fired). A ring point that passes the rule is returned unchanged. A ring
/// point that fails it is moved toward `target`, one step at a time, until the rule holds
/// (`step_toward`). The weapon range is not changed.
///
/// The loop ends, and the result is in range, by construction and not by a tuned margin:
/// - A step moves each coordinate that is not yet the coordinate of `target` toward it, and
///   never past it. So the two offsets from `target` only fall, and a coordinate that has
///   reached the coordinate of `target` stays there.
/// - The rule is monotone in the offsets: the rounded difference, the square and the sum of
///   squares do not fall when an offset grows. So the value that the rule tests only falls
///   from step to step.
/// - A step changes a coordinate by at least one quantum of `step_toward`, or at least half a
///   quantum when it crosses a binade upward (the sum rounds), or it reaches the target
///   coordinate. So after finitely many steps both offsets are 0, and a distance of 0 passes
///   the rule for every range above 0 (`0 <= range * range`, also when the square rounds to 0).
/// - The number of steps is small. The review of `mp1/combat-fixes` bounds the offset that has
///   to go by about `7.5 u r + 1.5 u M` (`u` is `2^-24`, `r` the range, `M` the largest
///   coordinate), and a step removes about `2 u M_c` in a component with a coordinate magnitude
///   `M_c`, where `M_c` is at least half the offset of that component (a half step at a binade
///   crossing removes `u M_c`). That is about 15 steps, and about 30 with half steps, at most.
///   `STAND_OFF_MAX_STEPS` is a hard bound over it, and the test
///   `weapon_stand_off_is_in_range_for_the_reviewed_case_classes` records the largest count.
///   If the bound were ever reached, the result is `target` itself: a distance of 0 passes the
///   rule, so the result is in range in every case (`stand_off_bounded`).
pub(crate) fn weapon_stand_off(
    target: bevy_math::Vec2,
    from: bevy_math::Vec2,
    range: f32,
) -> bevy_math::Vec2 {
    stand_off_steps(target, from, range).0
}

/// The hard bound of the loop of `weapon_stand_off` (about 30 steps by the derivation there).
const STAND_OFF_MAX_STEPS: u32 = 64;

/// `weapon_stand_off` and the number of steps that it took.
fn stand_off_steps(
    target: bevy_math::Vec2,
    from: bevy_math::Vec2,
    range: f32,
) -> (bevy_math::Vec2, u32) {
    let (point, steps, reached) = stand_off_bounded(target, from, range, STAND_OFF_MAX_STEPS);
    debug_assert!(
        reached,
        "the stand-off took {steps} steps and did not reach range {range} of {target:?} (from {from:?})"
    );
    (point, steps)
}

/// The walk of `weapon_stand_off` with a bound of `max_steps`. Returns the point, the number of
/// steps, and whether the walk reached the rule within the bound. When it did not, the point is
/// `target` (a distance of 0 passes the rule), so the point is in range in every case.
fn stand_off_bounded(
    target: bevy_math::Vec2,
    from: bevy_math::Vec2,
    range: f32,
    max_steps: u32,
) -> (bevy_math::Vec2, u32, bool) {
    let direction = (from - target).normalize_or_zero();
    let mut point = target + direction * range;
    let mut steps = 0;
    while steps < max_steps && !within_weapon_range(point, target, range) {
        point = bevy_math::Vec2::new(
            step_toward(point.x, target.x),
            step_toward(point.y, target.y),
        );
        steps += 1;
    }
    if within_weapon_range(point, target, range) {
        (point, steps, true)
    } else {
        (target, steps, false)
    }
}

/// One step of a coordinate toward `target`: one quantum, or the target coordinate itself when it
/// is within one quantum. The quantum is the spacing of `f32` at the larger magnitude of the two
/// coordinates, not at the magnitude of `coordinate`. Near 0 the spacing of `coordinate` is
/// finer than the rounding of the ring point that came from the (large) target coordinate, and
/// steps of that spacing take many more steps (36 in the case classes of the test, 3 with this
/// quantum). A coordinate that is already `target`
/// stays.
fn step_toward(coordinate: f32, target: f32) -> f32 {
    let magnitude = coordinate.abs().max(target.abs());
    let quantum = magnitude.next_up() - magnitude;
    let gap = target - coordinate;
    if gap.abs() <= quantum {
        target
    } else {
        coordinate + quantum.copysign(gap)
    }
}

fn choose_target(
    world: &World,
    origin: bevy_math::Vec2,
    parameters: &WeaponParameters,
    candidates: &CombatIndex<'_>,
    focus: Option<u32>,
) -> Option<u32> {
    let healths = &world.resource::<Healths>().0;
    let matches = |id: u32| {
        let health = healths.get(&id)?;
        if *health == 0 {
            return None;
        }
        let owner = *candidates.owners.get(&id)?;
        // One rule for every relation (`turts_rules::relation`); the team of a slot is the slot
        // itself until the match config gives teams.
        let relation_matches =
            crate::rules_adapter::holds_own_team(parameters.targets, parameters.owner, owner);
        if !relation_matches {
            return None;
        }
        let position = *candidates.positions.get(&id)?;
        within_weapon_range(origin, position, parameters.range)
            .then(|| (position.distance_squared(origin), id))
    };
    if let Some(focus) = focus
        && matches(focus).is_some()
    {
        return Some(focus);
    }
    candidates
        .grid
        .within(candidates.points, origin, parameters.range)
        .into_iter()
        .filter_map(|point_index| matches(candidates.points[point_index].id))
        .min_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
        })
        .map(|(_, id)| id)
}

fn fire(
    tx: &mut Transaction<'_>,
    weapon: u32,
    target: u32,
    parameters: &WeaponParameters,
    state: &mut WeaponState,
    ordered_attack: bool,
) -> Result<(), RejectReason> {
    let ammo_amount = if let Some((item, per_shot)) = &parameters.ammo {
        tx.inventory(Endpoint::Inventory(weapon))?;
        let mut account = tx.account(Endpoint::Inventory(weapon), item);
        let available = account
            .on_hand
            .checked_sub(account.reserved)
            .ok_or(RejectReason::IntegrityViolation)?;
        if available < *per_shot {
            state.status = WeaponStatus::OutOfAmmo;
            tx.stage_weapon(weapon, state.clone());
            return Ok(());
        }
        account.on_hand = account
            .on_hand
            .checked_sub(*per_shot)
            .ok_or(RejectReason::IntegrityViolation)?;
        tx.stage_account(Endpoint::Inventory(weapon), item.clone(), account);
        *per_shot
    } else {
        0
    };

    let damage = parameters
        .damage
        .checked_add(parameters.modifier?)
        .ok_or(RejectReason::ArithmeticOverflow)?;
    let health = tx.health(target)?;
    let applied = health.min(damage);
    let remaining = health
        .checked_sub(applied)
        .ok_or(RejectReason::IntegrityViolation)?;

    tx.stage_health(target, remaining);
    state.cooldown = parameters.cooldown_ticks?;
    state.status = if state.cooldown == 0 {
        WeaponStatus::Ready
    } else {
        WeaponStatus::Cooling
    };
    state.last_target = (remaining > 0 || ordered_attack).then_some(target);
    if ordered_attack {
        state.ordered_target_fired = Some(target);
    }
    tx.stage_weapon(weapon, state.clone());
    tx.stage_event(
        parameters.owner,
        EconomyEventKind::Fired {
            weapon,
            target,
            item: parameters.ammo.as_ref().map(|(item, _)| item.clone()),
            ammo: ammo_amount,
            damage,
            applied,
        },
    )?;
    if remaining == 0 {
        transaction::remove_unit(tx, target, RemoveCause::Killed { by: weapon })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use turts_mod_api::{
        Behaviour, ContentCatalog, ContentId, EngagePolicy, FirePolicy, Footprint, MapDef, MapSize,
        OrderDef, Placement, ResolvedAmmo, ResolvedInventory, ResolvedUnit, ResolvedWeapon,
        ResourceDef, ResourceKind, SlotRelation, SourceDef, TargetFilter, TargetKind, WorldPos,
    };

    use crate::{
        EconomyLedger, Order, OrderTargetSim, Position, RejectReason, SlotId, UnitId, UnitOrders,
        apply_command,
        bootstrap::InstalledCatalog,
        economy::combat::{Healths, WeaponState, WeaponStatus},
        economy::ledger::Account,
        headless_app, install_package, stage_package,
    };

    const WEAPON: &str = "combat_fixture:weapon";
    const TARGET: &str = "combat_fixture:target";
    const AMMO: &str = "combat_fixture:ammo";

    fn cid(value: &str) -> ContentId {
        ContentId::parse(value).unwrap()
    }

    fn unit(id: &str, health: Option<u64>) -> ResolvedUnit {
        ResolvedUnit {
            id: cid(id),
            name: id.into(),
            footprint: Footprint::Circle { radius: 0.5 },
            movement: None,
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: None,
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: health.map(|max| turts_mod_api::HealthDef { max }),
            weapon: None,
            removal: health.map(|_| turts_mod_api::RemovalDef {
                by_command: false,
                contents: turts_mod_api::ContentsPolicy::Destroy,
            }),
        }
    }

    fn weapon(cooldown_ms: u64, damage: u64) -> ResolvedUnit {
        let mut result = unit(WEAPON, Some(100));
        result.inventory = Some(ResolvedInventory {
            capacity: 10,
            accepts: vec![cid(AMMO)],
        });
        result.weapon = Some(ResolvedWeapon {
            range: 8.0,
            cooldown_ms,
            damage,
            ammo: Some(ResolvedAmmo {
                item: cid(AMMO),
                per_shot: 1,
            }),
            targets: TargetFilter {
                slots: SlotRelation::Other,
            },
        });
        result
    }

    fn app_with(tick_hz: u16, cooldown_ms: u64, damage: u64) -> bevy_app::App {
        app_with_units(
            tick_hz,
            vec![weapon(cooldown_ms, damage), unit(TARGET, Some(200))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
            ],
            BTreeMap::new(),
        )
    }

    fn placement(id: &str, owner_slot: u8, x: f32, y: f32) -> Placement {
        Placement {
            unit: id.into(),
            owner_slot,
            pos: WorldPos { x, y },
            contents: vec![],
            facing: None,
            start_order: None,
        }
    }

    fn app_with_units(
        tick_hz: u16,
        units: Vec<ResolvedUnit>,
        placements: Vec<Placement>,
        technologies: BTreeMap<ContentId, turts_mod_api::ResolvedTechnology>,
    ) -> bevy_app::App {
        let units = units
            .into_iter()
            .map(|definition| (definition.id.clone(), definition))
            .collect();
        let catalog = ContentCatalog {
            resources: BTreeMap::from([(
                cid(AMMO),
                ResourceDef {
                    id: "ammo".into(),
                    name: "Ammo".into(),
                    unit_name: "shot".into(),
                    kind: ResourceKind::Item,
                },
            )]),
            units,
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies,
            orders: BTreeMap::new(),
            order_queue_depths: std::collections::BTreeMap::new(),
            source_files: BTreeMap::new(),
        };
        let map = MapDef {
            id: "combat_fixture:map".into(),
            size: MapSize { x: 32.0, y: 32.0 },
            placements,
            nodes: vec![],
            slot_economy: vec![],
            belts: vec![],
        };
        let mut app = headless_app(tick_hz);
        let staged = stage_package(
            app.world(),
            catalog,
            SourceDef {
                file: "maps/combat.ron".into(),
                value: map,
            },
            tick_hz,
        )
        .unwrap();
        install_package(app.world_mut(), staged);
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .inventories
            .get_mut(&1)
            .unwrap()
            .items
            .insert(
                cid(AMMO),
                Account {
                    on_hand: 10,
                    reserved: 0,
                },
            );
        app
    }

    fn fired_ticks(app: &mut bevy_app::App, through: u32) -> Vec<u32> {
        let mut shots = Vec::new();
        let mut previous_health = app.world().resource::<Healths>().0[&2];
        for tick in 1..=through {
            app.update();
            let current_health = app.world().resource::<Healths>().0[&2];
            if current_health < previous_health {
                shots.push(tick);
            }
            previous_health = current_health;
        }
        shots
    }

    fn fired_events(app: &bevy_app::App) -> Vec<(u32, u32, Option<ContentId>, u64, u64, u64)> {
        app.world()
            .resource::<crate::economy::events::CommittedTick>()
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                crate::economy::events::EconomyEventKind::Fired {
                    weapon,
                    target,
                    item,
                    ammo,
                    damage,
                    applied,
                } => Some((*weapon, *target, item.clone(), *ammo, *damage, *applied)),
                _ => None,
            })
            .collect()
    }

    fn install_order(
        app: &mut bevy_app::App,
        id: &str,
        behaviour: Behaviour,
        target: TargetKind,
        fire: Option<FirePolicy>,
        engage: Option<EngagePolicy>,
    ) {
        let content_id = cid(id);
        app.world_mut()
            .resource_mut::<InstalledCatalog>()
            .0
            .orders
            .insert(
                content_id.clone(),
                OrderDef {
                    id: content_id.local.clone(),
                    name: id.into(),
                    behaviour,
                    target,
                    fire,
                    engage,
                    icon: String::new(),
                    hotkey: String::new(),
                    cursor: None,
                    card: None,
                    units: vec![WEAPON.into()],
                    description: String::new(),
                },
            );
    }

    fn set_order(app: &mut bevy_app::App, order: Order) {
        let entity = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap()
            .id();
        app.world_mut().entity_mut(entity).insert(UnitOrders {
            current: Some(order),
            ..Default::default()
        });
    }

    #[test]
    fn move_never_policy_suppresses_mobile_weapon_fire() {
        let mut app = app_with(20, 0, 5);
        install_order(
            &mut app,
            "combat_fixture:move_never",
            Behaviour::Move,
            TargetKind::Point,
            Some(FirePolicy::Never),
            None,
        );
        app.world_mut()
            .insert_resource(crate::orders::PlainMoveOrderIdentities(BTreeMap::from([(
                1,
                Order {
                    order: cid("combat_fixture:move_never"),
                    behaviour: Behaviour::Move,
                    target: OrderTargetSim::Point(bevy_math::Vec2::new(20.0, 20.0)),
                    group: 0,
                },
            )])));
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(actor)
            .insert(crate::MoveTarget(bevy_math::Vec2::new(20.0, 20.0)));
        app.update();
        assert_eq!(app.world().resource::<Healths>().0[&2], 200);
    }

    #[test]
    fn hold_in_range_policy_keeps_a_stationary_weapon_firing() {
        let mut app = app_with(20, 0, 5);
        install_order(
            &mut app,
            "combat_fixture:hold_fire",
            Behaviour::Hold,
            TargetKind::None,
            Some(FirePolicy::InRange),
            None,
        );
        set_order(
            &mut app,
            Order {
                order: cid("combat_fixture:hold_fire"),
                behaviour: Behaviour::Hold,
                target: OrderTargetSim::None,
                group: 0,
            },
        );
        app.update();
        assert_eq!(app.world().resource::<Healths>().0[&2], 195);
    }

    #[test]
    fn unknown_active_order_identity_cannot_fire() {
        let mut app = app_with(20, 0, 5);
        set_order(
            &mut app,
            Order {
                order: cid("combat_fixture:missing_order"),
                behaviour: Behaviour::Move,
                target: OrderTargetSim::Point(bevy_math::Vec2::new(20.0, 20.0)),
                group: 0,
            },
        );

        crate::economy::combat::run_combat(app.world_mut());

        assert_eq!(app.world().resource::<Healths>().0[&2], 200);
    }

    #[test]
    fn active_order_behavior_must_match_its_installed_definition_to_fire() {
        let mut app = app_with(20, 0, 5);
        install_order(
            &mut app,
            "combat_fixture:move_default",
            Behaviour::Move,
            TargetKind::Point,
            None,
            None,
        );
        set_order(
            &mut app,
            Order {
                order: cid("combat_fixture:move_default"),
                behaviour: Behaviour::Hold,
                target: OrderTargetSim::None,
                group: 0,
            },
        );

        crate::economy::combat::run_combat(app.world_mut());

        assert_eq!(app.world().resource::<Healths>().0[&2], 200);
    }

    #[test]
    fn authored_move_with_default_fire_policy_keeps_in_range_fire() {
        let mut app = app_with(20, 0, 5);
        install_order(
            &mut app,
            "combat_fixture:move_default",
            Behaviour::Move,
            TargetKind::Point,
            None,
            None,
        );
        set_order(
            &mut app,
            Order {
                order: cid("combat_fixture:move_default"),
                behaviour: Behaviour::Move,
                target: OrderTargetSim::Point(bevy_math::Vec2::new(20.0, 20.0)),
                group: 0,
            },
        );

        crate::economy::combat::run_combat(app.world_mut());

        assert_eq!(app.world().resource::<Healths>().0[&2], 195);
    }

    #[test]
    fn unit_attack_prefers_its_ordered_target_when_in_range() {
        let mut app = app_with_units(
            20,
            vec![
                weapon(1000, 5),
                unit(TARGET, Some(200)),
                unit(TARGET, Some(200)),
            ],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
                placement(TARGET, 1, 7.0, 5.0),
            ],
            BTreeMap::new(),
        );
        install_order(
            &mut app,
            "combat_fixture:attack",
            Behaviour::Attack,
            TargetKind::Unit,
            None,
            None,
        );
        set_order(
            &mut app,
            Order {
                order: cid("combat_fixture:attack"),
                behaviour: Behaviour::Attack,
                target: OrderTargetSim::Unit(3),
                group: 0,
            },
        );
        app.update();
        let health = &app.world().resource::<Healths>().0;
        assert_eq!(
            health[&2], 200,
            "the nearer non-ordered target stays untouched"
        );
        assert_eq!(health[&3], 195, "the ordered target receives the shot");
    }

    #[test]
    fn mobile_unit_attack_approaches_until_it_can_fire() {
        let mut attacker = weapon(1000, 5);
        attacker.movement = Some(turts_mod_api::MovementDef { speed: 3.0 });
        let mut app = app_with_units(
            20,
            vec![attacker, unit(TARGET, Some(200))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 20.0, 5.0),
            ],
            BTreeMap::new(),
        );
        install_attack_order(&mut app);
        set_order(&mut app, attack_order(2));
        for _ in 0..80 {
            app.update();
            if app.world().resource::<Healths>().0[&2] < 200 {
                break;
            }
        }
        assert!(app.world().resource::<Healths>().0[&2] < 200);
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        assert!(actor.get::<Position>().unwrap().0.x > 10.0);
        assert_eq!(
            actor
                .get::<UnitOrders>()
                .unwrap()
                .current
                .as_ref()
                .unwrap()
                .target,
            OrderTargetSim::Unit(2),
        );
    }

    /// An app with one mobile attacker (unit 1, radius 0.5, speed 3.0) with a weapon of `range`
    /// (4.0 is the Muster trooper) at `start`, and one target (unit 2, radius 0.5, 200 health) at
    /// `target`. Unit 1 has an Attack order on unit 2.
    fn approach_app(start: bevy_math::Vec2, target: bevy_math::Vec2, range: f32) -> bevy_app::App {
        let mut attacker = weapon(1000, 5);
        attacker.movement = Some(turts_mod_api::MovementDef { speed: 3.0 });
        attacker.weapon.as_mut().unwrap().range = range;
        let mut app = app_with_units(
            20,
            vec![attacker, unit(TARGET, Some(200))],
            vec![
                placement(WEAPON, 0, start.x, start.y),
                placement(TARGET, 1, target.x, target.y),
            ],
            BTreeMap::new(),
        );
        install_attack_order(&mut app);
        set_order(&mut app, attack_order(2));
        app
    }

    /// `approach_app`, stepped until the first shot or `ticks`. Returns the tick of the
    /// first shot and the distance between the two units at the end.
    fn approach_and_fire(
        start: bevy_math::Vec2,
        target: bevy_math::Vec2,
        range: f32,
        ticks: u32,
    ) -> (Option<u32>, f32) {
        let mut app = approach_app(start, target, range);
        let mut fired = None;
        for tick in 1..=ticks {
            app.update();
            if app.world().resource::<Healths>().0[&2] < 200 {
                fired = Some(tick);
                break;
            }
        }
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        (fired, actor.get::<Position>().unwrap().0.distance(target))
    }

    #[test]
    fn a_range_below_the_two_radii_never_fires_and_the_unit_rests_beside_the_target() {
        // PINS TODAY'S BEHAVIOUR; it is not a decision (open item X2: is the weapon range measured
        // centre to centre, as today, or edge to edge?). Both radii are 0.5, so the bodies touch
        // at a centre distance of 1.0. A range below 1.0 puts the ring point of `weapon_stand_off`
        // inside the target body. The arrival test of the movement refuses a point that overlaps
        // another body (`frame.rs`, `occupied`), so the unit rests at a free slot near the ring
        // point, outside the range. The order stays current, no failure is recorded, and the
        // weapon never fires. Shipped content cannot reach this (ranges 4.0 and 8.0).
        for range in [0.4_f32, 0.9] {
            let target = bevy_math::Vec2::new(10.0, 5.0);
            let mut app = approach_app(bevy_math::Vec2::new(5.0, 5.0), target, range);
            let position = |app: &bevy_app::App| {
                app.world()
                    .iter_entities()
                    .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
                    .unwrap()
                    .get::<Position>()
                    .unwrap()
                    .0
            };
            for _ in 0..200 {
                app.update();
            }
            let resting_at = position(&app);
            for _ in 0..200 {
                app.update();
            }
            assert_eq!(
                position(&app),
                resting_at,
                "range {range}: the unit rests and does not move on"
            );
            let gap = resting_at.distance(target);
            assert!(
                gap >= 1.0 - 1.0e-5 && gap > range,
                "range {range}: the unit rests outside the target body and outside its range (gap {gap})"
            );
            assert_eq!(
                app.world().resource::<Healths>().0[&2],
                200,
                "range {range}: the weapon never fires"
            );
            let actor = app
                .world()
                .iter_entities()
                .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
                .unwrap();
            let orders = actor.get::<UnitOrders>().unwrap();
            assert_eq!(
                orders.current.as_ref().map(|order| order.target.clone()),
                Some(OrderTargetSim::Unit(2)),
                "range {range}: the Attack order stays current"
            );
            assert!(
                orders.last_failure.is_none(),
                "range {range}: no failure is recorded"
            );
        }
    }

    #[test]
    fn attack_from_out_of_range_fires_on_every_bearing_and_distance() {
        // The approach and the fire check share one rule: the unit that stops at the end of its
        // approach is in range of its target, on every bearing. The bound is the walk at
        // 3.0 per second at 20 Hz (0.15 a tick) over the longest start distance, plus slack.
        let target = bevy_math::Vec2::new(16.0, 16.0);
        let mut failures = Vec::new();
        for distance in [4.5_f32, 7.3, 10.0, 13.7] {
            for step in 0..64 {
                let angle = step as f32 * std::f32::consts::TAU / 64.0;
                let start = target + bevy_math::Vec2::new(angle.cos(), angle.sin()) * distance;
                let bound = ((distance - 4.0) / 0.15).ceil() as u32 + 20;
                let (fired, stop) = approach_and_fire(start, target, 4.0, bound);
                if fired.is_none() {
                    failures.push((distance, step, start, stop));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "{} of 256 approaches never fired within the walk bound; first (distance, step, start, \
             stop distance): {:?}",
            failures.len(),
            failures.first()
        );
    }

    /// xorshift64 in `[0, 1)`: a fixed seed, so a run repeats.
    fn xorshift(seed: u64) -> impl FnMut() -> f32 {
        let mut state = seed;
        move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 40) as f32 / (1_u64 << 24) as f32
        }
    }

    /// One stand-off. It is in range by the rule that lets the weapon fire, it is not past the
    /// target, and it is the ring point itself when that passes the rule. Returns the number of
    /// steps of the stand-off (0 when the ring point passes the rule).
    fn check_stand_off(target: bevy_math::Vec2, from: bevy_math::Vec2, range: f32) -> u32 {
        let (point, steps) = super::stand_off_steps(target, from, range);
        assert_eq!(
            point,
            super::weapon_stand_off(target, from, range),
            "weapon_stand_off is the point of stand_off_steps"
        );
        assert!(
            super::within_weapon_range(point, target, range),
            "stand-off {point:?} is out of range {range} of {target:?} (from {from:?})"
        );
        let direction = (from - target).normalize_or_zero();
        assert!(
            (point.x - target.x) * direction.x >= 0.0 && (point.y - target.y) * direction.y >= 0.0,
            "stand-off {point:?} is past the target {target:?} (range {range}, from {from:?})"
        );
        let ring = target + direction * range;
        if super::within_weapon_range(ring, target, range) {
            assert_eq!(point, ring, "a ring point that is in range stays unchanged");
            assert_eq!(steps, 0);
        } else {
            assert!(
                steps > 0 && steps < super::STAND_OFF_MAX_STEPS,
                "the stand-off took {steps} steps"
            );
        }
        steps
    }

    #[test]
    fn weapon_stand_off_is_in_range_for_random_rings() {
        let mut next = xorshift(0x9E37_79B9_7F4A_7C15);
        let mut moved = 0_u32;
        for _ in 0..2_000_000 {
            let target = bevy_math::Vec2::new(next() * 128.0, next() * 128.0);
            let range = 0.25 + next() * 40.0;
            let angle = next() * std::f32::consts::TAU;
            let from = target
                + bevy_math::Vec2::new(angle.cos(), angle.sin()) * (range + 0.01 + next() * 30.0);
            moved += u32::from(check_stand_off(target, from, range) > 0);
        }
        assert!(moved > 0, "setup: the sample has rings that fail the rule");
    }

    #[test]
    fn step_toward_reaches_the_target_coordinate_and_never_passes_it() {
        // Across a binade boundary the gap can be below the quantum (the spacing at the larger
        // magnitude): the step must land on the target and not pass it.
        let below_one = 1.0_f32.next_down();
        assert_eq!(super::step_toward(below_one, 1.0), 1.0);
        assert_eq!(super::step_toward(1.0, below_one), below_one);
        // Across a binade boundary the step can be half a quantum (the sum rounds), and a gap of
        // 1.5 quanta needs the target: a step of 2 quanta would pass it.
        assert_eq!(
            super::step_toward(1.0_f32.next_down(), 1.0 + f32::EPSILON),
            1.0
        );
        // Walks of an even and an odd number of quanta, up and down, end on the target in exactly
        // that number of steps (a step of 2 quanta ends sooner), and are monotone.
        for (start, target, quanta) in [
            (1.0_f32, 1.0 + 100.0 * f32::EPSILON, 100),
            (1.0, 1.0 + 101.0 * f32::EPSILON, 101),
            (-3.0, -3.0 - 100.0 * 2.0 * f32::EPSILON, 100),
            (-3.0, -3.0 - 51.0 * 2.0 * f32::EPSILON, 51),
        ] {
            let mut coordinate = start;
            let mut steps = 0;
            while coordinate != target {
                let next = super::step_toward(coordinate, target);
                assert!(
                    (next - coordinate).abs() > 0.0
                        && (target - next).abs() <= (target - coordinate).abs(),
                    "{coordinate} to {next} does not approach {target}"
                );
                assert!(
                    (next - target) * (coordinate - target) >= 0.0,
                    "{next} is past {target}"
                );
                coordinate = next;
                steps += 1;
                assert!(
                    steps <= 1_000,
                    "the walk from {start} to {target} does not end"
                );
            }
            assert_eq!(
                steps, quanta,
                "the walk from {start} to {target} takes one step per quantum"
            );
        }
        // A coordinate on the target stays.
        assert_eq!(super::step_toward(7.5, 7.5), 7.5);
    }

    #[test]
    fn a_stand_off_walk_that_reaches_its_bound_returns_the_target() {
        // The fallback of `stand_off_bounded`: with a bound of 1 step, a ring point that needs
        // more steps ends on the target, and that point passes the rule. Find such a ring point
        // in a fixed sample first, so the test has a case that needs 2 steps or more.
        let mut next = xorshift(0xA5A5_1234_9876_5432);
        let mut found = 0_u32;
        for _ in 0..200_000 {
            let magnitude = [64.0_f32, 4096.0, 1_048_576.0][(next() * 3.0) as usize % 3];
            let target = bevy_math::Vec2::new(next() * magnitude, next() * magnitude);
            let range = [0.001_f32, 0.05, 1.0, 4.0, 40.0][(next() * 5.0) as usize % 5];
            let angle = next() * std::f32::consts::TAU;
            let from = target
                + bevy_math::Vec2::new(angle.cos(), angle.sin()) * range * (1.0001 + next() * 30.0);
            let (walked, steps) = super::stand_off_steps(target, from, range);
            if steps < 2 {
                continue;
            }
            found += 1;
            let (point, used, reached) = super::stand_off_bounded(target, from, range, 1);
            assert!(
                !reached && used == 1,
                "a bound of 1 is reached ({steps} steps needed)"
            );
            assert_eq!(
                point, target,
                "the bound is reached: the result is the target"
            );
            assert!(
                super::within_weapon_range(point, target, range),
                "the fallback point passes the rule"
            );
            // With the production bound the same case walks and does not fall back.
            let (point, used, reached) =
                super::stand_off_bounded(target, from, range, super::STAND_OFF_MAX_STEPS);
            assert!(reached && used == steps && point == walked);
        }
        assert!(
            found > 0,
            "setup: the sample has a ring point that needs 2 steps or more"
        );
    }

    #[test]
    fn weapon_stand_off_is_in_range_for_a_range_far_below_the_coordinates() {
        // The reproduction of the review of mp1/combat-fixes (performance-architect): a range of
        // 0.001 at a coordinate magnitude of 3960. A pull-in by a fixed factor of the rounding
        // bound crossed the target there, and the unit never fired.
        let target = bevy_math::Vec2::new(3713.1167, 3960.168);
        let from = bevy_math::Vec2::new(3713.114, 3960.1707);
        assert!(check_stand_off(target, from, 0.001) > 0);
    }

    #[test]
    fn weapon_stand_off_is_in_range_for_the_reviewed_case_classes() {
        // The case classes of the review, as fixed seeds: three coordinate magnitudes M, five
        // ranges r (down to r / M of 1e-6), and the bearings that stress the rounding: the four
        // axes, the four diagonals, and random ones.
        let directions = |kind: usize, next: &mut dyn FnMut() -> f32| match kind {
            0 => bevy_math::Vec2::X,
            1 => -bevy_math::Vec2::X,
            2 => bevy_math::Vec2::Y,
            3 => -bevy_math::Vec2::Y,
            4..=7 => {
                let sign = |bit: usize| if (kind >> bit) & 1 == 0 { 1.0 } else { -1.0 };
                bevy_math::Vec2::new(sign(0), sign(1)).normalize()
            }
            _ => {
                let angle = next() * std::f32::consts::TAU;
                bevy_math::Vec2::new(angle.cos(), angle.sin())
            }
        };
        let mut moved = 0_u32;
        let mut most_steps = 0_u32;
        let mut seed = 0xD1B5_4A32_D192_ED03_u64;
        for magnitude in [64.0_f32, 4096.0, 1_048_576.0] {
            for range in [0.001_f32, 0.05, 1.0, 4.0, 40.0] {
                for kind in 0..9 {
                    seed = seed.rotate_left(7) ^ 0x2545_F491_4F6C_DD1D;
                    let mut next = xorshift(seed);
                    for _ in 0..10_000 {
                        let target = bevy_math::Vec2::new(next() * magnitude, next() * magnitude);
                        let from =
                            target + directions(kind, &mut next) * range * (1.0001 + next() * 30.0);
                        let steps = check_stand_off(target, from, range);
                        moved += u32::from(steps > 0);
                        most_steps = most_steps.max(steps);
                    }
                }
            }
        }
        assert!(moved > 0, "setup: the sample has rings that fail the rule");
        println!("most stand-off steps: {most_steps}");
        // The derivation of `weapon_stand_off` gives at most about 30; 3 on the machine of the
        // first run (at most 4 in the 27.85 M cases of the review).
        assert!(most_steps <= 30, "the largest step count is {most_steps}");
    }

    fn attack_order(target: u32) -> Order {
        Order {
            order: cid("combat_fixture:attack"),
            behaviour: Behaviour::Attack,
            target: OrderTargetSim::Unit(target),
            group: 0,
        }
    }

    fn install_attack_order(app: &mut bevy_app::App) {
        install_order(
            app,
            "combat_fixture:attack",
            Behaviour::Attack,
            TargetKind::Unit,
            None,
            None,
        );
    }

    #[test]
    fn static_weapon_records_out_of_range_after_acceptance() {
        let mut app = app_with_units(
            20,
            vec![weapon(1000, 5), unit(TARGET, Some(200))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 20.0, 5.0),
            ],
            BTreeMap::new(),
        );
        install_attack_order(&mut app);
        let command = crate::Command {
            slot: SlotId(0),
            seq: 1,
            kind: crate::CommandKind::Order(Box::new(crate::OrderCommand {
                order: cid("combat_fixture:attack"),
                behaviour: Behaviour::Attack,
                target_kind: TargetKind::Unit,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Unit(2),
                queue: crate::QueueMode::Replace,
                group: 0,
                allowed_units: vec![WEAPON.into()],
                queue_depth: 16,
            })),
        };
        let accepted = apply_command(app.world_mut(), &command);
        assert_eq!(accepted.units.accepted, vec![1]);
        app.update();
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        assert_eq!(
            actor
                .get::<UnitOrders>()
                .unwrap()
                .last_failure
                .as_ref()
                .unwrap()
                .reason,
            RejectReason::OutOfRange,
        );
    }

    #[test]
    fn attack_target_gone_before_first_shot_is_a_failure() {
        let mut app = app_with(20, 1000, 5);
        install_attack_order(&mut app);
        set_order(&mut app, attack_order(2));
        let target = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 2))
            .unwrap()
            .id();
        app.world_mut().despawn(target);
        app.update();
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        assert_eq!(
            actor
                .get::<UnitOrders>()
                .unwrap()
                .last_failure
                .as_ref()
                .unwrap()
                .reason,
            RejectReason::TargetGone,
        );
    }

    #[test]
    fn attack_target_gone_after_a_shot_completes_normally() {
        let mut app = app_with(20, 0, 250);
        install_attack_order(&mut app);
        set_order(&mut app, attack_order(2));
        app.update();
        app.update();
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        let orders = actor.get::<UnitOrders>().unwrap();
        assert!(orders.current.is_none());
        assert!(orders.last_failure.is_none());
    }

    #[test]
    fn ordered_target_firing_survives_distractor_shots_before_target_loss() {
        let mut attacker = weapon(0, 5);
        attacker.movement = Some(turts_mod_api::MovementDef { speed: 3.0 });
        let mut app = app_with_units(
            20,
            vec![attacker, unit(TARGET, Some(200)), unit(TARGET, Some(200))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
                placement(TARGET, 1, 7.0, 5.0),
            ],
            BTreeMap::new(),
        );
        install_attack_order(&mut app);
        set_order(&mut app, attack_order(2));
        app.update();
        assert_eq!(app.world().resource::<Healths>().0[&2], 195);

        let target = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 2))
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(target)
            .get_mut::<Position>()
            .unwrap()
            .0 = bevy_math::Vec2::new(25.0, 25.0);
        app.update();
        assert_eq!(app.world().resource::<Healths>().0[&3], 195);
        assert_eq!(
            app.world().resource::<crate::economy::combat::Weapons>().0[&1].ordered_target_fired,
            Some(2),
            "a distractor shot must not replace direct-order firing evidence",
        );

        app.world_mut().despawn(target);
        app.update();
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        assert!(actor.get::<UnitOrders>().unwrap().current.is_none());
        assert!(actor.get::<UnitOrders>().unwrap().last_failure.is_none());
    }

    #[test]
    fn combat_skips_a_later_shooter_destroyed_this_phase() {
        let mut second_weapon = weapon(0, 5);
        second_weapon.id = cid("combat_fixture:second_weapon");
        let mut app = app_with_units(
            20,
            vec![weapon(0, 1_000), second_weapon, unit(TARGET, Some(200))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement("combat_fixture:second_weapon", 1, 6.0, 5.0),
                placement(TARGET, 1, 20.0, 20.0),
            ],
            BTreeMap::new(),
        );
        set_ammo(&mut app, 2, 10, 0);
        app.update();
        assert!(!app.world().resource::<Healths>().0.contains_key(&2));
        assert_eq!(
            fired_events(&app),
            vec![(1, 2, Some(cid(AMMO)), 1, 1_000, 100)]
        );
    }

    fn attack_point_order() -> Order {
        Order {
            order: cid("combat_fixture:attack_move"),
            behaviour: Behaviour::Attack,
            target: OrderTargetSim::Point(bevy_math::Vec2::new(20.0, 20.0)),
            group: 0,
        }
    }

    fn install_attack_move(app: &mut bevy_app::App) {
        install_order(
            app,
            "combat_fixture:attack_move",
            Behaviour::Attack,
            TargetKind::Point,
            None,
            Some(EngagePolicy::StopAndFire),
        );
        set_order(app, attack_point_order());
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(actor)
            .insert(crate::MoveTarget(bevy_math::Vec2::new(20.0, 20.0)));
    }

    #[test]
    fn attack_move_stops_to_fire_at_a_nearby_target() {
        let mut app = app_with(20, 1000, 5);
        install_attack_move(&mut app);
        app.update();
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        assert!(actor.get::<crate::MoveTarget>().is_none());
        assert_eq!(app.world().resource::<Healths>().0[&2], 195);
    }

    #[test]
    fn a_weapon_that_the_power_stub_disables_does_not_acquire_and_does_not_fire() {
        use crate::economy::power::PowerProbe;
        let mut app = app_with(20, 1000, 5);
        install_attack_move(&mut app);
        app.world_mut().insert_resource(PowerProbe {
            weapon_enabled: false,
            rate: turts_rules::Ratio::ONE,
        });
        for _ in 0..3 {
            app.update();
        }
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        assert!(
            actor.get::<crate::MoveTarget>().is_some(),
            "the weapon chose a target: acquisition does not call weapon_enabled"
        );
        assert_eq!(
            app.world().resource::<crate::economy::combat::Weapons>().0[&1].last_target,
            None
        );
        assert_eq!(app.world().resource::<Healths>().0[&2], 200);
        // The same world with the probe says "enabled": the weapon acts as in the test above.
        let mut enabled = app_with(20, 1000, 5);
        install_attack_move(&mut enabled);
        enabled.world_mut().insert_resource(PowerProbe {
            weapon_enabled: true,
            rate: turts_rules::Ratio::ONE,
        });
        enabled.update();
        assert_eq!(enabled.world().resource::<Healths>().0[&2], 195);
    }

    #[test]
    fn a_plain_weapon_that_the_power_stub_disables_does_not_fire() {
        use crate::economy::power::PowerProbe;
        let mut app = app_with(20, 0, 5);
        app.world_mut().insert_resource(PowerProbe {
            weapon_enabled: false,
            rate: turts_rules::Ratio::ONE,
        });
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(app.world().resource::<Healths>().0[&2], 200);
        assert!(fired_events(&app).is_empty());
    }

    #[test]
    fn the_outcome_slot_sees_a_unit_that_the_death_path_removed_in_the_same_tick() {
        use bevy_app::FixedUpdate;
        use bevy_ecs::prelude::*;
        #[derive(Resource, Default)]
        struct Seen(Vec<(u32, bool)>);
        fn probe(index: Res<crate::UnitIndex>, tick: Res<crate::Tick>, mut seen: ResMut<Seen>) {
            seen.0.push((tick.0, index.get(2).is_some()));
        }
        // One shot of 200 kills the target (200 health) in tick 1.
        let mut app = app_with(20, 0, 200);
        app.init_resource::<Seen>();
        app.add_systems(FixedUpdate, probe.in_set(crate::SimSet::Outcome));
        app.update();
        assert_eq!(app.world().resource::<Healths>().0.get(&2), None);
        assert_eq!(
            app.world().resource::<Seen>().0,
            [(1, false)],
            "the outcome check ran before the removal of the killed unit"
        );
    }

    #[test]
    fn attack_move_stays_paused_on_acquired_target_without_ammo() {
        let mut attacker = weapon(0, 5);
        attacker.movement = Some(turts_mod_api::MovementDef { speed: 10.0 });
        let mut app = app_with_units(
            20,
            vec![attacker, unit(TARGET, Some(200))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
            ],
            BTreeMap::new(),
        );
        install_attack_move(&mut app);
        set_ammo(&mut app, 1, 0, 0);

        app.update();
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        assert!(actor.get::<crate::MoveTarget>().is_none());
        assert_eq!(app.world().resource::<Healths>().0[&2], 200);
        assert_eq!(
            app.world().resource::<crate::economy::combat::Weapons>().0[&1].last_target,
            Some(2),
            "target acquisition survives the blocked first shot",
        );
        let acquired_position = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap()
            .get::<Position>()
            .unwrap()
            .0;

        for _ in 0..3 {
            app.update();
            let actor = app
                .world()
                .iter_entities()
                .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
                .unwrap();
            assert!(actor.get::<crate::MoveTarget>().is_none());
            assert_eq!(actor.get::<Position>().unwrap().0, acquired_position);
        }
    }

    #[test]
    fn no_order_no_ammo_keeps_legacy_target_history() {
        for previous_target in [None, Some(2)] {
            let mut app = app_with(20, 0, 5);
            set_ammo(&mut app, 1, 0, 0);
            app.world_mut()
                .resource_mut::<crate::economy::combat::Weapons>()
                .0
                .get_mut(&1)
                .unwrap()
                .last_target = previous_target;

            app.update();

            assert_eq!(
                app.world().resource::<crate::economy::combat::Weapons>().0[&1].last_target,
                previous_target,
                "the un-ordered M3 path keeps prior target history when no ammo is available",
            );
        }
    }

    #[test]
    fn suppressed_authored_policies_advance_cooldown_without_firing() {
        let cases = [
            (
                "combat_fixture:move_never_cooldown",
                Behaviour::Move,
                TargetKind::Point,
                OrderTargetSim::Point(bevy_math::Vec2::new(20.0, 20.0)),
                Some(FirePolicy::Never),
                None,
            ),
            (
                "combat_fixture:hold_never_cooldown",
                Behaviour::Hold,
                TargetKind::None,
                OrderTargetSim::None,
                Some(FirePolicy::Never),
                None,
            ),
            (
                "combat_fixture:attack_no_engage_cooldown",
                Behaviour::Attack,
                TargetKind::Point,
                OrderTargetSim::Point(bevy_math::Vec2::new(20.0, 20.0)),
                None,
                None,
            ),
        ];
        for (id, behaviour, target_kind, target, fire, engage) in cases {
            let mut app = app_with(20, 0, 5);
            install_order(&mut app, id, behaviour, target_kind, fire, engage);
            set_order(
                &mut app,
                Order {
                    order: cid(id),
                    behaviour,
                    target,
                    group: 0,
                },
            );
            app.world_mut()
                .resource_mut::<crate::economy::combat::Weapons>()
                .0
                .get_mut(&1)
                .unwrap()
                .cooldown = 3;

            crate::economy::combat::run_combat(app.world_mut());
            crate::economy::combat::run_combat(app.world_mut());
            assert_eq!(app.world().resource::<Healths>().0[&2], 200, "{id}");
            assert_eq!(
                app.world().resource::<crate::economy::combat::Weapons>().0[&1].cooldown,
                1,
                "{id} advances the enabled weapon timer",
            );

            install_order(
                &mut app,
                "combat_fixture:hold_in_range_after_cooldown",
                Behaviour::Hold,
                TargetKind::None,
                Some(FirePolicy::InRange),
                None,
            );
            set_order(
                &mut app,
                Order {
                    order: cid("combat_fixture:hold_in_range_after_cooldown"),
                    behaviour: Behaviour::Hold,
                    target: OrderTargetSim::None,
                    group: 0,
                },
            );
            crate::economy::combat::run_combat(app.world_mut());
            assert_eq!(app.world().resource::<Healths>().0[&2], 195, "{id}");
        }
    }

    #[test]
    fn attack_move_resumes_its_slot_after_the_engaged_target_is_gone() {
        let mut app = app_with(20, 0, 250);
        install_attack_move(&mut app);
        app.update();
        app.update();
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        assert_eq!(
            actor.get::<crate::MoveTarget>().unwrap().0,
            bevy_math::Vec2::new(20.0, 20.0),
        );
    }

    #[test]
    fn attack_move_stays_paused_when_a_second_target_remains_in_range() {
        let mut attacker = weapon(0, 250);
        attacker.movement = Some(turts_mod_api::MovementDef { speed: 3.0 });
        let mut app = app_with_units(
            20,
            vec![attacker, unit(TARGET, Some(200)), unit(TARGET, Some(200))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
                placement(TARGET, 1, 7.0, 5.0),
            ],
            BTreeMap::new(),
        );
        install_attack_move(&mut app);

        app.update();
        assert!(!app.world().resource::<Healths>().0.contains_key(&2));
        let paused_position = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap()
            .get::<Position>()
            .unwrap()
            .0;

        app.update();
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        assert_eq!(actor.get::<Position>().unwrap().0, paused_position);
        assert!(actor.get::<crate::MoveTarget>().is_none());
        assert!(!app.world().resource::<Healths>().0.contains_key(&3));

        app.update();
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        assert!(actor.get::<crate::MoveTarget>().is_some());
    }

    fn set_ammo(app: &mut bevy_app::App, unit: u32, amount: u64, reserved: u64) {
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .inventories
            .get_mut(&unit)
            .unwrap()
            .items
            .insert(
                cid(AMMO),
                Account {
                    on_hand: amount,
                    reserved,
                },
            );
    }

    #[test]
    fn shot_debits_ammo_and_damages_in_one_operation() {
        let mut app = app_with(20, 1_000, 10);
        app.world_mut()
            .resource_mut::<crate::economy::transaction::AuditTrail>()
            .enable();
        app.update();
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&1].items[&cid(AMMO)].on_hand,
            9
        );
        assert_eq!(app.world().resource::<Healths>().0[&2], 190);
        let operations = &app
            .world()
            .resource::<crate::economy::transaction::AuditTrail>()
            .operations;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].events.len(), 1);
        assert!(matches!(
            operations[0].events[0].kind,
            crate::economy::events::EconomyEventKind::Fired {
                damage: 10,
                applied: 10,
                ..
            }
        ));
    }

    #[test]
    fn nearest_target_wins_tie_lowest_unit_id() {
        let mut app = app_with_units(
            20,
            vec![weapon(0, 10), unit(TARGET, Some(100))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
                placement(TARGET, 1, 4.0, 5.0),
            ],
            BTreeMap::new(),
        );
        app.update();
        assert_eq!(fired_events(&app)[0].1, 2);
        assert_eq!(app.world().resource::<Healths>().0[&2], 90);
        assert_eq!(app.world().resource::<Healths>().0[&3], 100);
    }

    #[test]
    fn out_of_ammo_no_shot() {
        let mut app = app_with(20, 1_000, 10);
        set_ammo(&mut app, 1, 1, 1);
        app.update();
        assert!(fired_events(&app).is_empty());
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&1].items[&cid(AMMO)].on_hand,
            1
        );
        assert_eq!(app.world().resource::<Healths>().0[&2], 200);
        assert_eq!(
            app.world().resource::<crate::economy::combat::Weapons>().0[&1].status,
            WeaponStatus::OutOfAmmo
        );
    }

    #[test]
    fn optional_ammo_records_no_item_or_debit() {
        let mut unarmed = weapon(0, 10);
        unarmed.weapon.as_mut().unwrap().ammo = None;
        let mut app = app_with_units(
            20,
            vec![unarmed, unit(TARGET, Some(100))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
            ],
            BTreeMap::new(),
        );
        app.update();
        assert_eq!(app.world().resource::<Healths>().0[&2], 90);
        assert_eq!(fired_events(&app), vec![(1, 2, None, 0, 10, 10)]);
    }

    #[test]
    fn zero_cooldown_fires_at_most_once_per_tick() {
        let mut app = app_with(20, 0, 10);
        assert_eq!(fired_ticks(&mut app, 3), vec![1, 2, 3]);
        assert_eq!(app.world().resource::<Healths>().0[&2], 170);
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&1].items[&cid(AMMO)].on_hand,
            7
        );
    }

    #[test]
    fn target_out_of_range_no_shot() {
        let mut app = app_with(20, 1_000, 10);
        let entity = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 2))
            .unwrap()
            .id();
        app.world_mut()
            .entity_mut(entity)
            .get_mut::<crate::Position>()
            .unwrap()
            .0
            .x = 20.0;
        app.update();
        assert!(fired_events(&app).is_empty());
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&1].items[&cid(AMMO)].on_hand,
            10
        );
        assert_eq!(app.world().resource::<Healths>().0[&2], 200);
        assert_eq!(
            app.world().resource::<crate::economy::combat::Weapons>().0[&1].status,
            WeaponStatus::NoTarget
        );
    }

    #[test]
    fn hostility_other_ignores_same_owner() {
        let mut app = app_with_units(
            20,
            vec![weapon(0, 10), unit(TARGET, Some(100))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 0, 6.1, 5.0),
                placement(TARGET, 1, 7.3, 5.0),
            ],
            BTreeMap::new(),
        );
        app.update();
        assert_eq!(fired_events(&app)[0].1, 3);
        assert_eq!(app.world().resource::<Healths>().0[&2], 100);
        assert_eq!(app.world().resource::<Healths>().0[&3], 90);
    }

    #[test]
    fn overkill_applied_is_min_and_target_removed_once() {
        let mut target = unit(TARGET, Some(5));
        target.inventory = Some(ResolvedInventory {
            capacity: 10,
            accepts: vec![cid(AMMO)],
        });
        let mut app = app_with_units(
            20,
            vec![weapon(1_000, 10), target],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
            ],
            BTreeMap::new(),
        );
        set_ammo(&mut app, 2, 3, 0);
        app.world_mut()
            .resource_mut::<crate::economy::transaction::AuditTrail>()
            .enable();
        app.update();
        assert!(!app.world().resource::<Healths>().0.contains_key(&2));
        assert!(
            !app.world()
                .resource::<EconomyLedger>()
                .inventories
                .contains_key(&2)
        );
        assert_eq!(fired_events(&app), vec![(1, 2, Some(cid(AMMO)), 1, 10, 5)]);
        let destroyed = app
            .world()
            .resource::<crate::economy::events::CommittedTick>()
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    crate::economy::events::EconomyEventKind::Destroyed { unit: 2, .. }
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(destroyed.len(), 1);
        assert!(
            matches!(destroyed[0].kind, crate::economy::events::EconomyEventKind::Destroyed { ref items, .. }
            if items == &vec![crate::economy::events::DestroyedLine {
                location: crate::economy::events::DestroyedLocation::Inventory(2),
                item: cid(AMMO), amount: 3,
            }])
        );
        let events = &app
            .world()
            .resource::<crate::economy::transaction::AuditTrail>()
            .operations[0]
            .events;
        assert!(matches!(
            events[0].kind,
            crate::economy::events::EconomyEventKind::Fired { .. }
        ));
        assert!(matches!(
            events[1].kind,
            crate::economy::events::EconomyEventKind::Destroyed { .. }
        ));
    }

    #[test]
    fn retarget_after_kill() {
        let mut app = app_with_units(
            20,
            vec![weapon(1_000, 10), unit(TARGET, Some(5))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
                placement(TARGET, 1, 7.3, 5.0),
            ],
            BTreeMap::new(),
        );
        let mut shots = Vec::new();
        for _ in 0..21 {
            app.update();
            shots.extend(fired_events(&app));
        }
        assert_eq!(
            shots.iter().map(|shot| shot.1).collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert_eq!(
            shots.iter().map(|shot| shot.5).collect::<Vec<_>>(),
            vec![5, 5]
        );
        assert!(!app.world().resource::<Healths>().0.contains_key(&2));
        assert!(!app.world().resource::<Healths>().0.contains_key(&3));
    }

    #[test]
    fn later_weapon_retargets_after_same_tick_kill() {
        let mut app = app_with_units(
            20,
            vec![weapon(0, 10), unit(TARGET, Some(5))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(WEAPON, 0, 8.0, 5.0),
                placement(TARGET, 1, 6.2, 5.0),
                placement(TARGET, 1, 8.0, 7.0),
            ],
            BTreeMap::new(),
        );
        set_ammo(&mut app, 2, 10, 0);
        app.update();
        let shots = fired_events(&app);
        assert_eq!(
            shots
                .iter()
                .map(|shot| (shot.0, shot.1))
                .collect::<Vec<_>>(),
            vec![(1, 3), (2, 4)]
        );
        assert!(!app.world().resource::<Healths>().0.contains_key(&3));
        assert!(!app.world().resource::<Healths>().0.contains_key(&4));
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&1].items[&cid(AMMO)].on_hand,
            9
        );
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&2].items[&cid(AMMO)].on_hand,
            9
        );
    }

    #[test]
    fn modifier_overflow_rejects_the_whole_shot() {
        let technology = cid("combat_fixture:overflow");
        let mut app = app_with_units(
            20,
            vec![weapon(0, 1), unit(TARGET, Some(100))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
            ],
            BTreeMap::from([(
                technology.clone(),
                turts_mod_api::ResolvedTechnology {
                    id: technology.clone(),
                    name: "Overflow".into(),
                    prerequisites: vec![],
                    duration_ms: 1,
                    costs: vec![],
                    payment: turts_mod_api::PaymentPolicy::ReserveThenCharge,
                    cancel_refunds: turts_mod_api::CancelRefunds {
                        queued: turts_mod_api::RefundRatio {
                            numerator: 1,
                            denominator: 1,
                        },
                        running: turts_mod_api::RefundRatio {
                            numerator: 1,
                            denominator: 1,
                        },
                        output_blocked: turts_mod_api::RefundRatio {
                            numerator: 1,
                            denominator: 1,
                        },
                        destroyed: turts_mod_api::RefundRatio::ZERO,
                    },
                    effects: vec![turts_mod_api::ResolvedEffect::WeaponModifier {
                        unit: cid(WEAPON),
                        stat: turts_mod_api::WeaponStat::Damage,
                        add: u64::MAX,
                    }],
                },
            )]),
        );
        app.world_mut()
            .resource_mut::<crate::economy::research::ResearchState>()
            .completed
            .insert((0, technology));
        app.world_mut()
            .resource_mut::<crate::economy::transaction::AuditTrail>()
            .enable();
        app.update();
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&1].items[&cid(AMMO)].on_hand,
            10
        );
        assert_eq!(app.world().resource::<Healths>().0[&2], 100);
        assert_eq!(
            app.world().resource::<crate::economy::combat::Weapons>().0[&1],
            WeaponState::default()
        );
        let operation = &app
            .world()
            .resource::<crate::economy::transaction::AuditTrail>()
            .operations[0];
        assert_eq!(operation.before, operation.after);
        assert!(operation.events.is_empty());
    }

    #[test]
    fn research_bonus_applies_in_completion_tick() {
        let technology = cid("combat_fixture:reinforced");
        let mut lab = unit("combat_fixture:lab", None);
        lab.inventory = Some(ResolvedInventory {
            capacity: 5,
            accepts: vec![cid(AMMO)],
        });
        lab.research = Some(turts_mod_api::ResolvedResearchStation {
            technologies: vec![technology.clone()],
            queue_capacity: 1,
        });
        let definition = turts_mod_api::ResolvedTechnology {
            id: technology.clone(),
            name: "Reinforced".into(),
            prerequisites: vec![],
            duration_ms: 1_000,
            costs: vec![turts_mod_api::ResolvedCost {
                resource: cid(AMMO),
                amount: 1,
                source: turts_mod_api::CostSource::ProducerInventory,
            }],
            payment: turts_mod_api::PaymentPolicy::ReserveThenCharge,
            cancel_refunds: turts_mod_api::CancelRefunds {
                queued: turts_mod_api::RefundRatio {
                    numerator: 1,
                    denominator: 1,
                },
                running: turts_mod_api::RefundRatio {
                    numerator: 1,
                    denominator: 2,
                },
                output_blocked: turts_mod_api::RefundRatio {
                    numerator: 1,
                    denominator: 2,
                },
                destroyed: turts_mod_api::RefundRatio::ZERO,
            },
            effects: vec![turts_mod_api::ResolvedEffect::WeaponModifier {
                unit: cid(WEAPON),
                stat: turts_mod_api::WeaponStat::Damage,
                add: 10,
            }],
        };
        let mut app = app_with_units(
            20,
            vec![weapon(1_000, 10), lab, unit(TARGET, Some(200))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement("combat_fixture:lab", 0, 15.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
            ],
            BTreeMap::from([(technology.clone(), definition)]),
        );
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .inventories
            .get_mut(&2)
            .unwrap()
            .items
            .insert(
                cid(AMMO),
                Account {
                    on_hand: 1,
                    reserved: 0,
                },
            );
        crate::economy::production::queue_production(
            app.world_mut(),
            crate::SlotId(0),
            2,
            technology.clone(),
        )
        .unwrap();
        app.update();
        for _ in 0..19 {
            app.update();
        }
        assert!(
            !app.world()
                .resource::<crate::economy::research::ResearchState>()
                .completed
                .contains(&(0, technology.clone()))
        );
        app.update();
        assert!(
            app.world()
                .resource::<crate::economy::research::ResearchState>()
                .completed
                .contains(&(0, technology.clone()))
        );
        let events = &app
            .world()
            .resource::<crate::economy::events::CommittedTick>()
            .events;
        assert!(events.iter().any(|event| matches!(&event.kind,
            crate::economy::events::EconomyEventKind::ResearchCompleted { technology: done, .. }
                if done == &technology)));
        assert!(events.iter().any(|event| matches!(
            event.kind,
            crate::economy::events::EconomyEventKind::Fired {
                target: 3,
                damage: 20,
                applied: 20,
                ..
            }
        )));
        assert_eq!(app.world().resource::<Healths>().0[&3], 170);
    }

    #[test]
    fn removal_clears_live_and_firing_weapon_target_references() {
        let mut app = app_with_units(
            20,
            vec![weapon(1_000, 10), unit(TARGET, Some(5))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(WEAPON, 0, 8.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
            ],
            BTreeMap::new(),
        );
        set_ammo(&mut app, 2, 10, 0);
        app.world_mut()
            .resource_mut::<crate::economy::combat::Weapons>()
            .0
            .insert(
                1,
                WeaponState {
                    cooldown: 2,
                    status: WeaponStatus::Cooling,
                    last_target: Some(3),
                    ordered_target_fired: None,
                },
            );
        app.update();
        assert!(!app.world().resource::<Healths>().0.contains_key(&3));
        assert_eq!(
            app.world().resource::<crate::economy::combat::Weapons>().0[&1].last_target,
            None
        );
        assert_eq!(
            app.world().resource::<crate::economy::combat::Weapons>().0[&2].last_target,
            None
        );
    }

    /// The replicated weapon rows (live view and committed batch) of this tick, as
    /// `(weapon, last_target)`, for every row that names a unit that is not alive.
    fn weapon_rows_naming_absent_units(app: &bevy_app::App) -> Vec<(&'static str, u32, u32)> {
        let live: std::collections::BTreeSet<u32> = app
            .world()
            .iter_entities()
            .filter_map(|entity| entity.get::<UnitId>().map(|id| id.0))
            .collect();
        let view = crate::economy::views_m3::m3_world_view(app.world());
        let committed = app
            .world()
            .resource::<crate::economy::events::CommittedTick>();
        view.raw_weapons
            .iter()
            .map(|row| ("view", row))
            .chain(committed.weapons.iter().map(|row| ("delta", row)))
            .filter_map(|(source, row)| {
                row.last_target
                    .filter(|target| !live.contains(target))
                    .map(|target| (source, row.unit, target))
            })
            .collect()
    }

    #[test]
    fn ordered_attack_kill_replicates_no_reference_to_the_removed_target() {
        // Cooldown 1000 ms at 20 Hz = 20 ticks. One shot kills the target.
        let mut app = app_with(20, 1_000, 250);
        install_attack_order(&mut app);
        set_order(&mut app, attack_order(2));
        for tick in 1..=25 {
            app.update();
            assert_eq!(
                weapon_rows_naming_absent_units(&app),
                vec![],
                "tick {tick}: a replicated weapon row names a removed unit"
            );
        }
        assert!(!app.world().resource::<Healths>().0.contains_key(&2));
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        let orders = actor.get::<UnitOrders>().unwrap();
        assert!(orders.current.is_none(), "the attack order completes");
        assert!(
            orders.last_failure.is_none(),
            "a kill by the ordered attacker is a completion, not TargetGone"
        );
    }

    #[test]
    fn kill_by_another_weapon_clears_the_ordered_attacker_target_reference() {
        let mut app = app_with_units(
            20,
            vec![weapon(1_000, 5), unit(TARGET, Some(10))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(WEAPON, 0, 8.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
            ],
            BTreeMap::new(),
        );
        set_ammo(&mut app, 2, 10, 0);
        install_attack_order(&mut app);
        set_order(&mut app, attack_order(3));
        for tick in 1..=25 {
            app.update();
            assert_eq!(
                weapon_rows_naming_absent_units(&app),
                vec![],
                "tick {tick}: a replicated weapon row names a removed unit"
            );
        }
        assert!(!app.world().resource::<Healths>().0.contains_key(&3));
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == 1))
            .unwrap();
        let orders = actor.get::<UnitOrders>().unwrap();
        assert!(orders.current.is_none(), "the attack order completes");
        assert!(
            orders.last_failure.is_none(),
            "the ordered attacker fired at its target before the kill"
        );
    }

    #[test]
    fn killed_unit_has_final_health_zero_in_its_removal_tick() {
        let mut app = app_with(20, 0, 200);
        set_ammo(&mut app, 1, 10, 0);
        app.update();
        let committed = app
            .world()
            .resource::<crate::economy::events::CommittedTick>();
        assert_eq!(committed.removed, vec![2]);
        assert!(
            committed
                .healths
                .contains(&crate::economy::views_m3::RawHealthAudit {
                    unit: 2,
                    current: 0,
                })
        );
        assert!(!app.world().resource::<Healths>().0.contains_key(&2));
        app.update();
        assert!(
            app.world()
                .resource::<crate::economy::events::CommittedTick>()
                .healths
                .is_empty()
        );
    }

    #[test]
    fn shooter_can_kill_itself_without_resurrecting_removed_rows() {
        let mut armed = weapon(0, 200);
        armed.weapon.as_mut().unwrap().targets.slots = SlotRelation::Any;
        let mut app = app_with_units(
            20,
            vec![armed, unit(TARGET, Some(100))],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
            ],
            BTreeMap::new(),
        );
        app.update();
        assert!(!app.world().resource::<Healths>().0.contains_key(&1));
        assert!(
            !app.world()
                .resource::<crate::economy::combat::Weapons>()
                .0
                .contains_key(&1)
        );
        assert!(
            !app.world()
                .resource::<EconomyLedger>()
                .inventories
                .contains_key(&1)
        );
        let events = &app
            .world()
            .resource::<crate::economy::events::CommittedTick>()
            .events;
        assert!(matches!(
            events[0].kind,
            crate::economy::events::EconomyEventKind::Fired {
                weapon: 1,
                target: 1,
                ..
            }
        ));
        assert!(matches!(
            events[1].kind,
            crate::economy::events::EconomyEventKind::Destroyed { unit: 1, .. }
        ));
    }

    #[test]
    fn removed_gatherer_receiver_is_tolerated() {
        let mut receiver = unit(TARGET, Some(5));
        receiver.inventory = Some(ResolvedInventory {
            capacity: 10,
            accepts: vec![cid(AMMO)],
        });
        receiver.drop_off = Some(turts_mod_api::ResolvedDropOff {
            rules: vec![turts_mod_api::ResolvedDropOffRule {
                item: cid(AMMO),
                effect: turts_mod_api::ResolvedDropOffEffect::Store,
            }],
        });
        let mut gatherer = unit("combat_fixture:gatherer", None);
        gatherer.movement = Some(turts_mod_api::MovementDef { speed: 1.0 });
        gatherer.inventory = Some(ResolvedInventory {
            capacity: 4,
            accepts: vec![cid(AMMO)],
        });
        gatherer.gatherer = Some(turts_mod_api::ResolvedGatherer {
            items: vec![cid(AMMO)],
            gather_per_second: 1,
            unload_per_second: 1,
            interaction_range: 1.0,
        });
        let node = cid("combat_fixture:deposit");
        let mut app = app_with_units(
            20,
            vec![weapon(1_000, 10), receiver, gatherer],
            vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 6.2, 5.0),
                placement("combat_fixture:gatherer", 1, 10.0, 10.0),
            ],
            BTreeMap::new(),
        );
        app.world_mut()
            .resource_mut::<InstalledCatalog>()
            .0
            .nodes
            .insert(
                node.clone(),
                turts_mod_api::ResolvedNode {
                    id: node.clone(),
                    name: "Deposit".into(),
                    item: cid(AMMO),
                    radius: 0.5,
                },
            );
        app.world_mut()
            .resource_mut::<crate::economy::SurfaceNodes>()
            .0
            .push(crate::economy::NodeRecord {
                id: crate::NodeId(1),
                content_id: node,
                position: Position(crate::Vec2::new(10.0, 12.0)),
                remaining: 100,
            });
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .inventories
            .get_mut(&2)
            .unwrap()
            .items
            .insert(
                cid(AMMO),
                Account {
                    on_hand: 2,
                    reserved: 0,
                },
            );
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .inventories
            .get_mut(&3)
            .unwrap()
            .items
            .insert(
                cid(AMMO),
                Account {
                    on_hand: 3,
                    reserved: 0,
                },
            );
        crate::economy::gather::assign_gather(
            app.world_mut(),
            crate::SlotId(1),
            3,
            crate::NodeId(1),
        )
        .unwrap();
        {
            let mut orders = app
                .world_mut()
                .resource_mut::<crate::economy::EconomyOrders>();
            let order = orders.0.get_mut(&3).unwrap();
            order.receiver = Some(2);
            order.stage = crate::economy::gather::GatherStage::MovingToReceiver;
        }
        app.update();
        assert!(
            !app.world()
                .resource::<EconomyLedger>()
                .inventories
                .contains_key(&2)
        );
        assert_eq!(
            app.world().resource::<crate::economy::EconomyOrders>().0[&3].receiver,
            None
        );
        for _ in 0..100 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&3].items[&cid(AMMO)].on_hand,
            3
        );
        assert_eq!(
            app.world().resource::<crate::economy::EconomyOrders>().0[&3].receiver,
            None
        );
        assert!(matches!(
            app.world().resource::<crate::economy::EconomyOrders>().0[&3].stage,
            crate::economy::gather::GatherStage::Waiting {
                reason: crate::economy::gather::WaitReason::NoReceiver
            }
        ));
        assert_eq!(
            app.world().resource::<crate::economy::SurfaceNodes>().0[0].remaining,
            100
        );
        let remaining_stock = app
            .world()
            .resource::<EconomyLedger>()
            .inventories
            .values()
            .map(|inventory| {
                inventory
                    .items
                    .get(&cid(AMMO))
                    .map_or(0, |account| account.on_hand)
            })
            .sum::<u64>()
            + app.world().resource::<crate::economy::SurfaceNodes>().0[0].remaining;
        assert_eq!(
            remaining_stock, 112,
            "one fired ammo and two destroyed receiver cargo"
        );
    }

    #[test]
    fn cooldown_over_u32_tick_range_is_rejected_with_source() {
        let mut invalid_weapon = weapon(u64::MAX, 10);
        invalid_weapon.weapon.as_mut().unwrap().cooldown_ms = u64::MAX;
        let catalog = ContentCatalog {
            resources: BTreeMap::from([(
                cid(AMMO),
                ResourceDef {
                    id: "ammo".into(),
                    name: "Ammo".into(),
                    unit_name: "shot".into(),
                    kind: ResourceKind::Item,
                },
            )]),
            units: BTreeMap::from([
                (invalid_weapon.id.clone(), invalid_weapon),
                (cid(TARGET), unit(TARGET, Some(200))),
            ]),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: std::collections::BTreeMap::new(),
            source_files: BTreeMap::from([(cid(WEAPON), "units/weapon.ron".into())]),
        };
        let map = MapDef {
            id: "combat_fixture:map".into(),
            size: MapSize { x: 32.0, y: 32.0 },
            placements: vec![
                placement(WEAPON, 0, 5.0, 5.0),
                placement(TARGET, 1, 6.0, 5.0),
            ],
            nodes: vec![],
            slot_economy: vec![],
            belts: vec![],
        };
        let app = headless_app(20);
        let result = stage_package(
            app.world(),
            catalog,
            SourceDef {
                file: "maps/combat.ron".into(),
                value: map,
            },
            20,
        );
        let error = match result {
            Ok(_) => panic!("unrepresentable cooldown was accepted"),
            Err(error) => error,
        };
        assert_eq!(error.field_path, "weapon.cooldown_ms");
        assert_eq!(error.file.as_deref(), Some("units/weapon.ron"));
        assert!(error.message.contains("u32 cooldown ticks"));
    }

    #[test]
    fn weapon_period_is_twenty_ticks() {
        for (tick_hz, through, expected) in [
            (10, 21, vec![1, 11, 21]),
            (20, 41, vec![1, 21, 41]),
            (30, 61, vec![1, 31, 61]),
        ] {
            let mut app = app_with(tick_hz, 1_000, 10);
            assert_eq!(fired_ticks(&mut app, through), expected, "{tick_hz} Hz");
            assert_eq!(
                app.world().resource::<EconomyLedger>().inventories[&1].items[&cid(AMMO)].on_hand,
                7,
            );
            assert_eq!(
                app.world().resource::<crate::economy::combat::Healths>().0[&2],
                170
            );
        }
    }
}
