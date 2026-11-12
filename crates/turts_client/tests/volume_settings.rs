use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use turts_client::volume_settings::{
    SettingsStore, VolumeCategory, VolumeSettings, WEB_SETTINGS_KEY, decode_settings_json,
    native_settings_path, set_sound_on_in_json, set_volume_in_json,
};

static NEXT_STORE: AtomicU64 = AtomicU64::new(0);

struct IsolatedStore(PathBuf);

impl IsolatedStore {
    fn new() -> Self {
        let nonce = NEXT_STORE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "turts-volume-settings-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create isolated native settings directory");
        Self(root.join("settings.json"))
    }

    fn store(&self) -> SettingsStore {
        SettingsStore::for_path(self.0.clone())
    }
}

impl Drop for IsolatedStore {
    fn drop(&mut self) {
        if let Some(parent) = self.0.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}

fn assert_saved_volume(value: &serde_json::Value, expected: f64) {
    let actual = value.as_f64().expect("saved volume remains a JSON number");
    assert!(
        (actual - expected).abs() < 1e-6,
        "saved volume {actual} is close to {expected}"
    );
}

#[test]
fn native_volume_store_uses_settings_json_and_preserves_unowned_fields() {
    let isolated = IsolatedStore::new();
    assert_eq!(
        isolated.store().path().file_name().unwrap(),
        "settings.json"
    );
    fs::write(
        &isolated.0,
        r#"{"scroll_speed":2.5,"future":{"keep":true},"audio":{"master":0.9,"voice":0.8,"alerts":0.7,"effects":0.6}}"#,
    )
    .unwrap();

    isolated
        .store()
        .set_volume(VolumeCategory::Voice, 0.35)
        .unwrap();

    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&isolated.0).unwrap()).unwrap();
    assert_eq!(saved["scroll_speed"], 2.5);
    assert_eq!(saved["future"]["keep"], true);
    assert_saved_volume(&saved["audio"]["voice"], 0.35);
    assert_eq!(saved["audio"]["master"], 0.9);
    assert_eq!(saved["audio"]["alerts"], 0.7);
    assert_eq!(saved["audio"]["effects"], 0.6);

    let reloaded = isolated.store().load();
    assert_eq!(
        reloaded.volumes.voice, 0.35,
        "native reload restores the edited category"
    );
    assert_eq!(
        reloaded.volumes.master, 0.9,
        "native reload restores master gain"
    );
    assert!(
        reloaded.notice.is_none(),
        "valid settings do not emit a corruption notice"
    );
}

#[test]
fn malformed_browser_and_native_settings_default_with_one_notice() {
    assert_eq!(WEB_SETTINGS_KEY, "turts.v1.settings");
    let browser = decode_settings_json("{not json");
    assert_eq!(browser.volumes, VolumeSettings::default());
    assert!(
        browser
            .notice
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
    );
    for corrupt in [
        r#"{"audio":{"master":"bad"}}"#,
        r#"{"audio":{"voice":1.5}}"#,
        r#"{"audio":false}"#,
    ] {
        let loaded = decode_settings_json(corrupt);
        assert_eq!(
            loaded.volumes,
            VolumeSettings::default(),
            "corrupt audio values reset the audio settings"
        );
        assert!(
            loaded
                .notice
                .as_deref()
                .is_some_and(|text| !text.trim().is_empty()),
            "corrupt audio values produce one notice"
        );
    }
    let valid_with_unowned_fields =
        decode_settings_json(r#"{"future":{"keep":true},"audio":{"master":0.5}}"#);
    assert_eq!(valid_with_unowned_fields.volumes.master, 0.5);
    assert!(
        valid_with_unowned_fields.notice.is_none(),
        "unknown unrelated settings remain valid"
    );

    let isolated = IsolatedStore::new();
    fs::write(&isolated.0, "{not json").unwrap();
    let native = isolated.store().load();
    assert_eq!(native.volumes, VolumeSettings::default());
    assert!(
        native
            .notice
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
    );
}

#[test]
fn saving_after_corruption_repairs_only_the_audio_object_and_keeps_unrelated_settings() {
    let isolated = IsolatedStore::new();
    fs::write(&isolated.0, r#"{"future":{"keep":true},"audio":false}"#).unwrap();
    for (category, value) in [
        (VolumeCategory::Master, 0.9),
        (VolumeCategory::Voice, 0.8),
        (VolumeCategory::Alerts, 0.7),
        (VolumeCategory::Effects, 0.6),
    ] {
        isolated.store().set_volume(category, value).unwrap();
    }
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&isolated.0).unwrap()).unwrap();
    assert_eq!(saved["future"]["keep"], true);
    assert_saved_volume(&saved["audio"]["master"], 0.9);
    assert_saved_volume(&saved["audio"]["voice"], 0.8);
    assert_saved_volume(&saved["audio"]["alerts"], 0.7);
    assert_saved_volume(&saved["audio"]["effects"], 0.6);
    assert!(
        isolated.store().load().notice.is_none(),
        "saving known defaults repairs the corrupt audio object"
    );
}

