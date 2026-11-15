#![cfg(test)]
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
    crate::session::participant_trace::enable_replay(app.world_mut())?;
    {
        let mut execution = app.world_mut().resource_mut::<BotExecution>();
        execution.mode = mode;
        execution.inputs = inputs;
        execution.error = None;
    }
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
    app.add_systems(
        FixedLast,
        verify.after(crate::economylog::capture_completed_tick),
    );
    Ok(())
}
fn prepare(world: &mut World) {
    let Some(mut playback) = world.remove_resource::<Playback>() else {
        return;
    };
    if playback.error.is_none()
        && let Err(error) = prepare_inner(world, &mut playback)
    {
        playback.error = Some(error)
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
    let mut prepared_actors = std::collections::BTreeSet::new();
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
                if prepared_actors.insert(command.actor.clone()) {
                    prepare_recorded_actor(world, &command.actor)?;
                }
                actors::stage_bot_command(world, command)
                    .map_err(|error| format!("replay bot staging tick{tick}: {error:?}"))?;
            }
            ActorLogIdentity::Bot { .. } => {}
        }
    }
    Ok(())
}
fn prepare_recorded_actor(world: &mut World, actor: &BotActor) -> Result<(), String> {
    actors::prepare_reconciliation(world, actor)?;
    let batch = actors::actor_reconciliation(world, actor)?;
    let catalog = &world.resource::<crate::orders::ServerCatalog>().0;
    let book = world.resource::<actors::BotActors>();
    let mut retained = batch.unapplied.clone();
    for effect in &batch.included {
        if effect.terminal || effect.refused {
            continue;
        }
        let tracked = &book.entries[&actor.slot].requests[&effect.request];
        let recipe = tracked
            .admitted_work
            .as_ref()
            .map(|work| &work.recipe)
            .or_else(|| match &tracked.command.order.target {
                BotTarget::Content(content) | BotTarget::ContentAtPoint { content, .. } => {
                    Some(content)
                }
                _ => None,
            })
            .ok_or("recorded pending work lacks its recipe")?;
        let costs = catalog
            .production
            .get(recipe)
            .map(|recipe| recipe.costs.as_slice())
            .or_else(|| {
                catalog
                    .technologies
                    .get(recipe)
                    .map(|technology| technology.costs.as_slice())
            })
            .ok_or("recorded pending recipe is absent from the locked catalog")?;
        // Match live coordinator retirement: paid and authoritative reserved
        // amounts both release provisional debt, even while work is running.
        if has_unrepresented_cost(costs, effect)? {
            retained.push(effect.request.clone());
        }
    }
    actors::acknowledge(world, actor, batch.watermark, &retained)
}

fn has_unrepresented_cost(
    costs: &[turts_mod_api::ResolvedCost],
    effect: &turts_bot::ObservedRequestEffect,
) -> Result<bool, String> {
    for cost in costs {
        let endpoint = match cost.source {
            turts_mod_api::CostSource::SlotBalance => {
                turts_bot::SourceEndpoint::SlotBalance(effect.request.actor.slot)
            }
            turts_mod_api::CostSource::ProducerInventory => turts_bot::SourceEndpoint::Inventory {
                unit: effect.request.unit,
                port: turts_bot::InventoryPort::Main,
            },
        };
        let amount = |rows: &[turts_bot::AccountAmount]| {
            rows.iter()
                .find(|row| {
                    row.account.endpoint == endpoint && row.account.resource == cost.resource
                })
                .map_or(0, |row| row.amount)
        };
        let represented = amount(&effect.paid)
            .checked_add(amount(&effect.reserved))
            .ok_or("recorded paid and reserved amounts overflow")?;
        if represented > cost.amount {
            return Err("recorded work exceeds its authored cost".into());
        }
        if represented < cost.amount {
            return Ok(true);
        }
    }
    Ok(false)
}

