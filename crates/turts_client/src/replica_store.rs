//! The side store of the 6.8 records (task `L0-STORE`, ruling R-H; design 6.8, "No client
//! prediction").
//!
//! The replica is the candidate (units, nodes, economy, orders, ...) and this one side store. The
//! store holds the records that the interface reads through one getter each: the sites, the
//! `SiteProgress` records, the held construction orders, the supply records, the match state and
//! the job progress records. It is not a field of the candidate (MP1 adds none, design section 9)
//! and it is not cloned for each delta.
//!
//! One apply path. A message stages its store change next to its candidate:
//!
//! - a `Snapshot` stages a whole new store ([`stage_snapshot_store`]);
//! - a `WorldDelta` stages one [`StoreStep`] ([`stage_delta_step`]): only its changes, no clone of
//!   the store. A resync that replays retained deltas applies each step to the staged store of
//!   its Snapshot, never to the live store.
//!
//! `Replica::commit` then applies both or neither ([`StoreUpdate`]). A delta that fails to stage,
//! a stale Snapshot, a discarded delta and a resync that fails after its Snapshot change nothing.
//!
//! # Removal rule (what removes an entry when its unit is removed)
//!
//! A replicated STATE record must never keep a reference to a unit that is gone.
//!
//! | Table | Entry goes when |
//! | --- | --- |
//! | `sites` | the unit is in `sites_completed`, or in `removed` |
//! | `site_progress` | the unit is in `sites_completed`, or in `removed` |
//! | `construction_queues` | the constructor is in `removed`, or the record has no orders |
//! | `job_progress` | the producer is in `removed`, or the producer's `QueueState` no longer holds the job |
//! | `supply` | never: a supply record names a slot and a resource, and slots are permanent (D25) |
//! | `match_state` | never: it names slots only; the next `match_state` replaces it |
//!
//! A message that names a removed unit in a record (a record of a unit that the same message
//! removes) is refused, as the order records are (`merge_unit_orders`).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use turts_mod_api::{ContentId, ResourceKind};
use turts_net_protocol::{
    ConstructionQueue, EconomyState, HeldOrder, JobProgress, MatchState, RateRecord, RecordState,
    SiteProgress, SlotOutcome, SupplyRecord, UnitSnapshot, WorldDelta,
};
use turts_rules::charge::{CancelCase, CancelRefunds};
use turts_rules::progress;

use crate::replica::{ReplicaContext, UnitReplica};

/// Clones of a [`RecordStore`] since the process started. The apply path never clones the store:
/// a counter above 0 across the deltas of a run is a defect (`tests/mp1_store.rs`).
static STORE_CLONES: AtomicU64 = AtomicU64::new(0);

/// The number of [`RecordStore`] clones since the process started.
pub fn store_clone_count() -> u64 {
    STORE_CLONES.load(Ordering::Relaxed)
}

/// The 6.8 records that the interface reads. The fields are private: the only writer is the
/// staged apply path of the replica, and the readers use the getters.
#[derive(Debug, Default, PartialEq)]
pub struct RecordStore {
    tick: u32,
    /// The sites: units with `UnitSnapshot.site` that have not completed. Public.
    sites: BTreeSet<u32>,
    /// The progress of each own site, by site unit.
    site_progress: BTreeMap<u32, RateRecord>,
    /// The held `Construct` orders of each own constructor, by constructor. Never empty.
    construction_queues: BTreeMap<u32, Vec<HeldOrder>>,
    /// The supply of each own (slot, resource).
    supply: BTreeMap<(u8, String), SupplyRecord>,
    match_state: MatchState,
    /// The progress record of each job in `economy.queues`, by (producer, job).
    job_progress: BTreeMap<(u32, u64), RateRecord>,
}

impl Clone for RecordStore {
    fn clone(&self) -> Self {
        STORE_CLONES.fetch_add(1, Ordering::Relaxed);
        Self {
            tick: self.tick,
            sites: self.sites.clone(),
            site_progress: self.site_progress.clone(),
            construction_queues: self.construction_queues.clone(),
            supply: self.supply.clone(),
            match_state: self.match_state.clone(),
            job_progress: self.job_progress.clone(),
        }
    }
}

/// What a store apply does. `Replace` is a Snapshot (the store is replaced once); `Step` is one
/// delta (its changes only).
pub(crate) enum StoreUpdate {
    Replace(RecordStore),
    Step(StoreStep),
}