#[test]
fn corrupt_valid_json_settings_can_be_edited_and_reloaded_after_defaults_notice() {
    for (index, corrupt) in [
        r#"["future-root-shape"]"#,
        r#"{"future":{"keep":true},"audio":"bad"}"#,
    ]
    .into_iter()
    .enumerate()
    {
        let isolated = IsolatedStore::new();
        fs::write(&isolated.0, corrupt).unwrap();
        let before_edit = isolated.store().load();
        assert_eq!(
            before_edit.volumes,
            VolumeSettings::default(),
            "case {index} loads defaults"
        );
        assert!(
            before_edit.notice.is_some(),
            "case {index} emits a corruption notice"
        );

        isolated
            .store()
            .set_volume(VolumeCategory::Master, 0.4)
            .unwrap();

        let reloaded = isolated.store().load();
        assert_eq!(
            reloaded.volumes.master, 0.4,
            "case {index} persists the edited slider"
        );
        assert_eq!(
            reloaded.volumes.voice, 1.0,
            "case {index} retains default unedited sliders"
        );
        assert!(
            reloaded.notice.is_none(),
            "case {index} is repaired by the edit"
        );
        if index == 1 {
            let saved: serde_json::Value =
                serde_json::from_slice(&fs::read(&isolated.0).unwrap()).unwrap();
            assert_eq!(
                saved["future"]["keep"], true,
                "repair preserves unrelated root fields"
            );
        }
    }
}

#[test]
fn native_settings_file_is_under_the_user_config_turts_directory() {
    let config = std::env::temp_dir().join("isolated-user-config");
    assert_eq!(
        native_settings_path(&config),
        config.join("turts").join("settings.json")
    );
}

#[test]
fn shared_browser_settings_update_preserves_fields_owned_by_other_features() {
    let before = r#"{"scroll_speed":2.5,"ui_scale":1.25,"future":{"keep":true},"audio":{"master":0.9,"voice":0.8,"alerts":0.7,"effects":0.6}}"#;
    let updated = set_volume_in_json(Some(before), VolumeCategory::Alerts, 0.3).unwrap();
    let saved: serde_json::Value = serde_json::from_str(&updated).unwrap();
    assert_eq!(saved["scroll_speed"], 2.5);
    assert_eq!(saved["ui_scale"], 1.25);
    assert_eq!(saved["future"]["keep"], true);
    assert_eq!(saved["audio"]["master"], 0.9);
    assert_eq!(saved["audio"]["voice"], 0.8);
    assert_saved_volume(&saved["audio"]["alerts"], 0.3);
    assert_eq!(saved["audio"]["effects"], 0.6);
}

#[test]
fn volume_controls_produce_the_actual_master_times_category_gain() {
    let levels = VolumeSettings {
        master: 0.9,
        voice: 0.8,
        alerts: 0.7,
        effects: 0.6,
        ..Default::default()
    };
    assert_eq!(levels.gain(VolumeCategory::Master), 0.9);
    assert!((levels.gain(VolumeCategory::Voice) - 0.72).abs() < f32::EPSILON);
    assert!((levels.gain(VolumeCategory::Alerts) - 0.63).abs() < f32::EPSILON);
    assert!((levels.gain(VolumeCategory::Effects) - 0.54).abs() < f32::EPSILON);
}

const LEVELS: &str = r#"{"audio":{"master":0.35,"voice":0.2,"alerts":0.6,"effects":0.8}}"#;

