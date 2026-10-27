//! Atomic client replica for public world state and the local player's private economy.
//!
//! A batch is validated against resolved package definitions and staged as a complete candidate
//! before it can change the displayed world. While recovering, the replica retains a bounded,
//! sorted chain and freezes the latest coherent candidate.

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::platform::time::Instant;
use turts_mod_api::{ContentId, CostSource, ResourceKind};
use turts_net_protocol::{
    EconomyChanges, EconomyState, JobId, NodeId, NodeState, Pos, Snapshot, UnitSnapshot, WorldDelta,
};

use crate::{content::ClientContent, interp};

/// One unit on the client.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitReplica {
    pub content_id: String,
    pub owner_slot: u8,
    /// Ascending, consecutive ticks.
    pub samples: VecDeque<(u32, Pos)>,
}

/// The position of one unit at a presentation time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Presented {
    pub pos: Pos,
    pub lo_tick: u32,
    pub hi_tick: u32,
    pub fraction: f32,
}

#[derive(Debug, PartialEq)]
pub enum DeltaOutcome {
    Applied,
    Discarded,
    Buffered,
    ResyncRequested,
}

#[derive(Debug, PartialEq)]
pub enum SnapshotOutcome {
    Applied { replayed: u32 },
    ResyncRequested { replayed: u32 },
    IgnoredStale,
    Invalid,
}

/// Technical cap on retained batches while waiting for a reliable snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplicaConfig {
    pub max_buffered_batches: usize,
}

impl Default for ReplicaConfig {
    fn default() -> Self {
        Self {
            max_buffered_batches: 256,
        }
    }
}

/// The validated catalog and receiving slot used to check private replicated records.
#[derive(Debug, Clone)]
pub struct ReplicaContext {
    pub content: ClientContent,
    pub slot: u8,
}

impl ReplicaContext {
    pub fn new(content: ClientContent, slot: u8) -> Self {
        Self { content, slot }
    }
}

#[derive(Clone)]
struct Candidate {
    units: HashMap<u32, UnitReplica>,
    nodes: HashMap<NodeId, NodeState>,
    economy: EconomyState,
    tick: u32,
}

/// A replica keeps coherent samples and replaces on-wire absolute economy records atomically.
#[derive(Debug, Clone)]
pub struct Replica {
    pub tick_hz: u16,
    pub units: HashMap<u32, UnitReplica>,
    pub nodes: HashMap<NodeId, NodeState>,
    pub economy: EconomyState,
    pub snapshot_tick: Option<u32>,
    pub last_applied_tick: Option<u32>,
    pub latest_receipt: Option<Instant>,
    /// Some while a snapshot/recovery is pending. Retained deltas are sorted and bounded.
    pub resync: Option<Vec<WorldDelta>>,
    /// Highest observed tick is kept even if an overflow discards its triggering batch.
    pub highest_observed_tick: Option<u32>,
    /// Number of bounded-tail overflows, copied into ingest diagnostics.
    pub buffer_overflows: u32,
    context: Option<ReplicaContext>,
    config: ReplicaConfig,
    gap_anchor: Option<u32>,
    gap_seen: bool,
}

impl Replica {
    /// Preserve the original constructor for existing call sites. Live ingest sets a validated
    /// context from acquired packages and Welcome before any snapshot is accepted.
    pub fn new(tick_hz: u16) -> Self {
        Self::with_config(tick_hz, ReplicaConfig::default())
    }

    pub fn with_config(tick_hz: u16, config: ReplicaConfig) -> Self {
        Self {
            tick_hz,
            units: HashMap::new(),
            nodes: HashMap::new(),
            economy: empty_economy(),
            snapshot_tick: None,
            last_applied_tick: None,
            latest_receipt: None,
            resync: Some(Vec::new()),
            highest_observed_tick: None,
            buffer_overflows: 0,
            context: None,
            config: ReplicaConfig {
                max_buffered_batches: config.max_buffered_batches.max(1),
            },
            gap_anchor: None,
            gap_seen: false,
        }
    }

    pub fn set_context(&mut self, context: ReplicaContext) {
        self.context = Some(context);
    }

    pub fn max_buffered_batches(&self) -> usize {
        self.config.max_buffered_batches
    }

