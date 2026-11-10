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

/// Add the Task15 volume controls as children of the existing F10/Esc settings overlay.
/// Task16 can append its settings controls to the same parent entity.
pub fn spawn_volume_controls(commands: &mut Commands, parent: Entity, settings: VolumeSettings) {
    commands.entity(parent).with_children(|parent| {
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
    sliders: Query<(&SliderValue, &Children, &ComputedNode)>,
    mut thumbs: Query<(&mut Node, &ComputedNode), With<SliderThumb>>,
) {
    for (value, children, track) in &sliders {
        for child in children {
            if let Ok((mut node, thumb)) = thumbs.get_mut(*child) {
                let travel = (track.size().x - thumb.size().x).max(0.0);
                node.left = Val::Px(value.0 * travel);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadedSettings {
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
            },
            Err(_) => LoadedSettings {
                volumes: VolumeSettings::default(),
                notice: Some(CORRUPT_NOTICE.into()),
            },
        }
    }
    pub fn set_volume(&self, category: VolumeCategory, value: f32) -> Result<(), String> {
        let current = match fs::read_to_string(&self.path) {
            Ok(json) => Some(json),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("read settings {}: {error}", self.path.display())),
        };
        let updated = set_volume_in_json(current.as_deref(), category, value)?;
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
                volumes: VolumeSettings::default(),
                notice: Some(CORRUPT_NOTICE.into()),
            };
        }
    };
    let Some(audio) = value.get("audio") else {
        return LoadedSettings {
            volumes: VolumeSettings::default(),
            notice: None,
        };
    };
    if !audio.is_object() {
        return LoadedSettings {
            volumes: VolumeSettings::default(),
            notice: Some(CORRUPT_NOTICE.into()),
        };
    }
    let defaults = VolumeSettings::default();
    let parsed: Result<VolumeSettings, ()> = (|| {
        Ok(VolumeSettings {
            master: parse_volume(audio.get("master"), defaults.master)?,
            voice: parse_volume(audio.get("voice"), defaults.voice)?,
            alerts: parse_volume(audio.get("alerts"), defaults.alerts)?,
            effects: parse_volume(audio.get("effects"), defaults.effects)?,
        })
    })();
    match parsed {
        Ok(volumes) => LoadedSettings {
            volumes,
            notice: None,
        },
        Err(()) => LoadedSettings {
            volumes: defaults,
            notice: Some(CORRUPT_NOTICE.into()),
        },
    }
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
        return std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".config"))
            });
    }
    #[allow(unreachable_code)]
    None
}
