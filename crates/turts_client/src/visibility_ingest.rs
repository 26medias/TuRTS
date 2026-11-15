//! Ordered state transport adapter. Acknowledgement follows whole-view activation.
use bevy::platform::time::Instant;
use bevy::prelude::*;
use lightyear::prelude::*;
use turts_net_lightyear::{KNOWLEDGE_FRAME_LIMIT_BYTES, ReliableChannel};
use turts_net_protocol::{
    visibility::*,
    visibility_transaction::{
        StagedStateTransaction, StateTransactionAssembler, StateTransactionFrame,
    },
};

use crate::ingest::ReplicaState;

#[derive(Resource, Default)]
pub struct StateTransactions {
    assembler: Option<StateTransactionAssembler>,
    connection: Option<Entity>,
    match_id: Option<u64>,
    expected_map: Option<turts_mod_api::ContentId>,
    early: std::collections::VecDeque<(Entity, StateTransactionFrame, usize)>,
    early_heap: usize,
    early_entity: Option<Entity>,
    early_retirement: Option<StateTransactionStamp>,
    early_cancel: Option<(Entity, StateTransactionStamp)>,
    restart_after_welcome: bool,
    waiting_snapshot: bool,
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

    fn cancellation(&mut self, entity: Entity, stamp: StateTransactionStamp) {
        if self.match_id.is_some_and(|id| id != stamp.basis.match_id)
            || self
                .expected_map
                .as_ref()
                .is_some_and(|map| map != &stamp.basis.map)
        {
            return;
        }
        if self.ensure_context(entity, &stamp) {
            self.restricted = true;
            if let Some(assembler) = self.assembler.as_mut() {
                assembler.cancel(&stamp);
            }
            return;
        }
        // Exactly one bounded watermark is retained until Welcome supplies the match/slot.
        let map_bytes = stamp
            .basis
            .map
            .package
            .0
            .capacity()
            .saturating_add(stamp.basis.map.local.capacity());
        if map_bytes.saturating_add(std::mem::size_of::<StateTransactionStamp>()) > 4096 {
            return;
        }
        let replace = self.early_cancel.as_ref().is_none_or(|(old_entity, old)| {
            *old_entity != entity
                || old.basis.match_id != stamp.basis.match_id
                || old.basis.map != stamp.basis.map
                || old.transaction_id < stamp.transaction_id
        });
        if replace {
            self.early_cancel = Some((entity, stamp));
        }
    }
    fn apply_early_cancellation(&mut self) {
        if let Some((entity, stamp)) = self.early_cancel.take()
            && self.ensure_context(entity, &stamp)
        {
            self.restricted = true;
            if let Some(assembler) = self.assembler.as_mut() {
                assembler.cancel(&stamp);
            }
        }
    }

    fn clear_early(&mut self) {
        self.early = std::collections::VecDeque::new();
        self.early_heap = 0;
        self.early_entity = None;
    }
    fn early_bytes(&self) -> usize {
        self.early_heap
            .saturating_add(self.early_cancel.as_ref().map_or(0, |(_, stamp)| {
                stamp
                    .basis
                    .map
                    .package
                    .0
                    .capacity()
                    .saturating_add(stamp.basis.map.local.capacity())
            }))
            .saturating_add(self.early.capacity().saturating_mul(std::mem::size_of::<(
                Entity,
                StateTransactionFrame,
                usize,
            )>()))
    }
    fn retire_early(&mut self, entity: Entity, stamp: StateTransactionStamp) {
        for (_, frame, _) in &self.early {
            let candidate = frame_stamp(frame);
            if self
                .early_retirement
                .as_ref()
                .is_none_or(|old| old.transaction_id < candidate.transaction_id)
            {
                self.early_retirement = Some(candidate.clone());
            }
        }
        if self
            .early_retirement
            .as_ref()
            .is_none_or(|old| old.transaction_id < stamp.transaction_id)
        {
            self.early_retirement = Some(stamp);
        }
        self.clear_early();
        self.early_entity = Some(entity);
        self.restart_after_welcome = true;
        self.waiting_snapshot = true;
    }
    fn defer(&mut self, entity: Entity, frame: StateTransactionFrame) {
        if self.early_entity.is_some_and(|old| old != entity) {
            self.clear_early();
            self.early_retirement = None;
            if self
                .early_cancel
                .as_ref()
                .is_some_and(|(old_entity, _)| *old_entity != entity)
            {
                self.early_cancel = None;
            }
            self.restart_after_welcome = false;
            self.waiting_snapshot = false;
        }
        self.early_entity = Some(entity);
        let stamp = frame_stamp(&frame);
        if self.restart_after_welcome {
            self.retire_early(entity, stamp.clone());
            return;
        }
        let heap = frame_heap(&frame);
        let capacity = if self.early.len() == self.early.capacity() {
            self.early.capacity().saturating_mul(2).max(4)
        } else {
            self.early.capacity()
        };
        let required = self
            .early_heap
            .saturating_add(heap)
            .saturating_add(capacity.saturating_mul(std::mem::size_of::<(
                Entity,
                StateTransactionFrame,
                usize,
            )>()))
            .saturating_add(4096);
        if frame
            .encoded_bytes()
            .map_or(true, |size| size > KNOWLEDGE_FRAME_LIMIT_BYTES)
            || required > MAX_POST_BASE_PATCH_BYTES as usize
        {
            self.retire_early(entity, stamp.clone());
            return;
        }
        if self.early.len() == self.early.capacity()
            && self
                .early
                .try_reserve_exact(capacity - self.early.len())
                .is_err()
        {
            self.retire_early(entity, stamp.clone());
            return;
        }
        self.early_heap += heap;
        self.early.push_back((entity, frame, heap));
    }
    fn pop_early(&mut self) -> Option<(Entity, StateTransactionFrame)> {
        let (entity, frame, heap) = self.early.pop_front()?;
        self.early_heap -= heap;
        if self.early.is_empty() {
            self.clear_early();
        }
        Some((entity, frame))
    }