    pub fn buffered_batches(&self) -> usize {
        self.resync.as_ref().map_or(0, Vec::len)
    }

    pub fn resync_span_ticks(&self) -> Option<u32> {
        let high = self.highest_observed_tick?;
        let low = self
            .last_applied_tick
            .or(self.gap_anchor)
            .or_else(|| self.resync.as_ref()?.iter().map(|d| d.tick).min())?;
        Some(high.saturating_sub(low))
    }

    pub fn in_resync(&self) -> bool {
        self.resync.is_some()
    }

    pub fn apply_snapshot(&mut self, snapshot: &Snapshot, now: Instant) -> SnapshotOutcome {
        if self
            .last_applied_tick
            .is_some_and(|applied| snapshot.tick <= applied)
        {
            return SnapshotOutcome::IgnoredStale;
        }

        let Some(mut candidate) = self.stage_snapshot(snapshot) else {
            self.mark_gap(snapshot.tick);
            self.resync.get_or_insert_with(Vec::new);
            return SnapshotOutcome::Invalid;
        };

        let mut retained = self.resync.clone().unwrap_or_default();
        retained.sort_by_key(|d| (d.tick, d.base_tick));
        retained.dedup_by_key(|d| d.tick);
        let original_retained = retained.clone();
        let mut replayed = 0;
        let mut remaining = Vec::new();
        let mut replay_failed = false;
        for delta in retained {
            if delta.tick <= candidate.tick {
                continue;
            }
            if !self.chain_extends(candidate.tick, &delta) {
                replay_failed = true;
                remaining.push(delta);
                continue;
            }
            let Some(next) = self.stage_delta(&candidate, &delta) else {
                self.mark_gap(delta.tick);
                replay_failed = true;
                remaining.push(delta);
                continue;
            };
            candidate = next;
            replayed += 1;
        }

        // A recovery response is published only if its entire proposed tail is coherent. In
        // particular, do not expose a valid prefix while a later retained batch is invalid or
        // disconnected. Keep the pre-recovery display and all retained evidence for the next
        // reliable response.
        let uncovered_high_water = self
            .highest_observed_tick
            .is_some_and(|high| candidate.tick < high);
        let unresolved_gap = self.gap_seen && uncovered_high_water;
        if replay_failed || !remaining.is_empty() || unresolved_gap {
            self.resync = Some(original_retained);
            self.gap_seen = true;
            return SnapshotOutcome::ResyncRequested { replayed: 0 };
        }

        self.commit(candidate, now);
        self.snapshot_tick = Some(snapshot.tick);
        self.resync = None;
        self.gap_anchor = None;
        self.gap_seen = false;
        SnapshotOutcome::Applied { replayed }
    }

    pub fn apply_delta(&mut self, delta: &WorldDelta, now: Instant) -> DeltaOutcome {
        self.highest_observed_tick = Some(
            self.highest_observed_tick
                .map_or(delta.tick, |high| high.max(delta.tick)),
        );
        if self
            .last_applied_tick
            .is_some_and(|applied| delta.tick <= applied)
        {
            return DeltaOutcome::Discarded;
        }
        if self.resync.is_some() {
            self.buffer_delta(delta.clone());
            return DeltaOutcome::Buffered;
        }
        let Some(tick) = self.last_applied_tick else {
            self.mark_gap(delta.tick);
            self.resync = Some(Vec::new());
            self.buffer_delta(delta.clone());
            return DeltaOutcome::ResyncRequested;
        };
        if !self.chain_extends(tick, delta) {
            self.mark_gap(delta.tick);
            self.resync = Some(Vec::new());
            self.buffer_delta(delta.clone());
            return DeltaOutcome::ResyncRequested;
        }
        let current = self.candidate_at(tick);
        let Some(candidate) = self.stage_delta(&current, delta) else {
            self.mark_gap(delta.tick);
            self.resync = Some(Vec::new());
            self.buffer_delta(delta.clone());
            return DeltaOutcome::ResyncRequested;
        };
        self.commit(candidate, now);
        DeltaOutcome::Applied
    }

    fn chain_extends(&self, current_tick: u32, delta: &WorldDelta) -> bool {
        delta.base_tick == current_tick && current_tick.checked_add(1) == Some(delta.tick)
    }

