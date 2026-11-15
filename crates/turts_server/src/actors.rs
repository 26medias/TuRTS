//! Trusted participant identities and the staged ordinary bot command boundary.
//!
//! A player message never constructs a bot identity. Gameplay authorization remains
//! in the shared admission and simulation command paths.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use bevy::prelude::*;
use turts_audit::ActorLogRecord;
use turts_bot::{ActorResult, BotActor, EmittedCommand, ReconcileBatch, RequestKey, Watermark};
use turts_net_protocol::CommandRejectReason;
use turts_sim::{SlotId, Slots};

const DEFAULT_PENDING_MEMBERS: usize = 64;

pub(crate) struct TrackedRequest {
    pub command: EmittedCommand,
    pub result: Option<ActorResult>,
}

pub(crate) struct BotActorState {
    pub actor: BotActor,
    pub player_id: u64,
    pub last_seq: Option<u32>,
    pub published: Option<ReconcileBatch>,
    pub acknowledged: Option<Watermark>,
    pub requests: BTreeMap<RequestKey, TrackedRequest>,
    pub pending_limit: usize,
}

#[derive(Resource, Default)]
pub(crate) struct BotActors {
    pub entries: BTreeMap<u8, BotActorState>,
    pub staged: VecDeque<EmittedCommand>,
}

impl BotActors {
    pub fn contains_player_id(&self, player_id: u64) -> bool {
        self.entries
            .values()
            .any(|entry| entry.player_id == player_id)
    }

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
                player_id: u64::MAX - u64::from(actor.slot),
                last_seq: None,
                published: None,
                acknowledged: None,
                requests: BTreeMap::new(),
                pending_limit: DEFAULT_PENDING_MEMBERS,
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
    let entry = book
        .entries
        .get_mut(&command.actor.slot)
        .ok_or(CommandRejectReason::NotOwner)?;
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
        .len()
        .checked_add(command.units.len())
        .is_none_or(|count| count > entry.pending_limit)
    {
        return Err(CommandRejectReason::RateLimited);
    }
    for unit in &command.units {
        entry.requests.insert(
            RequestKey {
                actor: command.actor.clone(),
                sequence: command.sequence,
                unit: *unit,
            },
            TrackedRequest {
                command: command.clone(),
                result: None,
            },
        );
    }
    book.staged.push_back(command);
    Ok(())
}

pub(crate) fn actor_reconciliation(
    world: &World,
    actor: &BotActor,
) -> Result<ReconcileBatch, String> {
    world
        .get_resource::<BotActors>()
        .and_then(|book| book.entries.get(&actor.slot))
        .filter(|entry| entry.actor == *actor)
        .and_then(|entry| entry.published.clone())
        .ok_or_else(|| "no completed actor accounting basis is available".to_owned())
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
                .filter_map(|request| request.result.clone())
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
    let mut book = world
        .get_resource_mut::<BotActors>()
        .ok_or("unknown bot actor")?;
    let entry = book
        .entries
        .get_mut(&actor.slot)
        .filter(|entry| entry.actor == *actor)
        .ok_or("unknown bot actor")?;
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
    entry
        .requests
        .retain(|key, _| !represented.contains(key) || retained.contains(key));
    entry.acknowledged = Some(watermark);
    Ok(())
}

/// Live input only. Trusted replay supplies its recorded presence input instead.
pub(crate) fn human_present(world: &World) -> bool {
    world
        .iter_entities()
        .any(|entity| entity.contains::<crate::session::Session>())
}

#[derive(Resource, Default)]
struct ActorLogState {
    installed: bool,
    lifecycle_tick: Option<u32>,
}

/// The host publishes its locked installation once, before guards and submissions.
pub(crate) fn record_installation(world: &mut World, record: ActorLogRecord) -> Result<(), String> {
    if !matches!(&record, ActorLogRecord::Installation(_)) {
        return Err("actor installation port requires an Installation record".into());
    }
    world.init_resource::<ActorLogState>();
    if world.resource::<ActorLogState>().installed {
        return Err("actor installation was already recorded".into());
    }
    if let Some(mut sink) = world.get_resource_mut::<crate::economylog::EconomyLogSink>() {
        sink.write_actor(record);
        if let Some(error) = &sink.error {
            return Err(error.clone());
        }
    }
    world.resource_mut::<ActorLogState>().installed = true;
    Ok(())
}

/// Lifecycle is trusted runtime input. It is recorded before this tick's decisions.
pub(crate) fn record_lifecycle(world: &mut World, record: ActorLogRecord) -> Result<(), String> {
    let ActorLogRecord::Lifecycle { tick, .. } = &record else {
        return Err("actor lifecycle port requires a Lifecycle record".into());
    };
    if *tick != world.resource::<turts_sim::Tick>().0 {
        return Err("actor lifecycle does not name the current fixed tick".into());
    }
    let state = world
        .get_resource::<ActorLogState>()
        .ok_or("actor installation is missing")?;
    if !state.installed
        || state
            .lifecycle_tick
            .is_some_and(|previous| *tick <= previous)
    {
        return Err("actor lifecycle input is duplicate or precedes installation".into());
    }
    let tick = *tick;
    if let Some(mut sink) = world.get_resource_mut::<crate::economylog::EconomyLogSink>() {
        sink.write_actor(record);
        if let Some(error) = &sink.error {
            return Err(error.clone());
        }
    }
    world.resource_mut::<ActorLogState>().lifecycle_tick = Some(tick);
    Ok(())
}
