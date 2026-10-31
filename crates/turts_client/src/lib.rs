//! Client logic: fetch, acquisition, join, replica, interpolation, presentation, input.

pub mod acquire;
pub mod actions;
pub mod audio;
pub mod camera;
pub mod client_shell;
pub mod commands;
pub mod content;
pub mod core_adapter;
pub mod debug;
pub mod debug_feed;
pub mod economy_ui;
pub mod feedback;
pub mod fetch;
pub mod framelog;
pub mod ingest;
pub mod input;
pub mod interp;
pub mod join;
pub mod minimap;
pub mod options;
pub mod persist;
pub mod present;
pub mod render;
pub mod replica;
mod scenario;
pub mod ui_shared;
pub mod visuals;
pub mod widgets;

#[cfg(not(target_family = "wasm"))]
mod live_view;

use std::time::Duration;

use bevy::app::{App, Plugin, ScheduleRunnerPlugin};
use bevy::prelude::*;
use lightyear::prelude::MessageSystems;
use lightyear::prelude::client::ClientPlugins;
use turts_net_lightyear::{ProtocolPlugin, tick_duration};
use turts_net_protocol::Pos;

use crate::commands::{
    CommandSendStatus, CommandSequence, PendingCommands, repeat_trace, send_commands_traced,
    submit, submit_trace,
};
use crate::core_adapter::CoreAdapterPlugin;
use crate::debug::{DebugState, exit_when_done, rebuild_debug_state};
use crate::economy_ui::EconomyUiPlugin;
use crate::framelog::{FrameLog, write_frame_log};
use crate::ingest::{
    CommandResults, IngestCounters, IngestHooks, ReceiptWitnesses, ReplicaState,
    arm_drop_world_changes, ingest_messages,
};
use crate::join::{
    Acquisition, JoinState, Phase, auto_move, drive_acquisition, on_connected, on_disconnected,
    receive_rejection, start_connection, update_phase,
};
use crate::options::{ClientOptions, random_player_id};
use crate::present::{Presented, update_presented};
use crate::render::UnitRenderPlugin;
use crate::scenario::{
    ProduceHook, ScenarioState, TraceAction, TraceCommand, produce_command_if_ready,
};

/// The tick rate of the lightyear link (not of the simulation: `Welcome` carries that).
const LINK_TICK_HZ: u16 = 20; // prototype constant: measure at M1

/// The options of this run.
#[derive(Resource)]
pub struct ClientOpts(pub ClientOptions);

/// The placeholder identity of this session (design 5, `Hello`).
#[derive(Resource)]
pub struct PlayerId(pub u64);

/// All client logic: the join, ingest, the presentation record, the logs and the debug hooks.
pub struct ClientPlugin(pub ClientOptions);

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        let opts = self.0.clone();
        #[cfg(not(target_family = "wasm"))]
        let publish_live_view = !opts.headless && opts.out.is_some();
        #[cfg(not(target_family = "wasm"))]
        let publish_live_report = opts.live_report.is_some();
        app.insert_resource(PlayerId(opts.player_id.unwrap_or_else(random_player_id)))
            .insert_resource(Phase::FetchAdvertisement)
            .insert_resource(IngestHooks {
                drop_offsets: opts.drop_delta_offsets.clone(),
                drop_arrival_delta: opts.drop_arrival_delta,
                arrival_target: opts.auto_move.map(|(x, y)| Pos { x, y }),
                arrival_unit: opts.move_unit,
                hold_resync_snapshots: opts.hold_resync_snapshots,
                hold_resync_snapshots_remaining: opts.hold_resync_snapshot_count,
                initial_drop_world_changes: opts.drop_world_changes.clone(),
                ..Default::default()
            })
            .insert_resource(FrameLog::open(opts.frame_log.as_deref()))
            .insert_resource(ScenarioState::from_path(opts.command_trace.as_deref()))
            .insert_resource(ProduceHook::from_options(&opts))
            .insert_resource(ClientOpts(opts))
            .init_resource::<IngestCounters>()
            .init_resource::<ReplicaState>()
            .init_resource::<CommandResults>()
            .init_resource::<JoinState>()
            .init_resource::<Acquisition>()
            .init_resource::<Presented>()
            .init_resource::<DebugState>()
            .init_resource::<PendingCommands>()
            .init_resource::<CommandSequence>()
            .init_resource::<CommandSendStatus>()
            .init_resource::<ReceiptWitnesses>();
        app.add_observer(on_connected).add_observer(on_disconnected);
        app.add_systems(
            PreUpdate,
            (receive_rejection, ingest_messages).after(MessageSystems::Receive),
        );
        app.add_systems(
            Update,
            (
                drive_acquisition,
                start_connection,
                update_phase,
                update_presented,
                write_frame_log,
            )
                .chain(),
        );
        app.add_systems(
            PostUpdate,
            (
                auto_move,
                drive_scenario,
                drive_produce_hook,
                send_commands_traced,
            )
                .chain()
                .before(MessageSystems::Send),
        );
        app.add_systems(Last, (rebuild_debug_state, exit_when_done).chain());
        #[cfg(not(target_family = "wasm"))]
        if publish_live_view {
            app.add_systems(Last, live_view::publish_live_view.before(exit_when_done));
        }
        #[cfg(not(target_family = "wasm"))]
        if publish_live_report {
            app.init_resource::<live_view::LiveReportCursor>()
                .add_systems(Last, live_view::publish_live_report.before(exit_when_done));
        }
    }
}

