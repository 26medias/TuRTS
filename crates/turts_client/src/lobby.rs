//! Engine-owned lobby state and ordinary Ready/colour controls.
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button as UiButton};
use lightyear::prelude::*;
use turts_net_lightyear::ReliableChannel;
use turts_net_protocol::{
    LobbyAction, LobbyActionKind, LobbyRefusal, LobbyRefused, LobbyStart, LobbyState, SeatOccupant,
};

use crate::join::{AcquiredPackages, Phase};
use crate::rejoin::RejoinController;
use crate::ui_shared::UiCaptureRoot;
use crate::widgets::primitives::{AuthoredWidgetId, WidgetEnabled};

#[derive(Resource, Default)]
pub struct ClientLobby {
    pub state: Option<LobbyState>,
    desired_ready: Option<bool>,
    desired_colour: Option<u8>,
    last_sent: Option<(u32, LobbyActionKind)>,
    notice: Option<String>,
}

type LobbyReaders<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Option<&'static mut MessageReceiver<LobbyState>>,
        Option<&'static mut MessageReceiver<LobbyRefused>>,
    ),
    (With<Client>, Without<Disconnected>),
>;
type LobbyWriters<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static mut MessageSender<LobbyAction>),
    (With<Client>, Without<Disconnected>),
>;

pub fn receive_states(
    mut clients: LobbyReaders,
    mut lobby: ResMut<ClientLobby>,
    mut phase: ResMut<Phase>,
    mut rejoin: ResMut<RejoinController>,
    time: Res<Time<Real>>,
    mut witnesses: ResMut<crate::ingest::ReceiptWitnesses>,
) {
    for (entity, mut states, mut refusals) in &mut clients {
        let epoch = witnesses.epoch_for(entity.to_bits());
        for received in states
            .iter_mut()
            .flat_map(|states| states.receive_with_tick())
        {
            witnesses.record_message(
                epoch,
                received.channel_kind
                    == lightyear::transport::channel::ChannelKind::of::<ReliableChannel>(),
            );
            let state = received.data;
            if !rejoin.accepts_messages_from_at(entity.to_bits(), time.elapsed_secs_f64())
                || matches!(*phase, Phase::Failed(_) | Phase::Joined { .. })
            {
                continue;
            }
            let Some(you) = state.you.and_then(|you| state.seats.get(usize::from(you))) else {
                *phase = Phase::Failed("The server did not assign a lobby seat.".into());
                continue;
            };
            if matches!(you.occupant, SeatOccupant::Open) {
                *phase = Phase::Failed("The server assigned an open lobby seat.".into());
                continue;
            }
            *phase = Phase::Lobby;
            rejoin.advance_to(time.elapsed_secs_f64(), true);
            lobby.state = Some(state);
        }
        for received in refusals
            .iter_mut()
            .flat_map(|refusals| refusals.receive_with_tick())
        {
            witnesses.record_message(
                epoch,
                received.channel_kind
                    == lightyear::transport::channel::ChannelKind::of::<ReliableChannel>(),
            );
            let refusal = received.data;
            if !rejoin.accepts_messages_from(entity.to_bits()) || !matches!(*phase, Phase::Lobby) {
                continue;
            }
            match refusal.reason {
                LobbyRefusal::LobbyChanged => {}
                LobbyRefusal::ColourTaken => {
                    lobby.desired_colour = None;
                    lobby.notice = Some("Another player has this colour.".into());
                }
                LobbyRefusal::ColourUnknown => {
                    lobby.desired_colour = None;
                    lobby.notice = Some("This colour is unavailable.".into());
                }
                LobbyRefusal::NotInLobby => {
                    lobby.desired_ready = None;
                    lobby.desired_colour = None;
                    lobby.notice = Some("This lobby has closed.".into());
                }
            }
        }
    }
}

