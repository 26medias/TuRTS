//! Native persistence contract tests for Task16.
//!
//! Every filesystem assertion uses a unique directory under the OS temporary
//! directory. These tests never read or write HOME, XDG_CONFIG_HOME, or a real
//! user configuration directory.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::prelude::{App, MinimalPlugins};
use turts_client::{
    acquire::{Acquired, PresentationContent, VerifiedRootMod},
    content::ClientContent,
    core_adapter::CorePreferences,
    join::AcquiredPackages,
    options::parse_session_player_id,
    persist::{
        PersistPlugin, PersistenceConfigRoot, compatibility_line, load_native_bindings,
        load_native_groups, native_bindings_path, native_config_dir_from, native_groups_path,
        save_native_bindings, save_native_groups, web_bindings_key, web_groups_key,
    },
    volume_settings::{
        SettingsStore, VolumeCategory, VolumeSettings, decode_settings_json, native_settings_path,
    },
};
use turts_mod_api::{Binding, Button, InputMapDef, Mod, WheelDirection};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct TempConfig(PathBuf);

impl TempConfig {
    fn new() -> Self {
        let unique = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "turts-task16-persist-{}-{nonce}-{unique}",
            std::process::id()
        ));
        Self(root)
    }

    fn root(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempConfig {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn native_settings_write_creates_config_path_and_survives_reload() {
    let fixture = TempConfig::new();
    let path = native_settings_path(fixture.root());
    assert_eq!(path, fixture.root().join("turts/settings.json"));
    let store = SettingsStore::for_path(path.clone());

    assert_eq!(store.load().volumes, VolumeSettings::default());
    store
        .set_volume(VolumeCategory::Master, 0.25)
        .expect("native settings write should succeed inside the isolated fixture");

    let bytes = fs::read_to_string(&path).expect("settings file should be written");
    let json: serde_json::Value = serde_json::from_str(&bytes).expect("settings JSON should parse");
    assert_eq!(json["audio"]["master"], 0.25);
    assert_eq!(
        SettingsStore::for_path(path).load().volumes.master,
        0.25,
        "saved master volume should survive a fresh store load"
    );
}

#[test]
fn corrupt_native_volume_settings_use_defaults_and_one_notice_value() {
    let fixture = TempConfig::new();
    let path = native_settings_path(fixture.root());
    fs::create_dir_all(path.parent().expect("settings path has parent")).unwrap();
    fs::write(&path, "{not-json").unwrap();

    let loaded = SettingsStore::for_path(path).load();
    assert_eq!(loaded.volumes, VolumeSettings::default());
    assert_eq!(
        loaded.notice.as_deref(),
        Some("Settings were unreadable; default settings are in use.")
    );
}

#[test]
fn settings_updates_preserve_unknown_fields_and_all_existing_audio_categories() {
    let fixture = TempConfig::new();
    let path = native_settings_path(fixture.root());
    fs::create_dir_all(path.parent().expect("settings path has parent")).unwrap();
    fs::write(
        &path,
        r#"{"future_setting":{"enabled":true},"audio":{"master":0.8,"voice":0.3,"alerts":0.4,"effects":0.5}}"#,
    )
    .unwrap();

    SettingsStore::for_path(path.clone())
        .set_volume(VolumeCategory::Master, 0.6)
        .expect("updating one category should preserve the settings document");
    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(saved["future_setting"]["enabled"], true);
    assert_eq!(saved["audio"]["master"], f64::from(0.6_f32));
    assert_eq!(saved["audio"]["voice"], 0.3);
    assert_eq!(saved["audio"]["alerts"], 0.4);
    assert_eq!(saved["audio"]["effects"], 0.5);
}

#[test]
fn binding_and_group_updates_preserve_the_existing_settings_and_audio_document() {
    let fixture = TempConfig::new();
    let root = fixture.root();
    let settings_path = root.join("turts/settings.json");
    fs::create_dir_all(settings_path.parent().expect("settings parent")).unwrap();
    let original = r#"{"future_setting":{"enabled":true},"audio":{"master":0.8,"voice":0.3,"alerts":0.4,"effects":0.5}}"#;
    fs::write(&settings_path, original).unwrap();

    let binding_path = root.join("turts/bindings-forge-1.json");
    let group_path = root.join("turts/groups-41-1.json");
    let bindings = BTreeMap::from([(
        "key_move".to_owned(),
        vec![Binding::Key {
            code: "KeyQ".to_owned(),
            mods: vec![],
        }],
    )]);
    let groups = BTreeMap::from([(1, vec![7, 19])]);

    // Invoke both actual file-writing paths before asserting, so the red baseline
    // identifies their missing persistence behavior without touching user config.
    let binding_result = save_native_bindings(&binding_path, &bindings);
    let group_result = save_native_groups(&group_path, &groups);
    assert!(
        binding_result.is_ok(),
        "writing binding overrides must succeed under the isolated root"
    );
    assert!(
        group_result.is_ok(),
        "writing match-scoped control groups must succeed under the isolated root"
    );

    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(settings_path).unwrap()).unwrap();
    assert_eq!(saved["future_setting"]["enabled"], true);
    assert_eq!(saved["audio"]["master"], 0.8);
    assert_eq!(saved["audio"]["voice"], 0.3);
    assert_eq!(saved["audio"]["alerts"], 0.4);
    assert_eq!(saved["audio"]["effects"], 0.5);
}

#[test]
fn malformed_or_invalid_volume_values_fall_back_to_defaults_with_a_notice_value() {
    for text in [
        "{not-json",
        r#"{"audio":false}"#,
        r#"{"audio":{"master":"loud"}}"#,
        r#"{"audio":{"effects":1.1}}"#,
    ] {
        let loaded = decode_settings_json(text);
        assert_eq!(loaded.volumes, VolumeSettings::default(), "input: {text}");
        assert!(loaded.notice.is_some(), "input: {text}");
    }
}

#[test]
fn session_player_ids_are_lossless_decimal_u64_and_reject_invalid_text() {
    let cases = [
        ("0", Some(0)),
        ("18446744073709551615", Some(u64::MAX)),
        ("18446744073709551616", None),
        ("-1", None),
        ("1.0", None),
        ("", None),
    ];
    for (text, expected) in cases {
        assert_eq!(parse_session_player_id(text), expected, "input: {text:?}");
    }
}

#[test]
fn native_config_root_prefers_an_injected_xdg_path_then_injected_home_fallback() {
    let fixture = TempConfig::new();
    let xdg = fixture.root().join("xdg");
    let home = fixture.root().join("home");
    assert_eq!(
        native_config_dir_from(Some(&xdg), Some(&home)),
        Some(xdg.clone()),
        "an explicit XDG root must be used without reading process environment"
    );
    assert_eq!(
        native_config_dir_from(None, Some(&home)),
        Some(home.join(".config")),
        "the native fallback root must be HOME/.config"
    );
    assert_eq!(native_config_dir_from(None, None), None);
}

#[test]
fn compatibility_lines_keep_exact_semver_decimal_components() {
    assert_eq!(compatibility_line("1.2.3"), Some("1".into()));
    assert_eq!(compatibility_line("0.9.4"), Some("0.9".into()));
    assert_eq!(
        compatibility_line("9007199254740993.1.0"),
        Some("9007199254740993".into()),
        "major components must stay exact decimal text above JavaScript's safe integer range"
    );
    assert_eq!(
        compatibility_line("0.9007199254740993.7"),
        Some("0.9007199254740993".into()),
        "compatibility components must not be rounded through f64"
    );
    assert_eq!(compatibility_line("01.2.3"), None);
    assert_eq!(compatibility_line("1.2"), None);
    assert_eq!(compatibility_line("18446744073709551616.0.0"), None);
}

#[test]
fn native_binding_and_group_paths_are_isolated_under_the_supplied_root() {
    let fixture = TempConfig::new();
    let root = fixture.root();
    assert_eq!(
        native_bindings_path(root, "forge", "0.9.4")
            .expect("native binding path must be available for the verified root mod"),
        root.join("turts/bindings-forge-0.9.json")
    );
    assert_eq!(
        native_bindings_path(root, "forge", "1.2.3")
            .expect("major-version binding path must be available"),
        root.join("turts/bindings-forge-1.json")
    );
    assert_eq!(
        native_groups_path(root, u64::MAX, 7),
        root.join("turts/groups-18446744073709551615-7.json"),
        "match IDs must retain their exact decimal u64 representation"
    );
    assert!(
        !root.exists(),
        "path derivation must not create or touch its root"
    );
}

#[test]
fn browser_binding_and_group_keys_use_verified_namespaces() {
    assert_eq!(
        web_bindings_key("forge", "0.9.4"),
        Some("turts.v1.bindings.forge.0.9".into())
    );
    assert_eq!(
        web_bindings_key("forge", "1.2.3"),
        Some("turts.v1.bindings.forge.1".into())
    );
    assert_eq!(
        web_groups_key(u64::MAX, 7),
        "turts.v1.groups.18446744073709551615.7"
    );
}

#[test]
fn binding_namespaces_use_package_id_characters_without_case_or_punctuation_aliases() {
    let fixture = TempConfig::new();
    let valid = ["a", "mod_2", "z9_0"];
    for mod_id in valid {
        assert!(native_bindings_path(fixture.root(), mod_id, "1.0.0").is_some());
        assert!(web_bindings_key(mod_id, "1.0.0").is_some());
    }

    let invalid = ["", "Forge", "mod-name", "mod.name", "mod/name", "möd"];
    for mod_id in invalid {
        assert_eq!(
            native_bindings_path(fixture.root(), mod_id, "1.0.0"),
            None,
            "mod ID: {mod_id:?}"
        );
        assert_eq!(
            web_bindings_key(mod_id, "1.0.0"),
            None,
            "mod ID: {mod_id:?}"
        );
    }
}

#[test]
fn native_binding_overrides_are_written_and_loaded_from_the_same_mod_compatibility_file() {
    let fixture = TempConfig::new();
    let path = fixture.root().join("turts/bindings-forge-1.json");
    let saved = BTreeMap::from([(
        "key_move".to_owned(),
        vec![Binding::Key {
            code: "KeyQ".to_owned(),
            mods: vec![],
        }],
    )]);

    save_native_bindings(&path, &saved)
        .expect("a binding edit must write to the isolated native config root");
    let loaded = load_native_bindings(&path);
    assert_eq!(loaded.actions, saved);
    assert!(loaded.notice.is_none());
    let resolved = turts_interaction::input_map::effective_actions_with_runtime(
        &InputMapDef {
            actions: vec![],
            engine_overrides: vec![],
        },
        &loaded.actions,
    );
    assert_eq!(resolved["key_move"], saved["key_move"]);
    assert_eq!(
        resolved["menu"],
        vec![
            Binding::Key {
                code: "F10".to_owned(),
                mods: vec![],
            },
            Binding::Key {
                code: "Escape".to_owned(),
                mods: vec![],
            },
        ],
        "a runtime override must leave the actual unrelated engine menu default intact"
    );
}

#[test]
fn persistence_plugin_loads_binding_overrides_for_the_verified_root_package() {
    let fixture = TempConfig::new();
    let path = native_bindings_path(fixture.root(), "forge", "1.4.2")
        .expect("verified Forge package has a supported compatibility namespace");
    let overrides = BTreeMap::from([
        (
            "key_move".to_owned(),
            vec![Binding::Key {
                code: "KeyQ".to_owned(),
                mods: vec![],
            }],
        ),
        (
            "camera_pan_up".to_owned(),
            vec![Binding::Key {
                code: "KeyI".to_owned(),
                mods: vec![],
            }],
        ),
    ]);
    save_native_bindings(&path, &overrides).expect("isolated override fixture writes");

    let content = ClientContent {
        input_map: ron::from_str(include_str!("../../../mods/forge/input/bindings.ron"))
            .expect("the real Forge binding package parses"),
        ..Default::default()
    };
    let acquired = Acquired {
        lockfile_hash: "verified-test-lock-hash".into(),
        visuals: Default::default(),
        map_size: (128.0, 72.0),
        unit_names: Default::default(),
        content: content.clone(),
        presentation: PresentationContent::default(),
        verified_root_mod: Some(VerifiedRootMod {
            id: "forge".into(),
            version: "1.4.2".into(),
        }),
    };
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(PersistenceConfigRoot(Some(fixture.root().to_path_buf())))
        .insert_resource(AcquiredPackages(acquired))
        .init_resource::<CorePreferences>()
        .add_plugins(PersistPlugin);

    app.update();

    let preferences = app.world().resource::<CorePreferences>();
    assert_eq!(preferences.runtime_bindings, overrides);
    assert_eq!(
        turts_interaction::input_map::effective_actions_with_runtime(
            &content.input_map,
            &preferences.runtime_bindings,
        )["key_move"],
        vec![Binding::Key {
            code: "KeyQ".into(),
            mods: vec![],
        }],
        "the actual Core resolver receives the namespace loaded from verified root identity"
    );
    assert_eq!(
        turts_interaction::input_map::effective_actions_with_runtime(
            &content.input_map,
            &preferences.runtime_bindings,
        )["camera_pan_up"],
        vec![Binding::Key {
            code: "KeyI".into(),
            mods: vec![],
        }],
        "persisted engine defaults remain in the verified package namespace"
    );
}

#[test]
fn persistence_plugin_rejects_reserved_stored_chords_before_they_override_forge_defaults() {
    let fixture = TempConfig::new();
    let path = native_bindings_path(fixture.root(), "forge", "1.4.2")
        .expect("verified Forge package has a supported compatibility namespace");
    fs::create_dir_all(path.parent().expect("binding path has parent"))
        .expect("create only the isolated fixture directory");
    fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "actions": {
                "key_move": [{"Key": {"code": "F12", "mods": []}}],
                "camera_pan_up": [{"Key": {"code": "KeyI", "mods": []}}]
            }
        }))
        .expect("fixture binding document serializes"),
    )
    .expect("write the invalid chord only inside the isolated fixture directory");

    let content = ClientContent {
        input_map: ron::from_str(include_str!("../../../mods/forge/input/bindings.ron"))
            .expect("the real Forge binding package parses"),
        ..Default::default()
    };
    let defaults = turts_interaction::input_map::effective_actions(&content.input_map);
    let acquired = Acquired {
        lockfile_hash: "verified-test-lock-hash".into(),
        visuals: Default::default(),
        map_size: (128.0, 72.0),
        unit_names: Default::default(),
        content: content.clone(),
        presentation: PresentationContent::default(),
        verified_root_mod: Some(VerifiedRootMod {
            id: "forge".into(),
            version: "1.4.2".into(),
        }),
    };
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(PersistenceConfigRoot(Some(fixture.root().to_path_buf())))
        .insert_resource(AcquiredPackages(acquired))
        .init_resource::<CorePreferences>()
        .init_resource::<turts_client::debug_feed::NoticesFeed>()
        .add_plugins(PersistPlugin);

    app.update();

    let preferences = &app.world().resource::<CorePreferences>().runtime_bindings;
    assert!(
        preferences.is_empty(),
        "one reserved stored chord invalidates that binding document"
    );
    let effective = turts_interaction::input_map::effective_actions_with_runtime(
        &content.input_map,
        preferences,
    );
    assert_eq!(effective["key_move"], defaults["key_move"]);
    assert_eq!(effective["camera_pan_up"], defaults["camera_pan_up"]);
    let notices = &app
        .world()
        .resource::<turts_client::debug_feed::NoticesFeed>()
        .0;
    assert_eq!(
        notices.len(),
        1,
        "invalid stored chords use one existing deduplicated notice"
    );
    assert_eq!(
        notices[0]["text"],
        "Player settings were unreadable; defaults are in use."
    );
    app.update();
    assert_eq!(
        app.world()
            .resource::<turts_client::debug_feed::NoticesFeed>()
            .0
            .len(),
        1,
        "loading the same verified namespace again does not duplicate its notice"
    );
}