/// Send the optional one-shot Produce only after the coherent owner replica proves its gate.
fn drive_produce_hook(
    mut hook: ResMut<ProduceHook>,
    state: Res<ReplicaState>,
    mut pending: ResMut<PendingCommands>,
    status: Res<CommandSendStatus>,
) {
    if status.reconnect_required {
        return;
    }
    let (Some(request), Some(welcome), Some(replica)) =
        (hook.request.clone(), state.welcome, state.replica.as_ref())
    else {
        return;
    };
    let tick = replica.last_applied_tick;
    let completed = replica.research.iter().any(|record| {
        record.slot.0 == welcome.slot && record.completed.iter().any(|tech| tech == &request.1)
    });
    if completed && hook.research_completed_tick.is_none() {
        hook.research_completed_tick = tick;
    }
    if hook.sent {
        return;
    }
    let Some(command) = produce_command_if_ready(
        replica,
        welcome.slot,
        &request,
        hook.inventory_gate.as_ref(),
    ) else {
        return;
    };
    submit(&mut pending, command);
    hook.sent = true;
}

/// Drive one due trace action from the latest coherent server tick, after ingest and before send.
fn drive_scenario(
    mut scenario: ResMut<ScenarioState>,
    mut phase: ResMut<Phase>,
    state: Res<ReplicaState>,
    results: Res<CommandResults>,
    mut pending: ResMut<PendingCommands>,
    mut hooks: ResMut<IngestHooks>,
) {
    if !scenario.enabled {
        return;
    }
    if let Some(error) = scenario.error.clone() {
        *phase = Phase::Failed(error);
        return;
    }
    let Some(welcome) = state.welcome else { return };
    let Some(tick) = state
        .replica
        .as_ref()
        .and_then(|replica| replica.last_applied_tick)
    else {
        return;
    };
    scenario.observe_results(&results.0);
    let actions = match scenario.drive(tick, welcome.tick, &results.0) {
        Ok(actions) => actions,
        Err(error) => {
            scenario.error = Some(error.clone());
            *phase = Phase::Failed(error);
            return;
        }
    };
    for action in actions {
        match action {
            TraceAction::Submit { step_index, kind } => {
                scenario.record_trace_action(step_index);
                submit_trace(&mut pending, kind, step_index);
            }
            TraceAction::RepeatLastEnvelope {
                step_index,
                envelope,
            } => {
                scenario.record_trace_action(step_index);
                repeat_trace(&mut pending, envelope, step_index);
            }
            TraceAction::ArmDropWorldChanges {
                step_index,
                arm_tick,
                kinds,
            } => {
                scenario.record_arm(
                    step_index,
                    TraceCommand::ArmDropWorldChanges {
                        kinds: kinds.clone(),
                    },
                    arm_tick,
                );
                if let Some(replica) = state.replica.as_ref() {
                    arm_drop_world_changes(&mut hooks, kinds, replica.clone(), arm_tick);
                }
            }
        }
    }
}

/// The window of the presentation path. On wasm32 it binds to the page canvas `#turts` and
/// fits it to the parent; on native, `canvas` has no effect.
fn window_plugin() -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            title: "TuRTS".to_string(),
            canvas: Some("#turts".to_string()),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: true,
            ..default()
        }),
        ..default()
    }
}

