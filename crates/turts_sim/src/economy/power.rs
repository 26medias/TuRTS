//! Slot supply, immutable tick effects, and checked operating-resource admission.
//!
//! Creation callers prepare a grant against live totals, then apply it only after their
//! transaction validates. The Supply phase publishes those totals for the next tick.

use std::collections::{BTreeMap, BTreeSet};

use bevy_ecs::{prelude::Resource, world::World};
use turts_mod_api::{ContentCatalog, ContentId, ModError, ResolvedUnit, ResourceKind, ShortEffect};
use turts_rules::Ratio;

use crate::{
    Owner, RejectReason, Slots, UnitId, UnitType, bootstrap::InstalledCatalog, registry::UnitTypes,
    unit_index::unit_entity,
};

use super::{combat::Healths, construction::ConstructionState, events::SupplyRow};

type AccountKey = (u8, ContentId);

/// The supply totals of one slot and resource. Equality is sufficient supply.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SupplyTotal {
    pub(crate) provided: u64,
    pub(crate) consumed: u64,
}

impl SupplyTotal {
    fn short(self) -> bool {
        turts_rules::supply::is_short(self.provided, self.consumed)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Contribution {
    slot: u8,
    supply: BTreeMap<ContentId, SupplyTotal>,
    income: Option<(ContentId, u64)>,
}

impl Contribution {
    fn of(unit: &ResolvedUnit, slot: u8) -> Result<Self, RejectReason> {
        let mut contribution = Self {
            slot,
            ..Self::default()
        };
        for (rows, provided) in [(&unit.provides, true), (&unit.consumes, false)] {
            let mut seen = BTreeSet::new();
            for row in rows {
                let id = ContentId::parse(&row.resource)
                    .map_err(|_| RejectReason::IntegrityViolation)?;
                if row.amount == 0 || !seen.insert(id.clone()) {
                    return Err(RejectReason::IntegrityViolation);
                }
                let total = contribution.supply.entry(id).or_default();
                if provided {
                    total.provided = row.amount;
                } else {
                    total.consumed = row.amount;
                }
            }
        }
        if let Some(income) = &unit.income {
            if income.amount == 0 {
                return Err(RejectReason::IntegrityViolation);
            }
            contribution.income = Some((
                ContentId::parse(&income.resource).map_err(|_| RejectReason::IntegrityViolation)?,
                income.amount,
            ));
        }
        Ok(contribution)
    }

    fn is_empty(&self) -> bool {
        self.supply.is_empty() && self.income.is_none()
    }
}

/// A creation grant. Apply grants in the same order as their checked preparation.
/// A grant is prepared on either the authoritative state or a tentative fork of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Activation {
    contribution: Contribution,
    supply: Vec<(AccountKey, SupplyTotal)>,
    income: Option<(AccountKey, u64)>,
    expected_supply: Vec<(AccountKey, SupplyTotal)>,
    expected_income: Option<(AccountKey, u64)>,
}

/// Live admission totals are separate from the effect snapshot used during a tick.
/// No resource is installed for a catalog without operating capabilities.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OperatingResources {
    pub(crate) initialized: bool,
    pub(crate) effects: BTreeMap<AccountKey, SupplyTotal>,
    live_supply: BTreeMap<AccountKey, SupplyTotal>,
    live_income: BTreeMap<AccountKey, u64>,
    active: BTreeMap<u32, Contribution>,
    slot_count: u8,
}

/// Whether a catalog needs operating state. This keeps legacy worlds on their old path.
pub(crate) fn has_operating_resources(catalog: &ContentCatalog) -> bool {
    catalog
        .resources
        .values()
        .any(|resource| matches!(resource.kind, ResourceKind::Supply { .. }))
        || catalog.units.values().any(|unit| {
            !unit.provides.is_empty() || !unit.consumes.is_empty() || unit.income.is_some()
        })
}

