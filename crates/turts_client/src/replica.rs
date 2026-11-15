//! Atomic client replica for public world state and the local player's private economy.
//!
//! A batch is validated against resolved package definitions and staged as a complete candidate
//! before it can change the displayed world. While recovering, the replica retains a bounded,
//! sorted chain and freezes the latest coherent candidate.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use bevy::platform::time::Instant;
use turts_mod_api::{ContentId, CostSource, ResourceKind, SupplySide};
use turts_net_protocol::{
    BeltPieceRecord, EconomyChanges, EconomyState, ExtractorRecord, HealthRecord, HeldOrder,
    InserterRecord, InventoryPort, JobId, MatchState, NodeId, NodeState, Outcome, OutcomeReason,
    Pos, RateRecord, ResearchRecord, SlotOutcome, Snapshot, UnitId, UnitOrderRecord, UnitSnapshot,
    WeaponRecord, WorldDelta,
};
use turts_rules::charge::{CancelCase, CancelRefunds};

use crate::replica_store::{RecordStore, StoreStep, StoreUpdate};
use crate::{content::ClientContent, interp, replica_store};

/// One unit on the client.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitReplica {
    pub content_id: String,
    pub owner_slot: u8,
    pub facing: Option<turts_net_protocol::Facing>,
    /// Ascending, consecutive ticks.
    pub samples: VecDeque<(u32, Pos)>,
}

#[derive(Clone, Copy)]
pub(crate) struct UnitFacts<'a> {
    pub content_id: &'a str,
    pub owner_slot: u8,
    pub position: Option<Pos>,
}

pub(crate) trait UnitTable {
    fn get(&self, id: &u32) -> Option<UnitFacts<'_>>;
    fn rows(&self) -> impl Iterator<Item = (u32, UnitFacts<'_>)>;
    fn contains_key(&self, id: &u32) -> bool {
        self.get(id).is_some()
    }
}
impl UnitTable for HashMap<u32, UnitReplica> {
    fn get(&self, id: &u32) -> Option<UnitFacts<'_>> {
        HashMap::get(self, id).map(|unit| UnitFacts {
            content_id: &unit.content_id,
            owner_slot: unit.owner_slot,
            position: unit.samples.back().map(|(_, pos)| *pos),
        })
    }
    fn rows(&self) -> impl Iterator<Item = (u32, UnitFacts<'_>)> {
        self.iter().map(|(id, unit)| {
            (
                *id,
                UnitFacts {
                    content_id: &unit.content_id,
                    owner_slot: unit.owner_slot,
                    position: unit.samples.back().map(|(_, pos)| *pos),
                },
            )
        })
    }
}

#[derive(Clone, Copy)]
struct UnitOverlay<'a> {
    base: &'a HashMap<u32, UnitReplica>,
    spawned: &'a HashMap<u32, &'a UnitSnapshot>,
    positions: &'a HashMap<u32, Pos>,
    removed: &'a HashSet<u32>,
    include_removed: bool,
}
impl UnitTable for UnitOverlay<'_> {
    fn get(&self, id: &u32) -> Option<UnitFacts<'_>> {
        if !self.include_removed && self.removed.contains(id) {
            return None;
        }
        let mut unit = if let Some(unit) = self.spawned.get(id) {
            UnitFacts {
                content_id: &unit.content_id,
                owner_slot: unit.owner_slot.0,
                position: Some(unit.pos),
            }
        } else {
            UnitTable::get(self.base, id)?
        };
        if let Some(position) = self.positions.get(id) {
            unit.position = Some(*position);
        }
        Some(unit)
    }
    fn rows(&self) -> impl Iterator<Item = (u32, UnitFacts<'_>)> {
        self.base
            .keys()
            .chain(self.spawned.keys())
            .filter_map(move |id| self.get(id).map(|unit| (*id, unit)))
    }
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

/// How many health changes the replica keeps for each unit (design D 9, row DBG1).
/// Atomic staging keeps append steps and never copies the retained history.
pub const HEALTH_TRACE_WINDOW: usize = 64;

/// Why the replica asked for a `Snapshot`. Each reason has its own counter in the debug state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResyncReason {
    /// A delta did not extend the chain and the predecessor did not arrive within the grace
    /// (or the delta was malformed, or there was no baseline).
    ChainBreak,
    /// A delta that extends the chain failed validation (`stage_delta`).
    ValidationFailure,
    /// The replica waited for a `Snapshot`, no request was out, and the buffered span reached
    /// `IDLE_RESYNC_SPAN_TICKS` (`ingest.rs`).
    IdleSpan,
    /// A `Snapshot` answer could not be published (stale, invalid, or its tail is not coherent).
    Successor,
}

/// The longest virtual-time step of one server frame: `Time<Virtual>` clamps a frame to this
/// (`bevy_time-0.20.0/src/virt.rs:85`, `DEFAULT_MAX_DELTA`). Pinned for the running server app by
/// `crates/turts_server/src/main.rs` (test `server_frame_clamp_and_timestep_are_pinned`).
pub const MAX_FRAME_DELTA: std::time::Duration = std::time::Duration::from_millis(250);

/// The most fixed ticks that one server frame can run, so the most deltas that one flush can hold.
///
/// Bevy adds the clamped frame delta to the overstep and runs one tick for each whole timestep
/// (`bevy_time-0.20.0/src/fixed.rs:216-226, 243-255`). The overstep left by the frame before is
/// below one timestep `T`, so a frame runs at most `1 + floor((MAX_FRAME_DELTA - 1 ns) / T)` ticks
/// (5 at 20 Hz). `T` is `tick_duration(hz)` (`turts_net_lightyear`), rounded down to whole
/// nanoseconds, which is why `hz / 4` is one too small at some rates (12, 48, 52, 68 Hz ...).
/// The test `catchup_bound.rs` computes the bound with a real Bevy app.
pub fn max_catchup_ticks(tick_hz: u16) -> u32 {
    let tick_ns = turts_net_lightyear::tick_duration(tick_hz.max(1))
        .as_nanos()
        .max(1);
    let most = 1 + (MAX_FRAME_DELTA.as_nanos() - 1) / tick_ns;
    u32::try_from(most).unwrap_or(u32::MAX)
}

/// Technical cap on retained batches while waiting for a reliable snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplicaConfig {
    pub max_buffered_batches: usize,
    /// Reorder grace, in ticks of observed progress. A delta that is ahead of the chain is held;
    /// a resync is asked for only when `highest_observed_tick - last_applied_tick` is above this
    /// value. 0 means no hold: the first chain break asks at once (the M1 rule).
    pub reorder_grace_ticks: u32,
}

impl Default for ReplicaConfig {
    fn default() -> Self {
        Self {
            max_buffered_batches: 256,
            reorder_grace_ticks: 0,
        }
    }
}

/// The validated catalog and receiving slot used to check private replicated records.
#[derive(Debug, Clone)]
pub struct ReplicaContext {
    pub content: ClientContent,
    pub slot: u8,
    unit_ids: HashMap<String, ContentId>,
}

impl ReplicaContext {
    pub fn new(content: ClientContent, slot: u8) -> Self {
        let unit_ids = content
            .catalog
            .units
            .keys()
            .filter_map(|id| {
                let name = id.to_string();
                (ContentId::parse(&name).ok().as_ref() == Some(id)).then(|| (name, id.clone()))
            })
            .collect();
        Self {
            content,
            slot,
            unit_ids,
        }
    }
    pub(crate) fn unit_definition(&self, text: &str) -> Option<&turts_mod_api::ResolvedUnit> {
        if let Some(id) = self.unit_ids.get(text) {
            return self.content.catalog.units.get(id);
        }
        self.content
            .catalog
            .units
            .get(&ContentId::parse(text).ok()?)
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
    unit_orders: HashMap<u32, UnitOrderRecord>,
    health_append: Vec<(u32, HealthTraceEntry)>,
    health_latest: HashMap<u32, u64>,
    health_forget: HashSet<u32>,
    tick: u32,
}

struct AtomicPatch {
    spawned: Vec<(u32, UnitReplica)>,
    positions: Vec<(u32, Pos)>,
    removed: HashSet<u32>,
    nodes: Option<HashMap<NodeId, NodeState>>,
    economy: Option<EconomyState>,
    belts: Option<Vec<BeltPieceRecord>>,
    inserters: Option<Vec<InserterRecord>>,
    extractors: Option<Vec<ExtractorRecord>>,
    weapons: Option<Vec<WeaponRecord>>,
    healths: Option<Vec<HealthRecord>>,
    research: Option<Vec<ResearchRecord>>,
    orders_changed: HashMap<u32, UnitOrderRecord>,
    orders_cleared: HashSet<u32>,
    health_append: Vec<(u32, HealthTraceEntry)>,
    health_forget: Vec<u32>,
    store: StoreStep,
    terrain: Option<crate::terrain::store::Patch>,
}

/// A public health value observed in a coherent server tick. Equal values are omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthTraceEntry {
    pub tick: u32,
    pub current: u64,
}

/// A presentation event. Sequence numbers let independent visual systems consume it once.
#[derive(Debug, Clone)]
pub struct VisualEvent {
    pub sequence: u64,
    pub tick: u32,
    pub effect: turts_net_protocol::Effect,
}

