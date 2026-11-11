//! Player bindings and match-scoped group persistence.

use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use bevy::prelude::*;

use turts_mod_api::{Binding, PackageId};

const CORRUPT_NOTICE: &str = "Player settings were unreadable; defaults are in use.";
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

/// Bindings loaded from one mod-and-compatibility namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedBindings {
    pub actions: BTreeMap<String, Vec<Binding>>,
    pub notice: Option<String>,
}

/// Groups loaded from the current match and player slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedGroups {
    pub groups: BTreeMap<u8, Vec<u32>>,
    pub notice: Option<String>,
}

/// Isolated native root override used by the persistence owner and its filesystem contracts.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct PersistenceConfigRoot(pub Option<PathBuf>);

/// Load the player's bindings from one mod-and-compatibility namespace.
pub fn load_native_bindings(path: &Path) -> LoadedBindings {
    let value = match read_json(path) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return LoadedBindings {
                actions: BTreeMap::new(),
                notice: None,
            };
        }
        Err(()) => {
            return LoadedBindings {
                actions: BTreeMap::new(),
                notice: Some(CORRUPT_NOTICE.into()),
            };
        }
    };
    loaded_bindings(value)
}

fn loaded_bindings(value: serde_json::Value) -> LoadedBindings {
    let actions = value
        .get("actions")
        .cloned()
        .and_then(|value| serde_json::from_value::<BTreeMap<String, Vec<Binding>>>(value).ok());
    match actions {
        Some(actions)
            if actions.keys().all(|name| !name.trim().is_empty())
                && actions.values().flatten().all(valid_persisted_binding) =>
        {
            LoadedBindings {
                actions,
                notice: None,
            }
        }
        _ => LoadedBindings {
            actions: BTreeMap::new(),
            notice: Some(CORRUPT_NOTICE.into()),
        },
    }
}

fn valid_persisted_binding(binding: &Binding) -> bool {
    let mods = match binding {
        Binding::Key { mods, .. } | Binding::Pointer { mods, .. } => mods,
        Binding::Wheel(_) => return true,
    };
    if mods
        .iter()
        .enumerate()
        .any(|(index, modifier)| mods[..index].contains(modifier))
    {
        return false;
    }
    let Binding::Key { code, .. } = binding else {
        return true;
    };
    if !turts_mod_api::KNOWN_KEY_CODES.contains(&code.as_str()) {
        return false;
    }
    !turts_mod_api::RESERVED_BROWSER_KEYS
        .iter()
        .any(|(reserved_code, reserved_mods)| {
            *reserved_code == code
                && reserved_mods.len() == mods.len()
                && reserved_mods.iter().all(|modifier| mods.contains(modifier))
        })
}

/// Persist binding overrides in the player's native settings file.
pub fn save_native_bindings(
    path: &Path,
    actions: &BTreeMap<String, Vec<Binding>>,
) -> io::Result<()> {
    let value = serde_json::json!({"actions": actions});
    write_json(path, &value)
}

/// Load the group assignments scoped to one match and player slot.
pub fn load_native_groups(path: &Path) -> LoadedGroups {
    let value = match read_json(path) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return LoadedGroups {
                groups: BTreeMap::new(),
                notice: None,
            };
        }
        Err(()) => {
            return LoadedGroups {
                groups: BTreeMap::new(),
                notice: Some(CORRUPT_NOTICE.into()),
            };
        }
    };
    loaded_groups(value)
}

fn loaded_groups(value: serde_json::Value) -> LoadedGroups {
    let groups = value
        .get("groups")
        .cloned()
        .and_then(|value| serde_json::from_value::<BTreeMap<u8, Vec<u32>>>(value).ok());
    match groups {
        Some(groups) if groups.keys().all(|group| (1..=9).contains(group)) => LoadedGroups {
            groups,
            notice: None,
        },
        _ => LoadedGroups {
            groups: BTreeMap::new(),
            notice: Some(CORRUPT_NOTICE.into()),
        },
    }
}