/// The staged change of one delta. Built and validated by [`stage_delta_step`]; applied by
/// [`RecordStore::apply`], which cannot fail.
#[derive(Debug, Default)]
pub(crate) struct StoreStep {
    tick: u32,
    removed: Vec<u32>,
    sites_added: Vec<u32>,
    sites_completed: Vec<u32>,
    site_progress: Vec<(u32, RateRecord)>,
    /// Each producer whose `QueueState` the delta replaces, with the jobs that it holds now.
    queue_jobs: Vec<(u32, Vec<u64>)>,
    job_progress: Vec<((u32, u64), RateRecord)>,
    construction_queues: Vec<(u32, Vec<HeldOrder>)>,
    supply: Vec<SupplyRecord>,
    match_state: Option<MatchState>,
}

impl RecordStore {
    /// The tick of the last applied message. It equals the tick of the candidate.
    pub fn tick(&self) -> u32 {
        self.tick
    }

    /// The sites that exist: a building under construction (public).
    pub fn sites(&self) -> impl Iterator<Item = u32> + '_ {
        self.sites.iter().copied()
    }

    pub fn is_site(&self, unit: u32) -> bool {
        self.sites.contains(&unit)
    }

    /// The progress record of an own site.
    pub fn site_progress(&self, unit: u32) -> Option<&RateRecord> {
        self.site_progress.get(&unit)
    }

    /// The held construction orders of an own constructor. Empty when it holds none.
    pub fn construction_queue(&self, constructor: u32) -> &[HeldOrder] {
        self.construction_queues
            .get(&constructor)
            .map_or(&[], Vec::as_slice)
    }

    /// The supply record of (`slot`, `resource`).
    pub fn supply_record(&self, slot: u8, resource: &str) -> Option<&SupplyRecord> {
        self.supply.get(&(slot, resource.to_string()))
    }

    /// The match state. The empty state until a `Snapshot` or a delta brings one.
    pub fn match_state(&self) -> &MatchState {
        &self.match_state
    }

    /// The progress record of a job.
    pub fn job_progress(&self, producer: u32, job: u64) -> Option<&RateRecord> {
        self.job_progress.get(&(producer, job))
    }

    /// The (producer, job) keys that have a progress record, ascending.
    pub fn job_keys(&self) -> impl Iterator<Item = (u32, u64)> + '_ {
        self.job_progress.keys().copied()
    }

    /// The number of progress records (jobs and sites), for the cost record.
    pub fn record_count(&self) -> usize {
        self.job_progress.len() + self.site_progress.len()
    }

    /// Apply a staged step in place. The step is already validated, so this cannot fail.
    pub(crate) fn apply(&mut self, step: StoreStep) {
        for unit in &step.removed {
            self.sites.remove(unit);
            self.site_progress.remove(unit);
            self.construction_queues.remove(unit);
            self.drop_jobs_where(*unit, |_| true);
        }
        self.sites.extend(step.sites_added);
        for unit in &step.sites_completed {
            self.sites.remove(unit);
            self.site_progress.remove(unit);
        }
        for (producer, held) in &step.queue_jobs {
            self.drop_jobs_where(*producer, |job| !held.contains(&job));
        }
        self.site_progress.extend(step.site_progress);
        self.job_progress.extend(step.job_progress);
        for (constructor, orders) in step.construction_queues {
            if orders.is_empty() {
                self.construction_queues.remove(&constructor);
            } else {
                self.construction_queues.insert(constructor, orders);
            }
        }
        for record in step.supply {
            self.supply
                .insert((record.slot.0, record.resource.clone()), record);
        }
        if let Some(state) = step.match_state {
            self.match_state = state;
        }
        self.tick = step.tick;
    }

    /// Remove the job records of `producer` for which `drop` is true.
    fn drop_jobs_where(&mut self, producer: u32, drop: impl Fn(u64) -> bool) {
        let keys: Vec<(u32, u64)> = self
            .job_progress
            .range((producer, 0)..=(producer, u64::MAX))
            .map(|(key, _)| *key)
            .filter(|key| drop(key.1))
            .collect();
        for key in keys {
            self.job_progress.remove(&key);
        }
    }
}

/// A record in the form that `turts_rules::progress` evaluates. A job that waits for funds, and a
/// held record (6.4, erratum E21), keep their value.
pub(crate) fn rate_record(record: &RateRecord) -> progress::RateRecord {
    progress::RateRecord {
        done: record.done,
        total: record.total,
        per_tick: record.per_tick,
        at_tick: record.at_tick,
        stalled: record.held || matches!(record.state, RecordState::WaitingForFunds(_)),
    }
}

/// The value of `record` in raw units of 1/Q tick at `tick` (`progress::value_at`).
pub(crate) fn record_value(record: &RateRecord, tick: u32) -> u64 {
    progress::value_at(&rate_record(record), tick)
}

