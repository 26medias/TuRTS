//! Atomic client replica for public world state and the local player's private economy.
//!
//! A batch is validated against resolved package definitions and staged as a complete candidate
//! before it can change the displayed world. While recovering, the replica retains a bounded,
//! sorted chain and freezes the latest coherent candidate.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use bevy::platform::time::Instant;
use turts_mod_api::{ContentId, CostSource, ResourceKind};
use turts_net_protocol::{
    BeltPieceRecord, EconomyChanges, EconomyState, ExtractorRecord, HealthRecord, InserterRecord,
    InventoryPort, JobId, NodeId, NodeState, Pos, ResearchRecord, Snapshot, UnitSnapshot,
    WeaponRecord, WorldDelta,
};

use crate::{content::ClientContent, interp};

/// One unit on the client.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitReplica {
    pub content_id: String,
    pub owner_slot: u8,
    pub facing: Option<turts_net_protocol::Facing>,
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
    belts: Vec<BeltPieceRecord>,
    inserters: Vec<InserterRecord>,
    extractors: Vec<ExtractorRecord>,
    weapons: Vec<WeaponRecord>,
    healths: Vec<HealthRecord>,
    research: Vec<ResearchRecord>,
    health_trace: std::collections::BTreeMap<u32, Vec<HealthTraceEntry>>,
    tick: u32,
}

/// A public health value observed in a coherent server tick. Equal values are omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthTraceEntry {
    pub tick: u32,
    pub current: u64,
}

/// A replica keeps coherent samples and replaces on-wire absolute records atomically.
#[derive(Debug, Clone)]
pub struct Replica {
    pub tick_hz: u16,
    pub units: HashMap<u32, UnitReplica>,
    pub nodes: HashMap<NodeId, NodeState>,
    pub economy: EconomyState,
    pub belts: Vec<BeltPieceRecord>,
    pub inserters: Vec<InserterRecord>,
    pub extractors: Vec<ExtractorRecord>,
    pub weapons: Vec<WeaponRecord>,
    pub healths: Vec<HealthRecord>,
    pub research: Vec<ResearchRecord>,
    pub health_trace: std::collections::BTreeMap<u32, Vec<HealthTraceEntry>>,
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
            belts: Vec::new(),
            inserters: Vec::new(),
            extractors: Vec::new(),
            weapons: Vec::new(),
            healths: Vec::new(),
            research: Vec::new(),
            health_trace: std::collections::BTreeMap::new(),
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

    /// Change the retained-tail bound without replacing the replica or its coherent world.
    /// If the existing recovery tail is larger than the new bound, apply the same overflow
    /// policy as `buffer_delta`: discard the tail and retain a gap anchor.
    pub fn set_max_buffered_batches(&mut self, max_buffered_batches: usize) {
        let max_buffered_batches = max_buffered_batches.max(1);
        self.config.max_buffered_batches = max_buffered_batches;
        let Some(buffer) = self.resync.as_mut() else {
            return;
        };
        if buffer.len() > max_buffered_batches {
            let anchor = buffer.first().map(|delta| delta.base_tick);
            buffer.clear();
            self.buffer_overflows = self.buffer_overflows.saturating_add(1);
            self.gap_seen = true;
            if self.gap_anchor.is_none() {
                self.gap_anchor = self.last_applied_tick.or(anchor);
            }
        }
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
                    facing: unit.facing,
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
        let mut candidate = Candidate {
            units,
            nodes,
            economy: snapshot.economy.clone(),
            belts: snapshot.belts.clone(),
            inserters: snapshot.inserters.clone(),
            extractors: snapshot.extractors.clone(),
            weapons: snapshot.weapons.clone(),
            healths: Vec::new(),
            research: snapshot.research.clone(),
            health_trace: self.health_trace.clone(),
            tick: snapshot.tick,
        };
        if !valid_economy(
            context,
            &candidate.units,
            &candidate.nodes,
            &candidate.economy,
        ) || !valid_m3_records(context, &candidate.units, &candidate.nodes, &candidate)
            || !apply_health_records(context, snapshot.tick, &snapshot.healths, &mut candidate)
        {
            return None;
        }
        Some(candidate)
    }

