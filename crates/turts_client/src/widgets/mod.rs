//! Mod-authored retained UI tree and the allowlisted source/primitive adapters.

mod activation;
pub mod assets;
pub mod cursor;
pub mod guide;
pub mod hit;
pub mod placement;
pub mod primitives;
pub mod snapshot;
pub mod sources;
pub mod tooltip;
mod tree;
mod values;

use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button, observe};
use bevy::window::PrimaryWindow;
use turts_mod_api::{
    Anchor, ButtonAction, ButtonIcon, ContentId, Fill, ItemField, Predicate, Primitive, Size,
    TextValue, Value, Widget,
};

use crate::{
    core_adapter::{CoreInputQueue, FrameCloseSet},
    debug_feed::{AlertsFeed, GroupsFeed, NoticesFeed, SelectionFeed},
    ingest::ReplicaState,
    join::{AcquiredPackages, Phase},
    ui_shared::{UiCaptureRoot, UiPresentationSet},
};
use activation::*;
use tree::*;
use values::*;

#[derive(Resource, Default)]
struct WidgetTree {
    key: String,
    entities: Vec<Entity>,
}

#[derive(Component, Clone)]
struct WidgetBinding {
    primitive: Primitive,
    at: turts_mod_api::At,
    size: Size,
    item: Option<sources::ItemData>,
    tooltip: Option<Value>,
    row_tooltip: Option<Value>,
    visible: Vec<Predicate>,
    parent_size: Vec2,
    package: String,
    tooltip_style: turts_mod_api::TooltipStyle,
}

#[derive(Component, Clone)]
struct WidgetListRow {
    owner: Entity,
    key: String,
    template: Widget,
    index: usize,
}

fn widget_visibility(visible: bool) -> Visibility {
    if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

/// Retained widget runtime. All visible nodes and sources come from the accepted interface.
pub struct WidgetsPlugin;
impl Plugin for WidgetsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy::ui_widgets::ButtonPlugin>() {
            app.add_plugins(bevy::ui_widgets::ButtonPlugin);
        }
        if !app.is_plugin_added::<bevy::ui_widgets::ScrollAreaPlugin>() {
            app.add_plugins(bevy::ui_widgets::ScrollAreaPlugin);
        }
        app.init_resource::<guide::GuideOpen>()
            .init_resource::<WidgetTree>()
            .init_resource::<cursor::CursorState>()
            .init_resource::<assets::WidgetAssets>()
            .add_systems(
                Update,
                (
                    observe_guide_phase,
                    sync_widget_tree,
                    update_fit_layout,
                    update_widget_button_states,
                )
                    .chain()
                    .in_set(UiPresentationSet)
                    .after(crate::present::update_presented),
            )
            .add_systems(
                Update,
                assets::load_widget_assets
                    .run_if(resource_exists::<Assets<Image>>)
                    .run_if(resource_exists::<Assets<Font>>)
                    .before(sync_widget_tree),
            )
            .add_systems(
                Update,
                consume_shell_actions
                    .run_if(resource_exists::<crate::core_adapter::CoreEffects>)
                    .after(crate::core_adapter::CoreAdapterSet)
                    .after(observe_guide_phase)
                    .before(sync_widget_tree),
            )
            .add_systems(
                Last,
                cursor::update_cursor
                    .before(FrameCloseSet)
                    .before(bevy::window::CursorSystems::Update),
            )
            .add_systems(
                Last,
                tooltip::update_tooltips
                    .before(cursor::update_cursor)
                    .before(snapshot::publish_snapshot),
            )
            .add_systems(Last, snapshot::publish_snapshot.before(FrameCloseSet));
    }
}

fn observe_guide_phase(phase: Res<Phase>, mut guide: ResMut<guide::GuideOpen>) {
    guide::observe_phase(&phase, &mut guide);
}

fn consume_shell_actions(
    effects: Res<crate::core_adapter::CoreEffects>,
    mut guide: ResMut<guide::GuideOpen>,
    mut volumes: Option<ResMut<crate::volume_settings::VolumeSettings>>,
) {
    if !effects.is_changed() {
        return;
    }
    for effect in &effects.0 {
        let turts_interaction::Effect::ShellAction { action } = effect else {
            continue;
        };
        if action == "toggle_guide" {
            guide::toggle_guide(&mut guide);
        } else if action == "toggle_sound"
            && let Some(volumes) = volumes.as_mut()
        {
            // A key that a mod bound to the engine action `toggle_sound` (`engine_overrides`).
            volumes.sound_on = !volumes.sound_on;
        }
    }
}

#[cfg(test)]
mod tests;