impl OperatingResources {
    /// Build declared zero baselines. Unit activation remains a separate checked operation.
    pub(crate) fn from_catalog(catalog: &ContentCatalog, slot_count: u8) -> Self {
        if !has_operating_resources(catalog) {
            return Self::default();
        }
        let mut state = Self {
            initialized: true,
            slot_count,
            ..Self::default()
        };
        for slot in 0..slot_count {
            for (id, resource) in &catalog.resources {
                if matches!(resource.kind, ResourceKind::Supply { .. }) {
                    state
                        .live_supply
                        .insert((slot, id.clone()), SupplyTotal::default());
                }
            }
        }
        state.effects.clone_from(&state.live_supply);
        state
    }

    /// Preflight supply and the active-income bound as one decision. This does not mutate state.
    pub(crate) fn prepare_activation(
        &self,
        unit: &ResolvedUnit,
        slot: u8,
    ) -> Result<Activation, RejectReason> {
        let contribution = Contribution::of(unit, slot)?;
        if !contribution.is_empty() && (!self.initialized || slot >= self.slot_count) {
            return Err(RejectReason::IntegrityViolation);
        }
        let mut supply = Vec::with_capacity(contribution.supply.len());
        let mut expected_supply = Vec::with_capacity(contribution.supply.len());
        for (resource, added) in &contribution.supply {
            let key = (slot, resource.clone());
            let current = self
                .live_supply
                .get(&key)
                .ok_or(RejectReason::IntegrityViolation)?;
            let next = SupplyTotal {
                provided: current
                    .provided
                    .checked_add(added.provided)
                    .ok_or(RejectReason::ArithmeticOverflow)?,
                consumed: current
                    .consumed
                    .checked_add(added.consumed)
                    .ok_or(RejectReason::ArithmeticOverflow)?,
            };
            expected_supply.push((key.clone(), *current));
            supply.push((key, next));
        }
        let income = if let Some((resource, amount)) = &contribution.income {
            let key = (slot, resource.clone());
            let next = self
                .live_income
                .get(&key)
                .copied()
                .unwrap_or(0)
                .checked_add(*amount)
                .ok_or(RejectReason::ArithmeticOverflow)?;
            Some((key, next))
        } else {
            None
        };
        let expected_income = contribution.income.as_ref().map(|(resource, _)| {
            let key = (slot, resource.clone());
            let current = self.live_income.get(&key).copied().unwrap_or(0);
            (key, current)
        });
        Ok(Activation {
            contribution,
            supply,
            income,
            expected_supply,
            expected_income,
        })
    }

    /// Commit a prevalidated grant after the enclosing creation transaction succeeds.
    /// The caller must preserve preparation order and must not activate an already active id.
    pub(crate) fn apply_activation(&mut self, unit: u32, activation: Activation) {
        if activation.contribution.is_empty() {
            return;
        }
        assert!(
            !self.active.contains_key(&unit),
            "duplicate operating activation for unit {unit}"
        );
        // Validate the internal commit-order contract before writing any totals. A stale
        // tentative grant must never overwrite a previously admitted unit's contribution.
        for (key, expected) in &activation.expected_supply {
            assert_eq!(
                self.live_supply.get(key),
                Some(expected),
                "stale supply activation grant"
            );
        }
        if let Some((key, expected)) = &activation.expected_income {
            assert_eq!(
                self.live_income.get(key).copied().unwrap_or(0),
                *expected,
                "stale income activation grant"
            );
        }
        for (key, next) in activation.supply {
            self.live_supply.insert(key, next);
        }
        if let Some((key, next)) = activation.income {
            self.live_income.insert(key, next);
        }
        self.active.insert(unit, activation.contribution);
    }

    /// Remove the contribution once. Repeated removal is harmless. The tick snapshot is unchanged.
    pub(crate) fn remove(&mut self, unit: u32) {
        let Some(contribution) = self.active.remove(&unit) else {
            return;
        };
        for (resource, removed) in contribution.supply {
            let key = (contribution.slot, resource);
            if let Some(total) = self.live_supply.get_mut(&key) {
                // Only validated activations populate these totals. A broken subtraction is
                // an engine invariant failure, rather than an input overflow to saturate.
                assert!(total.provided >= removed.provided && total.consumed >= removed.consumed);
                total.provided -= removed.provided;
                total.consumed -= removed.consumed;
            }
        }
        if let Some((resource, amount)) = contribution.income {
            if let Some(total) = self.live_income.get_mut(&(contribution.slot, resource)) {
                assert!(*total >= amount);
                *total -= amount;
            }
        }
    }