    fn mark_gap(&mut self, observed_tick: u32) {
        if self.gap_anchor.is_none() {
            self.gap_anchor = self
                .last_applied_tick
                .or(Some(observed_tick.saturating_sub(1)));
        }
        self.gap_seen = true;
    }

    fn buffer_delta(&mut self, delta: WorldDelta) {
        let limit = self.config.max_buffered_batches;
        let buffer = self.resync.get_or_insert_with(Vec::new);
        if buffer.iter().any(|held| held.tick == delta.tick) {
            return;
        }
        if buffer.len() >= limit {
            buffer.clear();
            self.buffer_overflows = self.buffer_overflows.saturating_add(1);
            self.gap_seen = true;
            if self.gap_anchor.is_none() {
                self.gap_anchor = self.last_applied_tick.or(Some(delta.base_tick));
            }
            // Task 12 policy: discard the triggering batch as well as the old tail.
            return;
        }
        if self.last_applied_tick.is_none() && self.gap_anchor.is_none() {
            self.gap_anchor = Some(delta.base_tick);
        }
        buffer.push(delta);
        buffer.sort_by_key(|d| (d.tick, d.base_tick));
        let mut components: Vec<Vec<WorldDelta>> = Vec::new();
        for held in std::mem::take(buffer) {
            let joins = components
                .last()
                .and_then(|component| component.last())
                .is_some_and(|previous| {
                    held.base_tick == previous.tick
                        && held.base_tick.checked_add(1) == Some(held.tick)
                });
            if joins {
                components.last_mut().expect("component exists").push(held);
            } else {
                components.push(vec![held]);
            }
        }
        if components.len() > 1 {
            self.gap_seen = true;
            if self.gap_anchor.is_none() {
                self.gap_anchor = components
                    .first()
                    .and_then(|component| component.first())
                    .map(|first| first.base_tick);
            }
            *buffer = components.pop().unwrap_or_default();
        } else {
            *buffer = components.pop().unwrap_or_default();
        }
    }

    fn stage_snapshot(&self, snapshot: &Snapshot) -> Option<Candidate> {
        let context = self.context.as_ref()?;
        let mut units = HashMap::new();
        for unit in &snapshot.units {
            if units.contains_key(&unit.unit_id.0) || !valid_unit(context, unit) {
                return None;
            }
            units.insert(
                unit.unit_id.0,
                UnitReplica {
                    content_id: unit.content_id.clone(),
                    owner_slot: unit.owner_slot.0,
                    samples: VecDeque::from([(snapshot.tick, unit.pos)]),
                },
            );
        }
        let mut nodes = HashMap::new();
        for node in &snapshot.nodes {
            if nodes.contains_key(&node.id) || !valid_node(context, node) {
                return None;
            }
            nodes.insert(node.id, node.clone());
        }
        if !valid_economy(context, &units, &nodes, &snapshot.economy) {
            return None;
        }
        Some(Candidate {
            units,
            nodes,
            economy: snapshot.economy.clone(),
            tick: snapshot.tick,
        })
    }

    fn candidate_at(&self, tick: u32) -> Candidate {
        Candidate {
            units: self.units.clone(),
            nodes: self.nodes.clone(),
            economy: self.economy.clone(),
            tick,
        }
    }