/// Retry an explicit intent on a newer authoritative revision, without a timer or click loop.
pub fn send_intent(
    mut lobby: ResMut<ClientLobby>,
    phase: Res<Phase>,
    rejoin: Res<RejoinController>,
    mut senders: LobbyWriters,
) {
    if !matches!(*phase, Phase::Lobby) {
        return;
    }
    let Some((revision, ready, colour)) = lobby.state.as_ref().and_then(|state| {
        state
            .you
            .and_then(|you| state.seats.get(usize::from(you)))
            .map(|seat| (state.revision, seat.ready, seat.colour))
    }) else {
        return;
    };
    // Keep the latest local choice even when an older snapshot already matches it.
    // An opposite action can still be in flight and arrive in the next revision.
    let action = lobby
        .desired_colour
        .filter(|value| *value != colour)
        .map(LobbyActionKind::SetColour)
        .or_else(|| {
            lobby
                .desired_ready
                .filter(|value| *value != ready)
                .map(LobbyActionKind::SetReady)
        });
    let Some(action) = action else {
        return;
    };
    if lobby.last_sent == Some((revision, action)) {
        return;
    }
    for (entity, mut sender) in &mut senders {
        if rejoin.current_attempt_is(entity.to_bits()) {
            sender.send::<ReliableChannel>(LobbyAction { revision, action });
            lobby.last_sent = Some((revision, action));
            break;
        }
    }
}

#[derive(Component)]
pub(crate) enum LobbyButton {
    Ready,
    Colour(u8),
}

pub(crate) fn activate(
    trigger: On<Activate>,
    buttons: Query<(&LobbyButton, &WidgetEnabled)>,
    mut lobby: ResMut<ClientLobby>,
) {
    let Ok((button, enabled)) = buttons.get(trigger.entity) else {
        return;
    };
    if !enabled.0 {
        return;
    }
    let Some((ready, colour)) = lobby.state.as_ref().and_then(|state| {
        state
            .you
            .and_then(|you| state.seats.get(usize::from(you)))
            .map(|seat| (seat.ready, seat.colour))
    }) else {
        return;
    };
    lobby.notice = None;
    match button {
        LobbyButton::Ready => lobby.desired_ready = Some(!lobby.desired_ready.unwrap_or(ready)),
        LobbyButton::Colour(value)
            if *value != colour
                || lobby
                    .desired_colour
                    .is_some_and(|pending| pending != *value) =>
        {
            lobby.desired_ready = Some(false);
            lobby.desired_colour = Some(*value);
        }
        LobbyButton::Colour(_) => {}
    }
}

#[derive(Resource, Default)]
pub(crate) struct LobbyUi {
    root: Option<Entity>,
    camera: Option<Entity>,
}

#[derive(Component)]
pub(crate) struct LobbySwatch(u8);

fn label(parent: &mut ChildSpawnerCommands, id: &str, value: String, size: f32) {
    parent.spawn((
        Text::new(value),
        AuthoredWidgetId(id.into()),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::WHITE),
        bevy::picking::Pickable::IGNORE,
    ));
}