    /// Publish exactly one immutable snapshot for all effects of this tick.
    pub(crate) fn begin_tick(&mut self) {
        self.effects.clone_from(&self.live_supply);
    }

    pub(crate) fn income_amount(&self, slot: u8, resource: &ContentId) -> u64 {
        self.live_income
            .get(&(slot, resource.clone()))
            .copied()
            .unwrap_or(0)
    }
}

/// Prepare a grant without changing the world or any allocation counter.
pub(crate) fn can_activate(
    world: &World,
    unit: &ResolvedUnit,
    slot: u8,
) -> Result<Activation, RejectReason> {
    if let Some(state) = world.get_resource::<OperatingResources>() {
        state.prepare_activation(unit, slot)
    } else {
        OperatingResources::default().prepare_activation(unit, slot)
    }
}

/// Infallible commit hook. Preparation must occur before the enclosing transaction mutates.
pub(crate) fn commit_activate(world: &mut World, unit: u32, activation: Activation) {
    if activation.contribution.is_empty() {
        return;
    }
    world
        .resource_mut::<OperatingResources>()
        .apply_activation(unit, activation);
}

/// Infallible removal/zero-health hook. It preserves this tick's effect snapshot.
pub(crate) fn remove_active(world: &mut World, unit: u32) {
    if let Some(mut state) = world.get_resource_mut::<OperatingResources>() {
        state.remove(unit);
    }
}

/// Check the merged catalog's exact denominator scale, with source-aware diagnostics.
pub(crate) fn catalog_rate_q(catalog: &ContentCatalog) -> Result<u64, ModError> {
    let mut q = 1_u64;
    for unit in catalog.units.values() {
        for (owner, entries) in [
            (
                "producer",
                unit.producer
                    .as_ref()
                    .map(|value| value.when_short.as_slice()),
            ),
            (
                "constructor",
                unit.constructor
                    .as_ref()
                    .map(|value| value.when_short.as_slice()),
            ),
        ] {
            for (index, entry) in entries.into_iter().flatten().enumerate() {
                let ShortEffect::Rate(ratio) = entry.effect else {
                    continue;
                };
                let fail = |message: &str| {
                    ModError::new(
                        &unit.id.package.0,
                        &format!("{owner}.when_short[{index}].effect"),
                        message,
                    )
                    .with_file(
                        catalog
                            .source_files
                            .get(&unit.id)
                            .map_or("content", String::as_str),
                    )
                };
                if ratio.numerator == 0
                    || ratio.denominator == 0
                    || ratio.numerator > ratio.denominator
                {
                    return Err(fail("Rate needs 0 < numerator <= denominator"));
                }
                let denominator = u64::from(ratio.denominator);
                let mut a = q;
                let mut b = denominator;
                while b != 0 {
                    (a, b) = (b, a % b);
                }
                q = (q / a)
                    .checked_mul(denominator)
                    .ok_or_else(|| fail("Rate denominators overflow; use smaller denominators"))?;
                if q > 65_536 {
                    return Err(fail(
                        "the combined Rate denominator scale exceeds 65,536; use compatible denominators",
                    ));
                }
            }
        }
    }
    Ok(q)
}

