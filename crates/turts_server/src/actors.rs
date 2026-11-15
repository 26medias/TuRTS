//! Trusted participant identities and the staged ordinary bot command boundary.
//!
//! A player message never constructs a bot identity. Gameplay authorization remains
//! in the shared admission and simulation command paths.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

use bevy::prelude::*;
use turts_audit::{ActorLogIdentity, ActorLogRecord, ActorTask};
use turts_bot::{
    ActorResult, BotActor, BotQueueMode, BotTarget, CoordinatorLimits, EmittedCommand,
    ReconcileBatch, RequestKey, Watermark, WorkIdentity, WorkKind,
};
use turts_net_protocol::{
    CommandRejectReason, OrderEnvelope, OrderResult, OrderTarget, PlayerCommand, Pos,
};
use turts_sim::{SlotId, Slots};

const DEFAULT_PENDING_MEMBERS: usize = 64 * 256;

pub(crate) use crate::actor_accounting::TrackedRequest;

pub(crate) struct BotActorState {
    pub actor: BotActor,
    pub last_seq: Option<u32>,
    pub published: Option<ReconcileBatch>,
    pub acknowledged: Option<Watermark>,
    pub requests: BTreeMap<RequestKey, TrackedRequest>,
    pub pending_limit: usize,
    /// The last complete basis is retained, but cannot fund new decisions while stale.
    pub publication_error: Option<String>,
    demand_publication: bool,
}

#[derive(Resource)]
pub(crate) struct BotActors {
    pub entries: BTreeMap<u8, BotActorState>,
    pub staged: VecDeque<Arc<EmittedCommand>>,
    pending: BTreeMap<(u8, u32), PlayerCommand>,
    limits: CoordinatorLimits,
    row_limit: usize,
    last_published_tick: Option<u32>,
    error: Option<String>,
}
impl Default for BotActors {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            staged: VecDeque::new(),
            pending: BTreeMap::new(),
            limits: CoordinatorLimits::default(),
            row_limit: 65_536,
            last_published_tick: None,
            error: None,
        }
    }
}

pub(crate) struct PreparedActorLimits {
    limits: CoordinatorLimits,
    row_limit: usize,
    member_limit: usize,
    initial: BTreeMap<BotActor, ReconcileBatch>,
}
/// Prepare bounds and the initial accounting publication before permanent claims.
pub(crate) fn prepare_actor_limits(
    world: &World,
    actors: &[BotActor],
    limits: CoordinatorLimits,
    row_limit: usize,
) -> Result<PreparedActorLimits, String> {
    let member_limit = usize::try_from(limits.pending_requests)
        .ok()
        .and_then(|envelopes| {
            usize::try_from(limits.affected_units)
                .ok()
                .and_then(|units| envelopes.checked_mul(units))
        })
        .ok_or("bot pending member limit overflow")?;
    if limits.pending_requests == 0 || limits.affected_units == 0 || row_limit == 0 {
        return Err(
            "bot actor limits must allow pending envelopes, members and account rows".into(),
        );
    }
    let view = turts_sim::completed_world(world);
    let committed = turts_sim::committed_tick(world);
    let catalog = &world.resource::<crate::orders::ServerCatalog>().0;
    let watermark = Watermark {
        completed_tick: view.tick,
        revision: u64::from(view.tick),
    };
    let mut initial = BTreeMap::new();
    for actor in actors {
        let batch = crate::actor_accounting::build_batch(
            view,
            committed,
            catalog,
            actor,
            &mut BTreeMap::new(),
            watermark,
            row_limit,
        )?;
        if initial.insert(actor.clone(), batch).is_some() {
            return Err("duplicate prepared bot actor".into());
        }
    }
    Ok(PreparedActorLimits {
        limits,
        row_limit,
        member_limit,
        initial,
    })
}
/// Apply only the already validated setup after the matching atomic claim batch.
pub(crate) fn configure_actor_limits(world: &mut World, prepared: PreparedActorLimits) {
    let mut book = world.resource_mut::<BotActors>();
    book.limits = prepared.limits;
    book.row_limit = prepared.row_limit;
    for entry in book.entries.values_mut() {
        entry.pending_limit = prepared.member_limit;
        entry.published = prepared.initial.get(&entry.actor).cloned();
        entry.publication_error = None;
    }
}
#[derive(Resource)]
struct ActorPipelineInstalled;
pub(crate) fn install(app: &mut App) {
    if app.world().contains_resource::<ActorPipelineInstalled>() {
        return;
    }
    app.insert_resource(ActorPipelineInstalled);
    app.add_systems(
        FixedPreUpdate,
        dispatch_staged
            .after(crate::session::admit_player_commands)
            .before(turts_sim::SimSet::Commands),
    );
    app.add_systems(
        FixedPostUpdate,
        publish_completed.after(turts_sim::SimSet::Collect),
    );
}

