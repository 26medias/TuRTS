//! Inserter arm state and the fixed-tick transfer phase.

use std::collections::{BTreeMap, BTreeSet};

use bevy_app::{App, FixedUpdate};
use bevy_ecs::{prelude::Resource, schedule::IntoScheduleConfigs, world::World};
use turts_mod_api::ContentId;

use super::{
    ledger::Endpoint,
    research::Disabled,
    transaction::{Transaction, transact},
};
use crate::{
    RejectReason, SimClock, UnitId, boot_m3::try_ms_to_ticks, bootstrap::InstalledCatalog,
    registry::UnitTypes, world::UnitType,
};

/// Current arm phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArmStage {
    AtSource(Option<ArmWait>),
    ToDestination,
    AtDestination(ArmWait),
    Returning,
}

impl Default for ArmStage {
    fn default() -> Self {
        Self::AtSource(None)
    }
}

/// Why an arm is waiting at one end of its cycle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ArmWait {
    #[default]
    NoSource,
    NoDestination,
    SourceEmpty,
    DestinationFull,
}

/// Resolved endpoints and progress for one inserter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct InserterState {
    pub(crate) source: Option<u32>,
    pub(crate) destination: Option<u32>,
    pub(crate) stage: ArmStage,
    pub(crate) elapsed: u32,
}

/// Inserter state indexed by unit id.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Inserters(pub(crate) BTreeMap<u32, InserterState>);

#[derive(Debug, Clone)]
enum EndpointKind {
    Inventory(Endpoint),
    Belt {
        piece: u32,
        accepts: BTreeSet<ContentId>,
    },
}

/// Install the Inserters phase system.
pub(crate) fn install(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        run_inserters
            .in_set(crate::plugin::SimSet::Inserters)
            .run_if(not_stopped),
    );
}

fn not_stopped(
    stopped: Option<bevy_ecs::prelude::Res<crate::economy::events::MatchStopped>>,
) -> bool {
    stopped.is_none()
}

/// Process each arm once in ascending unit id order.
pub(crate) fn run_inserters(world: &mut World) {
    if !world.contains_resource::<Inserters>()
        || !world.contains_resource::<InstalledCatalog>()
        || !world.contains_resource::<UnitTypes>()
    {
        return;
    }
    let mut units = world
        .resource::<Inserters>()
        .0
        .keys()
        .copied()
        .collect::<Vec<_>>();
    units.sort_unstable();
    if units.is_empty() {
        return;
    }
    let tick_hz = u32::from(world.resource::<SimClock>().tick_hz);
    for unit in units {
        if world.resource::<Disabled>().0.contains(&unit) {
            continue;
        }
        let Some(snapshot) = world.resource::<Inserters>().0.get(&unit).cloned() else {
            continue;
        };
        let Some((swing_ticks, return_ticks, hand_accepts)) = arm_parameters(world, unit, tick_hz)
        else {
            continue;
        };
        // Endpoint ids are immutable for the tick. `remove_unit` clears a removed endpoint
        // before this phase runs, and the transaction reads the authoritative arm row again.
        let source = snapshot.source.map(|id| endpoint_kind(world, id, true));
        let destination = snapshot
            .destination
            .map(|id| endpoint_kind(world, id, false));
        let _ = transact(world, |tx| {
            let mut state = tx.inserter_state(unit)?;
            match state.stage {
                ArmStage::AtSource(_) => {
                    pick_item(
                        tx,
                        unit,
                        &mut state,
                        source.as_ref(),
                        destination.as_ref(),
                        &hand_accepts,
                    )?;
                }
                ArmStage::ToDestination => {
                    state.elapsed = state
                        .elapsed
                        .checked_add(1)
                        .ok_or(RejectReason::ArithmeticOverflow)?;
                    if state.elapsed == swing_ticks {
                        drop_item(tx, unit, &mut state, destination.as_ref())?;
                    }
                }
                ArmStage::AtDestination(_) => {
                    drop_item(tx, unit, &mut state, destination.as_ref())?;
                }
                ArmStage::Returning => {
                    state.elapsed = state
                        .elapsed
                        .checked_add(1)
                        .ok_or(RejectReason::ArithmeticOverflow)?;
                    if state.elapsed == return_ticks {
                        state.stage = ArmStage::AtSource(None);
                        state.elapsed = 0;
                        pick_item(
                            tx,
                            unit,
                            &mut state,
                            source.as_ref(),
                            destination.as_ref(),
                            &hand_accepts,
                        )?;
                    }
                }
            }
            tx.stage_inserter(unit, state);
            Ok(())
        });
    }
}