/// Validate all initial contributions before installing any operating state.
/// Call after package entities and health rows exist, and before the first published snapshot.
pub(crate) fn initialize(world: &mut World) -> Result<(), RejectReason> {
    let Some(catalog) = world.get_resource::<InstalledCatalog>() else {
        return Ok(());
    };
    if !has_operating_resources(&catalog.0) {
        world.remove_resource::<OperatingResources>();
        return Ok(());
    }
    let slot_count = world
        .get_resource::<Slots>()
        .ok_or(RejectReason::IntegrityViolation)?
        .count();
    let types = world
        .get_resource::<UnitTypes>()
        .ok_or(RejectReason::IntegrityViolation)?;
    let sites = world.get_resource::<ConstructionState>();
    let healths = world.get_resource::<Healths>();
    let mut state = OperatingResources::from_catalog(&catalog.0, slot_count);
    for entity in world.iter_entities() {
        let (Some(id), Some(kind), Some(owner)) = (
            entity.get::<UnitId>(),
            entity.get::<UnitType>(),
            entity.get::<Owner>(),
        ) else {
            continue;
        };
        if sites.is_some_and(|sites| sites.sites.contains_key(&id.0))
            || healths
                .and_then(|healths| healths.0.get(&id.0))
                .is_some_and(|health| *health == 0)
        {
            continue;
        }
        let entry = types.get(kind.0).ok_or(RejectReason::IntegrityViolation)?;
        let content =
            ContentId::parse(&entry.content_id).map_err(|_| RejectReason::IntegrityViolation)?;
        let unit = catalog
            .0
            .units
            .get(&content)
            .ok_or(RejectReason::IntegrityViolation)?;
        let activation = state.prepare_activation(unit, owner.0.0)?;
        state.apply_activation(id.0, activation);
    }
    state.begin_tick();
    world.insert_resource(state);
    Ok(())
}

/// The first phase after tick advance. Creation/removal hooks maintain the live totals.
pub(crate) fn run_supply(world: &mut World) {
    if let Some(mut state) = world.get_resource_mut::<OperatingResources>() {
        state.begin_tick();
    }
}

/// Complete zero-inclusive effect records, sorted by slot and qualified resource.
pub(crate) fn supply_rows(world: &World) -> Vec<SupplyRow> {
    world
        .get_resource::<OperatingResources>()
        .map_or_else(Vec::new, |state| {
            state
                .effects
                .iter()
                .map(|((slot, resource), total)| SupplyRow {
                    slot: *slot,
                    resource: resource.clone(),
                    provided: total.provided,
                    consumed: total.consumed,
                })
                .collect()
        })
}

/// Work rates are attached to capabilities, rather than to a unit as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkCapability {
    Producer,
    Constructor,
    Machine,
    Research,
}

fn definition(world: &World, unit: u32) -> Option<&ResolvedUnit> {
    let entity = unit_entity(world, unit)?;
    let kind = world.get::<UnitType>(entity)?;
    let entry = world.get_resource::<UnitTypes>()?.get(kind.0)?;
    let id = ContentId::parse(&entry.content_id).ok()?;
    world.get_resource::<InstalledCatalog>()?.0.units.get(&id)
}

fn slot_of(world: &World, unit: u32) -> Option<u8> {
    world
        .get::<Owner>(unit_entity(world, unit)?)
        .map(|owner| owner.0.0)
}

fn shortage(world: &World, slot: u8, reference: &str) -> bool {
    let Some(state) = world.get_resource::<OperatingResources>() else {
        return false;
    };
    let Ok(resource) = ContentId::parse(reference) else {
        return false;
    };
    state
        .effects
        .get(&(slot, resource))
        .is_some_and(|total| total.short())
}

/// Shortage does not change manual enable state. Existing callers enforce that state separately.
pub(crate) fn weapon_enabled(world: &World, unit: u32) -> bool {
    #[cfg(test)]
    if let Some(probe) = world.get_resource::<PowerProbe>() {
        return probe.weapon_enabled;
    }
    if !world.contains_resource::<OperatingResources>() {
        return true;
    }
    let Some(slot) = slot_of(world, unit) else {
        return true;
    };
    definition(world, unit)
        .and_then(|unit| unit.weapon.as_ref())
        .is_none_or(|weapon| {
            !weapon.when_short.iter().any(|entry| {
                entry.effect == ShortEffect::Disable && shortage(world, slot, &entry.resource)
            })
        })
}

