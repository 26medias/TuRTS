//! Extractor binding state and the fixed-tick extraction phase.

use std::collections::BTreeMap;

use bevy_ecs::{
    prelude::{Resource, World},
    schedule::IntoScheduleConfigs,
};
use turts_mod_api::ContentId;

use super::{
    gather::RateCarry,
    ledger::Endpoint,
    research::Disabled,
    transaction::{extract_from_node, transact},
};
use crate::{
    RejectReason, SimClock, UnitId, bootstrap::InstalledCatalog, ids::NodeId, registry::UnitTypes,
    world::UnitType,
};

/// Current extractor condition.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ExtractorStatus {
    #[default]
    Extracting,
    Full,
    Depleted,
}

/// A drill's bound node, rate remainder, and current condition.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExtractorState {
    pub(crate) node: Option<NodeId>,
    pub(crate) carry: RateCarry,
    pub(crate) status: ExtractorStatus,
}

/// Extractor states indexed by unit id.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Extractors(pub(crate) BTreeMap<u32, ExtractorState>);

/// Install the Extract phase system.
pub(crate) fn install(app: &mut bevy_app::App) {
    app.add_systems(
        bevy_app::FixedUpdate,
        run_extract
            .in_set(crate::SimSet::Extract)
            .run_if(not_stopped),
    );
}

fn not_stopped(world: &World) -> bool {
    !world.contains_resource::<crate::economy::events::MatchStopped>()
}

/// Advance each extractor in ascending unit id order. All state changes are transactional.
pub(crate) fn run_extract(world: &mut World) {
    if !world.contains_resource::<Extractors>() || !world.contains_resource::<InstalledCatalog>() {
        return;
    }
    let units = world
        .resource::<Extractors>()
        .0
        .keys()
        .copied()
        .collect::<Vec<_>>();
    for unit in units {
        let Some(entity) = find_unit(world, unit) else {
            continue;
        };
        let Some(unit_type) = world.get::<UnitType>(entity) else {
            continue;
        };
        let Some(type_entry) = world.resource::<UnitTypes>().get(unit_type.0) else {
            continue;
        };
        let Ok(unit_content) = ContentId::parse(&type_entry.content_id) else {
            continue;
        };
        let Some(definition) = world
            .resource::<InstalledCatalog>()
            .0
            .units
            .get(&unit_content)
        else {
            continue;
        };
        let Some(extractor) = definition.extractor.as_ref() else {
            continue;
        };
        let rate = extractor.per_second;
        let disabled = world.resource::<Disabled>().0.contains(&unit);
        let hz = world.resource::<SimClock>().tick_hz;
        // All per-unit writes, including carry and status, commit together.
        let _ = transact(world, |tx| {
            let mut state = tx.extractor_state(unit)?;
            if disabled {
                state.carry = RateCarry::default();
                tx.stage_extractor(unit, state);
                return Ok(());
            }
            let budget = state.carry.take_budget(rate, hz)?;
            let node_id = state.node.ok_or(RejectReason::UnknownNode)?;
            let node = tx.node(node_id)?;
            let endpoint = Endpoint::Inventory(unit);
            let capacity = tx.inventory(endpoint)?.capacity;
            let used = tx.inventory_used(endpoint)?;
            let free = u128::from(capacity)
                .checked_sub(used)
                .ok_or(RejectReason::IntegrityViolation)?;
            let amount = budget
                .min(node.remaining)
                .min(u64::try_from(free).unwrap_or(u64::MAX));
            let item = tx
                .catalog()?
                .0
                .nodes
                .get(&node.content_id)
                .ok_or(RejectReason::IntegrityViolation)?
                .item
                .clone();
            if amount > 0 {
                extract_from_node(tx, node_id, endpoint, item, amount)?;
            }
            let remaining_after = node.remaining - amount;
            let used_after = used + u128::from(amount);
            state.status = if remaining_after == 0 {
                ExtractorStatus::Depleted
            } else if used_after >= u128::from(capacity) {
                ExtractorStatus::Full
            } else {
                ExtractorStatus::Extracting
            };
            tx.stage_extractor(unit, state);
            Ok(())
        });
    }
}