/// A replica keeps coherent samples and replaces on-wire absolute records atomically.
#[derive(Debug, Clone)]
pub struct Replica {
    /// Complete permitted knowledge, committed with the gameplay tables. None is AllVisible.
    pub knowledge: Option<std::sync::Arc<turts_mod_api::KnowledgeSnapshot>>,
    /// Event-local permitted identities. These do not become live picking targets.
    pub event_units: Vec<turts_net_protocol::visibility::EventUnitObservation>,
    pub terrain: Option<crate::terrain::store::PermittedTerrain>,
    /// Bounded visual events from committed live deltas only. Never replayed by snapshots.
    pub visual_events: VecDeque<VisualEvent>,
    next_visual_event: u64,
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
    /// Owner-specific orders staged atomically with their unit snapshot and tick.
    pub unit_orders: HashMap<u32, UnitOrderRecord>,
    /// The side store of the 6.8 records (`replica_store`): it commits with the candidate and is
    /// read through the getters below. It is not a field of the candidate.
    store: RecordStore,
    /// `Welcome.start.start_tick` (design 6.7.13): the tick that `MatchClock` counts from. Set by
    /// `on_welcome` through `set_match_start`; 0 until then.
    start_tick: u32,
    /// The recent health changes of each unit: at most [`HEALTH_TRACE_WINDOW`] entries, the
    /// newest last (row DBG1). The alert `OwnUnitDestroyed` reads the last entry of a unit that
    /// is gone, so the window always keeps the newest entry.
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
    /// Deltas that are ahead of the chain, sorted by tick, no duplicate tick. Their predecessor
    /// may still arrive. At most `max_buffered_batches` and, by the grace rule, at most
    /// `reorder_grace_ticks` of them (see `apply_delta`).
    reorder: Vec<WorldDelta>,
    /// Deltas that were held and then applied when their predecessor arrived.
    pub reorder_recovered: u32,
    /// Deltas that were held (a delta counts once).
    pub reorder_held: u32,
    /// The reason of the resync that the last `apply_delta` or `apply_snapshot` started.
    pub last_resync_reason: Option<ResyncReason>,
    context: Option<ReplicaContext>,
    config: ReplicaConfig,
    gap_anchor: Option<u32>,
    gap_seen: bool,
    /// The recovery tail lost data (an overflow, or a hole older than the grace): a snapshot must
    /// then cover the high-water tick, and the `ahead` hold is refused.
    tail_lost: bool,
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
            knowledge: None,
            event_units: Vec::new(),
            terrain: None,
            visual_events: VecDeque::new(),
            next_visual_event: 0,
            units: HashMap::new(),
            nodes: HashMap::new(),
            economy: empty_economy(),
            belts: Vec::new(),
            inserters: Vec::new(),
            extractors: Vec::new(),
            weapons: Vec::new(),
            healths: Vec::new(),
            research: Vec::new(),
            unit_orders: HashMap::new(),
            store: RecordStore::default(),
            start_tick: 0,
            health_trace: std::collections::BTreeMap::new(),
            snapshot_tick: None,
            last_applied_tick: None,
            latest_receipt: None,
            resync: Some(Vec::new()),
            highest_observed_tick: None,
            buffer_overflows: 0,
            reorder: Vec::new(),
            reorder_recovered: 0,
            reorder_held: 0,
            last_resync_reason: None,
            context: None,
            config: ReplicaConfig {
                max_buffered_batches: config.max_buffered_batches.max(1),
                reorder_grace_ticks: config.reorder_grace_ticks,
            },
            gap_anchor: None,
            gap_seen: false,
            tail_lost: false,
        }
    }

    /// Validate all transaction tables before changing any active presentation state.
    /// The caller acknowledges the stamp only after this returns true.
    pub fn activate_transaction(
        &mut self,
        stamp: &turts_net_protocol::visibility::StateTransactionStamp,
        payload: &turts_net_protocol::visibility::StateTransactionPayload,
        now: Instant,
    ) -> bool {
        self.activate_transaction_reserved(stamp, payload, now, 0)
    }

    pub(crate) fn activate_transaction_reserved(
        &mut self,
        stamp: &turts_net_protocol::visibility::StateTransactionStamp,
        payload: &turts_net_protocol::visibility::StateTransactionPayload,
        now: Instant,
        reserved_bytes: usize,
    ) -> bool {
        use turts_net_protocol::visibility::StateTransactionPayload;
        let Some(context) = self.context.as_ref() else {
            return false;
        };
        if self.preflight_transaction(payload, reserved_bytes).is_err() {
            return false;
        }
        if stamp.basis.phase != turts_mod_api::ObservationPhase::Completed
            || self
                .last_applied_tick
                .is_some_and(|tick| stamp.basis.tick < tick)
        {
            return false;
        }
        if let StateTransactionPayload::Patch {
            state,
            knowledge,
            event_units,
        } = payload
        {
            return self.activate_atomic_patch(stamp, state, knowledge.as_ref(), event_units, now);
        }
        let (mut candidate, store, knowledge, effects, event_units) = match payload {
            StateTransactionPayload::Snapshot {
                state,
                knowledge,
                event_units,
            } => {
                if state.tick != stamp.basis.tick
                    || stamp.parent_transaction_id.is_some()
                    || knowledge.as_ref().is_some_and(|k| k.basis != stamp.basis)
                {
                    return false;
                }
                let Some((candidate, store)) = self.stage_snapshot(state) else {
                    return false;
                };
                (
                    candidate,
                    StoreUpdate::Replace(store),
                    knowledge.clone(),
                    &[][..],
                    event_units,
                )
            }
            StateTransactionPayload::Patch {
                state,
                knowledge,
                event_units,
            } => {
                if state.tick != stamp.basis.tick
                    || self.last_applied_tick != Some(state.base_tick)
                    || state.tick <= state.base_tick
                    || stamp.parent_transaction_id.is_none()
                {
                    return false;
                }
                let next_knowledge = match (&self.knowledge, knowledge) {
                    (None, None) => None,
                    (Some(current), Some(delta)) if delta.basis == stamp.basis => {
                        let Some(next) = crate::visibility::stage_delta(current, delta) else {
                            return false;
                        };
                        Some(next)
                    }
                    _ => return false,
                };
                let mut delta = state.as_ref().clone();
                let mut current = self.candidate_at(state.base_tick);
                if let Some(knowledge) = knowledge {
                    if knowledge.hidden_units.iter().any(|id| {
                        self.units
                            .get(id)
                            .is_some_and(|unit| unit.owner_slot == context.slot)
                    }) || knowledge
                        .hidden_units
                        .iter()
                        .any(|id| delta.removed.contains(&UnitId(*id)))
                        || knowledge
                            .revealed_units
                            .iter()
                            .any(|id| !delta.spawned.iter().any(|row| row.unit_id.0 == *id))
                        || knowledge
                            .revealed_nodes
                            .iter()
                            .any(|id| !delta.nodes.iter().any(|row| row.id.0 == *id))
                    {
                        return false;
                    }
                    delta
                        .removed
                        .extend(knowledge.hidden_units.iter().copied().map(UnitId));
                    for id in &knowledge.hidden_nodes {
                        if current.nodes.remove(&NodeId(*id)).is_none() {
                            return false;
                        }
                    }
                }
                let Some((candidate, step)) = self.stage_delta_owned(current, &self.store, &delta)
                else {
                    return false;
                };
                (
                    candidate,
                    StoreUpdate::Step(step),
                    next_knowledge,
                    state.records.effects.as_slice(),
                    event_units,
                )
            }
        };
        if let Some(knowledge) = &knowledge
            && (knowledge.basis != stamp.basis
                || knowledge.viewer != context.slot
                || !crate::visibility::valid_snapshot(knowledge)
                || !valid_known_entities(context, &candidate, knowledge))
        {
            return false;
        }
        let mut transient = HashSet::new();
        if event_units.iter().any(|row| {
            row.basis.match_id != stamp.basis.match_id
                || row.basis.map != stamp.basis.map
                || if knowledge.is_some() {
                    row.basis.tick != stamp.basis.tick
                } else {
                    row.basis.tick > stamp.basis.tick
                        || row.basis.phase != turts_mod_api::ObservationPhase::Completed
                }
                || !valid_unit(context, &row.unit)
                || !transient.insert(row.unit.unit_id.0)
        }) {
            return false;
        }
        let endpoint =
            |id: UnitId| candidate.units.contains_key(&id.0) || transient.contains(&id.0);
        if effects.iter().any(|effect| match effect {
            turts_net_protocol::Effect::Fired { unit, target, .. } => {
                !endpoint(*unit) || !endpoint(*target)
            }
            turts_net_protocol::Effect::Destroyed { unit }
            | turts_net_protocol::Effect::Credited { unit, .. } => !endpoint(*unit),
        }) {
            return false;
        }
        let terrain = match (&context.content.terrain, payload) {
            (None, StateTransactionPayload::Snapshot { state, .. })
                if state.records.terrain.is_none() =>
            {
                None
            }
            (
                Some(layout),
                StateTransactionPayload::Snapshot {
                    state,
                    knowledge,
                    event_units,
                },
            ) => {
                let Some(rows) = state.records.terrain.as_ref() else {
                    return false;
                };
                let dictionary = context
                    .content
                    .catalog
                    .terrain_materials
                    .keys()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                let authority = crate::terrain::authority::View {
                    context,
                    units: &candidate.units,
                    basis: &stamp.basis,
                    knowledge: knowledge.as_ref(),
                    event_units,
                    layout,
                    budget: std::cell::RefCell::new(turts_rules::terrain::geometry::Budget::new(
                        4_194_304,
                    )),
                };
                let Ok(terrain) = crate::terrain::store::PermittedTerrain::stage_snapshot(
                    layout.clone(),
                    &stamp.basis,
                    &dictionary,
                    rows,
                    &authority,
                    self.terrain.as_ref(),
                ) else {
                    return false;
                };
                Some(terrain)
            }
            _ => return false,
        };
        // Hide clears health history. An observed death keeps its killing hit for alerts.
        match payload {
            StateTransactionPayload::Patch {
                knowledge: Some(delta),
                ..
            } => {
                let hidden: HashSet<_> = delta.hidden_units.iter().copied().collect();
                candidate.health_forget.extend(hidden);
            }
            StateTransactionPayload::Snapshot {
                knowledge: Some(_), ..
            } => {
                candidate
                    .health_forget
                    .extend(self.health_trace.keys().copied().filter(|id| {
                        !candidate.units.contains_key(id)
                            && self
                                .units
                                .get(id)
                                .is_some_and(|unit| unit.owner_slot != context.slot)
                    }));
            }
            _ => {}
        }
        self.commit(candidate, store, now);
        if matches!(payload, StateTransactionPayload::Snapshot { .. }) {
            self.snapshot_tick = Some(stamp.basis.tick);
        }
        self.knowledge = knowledge.map(std::sync::Arc::new);
        self.event_units = event_units.clone();
        self.terrain = terrain;
        self.visual_events.clear();
        for effect in effects {
            self.next_visual_event += 1;
            self.visual_events.push_back(VisualEvent {
                sequence: self.next_visual_event,
                tick: stamp.basis.tick,
                effect: effect.clone(),
            });
        }
        self.resync = None;
        self.reorder.clear();
        self.highest_observed_tick = Some(stamp.basis.tick);
        self.gap_anchor = None;
        self.gap_seen = false;
        self.tail_lost = false;
        true
    }

    fn stage_atomic_patch(
        &self,
        stamp: &turts_net_protocol::visibility::StateTransactionStamp,
        delta: &WorldDelta,
        knowledge_delta: Option<&turts_mod_api::KnowledgeDelta>,
        event_units: &[turts_net_protocol::visibility::EventUnitObservation],
        knowledge: Option<&turts_mod_api::KnowledgeSnapshot>,
    ) -> Option<AtomicPatch> {
        let context = self.context.as_ref()?;
        if delta.tick != stamp.basis.tick
            || self.last_applied_tick != Some(delta.base_tick)
            || delta.tick <= delta.base_tick
            || stamp.parent_transaction_id.is_none()
        {
            return None;
        }
        let mut removed: HashSet<_> = delta.removed.iter().map(|id| id.0).collect();
        if removed.len() != delta.removed.len() {
            return None;
        }
        if let Some(visibility) = knowledge_delta {
            for id in &visibility.hidden_units {
                let unit = self.units.get(id)?;
                if unit.owner_slot == context.slot || !removed.insert(*id) {
                    return None;
                }
            }
            if visibility
                .revealed_units
                .iter()
                .any(|id| !delta.spawned.iter().any(|row| row.unit_id.0 == *id))
                || visibility
                    .revealed_nodes
                    .iter()
                    .any(|id| !delta.nodes.iter().any(|row| row.id.0 == *id))
            {
                return None;
            }
        }
        if removed.iter().any(|id| !self.units.contains_key(id)) {
            return None;
        }
        let mut spawned = HashMap::with_capacity(delta.spawned.len());
        for unit in &delta.spawned {
            if !valid_unit(context, unit)
                || self.units.contains_key(&unit.unit_id.0)
                || removed.contains(&unit.unit_id.0)
                || spawned.insert(unit.unit_id.0, unit).is_some()
            {
                return None;
            }
        }
        let mut positions = HashMap::with_capacity(delta.positions.len());
        for row in &delta.positions {
            if !finite_pos(row.pos)
                || positions.insert(row.unit_id.0, row.pos).is_some()
                || (!self.units.contains_key(&row.unit_id.0)
                    && !spawned.contains_key(&row.unit_id.0))
            {
                return None;
            }
        }
        let during = UnitOverlay {
            base: &self.units,
            spawned: &spawned,
            positions: &positions,
            removed: &removed,
            include_removed: true,
        };
        let final_units = UnitOverlay {
            include_removed: false,
            ..during
        };
        let mut nodes = None;
        if !delta.nodes.is_empty()
            || knowledge_delta.is_some_and(|visibility| !visibility.hidden_nodes.is_empty())
        {
            let mut next = self.nodes.clone();
            if let Some(visibility) = knowledge_delta {
                for id in &visibility.hidden_nodes {
                    next.remove(&NodeId(*id))?;
                }
            }
            let mut seen = HashSet::new();
            for node in &delta.nodes {
                if !valid_node(context, node) || !seen.insert(node.id) {
                    return None;
                }
                next.insert(node.id, node.clone());
            }
            nodes = Some(next);
        }
        let final_nodes = nodes.as_ref().unwrap_or(&self.nodes);
        if !validate_delta_m3(context, &during, final_nodes, delta, &removed) {
            return None;
        }
        let economy_changed = !delta.economy.balances.is_empty()
            || !delta.economy.inventories.is_empty()
            || !delta.economy.orders.is_empty()
            || !delta.economy.queues.is_empty()
            || removed.iter().any(|id| {
                self.units
                    .get(id)
                    .is_some_and(|unit| unit.owner_slot == context.slot)
            });
        let mut economy = None;
        if economy_changed {
            let mut next = self.economy.clone();
            if !merge_economy(&mut next, &delta.economy) {
                return None;
            }
            next.inventories
                .retain(|row| !removed.contains(&row.unit.0));
            next.orders.retain(|row| {
                !removed.contains(&row.unit.0)
                    && row.receiver.is_none_or(|id| !removed.contains(&id.0))
            });
            next.queues.retain(|row| !removed.contains(&row.producer.0));
            economy = Some(next);
        }
        let economy_view = economy.as_ref().unwrap_or(&self.economy);
        let belts = stage_rows(&self.belts, &delta.belts, &removed, |row| row.piece.0)?;
        let inserters = stage_rows(&self.inserters, &delta.inserters, &removed, |row| {
            row.unit.0
        })?;
        let extractors = stage_rows(&self.extractors, &delta.extractors, &removed, |row| {
            row.unit.0
        })?;
        let weapons = stage_rows(&self.weapons, &delta.weapons, &removed, |row| row.unit.0)?;
        let healths = stage_rows(&self.healths, &delta.healths, &removed, |row| row.unit.0)?;
        let research = stage_rows(&self.research, &delta.research, &HashSet::new(), |row| {
            u32::from(row.slot.0)
        })?;
        let records = M3Records {
            belts: staged_slice(&belts, &self.belts),
            inserters: staged_slice(&inserters, &self.inserters),
            extractors: staged_slice(&extractors, &self.extractors),
            weapons: staged_slice(&weapons, &self.weapons),
            healths: staged_slice(&healths, &self.healths),
            research: staged_slice(&research, &self.research),
        };
        if !valid_economy(context, &final_units, final_nodes, economy_view)
            || !valid_m3_records(context, &final_units, final_nodes, records)
        {
            return None;
        }
        let mut orders_changed = stage_unit_orders(context, &delta.unit_orders, &during)?;
        let mut orders_cleared = HashSet::new();
        for id in &delta.records.orders_cleared {
            if during
                .get(&id.0)
                .is_none_or(|unit| unit.owner_slot != context.slot)
                || removed.contains(&id.0)
                || orders_changed.contains_key(&id.0)
                || !orders_cleared.insert(id.0)
            {
                return None;
            }
        }
        orders_changed.retain(|id, _| !removed.contains(id));
        let store = replica_store::stage_delta_step(
            context,
            &self.store,
            &during,
            economy_view,
            delta,
            &removed,
        )?;
        if let Some(knowledge) = knowledge
            && (knowledge.basis != stamp.basis
                || knowledge.viewer != context.slot
                || !crate::visibility::valid_snapshot(knowledge)
                || !valid_known_view(context, &final_units, final_nodes, knowledge))
        {
            return None;
        }
        let mut transient = HashSet::new();
        for row in event_units {
            if row.basis.match_id != stamp.basis.match_id
                || row.basis.map != stamp.basis.map
                || (if knowledge.is_some() {
                    row.basis.tick != stamp.basis.tick
                } else {
                    row.basis.tick > stamp.basis.tick
                        || row.basis.phase != turts_mod_api::ObservationPhase::Completed
                })
                || !valid_unit(context, &row.unit)
                || !transient.insert(row.unit.unit_id.0)
            {
                return None;
            }
        }
        let endpoint = |id: UnitId| final_units.contains_key(&id.0) || transient.contains(&id.0);
        if delta.records.effects.iter().any(|effect| match effect {
            turts_net_protocol::Effect::Fired { unit, target, .. } => {
                !endpoint(*unit) || !endpoint(*target)
            }
            turts_net_protocol::Effect::Destroyed { unit }
            | turts_net_protocol::Effect::Credited { unit, .. } => !endpoint(*unit),
        }) {
            return None;
        }
        let positions = final_units
            .rows()
            .map(|(id, unit)| Some((id, unit.position?)))
            .collect::<Option<Vec<_>>>()?;
        let spawned = delta
            .spawned
            .iter()
            .map(|unit| {
                (
                    unit.unit_id.0,
                    UnitReplica {
                        content_id: unit.content_id.clone(),
                        owner_slot: unit.owner_slot.0,
                        facing: unit.facing,
                        samples: VecDeque::from([(delta.tick, unit.pos)]),
                    },
                )
            })
            .collect();
        let health_append = delta
            .healths
            .iter()
            .filter(|row| {
                self.health_trace
                    .get(&row.unit.0)
                    .and_then(|history| history.last())
                    .is_none_or(|last| last.current != row.current)
            })
            .map(|row| {
                (
                    row.unit.0,
                    HealthTraceEntry {
                        tick: delta.tick,
                        current: row.current,
                    },
                )
            })
            .collect();
        let health_forget =
            knowledge_delta.map_or_else(Vec::new, |visibility| visibility.hidden_units.clone());
        let terrain = match (
            &context.content.terrain,
            &self.terrain,
            &delta.records.terrain,
        ) {
            (None, None, None) => None,
            (Some(layout), Some(current), rows) => {
                let empty = turts_net_protocol::TerrainDelta::default();
                let authority = crate::terrain::authority::View {
                    context,
                    units: &final_units,
                    basis: &stamp.basis,
                    knowledge,
                    event_units,
                    layout,
                    budget: std::cell::RefCell::new(turts_rules::terrain::geometry::Budget::new(
                        4_194_304,
                    )),
                };
                Some(
                    current
                        .stage_delta(&stamp.basis, rows.as_ref().unwrap_or(&empty), &authority)
                        .ok()?,
                )
            }
            _ => return None,
        };
        Some(AtomicPatch {
            spawned,
            positions,
            removed,
            nodes,
            economy,
            belts,
            inserters,
            extractors,
            weapons,
            healths,
            research,
            orders_changed,
            orders_cleared,
            health_append,
            health_forget,
            store,
            terrain,
        })
    }

    fn activate_atomic_patch(
        &mut self,
        stamp: &turts_net_protocol::visibility::StateTransactionStamp,
        delta: &WorldDelta,
        knowledge_delta: Option<&turts_mod_api::KnowledgeDelta>,
        event_units: &[turts_net_protocol::visibility::EventUnitObservation],
        now: Instant,
    ) -> bool {
        let knowledge = match (&self.knowledge, knowledge_delta) {
            (None, None) => None,
            (Some(current), Some(delta)) if delta.basis == stamp.basis => {
                let Some(next) = crate::visibility::stage_delta(current, delta) else {
                    return false;
                };
                Some(next)
            }
            _ => return false,
        };
        let Some(patch) = self.stage_atomic_patch(
            stamp,
            delta,
            knowledge_delta,
            event_units,
            knowledge.as_ref(),
        ) else {
            return false;
        };
        // Validation is complete. No fallible record operation follows this boundary.
        if !patch.removed.is_empty() {
            self.units.retain(|id, _| !patch.removed.contains(id));
        }
        self.units.extend(patch.spawned);
        for (id, position) in patch.positions {
            if let Some(unit) = self.units.get_mut(&id) {
                if unit
                    .samples
                    .back()
                    .is_some_and(|(tick, _)| *tick == delta.tick)
                {
                    if let Some((_, point)) = unit.samples.back_mut() {
                        *point = position;
                    }
                } else {
                    unit.samples.push_back((delta.tick, position));
                }
                while unit
                    .samples
                    .front()
                    .is_some_and(|(tick, _)| *tick < delta.tick.saturating_sub(PRUNE_TICKS))
                {
                    unit.samples.pop_front();
                }
            }
        }
        if let Some(value) = patch.nodes {
            self.nodes = value;
        }
        if let Some(value) = patch.economy {
            self.economy = value;
        }
        if let Some(value) = patch.belts {
            self.belts = value;
        }
        if let Some(value) = patch.inserters {
            self.inserters = value;
        }
        if let Some(value) = patch.extractors {
            self.extractors = value;
        }
        if let Some(value) = patch.weapons {
            self.weapons = value;
        }
        if let Some(value) = patch.healths {
            self.healths = value;
        }
        if let Some(value) = patch.research {
            self.research = value;
        }
        if !patch.removed.is_empty() || !patch.orders_cleared.is_empty() {
            self.unit_orders
                .retain(|id, _| !patch.removed.contains(id) && !patch.orders_cleared.contains(id));
        }
        self.unit_orders.extend(patch.orders_changed);
        self.store.apply(patch.store);
        if let Some(terrain) = patch.terrain {
            self.terrain
                .as_mut()
                .expect("validated terrain basis")
                .commit(terrain);
        }
        for (id, entry) in patch.health_append {
            let history = self.health_trace.entry(id).or_default();
            history.push(entry);
            if history.len() > HEALTH_TRACE_WINDOW {
                history.drain(..history.len() - HEALTH_TRACE_WINDOW);
            }
        }
        for id in patch.health_forget {
            self.health_trace.remove(&id);
        }
        self.knowledge = knowledge.map(std::sync::Arc::new);
        self.event_units = event_units.to_vec();
        self.visual_events.clear();
        for effect in &delta.records.effects {
            self.next_visual_event += 1;
            self.visual_events.push_back(VisualEvent {
                sequence: self.next_visual_event,
                tick: delta.tick,
                effect: effect.clone(),
            });
        }
        self.last_applied_tick = Some(delta.tick);
        self.latest_receipt = Some(now);
        self.resync = None;
        self.reorder.clear();
        self.highest_observed_tick = Some(delta.tick);
        self.gap_anchor = None;
        self.gap_seen = false;
        self.tail_lost = false;
        true
    }

    /// Refuse before allocating a candidate, knowledge copy, or validation set.
    /// The estimate includes the decoded payload, candidate storage, and temporary expansions.
    fn preflight_transaction(
        &self,
        payload: &turts_net_protocol::visibility::StateTransactionPayload,
        reserved_bytes: usize,
    ) -> Result<(), crate::staging_budget::Limit> {
        use crate::staging_budget::{Budget, Limit};
        use turts_net_protocol::visibility::{MAX_PENDING_STATE_BYTES, StateTransactionPayload};
        let mut budget = Budget::new(
            (MAX_PENDING_STATE_BYTES as usize)
                .checked_sub(reserved_bytes)
                .ok_or(Limit)?,
        );
        // Fixed candidates, store steps, stamps, and small hash tables.
        budget.bytes(64 * 1024)?;
        budget.value(payload)?;
        if let Some(terrain) = &self.terrain {
            budget.value(&terrain.surfaces)?;
            budget.value(&terrain.anchors)?;
            budget.value(&terrain.work)?;
            budget.value(terrain.materials.as_ref())?;
            budget.bytes(
                terrain
                    .geometry
                    .retained_bytes()
                    .ok_or(Limit)?
                    .checked_mul(4)
                    .ok_or(Limit)?,
            )?;
            let cells = (terrain.layout.grid.columns as usize)
                .checked_mul(terrain.layout.grid.rows as usize)
                .ok_or(Limit)?;
            budget.bytes(cells.checked_mul(std::mem::size_of::<i32>()).ok_or(Limit)?)?;
        }
        // Health histories stay borrowed during staging. New append/forget rows are covered by the payload budget.
        if let StateTransactionPayload::Patch {
            state, knowledge, ..
        } = payload
        {
            let mut removed: HashSet<_> = state.removed.iter().map(|id| id.0).collect();
            if let Some(knowledge) = knowledge {
                removed.extend(knowledge.hidden_units.iter().copied());
            }
            let context = self.context.as_ref().ok_or(Limit)?;
            // The staged final-position vector and validator indexes contain IDs, not unit strings or histories.
            let rows = self
                .units
                .len()
                .saturating_add(self.healths.len())
                .saturating_add(self.weapons.len())
                .saturating_add(self.economy.inventories.len())
                .saturating_add(self.economy.orders.len())
                .saturating_add(self.economy.queues.len());
            budget.bytes(rows.checked_mul(64).ok_or(Limit)?)?;
            for (id, unit) in &self.units {
                if !removed.contains(id) && unit.samples.len() == unit.samples.capacity() {
                    let growth = unit
                        .samples
                        .capacity()
                        .max(2)
                        .checked_mul(2)
                        .and_then(|n| n.checked_mul(std::mem::size_of::<(u32, Pos)>() * 2))
                        .and_then(|n| n.checked_add(128))
                        .ok_or(Limit)?;
                    budget.bytes(growth)?;
                }
            }
            if !state.spawned.is_empty() {
                budget.bytes(
                    self.units
                        .capacity()
                        .checked_mul(std::mem::size_of::<(u32, UnitReplica)>() * 4 + 64)
                        .ok_or(Limit)?,
                )?;
            }
            if !state.nodes.is_empty()
                || knowledge
                    .as_ref()
                    .is_some_and(|delta| !delta.hidden_nodes.is_empty())
            {
                budget.value(&self.nodes)?;
            }
            if !state.unit_orders.is_empty() {
                budget.bytes(
                    self.unit_orders
                        .capacity()
                        .checked_mul(std::mem::size_of::<(u32, UnitOrderRecord)>() * 4 + 64)
                        .ok_or(Limit)?,
                )?;
            }
            if !state.economy.balances.is_empty()
                || !state.economy.inventories.is_empty()
                || !state.economy.orders.is_empty()
                || !state.economy.queues.is_empty()
                || removed.iter().any(|id| {
                    self.units
                        .get(id)
                        .is_some_and(|unit| unit.owner_slot == context.slot)
                })
            {
                budget.value(&self.economy)?;
            }
            if rows_changed(&self.belts, &state.belts, &removed, |row| row.piece.0) {
                budget.value(&self.belts)?;
            }
            if rows_changed(&self.inserters, &state.inserters, &removed, |row| {
                row.unit.0
            }) {
                budget.value(&self.inserters)?;
            }
            if rows_changed(&self.extractors, &state.extractors, &removed, |row| {
                row.unit.0
            }) {
                budget.value(&self.extractors)?;
            }
            if rows_changed(&self.weapons, &state.weapons, &removed, |row| row.unit.0) {
                budget.value(&self.weapons)?;
            }
            if rows_changed(&self.healths, &state.healths, &removed, |row| row.unit.0) {
                budget.value(&self.healths)?;
            }
            if !state.research.is_empty() {
                budget.value(&self.research)?;
            }
            if let Some(knowledge) = self.knowledge.as_deref() {
                budget.value(knowledge)?;
            }
        }
        Ok(())
    }

    /// Take the start tick of the match from `Welcome.start.start_tick` (`ingest::on_welcome`).
    pub fn set_match_start(&mut self, start_tick: u32) {
        self.start_tick = start_tick;
    }

    pub fn set_context(&mut self, context: ReplicaContext) {
        self.terrain = None;
        self.context = Some(context);
    }

    pub fn reorder_grace_ticks(&self) -> u32 {
        self.config.reorder_grace_ticks
    }

    /// Set the reorder grace (ticks of observed progress). Live ingest sets
    /// `max_catchup_ticks(tick_hz)` at `Welcome`.
    pub fn set_reorder_grace_ticks(&mut self, ticks: u32) {
        self.config.reorder_grace_ticks = ticks;
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

    /// Retained deltas: the recovery tail plus the held reorder window.
    pub fn buffered_batches(&self) -> usize {
        self.resync.as_ref().map_or(0, Vec::len) + self.reorder.len()
    }

    /// Deltas held for a late predecessor (not in resync).
    pub fn held_deltas(&self) -> usize {
        self.reorder.len()
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
        // Terrain/account/unit facts must enter through their one atomic transaction.
        if snapshot.records.terrain.is_some()
            || self
                .context
                .as_ref()
                .is_some_and(|context| context.content.terrain.is_some())
        {
            self.resync.get_or_insert_with(Vec::new);
            return SnapshotOutcome::Invalid;
        }
        if self
            .last_applied_tick
            .is_some_and(|applied| snapshot.tick <= applied)
        {
            return SnapshotOutcome::IgnoredStale;
        }

        let Some((mut candidate, mut staged_store)) = self.stage_snapshot(snapshot) else {
            self.mark_gap(snapshot.tick);
            self.resync.get_or_insert_with(Vec::new);
            self.last_resync_reason = Some(ResyncReason::Successor);
            return SnapshotOutcome::Invalid;
        };

        let mut retained = self.resync.clone().unwrap_or_default();
        retained.sort_by_key(|d| (d.tick, d.base_tick));
        retained.dedup_by_key(|d| d.tick);
        let original_retained = retained.clone();
        let mut replayed = 0;
        let mut remaining = Vec::new();
        // Deltas that are ahead of the snapshot chain: the hole may close within the grace.
        let mut ahead = Vec::new();
        let mut replay_failed = false;
        for delta in retained {
            if delta.tick <= candidate.tick {
                continue;
            }
            if !self.chain_extends(candidate.tick, &delta) {
                if delta.base_tick > candidate.tick
                    && delta.base_tick.checked_add(1) == Some(delta.tick)
                {
                    ahead.push(delta);
                } else {
                    replay_failed = true;
                    remaining.push(delta);
                }
                continue;
            }
            // The store step applies to the staged store of the Snapshot, never to the live store.
            let Some((next, step)) = self.stage_delta(&candidate, &staged_store, &delta) else {
                self.mark_gap(delta.tick);
                replay_failed = true;
                remaining.push(delta);
                continue;
            };
            candidate = next;
            staged_store.apply(step);
            replayed += 1;
        }
        let high = self.highest_observed_tick.unwrap_or(candidate.tick);
        // The deltas that are ahead may wait for their predecessor when the hole is inside the
        // grace and the tail lost nothing. A hole in the retained tail that was already known as
        // a loss does not block this: the snapshot covers it, and a new hole is judged here.
        let ahead_ok = !ahead.is_empty()
            && self.config.reorder_grace_ticks > 0
            && !self.tail_lost
            && ahead.len() <= self.config.max_buffered_batches
            && high.saturating_sub(candidate.tick) <= self.config.reorder_grace_ticks;
        if !ahead.is_empty() && !ahead_ok {
            // The hole is older than the grace: the snapshot does not cover it.
            replay_failed = true;
            remaining.append(&mut ahead);
        }

        // A recovery response is published only if its entire proposed tail is coherent. In
        // particular, do not expose a valid prefix while a later retained batch is invalid or
        // disconnected. Keep the pre-recovery display and all retained evidence for the next
        // reliable response.
        let uncovered_high_water = self
            .highest_observed_tick
            .is_some_and(|high| candidate.tick < high);
        let unresolved_gap = self.gap_seen && uncovered_high_water && !ahead_ok;
        if replay_failed || !remaining.is_empty() || unresolved_gap {
            self.resync = Some(original_retained);
            self.gap_seen = true;
            self.last_resync_reason = Some(ResyncReason::Successor);
            return SnapshotOutcome::ResyncRequested { replayed: 0 };
        }

        self.commit(candidate, StoreUpdate::Replace(staged_store), now);
        self.visual_events.clear();
        self.snapshot_tick = Some(snapshot.tick);
        self.resync = None;
        // The deltas that are ahead of the snapshot chain wait for their predecessor.
        self.reorder_held = self.reorder_held.saturating_add(ahead.len() as u32);
        self.reorder = ahead;
        self.gap_anchor = None;
        self.gap_seen = false;
        self.tail_lost = false;
        SnapshotOutcome::Applied { replayed }
    }

    /// Apply one delta of the unreliable, unordered `StateChannel`.
    ///
    /// The deltas of one server flush may arrive out of order (lightyear sends the fragments
    /// first and the singles by ascending size; see `tests/spike.rs`, P8). A delta that is ahead
    /// of the chain (`base_tick` above the last applied tick) is therefore held, and applied when
    /// its predecessor arrives. A resync starts only when the observed progress
    /// (`highest_observed_tick - last_applied_tick`) is above `reorder_grace_ticks`, so a truly
    /// lost delta gives exactly one resync, at the grace. A duplicate or old delta is discarded.
    /// Memory: at most `max_buffered_batches` held deltas, and a hold ends at the grace.
    pub fn apply_delta(&mut self, delta: &WorldDelta, now: Instant) -> DeltaOutcome {
        if delta.records.terrain.is_some()
            || self
                .context
                .as_ref()
                .is_some_and(|context| context.content.terrain.is_some())
        {
            self.resync.get_or_insert_with(Vec::new);
            return DeltaOutcome::ResyncRequested;
        }
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
            self.start_resync(delta.clone(), ResyncReason::ChainBreak);
            return DeltaOutcome::ResyncRequested;
        };
        if self.chain_extends(tick, delta) {
            if let Some(outcome) = self.apply_chained(delta, now) {
                return outcome;
            }
            let outcome = self.drain_reorder(now);
            if outcome != DeltaOutcome::Applied {
                return outcome;
            }
            return self.check_grace();
        }
        // Ahead of the chain: hold it while the grace lasts.
        let ahead = delta.base_tick > tick && delta.base_tick.checked_add(1) == Some(delta.tick);
        if ahead && self.config.reorder_grace_ticks > 0 {
            if self.reorder.iter().any(|held| held.tick == delta.tick) {
                return DeltaOutcome::Discarded;
            }
            if self.reorder.len() < self.config.max_buffered_batches {
                self.reorder.push(delta.clone());
                self.reorder.sort_by_key(|held| held.tick);
                self.reorder_held = self.reorder_held.saturating_add(1);
                return self.check_grace_buffered();
            }
        }
        self.start_resync(delta.clone(), ResyncReason::ChainBreak);
        DeltaOutcome::ResyncRequested
    }

    /// Stage and commit a delta that extends the chain. `None` when it applied; a resync outcome
    /// when validation failed.
    fn apply_chained(&mut self, delta: &WorldDelta, now: Instant) -> Option<DeltaOutcome> {
        let tick = self.last_applied_tick?;
        let current = self.candidate_at(tick);
        let Some((candidate, step)) = self.stage_delta(&current, &self.store, delta) else {
            self.start_resync(delta.clone(), ResyncReason::ValidationFailure);
            return Some(DeltaOutcome::ResyncRequested);
        };
        self.commit(candidate, StoreUpdate::Step(step), now);
        for effect in &delta.records.effects {
            self.next_visual_event += 1;
            self.visual_events.push_back(VisualEvent {
                sequence: self.next_visual_event,
                tick: delta.tick,
                effect: effect.clone(),
            });
        }
        while self.visual_events.len() > 4096 {
            self.visual_events.pop_front();
        }
        while self
            .visual_events
            .front()
            .is_some_and(|e| e.tick.saturating_add(4) < delta.tick)
        {
            self.visual_events.pop_front();
        }
        None
    }

    /// Apply the held deltas that now extend the chain. Held deltas that are old are dropped.
    fn drain_reorder(&mut self, now: Instant) -> DeltaOutcome {
        loop {
            let Some(applied) = self.last_applied_tick else {
                return DeltaOutcome::Applied;
            };
            self.reorder.retain(|held| held.tick > applied);
            let Some(index) = self
                .reorder
                .iter()
                .position(|held| self.chain_extends(applied, held))
            else {
                return DeltaOutcome::Applied;
            };
            let held = self.reorder.remove(index);
            if let Some(outcome) = self.apply_chained(&held, now) {
                return outcome;
            }
            self.reorder_recovered = self.reorder_recovered.saturating_add(1);
        }
    }

    /// After a hole closed: if held deltas remain beyond the grace, the next hole is a loss.
    fn check_grace(&mut self) -> DeltaOutcome {
        if self.grace_expired() {
            return self.expire_hold();
        }
        DeltaOutcome::Applied
    }

    fn check_grace_buffered(&mut self) -> DeltaOutcome {
        if self.grace_expired() {
            return self.expire_hold();
        }
        DeltaOutcome::Buffered
    }

    fn grace_expired(&self) -> bool {
        let (Some(high), Some(applied)) = (self.highest_observed_tick, self.last_applied_tick)
        else {
            return false;
        };
        !self.reorder.is_empty() && high.saturating_sub(applied) > self.config.reorder_grace_ticks
    }

    /// The predecessor did not arrive within the grace: the held deltas go to the recovery tail.
    fn expire_hold(&mut self) -> DeltaOutcome {
        let mut held = std::mem::take(&mut self.reorder);
        let first = held.remove(0);
        self.start_resync(first, ResyncReason::ChainBreak);
        for delta in held {
            self.buffer_delta(delta);
        }
        DeltaOutcome::ResyncRequested
    }

    /// Enter resync. `delta` and the held deltas go to the bounded recovery tail.
    fn start_resync(&mut self, delta: WorldDelta, reason: ResyncReason) {
        self.mark_gap(delta.tick);
        self.resync = Some(Vec::new());
        self.last_resync_reason = Some(reason);
        for held in std::mem::take(&mut self.reorder) {
            self.buffer_delta(held);
        }
        self.buffer_delta(delta);
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

    /// Keep a delta in the recovery tail (sorted, no duplicate tick, at most
    /// `max_buffered_batches`).
    ///
    /// With a reorder grace, the tail keeps every component of the chain: a hole between two
    /// components may be a late delta of the same flush. A component is dropped only when the
    /// hole after it is older than the grace (`highest_observed - its last tick`): that is a loss,
    /// and the snapshot must then cover the rest (`tail_lost`). With grace 0 (the M1 rule) only the
    /// newest component stays.
    fn buffer_delta(&mut self, delta: WorldDelta) {
        let limit = self.config.max_buffered_batches;
        let grace = self.config.reorder_grace_ticks;
        let buffer = self.resync.get_or_insert_with(Vec::new);
        if buffer.iter().any(|held| held.tick == delta.tick) {
            return;
        }
        if buffer.len() >= limit {
            buffer.clear();
            self.buffer_overflows = self.buffer_overflows.saturating_add(1);
            self.gap_seen = true;
            self.tail_lost = true;
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
        // The first component to keep: every earlier one has a hole after it that is a loss.
        let keep_from = if grace == 0 {
            components.len().saturating_sub(1)
        } else {
            let high = self.highest_observed_tick.unwrap_or(0);
            (0..components.len().saturating_sub(1))
                .rev()
                .find(|&i| {
                    let last = components[i].last().map_or(0, |d| d.tick);
                    high.saturating_sub(last) > grace
                })
                .map_or(0, |i| i + 1)
        };
        if keep_from > 0 {
            self.gap_seen = true;
            self.tail_lost = true;
            if self.gap_anchor.is_none() {
                self.gap_anchor = components
                    .first()
                    .and_then(|component| component.first())
                    .map(|first| first.base_tick);
            }
        }
        *buffer = components.into_iter().skip(keep_from).flatten().collect();
    }

    fn stage_snapshot(&self, snapshot: &Snapshot) -> Option<(Candidate, RecordStore)> {
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
            unit_orders: HashMap::new(),
            health_append: Vec::new(),
            health_latest: HashMap::new(),
            health_forget: HashSet::new(),
            tick: snapshot.tick,
        };
        candidate.unit_orders =
            stage_unit_orders(context, &snapshot.unit_orders, &candidate.units)?;
        let store = replica_store::stage_snapshot_store(
            context,
            snapshot,
            &candidate.units,
            &candidate.economy,
        )?;
        if !valid_economy(
            context,
            &candidate.units,
            &candidate.nodes,
            &candidate.economy,
        ) || !valid_m3_records(context, &candidate.units, &candidate.nodes, candidate.m3())
            || !apply_health_records(
                context,
                snapshot.tick,
                &snapshot.healths,
                &self.health_trace,
                &mut candidate,
            )
        {
            return None;
        }
        Some((candidate, store))
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
            unit_orders: self.unit_orders.clone(),
            health_append: Vec::new(),
            health_latest: HashMap::new(),
            health_forget: HashSet::new(),
            tick,
        }
    }

    fn stage_delta(
        &self,
        current: &Candidate,
        store: &RecordStore,
        delta: &WorldDelta,
    ) -> Option<(Candidate, StoreStep)> {
        self.stage_delta_owned(current.clone(), store, delta)
    }

    fn stage_delta_owned(
        &self,
        mut next: Candidate,
        store: &RecordStore,
        delta: &WorldDelta,
    ) -> Option<(Candidate, StoreStep)> {
        let context = self.context.as_ref()?;
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
            || !apply_health_records(
                context,
                delta.tick,
                &delta.healths,
                &self.health_trace,
                &mut next,
            )
            || !merge_economy(&mut next.economy, &delta.economy)
            || !merge_m3_records(&mut next, delta)
        {
            return None;
        }
        // A delta carries the complete record of each unit whose record changed, and
        // `orders_cleared` for each unit whose record left the view (design 6.8). The table is
        // merged in chain order; only a `Snapshot` replaces it (`stage_snapshot`).
        next.unit_orders = merge_unit_orders(
            context,
            std::mem::take(&mut next.unit_orders),
            &delta.unit_orders,
            &delta.records.orders_cleared,
            &next.units,
            &removed,
        )?;
        // The store change is staged next to the candidate: its step is applied by `commit` only
        // (or to the staged store of a resync), never here.
        let step = replica_store::stage_delta_step(
            context,
            store,
            &next.units,
            &next.economy,
            delta,
            &removed,
        )?;
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
            next.unit_orders.remove(id);
        }
        if !valid_economy(context, &next.units, &next.nodes, &next.economy)
            || !valid_m3_records(context, &next.units, &next.nodes, next.m3())
        {
            return None;
        }
        next.tick = delta.tick;
        Some((next, step))
    }

    fn commit(&mut self, candidate: Candidate, store: StoreUpdate, now: Instant) {
        self.units = candidate.units;
        self.nodes = candidate.nodes;
        self.economy = candidate.economy;
        self.belts = candidate.belts;
        self.inserters = candidate.inserters;
        self.extractors = candidate.extractors;
        self.weapons = candidate.weapons;
        self.healths = candidate.healths;
        self.research = candidate.research;
        self.unit_orders = candidate.unit_orders;
        match store {
            StoreUpdate::Replace(staged) => self.store = staged,
            StoreUpdate::Step(step) => self.store.apply(step),
        }
        for (id, entry) in candidate.health_append {
            let history = self.health_trace.entry(id).or_default();
            if history
                .last()
                .is_none_or(|last| last.current != entry.current)
            {
                history.push(entry);
                if history.len() > HEALTH_TRACE_WINDOW {
                    history.drain(..history.len() - HEALTH_TRACE_WINDOW);
                }
            }
        }
        for id in candidate.health_forget {
            self.health_trace.remove(&id);
        }
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

    // The getters of the side store (`L0-STORE`): the one source of each value that reads a 6.8
    // record. The interface reads these and keeps no storage of its own. A record is evaluated at
    // the tick of the last applied message, never later, so this is not prediction.

    /// The side store of the 6.8 records.
    pub fn store(&self) -> &RecordStore {
        &self.store
    }

    fn applied_tick(&self) -> u32 {
        self.last_applied_tick.unwrap_or(0)
    }

    /// `Value::Supply`: the own supply of `resource`, provided or consumed. `None` when the
    /// replica holds no record of it.
    pub fn supply(&self, resource: &str, side: SupplySide) -> Option<u64> {
        let slot = self.context.as_ref()?.slot;
        let record = self.store.supply_record(slot, resource)?;
        Some(match side {
            SupplySide::Provided => record.provided,
            SupplySide::Consumed => record.consumed,
        })
    }

    /// `Predicate::SupplyShort`, `AlertTrigger::SupplyShort` and the offline mark: true when the own
    /// consumed amount of `resource` is above the provided amount (design 6.2, "Computation";
    /// `turts_rules::supply::is_short`). False when the replica holds no record of the resource.
    /// The one place that derives it: no reader compares the two sides itself.
    pub fn supply_short(&self, resource: &str) -> bool {
        let Some(slot) = self.context.as_ref().map(|context| context.slot) else {
            return false;
        };
        self.store
            .supply_record(slot, resource)
            .is_some_and(|record| turts_rules::supply::is_short(record.provided, record.consumed))
    }

    /// Accepted owner-only repair action, if this source currently has one.
    pub fn repair(&self, repairer: u32) -> Option<&turts_net_protocol::RepairRecord> {
        self.store.repair(repairer)
    }

    pub fn repairs(&self) -> impl Iterator<Item = &turts_net_protocol::RepairRecord> {
        self.store.repairs()
    }

    /// `Source::ConstructionQueue`: the held orders of an own constructor.
    pub fn construction_queue(&self, constructor: u32) -> &[HeldOrder] {
        self.store.construction_queue(constructor)
    }

    /// `Source::Sites`: the sites that exist, by unit id.
    pub fn sites(&self) -> impl Iterator<Item = u32> + '_ {
        self.store.sites()
    }

    pub fn is_site(&self, unit: u32) -> bool {
        self.store.is_site(unit)
    }

    /// The progress record of an own site.
    pub fn site_progress(&self, unit: u32) -> Option<&RateRecord> {
        self.store.site_progress(unit)
    }

    /// The authoritative construction recipe that owns this site's refund policy.
    pub fn site_production(&self, unit: u32) -> Option<&str> {
        self.store.site_production(unit)
    }

    /// The value of an own site in raw units of 1/Q tick at the last applied tick.
    pub fn site_value(&self, unit: u32) -> Option<u64> {
        let record = self.store.site_progress(unit)?;
        Some(replica_store::record_value(record, self.applied_tick()))
    }

    /// The item field `Paid` of a site: `turts_rules::progress::paid_at`.
    pub fn site_paid(&self, unit: u32, cost: u64) -> Option<u64> {
        let record = self.store.site_progress(unit)?;
        Some(replica_store::record_paid(
            record,
            cost,
            self.applied_tick(),
        ))
    }

    /// The item field `Refund` of a site: `turts_rules::progress::refund_at`.
    pub fn site_refund(
        &self,
        unit: u32,
        cost: u64,
        case: CancelCase,
        refunds: &CancelRefunds,
    ) -> Option<u64> {
        let record = self.store.site_progress(unit)?;
        Some(replica_store::record_refund(
            record,
            cost,
            case,
            refunds,
            self.applied_tick(),
        ))
    }

    /// The progress record of a job (the item field `State` reads its `state` and `held`).
    pub fn job_progress(&self, producer: u32, job: u64) -> Option<&RateRecord> {
        self.store.job_progress(producer, job)
    }

    /// The value of a job in raw units of 1/Q tick at the last applied tick.
    pub fn job_value(&self, producer: u32, job: u64) -> Option<u64> {
        let record = self.store.job_progress(producer, job)?;
        Some(replica_store::record_value(record, self.applied_tick()))
    }

    /// The elapsed ticks of a job at the tick of the last applied delta, from its progress record:
    /// `done`, `total` and `per_tick` count raw units of 1/Q tick (`RateRecord`), so the value in
    /// units (`turts_rules::progress::value_at`) becomes ticks of base time with
    /// `base_ticks_done(units, duration_ticks, total)`, the duration read from the `JobState` of
    /// the queue. At Q = 1 the units are the ticks. Never later than the last applied tick, so this
    /// is not prediction. 0 when the replica holds no record of the job, or no queue holds the job.
    pub fn job_elapsed_ticks(&self, producer: u32, job: u64) -> u64 {
        let Some(record) = self.store.job_progress(producer, job) else {
            return 0;
        };
        let Some(duration_ticks) = self
            .economy
            .queues
            .iter()
            .filter(|queue| queue.producer.0 == producer)
            .flat_map(|queue| queue.jobs.iter())
            .find(|held| held.id.0 == job)
            .map(|held| held.duration_ticks)
        else {
            return 0;
        };
        let units = replica_store::record_value(record, self.applied_tick());
        turts_rules::progress::base_ticks_done(units, duration_ticks, record.total)
    }

    /// The item field `Paid` of a job: `turts_rules::progress::paid_at`.
    pub fn job_paid(&self, producer: u32, job: u64, cost: u64) -> Option<u64> {
        let record = self.store.job_progress(producer, job)?;
        Some(replica_store::record_paid(
            record,
            cost,
            self.applied_tick(),
        ))
    }

    /// The item field `Refund` of a job: `turts_rules::progress::refund_at`.
    pub fn job_refund(
        &self,
        producer: u32,
        job: u64,
        cost: u64,
        case: CancelCase,
        refunds: &CancelRefunds,
    ) -> Option<u64> {
        let record = self.store.job_progress(producer, job)?;
        Some(replica_store::record_refund(
            record,
            cost,
            case,
            refunds,
            self.applied_tick(),
        ))
    }

    /// The public match state (the empty state until a message brings one).
    pub fn match_state(&self) -> &MatchState {
        self.store.match_state()
    }

    /// The outcome record of the receiving slot.
    pub fn own_outcome(&self) -> Option<SlotOutcome> {
        let slot = self.context.as_ref()?.slot;
        replica_store::outcome_of(self.store.match_state(), slot)
    }

    /// `Value::Outcome`.
    pub fn outcome(&self) -> Option<Outcome> {
        self.own_outcome().map(|outcome| outcome.outcome)
    }

    /// `Value::OutcomeReason`.
    pub fn outcome_reason(&self) -> Option<OutcomeReason> {
        self.own_outcome().map(|outcome| outcome.reason)
    }

    /// `Value::MatchClock`: the ticks from the start tick of the match (`Welcome.start.start_tick`,
    /// set by `set_match_start`) to the last applied tick. 0 before the start. The clock stops with no
    /// store record: the server sends the `Snapshot` of the end tick and then no delta (design 6.7,
    /// the Ended row), so the applied tick stops at the end tick.
    pub fn match_clock_ticks(&self) -> u32 {
        self.applied_tick().saturating_sub(self.start_tick)
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
    finite_pos(unit.pos) && context.unit_definition(&unit.content_id).is_some()
}