pub(crate) fn full_rate() -> Ratio {
    Ratio::ONE
}

pub(crate) fn rate_for(world: &World, unit: u32, capability: WorkCapability) -> Ratio {
    if matches!(
        capability,
        WorkCapability::Machine | WorkCapability::Research
    ) {
        return full_rate();
    }
    #[cfg(test)]
    if let Some(probe) = world.get_resource::<PowerProbe>() {
        return probe.rate;
    }
    if !world.contains_resource::<OperatingResources>() {
        return full_rate();
    }
    let Some(definition) = definition(world, unit) else {
        return full_rate();
    };
    let entries = match capability {
        WorkCapability::Producer => definition
            .producer
            .as_ref()
            .map(|value| value.when_short.as_slice()),
        WorkCapability::Constructor => definition
            .constructor
            .as_ref()
            .map(|value| value.when_short.as_slice()),
        WorkCapability::Machine | WorkCapability::Research => None,
    };
    let Some(slot) = slot_of(world, unit) else {
        return full_rate();
    };
    entries
        .into_iter()
        .flatten()
        .fold(full_rate(), |rate, entry| {
            if let ShortEffect::Rate(other) = entry.effect
                && shortage(world, slot, &entry.resource)
            {
                let Some(other) =
                    Ratio::new(u64::from(other.numerator), u64::from(other.denominator))
                else {
                    return rate;
                };
                turts_rules::supply::minimum_rate(rate, other)
            } else {
                rate
            }
        })
}

/// Compatibility accessor for existing producer callers. Constructor callers use `rate_for`.
pub(crate) fn rate_factor(world: &World, unit: u32) -> Ratio {
    rate_for(world, unit, WorkCapability::Producer)
}

/// One raw progress unit is 1/Q of a simulation tick.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RateQ(pub(crate) u64);

impl Default for RateQ {
    fn default() -> Self {
        Self(1)
    }
}

pub(crate) fn rate_q(world: &World) -> u64 {
    world.get_resource::<RateQ>().map_or(1, |q| q.0)
}