/// The HUD field `Paid` of one cost entry at `tick` (`progress::paid_at`).
pub(crate) fn record_paid(record: &RateRecord, cost: u64, tick: u32) -> u64 {
    progress::paid_at(&rate_record(record), cost, tick)
}

/// The HUD field `Refund` of one cost entry at `tick` (`progress::refund_at`).
pub(crate) fn record_refund(
    record: &RateRecord,
    cost: u64,
    case: CancelCase,
    refunds: &CancelRefunds,
    tick: u32,
) -> u64 {
    progress::refund_at(&rate_record(record), cost, case, refunds, tick)
}

fn finite(pos: turts_net_protocol::Pos) -> bool {
    pos.x.is_finite() && pos.y.is_finite()
}

/// The unit is known, owned by the receiving slot and not removed by the same message.
fn own_unit(
    context: &ReplicaContext,
    units: &HashMap<u32, UnitReplica>,
    removed: &HashSet<u32>,
    unit: u32,
) -> bool {
    !removed.contains(&unit)
        && units
            .get(&unit)
            .is_some_and(|replica| replica.owner_slot == context.slot)
}

fn unique<T, K: Eq + std::hash::Hash>(records: &[T], key: impl Fn(&T) -> K) -> bool {
    let mut seen = HashSet::new();
    records.iter().all(|record| seen.insert(key(record)))
}

fn valid_orders(
    context: &ReplicaContext,
    units: &HashMap<u32, UnitReplica>,
    queue: &ConstructionQueue,
) -> bool {
    let Some(constructor) = units
        .get(&queue.constructor.0)
        .and_then(|unit| ContentId::parse(&unit.content_id).ok())
        .and_then(|id| context.content.catalog.units.get(&id))
        .and_then(|unit| unit.constructor.as_ref())
    else {
        return false;
    };
    if queue.orders.len() > usize::from(constructor.queue_capacity) {
        return false;
    }
    queue.orders.iter().all(|order| {
        finite(order.point)
            && ContentId::parse(&order.content).ok().is_some_and(|id| {
                constructor.buildings.contains(&id)
                    && context
                        .content
                        .catalog
                        .production
                        .get(&id)
                        .is_some_and(|recipe| {
                            context
                                .content
                                .catalog
                                .units
                                .get(&recipe.output_unit)
                                .is_some_and(|unit| unit.placement.is_some())
                        })
            })
    })
}

fn valid_supply(context: &ReplicaContext, record: &SupplyRecord) -> bool {
    record.slot.0 == context.slot
        && ContentId::parse(&record.resource)
            .ok()
            .and_then(|id| context.content.catalog.resources.get(&id))
            .is_some_and(|resource| matches!(resource.kind, ResourceKind::Supply { .. }))
}

fn valid_match_state(state: &MatchState) -> bool {
    unique(&state.connections, |connection| connection.slot.0)
}

/// The job records that the economy of the same message holds. A record of a job that the queues
/// do not hold is dropped (the job completed or was cancelled). `None` rejects the message: a
/// duplicate, or a producer that the same message removes.
fn held_jobs(
    records: &[JobProgress],
    economy: &EconomyState,
    removed: &HashSet<u32>,
) -> Option<Vec<((u32, u64), RateRecord)>> {
    let mut seen = HashSet::new();
    let mut kept = Vec::new();
    for record in records {
        let key = (record.producer.0, record.job.0);
        if removed.contains(&key.0) || !seen.insert(key) {
            return None;
        }
        let held = economy.queues.iter().any(|queue| {
            queue.producer.0 == key.0 && queue.jobs.iter().any(|job| job.id.0 == key.1)
        });
        if held {
            kept.push((key, record.progress.clone()));
        }
    }
    Some(kept)
}

/// The progress of each own site. A record names an own site that the message does not remove.
fn site_progress_rows(
    context: &ReplicaContext,
    units: &HashMap<u32, UnitReplica>,
    removed: &HashSet<u32>,
    is_site: impl Fn(u32) -> bool,
    records: &[SiteProgress],
) -> Option<Vec<(u32, RateRecord)>> {
    if !unique(records, |record| record.unit.0) {
        return None;
    }
    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        let unit = record.unit.0;
        if !own_unit(context, units, removed, unit) || !is_site(unit) {
            return None;
        }
        rows.push((unit, record.progress.clone()));
    }
    Some(rows)
}

fn construction_rows(
    context: &ReplicaContext,
    units: &HashMap<u32, UnitReplica>,
    removed: &HashSet<u32>,
    records: &[ConstructionQueue],
) -> Option<Vec<(u32, Vec<HeldOrder>)>> {
    if !unique(records, |record| record.constructor.0) {
        return None;
    }
    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        if !own_unit(context, units, removed, record.constructor.0)
            || !valid_orders(context, units, record)
        {
            return None;
        }
        rows.push((record.constructor.0, record.orders.clone()));
    }
    Some(rows)
}

