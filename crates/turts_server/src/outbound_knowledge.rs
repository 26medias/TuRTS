//! Connection-local, bounded atomic publication. Simulation never waits for a viewer.
use crate::{
    export::PermittedBaseline,
    session::{ReliableTraffic, Session},
};
use bevy::prelude::*;
use lightyear::prelude::*;
use std::{
    collections::{BTreeSet, VecDeque},
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
    created: Instant,
    started: Option<Instant>,
    end_sent: bool,
}
impl Pending {
    fn new(encoded: EncodedStateTransaction, now: Instant) -> Self {
        Self {
            encoded,
            created: now,
            started: None,
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
    patches: VecDeque<Pending>,
    patch_bytes: usize,
    next_id: u64,
    outstanding: BTreeSet<u16>,
    restarts: VecDeque<Instant>,
}
impl KnowledgeSender {
    #[cfg(test)]
    pub(crate) fn active_stamp(&self) -> Option<&StateTransactionStamp> {
        self.active.as_ref()
    }
    #[cfg(test)]
    pub(crate) fn benchmark_retained(&self) -> usize {
        self.retained()
    }
    #[cfg(test)]
    pub(crate) fn benchmark_drained(&self) -> bool {
        self.pending.is_none() && self.patches.is_empty() && self.outstanding.is_empty()
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
                    .saturating_mul(std::mem::size_of::<Pending>()),
            )
            .saturating_add(
                self.pending
                    .as_ref()
                    .map_or(0, |p| p.encoded.retained_bytes() + 2 * FRAME_LIMIT),
            )
            .saturating_add(self.patch_bytes)
            .saturating_add(
                self.outstanding
                    .len()
                    .saturating_mul(COPY_RESERVE_PER_FRAME),
            )
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
        // One watermark retires the entire sent/unsent post-base prefix on the other channel.
        let stamp = self
            .patches
            .back()
            .map(|patch| patch.encoded.begin().stamp.clone())
            .or_else(|| {
                self.pending
                    .as_ref()
                    .map(|pending| pending.encoded.begin().stamp.clone())
            });
        self.patches.clear();
        self.patch_bytes = 0;
        self.pending = None;
        stamp.map(|stamp| StateTransactionCancel { stamp, reason })
    }
    fn acknowledge(&mut self, stamp: &StateTransactionStamp) -> bool {
        let Some(base) = &self.pending else {
            return false;
        };
        if !base.end_sent {
            return false;
        }
        let prefix = if base.encoded.begin().stamp == *stamp {
            Some(0)
        } else {
            self.patches
                .iter()
                .position(|patch| patch.end_sent && patch.encoded.begin().stamp == *stamp)
                .filter(|index| {
                    self.patches
                        .iter()
                        .take(index + 1)
                        .all(|patch| patch.end_sent)
                })
                .map(|index| index + 1)
        };
        let Some(prefix) = prefix else {
            return false;
        };
        self.pending = None;
        for _ in 0..prefix {
            let patch = self.patches.pop_front().unwrap();
            self.patch_bytes -= patch.encoded.retained_bytes();
        }
        self.active = Some(stamp.clone());
        if let Some(patch) = self.patches.pop_front() {
            self.patch_bytes -= patch.encoded.retained_bytes();
            self.pending = Some(patch);
        }
        true
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
    // Once per application frame. One ordered enum preserves cross-kind transaction order.
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
fn validate_terrain_publication(world: &World) -> Result<(), String> {
    let installed = turts_sim::terrain::grid(world).is_some();
    if installed != turts_sim::completed_world(world).terrain.is_some() {
        return Err("Completed terrain basis is missing or belongs to a retired map".into());
    }
    if installed && !turts_sim::visibility::event_units_complete(world) {
        return Err("Terrain event witness context is incomplete".into());
    }
    Ok(())
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
    validate_terrain_publication(world)?;
    let tip = PermittedBaseline::capture_checked(
        turts_sim::completed_world(world),
        slot,
        policy.0.as_ref(),
    )?;
    if tip.knowledge.is_none()
        && tip.state.records.terrain.is_none()
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
    let mut superseded_incomplete = false;
    let mut sender = if let Some(mut existing) = world.entity_mut(entity).take::<KnowledgeSender>()
    {
        superseded_incomplete = existing.pending.is_some();
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
            outstanding: BTreeSet::new(),
            restarts: VecDeque::new(),
        }
    };
    if superseded_incomplete {
        sender.restart(now)?;
    } else {
        sender.full(now)?;
    }
    world.entity_mut(entity).insert(sender);
    Ok(true)
}
pub fn queue_tick(world: &mut World, entity: Entity, slot: SlotId) -> Result<bool, String> {
    validate_terrain_publication(world)?;
    let Some(mut sender) = world.entity_mut(entity).take::<KnowledgeSender>() else {
        if crate::permissions::completed(world)
            .0
            .knowledge_basis()
            .is_none()
            && turts_sim::completed_world(world).terrain.is_none()
        {
            return Ok(false);
        }
        return queue_full(world, entity, slot);
    };
    if sender.tip.state.tick == turts_sim::completed_world(world).tick {
        world.entity_mut(entity).insert(sender);
        return Ok(true);
    }
    #[cfg(test)]
    let profile = crate::bot_profile::start();
    let policy = crate::permissions::completed(world);
    let next = sender.tip.capture_reusing_metadata_checked(
        turts_sim::completed_world(world),
        slot,
        policy.0.as_ref(),
    )?;
    if !sender.tip.terrain_delta_compatible(&next) {
        sender.tip = next;
        let cancel = sender.cancel(StateTransactionCancelReason::Superseded);
        emit_cancel(world, entity, cancel);
        sender.restart(Instant::now())?;
        world.entity_mut(entity).insert(sender);
        return Ok(true);
    }
    #[cfg(test)]
    crate::bot_profile::finish("visibility.capture_permitted", profile);
    #[cfg(test)]
    let profile = crate::bot_profile::start();
    let facts = turts_sim::visibility::event_units(world, slot.0);
    #[cfg(test)]
    crate::bot_profile::finish("visibility.event_facts", profile);
    #[cfg(test)]
    let profile = crate::bot_profile::start();
    let payload = if sender.tip.knowledge.is_none() && next.knowledge.is_none() {
        sender.tip.all_visible_patch_checked(
            &next,
            turts_sim::committed_tick(world),
            slot,
            &sender.context,
            &facts,
        )?
    } else {
        sender
            .tip
            .patch_checked(&next, turts_sim::committed_tick(world), slot, &facts)?
    };
    #[cfg(test)]
    crate::bot_profile::finish("visibility.permitted_diff", profile);
    let parent = sender
        .patches
        .back()
        .map(|p| p.encoded.begin().stamp.transaction_id)
        .or_else(|| {
            sender
                .pending
                .as_ref()
                .map(|p| p.encoded.begin().stamp.transaction_id)
        })
        .or_else(|| sender.active.as_ref().map(|stamp| stamp.transaction_id));
    #[cfg(test)]
    let profile = crate::bot_profile::start();
    // No recoverable operation can fail between recycling the old names and
    // installing this complete replacement. Errors below discard this locally
    // taken sender; live callers disconnect before another publication round.
    sender.tip = next;
    #[cfg(test)]
    crate::bot_profile::finish("visibility.old_tip_drop", profile);
    if parent.is_none() {
        sender.full(Instant::now())?;
    } else {
        let stamp = sender.stamp(parent)?;
        #[cfg(test)]
        let profile = crate::bot_profile::start();
        let size = postcard::experimental::serialized_size(&payload).map_err(|e| e.to_string())?;
        let over_budget = sender.patch_bytes.saturating_add(size)
            > MAX_POST_BASE_PATCH_BYTES as usize
            || sender
                .retained()
                .saturating_add(size)
                .saturating_add(sender.tip.retained_bytes())
                > MAX_PENDING_STATE_BYTES as usize;
        #[cfg(test)]
        crate::bot_profile::finish("visibility.retained_admission_and_size", profile);
        if over_budget {
            let cancel = sender.cancel(StateTransactionCancelReason::BufferLimit);
            emit_cancel(world, entity, cancel);
            sender.restart(Instant::now())?;
        } else {
            #[cfg(test)]
            let profile = crate::bot_profile::start();
            let encoded = encode_transaction(stamp, &payload, FRAME_LIMIT)
                .map_err(|e| format!("Cannot encode permitted patch: {e:?}"))?;
            #[cfg(test)]
            crate::bot_profile::finish("visibility.encode", profile);
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
                sender
                    .patches
                    .push_back(Pending::new(encoded, Instant::now()));
            }
        }
    }
    world.entity_mut(entity).insert(sender);
    Ok(true)
}
struct MessageWriter<'a>(&'a mut lightyear_serde::writer::Writer);
impl postcard::ser_flavors::Flavor for MessageWriter<'_> {
    type Output = ();
    fn try_push(&mut self, value: u8) -> postcard::Result<()> {
        self.0.extend_from_slice(&[value]);
        Ok(())
    }
    fn try_extend(&mut self, bytes: &[u8]) -> postcard::Result<()> {
        self.0.extend_from_slice(bytes);
        Ok(())
    }
    fn finalize(self) -> postcard::Result<()> {
        Ok(())
    }
}
fn send_registered<M: serde::Serialize + 'static>(
    world: &mut World,
    entity: Entity,
    message: &M,
) -> Result<u16, String> {
    use lightyear_serde::ToBytes;
    let net_id = *world
        .resource::<MessageRegistry>()
        .kind_map
        .net_id(&std::any::TypeId::of::<M>().into())
        .ok_or("State message is not registered")?;
    let size =
        postcard::experimental::serialized_size(message).map_err(|error| error.to_string())?;
    if size > FRAME_LIMIT {
        return Err("State message exceeds the transport frame allowance".into());
    }
    let mut writer = lightyear_serde::writer::Writer::with_capacity(size + net_id.bytes_len());
    net_id
        .to_bytes(&mut writer)
        .map_err(|error| error.to_string())?;
    // These registered protocol records contain no Bevy entities or mapped serialization context.
    postcard::serialize_with_flavor(message, MessageWriter(&mut writer))
        .map_err(|error| error.to_string())?;
    world
        .get_mut::<Transport>(entity)
        .ok_or("Missing state transport")?
        .send_mut::<KnowledgeChannel>(writer.take_written())
        .map_err(|error| error.to_string())?
        .map(|id| id.0)
        .ok_or("State channel did not allocate a reliable message identity".into())
}
fn send_frame(
    world: &mut World,
    entity: Entity,
    frame: &StateTransactionFrame,
) -> Result<u16, String> {
    send_registered(world, entity, frame)
}

