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
    expected_map: Option<turts_mod_api::ContentId>,
    /// True once this connection uses atomic transport. Legacy deltas cannot bypass it.
    pub restricted: bool,
    pub activated: u64,
    pub rejected: u64,
}

impl StateTransactions {
    fn ensure_context(&mut self, entity: Entity, stamp: &StateTransactionStamp) -> bool {
        if self.match_id != Some(stamp.basis.match_id)
            || self.expected_map.as_ref() != Some(&stamp.basis.map)
        {
            return false;
        }
        if self.connection != Some(entity) {
            self.assembler = Some(StateTransactionAssembler::new(
                stamp.basis.match_id,
                stamp.basis.map.clone(),
                KNOWLEDGE_FRAME_LIMIT_BYTES,
            ));
            self.connection = Some(entity);
        }
        true
    }

    pub fn retained_bytes(&self) -> usize {
        self.assembler
            .as_ref()
            .map_or(0, StateTransactionAssembler::retained_bytes)
    }
}

/// The server installs the first authored map from its locked root game package.
fn expected_map(
    archives: &[turts_mod_runtime::archive::PackageArchive],
    root: &str,
) -> Option<turts_mod_api::ContentId> {
    let archive = archives
        .iter()
        .find(|archive| archive.manifest.id == root)?;
    let maps = archive.maps().ok()?;
    let (_, map) = maps.first()?;
    Some(turts_mod_api::ContentId::new(&archive.id(), &map.id))
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct ActivationBookkeeping<'w> {
    hooks: ResMut<'w, crate::ingest::IngestHooks>,
    counters: ResMut<'w, crate::ingest::IngestCounters>,
    options: Res<'w, crate::ClientOpts>,
    witnesses: ResMut<'w, crate::ingest::ReceiptWitnesses>,
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
    mut book: ActivationBookkeeping,
    acquisition: Option<Res<crate::join::Acquisition>>,
    acquired: Option<Res<crate::join::AcquiredPackages>>,
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
    if transactions.expected_map.is_none()
        && let (Some(acquisition), Some(acquired)) = (acquisition, acquired)
        && let Some(root) = acquired.0.verified_root_mod.as_ref()
    {
        transactions.expected_map = expected_map(acquisition.accepted_archives(), &root.id);
    }
    let mut refused = Vec::new();
    for (entity, mut receiver) in &mut cancels {
        for cancel in receiver.receive() {
            if !current(entity) || !transactions.ensure_context(entity, &cancel.stamp) {
                continue;
            }
            // Control traffic can outrun the first ordered state frame.
            transactions.restricted = true;
            if let Some(assembler) = transactions.assembler.as_mut() {
                assembler.cancel(&cancel.stamp);
            }
        }
    }
    for (entity, mut receiver) in &mut begins {
        for begin in receiver.receive() {
            if !current(entity) {
                continue;
            }
            if !transactions.ensure_context(entity, &begin.stamp) {
                refused.push((entity, begin.stamp));
                continue;
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
                        book.options.0.raw_world_log.as_deref(),
                        "state_transaction",
                        &staged.payload,
                    );
                    let epoch = book.witnesses.epoch_for(entity.to_bits());
                    book.witnesses.record_message(epoch, true);
                    let valid = state.replica.as_mut().is_some_and(|replica| {
                        replica.activate_transaction(&staged.stamp, &staged.payload, Instant::now())
                    });
                    if valid {
                        match &staged.payload {
                            StateTransactionPayload::Snapshot { .. } => {
                                book.hooks.outstanding_request = false;
                                book.counters.snapshots_applied =
                                    book.counters.snapshots_applied.saturating_add(1);
                            }
                            StateTransactionPayload::Patch { .. } => {
                                book.counters.deltas_received =
                                    book.counters.deltas_received.saturating_add(1)
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
                                    let epoch = book.witnesses.epoch_for(target.to_bits());
                                    book.witnesses.record_message(epoch, true);
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

#[cfg(test)]
mod tests {
    use super::*;
    use turts_mod_api::{ContentId, KnowledgeBasis, ObservationPhase};
    use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

    #[test]
    fn first_wrong_map_begin_cannot_pin_the_assembler() {
        let archive = PackageArchive::parse(
            &pack_dir(
                &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../mods/visibility_probe"),
            )
            .unwrap(),
        )
        .unwrap();
        let map = expected_map(&[archive], "visibility_probe").unwrap();
        assert_eq!(map, ContentId::parse("visibility_probe:field").unwrap());
        let mut state = StateTransactions {
            match_id: Some(7),
            expected_map: Some(map.clone()),
            ..Default::default()
        };
        let mut stamp = StateTransactionStamp {
            basis: KnowledgeBasis {
                match_id: 7,
                map: ContentId::parse("wrong:first").unwrap(),
                tick: 0,
                phase: ObservationPhase::Completed,
            },
            transaction_id: 1,
            parent_transaction_id: None,
        };
        let entity = Entity::from_bits(17);
        assert!(!state.ensure_context(entity, &stamp));
        assert!(state.assembler.is_none());
        assert!(state.connection.is_none());
        assert_eq!(state.retained_bytes(), 0);
        stamp.basis.map = map;
        assert!(state.ensure_context(entity, &stamp));
        assert!(state.assembler.is_some());
    }

    #[test]
    fn unverified_or_missing_root_never_selects_another_package_map() {
        let archive = PackageArchive::parse(
            &pack_dir(
                &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/acme"),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(expected_map(&[archive], "visibility_probe").is_none());
    }
}
