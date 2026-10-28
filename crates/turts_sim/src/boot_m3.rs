//! Prepared M3 capability rows and map structures installed by package boot.

use std::collections::BTreeMap;

use bevy_ecs::world::World;
use bevy_math::Vec2;
use turts_mod_api::{
    ContentCatalog, ContentId, Facing, Footprint, ModError, ResolvedInventory, ResolvedUnit,
    ResourceKind,
};

use crate::{
    RejectReason,
    bootstrap::InstalledCatalog,
    economy::{
        NodeRecord, SurfaceNodes,
        combat::{Healths, WeaponState, Weapons},
        conveyor::{BeltLine, Conveyors, ItemTable},
        extractor::{ExtractorState, Extractors},
        gather::RateCarry,
        inserter::{InserterState, Inserters},
        ledger::{EconomyLedger, Inventory},
        production::ProductionQueues,
        research::{Disabled, ResearchState},
    },
    ids::{IdAllocators, NodeId},
    map::{check_circle_bounds, check_overlap},
    registry::UnitTypes,
    world::{Owner, Position, UnitId, UnitType},
};

#[derive(Debug, Clone)]
pub(crate) struct PreparedCapabilityRows {
    pub(crate) inventory: Option<Inventory>,
    pub(crate) output_inventory: Option<Inventory>,
    pub(crate) extractor: Option<ExtractorState>,
    pub(crate) inserter: Option<InserterState>,
    pub(crate) weapon: Option<WeaponState>,
    pub(crate) health: Option<u64>,
    pub(crate) has_queue: bool,
}