/// Persist the group assignments for one match and player slot.
pub fn save_native_groups(path: &Path, groups: &BTreeMap<u8, Vec<u32>>) -> io::Result<()> {
    if groups.keys().any(|group| !(1..=9).contains(group)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "control group number must be between 1 and 9",
        ));
    }
    let value = serde_json::json!({"groups": groups});
    write_json(path, &value)
}

/// Derive the design's compatibility line without converting version components
/// through a floating-point representation.
///
/// Derive the design compatibility line without rounding 64-bit version components.
pub fn compatibility_line(version: &str) -> Option<String> {
    let (core, build) = version
        .split_once('+')
        .map_or((version, None), |(core, build)| (core, Some(build)));
    if build.is_some_and(|build| !valid_version_identifiers(build, false)) {
        return None;
    }
    let (core, prerelease) = core
        .split_once('-')
        .map_or((core, None), |(core, prerelease)| (core, Some(prerelease)));
    if prerelease.is_some_and(|part| !valid_version_identifiers(part, true)) {
        return None;
    }
    let mut parts = core.split('.');
    let major = parse_version_component(parts.next()?)?;
    let minor = parse_version_component(parts.next()?)?;
    let patch = parse_version_component(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    let _ = patch;
    Some(if major >= 1 {
        major.to_string()
    } else {
        format!("0.{minor}")
    })
}

/// Select the native configuration root from already-read platform paths.
/// Tests pass isolated paths here and never mutate HOME or XDG_CONFIG_HOME.
pub fn native_config_dir_from(
    xdg_config_home: Option<&Path>,
    home_dir: Option<&Path>,
) -> Option<PathBuf> {
    xdg_config_home
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .or_else(|| home_dir.map(|home| home.join(".config")))
}

/// Return the design's native and browser binding namespace for a verified mod
/// ID and semantic version.
pub fn native_bindings_path(
    config_dir: &Path,
    root_mod_id: &str,
    root_version: &str,
) -> Option<PathBuf> {
    let package = PackageId::parse(root_mod_id).ok()?;
    let line = compatibility_line(root_version)?;
    Some(
        config_dir
            .join("turts")
            .join(format!("bindings-{}-{line}.json", package.0)),
    )
}

pub fn web_bindings_key(root_mod_id: &str, root_version: &str) -> Option<String> {
    let package = PackageId::parse(root_mod_id).ok()?;
    let line = compatibility_line(root_version)?;
    Some(format!("turts.v1.bindings.{}.{line}", package.0))
}

/// Return the design's match-and-slot scoped group namespace.
pub fn native_groups_path(config_dir: &Path, match_id: u64, slot: u8) -> PathBuf {
    config_dir
        .join("turts")
        .join(format!("groups-{match_id}-{slot}.json"))
}

pub fn web_groups_key(match_id: u64, slot: u8) -> String {
    format!("turts.v1.groups.{match_id}.{slot}")
}

fn parse_version_component(component: &str) -> Option<u64> {
    if component.is_empty()
        || (component.len() > 1 && component.starts_with('0'))
        || !component.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    component.parse().ok()
}

fn valid_version_identifiers(part: &str, prerelease: bool) -> bool {
    !part.is_empty()
        && part.split('.').all(|identifier| {
            !identifier.is_empty()
                && identifier
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && (!prerelease
                    || !identifier.bytes().all(|byte| byte.is_ascii_digit())
                    || identifier == "0"
                    || !identifier.starts_with('0'))
        })
}

fn read_json(path: &Path) -> Result<Option<serde_json::Value>, ()> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(()),
    };
    let value = serde_json::from_str::<serde_json::Value>(&text).map_err(|_| ())?;
    if !value.is_object() {
        return Err(());
    }
    Ok(Some(value))
}