fn verify(world: &mut World) {
    let Some(mut playback) = world.remove_resource::<Playback>() else {
        return;
    };
    if playback.error.is_none()
        && let Err(error) = verify_inner(world, &mut playback)
    {
        playback.error = Some(error)
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
        app.add_systems(
            FixedLast,
            crate::economylog::capture_completed_tick.after(session::ParticipantTraceComplete),
        );
        path
    }
    fn human_stop(app: &mut App, connection: Entity) {
        let unit = turts_sim::completed_world(app.world())
            .units
            .iter()
            .find(|unit| unit.owner_slot == 0 && unit.content_id == "stockyard:worker")
            .expect("the human has an ordinary worker")
            .unit_id;
        app.world_mut()
            .resource_mut::<session::StagedPlayerCommands>()
            .0
            .push_back(session::StagedPlayerCommand {
                connection,
                slot: SlotId(0),
                envelope: turts_net_protocol::PlayerCommand {
                    seq: 1,
                    order: turts_net_protocol::OrderEnvelope {
                        order: "stockyard:order_stop".into(),
                        units: vec![turts_net_protocol::UnitId(unit)],
                        target: OrderTarget::None,
                        queue: QueueMode::Replace,
                        group: 0,
                    },
                },
            });
    }
    #[test]
    fn recorded_retirement_counts_paid_and_reserved_costs_before_work_finishes() {
        use turts_bot::{
            AccountAmount, InventoryPort, ObservedRequestEffect, RequestKey, SourceEndpoint,
            SpendKey, Watermark,
        };
        let actor = BotActor {
            slot: 1,
            profile: ContentId::parse("stockyard:opponent").unwrap(),
        };
        let resource = ContentId::parse("stockyard:timber").unwrap();
        let key = SpendKey {
            endpoint: SourceEndpoint::Inventory {
                unit: 7,
                port: InventoryPort::Main,
            },
            resource: resource.clone(),
        };
        let costs = [turts_mod_api::ResolvedCost {
            resource,
            amount: 12,
            source: turts_mod_api::CostSource::ProducerInventory,
        }];
        let mut effect = ObservedRequestEffect::held_placement(
            RequestKey {
                actor,
                sequence: 1,
                unit: 7,
            },
            Watermark {
                completed_tick: 1,
                revision: 1,
            },
        );
        for (paid, reserved, unpaid) in [
            (0, 0, true),
            (6, 0, true),
            (12, 0, false),
            (0, 12, false),
            (6, 6, false),
        ] {
            effect.paid = vec![AccountAmount {
                account: key.clone(),
                amount: paid,
            }];
            effect.reserved = vec![AccountAmount {
                account: key.clone(),
                amount: reserved,
            }];
            assert_eq!(has_unrepresented_cost(&costs, &effect).unwrap(), unpaid);
        }
        assert!(!effect.terminal);
        assert!(!has_unrepresented_cost(&[], &effect).unwrap());
    }
    #[test]
    fn recorded_actor_retires_terminal_receipts_beyond_the_envelope_limit() {
        let mut app = fixture();
        app.world_mut().resource_mut::<BotExecution>().mode = BotExecutionMode::RecordedCommands;
        let actor = app.world().resource::<BotHost>().bots[0].actor.clone();
        let unit = turts_sim::completed_world(app.world())
            .units
            .iter()
            .find(|unit| unit.owner_slot == actor.slot && unit.content_id == "stockyard:worker")
            .unwrap()
            .unit_id;
        for sequence in 1..=70 {
            prepare_recorded_actor(app.world_mut(), &actor).unwrap();
            actors::stage_bot_command(
                app.world_mut(),
                EmittedCommand {
                    actor: actor.clone(),
                    sequence,
                    task: TaskKey::new("replay", "bounded", sequence.to_string()),
                    units: vec![unit],
                    order: BotOrder {
                        content: ContentId::parse("stockyard:order_stop").unwrap(),
                        target: BotTarget::None,
                        queue: BotQueueMode::Replace,
                        group: sequence,
                    },
                },
            )
            .unwrap();
            app.update();
            assert!(
                app.world().resource::<actors::BotActors>().entries[&actor.slot]
                    .requests
                    .len()
                    <= 1
            );
            assert!(
                actors::actor_results(app.world(), &actor)
                    .iter()
                    .any(|result| result.accepted)
            );
        }
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
        human_stop(&mut app, human);
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
            if let Some(start) = pause
                && tick == start + 40
            {
                assert_eq!(
                    app.world().resource::<BotHost>().bots[0]
                        .coordinator
                        .pending()
                        .len(),
                    paused_pending,
                    "pause must retain in-flight COT commitments even when authoritative work advances"
                );
                let reconnected = app
                    .world_mut()
                    .spawn(session::Session {
                        last_seq: None,
                        slot: SlotId(0),
                        player_id: 7,
                    })
                    .id();
                human_stop(&mut app, reconnected);
            }
        }
        assert!(pause.is_some(), "fixture must pause real admitted COT work");
        assert!(paused_pending > 0);
        assert!(app.world().resource::<EconomyLogSink>().error.is_none());
        drop(app);
        let records = turts_audit::read_jsonl_file(&path).unwrap();
        let human_epochs = records
            .iter()
            .filter_map(|record| match record {
                EconomyLogRecord::Actor(ActorLogRecord::Submission {
                    actor: ActorLogIdentity::Human { connection, .. },
                    sequence,
                    ..
                }) => {
                    assert_eq!(*sequence, 1);
                    Some(*connection)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            human_epochs.len(),
            2,
            "record inputs on both transport epochs"
        );
        assert_ne!(human_epochs[0], human_epochs[1]);
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
