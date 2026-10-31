//! Client-side player command FIFO and one checked sequence per fresh connection.

use crate::ingest::{CommandResults, ReceiptWitnesses};
use crate::join::Phase;
use crate::scenario::ScenarioState;
use bevy::prelude::*;
use lightyear::prelude::*;
use std::collections::VecDeque;
use turts_mod_api::Behaviour;
use turts_net_lightyear::ReliableChannel;
use turts_net_protocol::{
    JobId, MAX_ORDER_UNITS, NodeId, OrderEnvelope, OrderTarget, PlayerCommand, Pos, QueueMode,
    UnitId,
};

/// Client-side semantic adapter for existing gameplay actions. The wire uses mod-owned orders.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ClientCommand {
    MoveTo {
        unit_ids: Vec<UnitId>,
        target: Pos,
    },
    Gather {
        unit_id: UnitId,
        node_id: NodeId,
    },
    Produce {
        producer_id: UnitId,
        recipe_id: String,
    },
    CancelProduction {
        producer_id: UnitId,
        job_id: JobId,
    },
    Remove {
        unit_id: UnitId,
    },
    SetEnabled {
        unit_id: UnitId,
        enabled: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PendingCommand {
    Submit {
        kind: ClientCommand,
        trace_step: Option<usize>,
    },
    Order {
        order: OrderEnvelope,
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

    fn allocate_many(&mut self, count: usize) -> Result<u32, SequenceExhausted> {
        let first = self.next.ok_or(SequenceExhausted)?;
        let count = u32::try_from(count).map_err(|_| SequenceExhausted)?;
        let last_offset = count.checked_sub(1).ok_or(SequenceExhausted)?;
        let last = first.checked_add(last_offset).ok_or(SequenceExhausted)?;
        self.next = last.checked_add(1);
        Ok(first)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceExhausted;
#[derive(Debug, Default, Resource)]
pub struct CommandSendStatus {
    pub reconnect_required: bool,
}
pub fn submit(pending: &mut PendingCommands, kind: ClientCommand) {
    pending.0.push_back(PendingCommand::Submit {
        kind,
        trace_step: None,
    });
}

pub fn submit_order(pending: &mut PendingCommands, order: OrderEnvelope) {
    pending.0.push_back(PendingCommand::Order { order });
}

pub(crate) fn submit_trace(pending: &mut PendingCommands, kind: ClientCommand, trace_step: usize) {
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

/// Resolve the legacy client action through package order definitions before sending it.
pub(crate) fn order_envelope(
    content: &crate::content::ClientContent,
    command: ClientCommand,
) -> Option<OrderEnvelope> {
    let (behaviour, units, target) = match command {
        ClientCommand::MoveTo { unit_ids, target } => {
            (Behaviour::Move, unit_ids, OrderTarget::Point(target))
        }
        ClientCommand::Gather { unit_id, node_id } => {
            (Behaviour::Gather, vec![unit_id], OrderTarget::Node(node_id))
        }
        ClientCommand::Produce {
            producer_id,
            recipe_id,
        } => (
            Behaviour::Produce,
            vec![producer_id],
            OrderTarget::Content(recipe_id),
        ),
        ClientCommand::CancelProduction {
            producer_id,
            job_id,
        } => (
            Behaviour::CancelProduction,
            vec![producer_id],
            OrderTarget::Job(job_id),
        ),
        ClientCommand::Remove { unit_id } => (Behaviour::Remove, vec![unit_id], OrderTarget::None),
        ClientCommand::SetEnabled { unit_id, enabled } => (
            Behaviour::SetEnabled(enabled),
            vec![unit_id],
            OrderTarget::None,
        ),
    };
    let order = content
        .catalog
        .orders
        .iter()
        .find(|(_, definition)| definition.behaviour == behaviour)
        .map(|(id, _)| id.to_string())?;
    Some(OrderEnvelope {
        order,
        units,
        target,
        queue: QueueMode::Replace,
        group: 0,
    })
}

/// Build a browser Move envelope from the first catalog order with Move behaviour.
pub fn move_order_envelope(
    content: &crate::content::ClientContent,
    unit_ids: Vec<UnitId>,
    target: Pos,
) -> Option<OrderEnvelope> {
    order_envelope(content, ClientCommand::MoveTo { unit_ids, target })
}

fn split_order(order: OrderEnvelope, first_seq: u32) -> Vec<PlayerCommand> {
    if order.units.len() <= MAX_ORDER_UNITS {
        return vec![PlayerCommand {
            seq: first_seq,
            order,
        }];
    }
    let group = first_seq;
    order
        .units
        .chunks(MAX_ORDER_UNITS)
        .enumerate()
        .map(|(index, units)| {
            let mut fragment = order.clone();
            fragment.units = units.to_vec();
            fragment.group = group;
            PlayerCommand {
                seq: first_seq + index as u32,
                order: fragment,
            }
        })
        .collect()
}

/// Send the single shared FIFO only after a fresh session has joined. Ordinary adapters must
/// provide their intended literal bodies before enqueueing; trace steps use the same path.
#[allow(clippy::too_many_arguments)] // Bevy system resources stay explicit at this boundary.
pub(crate) fn send_commands_traced(
    mut pending: ResMut<PendingCommands>,
    mut sequence: ResMut<CommandSequence>,
    mut status: ResMut<CommandSendStatus>,
    phase: Res<Phase>,
    replica_state: Res<crate::ingest::ReplicaState>,
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
            let (envelopes, trace_step, submitted) = match command {
                PendingCommand::Submit { kind, trace_step } => {
                    let submitted = kind.clone();
                    let Some(content) = replica_state.content.as_ref() else {
                        pending
                            .0
                            .push_front(PendingCommand::Submit { kind, trace_step });
                        break;
                    };
                    let Some(order) = order_envelope(content, kind) else {
                        continue;
                    };
                    let count = order.units.len().max(1).div_ceil(MAX_ORDER_UNITS);
                    let seq = match sequence.allocate_many(count) {
                        Ok(seq) => seq,
                        Err(_) => {
                            status.reconnect_required = true;
                            pending.0.clear();
                            scenario.error = Some("command sequence exhausted".into());
                            break;
                        }
                    };
                    (split_order(order, seq), trace_step, Some(submitted))
                }
                PendingCommand::Order { order } => {
                    let count = order.units.len().max(1).div_ceil(MAX_ORDER_UNITS);
                    let seq = match sequence.allocate_many(count) {
                        Ok(seq) => seq,
                        Err(_) => {
                            status.reconnect_required = true;
                            pending.0.clear();
                            scenario.error = Some("command sequence exhausted".into());
                            break;
                        }
                    };
                    (split_order(order, seq), None, None)
                }
                PendingCommand::Repeat {
                    envelope,
                    trace_step,
                } => (vec![envelope], Some(trace_step), None),
            };
            let last_index = envelopes.len().saturating_sub(1);
            for (index, envelope) in envelopes.into_iter().enumerate() {
                sender.send::<ReliableChannel>(envelope.clone());
                #[cfg(test)]
                if let Some(capture) = sent_capture.as_deref_mut() {
                    capture.0.push(envelope.clone());
                }
                witnesses.record_message(epoch, true);
                if index == last_index
                    && let Some(step) = trace_step
                    && let Err(error) =
                        scenario.record_sent(step, envelope, submitted.clone(), results.0.len())
                {
                    scenario.error = Some(error);
                }
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
    use crate::ClientOpts;
    use crate::ingest::{IngestHooks, ReplicaState, WelcomeInfo};
    use crate::join::{Phase, auto_move};
    use crate::options::parse_cli;
    use crate::replica::{Replica, UnitReplica};
    use crate::scenario::{ScenarioState, TraceAction};
    use bevy::prelude::IntoScheduleConfigs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use turts_mod_api::{ContentId, OrderDef, TargetKind};
    use turts_net_protocol::{OrderTarget, QueueMode};

    fn move_test_content() -> crate::content::ClientContent {
        let mut content = crate::content::ClientContent::default();
        for (id, behaviour) in [
            ("acme:order_attack", Behaviour::Attack),
            ("acme:order_move", Behaviour::Move),
            ("acme:order_move_second", Behaviour::Move),
        ] {
            let parsed = ContentId::parse(id).expect("valid test order id");
            content.catalog.orders.insert(
                parsed.clone(),
                OrderDef {
                    id: parsed.local,
                    name: id.into(),
                    behaviour,
                    target: TargetKind::Point,
                    fire: None,
                    engage: None,
                    icon: "move".into(),
                    hotkey: "KeyM".into(),
                    cursor: None,
                    card: None,
                    units: vec![],
                    description: String::new(),
                },
            );
        }
        content
    }

    #[test]
    fn move_hook_sends_the_first_move_order() {
        use lightyear::prelude::*;

        let args = [
            "--server",
            "http://127.0.0.1:7000",
            "--ws",
            "ws://127.0.0.1:7001",
            "--move",
            "20,8",
            "--move-unit",
            "7",
            "--move-after-ticks",
            "5",
        ]
        .map(str::to_owned);
        let mut options = parse_cli(&args).expect("valid --move test options");
        options.headless = true;

        let mut replica = Replica::new(20);
        replica.snapshot_tick = Some(10);
        replica.units.insert(
            7,
            UnitReplica {
                content_id: "acme:trooper".into(),
                owner_slot: 0,
                facing: None,
                samples: Default::default(),
            },
        );
        let state = ReplicaState {
            replica: Some(replica),
            welcome: Some(WelcomeInfo {
                slot: 0,
                tick: 10,
                tick_hz: 20,
                match_id: 0,
            }),
            content: Some(move_test_content()),
            config: Default::default(),
        };
        let mut app = App::new();
        app.insert_resource(ClientOpts(options))
            .insert_resource(state)
            .insert_resource(IngestHooks {
                highest_delta_tick: Some(15),
                ..Default::default()
            })
            .insert_resource(PendingCommands::default())
            .insert_resource(CommandSequence::default())
            .insert_resource(CommandSendStatus::default())
            .insert_resource(Phase::Joined { slot: 0 })
            .insert_resource(ReceiptWitnesses::default())
            .insert_resource(CommandResults::default())
            .insert_resource(ScenarioState::default())
            .insert_resource(SentCommandCapture::default());
        app.add_systems(Update, (auto_move, send_commands_traced).chain());
        app.world_mut().spawn((
            MessageSender::<PlayerCommand>::default(),
            RemoteId(PeerId::Netcode(5)),
            Connected,
        ));

        app.update();

        let sent = &app.world().resource::<SentCommandCapture>().0;
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].order.order, "acme:order_move");
        assert_eq!(sent[0].order.units, vec![UnitId(7)]);
        assert_eq!(
            sent[0].order.target,
            OrderTarget::Point(Pos { x: 20.0, y: 8.0 })
        );
        assert_eq!(sent[0].order.queue, QueueMode::Replace);
    }

    #[test]
    fn oversized_order_splits_at_k4_with_one_group_and_sequential_ids() {
        let order = OrderEnvelope {
            order: "acme:order_move".into(),
            units: (0..MAX_ORDER_UNITS + 1)
                .map(|unit| UnitId(unit as u32))
                .collect(),
            target: OrderTarget::Point(Pos { x: 8.0, y: 9.0 }),
            queue: QueueMode::Append,
            group: 0,
        };
        let commands = split_order(order, 50);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].seq, 50);
        assert_eq!(commands[1].seq, 51);
        assert_eq!(commands[0].order.units.len(), MAX_ORDER_UNITS);
        assert_eq!(commands[1].order.units.len(), 1);
        assert_eq!(commands[0].order.group, 50);
        assert_eq!(commands[1].order.group, 50);
        assert_eq!(commands[0].order.units.first(), Some(&UnitId(0)));
        assert_eq!(
            commands[0].order.units.last(),
            Some(&UnitId(MAX_ORDER_UNITS as u32 - 1))
        );
        assert_eq!(
            commands[1].order.units,
            vec![UnitId(MAX_ORDER_UNITS as u32)]
        );
    }

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
            .insert_resource(ReplicaState {
                content: Some(move_test_content()),
                ..Default::default()
            })
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
                    .order
                    .order,
                "acme:order_move"
            );
        }
        let ordinary_kinds = [
            ClientCommand::MoveTo {
                unit_ids: Vec::new(),
                target: turts_net_protocol::Pos { x: 6.0, y: 7.0 },
            },
            ClientCommand::MoveTo {
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
            ClientCommand::MoveTo {
                unit_ids: Vec::new(),
                target: turts_net_protocol::Pos { x: 1.0, y: 2.0 },
            },
            ClientCommand::MoveTo {
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
            assert_eq!(command.order.order, "acme:order_move");
            let ClientCommand::MoveTo { unit_ids, target } = kind else {
                panic!("expected MoveTo test command");
            };
            assert_eq!(command.order.units, unit_ids);
            assert_eq!(command.order.target, OrderTarget::Point(target));
            assert_eq!(command.order.queue, QueueMode::Replace);
        }
        assert_eq!(app.world().resource::<CommandSequence>().next, Some(5));
        assert!(app.world().resource::<PendingCommands>().0.is_empty());
        assert!(app.world().resource::<ScenarioState>().error.is_none());
    }
}