fn write_json(path: &Path, value: &serde_json::Value) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "settings path has no parent")
    })?;
    fs::create_dir_all(parent)?;
    let bytes = serde_json::to_vec(value)
        .map_err(|error| io::Error::other(format!("serialize settings: {error}")))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "settings path has no file name",
            )
        })?
        .to_string_lossy();
    let (temporary, mut file) = loop {
        let nonce = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(".{file_name}.{}.{}.tmp", std::process::id(), nonce));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&temporary) {
            Ok(file) => break (temporary, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let result = (|| {
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Native settings and per-match groups persistence owner.
pub struct PersistPlugin;
impl bevy::app::Plugin for PersistPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PersistenceConfigRoot>()
            .init_resource::<PersistenceRuntime>()
            .init_resource::<crate::core_adapter::CorePreferences>();
        #[cfg(not(target_family = "wasm"))]
        app.add_systems(
            Update,
            (load_verified_bindings, restore_groups_after_snapshot)
                .chain()
                .before(crate::core_adapter::CoreAdapterSet),
        )
        .add_systems(
            PostUpdate,
            (save_binding_overrides, save_group_assignments).chain(),
        );
        #[cfg(target_family = "wasm")]
        app.add_systems(
            Update,
            (load_web_bindings, restore_web_groups_after_snapshot)
                .chain()
                .before(crate::core_adapter::CoreAdapterSet),
        )
        .add_systems(PostUpdate, (save_web_bindings, save_web_groups).chain());
    }
}

impl Default for PersistenceConfigRoot {
    fn default() -> Self {
        #[cfg(not(target_family = "wasm"))]
        let root = crate::volume_settings::native_user_config_dir();
        #[cfg(target_family = "wasm")]
        let root = None;
        Self(root)
    }
}

#[derive(Resource, Default)]
struct PersistenceRuntime {
    binding_namespace: Option<String>,
    binding_snapshot: BTreeMap<String, Vec<Binding>>,
    group_namespace: Option<(u64, u8)>,
    group_snapshot: BTreeMap<u8, Vec<u32>>,
}

fn load_verified_bindings(
    acquired: Option<Res<crate::join::AcquiredPackages>>,
    root: Res<PersistenceConfigRoot>,
    mut preferences: ResMut<crate::core_adapter::CorePreferences>,
    mut state: ResMut<PersistenceRuntime>,
    mut notices: Option<ResMut<crate::debug_feed::NoticesFeed>>,
) {
    let Some(acquired) = acquired else { return };
    let Some(verified) = acquired.0.verified_root_mod.as_ref() else {
        return;
    };
    let Some(path) = root
        .0
        .as_deref()
        .and_then(|root| native_bindings_path(root, &verified.id, &verified.version))
    else {
        return;
    };
    let namespace = path.to_string_lossy().into_owned();
    if state.binding_namespace.as_deref() == Some(&namespace) {
        return;
    }

    let loaded = load_native_bindings(&path);
    let valid_actions: std::collections::HashSet<String> =
        turts_interaction::input_map::effective_actions(&acquired.0.content.input_map)
            .into_keys()
            .collect();
    preferences.runtime_bindings = loaded
        .actions
        .into_iter()
        .filter(|(action, _)| valid_actions.contains(action))
        .collect();
    state.binding_namespace = Some(namespace);
    state
        .binding_snapshot
        .clone_from(&preferences.runtime_bindings);
    if let Some(notice) = loaded.notice
        && let Some(notices) = notices.as_deref_mut()
    {
        push_notice(&mut notices.0, &notice);
    }
}

/// Delete only canonical group documents owned by other matches in this config root.
fn prune_old_native_group_namespaces(root: &Path, current_match: u64) -> io::Result<()> {
    let directory = root.join("turts");
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(parts) = name
            .strip_prefix("groups-")
            .and_then(|name| name.strip_suffix(".json"))
        else {
            continue;
        };
        let Some((match_text, slot_text)) = parts.split_once('-') else {
            continue;
        };
        let (Ok(match_id), Ok(slot)) = (match_text.parse::<u64>(), slot_text.parse::<u8>()) else {
            continue;
        };
        if match_id == current_match || name != format!("groups-{match_id}-{slot}.json") {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_file() || file_type.is_symlink() {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn restore_groups_after_snapshot(
    phase: Option<Res<crate::join::Phase>>,
    state: Option<Res<crate::ingest::ReplicaState>>,
    root: Res<PersistenceConfigRoot>,
    mut persistence: ResMut<PersistenceRuntime>,
    mut input: Option<ResMut<crate::core_adapter::CoreInputQueue>>,
    mut notices: Option<ResMut<crate::debug_feed::NoticesFeed>>,
) {
    let Some(crate::join::Phase::Joined { slot }) = phase.as_deref() else {
        return;
    };
    let (Some(state), Some(input)) = (state, input.as_deref_mut()) else {
        return;
    };
    if state.replica.as_ref().is_none_or(|replica| {
        replica.snapshot_tick.is_none() || replica.last_applied_tick.is_none()
    }) {
        return;
    }
    let Some(welcome) = state.welcome else { return };
    if welcome.slot != *slot {
        return;
    }
    let namespace = (welcome.match_id, welcome.slot);
    if persistence.group_namespace == Some(namespace) {
        return;
    }
    let Some(root) = root.0.as_deref() else {
        return;
    };
    if let Err(error) = prune_old_native_group_namespaces(root, welcome.match_id)
        && let Some(notices) = notices.as_deref_mut()
    {
        push_notice(
            &mut notices.0,
            &format!("Old control groups could not be removed: {error}"),
        );
    }
    let path = native_groups_path(root, welcome.match_id, welcome.slot);
    let loaded = load_native_groups(&path);
    let groups: Vec<_> = loaded
        .groups
        .iter()
        .map(|(group, units)| {
            (
                *group,
                units
                    .iter()
                    .copied()
                    .map(turts_net_protocol::UnitId)
                    .collect(),
            )
        })
        .collect();
    input
        .0
        .push(turts_interaction::InputEvent::RestoreGroups { groups });
    persistence.group_namespace = Some(namespace);
    persistence.group_snapshot = loaded.groups;
    if let Some(notice) = loaded.notice
        && let Some(notices) = notices.as_deref_mut()
    {
        push_notice(&mut notices.0, &notice);
    }
}

fn save_binding_overrides(
    acquired: Option<Res<crate::join::AcquiredPackages>>,
    root: Res<PersistenceConfigRoot>,
    preferences: Res<crate::core_adapter::CorePreferences>,
    mut state: ResMut<PersistenceRuntime>,
    mut notices: Option<ResMut<crate::debug_feed::NoticesFeed>>,
) {
    let Some(acquired) = acquired else { return };
    let Some(verified) = acquired.0.verified_root_mod.as_ref() else {
        return;
    };
    let Some(path) = root
        .0
        .as_deref()
        .and_then(|root| native_bindings_path(root, &verified.id, &verified.version))
    else {
        return;
    };
    let namespace = path.to_string_lossy();
    if state.binding_namespace.as_deref() != Some(namespace.as_ref())
        || state.binding_snapshot == preferences.runtime_bindings
    {
        return;
    }
    match save_native_bindings(&path, &preferences.runtime_bindings) {
        Ok(()) => state
            .binding_snapshot
            .clone_from(&preferences.runtime_bindings),
        Err(error) => {
            if let Some(notices) = notices.as_deref_mut() {
                push_notice(
                    &mut notices.0,
                    &format!("Player bindings could not be saved: {error}"),
                );
            }
        }
    }
}

fn save_group_assignments(
    phase: Option<Res<crate::join::Phase>>,
    state: Option<Res<crate::ingest::ReplicaState>>,
    groups: Option<Res<crate::debug_feed::GroupsFeed>>,
    root: Res<PersistenceConfigRoot>,
    mut persistence: ResMut<PersistenceRuntime>,
    mut notices: Option<ResMut<crate::debug_feed::NoticesFeed>>,
) {
    let (Some(crate::join::Phase::Joined { slot }), Some(state), Some(groups)) =
        (phase.as_deref(), state.as_deref(), groups.as_deref())
    else {
        return;
    };
    let Some(welcome) = state.welcome else { return };
    if welcome.slot != *slot
        || persistence.group_namespace != Some((welcome.match_id, welcome.slot))
        || persistence.group_snapshot == groups.0
    {
        return;
    }
    let Some(root) = root.0.as_deref() else {
        return;
    };
    let path = native_groups_path(root, welcome.match_id, welcome.slot);
    match save_native_groups(&path, &groups.0) {
        Ok(()) => persistence.group_snapshot.clone_from(&groups.0),
        Err(error) => {
            if let Some(notices) = notices.as_deref_mut() {
                push_notice(
                    &mut notices.0,
                    &format!("Control groups could not be saved: {error}"),
                );
            }
        }
    }
}

fn push_notice(notices: &mut Vec<serde_json::Value>, text: &str) {
    if notices.iter().any(|notice| {
        notice
            .get("text")
            .or_else(|| notice.get("message"))
            .and_then(serde_json::Value::as_str)
            == Some(text)
    }) {
        return;
    }
    notices.push(serde_json::json!({"text":text,"message":text}));
}

#[cfg(target_family = "wasm")]
fn web_storage() -> Result<web_sys::Storage, String> {
    let window = web_sys::window().ok_or_else(|| "browser window is unavailable".to_string())?;
    window
        .local_storage()
        .map_err(|_| "browser player-bindings storage is unavailable".to_string())?
        .ok_or_else(|| "browser player-bindings storage is unavailable".to_string())
}

#[cfg(target_family = "wasm")]
fn read_web_bindings(key: &str) -> Result<LoadedBindings, String> {
    let storage = web_storage()?;
    let Some(text) = storage
        .get_item(key)
        .map_err(|_| "browser player bindings could not be read".to_string())?
    else {
        return Ok(LoadedBindings {
            actions: BTreeMap::new(),
            notice: None,
        });
    };
    let value = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .filter(serde_json::Value::is_object);
    Ok(match value {
        Some(value) => loaded_bindings(value),
        None => LoadedBindings {
            actions: BTreeMap::new(),
            notice: Some(CORRUPT_NOTICE.into()),
        },
    })
}

#[cfg(target_family = "wasm")]
fn write_web_bindings(key: &str, actions: &BTreeMap<String, Vec<Binding>>) -> Result<(), String> {
    let value = serde_json::json!({ "actions": actions });
    let text = serde_json::to_string(&value)
        .map_err(|error| format!("player bindings could not be serialized: {error}"))?;
    web_storage()?
        .set_item(key, &text)
        .map_err(|_| "browser player bindings could not be written".to_string())
}

#[cfg(target_family = "wasm")]
fn read_web_groups(key: &str) -> Result<LoadedGroups, String> {
    let storage = web_storage()?;
    let Some(text) = storage
        .get_item(key)
        .map_err(|_| "browser control groups could not be read".to_string())?
    else {
        return Ok(LoadedGroups {
            groups: BTreeMap::new(),
            notice: None,
        });
    };
    let value = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .filter(serde_json::Value::is_object);
    Ok(match value {
        Some(value) => loaded_groups(value),
        None => LoadedGroups {
            groups: BTreeMap::new(),
            notice: Some(CORRUPT_NOTICE.into()),
        },
    })
}

#[cfg(target_family = "wasm")]
fn write_web_groups(key: &str, groups: &BTreeMap<u8, Vec<u32>>) -> Result<(), String> {
    let value = serde_json::json!({ "groups": groups });
    let text = serde_json::to_string(&value)
        .map_err(|error| format!("control groups could not be serialized: {error}"))?;
    web_storage()?
        .set_item(key, &text)
        .map_err(|_| "browser control groups could not be written".to_string())
}

#[cfg(target_family = "wasm")]
fn prune_old_web_group_namespaces(storage: &web_sys::Storage, match_id: u64) -> Result<(), String> {
    let count = storage
        .length()
        .map_err(|_| "browser control group namespaces could not be listed".to_string())?;
    let mut stale = Vec::new();
    for index in 0..count {
        let Some(key) = storage
            .key(index)
            .map_err(|_| "browser control group namespace could not be inspected".to_string())?
        else {
            continue;
        };
        let Some(suffix) = key.strip_prefix("turts.v1.groups.") else {
            continue;
        };
        let current_namespace =
            suffix
                .split_once('.')
                .is_some_and(|(stored_match, stored_slot)| {
                    stored_match.parse::<u64>().is_ok_and(|stored| {
                        stored == match_id && stored.to_string() == stored_match
                    }) && stored_slot
                        .parse::<u8>()
                        .is_ok_and(|slot| slot.to_string() == stored_slot)
                });
        if !current_namespace {
            stale.push(key);
        }
    }
    for key in stale {
        storage
            .remove_item(&key)
            .map_err(|_| "stale browser control groups could not be removed".to_string())?;
    }
    Ok(())
}

#[cfg(target_family = "wasm")]
fn load_web_bindings(
    acquired: Option<Res<crate::join::AcquiredPackages>>,
    mut preferences: ResMut<crate::core_adapter::CorePreferences>,
    mut state: ResMut<PersistenceRuntime>,
    mut notices: Option<ResMut<crate::debug_feed::NoticesFeed>>,
) {
    let Some(verified) = acquired
        .as_deref()
        .and_then(|acquired| acquired.0.verified_root_mod.as_ref())
    else {
        return;
    };
    let Some(key) = web_bindings_key(&verified.id, &verified.version) else {
        return;
    };
    if state.binding_namespace.as_deref() == Some(&key) {
        return;
    }
    let loaded = match read_web_bindings(&key) {
        Ok(loaded) => loaded,
        Err(error) => {
            if let Some(notices) = notices.as_deref_mut() {
                push_notice(
                    &mut notices.0,
                    &format!("Player bindings could not be loaded: {error}"),
                );
            }
            return;
        }
    };
    let Some(acquired) = acquired else { return };
    let valid_actions: std::collections::HashSet<String> =
        turts_interaction::input_map::effective_actions(&acquired.0.content.input_map)
            .into_keys()
            .collect();
    preferences.runtime_bindings = loaded
        .actions
        .into_iter()
        .filter(|(action, _)| valid_actions.contains(action))
        .collect();
    state.binding_namespace = Some(key);
    state
        .binding_snapshot
        .clone_from(&preferences.runtime_bindings);
    if let Some(notice) = loaded.notice
        && let Some(notices) = notices.as_deref_mut()
    {
        push_notice(&mut notices.0, &notice);
    }
}

#[cfg(target_family = "wasm")]
fn save_web_bindings(
    acquired: Option<Res<crate::join::AcquiredPackages>>,
    preferences: Res<crate::core_adapter::CorePreferences>,
    mut state: ResMut<PersistenceRuntime>,
    mut notices: Option<ResMut<crate::debug_feed::NoticesFeed>>,
) {
    let Some(verified) = acquired
        .as_deref()
        .and_then(|acquired| acquired.0.verified_root_mod.as_ref())
    else {
        return;
    };
    let Some(key) = web_bindings_key(&verified.id, &verified.version) else {
        return;
    };
    if state.binding_namespace.as_deref() != Some(&key)
        || state.binding_snapshot == preferences.runtime_bindings
    {
        return;
    }
    match write_web_bindings(&key, &preferences.runtime_bindings) {
        Ok(()) => state
            .binding_snapshot
            .clone_from(&preferences.runtime_bindings),
        Err(error) => {
            if let Some(notices) = notices.as_deref_mut() {
                push_notice(
                    &mut notices.0,
                    &format!("Player bindings could not be saved: {error}"),
                );
            }
        }
    }
}

#[cfg(target_family = "wasm")]
fn restore_web_groups_after_snapshot(
    phase: Option<Res<crate::join::Phase>>,
    state: Option<Res<crate::ingest::ReplicaState>>,
    mut persistence: ResMut<PersistenceRuntime>,
    mut input: Option<ResMut<crate::core_adapter::CoreInputQueue>>,
    mut notices: Option<ResMut<crate::debug_feed::NoticesFeed>>,
) {
    let Some(crate::join::Phase::Joined { slot }) = phase.as_deref() else {
        return;
    };
    let (Some(state), Some(input)) = (state, input.as_deref_mut()) else {
        return;
    };
    if state.replica.as_ref().is_none_or(|replica| {
        replica.snapshot_tick.is_none() || replica.last_applied_tick.is_none()
    }) {
        return;
    }
    let Some(welcome) = state.welcome else { return };
    if welcome.slot != *slot {
        return;
    }
    let namespace = (welcome.match_id, welcome.slot);
    if persistence.group_namespace == Some(namespace) {
        return;
    }
    let storage = match web_storage() {
        Ok(storage) => storage,
        Err(error) => {
            if let Some(notices) = notices.as_deref_mut() {
                push_notice(
                    &mut notices.0,
                    &format!("Player groups could not be loaded: {error}"),
                );
            }
            return;
        }
    };
    if let Err(error) = prune_old_web_group_namespaces(&storage, welcome.match_id) {
        if let Some(notices) = notices.as_deref_mut() {
            push_notice(
                &mut notices.0,
                &format!("Old player groups could not be removed: {error}"),
            );
        }
    }
    let key = web_groups_key(welcome.match_id, welcome.slot);
    let loaded = match read_web_groups(&key) {
        Ok(loaded) => loaded,
        Err(error) => {
            if let Some(notices) = notices.as_deref_mut() {
                push_notice(
                    &mut notices.0,
                    &format!("Player groups could not be loaded: {error}"),
                );
            }
            return;
        }
    };
    let groups: Vec<_> = loaded
        .groups
        .iter()
        .map(|(group, units)| {
            (
                *group,
                units
                    .iter()
                    .copied()
                    .map(turts_net_protocol::UnitId)
                    .collect(),
            )
        })
        .collect();
    input
        .0
        .push(turts_interaction::InputEvent::RestoreGroups { groups });
    persistence.group_namespace = Some(namespace);
    persistence.group_snapshot = loaded.groups;
    if let Some(notice) = loaded.notice
        && let Some(notices) = notices.as_deref_mut()
    {
        push_notice(&mut notices.0, &notice);
    }
}

#[cfg(target_family = "wasm")]
fn save_web_groups(
    phase: Option<Res<crate::join::Phase>>,
    state: Option<Res<crate::ingest::ReplicaState>>,
    groups: Option<Res<crate::debug_feed::GroupsFeed>>,
    mut persistence: ResMut<PersistenceRuntime>,
    mut notices: Option<ResMut<crate::debug_feed::NoticesFeed>>,
) {
    let (Some(crate::join::Phase::Joined { slot }), Some(state), Some(groups)) =
        (phase.as_deref(), state.as_deref(), groups.as_deref())
    else {
        return;
    };
    let Some(welcome) = state.welcome else { return };
    if welcome.slot != *slot
        || persistence.group_namespace != Some((welcome.match_id, welcome.slot))
        || persistence.group_snapshot == groups.0
    {
        return;
    }
    let key = web_groups_key(welcome.match_id, welcome.slot);
    match write_web_groups(&key, &groups.0) {
        Ok(()) => persistence.group_snapshot.clone_from(&groups.0),
        Err(error) => {
            if let Some(notices) = notices.as_deref_mut() {
                push_notice(
                    &mut notices.0,
                    &format!("Control groups could not be saved: {error}"),
                );
            }
        }
    }
}
