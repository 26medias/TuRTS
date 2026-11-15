//! Connection-local, bounded atomic publication. Simulation never waits for a viewer.
use crate::{
    export::PermittedBaseline,
    session::{ReliableTraffic, Session},
};
use bevy::prelude::*;
use lightyear::prelude::*;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
use turts_net_lightyear::{
    KNOWLEDGE_FRAME_LIMIT_BYTES as FRAME_LIMIT, KnowledgeChannel, ReliableChannel,
};
use turts_net_protocol::{
    MAX_PENDING_STATE_BYTES, MAX_POST_BASE_PATCH_BYTES, MAX_STATE_RESTARTS,
    STATE_RESTART_WINDOW_MS, STATE_TRANSACTION_TIMEOUT_MS, StateTransactionAck,
    StateTransactionCancel, StateTransactionCancelReason, StateTransactionStamp,
    visibility_transaction::{
        EncodedStateTransaction, StateTransactionFrame, encode_snapshot, encode_transaction,
        encoded_snapshot_size,
    },
};
use turts_sim::SlotId;
const WINDOW: usize = 64;
const FRAMES_PER_PUMP: usize = 16;
// Locked bridge cap, plus packet/retry bookkeeping and serialization workspace. Do not borrow it.
const BRIDGE_RESERVE: usize = 4 * 1024 * 1024;
const TRANSPORT_RESERVE: usize = 4 * 1024 * 1024;
const COPY_RESERVE_PER_FRAME: usize = 8192;
struct Pending {
    encoded: EncodedStateTransaction,
    deferred: Option<StateTransactionFrame>,
    started: Instant,
    end_sent: bool,
}
impl Pending {
    fn new(encoded: EncodedStateTransaction, now: Instant) -> Self {
        Self {
            encoded,
            deferred: None,
            started: now,
            end_sent: false,
        }
    }
}
#[derive(Component)]
pub struct KnowledgeSender {
    tip: PermittedBaseline,
    context: turts_mod_api::KnowledgeBasis,
    active: Option<StateTransactionStamp>,
    pending: Option<Pending>,
    patches: VecDeque<EncodedStateTransaction>,
    patch_bytes: usize,
    next_id: u64,
    outstanding: usize,
    restarts: VecDeque<Instant>,
}
impl KnowledgeSender {
    #[cfg(test)]
    pub(crate) fn active_stamp(&self) -> Option<&StateTransactionStamp> {
        self.active.as_ref()
    }
    fn retained(&self) -> usize {
        let map_heap = |basis: &turts_mod_api::KnowledgeBasis| {
            basis
                .map
                .package
                .0
                .capacity()
                .saturating_add(basis.map.local.capacity())
        };
        self.tip
            .retained_bytes()
            .saturating_add(std::mem::size_of::<Self>())
            .saturating_add(map_heap(&self.context))
            .saturating_add(
                self.active
                    .as_ref()
                    .map_or(0, |stamp| map_heap(&stamp.basis)),
            )
            .saturating_add(
                self.restarts
                    .capacity()
                    .saturating_mul(std::mem::size_of::<Instant>()),
            )
            .saturating_add(
                self.patches
                    .capacity()
                    .saturating_mul(std::mem::size_of::<EncodedStateTransaction>()),
            )
            .saturating_add(
                self.pending
                    .as_ref()
                    .map_or(0, |p| p.encoded.retained_bytes() + 2 * FRAME_LIMIT),
            )
            .saturating_add(self.patch_bytes)
            .saturating_add(self.outstanding.saturating_mul(COPY_RESERVE_PER_FRAME))
            .saturating_add(BRIDGE_RESERVE + TRANSPORT_RESERVE)
    }
    fn stamp(&mut self, parent: Option<u64>) -> Result<StateTransactionStamp, String> {
        let mut basis = self
            .tip
            .knowledge
            .as_ref()
            .map_or_else(|| self.context.clone(), |knowledge| knowledge.basis.clone());
        basis.tick = self.tip.state.tick;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or("state transaction sequence exhausted")?;
        Ok(StateTransactionStamp {
            basis,
            transaction_id: self.next_id,
            parent_transaction_id: parent,
        })
    }
    fn cancel(&mut self, reason: StateTransactionCancelReason) -> Option<StateTransactionCancel> {
        self.patches.clear();
        self.patch_bytes = 0;
        self.pending.take().map(|pending| StateTransactionCancel {
            stamp: pending.encoded.begin().stamp.clone(),
            reason,
        })
    }
    fn full(&mut self, now: Instant) -> Result<(), String> {
        let stamp = self.stamp(None)?;
        // Borrow the baseline. Admission precedes the only complete encoded allocation.
        let size = encoded_snapshot_size(&self.tip.state, self.tip.knowledge.as_ref())
            .map_err(|error| format!("Cannot size permitted state: {error:?}"))?;
        if self
            .retained()
            .saturating_add(self.tip.retained_bytes())
            .saturating_add(size)
            > MAX_PENDING_STATE_BYTES as usize
        {
            return Err("The permitted game state exceeds the 64 MiB connection budget".into());
        }
        let encoded = encode_snapshot(
            stamp,
            &self.tip.state,
            self.tip.knowledge.as_ref(),
            FRAME_LIMIT,
        )
        .map_err(|e| format!("Cannot encode permitted state: {e:?}"))?;
        self.pending = Some(Pending::new(encoded, now));
        Ok(())
    }
    fn restart(&mut self, now: Instant) -> Result<(), String> {
        while self.restarts.front().is_some_and(|time| {
            now.saturating_duration_since(*time) > Duration::from_millis(STATE_RESTART_WINDOW_MS)
        }) {
            self.restarts.pop_front();
        }
        self.restarts.push_back(now);
        if self.restarts.len() >= MAX_STATE_RESTARTS as usize {
            return Err("State transfer restarted three times in 30 seconds".into());
        }
        self.full(now)
    }
}
pub fn install(app: &mut App) {
    // Once per application frame: transport ACK completion events clear each PreUpdate.
    app.add_systems(PostUpdate, pump.before(MessageSystems::Send));
}
pub fn disconnect(world: &mut World, entity: Entity, reason: String) {
    eprintln!("turts_server: state connection {entity:?}: {reason}");
    world.entity_mut(entity).remove::<KnowledgeSender>();
    world.trigger(Unlink {
        entity,
        reason: UnlinkReason::UserRequested(Some(reason)),
    });
}
fn emit_cancel(world: &mut World, entity: Entity, cancel: Option<StateTransactionCancel>) {
    if let Some(cancel) = cancel
        && let Some(mut sender) = world.get_mut::<MessageSender<StateTransactionCancel>>(entity)
    {
        sender.send::<ReliableChannel>(cancel);
        if let Some(mut traffic) = world.get_resource_mut::<ReliableTraffic>() {
            traffic.record(entity);
        }
    }
}
/// Small AllVisible baselines retain their legacy fast path.
pub fn queue_full(world: &mut World, entity: Entity, slot: SlotId) -> Result<bool, String> {
    queue_snapshot(world, entity, slot, false)
}
/// A large legacy patch needs its first atomic baseline before any following patch can name it.
pub fn queue_oversized(world: &mut World, entity: Entity, slot: SlotId) -> Result<bool, String> {
    queue_snapshot(world, entity, slot, true)
}
fn queue_snapshot(
    world: &mut World,
    entity: Entity,
    slot: SlotId,
    force: bool,
) -> Result<bool, String> {
    let policy = crate::permissions::completed(world);
    if !turts_sim::visibility::context_ready(world) {
        return Err("Sight context is not ready for publication".into());
    }
    let tip =
        PermittedBaseline::capture(turts_sim::completed_world(world), slot, policy.0.as_ref());
    if tip.knowledge.is_none()
        && !force
        && world.get::<KnowledgeSender>(entity).is_none()
        && postcard::experimental::serialized_size(&tip.state).map_err(|error| error.to_string())?
            <= FRAME_LIMIT
    {
        return Ok(false);
    }
    let context = match &tip.knowledge {
        Some(knowledge) => knowledge.basis.clone(),
        None => turts_mod_api::KnowledgeBasis {
            match_id: world
                .get_resource::<crate::stats::MatchId>()
                .ok_or("Missing match identity for atomic state")?
                .0,
            map: world
                .get_resource::<turts_sim::bootstrap::InstalledMapIdentity>()
                .ok_or("Missing installed map identity for atomic state")?
                .0
                .clone(),
            tick: tip.state.tick,
            phase: turts_mod_api::ObservationPhase::Completed,
        },
    };
    let now = Instant::now();
    let mut sender = if let Some(mut existing) = world.entity_mut(entity).take::<KnowledgeSender>()
    {
        let cancel = existing.cancel(StateTransactionCancelReason::Superseded);
        emit_cancel(world, entity, cancel);
        existing.tip = tip;
        existing.context = context;
        existing
    } else {
        KnowledgeSender {
            tip,
            context,
            active: None,
            pending: None,
            patches: VecDeque::new(),
            patch_bytes: 0,
            next_id: 0,
            outstanding: 0,
            restarts: VecDeque::new(),
        }
    };
    sender.full(now)?;
    world.entity_mut(entity).insert(sender);
    Ok(true)
}
pub fn queue_tick(world: &mut World, entity: Entity, slot: SlotId) -> Result<bool, String> {
    let Some(mut sender) = world.entity_mut(entity).take::<KnowledgeSender>() else {
        if crate::permissions::completed(world)
            .0
            .knowledge_basis()
            .is_none()
        {
            return Ok(false);
        }
        return queue_full(world, entity, slot);
    };
    let policy = crate::permissions::completed(world);
    let next =
        PermittedBaseline::capture(turts_sim::completed_world(world), slot, policy.0.as_ref());
    if sender.tip.state.tick == next.state.tick {
        world.entity_mut(entity).insert(sender);
        return Ok(true);
    }
    let facts = turts_sim::visibility::event_units(world, slot.0);
    let payload = if sender.tip.knowledge.is_none() && next.knowledge.is_none() {
        sender.tip.all_visible_patch(
            &next,
            turts_sim::committed_tick(world),
            slot,
            &sender.context,
        )
    } else {
        sender
            .tip
            .patch(&next, turts_sim::committed_tick(world), slot, &facts)
    };
    let parent = sender
        .patches
        .back()
        .map(|p| p.begin().stamp.transaction_id)
        .or_else(|| {
            sender
                .pending
                .as_ref()
                .map(|p| p.encoded.begin().stamp.transaction_id)
        })
        .or_else(|| sender.active.as_ref().map(|stamp| stamp.transaction_id));
    sender.tip = next;
    if parent.is_none() {
        sender.full(Instant::now())?;
    } else {
        let stamp = sender.stamp(parent)?;
        let size = postcard::experimental::serialized_size(&payload).map_err(|e| e.to_string())?;
        if sender.patch_bytes.saturating_add(size) > MAX_POST_BASE_PATCH_BYTES as usize
            || sender
                .retained()
                .saturating_add(size)
                .saturating_add(sender.tip.retained_bytes())
                > MAX_PENDING_STATE_BYTES as usize
        {
            let cancel = sender.cancel(StateTransactionCancelReason::BufferLimit);
            emit_cancel(world, entity, cancel);
            sender.restart(Instant::now())?;
        } else {
            let encoded = encode_transaction(stamp, &payload, FRAME_LIMIT)
                .map_err(|e| format!("Cannot encode permitted patch: {e:?}"))?;
            if sender.pending.is_none() {
                sender.pending = Some(Pending::new(encoded, Instant::now()));
            } else if sender.patch_bytes.saturating_add(encoded.retained_bytes())
                > MAX_POST_BASE_PATCH_BYTES as usize
            {
                drop(encoded);
                let cancel = sender.cancel(StateTransactionCancelReason::BufferLimit);
                emit_cancel(world, entity, cancel);
                sender.restart(Instant::now())?;
            } else {
                sender.patch_bytes += encoded.retained_bytes();
                sender.patches.push_back(encoded);
            }
        }
    }
    world.entity_mut(entity).insert(sender);
    Ok(true)
}
fn send_frame(
    world: &mut World,
    entity: Entity,
    frame: StateTransactionFrame,
) -> Result<(), String> {
    match frame {
        StateTransactionFrame::Begin(frame) => world
            .get_mut::<MessageSender<turts_net_protocol::StateTransactionBegin>>(entity)
            .ok_or("Missing state Begin sender")?
            .send::<KnowledgeChannel>(frame),
        StateTransactionFrame::Chunk(frame) => world
            .get_mut::<MessageSender<turts_net_protocol::StateTransactionChunk>>(entity)
            .ok_or("Missing state Chunk sender")?
            .send::<KnowledgeChannel>(frame),
        StateTransactionFrame::End(frame) => world
            .get_mut::<MessageSender<turts_net_protocol::StateTransactionEnd>>(entity)
            .ok_or("Missing state End sender")?
            .send::<KnowledgeChannel>(frame),
    }
    Ok(())
}
fn pump_one(
    world: &mut World,
    entity: Entity,
    sender: &mut KnowledgeSender,
    now: Instant,
) -> Result<(), String> {
    let acknowledgements = world
        .get::<Transport>(entity)
        .and_then(|transport| {
            transport.channel_send(lightyear::transport::channel::ChannelKind::of::<
                KnowledgeChannel,
            >())
        })
        .map_or(0, |channel| channel.message_acks().len());
    sender.outstanding = sender
        .outstanding
        .checked_sub(acknowledgements)
        .ok_or("State transport returned unaccounted acknowledgement")?;
    let acks = world
        .get_mut::<MessageReceiver<StateTransactionAck>>(entity)
        .map_or_else(Vec::new, |mut receiver| {
            receiver.receive().take(WINDOW * 4).collect::<Vec<_>>()
        });
    for ack in acks {
        if sender
            .pending
            .as_ref()
            .is_some_and(|pending| pending.end_sent && pending.encoded.begin().stamp == ack.stamp)
        {
            sender.active = Some(ack.stamp);
            sender.pending = None;
            if let Some(encoded) = sender.patches.pop_front() {
                sender.patch_bytes -= encoded.retained_bytes();
                sender.pending = Some(Pending::new(encoded, now));
            }
        }
    }
    let cancels = world
        .get_mut::<MessageReceiver<StateTransactionCancel>>(entity)
        .map_or_else(Vec::new, |mut receiver| {
            receiver.receive().take(WINDOW * 4).collect::<Vec<_>>()
        });
    let cancelled = cancels.iter().any(|cancel| {
        sender
            .pending
            .as_ref()
            .is_some_and(|pending| pending.encoded.begin().stamp == cancel.stamp)
    });
    let timed_out = sender.pending.as_ref().is_some_and(|pending| {
        now.saturating_duration_since(pending.started)
            >= Duration::from_millis(STATE_TRANSACTION_TIMEOUT_MS)
    });
    if cancelled || timed_out {
        let cancel = sender.cancel(if timed_out {
            StateTransactionCancelReason::TimedOut
        } else {
            StateTransactionCancelReason::InvalidTransaction
        });
        emit_cancel(world, entity, cancel);
        sender.restart(now)?;
    }
    if sender.retained() > MAX_PENDING_STATE_BYTES as usize {
        return Err("State transfer exceeds the 64 MiB retained budget".into());
    }
    let mut first_kind = None;
    for _ in 0..FRAMES_PER_PUMP {
        if sender.outstanding >= WINDOW
            || sender.retained().saturating_add(COPY_RESERVE_PER_FRAME)
                > MAX_PENDING_STATE_BYTES as usize
        {
            break;
        }
        let Some(pending) = &mut sender.pending else {
            break;
        };
        let Some(frame) = pending
            .deferred
            .take()
            .or_else(|| pending.encoded.next_frame())
        else {
            break;
        };
        let kind = match frame {
            StateTransactionFrame::Begin(_) => 0,
            StateTransactionFrame::Chunk(_) => 1,
            StateTransactionFrame::End(_) => 2,
        };
        if first_kind.is_some_and(|first| first != kind) {
            pending.deferred = Some(frame);
            break;
        }
        first_kind = Some(kind);
        if kind == 2 {
            pending.end_sent = true;
        }
        send_frame(world, entity, frame)?;
        sender.outstanding += 1;
    }
    Ok(())
}
fn pump(world: &mut World) {
    let recipients = world
        .iter_entities()
        .filter(|entity| entity.contains::<Session>() && entity.contains::<KnowledgeSender>())
        .map(|entity| entity.id())
        .collect::<Vec<_>>();
    let now = Instant::now();
    for entity in recipients {
        let Some(mut sender) = world.entity_mut(entity).take::<KnowledgeSender>() else {
            continue;
        };
        match pump_one(world, entity, &mut sender, now) {
            Ok(()) => {
                world.entity_mut(entity).insert(sender);
            }
            Err(reason) => disconnect(world, entity, reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_mod_api::{
        ContentId, KnowledgeBasis, KnowledgeSnapshot, ObservationPhase, VisibilityGrid, WorldPos,
    };
    fn sender(now: Instant) -> KnowledgeSender {
        let tip = PermittedBaseline {
            state: turts_net_protocol::Snapshot::default(),
            knowledge: Some(KnowledgeSnapshot {
                basis: KnowledgeBasis {
                    match_id: 1,
                    map: ContentId::parse("fixture:map").unwrap(),
                    tick: 0,
                    phase: ObservationPhase::Completed,
                },
                viewer: 0,
                grid: VisibilityGrid {
                    size: WorldPos { x: 8.0, y: 8.0 },
                    cell_size: 2.0,
                    columns: 4,
                    rows: 4,
                },
                observed: vec![0; 2],
                explored: vec![0; 2],
                remembered_units: vec![],
                remembered_nodes: vec![],
                retention_truncated: false,
            }),
        };
        let context = tip.knowledge.as_ref().unwrap().basis.clone();
        let mut sender = KnowledgeSender {
            tip,
            context,
            active: None,
            pending: None,
            patches: VecDeque::new(),
            patch_bytes: 0,
            next_id: 0,
            outstanding: 0,
            restarts: VecDeque::new(),
        };
        sender.full(now).unwrap();
        sender
    }
    #[test]
    fn large_all_visible_snapshot_uses_real_map_context_and_keeps_an_atomic_patch_chain() {
        let mut app = turts_sim::headless_app(20);
        app.insert_resource(crate::stats::MatchId(17));
        let map = ContentId::parse("fixture:map").unwrap();
        app.insert_resource(turts_sim::bootstrap::InstalledMapIdentity(map.clone()));
        let entity = app.world_mut().spawn_empty().id();
        assert!(!queue_full(app.world_mut(), entity, SlotId(0)).unwrap());
        let mut view = turts_sim::WorldView::default();
        view.units = (1..=200)
            .map(|unit_id| turts_sim::world::UnitState {
                unit_id,
                content_id: "fixture:unit".into(),
                owner_slot: 0,
                facing: None,
                pos: bevy::math::Vec2::new(2.0, 2.0),
            })
            .collect();
        app.insert_resource(turts_sim::CompletedWorld(view));
        assert!(queue_full(app.world_mut(), entity, SlotId(0)).unwrap());
        let sender = app.world().get::<KnowledgeSender>(entity).unwrap();
        assert!(sender.tip.knowledge.is_none());
        let begin = sender.pending.as_ref().unwrap().encoded.begin();
        assert_eq!(begin.stamp.basis.map, map);
        assert_eq!(begin.stamp.basis.match_id, 17);
        assert!(begin.chunk_count > 1);
        app.world_mut()
            .resource_mut::<turts_sim::CompletedWorld>()
            .0
            .tick = 1;
        assert!(queue_tick(app.world_mut(), entity, SlotId(0)).unwrap());
        let sender = app.world().get::<KnowledgeSender>(entity).unwrap();
        assert_eq!(sender.patches.len(), 1);
        assert_eq!(
            sender.patches[0].begin().stamp.parent_transaction_id,
            Some(1)
        );
        assert_eq!(sender.patches[0].begin().stamp.basis.tick, 1);
    }
    #[test]
    fn logical_cancel_releases_unsent_buffers_but_carries_inflight_credit_until_ack() {
        let now = Instant::now();
        let mut sender = sender(now);
        sender.outstanding = WINDOW;
        let stamp = sender
            .pending
            .as_ref()
            .unwrap()
            .encoded
            .begin()
            .stamp
            .clone();
        let cancel = sender
            .cancel(StateTransactionCancelReason::TimedOut)
            .unwrap();
        assert_eq!(cancel.stamp, stamp);
        assert!(sender.pending.is_none());
        assert_eq!(sender.patch_bytes, 0);
        assert_eq!(sender.outstanding, WINDOW);
        sender.restart(now).unwrap();
        assert_eq!(sender.outstanding, WINDOW);
        assert!(
            sender
                .pending
                .as_ref()
                .unwrap()
                .encoded
                .begin()
                .stamp
                .transaction_id
                > stamp.transaction_id
        );
    }
    #[test]
    fn restart_window_disconnects_only_the_slow_viewer_and_old_attempts_expire() {
        let now = Instant::now();
        let mut slow = sender(now);
        let healthy = sender(now);
        for step in 0..2 {
            slow.cancel(StateTransactionCancelReason::TimedOut);
            slow.restart(now + Duration::from_secs(step * 5)).unwrap();
        }
        slow.cancel(StateTransactionCancelReason::TimedOut);
        assert!(
            slow.restart(now + Duration::from_secs(10))
                .unwrap_err()
                .contains("three times")
        );
        assert_eq!(healthy.restarts.len(), 0);
        assert_eq!(healthy.next_id, 1);
        assert!(healthy.pending.is_some());
        slow.restart(now + Duration::from_secs(41)).unwrap();
        assert_eq!(slow.restarts.len(), 1);
    }
    #[test]
    fn bounded_typed_send_window_does_not_block_a_second_connection() {
        use lightyear::prelude::server::{ClientOf, ServerPlugins};
        let mut app = App::new();
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.add_plugins(turts_net_lightyear::ProtocolPlugin);
        app.finish();
        app.cleanup();
        let spawn = |app: &mut App| {
            let registry = app.world().resource::<ChannelRegistry>();
            let mut transport = Transport::default();
            transport.add_channel_send_from_registry::<KnowledgeChannel>(registry);
            app.world_mut()
                .spawn((
                    ClientOf,
                    Link::default(),
                    transport,
                    RemoteId(PeerId::Local(42)),
                    Linked,
                    Connected,
                    Session {
                        slot: SlotId(0),
                        player_id: 42,
                        last_seq: None,
                    },
                    MessageSender::<turts_net_protocol::StateTransactionBegin>::default(),
                    MessageSender::<turts_net_protocol::StateTransactionChunk>::default(),
                    MessageSender::<turts_net_protocol::StateTransactionEnd>::default(),
                ))
                .id()
        };
        let slow_entity = spawn(&mut app);
        let healthy_entity = spawn(&mut app);
        let now = Instant::now();
        let mut slow = sender(now);
        slow.cancel(StateTransactionCancelReason::Superseded);
        slow.tip.state.units = (1..=2000)
            .map(|id| turts_net_protocol::UnitSnapshot {
                unit_id: turts_net_protocol::UnitId(id),
                content_id: "fixture:ordinary_unit_with_a_public_content_name".into(),
                owner_slot: turts_net_protocol::SlotId(0),
                facing: None,
                pos: turts_net_protocol::Pos { x: 1.0, y: 1.0 },
                site: false,
            })
            .collect();
        slow.full(now).unwrap();
        assert!(slow.pending.as_ref().unwrap().encoded.begin().chunk_count > WINDOW as u32);
        for _ in 0..20 {
            pump_one(app.world_mut(), slow_entity, &mut slow, now).unwrap();
        }
        assert_eq!(slow.outstanding, WINDOW);
        assert!(slow.retained() < MAX_PENDING_STATE_BYTES as usize);
        let mut healthy = sender(now);
        pump_one(app.world_mut(), healthy_entity, &mut healthy, now).unwrap();
        assert_eq!(healthy.outstanding, 1);
        assert!(
            healthy
                .pending
                .as_ref()
                .unwrap()
                .encoded
                .begin()
                .encoded_bytes
                < slow.pending.as_ref().unwrap().encoded.begin().encoded_bytes
        );
    }
    #[test]
    fn capacity_accounting_rejects_large_typed_storage_before_snapshot_encoding() {
        let now = Instant::now();
        let mut sender = sender(now);
        sender.cancel(StateTransactionCancelReason::Superseded);
        let content = String::with_capacity(MAX_PENDING_STATE_BYTES as usize);
        sender
            .tip
            .state
            .units
            .push(turts_net_protocol::UnitSnapshot {
                unit_id: turts_net_protocol::UnitId(1),
                content_id: content,
                owner_slot: turts_net_protocol::SlotId(0),
                facing: None,
                pos: turts_net_protocol::Pos { x: 1.0, y: 1.0 },
                site: false,
            });
        assert!(sender.tip.retained_bytes() >= MAX_PENDING_STATE_BYTES as usize);
        assert!(sender.full(now).unwrap_err().contains("64 MiB"));
        assert!(sender.pending.is_none());
    }
}