impl BotActors {
    pub fn contains_actor(&self, actor: &BotActor) -> bool {
        self.entries
            .get(&actor.slot)
            .is_some_and(|entry| entry.actor == *actor)
    }
}

/// Validate the complete installation before changing any permanent slot claim.
/// The host validates profiles and creates all factories before calling this port.
pub(crate) fn install_bot_actors(world: &mut World, actors: &[BotActor]) -> Result<(), String> {
    if actors.is_empty() {
        return Ok(());
    }
    let current = world.resource::<Slots>();
    let mut claims = Slots::new(current.count());
    for (slot, owner) in current.owners().into_iter().enumerate() {
        if let Some(owner) = owner {
            claims
                .claim_at(SlotId(slot as u8), owner)
                .map_err(|error| format!("cannot stage existing slot: {error:?}"))?;
        }
    }
    let mut seen = BTreeSet::new();
    for actor in actors {
        if !seen.insert(actor.slot)
            || world
                .get_resource::<BotActors>()
                .is_some_and(|book| book.entries.contains_key(&actor.slot))
        {
            return Err(format!("duplicate bot slot {}", actor.slot));
        }
        claims
            .claim_at(SlotId(actor.slot), u64::MAX - u64::from(actor.slot))
            .map_err(|error| format!("cannot reserve bot slot {}: {error:?}", actor.slot))?;
    }
    world.insert_resource(claims);
    world.init_resource::<BotActors>();
    world.init_resource::<crate::session::ReservedParticipantIds>();
    world.init_resource::<turts_sim::receipts::CommandWorkReceipts>();
    for actor in actors {
        world
            .resource_mut::<crate::session::ReservedParticipantIds>()
            .0
            .insert(u64::MAX - u64::from(actor.slot));
    }
    let mut book = world.resource_mut::<BotActors>();
    for actor in actors {
        book.entries.insert(
            actor.slot,
            BotActorState {
                actor: actor.clone(),
                last_seq: None,
                published: None,
                acknowledged: None,
                requests: BTreeMap::new(),
                pending_limit: DEFAULT_PENDING_MEMBERS,
                publication_error: None,
                demand_publication: false,
            },
        );
    }
    Ok(())
}

