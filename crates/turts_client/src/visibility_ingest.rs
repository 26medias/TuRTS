//! Ordered state transport adapter. Acknowledgement follows whole-view activation.
use bevy::platform::time::Instant;
use bevy::prelude::*;
use lightyear::prelude::*;
use turts_net_lightyear::{KNOWLEDGE_FRAME_LIMIT_BYTES, ReliableChannel};
use turts_net_protocol::{visibility::*, visibility_transaction::StateTransactionAssembler};

use crate::ingest::ReplicaState;

#[derive(Resource, Default)]
pub struct StateTransactions {
    assembler: Option<StateTransactionAssembler>,
    connection: Option<Entity>,
    match_id: Option<u64>,
    /// True once this connection uses atomic transport. Legacy deltas cannot bypass it.
    pub restricted: bool,
    pub activated: u64,
    pub rejected: u64,
}

impl StateTransactions {
    pub fn retained_bytes(&self) -> usize {
        self.assembler
            .as_ref()
            .map_or(0, StateTransactionAssembler::retained_bytes)
    }
}

/// Split from ordinary receipt ingest so command results keep their own bounded path.
#[allow(clippy::too_many_arguments)]
pub fn receive_transactions(
    mut begins: Query<(Entity, &mut MessageReceiver<StateTransactionBegin>)>,
    mut chunks: Query<(Entity, &mut MessageReceiver<StateTransactionChunk>)>,
    mut ends: Query<(Entity, &mut MessageReceiver<StateTransactionEnd>)>,
    mut cancels: Query<(Entity, &mut MessageReceiver<StateTransactionCancel>)>,
    mut acknowledgements: Query<(Entity, &mut MessageSender<StateTransactionAck>)>,
    mut cancellation_senders: Query<(Entity, &mut MessageSender<StateTransactionCancel>)>,
    mut transactions: ResMut<StateTransactions>,
    mut state: ResMut<ReplicaState>,
    mut selection: Option<ResMut<crate::ui_shared::Selection>>,
    mut presented: ResMut<crate::present::Presented>,
    mut hooks: ResMut<crate::ingest::IngestHooks>,
    mut counters: ResMut<crate::ingest::IngestCounters>,
    options: Res<crate::ClientOpts>,
    mut witnesses: ResMut<crate::ingest::ReceiptWitnesses>,
    attempt: Option<Res<crate::rejoin::RejoinController>>,
    time: Res<Time<Real>>,
) {
    let now_ms = time.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let current = |entity: Entity| {
        super::ingest::is_current_attempt_at(attempt.as_deref(), entity, time.elapsed_secs_f64())
    };
    let welcome = state.welcome;
    if transactions.match_id != welcome.map(|w| w.match_id) {
        *transactions = StateTransactions::default();
        transactions.match_id = welcome.map(|w| w.match_id);
    }
    let mut refused = Vec::new();
    for (entity, mut receiver) in &mut cancels {
        for cancel in receiver.receive() {
            if !current(entity) || welcome.is_none_or(|w| w.match_id != cancel.stamp.basis.match_id)
            {
                continue;
            }
            // Control traffic can outrun the first ordered state frame.
            // Establish the single map context before retiring that stamp.
            if transactions.connection != Some(entity) {
                transactions.assembler = Some(StateTransactionAssembler::new(
                    cancel.stamp.basis.match_id,
                    cancel.stamp.basis.map.clone(),
                    KNOWLEDGE_FRAME_LIMIT_BYTES,
                ));
                transactions.connection = Some(entity);
            }
            transactions.restricted = true;
            if let Some(assembler) = transactions.assembler.as_mut() {
                assembler.cancel(&cancel.stamp);
            }
        }
    }
    for (entity, mut receiver) in &mut begins {
        for begin in receiver.receive() {
            if !current(entity) || welcome.is_none_or(|w| w.match_id != begin.stamp.basis.match_id)
            {
                continue;
            }
            if transactions.connection != Some(entity) {
                transactions.assembler = Some(StateTransactionAssembler::new(
                    begin.stamp.basis.match_id,
                    begin.stamp.basis.map.clone(),
                    KNOWLEDGE_FRAME_LIMIT_BYTES,
                ));
                transactions.connection = Some(entity);
            }
            transactions.restricted = true;
            if transactions
                .assembler
                .as_ref()
                .is_some_and(|a| a.is_obsolete(&begin.stamp))
            {
                continue;
            }
            if transactions
                .assembler
                .as_mut()
                .is_none_or(|a| a.begin(begin.clone(), now_ms).is_err())
            {
                refused.push((entity, begin.stamp));
            }
        }
    }
    for (entity, mut receiver) in &mut chunks {
        for chunk in receiver.receive() {
            if !current(entity) || transactions.connection != Some(entity) {
                continue;
            }
            if transactions
                .assembler
                .as_ref()
                .is_some_and(|a| a.is_obsolete(&chunk.stamp))
            {
                continue;
            }
            if transactions
                .assembler
                .as_mut()
                .is_none_or(|a| a.chunk(&chunk).is_err())
            {
                refused.push((entity, chunk.stamp));
            }
        }
    }
    for (entity, mut receiver) in &mut ends {
        for end in receiver.receive() {
            if !current(entity) || transactions.connection != Some(entity) {
                continue;
            }
            if transactions
                .assembler
                .as_ref()
                .is_some_and(|a| a.is_obsolete(&end.stamp))
            {
                continue;
            }
            let staged = transactions.assembler.as_mut().map(|a| a.end(&end));
            match staged {
                Some(Ok(Some(staged))) => {
                    crate::ingest::append_raw_world_dto(
                        options.0.raw_world_log.as_deref(),
                        "state_transaction",
                        &staged.payload,
                    );
                    let epoch = witnesses.epoch_for(entity.to_bits());
                    witnesses.record_message(epoch, true);
                    let valid = state.replica.as_mut().is_some_and(|replica| {
                        replica.activate_transaction(&staged.stamp, &staged.payload, Instant::now())
                    });
                    if valid {
                        match &staged.payload {
                            StateTransactionPayload::Snapshot { .. } => {
                                hooks.outstanding_request = false;
                                counters.snapshots_applied =
                                    counters.snapshots_applied.saturating_add(1);
                            }
                            StateTransactionPayload::Patch { .. } => {
                                counters.deltas_received =
                                    counters.deltas_received.saturating_add(1)
                            }
                        }
                        // Remove old interpolation and selection in this same schedule step.
                        if let Some(replica) = state.replica.as_ref() {
                            presented
                                .units
                                .retain(|unit| replica.units.contains_key(&unit.unit_id));
                            if let Some(selection) = selection.as_mut()
                                && selection
                                    .0
                                    .is_some_and(|id| !replica.units.contains_key(&id))
                            {
                                selection.0 = None;
                            }
                        }
                        if let Some(Ok(ack)) = transactions
                            .assembler
                            .as_mut()
                            .map(|a| a.activate(&staged.stamp))
                        {
                            for (target, mut sender) in &mut acknowledgements {
                                if target == entity {
                                    sender.send::<ReliableChannel>(ack.clone());
                                    let epoch = witnesses.epoch_for(target.to_bits());
                                    witnesses.record_message(epoch, true);
                                }
                            }
                            transactions.activated += 1;
                        }
                    } else {
                        refused.push((entity, staged.stamp));
                    }
                }
                Some(Ok(None)) => {}
                _ => refused.push((entity, end.stamp)),
            }
        }
    }
    if let Some(stamp) = transactions
        .assembler
        .as_mut()
        .and_then(|a| a.expire(now_ms))
        && let Some(entity) = transactions.connection
    {
        for (target, mut sender) in &mut cancellation_senders {
            if target == entity {
                sender.send::<ReliableChannel>(StateTransactionCancel {
                    stamp: stamp.clone(),
                    reason: StateTransactionCancelReason::TimedOut,
                });
            }
        }
    }
    for (entity, stamp) in refused {
        transactions.rejected += 1;
        if let Some(assembler) = transactions.assembler.as_mut() {
            assembler.cancel(&stamp);
        }
        for (target, mut sender) in &mut cancellation_senders {
            if target == entity {
                sender.send::<ReliableChannel>(StateTransactionCancel {
                    stamp: stamp.clone(),
                    reason: StateTransactionCancelReason::InvalidTransaction,
                });
            }
        }
    }
}