    fn candidate_at(&self, tick: u32) -> Candidate {
        Candidate {
            units: self.units.clone(),
            nodes: self.nodes.clone(),
            economy: self.economy.clone(),
            belts: self.belts.clone(),
            inserters: self.inserters.clone(),
            extractors: self.extractors.clone(),
            weapons: self.weapons.clone(),
            healths: self.healths.clone(),
            research: self.research.clone(),
            health_trace: self.health_trace.clone(),
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
                    facing: unit.facing,
                    samples: VecDeque::from([(delta.tick, unit.pos)]),
                },
            );
        }

        let mut positions = HashMap::new();
        for position in &delta.positions {
            if !finite_pos(position.pos)
                || positions.insert(position.unit_id.0, position.pos).is_some()
                || !next.units.contains_key(&position.unit_id.0)
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
        if !validate_delta_m3(context, &next.units, &next.nodes, delta, &removed)
            || !apply_health_records(context, delta.tick, &delta.healths, &mut next)
            || !merge_economy(&mut next.economy, &delta.economy)
            || !merge_m3_records(&mut next, delta)
        {
            return None;
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
            next.belts.retain(|record| record.piece.0 != *id);
            next.inserters.retain(|record| record.unit.0 != *id);
            next.extractors.retain(|record| record.unit.0 != *id);
            next.weapons.retain(|record| record.unit.0 != *id);
            next.healths.retain(|record| record.unit.0 != *id);
        }
        if !valid_economy(context, &next.units, &next.nodes, &next.economy)
            || !valid_m3_records(context, &next.units, &next.nodes, &next)
        {
            return None;
        }
        next.tick = delta.tick;
        Some(next)
    }

    fn commit(&mut self, candidate: Candidate, now: Instant) {
        self.units = candidate.units;
        self.nodes = candidate.nodes;
        self.economy = candidate.economy;
        self.belts = candidate.belts;
        self.inserters = candidate.inserters;
        self.extractors = candidate.extractors;
        self.weapons = candidate.weapons;
        self.healths = candidate.healths;
        self.research = candidate.research;
        self.health_trace = candidate.health_trace;
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
        let port_key = (
            inventory.unit.0,
            match inventory.port {
                InventoryPort::Main => 0u8,
                InventoryPort::Output => 1u8,
            },
        );
        if !inventories.insert(port_key) {
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
        let Some(definition) =
            context
                .content
                .catalog
                .units
                .get(&unit_id)
                .and_then(|u| match inventory.port {
                    InventoryPort::Main => u.inventory.as_ref(),
                    InventoryPort::Output => u.output_inventory.as_ref(),
                })
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
        let Some(unit_state) = units.get(&order.unit.0).filter(|u| u.owner_slot == slot) else {
            return false;
        };
        let Some(worker_type) = parse_id(&unit_state.content_id) else {
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
        let Some(owner_unit) = units
            .get(&queue.producer.0)
            .filter(|unit| unit.owner_slot == slot)
        else {
            return false;
        };
        let Some(owner_type) = parse_id(&owner_unit.content_id) else {
            return false;
        };
        let Some(unit_def) = context.content.catalog.units.get(&owner_type) else {
            return false;
        };

        // The server keeps producer, machine, and research jobs in one per-unit queue map.
        // Package validation guarantees that a unit has at most one work capability.
        let producer_def = unit_def.producer.as_ref();
        let machine_def = unit_def.machine.as_ref();
        let research_def = unit_def.research.as_ref();
        let queue_capacity = producer_def
            .map(|producer| producer.queue_capacity)
            .or_else(|| machine_def.map(|_| 1))
            .or_else(|| research_def.map(|research| research.queue_capacity));
        let Some(queue_capacity) = queue_capacity else {
            return false;
        };
        if queue.jobs.len() > usize::from(queue_capacity) {
            return false;
        }

        for job in &queue.jobs {
            let Some(recipe_id) = parse_id(&job.recipe) else {
                return false;
            };
            if !job_ids.insert(job.id) {
                return false;
            }
            let expected_cost_lines = if let Some(producer) = producer_def {
                if !producer.recipes.contains(&recipe_id) {
                    return false;
                }
                let Some(definition) = context.content.catalog.production.get(&recipe_id) else {
                    return false;
                };
                definition
                    .costs
                    .iter()
                    .map(|cost| (cost.source, cost.resource.clone(), cost.amount))
                    .collect::<Vec<_>>()
            } else if let Some(machine) = machine_def {
                if machine.recipe != recipe_id {
                    return false;
                }
                let Some(definition) = context.content.catalog.recipes.get(&recipe_id) else {
                    return false;
                };
                definition
                    .inputs
                    .iter()
                    .map(|input| {
                        (
                            CostSource::ProducerInventory,
                            input.item.clone(),
                            input.amount,
                        )
                    })
                    .collect::<Vec<_>>()
            } else if let Some(research) = research_def {
                if !research.technologies.contains(&recipe_id) {
                    return false;
                }
                let Some(definition) = context.content.catalog.technologies.get(&recipe_id) else {
                    return false;
                };
                definition
                    .costs
                    .iter()
                    .map(|cost| (cost.source, cost.resource.clone(), cost.amount))
                    .collect::<Vec<_>>()
            } else {
                return false;
            };
            let mut expected_costs = BTreeMap::<(CostSource, ContentId), u64>::new();
            for (source, resource, amount) in expected_cost_lines {
                let total = expected_costs.entry((source, resource)).or_default();
                let Some(sum) = total.checked_add(amount) else {
                    return false;
                };
                *total = sum;
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
                        || expected_costs
                            .get(&(cost.source, resource_id.clone()))
                            .is_none_or(|amount| cost.amount > *amount)
                    {
                        return false;
                    }
                    if cost.source == CostSource::ProducerInventory {
                        let Some(inventory_def) = unit_def.inventory.as_ref() else {
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
    let mut staged = current.clone();
    if !replace_keyed(&mut staged.balances, &changes.balances, |record| {
        record.resource.clone()
    }) || !replace_keyed(&mut staged.inventories, &changes.inventories, |record| {
        (
            record.unit.0,
            match record.port {
                InventoryPort::Main => 0u8,
                InventoryPort::Output => 1u8,
            },
        )
    }) || !replace_keyed(&mut staged.orders, &changes.orders, |record| record.unit.0)
        || !replace_keyed(&mut staged.queues, &changes.queues, |record| {
            record.producer.0
        })
    {
        return false;
    }
    *current = staged;
    true
}

/// Apply unique absolute replacements. Duplicate input keys fail before any state changes.
pub(crate) fn replace_keyed<T: Clone, K: Eq + std::hash::Hash>(
    current: &mut Vec<T>,
    changes: &[T],
    key: impl Fn(&T) -> K,
) -> bool {
    let mut seen = HashSet::new();
    if !changes.iter().all(|record| seen.insert(key(record))) {
        return false;
    }
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
    true
}

fn merge_m3_records(candidate: &mut Candidate, delta: &WorldDelta) -> bool {
    replace_keyed(&mut candidate.belts, &delta.belts, |record| record.piece.0)
        && replace_keyed(&mut candidate.inserters, &delta.inserters, |record| {
            record.unit.0
        })
        && replace_keyed(&mut candidate.extractors, &delta.extractors, |record| {
            record.unit.0
        })
        && replace_keyed(&mut candidate.weapons, &delta.weapons, |record| {
            record.unit.0
        })
        && replace_keyed(&mut candidate.research, &delta.research, |record| {
            record.slot.0
        })
}

fn validate_delta_m3(
    context: &ReplicaContext,
    units: &HashMap<u32, UnitReplica>,
    nodes: &HashMap<NodeId, NodeState>,
    delta: &WorldDelta,
    removed: &HashSet<u32>,
) -> bool {
    let removed_has = |id: u32| removed.contains(&id);
    if delta.belts.iter().any(|record| removed_has(record.piece.0))
        || delta
            .inserters
            .iter()
            .any(|record| removed_has(record.unit.0))
        || delta
            .extractors
            .iter()
            .any(|record| removed_has(record.unit.0))
        || delta
            .weapons
            .iter()
            .any(|record| removed_has(record.unit.0))
        || delta
            .economy
            .inventories
            .iter()
            .any(|record| removed_has(record.unit.0))
        || delta.economy.orders.iter().any(|record| {
            removed_has(record.unit.0) || record.receiver.is_some_and(|id| removed_has(id.0))
        })
        || delta
            .economy
            .queues
            .iter()
            .any(|record| removed_has(record.producer.0))
    {
        return false;
    }
    let mut candidate = Candidate {
        units: units.clone(),
        nodes: nodes.clone(),
        economy: empty_economy(),
        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
        health_trace: Default::default(),
        tick: delta.tick,
    };
    candidate.belts = delta.belts.clone();
    candidate.inserters = delta.inserters.clone();
    candidate.extractors = delta.extractors.clone();
    candidate.weapons = delta.weapons.clone();
    candidate.research = delta.research.clone();
    let changes_unique = unique_by(&delta.belts, |record| record.piece.0)
        && unique_by(&delta.inserters, |record| record.unit.0)
        && unique_by(&delta.extractors, |record| record.unit.0)
        && unique_by(&delta.weapons, |record| record.unit.0)
        && unique_by(&delta.healths, |record| record.unit.0)
        && unique_by(&delta.research, |record| record.slot.0);
    changes_unique
        && valid_m3_records(context, &candidate.units, &candidate.nodes, &candidate)
        && delta
            .healths
            .iter()
            .all(|record| valid_health(context, &candidate.units, record))
}

fn unique_by<T, K: Eq + std::hash::Hash>(records: &[T], key: impl Fn(&T) -> K) -> bool {
    let mut seen = HashSet::new();
    records.iter().all(|record| seen.insert(key(record)))
}

fn unit_definition<'a>(
    context: &'a ReplicaContext,
    units: &HashMap<u32, UnitReplica>,
    id: u32,
    owner_only: bool,
) -> Option<&'a turts_mod_api::ResolvedUnit> {
    let unit = units.get(&id)?;
    if owner_only && unit.owner_slot != context.slot {
        return None;
    }
    let content_id = parse_id(&unit.content_id)?;
    context.content.catalog.units.get(&content_id)
}

fn valid_health(
    context: &ReplicaContext,
    units: &HashMap<u32, UnitReplica>,
    record: &HealthRecord,
) -> bool {
    unit_definition(context, units, record.unit.0, false)
        .and_then(|definition| definition.health.as_ref())
        .is_some_and(|health| record.current <= health.max)
}

fn apply_health_records(
    context: &ReplicaContext,
    tick: u32,
    changes: &[HealthRecord],
    candidate: &mut Candidate,
) -> bool {
    if !unique_by(changes, |record| record.unit.0)
        || !changes
            .iter()
            .all(|record| valid_health(context, &candidate.units, record))
        || !replace_keyed(&mut candidate.healths, changes, |record| record.unit.0)
    {
        return false;
    }
    for record in changes {
        let history = candidate.health_trace.entry(record.unit.0).or_default();
        if history
            .last()
            .is_none_or(|last| last.current != record.current)
        {
            history.push(HealthTraceEntry {
                tick,
                current: record.current,
            });
        }
    }
    true
}

fn valid_m3_records(
    context: &ReplicaContext,
    units: &HashMap<u32, UnitReplica>,
    nodes: &HashMap<NodeId, NodeState>,
    candidate: &Candidate,
) -> bool {
    if !unique_by(&candidate.belts, |record| record.piece.0)
        || !unique_by(&candidate.inserters, |record| record.unit.0)
        || !unique_by(&candidate.extractors, |record| record.unit.0)
        || !unique_by(&candidate.weapons, |record| record.unit.0)
        || !unique_by(&candidate.healths, |record| record.unit.0)
        || !unique_by(&candidate.research, |record| record.slot.0)
    {
        return false;
    }
    for record in &candidate.belts {
        let Some(definition) = unit_definition(context, units, record.piece.0, true)
            .and_then(|unit| unit.conveyor.as_ref())
        else {
            return false;
        };
        if record.slots.len() != usize::from(definition.slots_per_piece)
            || record.slots.iter().flatten().any(|item| {
                parse_id(item).is_none_or(|id| {
                    !definition.accepts.contains(&id)
                        || context
                            .content
                            .catalog
                            .resources
                            .get(&id)
                            .is_none_or(|resource| resource.kind != ResourceKind::Item)
                })
            })
        {
            return false;
        }
    }
    for record in &candidate.inserters {
        let Some(unit_def) = unit_definition(context, units, record.unit.0, true)
            .filter(|unit| unit.inserter.is_some())
        else {
            return false;
        };
        for (reference, source) in [(record.source, true), (record.destination, false)] {
            if let Some(reference) = reference {
                let Some(target) = unit_definition(context, units, reference.0, true) else {
                    return false;
                };
                let capable = if source {
                    target.output_inventory.is_some()
                        || target.inventory.is_some()
                        || target.conveyor.is_some()
                } else {
                    target.inventory.is_some() || target.conveyor.is_some()
                };
                if !capable || reference.0 == record.unit.0 {
                    return false;
                }
            }
        }
        let _ = unit_def;
    }
    for record in &candidate.extractors {
        let Some(unit_def) = unit_definition(context, units, record.unit.0, true)
            .filter(|unit| unit.extractor.is_some())
        else {
            return false;
        };
        let Some(inventory) = unit_def.inventory.as_ref() else {
            return false;
        };
        if record.node.is_some_and(|id| {
            nodes
                .get(&id)
                .and_then(|node| parse_id(&node.content_id))
                .and_then(|id| context.content.catalog.nodes.get(&id))
                .is_none_or(|node| !inventory.accepts.contains(&node.item))
        }) {
            return false;
        }
    }
    for record in &candidate.weapons {
        if unit_definition(context, units, record.unit.0, true)
            .is_none_or(|unit| unit.weapon.is_none())
            || record
                .last_target
                .is_some_and(|target| !units.contains_key(&target.0))
        {
            return false;
        }
    }
    for record in &candidate.healths {
        if !valid_health(context, units, record) {
            return false;
        }
    }
    for record in &candidate.research {
        if record.slot.0 != context.slot {
            return false;
        }
        let mut completed = HashSet::new();
        if record.completed.iter().any(|technology| {
            parse_id(technology).is_none_or(|id| {
                !completed.insert(id.clone())
                    || !context.content.catalog.technologies.contains_key(&id)
            })
        }) {
            return false;
        }
    }
    true
}
