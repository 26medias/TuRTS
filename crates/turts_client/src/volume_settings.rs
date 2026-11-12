//! Task15 volume controls and their share of the client settings document.

use bevy::prelude::*;
use bevy::ui_widgets::{Slider, SliderRange, SliderStep, SliderThumb, SliderValue};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const WEB_SETTINGS_KEY: &str = "turts.v1.settings";
const CORRUPT_NOTICE: &str = "Settings were unreadable; default settings are in use.";

/// Resolve `<user config dir>/turts/settings.json` for native clients.
pub fn native_settings_path(user_config_dir: &Path) -> PathBuf {
    user_config_dir.join("turts").join("settings.json")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VolumeCategory {
    Master,
    Voice,
    Alerts,
    Effects,
}

/// Marker attached to the four volume sliders so the shared menu can host them directly.
#[derive(Component, Clone, Copy, Debug)]
pub struct VolumeControl(pub VolumeCategory);

impl VolumeCategory {
    fn key(self) -> &'static str {
        match self {
            Self::Master => "master",
            Self::Voice => "voice",
            Self::Alerts => "alerts",
            Self::Effects => "effects",
        }
    }
}

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct VolumeSettings {
    pub master: f32,
    pub voice: f32,
    pub alerts: f32,
    pub effects: f32,
}

impl Default for VolumeSettings {
    fn default() -> Self {
        Self {
            master: 1.0,
            voice: 1.0,
            alerts: 1.0,
            effects: 1.0,
        }
    }
}

impl VolumeSettings {
    pub fn value(self, category: VolumeCategory) -> f32 {
        match category {
            VolumeCategory::Master => self.master,
            VolumeCategory::Voice => self.voice,
            VolumeCategory::Alerts => self.alerts,
            VolumeCategory::Effects => self.effects,
        }
    }
    /// Effective category gain after the master control.
    pub fn gain(self, category: VolumeCategory) -> f32 {
        match category {
            VolumeCategory::Master => self.master,
            _ => self.master * self.value(category),
        }
    }
    fn set(&mut self, category: VolumeCategory, value: f32) {
        let value = normalized(value);
        match category {
            VolumeCategory::Master => self.master = value,
            VolumeCategory::Voice => self.voice = value,
            VolumeCategory::Alerts => self.alerts = value,
            VolumeCategory::Effects => self.effects = value,
        }
    }
}

fn normalized(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

/// The menu row "Sound: off" or "Sound: on" (design 10.1, ruling O15).
#[derive(Component, Clone, Copy, Debug)]
pub struct SoundStatusText;

/// Add the Task15 volume controls as children of the existing F10/Esc settings overlay.
/// Task16 can append its settings controls to the same parent entity. `sound_status` is the
/// first text of the sound state label; the shell keeps it current. The label is on the line of
/// the "Audio levels" heading, so the menu layout keeps its height.
pub fn spawn_volume_controls(
    commands: &mut Commands,
    parent: Entity,
    settings: VolumeSettings,
    sound_status: &str,
) {
    commands.entity(parent).with_children(|parent| {
        parent
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                ..default()
            })
            .with_children(|heading| {
                heading.spawn((
                    Text::new("Audio levels"),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.78, 0.84, 0.9)),
                ));
                heading.spawn((
                    SoundStatusText,
                    Text::new(sound_status),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        for (label, category) in [
            ("Master", VolumeCategory::Master),
            ("Voice", VolumeCategory::Voice),
            ("Alerts", VolumeCategory::Alerts),
            ("Effects", VolumeCategory::Effects),
        ] {
            let value = settings.value(category);
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(34.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Text::new(label),
                        Node {
                            width: Val::Px(74.0),
                            ..default()
                        },
                    ));
                    row.spawn((
                        Node {
                            width: Val::Px(0.0),
                            min_width: Val::Px(0.0),
                            flex_grow: 1.0,
                            flex_shrink: 1.0,
                            height: Val::Px(12.0),
                            position_type: PositionType::Relative,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.20, 0.24, 0.30)),
                        Slider {
                            track_click: bevy::ui_widgets::TrackClick::Snap,
                            ..default()
                        },
                        SliderRange::new(0.0, 1.0),
                        SliderStep(0.01),
                        SliderValue(value),
                        VolumeControl(category),
                    ))
                    .with_children(|track| {
                        track.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Percent(value * 100.0),
                                top: Val::Px(-4.0),
                                width: Val::Px(20.0),
                                height: Val::Px(20.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.40, 0.75, 0.95)),
                            SliderThumb,
                        ));
                    });
                });
        }
    });
}

