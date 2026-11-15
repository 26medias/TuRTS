//! Trusted participant inputs and bounded, completed-tick replay evidence.
use std::collections::{BTreeMap, VecDeque};

use bevy::prelude::*;
use turts_audit::{ActorLogIdentity, ActorLogRecord, ActorTask};
use turts_net_protocol::{CommandRejectReason, OrderResult, PlayerCommand};
use turts_sim::{CommandQueue, CommandResults, SlotId, Slots, Tick};

use super::{Advertised, PendingOrderCommands, RateLimitBuckets};
use crate::economylog::EconomyLogSink;

#[derive(Resource, Default)]
struct Trace {
    installed: bool,
    tick: Option<u32>,
    ordinal: u64,
    records: Vec<ActorLogRecord>,
    accepted: BTreeMap<(u8, u32), (u64, ActorLogIdentity)>,
}

#[derive(Resource, Default)]
struct TrustedReplay {
    staged: VecDeque<(ActorLogIdentity, PlayerCommand)>,
    sequences: BTreeMap<u64, (u64, u8, Option<u32>)>,
}

pub(crate) fn enable_replay(world: &mut World) -> Result<(), String> {
    if world.resource::<Tick>().0 != 0 || world.contains_resource::<TrustedReplay>() {
        return Err("trusted replay must be enabled once before tick one".into());
    }
    world.init_resource::<TrustedReplay>();
    Ok(())
}

pub(crate) fn replay_enabled(world: &World) -> bool {
    world.contains_resource::<TrustedReplay>()
}

pub(crate) fn actor_trace(world: &World) -> &[ActorLogRecord] {
    world
        .get_resource::<Trace>()
        .map_or(&[], |trace| &trace.records)
}

fn emit(world: &mut World, record: ActorLogRecord) -> Result<(), String> {
    if let Some(mut sink) = world.get_resource_mut::<EconomyLogSink>() {
        sink.write_actor(record.clone());
        if let Some(error) = &sink.error {
            return Err(error.clone());
        }
    }
    world.resource_mut::<Trace>().records.push(record);
    Ok(())
}

pub(crate) fn record(world: &mut World, record: ActorLogRecord) -> Result<(), String> {
    world.init_resource::<Trace>();
    match &record {
        ActorLogRecord::Installation(_) => {
            if world.resource::<Trace>().installed {
                return Err("actor installation was already recorded".into());
            }
            emit(world, record)?;
            world.resource_mut::<Trace>().installed = true;
        }
        ActorLogRecord::Lifecycle { tick, .. } => {
            let trace = world.resource::<Trace>();
            if !trace.installed
                || *tick != world.resource::<Tick>().0
                || trace.tick.is_some_and(|previous| *tick <= previous)
                || !trace.accepted.is_empty()
            {
                return Err(
                    "actor lifecycle is duplicate, out of order, or has unfinished commands".into(),
                );
            }
            let tick = *tick;
            let mut trace = world.resource_mut::<Trace>();
            trace.records.clear();
            trace.ordinal = 0;
            trace.tick = Some(tick);
            emit(world, record)?;
        }
        _ => return Err("trusted record port only accepts installation and lifecycle".into()),
    }
    Ok(())
}

pub(crate) fn submission(
    world: &mut World,
    actor: &ActorLogIdentity,
    task: Option<ActorTask>,
    envelope: &PlayerCommand,
) -> Result<Option<u64>, String> {
    let Some(trace) = world.get_resource::<Trace>() else {
        return Ok(None);
    };
    if !trace.installed {
        return Ok(None);
    }
    let tick = world.resource::<Tick>().0;
    if trace.tick != Some(tick) {
        return Err("actor submission has no current lifecycle".into());
    }
    let ordinal = trace
        .ordinal
        .checked_add(1)
        .ok_or("actor admission ordinal exhausted")?;
    world.resource_mut::<Trace>().ordinal = ordinal;
    emit(
        world,
        ActorLogRecord::Submission {
            tick,
            ordinal,
            actor: actor.clone(),
            sequence: envelope.seq,
            task,
            envelope: envelope.clone(),
        },
    )?;
    Ok(Some(ordinal))
}

pub(crate) fn accept(
    world: &mut World,
    slot: u8,
    sequence: u32,
    ordinal: Option<u64>,
    actor: ActorLogIdentity,
) -> Result<(), String> {
    let Some(ordinal) = ordinal else {
        return Ok(());
    };
    let mut trace = world.resource_mut::<Trace>();
    if trace.accepted.contains_key(&(slot, sequence)) {
        return Err("accepted participant request identity was reused".into());
    }
    trace.accepted.insert((slot, sequence), (ordinal, actor));
    Ok(())
}

pub(crate) fn result(
    world: &mut World,
    slot: u8,
    sequence: u32,
    result: &OrderResult,
) -> Result<(), String> {
    let Some((ordinal, actor)) = world
        .get_resource_mut::<Trace>()
        .and_then(|mut trace| trace.accepted.remove(&(slot, sequence)))
    else {
        return Ok(());
    };
    emit(
        world,
        ActorLogRecord::Result {
            tick: world.resource::<Tick>().0,
            ordinal,
            actor,
            sequence,
            result: result.clone(),
        },
    )
}

