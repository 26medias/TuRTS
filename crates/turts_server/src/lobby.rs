//! Authoritative pre-match seats, actions, countdown, and the one start boundary.
use std::collections::BTreeMap;

use bevy::prelude::*;
use lightyear::prelude::server::ClientOf;
use lightyear::prelude::*;
use turts_mod_api::StartPolicy;
use turts_net_lightyear::ReliableChannel;
use turts_net_protocol::{
    LobbyAction, LobbyActionKind, LobbyRefusal, LobbyRefused, LobbyStart, LobbyState, MatchStart,
    OrderResult, SeatOccupant, SlotId, Welcome,
};
use turts_sim::{MatchConfig, Slots, Tick};

use crate::room::{ConnectionKey, JoinOutcome, Room, RoomState, SeatTable};
use crate::session::{Advertised, Session, SnapshotDue};

#[derive(Component, Debug)]
pub struct LobbyMember;

#[derive(Resource)]
pub struct HostedLobby {
    pub room: Room,
    countdown_secs: u32,
    deadline: Option<f64>,
    budgets: BTreeMap<Entity, (f64, f64)>,
    last_broadcast: Option<(u32, u32)>,
    pub start: Option<MatchStart>,
}

impl HostedLobby {
    pub fn new(policy: StartPolicy, palette_len: u8) -> Self {
        let (policy, countdown_secs) = match policy {
            StartPolicy::AllSeated { countdown_secs } => {
                (crate::room::StartPolicy::AllSeated, countdown_secs)
            }
            StartPolicy::AllSeatedReady { countdown_secs } => {
                (crate::room::StartPolicy::AllSeatedReady, countdown_secs)
            }
            StartPolicy::Immediate => unreachable!("Immediate has no lobby"),
        };
        Self {
            room: Room::boot(policy, false, turts_sim::slots::SLOT_COUNT, palette_len),
            countdown_secs,
            deadline: None,
            budgets: BTreeMap::new(),
            last_broadcast: None,
            start: None,
        }
    }

    pub fn waiting(&self) -> bool {
        matches!(self.room.state(), RoomState::Lobby | RoomState::Countdown)
    }

    pub fn join(&mut self, entity: Entity, player: u64) -> Result<(), &'static str> {
        match self.room.join(ConnectionKey(entity.to_bits()), player) {
            JoinOutcome::Seated { .. } | JoinOutcome::Reconnected { .. } => Ok(()),
            JoinOutcome::Refused(reason) => Err(reason.reason()),
            _ => Err("the server state does not allow this join"),
        }
    }

    fn admit(&mut self, entity: Entity, now: f64) -> bool {
        let budget = self.budgets.entry(entity).or_insert((now, 64.0));
        budget.1 = (budget.1 + (now - budget.0).max(0.0) * 32.0).min(64.0);
        budget.0 = now;
        if budget.1 < 1.0 {
            return false;
        }
        budget.1 -= 1.0;
        true
    }

    fn state(&self, entity: Entity, now: f64) -> LobbyState {
        LobbyState {
            revision: self.room.seats().revision(),
            you: self
                .room
                .seats()
                .seat_of_connection(ConnectionKey(entity.to_bits())),
            seats: (0..self.room.seats().seat_count())
                .map(|index| {
                    let seat = self.room.seats().seat(index);
                    turts_net_protocol::Seat {
                        index,
                        occupant: if seat.is_some() {
                            SeatOccupant::Human {
                                label: SeatTable::label(index),
                            }
                        } else {
                            SeatOccupant::Open
                        },
                        ready: seat.is_some_and(|seat| seat.ready),
                        colour: seat.map_or(index, |seat| seat.colour),
                    }
                })
                .collect(),
            settings: Vec::new(),
            start: self
                .deadline
                .map_or(LobbyStart::Waiting, |end| LobbyStart::Countdown {
                    remaining_ms: ((end - now).max(0.0) * 1000.0).ceil() as u32,
                }),
        }
    }
}