fn prepared_inventory(definition: &ResolvedInventory) -> Inventory {
    Inventory {
        capacity: definition.capacity,
        accepted_items: definition.accepts.iter().cloned().collect(),
        items: BTreeMap::new(),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CapabilityBinding {
    pub(crate) extractor_node: Option<NodeId>,
    pub(crate) inserter_source: Option<u32>,
    pub(crate) inserter_destination: Option<u32>,
}

/// Build every M3 side-table row without changing the world. Callers can validate the full
/// operation before committing other state, then pass the result to `apply_capability_rows`.
pub(crate) fn install_capability_rows(
    definition: &ResolvedUnit,
    binding: CapabilityBinding,
) -> PreparedCapabilityRows {
    PreparedCapabilityRows {
        inventory: definition.inventory.as_ref().map(prepared_inventory),
        output_inventory: definition.output_inventory.as_ref().map(prepared_inventory),
        extractor: definition.extractor.as_ref().map(|_| ExtractorState {
            node: binding.extractor_node,
            carry: RateCarry::default(),
            status: Default::default(),
        }),
        inserter: definition.inserter.as_ref().map(|_| InserterState {
            source: binding.inserter_source,
            destination: binding.inserter_destination,
            ..Default::default()
        }),
        weapon: definition.weapon.as_ref().map(|_| WeaponState::default()),
        health: definition.health.map(|health| health.max),
        has_queue: definition.producer.is_some()
            || definition.machine.is_some()
            || definition.research.is_some(),
    }
}

/// Install prevalidated rows. The operation is infallible after the caller begins mutation.
pub(crate) fn apply_capability_rows(world: &mut World, id: u32, rows: PreparedCapabilityRows) {
    if let Some(inventory) = rows.inventory {
        world
            .resource_mut::<EconomyLedger>()
            .inventories
            .entry(id)
            .or_insert(inventory);
    }
    if let Some(inventory) = rows.output_inventory {
        world
            .resource_mut::<EconomyLedger>()
            .output_inventories
            .insert(id, inventory);
    }
    if let Some(row) = rows.extractor {
        world.resource_mut::<Extractors>().0.insert(id, row);
    }
    if let Some(row) = rows.inserter {
        world.resource_mut::<Inserters>().0.insert(id, row);
    }
    if let Some(row) = rows.weapon {
        world.resource_mut::<Weapons>().0.insert(id, row);
    }
    if let Some(health) = rows.health {
        world.resource_mut::<Healths>().0.insert(id, health);
    }
    if rows.has_queue {
        world
            .resource_mut::<ProductionQueues>()
            .0
            .entry(id)
            .or_default();
    }
}

/// Resolve bindings for a unit that is about to spawn. This reads only current state and the
/// proposed position; callers must prepare the resulting rows before transaction commit begins.
pub(crate) fn resolve_spawn_capability_binding(
    world: &World,
    definition: &ResolvedUnit,
    position: Vec2,
    owner_slot: u8,
    facing: Option<Facing>,
) -> Result<CapabilityBinding, crate::RejectReason> {
    let catalog = world
        .get_resource::<InstalledCatalog>()
        .ok_or(RejectReason::IntegrityViolation)?;
    let types = world
        .get_resource::<UnitTypes>()
        .ok_or(RejectReason::IntegrityViolation)?;
    let mut geometry = Vec::new();
    for entity in world.iter_entities() {
        let (Some(id), Some(kind), Some(owner), Some(pos)) = (
            entity.get::<UnitId>(),
            entity.get::<UnitType>(),
            entity.get::<Owner>(),
            entity.get::<Position>(),
        ) else {
            continue;
        };
        let entry = types.get(kind.0).ok_or(RejectReason::IntegrityViolation)?;
        let content =
            ContentId::parse(&entry.content_id).map_err(|_| RejectReason::IntegrityViolation)?;
        let unit = catalog
            .0
            .units
            .get(&content)
            .ok_or(RejectReason::IntegrityViolation)?;
        geometry.push(BootUnit {
            id: id.0,
            owner: owner.0.0,
            pos: pos.0,
            radius: entry.radius,
            is_inserter: unit.inserter.is_some(),
            has_main_inventory: unit.inventory.is_some(),
            has_output_inventory: unit.output_inventory.is_some(),
            is_conveyor: unit.conveyor.is_some(),
        });
    }
    let mut binding = CapabilityBinding::default();
    if let Some(extractor) = &definition.extractor {
        let inventory = definition
            .inventory
            .as_ref()
            .ok_or(RejectReason::IntegrityViolation)?;
        let Footprint::Circle { radius } = definition.footprint;
        binding.extractor_node = Some(
            bind_extractor_node(
                &world
                    .get_resource::<SurfaceNodes>()
                    .ok_or(RejectReason::IntegrityViolation)?
                    .0,
                &catalog.0,
                position,
                radius,
                extractor.interaction_range,
                &inventory.accepts,
            )
            .map_err(|_| RejectReason::IntegrityViolation)?,
        );
    }
    if let Some(inserter) = &definition.inserter {
        let facing = facing.ok_or(RejectReason::IntegrityViolation)?;
        let direction = facing_vector(facing);
        let pick = position - direction * inserter.reach;
        let drop = position + direction * inserter.reach;
        binding.inserter_source = Some(
            resolve_endpoint(&geometry, pick, owner_slot, true)
                .map_err(|_| RejectReason::IntegrityViolation)?,
        );
        binding.inserter_destination = Some(
            resolve_endpoint(&geometry, drop, owner_slot, false)
                .map_err(|_| RejectReason::IntegrityViolation)?,
        );
    }
    Ok(binding)
}

#[derive(Debug, Default)]
pub(crate) struct PreparedMapRows {
    pub(crate) extra_units: Vec<(u32, UnitType, u8, Vec2)>,
    pub(crate) conveyors: Conveyors,
    pub(crate) item_table: ItemTable,
    pub(crate) capability_rows: Vec<(u32, PreparedCapabilityRows)>,
}

pub(crate) struct MapStageContext<'a> {
    pub(crate) nodes: &'a [NodeRecord],
    pub(crate) types: &'a UnitTypes,
    pub(crate) allocators: &'a mut IdAllocators,
    pub(crate) existing_spatial: &'a mut Vec<(Vec2, f32, String)>,
}

pub(crate) fn stage_m3_map(
    catalog: &ContentCatalog,
    map: &turts_mod_api::MapDef,
    source_file: &str,
    units: &[(u32, turts_mod_api::UnitTypeId, u8, Vec2)],
    context: MapStageContext<'_>,
) -> Result<PreparedMapRows, ModError> {
    let MapStageContext {
        nodes,
        types,
        allocators,
        existing_spatial,
    } = context;
    let package = catalog
        .units
        .keys()
        .next()
        .or_else(|| catalog.nodes.keys().next())
        .or_else(|| catalog.resources.keys().next())
        .or_else(|| catalog.production.keys().next())
        .or_else(|| catalog.recipes.keys().next())
        .or_else(|| catalog.technologies.keys().next())
        .map(|id| id.package.0.clone())
        .or_else(|| {
            map.belts
                .first()
                .and_then(|line_def| ContentId::parse(&line_def.conveyor).ok())
                .map(|id| id.package.0)
        })
        .unwrap_or_default();
    let error = |field: &str, message: String| {
        ModError::new(&package, field, message).with_file(source_file)
    };
    let mut prepared = PreparedMapRows::default();
    let mut capability_rows = Vec::new();
    prepared.item_table =
        sorted_item_table(catalog).map_err(|message| error("resources", message))?;

    let mut geometry = Vec::<BootUnit>::with_capacity(units.len());
    for (id, kind, owner, pos) in units {
        let entry = types
            .get(*kind)
            .ok_or_else(|| error("placements", "unknown unit type".into()))?;
        let content =
            ContentId::parse(&entry.content_id).map_err(|message| error("placements", message))?;
        let definition = catalog
            .units
            .get(&content)
            .ok_or_else(|| error("placements", format!("unknown unit definition `{content}`")))?;
        geometry.push(BootUnit {
            id: *id,
            owner: *owner,
            pos: *pos,
            radius: entry.radius,
            is_inserter: definition.inserter.is_some(),
            has_main_inventory: definition.inventory.is_some(),
            has_output_inventory: definition.output_inventory.is_some(),
            is_conveyor: definition.conveyor.is_some(),
        });
    }

    for (line_index, line_def) in map.belts.iter().enumerate() {
        let field = format!("belts[{line_index}]");
        let conveyor_id = ContentId::parse(&line_def.conveyor)
            .map_err(|message| error(&format!("{field}.conveyor"), message))?;
        let conveyor = catalog
            .units
            .get(&conveyor_id)
            .and_then(|unit| unit.conveyor.as_ref())
            .ok_or_else(|| {
                error(
                    &format!("{field}.conveyor"),
                    format!("`{conveyor_id}` has no conveyor capability"),
                )
            })?;
        let unit_type = types.find(&conveyor_id.to_string()).ok_or_else(|| {
            error(
                &format!("{field}.conveyor"),
                format!("unknown unit type `{conveyor_id}`"),
            )
        })?;
        let entry = types.get(unit_type).expect("resolved conveyor type exists");
        let conveyor_definition = &catalog.units[&conveyor_id];
        let Footprint::Circle { radius } = catalog.units[&conveyor_id].footprint;
        let pitch = radius * 2.0;
        let direction = facing_vector(line_def.facing);
        if line_def.pieces == 0 {
            return Err(error(&format!("{field}.pieces"), "must be positive".into()));
        }
        let slot_count = usize::from(line_def.pieces)
            .checked_mul(usize::from(conveyor.slots_per_piece))
            .ok_or_else(|| {
                error(
                    &format!("{field}.pieces"),
                    "belt slot count overflows".into(),
                )
            })?;
        let start = Vec2::new(line_def.start.x, line_def.start.y);
        let first_id = allocators.next_values().0;
        let mut pieces = Vec::with_capacity(usize::from(line_def.pieces));
        for piece_index in 0..line_def.pieces {
            let id = allocators
                .allocate_unit()
                .map_err(|message| error(&format!("{field}.pieces"), message))?;
            let pos = start + direction * (pitch * f32::from(piece_index));
            check_circle_bounds(pos, radius, map.size.x, map.size.y)
                .map_err(|message| error(&format!("{field}.start"), message))?;
            check_overlap(pos, radius, existing_spatial)
                .map_err(|message| error(&format!("{field}.start"), message))?;
            existing_spatial.push((pos, radius, format!("{conveyor_id} piece {piece_index}")));
            prepared
                .extra_units
                .push((id, UnitType(unit_type), line_def.owner_slot, pos));
            capability_rows.push((
                id,
                install_capability_rows(conveyor_definition, CapabilityBinding::default()),
            ));
            pieces.push(id);
            geometry.push(BootUnit {
                id,
                owner: line_def.owner_slot,
                pos,
                radius: entry.radius,
                is_inserter: false,
                has_main_inventory: catalog.units[&conveyor_id].inventory.is_some(),
                has_output_inventory: catalog.units[&conveyor_id].output_inventory.is_some(),
                is_conveyor: true,
            });
        }
        let mut by_piece = BTreeMap::new();
        for (index, piece) in pieces.iter().copied().enumerate() {
            by_piece.insert(
                piece,
                (first_id, u16::try_from(index).expect("piece count is u16")),
            );
        }
        prepared.conveyors.piece_line.extend(by_piece);
        prepared.conveyors.lines.insert(
            first_id,
            BeltLine {
                def: conveyor_id,
                owner: line_def.owner_slot,
                pieces,
                slots: vec![0; slot_count],
                slots_per_piece: conveyor.slots_per_piece,
                slots_per_second: conveyor.slots_per_second,
                carry: RateCarry::default(),
            },
        );
    }

    let mut rows = Vec::with_capacity(geometry.len());
    for (placement_index, placement) in map.placements.iter().enumerate() {
        let unit_id = ContentId::parse(&placement.unit)
            .map_err(|message| error(&format!("placements[{placement_index}].unit"), message))?;
        let definition = catalog.units.get(&unit_id).ok_or_else(|| {
            error(
                &format!("placements[{placement_index}].unit"),
                format!("unknown unit `{unit_id}`"),
            )
        })?;
        let (id, _, owner, pos) = units[placement_index];
        let mut binding = CapabilityBinding::default();
        if let Some(extractor) = &definition.extractor {
            let inventory = definition.inventory.as_ref().ok_or_else(|| {
                error(
                    &format!("placements[{placement_index}].unit"),
                    "extractor requires an inventory".into(),
                )
            })?;
            let unit_radius =
                entry_radius(types, &unit_id).expect("registered extractor has a radius");
            binding.extractor_node = Some(
                bind_extractor_node(
                    nodes,
                    catalog,
                    pos,
                    unit_radius,
                    extractor.interaction_range,
                    &inventory.accepts,
                )
                .map_err(|message| {
                    error(&format!("placements[{placement_index}].unit"), message)
                })?,
            );
        }
        if let Some(inserter) = &definition.inserter {
            let facing = map.placements[placement_index].facing.ok_or_else(|| {
                error(
                    &format!("placements[{placement_index}].facing"),
                    "inserter requires a facing".into(),
                )
            })?;
            let direction = facing_vector(facing);
            let pick = pos - direction * inserter.reach;
            let drop = pos + direction * inserter.reach;
            binding.inserter_source = Some(
                resolve_endpoint(&geometry, pick, owner, true).map_err(|message| {
                    error(&format!("placements[{placement_index}].facing"), message)
                })?,
            );
            binding.inserter_destination = Some(
                resolve_endpoint(&geometry, drop, owner, false).map_err(|message| {
                    error(&format!("placements[{placement_index}].facing"), message)
                })?,
            );
        }
        rows.push((id, install_capability_rows(definition, binding)));
    }
    capability_rows.extend(rows);
    prepared.capability_rows = capability_rows;
    Ok(prepared)
}

#[derive(Debug, Clone)]
struct BootUnit {
    id: u32,
    owner: u8,
    pos: Vec2,
    radius: f32,
    is_inserter: bool,
    has_main_inventory: bool,
    has_output_inventory: bool,
    is_conveyor: bool,
}

fn sorted_item_table(catalog: &ContentCatalog) -> Result<ItemTable, String> {
    let mut items = catalog
        .resources
        .iter()
        .filter_map(|(id, resource)| (resource.kind == ResourceKind::Item).then_some(id.clone()))
        .collect::<Vec<_>>();
    items.sort();
    if items.len() > usize::from(u16::MAX) {
        return Err(format!(
            "world contains {} items, maximum is {}",
            items.len(),
            u16::MAX
        ));
    }
    Ok(ItemTable(items))
}

fn bind_extractor_node(
    nodes: &[NodeRecord],
    catalog: &ContentCatalog,
    position: Vec2,
    unit_radius: f32,
    interaction_range: f32,
    accepts: &[ContentId],
) -> Result<NodeId, String> {
    let (node, item) = nearest_node(nodes, catalog, position, unit_radius, interaction_range)
        .ok_or_else(|| "extractor has no node in range".to_string())?;
    if !accepts.contains(&item) {
        return Err(format!(
            "nearest node {} yields `{item}`, which the extractor inventory does not accept",
            node.0
        ));
    }
    Ok(node)
}

fn nearest_node(
    nodes: &[NodeRecord],
    catalog: &ContentCatalog,
    position: Vec2,
    unit_radius: f32,
    interaction_range: f32,
) -> Option<(NodeId, ContentId)> {
    nodes
        .iter()
        .filter_map(|node| {
            let node_def = catalog.nodes.get(&node.content_id)?;
            let distance_squared = position.distance_squared(node.position.0);
            let reach = unit_radius + node_def.radius + interaction_range;
            (distance_squared <= reach * reach).then_some((node.id, distance_squared))
        })
        .min_by(|(left_id, left_distance), (right_id, right_distance)| {
            left_distance
                .total_cmp(right_distance)
                .then(left_id.cmp(right_id))
        })
        .and_then(|(id, _)| {
            nodes.iter().find(|node| node.id == id).and_then(|node| {
                catalog
                    .nodes
                    .get(&node.content_id)
                    .map(|definition| (id, definition.item.clone()))
            })
        })
}

fn resolve_endpoint(
    units: &[BootUnit],
    point: Vec2,
    owner: u8,
    source: bool,
) -> Result<u32, String> {
    let mut matches = units
        .iter()
        .filter(|unit| {
            !unit.is_inserter && unit.pos.distance_squared(point) <= unit.radius * unit.radius
        })
        .collect::<Vec<_>>();
    matches.sort_by_key(|unit| unit.id);
    let [unit] = matches.as_slice() else {
        return Err(if matches.is_empty() {
            format!(
                "{} endpoint at ({}, {}) matches no unit",
                if source { "source" } else { "destination" },
                point.x,
                point.y
            )
        } else {
            format!(
                "{} endpoint at ({}, {}) matches multiple units",
                if source { "source" } else { "destination" },
                point.x,
                point.y
            )
        });
    };
    if unit.owner != owner {
        return Err(format!(
            "endpoint unit {} is owned by slot {}, expected slot {owner}",
            unit.id, unit.owner
        ));
    }
    let usable = if source {
        unit.has_output_inventory || unit.has_main_inventory || unit.is_conveyor
    } else {
        unit.has_main_inventory || unit.is_conveyor
    };
    if !usable {
        return Err(format!(
            "endpoint unit {} has no compatible inventory or conveyor",
            unit.id
        ));
    }
    Ok(unit.id)
}

fn entry_radius(types: &UnitTypes, id: &ContentId) -> Option<f32> {
    let unit_type = types.find(&id.to_string())?;
    types.get(unit_type).map(|entry| entry.radius)
}

pub(crate) fn facing_vector(facing: Facing) -> Vec2 {
    match facing {
        Facing::PosX => Vec2::X,
        Facing::NegX => Vec2::NEG_X,
        Facing::PosY => Vec2::Y,
        Facing::NegY => Vec2::NEG_Y,
    }
}

#[allow(dead_code)] // used by the focused boot conversion regression and later M3 phases
pub(crate) fn ms_to_ticks(ms: u64, tick_hz: u32) -> u64 {
    try_ms_to_ticks(ms, tick_hz).expect("millisecond duration exceeds u64 tick range")
}

pub(crate) fn try_ms_to_ticks(ms: u64, tick_hz: u32) -> Option<u64> {
    if tick_hz == 0 {
        return None;
    }
    let ticks = u128::from(ms)
        .checked_mul(u128::from(tick_hz))?
        .checked_add(999)?
        / 1000;
    u64::try_from(ticks).ok()
}

pub(crate) fn initialize_m3_resources(world: &mut World) {
    world.init_resource::<Conveyors>();
    world.init_resource::<ItemTable>();
    world.init_resource::<Inserters>();
    world.init_resource::<Extractors>();
    world.init_resource::<Weapons>();
    world.init_resource::<Healths>();
    world.init_resource::<ResearchState>();
    world.init_resource::<Disabled>();
}

#[cfg(test)]
mod tests {
    use super::{
        BootUnit, CapabilityBinding, bind_extractor_node, install_capability_rows, ms_to_ticks,
        nearest_node, resolve_endpoint, resolve_spawn_capability_binding, sorted_item_table,
    };
    use crate::{
        IdAllocators, UnitTypes,
        economy::{
            NodeRecord, SurfaceNodes, combat::Healths, extractor::Extractors, ledger::EconomyLedger,
        },
        world::Position,
    };
    use bevy_math::Vec2;
    use std::collections::BTreeMap;
    use turts_mod_api::{
        ContentCatalog, ContentId, Footprint, HealthDef, MovementDef, ResolvedInventory,
        ResolvedNode, ResolvedUnit, ResourceDef, ResourceKind,
    };

    #[test]
    fn ms_to_ticks_ceils_at_10_20_30_hz() {
        assert_eq!(ms_to_ticks(250, 10), 3);
        assert_eq!(ms_to_ticks(250, 20), 5);
        assert_eq!(ms_to_ticks(250, 30), 8);
        assert_eq!(ms_to_ticks(2_000, 10), 20);
        assert_eq!(ms_to_ticks(2_000, 20), 40);
        assert_eq!(ms_to_ticks(2_000, 30), 60);
    }

    #[test]
    fn boot_item_table_is_sorted_and_zero_remains_empty() {
        let item_z = ContentId::parse("forge:zeta").unwrap();
        let item_a = ContentId::parse("forge:alpha").unwrap();
        let mut catalog = empty_catalog();
        catalog
            .resources
            .insert(item_z.clone(), resource(&item_z, ResourceKind::Item));
        catalog
            .resources
            .insert(item_a.clone(), resource(&item_a, ResourceKind::Item));
        let table = sorted_item_table(&catalog).unwrap();
        assert_eq!(table.content(0), None);
        assert_eq!(table.content(1), Some(&item_a));
        assert_eq!(table.content(2), Some(&item_z));
        assert_eq!(table.item_ix(&item_z), Some(2));
    }

    #[test]
    fn item_table_limit_counts_all_items_and_rejects_u16_overflow() {
        let mut catalog = empty_catalog();
        for index in 0..=usize::from(u16::MAX) {
            let local = format!("item_{index:05}");
            let id = ContentId::parse(&format!("forge:{local}")).unwrap();
            catalog
                .resources
                .insert(id.clone(), resource(&id, ResourceKind::Item));
        }
        assert!(
            sorted_item_table(&catalog)
                .unwrap_err()
                .contains("65536 items")
        );
    }

    #[test]
    fn inserter_endpoints_resolve_as_table() {
        let mut units = vec![
            (1, 6.0, 8.0, 1.2, 0, false, true, false, false),
            (2, 8.0, 8.0, 0.4, 0, true, true, false, false),
            (3, 15.0, 8.0, 0.4, 0, true, true, false, false),
            (4, 17.0, 8.0, 1.2, 0, false, true, true, false),
            (5, 17.0, 10.0, 0.4, 0, true, true, false, false),
            (6, 17.0, 6.0, 0.4, 0, true, true, false, false),
            (7, 17.0, 12.0, 1.2, 0, false, true, false, false),
            (8, 17.0, 4.0, 1.2, 0, false, true, false, false),
            (9, 24.0, 12.0, 0.6, 1, false, false, false, false),
            (10, 30.0, 4.0, 0.6, 1, false, false, false, false),
        ]
        .into_iter()
        .map(as_boot_unit)
        .collect::<Vec<_>>();
        for (offset, x) in (9..=14).enumerate() {
            units.push(as_boot_unit((
                11 + offset as u32,
                x as f32,
                8.0,
                0.5,
                0,
                false,
                false,
                false,
                true,
            )));
        }
        let cases = [
            ((7.0, 8.0), true, 1),
            ((9.0, 8.0), false, 11),
            ((14.0, 8.0), true, 16),
            ((16.0, 8.0), false, 4),
            ((17.0, 9.0), true, 4),
            ((17.0, 11.0), false, 7),
            ((17.0, 7.0), true, 4),
            ((17.0, 5.0), false, 8),
        ];
        for ((x, y), source, expected) in cases {
            assert_eq!(
                resolve_endpoint(&units, Vec2::new(x, y), 0, source),
                Ok(expected)
            );
        }
    }

    #[test]
    fn ambiguous_endpoint_matches_reject_even_at_tangent_boundaries() {
        let units = [
            as_boot_unit((1, -1.0, 0.0, 1.0, 0, false, true, false, false)),
            as_boot_unit((2, 1.0, 0.0, 1.0, 0, false, true, false, false)),
        ];
        assert!(
            resolve_endpoint(&units, Vec2::ZERO, 0, true)
                .unwrap_err()
                .contains("matches multiple units")
        );
    }

    fn as_boot_unit(row: (u32, f32, f32, f32, u8, bool, bool, bool, bool)) -> BootUnit {
        BootUnit {
            id: row.0,
            owner: row.4,
            pos: Vec2::new(row.1, row.2),
            radius: row.3,
            is_inserter: row.5,
            has_main_inventory: row.6,
            has_output_inventory: row.7,
            is_conveyor: row.8,
        }
    }

    #[test]
    fn extractor_chooses_nearest_then_node_id_before_item_compatibility() {
        let ore = ContentId::parse("forge:ore").unwrap();
        let first = ContentId::parse("forge:vein_a").unwrap();
        let second = ContentId::parse("forge:vein_b").unwrap();
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
            units: BTreeMap::new(),
            nodes: BTreeMap::from([
                (
                    first.clone(),
                    ResolvedNode {
                        id: first.clone(),
                        name: "A".into(),
                        item: ore.clone(),
                        radius: 1.0,
                    },
                ),
                (
                    second.clone(),
                    ResolvedNode {
                        id: second.clone(),
                        name: "B".into(),
                        item: ore.clone(),
                        radius: 1.0,
                    },
                ),
            ]),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            source_files: BTreeMap::new(),
        };
        let nodes = SurfaceNodes(vec![
            NodeRecord {
                id: crate::NodeId(1),
                content_id: first.clone(),
                position: Position(Vec2::new(3.0, 0.0)),
                remaining: 10,
            },
            NodeRecord {
                id: crate::NodeId(2),
                content_id: second.clone(),
                position: Position(Vec2::new(1.0, 0.0)),
                remaining: 10,
            },
        ]);
        assert_eq!(
            nearest_node(&nodes.0, &catalog, Vec2::ZERO, 1.0, 2.0),
            Some((crate::NodeId(2), ore.clone()))
        );
        let tied = SurfaceNodes(vec![
            NodeRecord {
                id: crate::NodeId(9),
                content_id: second.clone(),
                position: Position(Vec2::new(1.0, 0.0)),
                remaining: 10,
            },
            NodeRecord {
                id: crate::NodeId(3),
                content_id: first.clone(),
                position: Position(Vec2::new(-1.0, 0.0)),
                remaining: 10,
            },
        ]);
        assert_eq!(
            nearest_node(&tied.0, &catalog, Vec2::ZERO, 1.0, 0.0),
            Some((crate::NodeId(3), ore.clone()))
        );

        let incompatible = ContentId::parse("forge:vein_c").unwrap();
        let stone = ContentId::parse("forge:stone").unwrap();
        let mut catalog = catalog;
        catalog.nodes.insert(
            incompatible.clone(),
            ResolvedNode {
                id: incompatible.clone(),
                name: "Stone".into(),
                item: stone.clone(),
                radius: 1.0,
            },
        );
        let candidates = SurfaceNodes(vec![
            NodeRecord {
                id: crate::NodeId(7),
                content_id: second,
                position: Position(Vec2::new(1.0, 0.0)),
                remaining: 10,
            },
            NodeRecord {
                id: crate::NodeId(8),
                content_id: incompatible,
                position: Position(Vec2::new(0.5, 0.0)),
                remaining: 10,
            },
        ]);
        let selected = nearest_node(&candidates.0, &catalog, Vec2::ZERO, 1.0, 1.0).unwrap();
        assert_eq!(selected, (crate::NodeId(8), stone));
        assert!(
            bind_extractor_node(&candidates.0, &catalog, Vec2::ZERO, 1.0, 1.0, &[ore])
                .unwrap_err()
                .contains("nearest node 8")
        );
    }

    fn empty_catalog() -> ContentCatalog {
        ContentCatalog {
            resources: BTreeMap::new(),
            units: BTreeMap::new(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            source_files: BTreeMap::new(),
        }
    }

    fn resource(id: &ContentId, kind: ResourceKind) -> ResourceDef {
        ResourceDef {
            id: id.local.clone(),
            name: id.local.clone(),
            unit_name: id.local.clone(),
            kind,
        }
    }

    fn health_unit_catalog() -> ContentCatalog {
        let drone_id = ContentId::parse("forge:drone").unwrap();
        let shell = ContentId::parse("forge:shell").unwrap();
        let drone = ResolvedUnit {
            id: drone_id.clone(),
            name: "Drone".into(),
            footprint: Footprint::Circle { radius: 0.6 },
            movement: Some(MovementDef { speed: 2.0 }),
            inventory: Some(ResolvedInventory {
                capacity: 5,
                accepts: vec![shell.clone()],
            }),
            gatherer: None,
            drop_off: None,
            producer: None,
            output_inventory: Some(ResolvedInventory {
                capacity: 2,
                accepts: vec![shell.clone()],
            }),
            extractor: Some(turts_mod_api::ExtractorDef {
                per_second: 2,
                interaction_range: 0.5,
            }),
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: Some(HealthDef { max: 100 }),
            weapon: None,
            removal: None,
        };
        ContentCatalog {
            resources: BTreeMap::from([(
                shell.clone(),
                ResourceDef {
                    id: "shell".into(),
                    name: "Shell".into(),
                    unit_name: "shell".into(),
                    kind: ResourceKind::Item,
                },
            )]),
            units: BTreeMap::from([(drone_id.clone(), drone)]),
            nodes: {
                let id = ContentId::parse("forge:ore_vein").unwrap();
                BTreeMap::from([(
                    id.clone(),
                    ResolvedNode {
                        id,
                        name: "Vein".into(),
                        item: shell,
                        radius: 1.0,
                    },
                )])
            },
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            source_files: BTreeMap::new(),
        }
    }

    #[test]
    fn synthetic_spawn_installs_shared_health_and_output_inventory_rows() {
        let mut app = crate::headless_app(20);
        let world = app.world_mut();
        let catalog = health_unit_catalog();
        let types = UnitTypes::from_catalog(&catalog, &UnitTypes::default()).unwrap();
        let unit_type = types.find("forge:drone").unwrap();
        world.insert_resource(types);
        world.insert_resource(crate::bootstrap::InstalledCatalog(catalog));
        world.insert_resource(SurfaceNodes(vec![NodeRecord {
            id: crate::NodeId(44),
            content_id: ContentId::parse("forge:ore_vein").unwrap(),
            position: Position(Vec2::new(2.0, 2.0)),
            remaining: 20,
        }]));
        world
            .resource_mut::<IdAllocators>()
            .set_next_values(17, 1, 1);
        let ids = crate::spawn_synthetic_units(world, unit_type, &[Vec2::new(2.0, 2.0)]).unwrap();
        assert_eq!(ids, [17]);
        assert_eq!(world.resource::<super::Healths>().0.get(&17), Some(&100));
        assert_eq!(
            world
                .resource::<crate::EconomyLedger>()
                .output_inventories
                .get(&17)
                .unwrap()
                .capacity,
            2
        );
        assert_eq!(
            world
                .resource::<super::Extractors>()
                .0
                .get(&17)
                .unwrap()
                .node,
            Some(crate::NodeId(44)),
        );
    }

    #[test]
    fn transaction_spawn_commit_installs_prepared_m3_rows() {
        let mut app = crate::headless_app(20);
        let world = app.world_mut();
        let catalog = health_unit_catalog();
        let types = UnitTypes::from_catalog(&catalog, &UnitTypes::default()).unwrap();
        let unit_type = types.find("forge:drone").unwrap();
        world.insert_resource(types);
        world.insert_resource(crate::bootstrap::InstalledCatalog(catalog.clone()));
        world.insert_resource(SurfaceNodes(vec![NodeRecord {
            id: crate::NodeId(44),
            content_id: ContentId::parse("forge:ore_vein").unwrap(),
            position: Position(Vec2::new(2.0, 2.0)),
            remaining: 20,
        }]));
        let definition = &catalog.units[&ContentId::parse("forge:drone").unwrap()];
        let binding = CapabilityBinding {
            extractor_node: Some(crate::NodeId(44)),
            inserter_source: None,
            inserter_destination: None,
        };
        let rows = install_capability_rows(definition, binding);
        let id = crate::economy::transaction::transact(world, |tx| {
            let id = tx.allocate_unit_id()?;
            tx.stage_spawn(crate::economy::transaction::SpawnWrite {
                id,
                unit_type,
                owner_slot: 0,
                position: Vec2::new(2.0, 2.0),
                state: crate::world::UnitState {
                    unit_id: id,
                    content_id: "forge:drone".into(),
                    owner_slot: 0,
                    pos: Vec2::new(2.0, 2.0),
                },
                inventory: None,
                gatherer: false,
                producer: false,
                capability_rows: rows,
            });
            Ok(id)
        })
        .unwrap();

        assert_eq!(world.resource::<Healths>().0.get(&id), Some(&100));
        assert_eq!(
            world.resource::<Extractors>().0[&id].node,
            Some(crate::NodeId(44))
        );
        assert_eq!(
            world.resource::<EconomyLedger>().inventories[&id].capacity,
            5
        );
        assert_eq!(
            world.resource::<EconomyLedger>().output_inventories[&id].capacity,
            2
        );
    }

    #[test]
    fn failed_spawn_binding_leaves_world_and_allocator_unchanged() {
        let mut app = crate::headless_app(20);
        let world = app.world_mut();
        let mut definition = health_unit_catalog()
            .units
            .remove(&ContentId::parse("forge:drone").unwrap())
            .unwrap();
        definition.inserter = Some(turts_mod_api::InserterDef {
            reach: 1.0,
            swing_ms: 250,
            return_ms: 250,
        });
        let catalog = health_unit_catalog();
        let types = UnitTypes::from_catalog(&catalog, &UnitTypes::default()).unwrap();
        world.insert_resource(types);
        world.insert_resource(crate::bootstrap::InstalledCatalog(catalog));
        world.insert_resource(SurfaceNodes(vec![NodeRecord {
            id: crate::NodeId(44),
            content_id: ContentId::parse("forge:ore_vein").unwrap(),
            position: Position(Vec2::new(4.0, 4.0)),
            remaining: 20,
        }]));
        let before_allocator = *world.resource::<IdAllocators>();
        let before_count = crate::gameplay_entity_count(world);
        assert_eq!(
            resolve_spawn_capability_binding(world, &definition, Vec2::new(4.0, 4.0), 0, None),
            Err(crate::RejectReason::IntegrityViolation),
        );
        assert_eq!(*world.resource::<IdAllocators>(), before_allocator);
        assert_eq!(crate::gameplay_entity_count(world), before_count);
        let prepared = install_capability_rows(&definition, CapabilityBinding::default());
        assert_eq!(prepared.inserter.unwrap().source, None);
    }
}