pub(crate) fn sync_ui(
    phase: Option<Res<Phase>>,
    lobby: Option<Res<ClientLobby>>,
    acquired: Option<Res<AcquiredPackages>>,
    mut ui: ResMut<LobbyUi>,
    mut commands: Commands,
) {
    let state = lobby
        .as_deref()
        .filter(|_| {
            phase
                .as_deref()
                .is_some_and(|phase| matches!(phase, Phase::Lobby))
        })
        .and_then(|lobby| lobby.state.as_ref());
    let Some(state) = state else {
        if let Some(root) = ui.root.take() {
            commands.entity(root).despawn();
        }
        if let Some(camera) = ui.camera.take() {
            commands.entity(camera).despawn();
        }
        return;
    };
    let lobby = lobby.as_deref().unwrap();
    let remaining = match state.start {
        LobbyStart::Waiting => 0,
        LobbyStart::Countdown { remaining_ms } => remaining_ms.div_ceil(1000),
    };
    if ui.root.is_some() {
        return;
    }
    let camera = *ui.camera.get_or_insert_with(|| {
        commands
            .spawn((
                Camera2d,
                Camera {
                    order: 101,
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
            ))
            .id()
    });
    let colours = acquired
        .as_deref()
        .and_then(|acquired| acquired.0.presentation.interface.as_ref())
        .map(|interface| interface.slot_colours.clone())
        .unwrap_or_default();
    let you = state.you.unwrap();
    let ready = state.seats[usize::from(you)].ready;
    ui.root = Some(
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.025, 0.055, 0.045)),
                UiCaptureRoot,
                bevy::ui::FocusPolicy::Block,
                ZIndex(i32::MAX - 3),
                UiTargetCamera(camera),
                AuthoredWidgetId("lobby.root".into()),
            ))
            .with_children(|screen| {
                screen
                    .spawn((
                        Node {
                            width: Val::Px(520.0),
                            max_width: Val::Percent(92.0),
                            padding: UiRect::all(Val::Px(24.0)),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(18.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.06, 0.12, 0.10)),
                    ))
                    .with_children(|panel| {
                        label(panel, "lobby.title", "Skirmish lobby".into(), 28.0);
                        label(
                            panel,
                            "lobby.instructions",
                            "Choose a colour. Press Ready when you want to start.".into(),
                            16.0,
                        );
                        for seat in &state.seats {
                            let text = match &seat.occupant {
                                SeatOccupant::Open => {
                                    format!("Player {}: Waiting for player", seat.index + 1)
                                }
                                _ => format!(
                                    "Player {}{}: {}",
                                    seat.index + 1,
                                    if seat.index == you { " (you)" } else { "" },
                                    if seat.ready { "Ready" } else { "Not ready" }
                                ),
                            };
                            let colour = colours
                                .get(usize::from(seat.colour))
                                .and_then(|hex| crate::widgets::primitives::parse_colour(hex))
                                .unwrap_or(Color::WHITE);
                            panel
                                .spawn(Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(12.0),
                                    align_items: AlignItems::Center,
                                    ..default()
                                })
                                .with_children(|row| {
                                    row.spawn((
                                        Node {
                                            width: Val::Px(18.0),
                                            height: Val::Px(18.0),
                                            ..default()
                                        },
                                        BackgroundColor(colour),
                                        LobbySwatch(seat.index),
                                        bevy::picking::Pickable::IGNORE,
                                    ));
                                    label(row, &format!("lobby.seat.{}", seat.index), text, 19.0);
                                });
                        }
                        panel
                            .spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(12.0),
                                ..default()
                            })
                            .with_children(|row| {
                                for (index, hex) in colours.iter().enumerate() {
                                    let index = index as u8;
                                    let enabled = !state.seats.iter().any(|seat| {
                                        seat.index != you
                                            && !matches!(seat.occupant, SeatOccupant::Open)
                                            && seat.colour == index
                                    });
                                    let colour = crate::widgets::primitives::parse_colour(hex)
                                        .unwrap_or(Color::WHITE);
                                    let mut button = row.spawn((
                                        UiButton,
                                        LobbyButton::Colour(index),
                                        WidgetEnabled(enabled),
                                        AuthoredWidgetId(format!("lobby.colour.{index}")),
                                        Node {
                                            width: Val::Px(42.0),
                                            height: Val::Px(34.0),
                                            border: UiRect::all(Val::Px(2.0)),
                                            ..default()
                                        },
                                        BorderColor::all(
                                            if state.seats[usize::from(you)].colour == index {
                                                Color::WHITE
                                            } else {
                                                Color::srgb(0.2, 0.2, 0.2)
                                            },
                                        ),
                                        BackgroundColor(colour),
                                    ));
                                    if !enabled {
                                        button.insert(bevy::ui::InteractionDisabled);
                                    }
                                }
                            });
                        panel
                            .spawn((
                                UiButton,
                                LobbyButton::Ready,
                                WidgetEnabled(true),
                                AuthoredWidgetId("lobby.ready".into()),
                                Node {
                                    padding: UiRect::all(Val::Px(12.0)),
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.15, 0.38, 0.28)),
                            ))
                            .with_children(|button| {
                                label(
                                    button,
                                    "lobby.ready.label",
                                    if ready { "Cancel Ready" } else { "Ready" }.into(),
                                    22.0,
                                )
                            });
                        label(
                            panel,
                            "lobby.status",
                            lobby.notice.clone().unwrap_or_else(|| {
                                if remaining > 0 {
                                    format!("Starting in {remaining}")
                                } else {
                                    "Both players must be ready.".into()
                                }
                            }),
                            17.0,
                        );
                    });
            })
            .id(),
    );
}