    fn stage_delta(&self, current: &Candidate, delta: &WorldDelta) -> Option<Candidate> {
        let context = self.context.as_ref()?;
        let mut next = current.clone();
        let removed: HashSet<u32> = delta.removed.iter().map(|id| id.0).collect();
        if removed.len() != delta.removed.len()
            || removed.iter().any(|id| !next.units.contains_key(id))
        {
            return None;
        }
        let mut spawned = HashSet::new();
        for unit in &delta.spawned {
            if !spawned.insert(unit.unit_id.0)
                || next.units.contains_key(&unit.unit_id.0)
                || removed.contains(&unit.unit_id.0)
                || !valid_unit(context, unit)
            {
                return None;
            }
            next.units.insert(
                unit.unit_id.0,
                UnitReplica {
                    content_id: unit.content_id.clone(),
                    owner_slot: unit.owner_slot.0,
                    samples: VecDeque::from([(delta.tick, unit.pos)]),
                },
            );
        }
        for id in &removed {
            next.units.remove(id);
            next.economy
                .inventories
                .retain(|record| record.unit.0 != *id);
            next.economy.orders.retain(|record| {
                record.unit.0 != *id && record.receiver.is_none_or(|receiver| receiver.0 != *id)
            });
            next.economy
                .queues
                .retain(|record| record.producer.0 != *id);
        }

        let mut positions = HashMap::new();
        for position in &delta.positions {
            if !finite_pos(position.pos)
                || positions.insert(position.unit_id.0, position.pos).is_some()
                || !next.units.contains_key(&position.unit_id.0)
                || removed.contains(&position.unit_id.0)
            {
                return None;
            }
        }
        for (id, unit) in &mut next.units {
            if spawned.contains(id) {
                if let Some(position) = positions.get(id) {
                    unit.samples.back_mut().expect("spawn sample").1 = *position;
                }
                continue;
            }
            let position = positions
                .get(id)
                .copied()
                .or_else(|| unit.samples.back().map(|s| s.1))?;
            unit.samples.push_back((delta.tick, position));
            let keep_from = delta.tick.saturating_sub(PRUNE_TICKS);
            while unit
                .samples
                .front()
                .is_some_and(|&(tick, _)| tick < keep_from)
            {
                unit.samples.pop_front();
            }
        }

        let mut changed_nodes = HashSet::new();
        for node in &delta.nodes {
            if !changed_nodes.insert(node.id) || !valid_node(context, node) {
                return None;
            }
            next.nodes.insert(node.id, node.clone());
        }
        if !merge_economy(&mut next.economy, &delta.economy) {
            return None;
        }
        if !valid_economy(context, &next.units, &next.nodes, &next.economy) {
            return None;
        }
        next.tick = delta.tick;
        Some(next)
    }

    fn commit(&mut self, candidate: Candidate, now: Instant) {
        self.units = candidate.units;
        self.nodes = candidate.nodes;
        self.economy = candidate.economy;
        self.last_applied_tick = Some(candidate.tick);
        self.latest_receipt = Some(now);
        while self.units.values().any(|unit| {
            unit.samples
                .front()
                .is_some_and(|(tick, _)| *tick < candidate.tick.saturating_sub(PRUNE_TICKS))
        }) {
            for unit in self.units.values_mut() {
                while unit
                    .samples
                    .front()
                    .is_some_and(|(tick, _)| *tick < candidate.tick.saturating_sub(PRUNE_TICKS))
                {
                    unit.samples.pop_front();
                }
            }
        }
    }

    /// `None` before the first snapshot.
    pub fn presentation_time(&self, now: Instant) -> Option<f64> {
        let latest = self.last_applied_tick?;
        let receipt = self.latest_receipt?;
        let since = now.saturating_duration_since(receipt).as_secs_f64();
        Some(interp::presentation_time(latest, since, self.tick_hz))
    }

    /// The position of `unit_id` at presentation time `t`. It never reads above the latest sample.
    pub fn sample(&self, unit_id: u32, t: f64) -> Option<Presented> {
        let samples = &self.units.get(&unit_id)?.samples;
        let &(first, first_pos) = samples.front()?;
        let &(last, last_pos) = samples.back()?;
        let floor = t.floor();
        if floor.is_nan() || floor < f64::from(first) {
            return Some(Presented {
                pos: first_pos,
                lo_tick: first,
                hi_tick: first,
                fraction: 0.0,
            });
        }
        if floor >= f64::from(last) {
            return Some(Presented {
                pos: last_pos,
                lo_tick: last,
                hi_tick: last,
                fraction: 0.0,
            });
        }
        let lo = floor as u32;
        let hi = lo + 1;
        let fraction = (t - floor) as f32;
        let a = samples[(lo - first) as usize].1;
        let b = samples[(hi - first) as usize].1;
        Some(Presented {
            pos: interp::lerp(a, b, fraction),
            lo_tick: lo,
            hi_tick: hi,
            fraction,
        })
    }
}

const PRUNE_TICKS: u32 = 16;

fn empty_economy() -> EconomyState {
    EconomyState {
        balances: Vec::new(),
        inventories: Vec::new(),
        orders: Vec::new(),
        queues: Vec::new(),
    }
}