#[test]
fn stored_binding_parser_rejects_unknown_duplicate_and_reserved_key_chords() {
    let fixture = TempConfig::new();
    let path = fixture.root().join("turts/bindings-forge-1.json");
    let cases = [
        Binding::Key {
            code: "NotARealCode".into(),
            mods: vec![],
        },
        Binding::Key {
            code: "KeyQ".into(),
            mods: vec![Mod::Ctrl, Mod::Ctrl],
        },
        Binding::Key {
            code: "F12".into(),
            mods: vec![],
        },
        Binding::Pointer {
            button: Button::Back,
            mods: vec![Mod::Ctrl, Mod::Ctrl],
        },
    ];
    for invalid in cases {
        let actions = BTreeMap::from([("key_move".to_owned(), vec![invalid])]);
        save_native_bindings(&path, &actions).expect("write only to the isolated fixture path");
        let loaded = load_native_bindings(&path);
        assert!(
            loaded.actions.is_empty(),
            "invalid stored key chord defaults the namespace"
        );
        assert_eq!(
            loaded.notice.as_deref(),
            Some("Player settings were unreadable; defaults are in use.")
        );
    }
}

#[test]
fn stored_binding_parser_keeps_valid_key_pointer_and_wheel_bindings() {
    let fixture = TempConfig::new();
    let path = fixture.root().join("turts/bindings-forge-1.json");
    let actions = BTreeMap::from([(
        "key_move".to_owned(),
        vec![
            Binding::Key {
                code: "KeyQ".into(),
                mods: vec![],
            },
            Binding::Pointer {
                button: Button::Back,
                mods: vec![Mod::Shift],
            },
            Binding::Wheel(WheelDirection::Down),
        ],
    )]);
    save_native_bindings(&path, &actions).expect("write valid controls to an isolated path");
    let loaded = load_native_bindings(&path);
    assert_eq!(loaded.actions, actions);
    assert!(loaded.notice.is_none());
}

