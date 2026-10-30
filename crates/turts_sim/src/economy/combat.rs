//! Weapon and health state, plus the fixed-tick combat phase.

use std::collections::BTreeMap;

use bevy_app::{App, FixedUpdate};
use bevy_ecs::{prelude::Resource, schedule::IntoScheduleConfigs, world::World};
use turts_mod_api::{ContentId, SlotRelation, WeaponStat};

use super::{
    events::{EconomyEventKind, RemoveCause},
    ledger::Endpoint,
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

/// Run each enabled weapon once, in ascending unit-id order.
pub(crate) fn run_combat(world: &mut World) {
    if !world.contains_resource::<Weapons>()
        || !world.contains_resource::<Healths>()
        || !world.contains_resource::<InstalledCatalog>()
        || !world.contains_resource::<UnitTypes>()
    {
        return;
    }
    let mut weapons = world
        .resource::<Weapons>()
        .0
        .keys()
        .copied()
        .collect::<Vec<_>>();
    weapons.sort_unstable();
    for weapon in weapons {
        if world.resource::<Disabled>().0.contains(&weapon) || !unit_exists(world, weapon) {
            continue;
        }
        let Some(parameters) = weapon_parameters(world, weapon) else {
            continue;
        };
        let target = choose_target(world, weapon, &parameters);
        let _ = transaction::transact(world, |tx| {
            tx.mark_m3();
            let mut state = tx.weapon_state(weapon)?;
            if state.cooldown > 0 {
                state.cooldown -= 1;
            }
            if state.cooldown > 0 {
                state.status = WeaponStatus::Cooling;
                tx.stage_weapon(weapon, state);
                return Ok(());
            }

            let Some(target) = target else {
                state.status = WeaponStatus::NoTarget;
                state.last_target = None;
                tx.stage_weapon(weapon, state);
                return Ok(());
            };
            fire(tx, weapon, target, &parameters, &mut state)
        });
    }
}

fn unit_exists(world: &World, unit: u32) -> bool {
    world
        .iter_entities()
        .any(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == unit))
}

fn weapon_parameters(world: &World, weapon: u32) -> Option<WeaponParameters> {
    let entity = world
        .iter_entities()
        .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == weapon))?;
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

fn choose_target(world: &World, weapon: u32, parameters: &WeaponParameters) -> Option<u32> {
    let origin = world
        .iter_entities()
        .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == weapon))?
        .get::<Position>()?
        .0;
    let range_squared = parameters.range * parameters.range;
    let healths = &world.resource::<Healths>().0;
    world
        .iter_entities()
        .filter_map(|entity| {
            let id = entity.get::<UnitId>()?.0;
            let health = healths.get(&id)?;
            if *health == 0 {
                return None;
            }
            let owner = entity.get::<Owner>()?.0.0;
            let relation_matches = match parameters.targets {
                SlotRelation::Other => owner != parameters.owner,
                SlotRelation::Same => owner == parameters.owner,
                SlotRelation::Any => true,
            };
            if !relation_matches {
                return None;
            }
            let distance_squared = entity.get::<Position>()?.0.distance_squared(origin);
            (distance_squared <= range_squared).then_some((distance_squared, id))
        })
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
    state.last_target = (remaining > 0).then_some(target);
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
        ContentCatalog, ContentId, Footprint, MapDef, MapSize, Placement, ResolvedAmmo,
        ResolvedInventory, ResolvedUnit, ResolvedWeapon, ResourceDef, ResourceKind, SlotRelation,
        SourceDef, TargetFilter, WorldPos,
    };

    use crate::{
        EconomyLedger, Position, UnitId,
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
