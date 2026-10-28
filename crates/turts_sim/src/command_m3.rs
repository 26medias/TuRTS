//! M3 player command policy for removal and unit enable state.

use bevy_ecs::{entity::Entity, world::World};
use turts_mod_api::{ContentId, ResolvedUnit};

use crate::{
    RejectReason,
    bootstrap::InstalledCatalog,
    economy::{
        events::{EconomyEventKind, RemoveCause},
        transaction::{remove_unit, transact},
    },
    registry::UnitTypes,
    slots::SlotId,
    world::{Owner, UnitId, UnitType},
};

fn target(world: &World, slot: SlotId, unit_id: u32) -> Result<(Entity, u8), RejectReason> {
    let entity = world
        .iter_entities()
        .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == unit_id))
        .map(|entity| entity.id())
        .ok_or(RejectReason::UnknownUnit)?;
    let owner = world
        .get::<Owner>(entity)
        .map(|owner| owner.0)
        .ok_or(RejectReason::IntegrityViolation)?;
    if owner != slot {
        return Err(RejectReason::NotOwner);
    }
    Ok((entity, owner.0))
}

fn definition(world: &World, entity: Entity) -> Result<&ResolvedUnit, RejectReason> {
    let type_id = world
        .get::<UnitType>(entity)
        .ok_or(RejectReason::IntegrityViolation)?
        .0;
    let content_id = &world
        .resource::<UnitTypes>()
        .get(type_id)
        .ok_or(RejectReason::IntegrityViolation)?
        .content_id;
    let id = ContentId::parse(content_id).map_err(|_| RejectReason::IntegrityViolation)?;
    world
        .resource::<InstalledCatalog>()
        .0
        .units
        .get(&id)
        .ok_or(RejectReason::IntegrityViolation)
}

pub(crate) fn remove(world: &mut World, slot: SlotId, unit_id: u32) -> Result<(), RejectReason> {
    let (entity, _) = target(world, slot, unit_id)?;
    if !definition(world, entity)?
        .removal
        .as_ref()
        .is_some_and(|policy| policy.by_command)
    {
        return Err(RejectReason::NotRemovable);
    }
    transact(world, |tx| remove_unit(tx, unit_id, RemoveCause::Removed))
}