#[test]
fn native_group_assignments_round_trip_without_crossing_match_or_slot_paths() {
    let fixture = TempConfig::new();
    let match_a_slot_1 = fixture.root().join("turts/groups-41-1.json");
    let match_a_slot_2 = fixture.root().join("turts/groups-41-2.json");
    let match_b_slot_1 = fixture.root().join("turts/groups-42-1.json");
    let saved = BTreeMap::from([(1, vec![7, 19]), (9, vec![u32::MAX])]);

    save_native_groups(&match_a_slot_1, &saved)
        .expect("group edits must be written under the actual match and player slot");
    assert_eq!(load_native_groups(&match_a_slot_1).groups, saved);
    assert!(load_native_groups(&match_a_slot_2).groups.is_empty());
    assert!(load_native_groups(&match_b_slot_1).groups.is_empty());
}

#[test]
fn malformed_binding_and_group_documents_default_with_a_single_notice_value() {
    let fixture = TempConfig::new();
    let binding_path = fixture.root().join("turts/bindings-forge-1.json");
    let group_path = fixture.root().join("turts/groups-41-1.json");
    fs::create_dir_all(binding_path.parent().expect("binding parent"))
        .expect("create only the isolated fixture directory");
    fs::create_dir_all(group_path.parent().expect("group parent"))
        .expect("create only the isolated fixture directory");
    fs::write(&binding_path, br#"{"actions":{"select":false}}"#)
        .expect("write malformed fixture bindings");
    fs::write(&group_path, br#"{"1":["not-a-unit-id"]}"#).expect("write malformed fixture groups");

    let bindings = load_native_bindings(&binding_path);
    assert!(bindings.actions.is_empty());
    assert!(bindings.notice.is_some());
    let groups = load_native_groups(&group_path);
    assert!(groups.groups.is_empty());
    assert!(groups.notice.is_some());
}

/// Exercise the real native path resolvers and persistence APIs. An optional private
/// raw receipt supplements the assertions; the external runner supplies its own exit
/// code, source hash and receipt hash only after this process has exited.
#[test]
fn per1_native_paths() {
    use turts_mod_api::Manifest;
    use turts_mod_runtime::hash::sha256_hex;

    fn now_ms() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("native proof clock follows epoch")
            .as_millis()
    }
    fn operation(path: &Path, operation: &str, method: &str) -> serde_json::Value {
        let bytes = fs::read(path).expect("actual persisted file must reopen");
        let readback = fs::read(path).expect("actual persisted file must reopen again");
        assert_eq!(bytes, readback, "native readback bytes must be stable");
        serde_json::json!({
            "operation": operation,
            "pid": std::process::id(),
            "path": path,
            "method": method,
            "success": true,
            "observed_unix_ms": now_ms(),
            "byte_length": bytes.len(),
            "sha256": sha256_hex(&bytes).trim_start_matches("sha256:"),
            "readback_sha256": sha256_hex(&readback).trim_start_matches("sha256:"),
        })
    }

    let started = now_ms();
    let fixture = TempConfig::new();
    let manifest: Manifest = ron::from_str(include_str!("../../../mods/forge/manifest.ron"))
        .expect("actual Forge manifest parses");
    let match_id = u64::MAX - 2;
    let slot = 1;
    let bindings_path = native_bindings_path(fixture.root(), &manifest.id, &manifest.version)
        .expect("actual mod namespace resolves");
    let settings_path = native_settings_path(fixture.root());
    let groups_path = native_groups_path(fixture.root(), match_id, slot);
    assert_eq!(
        bindings_path,
        fixture.root().join("turts/bindings-forge-0.2.json")
    );
    assert_eq!(settings_path, fixture.root().join("turts/settings.json"));
    assert_eq!(
        groups_path,
        fixture
            .root()
            .join("turts/groups-18446744073709551613-1.json")
    );

    let bindings = BTreeMap::from([(
        "key_move".to_owned(),
        vec![Binding::Key {
            code: "KeyQ".to_owned(),
            mods: vec![],
        }],
    )]);
    save_native_bindings(&bindings_path, &bindings).expect("real native binding write succeeds");
    let binding_write = operation(&bindings_path, "write", "save_native_bindings");
    let loaded = load_native_bindings(&bindings_path);
    assert_eq!(
        loaded.actions, bindings,
        "native binding reload preserves override"
    );
    assert!(loaded.notice.is_none());
    let binding_read = operation(&bindings_path, "read", "load_native_bindings");

    let store = SettingsStore::for_path(settings_path.clone());
    store
        .set_volume(VolumeCategory::Master, 0.25)
        .expect("real native settings write succeeds");
    let settings_write = operation(&settings_path, "write", "SettingsStore::set_volume");
    let settings = store.load();
    assert_eq!(
        settings.volumes.master, 0.25,
        "native settings reload preserves volume"
    );
    assert!(settings.notice.is_none());
    let settings_read = operation(&settings_path, "read", "SettingsStore::load");

    let groups = BTreeMap::from([(1, vec![7, 19])]);
    save_native_groups(&groups_path, &groups).expect("real native group write succeeds");
    let group_write = operation(&groups_path, "write", "save_native_groups");
    let loaded = load_native_groups(&groups_path);
    assert_eq!(loaded.groups, groups, "native group reload preserves IDs");
    assert!(loaded.notice.is_none());
    let group_read = operation(&groups_path, "read", "load_native_groups");

    if let Some(output) = std::env::var_os("MC_PER1_NATIVE_RAW_RECEIPT") {
        let output = PathBuf::from(output);
        assert!(
            output.is_absolute(),
            "native raw receipt requires an absolute path"
        );
        let parent = output
            .parent()
            .expect("receipt has a parent")
            .canonicalize()
            .expect("private receipt parent already exists");
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("native workspace root resolves");
        assert!(
            !parent.starts_with(workspace),
            "native raw receipt stays outside the served worktree"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&parent)
                    .expect("private receipt parent exists")
                    .permissions()
                    .mode()
                    & 0o077,
                0,
                "native raw receipt parent must be private"
            );
        }
        let output = parent.join(output.file_name().expect("receipt has a file name"));
        let executable = std::env::current_exe().expect("actual test executable resolves");
        let executable_bytes = fs::read(&executable).expect("actual test executable can be hashed");
        let relative = |path: &Path| {
            path.strip_prefix(fixture.root())
                .expect("path stays inside fixture")
                .to_path_buf()
        };
        let report = serde_json::json!({
            "schema": "per1-native-path-raw-v1",
            "receipt_path": output,
            "process": {
                "pid": std::process::id(),
                "argv": std::env::args().collect::<Vec<_>>(),
                "executable": executable,
                "executable_sha256": sha256_hex(&executable_bytes).trim_start_matches("sha256:"),
                "started_unix_ms": started,
                "ended_unix_ms": now_ms(),
            },
            "config_root": fixture.root(),
            "inputs": {"mod_id": manifest.id, "version": manifest.version,
                "match_id_decimal": match_id.to_string(), "slot": slot},
            "paths": {"bindings": relative(&bindings_path), "settings": relative(&settings_path), "groups": relative(&groups_path)},
            "reads": [binding_read, settings_read, group_read],
            "writes": [binding_write, settings_write, group_write],
        });
        use std::io::Write;
        #[cfg(unix)]
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options
            .open(output)
            .expect("private native receipt creates exactly once");
        file.write_all(&serde_json::to_vec_pretty(&report).expect("raw receipt serializes"))
            .expect("native receipt write succeeds");
    }
}