/// Existing focused callers can still override the seam without constructing a whole catalog.
#[cfg(test)]
#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct PowerProbe {
    pub(crate) weapon_enabled: bool,
    pub(crate) rate: Ratio,
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_mod_api::{
        ConstructProgress, Footprint, IncomeDef, ResolvedConstructor, ResolvedProducer,
        ResolvedWeapon, ResourceDef, SupplyAmount, SupplyCounts, SupplyScope, TargetFilter,
        WeaponRangeMode, WhenShort,
    };

    fn id(value: &str) -> ContentId {
        ContentId::parse(value).unwrap()
    }
    fn amount(resource: &str, amount: u64) -> SupplyAmount {
        SupplyAmount {
            resource: resource.into(),
            amount,
        }
    }
    fn rate(resource: &str, numerator: u32, denominator: u32) -> WhenShort {
        WhenShort {
            resource: resource.into(),
            effect: ShortEffect::Rate(turts_mod_api::Ratio {
                numerator,
                denominator,
            }),
        }
    }
    fn unit(name: &str) -> ResolvedUnit {
        ResolvedUnit {
            acquire: None,
            armour: None,
            id: id(name),
            name: name.into(),
            footprint: Footprint::Circle { radius: 0.5 },
            movement: None,
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: None,
            constructor: None,
            placement: None,
            build_area: None,
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: None,
            weapon: None,
            removal: None,
            provides: vec![],
            consumes: vec![],
            income: None,
        }
    }
    fn catalog(units: Vec<ResolvedUnit>) -> ContentCatalog {
        let supply = ResourceKind::Supply {
            scope: SupplyScope::Slot,
            counts: SupplyCounts::Completed,
        };
        let resources = [
            ("test:power", supply.clone()),
            ("test:capacity", supply),
            ("test:credits", ResourceKind::Balance),
        ]
        .into_iter()
        .map(|(name, kind)| {
            (
                id(name),
                ResourceDef {
                    id: name.rsplit(':').next().unwrap().into(),
                    name: name.into(),
                    unit_name: String::new(),
                    kind,
                },
            )
        })
        .collect();
        ContentCatalog {
            combat: None,
            resources,
            units: units
                .into_iter()
                .map(|unit| (unit.id.clone(), unit))
                .collect(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: BTreeMap::new(),
            source_files: BTreeMap::new(),
        }
    }
    fn world(catalog: ContentCatalog, placements: &[(u32, &str, u8)]) -> World {
        let types = UnitTypes::from_catalog(&catalog, &UnitTypes::default()).unwrap();
        let mut world = World::new();
        for (unit, name, slot) in placements {
            let kind = types.find(name).unwrap();
            world.spawn((UnitId(*unit), UnitType(kind), Owner(crate::SlotId(*slot))));
        }
        world.insert_resource(types);
        world.insert_resource(InstalledCatalog(catalog));
        world.insert_resource(Slots::new(2));
        world.insert_resource(ConstructionState::default());
        world.insert_resource(Healths::default());
        world
    }

    #[test]
    fn declared_resources_have_zero_baselines_for_every_slot() {
        let mut world = world(catalog(vec![]), &[]);
        initialize(&mut world).unwrap();
        let rows = supply_rows(&world);
        assert_eq!(rows.len(), 4);
        assert!(
            rows.iter()
                .all(|row| row.provided == 0 && row.consumed == 0)
        );
        assert_eq!(
            rows.iter().map(|row| row.slot).collect::<Vec<_>>(),
            vec![0, 0, 1, 1]
        );
    }

    #[test]
    fn live_removal_waits_for_next_snapshot_and_manual_disable_keeps_totals() {
        let mut provider = unit("test:provider");
        provider.provides = vec![amount("test:power", 10)];
        let mut consumer = unit("test:consumer");
        consumer.consumes = vec![amount("test:power", 12)];
        let mut world = world(
            catalog(vec![provider, consumer]),
            &[
                (10, "test:provider", 0),
                (20, "test:consumer", 0),
                (30, "test:consumer", 1),
            ],
        );
        world.insert_resource(super::super::research::Disabled(BTreeSet::from([20])));
        initialize(&mut world).unwrap();
        let key = (0, id("test:power"));
        assert_eq!(
            world.resource::<OperatingResources>().effects[&key],
            SupplyTotal {
                provided: 10,
                consumed: 12
            }
        );
        assert_eq!(
            world.resource::<OperatingResources>().effects[&(1, id("test:power"))].provided,
            0
        );
        remove_active(&mut world, 10);
        remove_active(&mut world, 10);
        assert_eq!(
            world.resource::<OperatingResources>().effects[&key].provided,
            10
        );
        run_supply(&mut world);
        assert_eq!(
            world.resource::<OperatingResources>().effects[&key],
            SupplyTotal {
                provided: 0,
                consumed: 12
            }
        );
        assert!(
            world
                .resource::<super::super::research::Disabled>()
                .0
                .contains(&20)
        );
    }

    #[test]
    fn boot_excludes_sites_and_dead_units_but_counts_unarmoured_healthless_providers() {
        let mut provider = unit("test:provider");
        provider.provides = vec![amount("test:power", 10)];
        let mut world = world(
            catalog(vec![provider]),
            &[
                (1, "test:provider", 0),
                (2, "test:provider", 0),
                (3, "test:provider", 0),
            ],
        );
        world.resource_mut::<Healths>().0.insert(2, 0);
        world.resource_mut::<ConstructionState>().sites.insert(
            3,
            super::super::construction::Site {
                constructor: 1,
                owner_slot: 0,
                recipe: id("test:build"),
                job: crate::JobId(1),
                done: 0,
                total: 10,
                charged: vec![],
            },
        );
        initialize(&mut world).unwrap();
        assert_eq!(
            world.resource::<OperatingResources>().effects[&(0, id("test:power"))].provided,
            10
        );
    }

    #[test]
    fn boot_overflow_refuses_without_installing_partial_state() {
        let mut full = unit("test:full");
        full.provides = vec![amount("test:power", u64::MAX)];
        let mut extra = unit("test:extra");
        extra.provides = vec![amount("test:power", 1)];
        let mut world = world(
            catalog(vec![full, extra]),
            &[(1, "test:full", 0), (2, "test:extra", 0)],
        );
        let before_entities = world.iter_entities().count();
        assert_eq!(
            initialize(&mut world),
            Err(RejectReason::ArithmeticOverflow)
        );
        assert!(!world.contains_resource::<OperatingResources>());
        assert_eq!(world.iter_entities().count(), before_entities);
    }

    #[test]
    fn supply_and_income_preflight_is_atomic_and_removal_releases_both_bounds() {
        let mut full = unit("test:full");
        full.provides = vec![amount("test:power", u64::MAX - 1)];
        full.income = Some(IncomeDef {
            resource: "test:credits".into(),
            amount: u64::MAX,
            period_ms: 1000,
        });
        let mut extra = unit("test:extra");
        extra.provides = vec![amount("test:power", 1)];
        extra.income = Some(IncomeDef {
            resource: "test:credits".into(),
            amount: 1,
            period_ms: 1000,
        });
        let catalog = catalog(vec![full.clone(), extra.clone()]);
        let mut state = OperatingResources::from_catalog(&catalog, 2);
        state.apply_activation(100, state.prepare_activation(&full, 0).unwrap());
        let before = state.clone();
        assert_eq!(
            state.prepare_activation(&extra, 0),
            Err(RejectReason::ArithmeticOverflow)
        );
        assert_eq!(state, before);
        assert!(state.prepare_activation(&extra, 1).is_ok());
        state.remove(100);
        state.apply_activation(101, state.prepare_activation(&extra, 0).unwrap());
        assert_eq!(state.income_amount(0, &id("test:credits")), 1);
        state.begin_tick();
        assert_eq!(state.effects[&(0, id("test:power"))].provided, 1);
    }

    #[test]
    fn tentative_forks_do_not_change_live_or_effect_state_until_committed() {
        let mut provider = unit("test:provider");
        provider.provides = vec![amount("test:power", 5)];
        let mut world = world(catalog(vec![provider.clone()]), &[]);
        initialize(&mut world).unwrap();
        let before = world.resource::<OperatingResources>().clone();
        let mut fork = before.clone();
        fork.apply_activation(5, fork.prepare_activation(&provider, 0).unwrap());
        assert_eq!(world.resource::<OperatingResources>(), &before);
        assert_eq!(fork.effects, before.effects);
        let grant = can_activate(&world, &provider, 0).unwrap();
        commit_activate(&mut world, 5, grant);
        assert_eq!(
            world.resource::<OperatingResources>().effects,
            before.effects
        );
        run_supply(&mut world);
        assert_eq!(
            world.resource::<OperatingResources>().effects[&(0, id("test:power"))].provided,
            5
        );
    }

    #[test]
    fn hybrid_capabilities_use_independent_minimum_rates_and_weapons_keep_manual_state() {
        let mut hybrid = unit("test:hybrid");
        hybrid.consumes = vec![amount("test:power", 10), amount("test:capacity", 10)];
        hybrid.producer = Some(ResolvedProducer {
            recipes: vec![],
            queue_capacity: 1,
            spawn_offsets: vec![],
            when_short: vec![rate("test:power", 1, 2), rate("test:capacity", 1, 3)],
        });
        hybrid.constructor = Some(ResolvedConstructor {
            buildings: vec![],
            concurrent_sites: 1,
            queue_capacity: 1,
            progress: ConstructProgress::Autonomous,
            when_short: vec![rate("test:power", 3, 4)],
        });
        hybrid.weapon = Some(ResolvedWeapon {
            damage_type: None,
            bonus: vec![],
            range: 5.0,
            range_mode: WeaponRangeMode::Centre,
            cooldown_ms: 1000,
            damage: 10,
            ammo: None,
            targets: TargetFilter {
                slots: turts_mod_api::SlotRelation::Enemy,
            },
            when_short: vec![WhenShort {
                resource: "test:power".into(),
                effect: ShortEffect::Disable,
            }],
        });
        let mut provider = unit("test:provider");
        provider.provides = vec![amount("test:power", 10), amount("test:capacity", 10)];
        let mut world = world(
            catalog(vec![hybrid, provider.clone()]),
            &[(1, "test:hybrid", 0)],
        );
        world.insert_resource(super::super::research::Disabled(BTreeSet::from([1])));
        initialize(&mut world).unwrap();
        assert_eq!(
            rate_for(&world, 1, WorkCapability::Producer),
            Ratio::new(1, 3).unwrap()
        );
        assert_eq!(
            rate_for(&world, 1, WorkCapability::Constructor),
            Ratio::new(3, 4).unwrap()
        );
        assert_eq!(rate_for(&world, 1, WorkCapability::Machine), Ratio::ONE);
        assert_eq!(rate_for(&world, 1, WorkCapability::Research), Ratio::ONE);
        assert!(!weapon_enabled(&world, 1));
        let grant = can_activate(&world, &provider, 0).unwrap();
        commit_activate(&mut world, 2, grant);
        assert!(!weapon_enabled(&world, 1));
        run_supply(&mut world);
        assert_eq!(rate_factor(&world, 1), Ratio::ONE);
        assert_eq!(rate_for(&world, 1, WorkCapability::Constructor), Ratio::ONE);
        assert!(weapon_enabled(&world, 1));
        assert!(
            world
                .resource::<super::super::research::Disabled>()
                .0
                .contains(&1)
        );
    }

    #[test]
    fn merged_catalog_scale_is_exact_and_rejects_cross_package_lcm_over_the_limit() {
        let mut first = unit("first:unit");
        first.producer = Some(ResolvedProducer {
            recipes: vec![],
            queue_capacity: 1,
            spawn_offsets: vec![],
            when_short: vec![rate("test:power", 1, 6)],
        });
        let mut second = unit("second:unit");
        second.constructor = Some(ResolvedConstructor {
            buildings: vec![],
            concurrent_sites: 1,
            queue_capacity: 1,
            progress: ConstructProgress::Autonomous,
            when_short: vec![rate("test:power", 1, 10)],
        });
        let mut catalog = catalog(vec![first, second]);
        assert_eq!(catalog_rate_q(&catalog).unwrap(), 30);
        catalog
            .units
            .get_mut(&id("first:unit"))
            .unwrap()
            .producer
            .as_mut()
            .unwrap()
            .when_short = vec![rate("test:power", 1, 257)];
        catalog
            .units
            .get_mut(&id("second:unit"))
            .unwrap()
            .constructor
            .as_mut()
            .unwrap()
            .when_short = vec![rate("test:power", 1, 263)];
        catalog
            .source_files
            .insert(id("second:unit"), "units/unit.ron".into());
        let error = catalog_rate_q(&catalog).unwrap_err();
        assert_eq!(error.package, "second");
        assert_eq!(error.file.as_deref(), Some("units/unit.ron"));
        assert_eq!(error.field_path, "constructor.when_short[0].effect");
        assert!(error.message.contains("65,536"));
    }

    #[test]
    fn no_operating_catalog_keeps_q_one_and_installs_no_new_resource() {
        let mut catalog = catalog(vec![unit("test:plain")]);
        catalog.resources.clear();
        let mut world = world(catalog.clone(), &[(1, "test:plain", 0)]);
        initialize(&mut world).unwrap();
        run_supply(&mut world);
        assert!(!world.contains_resource::<OperatingResources>());
        assert!(supply_rows(&world).is_empty());
        assert_eq!(catalog_rate_q(&catalog).unwrap(), 1);
        assert_eq!(rate_q(&world), 1);
        assert_eq!(rate_factor(&world, 1), Ratio::ONE);
        assert!(weapon_enabled(&world, 1));
    }
}