fn stage_unit_orders(
    context: &ReplicaContext,
    records: &[UnitOrderRecord],
    units: &impl UnitTable,
) -> Option<HashMap<u32, UnitOrderRecord>> {
    let mut staged = HashMap::with_capacity(records.len());
    for record in records {
        let unit = units.get(&record.unit.0)?;
        if unit.owner_slot != context.slot || staged.contains_key(&record.unit.0) {
            return None;
        }
        let valid_order = |id: &str| {
            ContentId::parse(id)
                .ok()
                .is_some_and(|id| context.content.catalog.orders.contains_key(&id))
        };
        if record
            .current
            .iter()
            .any(|summary| !valid_order(&summary.order))
            || record
                .queued
                .iter()
                .any(|summary| !valid_order(&summary.order))
            || record
                .last_failure
                .as_ref()
                .is_some_and(|failure| !valid_order(&failure.order))
            || record
                .current
                .iter()
                .chain(&record.queued)
                .any(|summary| match &summary.target {
                    turts_net_protocol::OrderTarget::Point(point) => !finite_pos(*point),
                    turts_net_protocol::OrderTarget::TerrainWork(work) => {
                        !crate::terrain::valid_designation(
                            &context.content,
                            unit.content_id,
                            &summary.order,
                            work,
                        )
                    }
                    // A placement is a content at a point: the point must be finite too.
                    turts_net_protocol::OrderTarget::ContentAtPoint { point, .. } => {
                        !finite_pos(*point)
                    }
                    _ => false,
                })
        {
            return None;
        }
        staged.insert(record.unit.0, record.clone());
    }
    Some(staged)
}