#[test]
fn joined_native_persistence_removes_only_group_files_of_other_matches() {
    use turts_client::{
        core_adapter::CoreInputQueue,
        ingest::{ReplicaState, WelcomeInfo},
        join::Phase,
        replica::Replica,
    };
    let fixture = TempConfig::new();
    let current_match = 42;
    let groups = BTreeMap::from([(1, vec![7, 8])]);
    let stale = [
        native_groups_path(fixture.root(), 7, 0),
        native_groups_path(fixture.root(), u64::MAX, 7),
    ];
    for path in &stale {
        save_native_groups(path, &groups).unwrap();
    }
    let current = native_groups_path(fixture.root(), current_match, 0);
    let other_slot = native_groups_path(fixture.root(), current_match, 1);
    save_native_groups(&current, &groups).unwrap();
    save_native_groups(&other_slot, &groups).unwrap();
    let settings = native_settings_path(fixture.root());
    fs::write(&settings, "{\"audio\":{\"master\":0.4}}").unwrap();
    let bindings = native_bindings_path(fixture.root(), "forge", "0.2.0").unwrap();
    save_native_bindings(&bindings, &BTreeMap::new()).unwrap();
    let unknown = fixture.root().join("turts/groups-user-notes.json");
    fs::write(&unknown, "private notes").unwrap();
    let kept = [&current, &other_slot, &settings, &bindings, &unknown]
        .map(|path| (path.to_path_buf(), fs::read(path).unwrap()));
    let mut replica = Replica::new(20);
    replica.snapshot_tick = Some(1);
    replica.last_applied_tick = Some(1);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(PersistenceConfigRoot(Some(fixture.root().to_path_buf())))
        .init_resource::<CorePreferences>()
        .init_resource::<CoreInputQueue>()
        .insert_resource(ReplicaState {
            replica: Some(replica),
            welcome: Some(WelcomeInfo {
                slot: 0,
                tick: 1,
                tick_hz: 20,
                match_id: current_match,
            }),
            ..Default::default()
        })
        .insert_resource(Phase::Joining)
        .add_plugins(PersistPlugin);
    app.update();
    assert!(
        stale.iter().all(|path| path.exists()),
        "pre-join state must preserve old native groups"
    );
    app.insert_resource(Phase::Joined { slot: 0 });
    app.update();
    assert!(
        stale.iter().all(|path| !path.exists()),
        "actual join must remove each native group file of another match"
    );
    for (path, expected) in kept {
        assert_eq!(
            fs::read(path).unwrap(),
            expected,
            "current match, other slot and unrelated documents must stay byte-exact"
        );
    }
    assert!(app.world().resource::<CoreInputQueue>().0.iter().any(|event| matches!(event,
        turts_interaction::InputEvent::RestoreGroups { groups } if groups == &vec![(1, vec![turts_net_protocol::UnitId(7),turts_net_protocol::UnitId(8)])])),
        "the actual persistence plugin must still restore the current native group");
}