    fn frame(
        &mut self,
        entity: Entity,
        frame: StateTransactionFrame,
        now_ms: u64,
    ) -> Result<Option<StagedStateTransaction>, StateTransactionStamp> {
        let stamp = frame_stamp(&frame).clone();
        if !self.ensure_context(entity, &stamp) {
            return Err(stamp);
        }
        self.restricted = true;
        if self
            .assembler
            .as_ref()
            .is_some_and(|assembler| assembler.is_obsolete(&stamp))
        {
            return Ok(None);
        }
        let reserved = self.early_bytes();
        if self.waiting_snapshot {
            let fresh = matches!(&frame, StateTransactionFrame::Begin(begin) if begin.kind == StateTransactionKind::Snapshot)
                && self
                    .assembler
                    .as_ref()
                    .is_some_and(|assembler| !assembler.is_obsolete(&stamp));
            if !fresh {
                if let Some(assembler) = self.assembler.as_mut() {
                    assembler.cancel(&stamp);
                }
                return Ok(None);
            }
            self.waiting_snapshot = false;
        }
        if let StateTransactionFrame::Begin(begin) = &frame {
            let required = self
                .assembler
                .as_ref()
                .and_then(|assembler| assembler.required_staging_bytes(begin).ok());
            if required.is_some_and(|bytes| {
                bytes.saturating_add(reserved) > MAX_PENDING_STATE_BYTES as usize
            }) {
                self.retire_early(entity, stamp.clone());
                if let Some(assembler) = self.assembler.as_mut() {
                    assembler.cancel(&stamp);
                }
                return Ok(None);
            }
        }
        let Some(assembler) = self.assembler.as_mut() else {
            return Err(stamp);
        };
        let result = match frame {
            StateTransactionFrame::Begin(value) => assembler.begin(value, now_ms).map(|_| None),
            StateTransactionFrame::Chunk(value) => assembler.chunk(&value).map(|_| None),
            StateTransactionFrame::End(value) => assembler.end_with_budget(&value, reserved),
        };
        result.map_err(|_| {
            assembler.cancel(&stamp);
            stamp
        })
    }

    pub fn retained_bytes(&self) -> usize {
        self.early_bytes().saturating_add(
            self.assembler
                .as_ref()
                .map_or(0, StateTransactionAssembler::retained_bytes),
        )
    }
}