/// Stage an internally identified request. This neither executes nor approves it.
pub(crate) fn stage_bot_command(
    world: &mut World,
    command: EmittedCommand,
) -> Result<(), CommandRejectReason> {
    let completed_tick = world
        .get_resource::<turts_sim::CompletedWorld>()
        .map(|completed| completed.0.tick);
    let Some(mut book) = world.get_resource_mut::<BotActors>() else {
        return Err(CommandRejectReason::NotOwner);
    };
    if !book.contains_actor(&command.actor) {
        return Err(CommandRejectReason::NotOwner);
    }
    if command.units.is_empty() {
        return Err(CommandRejectReason::EmptyCommand);
    }
    if command.units.len() > turts_net_protocol::MAX_ORDER_UNITS {
        return Err(CommandRejectReason::TooManyUnits);
    }
    let mut units = BTreeSet::new();
    if command.units.iter().any(|unit| !units.insert(*unit)) {
        return Err(CommandRejectReason::DuplicateUnit);
    }
    if book.error.is_some() {
        return Err(CommandRejectReason::Internal);
    }
    let envelope_limit = book.limits.pending_requests as usize;
    if command.units.len() > book.limits.affected_units as usize {
        return Err(CommandRejectReason::TooManyUnits);
    }
    let entry = book
        .entries
        .get_mut(&command.actor.slot)
        .ok_or(CommandRejectReason::NotOwner)?;
    if entry.publication_error.is_some()
        || entry.demand_publication
            && entry.published.as_ref().is_none_or(|batch| {
                !batch.complete || Some(batch.watermark.completed_tick) != completed_tick
            })
    {
        return Err(CommandRejectReason::RateLimited);
    }
    if entry.last_seq.is_some_and(|last| command.sequence <= last)
        || entry
            .requests
            .keys()
            .any(|key| key.sequence == command.sequence)
    {
        return Err(CommandRejectReason::BadSequence);
    }
    if entry
        .requests
        .keys()
        .map(|key| key.sequence)
        .collect::<BTreeSet<_>>()
        .len()
        >= envelope_limit
    {
        return Err(CommandRejectReason::RateLimited);
    }
    if entry
        .requests
        .len()
        .checked_add(command.units.len())
        .is_none_or(|count| count > entry.pending_limit)
    {
        return Err(CommandRejectReason::RateLimited);
    }
    // The queue and all per-member receipts share one immutable intent. A group
    // never copies its complete unit vector once for every affected unit.
    let command = Arc::new(command);
    for unit in &command.units {
        entry.requests.insert(
            RequestKey {
                actor: command.actor.clone(),
                sequence: command.sequence,
                unit: *unit,
            },
            TrackedRequest::new(Arc::clone(&command)),
        );
    }
    book.staged.push_back(command);
    Ok(())
}

/// The public host consumes accounts only on decision ticks. Receipt facts still
/// advance at every Collect, including while the host is paused.
pub(crate) fn enable_demand_publication(world: &mut World) {
    for entry in world.resource_mut::<BotActors>().entries.values_mut() {
        entry.demand_publication = true;
    }
}

pub(crate) fn prepare_reconciliation(world: &mut World, actor: &BotActor) -> Result<(), String> {
    let mut book = world
        .remove_resource::<BotActors>()
        .ok_or("unknown bot actor")?;
    let result = (|| {
        let entry = book
            .entries
            .get_mut(&actor.slot)
            .filter(|entry| entry.actor == *actor)
            .ok_or("unknown bot actor")?;
        let view = turts_sim::completed_world(world);
        if entry
            .published
            .as_ref()
            .is_some_and(|batch| batch.complete && batch.watermark.completed_tick == view.tick)
            && entry.publication_error.is_none()
        {
            return Ok(());
        }
        let watermark = Watermark {
            completed_tick: view.tick,
            revision: u64::from(view.tick),
        };
        match crate::actor_accounting::build_batch(
            view,
            turts_sim::committed_tick(world),
            &world.resource::<crate::orders::ServerCatalog>().0,
            &entry.actor,
            &mut entry.requests,
            watermark,
            book.row_limit,
        ) {
            Ok(batch) => {
                entry.published = Some(batch);
                entry.publication_error = None;
                Ok(())
            }
            Err(error) => {
                entry.publication_error = Some(error.clone());
                Err(error)
            }
        }
    })();
    world.insert_resource(book);
    if let Err(error) = &result
        && error != crate::actor_accounting::ROW_LIMIT_EXCEEDED
    {
        fail_pipeline(world, error.clone());
    }
    result
}