pub(crate) fn set_enabled(
    world: &mut World,
    slot: SlotId,
    unit_id: u32,
    enabled: bool,
) -> Result<(), RejectReason> {
    let (entity, owner) = target(world, slot, unit_id)?;
    let unit = definition(world, entity)?;
    let toggleable = unit.extractor.is_some()
        || unit.inserter.is_some()
        || unit.machine.is_some()
        || unit.research.is_some()
        || unit.producer.is_some()
        || unit.weapon.is_some();
    if !toggleable {
        return Err(RejectReason::NotToggleable);
    }

    let disable = !enabled;
    let reset_extractor = disable && unit.extractor.is_some();
    transact(world, |tx| {
        if tx.disabled_contains(unit_id) == disable {
            return Ok(());
        }
        tx.stage_disabled(unit_id, disable);
        if reset_extractor {
            let mut state = tx.extractor_state(unit_id)?;
            state.carry = Default::default();
            tx.stage_extractor(unit_id, state);
        }
        tx.stage_event(
            owner,
            EconomyEventKind::EnabledChanged {
                unit: unit_id,
                enabled,
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AuditTrail, Command, CommandKind, apply_command,
        bootstrap::InstalledCatalog,
        economy::{
            events::{NextEventOrdinal, PendingEconomyEvents},
            extractor::{ExtractorState, Extractors},
            ledger::{Account, EconomyLedger, Inventory},
            research::Disabled,
        },
        headless_app,
        world::{Position, UnitType},
    };
    use bevy_math::Vec2;
    use std::collections::{BTreeMap, BTreeSet};
    use turts_mod_api::{
        ContentCatalog, ContentId, ContentsPolicy, ConveyorDef, ExtractorDef, Footprint,
        InventoryDef, PackageId, RemovalDef, ResolvedConveyor, ResolvedInventory, ResolvedUnit,
        ResourceDef, ResourceKind, UnitDef, UnitRegistry,
    };

    #[derive(Clone, Copy)]
    enum Kind {
        Drill,
        Belt,
        Drone,
    }

    fn fixture(kind: Kind, removal: Option<bool>) -> (bevy_app::App, u32) {
        let mut app = headless_app(20);
        let package = PackageId::parse("test").unwrap();
        let unit_id = ContentId::new(&package, "fixture");
        let ore = ContentId::new(&package, "ore");
        let (extractor, conveyor) = match kind {
            Kind::Drill => (
                Some(ExtractorDef {
                    per_second: 1,
                    interaction_range: 1.0,
                }),
                None,
            ),
            Kind::Belt => (
                None,
                Some(ResolvedConveyor {
                    slots_per_piece: 1,
                    slots_per_second: 1,
                    accepts: vec![],
                }),
            ),
            Kind::Drone => (None, None),
        };
        let mut types = UnitTypes::default();
        let type_id = types
            .register_unit_type(
                &package,
                UnitDef {
                    id: "fixture".into(),
                    name: "Fixture".into(),
                    footprint: Footprint::Circle { radius: 0.5 },
                    movement: None,
                    inventory: matches!(kind, Kind::Drill).then(|| InventoryDef {
                        capacity: 10,
                        accepts: vec!["ore".into()],
                    }),
                    gatherer: None,
                    drop_off: None,
                    producer: None,
                    output_inventory: None,
                    extractor: extractor.clone(),
                    conveyor: conveyor.as_ref().map(|c| ConveyorDef {
                        slots_per_piece: c.slots_per_piece,
                        slots_per_second: c.slots_per_second,
                        accepts: vec![],
                    }),
                    inserter: None,
                    machine: None,
                    research: None,
                    health: None,
                    weapon: None,
                    removal: removal.map(|by_command| RemovalDef {
                        by_command,
                        contents: ContentsPolicy::Destroy,
                    }),
                },
            )
            .unwrap();
        let mut catalog = ContentCatalog {
            resources: BTreeMap::from([(
                ore.clone(),
                ResourceDef {
                    id: "ore".into(),
                    name: "Ore".into(),
                    unit_name: "ore".into(),
                    kind: ResourceKind::Item,
                },
            )]),
            units: BTreeMap::new(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            source_files: BTreeMap::new(),
        };
        catalog.units.insert(
            unit_id.clone(),
            ResolvedUnit {
                id: unit_id,
                name: "Fixture".into(),
                footprint: Footprint::Circle { radius: 0.5 },
                movement: None,
                inventory: matches!(kind, Kind::Drill).then(|| ResolvedInventory {
                    capacity: 10,
                    accepts: vec![ore.clone()],
                }),
                gatherer: None,
                drop_off: None,
                producer: None,
                output_inventory: None,
                extractor,
                conveyor,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: removal.map(|by_command| RemovalDef {
                    by_command,
                    contents: ContentsPolicy::Destroy,
                }),
            },
        );
        app.world_mut().insert_resource(types);
        app.world_mut().insert_resource(InstalledCatalog(catalog));
        if matches!(kind, Kind::Drill) {
            app.world_mut()
                .resource_mut::<EconomyLedger>()
                .inventories
                .insert(
                    7,
                    Inventory {
                        capacity: 10,
                        accepted_items: BTreeSet::from([ore.clone()]),
                        items: BTreeMap::from([(ore.clone(), Account::default())]),
                    },
                );
        }
        app.world_mut().resource_mut::<NextEventOrdinal>().0 = Some(0);
        app.world_mut().resource_mut::<AuditTrail>().enable();
        app.world_mut().spawn((
            UnitId(7),
            Owner(SlotId(0)),
            UnitType(type_id),
            Position(Vec2::ZERO),
        ));
        if matches!(kind, Kind::Drill) {
            app.world_mut()
                .resource_mut::<Extractors>()
                .0
                .insert(7, ExtractorState::default());
        }
        (app, 7)
    }

    fn command(slot: u8, kind: CommandKind) -> Command {
        Command {
            slot: SlotId(slot),
            seq: 1,
            kind,
        }
    }

    #[test]
    fn remove_requires_ownership_then_removal_flag() {
        let (mut app, unit) = fixture(Kind::Drone, Some(false));
        assert_eq!(
            apply_command(
                app.world_mut(),
                &command(1, CommandKind::Remove { unit_id: unit })
            )
            .outcome,
            Err(RejectReason::NotOwner)
        );
        assert_eq!(
            apply_command(
                app.world_mut(),
                &command(0, CommandKind::Remove { unit_id: unit })
            )
            .outcome,
            Err(RejectReason::NotRemovable)
        );
    }

    #[test]
    fn remove_drone_by_command_rejected_not_removable() {
        let (mut app, unit) = fixture(Kind::Drone, Some(false));
        assert_eq!(
            apply_command(
                app.world_mut(),
                &command(0, CommandKind::Remove { unit_id: unit })
            )
            .outcome,
            Err(RejectReason::NotRemovable)
        );
    }

    #[test]
    fn set_enabled_noop_when_same_value() {
        let (mut app, unit) = fixture(Kind::Drill, None);
        let disabled_before = app.world().resource::<Disabled>().clone();
        let extractor_before = app.world().resource::<Extractors>().clone();
        let result = apply_command(
            app.world_mut(),
            &command(
                0,
                CommandKind::SetEnabled {
                    unit_id: unit,
                    enabled: true,
                },
            ),
        );
        assert_eq!(result.outcome, Ok(crate::Applied { job_id: None }));
        let audit = &app.world().resource::<AuditTrail>().commands[0];
        assert_eq!(audit.before, audit.after);
        assert_eq!(*app.world().resource::<Disabled>(), disabled_before);
        assert_eq!(*app.world().resource::<Extractors>(), extractor_before);
        assert!(app.world().resource::<PendingEconomyEvents>().0.is_empty());
    }

    #[test]
    fn set_enabled_not_toggleable_for_belt() {
        let (mut app, unit) = fixture(Kind::Belt, None);
        for enabled in [true, false] {
            assert_eq!(
                apply_command(
                    app.world_mut(),
                    &command(
                        0,
                        CommandKind::SetEnabled {
                            unit_id: unit,
                            enabled
                        },
                    )
                )
                .outcome,
                Err(RejectReason::NotToggleable)
            );
        }
    }

    #[test]
    fn cross_owner_commands_rejected_not_owner() {
        let (mut app, unit) = fixture(Kind::Drill, Some(true));
        assert_eq!(
            apply_command(
                app.world_mut(),
                &command(1, CommandKind::Remove { unit_id: unit })
            )
            .outcome,
            Err(RejectReason::NotOwner)
        );
        assert_eq!(
            apply_command(
                app.world_mut(),
                &command(
                    1,
                    CommandKind::SetEnabled {
                        unit_id: unit,
                        enabled: false
                    }
                )
            )
            .outcome,
            Err(RejectReason::NotOwner)
        );
    }

    #[test]
    fn remove_command_applies_declared_policy() {
        let (mut app, unit) = fixture(Kind::Drone, Some(true));
        assert_eq!(
            apply_command(
                app.world_mut(),
                &command(0, CommandKind::Remove { unit_id: unit })
            )
            .outcome,
            Ok(crate::Applied { job_id: None })
        );
        assert!(
            !app.world()
                .iter_entities()
                .any(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == unit))
        );
    }

    #[test]
    fn disabling_extractor_resets_carry_and_stages_event_atomically() {
        let (mut app, unit) = fixture(Kind::Drill, None);
        {
            let mut extractors = app.world_mut().resource_mut::<Extractors>();
            let carry = &mut extractors.0.get_mut(&unit).unwrap().carry;
            carry.take_budget(1, 20).unwrap();
            assert_ne!(carry.remainder(), 0);
        }
        let before = app.world().resource::<AuditTrail>().commands.len();
        assert_eq!(
            apply_command(
                app.world_mut(),
                &command(
                    0,
                    CommandKind::SetEnabled {
                        unit_id: unit,
                        enabled: false
                    }
                )
            )
            .outcome,
            Ok(crate::Applied { job_id: None })
        );
        assert!(app.world().resource::<Disabled>().0.contains(&unit));
        assert_eq!(
            app.world().resource::<Extractors>().0[&unit]
                .carry
                .remainder(),
            0
        );
        assert_eq!(
            app.world().resource::<AuditTrail>().commands.len(),
            before + 1
        );
        assert!(
            app.world()
                .resource::<PendingEconomyEvents>()
                .0
                .iter()
                .any(|event| event.kind
                    == EconomyEventKind::EnabledChanged {
                        unit,
                        enabled: false
                    })
        );
    }
}