#[test]
fn the_sound_is_off_by_default_and_a_stored_profile_with_no_flag_reads_as_off() {
    assert!(
        !VolumeSettings::default().sound_on,
        "the default settings make no sound"
    );
    let isolated = IsolatedStore::new();
    assert!(
        !isolated.store().load().volumes.sound_on,
        "a client with no stored document makes no sound"
    );
    // A document from before the flag existed has levels and no flag. It reads as the default:
    // the sound is off. The CEO decision (the game makes no sound by default) wants this.
    let old = decode_settings_json(LEVELS);
    assert!(!old.volumes.sound_on, "an old stored profile turns off");
    assert_eq!(old.volumes.master, 0.35, "its levels stay");
    assert_eq!(old.volumes.alerts, 0.6);
    assert!(old.notice.is_none(), "an old document is not corrupt");
    assert!(!decode_settings_json("{}").volumes.sound_on);
    assert!(!decode_settings_json(r#"{"audio":{}}"#).volumes.sound_on);
}

#[test]
fn a_stored_sound_on_flag_gives_sound_and_keeps_the_levels() {
    let loaded = decode_settings_json(
        r#"{"audio":{"master":0.35,"voice":0.2,"alerts":0.6,"effects":0.8,"sound_on":true}}"#,
    );
    assert!(
        loaded.volumes.sound_on,
        "the stored flag turns the sound on"
    );
    assert_eq!(loaded.volumes.master, 0.35);
    assert_eq!(loaded.volumes.voice, 0.2);
    assert!(loaded.notice.is_none());
    let off = decode_settings_json(r#"{"audio":{"master":0.35,"sound_on":false}}"#);
    assert!(!off.volumes.sound_on);
    assert_eq!(off.volumes.master, 0.35);
}

#[test]
fn an_invalid_stored_sound_flag_falls_back_like_an_invalid_level() {
    for bad in [r#""yes""#, "1", "0", "null", "[]", r#"{"on":true}"#, "0.5"] {
        let loaded = decode_settings_json(&format!(
            r#"{{"audio":{{"master":0.35,"sound_on":{bad}}}}}"#
        ));
        assert_eq!(
            loaded.volumes,
            VolumeSettings::default(),
            "a flag {bad} resets the audio settings to the defaults, as a bad level does"
        );
        assert!(!loaded.volumes.sound_on);
        assert!(
            loaded
                .notice
                .as_deref()
                .is_some_and(|text| !text.trim().is_empty()),
            "a flag {bad} gives one notice"
        );
    }
}

#[test]
fn the_flag_is_written_into_the_audio_object_and_the_levels_and_other_fields_stay() {
    let before = r#"{"future":{"keep":true},"controls":{"ui_scale":1.25},"audio":{"master":0.35,"voice":0.2,"alerts":0.6,"effects":0.8}}"#;
    let on = set_sound_on_in_json(Some(before), true).unwrap();
    let saved: serde_json::Value = serde_json::from_str(&on).unwrap();
    assert_eq!(saved["audio"]["sound_on"], true);
    assert_eq!(saved["future"]["keep"], true);
    assert_eq!(saved["controls"]["ui_scale"], 1.25);
    for (key, value) in [
        ("master", 0.35),
        ("voice", 0.2),
        ("alerts", 0.6),
        ("effects", 0.8),
    ] {
        assert_saved_volume(&saved["audio"][key], value);
    }
    let off = set_sound_on_in_json(Some(&on), false).unwrap();
    let reloaded = decode_settings_json(&off);
    assert!(!reloaded.volumes.sound_on);
    assert_eq!(
        (
            reloaded.volumes.master,
            reloaded.volumes.voice,
            reloaded.volumes.alerts,
            reloaded.volumes.effects
        ),
        (0.35, 0.2, 0.6, 0.8),
        "the slider values are kept across off and on"
    );
    let on_again = decode_settings_json(&set_sound_on_in_json(Some(&off), true).unwrap());
    assert!(on_again.volumes.sound_on);
    assert_eq!(on_again.volumes.alerts, 0.6);
    // A missing or damaged document becomes a document with the flag.
    let fresh: serde_json::Value =
        serde_json::from_str(&set_sound_on_in_json(None, true).unwrap()).unwrap();
    assert_eq!(fresh["audio"]["sound_on"], true);
    let damaged: serde_json::Value =
        serde_json::from_str(&set_sound_on_in_json(Some("{not json"), true).unwrap()).unwrap();
    assert_eq!(damaged["audio"]["sound_on"], true);
}

#[test]
fn the_native_store_keeps_the_flag_between_sessions() {
    let isolated = IsolatedStore::new();
    fs::write(&isolated.0, LEVELS).unwrap();
    isolated.store().set_sound_on(true).unwrap();
    let reloaded = isolated.store().load();
    assert!(reloaded.volumes.sound_on);
    assert_eq!(reloaded.volumes.voice, 0.2);
    isolated.store().set_sound_on(false).unwrap();
    assert!(!isolated.store().load().volumes.sound_on);
    assert_eq!(isolated.store().load().volumes.effects, 0.8);
}