pub(crate) fn actor_reconciliation(
    world: &World,
    actor: &BotActor,
) -> Result<ReconcileBatch, String> {
    let entry = world
        .get_resource::<BotActors>()
        .and_then(|book| book.entries.get(&actor.slot))
        .filter(|entry| entry.actor == *actor)
        .ok_or("unknown bot actor")?;
    if let Some(error) = &entry.publication_error {
        return Err(format!(
            "bot accounting publication is awaiting a fresh complete basis: {error}"
        ));
    }
    if entry.demand_publication
        && entry.published.as_ref().is_none_or(|batch| {
            batch.watermark.completed_tick != turts_sim::completed_world(world).tick
        })
    {
        return Err("accounting must be prepared from the latest completed tick".into());
    }
    entry
        .published
        .clone()
        .ok_or_else(|| "no completed actor accounting basis is available".into())
}

pub(crate) fn actor_results(world: &World, actor: &BotActor) -> Vec<ActorResult> {
    world
        .get_resource::<BotActors>()
        .and_then(|book| book.entries.get(&actor.slot))
        .filter(|entry| entry.actor == *actor)
        .map(|entry| {
            entry
                .requests
                .values()
                .filter_map(|request| request.result.as_deref().cloned())
                .collect()
        })
        .unwrap_or_default()
}

/// Release represented host receipts only after the coordinator accepts this basis.
pub(crate) fn acknowledge(
    world: &mut World,
    actor: &BotActor,
    watermark: Watermark,
    retained: &[RequestKey],
) -> Result<(), String> {
    let completed_tick = turts_sim::completed_world(world).tick;
    let mut book = world
        .get_resource_mut::<BotActors>()
        .ok_or("unknown bot actor")?;
    let entry = book
        .entries
        .get_mut(&actor.slot)
        .filter(|entry| entry.actor == *actor)
        .ok_or("unknown bot actor")?;
    if entry.publication_error.is_some()
        || entry.demand_publication && watermark.completed_tick != completed_tick
    {
        return Err("cannot acknowledge a stale blocked accounting publication".into());
    }
    let batch = entry
        .published
        .as_ref()
        .filter(|batch| batch.complete && batch.watermark == watermark)
        .ok_or("acknowledgement does not name a complete published basis")?;
    if entry.acknowledged.is_some_and(|old| watermark < old)
        || retained
            .iter()
            .any(|key| key.actor != *actor || !entry.requests.contains_key(key))
    {
        return Err("invalid actor acknowledgement".into());
    }
    let represented = batch
        .included
        .iter()
        .map(|effect| effect.request.clone())
        .collect::<BTreeSet<_>>();
    let retained = retained.iter().collect::<BTreeSet<_>>();
    let forgotten = entry
        .requests
        .iter()
        .filter(|(key, _)| represented.contains(*key) && !retained.contains(*key))
        .filter_map(|(_, tracked)| tracked.admitted_work.as_ref().map(|work| work.job.0))
        .collect::<BTreeSet<_>>();
    entry
        .requests
        .retain(|key, _| !represented.contains(key) || retained.contains(key));
    entry.acknowledged = Some(watermark);
    let _ = book;
    if let Some(mut receipts) = world.get_resource_mut::<turts_sim::receipts::CommandWorkReceipts>()
    {
        for job in forgotten {
            receipts.forget_job(job);
        }
    }
    Ok(())
}

/// Live input only. Trusted replay supplies its recorded presence input instead.
pub(crate) fn human_present(world: &World) -> bool {
    world
        .iter_entities()
        .any(|entity| entity.contains::<crate::session::Session>())
}

/// Trusted trace ports are shared with transported and replayed human inputs.
pub(crate) fn record_installation(world: &mut World, record: ActorLogRecord) -> Result<(), String> {
    crate::session::participant_trace::record(world, record)
}
pub(crate) fn record_lifecycle(world: &mut World, record: ActorLogRecord) -> Result<(), String> {
    crate::session::participant_trace::record(world, record)
}
#[cfg(test)]
pub(crate) fn actor_trace(world: &World) -> &[ActorLogRecord] {
    crate::session::participant_trace::actor_trace(world)
}
#[cfg(test)]
pub(crate) fn stage_recorded_human(
    world: &mut World,
    actor: ActorLogIdentity,
    envelope: PlayerCommand,
) -> Result<(), String> {
    crate::session::participant_trace::stage_recorded_human(world, actor, envelope)
}