fn frame_stamp(frame: &StateTransactionFrame) -> &StateTransactionStamp {
    match frame {
        StateTransactionFrame::Begin(value) => &value.stamp,
        StateTransactionFrame::Chunk(value) => &value.stamp,
        StateTransactionFrame::End(value) => &value.stamp,
    }
}
fn frame_heap(frame: &StateTransactionFrame) -> usize {
    let map = &frame_stamp(frame).basis.map;
    let payload = match frame {
        StateTransactionFrame::Chunk(value) => value.bytes.capacity(),
        _ => 0,
    };
    payload
        .saturating_add(map.package.0.capacity())
        .saturating_add(map.local.capacity())
        .saturating_add(64)
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
    let [(_, map)] = maps.as_slice() else {
        return None;
    };
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
    mut frames: Query<(Entity, &mut MessageReceiver<StateTransactionFrame>)>,
    mut cancels: Query<(Entity, &mut MessageReceiver<StateTransactionCancel>)>,
    mut baseline_requests: Query<(
        Entity,
        &mut MessageSender<turts_net_protocol::ResyncRequest>,
    )>,
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
        if transactions.match_id.is_none() && welcome.is_some() {
            // Preserve bounded frames that arrived before the reliable Welcome.
            transactions.match_id = welcome.map(|w| w.match_id);
        } else {
            *transactions = StateTransactions::default();
            transactions.match_id = welcome.map(|w| w.match_id);
        }
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
            if current(entity) {
                transactions.cancellation(entity, cancel.stamp);
            }
        }
    }
    let ready = welcome.is_some() && transactions.expected_map.is_some();
    if transactions
        .early_entity
        .is_some_and(|entity| !current(entity))
    {
        transactions.clear_early();
        transactions.early_retirement = None;
        transactions.restart_after_welcome = false;
        transactions.waiting_snapshot = false;
    }
    if transactions
        .early_cancel
        .as_ref()
        .is_some_and(|(entity, _)| !current(*entity))
    {
        transactions.early_cancel = None;
    }
    if ready {
        transactions.apply_early_cancellation();
    }
    if ready
        && let Some(stamp) = transactions.early_retirement.take()
        && let Some(entity) = transactions.early_entity
        && transactions.ensure_context(entity, &stamp)
        && let Some(assembler) = transactions.assembler.as_mut()
    {
        assembler.cancel(&stamp);
    }
    {
        let mut consume =
            |transactions: &mut StateTransactions, entity: Entity, frame: StateTransactionFrame| {
                match transactions.frame(entity, frame, now_ms) {
                    Ok(Some(staged)) => {
                        crate::ingest::append_raw_world_dto(
                            book.options.0.raw_world_log.as_deref(),
                            "state_transaction",
                            &staged.payload,
                        );
                        let epoch = book.witnesses.epoch_for(entity.to_bits());
                        book.witnesses.record_message(epoch, true);
                        let valid = state.replica.as_mut().is_some_and(|replica| {
                            replica.activate_transaction_reserved(
                                &staged.stamp,
                                &staged.payload,
                                Instant::now(),
                                transactions.early_bytes(),
                            )
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
                            if let Some(assembler) = transactions.assembler.as_mut() {
                                assembler.cancel(&staged.stamp);
                            }
                            refused.push((entity, staged.stamp));
                        }
                    }
                    Ok(None) => {}
                    Err(stamp) => refused.push((entity, stamp)),
                }
            };
        if ready {
            while let Some((entity, frame)) = transactions.pop_early() {
                if current(entity) {
                    consume(&mut transactions, entity, frame);
                }
            }
        }
        for (entity, mut receiver) in &mut frames {
            for frame in receiver.receive() {
                if !current(entity) {
                    continue;
                }
                if ready {
                    consume(&mut transactions, entity, frame);
                } else {
                    transactions.defer(entity, frame);
                }
            }
        }
    }
    if ready && transactions.restart_after_welcome {
        for (entity, mut sender) in &mut baseline_requests {
            if current(entity) {
                sender.send::<ReliableChannel>(turts_net_protocol::ResyncRequest);
                book.hooks.outstanding_request = true;
                book.counters.resync_requests_sent =
                    book.counters.resync_requests_sent.saturating_add(1);
                book.counters.resync_by_validation =
                    book.counters.resync_by_validation.saturating_add(1);
            }
        }
        transactions.restart_after_welcome = false;
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

    #[test]
    fn two_complete_transactions_in_one_application_batch_activate_in_wire_order() {
        use turts_net_protocol::visibility_transaction::encode_transaction;
        use turts_net_protocol::{Snapshot, WorldDelta};
        let archive = PackageArchive::parse(
            &pack_dir(
                &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../mods/visibility_probe"),
            )
            .unwrap(),
        )
        .unwrap();
        let map = expected_map(std::slice::from_ref(&archive), "visibility_probe").unwrap();
        let mut replica = crate::replica::Replica::new(20);
        replica.set_context(crate::replica::ReplicaContext::new(
            crate::content::ClientContent::from_archives(&[archive]).unwrap(),
            0,
        ));
        let basis = KnowledgeBasis {
            match_id: 7,
            map: map.clone(),
            tick: 1,
            phase: ObservationPhase::Completed,
        };
        let first = StateTransactionStamp {
            basis: basis.clone(),
            transaction_id: 1,
            parent_transaction_id: None,
        };
        let second = StateTransactionStamp {
            basis: KnowledgeBasis { tick: 2, ..basis },
            transaction_id: 2,
            parent_transaction_id: Some(1),
        };
        let payloads = [
            (
                first,
                StateTransactionPayload::Snapshot {
                    state: Box::new(Snapshot {
                        tick: 1,
                        ..Default::default()
                    }),
                    knowledge: None,
                    event_units: vec![],
                },
            ),
            (
                second,
                StateTransactionPayload::Patch {
                    state: Box::new(WorldDelta {
                        tick: 2,
                        base_tick: 1,
                        ..Default::default()
                    }),
                    knowledge: None,
                    event_units: vec![],
                },
            ),
        ];
        let mut batch = Vec::new();
        for (stamp, payload) in payloads {
            let mut encoded =
                encode_transaction(stamp, &payload, KNOWLEDGE_FRAME_LIMIT_BYTES).unwrap();
            while let Some(frame) = encoded.next_frame() {
                batch.push(frame);
            }
        }
        let mut transactions = StateTransactions {
            match_id: Some(7),
            expected_map: Some(map),
            ..Default::default()
        };
        let entity = Entity::from_bits(17);
        let mut activated = Vec::new();
        // One receiver's ordered batch is consumed directly. No frame-kind regrouping is allowed.
        for frame in batch {
            if let Some(staged) = transactions.frame(entity, frame, 0).unwrap() {
                assert!(replica.activate_transaction(
                    &staged.stamp,
                    &staged.payload,
                    Instant::now()
                ));
                let ack = transactions
                    .assembler
                    .as_mut()
                    .unwrap()
                    .activate(&staged.stamp)
                    .unwrap();
                activated.push(ack.stamp.transaction_id);
            }
        }
        assert_eq!(activated, vec![1, 2]);
        assert_eq!(replica.last_applied_tick, Some(2));
        assert_eq!(transactions.retained_bytes(), 0);
    }

    #[test]
    fn state_before_or_after_welcome_activates_the_same_complete_view() {
        use turts_net_protocol::visibility_transaction::encode_transaction;
        let mut outcomes = Vec::new();
        for welcome_first in [true, false] {
            let archive = PackageArchive::parse(
                &pack_dir(
                    &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../../mods/visibility_probe"),
                )
                .unwrap(),
            )
            .unwrap();
            let map = expected_map(std::slice::from_ref(&archive), "visibility_probe").unwrap();
            let stamp = StateTransactionStamp {
                basis: KnowledgeBasis {
                    match_id: 7,
                    map: map.clone(),
                    tick: 1,
                    phase: ObservationPhase::Completed,
                },
                transaction_id: 1,
                parent_transaction_id: None,
            };
            let payload = StateTransactionPayload::Snapshot {
                state: Box::new(turts_net_protocol::Snapshot {
                    tick: 1,
                    ..Default::default()
                }),
                knowledge: None,
                event_units: vec![],
            };
            let mut encoded =
                encode_transaction(stamp, &payload, KNOWLEDGE_FRAME_LIMIT_BYTES).unwrap();
            let entity = Entity::from_bits(17);
            let mut transactions = StateTransactions {
                expected_map: Some(map),
                ..Default::default()
            };
            let mut replica: Option<crate::replica::Replica> = None;
            if !welcome_first {
                while let Some(frame) = encoded.next_frame() {
                    transactions.defer(entity, frame);
                }
                assert!(replica.is_none());
                assert!(transactions.assembler.is_none());
                assert!(transactions.retained_bytes() > 0);
                assert!(transactions.retained_bytes() <= MAX_POST_BASE_PATCH_BYTES as usize);
            }
            // Only the actual Welcome supplies match/slot and creates a replica context.
            transactions.match_id = Some(7);
            let mut joined = crate::replica::Replica::new(20);
            joined.set_context(crate::replica::ReplicaContext::new(
                crate::content::ClientContent::from_archives(&[archive]).unwrap(),
                0,
            ));
            replica = Some(joined);
            let mut acked = Vec::new();
            loop {
                let next = if welcome_first {
                    encoded.next_frame().map(|frame| (entity, frame))
                } else {
                    transactions.pop_early()
                };
                let Some((entity, frame)) = next else {
                    break;
                };
                if let Some(staged) = transactions.frame(entity, frame, 0).unwrap() {
                    assert!(replica.as_mut().unwrap().activate_transaction_reserved(
                        &staged.stamp,
                        &staged.payload,
                        Instant::now(),
                        transactions.early_bytes()
                    ));
                    acked.push(
                        transactions
                            .assembler
                            .as_mut()
                            .unwrap()
                            .activate(&staged.stamp)
                            .unwrap()
                            .stamp
                            .transaction_id,
                    );
                }
            }
            outcomes.push((replica.unwrap().last_applied_tick, acked));
            assert_eq!(transactions.retained_bytes(), 0);
        }
        assert_eq!(outcomes[0], (Some(1), vec![1]));
        assert_eq!(outcomes[0], outcomes[1]);
    }

    #[test]
    fn early_frame_capacity_overflow_waits_for_one_real_context_resync() {
        let stamp = StateTransactionStamp {
            basis: KnowledgeBasis {
                match_id: 7,
                map: ContentId::parse("visibility_probe:field").unwrap(),
                tick: 1,
                phase: ObservationPhase::Completed,
            },
            transaction_id: 1,
            parent_transaction_id: None,
        };
        let frame = StateTransactionFrame::Chunk(StateTransactionChunk {
            stamp,
            index: 0,
            bytes: Vec::with_capacity(MAX_POST_BASE_PATCH_BYTES as usize + 1),
        });
        let mut transactions = StateTransactions::default();
        transactions.defer(Entity::from_bits(17), frame);
        assert!(transactions.restart_after_welcome);
        assert!(transactions.waiting_snapshot);
        assert!(transactions.assembler.is_none());
        assert_eq!(transactions.retained_bytes(), 0);
    }

    #[test]
    fn cancellation_before_welcome_retires_early_and_late_frames_before_replacement() {
        use turts_net_protocol::visibility_transaction::encode_transaction;
        let archive = PackageArchive::parse(
            &pack_dir(
                &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../mods/visibility_probe"),
            )
            .unwrap(),
        )
        .unwrap();
        let map = expected_map(std::slice::from_ref(&archive), "visibility_probe").unwrap();
        let stamp = |id| StateTransactionStamp {
            basis: KnowledgeBasis {
                match_id: 7,
                map: map.clone(),
                tick: id as u32,
                phase: ObservationPhase::Completed,
            },
            transaction_id: id,
            parent_transaction_id: None,
        };
        let frames = |id| {
            let payload = StateTransactionPayload::Snapshot {
                state: Box::new(turts_net_protocol::Snapshot {
                    tick: id as u32,
                    ..Default::default()
                }),
                knowledge: None,
                event_units: vec![],
            };
            let mut encoded =
                encode_transaction(stamp(id), &payload, KNOWLEDGE_FRAME_LIMIT_BYTES).unwrap();
            let mut frames = Vec::new();
            while let Some(frame) = encoded.next_frame() {
                frames.push(frame);
            }
            frames
        };
        let entity = Entity::from_bits(17);
        let mut transactions = StateTransactions::default();
        transactions.cancellation(entity, stamp(3));
        assert!(transactions.assembler.is_none());
        assert!(transactions.early_cancel.is_some());
        let old_frames = (1..=3).flat_map(frames).collect::<Vec<_>>();
        for frame in &old_frames {
            transactions.defer(entity, frame.clone());
        }
        // A real Welcome and its verified root map now establish the connection context.
        transactions.match_id = Some(7);
        transactions.expected_map = Some(map.clone());
        transactions.apply_early_cancellation();
        assert!(transactions.early_cancel.is_none());
        while let Some((entity, frame)) = transactions.pop_early() {
            assert!(transactions.frame(entity, frame, 0).unwrap().is_none());
        }
        for frame in old_frames {
            assert!(transactions.frame(entity, frame, 0).unwrap().is_none());
        }
        let mut replica = crate::replica::Replica::new(20);
        replica.set_context(crate::replica::ReplicaContext::new(
            crate::content::ClientContent::from_archives(&[archive]).unwrap(),
            0,
        ));
        let mut activated = Vec::new();
        for frame in frames(4) {
            if let Some(staged) = transactions.frame(entity, frame, 0).unwrap() {
                assert!(replica.activate_transaction(
                    &staged.stamp,
                    &staged.payload,
                    Instant::now()
                ));
                activated.push(
                    transactions
                        .assembler
                        .as_mut()
                        .unwrap()
                        .activate(&staged.stamp)
                        .unwrap()
                        .stamp
                        .transaction_id,
                );
            }
        }
        assert_eq!(activated, vec![4]);
        assert_eq!(replica.last_applied_tick, Some(4));
        assert!(!transactions.restart_after_welcome);
    }
}
