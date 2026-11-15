//! Headless acceptance playback of the trusted typed log. No CSV or private mutation commands.
use super::{BotExecution, BotExecutionMode, BotHost, LifecycleInput};
use crate::{actors, economylog::EconomyLogSink, session};
use bevy::prelude::*;
use std::collections::BTreeMap;
use turts_audit::{
    ActorInstallation, ActorLogIdentity, ActorLogRecord, ActorPhase, EconomyLogRecord,
};
use turts_bot::{BotActor, BotOrder, BotQueueMode, BotTarget, EmittedCommand, TaskKey};
use turts_mod_api::{ContentId, WorldPos};
use turts_net_protocol::{OrderTarget, QueueMode};
use turts_sim::{AuditState, SimSet, SlotId, Slots, Tick};

pub(crate) struct ReplayTape {
    installation: ActorInstallation,
    initial: AuditState,
    states: BTreeMap<u32, AuditState>,
    actors: BTreeMap<u32, Vec<ActorLogRecord>>,
    claims: BTreeMap<u32, Vec<(u64, u8)>>,
}
impl ReplayTape {
    /// Run the independent stream validator before accepting any replay input.
    pub fn from_records(records: &[EconomyLogRecord]) -> Result<Self, String> {
        turts_audit::validate_records(records)?;
        let mut initial = None;
        let mut installation = None;
        let mut states = BTreeMap::new();
        let mut actors = BTreeMap::<u32, Vec<ActorLogRecord>>::new();
        let mut claims = BTreeMap::<u32, Vec<(u64, u8)>>::new();
        for record in records {
            match record {
                EconomyLogRecord::Initial(state) => initial = Some(state.clone()),
                EconomyLogRecord::Actor(ActorLogRecord::Installation(header)) => {
                    installation = Some(header.clone())
                }
                EconomyLogRecord::Actor(record) => {
                    let tick = match record {
                        ActorLogRecord::Lifecycle { tick, .. }
                        | ActorLogRecord::Submission { tick, .. }
                        | ActorLogRecord::Result { tick, .. } => *tick,
                        ActorLogRecord::Installation(_) => {
                            unreachable!("installation handled separately")
                        }
                    };
                    actors.entry(tick).or_default().push(record.clone());
                }
                EconomyLogRecord::Tick { tick, state, .. } => {
                    states.insert(*tick, state.clone());
                }
                EconomyLogRecord::Claim {
                    tick,
                    player_id,
                    slot,
                } => claims.entry(*tick).or_default().push((*player_id, *slot)),
                _ => {}
            }
        }
        Ok(Self {
            installation: installation.ok_or("replay requires a locked bot installation")?,
            initial: initial.ok_or("replay requires an initial state")?,
            states,
            actors,
            claims,
        })
    }
    pub fn last_tick(&self) -> u32 {
        self.states
            .last_key_value()
            .map_or(self.initial.economy.tick, |(tick, _)| *tick)
    }
}
#[derive(Resource)]
pub(crate) struct Playback {
    tape: ReplayTape,
    mode: BotExecutionMode,
    last_prepared: Option<u32>,
    pub error: Option<String>,
    pub verified_ticks: u32,
}
/// Call after restoring the locked package/configuration/initial claims and before tick one.
/// Playback verifies that restoration. It never repairs a mismatch by editing gameplay state.
pub(crate) fn install(
    app: &mut App,
    tape: ReplayTape,
    mode: BotExecutionMode,
) -> Result<(), String> {
    if mode == BotExecutionMode::Live {
        return Err("replay requires an explicit playback mode".into());
    }
    if app.world().contains_resource::<Playback>() || app.world().resource::<Tick>().0 != 0 {
        return Err("replay must be installed once before the first tick".into());
    }
    let host = app
        .world()
        .get_resource::<BotHost>()
        .ok_or("replay requires the recorded public bot installation")?;
    if host.installation != tape.installation {
        return Err("replay installation, profile hashes, seed, or limits differ".into());
    }
    if host.installation_recorded {
        return Err("replay cannot replace an already active bot host".into());
    }
    let actual = turts_sim::audit_state(app.world_mut());
    if actual != tape.initial {
        return Err("replay initial authoritative state or claims differ".into());
    }
    let mut inputs = BTreeMap::new();
    for (tick, records) in &tape.actors {
        for record in records {
            if let ActorLogRecord::Lifecycle {
                humans_present,
                phase,
                ..
            } = record
            {
                let previous = tape
                    .states
                    .get(&tick.saturating_sub(1))
                    .unwrap_or(&tape.initial);
                let defeated = previous
                    .economy
                    .match_outcomes
                    .as_ref()
                    .into_iter()
                    .flat_map(|o| o.slots.iter().enumerate())
                    .filter_map(|(slot, result)| {
                        result
                            .filter(|r| r.outcome == turts_sim::outcome::Outcome::Defeat)
                            .map(|_| slot as u8)
                    })
                    .collect();
                inputs.insert(
                    *tick,
                    LifecycleInput {
                        scheduler_tick: *tick,
                        human_present: *humans_present,
                        running: *phase == ActorPhase::Running,
                        ended: *phase == ActorPhase::Ended,
                        defeated,
                    },
                );
            }
        }
    }
    let mut execution = app.world_mut().resource_mut::<BotExecution>();
    execution.mode = mode;
    execution.inputs = inputs;
    execution.error = None;
    drop(execution);
    if mode == BotExecutionMode::RecordedCommands {
        actors::record_installation(
            app.world_mut(),
            ActorLogRecord::Installation(tape.installation.clone()),
        )?;
        app.world_mut()
            .resource_mut::<BotHost>()
            .installation_recorded = true;
    }
    app.insert_resource(Playback {
        tape,
        mode,
        last_prepared: None,
        error: None,
        verified_ticks: 0,
    });
    app.add_systems(
        FixedPreUpdate,
        prepare
            .after(SimSet::Construction)
            .before(super::drive)
            .before(session::admit_player_commands),
    );
    app.add_systems(FixedLast, verify);
    Ok(())
}
fn prepare(world: &mut World) {
    let Some(mut playback) = world.remove_resource::<Playback>() else {
        return;
    };
    if playback.error.is_none() {
        if let Err(error) = prepare_inner(world, &mut playback) {
            playback.error = Some(error)
        }
    }
    world.insert_resource(playback);
}
fn prepare_inner(world: &mut World, playback: &mut Playback) -> Result<(), String> {
    let tick = world.resource::<Tick>().0;
    if playback.last_prepared == Some(tick) {
        return Err(format!("replay tick {tick} was prepared twice"));
    }
    if tick > playback.tape.last_tick() {
        return Ok(());
    }
    playback.last_prepared = Some(tick);
    if let Some(claims) = playback.tape.claims.get(&tick.saturating_sub(1)) {
        for (player, slot) in claims {
            world
                .resource_mut::<Slots>()
                .claim_at(SlotId(*slot), *player)
                .map_err(|error| format!("replay claim tick{tick}: {error:?}"))?;
            if let Some(mut sink) = world.get_resource_mut::<EconomyLogSink>() {
                sink.write_claim(tick.saturating_sub(1), *player, *slot)
            }
        }
    }
    let records = playback
        .tape
        .actors
        .get(&tick)
        .ok_or_else(|| format!("replay lacks lifecycle at tick {tick}"))?;
    if playback.mode == BotExecutionMode::RecordedCommands {
        let lifecycle = records
            .iter()
            .find(|record| matches!(record, ActorLogRecord::Lifecycle { .. }))
            .ok_or("recorded tick lacks lifecycle")?;
        actors::record_lifecycle(world, lifecycle.clone())?;
    }
    for record in records {
        let ActorLogRecord::Submission {
            actor, envelope, ..
        } = record
        else {
            continue;
        };
        match actor {
            ActorLogIdentity::Human { .. } => {
                actors::stage_recorded_human(world, actor.clone(), envelope.clone())
                    .map_err(|error| format!("replay human staging tick{tick}: {error:?}"))?
            }
            ActorLogIdentity::Bot { .. } if playback.mode == BotExecutionMode::RecordedCommands => {
                let command = bot_command(record)?;
                // Refusal is a recorded effect too. The shared path retains its result witness.
                let _ = actors::stage_bot_command(world, command);
            }
            ActorLogIdentity::Bot { .. } => {}
        }
    }
    Ok(())
}
fn verify(world: &mut World) {
    let Some(mut playback) = world.remove_resource::<Playback>() else {
        return;
    };
    if playback.error.is_none() {
        if let Err(error) = verify_inner(world, &mut playback) {
            playback.error = Some(error)
        }
    }
    world.insert_resource(playback);
}
fn verify_inner(world: &mut World, playback: &mut Playback) -> Result<(), String> {
    let tick = world.resource::<Tick>().0;
    let Some(expected) = playback.tape.states.get(&tick) else {
        return Ok(());
    };
    let actual = turts_sim::audit_state(world);
    if actual != *expected {
        return Err(format!(
            "replay authoritative state first differs at completed tick {tick}"
        ));
    }
    let actual = actors::actor_trace(world)
        .iter()
        .filter(|record| !matches!(record, ActorLogRecord::Installation(_)))
        .cloned()
        .collect::<Vec<_>>();
    let expected = playback.tape.actors.get(&tick).cloned().unwrap_or_default();
    if actual != expected {
        let index = actual
            .iter()
            .zip(&expected)
            .position(|(a, b)| a != b)
            .unwrap_or(actual.len().min(expected.len()));
        return Err(format!(
            "replay actor trace first differs at tick {tick}, entry {index}: actual {:?}, expected {:?}",
            actual.get(index),
            expected.get(index)
        ));
    }
    if let Some(error) = world.resource::<BotExecution>().error.as_ref() {
        return Err(format!("replay host tick {tick}: {error}"));
    }
    playback.verified_ticks = playback
        .verified_ticks
        .checked_add(1)
        .ok_or("replay verification count overflow")?;
    Ok(())
}
fn bot_command(record: &ActorLogRecord) -> Result<EmittedCommand, String> {
    let ActorLogRecord::Submission {
        actor: ActorLogIdentity::Bot { profile, slot },
        sequence,
        task,
        envelope,
        ..
    } = record
    else {
        return Err("replay expected a bot submission".into());
    };
    let task = task
        .as_ref()
        .ok_or("recorded bot submission lacks its task")?;
    if *sequence != envelope.seq {
        return Err("recorded bot command sequence differs from envelope".into());
    }
    let target = match &envelope.order.target {
        OrderTarget::None => BotTarget::None,
        OrderTarget::Point(point) => BotTarget::Point(WorldPos {
            x: point.x,
            y: point.y,
        }),
        OrderTarget::Unit(unit) => BotTarget::Unit(unit.0),
        OrderTarget::Node(node) => BotTarget::Node(node.0),
        OrderTarget::Job(job) => BotTarget::Job(job.0),
        OrderTarget::Content(content) => BotTarget::Content(ContentId::parse(content)?),
        OrderTarget::ContentAtPoint { content, point } => BotTarget::ContentAtPoint {
            content: ContentId::parse(content)?,
            point: WorldPos {
                x: point.x,
                y: point.y,
            },
        },
    };
    Ok(EmittedCommand {
        actor: BotActor {
            profile: profile.clone(),
            slot: *slot,
        },
        sequence: *sequence,
        task: TaskKey::new(&task.controller, &task.goal, &task.instance),
        units: envelope.order.units.iter().map(|u| u.0).collect(),
        order: BotOrder {
            content: ContentId::parse(&envelope.order.order)?,
            target,
            queue: match envelope.order.queue {
                QueueMode::Replace => BotQueueMode::Replace,
                QueueMode::Append => BotQueueMode::Append,
            },
            group: envelope.order.group,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_sim::AuditTrail;
    fn fixture() -> App {
        super::super::scenarios::live_package_with_guard(
            "stockyard",
            Some(turts_mod_api::BotStartWhen::HumanPresent),
        )
    }
    fn capture(app: &mut App, name: &str) -> std::path::PathBuf {
        let root = std::env::var_os("TURTS_BOT_EVIDENCE_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::env::temp_dir().join(format!("turts-public-bot-host-{}", std::process::id()))
            });
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(name);
        app.world_mut().resource_mut::<AuditTrail>().enable();
        let initial = turts_sim::audit_state(app.world_mut());
        let sink =
            EconomyLogSink::create(&path, app.world().resource::<AuditTrail>(), initial).unwrap();
        app.insert_resource(sink);
        app.add_systems(FixedLast, crate::economylog::capture_completed_tick);
        path
    }
    #[test]
    fn recorded_and_regenerated_cot_pause_replay_compare_every_tick_and_command() {
        let mut app = fixture();
        let path = capture(&mut app, "cot-pause-original.jsonl");
        let mut pause = None;
        let human = app
            .world_mut()
            .query_filtered::<Entity, With<session::Session>>()
            .single(app.world())
            .unwrap();
        let mut paused_pending = 0;
        for _ in 0..400 {
            app.update();
            let tick = app.world().resource::<Tick>().0;
            let has_cot = turts_sim::completed_world(app.world())
                .operating
                .as_ref()
                .is_some_and(|op| op.jobs.iter().any(|job| job.funding.is_some()));
            if pause.is_none()
                && has_cot
                && !app.world().resource::<BotHost>().bots[0]
                    .coordinator
                    .pending()
                    .is_empty()
            {
                paused_pending = app.world().resource::<BotHost>().bots[0]
                    .coordinator
                    .pending()
                    .len();
                app.world_mut().despawn(human);
                pause = Some(tick);
            }
            if let Some(start) = pause {
                if tick == start + 40 {
                    assert_eq!(
                        app.world().resource::<BotHost>().bots[0]
                            .coordinator
                            .pending()
                            .len(),
                        paused_pending,
                        "pause must retain in-flight COT commitments even when authoritative work advances"
                    );
                    app.world_mut().spawn(session::Session {
                        last_seq: None,
                        slot: SlotId(0),
                        player_id: 7,
                    });
                }
            }
        }
        assert!(pause.is_some(), "fixture must pause real admitted COT work");
        assert!(paused_pending > 0);
        assert!(app.world().resource::<EconomyLogSink>().error.is_none());
        drop(app);
        let records = turts_audit::read_jsonl_file(&path).unwrap();
        turts_audit::validate_records(&records).unwrap();
        let mut mismatched = ReplayTape::from_records(&records).unwrap();
        mismatched.installation.bots[0].seed ^= 1;
        let mut untouched = fixture();
        let claims = untouched.world().resource::<Slots>().owners();
        assert!(
            install(
                &mut untouched,
                mismatched,
                BotExecutionMode::RegenerateAndCompare
            )
            .unwrap_err()
            .contains("installation")
        );
        assert_eq!(untouched.world().resource::<Tick>().0, 0);
        assert_eq!(untouched.world().resource::<Slots>().owners(), claims);
        for (mode, name) in [
            (BotExecutionMode::RecordedCommands, "cot-recorded.jsonl"),
            (
                BotExecutionMode::RegenerateAndCompare,
                "cot-regenerated.jsonl",
            ),
        ] {
            let tape = ReplayTape::from_records(&records).unwrap();
            let end = tape.last_tick();
            let mut app = fixture();
            let replay_path = capture(&mut app, name);
            install(&mut app, tape, mode).unwrap();
            while app.world().resource::<Tick>().0 < end {
                app.update();
                assert!(
                    app.world().resource::<Playback>().error.is_none(),
                    "{:?}",
                    app.world().resource::<Playback>().error
                );
            }
            assert_eq!(app.world().resource::<Playback>().verified_ticks, end);
            assert!(app.world().resource::<EconomyLogSink>().error.is_none());
            drop(app);
            let replayed = turts_audit::read_jsonl_file(&replay_path).unwrap();
            turts_audit::validate_records(&replayed).unwrap();
        }
        eprintln!("public bot COT replay evidence: {}", path.display());
    }
}