/// Merge the order records of one delta into the table, in chain order. `None` rejects the
/// delta: a record that is not valid (`stage_unit_orders`), a unit in both lists, or a cleared
/// unit that is unknown, not owned or removed by the same delta (the server never names a
/// removed unit in `orders_cleared`).
fn merge_unit_orders(
    context: &ReplicaContext,
    mut table: HashMap<u32, UnitOrderRecord>,
    records: &[UnitOrderRecord],
    cleared: &[UnitId],
    units: &impl UnitTable,
    removed: &HashSet<u32>,
) -> Option<HashMap<u32, UnitOrderRecord>> {
    let changed = stage_unit_orders(context, records, units)?;
    let mut seen_cleared = HashSet::new();
    for unit in cleared {
        let known = units
            .get(&unit.0)
            .is_some_and(|replica| replica.owner_slot == context.slot);
        if !known
            || removed.contains(&unit.0)
            || changed.contains_key(&unit.0)
            || !seen_cleared.insert(unit.0)
        {
            return None;
        }
        table.remove(&unit.0);
    }
    table.extend(changed);
    Some(table)
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
    units: &impl UnitTable,
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
        let Some(unit_id) = parse_id(unit.content_id) else {
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
        let Some(worker_type) = parse_id(unit_state.content_id) else {
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
            let Some(receiver_type) = parse_id(receiver.content_id) else {
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
        let Some(owner_type) = parse_id(owner_unit.content_id) else {
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
            let mut expected_cost_totals = BTreeMap::<(CostSource, ContentId), u128>::new();
            for (source, resource, amount) in expected_cost_lines {
                let total = expected_cost_totals.entry((source, resource)).or_default();
                let Some(sum) = total.checked_add(u128::from(amount)) else {
                    return false;
                };
                *total = sum;
            }
            let mut expected_costs = BTreeMap::<(CostSource, ContentId), u64>::new();
            for (key, total) in expected_cost_totals {
                let Ok(total) = u64::try_from(total) else {
                    return false;
                };
                expected_costs.insert(key, total);
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
                            || !economy.inventories.iter().any(|inventory| {
                                inventory.unit == queue.producer
                                    && inventory.port == InventoryPort::Main
                            })
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
    if !replace_keyed(&mut current.balances, &changes.balances, |record| {
        record.resource.clone()
    }) || !replace_keyed(&mut current.inventories, &changes.inventories, |record| {
        (
            record.unit.0,
            match record.port {
                InventoryPort::Main => 0u8,
                InventoryPort::Output => 1u8,
            },
        )
    }) || !replace_keyed(&mut current.orders, &changes.orders, |record| record.unit.0)
        || !replace_keyed(&mut current.queues, &changes.queues, |record| {
            record.producer.0
        })
    {
        return false;
    }
    true
}

/// Apply unique absolute replacements. Duplicate input keys fail before any state changes.
pub(crate) fn replace_keyed<T: Clone, K: Eq + std::hash::Hash>(
    current: &mut Vec<T>,
    changes: &[T],
    key: impl Fn(&T) -> K,
) -> bool {
    if changes.is_empty() {
        return true;
    }
    let mut pending = HashMap::with_capacity(changes.len());
    for (index, change) in changes.iter().enumerate() {
        if pending.insert(key(change), index).is_some() {
            return false;
        }
    }
    // Preserve the existing row order and append newly seen keys in wire order.
    for existing in current.iter_mut() {
        if let Some(index) = pending.remove(&key(existing)) {
            *existing = changes[index].clone();
        }
    }
    if !pending.is_empty() {
        current.reserve(pending.len());
        for change in changes {
            if pending.remove(&key(change)).is_some() {
                current.push(change.clone());
            }
        }
    }
    true
}

fn rows_changed<T>(
    current: &[T],
    changes: &[T],
    removed: &HashSet<u32>,
    key: impl Fn(&T) -> u32,
) -> bool {
    !changes.is_empty()
        || (!removed.is_empty() && current.iter().any(|row| removed.contains(&key(row))))
}

fn stage_rows<T: Clone>(
    current: &[T],
    changes: &[T],
    removed: &HashSet<u32>,
    key: impl Fn(&T) -> u32,
) -> Option<Option<Vec<T>>> {
    if !rows_changed(current, changes, removed, &key) {
        return Some(None);
    }
    let mut next = current.to_vec();
    if !replace_keyed(&mut next, changes, &key) {
        return None;
    }
    next.retain(|row| !removed.contains(&key(row)));
    Some(Some(next))
}
fn staged_slice<'a, T>(staged: &'a Option<Vec<T>>, current: &'a [T]) -> &'a [T] {
    staged.as_deref().unwrap_or(current)
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
    units: &impl UnitTable,
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
    valid_m3_records(
        context,
        units,
        nodes,
        M3Records {
            belts: &delta.belts,
            inserters: &delta.inserters,
            extractors: &delta.extractors,
            weapons: &delta.weapons,
            healths: &delta.healths,
            research: &delta.research,
        },
    )
}

fn unique_by<T, K: Eq + std::hash::Hash>(records: &[T], key: impl Fn(&T) -> K) -> bool {
    let mut seen = HashSet::new();
    records.iter().all(|record| seen.insert(key(record)))
}

fn unit_definition<'a>(
    context: &'a ReplicaContext,
    units: &impl UnitTable,
    id: u32,
    owner_only: bool,
) -> Option<&'a turts_mod_api::ResolvedUnit> {
    let unit = units.get(&id)?;
    if owner_only && unit.owner_slot != context.slot {
        return None;
    }
    context.unit_definition(unit.content_id)
}

fn valid_health(context: &ReplicaContext, units: &impl UnitTable, record: &HealthRecord) -> bool {
    unit_definition(context, units, record.unit.0, false)
        .and_then(|definition| definition.health.as_ref())
        .is_some_and(|health| record.current <= health.max)
}

fn apply_health_records(
    context: &ReplicaContext,
    tick: u32,
    changes: &[HealthRecord],
    base_history: &BTreeMap<u32, Vec<HealthTraceEntry>>,
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
        let previous = candidate
            .health_latest
            .get(&record.unit.0)
            .copied()
            .or_else(|| {
                base_history
                    .get(&record.unit.0)
                    .and_then(|history| history.last())
                    .map(|entry| entry.current)
            });
        if previous != Some(record.current) {
            candidate.health_append.push((
                record.unit.0,
                HealthTraceEntry {
                    tick,
                    current: record.current,
                },
            ));
            candidate
                .health_latest
                .insert(record.unit.0, record.current);
        }
    }
    true
}

#[derive(Clone, Copy)]
struct M3Records<'a> {
    belts: &'a [BeltPieceRecord],
    inserters: &'a [InserterRecord],
    extractors: &'a [ExtractorRecord],
    weapons: &'a [WeaponRecord],
    healths: &'a [HealthRecord],
    research: &'a [ResearchRecord],
}
impl Candidate {
    fn m3(&self) -> M3Records<'_> {
        M3Records {
            belts: &self.belts,
            inserters: &self.inserters,
            extractors: &self.extractors,
            weapons: &self.weapons,
            healths: &self.healths,
            research: &self.research,
        }
    }
}