fn arm_parameters(
    world: &World,
    unit: u32,
    tick_hz: u32,
) -> Option<(u32, u32, BTreeSet<ContentId>)> {
    let entity = find_unit(world, unit)?;
    let unit_type = world.get::<UnitType>(entity)?;
    let type_entry = world.resource::<UnitTypes>().get(unit_type.0)?;
    let content = ContentId::parse(&type_entry.content_id).ok()?;
    let definition = world.resource::<InstalledCatalog>().0.units.get(&content)?;
    let inserter = definition.inserter.as_ref()?;
    let swing = u32::try_from(try_ms_to_ticks(inserter.swing_ms, tick_hz)?).ok()?;
    let returning = u32::try_from(try_ms_to_ticks(inserter.return_ms, tick_hz)?).ok()?;
    let hand = definition.inventory.as_ref()?;
    Some((swing, returning, hand.accepts.iter().cloned().collect()))
}

fn endpoint_kind(world: &World, unit: u32, source: bool) -> Option<EndpointKind> {
    let entity = find_unit(world, unit)?;
    let unit_type = world.get::<UnitType>(entity)?;
    let type_entry = world.resource::<UnitTypes>().get(unit_type.0)?;
    let content = ContentId::parse(&type_entry.content_id).ok()?;
    let definition = world.resource::<InstalledCatalog>().0.units.get(&content)?;
    if let Some(conveyor) = &definition.conveyor {
        return Some(EndpointKind::Belt {
            piece: unit,
            accepts: conveyor.accepts.iter().cloned().collect(),
        });
    }
    if source && definition.output_inventory.is_some() {
        Some(EndpointKind::Inventory(Endpoint::Output(unit)))
    } else if definition.inventory.is_some() {
        Some(EndpointKind::Inventory(Endpoint::Inventory(unit)))
    } else {
        None
    }
}

fn find_unit(world: &World, id: u32) -> Option<bevy_ecs::entity::Entity> {
    world
        .iter_entities()
        .find_map(|entity| (entity.get::<UnitId>()?.0 == id).then_some(entity.id()))
}

fn pick_item(
    tx: &mut Transaction<'_>,
    unit: u32,
    state: &mut InserterState,
    source: Option<&Option<EndpointKind>>,
    destination: Option<&Option<EndpointKind>>,
    hand_accepts: &BTreeSet<ContentId>,
) -> Result<(), RejectReason> {
    let Some(Some(source)) = source else {
        state.stage = ArmStage::AtSource(Some(ArmWait::NoSource));
        state.elapsed = 0;
        return Ok(());
    };
    let Some(Some(destination)) = destination else {
        state.stage = ArmStage::AtSource(Some(ArmWait::NoDestination));
        state.elapsed = 0;
        return Ok(());
    };
    let hand_endpoint = Endpoint::Inventory(unit);
    let hand = tx.inventory(hand_endpoint)?;
    let hand_free = u128::from(hand.capacity)
        .checked_sub(tx.inventory_used(hand_endpoint)?)
        .ok_or(RejectReason::IntegrityViolation)?;
    if hand_free == 0 {
        state.stage = ArmStage::AtSource(Some(ArmWait::SourceEmpty));
        state.elapsed = 0;
        return Ok(());
    }

    match source {
        EndpointKind::Inventory(source_endpoint) => {
            let source_inventory = tx.inventory(*source_endpoint)?;
            let candidates = source_inventory
                .items
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>();
            let mut saw_candidate = false;
            for item in candidates {
                if !hand_accepts.contains(&item) || !destination_accepts(tx, destination, &item)? {
                    continue;
                }
                let account = tx.account(*source_endpoint, &item);
                let available = account
                    .on_hand
                    .checked_sub(account.reserved)
                    .ok_or(RejectReason::IntegrityViolation)?;
                if available == 0 {
                    continue;
                }
                saw_candidate = true;
                if !destination_has_room(tx, destination, &item)? {
                    continue;
                }
                let amount = available.min(u64::try_from(hand_free).unwrap_or(u64::MAX));
                let mut source_after = account;
                source_after.on_hand -= amount;
                let mut hand_after = tx.account(hand_endpoint, &item);
                hand_after.on_hand = hand_after
                    .on_hand
                    .checked_add(amount)
                    .ok_or(RejectReason::ArithmeticOverflow)?;
                tx.stage_account(*source_endpoint, item.clone(), source_after);
                tx.stage_account(hand_endpoint, item, hand_after);
                state.stage = ArmStage::ToDestination;
                state.elapsed = 0;
                return Ok(());
            }
            state.stage = ArmStage::AtSource(Some(if saw_candidate {
                ArmWait::DestinationFull
            } else {
                ArmWait::SourceEmpty
            }));
        }
        EndpointKind::Belt { piece, .. } => {
            let (line_id, piece_index) = tx.conveyor_piece(*piece)?;
            let mut line = tx.conveyor_line(line_id)?;
            let slots_per_piece = usize::from(line.slots_per_piece);
            let start = piece_index
                .checked_mul(slots_per_piece)
                .ok_or(RejectReason::ArithmeticOverflow)?;
            let end = start
                .checked_add(slots_per_piece)
                .ok_or(RejectReason::ArithmeticOverflow)?;
            if end > line.slots.len() {
                return Err(RejectReason::IntegrityViolation);
            }
            let mut selected = None;
            for index in (start..end).rev() {
                let item_ix = line.slots[index];
                if item_ix == 0 {
                    continue;
                }
                let item = tx
                    .item_content(item_ix)
                    .ok_or(RejectReason::IntegrityViolation)?;
                if hand_accepts.contains(&item) && destination_accepts(tx, destination, &item)? {
                    selected = Some((index, item));
                    break;
                }
            }
            if let Some((index, item)) = selected {
                if destination_has_room(tx, destination, &item)? {
                    let mut hand_after = tx.account(hand_endpoint, &item);
                    hand_after.on_hand = hand_after
                        .on_hand
                        .checked_add(1)
                        .ok_or(RejectReason::ArithmeticOverflow)?;
                    line.slots[index] = 0;
                    tx.stage_account(hand_endpoint, item, hand_after);
                    tx.stage_conveyor_line(line_id, line);
                    state.stage = ArmStage::ToDestination;
                    state.elapsed = 0;
                    return Ok(());
                }
                state.stage = ArmStage::AtSource(Some(ArmWait::DestinationFull));
            } else {
                state.stage = ArmStage::AtSource(Some(ArmWait::SourceEmpty));
            }
        }
    }
    state.elapsed = 0;
    Ok(())
}