pub(crate) fn update_ui(
    phase: Option<Res<Phase>>,
    lobby: Option<Res<ClientLobby>>,
    acquired: Option<Res<AcquiredPackages>>,
    mut texts: Query<(&AuthoredWidgetId, &mut Text)>,
    mut buttons: Query<(Entity, &LobbyButton, &mut WidgetEnabled, &mut BorderColor)>,
    mut swatches: Query<(&LobbySwatch, &mut BackgroundColor)>,
    mut commands: Commands,
) {
    if !phase
        .as_deref()
        .is_some_and(|phase| matches!(phase, Phase::Lobby))
    {
        return;
    }
    let Some(lobby) = lobby.as_deref() else {
        return;
    };
    let Some(state) = &lobby.state else {
        return;
    };
    let Some(you) = state.you else {
        return;
    };
    let Some(own) = state.seats.get(usize::from(you)) else {
        return;
    };
    let colours = acquired
        .as_deref()
        .and_then(|acquired| acquired.0.presentation.interface.as_ref())
        .map(|interface| &interface.slot_colours);
    let colour_for = |index: u8| {
        colours
            .and_then(|colours| colours.get(usize::from(index)))
            .and_then(|hex| crate::widgets::primitives::parse_colour(hex))
            .unwrap_or(Color::WHITE)
    };
    let status = lobby.notice.clone().unwrap_or_else(|| match state.start {
        LobbyStart::Countdown { remaining_ms } => {
            format!("Starting in {}", remaining_ms.div_ceil(1000))
        }
        LobbyStart::Waiting
            if lobby.desired_ready.is_some_and(|value| value != own.ready)
                || lobby
                    .desired_colour
                    .is_some_and(|value| value != own.colour) =>
        {
            "Waiting for the server.".into()
        }
        LobbyStart::Waiting => "Both players must be ready.".into(),
    });
    for (id, mut text) in &mut texts {
        let value = if id.0 == "lobby.status" {
            Some(status.clone())
        } else if id.0 == "lobby.ready.label" {
            Some(if own.ready { "Cancel Ready" } else { "Ready" }.to_string())
        } else if let Some(index) =
            id.0.strip_prefix("lobby.seat.")
                .and_then(|index| index.parse::<usize>().ok())
        {
            state.seats.get(index).map(|seat| match seat.occupant {
                SeatOccupant::Open => format!("Player {}: Waiting for player", seat.index + 1),
                _ => format!(
                    "Player {}{}: {}",
                    seat.index + 1,
                    if seat.index == you { " (you)" } else { "" },
                    if seat.ready { "Ready" } else { "Not ready" }
                ),
            })
        } else {
            None
        };
        if let Some(value) = value
            && text.0 != value
        {
            text.0 = value;
        }
    }
    for (entity, action, mut enabled, mut border) in &mut buttons {
        if let LobbyButton::Colour(index) = action {
            let available = !state.seats.iter().any(|seat| {
                seat.index != you
                    && !matches!(seat.occupant, SeatOccupant::Open)
                    && seat.colour == *index
            });
            if enabled.0 != available {
                enabled.0 = available;
                if available {
                    commands
                        .entity(entity)
                        .remove::<bevy::ui::InteractionDisabled>();
                } else {
                    commands
                        .entity(entity)
                        .insert(bevy::ui::InteractionDisabled);
                }
            }
            let value = BorderColor::all(if own.colour == *index {
                Color::WHITE
            } else {
                Color::srgb(0.2, 0.2, 0.2)
            });
            if *border != value {
                *border = value;
            }
        }
    }
    for (seat, mut background) in &mut swatches {
        if let Some(seat) = state.seats.get(usize::from(seat.0)) {
            let colour = colour_for(seat.colour);
            if background.0 != colour {
                background.0 = colour;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> LobbyState {
        LobbyState {
            revision: 1,
            you: Some(0),
            seats: vec![
                turts_net_protocol::Seat {
                    index: 0,
                    occupant: SeatOccupant::Human {
                        label: "Player 1".into(),
                    },
                    ready: false,
                    colour: 0,
                },
                turts_net_protocol::Seat {
                    index: 1,
                    occupant: SeatOccupant::Open,
                    ready: false,
                    colour: 1,
                },
            ],
            settings: Vec::new(),
            start: LobbyStart::Waiting,
        }
    }

    fn ready_button(world: &mut World) -> Entity {
        let mut query = world.query::<(Entity, &AuthoredWidgetId)>();
        query
            .iter(world)
            .find(|(_, id)| id.0 == "lobby.ready")
            .unwrap()
            .0
    }

    #[test]
    fn ready_activation_sets_intent_and_countdown_keeps_the_same_button() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Phase::Lobby)
            .insert_resource(ClientLobby {
                state: Some(state()),
                ..default()
            })
            .init_resource::<LobbyUi>()
            .add_systems(Update, (sync_ui, update_ui).chain())
            .add_observer(activate);
        app.update();
        let button = ready_button(app.world_mut());
        app.world_mut().trigger(Activate { entity: button });
        assert_eq!(
            app.world().resource::<ClientLobby>().desired_ready,
            Some(true)
        );
        {
            let mut lobby = app.world_mut().resource_mut::<ClientLobby>();
            let state = lobby.state.as_mut().unwrap();
            state.seats[0].ready = true;
            state.revision += 1;
            state.start = LobbyStart::Countdown { remaining_ms: 3000 };
        }
        app.update();
        assert_eq!(ready_button(app.world_mut()), button);
        app.world_mut()
            .resource_mut::<ClientLobby>()
            .state
            .as_mut()
            .unwrap()
            .start = LobbyStart::Countdown { remaining_ms: 2000 };
        app.update();
        assert_eq!(ready_button(app.world_mut()), button);
        app.world_mut().resource_mut::<ClientLobby>().desired_ready = None;
        app.world_mut().trigger(Activate { entity: button });
        assert_eq!(
            app.world().resource::<ClientLobby>().desired_ready,
            Some(false)
        );
    }

    #[test]
    fn a_waiting_lobby_does_not_expire_the_stored_identity_join_window() {
        let mut rejoin = RejoinController::for_stored_identity();
        assert!(rejoin.start_attempt_at(7, 0.0));
        assert!(rejoin.on_connected_at(7, 0.0));
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Phase::Lobby)
            .insert_resource(rejoin)
            .init_resource::<crate::ingest::ReplicaState>()
            .add_systems(Update, crate::join::advance_rejoin);
        app.update();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs(30),
        ));
        app.update();
        assert_eq!(*app.world().resource::<Phase>(), Phase::Lobby);
        assert!(
            app.world()
                .resource::<RejoinController>()
                .terminal_text()
                .is_none()
        );
    }
    #[test]
    fn first_state_reader_does_not_require_a_refusal_that_never_arrived() {
        let mut world = World::new();
        let peer = world
            .spawn((Client, MessageReceiver::<LobbyState>::default()))
            .id();
        let mut query = bevy::ecs::system::SystemState::<LobbyReaders>::new(&mut world);
        assert_eq!(
            query
                .get_mut(&mut world)
                .unwrap()
                .iter_mut()
                .map(|(entity, _, _)| entity)
                .collect::<Vec<_>>()
                .len(),
            1,
            "a lazily inserted state receiver must work without a refusal receiver"
        );
        assert!(world.get::<MessageReceiver<LobbyRefused>>(peer).is_none());
    }

    #[test]
    fn colour_choice_can_return_to_the_current_colour_before_the_echo() {
        let mut app = App::new();
        app.insert_resource(ClientLobby {
            state: Some(state()),
            desired_ready: Some(true),
            desired_colour: Some(2),
            ..default()
        })
        .add_observer(activate);
        let button = app
            .world_mut()
            .spawn((LobbyButton::Colour(0), WidgetEnabled(true)))
            .id();
        app.world_mut().trigger(Activate { entity: button });
        let lobby = app.world().resource::<ClientLobby>();
        assert_eq!(lobby.desired_colour, Some(0));
        assert_eq!(lobby.desired_ready, Some(false));
    }

    #[test]
    fn independent_review_unready_survives_inflight_ready_echo() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Phase::Lobby)
            .insert_resource(ClientLobby {
                state: Some(state()),
                desired_ready: Some(true),
                ..default()
            })
            .add_systems(Update, send_intent);
        let peer = app
            .world_mut()
            .spawn((Client, MessageSender::<LobbyAction>::default()))
            .id();
        let mut rejoin = RejoinController::for_stored_identity();
        assert!(rejoin.start_attempt_at(peer.to_bits(), 0.0));
        assert!(rejoin.on_connected_at(peer.to_bits(), 0.0));
        app.insert_resource(rejoin);
        app.update();
        assert_eq!(
            app.world().resource::<ClientLobby>().last_sent,
            Some((1, LobbyActionKind::SetReady(true)))
        );
        // A second click cancels Ready before the first server echo arrives.
        {
            let mut lobby = app.world_mut().resource_mut::<ClientLobby>();
            lobby.desired_ready = Some(false);
            lobby.last_sent = None;
        }
        app.update();
        {
            let mut lobby = app.world_mut().resource_mut::<ClientLobby>();
            let state = lobby.state.as_mut().unwrap();
            state.revision = 2;
            state.seats[0].ready = true;
        }
        app.update();
        assert_eq!(
            app.world().resource::<ClientLobby>().last_sent,
            Some((2, LobbyActionKind::SetReady(false))),
            "the pending Unready intent must be sent after the in-flight Ready echo"
        );
    }
}