pub fn retire_connection(
    trigger: On<Add<Disconnected>>,
    mut lobby: Option<ResMut<HostedLobby>>,
    mut commands: Commands,
) {
    if let Some(lobby) = lobby.as_mut() {
        lobby
            .room
            .connection_ended(ConnectionKey(trigger.entity.to_bits()));
        lobby.budgets.remove(&trigger.entity);
        commands.entity(trigger.entity).remove::<LobbyMember>();
    }
}

/// Drain pre-start commands before the start boundary, including a final Ready in this frame.
type LobbyCommands<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut MessageReceiver<turts_net_protocol::PlayerCommand>,
        &'static mut MessageSender<turts_net_protocol::CommandAck>,
        &'static mut MessageSender<OrderResult>,
    ),
    (With<LobbyMember>, Without<Disconnected>),
>;
type LobbyActions<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut MessageReceiver<LobbyAction>,
        &'static mut MessageSender<LobbyRefused>,
    ),
    (With<ClientOf>, Without<Disconnected>),
>;

pub fn reject_gameplay(
    mut lobby: Option<ResMut<HostedLobby>>,
    time: Res<Time<Real>>,
    mut clients: LobbyCommands,
    mut traffic: ResMut<crate::session::ReliableTraffic>,
) {
    let Some(lobby) = lobby.as_mut() else {
        return;
    };
    for (entity, mut receiver, mut acks, mut replies) in &mut clients {
        for command in receiver.receive() {
            if !lobby.admit(entity, time.elapsed_secs_f64()) {
                continue;
            }
            acks.send::<ReliableChannel>(turts_net_protocol::CommandAck {
                seq: command.seq,
                tick: 0,
            });
            traffic.record(entity);
            traffic.record(entity);
            replies.send::<ReliableChannel>(OrderResult {
                seq: command.seq,
                tick: 0,
                envelope: Some(turts_net_protocol::CommandRejectReason::MatchNotStarted),
                accepted: Vec::new(),
                refused: Vec::new(),
                job_id: None,
            });
        }
    }
}

pub fn receive_actions(
    mut lobby: Option<ResMut<HostedLobby>>,
    time: Res<Time<Real>>,
    mut clients: LobbyActions,
    mut traffic: ResMut<crate::session::ReliableTraffic>,
) {
    let Some(lobby) = lobby.as_mut() else {
        return;
    };
    for (entity, mut receiver, mut replies) in &mut clients {
        for action in receiver.receive() {
            if !lobby.admit(entity, time.elapsed_secs_f64()) {
                continue;
            }
            let key = ConnectionKey(entity.to_bits());
            let refusal = if !lobby.waiting()
                || lobby.room.seats().seat_of_connection(key).is_none()
            {
                Some(LobbyRefusal::NotInLobby)
            } else if action.revision != lobby.room.seats().revision() {
                Some(LobbyRefusal::LobbyChanged)
            } else {
                match action.action {
                    LobbyActionKind::SetReady(ready) => {
                        lobby.room.set_ready(key, ready);
                        None
                    }
                    LobbyActionKind::SetColour(colour) => lobby.room.set_colour(key, colour).err(),
                }
            };
            if let Some(reason) = refusal {
                traffic.record(entity);
                replies.send::<ReliableChannel>(LobbyRefused {
                    revision: lobby.room.seats().revision(),
                    reason,
                });
            }
        }
    }
}