fn pump_one(
    world: &mut World,
    entity: Entity,
    sender: &mut KnowledgeSender,
    now: Instant,
) -> Result<(), String> {
    if let Some(channel) = world.get::<Transport>(entity).and_then(|transport| {
        transport.channel_send(lightyear::transport::channel::ChannelKind::of::<
            KnowledgeChannel,
        >())
    }) {
        for id in channel.message_acks() {
            sender.outstanding.remove(&id.0);
        }
    }
    let acks = world
        .get_mut::<MessageReceiver<StateTransactionAck>>(entity)
        .map_or_else(Vec::new, |mut receiver| {
            receiver.receive().take(WINDOW * 4).collect::<Vec<_>>()
        });
    for ack in acks {
        sender.acknowledge(&ack.stamp);
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
            || sender
                .patches
                .iter()
                .any(|patch| patch.started.is_some() && patch.encoded.begin().stamp == cancel.stamp)
    });
    // Enqueueing or sending a newer patch never resets the oldest unacknowledged basis deadline.
    let timed_out = sender.pending.as_ref().is_some_and(|pending| {
        now.saturating_duration_since(pending.started.unwrap_or(pending.created))
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
    for _ in 0..FRAMES_PER_PUMP {
        if sender.outstanding.len() >= WINDOW
            || sender.retained().saturating_add(COPY_RESERVE_PER_FRAME)
                > MAX_PENDING_STATE_BYTES as usize
        {
            break;
        }
        let target = if sender
            .pending
            .as_ref()
            .is_some_and(|pending| !pending.end_sent)
        {
            sender.pending.as_mut()
        } else {
            sender.patches.iter_mut().find(|patch| !patch.end_sent)
        };
        let Some(target) = target else {
            break;
        };
        let Some(frame) = target.encoded.next_frame() else {
            break;
        };
        target.started.get_or_insert(now);
        let id = send_frame(world, entity, &frame)?;
        if matches!(frame, StateTransactionFrame::End(_)) {
            target.end_sent = true;
        }
        if !sender.outstanding.insert(id) {
            return Err("State transport reused an outstanding message identity".into());
        }
    }
    Ok(())
}

pub(crate) fn pump(world: &mut World) {
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
            outstanding: BTreeSet::new(),
            restarts: VecDeque::new(),
        };
        sender.full(now).unwrap();
        sender
    }
    fn recycling_fixture() -> (App, Entity) {
        let mut app = turts_sim::headless_app(20);
        app.insert_resource(crate::stats::MatchId(17));
        app.insert_resource(turts_sim::InstalledMapIdentity(
            ContentId::parse("fixture:map").unwrap(),
        ));
        let view = turts_sim::WorldView {
            units: (1..=2)
                .map(|unit_id| turts_sim::world::UnitState {
                    unit_id,
                    content_id: "fixture:unit".into(),
                    owner_slot: 0,
                    facing: None,
                    pos: bevy::math::Vec2::new(unit_id as f32, 2.),
                })
                .collect(),
            ..Default::default()
        };
        app.insert_resource(turts_sim::CompletedWorld(view));
        let entity = app.world_mut().spawn_empty().id();
        assert!(queue_oversized(app.world_mut(), entity, SlotId(0)).unwrap());
        (app, entity)
    }
    fn decode_pending(
        pending: &mut Pending,
        assembler: &mut turts_net_protocol::visibility_transaction::StateTransactionAssembler,
    ) -> turts_net_protocol::visibility_transaction::StagedStateTransaction {
        let mut staged = None;
        while let Some(frame) = pending.encoded.next_frame() {
            match frame {
                StateTransactionFrame::Begin(begin) => assembler.begin(begin, 0).unwrap(),
                StateTransactionFrame::Chunk(chunk) => assembler.chunk(&chunk).unwrap(),
                StateTransactionFrame::End(end) => staged = assembler.end(&end).unwrap(),
            }
        }
        assert!(pending.encoded.fully_queued());
        pending.end_sent = true;
        staged.unwrap()
    }
    #[test]
    fn recycled_names_survive_ring_overflow_restart_and_the_following_old_endpoint() {
        use turts_net_protocol::{
            StateTransactionPayload, visibility_transaction::StateTransactionAssembler,
        };
        let (mut app, entity) = recycling_fixture();
        let mut assembler = StateTransactionAssembler::new(
            17,
            ContentId::parse("fixture:map").unwrap(),
            FRAME_LIMIT,
        );
        let now = Instant::now();
        let active = {
            let mut sender = app.world_mut().get_mut::<KnowledgeSender>(entity).unwrap();
            let base = decode_pending(sender.pending.as_mut().unwrap(), &mut assembler);
            let ack = assembler.activate(&base.stamp).unwrap();
            assert!(sender.acknowledge(&ack.stamp));
            sender.outstanding.extend([19, 20]);
            let active = sender.active.clone().unwrap();
            sender.tip.state.tick = 1;
            let first = sender.stamp(Some(active.transaction_id)).unwrap();
            let empty = StateTransactionPayload::Patch {
                state: Box::new(turts_net_protocol::WorldDelta {
                    tick: 1,
                    base_tick: 0,
                    ..Default::default()
                }),
                knowledge: None,
                event_units: vec![],
            };
            sender.pending = Some(Pending::new(
                encode_transaction(first.clone(), &empty, FRAME_LIMIT).unwrap(),
                now,
            ));
            sender.tip.state.tick = 2;
            let tail = sender.stamp(Some(first.transaction_id)).unwrap();
            // This synthetic, unsent buffer fills the actual encoded ring storage.
            // It tests allocation/restart boundaries, not authored game content.
            let mut large = StateTransactionPayload::Patch {
                state: Box::new(turts_net_protocol::WorldDelta {
                    tick: 2,
                    base_tick: 1,
                    economy: turts_net_protocol::EconomyChanges {
                        balances: vec![turts_net_protocol::BalanceState {
                            resource: String::new(),
                            on_hand: 0,
                            reserved: 0,
                        }],
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                knowledge: None,
                event_units: vec![],
            };
            let overhead = encode_transaction(tail.clone(), &large, FRAME_LIMIT)
                .unwrap()
                .retained_bytes();
            if let StateTransactionPayload::Patch { state, .. } = &mut large {
                state.economy.balances[0].resource =
                    "x".repeat(MAX_POST_BASE_PATCH_BYTES as usize - overhead - 16);
            }
            let encoded = encode_transaction(tail, &large, FRAME_LIMIT).unwrap();
            sender.patch_bytes = encoded.retained_bytes();
            assert!(sender.patch_bytes <= MAX_POST_BASE_PATCH_BYTES as usize);
            assert!(MAX_POST_BASE_PATCH_BYTES as usize - sender.patch_bytes < 64);
            sender.patches.push_back(Pending::new(encoded, now));
            active
        };
        {
            let mut completed = app.world_mut().resource_mut::<turts_sim::CompletedWorld>();
            completed.0.tick = 3;
            completed.0.units[0].pos.x += 1.;
        }
        app.insert_resource(turts_sim::CommittedTick {
            tick: 3,
            ..Default::default()
        });
        assert!(queue_tick(app.world_mut(), entity, SlotId(0)).unwrap());
        let restarted = {
            let mut sender = app.world_mut().get_mut::<KnowledgeSender>(entity).unwrap();
            assert_eq!(sender.active.as_ref(), Some(&active));
            assert_eq!(sender.outstanding, BTreeSet::from([19, 20]));
            assert!(sender.patches.is_empty());
            assert_eq!(sender.patch_bytes, 0);
            assert_eq!(sender.restarts.len(), 1);
            assert!(
                sender
                    .tip
                    .state
                    .units
                    .iter()
                    .all(|u| u.content_id == "fixture:unit")
            );
            let full = decode_pending(sender.pending.as_mut().unwrap(), &mut assembler);
            let StateTransactionPayload::Snapshot { state, .. } = &full.payload else {
                panic!("overflow must restart a full basis")
            };
            assert_eq!(state.tick, 3);
            assert!(state.units.iter().all(|u| u.content_id == "fixture:unit"));
            assert_eq!(
                assembler.active(),
                Some(&active),
                "decoding does not activate the replacement"
            );
            let ack = assembler.activate(&full.stamp).unwrap();
            assert!(sender.acknowledge(&ack.stamp));
            assert_eq!(
                sender.outstanding,
                BTreeSet::from([19, 20]),
                "activation ACK does not release transport credit"
            );
            full.stamp
        };
        {
            let mut completed = app.world_mut().resource_mut::<turts_sim::CompletedWorld>();
            completed.0.tick = 4;
            completed.0.units.retain(|u| u.unit_id != 2);
        }
        app.insert_resource(turts_sim::CommittedTick {
            tick: 4,
            removed: vec![2],
            effects: vec![
                turts_sim::economy::events::EffectRow::Fired {
                    unit: 1,
                    target: 2,
                    weapon: 0,
                    reduced: false,
                },
                turts_sim::economy::events::EffectRow::Destroyed { unit: 2 },
            ],
            ..Default::default()
        });
        assert!(queue_tick(app.world_mut(), entity, SlotId(0)).unwrap());
        let mut sender = app.world_mut().get_mut::<KnowledgeSender>(entity).unwrap();
        assert_eq!(sender.active.as_ref(), Some(&restarted));
        let next = decode_pending(sender.pending.as_mut().unwrap(), &mut assembler);
        let StateTransactionPayload::Patch { event_units, .. } = next.payload else {
            panic!("next tick must be a patch")
        };
        let old = event_units.iter().find(|u| u.unit.unit_id.0 == 2).unwrap();
        assert_eq!(old.unit.content_id, "fixture:unit");
        assert_eq!(old.basis.tick, 3);
    }
    #[test]
    fn fatal_sequence_exhaustion_discards_the_taken_sender_without_corrupting_the_active_view() {
        use turts_net_protocol::{
            StateTransactionPayload, visibility_transaction::StateTransactionAssembler,
        };
        let (mut app, entity) = recycling_fixture();
        let mut assembler = StateTransactionAssembler::new(
            17,
            ContentId::parse("fixture:map").unwrap(),
            FRAME_LIMIT,
        );
        let base = {
            let mut sender = app.world_mut().get_mut::<KnowledgeSender>(entity).unwrap();
            let base = decode_pending(sender.pending.as_mut().unwrap(), &mut assembler);
            let ack = assembler.activate(&base.stamp).unwrap();
            assert!(sender.acknowledge(&ack.stamp));
            sender.next_id = u64::MAX;
            base
        };
        {
            let mut view = app.world_mut().resource_mut::<turts_sim::CompletedWorld>();
            view.0.tick = 1;
            view.0.units[0].pos.x += 1.;
        }
        let failure = queue_tick(app.world_mut(), entity, SlotId(0)).unwrap_err();
        assert!(failure.contains("sequence exhausted"));
        assert!(
            app.world().get::<KnowledgeSender>(entity).is_none(),
            "fatal errors must not reinsert a partial cache; live callers disconnect"
        );
        assert_eq!(assembler.active(), Some(&base.stamp));
        let StateTransactionPayload::Snapshot { state, .. } = base.payload else {
            panic!()
        };
        assert!(state.units.iter().all(|u| u.content_id == "fixture:unit"));
        assert!(
            turts_sim::completed_world(app.world())
                .units
                .iter()
                .all(|u| u.content_id == "fixture:unit")
        );
    }
    #[test]
    fn explicit_full_resync_counts_incomplete_replacements_but_not_completed_views() {
        let mut app = turts_sim::headless_app(20);
        app.insert_resource(crate::stats::MatchId(1));
        app.insert_resource(turts_sim::InstalledMapIdentity(
            ContentId::parse("fixture:map").unwrap(),
        ));
        let slow = app.world_mut().spawn_empty().id();
        let healthy = app.world_mut().spawn_empty().id();
        assert!(queue_oversized(app.world_mut(), slow, SlotId(0)).unwrap());
        for attempt in 1..=2 {
            assert!(queue_oversized(app.world_mut(), slow, SlotId(0)).unwrap());
            assert_eq!(
                app.world()
                    .get::<KnowledgeSender>(slow)
                    .unwrap()
                    .restarts
                    .len(),
                attempt
            );
        }
        let failure = queue_oversized(app.world_mut(), slow, SlotId(0)).unwrap_err();
        assert!(failure.contains("three times in 30 seconds"));
        for _ in 0..4 {
            assert!(queue_oversized(app.world_mut(), healthy, SlotId(0)).unwrap());
            let mut sender = app.world_mut().get_mut::<KnowledgeSender>(healthy).unwrap();
            let pending = sender.pending.as_mut().unwrap();
            pending.end_sent = true;
            let stamp = pending.encoded.begin().stamp.clone();
            assert!(sender.acknowledge(&stamp));
            assert!(sender.restarts.is_empty());
        }
    }
    #[test]
    fn large_all_visible_snapshot_uses_real_map_context_and_keeps_an_atomic_patch_chain() {
        let mut app = turts_sim::headless_app(20);
        app.insert_resource(crate::stats::MatchId(17));
        let map = ContentId::parse("fixture:map").unwrap();
        app.insert_resource(turts_sim::bootstrap::InstalledMapIdentity(map.clone()));
        let entity = app.world_mut().spawn_empty().id();
        assert!(!queue_full(app.world_mut(), entity, SlotId(0)).unwrap());
        let view = turts_sim::WorldView {
            units: (1..=200)
                .map(|unit_id| turts_sim::world::UnitState {
                    unit_id,
                    content_id: "fixture:unit".into(),
                    owner_slot: 0,
                    facing: None,
                    pos: bevy::math::Vec2::new(2.0, 2.0),
                })
                .collect(),
            ..Default::default()
        };
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
            sender.patches[0]
                .encoded
                .begin()
                .stamp
                .parent_transaction_id,
            Some(1)
        );
        assert_eq!(sender.patches[0].encoded.begin().stamp.basis.tick, 1);
    }
    #[test]
    fn logical_cancel_releases_unsent_buffers_but_carries_inflight_credit_until_ack() {
        let now = Instant::now();
        let mut sender = sender(now);
        sender.outstanding = (0..WINDOW as u16).collect();
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
        assert_eq!(sender.outstanding.len(), WINDOW);
        sender.restart(now).unwrap();
        assert_eq!(sender.outstanding.len(), WINDOW);
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
    fn bounded_registered_send_window_does_not_block_a_second_connection() {
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
        assert_eq!(slow.outstanding.len(), WINDOW);
        assert!(slow.retained() < MAX_PENDING_STATE_BYTES as usize);
        let mut healthy = sender(now);
        pump_one(app.world_mut(), healthy_entity, &mut healthy, now).unwrap();
        assert!(!healthy.outstanding.is_empty());
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

#[cfg(test)]
#[path = "outbound_knowledge_pipeline_tests.rs"]
mod pipeline_tests;