/// Builds the client app. Headless: `MinimalPlugins` at 60 Hz. Otherwise `DefaultPlugins` with
/// the window, the scene and the click input. `--run-secs` ends the run with `AppExit`.
pub fn build_app(opts: ClientOptions) -> App {
    let mut app = App::new();
    let headless = opts.headless;
    if headless {
        app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 60.0),
        )));
    } else {
        app.add_plugins(DefaultPlugins.set(window_plugin()));
    }
    app.add_plugins(ClientPlugins {
        tick_duration: tick_duration(LINK_TICK_HZ),
    });
    app.add_plugins(ProtocolPlugin);
    app.add_plugins(ClientPlugin(opts));
    app.add_plugins((
        CoreAdapterPlugin,
        actions::ActionsPlugin,
        camera::ClientCameraPlugin,
        widgets::WidgetsPlugin,
        visuals::VisualsPlugin,
        minimap::MinimapPlugin,
        feedback::FeedbackPlugin,
        audio::AudioPlugin,
        client_shell::ClientShellPlugin,
        persist::PersistPlugin,
    ));
    if !headless {
        app.add_plugins((UnitRenderPlugin, EconomyUiPlugin));
    }
    app
}

#[cfg(test)]
mod produce_hook_observation_tests {
    use super::drive_produce_hook;
    use crate::commands::ClientCommand;
    use crate::{
        commands::{CommandSendStatus, PendingCommands},
        ingest::{ReplicaState, WelcomeInfo},
        replica::{Replica, UnitReplica},
        scenario::ProduceHook,
    };
    use bevy::prelude::{Schedule, World};
    use std::collections::VecDeque;
    use turts_net_protocol::{Pos, ResearchRecord, SlotId, UnitId};

    #[test]
    fn sent_produce_still_observes_first_later_owner_completion_once() {
        let mut replica = Replica::new(20);
        replica.last_applied_tick = Some(10);
        replica.units.insert(
            7,
            UnitReplica {
                content_id: "forge:lab".into(),
                owner_slot: 0,
                facing: None,
                samples: VecDeque::from([(10, Pos { x: 8.0, y: 8.0 })]),
            },
        );
        let mut replica_state = ReplicaState {
            replica: Some(replica),
            welcome: Some(WelcomeInfo {
                slot: 0,
                tick: 10,
                tick_hz: 20,
                match_id: 0,
            }),
            ..Default::default()
        };
        let mut world = World::new();
        world.insert_resource(ProduceHook {
            request: Some((7, "forge:hardened_shells".into())),
            sent: false,
            ..Default::default()
        });
        world.insert_resource(replica_state);
        world.insert_resource(PendingCommands::default());
        world.insert_resource(CommandSendStatus::default());
        let mut schedule = Schedule::default();
        schedule.add_systems(drive_produce_hook);

        schedule.run(&mut world);
        assert!(world.resource::<ProduceHook>().sent);
        assert_eq!(world.resource::<PendingCommands>().0.len(), 1);

        replica_state = world.remove_resource::<ReplicaState>().unwrap();
        let replica = replica_state.replica.as_mut().unwrap();
        replica.last_applied_tick = Some(90);
        replica.research.push(ResearchRecord {
            slot: SlotId(0),
            completed: vec!["forge:hardened_shells".into()],
        });
        world.insert_resource(replica_state);
        schedule.run(&mut world);
        assert_eq!(
            world.resource::<ProduceHook>().research_completed_tick,
            Some(90)
        );
        assert_eq!(world.resource::<PendingCommands>().0.len(), 1);

        let mut replica_state = world.remove_resource::<ReplicaState>().unwrap();
        replica_state.replica.as_mut().unwrap().last_applied_tick = Some(91);
        world.insert_resource(replica_state);
        schedule.run(&mut world);
        assert_eq!(
            world.resource::<ProduceHook>().research_completed_tick,
            Some(90)
        );
        assert_eq!(world.resource::<PendingCommands>().0.len(), 1);
        assert!(matches!(
            world.resource::<PendingCommands>().0.front(),
            Some(crate::commands::PendingCommand::Submit {
                kind: ClientCommand::Produce {
                    producer_id: UnitId(7),
                    ..
                },
                ..
            })
        ));
    }
}