fn supply_rows(context: &ReplicaContext, records: &[SupplyRecord]) -> Option<Vec<SupplyRecord>> {
    if !unique(records, |record| (record.slot.0, record.resource.clone()))
        || !records.iter().all(|record| valid_supply(context, record))
    {
        return None;
    }
    Some(records.to_vec())
}

/// Stage the store of a `Snapshot`: a whole new store. `units` and `economy` are those of the
/// staged candidate. `None` rejects the Snapshot.
pub(crate) fn stage_snapshot_store(
    context: &ReplicaContext,
    snapshot: &turts_net_protocol::Snapshot,
    units: &HashMap<u32, UnitReplica>,
    economy: &EconomyState,
) -> Option<RecordStore> {
    let no_removed = HashSet::new();
    let records = &snapshot.records;
    let sites: BTreeSet<u32> = snapshot
        .units
        .iter()
        .filter(|unit| unit.site)
        .map(|unit| unit.unit_id.0)
        .collect();
    let site_progress = site_progress_rows(
        context,
        units,
        &no_removed,
        |unit| sites.contains(&unit),
        &records.sites,
    )?;
    let construction_queues =
        construction_rows(context, units, &no_removed, &records.construction_queues)?
            .into_iter()
            .filter(|(_, orders)| !orders.is_empty())
            .collect();
    let supply = supply_rows(context, &records.supply)?;
    let match_state = records.match_state.clone().unwrap_or_default();
    if !valid_match_state(&match_state) {
        return None;
    }
    Some(RecordStore {
        tick: snapshot.tick,
        sites,
        site_progress: site_progress.into_iter().collect(),
        construction_queues,
        supply: supply
            .into_iter()
            .map(|record| ((record.slot.0, record.resource.clone()), record))
            .collect(),
        match_state,
        job_progress: held_jobs(&records.jobs, economy, &no_removed)?
            .into_iter()
            .collect(),
    })
}

/// Stage the store change of one delta against `base`, the store of the previous message of the
/// chain (the live store, or the staged store of a resync). `units` holds the units after the
/// spawns of this delta and before its removals; `economy` is the economy after the merge of this
/// delta. `None` rejects the delta.
pub(crate) fn stage_delta_step(
    context: &ReplicaContext,
    base: &RecordStore,
    units: &HashMap<u32, UnitReplica>,
    economy: &EconomyState,
    delta: &WorldDelta,
    removed: &HashSet<u32>,
) -> Option<StoreStep> {
    let records = &delta.records;
    let sites_added: Vec<u32> = delta
        .spawned
        .iter()
        .filter(|unit: &&UnitSnapshot| unit.site)
        .map(|unit| unit.unit_id.0)
        .collect();
    let is_site = |unit: u32| base.sites.contains(&unit) || sites_added.contains(&unit);
    if !unique(&records.sites_completed, |unit| unit.0)
        || records.sites_completed.iter().any(|unit| {
            !is_site(unit.0) || removed.contains(&unit.0) || !units.contains_key(&unit.0)
        })
    {
        return None;
    }
    let sites_completed: Vec<u32> = records.sites_completed.iter().map(|unit| unit.0).collect();
    // A record of a site that completes in the same delta is dropped with the site.
    let site_progress = site_progress_rows(context, units, removed, is_site, &records.sites)?
        .into_iter()
        .filter(|(unit, _)| !sites_completed.contains(unit))
        .collect();
    if let Some(state) = &records.match_state
        && !valid_match_state(state)
    {
        return None;
    }
    Some(StoreStep {
        tick: delta.tick,
        removed: removed.iter().copied().collect(),
        sites_added,
        sites_completed,
        site_progress,
        queue_jobs: delta
            .economy
            .queues
            .iter()
            .map(|queue| {
                (
                    queue.producer.0,
                    queue.jobs.iter().map(|job| job.id.0).collect(),
                )
            })
            .collect(),
        job_progress: held_jobs(&records.jobs, economy, removed)?,
        construction_queues: construction_rows(
            context,
            units,
            removed,
            &records.construction_queues,
        )?,
        supply: supply_rows(context, &records.supply)?,
        match_state: records.match_state.clone(),
    })
}

/// The outcome of `slot` (`Value::Outcome`, `Value::OutcomeReason`).
pub(crate) fn outcome_of(state: &MatchState, slot: u8) -> Option<SlotOutcome> {
    state.outcomes.get(usize::from(slot)).copied().flatten()
}