fn wire_command(command: &EmittedCommand) -> PlayerCommand {
    let point = |point: &turts_mod_api::WorldPos| Pos {
        x: point.x,
        y: point.y,
    };
    PlayerCommand {
        seq: command.sequence,
        order: OrderEnvelope {
            order: command.order.content.to_string(),
            units: command
                .units
                .iter()
                .copied()
                .map(turts_net_protocol::UnitId)
                .collect(),
            target: match &command.order.target {
                BotTarget::None => OrderTarget::None,
                BotTarget::Point(at) => OrderTarget::Point(point(at)),
                BotTarget::Unit(unit) => OrderTarget::Unit(turts_net_protocol::UnitId(*unit)),
                BotTarget::Node(node) => OrderTarget::Node(turts_net_protocol::NodeId(*node)),
                BotTarget::Job(job) => OrderTarget::Job(turts_net_protocol::JobId(*job)),
                BotTarget::Content(content) => OrderTarget::Content(content.to_string()),
                BotTarget::ContentAtPoint { content, point: at } => OrderTarget::ContentAtPoint {
                    content: content.to_string(),
                    point: point(at),
                },
            },
            queue: match command.order.queue {
                BotQueueMode::Replace => turts_net_protocol::QueueMode::Replace,
                BotQueueMode::Append => turts_net_protocol::QueueMode::Append,
            },
            group: command.order.group,
        },
    }
}
fn fail_pipeline(world: &mut World, error: String) {
    eprintln!("turts_server: bot actor pipeline: {error}");
    let mut book = world.resource_mut::<BotActors>();
    book.error = Some(error.clone());
    for entry in book.entries.values_mut() {
        entry.published = None;
    }
    if let Some(mut execution) = world.get_resource_mut::<crate::bots::BotExecution>() {
        execution.error = Some(error);
    }
}
/// Humans already admitted in this tick retain their ordinary queue order.
/// Bot envelopes retain the coordinator's deterministic staged order.
pub(crate) fn dispatch_staged(world: &mut World) {
    let Some(mut book) = world.get_resource_mut::<BotActors>() else {
        return;
    };
    if book.error.is_some() {
        return;
    }
    let staged = std::mem::take(&mut book.staged);
    let _ = book;
    let tick = world.resource::<turts_sim::Tick>().0;
    let hz = world.resource::<crate::session::Advertised>().tick_hz;
    for command in staged {
        if world
            .resource::<BotActors>()
            .entries
            .get(&command.actor.slot)
            .is_some_and(|entry| entry.publication_error.is_some())
        {
            // Already staged work stays bounded by the existing request limits.
            // A temporary publication failure must not silently lose its result.
            world.resource_mut::<BotActors>().staged.push_back(command);
            continue;
        }
        let identity = ActorLogIdentity::Bot {
            profile: command.actor.profile.clone(),
            slot: command.actor.slot,
        };
        let envelope = wire_command(&command);
        let task = Some(ActorTask {
            controller: command.task.controller.clone(),
            goal: command.task.goal.clone(),
            instance: command.task.instance.clone(),
        });
        let ordinal = match crate::session::participant_trace::submission(
            world, &identity, task, &envelope,
        ) {
            Ok(ordinal) => ordinal,
            Err(error) => {
                crate::session::participant_trace::fail(world, error);
                return;
            }
        };
        let admitted = world
            .resource_mut::<crate::session::RateLimitBuckets>()
            .admit(command.actor.slot, tick, hz, command.units.len());
        let newer = world
            .resource_mut::<BotActors>()
            .entries
            .get_mut(&command.actor.slot)
            .filter(|entry| entry.actor == command.actor)
            .is_some_and(|entry| {
                crate::session::advance_sequence(&mut entry.last_seq, command.sequence)
            });
        match crate::session::admit_actor_envelope(
            world,
            SlotId(command.actor.slot),
            &envelope,
            admitted,
            newer,
        ) {
            Ok(commands) => {
                if let Err(error) = crate::session::participant_trace::accept(
                    world,
                    command.actor.slot,
                    command.sequence,
                    ordinal,
                    identity,
                ) {
                    crate::session::participant_trace::fail(world, error);
                    return;
                }
                world
                    .resource_mut::<turts_sim::receipts::CommandWorkReceipts>()
                    .watch(SlotId(command.actor.slot), command.sequence);
                world
                    .resource_mut::<BotActors>()
                    .pending
                    .insert((command.actor.slot, command.sequence), envelope);
                world
                    .resource_mut::<turts_sim::CommandQueue>()
                    .0
                    .extend(commands);
            }
            Err((mut result, reason)) => {
                result.tick = tick;
                if let Err(error) = crate::session::participant_trace::rejection(
                    world, &identity, ordinal, &envelope, &result, reason,
                ) {
                    crate::session::participant_trace::fail(world, error);
                    return;
                }
                finalize_members(world, &command.actor, &envelope, &result, BTreeMap::new());
            }
        }
    }
}
fn finalize_members(
    world: &mut World,
    actor: &BotActor,
    envelope: &PlayerCommand,
    result: &OrderResult,
    hints: BTreeMap<u32, turts_sim::receipts::AdmittedWork>,
) {
    let catalog = &world.resource::<crate::orders::ServerCatalog>().0;
    let kinds = hints
        .iter()
        .map(|(unit, hint)| {
            (
                *unit,
                if hint.construction {
                    WorkKind::Construction
                } else if catalog.technologies.contains_key(&hint.recipe) {
                    WorkKind::Research
                } else {
                    WorkKind::Production
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut book = world.resource_mut::<BotActors>();
    let Some(entry) = book
        .entries
        .get_mut(&actor.slot)
        .filter(|entry| entry.actor == *actor)
    else {
        return;
    };
    for unit in &envelope.order.units {
        let key = RequestKey {
            actor: actor.clone(),
            sequence: envelope.seq,
            unit: unit.0,
        };
        let Some(tracked) = entry.requests.get_mut(&key) else {
            continue;
        };
        let accepted = result.accepted.contains(unit);
        let reason = result
            .refused
            .iter()
            .find(|row| row.unit == *unit)
            .map(|row| row.reason)
            .or(result.envelope);
        let hint = hints.get(&unit.0).cloned().filter(|_| accepted);
        let work = hint.as_ref().map(|hint| WorkIdentity {
            unit: hint.unit,
            id: hint.job.0,
            kind: kinds[&unit.0],
        });
        tracked.finalize(
            ActorResult {
                request: key,
                effect_watermark: Watermark {
                    completed_tick: result.tick,
                    revision: u64::from(result.tick),
                },
                accepted,
                work,
                reason: reason.map(|reason| format!("{reason:?}")),
            },
            hint,
        );
    }
}
/// Finalize member outcomes and exact work hints before publishing the immutable basis.
pub(crate) fn publish_completed(world: &mut World) {
    let Some(book) = world.get_resource::<BotActors>() else {
        return;
    };
    if book.error.is_some() {
        return;
    }
    let tick = turts_sim::completed_world(world).tick;
    if book.last_published_tick == Some(tick) {
        return;
    }
    #[cfg(test)]
    let results_started = crate::bot_profile::start();
    let pending = book.pending.clone();
    for ((slot, seq), envelope) in pending {
        let actor = world.resource::<BotActors>().entries[&slot].actor.clone();
        let results = world
            .resource::<turts_sim::CommandResults>()
            .0
            .iter()
            .filter(|result| result.slot.0 == slot && result.seq == seq)
            .cloned()
            .collect::<Vec<_>>();
        if results.is_empty() {
            fail_pipeline(
                world,
                format!("ordinary bot command {slot}/{seq} has no completed result"),
            );
            return;
        }
        let mut result = crate::orders::to_wire(&results, &envelope);
        result.tick = tick;
        if let Err(error) = crate::session::participant_trace::result(world, slot, seq, &result) {
            crate::session::participant_trace::fail(world, error);
            return;
        }
        let hints = world
            .resource_mut::<turts_sim::receipts::CommandWorkReceipts>()
            .take(SlotId(slot), seq);
        finalize_members(world, &actor, &envelope, &result, hints);
        world
            .resource_mut::<BotActors>()
            .pending
            .remove(&(slot, seq));
    }
    #[cfg(test)]
    crate::bot_profile::finish("publication.results", results_started);
    #[cfg(test)]
    let hints_started = crate::bot_profile::start();
    // Held placement happens before the next command phase. Its hint must be
    // collected even when its site also completes or dies in the current tick.
    let keys = world
        .resource::<BotActors>()
        .entries
        .iter()
        .flat_map(|(slot, entry)| entry.requests.keys().map(|key| (*slot, key.sequence)))
        .collect::<BTreeSet<_>>();
    for (slot, seq) in keys {
        let hints = world
            .resource_mut::<turts_sim::receipts::CommandWorkReceipts>()
            .take(SlotId(slot), seq);
        if hints.is_empty() {
            continue;
        }
        if let Some(entry) = world.resource_mut::<BotActors>().entries.get_mut(&slot) {
            for (unit, hint) in hints {
                let key = RequestKey {
                    actor: entry.actor.clone(),
                    sequence: seq,
                    unit,
                };
                if let Some(tracked) = entry.requests.get_mut(&key) {
                    tracked.admitted_work = Some(hint);
                }
            }
        }
    }
    #[cfg(test)]
    crate::bot_profile::finish("publication.held_hints", hints_started);
    // The completed resources remain borrowed in place. Only actor bookkeeping
    // leaves the World, so publication does not clone the package or full world.
    let mut book = world
        .remove_resource::<BotActors>()
        .expect("actor book was checked above");
    let result = (|| {
        let view = turts_sim::completed_world(world);
        let committed = turts_sim::committed_tick(world);
        let catalog = &world.resource::<crate::orders::ServerCatalog>().0;
        let row_limit = book.row_limit;
        for entry in book.entries.values_mut() {
            let watermark = Watermark {
                completed_tick: tick,
                revision: u64::from(tick),
            };
            if entry.demand_publication {
                crate::actor_accounting::capture_intermediate_receipts(
                    view,
                    committed,
                    catalog,
                    &entry.actor,
                    &mut entry.requests,
                    watermark,
                )?;
                continue;
            }
            match crate::actor_accounting::build_batch(
                view,
                committed,
                catalog,
                &entry.actor,
                &mut entry.requests,
                watermark,
                row_limit,
            ) {
                Ok(batch) => {
                    entry.published = Some(batch);
                    if entry.publication_error.take().is_some() {
                        eprintln!(
                            "turts_server: bot slot {} accounting publication restored",
                            entry.actor.slot
                        );
                    }
                }
                Err(error) if error == crate::actor_accounting::ROW_LIMIT_EXCEEDED => {
                    // A successful batch already captures receipts atomically. Only
                    // a blocked publication needs this separate retained capture.
                    crate::actor_accounting::capture_receipts(
                        view,
                        committed,
                        catalog,
                        &entry.actor,
                        &mut entry.requests,
                        watermark,
                    )?;
                    if entry.publication_error.as_ref() != Some(&error) {
                        eprintln!(
                            "turts_server: bot slot {} accounting publication held: {error}",
                            entry.actor.slot
                        );
                    }
                    entry.publication_error = Some(error);
                }
                Err(error) => return Err(error),
            }
        }
        book.last_published_tick = Some(tick);
        Ok::<(), String>(())
    })();
    world.insert_resource(book);
    if let Err(error) = result {
        fail_pipeline(world, error);
    }
}

#[cfg(test)]
#[path = "actors_tests.rs"]
mod tests;
