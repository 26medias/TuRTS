//! Shared F10/Escape settings overlay. Task16 composes its additional controls here.

mod bindings_editor;
mod notices;
mod settings;

use std::collections::BTreeMap;

use bevy::input_focus::FocusedInput;
use bevy::prelude::*;
use bevy::ui_widgets::{
    Activate, Button as UiButton, ButtonPlugin, Slider, SliderRange, SliderStep, SliderThumb,
    SliderValue,
};
use bevy::window::{PrimaryWindow, WindowEvent};
use turts_mod_api::Binding;

use crate::{
    core_adapter::{CoreAdapterSet, CoreEffects, CorePreferences},
    debug_feed::NoticesFeed,
    feedback::FeedbackSet,
    ingest::ReplicaState,
    join::{AcquiredPackages, Phase},
    rejoin::RejoinController,
    ui_shared::{UiCaptureRoot, UiPresentationSet},
    volume_settings::{VolumeSettings, spawn_volume_controls},
};
pub use bindings_editor::BindingLabels;
pub(crate) use bindings_editor::binding_label;
use bindings_editor::*;
use notices::*;
pub(crate) use notices::{ClientNotice, sync_client_notice, sync_rejoin_camera};
pub(crate) use settings::ClientSettings;
use settings::*;

#[derive(Component)]
pub(crate) struct SettingsMenuOverlay;

/// The width of the action label column of the menu. An order name of up to 20 characters
/// ("Disable this machine", 192 px in the 16 px default font) stays on one line, so each
/// single-key row keeps the same height.
const MENU_ACTION_LABEL_WIDTH: f32 = 200.0;
/// The start width of the key column. It grows into the free width of the row; the label, the
/// keys and the two buttons fit in the 476 px of the panel without a shrink.
const MENU_BINDING_BASIS_WIDTH: f32 = 120.0;
/// The fixed height of the binding feedback block: one 14 px status line (20 px), the gap
/// (8 px) and the Swap/Cancel row (24 px). The rows do not move when the text shows or clears.
const BINDING_FEEDBACK_HEIGHT: f32 = 52.0;

#[derive(Component)]
pub(crate) struct RejoinOverlayCamera;

#[derive(Component)]
pub(crate) struct FocusOverlay;

#[derive(Component, Clone)]
struct BindingButton {
    action: String,
    kind: BindingButtonKind,
}

#[derive(Resource, Default)]
pub(crate) struct BindingEditorState {
    pub(crate) capturing: Option<String>,
    conflict: Option<(String, Binding, Vec<String>)>,
    status: Option<String>,
    capture_after_message: Option<usize>,
}

#[derive(Component, Clone)]
struct BindingValueText {
    action: String,
}

/// Own the one settings overlay reached through the interaction core's existing `menu` action.
pub struct ClientShellPlugin;

impl Plugin for ClientShellPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ButtonPlugin>() {
            app.add_plugins(ButtonPlugin);
        }
        if !app.is_plugin_added::<bevy::ui_widgets::ScrollAreaPlugin>() {
            app.add_plugins(bevy::ui_widgets::ScrollAreaPlugin);
        }
        app.add_plugins(crate::client_screens::ClientScreensPlugin);
        app.init_resource::<ClientSettings>()
            .init_resource::<CorePreferences>()
            .init_resource::<BindingEditorState>()
            .init_resource::<BindingInputTimeline>()
            .init_resource::<BindingLabels>()
            .init_resource::<crate::visuals::HealthDisplay>()
            .add_message::<WindowEvent>()
            .add_systems(
                First,
                record_binding_input_timeline
                    .after(bevy::ecs::message::MessageUpdateSystems)
                    .before(bevy::picking::PickingSystems::Input),
            )
            .add_systems(
                Update,
                (apply_client_setting_sliders, apply_client_settings)
                    .chain()
                    .after(crate::audio::SettingsLoadSet)
                    .before(CoreAdapterSet),
            )
            .add_systems(Update, sync_settings_toggle_labels)
            .add_systems(
                Update,
                sync_interaction_settings
                    .after(crate::audio::SettingsLoadSet)
                    .after(apply_client_setting_sliders)
                    .before(CoreAdapterSet),
            )
            .add_systems(
                Update,
                update_pointer_lock
                    .after(crate::audio::SettingsLoadSet)
                    .after(apply_client_setting_sliders)
                    .before(CoreAdapterSet),
            )
            .add_observer(on_binding_pointer_press)
            .add_observer(on_binding_pointer_click)
            .add_observer(on_binding_keyboard_activation)
            .add_observer(on_binding_button_activated);
        app.add_observer(on_settings_toggle_activated);
        app.add_systems(
            Update,
            toggle_settings_menu
                .after(CoreAdapterSet)
                .before(UiPresentationSet),
        );
        app.add_systems(
            Update,
            (capture_physical_binding, sync_binding_feedback)
                .chain()
                .after(toggle_settings_menu)
                .before(UiPresentationSet),
        )
        .add_systems(
            Update,
            sync_binding_value_labels
                .after(toggle_settings_menu)
                .after(capture_physical_binding),
        );
        app.add_systems(
            Update,
            sync_client_notice
                .after(FeedbackSet)
                .before(UiPresentationSet),
        );
        app.add_systems(
            Update,
            sync_focus_overlay
                .after(sync_client_notice)
                .after(CoreAdapterSet)
                .before(UiPresentationSet),
        );
        app.add_systems(
            Update,
            sync_rejoin_camera
                .after(sync_focus_overlay)
                .before(UiPresentationSet),
        );
        app.init_resource::<SoundLockTimer>().add_systems(
            Update,
            (
                sync_sound_off_notice.after(sync_focus_overlay),
                sync_sound_status_label.after(toggle_settings_menu),
            )
                .before(UiPresentationSet),
        );
    }
}

#[cfg(test)]
pub(crate) mod tests;