fn parse_id(text: &str) -> Option<ContentId> {
    ContentId::parse(text).ok()
}

fn finite_pos(pos: Pos) -> bool {
    pos.x.is_finite() && pos.y.is_finite()
}

fn valid_unit(context: &ReplicaContext, unit: &UnitSnapshot) -> bool {
    finite_pos(unit.pos)
        && parse_id(&unit.content_id)
            .is_some_and(|id| context.content.catalog.units.contains_key(&id))
}

fn valid_node(context: &ReplicaContext, node: &NodeState) -> bool {
    if !finite_pos(node.pos) {
        return false;
    }
    let Some(id) = parse_id(&node.content_id) else {
        return false;
    };
    let Some(definition) = context.content.catalog.nodes.get(&id) else {
        return false;
    };
    let Some(item) = context.content.catalog.resources.get(&definition.item) else {
        return false;
    };
    item.kind == ResourceKind::Item
}

fn valid_economy(
    context: &ReplicaContext,
    units: &HashMap<u32, UnitReplica>,
    nodes: &HashMap<NodeId, NodeState>,
    economy: &EconomyState,
) -> bool {
    let slot = context.slot;
    let mut balances = HashSet::new();
    for balance in &economy.balances {
        let Some(resource) = parse_id(&balance.resource) else {
            return false;
        };
        if !balances.insert(resource.clone())
            || context
                .content
                .catalog
                .resources
                .get(&resource)
                .is_none_or(|definition| definition.kind != ResourceKind::Balance)
            || balance.reserved > balance.on_hand
        {
            return false;
        }
    }

    let mut inventories = HashSet::new();
    for inventory in &economy.inventories {
        if !inventories.insert(inventory.unit.0) {
            return false;
        }
        let Some(unit) = units
            .get(&inventory.unit.0)
            .filter(|u| u.owner_slot == slot)
        else {
            return false;
        };
        let Some(unit_id) = parse_id(&unit.content_id) else {
            return false;
        };
        let Some(definition) = context
            .content
            .catalog
            .units
            .get(&unit_id)
            .and_then(|u| u.inventory.as_ref())
        else {
            return false;
        };
        let mut resources = HashSet::new();
        let mut total = 0u64;
        for item in &inventory.items {
            let Some(resource) = parse_id(&item.resource) else {
                return false;
            };
            if !resources.insert(resource.clone())
                || !definition.accepts.contains(&resource)
                || context
                    .content
                    .catalog
                    .resources
                    .get(&resource)
                    .is_none_or(|resource| resource.kind != ResourceKind::Item)
                || item.reserved > item.on_hand
            {
                return false;
            }
            let Some(next_total) = total.checked_add(item.on_hand) else {
                return false;
            };
            total = next_total;
        }
        if total > definition.capacity {
            return false;
        }
    }

    let mut orders = HashSet::new();
    for order in &economy.orders {
        if !orders.insert(order.unit.0) {
            return false;
        }
        let Some(worker) = units.get(&order.unit.0).filter(|u| u.owner_slot == slot) else {
            return false;
        };
        let Some(worker_type) = parse_id(&worker.content_id) else {
            return false;
        };
        let Some(worker_def) = context.content.catalog.units.get(&worker_type) else {
            return false;
        };
        let Some(gatherer) = worker_def.gatherer.as_ref() else {
            return false;
        };
        let node_item = if let Some(node_id) = order.node {
            let Some(node) = nodes.get(&node_id) else {
                return false;
            };
            let Some(node_type) = parse_id(&node.content_id) else {
                return false;
            };
            let Some(node_def) = context.content.catalog.nodes.get(&node_type) else {
                return false;
            };
            if !gatherer.items.contains(&node_def.item) {
                return false;
            }
            Some(node_def.item.clone())
        } else {
            None
        };
        if let Some(receiver_id) = order.receiver {
            let Some(receiver) = units.get(&receiver_id.0).filter(|u| u.owner_slot == slot) else {
                return false;
            };
            let Some(receiver_type) = parse_id(&receiver.content_id) else {
                return false;
            };
            let Some(receiver_def) = context.content.catalog.units.get(&receiver_type) else {
                return false;
            };
            let Some(drop_off) = receiver_def.drop_off.as_ref() else {
                return false;
            };
            let item_matches =
                |item: &ContentId| drop_off.rules.iter().any(|rule| rule.item == *item);
            if node_item.as_ref().is_some_and(|item| !item_matches(item))
                || (node_item.is_none() && !gatherer.items.iter().any(item_matches))
            {
                return false;
            }
        }
    }

    let mut queue_producers = HashSet::new();
    let mut job_ids = HashSet::<JobId>::new();
    for queue in &economy.queues {
        if !queue_producers.insert(queue.producer.0) {
            return false;
        }
        let Some(producer) = units
            .get(&queue.producer.0)
            .filter(|u| u.owner_slot == slot)
        else {
            return false;
        };
        let Some(producer_type) = parse_id(&producer.content_id) else {
            return false;
        };
        let Some(producer_def) = context
            .content
            .catalog
            .units
            .get(&producer_type)
            .and_then(|u| u.producer.as_ref())
        else {
            return false;
        };
        if queue.jobs.len() > usize::from(producer_def.queue_capacity) {
            return false;
        }
        for job in &queue.jobs {
            let Some(recipe_id) = parse_id(&job.recipe) else {
                return false;
            };
            let Some(recipe) = context.content.catalog.production.get(&recipe_id) else {
                return false;
            };
            if !job_ids.insert(job.id) || !producer_def.recipes.contains(&recipe_id) {
                return false;
            }
            for costs in [&job.charged, &job.reserved] {
                let mut endpoints = HashSet::new();
                for cost in costs {
                    let Some(resource_id) = parse_id(&cost.resource) else {
                        return false;
                    };
                    if !endpoints.insert((cost.source as u8, resource_id.clone())) {
                        return false;
                    }
                    let expected_kind = match cost.source {
                        CostSource::SlotBalance => ResourceKind::Balance,
                        CostSource::ProducerInventory => ResourceKind::Item,
                    };
                    if context
                        .content
                        .catalog
                        .resources
                        .get(&resource_id)
                        .is_none_or(|resource| resource.kind != expected_kind)
                        || !recipe.costs.iter().any(|expected| {
                            expected.source == cost.source
                                && expected.resource == resource_id
                                && cost.amount <= expected.amount
                        })
                    {
                        return false;
                    }
                    if cost.source == CostSource::ProducerInventory {
                        let Some(inventory_def) = context
                            .content
                            .catalog
                            .units
                            .get(&producer_type)
                            .and_then(|u| u.inventory.as_ref())
                        else {
                            return false;
                        };
                        if !inventory_def.accepts.contains(&resource_id)
                            || !economy
                                .inventories
                                .iter()
                                .any(|inventory| inventory.unit == queue.producer)
                        {
                            return false;
                        }
                    }
                }
            }
        }
    }
    true
}