fn destination_accepts(
    tx: &Transaction<'_>,
    destination: &EndpointKind,
    item: &ContentId,
) -> Result<bool, RejectReason> {
    match destination {
        EndpointKind::Inventory(endpoint) => {
            Ok(tx.inventory(*endpoint)?.accepted_items.contains(item))
        }
        EndpointKind::Belt { accepts, .. } => Ok(accepts.contains(item)),
    }
}

fn destination_has_room(
    tx: &Transaction<'_>,
    destination: &EndpointKind,
    item: &ContentId,
) -> Result<bool, RejectReason> {
    match destination {
        EndpointKind::Inventory(endpoint) => {
            let inventory = tx.inventory(*endpoint)?;
            Ok(inventory.accepted_items.contains(item)
                && tx.inventory_used(*endpoint)? < u128::from(inventory.capacity))
        }
        EndpointKind::Belt { piece, accepts } => {
            if !accepts.contains(item) {
                return Ok(false);
            }
            let (line_id, piece_index) = tx.conveyor_piece(*piece)?;
            let line = tx.conveyor_line(line_id)?;
            let first_slot = piece_index
                .checked_mul(usize::from(line.slots_per_piece))
                .ok_or(RejectReason::ArithmeticOverflow)?;
            let slot = line
                .slots
                .get(first_slot)
                .ok_or(RejectReason::IntegrityViolation)?;
            Ok(*slot == 0)
        }
    }
}