/// Slider input changes the existing settings resource; persistence and live gain updates consume it.
pub fn apply_volume_control_changes(
    mut settings: ResMut<VolumeSettings>,
    changed: Query<(&VolumeControl, &SliderValue), Changed<SliderValue>>,
) {
    for (control, value) in &changed {
        settings.set(control.0, value.0);
    }
}

/// Keep the displayed slider values in sync after loading saved settings or a page reload.
pub fn sync_volume_controls(
    settings: Res<VolumeSettings>,
    mut commands: Commands,
    sliders: Query<(Entity, &VolumeControl, &SliderValue)>,
) {
    if !settings.is_changed() {
        return;
    }
    for (entity, control, value) in &sliders {
        let saved = settings.value(control.0);
        if (value.0 - saved).abs() > f32::EPSILON {
            commands.entity(entity).insert(SliderValue(saved));
        }
    }
}

pub fn sync_volume_thumb_positions(
    sliders: Query<(&SliderValue, &SliderRange, &Children, &ComputedNode)>,
    mut thumbs: Query<(&mut Node, &ComputedNode), With<SliderThumb>>,
) {
    for (value, range, children, track) in &sliders {
        for child in children {
            if let Ok((mut node, thumb)) = thumbs.get_mut(*child) {
                let travel =
                    (track.size().x - thumb.size().x).max(0.0) * track.inverse_scale_factor();
                node.left = Val::Px(range.thumb_position(range.clamp(value.0)) * travel);
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoadedSettings {
    pub(crate) controls: crate::client_preferences::ControlSettings,
    pub volumes: VolumeSettings,
    pub notice: Option<String>,
}

pub struct SettingsStore {
    path: PathBuf,
}
impl SettingsStore {
    pub fn for_path(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn load(&self) -> LoadedSettings {
        match fs::read_to_string(&self.path) {
            Ok(json) => decode_settings_json(&json),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => LoadedSettings {
                volumes: VolumeSettings::default(),
                notice: None,
                ..Default::default()
            },
            Err(_) => LoadedSettings {
                volumes: VolumeSettings::default(),
                notice: Some(CORRUPT_NOTICE.into()),
                ..Default::default()
            },
        }
    }
    pub fn set_volume(&self, category: VolumeCategory, value: f32) -> Result<(), String> {
        let current = self.read_document()?;
        let updated = set_volume_in_json(current.as_deref(), category, value)?;
        self.write_document(&updated)
    }

    pub(crate) fn set_controls(
        &self,
        controls: &crate::client_preferences::ControlSettings,
    ) -> Result<(), String> {
        let current = self.read_document()?;
        let updated = set_controls_in_json(current.as_deref(), controls)?;
        self.write_document(&updated)
    }

    fn read_document(&self) -> Result<Option<String>, String> {
        match fs::read_to_string(&self.path) {
            Ok(json) => Ok(Some(json)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("read settings {}: {error}", self.path.display())),
        }
    }

    fn write_document(&self, updated: &str) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| "settings path has no parent".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("create settings directory {}: {error}", parent.display()))?;
        let temporary = self.path.with_extension("json.tmp");
        fs::write(&temporary, updated)
            .map_err(|error| format!("write settings {}: {error}", temporary.display()))?;
        fs::rename(&temporary, &self.path)
            .map_err(|error| format!("replace settings {}: {error}", self.path.display()))
    }
}

fn parse_volume(value: Option<&serde_json::Value>, fallback: f32) -> Result<f32, ()> {
    let Some(value) = value else {
        return Ok(fallback);
    };
    let Some(number) = value.as_f64() else {
        return Err(());
    };
    if !number.is_finite() || !(0.0..=1.0).contains(&number) {
        return Err(());
    }
    Ok(number as f32)
}

pub fn decode_settings_json(json: &str) -> LoadedSettings {
    let value: serde_json::Value = match serde_json::from_str::<serde_json::Value>(json) {
        Ok(value) if value.is_object() => value,
        _ => {
            return LoadedSettings {
                notice: Some(CORRUPT_NOTICE.into()),
                ..Default::default()
            };
        }
    };
    let parsed = (|| -> Result<_, ()> {
        let defaults = VolumeSettings::default();
        let volumes = match value.get("audio") {
            None => defaults,
            Some(audio) if audio.is_object() => VolumeSettings {
                master: parse_volume(audio.get("master"), defaults.master)?,
                voice: parse_volume(audio.get("voice"), defaults.voice)?,
                alerts: parse_volume(audio.get("alerts"), defaults.alerts)?,
                effects: parse_volume(audio.get("effects"), defaults.effects)?,
            },
            Some(_) => return Err(()),
        };
        let controls = match value.get("controls") {
            None => crate::client_preferences::ControlSettings::default(),
            Some(value) => {
                serde_json::from_value::<crate::client_preferences::ControlSettings>(value.clone())
                    .map_err(|_| ())?
            }
        };
        if !controls.valid() {
            return Err(());
        }
        Ok((volumes, controls))
    })();
    match parsed {
        Ok((volumes, controls)) => LoadedSettings {
            volumes,
            controls,
            notice: None,
        },
        Err(()) => LoadedSettings {
            notice: Some(CORRUPT_NOTICE.into()),
            ..Default::default()
        },
    }
}

pub(crate) fn set_controls_in_json(
    json: Option<&str>,
    controls: &crate::client_preferences::ControlSettings,
) -> Result<String, String> {
    if !controls.valid() {
        return Err("client settings are outside their supported ranges".into());
    }
    let mut root = json
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    let mut object = root
        .get("controls")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    let known = serde_json::to_value(controls)
        .map_err(|error| format!("serialize client settings: {error}"))?;
    for (key, value) in known
        .as_object()
        .ok_or_else(|| "client settings are not an object".to_string())?
    {
        object.insert(key.clone(), value.clone());
    }
    root.as_object_mut()
        .ok_or_else(|| "settings document is not an object".to_string())?
        .insert("controls".into(), serde_json::Value::Object(object));
    serde_json::to_string(&root).map_err(|error| format!("serialize settings: {error}"))
}

pub fn set_volume_in_json(
    json: Option<&str>,
    category: VolumeCategory,
    value: f32,
) -> Result<String, String> {
    let mut root = match json {
        Some(json) => serde_json::from_str::<serde_json::Value>(json)
            .unwrap_or_else(|_| serde_json::json!({})),
        None => serde_json::json!({}),
    };
    if !root.is_object() {
        root = serde_json::json!({});
    }
    let object = root
        .as_object_mut()
        .expect("non-object settings are replaced with an empty document");
    if !object.contains_key("audio") {
        object.insert("audio".into(), serde_json::json!({}));
    }
    if !object
        .get("audio")
        .is_some_and(serde_json::Value::is_object)
    {
        object.insert("audio".into(), serde_json::json!({}));
    }
    let audio = object
        .get_mut("audio")
        .and_then(serde_json::Value::as_object_mut)
        .expect("corrupt audio settings are replaced with an empty object");
    audio.insert(category.key().into(), serde_json::json!(normalized(value)));
    serde_json::to_string(&root).map_err(|error| format!("serialize settings: {error}"))
}

#[cfg(target_family = "wasm")]
pub(crate) fn read_web_settings() -> Result<Option<String>, String> {
    let window = web_sys::window().ok_or_else(|| "browser window is unavailable".to_string())?;
    let storage = window
        .local_storage()
        .map_err(|_| "browser settings storage is unavailable".to_string())?
        .ok_or_else(|| "browser settings storage is unavailable".to_string())?;
    storage
        .get_item(WEB_SETTINGS_KEY)
        .map_err(|_| "browser settings could not be read".to_string())
}

#[cfg(target_family = "wasm")]
pub(crate) fn write_web_settings(value: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or_else(|| "browser window is unavailable".to_string())?;
    let storage = window
        .local_storage()
        .map_err(|_| "browser settings storage is unavailable".to_string())?
        .ok_or_else(|| "browser settings storage is unavailable".to_string())?;
    storage
        .set_item(WEB_SETTINGS_KEY, value)
        .map_err(|_| "browser settings could not be written".to_string())
}

pub(crate) fn native_user_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("APPDATA").map(PathBuf::from);
    }
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("Library").join("Application Support"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return native_user_config_dir_from(
            std::env::var_os("XDG_CONFIG_HOME").as_deref(),
            std::env::var_os("HOME").as_deref(),
        );
    }
    #[allow(unreachable_code)]
    None
}

#[cfg(all(unix, not(target_os = "macos")))]
fn native_user_config_dir_from(
    xdg_config_home: Option<&std::ffi::OsStr>,
    home_dir: Option<&std::ffi::OsStr>,
) -> Option<PathBuf> {
    crate::persist::native_config_dir_from(xdg_config_home.map(Path::new), home_dir.map(Path::new))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn empty_xdg_config_home_uses_the_injected_home_fallback() {
        let home = Path::new("/isolated/task16-home");
        assert_eq!(
            native_user_config_dir_from(Some(std::ffi::OsStr::new("")), Some(home.as_os_str())),
            Some(home.join(".config")),
            "an empty XDG_CONFIG_HOME must not resolve relative settings under the cwd"
        );
    }

    #[test]
    fn task16_actual_slider_thumbs_follow_their_ranges_at_each_layout_scale() {
        for scale in [1.0, 1.5, 2.0, 3.0] {
            for (minimum, maximum, value, fraction) in [
                (0.0, 1.0, 0.8, 0.8),
                (0.25, 3.0, 1.0, 0.75 / 2.75),
                (0.75, 2.0, 1.0, 0.2),
                (100.0, 1000.0, 400.0, 1.0 / 3.0),
                (100.0, 1000.0, 2000.0, 1.0),
            ] {
                let mut app = App::new();
                app.add_systems(Update, sync_volume_thumb_positions);
                let thumb = app
                    .world_mut()
                    .spawn((
                        Node::default(),
                        SliderThumb,
                        ComputedNode {
                            size: Vec2::new(20.0 * scale, 20.0 * scale),
                            inverse_scale_factor: 1.0 / scale,
                            ..default()
                        },
                    ))
                    .id();
                app.world_mut()
                    .spawn((
                        SliderValue(value),
                        SliderRange::new(minimum, maximum),
                        ComputedNode {
                            size: Vec2::new(200.0 * scale, 12.0 * scale),
                            inverse_scale_factor: 1.0 / scale,
                            ..default()
                        },
                    ))
                    .add_child(thumb);
                app.update();
                let Val::Px(left) = app.world().get::<Node>(thumb).unwrap().left else {
                    panic!("slider thumb needs logical-pixel position")
                };
                assert!(
                    (left - 180.0 * fraction).abs() < 0.001,
                    "range {minimum}..{maximum}, value {value}, layout scale {scale}: expected {}, found {left}",
                    180.0 * fraction
                );
            }
        }
    }

    #[test]
    fn changed_slider_applies_only_its_category_to_the_live_settings_resource() {
        let initial = VolumeSettings {
            master: 0.8,
            voice: 0.3,
            alerts: 0.4,
            effects: 0.5,
        };
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(initial)
            .add_systems(Update, apply_volume_control_changes);
        app.world_mut()
            .spawn((VolumeControl(VolumeCategory::Voice), SliderValue(0.65)));

        app.update();

        assert_eq!(
            *app.world().resource::<VolumeSettings>(),
            VolumeSettings {
                master: 0.8,
                voice: 0.65,
                alerts: 0.4,
                effects: 0.5,
            },
            "editing voice must preserve master, alerts, and effects"
        );
    }

    #[test]
    fn loaded_settings_are_applied_back_to_the_existing_category_slider() {
        let settings = VolumeSettings {
            master: 0.8,
            voice: 0.3,
            alerts: 0.4,
            effects: 0.5,
        };
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(settings)
            .add_systems(Update, sync_volume_controls);
        let slider = app
            .world_mut()
            .spawn((VolumeControl(VolumeCategory::Alerts), SliderValue(0.9)))
            .id();

        app.update();

        assert_eq!(
            app.world().entity(slider).get::<SliderValue>(),
            Some(&SliderValue(0.4)),
            "the existing alerts control reflects the effective loaded setting"
        );
    }
}