fn merge_economy(current: &mut EconomyState, changes: &EconomyChanges) -> bool {
    if !unique(
        changes
            .balances
            .iter()
            .map(|record| record.resource.as_str()),
    ) || !unique(changes.inventories.iter().map(|record| record.unit.0))
        || !unique(changes.orders.iter().map(|record| record.unit.0))
        || !unique(changes.queues.iter().map(|record| record.producer.0))
    {
        return false;
    }
    merge_by(&mut current.balances, &changes.balances, |record| {
        &record.resource
    });
    merge_by(&mut current.inventories, &changes.inventories, |record| {
        &record.unit
    });
    merge_by(&mut current.orders, &changes.orders, |record| &record.unit);
    merge_by(&mut current.queues, &changes.queues, |record| {
        &record.producer
    });
    true
}

fn unique<T: Eq + std::hash::Hash>(keys: impl Iterator<Item = T>) -> bool {
    let mut seen = HashSet::new();
    keys.into_iter().all(|key| seen.insert(key))
}

fn merge_by<T: Clone, K: Eq + std::hash::Hash>(
    current: &mut Vec<T>,
    changes: &[T],
    key: impl Fn(&T) -> &K,
) {
    for change in changes {
        let change_key = key(change);
        if let Some(index) = current
            .iter()
            .position(|existing| key(existing) == change_key)
        {
            current[index] = change.clone();
        } else {
            current.push(change.clone());
        }
    }
}