fn valid_m3_records(
    context: &ReplicaContext,
    units: &impl UnitTable,
    nodes: &HashMap<NodeId, NodeState>,
    candidate: M3Records<'_>,
) -> bool {
    if !unique_by(candidate.belts, |record| record.piece.0)
        || !unique_by(candidate.inserters, |record| record.unit.0)
        || !unique_by(candidate.extractors, |record| record.unit.0)
        || !unique_by(candidate.weapons, |record| record.unit.0)
        || !unique_by(candidate.healths, |record| record.unit.0)
        || !unique_by(candidate.research, |record| record.slot.0)
    {
        return false;
    }
    for record in candidate.belts {
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
    for record in candidate.inserters {
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
    for record in candidate.extractors {
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
    for record in candidate.weapons {
        if unit_definition(context, units, record.unit.0, true)
            .is_none_or(|unit| unit.weapon.is_none())
            || record
                .last_target
                .is_some_and(|target| !units.contains_key(&target.0))
        {
            return false;
        }
    }
    for record in candidate.healths {
        if !valid_health(context, units, record) {
            return false;
        }
    }
    for record in candidate.research {
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
        let expected = completed
            .iter()
            .filter(|id| {
                context
                    .content
                    .catalog
                    .technologies
                    .get(*id)
                    .is_some_and(|technology| {
                        technology.effects.iter().any(|effect| {
                            matches!(effect, turts_mod_api::ResolvedEffect::ArmourModifier { .. })
                        })
                    })
            })
            .cloned()
            .collect::<HashSet<_>>();
        let mut ordered = HashSet::new();
        if record.armour_order.iter().any(|text| {
            parse_id(text).is_none_or(|id| !expected.contains(&id) || !ordered.insert(id))
        }) || ordered != expected
        {
            return false;
        }
    }
    true
}

fn valid_known_entities(
    context: &ReplicaContext,
    candidate: &Candidate,
    knowledge: &turts_mod_api::KnowledgeSnapshot,
) -> bool {
    valid_known_view(context, &candidate.units, &candidate.nodes, knowledge)
}

fn valid_known_view(
    context: &ReplicaContext,
    units: &impl UnitTable,
    nodes: &HashMap<NodeId, NodeState>,
    knowledge: &turts_mod_api::KnowledgeSnapshot,
) -> bool {
    use crate::visibility::{FogCell, position_state};
    let current = |pos: Pos| {
        position_state(knowledge, turts_mod_api::WorldPos { x: pos.x, y: pos.y })
            == Some(FogCell::Observed)
    };
    let stale_units: HashSet<_> = knowledge
        .remembered_units
        .iter()
        .map(|row| row.unit)
        .collect();
    let stale_nodes: HashSet<_> = knowledge
        .remembered_nodes
        .iter()
        .map(|row| row.node)
        .collect();
    units.rows().all(|(id, unit)| {
        (unit.owner_slot == context.slot || unit.position.is_some_and(current))
            && !stale_units.contains(&id)
    }) && nodes
        .values()
        .all(|node| current(node.pos) && !stale_nodes.contains(&node.id.0))
        && knowledge
            .remembered_units
            .iter()
            .all(|row| context.content.catalog.units.contains_key(&row.content))
        && knowledge
            .remembered_nodes
            .iter()
            .all(|row| context.content.catalog.nodes.contains_key(&row.content))
}