/// Exclusive so claims, config, welcomes, and snapshots share one atomic start boundary.
pub fn drive(world: &mut World) {
    let Some(mut lobby) = world.remove_resource::<HostedLobby>() else {
        return;
    };
    let now = world.resource::<Time<Real>>().elapsed_secs_f64();
    if !lobby.waiting() {
        if world
            .get_resource::<turts_sim::outcome::MatchOutcomes>()
            .is_some_and(|outcome| outcome.end_tick.is_some())
        {
            lobby.room.end_match();
        }
        world.insert_resource(lobby);
        return;
    }
    if lobby.room.state() != RoomState::Countdown {
        lobby.deadline = None;
    }
    if lobby.room.start_due() && lobby.deadline.is_none() && lobby.countdown_secs > 0 {
        lobby.room.begin_countdown();
        lobby.deadline = Some(now + f64::from(lobby.countdown_secs));
    }
    let start_due = lobby.room.start_due() && lobby.deadline.is_none_or(|deadline| now >= deadline);
    if start_due {
        let plan = lobby.room.start().expect("checked start conditions");
        let start = MatchStart {
            start_tick: 1,
            teams: plan.claims.iter().map(|claim| claim.slot.0).collect(),
            colours: plan.claims.iter().map(|claim| claim.colour).collect(),
        };
        let hz = world.resource::<Advertised>().tick_hz;
        let match_id = world.resource::<crate::stats::MatchId>().0;
        // No restricted world is published before the lobby's final teams exist.
        // A failed context must stop this start instead of exposing placeholder sight.
        if let Err(error) = turts_sim::visibility::initialize_context(world, match_id, &start.teams)
        {
            eprintln!("turts_server: cannot initialize match visibility: {error}");
            world.write_message(bevy::app::AppExit::error());
            world.insert_resource(lobby);
            return;
        }
        turts_sim::initialize_completed_world(world);
        plan.claim_in(&mut world.resource_mut::<Slots>())
            .expect("lobby slots are unclaimed until start");
        let tick = world.resource::<Tick>().0;
        for claim in &plan.claims {
            let entity = Entity::from_bits(claim.connection.0);
            if let Some(mut log) = world.get_resource_mut::<crate::economylog::EconomyLogSink>() {
                log.write_claim(tick, claim.player_id, claim.slot.0);
            }
            world.entity_mut(entity).remove::<LobbyMember>().insert((
                Session {
                    last_seq: None,
                    slot: claim.slot,
                    player_id: claim.player_id,
                },
                SnapshotDue,
            ));
            if let Some(mut sender) = world.get_mut::<MessageSender<Welcome>>(entity) {
                sender.send::<ReliableChannel>(Welcome {
                    slot_id: SlotId(claim.slot.0),
                    tick,
                    tick_hz: hz,
                    match_id,
                    start: start.clone(),
                });
            }
        }
        for claim in &plan.claims {
            world
                .resource_mut::<crate::session::ReliableTraffic>()
                .record(Entity::from_bits(claim.connection.0));
        }
        world.insert_resource(MatchConfig {
            tick_hz: hz,
            start_tick: 1,
            teams: start.teams.clone(),
            settings: Vec::new(),
        });
        lobby.start = Some(start);
    } else {
        let remaining = lobby.deadline.map_or(0, |end| {
            (((end - now).max(0.0) * 1000.0).ceil() as u32).div_ceil(1000)
        });
        let signature = (lobby.room.seats().revision(), remaining);
        if lobby.last_broadcast != Some(signature) {
            let entities = {
                let mut query =
                    world.query_filtered::<Entity, (With<LobbyMember>, Without<Disconnected>)>();
                query.iter(world).collect::<Vec<_>>()
            };
            for entity in entities {
                let state = lobby.state(entity, now);
                if let Some(mut sender) = world.get_mut::<MessageSender<LobbyState>>(entity) {
                    sender.send::<ReliableChannel>(state);
                }
                world
                    .resource_mut::<crate::session::ReliableTraffic>()
                    .record(entity);
            }
            lobby.last_broadcast = Some(signature);
        }
    }
    world.insert_resource(lobby);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lobby_budget_refills_with_real_time_while_simulation_is_stopped() {
        let mut lobby = HostedLobby::new(StartPolicy::AllSeatedReady { countdown_secs: 3 }, 2);
        let entity = Entity::from_bits(123);
        for _ in 0..64 {
            assert!(lobby.admit(entity, 0.0));
        }
        assert!(!lobby.admit(entity, 0.0));
        for _ in 0..16 {
            assert!(lobby.admit(entity, 0.5));
        }
        assert!(!lobby.admit(entity, 0.5));
    }
}