fn find_unit(world: &World, id: u32) -> Option<bevy_ecs::entity::Entity> {
    world
        .iter_entities()
        .find_map(|entity| (entity.get::<UnitId>()?.0 == id).then_some(entity.id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EconomyLedger, Owner, Position, SlotId, SurfaceNodes, UnitId,
        economy::{
            NodeRecord,
            events::{EconomyEventKind, PendingEconomyEvents},
            ledger::{Account, Inventory},
            research::Disabled,
        },
        ids::NodeId,
    };
    use bevy_math::Vec2;
    use std::collections::{BTreeMap, BTreeSet};
    use turts_mod_api::{
        ContentCatalog, ExtractorDef, Footprint, ResolvedInventory, ResolvedNode, ResolvedUnit,
        ResourceDef, ResourceKind,
    };

    const DRILL: u32 = 7;
    const NODE: u32 = 51;

    fn fixture(hz: u16, rate: u64, remaining: u64, capacity: u64) -> bevy_app::App {
        let mut app = crate::headless_app(hz);
        let ore = ContentId::parse("fixture:ore").unwrap();
        let drill_id = ContentId::parse("fixture:drill").unwrap();
        let node_content = ContentId::parse("fixture:deposit").unwrap();
        let inventory = ResolvedInventory {
            capacity,
            accepts: vec![ore.clone()],
        };
        let drill = ResolvedUnit {
            id: drill_id.clone(),
            name: "Drill".into(),
            footprint: Footprint::Circle { radius: 1.0 },
            movement: None,
            inventory: Some(inventory),
            gatherer: None,
            drop_off: None,
            producer: None,
            output_inventory: None,
            extractor: Some(ExtractorDef {
                per_second: rate,
                interaction_range: 1.0,
            }),
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: None,
            weapon: None,
            removal: None,
        };
        let catalog = ContentCatalog {
            resources: BTreeMap::from([(
                ore.clone(),
                ResourceDef {
                    id: "ore".into(),
                    name: "Ore".into(),
                    unit_name: "ore".into(),
                    kind: ResourceKind::Item,
                },
            )]),
            units: BTreeMap::from([(drill_id.clone(), drill)]),
            nodes: BTreeMap::from([(
                node_content.clone(),
                ResolvedNode {
                    id: node_content.clone(),
                    name: "Deposit".into(),
                    item: ore.clone(),
                    radius: 1.0,
                },
            )]),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            source_files: BTreeMap::new(),
        };
        let world = app.world_mut();
        let types = UnitTypes::from_catalog(&catalog, &UnitTypes::default()).unwrap();
        let unit_type = types.find("fixture:drill").unwrap();
        world.insert_resource(types);
        world.insert_resource(InstalledCatalog(catalog));
        world.insert_resource(SurfaceNodes(vec![NodeRecord {
            id: NodeId(NODE),
            content_id: node_content,
            position: crate::world::Position(Vec2::ZERO),
            remaining,
        }]));
        world.resource_mut::<Extractors>().0.insert(
            DRILL,
            ExtractorState {
                node: Some(NodeId(NODE)),
                ..Default::default()
            },
        );
        world.resource_mut::<EconomyLedger>().inventories.insert(
            DRILL,
            Inventory {
                capacity,
                accepted_items: BTreeSet::from([ore]),
                items: BTreeMap::new(),
            },
        );
        world.spawn((
            UnitId(DRILL),
            Owner(SlotId(0)),
            UnitType(unit_type),
            Position(Vec2::ZERO),
        ));
        app
    }

    fn step(app: &mut bevy_app::App) {
        run_extract(app.world_mut());
    }

    fn ore_count(app: &bevy_app::App) -> u64 {
        let ore = ContentId::parse("fixture:ore").unwrap();
        app.world().resource::<EconomyLedger>().inventories[&DRILL]
            .items
            .get(&ore)
            .map_or(0, |a| a.on_hand)
    }

    fn extracted_events(app: &bevy_app::App) -> usize {
        app.world()
            .resource::<PendingEconomyEvents>()
            .0
            .iter()
            .filter(|event| matches!(event.kind, EconomyEventKind::Extracted { .. }))
            .count()
    }

    #[test]
    fn extractor_rate_with_carry_exact() {
        let mut app = fixture(20, 2, 100, 100);
        let mut ticks = Vec::new();
        for tick in 1..=30 {
            step(&mut app);
            if ore_count(&app) > ticks.len() as u64 {
                ticks.push(tick);
            }
        }
        assert_eq!(ticks, [10, 20, 30]);
    }

    #[test]
    fn extractor_stops_when_full() {
        let mut app = fixture(20, 420, 100, 2);
        let ore = ContentId::parse("fixture:ore").unwrap();
        app.world_mut()
            .resource_mut::<EconomyLedger>()
            .inventories
            .get_mut(&DRILL)
            .unwrap()
            .items
            .insert(
                ore.clone(),
                Account {
                    on_hand: 2,
                    reserved: 0,
                },
            );
        let node_before = app.world().resource::<SurfaceNodes>().0[0].remaining;
        let inventory_before = app.world().resource::<EconomyLedger>().inventories[&DRILL].clone();
        step(&mut app);
        assert_eq!(
            app.world().resource::<Extractors>().0[&DRILL].status,
            ExtractorStatus::Full
        );
        assert_eq!(
            app.world().resource::<SurfaceNodes>().0[0].remaining,
            node_before
        );
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&DRILL],
            inventory_before
        );
        assert_eq!(extracted_events(&app), 0);
    }

    #[test]
    fn extractor_clamps_budget_to_free_capacity() {
        let mut app = fixture(20, 420, 100, 2);
        step(&mut app);
        assert_eq!(ore_count(&app), 2);
        assert_eq!(app.world().resource::<SurfaceNodes>().0[0].remaining, 98);
        assert_eq!(
            app.world().resource::<Extractors>().0[&DRILL].status,
            ExtractorStatus::Full
        );
    }

    #[test]
    fn extractor_depletes() {
        let mut app = fixture(20, 440, 21, 50);
        step(&mut app);
        assert_eq!(ore_count(&app), 21);
        assert_eq!(app.world().resource::<SurfaceNodes>().0[0].remaining, 0);
        assert_eq!(
            app.world().resource::<Extractors>().0[&DRILL].status,
            ExtractorStatus::Depleted
        );
        step(&mut app);
        assert_eq!(ore_count(&app), 21);
        assert_eq!(app.world().resource::<SurfaceNodes>().0[0].remaining, 0);
    }

    #[test]
    fn disabled_extractor_takes_no_budget_and_resets_carry() {
        let mut app = fixture(20, 2, 100, 100);
        {
            let mut extractors = app.world_mut().resource_mut::<Extractors>();
            let carry = &mut extractors.0.get_mut(&DRILL).unwrap().carry;
            for _ in 0..9 {
                carry.take_budget(2, 20).unwrap();
            }
            assert_eq!(carry.remainder(), 18);
        }
        app.world_mut().resource_mut::<Disabled>().0.insert(DRILL);
        for _ in 0..9 {
            step(&mut app);
        }
        assert_eq!(ore_count(&app), 0);
        assert_eq!(
            app.world().resource::<Extractors>().0[&DRILL]
                .carry
                .remainder(),
            0
        );
        app.world_mut().resource_mut::<Disabled>().0.remove(&DRILL);
        for _ in 0..9 {
            step(&mut app);
        }
        assert_eq!(ore_count(&app), 0);
        step(&mut app);
        assert_eq!(ore_count(&app), 1);
    }

    #[test]
    fn failed_extraction_changes_neither_node_nor_inventory() {
        let mut app = fixture(20, 2, 1, 10);
        let ore = ContentId::parse("fixture:ore").unwrap();
        let before_inventory = app.world().resource::<EconomyLedger>().inventories[&DRILL].clone();
        let result = transact(app.world_mut(), |tx| {
            extract_from_node(tx, NodeId(NODE), Endpoint::Inventory(DRILL), ore.clone(), 2)
        });
        assert_eq!(result, Err(RejectReason::NodeDepleted));
        assert_eq!(app.world().resource::<SurfaceNodes>().0[0].remaining, 1);
        assert_eq!(
            app.world().resource::<EconomyLedger>().inventories[&DRILL],
            before_inventory
        );
    }

    #[test]
    fn extractor_rate_at_10_and_30_hz() {
        for (hz, interval) in [(10, 5), (30, 15)] {
            let mut app = fixture(hz, 2, 100, 10);
            let mut ticks = Vec::new();
            for tick in 1..=interval * 3 {
                step(&mut app);
                if ore_count(&app) > ticks.len() as u64 {
                    ticks.push(tick);
                }
            }
            assert_eq!(ticks, [interval, interval * 2, interval * 3]);
        }
    }
}