pub(crate) fn rejection(
    world: &mut World,
    actor: &ActorLogIdentity,
    ordinal: Option<u64>,
    envelope: &PlayerCommand,
    result: &OrderResult,
    reason: CommandRejectReason,
) -> Result<(), String> {
    let Some(ordinal) = ordinal else {
        return Ok(());
    };
    if world.contains_resource::<EconomyLogSink>() {
        if !world
            .get_resource::<turts_sim::AuditTrail>()
            .is_some_and(|trail| trail.enabled && trail.valid)
        {
            return Err("participant refusal has no valid economy audit capture".into());
        }
        let raw = postcard::to_allocvec(envelope).map_err(|error| error.to_string())?;
        let state = turts_sim::audit_state(world);
        let (connection, player_id, slot) = match actor {
            ActorLogIdentity::Human {
                connection,
                player_id,
                slot,
            } => (*connection, *player_id, *slot),
            ActorLogIdentity::Bot { slot, .. } => (0, u64::MAX - u64::from(*slot), *slot),
        };
        let tick = world.resource::<Tick>().0;
        let mut sink = world.resource_mut::<EconomyLogSink>();
        sink.write_admission(
            ordinal,
            connection,
            player_id,
            slot,
            envelope.seq,
            tick,
            reason,
            raw,
            state.clone(),
            state,
        );
        if let Some(error) = &sink.error {
            return Err(error.clone());
        }
    }
    emit(
        world,
        ActorLogRecord::Result {
            tick: world.resource::<Tick>().0,
            ordinal,
            actor: actor.clone(),
            sequence: envelope.seq,
            result: result.clone(),
        },
    )
}

pub(crate) fn stage_recorded_human(
    world: &mut World,
    identity: ActorLogIdentity,
    envelope: PlayerCommand,
) -> Result<(), String> {
    let ActorLogIdentity::Human {
        player_id,
        slot,
        connection,
    } = &identity
    else {
        return Err("recorded human port requires a human identity".into());
    };
    if *connection == 0
        || world.resource::<Slots>().slot_of(*player_id) != Some(SlotId(*slot))
        || world
            .get_resource::<super::ReservedParticipantIds>()
            .is_some_and(|ids| ids.0.contains(player_id))
    {
        return Err("recorded human differs from its permanent slot claim".into());
    }
    let mut replay = world
        .get_resource_mut::<TrustedReplay>()
        .ok_or("trusted replay is not enabled")?;
    if replay.staged.len() >= 4096 {
        return Err("recorded participant input bound exceeded".into());
    }
    if replay
        .sequences
        .get(connection)
        .is_some_and(|(player, owner, _)| *player != *player_id || *owner != *slot)
    {
        return Err("recorded connection changed its identity".into());
    }
    replay.staged.push_back((identity, envelope));
    Ok(())
}

pub(crate) fn admit_recorded_humans(world: &mut World) {
    let Some(mut replay) = world.get_resource_mut::<TrustedReplay>() else {
        return;
    };
    let batch = std::mem::take(&mut replay.staged);
    for (identity, envelope) in batch {
        let ActorLogIdentity::Human {
            player_id,
            slot,
            connection,
        } = identity.clone()
        else {
            unreachable!()
        };
        let tick = world.resource::<Tick>().0;
        let hz = world.resource::<Advertised>().tick_hz;
        let admitted = world.resource_mut::<RateLimitBuckets>().admit(
            slot,
            tick,
            hz,
            envelope.order.units.len(),
        );
        let newer = {
            let mut replay = world.resource_mut::<TrustedReplay>();
            let state = replay
                .sequences
                .entry(connection)
                .or_insert((player_id, slot, None));
            super::advance_sequence(&mut state.2, envelope.seq)
        };
        let attempt = submission(world, &identity, None, &envelope);
        let outcome = attempt.and_then(|ordinal| {
            match super::admit_actor_envelope(world, SlotId(slot), &envelope, admitted, newer) {
                Ok(commands) => {
                    accept(world, slot, envelope.seq, ordinal, identity)?;
                    world
                        .resource_mut::<PendingOrderCommands>()
                        .0
                        .insert((slot, envelope.seq), envelope);
                    world.resource_mut::<CommandQueue>().0.extend(commands);
                    Ok(())
                }
                Err((mut refused, reason)) => {
                    refused.tick = tick;
                    rejection(world, &identity, ordinal, &envelope, &refused, reason)
                }
            }
        });
        if let Err(error) = outcome {
            fail(world, error);
            break;
        }
    }
}

pub(crate) fn completed(world: &mut World) {
    let messages = {
        let results = world.resource::<CommandResults>();
        let pending = world.resource::<PendingOrderCommands>();
        let Some(trace) = world.get_resource::<Trace>() else {
            return;
        };
        trace
            .accepted
            .keys()
            .filter_map(|key| {
                let command = pending.0.get(key)?;
                let rows = results
                    .0
                    .iter()
                    .filter(|row| (row.slot.0, row.seq) == *key)
                    .cloned()
                    .collect::<Vec<_>>();
                if rows.is_empty() {
                    return None;
                }
                let mut message = crate::orders::to_wire(&rows, command);
                message.tick = world.resource::<Tick>().0;
                Some((*key, message))
            })
            .collect::<Vec<_>>()
    };
    for ((slot, seq), message) in messages {
        if let Err(error) = result(world, slot, seq, &message) {
            fail(world, error);
            break;
        }
    }
}

pub(crate) fn fail(world: &mut World, error: String) {
    if let Some(mut sink) = world.get_resource_mut::<EconomyLogSink>() {
        sink.error = Some(error.clone());
    }
    // This is an invariant failure. Continuing would publish an incomplete trusted stream.
    panic!("participant trace: {error}");
}
