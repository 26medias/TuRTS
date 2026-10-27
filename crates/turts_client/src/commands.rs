//! Client-side player command FIFO and one checked sequence per fresh connection.

use crate::ingest::{CommandResults, ReceiptWitnesses};
use crate::join::Phase;
use crate::scenario::ScenarioState;
use bevy::prelude::*;
use lightyear::prelude::*;
use std::collections::VecDeque;
use turts_net_lightyear::ReliableChannel;
use turts_net_protocol::{PlayerCommand, PlayerCommandKind};

#[derive(Debug, Clone, PartialEq)]
pub enum PendingCommand {
    Submit {
        kind: PlayerCommandKind,
        trace_step: Option<usize>,
    },
    Repeat {
        envelope: PlayerCommand,
        trace_step: usize,
    },
}

#[derive(Debug, Default, Resource)]
pub struct PendingCommands(pub VecDeque<PendingCommand>);

#[derive(Debug, Resource)]
pub struct CommandSequence {
    pub next: Option<u32>,
}
impl Default for CommandSequence {
    fn default() -> Self {
        Self { next: Some(1) }
    }
}
impl CommandSequence {
    pub fn allocate(&mut self) -> Result<u32, SequenceExhausted> {
        let current = self.next.ok_or(SequenceExhausted)?;
        self.next = current.checked_add(1);
        Ok(current)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceExhausted;
#[derive(Debug, Default, Resource)]
pub struct CommandSendStatus {
    pub reconnect_required: bool,
}
pub fn submit(pending: &mut PendingCommands, kind: PlayerCommandKind) {
    pending.0.push_back(PendingCommand::Submit {
        kind,
        trace_step: None,
    });
}

pub(crate) fn submit_trace(
    pending: &mut PendingCommands,
    kind: PlayerCommandKind,
    trace_step: usize,
) {
    pending.0.push_back(PendingCommand::Submit {
        kind,
        trace_step: Some(trace_step),
    });
}

pub(crate) fn repeat_trace(
    pending: &mut PendingCommands,
    envelope: PlayerCommand,
    trace_step: usize,
) {
    pending.0.push_back(PendingCommand::Repeat {
        envelope,
        trace_step,
    });
}

/// Owned mobile ids, ascending, for convenience adapters that expand an implicit all-units move
/// before it enters the shared FIFO. Explicit command bodies must remain literal.
pub fn owned_mobile_ids(state: &crate::ingest::ReplicaState) -> Vec<turts_net_protocol::UnitId> {
    let (Some(welcome), Some(replica), Some(content)) = (
        state.welcome,
        state.replica.as_ref(),
        state.content.as_ref(),
    ) else {
        return Vec::new();
    };
    let mut ids: Vec<_> = replica
        .units
        .iter()
        .filter_map(|(&id, unit)| {
            if unit.owner_slot != welcome.slot {
                return None;
            }
            let content_id = turts_mod_api::ContentId::parse(&unit.content_id).ok()?;
            let definition = content.catalog.units.get(&content_id)?;
            definition
                .movement
                .as_ref()
                .map(|_| turts_net_protocol::UnitId(id))
        })
        .collect();
    ids.sort_by_key(|id| id.0);
    ids
}

/// Send the single shared FIFO only after a fresh session has joined. Ordinary adapters must
/// provide their intended literal bodies before enqueueing; trace steps use the same path.
#[allow(clippy::too_many_arguments)] // Bevy system resources stay explicit at this boundary.
pub(crate) fn send_commands_traced(
    mut pending: ResMut<PendingCommands>,
    mut sequence: ResMut<CommandSequence>,
    mut status: ResMut<CommandSendStatus>,
    phase: Res<Phase>,
    mut senders: Query<(Entity, &mut MessageSender<PlayerCommand>), With<Connected>>,
    mut witnesses: ResMut<ReceiptWitnesses>,
    mut scenario: ResMut<ScenarioState>,
    results: Res<CommandResults>,
    #[cfg(test)] mut sent_capture: Option<ResMut<SentCommandCapture>>,
) {
    if !matches!(*phase, Phase::Joined { .. }) || senders.is_empty() || status.reconnect_required {
        return;
    }
    for (entity, mut sender) in &mut senders {
        let epoch = witnesses.epoch_for(entity.to_bits());
        while let Some(command) = pending.0.pop_front() {
            let (envelope, trace_step) = match command {
                PendingCommand::Submit { kind, trace_step } => {
                    let seq = match sequence.allocate() {
                        Ok(seq) => seq,
                        Err(_) => {
                            status.reconnect_required = true;
                            pending.0.clear();
                            scenario.error = Some("command sequence exhausted".into());
                            break;
                        }
                    };
                    (PlayerCommand { seq, kind }, trace_step)
                }
                PendingCommand::Repeat {
                    envelope,
                    trace_step,
                } => (envelope, Some(trace_step)),
            };
            sender.send::<ReliableChannel>(envelope.clone());
            #[cfg(test)]
            if let Some(capture) = sent_capture.as_deref_mut() {
                capture.0.push(envelope.clone());
            }
            witnesses.record_message(epoch, true);
            if let Some(step) = trace_step
                && let Err(error) = scenario.record_sent(step, envelope, results.0.len())
            {
                scenario.error = Some(error);
            }
        }
    }
}

#[cfg(test)]
#[derive(Resource, Default)]
pub(crate) struct SentCommandCapture(Vec<PlayerCommand>);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{ScenarioState, TraceAction};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn trace_sender_preserves_empty_static_and_foreign_move_bodies() {
        let temp_name = format!(
            "turts-command-literal-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(temp_name);
        std::fs::write(
            &path,
            r#"[
                {"after_ticks":0,"command":{"command":"submit","data":{"MoveTo":{"unit_ids":[],"target":{"x":1.0,"y":2.0}}}},"require_result":false},
                {"after_ticks":0,"command":{"command":"submit","data":{"MoveTo":{"unit_ids":[3,99],"target":{"x":4.0,"y":5.0}}}},"require_result":false}
            ]"#,
        )
        .expect("write temporary trace");
        let scenario = ScenarioState::from_path(Some(&path));
        std::fs::remove_file(path).expect("remove temporary trace");

        let mut app = App::new();
        app.insert_resource(PendingCommands::default())
            .insert_resource(CommandSequence::default())
            .insert_resource(CommandSendStatus::default())
            .insert_resource(Phase::Joined { slot: 0 })
            .insert_resource(ReceiptWitnesses::default())
            .insert_resource(CommandResults::default())
            .insert_resource(scenario)
            .insert_resource(SentCommandCapture::default());
        app.add_systems(Update, send_commands_traced);
        app.world_mut().spawn((
            MessageSender::<PlayerCommand>::default(),
            RemoteId(PeerId::Netcode(5)),
            Connected,
        ));

        for step_index in 0..2 {
            let actions = app
                .world_mut()
                .resource_mut::<ScenarioState>()
                .drive(0, 0, &[])
                .expect("trace action");
            let [
                TraceAction::Submit {
                    step_index: actual_step,
                    kind,
                },
            ] = actions.as_slice()
            else {
                panic!("expected one literal MoveTo action, got {actions:?}");
            };
            assert_eq!(*actual_step, step_index);
            let kind = kind.clone();
            app.world_mut()
                .resource_mut::<ScenarioState>()
                .record_trace_action(step_index);
            submit_trace(
                &mut app.world_mut().resource_mut::<PendingCommands>(),
                kind.clone(),
                step_index,
            );
            app.update();
            let scenario = app.world().resource::<ScenarioState>();
            assert_eq!(
                scenario.bindings[step_index]
                    .envelope
                    .as_ref()
                    .unwrap()
                    .kind,
                kind
            );
        }
        let ordinary_kinds = [
            PlayerCommandKind::MoveTo {
                unit_ids: Vec::new(),
                target: turts_net_protocol::Pos { x: 6.0, y: 7.0 },
            },
            PlayerCommandKind::MoveTo {
                unit_ids: vec![
                    turts_net_protocol::UnitId(3),
                    turts_net_protocol::UnitId(99),
                ],
                target: turts_net_protocol::Pos { x: 8.0, y: 9.0 },
            },
        ];
        for kind in ordinary_kinds.iter().cloned() {
            submit(&mut app.world_mut().resource_mut::<PendingCommands>(), kind);
        }
        app.update();

        let sent = &app.world().resource::<SentCommandCapture>().0;
        let expected_kinds = [
            PlayerCommandKind::MoveTo {
                unit_ids: Vec::new(),
                target: turts_net_protocol::Pos { x: 1.0, y: 2.0 },
            },
            PlayerCommandKind::MoveTo {
                unit_ids: vec![
                    turts_net_protocol::UnitId(3),
                    turts_net_protocol::UnitId(99),
                ],
                target: turts_net_protocol::Pos { x: 4.0, y: 5.0 },
            },
            ordinary_kinds[0].clone(),
            ordinary_kinds[1].clone(),
        ];
        assert_eq!(sent.len(), expected_kinds.len());
        for (index, (command, kind)) in sent.iter().zip(expected_kinds).enumerate() {
            assert_eq!(command.seq, index as u32 + 1);
            assert_eq!(command.kind, kind);
        }
        assert_eq!(app.world().resource::<CommandSequence>().next, Some(5));
        assert!(app.world().resource::<PendingCommands>().0.is_empty());
        assert!(app.world().resource::<ScenarioState>().error.is_none());
    }
}