fn drop_item(
    tx: &mut Transaction<'_>,
    unit: u32,
    state: &mut InserterState,
    destination: Option<&Option<EndpointKind>>,
) -> Result<(), RejectReason> {
    let Some(Some(destination)) = destination else {
        state.stage = ArmStage::AtDestination(ArmWait::NoDestination);
        return Ok(());
    };
    let hand_endpoint = Endpoint::Inventory(unit);
    let hand = tx.inventory(hand_endpoint)?;
    let item = hand.items.keys().find_map(|item| {
        let account = tx.account(hand_endpoint, item);
        (account.on_hand > account.reserved).then(|| item.clone())
    });
    let Some(item) = item else {
        state.stage = ArmStage::Returning;
        state.elapsed = 0;
        return Ok(());
    };
    match destination {
        EndpointKind::Inventory(destination_endpoint) => {
            let inventory = tx.inventory(*destination_endpoint)?;
            if !inventory.accepted_items.contains(&item) {
                state.stage = ArmStage::AtDestination(ArmWait::NoDestination);
                return Ok(());
            }
            let used = tx.inventory_used(*destination_endpoint)?;
            let free = u128::from(inventory.capacity)
                .checked_sub(used)
                .ok_or(RejectReason::IntegrityViolation)?;
            let held = tx.account(hand_endpoint, &item);
            let held_available = held
                .on_hand
                .checked_sub(held.reserved)
                .ok_or(RejectReason::IntegrityViolation)?;
            let amount = held_available.min(u64::try_from(free).unwrap_or(u64::MAX));
            if amount == 0 {
                state.stage = ArmStage::AtDestination(ArmWait::DestinationFull);
                return Ok(());
            }
            let mut hand_after = held;
            hand_after.on_hand -= amount;
            let mut destination_after = tx.account(*destination_endpoint, &item);
            destination_after.on_hand = destination_after
                .on_hand
                .checked_add(amount)
                .ok_or(RejectReason::ArithmeticOverflow)?;
            tx.stage_account(hand_endpoint, item.clone(), hand_after);
            tx.stage_account(*destination_endpoint, item, destination_after);
            if hand_after.on_hand == 0 {
                state.stage = ArmStage::Returning;
                state.elapsed = 0;
            } else {
                state.stage = ArmStage::AtDestination(ArmWait::DestinationFull);
            }
        }
        EndpointKind::Belt { piece, accepts } => {
            if !accepts.contains(&item) {
                state.stage = ArmStage::AtDestination(ArmWait::NoDestination);
                return Ok(());
            }
            let (line_id, piece_index) = tx.conveyor_piece(*piece)?;
            let mut line = tx.conveyor_line(line_id)?;
            let first_slot = piece_index
                .checked_mul(usize::from(line.slots_per_piece))
                .ok_or(RejectReason::ArithmeticOverflow)?;
            let slot = line
                .slots
                .get(first_slot)
                .ok_or(RejectReason::IntegrityViolation)?;
            if *slot != 0 {
                state.stage = ArmStage::AtDestination(ArmWait::DestinationFull);
                return Ok(());
            }
            let item_ix = tx
                .item_index(&item)
                .ok_or(RejectReason::IntegrityViolation)?;
            let mut held = tx.account(hand_endpoint, &item);
            let amount = held
                .on_hand
                .checked_sub(held.reserved)
                .ok_or(RejectReason::IntegrityViolation)?;
            if amount == 0 {
                state.stage = ArmStage::Returning;
                state.elapsed = 0;
                return Ok(());
            }
            line.slots[first_slot] = item_ix;
            held.on_hand -= 1;
            tx.stage_account(hand_endpoint, item, held);
            tx.stage_conveyor_line(line_id, line);
            if held.on_hand == 0 {
                state.stage = ArmStage::Returning;
                state.elapsed = 0;
            } else {
                state.stage = ArmStage::AtDestination(ArmWait::DestinationFull);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Owner, Position, SlotId, UnitId,
        bootstrap::InstalledCatalog,
        economy::{
            conveyor::{BeltLine, Conveyors, ItemTable},
            events::RemoveCause,
            ledger::{Account, EconomyLedger, Inventory},
            research::Disabled,
            transaction::{remove_unit, transact},
        },
        registry::UnitTypes,
        world::UnitType,
    };
    use bevy_math::Vec2;
    use std::collections::BTreeMap;
    use turts_mod_api::{
        ContentCatalog, ContentId, Footprint, InserterDef, ResolvedConveyor, ResolvedInventory,
        ResolvedUnit, ResourceDef, ResourceKind,
    };

    const ARM: u32 = 100;
    const ARM2: u32 = 101;
    const SOURCE: u32 = 10;
    const DESTINATION: u32 = 20;
    const BELT_FIRST: u32 = 30;
    const BELT_SECOND: u32 = 31;

    fn id(name: &str) -> ContentId {
        ContentId::parse(&format!("probe:{name}")).unwrap()
    }

    fn resolved_unit(
        unit_id: ContentId,
        inventory: Option<ResolvedInventory>,
        output_inventory: Option<ResolvedInventory>,
        inserter: Option<InserterDef>,
        conveyor: Option<ResolvedConveyor>,
    ) -> ResolvedUnit {
        ResolvedUnit {
            id: unit_id,
            name: "Fixture unit".into(),
            footprint: Footprint::Circle { radius: 0.25 },
            movement: None,
            inventory,
            gatherer: None,
            drop_off: None,
            producer: None,
            output_inventory,
            extractor: None,
            conveyor,
            inserter,
            machine: None,
            research: None,
            health: None,
            weapon: None,
            removal: None,
        }
    }

    fn inventory(capacity: u64, accepts: &[ContentId], on_hand: u64) -> Inventory {
        Inventory {
            capacity,
            accepted_items: accepts.iter().cloned().collect(),
            items: BTreeMap::from([(
                accepts[0].clone(),
                Account {
                    on_hand,
                    reserved: 0,
                },
            )]),
        }
    }

    fn fixture(
        hz: u16,
        hand_capacity: u64,
        source_amount: u64,
        destination_capacity: u64,
        destination_amount: u64,
    ) -> bevy_app::App {
        let ore = id("ore");
        let coal = id("coal");
        let arm_def = resolved_unit(
            id("arm"),
            Some(ResolvedInventory {
                capacity: hand_capacity,
                accepts: vec![ore.clone(), coal.clone()],
            }),
            None,
            Some(InserterDef {
                reach: 1.0,
                swing_ms: 250,
                return_ms: 250,
            }),
            None,
        );
        let source_def = resolved_unit(
            id("source"),
            Some(ResolvedInventory {
                capacity: 10,
                accepts: vec![ore.clone(), coal.clone()],
            }),
            None,
            None,
            None,
        );
        let destination_def = resolved_unit(
            id("destination"),
            Some(ResolvedInventory {
                capacity: destination_capacity,
                accepts: vec![ore.clone(), coal.clone()],
            }),
            None,
            None,
            None,
        );
        let output_source_def = resolved_unit(
            id("output_source"),
            Some(ResolvedInventory {
                capacity: 10,
                accepts: vec![ore.clone(), coal.clone()],
            }),
            Some(ResolvedInventory {
                capacity: 10,
                accepts: vec![ore.clone(), coal.clone()],
            }),
            None,
            None,
        );
        let belt_def = resolved_unit(
            id("belt"),
            None,
            None,
            None,
            Some(ResolvedConveyor {
                slots_per_piece: 2,
                slots_per_second: 0,
                accepts: vec![ore.clone(), coal.clone()],
            }),
        );
        let catalog = ContentCatalog {
            resources: BTreeMap::from([
                (
                    ore.clone(),
                    ResourceDef {
                        id: "ore".into(),
                        name: "Ore".into(),
                        unit_name: "ore".into(),
                        kind: ResourceKind::Item,
                    },
                ),
                (
                    coal.clone(),
                    ResourceDef {
                        id: "coal".into(),
                        name: "Coal".into(),
                        unit_name: "coal".into(),
                        kind: ResourceKind::Item,
                    },
                ),
            ]),
            units: BTreeMap::from([
                (arm_def.id.clone(), arm_def),
                (source_def.id.clone(), source_def),
                (destination_def.id.clone(), destination_def),
                (output_source_def.id.clone(), output_source_def),
                (belt_def.id.clone(), belt_def),
            ]),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            source_files: BTreeMap::new(),
        };
        let unit_types = UnitTypes::from_catalog(&catalog, &UnitTypes::default()).unwrap();
        let type_id = |name: &str| unit_types.find(&format!("probe:{name}")).unwrap();
        let fixture_types = [
            type_id("arm"),
            type_id("source"),
            type_id("destination"),
            type_id("output_source"),
            type_id("belt"),
        ];
        let mut app = crate::headless_app(hz);
        let world = app.world_mut();
        world.insert_resource(unit_types);
        world.insert_resource(InstalledCatalog(catalog));
        for (unit, unit_type) in [
            (ARM, fixture_types[0]),
            (SOURCE, fixture_types[1]),
            (DESTINATION, fixture_types[2]),
            (11, fixture_types[3]),
            (BELT_FIRST, fixture_types[4]),
            (BELT_SECOND, fixture_types[4]),
            (ARM2, fixture_types[0]),
        ] {
            world.spawn((
                UnitId(unit),
                Owner(SlotId(0)),
                Position(Vec2::new(unit as f32, 0.0)),
                UnitType(unit_type),
            ));
        }
        world.resource_mut::<EconomyLedger>().inventories.extend([
            (
                ARM,
                inventory(hand_capacity, &[ore.clone(), coal.clone()], 0),
            ),
            (
                ARM2,
                inventory(hand_capacity, &[ore.clone(), coal.clone()], 0),
            ),
            (
                SOURCE,
                inventory(10, &[ore.clone(), coal.clone()], source_amount),
            ),
            (
                DESTINATION,
                inventory(
                    destination_capacity,
                    &[ore.clone(), coal.clone()],
                    destination_amount,
                ),
            ),
            (11, inventory(10, &[ore.clone(), coal.clone()], 7)),
        ]);
        let output = inventory(10, &[ore.clone(), coal.clone()], 1);
        world
            .resource_mut::<EconomyLedger>()
            .output_inventories
            .insert(11, output);
        world.insert_resource(Inserters(BTreeMap::from([(
            ARM,
            InserterState {
                source: Some(SOURCE),
                destination: Some(DESTINATION),
                stage: ArmStage::AtSource(None),
                elapsed: 0,
            },
        )])));
        world.insert_resource(ItemTable(vec![coal, ore]));
        world.resource_mut::<Conveyors>().lines.insert(
            BELT_FIRST,
            BeltLine {
                def: id("belt"),
                owner: 0,
                pieces: vec![BELT_FIRST, BELT_SECOND],
                slots: vec![0; 4],
                slots_per_piece: 2,
                slots_per_second: 0,
                carry: crate::economy::gather::RateCarry::default(),
            },
        );
        world.resource_mut::<Conveyors>().piece_line.extend([
            (BELT_FIRST, (BELT_FIRST, 0)),
            (BELT_SECOND, (BELT_FIRST, 1)),
        ]);
        app
    }

    fn tick(app: &mut bevy_app::App) {
        app.update();
    }

    fn hand_amount(app: &bevy_app::App, arm: u32, item: &ContentId) -> u64 {
        app.world().resource::<EconomyLedger>().inventories[&arm]
            .items
            .get(item)
            .map_or(0, |account| account.on_hand)
    }

    #[test]
    fn period_is_ten_ticks() {
        let ore = id("ore");
        let mut app = fixture(20, 1, 2, 10, 0);
        tick(&mut app); // tick 1: pick
        assert_eq!(hand_amount(&app, ARM, &ore), 1);
        assert_eq!(
            app.world().resource::<Inserters>().0[&ARM].stage,
            ArmStage::ToDestination
        );
        for _ in 0..4 {
            tick(&mut app);
        }
        assert_eq!(hand_amount(&app, ARM, &ore), 1, "no drop before t+5");
        tick(&mut app); // tick 6: drop
        assert_eq!(hand_amount(&app, ARM, &ore), 0);
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&DESTINATION].items[&ore].on_hand,
            1
        );
        for _ in 0..4 {
            tick(&mut app);
        }
        assert_eq!(hand_amount(&app, ARM, &ore), 0, "no next pick before t+10");
        tick(&mut app); // tick 11: return completes and picks
        assert_eq!(hand_amount(&app, ARM, &ore), 1);
    }

    #[test]
    fn pick_requires_destination_room_now() {
        let ore = id("ore");
        let mut app = fixture(20, 2, 2, 2, 2);
        tick(&mut app);
        assert_eq!(hand_amount(&app, ARM, &ore), 0);
        assert_eq!(
            app.world().resource::<Inserters>().0[&ARM].stage,
            ArmStage::AtSource(Some(ArmWait::DestinationFull))
        );
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .inventories
            .get_mut(&DESTINATION)
            .unwrap()
            .items
            .get_mut(&ore)
            .unwrap()
            .on_hand -= 1;
        tick(&mut app);
        assert_eq!(
            hand_amount(&app, ARM, &ore),
            2,
            "K7 checks positive room but does not reserve or clamp it"
        );
    }

    #[test]
    fn held_item_kept_when_destination_fills() {
        let ore = id("ore");
        let mut app = fixture(20, 2, 2, 2, 0);
        tick(&mut app); // take two while room exists
        assert_eq!(hand_amount(&app, ARM, &ore), 2);
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .inventories
            .get_mut(&DESTINATION)
            .unwrap()
            .items
            .get_mut(&ore)
            .unwrap()
            .on_hand = 1;
        for _ in 0..5 {
            tick(&mut app);
        }
        assert_eq!(
            hand_amount(&app, ARM, &ore),
            1,
            "only the one free destination slot is dropped"
        );
        assert_eq!(
            app.world().resource::<Inserters>().0[&ARM].stage,
            ArmStage::AtDestination(ArmWait::DestinationFull)
        );
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .inventories
            .get_mut(&DESTINATION)
            .unwrap()
            .items
            .get_mut(&ore)
            .unwrap()
            .on_hand -= 1;
        tick(&mut app);
        assert_eq!(hand_amount(&app, ARM, &ore), 0);
    }

    #[test]
    fn belt_drop_only_into_first_slot_of_piece() {
        let mut app = fixture(20, 1, 1, 1, 0);
        app.world_mut()
            .resource_mut::<Inserters>()
            .0
            .get_mut(&ARM)
            .unwrap()
            .destination = Some(BELT_SECOND);
        {
            let mut conveyors = app.world_mut().resource_mut::<Conveyors>();
            conveyors.lines.get_mut(&BELT_FIRST).unwrap().slots[3] = 1;
        }
        tick(&mut app);
        for _ in 0..5 {
            tick(&mut app);
        }
        let line = &app.world().resource::<Conveyors>().lines[&BELT_FIRST];
        assert_eq!(line.slots, vec![0, 0, 2, 1]);
    }

    #[test]
    fn belt_pick_takes_downstream_most_item() {
        let ore = id("ore");
        let mut app = fixture(20, 1, 0, 10, 0);
        app.world_mut()
            .resource_mut::<Inserters>()
            .0
            .get_mut(&ARM)
            .unwrap()
            .source = Some(BELT_FIRST);
        app.world_mut()
            .resource_mut::<Conveyors>()
            .lines
            .get_mut(&BELT_FIRST)
            .unwrap()
            .slots = vec![1, 2, 1, 0];
        tick(&mut app);
        assert_eq!(
            hand_amount(&app, ARM, &ore),
            1,
            "pick the downstream-most item in this piece"
        );
        assert_eq!(
            app.world().resource::<Conveyors>().lines[&BELT_FIRST].slots,
            vec![1, 0, 1, 0]
        );
    }

    #[test]
    fn lowest_unit_id_wins_shared_source() {
        let ore = id("ore");
        let mut app = fixture(20, 1, 1, 10, 0);
        app.world_mut().resource_mut::<Inserters>().0.insert(
            ARM2,
            InserterState {
                source: Some(SOURCE),
                destination: Some(DESTINATION),
                stage: ArmStage::AtSource(None),
                elapsed: 0,
            },
        );
        tick(&mut app);
        assert_eq!(hand_amount(&app, ARM, &ore), 1);
        assert_eq!(hand_amount(&app, ARM2, &ore), 0);
        assert_eq!(
            app.world().resource::<Inserters>().0[&ARM2].stage,
            ArmStage::AtSource(Some(ArmWait::SourceEmpty))
        );
    }

    #[test]
    fn target_removed_sets_endpoint_none_and_hand_kept() {
        let ore = id("ore");
        let mut app = fixture(20, 1, 1, 10, 0);
        tick(&mut app);
        assert_eq!(hand_amount(&app, ARM, &ore), 1);
        transact(app.world_mut(), |tx| {
            tx.mark_m3();
            remove_unit(tx, DESTINATION, RemoveCause::Removed)
        })
        .unwrap();
        let state = &app.world().resource::<Inserters>().0[&ARM];
        assert_eq!(state.destination, None);
        assert_eq!(hand_amount(&app, ARM, &ore), 1);
        for _ in 0..5 {
            tick(&mut app);
        }
        assert_eq!(hand_amount(&app, ARM, &ore), 1);
        assert_eq!(
            app.world().resource::<Inserters>().0[&ARM].stage,
            ArmStage::AtDestination(ArmWait::NoDestination)
        );
    }

    #[test]
    fn disabled_inserter_frozen() {
        let ore = id("ore");
        let mut app = fixture(20, 1, 2, 10, 0);
        tick(&mut app);
        let frozen = app.world().resource::<Inserters>().0[&ARM].clone();
        app.world_mut().resource_mut::<Disabled>().0.insert(ARM);
        for _ in 0..8 {
            tick(&mut app);
        }
        assert_eq!(app.world().resource::<Inserters>().0[&ARM], frozen);
        assert_eq!(hand_amount(&app, ARM, &ore), 1);
        app.world_mut().resource_mut::<Disabled>().0.remove(&ARM);
        tick(&mut app);
        assert_eq!(hand_amount(&app, ARM, &ore), 1);
    }

    #[test]
    fn hand_cap_two_moves_two_items() {
        let ore = id("ore");
        let mut app = fixture(20, 2, 2, 10, 0);
        tick(&mut app);
        assert_eq!(hand_amount(&app, ARM, &ore), 2);
        for _ in 0..5 {
            tick(&mut app);
        }
        assert_eq!(hand_amount(&app, ARM, &ore), 0);
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&DESTINATION].items[&ore].on_hand,
            2
        );
    }

    #[test]
    fn output_inventory_is_the_source_when_declared() {
        let ore = id("ore");
        let mut app = fixture(20, 1, 0, 10, 0);
        app.world_mut()
            .resource_mut::<Inserters>()
            .0
            .get_mut(&ARM)
            .unwrap()
            .source = Some(11);
        tick(&mut app);
        assert_eq!(hand_amount(&app, ARM, &ore), 1);
        assert_eq!(
            app.world().resource::<EconomyLedger>().output_inventories[&11].items[&ore].on_hand,
            0
        );
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&11].items[&ore].on_hand,
            7
        );
    }

    #[test]
    fn inventory_and_arm_write_share_one_transaction() {
        let ore = id("ore");
        let mut app = fixture(20, 1, 2, 10, 0);
        app.world_mut()
            .resource_mut::<crate::economy::transaction::AuditTrail>()
            .enable();
        tick(&mut app);
        let trail = app
            .world()
            .resource::<crate::economy::transaction::AuditTrail>();
        let changed_inventory_operations = trail
            .operations
            .iter()
            .filter(|operation| {
                operation.before.economy.economy.inventories
                    != operation.after.economy.economy.inventories
            })
            .collect::<Vec<_>>();
        assert_eq!(changed_inventory_operations.len(), 1);
        let inventory_amount = |views: &[crate::economy::views::InventoryView], unit| {
            views
                .iter()
                .find(|view| view.unit == unit)
                .and_then(|view| view.items.iter().find(|item| item.resource == ore))
                .map_or(0, |item| item.on_hand)
        };
        let operation = changed_inventory_operations[0];
        let before = &operation.before.economy.economy.inventories;
        let after = &operation.after.economy.economy.inventories;
        assert_eq!(inventory_amount(before, SOURCE), 2);
        assert_eq!(inventory_amount(before, ARM), 0);
        assert_eq!(inventory_amount(after, SOURCE), 1);
        assert_eq!(inventory_amount(after, ARM), 1);
    }

    #[test]
    fn reserved_source_stock_is_not_pickable() {
        let ore = id("ore");
        let mut app = fixture(20, 1, 1, 10, 0);
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .inventories
            .get_mut(&SOURCE)
            .unwrap()
            .items
            .get_mut(&ore)
            .unwrap()
            .reserved = 1;
        tick(&mut app);
        assert_eq!(hand_amount(&app, ARM, &ore), 0);
        assert_eq!(
            app.world().resource::<Inserters>().0[&ARM].stage,
            ArmStage::AtSource(Some(ArmWait::SourceEmpty))
        );
    }

    #[test]
    fn missing_declared_output_does_not_fall_back_to_main() {
        let ore = id("ore");
        let mut app = fixture(20, 1, 0, 10, 0);
        app.world_mut()
            .resource_mut::<Inserters>()
            .0
            .get_mut(&ARM)
            .unwrap()
            .source = Some(11);
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .output_inventories
            .remove(&11);
        tick(&mut app);
        assert_eq!(hand_amount(&app, ARM, &ore), 0);
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&11].items[&ore].on_hand,
            7
        );
    }

    #[test]
    fn duration_boundaries_use_ceil_at_ten_and_thirty_hz() {
        for (hz, swing, next_pick) in [(10, 3, 7), (30, 8, 17)] {
            let ore = id("ore");
            let mut app = fixture(hz, 1, 2, 10, 0);
            tick(&mut app);
            for _ in 1..swing {
                tick(&mut app);
                assert_eq!(
                    hand_amount(&app, ARM, &ore),
                    1,
                    "drop must not precede ceil swing tick {swing} at {hz} Hz"
                );
            }
            tick(&mut app);
            assert_eq!(
                hand_amount(&app, ARM, &ore),
                0,
                "drop occurs on ceil swing tick {swing} at {hz} Hz"
            );
            for _ in swing + 1..next_pick - 1 {
                tick(&mut app);
                assert_eq!(
                    hand_amount(&app, ARM, &ore),
                    0,
                    "next pick must not precede tick {next_pick} at {hz} Hz"
                );
            }
            tick(&mut app);
            assert_eq!(
                hand_amount(&app, ARM, &ore),
                1,
                "pick occurs at tick {next_pick} at {hz} Hz"
            );
        }
    }
}
